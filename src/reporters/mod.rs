use std::collections::BTreeMap;

use serde::Serialize;

use crate::cli::{DiagnosticDetail, LintMode, OutputFormat};

pub const SUPPORTED_REPORTERS: &[&str] = &["text", "json", "sarif", "github"];
pub const SARIF_SCHEMA_URI: &str =
    "https://docs.oasis-open.org/sarif/sarif/v2.1.0/os/schemas/sarif-schema-2.1.0.json";

const METADATA_RULE_ID: &str = "metadata-review-fields";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
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
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Report {
    pub status: String,
    pub changed_paths: Vec<String>,
    pub matched_rule_count: usize,
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuleCount {
    pub rule_id: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SarifLog {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: String,
    pub runs: Vec<SarifRun>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SarifRun {
    pub tool: SarifTool,
    pub results: Vec<SarifResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<SarifProperties>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SarifTool {
    pub driver: SarifDriver,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SarifDriver {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(rename = "informationUri", skip_serializing_if = "Option::is_none")]
    pub information_uri: Option<String>,
    pub rules: Vec<SarifRule>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SarifRule {
    pub id: String,
    #[serde(rename = "shortDescription")]
    pub short_description: SarifMessage,
    #[serde(rename = "defaultConfiguration")]
    pub default_configuration: SarifDefaultConfiguration,
    #[serde(rename = "helpUri", skip_serializing_if = "Option::is_none")]
    pub help_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SarifDefaultConfiguration {
    pub level: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SarifResult {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub level: String,
    pub message: SarifMessage,
    pub locations: Vec<SarifLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<SarifResultProperties>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    pub physical_location: SarifPhysicalLocation,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    pub artifact_location: SarifArtifactLocation,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SarifArtifactLocation {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SarifMessage {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SarifProperties {
    pub status: String,
    #[serde(rename = "changedPaths")]
    pub changed_paths: Vec<String>,
    #[serde(rename = "matchedRuleCount")]
    pub matched_rule_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SarifResultProperties {
    #[serde(rename = "problemType")]
    pub problem_type: String,
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
        }
    }

    fn severity_rank(&self) -> usize {
        match self.problem_type.as_str() {
            "missing-review" => 0,
            "missing-metadata" => 1,
            _ => 2,
        }
    }
}

pub fn emit_no_changed_paths(
    format: OutputFormat,
    detail: DiagnosticDetail,
    page: usize,
    page_size: usize,
) {
    match format {
        OutputFormat::Text => {
            println!("Docpact: no changed paths to inspect.");
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&build_report(&[], &[], 0, detail, page, page_size))
                    .expect("report should serialize")
            );
        }
        OutputFormat::Sarif => {
            println!(
                "{}",
                serde_json::to_string_pretty(&build_sarif_log(&[], &[], 0, LintMode::Warn))
                    .expect("sarif log should serialize")
            );
        }
    }
}

pub fn emit_problems(
    problems: &[Problem],
    changed_paths: &[String],
    matched_rule_count: usize,
    mode: LintMode,
    format: OutputFormat,
    detail: DiagnosticDetail,
    page: usize,
    page_size: usize,
) {
    match format {
        OutputFormat::Text => {
            let report = build_report(
                problems,
                changed_paths,
                matched_rule_count,
                detail,
                page,
                page_size,
            );
            emit_text_problems(&report, mode);
            emit_annotations(problems, mode);
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&build_report(
                    problems,
                    changed_paths,
                    matched_rule_count,
                    detail,
                    page,
                    page_size,
                ))
                .expect("report should serialize")
            );
            emit_annotations(problems, mode);
        }
        OutputFormat::Sarif => {
            println!(
                "{}",
                serde_json::to_string_pretty(&build_sarif_log(
                    problems,
                    changed_paths,
                    matched_rule_count,
                    mode,
                ))
                .expect("sarif log should serialize")
            );
        }
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
    let paged = build_paginated_diagnostics(problems, detail, page, page_size);
    let status = if problems.is_empty() { "ok" } else { "fail" };

    Report {
        status: status.into(),
        changed_paths: changed_paths.to_vec(),
        matched_rule_count,
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

fn build_paginated_diagnostics(
    problems: &[Problem],
    detail: DiagnosticDetail,
    requested_page: usize,
    page_size: usize,
) -> PaginatedDiagnostics {
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
                &right.message,
            ))
    });

    let total_count = diagnostics.len();
    let total_pages = usize::max(1, total_count.div_ceil(page_size));
    let page = requested_page.min(total_pages);
    let has_next_page = page < total_pages;
    let next_page = has_next_page.then_some(page + 1);

    let mut counts_by_type = BTreeMap::new();
    let mut counts_by_rule = BTreeMap::new();
    for problem in &diagnostics {
        *counts_by_type
            .entry(problem.problem_type.clone())
            .or_insert(0usize) += 1;
        *counts_by_rule
            .entry(problem.rule_id.clone())
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

    let items = if detail == DiagnosticDetail::Summary || diagnostics.is_empty() {
        Vec::new()
    } else {
        let start = (page - 1) * page_size;
        let end = usize::min(start + page_size, total_count);
        diagnostics[start..end]
            .iter()
            .enumerate()
            .map(|(offset, problem)| {
                let index = start + offset + 1;
                DiagnosticItem {
                    diagnostic_id: format!("d{index:03}"),
                    problem_type: problem.problem_type.clone(),
                    path: problem.path.clone(),
                    rule_id: problem.rule_id.clone(),
                    required_mode: problem.required_mode.clone(),
                    failure_reason: problem.failure_reason.clone(),
                    suggested_action: problem.suggested_action.clone(),
                    rule_source: (detail == DiagnosticDetail::Full)
                        .then(|| problem.rule_source.clone())
                        .flatten(),
                    trigger_paths: if detail == DiagnosticDetail::Full {
                        problem.trigger_paths.clone()
                    } else {
                        Vec::new()
                    },
                }
            })
            .collect()
    };

    let summary = ReportSummary {
        total_count,
        counts_by_type,
        top_rules,
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

pub fn build_sarif_log(
    problems: &[Problem],
    changed_paths: &[String],
    matched_rule_count: usize,
    mode: LintMode,
) -> SarifLog {
    let level = sarif_level(mode).to_string();

    SarifLog {
        schema: SARIF_SCHEMA_URI.into(),
        version: "2.1.0".into(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: env!("CARGO_PKG_NAME").into(),
                    version: Some(env!("CARGO_PKG_VERSION").into()),
                    information_uri: None,
                    rules: sarif_rules(mode),
                },
            },
            results: problems
                .iter()
                .map(|problem| SarifResult {
                    rule_id: sarif_rule_id(problem).into(),
                    level: level.clone(),
                    message: SarifMessage {
                        text: problem.message.clone(),
                    },
                    locations: vec![SarifLocation {
                        physical_location: SarifPhysicalLocation {
                            artifact_location: SarifArtifactLocation {
                                uri: problem.path.clone(),
                            },
                        },
                    }],
                    properties: Some(SarifResultProperties {
                        problem_type: problem.problem_type.clone(),
                    }),
                })
                .collect(),
            properties: Some(SarifProperties {
                status: if problems.is_empty() {
                    "ok".into()
                } else {
                    "fail".into()
                },
                changed_paths: changed_paths.to_vec(),
                matched_rule_count,
            }),
        }],
    }
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
                text: "A touched key document is missing required review metadata.".into(),
            },
            default_configuration: SarifDefaultConfiguration { level },
            help_uri: None,
        },
    ]
}

fn sarif_rule_id(problem: &Problem) -> &'static str {
    match problem.problem_type.as_str() {
        "missing-review" => "missing-review",
        "missing-metadata" => METADATA_RULE_ID,
        _ => "unknown-problem",
    }
}

fn sarif_level(mode: LintMode) -> &'static str {
    match mode {
        LintMode::Enforce => "error",
        LintMode::Warn => "warning",
    }
}

fn emit_text_problems(report: &Report, mode: LintMode) {
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
        "Summary: total={}, counts_by_type={}, top_rules={}, page={}/{}, page_size={}",
        report.total_count,
        format_counts_by_type(&report.summary.counts_by_type),
        format_top_rules(&report.summary.top_rules),
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

fn emit_annotations(problems: &[Problem], mode: LintMode) {
    if std::env::var_os("GITHUB_ACTIONS").is_none() {
        return;
    }

    let level = match mode {
        LintMode::Enforce => "error",
        LintMode::Warn => "warning",
    };

    for problem in problems {
        println!("::{level} file={}::{}", problem.path, problem.message);
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::{DiagnosticDetail, LintMode};

    use super::{Problem, SARIF_SCHEMA_URI, build_report, build_sarif_log};

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
            "missing review".into(),
        )
    }

    #[test]
    fn build_report_marks_empty_run_as_ok() {
        let report = build_report(&[], &[], 0, DiagnosticDetail::Compact, 1, 5);
        assert_eq!(report.status, "ok");
        assert!(report.items.is_empty());
        assert_eq!(report.total_pages, 1);
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
    fn build_report_sorts_diagnostics_stably() {
        let problems = vec![
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
        ];

        let report = build_report(&problems, &[], 1, DiagnosticDetail::Compact, 1, 5);
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
    fn build_sarif_log_emits_valid_top_level_shape_for_empty_results() {
        let sarif = build_sarif_log(&[], &[], 0, LintMode::Warn);
        assert_eq!(sarif.schema, SARIF_SCHEMA_URI);
        assert_eq!(sarif.version, "2.1.0");
        assert_eq!(sarif.runs.len(), 1);
        assert!(sarif.runs[0].results.is_empty());
        assert_eq!(sarif.runs[0].tool.driver.rules.len(), 2);
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
            run.properties.as_ref().map(|p| p.changed_paths.clone()),
            Some(vec!["src/index.ts".into()])
        );
        assert_eq!(
            run.tool.driver.rules[0].default_configuration.level,
            "error"
        );
    }
}
