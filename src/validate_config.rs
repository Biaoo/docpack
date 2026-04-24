use miette::Result;
use serde::Serialize;

use crate::AppExit;
use crate::cli::{ValidateConfigArgs, ValidateConfigOutputFormat};
use crate::config::{
    ConfigValidationProblem, load_catalog_configs, load_coverage_configs,
    load_doc_inventory_configs, load_freshness_configs, load_impact_files, load_ownership_configs,
    load_routing_configs, root_dir_from_option, validate_config_graph,
    validate_loaded_catalog_configs, validate_loaded_coverage_configs,
    validate_loaded_doc_inventory_configs, validate_loaded_freshness_configs,
    validate_loaded_ownership_configs, validate_loaded_routing_configs, validate_loaded_rules,
    validate_ownership_path_conflicts,
};
use crate::git::get_tracked_paths;

pub const VALIDATE_CONFIG_SCHEMA_VERSION: &str = "docpact.validate-config.v1";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ValidateConfigReport {
    pub schema_version: String,
    pub tool_name: String,
    pub tool_version: String,
    pub command: String,
    pub summary: ValidateConfigSummary,
    pub warnings: Vec<ValidateConfigWarning>,
    pub problems: Vec<ValidateConfigProblem>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ValidateConfigSummary {
    pub status: String,
    pub strict: bool,
    pub rule_count: usize,
    pub problem_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ValidateConfigWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ValidateConfigProblem {
    pub code: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    pub message: String,
}

pub fn run(args: ValidateConfigArgs) -> Result<AppExit> {
    let format = args.format;
    let report = execute(&args)?;
    emit_report(&report, format);
    if report.summary.status == "fail" {
        Ok(AppExit::LintFailure)
    } else {
        Ok(AppExit::Success)
    }
}

pub fn execute(args: &ValidateConfigArgs) -> Result<ValidateConfigReport> {
    let root_dir = root_dir_from_option(args.root.as_deref())?;
    let rules = load_impact_files(&root_dir, args.config.as_deref())?;
    let coverage_configs = load_coverage_configs(&root_dir, args.config.as_deref())?;
    let freshness_configs = load_freshness_configs(&root_dir, args.config.as_deref())?;
    let routing_configs = load_routing_configs(&root_dir, args.config.as_deref())?;
    let doc_inventory_configs = load_doc_inventory_configs(&root_dir, args.config.as_deref())?;
    let catalog_configs = load_catalog_configs(&root_dir, args.config.as_deref())?;
    let ownership_configs = load_ownership_configs(&root_dir, args.config.as_deref())?;

    let mut warnings = Vec::new();
    if !args.strict {
        warnings.push(ValidateConfigWarning {
            code: "strict-validation-skipped".into(),
            message: "default mode only checks that effective config loads; use --strict for graph and ownership checks"
                .into(),
        });
        return Ok(build_report(args.strict, rules.len(), warnings, Vec::new()));
    }

    let mut problems = validate_config_graph(&root_dir, args.config.as_deref())?;
    problems.extend(validate_loaded_rules(&rules));
    problems.extend(validate_loaded_coverage_configs(&coverage_configs));
    problems.extend(validate_loaded_freshness_configs(&freshness_configs));
    problems.extend(validate_loaded_routing_configs(&routing_configs));
    problems.extend(validate_loaded_doc_inventory_configs(
        &doc_inventory_configs,
    ));
    problems.extend(validate_loaded_catalog_configs(&catalog_configs));
    problems.extend(validate_loaded_ownership_configs(
        &ownership_configs,
        &catalog_configs,
    ));
    let ownership_domain_count = ownership_configs
        .iter()
        .map(|config| config.ownership.domains.len())
        .sum::<usize>();
    if ownership_domain_count > 0 {
        let tracked_paths = get_tracked_paths(&root_dir)?;
        let analysis = crate::config::analyze_ownership_paths(&tracked_paths, &ownership_configs);
        problems.extend(validate_ownership_path_conflicts(&analysis));
    }
    problems.sort_by(|left, right| {
        (&left.source, &left.rule_id, &left.message).cmp(&(
            &right.source,
            &right.rule_id,
            &right.message,
        ))
    });
    Ok(build_report(args.strict, rules.len(), warnings, problems))
}

fn build_report(
    strict: bool,
    rule_count: usize,
    warnings: Vec<ValidateConfigWarning>,
    problems: Vec<ConfigValidationProblem>,
) -> ValidateConfigReport {
    let problems = problems
        .into_iter()
        .map(problem_summary)
        .collect::<Vec<_>>();
    ValidateConfigReport {
        schema_version: VALIDATE_CONFIG_SCHEMA_VERSION.into(),
        tool_name: env!("CARGO_PKG_NAME").into(),
        tool_version: env!("CARGO_PKG_VERSION").into(),
        command: "validate-config".into(),
        summary: ValidateConfigSummary {
            status: if problems.is_empty() { "ok" } else { "fail" }.into(),
            strict,
            rule_count,
            problem_count: problems.len(),
        },
        warnings,
        problems,
    }
}

fn problem_summary(problem: ConfigValidationProblem) -> ValidateConfigProblem {
    ValidateConfigProblem {
        code: "invalid-config".into(),
        source: problem.source,
        rule_id: problem.rule_id,
        message: problem.message,
    }
}

fn emit_report(report: &ValidateConfigReport, format: ValidateConfigOutputFormat) {
    match format {
        ValidateConfigOutputFormat::Text => print!("{}", render_text_report(report)),
        ValidateConfigOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(report).expect("validate-config report should serialize")
        ),
    }
}

fn render_text_report(report: &ValidateConfigReport) -> String {
    let mut output = String::new();
    if report.summary.status == "ok" {
        if report.summary.strict {
            output.push_str(&format!(
                "Docpact validate-config: pass in strict mode ({} rule(s)).\n",
                report.summary.rule_count
            ));
            output
                .push_str("Next: run `docpact lint --root .` or `docpact route --paths <path>`.\n");
        } else {
            output.push_str(&format!(
                "Docpact validate-config: config loads ({} rule(s)).\n",
                report.summary.rule_count
            ));
            output.push_str(
                "Next: run `docpact validate-config --strict` for graph and ownership checks.\n",
            );
        }
    } else {
        output.push_str(&format!(
            "Docpact validate-config: failed with {} problem(s).\n",
            report.summary.problem_count
        ));
        output.push_str("Problems:\n");
        for problem in &report.problems {
            match &problem.rule_id {
                Some(rule_id) => output.push_str(&format!(
                    "- [{}] {} (rule `{}`): {}\n",
                    problem.code, problem.source, rule_id, problem.message
                )),
                None => output.push_str(&format!(
                    "- [{}] {}: {}\n",
                    problem.code, problem.source, problem.message
                )),
            }
        }
        output.push_str(
            "Next: fix the config entries above, then rerun `docpact validate-config --strict`.\n",
        );
    }

    if !report.warnings.is_empty() {
        output.push_str("Warnings:\n");
        for warning in &report.warnings {
            output.push_str(&format!("- {}: {}\n", warning.code, warning.message));
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{VALIDATE_CONFIG_SCHEMA_VERSION, execute, render_text_report, run};
    use crate::AppExit;
    use crate::cli::{ValidateConfigArgs, ValidateConfigOutputFormat};
    use crate::config::{CONFIG_FILE, DOC_ROOT_DIR};

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{nanos}-{}", std::process::id()));
        fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }

    fn git(root: &std::path::Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("git command should run");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("git stdout should be utf-8")
            .trim()
            .to_string()
    }

    fn init_git_repo(root: &std::path::Path) {
        fs::create_dir_all(root).expect("repo root should exist");
        git(root, &["init"]);
        git(root, &["config", "user.name", "Docpact Tests"]);
        git(root, &["config", "user.email", "docpact@example.com"]);
    }

    #[test]
    fn strict_validate_config_returns_lint_failure_for_invalid_rules() {
        let root = temp_dir("docpact-validate-config");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("doc root should exist");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: repo
lastReviewedAt: "2026-04-21"
lastReviewedCommit: "abc"
repo:
  id: example
rules:
  - id: duplicate-rule
    scope: repo
    repo: example
    triggers:
      - path: src/***
        kind: code
    requiredDocs:
      - path: docs/*.md
        mode: invalid_mode
    reason: example
  - id: duplicate-rule
    scope: repo
    repo: example
    triggers: []
    requiredDocs: []
    reason: second
"#,
        )
        .expect("config should be written");

        let exit = run(ValidateConfigArgs {
            root: Some(root),
            config: None,
            strict: true,
            format: ValidateConfigOutputFormat::Text,
        })
        .expect("strict validation should execute");

        assert_eq!(exit, AppExit::LintFailure);
    }

    #[test]
    fn non_strict_validate_config_remains_compatible() {
        let root = temp_dir("docpact-validate-config-compat");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("doc root should exist");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: repo
lastReviewedAt: "2026-04-21"
lastReviewedCommit: "abc"
repo:
  id: example
rules:
  - id: compatibility-check
    scope: repo
    repo: example
    triggers:
      - path: src/***
        kind: code
    requiredDocs:
      - path: docs/*.md
        mode: invalid_mode
    reason: example
"#,
        )
        .expect("config should be written");

        let exit = run(ValidateConfigArgs {
            root: Some(root),
            config: None,
            strict: false,
            format: ValidateConfigOutputFormat::Text,
        })
        .expect("non-strict validation should execute");

        assert_eq!(exit, AppExit::Success);
    }

    #[test]
    fn strict_validate_config_fails_for_tracked_path_ownership_conflicts() {
        let root = temp_dir("docpact-validate-ownership-conflict");
        init_git_repo(&root);
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("doc root should exist");
        fs::create_dir_all(root.join("src/conflict")).expect("src dir should exist");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: repo
catalog:
  repos:
    - id: app
      path: .
    - id: edge
      path: edge
ownership:
  domains:
    - id: app-domain
      paths:
        include:
          - src/**
      ownerRepo: app
    - id: edge-domain
      paths:
        include:
          - src/conflict/**
      ownerRepo: edge
rules: []
"#,
        )
        .expect("config should be written");
        fs::write(root.join("src/conflict/index.ts"), "export const x = 1;\n")
            .expect("tracked file should be written");
        git(&root, &["add", "."]);
        git(&root, &["commit", "-m", "Add conflict sample"]);

        let exit = run(ValidateConfigArgs {
            root: Some(root),
            config: None,
            strict: true,
            format: ValidateConfigOutputFormat::Text,
        })
        .expect("strict validation should execute");

        assert_eq!(exit, AppExit::LintFailure);
    }

    #[test]
    fn strict_validate_config_returns_json_ready_report() {
        let root = temp_dir("docpact-validate-config-json");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("doc root should exist");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: repo
rules:
  - id: invalid
    scope: repo
    repo: demo
    triggers:
      - path: src/***
        kind: code
    requiredDocs:
      - path: docs/*.md
        mode: invalid_mode
    reason: example
"#,
        )
        .expect("config should be written");

        let report = execute(&ValidateConfigArgs {
            root: Some(root),
            config: None,
            strict: true,
            format: ValidateConfigOutputFormat::Json,
        })
        .expect("validation should execute");
        let rendered = render_text_report(&report);

        assert_eq!(report.schema_version, VALIDATE_CONFIG_SCHEMA_VERSION);
        assert_eq!(report.command, "validate-config");
        assert_eq!(report.summary.status, "fail");
        assert_eq!(report.problems[0].code, "invalid-config");
        assert!(rendered.contains("Docpact validate-config: failed"));
        assert!(rendered.contains("Next: fix the config entries above"));
    }
}
