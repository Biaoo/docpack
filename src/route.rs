use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use miette::{Result, bail};
use serde::Serialize;

use crate::AppExit;
use crate::cli::{RouteArgs, RouteOutputFormat};
use crate::config::{load_impact_files, normalize_path, resolve_rule_path, root_dir_from_option};
use crate::git::get_tracked_paths;
use crate::rules::{RequiredDocMode, matches_pattern};

pub const ROUTE_SCHEMA_VERSION: &str = "docpact.route.v1";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RouteReport {
    pub schema_version: String,
    pub tool_name: String,
    pub tool_version: String,
    pub summary: RouteSummary,
    pub recommended_docs: Vec<RouteRecommendation>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RouteSummary {
    pub input_path_count: usize,
    pub matched_rule_count: usize,
    pub recommended_doc_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RouteRecommendation {
    pub path: String,
    pub rule_ids: Vec<String>,
    pub matched_input_paths: Vec<String>,
    pub matched_trigger_paths: Vec<String>,
    pub modes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedInput {
    original: String,
    candidates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecommendationBuilder {
    path: String,
    rule_ids: BTreeSet<String>,
    matched_input_paths: BTreeSet<String>,
    matched_trigger_paths: BTreeSet<String>,
    modes: BTreeSet<RequiredDocMode>,
}

pub fn run(args: RouteArgs) -> Result<AppExit> {
    let report = execute(&args)?;
    emit_report(&report, args.format);
    Ok(AppExit::Success)
}

pub fn execute(args: &RouteArgs) -> Result<RouteReport> {
    let root_dir = root_dir_from_option(args.root.as_deref())?;
    let loaded_rules = load_impact_files(&root_dir, args.config.as_deref())?;
    let inputs = parse_input_paths(&args.paths)?;
    let resolved_inputs = resolve_inputs(&root_dir, &inputs)?;

    let mut matched_rule_keys = BTreeSet::new();
    let mut recommendations = BTreeMap::<String, RecommendationBuilder>::new();

    for input in &resolved_inputs {
        for candidate_path in &input.candidates {
            for loaded in &loaded_rules {
                let matched_triggers = loaded
                    .rule
                    .triggers
                    .iter()
                    .map(|trigger| resolve_rule_path(&loaded.base_dir, &trigger.path))
                    .filter(|trigger_path| matches_pattern(candidate_path, trigger_path))
                    .collect::<Vec<_>>();

                if matched_triggers.is_empty() {
                    continue;
                }

                matched_rule_keys.insert(format!("{}::{}", loaded.config_source, loaded.rule.id));

                for required_doc in &loaded.rule.required_docs {
                    let path = resolve_rule_path(&loaded.base_dir, &required_doc.path);
                    let entry = recommendations.entry(path.clone()).or_insert_with(|| {
                        RecommendationBuilder {
                            path,
                            rule_ids: BTreeSet::new(),
                            matched_input_paths: BTreeSet::new(),
                            matched_trigger_paths: BTreeSet::new(),
                            modes: BTreeSet::new(),
                        }
                    });
                    entry.rule_ids.insert(loaded.rule.id.clone());
                    entry.matched_input_paths.insert(input.original.clone());
                    entry
                        .matched_trigger_paths
                        .extend(matched_triggers.iter().cloned());
                    entry
                        .modes
                        .insert(RequiredDocMode::from_option(required_doc.mode.as_deref()));
                }
            }
        }
    }

    let recommended_docs = recommendations
        .into_values()
        .map(|entry| RouteRecommendation {
            path: entry.path,
            rule_ids: entry.rule_ids.into_iter().collect(),
            matched_input_paths: entry.matched_input_paths.into_iter().collect(),
            matched_trigger_paths: entry.matched_trigger_paths.into_iter().collect(),
            modes: entry
                .modes
                .into_iter()
                .map(|mode| mode.as_str().to_string())
                .collect(),
        })
        .collect::<Vec<_>>();

    Ok(RouteReport {
        schema_version: ROUTE_SCHEMA_VERSION.into(),
        tool_name: env!("CARGO_PKG_NAME").into(),
        tool_version: env!("CARGO_PKG_VERSION").into(),
        summary: RouteSummary {
            input_path_count: inputs.len(),
            matched_rule_count: matched_rule_keys.len(),
            recommended_doc_count: recommended_docs.len(),
        },
        recommended_docs,
    })
}

fn parse_input_paths(paths: &str) -> Result<Vec<String>> {
    let values = paths
        .split(',')
        .map(|value| normalize_path(value.trim()))
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    if values.is_empty() {
        bail!("Pass at least one non-empty path through --paths.");
    }

    Ok(values)
}

fn resolve_inputs(root_dir: &Path, inputs: &[String]) -> Result<Vec<ResolvedInput>> {
    let tracked_paths = if inputs.iter().any(|value| has_glob_syntax(value)) {
        Some(get_tracked_paths(root_dir)?)
    } else {
        None
    };

    let mut resolved = Vec::with_capacity(inputs.len());

    for input in inputs {
        if has_glob_syntax(input) {
            let candidates = tracked_paths
                .as_ref()
                .expect("tracked paths should exist when glob syntax is present")
                .iter()
                .filter(|tracked| matches_pattern(tracked, input))
                .cloned()
                .collect::<Vec<_>>();
            resolved.push(ResolvedInput {
                original: input.clone(),
                candidates,
            });
        } else {
            resolved.push(ResolvedInput {
                original: input.clone(),
                candidates: vec![input.clone()],
            });
        }
    }

    Ok(resolved)
}

fn has_glob_syntax(value: &str) -> bool {
    value.contains('*') || value.contains('?')
}

fn emit_report(report: &RouteReport, format: RouteOutputFormat) {
    match format {
        RouteOutputFormat::Text => emit_text_report(report),
        RouteOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(report).expect("route report should serialize")
        ),
    }
}

fn emit_text_report(report: &RouteReport) {
    println!("Docpact route recommendations:");
    println!(
        "Summary: input_paths={} matched_rules={} recommended_docs={}",
        report.summary.input_path_count,
        report.summary.matched_rule_count,
        report.summary.recommended_doc_count,
    );

    println!("Recommended docs:");
    if report.recommended_docs.is_empty() {
        println!("- none");
        return;
    }

    for recommendation in &report.recommended_docs {
        println!(
            "- path={} rules={} inputs={}",
            recommendation.path,
            recommendation.rule_ids.join(","),
            recommendation.matched_input_paths.join(","),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{ROUTE_SCHEMA_VERSION, execute};
    use crate::cli::{RouteArgs, RouteOutputFormat};

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{nanos}-{}", std::process::id()));
        fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }

    fn git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .expect("git should run");
        assert!(
            status.success(),
            "git command failed: git {}",
            args.join(" ")
        );
    }

    fn init_git_repo(root: &Path) {
        git(root, &["init"]);
        git(root, &["config", "user.name", "Codex"]);
        git(root, &["config", "user.email", "codex@example.com"]);
    }

    fn base_args(root: PathBuf, paths: &str) -> RouteArgs {
        RouteArgs {
            root: Some(root),
            config: None,
            paths: paths.into(),
            format: RouteOutputFormat::Json,
        }
    }

    #[test]
    fn route_reports_required_docs_for_direct_paths() {
        let root = temp_dir("docpact-route-direct");
        init_git_repo(&root);
        fs::create_dir_all(root.join(".docpact")).expect("doc dir");
        fs::create_dir_all(root.join("src/payments")).expect("payments dir");
        fs::create_dir_all(root.join("docs")).expect("docs dir");

        fs::write(
            root.join(".docpact/config.yaml"),
            r#"
version: 1
layout: repo
repo:
  id: demo
rules:
  - id: payments-docs
    scope: repo
    repo: demo
    triggers:
      - path: src/payments/**
        kind: code
    requiredDocs:
      - path: docs/payments.md
        mode: body_update_required
    reason: Keep payments docs aligned.
"#,
        )
        .expect("config should be written");
        fs::write(
            root.join("src/payments/charge.ts"),
            "export const charge = 1;\n",
        )
        .expect("source file should be written");
        fs::write(root.join("docs/payments.md"), "# Payments\n")
            .expect("doc file should be written");
        git(&root, &["add", "."]);

        let report =
            execute(&base_args(root.clone(), "src/payments/charge.ts")).expect("route report");

        assert_eq!(report.schema_version, ROUTE_SCHEMA_VERSION);
        assert_eq!(report.summary.input_path_count, 1);
        assert_eq!(report.summary.matched_rule_count, 1);
        assert_eq!(report.summary.recommended_doc_count, 1);
        assert_eq!(report.recommended_docs[0].path, "docs/payments.md");
        assert_eq!(report.recommended_docs[0].rule_ids, vec!["payments-docs"]);
        assert_eq!(
            report.recommended_docs[0].matched_input_paths,
            vec!["src/payments/charge.ts"]
        );
        assert_eq!(
            report.recommended_docs[0].matched_trigger_paths,
            vec!["src/payments/**"]
        );
        assert_eq!(
            report.recommended_docs[0].modes,
            vec!["body_update_required"]
        );
    }

    #[test]
    fn route_expands_glob_inputs_against_tracked_paths() {
        let root = temp_dir("docpact-route-glob");
        init_git_repo(&root);
        fs::create_dir_all(root.join(".docpact")).expect("doc dir");
        fs::create_dir_all(root.join("src/auth")).expect("auth dir");
        fs::create_dir_all(root.join("docs")).expect("docs dir");

        fs::write(
            root.join(".docpact/config.yaml"),
            r#"
version: 1
layout: repo
repo:
  id: demo
rules:
  - id: auth-docs
    scope: repo
    repo: demo
    triggers:
      - path: src/auth/**
        kind: code
    requiredDocs:
      - path: docs/auth.md
        mode: review_or_update
    reason: Keep auth docs aligned.
"#,
        )
        .expect("config should be written");
        fs::write(root.join("src/auth/login.ts"), "export const login = 1;\n")
            .expect("auth file should be written");
        fs::write(
            root.join("src/auth/session.ts"),
            "export const session = 1;\n",
        )
        .expect("auth session file should be written");
        fs::write(root.join("docs/auth.md"), "# Auth\n").expect("doc file should be written");
        git(&root, &["add", "."]);

        let report = execute(&base_args(root.clone(), "src/auth/**")).expect("route report");

        assert_eq!(report.summary.input_path_count, 1);
        assert_eq!(report.summary.matched_rule_count, 1);
        assert_eq!(report.summary.recommended_doc_count, 1);
        assert_eq!(report.recommended_docs[0].path, "docs/auth.md");
        assert_eq!(report.recommended_docs[0].rule_ids, vec!["auth-docs"]);
        assert_eq!(
            report.recommended_docs[0].matched_input_paths,
            vec!["src/auth/**"]
        );
    }

    #[test]
    fn route_returns_empty_recommendations_when_no_rules_match() {
        let root = temp_dir("docpact-route-empty");
        init_git_repo(&root);
        fs::create_dir_all(root.join(".docpact")).expect("doc dir");
        fs::create_dir_all(root.join("src/auth")).expect("auth dir");

        fs::write(
            root.join(".docpact/config.yaml"),
            r#"
version: 1
layout: repo
repo:
  id: demo
rules:
  - id: auth-docs
    scope: repo
    repo: demo
    triggers:
      - path: src/auth/**
        kind: code
    requiredDocs:
      - path: docs/auth.md
        mode: review_or_update
    reason: Keep auth docs aligned.
"#,
        )
        .expect("config should be written");
        git(&root, &["add", "."]);

        let report = execute(&base_args(root.clone(), "src/payments/charge.ts"))
            .expect("route report should execute");

        assert_eq!(report.summary.input_path_count, 1);
        assert_eq!(report.summary.matched_rule_count, 0);
        assert_eq!(report.summary.recommended_doc_count, 0);
        assert!(report.recommended_docs.is_empty());
    }
}
