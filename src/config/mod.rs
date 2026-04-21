use std::fs;
use std::path::{Path, PathBuf};

use miette::{IntoDiagnostic, Result, bail, miette};
use serde::Deserialize;
use yaml_serde::Value;

pub const DOC_ROOT_DIR: &str = ".ai-doc-lint";
pub const CONFIG_FILE: &str = ".ai-doc-lint/config.yaml";

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
    rules: Vec<Rule>,
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

pub fn detect_impact_layout(root_dir: &Path, config_override: Option<&Path>) -> Result<ImpactLayout> {
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
    yaml_serde::from_str::<Value>(text).map_err(|error| {
        miette!(
            "{source_label} is not valid YAML for ai-doc-lint. {}",
            error
        )
    })
}

pub fn load_yaml_value(abs_path: &Path, source_label: &str) -> Result<Value> {
    let text = fs::read_to_string(abs_path).into_diagnostic()?;
    parse_yaml_value(&text, source_label)
}

fn load_config_file(abs_path: &Path, source_label: &str) -> Result<ConfigFile> {
    let text = fs::read_to_string(abs_path).into_diagnostic()?;
    yaml_serde::from_str::<ConfigFile>(&text)
        .map_err(|error| miette!("{source_label} is not a valid ai-doc-lint config file. {error}"))
}

fn descriptor(abs_path: PathBuf, rel_path: String, base_dir: String) -> ImpactFileDescriptor {
    ImpactFileDescriptor {
        abs_path,
        rel_path,
        base_dir,
    }
}

pub fn list_impact_files(root_dir: &Path, config_override: Option<&Path>) -> Result<Vec<ImpactFileDescriptor>> {
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

pub fn load_impact_files(root_dir: &Path, config_override: Option<&Path>) -> Result<Vec<LoadedRule>> {
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

    use super::{CONFIG_FILE, DOC_ROOT_DIR, ImpactLayout, detect_impact_layout, load_impact_files, normalize_path, resolve_rule_path};

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
        let root = temp_dir("ai-doc-lint-layout");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("doc root dir should exist");

        assert_eq!(detect_impact_layout(&root, None).expect("layout should resolve"), ImpactLayout::None);

        fs::write(
            root.join(CONFIG_FILE),
            "version: 1\nlayout: repo\nlastReviewedAt: 2026-04-18\nlastReviewedCommit: abc\n",
        )
        .expect("repo config");
        assert_eq!(detect_impact_layout(&root, None).expect("layout should resolve"), ImpactLayout::Repo);

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
        let root = temp_dir("ai-doc-lint-load");
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
      - path: .ai-doc-lint/config.yaml
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
      - path: .ai-doc-lint/config.yaml
        mode: review_or_update
    reason: repo
"#,
        )
        .expect("subrepo config");

        let loaded = load_impact_files(&root, None).expect("impact files should load");
        assert_eq!(loaded.len(), 2);
        assert_eq!(
            resolve_rule_path("subrepo", ".ai-doc-lint/config.yaml"),
            "subrepo/.ai-doc-lint/config.yaml"
        );
    }
}
