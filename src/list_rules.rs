use miette::Result;
use serde::Serialize;

use crate::AppExit;
use crate::cli::{ListRulesArgs, ListRulesOutputFormat};
use crate::config::{load_impact_files, resolve_rule_path, root_dir_from_option};
use crate::reporters::OutputWarning;
use crate::rules::RequiredDocMode;

pub const LIST_RULES_SCHEMA_VERSION: &str = "docpact.list-rules.v1";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ListRulesReport {
    pub schema_version: String,
    pub tool_name: String,
    pub tool_version: String,
    pub command: String,
    pub warnings: Vec<OutputWarning>,
    pub rule_count: usize,
    pub rules: Vec<ListedRule>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ListedRule {
    pub id: String,
    pub scope: String,
    pub repo: String,
    pub description: String,
    pub rule_source: String,
    pub config_source: String,
    pub provenance_kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_profile: Option<String>,
    pub base_dir: String,
    pub trigger_count: usize,
    pub required_doc_count: usize,
    pub triggers: Vec<ListedTrigger>,
    pub required_docs: Vec<ListedRequiredDoc>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ListedTrigger {
    pub original_path: String,
    pub path: String,
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ListedRequiredDoc {
    pub original_path: String,
    pub path: String,
    pub mode: String,
}

pub fn run(args: ListRulesArgs) -> Result<AppExit> {
    let report = execute(&args)?;
    emit_report(&report, args.format);
    Ok(AppExit::Success)
}

pub fn execute(args: &ListRulesArgs) -> Result<ListRulesReport> {
    let root_dir = root_dir_from_option(args.root.as_deref())?;
    let loaded_rules = load_impact_files(&root_dir, args.config.as_deref())?;
    let rules = loaded_rules
        .into_iter()
        .map(|loaded| {
            let triggers = loaded
                .rule
                .triggers
                .iter()
                .map(|trigger| ListedTrigger {
                    original_path: trigger.path.clone(),
                    path: resolve_rule_path(&loaded.base_dir, &trigger.path),
                    kind: trigger.kind.clone(),
                })
                .collect::<Vec<_>>();
            let required_docs = loaded
                .rule
                .required_docs
                .iter()
                .map(|doc| ListedRequiredDoc {
                    original_path: doc.path.clone(),
                    path: resolve_rule_path(&loaded.base_dir, &doc.path),
                    mode: RequiredDocMode::from_option(doc.mode.as_deref())
                        .as_str()
                        .to_string(),
                })
                .collect::<Vec<_>>();

            ListedRule {
                id: loaded.rule.id,
                scope: loaded.rule.scope,
                repo: loaded.rule.repo,
                description: loaded.rule.reason,
                rule_source: loaded.source,
                config_source: loaded.config_source,
                provenance_kind: loaded.provenance.origin_kind.as_str().into(),
                workspace_profile: loaded.provenance.workspace_profile,
                base_dir: loaded.base_dir,
                trigger_count: triggers.len(),
                required_doc_count: required_docs.len(),
                triggers,
                required_docs,
            }
        })
        .collect::<Vec<_>>();

    Ok(ListRulesReport {
        schema_version: LIST_RULES_SCHEMA_VERSION.into(),
        tool_name: env!("CARGO_PKG_NAME").into(),
        tool_version: env!("CARGO_PKG_VERSION").into(),
        command: "list-rules".into(),
        warnings: Vec::new(),
        rule_count: rules.len(),
        rules,
    })
}

fn emit_report(report: &ListRulesReport, format: ListRulesOutputFormat) {
    match format {
        ListRulesOutputFormat::Text => emit_text_report(report),
        ListRulesOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(report).expect("list-rules report should serialize")
        ),
    }
}

fn emit_text_report(report: &ListRulesReport) {
    println!(
        "Docpact list-rules: {} effective rule(s).",
        report.rule_count
    );

    if report.rules.is_empty() {
        println!("Rules:");
        println!("- none");
        println!(
            "Next: add rules to `.docpact/config.yaml`, or run `docpact doctor --root .` for onboarding diagnostics."
        );
        return;
    }

    println!("Rules:");
    for rule in &report.rules {
        println!(
            "- {}: {} trigger(s) -> {} required doc(s) (source: {}, origin: {}, scope: {}, repo: {})",
            rule.id,
            rule.trigger_count,
            rule.required_doc_count,
            rule.rule_source,
            rule.provenance_kind,
            rule.scope,
            rule.repo,
        );
        println!("  description: {}", rule.description);
        if let Some(workspace_profile) = &rule.workspace_profile {
            println!("  workspace profile: {workspace_profile}");
        }
        if rule.triggers.is_empty() {
            println!("  triggers: none");
        } else {
            for trigger in &rule.triggers {
                match &trigger.kind {
                    Some(kind) => println!(
                        "  trigger: {} (from {}; kind: {})",
                        trigger.path, trigger.original_path, kind
                    ),
                    None => println!(
                        "  trigger: {} (from {})",
                        trigger.path, trigger.original_path
                    ),
                }
            }
        }

        if rule.required_docs.is_empty() {
            println!("  required docs: none");
        } else {
            for required_doc in &rule.required_docs {
                println!(
                    "  expects: {} (from {}; mode: {})",
                    required_doc.path, required_doc.original_path, required_doc.mode
                );
            }
        }
    }
    println!(
        "Next: use `docpact explain <path> --root .` to inspect one path against these rules."
    );
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{LIST_RULES_SCHEMA_VERSION, execute};
    use crate::cli::{ListRulesArgs, ListRulesOutputFormat};

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{nanos}-{}", std::process::id()));
        fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }

    fn base_args(root: &std::path::Path) -> ListRulesArgs {
        ListRulesArgs {
            root: Some(root.to_path_buf()),
            config: None,
            format: ListRulesOutputFormat::Json,
        }
    }

    #[test]
    fn list_rules_reports_repo_rules_with_resolved_paths() {
        let root = temp_dir("docpact-list-rules-repo");
        fs::create_dir_all(root.join(".docpact")).expect("doc root should exist");
        fs::write(
            root.join(".docpact/config.yaml"),
            r#"version: 1
layout: repo
rules:
  - id: api-docs
    scope: repo
    repo: root
    triggers:
      - path: src/api/**
        kind: code
    requiredDocs:
      - path: docs/api.md
        mode: metadata_refresh_required
    reason: Keep API docs aligned.
"#,
        )
        .expect("config should be written");

        let report = execute(&base_args(&root)).expect("list-rules should execute");

        assert_eq!(report.schema_version, LIST_RULES_SCHEMA_VERSION);
        assert_eq!(report.command, "list-rules");
        assert!(report.warnings.is_empty());
        assert_eq!(report.rule_count, 1);
        let rule = &report.rules[0];
        assert_eq!(rule.id, "api-docs");
        assert_eq!(rule.description, "Keep API docs aligned.");
        assert_eq!(rule.rule_source, ".docpact/config.yaml");
        assert_eq!(rule.config_source, ".docpact/config.yaml");
        assert_eq!(rule.provenance_kind, "root-local");
        assert_eq!(rule.workspace_profile, None);
        assert_eq!(rule.triggers[0].path, "src/api/**");
        assert_eq!(rule.triggers[0].original_path, "src/api/**");
        assert_eq!(rule.required_docs[0].path, "docs/api.md");
        assert_eq!(rule.required_docs[0].mode, "metadata_refresh_required");
    }

    #[test]
    fn list_rules_reports_workspace_rules_with_repo_relative_paths() {
        let root = temp_dir("docpact-list-rules-workspace");
        fs::create_dir_all(root.join(".docpact")).expect("doc root should exist");
        fs::create_dir_all(root.join("service/.docpact")).expect("workspace doc root should exist");
        fs::write(
            root.join(".docpact/config.yaml"),
            r#"version: 1
layout: workspace
workspace:
  name: demo
  profiles:
    default:
      rules:
        - id: service-docs
          scope: workspace
          repo: workspace
          triggers:
            - path: src/**
              kind: code
          requiredDocs:
            - path: docs/service.md
          reason: Keep service docs aligned.
rules:
  - id: root-only
    scope: workspace
    repo: workspace
    triggers:
      - path: AGENTS.md
        kind: doc
    requiredDocs:
      - path: .docpact/config.yaml
    reason: Root rule.
"#,
        )
        .expect("root config should be written");
        fs::write(
            root.join("service/.docpact/config.yaml"),
            r#"version: 1
layout: repo
inherit:
  workspace_profile: default
overrides:
  rules:
    replace:
      - id: service-docs
        scope: repo
        repo: service
        triggers:
          - path: src/**
            kind: code
        requiredDocs:
          - path: docs/service.md
        reason: Keep service docs aligned.
"#,
        )
        .expect("workspace config should be written");

        let report = execute(&base_args(&root)).expect("list-rules should execute");

        assert_eq!(report.rule_count, 2);
        let inherited_rule = report
            .rules
            .iter()
            .find(|rule| rule.id == "service-docs")
            .expect("inherited rule should exist");
        assert_eq!(
            inherited_rule.rule_source,
            "service/.docpact/config.yaml#overrides.rules.replace.service-docs"
        );
        assert_eq!(inherited_rule.config_source, "service/.docpact/config.yaml");
        assert_eq!(inherited_rule.provenance_kind, "override-replace");
        assert_eq!(inherited_rule.workspace_profile.as_deref(), Some("default"));
        assert_eq!(inherited_rule.base_dir, "service");
        assert_eq!(inherited_rule.triggers[0].path, "service/src/**");
        assert_eq!(
            inherited_rule.required_docs[0].path,
            "service/docs/service.md"
        );
        assert_eq!(inherited_rule.required_docs[0].mode, "review_or_update");

        let root_rule = report
            .rules
            .iter()
            .find(|rule| rule.id == "root-only")
            .expect("root rule should exist");
        assert_eq!(root_rule.rule_source, ".docpact/config.yaml");
        assert_eq!(root_rule.provenance_kind, "root-local");
    }
}
