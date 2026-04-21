use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use miette::{IntoDiagnostic, Result, bail, miette};
use serde::Deserialize;
use yaml_serde::Value;

pub const DOC_ROOT_DIR: &str = ".docpact";
pub const CONFIG_FILE: &str = ".docpact/config.yaml";
pub const SUPPORTED_REQUIRED_DOC_MODES: &[&str] = &[
    "review_or_update",
    "metadata_refresh_required",
    "body_update_required",
    "must_exist",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpactLayout {
    Workspace,
    Repo,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ConfigLayout {
    Workspace,
    Repo,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct ConfigFile {
    layout: ConfigLayout,
    #[serde(default)]
    coverage: CoverageConfig,
    #[serde(default)]
    rules: Vec<Rule>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct CoverageConfig {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Rule {
    pub id: String,
    pub scope: String,
    pub repo: String,
    #[serde(default)]
    pub triggers: Vec<Trigger>,
    #[serde(rename = "requiredDocs", default)]
    pub required_docs: Vec<RequiredDoc>,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Trigger {
    pub path: String,
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RequiredDoc {
    pub path: String,
    pub mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpactFileDescriptor {
    pub abs_path: PathBuf,
    pub rel_path: String,
    pub base_dir: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedRule {
    pub source: String,
    pub base_dir: String,
    pub rule: Rule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedCoverageConfig {
    pub source: String,
    pub base_dir: String,
    pub coverage: CoverageConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigValidationProblem {
    pub source: String,
    pub rule_id: Option<String>,
    pub message: String,
}

pub fn normalize_path(value: &str) -> String {
    let replaced = value.replace('\\', "/");
    let trimmed = replaced.trim_start_matches("./");
    let mut output = String::with_capacity(trimmed.len());
    let mut previous_was_slash = false;

    for ch in trimmed.chars() {
        if ch == '/' {
            if !previous_was_slash {
                output.push(ch);
            }
            previous_was_slash = true;
        } else {
            previous_was_slash = false;
            output.push(ch);
        }
    }

    output
}

pub fn path_relative_to(root_dir: &Path, path: &Path) -> String {
    match path.strip_prefix(root_dir) {
        Ok(relative) => normalize_path(&relative.to_string_lossy()),
        Err(_) => normalize_path(&path.to_string_lossy()),
    }
}

fn layout_from_config(layout: ConfigLayout) -> ImpactLayout {
    match layout {
        ConfigLayout::Workspace => ImpactLayout::Workspace,
        ConfigLayout::Repo => ImpactLayout::Repo,
    }
}

pub fn detect_impact_layout(
    root_dir: &Path,
    config_override: Option<&Path>,
) -> Result<ImpactLayout> {
    let config_path = match config_override {
        Some(path) => path.to_path_buf(),
        None => root_dir.join(CONFIG_FILE),
    };

    if !config_path.exists() {
        return Ok(ImpactLayout::None);
    }

    let rel_path = path_relative_to(root_dir, &config_path);
    let parsed = load_config_file(&config_path, &rel_path)?;
    Ok(layout_from_config(parsed.layout))
}

pub fn parse_yaml_value(text: &str, source_label: &str) -> Result<Value> {
    yaml_serde::from_str::<Value>(text)
        .map_err(|error| miette!("{source_label} is not valid YAML for docpact. {}", error))
}

pub fn load_yaml_value(abs_path: &Path, source_label: &str) -> Result<Value> {
    let text = fs::read_to_string(abs_path).into_diagnostic()?;
    parse_yaml_value(&text, source_label)
}

fn load_config_file(abs_path: &Path, source_label: &str) -> Result<ConfigFile> {
    let text = fs::read_to_string(abs_path).into_diagnostic()?;
    yaml_serde::from_str::<ConfigFile>(&text)
        .map_err(|error| miette!("{source_label} is not a valid docpact config file. {error}"))
}

fn descriptor(abs_path: PathBuf, rel_path: String, base_dir: String) -> ImpactFileDescriptor {
    ImpactFileDescriptor {
        abs_path,
        rel_path,
        base_dir,
    }
}

pub fn list_impact_files(
    root_dir: &Path,
    config_override: Option<&Path>,
) -> Result<Vec<ImpactFileDescriptor>> {
    let root_config_path = match config_override {
        Some(path) => path.to_path_buf(),
        None => root_dir.join(CONFIG_FILE),
    };

    if !root_config_path.exists() {
        return Ok(Vec::new());
    }

    let rel_path = path_relative_to(root_dir, &root_config_path);
    let parsed = load_config_file(&root_config_path, &rel_path)?;
    let mut results = vec![descriptor(root_config_path, rel_path, String::new())];

    if parsed.layout == ConfigLayout::Workspace {
        results.extend(list_workspace_repo_files(root_dir)?);
    }

    Ok(results)
}

fn list_workspace_repo_files(root_dir: &Path) -> Result<Vec<ImpactFileDescriptor>> {
    let mut repo_dirs = fs::read_dir(root_dir)
        .into_diagnostic()?
        .collect::<std::result::Result<Vec<_>, _>>()
        .into_diagnostic()?;

    repo_dirs.sort_by_key(|entry| entry.file_name());

    let mut results = Vec::new();

    for entry in repo_dirs {
        let file_type = entry.file_type().into_diagnostic()?;
        if !file_type.is_dir() {
            continue;
        }

        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }

        let repo_config = entry.path().join(CONFIG_FILE);
        if !repo_config.exists() {
            continue;
        }

        results.push(descriptor(
            repo_config,
            normalize_path(&format!("{name}/{CONFIG_FILE}")),
            name.into_owned(),
        ));
    }

    Ok(results)
}

pub fn load_impact_files(
    root_dir: &Path,
    config_override: Option<&Path>,
) -> Result<Vec<LoadedRule>> {
    let mut loaded = Vec::new();

    for descriptor in list_impact_files(root_dir, config_override)? {
        let parsed = load_config_file(&descriptor.abs_path, &descriptor.rel_path)?;
        for rule in parsed.rules {
            loaded.push(LoadedRule {
                source: descriptor.rel_path.clone(),
                base_dir: descriptor.base_dir.clone(),
                rule,
            });
        }
    }

    Ok(loaded)
}

pub fn load_coverage_configs(
    root_dir: &Path,
    config_override: Option<&Path>,
) -> Result<Vec<LoadedCoverageConfig>> {
    let mut loaded = Vec::new();

    for descriptor in list_impact_files(root_dir, config_override)? {
        let parsed = load_config_file(&descriptor.abs_path, &descriptor.rel_path)?;
        loaded.push(LoadedCoverageConfig {
            source: descriptor.rel_path,
            base_dir: descriptor.base_dir,
            coverage: parsed.coverage,
        });
    }

    Ok(loaded)
}

pub fn validate_loaded_rules(loaded_rules: &[LoadedRule]) -> Vec<ConfigValidationProblem> {
    let mut problems = Vec::new();
    let mut rule_sources = BTreeMap::<String, Vec<&LoadedRule>>::new();

    for loaded in loaded_rules {
        if !loaded.rule.id.trim().is_empty() {
            rule_sources
                .entry(loaded.rule.id.clone())
                .or_default()
                .push(loaded);
        }
    }

    for (rule_id, entries) in &rule_sources {
        if entries.len() < 2 {
            continue;
        }

        let all_sources = entries
            .iter()
            .map(|entry| entry.source.as_str())
            .collect::<Vec<_>>();

        for entry in entries {
            let other_sources = all_sources
                .iter()
                .copied()
                .filter(|source| *source != entry.source)
                .collect::<Vec<_>>()
                .join(", ");

            problems.push(ConfigValidationProblem {
                source: entry.source.clone(),
                rule_id: Some(rule_id.clone()),
                message: format!("duplicate rule id `{rule_id}` also found in: {other_sources}"),
            });
        }
    }

    for loaded in loaded_rules {
        validate_rule(loaded, &mut problems);
    }

    problems.sort_by(|left, right| {
        (&left.source, &left.rule_id, &left.message).cmp(&(
            &right.source,
            &right.rule_id,
            &right.message,
        ))
    });
    problems
}

pub fn validate_loaded_coverage_configs(
    loaded_configs: &[LoadedCoverageConfig],
) -> Vec<ConfigValidationProblem> {
    let mut problems = Vec::new();

    for loaded in loaded_configs {
        for (index, pattern) in loaded.coverage.include.iter().enumerate() {
            if let Some(message) = validate_trigger_path(pattern) {
                problems.push(ConfigValidationProblem {
                    source: loaded.source.clone(),
                    rule_id: None,
                    message: format!("coverage.include[{index}] {message}"),
                });
            }
        }

        for (index, pattern) in loaded.coverage.exclude.iter().enumerate() {
            if let Some(message) = validate_trigger_path(pattern) {
                problems.push(ConfigValidationProblem {
                    source: loaded.source.clone(),
                    rule_id: None,
                    message: format!("coverage.exclude[{index}] {message}"),
                });
            }
        }
    }

    problems.sort_by(|left, right| {
        (&left.source, &left.rule_id, &left.message).cmp(&(
            &right.source,
            &right.rule_id,
            &right.message,
        ))
    });

    problems
}

fn validate_rule(loaded: &LoadedRule, problems: &mut Vec<ConfigValidationProblem>) {
    let rule = &loaded.rule;
    let rule_id = if rule.id.trim().is_empty() {
        None
    } else {
        Some(rule.id.clone())
    };

    if rule.id.trim().is_empty() {
        push_problem(problems, loaded, None, "rule id must not be empty".into());
    }

    if rule.triggers.is_empty() {
        push_problem(
            problems,
            loaded,
            rule_id.clone(),
            "rule must define at least one trigger".into(),
        );
    }

    if rule.required_docs.is_empty() {
        push_problem(
            problems,
            loaded,
            rule_id.clone(),
            "rule must define at least one required doc".into(),
        );
    }

    for (index, trigger) in rule.triggers.iter().enumerate() {
        if let Some(message) = validate_trigger_path(&trigger.path) {
            push_problem(
                problems,
                loaded,
                rule_id.clone(),
                format!("triggers[{index}].path {message}"),
            );
        }
    }

    for (index, doc) in rule.required_docs.iter().enumerate() {
        if let Some(message) = validate_required_doc_path(&doc.path) {
            push_problem(
                problems,
                loaded,
                rule_id.clone(),
                format!("requiredDocs[{index}].path {message}"),
            );
        }

        if let Some(mode) = &doc.mode {
            if !SUPPORTED_REQUIRED_DOC_MODES.contains(&mode.as_str()) {
                push_problem(
                    problems,
                    loaded,
                    rule_id.clone(),
                    format!(
                        "requiredDocs[{index}].mode `{mode}` is invalid; expected one of: {}",
                        SUPPORTED_REQUIRED_DOC_MODES.join(", ")
                    ),
                );
            }
        }
    }
}

fn push_problem(
    problems: &mut Vec<ConfigValidationProblem>,
    loaded: &LoadedRule,
    rule_id: Option<String>,
    message: String,
) {
    problems.push(ConfigValidationProblem {
        source: loaded.source.clone(),
        rule_id,
        message,
    });
}

fn validate_trigger_path(path: &str) -> Option<String> {
    validate_repo_relative_path(path, true)
}

fn validate_required_doc_path(path: &str) -> Option<String> {
    let base_message = validate_repo_relative_path(path, false)?;
    Some(base_message)
}

fn validate_repo_relative_path(path: &str, allow_glob: bool) -> Option<String> {
    let trimmed = path.trim();

    if trimmed.is_empty() {
        return Some("must not be empty".into());
    }

    if trimmed.starts_with('/') || looks_like_windows_absolute_path(trimmed) {
        return Some(format!("must be repo-relative, got `{trimmed}`"));
    }

    if trimmed.contains('\\') {
        return Some("must use `/` separators".into());
    }

    let segments = trimmed.split('/').collect::<Vec<_>>();
    if segments.iter().any(|segment| segment.is_empty()) {
        return Some("must not contain empty path segments".into());
    }

    for segment in segments {
        if matches!(segment, "." | "..") {
            return Some("must not contain `.` or `..` segments".into());
        }

        if allow_glob {
            if segment.contains("**") && segment != "**" {
                return Some(format!(
                    "contains malformed glob segment `{segment}`; `**` must occupy a full path segment"
                ));
            }
        } else if segment.contains('*') || segment.contains('?') {
            return Some("must be an exact document path, not a glob".into());
        }
    }

    None
}

fn looks_like_windows_absolute_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3
        && bytes[1] == b':'
        && bytes[0].is_ascii_alphabetic()
        && matches!(bytes[2], b'/' | b'\\')
}

pub fn resolve_rule_path(base_dir: &str, rel_pattern: &str) -> String {
    let rel_pattern = normalize_path(rel_pattern);
    if base_dir.is_empty() {
        return rel_pattern;
    }
    normalize_path(&format!("{base_dir}/{rel_pattern}"))
}

pub fn root_dir_from_option(root: Option<&Path>) -> Result<PathBuf> {
    match root {
        Some(path) => Ok(path.to_path_buf()),
        None => std::env::current_dir().into_diagnostic(),
    }
}

pub fn require_existing_path(path: &Path) -> Result<()> {
    if path.exists() {
        Ok(())
    } else {
        bail!("Path does not exist: {}", path.display())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        CONFIG_FILE, CoverageConfig, DOC_ROOT_DIR, ImpactLayout, LoadedCoverageConfig, LoadedRule,
        RequiredDoc, Rule, Trigger, detect_impact_layout, load_coverage_configs, load_impact_files,
        normalize_path, resolve_rule_path, validate_loaded_coverage_configs, validate_loaded_rules,
    };

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
    fn normalize_path_supports_repo_relative_matching() {
        assert_eq!(
            normalize_path(".\\tiangong-lca-next\\config\\routes.ts"),
            "tiangong-lca-next/config/routes.ts"
        );
    }

    #[test]
    fn detect_impact_layout_distinguishes_workspace_and_repo_roots() {
        let root = temp_dir("docpact-layout");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("doc root dir should exist");

        assert_eq!(
            detect_impact_layout(&root, None).expect("layout should resolve"),
            ImpactLayout::None
        );

        fs::write(
            root.join(CONFIG_FILE),
            "version: 1\nlayout: repo\nlastReviewedAt: 2026-04-18\nlastReviewedCommit: abc\n",
        )
        .expect("repo config");
        assert_eq!(
            detect_impact_layout(&root, None).expect("layout should resolve"),
            ImpactLayout::Repo
        );

        fs::write(
            root.join(CONFIG_FILE),
            "version: 1\nlayout: workspace\nlastReviewedAt: 2026-04-18\nlastReviewedCommit: abc\n",
        )
        .expect("workspace config");
        assert_eq!(
            detect_impact_layout(&root, None).expect("layout should resolve"),
            ImpactLayout::Workspace
        );
    }

    #[test]
    fn load_impact_files_resolves_repo_local_paths() {
        let root = temp_dir("docpact-load");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("root doc dir");
        fs::create_dir_all(root.join(format!("subrepo/{DOC_ROOT_DIR}"))).expect("subrepo doc dir");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: workspace
lastReviewedAt: "2026-04-18"
lastReviewedCommit: "abc"
workspace:
  name: demo
rules:
  - id: root-rule
    scope: workspace
    repo: workspace
    triggers:
      - path: AGENTS.md
        kind: doc-contract
    requiredDocs:
      - path: .docpact/config.yaml
        mode: review_or_update
    reason: root
"#,
        )
        .expect("root config");

        fs::write(
            root.join(format!("subrepo/{CONFIG_FILE}")),
            r#"
version: 1
layout: repo
lastReviewedAt: "2026-04-18"
lastReviewedCommit: "abc"
repo:
  id: subrepo
rules:
  - id: repo-rule
    scope: repo
    repo: subrepo
    triggers:
      - path: src/**
        kind: code
    requiredDocs:
      - path: .docpact/config.yaml
        mode: review_or_update
    reason: repo
"#,
        )
        .expect("subrepo config");

        let loaded = load_impact_files(&root, None).expect("impact files should load");
        assert_eq!(loaded.len(), 2);
        assert_eq!(
            resolve_rule_path("subrepo", ".docpact/config.yaml"),
            "subrepo/.docpact/config.yaml"
        );
    }

    #[test]
    fn strict_validation_reports_duplicate_ids_and_invalid_rule_shapes() {
        let loaded = vec![
            LoadedRule {
                source: ".docpact/config.yaml".into(),
                base_dir: String::new(),
                rule: Rule {
                    id: "duplicate-rule".into(),
                    scope: "repo".into(),
                    repo: "example".into(),
                    triggers: vec![Trigger {
                        path: "src/***".into(),
                        kind: Some("code".into()),
                    }],
                    required_docs: vec![RequiredDoc {
                        path: "docs/*.md".into(),
                        mode: Some("not-a-real-mode".into()),
                    }],
                    reason: "example".into(),
                },
            },
            LoadedRule {
                source: "child/.docpact/config.yaml".into(),
                base_dir: "child".into(),
                rule: Rule {
                    id: "duplicate-rule".into(),
                    scope: "repo".into(),
                    repo: "child".into(),
                    triggers: Vec::new(),
                    required_docs: Vec::new(),
                    reason: "child".into(),
                },
            },
        ];

        let problems = validate_loaded_rules(&loaded);
        let messages = problems
            .iter()
            .map(|problem| format!("{}: {}", problem.source, problem.message))
            .collect::<Vec<_>>();

        assert!(
            messages
                .iter()
                .any(|message| message.contains("duplicate rule id `duplicate-rule`"))
        );
        assert!(messages.iter().any(|message| {
            message.contains("triggers[0].path contains malformed glob segment `***`")
        }));
        assert!(messages.iter().any(|message| {
            message.contains("requiredDocs[0].path must be an exact document path")
        }));
        assert!(
            messages.iter().any(
                |message| message.contains("requiredDocs[0].mode `not-a-real-mode` is invalid")
            )
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("rule must define at least one trigger"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("rule must define at least one required doc"))
        );
    }

    #[test]
    fn load_coverage_configs_resolves_workspace_entries() {
        let root = temp_dir("docpact-load-coverage");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("root doc dir");
        fs::create_dir_all(root.join(format!("subrepo/{DOC_ROOT_DIR}"))).expect("subrepo doc dir");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: workspace
lastReviewedAt: "2026-04-18"
lastReviewedCommit: "abc"
coverage:
  include:
    - docs/**
  exclude:
    - vendor/**
workspace:
  name: demo
rules: []
"#,
        )
        .expect("root config");

        fs::write(
            root.join(format!("subrepo/{CONFIG_FILE}")),
            r#"
version: 1
layout: repo
lastReviewedAt: "2026-04-18"
lastReviewedCommit: "abc"
coverage:
  include:
    - src/**
  exclude:
    - dist/**
repo:
  id: subrepo
rules: []
"#,
        )
        .expect("subrepo config");

        let loaded = load_coverage_configs(&root, None).expect("coverage configs should load");
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].coverage.include, vec!["docs/**".to_string()]);
        assert_eq!(loaded[1].base_dir, "subrepo");
        assert_eq!(loaded[1].coverage.exclude, vec!["dist/**".to_string()]);
    }

    #[test]
    fn strict_validation_reports_invalid_coverage_patterns() {
        let loaded = vec![LoadedCoverageConfig {
            source: ".docpact/config.yaml".into(),
            base_dir: String::new(),
            coverage: CoverageConfig {
                include: vec!["src/***".into()],
                exclude: vec!["..".into()],
            },
        }];

        let problems = validate_loaded_coverage_configs(&loaded);
        let messages = problems
            .iter()
            .map(|problem| problem.message.clone())
            .collect::<Vec<_>>();

        assert!(
            messages
                .iter()
                .any(|message| message.contains("coverage.include[0] contains malformed glob"))
        );
        assert!(
            messages
                .iter()
                .any(|message| message.contains("coverage.exclude[0] must not contain `.` or `..`"))
        );
    }
}
