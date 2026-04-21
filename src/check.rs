use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

use miette::Result;
use yaml_serde::Value;

use crate::AppExit;
use crate::cli::CheckArgs;
use crate::config::{load_yaml_value, parse_yaml_value, root_dir_from_option};
use crate::git::{FileComparison, get_changed_paths, get_file_comparison};
use crate::metadata::{
    build_doc_problems, markdown_body, missing_markdown_review_metadata,
    missing_yaml_review_metadata_from_value, parse_frontmatter_scalar_values,
};
use crate::reporters::{Problem, emit_no_changed_paths, emit_problems};
use crate::rules::{ExpectedDoc, MatchedRule, RequiredDocMode, collect_expected_docs, match_rules};

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
    let mut problems =
        build_required_doc_problems(&root_dir, args, &changed_paths, &expected_docs)?;
    problems.extend(build_doc_problems(&root_dir, &changed_paths)?);

    Ok(CheckRun {
        problems,
        changed_paths,
        matched_rules,
    })
}

pub fn build_required_doc_problems(
    root_dir: &Path,
    args: &CheckArgs,
    changed_paths: &[String],
    expected_docs: &BTreeMap<String, ExpectedDoc>,
) -> Result<Vec<Problem>> {
    let changed = changed_paths.iter().cloned().collect::<HashSet<_>>();
    let mut problems = Vec::new();

    for entry in expected_docs.values() {
        let exists = root_dir.join(&entry.path).exists();
        let touched = changed.contains(&entry.path);
        let mut unmet = Vec::new();

        if entry.modes.contains(&RequiredDocMode::MustExist) && !exists {
            unmet.push("required doc does not exist for mode `must_exist`".to_string());
        }

        let touch_modes = entry
            .modes
            .iter()
            .copied()
            .filter(|mode| *mode != RequiredDocMode::MustExist)
            .collect::<BTreeSet<_>>();

        if !touch_modes.is_empty() {
            if !touched {
                unmet.push(format!(
                    "required doc was not touched for mode(s): {}",
                    join_modes(&touch_modes)
                ));
            } else if !exists {
                unmet.push(format!(
                    "required doc was touched but does not exist after the change for mode(s): {}",
                    join_modes(&touch_modes)
                ));
            } else {
                if entry
                    .modes
                    .contains(&RequiredDocMode::MetadataRefreshRequired)
                    && !metadata_refresh_satisfied(root_dir, args, &entry.path)?
                {
                    unmet.push(
                        "review metadata was not refreshed with a substantive review marker change"
                            .to_string(),
                    );
                }

                if entry.modes.contains(&RequiredDocMode::BodyUpdateRequired)
                    && !body_update_satisfied(root_dir, args, &entry.path)?
                {
                    unmet.push(
                        "doc body was not updated beyond review metadata changes for mode `body_update_required`"
                            .to_string(),
                    );
                }
            }
        }

        if !unmet.is_empty() {
            problems.push(Problem::missing_review(
                entry.path.clone(),
                format!(
                    "{}. Triggered by {} via rule(s): {}",
                    unmet.join("; "),
                    join_sorted(&entry.changed_paths),
                    join_sorted(&entry.rules)
                ),
            ));
        }
    }

    Ok(problems)
}

fn metadata_refresh_satisfied(root_dir: &Path, args: &CheckArgs, rel_path: &str) -> Result<bool> {
    let comparison = get_file_comparison(root_dir, args, rel_path)?;

    if is_markdown_path(rel_path) {
        let current = match comparison.current.as_deref() {
            Some(current) => current,
            None => return Ok(false),
        };
        if !missing_markdown_review_metadata(current).is_empty() {
            return Ok(false);
        }

        let current_values = review_metadata_values_from_markdown(current);
        return Ok(match comparison.previous.as_deref() {
            Some(previous) => review_metadata_values_from_markdown(previous) != current_values,
            None => true,
        });
    }

    if is_yaml_path(rel_path) {
        let current = load_yaml_value(&root_dir.join(rel_path), rel_path)?;
        if !missing_yaml_review_metadata_from_value(&current).is_empty() {
            return Ok(false);
        }

        let current_values = review_metadata_values_from_yaml(&current);
        return Ok(match comparison.previous.as_deref() {
            Some(previous) => match parse_yaml_value(previous, rel_path) {
                Ok(previous) => review_metadata_values_from_yaml(&previous) != current_values,
                Err(_) => true,
            },
            None => true,
        });
    }

    Ok(true)
}

fn body_update_satisfied(root_dir: &Path, args: &CheckArgs, rel_path: &str) -> Result<bool> {
    let comparison = get_file_comparison(root_dir, args, rel_path)?;

    if is_markdown_path(rel_path) {
        let current = match comparison.current.as_deref() {
            Some(current) => current,
            None => return Ok(false),
        };

        return Ok(match comparison.previous.as_deref() {
            Some(previous) => markdown_body(previous) != markdown_body(current),
            None => true,
        });
    }

    if is_yaml_path(rel_path) {
        let current = load_yaml_value(&root_dir.join(rel_path), rel_path)?;
        let current = strip_review_metadata_from_yaml(current);
        return Ok(match comparison.previous.as_deref() {
            Some(previous) => match parse_yaml_value(previous, rel_path) {
                Ok(previous) => strip_review_metadata_from_yaml(previous) != current,
                Err(_) => true,
            },
            None => true,
        });
    }

    Ok(file_contents_changed(&comparison))
}

fn file_contents_changed(comparison: &FileComparison) -> bool {
    comparison.previous != comparison.current
}

fn review_metadata_values_from_markdown(text: &str) -> BTreeMap<String, String> {
    let values = parse_frontmatter_scalar_values(text);
    values
        .into_iter()
        .filter(|(key, _)| matches!(key.as_str(), "lastReviewedAt" | "lastReviewedCommit"))
        .collect()
}

fn review_metadata_values_from_yaml(value: &Value) -> BTreeMap<String, Value> {
    let mapping = match value {
        Value::Mapping(mapping) => mapping,
        _ => return BTreeMap::new(),
    };

    let mut values = BTreeMap::new();
    for key in ["lastReviewedAt", "lastReviewedCommit"] {
        if let Some(value) = mapping.get(Value::String(key.to_string())) {
            values.insert(key.to_string(), value.clone());
        }
    }
    values
}

fn strip_review_metadata_from_yaml(value: Value) -> Value {
    let Value::Mapping(mut mapping) = value else {
        return value;
    };

    for key in ["lastReviewedAt", "lastReviewedCommit"] {
        mapping.remove(Value::String(key.to_string()));
    }

    Value::Mapping(mapping)
}

fn is_markdown_path(path: &str) -> bool {
    path.ends_with(".md")
}

fn is_yaml_path(path: &str) -> bool {
    path.ends_with(".yaml") || path.ends_with(".yml")
}

fn join_sorted(values: &BTreeSet<String>) -> String {
    values.iter().cloned().collect::<Vec<_>>().join(", ")
}

fn join_modes(values: &BTreeSet<RequiredDocMode>) -> String {
    values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::cli::{CheckArgs, LintMode, OutputFormat};
    use crate::rules::{ExpectedDoc, RequiredDocMode};

    use super::{build_required_doc_problems, execute};

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{nanos}-{}", std::process::id()));
        fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }

    fn base_args(root: PathBuf) -> CheckArgs {
        CheckArgs {
            root: Some(root),
            config: None,
            base: None,
            head: None,
            files: None,
            staged: false,
            worktree: false,
            merge_base: None,
            mode: LintMode::Warn,
            format: OutputFormat::Text,
        }
    }

    fn git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .expect("git should run");
        assert!(
            status.success(),
            "git command failed: git {}",
            args.join(" ")
        );
    }

    fn init_git_repo(root: &Path) {
        git(root, &["init"]);
        git(root, &["config", "user.name", "Codex"]);
        git(root, &["config", "user.email", "codex@example.com"]);
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
        fs::write(
            root.join(".ai-doc-lint/quality-rubric.md"),
            "# Missing frontmatter\n",
        )
        .expect("doc file");

        let mut args = base_args(root);
        args.files = Some("src/index.ts,.ai-doc-lint/quality-rubric.md".into());

        let run = execute(&args).expect("check should execute");

        assert_eq!(run.problems.len(), 2);
        assert_eq!(run.problems[0].problem_type, "missing-review");
        assert_eq!(run.problems[1].problem_type, "missing-metadata");
    }

    #[test]
    fn must_exist_mode_allows_untouched_existing_doc() {
        let root = temp_dir("ai-doc-lint-check-must-exist");
        fs::create_dir_all(root.join(".ai-doc-lint")).expect("doc dir");
        fs::write(root.join("README.md"), "# Present\n").expect("readme");

        let mut expected = BTreeMap::new();
        expected.insert(
            "README.md".into(),
            ExpectedDoc {
                path: "README.md".into(),
                rules: ["repo-rule".into()].into_iter().collect(),
                changed_paths: ["src/index.ts".into()].into_iter().collect(),
                modes: [RequiredDocMode::MustExist].into_iter().collect(),
            },
        );

        let problems = build_required_doc_problems(
            &root,
            &base_args(root.clone()),
            &["src/index.ts".into()],
            &expected,
        )
        .expect("mode evaluation should succeed");
        assert!(problems.is_empty());
    }

    #[test]
    fn metadata_refresh_required_fails_when_review_metadata_does_not_change() {
        let root = temp_dir("ai-doc-lint-check-metadata-mode");
        init_git_repo(&root);
        fs::create_dir_all(root.join(".ai-doc-lint")).expect("doc dir");
        fs::create_dir_all(root.join("docs")).expect("docs dir");
        fs::create_dir_all(root.join("src")).expect("src dir");

        fs::write(
            root.join(".ai-doc-lint/config.yaml"),
            r#"
version: 1
layout: repo
lastReviewedAt: "2026-04-21"
lastReviewedCommit: "base"
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
      - path: docs/api.md
        mode: metadata_refresh_required
    reason: repo
"#,
        )
        .expect("config");

        fs::write(root.join("src/index.ts"), "export const x = 1;\n").expect("src");
        fs::write(
            root.join("docs/api.md"),
            r#"---
lastReviewedAt: 2026-04-20
lastReviewedCommit: base
---

# API

Old body
"#,
        )
        .expect("doc");

        git(&root, &["add", "."]);
        git(&root, &["commit", "-m", "base"]);

        fs::write(root.join("src/index.ts"), "export const x = 2;\n").expect("src update");
        fs::write(
            root.join("docs/api.md"),
            r#"---
lastReviewedAt: 2026-04-20
lastReviewedCommit: base
---

# API

New body without metadata refresh
"#,
        )
        .expect("doc update");

        let mut args = base_args(root);
        args.worktree = true;

        let run = execute(&args).expect("check should execute");
        assert_eq!(run.problems.len(), 1);
        assert!(
            run.problems[0]
                .message
                .contains("review metadata was not refreshed")
        );
    }

    #[test]
    fn body_update_required_fails_for_metadata_only_change() {
        let root = temp_dir("ai-doc-lint-check-body-mode");
        init_git_repo(&root);
        fs::create_dir_all(root.join(".ai-doc-lint")).expect("doc dir");
        fs::create_dir_all(root.join("docs")).expect("docs dir");
        fs::create_dir_all(root.join("src")).expect("src dir");

        fs::write(
            root.join(".ai-doc-lint/config.yaml"),
            r#"
version: 1
layout: repo
lastReviewedAt: "2026-04-21"
lastReviewedCommit: "base"
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
      - path: docs/api.md
        mode: body_update_required
    reason: repo
"#,
        )
        .expect("config");

        fs::write(root.join("src/index.ts"), "export const x = 1;\n").expect("src");
        fs::write(
            root.join("docs/api.md"),
            r#"---
lastReviewedAt: 2026-04-20
lastReviewedCommit: base
---

# API

Stable body
"#,
        )
        .expect("doc");

        git(&root, &["add", "."]);
        git(&root, &["commit", "-m", "base"]);

        fs::write(root.join("src/index.ts"), "export const x = 2;\n").expect("src update");
        fs::write(
            root.join("docs/api.md"),
            r#"---
lastReviewedAt: 2026-04-21
lastReviewedCommit: head
---

# API

Stable body
"#,
        )
        .expect("doc update");

        let mut args = base_args(root);
        args.worktree = true;

        let run = execute(&args).expect("check should execute");
        assert_eq!(run.problems.len(), 1);
        assert!(run.problems[0].message.contains("body was not updated"));
    }

    #[test]
    fn body_update_required_passes_when_body_changes() {
        let root = temp_dir("ai-doc-lint-check-body-pass");
        init_git_repo(&root);
        fs::create_dir_all(root.join(".ai-doc-lint")).expect("doc dir");
        fs::create_dir_all(root.join("docs")).expect("docs dir");
        fs::create_dir_all(root.join("src")).expect("src dir");

        fs::write(
            root.join(".ai-doc-lint/config.yaml"),
            r#"
version: 1
layout: repo
lastReviewedAt: "2026-04-21"
lastReviewedCommit: "base"
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
      - path: docs/api.md
        mode: body_update_required
    reason: repo
"#,
        )
        .expect("config");

        fs::write(root.join("src/index.ts"), "export const x = 1;\n").expect("src");
        fs::write(
            root.join("docs/api.md"),
            r#"---
lastReviewedAt: 2026-04-20
lastReviewedCommit: base
---

# API

Stable body
"#,
        )
        .expect("doc");

        git(&root, &["add", "."]);
        git(&root, &["commit", "-m", "base"]);

        fs::write(root.join("src/index.ts"), "export const x = 2;\n").expect("src update");
        fs::write(
            root.join("docs/api.md"),
            r#"---
lastReviewedAt: 2026-04-21
lastReviewedCommit: head
---

# API

Updated body
"#,
        )
        .expect("doc update");

        let mut args = base_args(root);
        args.worktree = true;

        let run = execute(&args).expect("check should execute");
        assert!(run.problems.is_empty());
    }
}
