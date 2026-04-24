use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use miette::Result;
use serde::Serialize;

use crate::AppExit;
use crate::cli::{DoctorArgs, DoctorOutputFormat};
use crate::config::{
    CONFIG_FILE, ConfigBlockSourceKind, EffectiveConfig, ImpactLayout, analyze_ownership_paths,
    detect_impact_layout, load_catalog_configs, load_effective_configs, load_ownership_configs,
    normalize_path, path_relative_to, resolve_rule_path, root_dir_from_option,
};
use crate::git::get_tracked_paths;
use crate::reporters::OutputWarning;

pub const DOCTOR_SCHEMA_VERSION: &str = "docpact.doctor.v1";

const CODE_MISSING_CONFIG: &str = "missing-config";
const CODE_CONFIG_LOAD_FAILED: &str = "config-load-failed";
const CODE_EMPTY_RULE_GRAPH: &str = "empty-rule-graph";
const CODE_MISSING_COVERAGE_SCOPE: &str = "missing-coverage-scope";
const CODE_MISSING_GOVERNED_DOCS: &str = "missing-governed-docs";
const CODE_MISSING_DOC_INVENTORY: &str = "missing-doc-inventory";
const CODE_MISSING_FRESHNESS_CONFIG: &str = "missing-freshness-config";
const CODE_OWNERSHIP_OVERLAP: &str = "ownership-overlap";
const CODE_OWNERSHIP_CONFLICT: &str = "ownership-conflict";
const CODE_OWNERSHIP_ANALYSIS_UNAVAILABLE: &str = "ownership-analysis-unavailable";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DoctorReport {
    pub schema_version: String,
    pub tool_name: String,
    pub tool_version: String,
    pub command: String,
    pub warnings: Vec<OutputWarning>,
    pub summary: DoctorSummary,
    pub configs: Vec<DoctorConfig>,
    pub findings: Vec<DoctorFinding>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DoctorSummary {
    pub config_present: bool,
    pub layout: String,
    pub effective_config_count: usize,
    pub inherited_config_count: usize,
    pub rule_count: usize,
    pub catalog_repo_count: usize,
    pub ownership_domain_count: usize,
    pub ownership_overlap_count: usize,
    pub ownership_conflict_count: usize,
    pub coverage_configured: bool,
    pub routing_configured: bool,
    pub doc_inventory_configured: bool,
    pub freshness_configured: bool,
    pub routing_intent_count: usize,
    pub governed_doc_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DoctorConfig {
    pub source: String,
    pub base_dir: String,
    pub rule_count: usize,
    pub catalog_repo_count: usize,
    pub ownership_domain_count: usize,
    pub governed_doc_count: usize,
    pub inheritance_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_profile: Option<String>,
    pub coverage_resolution: String,
    pub routing_resolution: String,
    pub doc_inventory_resolution: String,
    pub freshness_resolution: String,
    pub routing_intent_count: usize,
    pub override_add_count: usize,
    pub override_replace_count: usize,
    pub override_disable_count: usize,
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
                effective_config_count: 0,
                inherited_config_count: 0,
                rule_count: 0,
                catalog_repo_count: 0,
                ownership_domain_count: 0,
                ownership_overlap_count: 0,
                ownership_conflict_count: 0,
                coverage_configured: false,
                routing_configured: false,
                doc_inventory_configured: false,
                freshness_configured: false,
                routing_intent_count: 0,
                governed_doc_count: 0,
            },
            Vec::new(),
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
                    effective_config_count: 0,
                    inherited_config_count: 0,
                    rule_count: 0,
                    catalog_repo_count: 0,
                    ownership_domain_count: 0,
                    ownership_overlap_count: 0,
                    ownership_conflict_count: 0,
                    coverage_configured: false,
                    routing_configured: false,
                    doc_inventory_configured: false,
                    freshness_configured: false,
                    routing_intent_count: 0,
                    governed_doc_count: 0,
                },
                Vec::new(),
                vec![DoctorFinding {
                    code: CODE_CONFIG_LOAD_FAILED.into(),
                    severity: DoctorSeverity::Error.as_str().into(),
                    message,
                    source,
                }],
            ));
        }
    };

    let effective_configs = match load_effective_configs(&root_dir, args.config.as_deref()) {
        Ok(configs) => configs,
        Err(error) => {
            let message = error.to_string();
            let source = extract_config_source(&message).unwrap_or_else(|| config_source.clone());
            return Ok(report_with_findings(
                DoctorSummary {
                    config_present: true,
                    layout: layout_label(layout).into(),
                    effective_config_count: 0,
                    inherited_config_count: 0,
                    rule_count: 0,
                    catalog_repo_count: 0,
                    ownership_domain_count: 0,
                    ownership_overlap_count: 0,
                    ownership_conflict_count: 0,
                    coverage_configured: false,
                    routing_configured: false,
                    doc_inventory_configured: false,
                    freshness_configured: false,
                    routing_intent_count: 0,
                    governed_doc_count: 0,
                },
                Vec::new(),
                vec![DoctorFinding {
                    code: CODE_CONFIG_LOAD_FAILED.into(),
                    severity: DoctorSeverity::Error.as_str().into(),
                    message,
                    source,
                }],
            ));
        }
    };

    let catalog_configs = load_catalog_configs(&root_dir, args.config.as_deref())?;
    let ownership_configs = load_ownership_configs(&root_dir, args.config.as_deref())?;
    let configs = build_doctor_configs(&effective_configs, &catalog_configs, &ownership_configs);
    let rule_count = effective_configs
        .iter()
        .map(|config| config.rules.len())
        .sum();
    let catalog_repo_count = catalog_configs
        .iter()
        .map(|config| config.catalog.repos.len())
        .sum();
    let ownership_domain_count = ownership_configs
        .iter()
        .map(|config| config.ownership.domains.len())
        .sum();
    let governed_doc_count = effective_configs
        .iter()
        .flat_map(|config| {
            config.rules.iter().flat_map(|loaded| {
                loaded
                    .rule
                    .required_docs
                    .iter()
                    .map(|doc| resolve_rule_path(&loaded.base_dir, &doc.path))
            })
        })
        .collect::<BTreeSet<_>>()
        .len();
    let coverage_configured = effective_configs.iter().any(|config| {
        !config.coverage.coverage.include.is_empty() || !config.coverage.coverage.exclude.is_empty()
    });
    let doc_inventory_configured = effective_configs.iter().any(|config| {
        !config.doc_inventory.doc_inventory.include.is_empty()
            || !config.doc_inventory.doc_inventory.exclude.is_empty()
    });
    let routing_configured = effective_configs
        .iter()
        .any(|config| !config.routing.routing.intents.is_empty());
    let freshness_configured = effective_configs
        .iter()
        .any(|config| config.freshness.resolution.origin_kind != ConfigBlockSourceKind::Default);
    let routing_intent_count = effective_configs
        .iter()
        .map(|config| config.routing.routing.intents.len())
        .sum();
    let inherited_config_count = effective_configs
        .iter()
        .filter(|config| config.inheritance.is_some())
        .count();
    let ownership_analysis = if ownership_domain_count > 0 {
        match get_tracked_paths(&root_dir) {
            Ok(tracked_paths) => Some(analyze_ownership_paths(&tracked_paths, &ownership_configs)),
            Err(_) => None,
        }
    } else {
        None
    };
    let ownership_overlap_count = ownership_analysis
        .as_ref()
        .map(|analysis| analysis.overlaps.len())
        .unwrap_or(0);
    let ownership_conflict_count = ownership_analysis
        .as_ref()
        .map(|analysis| analysis.conflicts.len())
        .unwrap_or(0);

    let summary = DoctorSummary {
        config_present: true,
        layout: layout_label(layout).into(),
        effective_config_count: effective_configs.len(),
        inherited_config_count,
        rule_count,
        catalog_repo_count,
        ownership_domain_count,
        ownership_overlap_count,
        ownership_conflict_count,
        coverage_configured,
        routing_configured,
        doc_inventory_configured,
        freshness_configured,
        routing_intent_count,
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
                source: config_source.clone(),
            });
        }

        if summary.ownership_domain_count > 0 {
            match ownership_analysis {
                Some(ref analysis) => {
                    if !analysis.conflicts.is_empty() {
                        findings.push(DoctorFinding {
                            code: CODE_OWNERSHIP_CONFLICT.into(),
                            severity: DoctorSeverity::Error.as_str().into(),
                            message: format!(
                                "{} tracked path(s) match ownership domains with conflicting ownerRepo values. Examples: {}",
                                analysis.conflicts.len(),
                                summarize_paths(
                                    analysis
                                        .conflicts
                                        .iter()
                                        .map(|conflict| conflict.path.as_str())
                                        .collect::<Vec<_>>()
                                )
                            ),
                            source: config_source.clone(),
                        });
                    }

                    if !analysis.overlaps.is_empty() {
                        findings.push(DoctorFinding {
                            code: CODE_OWNERSHIP_OVERLAP.into(),
                            severity: DoctorSeverity::Warn.as_str().into(),
                            message: format!(
                                "{} tracked path(s) match multiple ownership domains with the same ownerRepo. Examples: {}",
                                analysis.overlaps.len(),
                                summarize_paths(
                                    analysis
                                        .overlaps
                                        .iter()
                                        .map(|overlap| overlap.path.as_str())
                                        .collect::<Vec<_>>()
                                )
                            ),
                            source: config_source.clone(),
                        });
                    }
                }
                None => findings.push(DoctorFinding {
                    code: CODE_OWNERSHIP_ANALYSIS_UNAVAILABLE.into(),
                    severity: DoctorSeverity::Warn.as_str().into(),
                    message:
                        "Ownership analysis could not read tracked files from git, so overlap/conflict checks were skipped."
                            .into(),
                    source: config_source.clone(),
                }),
            }
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

    Ok(report_with_findings(summary, configs, findings))
}

fn report_with_findings(
    summary: DoctorSummary,
    configs: Vec<DoctorConfig>,
    findings: Vec<DoctorFinding>,
) -> DoctorReport {
    DoctorReport {
        schema_version: DOCTOR_SCHEMA_VERSION.into(),
        tool_name: env!("CARGO_PKG_NAME").into(),
        tool_version: env!("CARGO_PKG_VERSION").into(),
        command: "doctor".into(),
        warnings: Vec::new(),
        summary,
        configs,
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
    let status = if report.findings.is_empty() {
        "pass"
    } else {
        "attention required"
    };
    println!("Docpact doctor: {status}.");
    println!(
        "Summary: config_present={}, layout={}, effective_configs={}, inherited_configs={}, rule_count={}, catalog_repos={}, ownership_domains={}, ownership_overlaps={}, ownership_conflicts={}, coverage_configured={}, routing_configured={}, doc_inventory_configured={}, freshness_configured={}, routing_intents={}, governed_doc_count={}",
        report.summary.config_present,
        report.summary.layout,
        report.summary.effective_config_count,
        report.summary.inherited_config_count,
        report.summary.rule_count,
        report.summary.catalog_repo_count,
        report.summary.ownership_domain_count,
        report.summary.ownership_overlap_count,
        report.summary.ownership_conflict_count,
        report.summary.coverage_configured,
        report.summary.routing_configured,
        report.summary.doc_inventory_configured,
        report.summary.freshness_configured,
        report.summary.routing_intent_count,
        report.summary.governed_doc_count,
    );
    println!("Configs:");
    if report.configs.is_empty() {
        println!("- none");
    } else {
        for config in &report.configs {
            println!(
                "- {}: {} rule(s), {} governed doc(s), routing intents={}, inherited={}, profile={}, base_dir={}",
                config.source,
                config.rule_count,
                config.governed_doc_count,
                config.routing_intent_count,
                config.inheritance_enabled,
                config
                    .workspace_profile
                    .clone()
                    .unwrap_or_else(|| "-".into()),
                if config.base_dir.is_empty() {
                    ".".to_string()
                } else {
                    config.base_dir.clone()
                },
            );
            println!(
                "  coverage={}, routing={}, doc_inventory={}, freshness={}, overrides(add={}, replace={}, disable={})",
                config.coverage_resolution,
                config.routing_resolution,
                config.doc_inventory_resolution,
                config.freshness_resolution,
                config.override_add_count,
                config.override_replace_count,
                config.override_disable_count,
            );
        }
    }
    println!("Findings:");
    if report.findings.is_empty() {
        println!("- none");
        println!(
            "Next: run `docpact lint --root . <diff-source>` or `docpact route --paths <path>`."
        );
        return;
    }

    for finding in &report.findings {
        println!(
            "- [{}] {} {}: {}",
            finding.severity, finding.code, finding.source, finding.message
        );
    }
    println!("Next: fix the findings above, then rerun `docpact doctor --root .`.");
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

fn summarize_paths(paths: Vec<&str>) -> String {
    let examples = paths.into_iter().take(3).collect::<Vec<_>>();
    if examples.is_empty() {
        "-".into()
    } else {
        examples.join(", ")
    }
}

fn build_doctor_configs(
    effective_configs: &[EffectiveConfig],
    catalog_configs: &[crate::config::LoadedCatalogConfig],
    ownership_configs: &[crate::config::LoadedOwnershipConfig],
) -> Vec<DoctorConfig> {
    let catalog_counts = catalog_configs
        .iter()
        .map(|config| (config.source.as_str(), config.catalog.repos.len()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let ownership_counts = ownership_configs
        .iter()
        .map(|config| (config.source.as_str(), config.ownership.domains.len()))
        .collect::<std::collections::BTreeMap<_, _>>();

    effective_configs
        .iter()
        .map(|config| {
            let governed_doc_count = config
                .rules
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

            DoctorConfig {
                source: config.source.clone(),
                base_dir: config.base_dir.clone(),
                rule_count: config.rules.len(),
                catalog_repo_count: *catalog_counts.get(config.source.as_str()).unwrap_or(&0),
                ownership_domain_count: *ownership_counts.get(config.source.as_str()).unwrap_or(&0),
                governed_doc_count,
                inheritance_enabled: config.inheritance.is_some(),
                workspace_profile: config
                    .inheritance
                    .as_ref()
                    .map(|inheritance| inheritance.workspace_profile.clone()),
                coverage_resolution: config.coverage.resolution.origin_kind.as_str().into(),
                routing_resolution: config.routing.resolution.origin_kind.as_str().into(),
                doc_inventory_resolution: config
                    .doc_inventory
                    .resolution
                    .origin_kind
                    .as_str()
                    .into(),
                freshness_resolution: config.freshness.resolution.origin_kind.as_str().into(),
                routing_intent_count: config.routing.routing.intents.len(),
                override_add_count: config
                    .inheritance
                    .as_ref()
                    .map(|inheritance| inheritance.add_count)
                    .unwrap_or(0),
                override_replace_count: config
                    .inheritance
                    .as_ref()
                    .map(|inheritance| inheritance.replace_count)
                    .unwrap_or(0),
                override_disable_count: config
                    .inheritance
                    .as_ref()
                    .map(|inheritance| inheritance.disable_count)
                    .unwrap_or(0),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        CODE_CONFIG_LOAD_FAILED, CODE_EMPTY_RULE_GRAPH, CODE_MISSING_CONFIG,
        CODE_MISSING_COVERAGE_SCOPE, CODE_MISSING_DOC_INVENTORY, CODE_MISSING_FRESHNESS_CONFIG,
        CODE_MISSING_GOVERNED_DOCS, CODE_OWNERSHIP_OVERLAP, DOCTOR_SCHEMA_VERSION, execute,
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
    fn doctor_reports_missing_config() {
        let root = temp_dir("docpact-doctor-missing");

        let report = execute(&base_args(&root)).expect("doctor should execute");

        assert_eq!(report.schema_version, DOCTOR_SCHEMA_VERSION);
        assert_eq!(report.command, "doctor");
        assert!(report.warnings.is_empty());
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
routing:
  intents:
    api:
      paths:
        - src/**
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
        assert!(report.summary.routing_configured);
        assert!(report.summary.doc_inventory_configured);
        assert!(report.summary.freshness_configured);
        assert_eq!(report.summary.routing_intent_count, 1);
        assert_eq!(report.summary.governed_doc_count, 1);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn doctor_reports_inheritance_and_override_details() {
        let root = temp_dir("docpact-doctor-inheritance");
        fs::create_dir_all(root.join(".docpact")).expect("doc root should exist");
        fs::create_dir_all(root.join("service/.docpact")).expect("service doc root should exist");
        fs::write(
            root.join(CONFIG_FILE),
            r#"version: 1
layout: workspace
workspace:
  name: demo
  profiles:
    default:
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
      routing:
        intents:
          service:
            paths:
              - src/**
      rules:
        - id: inherited-rule
          scope: workspace
          repo: workspace
          triggers:
            - path: src/**
              kind: code
          requiredDocs:
            - path: docs/guide.md
          reason: inherited
        - id: inherited-disable
          scope: workspace
          repo: workspace
          triggers:
            - path: src/legacy/**
              kind: code
          requiredDocs:
            - path: docs/legacy.md
          reason: disable me
rules:
  - id: root-only
    scope: workspace
    repo: workspace
    triggers:
      - path: AGENTS.md
        kind: doc
    requiredDocs:
      - path: .docpact/config.yaml
    reason: root
"#,
        )
        .expect("root config");
        fs::write(
            root.join("service/.docpact/config.yaml"),
            r#"version: 1
layout: repo
inherit:
  workspace_profile: default
overrides:
  rules:
    add:
      - id: local-extra
        scope: repo
        repo: service
        triggers:
          - path: src/payments/**
            kind: code
        requiredDocs:
          - path: docs/payments.md
        reason: local
    replace:
      - id: inherited-rule
        scope: repo
        repo: service
        triggers:
          - path: src/app/**
            kind: code
        requiredDocs:
          - path: docs/app.md
        reason: replaced
    disable:
      - id: inherited-disable
        reason: not applicable
  coverage:
    mode: merge
    include:
      - tests/**
  docInventory:
    mode: replace
    include:
      - README.md
  freshness:
    mode: replace
    warn_after_commits: 21
    warn_after_days: 34
    critical_after_days: 55
  routing:
    mode: merge
    intents:
      payments:
        paths:
          - src/payments/**
"#,
        )
        .expect("service config");

        let report = execute(&base_args(&root)).expect("doctor should execute");

        assert_eq!(report.summary.layout, "workspace");
        assert_eq!(report.summary.effective_config_count, 2);
        assert_eq!(report.summary.inherited_config_count, 1);

        let service = report
            .configs
            .iter()
            .find(|config| config.base_dir == "service")
            .expect("service config should exist");
        assert!(service.inheritance_enabled);
        assert_eq!(service.workspace_profile.as_deref(), Some("default"));
        assert_eq!(service.override_add_count, 1);
        assert_eq!(service.override_replace_count, 1);
        assert_eq!(service.override_disable_count, 1);
        assert_eq!(service.coverage_resolution, "override-merge");
        assert_eq!(service.routing_resolution, "override-merge");
        assert_eq!(service.doc_inventory_resolution, "override-replace");
        assert_eq!(service.freshness_resolution, "override-replace");
        assert_eq!(service.routing_intent_count, 2);
    }

    #[test]
    fn doctor_surfaces_same_owner_ownership_overlap_as_warning() {
        let root = temp_dir("docpact-doctor-ownership-overlap");
        init_git_repo(&root);
        fs::create_dir_all(root.join(".docpact")).expect("doc root should exist");
        fs::create_dir_all(root.join("src/payments")).expect("src dir should exist");

        fs::write(
            root.join(CONFIG_FILE),
            r#"version: 1
layout: repo
catalog:
  repos:
    - id: sample
      path: .
ownership:
  domains:
    - id: broad
      paths:
        include:
          - src/**
      ownerRepo: sample
    - id: payments
      paths:
        include:
          - src/payments/**
      ownerRepo: sample
rules:
  - id: api-docs
    scope: repo
    repo: sample
    triggers:
      - path: src/**
        kind: code
    requiredDocs:
      - path: README.md
    reason: sample
"#,
        )
        .expect("config should be written");
        fs::write(root.join("src/payments/charge.ts"), "export const x = 1;\n")
            .expect("tracked file should be written");
        git(&root, &["add", "."]);
        git(&root, &["commit", "-m", "Add tracked overlap sample"]);

        let report = execute(&base_args(&root)).expect("doctor should execute");

        assert_eq!(report.summary.ownership_domain_count, 2);
        assert_eq!(report.summary.ownership_overlap_count, 1);
        assert_eq!(report.summary.ownership_conflict_count, 0);
        assert!(report.findings.iter().any(|finding| {
            finding.code == CODE_OWNERSHIP_OVERLAP && finding.severity == "warn"
        }));
    }
}
