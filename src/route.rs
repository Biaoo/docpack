use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use miette::{IntoDiagnostic, Result, bail};
use serde::Serialize;

use crate::AppExit;
use crate::cli::{RouteArgs, RouteDetail, RouteOutputFormat};
use crate::config::{
    LoadedRoutingConfig, load_impact_files, load_routing_configs, normalize_path,
    resolve_rule_path, root_dir_from_option,
};
use crate::freshness::{FreshnessItem, RouteFreshnessTarget, execute_route_freshness_with_today};
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
    pub module_input_count: usize,
    pub intent_input_count: usize,
    pub matched_rule_count: usize,
    pub recommended_doc_count: usize,
    pub freshness_warning_count: usize,
    pub critical_freshness_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RouteRecommendation {
    pub path: String,
    pub priority: String,
    pub match_reason: RouteMatchReason,
    pub score_breakdown: RouteScoreBreakdown,
    pub freshness_level: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freshness_warning: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub review_reference_problems: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub config_sources: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rule_sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RouteMatchReason {
    pub rule_ids: Vec<String>,
    pub matched_input_paths: Vec<String>,
    pub matched_trigger_paths: Vec<String>,
    pub modes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RouteScoreBreakdown {
    pub mode_score: usize,
    pub specificity_score: usize,
    pub matched_input_count: usize,
    pub matched_rule_count: usize,
    pub freshness_penalty: usize,
    pub total_score: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedInput {
    original: String,
    candidates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedInputs {
    explicit_path_count: usize,
    module_count: usize,
    intent_count: usize,
    resolved_inputs: Vec<ResolvedInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RecommendationBuilder {
    path: String,
    rule_ids: BTreeSet<String>,
    matched_input_paths: BTreeSet<String>,
    matched_trigger_paths: BTreeSet<String>,
    modes: BTreeSet<RequiredDocMode>,
    config_sources: BTreeSet<String>,
    rule_sources: BTreeSet<String>,
    best_specificity_score: usize,
}

pub fn run(args: RouteArgs) -> Result<AppExit> {
    let report = execute(&args)?;
    emit_report(&report, args.format, args.detail, args.limit);
    Ok(AppExit::Success)
}

pub fn execute(args: &RouteArgs) -> Result<RouteReport> {
    let today = today_date_string()?;
    execute_with_today(args, &today)
}

fn execute_with_today(args: &RouteArgs, today: &str) -> Result<RouteReport> {
    let root_dir = root_dir_from_option(args.root.as_deref())?;
    let loaded_rules = load_impact_files(&root_dir, args.config.as_deref())?;
    let routing_configs = load_routing_configs(&root_dir, args.config.as_deref())?;
    let prepared_inputs = prepare_inputs(&root_dir, args, &routing_configs)?;

    let mut matched_rule_keys = BTreeSet::new();
    let mut recommendations = BTreeMap::<String, RecommendationBuilder>::new();

    for input in &prepared_inputs.resolved_inputs {
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
                let specificity_score = matched_triggers
                    .iter()
                    .map(|trigger| trigger_specificity_score(trigger))
                    .max()
                    .unwrap_or_default();

                for required_doc in &loaded.rule.required_docs {
                    let path = resolve_rule_path(&loaded.base_dir, &required_doc.path);
                    let entry = recommendations.entry(path.clone()).or_insert_with(|| {
                        RecommendationBuilder {
                            path,
                            rule_ids: BTreeSet::new(),
                            matched_input_paths: BTreeSet::new(),
                            matched_trigger_paths: BTreeSet::new(),
                            modes: BTreeSet::new(),
                            config_sources: BTreeSet::new(),
                            rule_sources: BTreeSet::new(),
                            best_specificity_score: 0,
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
                    entry.config_sources.insert(loaded.config_source.clone());
                    entry.rule_sources.insert(loaded.source.clone());
                    entry.best_specificity_score =
                        entry.best_specificity_score.max(specificity_score);
                }
            }
        }
    }

    let freshness_targets = recommendations
        .values()
        .map(|entry| RouteFreshnessTarget {
            path: entry.path.clone(),
            config_sources: entry.config_sources.iter().cloned().collect(),
            associated_patterns: entry.matched_trigger_paths.iter().cloned().collect(),
        })
        .collect::<Vec<_>>();
    let freshness_by_path = execute_route_freshness_with_today(
        &root_dir,
        args.config.as_deref(),
        &freshness_targets,
        today,
    )?;

    let include_sources = args.detail == RouteDetail::Full;
    let mut recommended_docs = recommendations
        .into_values()
        .map(|entry| {
            let freshness = freshness_by_path.get(&entry.path);
            build_recommendation(entry, freshness, include_sources)
        })
        .collect::<Vec<_>>();

    recommended_docs.sort_by(compare_recommendations);

    let freshness_warning_count = recommended_docs
        .iter()
        .filter(|item| item.freshness_level != "ok" || !item.review_reference_problems.is_empty())
        .count();
    let critical_freshness_count = recommended_docs
        .iter()
        .filter(|item| item.freshness_level == "critical")
        .count();

    Ok(RouteReport {
        schema_version: ROUTE_SCHEMA_VERSION.into(),
        tool_name: env!("CARGO_PKG_NAME").into(),
        tool_version: env!("CARGO_PKG_VERSION").into(),
        summary: RouteSummary {
            input_path_count: prepared_inputs.explicit_path_count,
            module_input_count: prepared_inputs.module_count,
            intent_input_count: prepared_inputs.intent_count,
            matched_rule_count: matched_rule_keys.len(),
            recommended_doc_count: recommended_docs.len(),
            freshness_warning_count,
            critical_freshness_count,
        },
        recommended_docs,
    })
}

fn build_recommendation(
    entry: RecommendationBuilder,
    freshness: Option<&FreshnessItem>,
    include_sources: bool,
) -> RouteRecommendation {
    let mode_score = entry
        .modes
        .iter()
        .map(|mode| mode_score(*mode))
        .max()
        .unwrap_or_default();
    let matched_input_count = entry.matched_input_paths.len();
    let matched_rule_count = entry.rule_ids.len();
    let base_score = mode_score
        + entry.best_specificity_score
        + matched_input_count * 3
        + matched_rule_count * 2;
    let freshness_penalty = freshness.map(freshness_penalty).unwrap_or_default();
    let total_score = base_score.saturating_sub(freshness_penalty);
    let priority = priority_from_score(total_score);
    let freshness_level = freshness
        .map(|item| item.staleness_level.clone())
        .unwrap_or_else(|| "ok".into());
    let review_reference_problems = freshness
        .map(|item| item.review_reference_problems.clone())
        .unwrap_or_default();
    let freshness_warning = build_freshness_warning(freshness);

    RouteRecommendation {
        path: entry.path,
        priority: priority.into(),
        match_reason: RouteMatchReason {
            rule_ids: entry.rule_ids.into_iter().collect(),
            matched_input_paths: entry.matched_input_paths.into_iter().collect(),
            matched_trigger_paths: entry.matched_trigger_paths.into_iter().collect(),
            modes: entry
                .modes
                .into_iter()
                .map(|mode| mode.as_str().to_string())
                .collect(),
        },
        score_breakdown: RouteScoreBreakdown {
            mode_score,
            specificity_score: entry.best_specificity_score,
            matched_input_count,
            matched_rule_count,
            freshness_penalty,
            total_score,
        },
        freshness_level,
        freshness_warning,
        review_reference_problems,
        config_sources: if include_sources {
            entry.config_sources.into_iter().collect()
        } else {
            Vec::new()
        },
        rule_sources: if include_sources {
            entry.rule_sources.into_iter().collect()
        } else {
            Vec::new()
        },
    }
}

fn compare_recommendations(left: &RouteRecommendation, right: &RouteRecommendation) -> Ordering {
    right
        .score_breakdown
        .total_score
        .cmp(&left.score_breakdown.total_score)
        .then_with(|| {
            right
                .score_breakdown
                .mode_score
                .cmp(&left.score_breakdown.mode_score)
        })
        .then_with(|| {
            right
                .score_breakdown
                .specificity_score
                .cmp(&left.score_breakdown.specificity_score)
        })
        .then_with(|| {
            right
                .score_breakdown
                .matched_input_count
                .cmp(&left.score_breakdown.matched_input_count)
        })
        .then_with(|| {
            right
                .score_breakdown
                .matched_rule_count
                .cmp(&left.score_breakdown.matched_rule_count)
        })
        .then_with(|| left.path.cmp(&right.path))
}

fn mode_score(mode: RequiredDocMode) -> usize {
    match mode {
        RequiredDocMode::BodyUpdateRequired => 40,
        RequiredDocMode::MetadataRefreshRequired => 30,
        RequiredDocMode::ReviewOrUpdate => 20,
        RequiredDocMode::MustExist => 10,
    }
}

fn priority_from_score(total_score: usize) -> &'static str {
    if total_score >= 50 {
        "high"
    } else if total_score >= 30 {
        "medium"
    } else {
        "low"
    }
}

fn trigger_specificity_score(pattern: &str) -> usize {
    let segments = pattern
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let segment_count = segments.len();
    let wildcard_segments = segments
        .iter()
        .filter(|segment| segment.contains('*') || segment.contains('?'))
        .count();
    let recursive_segments = segments.iter().filter(|segment| **segment == "**").count();
    let literal_segments = segment_count.saturating_sub(wildcard_segments);
    let literal_chars = pattern
        .chars()
        .filter(|ch| *ch != '*' && *ch != '?' && *ch != '/')
        .count();

    let raw_score = literal_segments * 4 + (literal_chars.min(12) / 2) + segment_count.min(4);
    let penalty = wildcard_segments * 3 + recursive_segments * 4;

    raw_score.saturating_sub(penalty).min(20)
}

fn prepare_inputs(
    root_dir: &Path,
    args: &RouteArgs,
    routing_configs: &[LoadedRoutingConfig],
) -> Result<PreparedInputs> {
    let explicit_paths = parse_optional_csv_inputs(args.paths.as_deref())?;
    let modules = parse_named_inputs(&args.module, "module")?;
    let intents = parse_named_inputs(&args.intent, "intent")?;

    if explicit_paths.is_empty() && modules.is_empty() && intents.is_empty() {
        bail!("Pass at least one non-empty route input through --paths, --module, or --intent.");
    }

    let tracked_paths = get_tracked_paths(root_dir)?;
    let mut resolved_inputs = Vec::new();

    for input in &explicit_paths {
        if has_glob_syntax(input) {
            let candidates = tracked_paths
                .iter()
                .filter(|tracked| matches_pattern(tracked, input))
                .cloned()
                .collect::<Vec<_>>();
            resolved_inputs.push(ResolvedInput {
                original: input.clone(),
                candidates,
            });
        } else {
            resolved_inputs.push(ResolvedInput {
                original: input.clone(),
                candidates: vec![input.clone()],
            });
        }
    }

    for module in &modules {
        if has_glob_syntax(module) {
            bail!(
                "`--module` does not accept glob syntax; pass a repo-relative path prefix instead."
            );
        }

        let prefix = module.trim_end_matches('/').to_string();
        let candidates = tracked_paths
            .iter()
            .filter(|tracked| matches_module_scope(tracked, &prefix))
            .cloned()
            .collect::<Vec<_>>();
        resolved_inputs.push(ResolvedInput {
            original: format!("module:{prefix}"),
            candidates,
        });
    }

    let intent_index = build_intent_index(routing_configs)?;
    for intent in &intents {
        let Some(patterns) = intent_index.get(intent) else {
            bail!("Unknown routing intent alias `{intent}`.");
        };

        let candidates = tracked_paths
            .iter()
            .filter(|tracked| {
                patterns
                    .iter()
                    .any(|pattern| matches_pattern(tracked, pattern))
            })
            .cloned()
            .collect::<Vec<_>>();
        resolved_inputs.push(ResolvedInput {
            original: format!("intent:{intent}"),
            candidates,
        });
    }

    Ok(PreparedInputs {
        explicit_path_count: explicit_paths.len(),
        module_count: modules.len(),
        intent_count: intents.len(),
        resolved_inputs,
    })
}

fn parse_optional_csv_inputs(values: Option<&str>) -> Result<Vec<String>> {
    Ok(values
        .unwrap_or_default()
        .split(',')
        .map(|value| normalize_path(value.trim()))
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>())
}

fn parse_named_inputs(values: &[String], flag_name: &str) -> Result<Vec<String>> {
    let parsed = values
        .iter()
        .map(|value| normalize_path(value.trim()))
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    if values.iter().any(|value| value.trim().is_empty()) {
        bail!("`--{flag_name}` must not include empty values.");
    }

    Ok(parsed)
}

fn build_intent_index(
    routing_configs: &[LoadedRoutingConfig],
) -> Result<BTreeMap<String, Vec<String>>> {
    let mut index = BTreeMap::<String, (String, Vec<String>)>::new();

    for loaded in routing_configs {
        for (alias, intent) in &loaded.routing.intents {
            let resolved_patterns = intent
                .paths
                .iter()
                .map(|pattern| resolve_rule_path(&loaded.base_dir, pattern))
                .collect::<Vec<_>>();

            if let Some((existing_source, _)) = index.get(alias) {
                bail!(
                    "routing intent alias `{alias}` is ambiguous across `{existing_source}` and `{}`",
                    loaded.source
                );
            }

            index.insert(alias.clone(), (loaded.source.clone(), resolved_patterns));
        }
    }

    Ok(index
        .into_iter()
        .map(|(alias, (_, patterns))| (alias, patterns))
        .collect())
}

fn matches_module_scope(tracked_path: &str, module: &str) -> bool {
    tracked_path == module
        || tracked_path
            .strip_prefix(module)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn has_glob_syntax(value: &str) -> bool {
    value.contains('*') || value.contains('?')
}

fn freshness_penalty(item: &FreshnessItem) -> usize {
    match item.staleness_level.as_str() {
        "critical" => 20,
        "warn" => 10,
        _ => 0,
    }
}

fn build_freshness_warning(item: Option<&FreshnessItem>) -> Option<String> {
    let item = item?;
    let mut parts = Vec::new();

    match item.staleness_level.as_str() {
        "critical" => parts.push("potentially stale (critical)".to_string()),
        "warn" => parts.push("potentially stale (warn)".to_string()),
        _ => {}
    }

    if !item.review_reference_problems.is_empty() {
        parts.push(format!(
            "review references: {}",
            item.review_reference_problems.join(",")
        ));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("; "))
    }
}

fn today_date_string() -> Result<String> {
    let output = std::process::Command::new("date")
        .args(["+%F"])
        .output()
        .into_diagnostic()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!("date +%F failed: {stderr}");
    }

    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_string())
        .map_err(|error| miette::miette!("date output was not valid UTF-8: {error}"))
}

fn emit_report(
    report: &RouteReport,
    format: RouteOutputFormat,
    detail: RouteDetail,
    limit: Option<usize>,
) {
    match format {
        RouteOutputFormat::Text => print!("{}", render_text_report(report, detail, limit)),
        RouteOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(report).expect("route report should serialize")
        ),
    }
}

fn render_text_report(report: &RouteReport, detail: RouteDetail, limit: Option<usize>) -> String {
    let mut output = String::new();
    output.push_str("Docpact route recommendations:\n");
    output.push_str(&format!(
        "Summary: input_paths={} modules={} intents={} matched_rules={} recommended_docs={} freshness_warnings={} critical_freshness={}\n",
        report.summary.input_path_count,
        report.summary.module_input_count,
        report.summary.intent_input_count,
        report.summary.matched_rule_count,
        report.summary.recommended_doc_count,
        report.summary.freshness_warning_count,
        report.summary.critical_freshness_count,
    ));

    let displayed = limit
        .map(|value| value.min(report.recommended_docs.len()))
        .unwrap_or(report.recommended_docs.len());
    if displayed < report.recommended_docs.len() {
        output.push_str(&format!(
            "Recommended docs (showing {} of {}):\n",
            displayed,
            report.recommended_docs.len()
        ));
    } else {
        output.push_str("Recommended docs:\n");
    }

    if report.recommended_docs.is_empty() {
        output.push_str("- none\n");
        return output;
    }

    for recommendation in report.recommended_docs.iter().take(displayed) {
        output.push_str(&format!(
            "- path={} priority={} freshness={} rules={} inputs={}\n",
            recommendation.path,
            recommendation.priority,
            recommendation.freshness_level,
            recommendation.match_reason.rule_ids.join(","),
            recommendation.match_reason.matched_input_paths.join(","),
        ));

        if detail == RouteDetail::Compact {
            if let Some(warning) = &recommendation.freshness_warning {
                output.push_str(&format!("  freshness_warning={warning}\n"));
            }
            continue;
        }

        output.push_str(&format!(
            "  triggers={}\n",
            recommendation.match_reason.matched_trigger_paths.join(",")
        ));
        output.push_str(&format!(
            "  modes={}\n",
            recommendation.match_reason.modes.join(",")
        ));
        output.push_str(&format!(
            "  score mode={} specificity={} matched_inputs={} matched_rules={} freshness_penalty={} total={}\n",
            recommendation.score_breakdown.mode_score,
            recommendation.score_breakdown.specificity_score,
            recommendation.score_breakdown.matched_input_count,
            recommendation.score_breakdown.matched_rule_count,
            recommendation.score_breakdown.freshness_penalty,
            recommendation.score_breakdown.total_score,
        ));
        if let Some(warning) = &recommendation.freshness_warning {
            output.push_str(&format!("  freshness_warning={warning}\n"));
        }
        if !recommendation.review_reference_problems.is_empty() {
            output.push_str(&format!(
                "  review_reference_problems={}\n",
                recommendation.review_reference_problems.join(",")
            ));
        }
        if !recommendation.config_sources.is_empty() {
            output.push_str(&format!(
                "  config_sources={}\n",
                recommendation.config_sources.join(",")
            ));
        }
        if !recommendation.rule_sources.is_empty() {
            output.push_str(&format!(
                "  rule_sources={}\n",
                recommendation.rule_sources.join(",")
            ));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{ROUTE_SCHEMA_VERSION, execute_with_today, render_text_report};
    use crate::cli::{RouteArgs, RouteDetail, RouteOutputFormat};

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

    fn git_stdout(root: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("git should run");
        assert!(
            output.status.success(),
            "git command failed: git {}",
            args.join(" ")
        );
        String::from_utf8(output.stdout)
            .expect("git stdout should be utf-8")
            .trim()
            .to_string()
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
            paths: Some(paths.into()),
            module: Vec::new(),
            intent: Vec::new(),
            detail: RouteDetail::Compact,
            limit: None,
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

        let report = execute_with_today(
            &base_args(root.clone(), "src/payments/charge.ts"),
            "2026-04-22",
        )
        .expect("route report");

        assert_eq!(report.schema_version, ROUTE_SCHEMA_VERSION);
        assert_eq!(report.summary.input_path_count, 1);
        assert_eq!(report.summary.matched_rule_count, 1);
        assert_eq!(report.summary.recommended_doc_count, 1);
        assert_eq!(report.summary.freshness_warning_count, 1);
        assert_eq!(report.summary.critical_freshness_count, 0);
        let recommendation = &report.recommended_docs[0];
        assert_eq!(recommendation.path, "docs/payments.md");
        assert_eq!(recommendation.priority, "high");
        assert_eq!(recommendation.match_reason.rule_ids, vec!["payments-docs"]);
        assert_eq!(
            recommendation.match_reason.matched_input_paths,
            vec!["src/payments/charge.ts"]
        );
        assert_eq!(
            recommendation.match_reason.matched_trigger_paths,
            vec!["src/payments/**"]
        );
        assert_eq!(
            recommendation.match_reason.modes,
            vec!["body_update_required"]
        );
        assert_eq!(recommendation.score_breakdown.mode_score, 40);
        assert!(recommendation.score_breakdown.total_score >= 50);
        assert_eq!(recommendation.freshness_level, "ok");
        assert!(recommendation.freshness_warning.is_some());
        assert!(
            recommendation
                .review_reference_problems
                .contains(&"missing-lastReviewedCommit".to_string())
        );
        assert!(recommendation.config_sources.is_empty());
        assert!(recommendation.rule_sources.is_empty());
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

        let report = execute_with_today(&base_args(root.clone(), "src/auth/**"), "2026-04-22")
            .expect("route report");

        assert_eq!(report.summary.input_path_count, 1);
        assert_eq!(report.summary.matched_rule_count, 1);
        assert_eq!(report.summary.recommended_doc_count, 1);
        assert_eq!(report.recommended_docs[0].path, "docs/auth.md");
        assert_eq!(report.recommended_docs[0].priority, "medium");
        assert_eq!(
            report.recommended_docs[0].match_reason.matched_input_paths,
            vec!["src/auth/**"]
        );
    }

    #[test]
    fn route_expands_module_inputs_against_tracked_paths() {
        let root = temp_dir("docpact-route-module");
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
        mode: review_or_update
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

        let mut args = base_args(root.clone(), "");
        args.paths = None;
        args.module = vec!["src/payments".into()];
        let report = execute_with_today(&args, "2026-04-22").expect("route report");

        assert_eq!(report.summary.input_path_count, 0);
        assert_eq!(report.summary.module_input_count, 1);
        assert_eq!(report.summary.intent_input_count, 0);
        assert_eq!(report.summary.recommended_doc_count, 1);
        assert_eq!(
            report.recommended_docs[0].match_reason.matched_input_paths,
            vec!["module:src/payments"]
        );
    }

    #[test]
    fn route_resolves_controlled_intent_aliases() {
        let root = temp_dir("docpact-route-intent");
        init_git_repo(&root);
        fs::create_dir_all(root.join(".docpact")).expect("doc dir");
        fs::create_dir_all(root.join("src/auth")).expect("auth dir");
        fs::create_dir_all(root.join("docs")).expect("docs dir");

        fs::write(
            root.join(".docpact/config.yaml"),
            r#"
version: 1
layout: repo
routing:
  intents:
    auth:
      paths:
        - src/auth/**
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
        mode: body_update_required
    reason: Keep auth docs aligned.
"#,
        )
        .expect("config should be written");
        fs::write(root.join("src/auth/login.ts"), "export const login = 1;\n")
            .expect("source file should be written");
        fs::write(root.join("docs/auth.md"), "# Auth\n").expect("doc file should be written");
        git(&root, &["add", "."]);

        let mut args = base_args(root.clone(), "");
        args.paths = None;
        args.intent = vec!["auth".into()];
        let report = execute_with_today(&args, "2026-04-22").expect("route report");

        assert_eq!(report.summary.input_path_count, 0);
        assert_eq!(report.summary.module_input_count, 0);
        assert_eq!(report.summary.intent_input_count, 1);
        assert_eq!(report.summary.recommended_doc_count, 1);
        assert_eq!(
            report.recommended_docs[0].match_reason.matched_input_paths,
            vec!["intent:auth"]
        );
    }

    #[test]
    fn route_rejects_unknown_intent_aliases() {
        let root = temp_dir("docpact-route-unknown-intent");
        init_git_repo(&root);
        fs::create_dir_all(root.join(".docpact")).expect("doc dir");
        fs::write(
            root.join(".docpact/config.yaml"),
            r#"
version: 1
layout: repo
repo:
  id: demo
rules: []
"#,
        )
        .expect("config should be written");
        git(&root, &["add", "."]);

        let mut args = base_args(root.clone(), "");
        args.paths = None;
        args.intent = vec!["missing".into()];
        let error = execute_with_today(&args, "2026-04-22").expect_err("route should fail");
        assert!(
            error
                .to_string()
                .contains("Unknown routing intent alias `missing`")
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

        let report = execute_with_today(
            &base_args(root.clone(), "src/payments/charge.ts"),
            "2026-04-22",
        )
        .expect("route report should execute");

        assert_eq!(report.summary.input_path_count, 1);
        assert_eq!(report.summary.matched_rule_count, 0);
        assert_eq!(report.summary.recommended_doc_count, 0);
        assert!(report.recommended_docs.is_empty());
    }

    #[test]
    fn route_ranking_prefers_stronger_modes_then_specificity() {
        let root = temp_dir("docpact-route-ranking");
        init_git_repo(&root);
        fs::create_dir_all(root.join(".docpact")).expect("doc dir");
        fs::create_dir_all(root.join("src/payments/admin")).expect("payments dir");
        fs::create_dir_all(root.join("docs")).expect("docs dir");

        fs::write(
            root.join(".docpact/config.yaml"),
            r#"
version: 1
layout: repo
repo:
  id: demo
rules:
  - id: broad-review
    scope: repo
    repo: demo
    triggers:
      - path: src/payments/**
        kind: code
    requiredDocs:
      - path: docs/broad.md
        mode: review_or_update
    reason: broad
  - id: exact-body
    scope: repo
    repo: demo
    triggers:
      - path: src/payments/admin/panel.ts
        kind: code
    requiredDocs:
      - path: docs/exact.md
        mode: body_update_required
    reason: exact
  - id: exact-metadata
    scope: repo
    repo: demo
    triggers:
      - path: src/payments/admin/panel.ts
        kind: code
    requiredDocs:
      - path: docs/meta.md
        mode: metadata_refresh_required
    reason: metadata
"#,
        )
        .expect("config should be written");
        fs::write(
            root.join("src/payments/admin/panel.ts"),
            "export const panel = 1;\n",
        )
        .expect("source file should be written");
        fs::write(root.join("docs/broad.md"), "# Broad\n").expect("doc file");
        fs::write(root.join("docs/exact.md"), "# Exact\n").expect("doc file");
        fs::write(root.join("docs/meta.md"), "# Meta\n").expect("doc file");
        git(&root, &["add", "."]);

        let report = execute_with_today(
            &base_args(root.clone(), "src/payments/admin/panel.ts"),
            "2026-04-22",
        )
        .expect("route report should execute");

        let ordered_paths = report
            .recommended_docs
            .iter()
            .map(|item| item.path.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ordered_paths,
            vec!["docs/exact.md", "docs/meta.md", "docs/broad.md"]
        );
        assert_eq!(report.recommended_docs[0].priority, "high");
        assert_eq!(report.recommended_docs[1].priority, "high");
        assert_eq!(report.recommended_docs[2].priority, "medium");
    }

    #[test]
    fn route_full_detail_exposes_sources_and_full_text_explanations() {
        let root = temp_dir("docpact-route-full");
        init_git_repo(&root);
        fs::create_dir_all(root.join(".docpact")).expect("doc dir");
        fs::create_dir_all(root.join("src/api")).expect("api dir");
        fs::create_dir_all(root.join("docs")).expect("docs dir");

        fs::write(
            root.join(".docpact/config.yaml"),
            r#"
version: 1
layout: repo
repo:
  id: demo
rules:
  - id: api-docs
    scope: repo
    repo: demo
    triggers:
      - path: src/api/**
        kind: code
    requiredDocs:
      - path: docs/api.md
        mode: review_or_update
    reason: api
"#,
        )
        .expect("config should be written");
        fs::write(root.join("src/api/client.ts"), "export const client = 1;\n")
            .expect("source file should be written");
        fs::write(root.join("docs/api.md"), "# API\n").expect("doc file");
        git(&root, &["add", "."]);

        let mut args = base_args(root.clone(), "src/api/client.ts");
        args.detail = RouteDetail::Full;
        let report = execute_with_today(&args, "2026-04-22").expect("route report should execute");
        let recommendation = &report.recommended_docs[0];
        assert_eq!(recommendation.config_sources, vec![".docpact/config.yaml"]);
        assert_eq!(recommendation.rule_sources, vec![".docpact/config.yaml"]);

        let rendered = render_text_report(&report, RouteDetail::Full, Some(1));
        assert!(rendered.contains("priority="));
        assert!(rendered.contains("freshness="));
        assert!(rendered.contains("triggers=src/api/**"));
        assert!(rendered.contains("score mode="));
        assert!(rendered.contains("freshness_penalty="));
        assert!(rendered.contains("config_sources=.docpact/config.yaml"));
        assert!(rendered.contains("rule_sources=.docpact/config.yaml"));
    }

    #[test]
    fn route_text_limit_only_affects_rendered_rows() {
        let root = temp_dir("docpact-route-limit");
        init_git_repo(&root);
        fs::create_dir_all(root.join(".docpact")).expect("doc dir");
        fs::create_dir_all(root.join("src")).expect("src dir");
        fs::create_dir_all(root.join("docs")).expect("docs dir");

        fs::write(
            root.join(".docpact/config.yaml"),
            r#"
version: 1
layout: repo
repo:
  id: demo
rules:
  - id: one
    scope: repo
    repo: demo
    triggers:
      - path: src/file-a.ts
        kind: code
    requiredDocs:
      - path: docs/a.md
        mode: review_or_update
    reason: a
  - id: two
    scope: repo
    repo: demo
    triggers:
      - path: src/file-b.ts
        kind: code
    requiredDocs:
      - path: docs/b.md
        mode: review_or_update
    reason: b
"#,
        )
        .expect("config should be written");
        fs::write(root.join("src/file-a.ts"), "export const a = 1;\n").expect("source file");
        fs::write(root.join("src/file-b.ts"), "export const b = 1;\n").expect("source file");
        fs::write(root.join("docs/a.md"), "# A\n").expect("doc file");
        fs::write(root.join("docs/b.md"), "# B\n").expect("doc file");
        git(&root, &["add", "."]);

        let report = execute_with_today(
            &base_args(root.clone(), "src/file-a.ts,src/file-b.ts"),
            "2026-04-22",
        )
        .expect("route report should execute");
        let rendered = render_text_report(&report, RouteDetail::Compact, Some(1));
        assert!(rendered.contains("showing 1 of 2"));
        assert!(rendered.contains("path=docs/a.md") || rendered.contains("path=docs/b.md"));
    }

    #[test]
    fn route_demotes_stale_docs_and_surfaces_invalid_review_references() {
        let root = temp_dir("docpact-route-freshness");
        init_git_repo(&root);
        fs::create_dir_all(root.join(".docpact")).expect("doc dir");
        fs::create_dir_all(root.join("src/api")).expect("api dir");
        fs::create_dir_all(root.join("docs")).expect("docs dir");

        fs::write(
            root.join(".docpact/config.yaml"),
            r#"
version: 1
layout: repo
freshness:
  warn_after_commits: 1
  warn_after_days: 30
  critical_after_days: 180
repo:
  id: demo
rules:
  - id: api-stale
    scope: repo
    repo: demo
    triggers:
      - path: src/api/client.ts
        kind: code
    requiredDocs:
      - path: docs/stale.md
        mode: body_update_required
      - path: docs/broken.md
        mode: body_update_required
    reason: stale
  - id: api-fresh
    scope: repo
    repo: demo
    triggers:
      - path: src/api/client.ts
        kind: code
    requiredDocs:
      - path: docs/fresh.md
        mode: body_update_required
    reason: fresh
"#,
        )
        .expect("config");
        fs::write(root.join("src/api/client.ts"), "export const client = 1;\n").expect("src");
        fs::write(
            root.join("docs/stale.md"),
            "---\nlastReviewedAt: 2025-01-01\nlastReviewedCommit: deadbeef\n---\n# Stale\n",
        )
        .expect("stale doc");
        fs::write(root.join("docs/fresh.md"), "# Fresh\n").expect("fresh doc");
        fs::write(root.join("docs/broken.md"), "# Broken\n").expect("broken doc");
        git(&root, &["add", "."]);
        let base = git_commit_all(&root, "base");

        fs::write(root.join("src/api/client.ts"), "export const client = 2;\n").expect("src");
        git(&root, &["add", "src/api/client.ts"]);
        let _head = git_commit_all(&root, "change");

        fs::write(
            root.join("docs/fresh.md"),
            format!("---\nlastReviewedAt: 2026-04-20\nlastReviewedCommit: {base}\n---\n# Fresh\n"),
        )
        .expect("fresh doc update");

        let report =
            execute_with_today(&base_args(root.clone(), "src/api/client.ts"), "2026-04-22")
                .expect("route report should execute");
        let ordered_paths = report
            .recommended_docs
            .iter()
            .map(|item| item.path.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ordered_paths,
            vec!["docs/broken.md", "docs/fresh.md", "docs/stale.md"]
        );
        assert_eq!(report.summary.freshness_warning_count, 3);
        assert_eq!(report.summary.critical_freshness_count, 1);

        let broken = report
            .recommended_docs
            .iter()
            .find(|item| item.path == "docs/broken.md")
            .expect("broken recommendation");
        assert_eq!(broken.freshness_level, "ok");
        assert!(
            broken
                .review_reference_problems
                .contains(&"missing-lastReviewedCommit".to_string())
        );
        assert!(broken.freshness_warning.is_some());

        let stale = report
            .recommended_docs
            .iter()
            .find(|item| item.path == "docs/stale.md")
            .expect("stale recommendation");
        assert_eq!(stale.freshness_level, "critical");
        assert!(
            stale
                .review_reference_problems
                .contains(&"invalid-lastReviewedCommit".to_string())
        );
        assert!(stale.score_breakdown.freshness_penalty > 0);
        assert!(stale.freshness_warning.is_some());

        let fresh = report
            .recommended_docs
            .iter()
            .find(|item| item.path == "docs/fresh.md")
            .expect("fresh recommendation");
        assert_eq!(fresh.freshness_level, "warn");
        assert_eq!(fresh.score_breakdown.freshness_penalty, 10);
        assert!(fresh.freshness_warning.is_some());
    }

    fn git_commit_all(root: &Path, message: &str) -> String {
        git(root, &["commit", "-m", message]);
        git_stdout(root, &["rev-parse", "HEAD"])
    }
}
