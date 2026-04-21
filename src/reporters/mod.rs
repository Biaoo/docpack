use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::cli::{DiagnosticDetail, DiagnosticsOutputFormat, LintMode, OutputFormat};
use crate::freshness::{FreshnessItem, FreshnessSummary, LintFreshnessReport};

pub const SUPPORTED_REPORTERS: &[&str] = &["text", "json", "sarif", "github"];
pub const SARIF_SCHEMA_URI: &str =
    "https://docs.oasis-open.org/sarif/sarif/v2.1.0/os/schemas/sarif-schema-2.1.0.json";
pub const DIAGNOSTICS_SCHEMA_VERSION: &str = "docpact.diagnostics.v1";

const METADATA_RULE_ID: &str = "metadata-review-fields";
const UNCOVERED_CHANGE_RULE_ID: &str = "coverage-uncovered-change";

fn default_freshness_status() -> String {
    "ok".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Problem {
    #[serde(rename = "type")]
    pub problem_type: String,
    pub path: String,
    pub message: String,
    pub rule_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_mode: Option<String>,
    pub failure_reason: String,
    pub suggested_action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trigger_paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticsArtifact {
    pub schema_version: String,
    pub tool_name: String,
    pub tool_version: String,
    pub status: String,
    pub changed_paths: Vec<String>,
    pub matched_rule_count: usize,
    pub uncovered_changed_paths: Vec<String>,
    pub coverage_status: String,
    #[serde(default = "default_freshness_status")]
    pub freshness_status: String,
    #[serde(default)]
    pub freshness_summary: FreshnessSummary,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stale_docs: Vec<FreshnessItem>,
    pub summary: ArtifactSummary,
    pub diagnostics: Vec<DiagnosticRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactSummary {
    pub total_count: usize,
    pub counts_by_type: BTreeMap<String, usize>,
    pub top_rules: Vec<RuleCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Report {
    pub status: String,
    pub changed_paths: Vec<String>,
    pub matched_rule_count: usize,
    pub uncovered_changed_paths: Vec<String>,
    pub coverage_status: String,
    #[serde(default = "default_freshness_status")]
    pub freshness_status: String,
    pub freshness_summary: FreshnessSummary,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stale_docs: Vec<FreshnessItem>,
    pub detail: String,
    pub summary: ReportSummary,
    pub items: Vec<DiagnosticItem>,
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
    pub has_next_page: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReportSummary {
    pub total_count: usize,
    pub counts_by_type: BTreeMap<String, usize>,
    pub top_rules: Vec<RuleCount>,
    pub page: usize,
    pub total_pages: usize,
    pub has_next_page: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleCount {
    pub rule_id: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticItem {
    pub diagnostic_id: String,
    #[serde(rename = "type")]
    pub problem_type: String,
    pub path: String,
    pub rule_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_mode: Option<String>,
    pub failure_reason: String,
    pub suggested_action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trigger_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticRecord {
    pub diagnostic_id: String,
    #[serde(rename = "type")]
    pub problem_type: String,
    pub path: String,
    pub message: String,
    pub rule_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_mode: Option<String>,
    pub failure_reason: String,
    pub suggested_action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trigger_paths: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifLog {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: String,
    pub runs: Vec<SarifRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifRun {
    pub tool: SarifTool,
    pub results: Vec<SarifResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<SarifProperties>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifTool {
    pub driver: SarifDriver,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifDriver {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(rename = "informationUri", skip_serializing_if = "Option::is_none")]
    pub information_uri: Option<String>,
    pub rules: Vec<SarifRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifRule {
    pub id: String,
    #[serde(rename = "shortDescription")]
    pub short_description: SarifMessage,
    #[serde(rename = "defaultConfiguration")]
    pub default_configuration: SarifDefaultConfiguration,
    #[serde(rename = "helpUri", skip_serializing_if = "Option::is_none")]
    pub help_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifDefaultConfiguration {
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifResult {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub level: String,
    pub message: SarifMessage,
    pub locations: Vec<SarifLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<SarifResultProperties>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    pub physical_location: SarifPhysicalLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    pub artifact_location: SarifArtifactLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifArtifactLocation {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifMessage {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifProperties {
    pub status: String,
    #[serde(rename = "changedPaths")]
    pub changed_paths: Vec<String>,
    #[serde(rename = "matchedRuleCount")]
    pub matched_rule_count: usize,
    #[serde(rename = "uncoveredChangedPaths")]
    pub uncovered_changed_paths: Vec<String>,
    #[serde(rename = "coverageStatus")]
    pub coverage_status: String,
    #[serde(rename = "freshnessStatus")]
    pub freshness_status: String,
    #[serde(rename = "freshnessSummary")]
    pub freshness_summary: FreshnessSummary,
    #[serde(rename = "staleDocs", default, skip_serializing_if = "Vec::is_empty")]
    pub stale_docs: Vec<FreshnessItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SarifResultProperties {
    #[serde(rename = "problemType")]
    pub problem_type: String,
    #[serde(rename = "diagnosticId")]
    pub diagnostic_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PaginatedDiagnostics {
    summary: ReportSummary,
    items: Vec<DiagnosticItem>,
    page: usize,
    page_size: usize,
    total_count: usize,
    total_pages: usize,
    has_next_page: bool,
    next_page: Option<usize>,
}

impl Problem {
    pub fn missing_metadata(path: String, message: String) -> Self {
        Self {
            problem_type: "missing-metadata".into(),
            path,
            message,
            rule_id: METADATA_RULE_ID.into(),
            required_mode: None,
            failure_reason: "missing_review_metadata_keys".into(),
            suggested_action: "add_review_metadata".into(),
            rule_source: None,
            trigger_paths: Vec::new(),
            rule_reason: None,
        }
    }

    pub fn uncovered_change(path: String) -> Self {
        Self {
            problem_type: "uncovered-change".into(),
            path: path.clone(),
            message: format!(
                "Changed path is not covered by any docpact rule trigger. Add a matching rule or exclude it from coverage: {path}."
            ),
            rule_id: UNCOVERED_CHANGE_RULE_ID.into(),
            required_mode: None,
            failure_reason: "unmatched_changed_path".into(),
            suggested_action: "add_rule_or_exclude_path".into(),
            rule_source: None,
            trigger_paths: Vec::new(),
            rule_reason: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn missing_review(
        path: String,
        rule_id: String,
        rule_source: String,
        required_mode: String,
        failure_reason: String,
        suggested_action: String,
        trigger_paths: Vec<String>,
        rule_reason: String,
        message: String,
    ) -> Self {
        Self {
            problem_type: "missing-review".into(),
            path,
            message,
            rule_id,
            required_mode: Some(required_mode),
            failure_reason,
            suggested_action,
            rule_source: Some(rule_source),
            trigger_paths,
            rule_reason: Some(rule_reason),
        }
    }

    fn severity_rank(&self) -> usize {
        match self.problem_type.as_str() {
            "missing-review" => 0,
            "missing-metadata" => 1,
            "uncovered-change" => 2,
            _ => 3,
        }
    }
}

impl DiagnosticRecord {
    fn to_item(&self, detail: DiagnosticDetail) -> DiagnosticItem {
        DiagnosticItem {
            diagnostic_id: self.diagnostic_id.clone(),
            problem_type: self.problem_type.clone(),
            path: self.path.clone(),
            rule_id: self.rule_id.clone(),
            required_mode: self.required_mode.clone(),
            failure_reason: self.failure_reason.clone(),
            suggested_action: self.suggested_action.clone(),
            rule_source: (detail == DiagnosticDetail::Full)
                .then(|| self.rule_source.clone())
                .flatten(),
            trigger_paths: if detail == DiagnosticDetail::Full {
                self.trigger_paths.clone()
            } else {
                Vec::new()
            },
        }
    }
}

pub fn emit_lint_output(
    artifact: &DiagnosticsArtifact,
    mode: LintMode,
    format: OutputFormat,
    detail: DiagnosticDetail,
    page: usize,
    page_size: usize,
) -> Report {
    let report = build_report_from_artifact(artifact, detail, page, page_size);

    match format {
        OutputFormat::Text => {
            if artifact.changed_paths.is_empty() {
                println!("Docpact: no changed paths to inspect.");
            } else {
                emit_text_report(&report, mode);
                emit_annotations(&artifact.diagnostics, mode);
            }
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("report should serialize")
            );
        }
        OutputFormat::Sarif => {
            println!(
                "{}",
                serde_json::to_string_pretty(&build_sarif_log_from_artifact(artifact, mode))
                    .expect("sarif log should serialize")
            );
        }
    }

    report
}

pub fn emit_diagnostic_show(record: &DiagnosticRecord, format: DiagnosticsOutputFormat) {
    match format {
        DiagnosticsOutputFormat::Text => {
            println!("Diagnostic {}", record.diagnostic_id);
            println!("type={}", record.problem_type);
            println!("path={}", record.path);
            println!("rule_id={}", record.rule_id);
            println!(
                "required_mode={}",
                record.required_mode.as_deref().unwrap_or("n/a")
            );
            println!("failure_reason={}", record.failure_reason);
            println!("suggested_action={}", record.suggested_action);
            if let Some(rule_source) = &record.rule_source {
                println!("rule_source={rule_source}");
            }
            if !record.trigger_paths.is_empty() {
                println!("trigger_paths={}", record.trigger_paths.join(","));
            }
            if let Some(rule_reason) = &record.rule_reason {
                println!("rule_reason={rule_reason}");
            }
            println!("message={}", record.message);
        }
        DiagnosticsOutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(record).expect("diagnostic should serialize")
            );
        }
    }
}

pub fn emit_report_hint(format: OutputFormat, report_path: &str, drilldown_id: Option<&str>) {
    for line in report_hint_lines(report_path, drilldown_id) {
        match format {
            OutputFormat::Text => println!("{line}"),
            OutputFormat::Json | OutputFormat::Sarif => eprintln!("{line}"),
        }
    }
}

pub fn report_hint_lines(report_path: &str, drilldown_id: Option<&str>) -> Vec<String> {
    let mut lines = vec![format!("Detailed report saved to {report_path}")];
    if let Some(id) = drilldown_id {
        lines.push(format!(
            "Use `docpact diagnostics show --report {report_path} --id {id}` for drill-down."
        ));
    }
    lines
}

pub fn build_diagnostics_artifact(
    problems: &[Problem],
    changed_paths: &[String],
    matched_rule_count: usize,
) -> DiagnosticsArtifact {
    build_diagnostics_artifact_with_freshness(problems, changed_paths, matched_rule_count, None)
}

pub fn build_diagnostics_artifact_with_freshness(
    problems: &[Problem],
    changed_paths: &[String],
    matched_rule_count: usize,
    lint_freshness: Option<&LintFreshnessReport>,
) -> DiagnosticsArtifact {
    let diagnostics = sorted_diagnostics(problems)
        .into_iter()
        .enumerate()
        .map(|(index, problem)| DiagnosticRecord {
            diagnostic_id: format!("d{:03}", index + 1),
            problem_type: problem.problem_type,
            path: problem.path,
            message: problem.message,
            rule_id: problem.rule_id,
            required_mode: problem.required_mode,
            failure_reason: problem.failure_reason,
            suggested_action: problem.suggested_action,
            rule_source: problem.rule_source,
            trigger_paths: problem.trigger_paths,
            rule_reason: problem.rule_reason,
        })
        .collect::<Vec<_>>();

    let mut counts_by_type = BTreeMap::new();
    let mut counts_by_rule = BTreeMap::new();
    let uncovered_changed_paths = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.problem_type == "uncovered-change")
        .map(|diagnostic| diagnostic.path.clone())
        .collect::<Vec<_>>();
    for diagnostic in &diagnostics {
        *counts_by_type
            .entry(diagnostic.problem_type.clone())
            .or_insert(0usize) += 1;
        *counts_by_rule
            .entry(diagnostic.rule_id.clone())
            .or_insert(0usize) += 1;
    }

    let mut top_rules = counts_by_rule
        .into_iter()
        .map(|(rule_id, count)| RuleCount { rule_id, count })
        .collect::<Vec<_>>();
    top_rules.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.rule_id.cmp(&right.rule_id))
    });

    DiagnosticsArtifact {
        schema_version: DIAGNOSTICS_SCHEMA_VERSION.into(),
        tool_name: env!("CARGO_PKG_NAME").into(),
        tool_version: env!("CARGO_PKG_VERSION").into(),
        status: if diagnostics.is_empty() {
            "ok".into()
        } else {
            "fail".into()
        },
        changed_paths: changed_paths.to_vec(),
        matched_rule_count,
        uncovered_changed_paths: uncovered_changed_paths.clone(),
        coverage_status: if uncovered_changed_paths.is_empty() {
            "ok".into()
        } else {
            "has-uncovered-change".into()
        },
        freshness_status: lint_freshness
            .map(|report| report.freshness_status.clone())
            .unwrap_or_else(|| "ok".into()),
        freshness_summary: lint_freshness
            .map(|report| report.summary.clone())
            .unwrap_or_default(),
        stale_docs: lint_freshness
            .map(|report| report.stale_docs.clone())
            .unwrap_or_default(),
        summary: ArtifactSummary {
            total_count: diagnostics.len(),
            counts_by_type,
            top_rules,
        },
        diagnostics,
    }
}

pub fn build_report(
    problems: &[Problem],
    changed_paths: &[String],
    matched_rule_count: usize,
    detail: DiagnosticDetail,
    page: usize,
    page_size: usize,
) -> Report {
    let artifact = build_diagnostics_artifact(problems, changed_paths, matched_rule_count);
    build_report_from_artifact(&artifact, detail, page, page_size)
}

pub fn build_report_from_artifact(
    artifact: &DiagnosticsArtifact,
    detail: DiagnosticDetail,
    page: usize,
    page_size: usize,
) -> Report {
    let paged = build_paginated_diagnostics(&artifact.diagnostics, detail, page, page_size);

    Report {
        status: artifact.status.clone(),
        changed_paths: artifact.changed_paths.clone(),
        matched_rule_count: artifact.matched_rule_count,
        uncovered_changed_paths: artifact.uncovered_changed_paths.clone(),
        coverage_status: artifact.coverage_status.clone(),
        freshness_status: artifact.freshness_status.clone(),
        freshness_summary: artifact.freshness_summary.clone(),
        stale_docs: artifact.stale_docs.clone(),
        detail: detail.as_str().into(),
        summary: paged.summary.clone(),
        items: paged.items,
        page: paged.page,
        page_size: paged.page_size,
        total_count: paged.total_count,
        total_pages: paged.total_pages,
        has_next_page: paged.has_next_page,
        next_page: paged.next_page,
    }
}

pub fn build_sarif_log(
    problems: &[Problem],
    changed_paths: &[String],
    matched_rule_count: usize,
    mode: LintMode,
) -> SarifLog {
    let artifact = build_diagnostics_artifact(problems, changed_paths, matched_rule_count);
    build_sarif_log_from_artifact(&artifact, mode)
}

pub fn build_sarif_log_from_artifact(artifact: &DiagnosticsArtifact, mode: LintMode) -> SarifLog {
    let level = sarif_level(mode).to_string();

    SarifLog {
        schema: SARIF_SCHEMA_URI.into(),
        version: "2.1.0".into(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: artifact.tool_name.clone(),
                    version: Some(artifact.tool_version.clone()),
                    information_uri: None,
                    rules: sarif_rules(mode),
                },
            },
            results: artifact
                .diagnostics
                .iter()
                .map(|diagnostic| SarifResult {
                    rule_id: sarif_rule_id(diagnostic).into(),
                    level: level.clone(),
                    message: SarifMessage {
                        text: diagnostic.message.clone(),
                    },
                    locations: vec![SarifLocation {
                        physical_location: SarifPhysicalLocation {
                            artifact_location: SarifArtifactLocation {
                                uri: diagnostic.path.clone(),
                            },
                        },
                    }],
                    properties: Some(SarifResultProperties {
                        problem_type: diagnostic.problem_type.clone(),
                        diagnostic_id: diagnostic.diagnostic_id.clone(),
                    }),
                })
                .collect(),
            properties: Some(SarifProperties {
                status: artifact.status.clone(),
                changed_paths: artifact.changed_paths.clone(),
                matched_rule_count: artifact.matched_rule_count,
                uncovered_changed_paths: artifact.uncovered_changed_paths.clone(),
                coverage_status: artifact.coverage_status.clone(),
                freshness_status: artifact.freshness_status.clone(),
                freshness_summary: artifact.freshness_summary.clone(),
                stale_docs: artifact.stale_docs.clone(),
            }),
        }],
    }
}

fn build_paginated_diagnostics(
    diagnostics: &[DiagnosticRecord],
    detail: DiagnosticDetail,
    requested_page: usize,
    page_size: usize,
) -> PaginatedDiagnostics {
    let total_count = diagnostics.len();
    let total_pages = usize::max(1, total_count.div_ceil(page_size));
    let page = requested_page.min(total_pages);
    let has_next_page = page < total_pages;
    let next_page = has_next_page.then_some(page + 1);

    let items = if detail == DiagnosticDetail::Summary || diagnostics.is_empty() {
        Vec::new()
    } else {
        let start = (page - 1) * page_size;
        let end = usize::min(start + page_size, total_count);
        diagnostics[start..end]
            .iter()
            .map(|record| record.to_item(detail))
            .collect()
    };

    let summary = ReportSummary {
        total_count: artifact_total_count(diagnostics),
        counts_by_type: counts_by_type(diagnostics),
        top_rules: top_rules(diagnostics),
        page,
        total_pages,
        has_next_page,
        next_page,
    };

    PaginatedDiagnostics {
        summary,
        items,
        page,
        page_size,
        total_count,
        total_pages,
        has_next_page,
        next_page,
    }
}

fn emit_text_report(report: &Report, mode: LintMode) {
    if report.total_count == 0 {
        println!("Docpact: no problems found.");
        return;
    }

    let heading = match mode {
        LintMode::Enforce => "Docpact found blocking problems:",
        LintMode::Warn => "Docpact found warnings:",
    };

    println!("{heading}");
    println!(
        "Summary: total={}, counts_by_type={}, top_rules={}, coverage_status={}, uncovered={}, freshness_status={}, stale_docs={}, critical_stale_docs={}, invalid_baselines={}, page={}/{}, page_size={}",
        report.total_count,
        format_counts_by_type(&report.summary.counts_by_type),
        format_top_rules(&report.summary.top_rules),
        report.coverage_status,
        report.uncovered_changed_paths.len(),
        report.freshness_status,
        report.freshness_summary.stale_doc_count,
        report.freshness_summary.critical_count,
        report.freshness_summary.invalid_baseline_count,
        report.page,
        report.total_pages,
        report.page_size,
    );

    if let Some(next_page) = report.next_page {
        println!("Next page: --diagnostics-page {next_page}");
    }

    if report.detail == DiagnosticDetail::Summary.as_str() {
        return;
    }

    for item in &report.items {
        println!("{}", format_item(item));
    }
}

fn counts_by_type(diagnostics: &[DiagnosticRecord]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for diagnostic in diagnostics {
        *counts
            .entry(diagnostic.problem_type.clone())
            .or_insert(0usize) += 1;
    }
    counts
}

fn top_rules(diagnostics: &[DiagnosticRecord]) -> Vec<RuleCount> {
    let mut counts = BTreeMap::new();
    for diagnostic in diagnostics {
        *counts.entry(diagnostic.rule_id.clone()).or_insert(0usize) += 1;
    }

    let mut rules = counts
        .into_iter()
        .map(|(rule_id, count)| RuleCount { rule_id, count })
        .collect::<Vec<_>>();
    rules.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.rule_id.cmp(&right.rule_id))
    });
    rules
}

fn artifact_total_count(diagnostics: &[DiagnosticRecord]) -> usize {
    diagnostics.len()
}

fn sorted_diagnostics(problems: &[Problem]) -> Vec<Problem> {
    let mut diagnostics = problems.to_vec();
    diagnostics.sort_by(|left, right| {
        (
            left.severity_rank(),
            &left.problem_type,
            &left.rule_id,
            &left.path,
            &left.required_mode,
            &left.failure_reason,
            &left.rule_source,
            &left.trigger_paths,
            &left.rule_reason,
            &left.message,
        )
            .cmp(&(
                right.severity_rank(),
                &right.problem_type,
                &right.rule_id,
                &right.path,
                &right.required_mode,
                &right.failure_reason,
                &right.rule_source,
                &right.trigger_paths,
                &right.rule_reason,
                &right.message,
            ))
    });
    diagnostics
}

fn sarif_rules(mode: LintMode) -> Vec<SarifRule> {
    let level = sarif_level(mode).to_string();

    vec![
        SarifRule {
            id: "missing-review".into(),
            short_description: SarifMessage {
                text: "A required reviewed document is missing or did not satisfy its required review mode."
                    .into(),
            },
            default_configuration: SarifDefaultConfiguration {
                level: level.clone(),
            },
            help_uri: None,
        },
        SarifRule {
            id: METADATA_RULE_ID.into(),
            short_description: SarifMessage {
                text: "A touched governed required document is missing required review metadata."
                    .into(),
            },
            default_configuration: SarifDefaultConfiguration {
                level: level.clone(),
            },
            help_uri: None,
        },
        SarifRule {
            id: UNCOVERED_CHANGE_RULE_ID.into(),
            short_description: SarifMessage {
                text: "A changed path was not covered by any docpact rule trigger.".into(),
            },
            default_configuration: SarifDefaultConfiguration { level },
            help_uri: None,
        },
    ]
}

fn sarif_rule_id(problem: &DiagnosticRecord) -> &'static str {
    match problem.problem_type.as_str() {
        "missing-review" => "missing-review",
        "missing-metadata" => METADATA_RULE_ID,
        "uncovered-change" => UNCOVERED_CHANGE_RULE_ID,
        _ => "unknown-problem",
    }
}

fn sarif_level(mode: LintMode) -> &'static str {
    match mode {
        LintMode::Enforce => "error",
        LintMode::Warn => "warning",
    }
}

fn format_counts_by_type(counts_by_type: &BTreeMap<String, usize>) -> String {
    counts_by_type
        .iter()
        .map(|(problem_type, count)| format!("{problem_type}={count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_top_rules(top_rules: &[RuleCount]) -> String {
    top_rules
        .iter()
        .map(|entry| format!("{}={}", entry.rule_id, entry.count))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_item(item: &DiagnosticItem) -> String {
    let mut parts = vec![
        format!("- [{}]", item.diagnostic_id),
        format!("type={}", item.problem_type),
        format!("path={}", item.path),
        format!("rule={}", item.rule_id),
        format!("mode={}", item.required_mode.as_deref().unwrap_or("n/a")),
        format!("reason={}", item.failure_reason),
        format!("action={}", item.suggested_action),
    ];

    if let Some(rule_source) = &item.rule_source {
        parts.push(format!("rule_source={rule_source}"));
    }

    if !item.trigger_paths.is_empty() {
        parts.push(format!("trigger_paths={}", item.trigger_paths.join(",")));
    }

    parts.join(" ")
}

fn emit_annotations(diagnostics: &[DiagnosticRecord], mode: LintMode) {
    if std::env::var_os("GITHUB_ACTIONS").is_none() {
        return;
    }

    let level = match mode {
        LintMode::Enforce => "error",
        LintMode::Warn => "warning",
    };

    for diagnostic in diagnostics {
        println!("::{level} file={}::{}", diagnostic.path, diagnostic.message);
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::{DiagnosticDetail, LintMode};
    use crate::freshness::{FreshnessItem, FreshnessSummary, LintFreshnessReport};

    use super::{
        DIAGNOSTICS_SCHEMA_VERSION, Problem, SARIF_SCHEMA_URI, build_diagnostics_artifact,
        build_diagnostics_artifact_with_freshness, build_report, build_report_from_artifact,
        build_sarif_log, build_sarif_log_from_artifact, report_hint_lines,
    };

    fn review_problem(
        path: &str,
        rule_id: &str,
        required_mode: &str,
        failure_reason: &str,
    ) -> Problem {
        Problem::missing_review(
            path.into(),
            rule_id.into(),
            ".docpact/config.yaml".into(),
            required_mode.into(),
            failure_reason.into(),
            "touch_required_doc".into(),
            vec!["src/index.ts".into()],
            "repo rationale".into(),
            "missing review".into(),
        )
    }

    fn uncovered_problem(path: &str) -> Problem {
        Problem::uncovered_change(path.into())
    }

    fn lint_freshness_report() -> LintFreshnessReport {
        LintFreshnessReport {
            freshness_status: "has-critical-stale-doc".into(),
            summary: FreshnessSummary {
                governed_doc_count: 2,
                fresh_doc_count: 0,
                stale_doc_count: 2,
                warn_count: 1,
                critical_count: 1,
                invalid_baseline_count: 1,
            },
            stale_docs: vec![FreshnessItem {
                path: "docs/api.md".into(),
                last_reviewed_commit: Some("abc123".into()),
                last_reviewed_at: Some("2026-01-01".into()),
                commits_since_review: Some(8),
                days_since_review: Some(100),
                associated_changed_paths: vec!["src/api/client.ts".into()],
                associated_changed_paths_count: 1,
                staleness_level: "critical".into(),
                baseline_problems: vec!["invalid-lastReviewedCommit".into()],
            }],
        }
    }

    #[test]
    fn build_report_marks_empty_run_as_ok() {
        let report = build_report(&[], &[], 0, DiagnosticDetail::Compact, 1, 5);
        assert_eq!(report.status, "ok");
        assert!(report.items.is_empty());
        assert_eq!(report.total_pages, 1);
    }

    #[test]
    fn build_artifact_uses_stable_ids_and_schema() {
        let artifact = build_diagnostics_artifact(
            &[review_problem(
                "docs/api.md",
                "repo-rule",
                "review_or_update",
                "required_doc_not_touched",
            )],
            &["src/index.ts".into()],
            1,
        );

        assert_eq!(artifact.schema_version, DIAGNOSTICS_SCHEMA_VERSION);
        assert_eq!(artifact.diagnostics[0].diagnostic_id, "d001");
        assert_eq!(
            artifact.diagnostics[0].rule_reason.as_deref(),
            Some("repo rationale")
        );
    }

    #[test]
    fn build_artifact_includes_coverage_summary_fields() {
        let artifact = build_diagnostics_artifact_with_freshness(
            &[
                review_problem(
                    "docs/api.md",
                    "repo-rule",
                    "review_or_update",
                    "required_doc_not_touched",
                ),
                uncovered_problem("src/payments/charge.ts"),
            ],
            &["src/payments/charge.ts".into(), "src/api/client.ts".into()],
            1,
            Some(&lint_freshness_report()),
        );

        assert_eq!(
            artifact.uncovered_changed_paths,
            vec!["src/payments/charge.ts"]
        );
        assert_eq!(artifact.coverage_status, "has-uncovered-change");
        assert_eq!(artifact.freshness_status, "has-critical-stale-doc");
        assert_eq!(artifact.freshness_summary.critical_count, 1);
        assert_eq!(artifact.stale_docs.len(), 1);
    }

    #[test]
    fn build_report_uses_summary_first_paging() {
        let problems = vec![
            review_problem(
                "docs/d.md",
                "repo-rule",
                "review_or_update",
                "required_doc_not_touched",
            ),
            review_problem(
                "docs/c.md",
                "repo-rule",
                "review_or_update",
                "required_doc_not_touched",
            ),
            review_problem(
                "docs/b.md",
                "repo-rule",
                "review_or_update",
                "required_doc_not_touched",
            ),
            review_problem(
                "docs/a.md",
                "repo-rule",
                "review_or_update",
                "required_doc_not_touched",
            ),
            Problem::missing_metadata(
                ".docpact/quality-rubric.md".into(),
                "Missing Markdown metadata keys: lastReviewedAt".into(),
            ),
            review_problem(
                "docs/e.md",
                "repo-rule",
                "review_or_update",
                "required_doc_not_touched",
            ),
        ];

        let page_one = build_report(&problems, &[], 1, DiagnosticDetail::Compact, 1, 5);
        assert_eq!(page_one.total_count, 6);
        assert_eq!(page_one.page_size, 5);
        assert_eq!(page_one.total_pages, 2);
        assert_eq!(page_one.next_page, Some(2));
        assert_eq!(page_one.items.len(), 5);
        assert_eq!(page_one.items[0].diagnostic_id, "d001");
        assert_eq!(page_one.items[4].diagnostic_id, "d005");

        let page_two = build_report(&problems, &[], 1, DiagnosticDetail::Compact, 2, 5);
        assert_eq!(page_two.items.len(), 1);
        assert_eq!(page_two.items[0].diagnostic_id, "d006");
        assert_eq!(page_two.items[0].problem_type, "missing-metadata");
    }

    #[test]
    fn build_report_from_artifact_sorts_diagnostics_stably() {
        let artifact = build_diagnostics_artifact(
            &[
                review_problem(
                    "docs/z.md",
                    "z-rule",
                    "review_or_update",
                    "required_doc_not_touched",
                ),
                Problem::missing_metadata(
                    ".docpact/quality-rubric.md".into(),
                    "Missing Markdown metadata keys: lastReviewedAt".into(),
                ),
                review_problem(
                    "docs/a.md",
                    "a-rule",
                    "review_or_update",
                    "required_doc_not_touched",
                ),
            ],
            &[],
            1,
        );

        let report = build_report_from_artifact(&artifact, DiagnosticDetail::Compact, 1, 5);
        assert_eq!(report.items.len(), 3);
        assert_eq!(report.items[0].rule_id, "a-rule");
        assert_eq!(report.items[1].rule_id, "z-rule");
        assert_eq!(report.items[2].rule_id, "metadata-review-fields");
    }

    #[test]
    fn build_report_full_detail_includes_rule_source_and_trigger_paths() {
        let report = build_report(
            &[review_problem(
                "docs/api.md",
                "repo-rule",
                "review_or_update",
                "required_doc_not_touched",
            )],
            &[],
            1,
            DiagnosticDetail::Full,
            1,
            5,
        );

        assert_eq!(
            report.items[0].rule_source.as_deref(),
            Some(".docpact/config.yaml")
        );
        assert_eq!(report.items[0].trigger_paths, vec!["src/index.ts"]);
    }

    #[test]
    fn build_report_summary_detail_omits_items() {
        let report = build_report(
            &[review_problem(
                "docs/api.md",
                "repo-rule",
                "review_or_update",
                "required_doc_not_touched",
            )],
            &[],
            1,
            DiagnosticDetail::Summary,
            1,
            5,
        );

        assert!(report.items.is_empty());
        assert_eq!(report.summary.total_count, 1);
    }

    #[test]
    fn report_hint_lines_include_drill_down_command() {
        let lines = report_hint_lines(".docpact/runs/latest.json", Some("d001"));
        assert_eq!(lines.len(), 2);
        assert!(lines[1].contains("docpact diagnostics show"));
        assert!(lines[1].contains("--id d001"));
    }

    #[test]
    fn build_sarif_log_emits_valid_top_level_shape_for_empty_results() {
        let sarif = build_sarif_log(&[], &[], 0, LintMode::Warn);
        assert_eq!(sarif.schema, SARIF_SCHEMA_URI);
        assert_eq!(sarif.version, "2.1.0");
        assert_eq!(sarif.runs.len(), 1);
        assert!(sarif.runs[0].results.is_empty());
        assert_eq!(sarif.runs[0].tool.driver.rules.len(), 3);
        assert_eq!(
            sarif.runs[0].properties.as_ref().map(|p| p.status.as_str()),
            Some("ok")
        );
    }

    #[test]
    fn build_sarif_log_maps_problems_to_results() {
        let problems = vec![review_problem(
            "docs/api.md",
            "repo-rule",
            "review_or_update",
            "required_doc_not_touched",
        )];

        let sarif = build_sarif_log(&problems, &["src/index.ts".into()], 1, LintMode::Enforce);
        let run = &sarif.runs[0];
        let result = &run.results[0];

        assert_eq!(result.rule_id, "missing-review");
        assert_eq!(result.level, "error");
        assert_eq!(
            result.locations[0].physical_location.artifact_location.uri,
            "docs/api.md"
        );
        assert_eq!(
            result.properties.as_ref().map(|p| p.problem_type.as_str()),
            Some("missing-review")
        );
        assert_eq!(
            result.properties.as_ref().map(|p| p.diagnostic_id.as_str()),
            Some("d001")
        );
        assert_eq!(
            run.properties.as_ref().map(|p| p.changed_paths.clone()),
            Some(vec!["src/index.ts".into()])
        );
        assert_eq!(
            run.tool.driver.rules[0].default_configuration.level,
            "error"
        );
        assert_eq!(run.tool.driver.name, "docpact");
    }

    #[test]
    fn build_sarif_log_keeps_diagnostic_id_aligned_with_report() {
        let problems = vec![
            review_problem(
                "docs/b.md",
                "repo-rule",
                "review_or_update",
                "required_doc_not_touched",
            ),
            review_problem(
                "docs/a.md",
                "repo-rule",
                "review_or_update",
                "required_doc_not_touched",
            ),
        ];

        let artifact = build_diagnostics_artifact(&problems, &["src/index.ts".into()], 1);
        let report = build_report_from_artifact(&artifact, DiagnosticDetail::Compact, 1, 5);
        let sarif = build_sarif_log_from_artifact(&artifact, LintMode::Warn);

        assert_eq!(report.items[0].diagnostic_id, "d001");
        assert_eq!(report.items[1].diagnostic_id, "d002");
        assert_eq!(
            sarif.runs[0].results[0]
                .properties
                .as_ref()
                .map(|p| p.diagnostic_id.as_str()),
            Some("d001")
        );
        assert_eq!(
            sarif.runs[0].results[1]
                .properties
                .as_ref()
                .map(|p| p.diagnostic_id.as_str()),
            Some("d002")
        );
    }

    #[test]
    fn build_sarif_log_exposes_coverage_status_and_uncovered_paths() {
        let problems = vec![uncovered_problem("src/payments/charge.ts")];

        let sarif = build_sarif_log(
            &problems,
            &["src/payments/charge.ts".into(), "src/api/client.ts".into()],
            0,
            LintMode::Warn,
        );
        let run = &sarif.runs[0];

        assert_eq!(run.results[0].rule_id, "coverage-uncovered-change");
        assert_eq!(
            run.properties
                .as_ref()
                .map(|properties| properties.coverage_status.as_str()),
            Some("has-uncovered-change")
        );
        assert_eq!(
            run.properties
                .as_ref()
                .map(|properties| properties.uncovered_changed_paths.clone()),
            Some(vec!["src/payments/charge.ts".into()])
        );
    }

    #[test]
    fn build_sarif_log_exposes_freshness_summary() {
        let artifact = build_diagnostics_artifact_with_freshness(
            &[review_problem(
                "docs/api.md",
                "repo-rule",
                "review_or_update",
                "required_doc_not_touched",
            )],
            &["src/api/client.ts".into()],
            1,
            Some(&lint_freshness_report()),
        );
        let sarif = build_sarif_log_from_artifact(&artifact, LintMode::Warn);
        let run = &sarif.runs[0];

        assert_eq!(
            run.properties
                .as_ref()
                .map(|properties| properties.freshness_status.as_str()),
            Some("has-critical-stale-doc")
        );
        assert_eq!(
            run.properties
                .as_ref()
                .map(|properties| properties.freshness_summary.critical_count),
            Some(1)
        );
        assert_eq!(
            run.properties
                .as_ref()
                .map(|properties| properties.stale_docs[0].path.as_str()),
            Some("docs/api.md")
        );
    }
}
