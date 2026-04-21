use miette::Result;

use crate::AppExit;
use crate::cli::ValidateConfigArgs;
use crate::config::{load_impact_files, root_dir_from_option, validate_loaded_rules};

pub fn run(args: ValidateConfigArgs) -> Result<AppExit> {
    let root_dir = root_dir_from_option(args.root.as_deref())?;
    let rules = load_impact_files(&root_dir, args.config.as_deref())?;

    if !args.strict {
        println!(
            "Docpact config loaded successfully: {} rule(s).",
            rules.len()
        );
        return Ok(AppExit::Success);
    }

    let problems = validate_loaded_rules(&rules);
    if problems.is_empty() {
        println!(
            "Docpact strict config validation passed: {} rule(s).",
            rules.len()
        );
        return Ok(AppExit::Success);
    }

    println!("Docpact found invalid config definitions:");
    for problem in problems {
        match problem.rule_id {
            Some(rule_id) => {
                println!(
                    "- [invalid-config] {} (rule `{}`): {}",
                    problem.source, rule_id, problem.message
                );
            }
            None => {
                println!("- [invalid-config] {}: {}", problem.source, problem.message);
            }
        }
    }

    Ok(AppExit::LintFailure)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::run;
    use crate::AppExit;
    use crate::cli::ValidateConfigArgs;
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

    #[test]
    fn strict_validate_config_returns_lint_failure_for_invalid_rules() {
        let root = temp_dir("docpact-validate-config");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("doc root should exist");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: repo
lastReviewedAt: "2026-04-21"
lastReviewedCommit: "abc"
repo:
  id: example
rules:
  - id: duplicate-rule
    scope: repo
    repo: example
    triggers:
      - path: src/***
        kind: code
    requiredDocs:
      - path: docs/*.md
        mode: invalid_mode
    reason: example
  - id: duplicate-rule
    scope: repo
    repo: example
    triggers: []
    requiredDocs: []
    reason: second
"#,
        )
        .expect("config should be written");

        let exit = run(ValidateConfigArgs {
            root: Some(root),
            config: None,
            strict: true,
        })
        .expect("strict validation should execute");

        assert_eq!(exit, AppExit::LintFailure);
    }

    #[test]
    fn non_strict_validate_config_remains_compatible() {
        let root = temp_dir("docpact-validate-config-compat");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("doc root should exist");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: repo
lastReviewedAt: "2026-04-21"
lastReviewedCommit: "abc"
repo:
  id: example
rules:
  - id: compatibility-check
    scope: repo
    repo: example
    triggers:
      - path: src/***
        kind: code
    requiredDocs:
      - path: docs/*.md
        mode: invalid_mode
    reason: example
"#,
        )
        .expect("config should be written");

        let exit = run(ValidateConfigArgs {
            root: Some(root),
            config: None,
            strict: false,
        })
        .expect("non-strict validation should execute");

        assert_eq!(exit, AppExit::Success);
    }
}
