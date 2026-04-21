use std::collections::{BTreeMap, HashSet};

use miette::Result;

use crate::cli::CheckArgs;
use crate::config::root_dir_from_option;
use crate::git::get_changed_paths;
use crate::metadata::build_doc_problems;
use crate::reporters::{Problem, emit_no_changed_paths, emit_problems};
use crate::rules::{ExpectedDoc, MatchedRule, collect_expected_docs, match_rules};
use crate::AppExit;

#[derive(Debug, Clone)]
pub struct CheckRun {
    pub problems: Vec<Problem>,
    pub changed_paths: Vec<String>,
    pub matched_rules: Vec<MatchedRule>,
}

pub fn run(args: CheckArgs) -> Result<AppExit> {
    let run = execute(&args)?;
    if run.changed_paths.is_empty() {
        emit_no_changed_paths(args.format);
        return Ok(AppExit::Success);
    }

    emit_problems(
        &run.problems,
        &run.changed_paths,
        run.matched_rules.len(),
        args.mode,
        args.format,
    );

    if args.mode == crate::cli::LintMode::Enforce && !run.problems.is_empty() {
        Ok(AppExit::LintFailure)
    } else {
        Ok(AppExit::Success)
    }
}

pub fn execute(args: &CheckArgs) -> Result<CheckRun> {
    let root_dir = root_dir_from_option(args.root.as_deref())?;
    let changed_paths = get_changed_paths(&root_dir, args)?;
    if changed_paths.is_empty() {
        return Ok(CheckRun {
            problems: Vec::new(),
            changed_paths,
            matched_rules: Vec::new(),
        });
    }

    let loaded_rules = crate::config::load_impact_files(&root_dir, args.config.as_deref())?;
    let matched_rules = match_rules(&changed_paths, &loaded_rules);
    let expected_docs = collect_expected_docs(&matched_rules);
    let mut problems = build_missing_doc_problems(&changed_paths, &expected_docs);
    problems.extend(build_doc_problems(&root_dir, &changed_paths)?);

    Ok(CheckRun {
        problems,
        changed_paths,
        matched_rules,
    })
}

pub fn build_missing_doc_problems(
    changed_paths: &[String],
    expected_docs: &BTreeMap<String, ExpectedDoc>,
) -> Vec<Problem> {
    let changed = changed_paths.iter().cloned().collect::<HashSet<_>>();
    let mut problems = Vec::new();

    for entry in expected_docs.values() {
        if changed.contains(&entry.path) {
            continue;
        }

        problems.push(Problem::missing_review(
            entry.path.clone(),
            format!(
                "Expected reviewed doc was not touched. Triggered by {} via rule(s): {}",
                join_sorted(&entry.changed_paths),
                join_sorted(&entry.rules)
            ),
        ));
    }

    problems
}

fn join_sorted(values: &std::collections::BTreeSet<String>) -> String {
    values.iter().cloned().collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::cli::{CheckArgs, LintMode, OutputFormat};

    use super::{build_missing_doc_problems, execute};

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{nanos}-{}", std::process::id()));
        fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }

    #[test]
    fn execute_reports_missing_review_and_metadata() {
        let root = temp_dir("ai-doc-lint-check");
        fs::create_dir_all(root.join(".ai-doc-lint")).expect("doc dir");
        fs::create_dir_all(root.join("src")).expect("src dir");

        fs::write(
            root.join(".ai-doc-lint/config.yaml"),
            r#"
version: 1
layout: repo
lastReviewedAt: "2026-04-18"
lastReviewedCommit: "abc"
repo:
  id: example
rules:
  - id: repo-rule
    scope: repo
    repo: example
    triggers:
      - path: src/**
        kind: code
    requiredDocs:
      - path: .ai-doc-lint/config.yaml
        mode: review_or_update
    reason: repo
"#,
        )
        .expect("impact config");

        fs::write(root.join("src/index.ts"), "export const x = 1;\n").expect("source file");
        fs::write(root.join(".ai-doc-lint/quality-rubric.md"), "# Missing frontmatter\n")
            .expect("doc file");

        let run = execute(&CheckArgs {
            root: Some(root),
            config: None,
            base: None,
            head: None,
            files: Some("src/index.ts,.ai-doc-lint/quality-rubric.md".into()),
            staged: false,
            worktree: false,
            merge_base: None,
            mode: LintMode::Warn,
            format: OutputFormat::Text,
        })
        .expect("check should execute");

        assert_eq!(run.problems.len(), 2);
        assert_eq!(run.problems[0].problem_type, "missing-review");
        assert_eq!(run.problems[1].problem_type, "missing-metadata");
    }

    #[test]
    fn build_missing_doc_problems_skips_touched_docs() {
        let mut expected = std::collections::BTreeMap::new();
        expected.insert(
            ".ai-doc-lint/config.yaml".into(),
            crate::rules::ExpectedDoc {
                path: ".ai-doc-lint/config.yaml".into(),
                rules: ["repo-rule".into()].into_iter().collect(),
                changed_paths: ["src/index.ts".into()].into_iter().collect(),
                modes: ["review_or_update".into()].into_iter().collect(),
            },
        );

        let no_problem =
            build_missing_doc_problems(&[".ai-doc-lint/config.yaml".into()], &expected);
        assert!(no_problem.is_empty());
    }
}
