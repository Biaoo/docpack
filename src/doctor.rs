use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use miette::Result;
use serde::Serialize;
use yaml_serde::Value;

use crate::AppExit;
use crate::cli::{DoctorArgs, DoctorOutputFormat};
use crate::config::{
    CONFIG_FILE, ImpactLayout, detect_impact_layout, list_impact_files, load_coverage_configs,
    load_doc_inventory_configs, load_freshness_configs, load_impact_files, load_yaml_value,
    normalize_path, path_relative_to, resolve_rule_path, root_dir_from_option,
};

pub const DOCTOR_SCHEMA_VERSION: &str = "docpact.doctor.v1";

const CODE_MISSING_CONFIG: &str = "missing-config";
const CODE_CONFIG_LOAD_FAILED: &str = "config-load-failed";
const CODE_EMPTY_RULE_GRAPH: &str = "empty-rule-graph";
const CODE_MISSING_COVERAGE_SCOPE: &str = "missing-coverage-scope";
const CODE_MISSING_GOVERNED_DOCS: &str = "missing-governed-docs";
const CODE_MISSING_DOC_INVENTORY: &str = "missing-doc-inventory";
const CODE_MISSING_FRESHNESS_CONFIG: &str = "missing-freshness-config";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DoctorReport {
    pub schema_version: String,
    pub tool_name: String,
    pub tool_version: String,
    pub summary: DoctorSummary,
    pub findings: Vec<DoctorFinding>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DoctorSummary {
    pub config_present: bool,
    pub layout: String,
    pub rule_count: usize,
    pub coverage_configured: bool,
    pub doc_inventory_configured: bool,
    pub freshness_configured: bool,
    pub governed_doc_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DoctorFinding {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DoctorSeverity {
    Warn,
    Error,
}

impl DoctorSeverity {
    fn as_str(self) -> &'static str {
        match self {
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigBlock {
    Coverage,
    DocInventory,
}

pub fn run(args: DoctorArgs) -> Result<AppExit> {
    let report = execute(&args)?;
    emit_report(&report, args.format);
    Ok(AppExit::Success)
}

pub fn execute(args: &DoctorArgs) -> Result<DoctorReport> {
    let root_dir = root_dir_from_option(args.root.as_deref())?;
    let config_path = resolve_config_path(&root_dir, args.config.as_deref());
    let config_source = display_config_source(&root_dir, &config_path);

    if !config_path.exists() {
        return Ok(report_with_findings(
            DoctorSummary {
                config_present: false,
                layout: "none".into(),
                rule_count: 0,
                coverage_configured: false,
                doc_inventory_configured: false,
                freshness_configured: false,
                governed_doc_count: 0,
            },
            vec![DoctorFinding {
                code: CODE_MISSING_CONFIG.into(),
                severity: DoctorSeverity::Error.as_str().into(),
                message: format!(
                    "No docpact config was found at `{}`. Create a config before onboarding the repository.",
                    config_source
                ),
                source: config_source,
            }],
        ));
    }

    let layout = match detect_impact_layout(&root_dir, args.config.as_deref()) {
        Ok(layout) => layout,
        Err(error) => {
            let message = error.to_string();
            let source = extract_config_source(&message).unwrap_or_else(|| config_source.clone());
            return Ok(report_with_findings(
                DoctorSummary {
                    config_present: true,
                    layout: "unknown".into(),
                    rule_count: 0,
                    coverage_configured: false,
                    doc_inventory_configured: false,
                    freshness_configured: false,
                    governed_doc_count: 0,
                },
                vec![DoctorFinding {
                    code: CODE_CONFIG_LOAD_FAILED.into(),
                    severity: DoctorSeverity::Error.as_str().into(),
                    message,
                    source,
                }],
            ));
        }
    };

    let impact_files = match list_impact_files(&root_dir, args.config.as_deref()) {
        Ok(files) => files,
        Err(error) => {
            let message = error.to_string();
            let source = extract_config_source(&message).unwrap_or_else(|| config_source.clone());
            return Ok(report_with_findings(
                DoctorSummary {
                    config_present: true,
                    layout: layout_label(layout).into(),
                    rule_count: 0,
                    coverage_configured: false,
                    doc_inventory_configured: false,
                    freshness_configured: false,
                    governed_doc_count: 0,
                },
                vec![DoctorFinding {
                    code: CODE_CONFIG_LOAD_FAILED.into(),
                    severity: DoctorSeverity::Error.as_str().into(),
                    message,
                    source,
                }],
            ));
        }
    };

    let loaded_rules = match load_impact_files(&root_dir, args.config.as_deref()) {
        Ok(rules) => rules,
        Err(error) => {
            let message = error.to_string();
            let source = extract_config_source(&message).unwrap_or_else(|| config_source.clone());
            return Ok(report_with_findings(
                DoctorSummary {
                    config_present: true,
                    layout: layout_label(layout).into(),
                    rule_count: 0,
                    coverage_configured: false,
                    doc_inventory_configured: false,
                    freshness_configured: false,
                    governed_doc_count: 0,
                },
                vec![DoctorFinding {
                    code: CODE_CONFIG_LOAD_FAILED.into(),
                    severity: DoctorSeverity::Error.as_str().into(),
                    message,
                    source,
                }],
            ));
        }
    };

    let coverage_configured = has_effective_coverage_scope(&root_dir, &impact_files)?;
    let doc_inventory_configured = has_effective_doc_inventory_scope(&root_dir, &impact_files)?;
    let freshness_configured = has_explicit_freshness_config(&root_dir, &impact_files)?;

    let _ = load_coverage_configs(&root_dir, args.config.as_deref())?;
    let _ = load_doc_inventory_configs(&root_dir, args.config.as_deref())?;
    let _ = load_freshness_configs(&root_dir, args.config.as_deref())?;

    let governed_doc_count = loaded_rules
        .iter()
        .flat_map(|loaded| {
            loaded
                .rule
                .required_docs
                .iter()
                .map(|doc| resolve_rule_path(&loaded.base_dir, &doc.path))
        })
        .collect::<BTreeSet<_>>()
        .len();

    let summary = DoctorSummary {
        config_present: true,
        layout: layout_label(layout).into(),
        rule_count: loaded_rules.len(),
        coverage_configured,
        doc_inventory_configured,
        freshness_configured,
        governed_doc_count,
    };

    let mut findings = Vec::new();
    if summary.rule_count == 0 {
        findings.push(DoctorFinding {
            code: CODE_EMPTY_RULE_GRAPH.into(),
            severity: DoctorSeverity::Error.as_str().into(),
            message:
                "The loaded config graph contains zero rules. Add at least one rule before relying on docpact enforcement."
                    .into(),
            source: config_source.clone(),
        });
    } else {
        if summary.governed_doc_count == 0 {
            findings.push(DoctorFinding {
                code: CODE_MISSING_GOVERNED_DOCS.into(),
                severity: DoctorSeverity::Error.as_str().into(),
                message:
                    "The loaded rules do not resolve to any governed required docs. Add `requiredDocs` targets before using lint as a governance signal."
                        .into(),
                source: config_source.clone(),
            });
        }

        if !summary.coverage_configured {
            findings.push(DoctorFinding {
                code: CODE_MISSING_COVERAGE_SCOPE.into(),
                severity: DoctorSeverity::Warn.as_str().into(),
                message:
                    "No coverage scope is configured; diff coverage defaults to all changed paths. Configure `coverage.include` or `coverage.exclude` if the intended governance scope is narrower."
                        .into(),
                source: config_source.clone(),
            });
        }

        if !summary.doc_inventory_configured {
            findings.push(DoctorFinding {
                code: CODE_MISSING_DOC_INVENTORY.into(),
                severity: DoctorSeverity::Warn.as_str().into(),
                message:
                    "No explicit doc inventory scope is configured; repository coverage audit will infer inventory from all tracked Markdown/YAML docs."
                        .into(),
                source: config_source.clone(),
            });
        }

        if !summary.freshness_configured {
            findings.push(DoctorFinding {
                code: CODE_MISSING_FRESHNESS_CONFIG.into(),
                severity: DoctorSeverity::Warn.as_str().into(),
                message:
                    "No explicit freshness config is present; repository freshness audit falls back to default thresholds."
                        .into(),
                source: config_source,
            });
        }
    }

    findings.sort_by(|left, right| {
        (&left.severity, &left.code, &left.source, &left.message).cmp(&(
            &right.severity,
            &right.code,
            &right.source,
            &right.message,
        ))
    });

    Ok(report_with_findings(summary, findings))
}

fn report_with_findings(summary: DoctorSummary, findings: Vec<DoctorFinding>) -> DoctorReport {
    DoctorReport {
        schema_version: DOCTOR_SCHEMA_VERSION.into(),
        tool_name: env!("CARGO_PKG_NAME").into(),
        tool_version: env!("CARGO_PKG_VERSION").into(),
        summary,
        findings,
    }
}

fn emit_report(report: &DoctorReport, format: DoctorOutputFormat) {
    match format {
        DoctorOutputFormat::Text => emit_text_report(report),
        DoctorOutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(report).expect("doctor report should serialize")
        ),
    }
}

fn emit_text_report(report: &DoctorReport) {
    println!("Docpact doctor:");
    println!(
        "Summary: config_present={}, layout={}, rule_count={}, coverage_configured={}, doc_inventory_configured={}, freshness_configured={}, governed_doc_count={}",
        report.summary.config_present,
        report.summary.layout,
        report.summary.rule_count,
        report.summary.coverage_configured,
        report.summary.doc_inventory_configured,
        report.summary.freshness_configured,
        report.summary.governed_doc_count,
    );
    println!("Findings:");
    if report.findings.is_empty() {
        println!("- none");
        return;
    }

    for finding in &report.findings {
        println!(
            "- [{}] {} {}: {}",
            finding.severity, finding.code, finding.source, finding.message
        );
    }
}

fn resolve_config_path(root_dir: &Path, config_override: Option<&Path>) -> PathBuf {
    match config_override {
        Some(path) => path.to_path_buf(),
        None => root_dir.join(CONFIG_FILE),
    }
}

fn display_config_source(root_dir: &Path, path: &Path) -> String {
    if path.is_absolute() {
        path_relative_to(root_dir, path)
    } else {
        normalize_path(&path.to_string_lossy())
    }
}

fn layout_label(layout: ImpactLayout) -> &'static str {
    match layout {
        ImpactLayout::Repo => "repo",
        ImpactLayout::Workspace => "workspace",
        ImpactLayout::None => "none",
    }
}

fn extract_config_source(message: &str) -> Option<String> {
    for marker in [
        " is not a valid docpact config file.",
        " is not valid YAML for docpact.",
    ] {
        if let Some(index) = message.find(marker) {
            return Some(message[..index].to_string());
        }
    }
    None
}

fn has_effective_coverage_scope(
    root_dir: &Path,
    impact_files: &[crate::config::ImpactFileDescriptor],
) -> Result<bool> {
    has_effective_block(root_dir, impact_files, ConfigBlock::Coverage)
}

fn has_effective_doc_inventory_scope(
    root_dir: &Path,
    impact_files: &[crate::config::ImpactFileDescriptor],
) -> Result<bool> {
    has_effective_block(root_dir, impact_files, ConfigBlock::DocInventory)
}

fn has_explicit_freshness_config(
    root_dir: &Path,
    impact_files: &[crate::config::ImpactFileDescriptor],
) -> Result<bool> {
    for descriptor in impact_files {
        let value = load_yaml_value(&descriptor.abs_path, &descriptor.rel_path)?;
        if top_level_mapping_has_key(&value, "freshness") {
            return Ok(true);
        }
    }

    let _ = root_dir;
    Ok(false)
}

fn has_effective_block(
    root_dir: &Path,
    impact_files: &[crate::config::ImpactFileDescriptor],
    block: ConfigBlock,
) -> Result<bool> {
    for descriptor in impact_files {
        let value = load_yaml_value(&descriptor.abs_path, &descriptor.rel_path)?;
        if block_has_non_empty_patterns(&value, block) {
            return Ok(true);
        }
    }

    let _ = root_dir;
    Ok(false)
}

fn top_level_mapping_has_key(value: &Value, key: &str) -> bool {
    let Value::Mapping(mapping) = value else {
        return false;
    };
    mapping.contains_key(Value::String(key.to_string()))
}

fn block_has_non_empty_patterns(value: &Value, block: ConfigBlock) -> bool {
    let block_name = match block {
        ConfigBlock::Coverage => "coverage",
        ConfigBlock::DocInventory => "docInventory",
    };
    let Value::Mapping(root_mapping) = value else {
        return false;
    };
    let Some(Value::Mapping(block_mapping)) = root_mapping.get(Value::String(block_name.into()))
    else {
        return false;
    };

    for key in ["include", "exclude"] {
        let Some(Value::Sequence(sequence)) = block_mapping.get(Value::String(key.into())) else {
            continue;
        };
        if !sequence.is_empty() {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        CODE_CONFIG_LOAD_FAILED, CODE_EMPTY_RULE_GRAPH, CODE_MISSING_CONFIG,
        CODE_MISSING_COVERAGE_SCOPE, CODE_MISSING_DOC_INVENTORY, CODE_MISSING_FRESHNESS_CONFIG,
        CODE_MISSING_GOVERNED_DOCS, DOCTOR_SCHEMA_VERSION, execute,
    };
    use crate::cli::{DoctorArgs, DoctorOutputFormat};
    use crate::config::CONFIG_FILE;

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{nanos}-{}", std::process::id()));
        fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }

    fn base_args(root: &std::path::Path) -> DoctorArgs {
        DoctorArgs {
            root: Some(root.to_path_buf()),
            config: None,
            format: DoctorOutputFormat::Json,
        }
    }

    #[test]
    fn doctor_reports_missing_config() {
        let root = temp_dir("docpact-doctor-missing");

        let report = execute(&base_args(&root)).expect("doctor should execute");

        assert_eq!(report.schema_version, DOCTOR_SCHEMA_VERSION);
        assert!(!report.summary.config_present);
        assert_eq!(report.summary.layout, "none");
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].code, CODE_MISSING_CONFIG);
    }

    #[test]
    fn doctor_reports_empty_rule_graph() {
        let root = temp_dir("docpact-doctor-empty");
        fs::create_dir_all(root.join(".docpact")).expect("doc root should exist");
        fs::write(
            root.join(CONFIG_FILE),
            r#"version: 1
layout: repo
rules: []
"#,
        )
        .expect("config should be written");

        let report = execute(&base_args(&root)).expect("doctor should execute");

        assert_eq!(report.summary.config_present, true);
        assert_eq!(report.summary.rule_count, 0);
        assert_eq!(report.findings[0].code, CODE_EMPTY_RULE_GRAPH);
    }

    #[test]
    fn doctor_reports_governance_gaps_without_repeating_strict_validation() {
        let root = temp_dir("docpact-doctor-gaps");
        fs::create_dir_all(root.join(".docpact")).expect("doc root should exist");
        fs::write(
            root.join(CONFIG_FILE),
            r#"version: 1
layout: repo
rules:
  - id: no-governed-docs
    scope: repo
    repo: sample
    triggers:
      - path: src/**
        kind: code
    requiredDocs: []
    reason: sample
"#,
        )
        .expect("config should be written");

        let report = execute(&base_args(&root)).expect("doctor should execute");

        assert_eq!(report.summary.rule_count, 1);
        let codes = report
            .findings
            .iter()
            .map(|finding| finding.code.as_str())
            .collect::<Vec<_>>();
        assert!(codes.contains(&CODE_MISSING_GOVERNED_DOCS));
        assert!(codes.contains(&CODE_MISSING_COVERAGE_SCOPE));
        assert!(codes.contains(&CODE_MISSING_DOC_INVENTORY));
        assert!(codes.contains(&CODE_MISSING_FRESHNESS_CONFIG));
        assert!(!codes.contains(&"invalid-config"));
    }

    #[test]
    fn doctor_reports_explicit_config_load_failures() {
        let root = temp_dir("docpact-doctor-invalid");
        fs::create_dir_all(root.join(".docpact")).expect("doc root should exist");
        fs::write(root.join(CONFIG_FILE), "layout: [").expect("config should be written");

        let report = execute(&base_args(&root)).expect("doctor should execute");

        assert_eq!(report.summary.layout, "unknown");
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].code, CODE_CONFIG_LOAD_FAILED);
    }

    #[test]
    fn doctor_marks_explicit_scopes_and_thresholds_as_configured() {
        let root = temp_dir("docpact-doctor-configured");
        fs::create_dir_all(root.join(".docpact")).expect("doc root should exist");
        fs::write(
            root.join(CONFIG_FILE),
            r#"version: 1
layout: repo
coverage:
  include:
    - src/**
docInventory:
  include:
    - docs/**
freshness:
  warn_after_commits: 10
  warn_after_days: 30
  critical_after_days: 60
rules:
  - id: api-docs
    scope: repo
    repo: sample
    triggers:
      - path: src/**
        kind: code
    requiredDocs:
      - path: docs/api.md
    reason: sample
"#,
        )
        .expect("config should be written");

        let report = execute(&base_args(&root)).expect("doctor should execute");

        assert!(report.summary.coverage_configured);
        assert!(report.summary.doc_inventory_configured);
        assert!(report.summary.freshness_configured);
        assert_eq!(report.summary.governed_doc_count, 1);
        assert!(report.findings.is_empty());
    }
}
