use serde::Serialize;

use crate::cli::{LintMode, OutputFormat};

pub const SUPPORTED_REPORTERS: &[&str] = &["text", "json", "sarif", "github"];
pub const SARIF_SCHEMA_URI: &str =
    "https://docs.oasis-open.org/sarif/sarif/v2.1.0/os/schemas/sarif-schema-2.1.0.json";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Problem {
    #[serde(rename = "type")]
    pub problem_type: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Report {
    pub status: String,
    pub changed_paths: Vec<String>,
    pub problems: Vec<Problem>,
    pub matched_rule_count: usize,
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

impl Problem {
    pub fn missing_metadata(path: String, message: String) -> Self {
        Self {
            problem_type: "missing-metadata".into(),
            path,
            message,
        }
    }

    pub fn missing_review(path: String, message: String) -> Self {
        Self {
            problem_type: "missing-review".into(),
            path,
            message,
        }
    }
}

pub fn format_problem(problem: &Problem) -> String {
    format!(
        "- [{}] {}: {}",
        problem.problem_type, problem.path, problem.message
    )
}

pub fn emit_no_changed_paths(format: OutputFormat) {
    match format {
        OutputFormat::Text => {
            println!("AI doc lint: no changed paths to inspect.");
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&build_report(&[], &[], 0))
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
) {
    match format {
        OutputFormat::Text => emit_text_problems(problems, mode),
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&build_report(
                    problems,
                    changed_paths,
                    matched_rule_count
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
                    mode
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
) -> Report {
    let status = if problems.is_empty() { "ok" } else { "fail" };
    Report {
        status: status.into(),
        changed_paths: changed_paths.to_vec(),
        problems: problems.to_vec(),
        matched_rule_count,
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
                    rule_id: problem.problem_type.clone(),
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
            id: "missing-metadata".into(),
            short_description: SarifMessage {
                text: "A touched key document is missing required review metadata.".into(),
            },
            default_configuration: SarifDefaultConfiguration { level },
            help_uri: None,
        },
    ]
}

fn sarif_level(mode: LintMode) -> &'static str {
    match mode {
        LintMode::Enforce => "error",
        LintMode::Warn => "warning",
    }
}

fn emit_text_problems(problems: &[Problem], mode: LintMode) {
    if problems.is_empty() {
        println!("AI doc lint: no problems found.");
        return;
    }

    let heading = match mode {
        LintMode::Enforce => "AI doc lint found blocking problems:",
        LintMode::Warn => "AI doc lint found warnings:",
    };

    println!("{heading}");
    for problem in problems {
        println!("{}", format_problem(problem));
    }

    emit_annotations(problems, mode);
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
    use crate::cli::LintMode;

    use super::{Problem, SARIF_SCHEMA_URI, build_report, build_sarif_log};

    #[test]
    fn build_report_marks_empty_run_as_ok() {
        let report = build_report(&[], &[], 0);
        assert_eq!(report.status, "ok");
        assert!(report.problems.is_empty());
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
        let problems = vec![Problem::missing_review(
            "docs/api.md".into(),
            "Expected reviewed doc was not touched.".into(),
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
