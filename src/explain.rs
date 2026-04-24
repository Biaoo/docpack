use miette::Result;
use serde::Serialize;

use crate::AppExit;
use crate::cli::{ExplainArgs, ExplainOutputFormat};
use crate::config::{load_impact_files, normalize_path, resolve_rule_path, root_dir_from_option};
use crate::rules::{ExpectedDoc, RequiredDocMode, collect_expected_docs, match_rules};

pub const EXPLAIN_SCHEMA_VERSION: &str = "docpact.explain.v1";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExplainReport {
    pub schema_version: String,
    pub tool_name: String,
    pub tool_version: String,
    pub command: String,
    pub summary: ExplainSummary,
    pub warnings: Vec<ExplainWarning>,
    pub path: String,
    pub matched_rules: Vec<ExplainRuleMatch>,
    pub expected_docs: Vec<ExplainExpectedDoc>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExplainSummary {
    pub matched_rule_count: usize,
    pub expected_doc_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExplainWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExplainRuleMatch {
    pub rule_id: String,
    pub source: String,
    pub config_source: String,
    pub triggers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExplainExpectedDoc {
    pub path: String,
    pub action: String,
    pub modes: Vec<String>,
    pub rules: Vec<String>,
    pub changed_paths: Vec<String>,
}

pub fn run(args: ExplainArgs) -> Result<AppExit> {
    let format = args.format;
    let report = execute(&args)?;
    emit_report(&report, format);
    Ok(AppExit::Success)
}

pub fn execute(args: &ExplainArgs) -> Result<ExplainReport> {
    let root_dir = root_dir_from_option(args.root.as_deref())?;
    let loaded_rules = load_impact_files(&root_dir, args.config.as_deref())?;
    let path = normalize_path(&args.path.to_string_lossy());
    let matches = match_rules(std::slice::from_ref(&path), &loaded_rules);
    let expected = collect_expected_docs(&matches);

    let matched_rules = matches
        .iter()
        .map(|matched| ExplainRuleMatch {
            rule_id: matched.rule.id.clone(),
            source: matched.source.clone(),
            config_source: matched.config_source.clone(),
            triggers: matched
                .rule
                .triggers
                .iter()
                .map(|trigger| resolve_rule_path(&matched.base_dir, &trigger.path))
                .collect(),
        })
        .collect::<Vec<_>>();
    let expected_docs = expected
        .values()
        .map(expected_doc_summary)
        .collect::<Vec<_>>();
    let warnings = if matched_rules.is_empty() {
        vec![ExplainWarning {
            code: "no-rule-matches".into(),
            message:
                "path did not match any effective rule trigger; route will not produce governed docs from rules"
                    .into(),
        }]
    } else {
        Vec::new()
    };

    Ok(ExplainReport {
        schema_version: EXPLAIN_SCHEMA_VERSION.into(),
        tool_name: env!("CARGO_PKG_NAME").into(),
        tool_version: env!("CARGO_PKG_VERSION").into(),
        command: "explain".into(),
        summary: ExplainSummary {
            matched_rule_count: matched_rules.len(),
            expected_doc_count: expected_docs.len(),
        },
        warnings,
        path,
        matched_rules,
        expected_docs,
    })
}

fn emit_report(report: &ExplainReport, format: ExplainOutputFormat) {
    match format {
        ExplainOutputFormat::Text => print!("{}", render_text_report(report)),
        ExplainOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(report).expect("explain report should serialize")
        ),
    }
}

fn render_text_report(report: &ExplainReport) -> String {
    let mut output = String::new();
    if report.summary.matched_rule_count == 0 {
        output.push_str(&format!(
            "Docpact explain: no matching rules for {}.\n",
            report.path
        ));
        output.push_str("Warnings:\n");
        for warning in &report.warnings {
            output.push_str(&format!("- {}: {}\n", warning.code, warning.message));
        }
        output.push_str(&format!(
            "Next: run `docpact route --paths {}` for advisory navigation, or add a matching rule if this path should be governed.\n",
            report.path
        ));
        return output;
    }

    output.push_str(&format!(
        "Docpact explain: {} matches {} rule(s) and expects {} doc(s).\n",
        report.path, report.summary.matched_rule_count, report.summary.expected_doc_count
    ));
    output.push_str("Rules:\n");
    for matched in &report.matched_rules {
        output.push_str(&format!(
            "- {} from {} (triggers: {})\n",
            matched.rule_id,
            matched.source,
            matched.triggers.join(",")
        ));
    }
    output.push_str("Expected docs:\n");
    if report.expected_docs.is_empty() {
        output.push_str("- none\n");
    } else {
        for expected in &report.expected_docs {
            output.push_str(&format!(
                "- {} {} (modes: {}; rules: {})\n",
                expected.action,
                expected.path,
                expected.modes.join(","),
                expected.rules.join(",")
            ));
        }
    }
    output.push_str(&format!(
        "Next: run `docpact route --paths {}` for prioritized docs and workspace context.\n",
        report.path
    ));
    output
}

fn expected_doc_summary(expected: &ExpectedDoc) -> ExplainExpectedDoc {
    let modes = expected.modes.iter().copied().collect::<Vec<_>>();
    ExplainExpectedDoc {
        path: expected.path.clone(),
        action: action_for_modes(&modes).into(),
        modes: modes.iter().map(ToString::to_string).collect(),
        rules: expected.rules.iter().cloned().collect(),
        changed_paths: expected.changed_paths.iter().cloned().collect(),
    }
}

fn action_for_modes(modes: &[RequiredDocMode]) -> &'static str {
    if modes.contains(&RequiredDocMode::BodyUpdateRequired) {
        "update body for"
    } else if modes.contains(&RequiredDocMode::MetadataRefreshRequired) {
        "refresh metadata for"
    } else if modes.contains(&RequiredDocMode::MustExist) {
        "ensure exists"
    } else {
        "review or update"
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{EXPLAIN_SCHEMA_VERSION, execute, render_text_report};
    use crate::cli::{ExplainArgs, ExplainOutputFormat};
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

    fn base_args(root: PathBuf, path: &str) -> ExplainArgs {
        ExplainArgs {
            path: PathBuf::from(path),
            root: Some(root),
            config: None,
            format: ExplainOutputFormat::Text,
        }
    }

    #[test]
    fn explain_returns_json_ready_report() {
        let root = temp_dir("docpact-explain");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("doc root should exist");
        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: repo
rules:
  - id: api-docs
    scope: repo
    repo: demo
    triggers:
      - path: src/api/**
        kind: code
    requiredDocs:
      - path: docs/api.md
        mode: body_update_required
    reason: API changes require docs
"#,
        )
        .expect("config should be written");

        let report = execute(&base_args(root, "src/api/client.ts")).expect("explain report");

        assert_eq!(report.schema_version, EXPLAIN_SCHEMA_VERSION);
        assert_eq!(report.command, "explain");
        assert_eq!(report.summary.matched_rule_count, 1);
        assert_eq!(report.expected_docs[0].action, "update body for");
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn explain_warns_when_no_rule_matches() {
        let root = temp_dir("docpact-explain-empty");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("doc root should exist");
        fs::write(
            root.join(CONFIG_FILE),
            "version: 1\nlayout: repo\nrules: []\n",
        )
        .expect("config should be written");

        let report = execute(&base_args(root, "src/unknown.ts")).expect("explain report");
        let rendered = render_text_report(&report);

        assert_eq!(report.summary.matched_rule_count, 0);
        assert_eq!(report.warnings[0].code, "no-rule-matches");
        assert!(rendered.contains("Docpact explain: no matching rules"));
        assert!(rendered.contains("Next: run `docpact route --paths src/unknown.ts`"));
    }
}
