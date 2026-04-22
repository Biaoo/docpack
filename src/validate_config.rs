use miette::Result;

use crate::AppExit;
use crate::cli::ValidateConfigArgs;
use crate::config::{
    load_catalog_configs, load_coverage_configs, load_doc_inventory_configs,
    load_freshness_configs, load_impact_files, load_ownership_configs, load_routing_configs,
    root_dir_from_option, validate_config_graph, validate_loaded_catalog_configs,
    validate_loaded_coverage_configs, validate_loaded_doc_inventory_configs,
    validate_loaded_freshness_configs, validate_loaded_ownership_configs,
    validate_loaded_routing_configs, validate_loaded_rules, validate_ownership_path_conflicts,
};
use crate::git::get_tracked_paths;

pub fn run(args: ValidateConfigArgs) -> Result<AppExit> {
    let root_dir = root_dir_from_option(args.root.as_deref())?;
    let rules = load_impact_files(&root_dir, args.config.as_deref())?;
    let coverage_configs = load_coverage_configs(&root_dir, args.config.as_deref())?;
    let freshness_configs = load_freshness_configs(&root_dir, args.config.as_deref())?;
    let routing_configs = load_routing_configs(&root_dir, args.config.as_deref())?;
    let doc_inventory_configs = load_doc_inventory_configs(&root_dir, args.config.as_deref())?;
    let catalog_configs = load_catalog_configs(&root_dir, args.config.as_deref())?;
    let ownership_configs = load_ownership_configs(&root_dir, args.config.as_deref())?;

    if !args.strict {
        println!(
            "Docpact config loaded successfully: {} rule(s).",
            rules.len()
        );
        return Ok(AppExit::Success);
    }

    let mut problems = validate_config_graph(&root_dir, args.config.as_deref())?;
    problems.extend(validate_loaded_rules(&rules));
    problems.extend(validate_loaded_coverage_configs(&coverage_configs));
    problems.extend(validate_loaded_freshness_configs(&freshness_configs));
    problems.extend(validate_loaded_routing_configs(&routing_configs));
    problems.extend(validate_loaded_doc_inventory_configs(
        &doc_inventory_configs,
    ));
    problems.extend(validate_loaded_catalog_configs(&catalog_configs));
    problems.extend(validate_loaded_ownership_configs(
        &ownership_configs,
        &catalog_configs,
    ));
    let ownership_domain_count = ownership_configs
        .iter()
        .map(|config| config.ownership.domains.len())
        .sum::<usize>();
    if ownership_domain_count > 0 {
        let tracked_paths = get_tracked_paths(&root_dir)?;
        let analysis = crate::config::analyze_ownership_paths(&tracked_paths, &ownership_configs);
        problems.extend(validate_ownership_path_conflicts(&analysis));
    }
    problems.sort_by(|left, right| {
        (&left.source, &left.rule_id, &left.message).cmp(&(
            &right.source,
            &right.rule_id,
            &right.message,
        ))
    });
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
    use std::process::Command;
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

    #[test]
    fn strict_validate_config_fails_for_tracked_path_ownership_conflicts() {
        let root = temp_dir("docpact-validate-ownership-conflict");
        init_git_repo(&root);
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("doc root should exist");
        fs::create_dir_all(root.join("src/conflict")).expect("src dir should exist");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: repo
catalog:
  repos:
    - id: app
      path: .
    - id: edge
      path: edge
ownership:
  domains:
    - id: app-domain
      paths:
        include:
          - src/**
      ownerRepo: app
    - id: edge-domain
      paths:
        include:
          - src/conflict/**
      ownerRepo: edge
rules: []
"#,
        )
        .expect("config should be written");
        fs::write(root.join("src/conflict/index.ts"), "export const x = 1;\n")
            .expect("tracked file should be written");
        git(&root, &["add", "."]);
        git(&root, &["commit", "-m", "Add conflict sample"]);

        let exit = run(ValidateConfigArgs {
            root: Some(root),
            config: None,
            strict: true,
        })
        .expect("strict validation should execute");

        assert_eq!(exit, AppExit::LintFailure);
    }
}
