use serde::Serialize;

use crate::cli::{LintMode, OutputFormat};

pub const SUPPORTED_REPORTERS: &[&str] = &["text", "json", "sarif", "github"];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Problem {
    #[serde(rename = "type")]
    pub problem_type: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub status: String,
    pub changed_paths: Vec<String>,
    pub problems: Vec<Problem>,
    pub matched_rule_count: usize,
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
    format!("- [{}] {}: {}", problem.problem_type, problem.path, problem.message)
}

pub fn emit_no_changed_paths(format: OutputFormat) {
    match format {
        OutputFormat::Text | OutputFormat::Sarif => {
            println!("AI doc lint: no changed paths to inspect.");
        }
        OutputFormat::Json => {
            let report = Report {
                status: "ok".into(),
                changed_paths: Vec::new(),
                problems: Vec::new(),
                matched_rule_count: 0,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("report should serialize")
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
        OutputFormat::Text | OutputFormat::Sarif => emit_text_problems(problems, mode),
        OutputFormat::Json => {
            let status = if problems.is_empty() { "ok" } else { "fail" };
            let report = Report {
                status: status.into(),
                changed_paths: changed_paths.to_vec(),
                problems: problems.to_vec(),
                matched_rule_count,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&report).expect("report should serialize")
            );
            emit_annotations(problems, mode);
        }
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
