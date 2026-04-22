use std::collections::{BTreeMap, BTreeSet};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OverrideMode {
    Merge,
    Replace,
}

impl OverrideMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Replace => "replace",
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct ConfigFile {
    layout: ConfigLayout,
    #[serde(default)]
    coverage: Option<CoverageConfig>,
    #[serde(default)]
    freshness: Option<FreshnessConfig>,
    #[serde(default)]
    routing: Option<RoutingConfig>,
    #[serde(default)]
    catalog: Option<CatalogConfig>,
    #[serde(default)]
    ownership: Option<OwnershipConfig>,
    #[serde(rename = "docInventory", default)]
    doc_inventory: Option<DocInventoryConfig>,
    #[serde(default)]
    rules: Vec<Rule>,
    #[serde(default)]
    workspace: Option<WorkspaceSection>,
    #[serde(default)]
    inherit: Option<InheritConfig>,
    #[serde(default)]
    overrides: Option<OverridesConfig>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
struct WorkspaceSection {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub profiles: BTreeMap<String, WorkspaceProfile>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
struct WorkspaceProfile {
    #[serde(default)]
    pub coverage: Option<CoverageConfig>,
    #[serde(default)]
    pub freshness: Option<FreshnessConfig>,
    #[serde(default)]
    pub routing: Option<RoutingConfig>,
    #[serde(rename = "docInventory", default)]
    pub doc_inventory: Option<DocInventoryConfig>,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct InheritConfig {
    pub workspace_profile: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
struct OverridesConfig {
    #[serde(default)]
    pub rules: RuleOverrides,
    #[serde(default)]
    pub coverage: Option<ScopedPatternOverride>,
    #[serde(rename = "docInventory", default)]
    pub doc_inventory: Option<ScopedPatternOverride>,
    #[serde(default)]
    pub freshness: Option<FreshnessOverride>,
    #[serde(default)]
    pub routing: Option<RoutingOverride>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CatalogConfig {
    #[serde(default)]
    pub repos: Vec<CatalogRepo>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CatalogRepo {
    pub id: String,
    pub path: String,
    #[serde(rename = "canonicalRepo", default)]
    pub canonical_repo: Option<String>,
    #[serde(rename = "entryDoc", default)]
    pub entry_doc: Option<String>,
    #[serde(rename = "branchPolicyDoc", default)]
    pub branch_policy_doc: Option<String>,
    #[serde(rename = "workflowDocs", default)]
    pub workflow_docs: Vec<String>,
    #[serde(rename = "integrationDocs", default)]
    pub integration_docs: Vec<String>,
    #[serde(rename = "workspaceIntegrationRequired", default)]
    pub workspace_integration_required: bool,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OwnershipConfig {
    #[serde(default)]
    pub domains: Vec<OwnershipDomain>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OwnershipDomain {
    pub id: String,
    pub paths: OwnershipPaths,
    #[serde(rename = "ownerRepo")]
    pub owner_repo: String,
    #[serde(rename = "nonOwnerRepos", default)]
    pub non_owner_repos: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OwnershipPaths {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
struct RuleOverrides {
    #[serde(default)]
    pub add: Vec<Rule>,
    #[serde(default)]
    pub replace: Vec<Rule>,
    #[serde(default)]
    pub disable: Vec<RuleDisable>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct RuleDisable {
    pub id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct ScopedPatternOverride {
    pub mode: OverrideMode,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct FreshnessOverride {
    pub mode: OverrideMode,
    #[serde(default = "default_warn_after_commits")]
    pub warn_after_commits: usize,
    #[serde(default = "default_warn_after_days")]
    pub warn_after_days: usize,
    #[serde(default = "default_critical_after_days")]
    pub critical_after_days: usize,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct RoutingConfig {
    #[serde(default)]
    pub intents: BTreeMap<String, RoutingIntent>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RoutingIntent {
    #[serde(default)]
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct RoutingOverride {
    pub mode: OverrideMode,
    #[serde(default)]
    pub intents: BTreeMap<String, RoutingIntent>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct CoverageConfig {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct FreshnessConfig {
    #[serde(default = "default_warn_after_commits")]
    pub warn_after_commits: usize,
    #[serde(default = "default_warn_after_days")]
    pub warn_after_days: usize,
    #[serde(default = "default_critical_after_days")]
    pub critical_after_days: usize,
}

impl Default for FreshnessConfig {
    fn default() -> Self {
        Self {
            warn_after_commits: default_warn_after_commits(),
            warn_after_days: default_warn_after_days(),
            critical_after_days: default_critical_after_days(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct DocInventoryConfig {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleOriginKind {
    RootLocal,
    RepoLocal,
    WorkspaceProfile,
    OverrideAdd,
    OverrideReplace,
}

impl RuleOriginKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RootLocal => "root-local",
            Self::RepoLocal => "repo-local",
            Self::WorkspaceProfile => "workspace-profile",
            Self::OverrideAdd => "override-add",
            Self::OverrideReplace => "override-replace",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleProvenance {
    pub config_source: String,
    pub origin_kind: RuleOriginKind,
    pub workspace_profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedRule {
    pub source: String,
    pub config_source: String,
    pub base_dir: String,
    pub rule: Rule,
    pub provenance: RuleProvenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigBlockSourceKind {
    Local,
    WorkspaceProfile,
    OverrideMerge,
    OverrideReplace,
    Default,
}

impl ConfigBlockSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::WorkspaceProfile => "workspace-profile",
            Self::OverrideMerge => "override-merge",
            Self::OverrideReplace => "override-replace",
            Self::Default => "default",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockResolution {
    pub origin_kind: ConfigBlockSourceKind,
    pub workspace_profile: Option<String>,
    pub mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedCoverageConfig {
    pub source: String,
    pub base_dir: String,
    pub coverage: CoverageConfig,
    pub resolution: BlockResolution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedFreshnessConfig {
    pub source: String,
    pub base_dir: String,
    pub freshness: FreshnessConfig,
    pub resolution: BlockResolution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedDocInventoryConfig {
    pub source: String,
    pub base_dir: String,
    pub doc_inventory: DocInventoryConfig,
    pub resolution: BlockResolution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedRoutingConfig {
    pub source: String,
    pub base_dir: String,
    pub routing: RoutingConfig,
    pub resolution: BlockResolution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedCatalogConfig {
    pub source: String,
    pub base_dir: String,
    pub catalog: CatalogConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedOwnershipConfig {
    pub source: String,
    pub base_dir: String,
    pub ownership: OwnershipConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipDomainMatch {
    pub source: String,
    pub base_dir: String,
    pub domain_id: String,
    pub owner_repo: String,
    pub non_owner_repos: Vec<String>,
    pub matched_include: String,
    pub static_prefix_len: usize,
    pub wildcard_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipPathAnalysis {
    pub path: String,
    pub matches: Vec<OwnershipDomainMatch>,
    pub selected: OwnershipDomainMatch,
    pub has_conflict: bool,
    pub has_overlap: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipConflict {
    pub path: String,
    pub owner_repos: Vec<String>,
    pub domain_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipOverlap {
    pub path: String,
    pub owner_repo: String,
    pub domain_ids: Vec<String>,
    pub selected_domain_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnershipAnalysis {
    pub paths: Vec<OwnershipPathAnalysis>,
    pub conflicts: Vec<OwnershipConflict>,
    pub overlaps: Vec<OwnershipOverlap>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InheritanceResolution {
    pub workspace_profile: String,
    pub add_count: usize,
    pub replace_count: usize,
    pub disable_count: usize,
    pub disabled_rule_ids: Vec<String>,
    pub coverage_mode: Option<String>,
    pub doc_inventory_mode: Option<String>,
    pub freshness_mode: Option<String>,
    pub routing_mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveConfig {
    pub source: String,
    pub base_dir: String,
    pub rules: Vec<LoadedRule>,
    pub coverage: LoadedCoverageConfig,
    pub freshness: LoadedFreshnessConfig,
    pub routing: LoadedRoutingConfig,
    pub doc_inventory: LoadedDocInventoryConfig,
    pub inheritance: Option<InheritanceResolution>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigValidationProblem {
    pub source: String,
    pub rule_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone)]
struct ParsedImpactFile {
    descriptor: ImpactFileDescriptor,
    parsed: ConfigFile,
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

fn default_warn_after_commits() -> usize {
    50
}

fn default_warn_after_days() -> usize {
    90
}

fn default_critical_after_days() -> usize {
    180
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

fn load_parsed_impact_files(
    root_dir: &Path,
    config_override: Option<&Path>,
) -> Result<Vec<ParsedImpactFile>> {
    let mut parsed_files = Vec::new();

    for descriptor in list_impact_files(root_dir, config_override)? {
        parsed_files.push(ParsedImpactFile {
            parsed: load_config_file(&descriptor.abs_path, &descriptor.rel_path)?,
            descriptor,
        });
    }

    Ok(parsed_files)
}

pub fn load_effective_configs(
    root_dir: &Path,
    config_override: Option<&Path>,
) -> Result<Vec<EffectiveConfig>> {
    let parsed_files = load_parsed_impact_files(root_dir, config_override)?;
    if parsed_files.is_empty() {
        return Ok(Vec::new());
    }

    let root = &parsed_files[0];
    match root.parsed.layout {
        ConfigLayout::Repo => Ok(vec![resolve_repo_local_effective_config(root)?]),
        ConfigLayout::Workspace => {
            let mut effective = vec![resolve_workspace_root_effective_config(root)?];
            for child in parsed_files.iter().skip(1) {
                effective.push(resolve_workspace_child_effective_config(root, child)?);
            }
            Ok(effective)
        }
    }
}

pub fn load_impact_files(
    root_dir: &Path,
    config_override: Option<&Path>,
) -> Result<Vec<LoadedRule>> {
    let mut loaded = Vec::new();

    for effective in load_effective_configs(root_dir, config_override)? {
        loaded.extend(effective.rules);
    }

    Ok(loaded)
}

pub fn load_coverage_configs(
    root_dir: &Path,
    config_override: Option<&Path>,
) -> Result<Vec<LoadedCoverageConfig>> {
    Ok(load_effective_configs(root_dir, config_override)?
        .into_iter()
        .map(|effective| effective.coverage)
        .collect())
}

pub fn load_freshness_configs(
    root_dir: &Path,
    config_override: Option<&Path>,
) -> Result<Vec<LoadedFreshnessConfig>> {
    Ok(load_effective_configs(root_dir, config_override)?
        .into_iter()
        .map(|effective| effective.freshness)
        .collect())
}

pub fn load_routing_configs(
    root_dir: &Path,
    config_override: Option<&Path>,
) -> Result<Vec<LoadedRoutingConfig>> {
    Ok(load_effective_configs(root_dir, config_override)?
        .into_iter()
        .map(|effective| effective.routing)
        .collect())
}

pub fn load_catalog_configs(
    root_dir: &Path,
    config_override: Option<&Path>,
) -> Result<Vec<LoadedCatalogConfig>> {
    Ok(load_parsed_impact_files(root_dir, config_override)?
        .into_iter()
        .map(|parsed| LoadedCatalogConfig {
            source: parsed.descriptor.rel_path,
            base_dir: parsed.descriptor.base_dir,
            catalog: normalize_catalog_config(&parsed.parsed.catalog.unwrap_or_default()),
        })
        .collect())
}

pub fn load_ownership_configs(
    root_dir: &Path,
    config_override: Option<&Path>,
) -> Result<Vec<LoadedOwnershipConfig>> {
    Ok(load_parsed_impact_files(root_dir, config_override)?
        .into_iter()
        .map(|parsed| LoadedOwnershipConfig {
            source: parsed.descriptor.rel_path,
            base_dir: parsed.descriptor.base_dir,
            ownership: normalize_ownership_config(&parsed.parsed.ownership.unwrap_or_default()),
        })
        .collect())
}

pub fn load_doc_inventory_configs(
    root_dir: &Path,
    config_override: Option<&Path>,
) -> Result<Vec<LoadedDocInventoryConfig>> {
    Ok(load_effective_configs(root_dir, config_override)?
        .into_iter()
        .map(|effective| effective.doc_inventory)
        .collect())
}

pub fn validate_config_graph(
    root_dir: &Path,
    config_override: Option<&Path>,
) -> Result<Vec<ConfigValidationProblem>> {
    let parsed_files = load_parsed_impact_files(root_dir, config_override)?;
    Ok(validate_parsed_configs(&parsed_files))
}

fn resolve_repo_local_effective_config(parsed: &ParsedImpactFile) -> Result<EffectiveConfig> {
    if parsed.parsed.inherit.is_some() {
        bail!(
            "{} declares `inherit`, but workspace inheritance is only available when a workspace root config is the active entrypoint",
            parsed.descriptor.rel_path
        );
    }
    if parsed.parsed.overrides.is_some() {
        bail!(
            "{} declares `overrides` without `inherit.workspace_profile`",
            parsed.descriptor.rel_path
        );
    }

    Ok(EffectiveConfig {
        source: parsed.descriptor.rel_path.clone(),
        base_dir: parsed.descriptor.base_dir.clone(),
        rules: parsed
            .parsed
            .rules
            .iter()
            .cloned()
            .map(|rule| LoadedRule {
                source: parsed.descriptor.rel_path.clone(),
                config_source: parsed.descriptor.rel_path.clone(),
                base_dir: parsed.descriptor.base_dir.clone(),
                provenance: RuleProvenance {
                    config_source: parsed.descriptor.rel_path.clone(),
                    origin_kind: if parsed.descriptor.base_dir.is_empty() {
                        RuleOriginKind::RootLocal
                    } else {
                        RuleOriginKind::RepoLocal
                    },
                    workspace_profile: None,
                },
                rule,
            })
            .collect(),
        coverage: LoadedCoverageConfig {
            source: parsed.descriptor.rel_path.clone(),
            base_dir: parsed.descriptor.base_dir.clone(),
            coverage: parsed.parsed.coverage.clone().unwrap_or_default(),
            resolution: BlockResolution {
                origin_kind: if parsed.parsed.coverage.is_some() {
                    ConfigBlockSourceKind::Local
                } else {
                    ConfigBlockSourceKind::Default
                },
                workspace_profile: None,
                mode: None,
            },
        },
        freshness: LoadedFreshnessConfig {
            source: parsed.descriptor.rel_path.clone(),
            base_dir: parsed.descriptor.base_dir.clone(),
            freshness: parsed.parsed.freshness.clone().unwrap_or_default(),
            resolution: BlockResolution {
                origin_kind: if parsed.parsed.freshness.is_some() {
                    ConfigBlockSourceKind::Local
                } else {
                    ConfigBlockSourceKind::Default
                },
                workspace_profile: None,
                mode: None,
            },
        },
        routing: LoadedRoutingConfig {
            source: parsed.descriptor.rel_path.clone(),
            base_dir: parsed.descriptor.base_dir.clone(),
            routing: parsed.parsed.routing.clone().unwrap_or_default(),
            resolution: BlockResolution {
                origin_kind: if parsed.parsed.routing.is_some() {
                    ConfigBlockSourceKind::Local
                } else {
                    ConfigBlockSourceKind::Default
                },
                workspace_profile: None,
                mode: None,
            },
        },
        doc_inventory: LoadedDocInventoryConfig {
            source: parsed.descriptor.rel_path.clone(),
            base_dir: parsed.descriptor.base_dir.clone(),
            doc_inventory: parsed.parsed.doc_inventory.clone().unwrap_or_default(),
            resolution: BlockResolution {
                origin_kind: if parsed.parsed.doc_inventory.is_some() {
                    ConfigBlockSourceKind::Local
                } else {
                    ConfigBlockSourceKind::Default
                },
                workspace_profile: None,
                mode: None,
            },
        },
        inheritance: None,
    })
}

fn resolve_workspace_root_effective_config(root: &ParsedImpactFile) -> Result<EffectiveConfig> {
    if root.parsed.inherit.is_some() {
        bail!(
            "{} must not declare `inherit` because workspace roots cannot inherit from another config",
            root.descriptor.rel_path
        );
    }
    if root.parsed.overrides.is_some() {
        bail!(
            "{} must not declare `overrides` because workspace roots define local config and reusable profiles directly",
            root.descriptor.rel_path
        );
    }

    Ok(EffectiveConfig {
        source: root.descriptor.rel_path.clone(),
        base_dir: root.descriptor.base_dir.clone(),
        rules: root
            .parsed
            .rules
            .iter()
            .cloned()
            .map(|rule| LoadedRule {
                source: root.descriptor.rel_path.clone(),
                config_source: root.descriptor.rel_path.clone(),
                base_dir: root.descriptor.base_dir.clone(),
                provenance: RuleProvenance {
                    config_source: root.descriptor.rel_path.clone(),
                    origin_kind: RuleOriginKind::RootLocal,
                    workspace_profile: None,
                },
                rule,
            })
            .collect(),
        coverage: LoadedCoverageConfig {
            source: root.descriptor.rel_path.clone(),
            base_dir: root.descriptor.base_dir.clone(),
            coverage: root.parsed.coverage.clone().unwrap_or_default(),
            resolution: BlockResolution {
                origin_kind: if root.parsed.coverage.is_some() {
                    ConfigBlockSourceKind::Local
                } else {
                    ConfigBlockSourceKind::Default
                },
                workspace_profile: None,
                mode: None,
            },
        },
        freshness: LoadedFreshnessConfig {
            source: root.descriptor.rel_path.clone(),
            base_dir: root.descriptor.base_dir.clone(),
            freshness: root.parsed.freshness.clone().unwrap_or_default(),
            resolution: BlockResolution {
                origin_kind: if root.parsed.freshness.is_some() {
                    ConfigBlockSourceKind::Local
                } else {
                    ConfigBlockSourceKind::Default
                },
                workspace_profile: None,
                mode: None,
            },
        },
        routing: LoadedRoutingConfig {
            source: root.descriptor.rel_path.clone(),
            base_dir: root.descriptor.base_dir.clone(),
            routing: root.parsed.routing.clone().unwrap_or_default(),
            resolution: BlockResolution {
                origin_kind: if root.parsed.routing.is_some() {
                    ConfigBlockSourceKind::Local
                } else {
                    ConfigBlockSourceKind::Default
                },
                workspace_profile: None,
                mode: None,
            },
        },
        doc_inventory: LoadedDocInventoryConfig {
            source: root.descriptor.rel_path.clone(),
            base_dir: root.descriptor.base_dir.clone(),
            doc_inventory: root.parsed.doc_inventory.clone().unwrap_or_default(),
            resolution: BlockResolution {
                origin_kind: if root.parsed.doc_inventory.is_some() {
                    ConfigBlockSourceKind::Local
                } else {
                    ConfigBlockSourceKind::Default
                },
                workspace_profile: None,
                mode: None,
            },
        },
        inheritance: None,
    })
}

fn resolve_workspace_child_effective_config(
    root: &ParsedImpactFile,
    child: &ParsedImpactFile,
) -> Result<EffectiveConfig> {
    if child.parsed.layout != ConfigLayout::Repo {
        bail!(
            "{} must use `layout: repo` when loaded from a workspace root",
            child.descriptor.rel_path
        );
    }

    let Some(inherit) = child.parsed.inherit.as_ref() else {
        if child.parsed.overrides.is_some() {
            bail!(
                "{} declares `overrides` without `inherit.workspace_profile`",
                child.descriptor.rel_path
            );
        }
        return resolve_repo_local_effective_config(child);
    };

    if !child.parsed.rules.is_empty()
        || child.parsed.coverage.is_some()
        || child.parsed.freshness.is_some()
        || child.parsed.routing.is_some()
        || child.parsed.doc_inventory.is_some()
    {
        bail!(
            "{} uses workspace inheritance and therefore must place all runtime changes under `overrides` instead of top-level `rules`, `coverage`, `freshness`, `routing`, or `docInventory`",
            child.descriptor.rel_path
        );
    }

    let workspace = root.parsed.workspace.as_ref().ok_or_else(|| {
        miette!(
            "{} is a workspace config but does not define a `workspace` block",
            root.descriptor.rel_path
        )
    })?;
    let profile = workspace
        .profiles
        .get(&inherit.workspace_profile)
        .ok_or_else(|| {
            miette!(
                "{} references missing workspace profile `{}` from {}",
                child.descriptor.rel_path,
                inherit.workspace_profile,
                root.descriptor.rel_path
            )
        })?;

    let overrides = child.parsed.overrides.clone().unwrap_or_default();
    let resolved_rules = resolve_inherited_rules(
        root,
        child,
        &inherit.workspace_profile,
        profile,
        &overrides.rules,
    )?;
    let (coverage, coverage_resolution) = resolve_scoped_patterns(
        profile.coverage.as_ref(),
        overrides.coverage.as_ref(),
        &inherit.workspace_profile,
    );
    let (routing, routing_resolution) = resolve_routing(
        profile.routing.as_ref(),
        overrides.routing.as_ref(),
        &inherit.workspace_profile,
    );
    let (doc_inventory, doc_inventory_resolution) = resolve_doc_inventory(
        profile.doc_inventory.as_ref(),
        overrides.doc_inventory.as_ref(),
        &inherit.workspace_profile,
    );
    let (freshness, freshness_resolution) = resolve_freshness(
        profile.freshness.as_ref(),
        overrides.freshness.as_ref(),
        &inherit.workspace_profile,
    )?;

    Ok(EffectiveConfig {
        source: child.descriptor.rel_path.clone(),
        base_dir: child.descriptor.base_dir.clone(),
        rules: resolved_rules,
        coverage: LoadedCoverageConfig {
            source: child.descriptor.rel_path.clone(),
            base_dir: child.descriptor.base_dir.clone(),
            coverage,
            resolution: coverage_resolution,
        },
        freshness: LoadedFreshnessConfig {
            source: child.descriptor.rel_path.clone(),
            base_dir: child.descriptor.base_dir.clone(),
            freshness,
            resolution: freshness_resolution,
        },
        routing: LoadedRoutingConfig {
            source: child.descriptor.rel_path.clone(),
            base_dir: child.descriptor.base_dir.clone(),
            routing,
            resolution: routing_resolution,
        },
        doc_inventory: LoadedDocInventoryConfig {
            source: child.descriptor.rel_path.clone(),
            base_dir: child.descriptor.base_dir.clone(),
            doc_inventory,
            resolution: doc_inventory_resolution,
        },
        inheritance: Some(InheritanceResolution {
            workspace_profile: inherit.workspace_profile.clone(),
            add_count: overrides.rules.add.len(),
            replace_count: overrides.rules.replace.len(),
            disable_count: overrides.rules.disable.len(),
            disabled_rule_ids: overrides
                .rules
                .disable
                .iter()
                .map(|rule| rule.id.clone())
                .collect(),
            coverage_mode: overrides
                .coverage
                .as_ref()
                .map(|override_block| override_block.mode.as_str().to_string()),
            doc_inventory_mode: overrides
                .doc_inventory
                .as_ref()
                .map(|override_block| override_block.mode.as_str().to_string()),
            freshness_mode: overrides
                .freshness
                .as_ref()
                .map(|override_block| override_block.mode.as_str().to_string()),
            routing_mode: overrides
                .routing
                .as_ref()
                .map(|override_block| override_block.mode.as_str().to_string()),
        }),
    })
}

fn resolve_inherited_rules(
    root: &ParsedImpactFile,
    child: &ParsedImpactFile,
    profile_name: &str,
    profile: &WorkspaceProfile,
    overrides: &RuleOverrides,
) -> Result<Vec<LoadedRule>> {
    let mut ordered = Vec::<(String, LoadedRule)>::new();

    for rule in &profile.rules {
        let source =
            workspace_profile_rule_source(&root.descriptor.rel_path, profile_name, &rule.id);
        ordered.push((
            rule.id.clone(),
            LoadedRule {
                source,
                config_source: child.descriptor.rel_path.clone(),
                base_dir: child.descriptor.base_dir.clone(),
                provenance: RuleProvenance {
                    config_source: root.descriptor.rel_path.clone(),
                    origin_kind: RuleOriginKind::WorkspaceProfile,
                    workspace_profile: Some(profile_name.to_string()),
                },
                rule: rule.clone(),
            },
        ));
    }

    for rule in &overrides.replace {
        let source = override_rule_source(&child.descriptor.rel_path, "replace", &rule.id);
        let Some(position) = ordered.iter().position(|(id, _)| id == &rule.id) else {
            bail!(
                "{} tries to replace inherited rule `{}` but that rule does not exist in workspace profile `{}`",
                child.descriptor.rel_path,
                rule.id,
                profile_name
            );
        };
        ordered[position] = (
            rule.id.clone(),
            LoadedRule {
                source,
                config_source: child.descriptor.rel_path.clone(),
                base_dir: child.descriptor.base_dir.clone(),
                provenance: RuleProvenance {
                    config_source: child.descriptor.rel_path.clone(),
                    origin_kind: RuleOriginKind::OverrideReplace,
                    workspace_profile: Some(profile_name.to_string()),
                },
                rule: rule.clone(),
            },
        );
    }

    let profile_rule_ids = profile
        .rules
        .iter()
        .map(|rule| rule.id.as_str())
        .collect::<BTreeSet<_>>();

    for disabled in &overrides.disable {
        if !profile_rule_ids.contains(disabled.id.as_str()) {
            bail!(
                "{} tries to disable `{}` but that rule is not inherited from workspace profile `{}`",
                child.descriptor.rel_path,
                disabled.id,
                profile_name
            );
        }
        let Some(position) = ordered.iter().position(|(id, _)| id == &disabled.id) else {
            bail!(
                "{} tries to disable `{}` but that rule is not active after replacements",
                child.descriptor.rel_path,
                disabled.id
            );
        };
        ordered.remove(position);
    }

    for rule in &overrides.add {
        if ordered.iter().any(|(id, _)| id == &rule.id) {
            bail!(
                "{} tries to add rule `{}` but that id is already active in inherited config",
                child.descriptor.rel_path,
                rule.id
            );
        }
        let source = override_rule_source(&child.descriptor.rel_path, "add", &rule.id);
        ordered.push((
            rule.id.clone(),
            LoadedRule {
                source,
                config_source: child.descriptor.rel_path.clone(),
                base_dir: child.descriptor.base_dir.clone(),
                provenance: RuleProvenance {
                    config_source: child.descriptor.rel_path.clone(),
                    origin_kind: RuleOriginKind::OverrideAdd,
                    workspace_profile: Some(profile_name.to_string()),
                },
                rule: rule.clone(),
            },
        ));
    }

    Ok(ordered.into_iter().map(|(_, loaded)| loaded).collect())
}

fn resolve_scoped_patterns(
    inherited: Option<&CoverageConfig>,
    override_block: Option<&ScopedPatternOverride>,
    profile_name: &str,
) -> (CoverageConfig, BlockResolution) {
    match override_block {
        Some(override_block) if override_block.mode == OverrideMode::Replace => (
            CoverageConfig {
                include: sorted_unique_patterns(&override_block.include),
                exclude: sorted_unique_patterns(&override_block.exclude),
            },
            BlockResolution {
                origin_kind: ConfigBlockSourceKind::OverrideReplace,
                workspace_profile: Some(profile_name.to_string()),
                mode: Some(override_block.mode.as_str().to_string()),
            },
        ),
        Some(override_block) => {
            let base = inherited.cloned().unwrap_or_default();
            (
                CoverageConfig {
                    include: merge_patterns(&base.include, &override_block.include),
                    exclude: merge_patterns(&base.exclude, &override_block.exclude),
                },
                BlockResolution {
                    origin_kind: ConfigBlockSourceKind::OverrideMerge,
                    workspace_profile: Some(profile_name.to_string()),
                    mode: Some(override_block.mode.as_str().to_string()),
                },
            )
        }
        None => match inherited {
            Some(inherited) => (
                inherited.clone(),
                BlockResolution {
                    origin_kind: ConfigBlockSourceKind::WorkspaceProfile,
                    workspace_profile: Some(profile_name.to_string()),
                    mode: None,
                },
            ),
            None => (
                CoverageConfig::default(),
                BlockResolution {
                    origin_kind: ConfigBlockSourceKind::Default,
                    workspace_profile: Some(profile_name.to_string()),
                    mode: None,
                },
            ),
        },
    }
}

fn resolve_doc_inventory(
    inherited: Option<&DocInventoryConfig>,
    override_block: Option<&ScopedPatternOverride>,
    profile_name: &str,
) -> (DocInventoryConfig, BlockResolution) {
    match override_block {
        Some(override_block) if override_block.mode == OverrideMode::Replace => (
            DocInventoryConfig {
                include: sorted_unique_patterns(&override_block.include),
                exclude: sorted_unique_patterns(&override_block.exclude),
            },
            BlockResolution {
                origin_kind: ConfigBlockSourceKind::OverrideReplace,
                workspace_profile: Some(profile_name.to_string()),
                mode: Some(override_block.mode.as_str().to_string()),
            },
        ),
        Some(override_block) => {
            let base = inherited.cloned().unwrap_or_default();
            (
                DocInventoryConfig {
                    include: merge_patterns(&base.include, &override_block.include),
                    exclude: merge_patterns(&base.exclude, &override_block.exclude),
                },
                BlockResolution {
                    origin_kind: ConfigBlockSourceKind::OverrideMerge,
                    workspace_profile: Some(profile_name.to_string()),
                    mode: Some(override_block.mode.as_str().to_string()),
                },
            )
        }
        None => match inherited {
            Some(inherited) => (
                inherited.clone(),
                BlockResolution {
                    origin_kind: ConfigBlockSourceKind::WorkspaceProfile,
                    workspace_profile: Some(profile_name.to_string()),
                    mode: None,
                },
            ),
            None => (
                DocInventoryConfig::default(),
                BlockResolution {
                    origin_kind: ConfigBlockSourceKind::Default,
                    workspace_profile: Some(profile_name.to_string()),
                    mode: None,
                },
            ),
        },
    }
}

fn resolve_freshness(
    inherited: Option<&FreshnessConfig>,
    override_block: Option<&FreshnessOverride>,
    profile_name: &str,
) -> Result<(FreshnessConfig, BlockResolution)> {
    match override_block {
        Some(override_block) if override_block.mode != OverrideMode::Replace => bail!(
            "freshness overrides only support `mode: replace` in the first inheritance release"
        ),
        Some(override_block) => Ok((
            FreshnessConfig {
                warn_after_commits: override_block.warn_after_commits,
                warn_after_days: override_block.warn_after_days,
                critical_after_days: override_block.critical_after_days,
            },
            BlockResolution {
                origin_kind: ConfigBlockSourceKind::OverrideReplace,
                workspace_profile: Some(profile_name.to_string()),
                mode: Some(override_block.mode.as_str().to_string()),
            },
        )),
        None => match inherited {
            Some(inherited) => Ok((
                inherited.clone(),
                BlockResolution {
                    origin_kind: ConfigBlockSourceKind::WorkspaceProfile,
                    workspace_profile: Some(profile_name.to_string()),
                    mode: None,
                },
            )),
            None => Ok((
                FreshnessConfig::default(),
                BlockResolution {
                    origin_kind: ConfigBlockSourceKind::Default,
                    workspace_profile: Some(profile_name.to_string()),
                    mode: None,
                },
            )),
        },
    }
}

fn resolve_routing(
    inherited: Option<&RoutingConfig>,
    override_block: Option<&RoutingOverride>,
    profile_name: &str,
) -> (RoutingConfig, BlockResolution) {
    match override_block {
        Some(override_block) if override_block.mode == OverrideMode::Replace => (
            RoutingConfig {
                intents: normalize_routing_intents(&override_block.intents),
            },
            BlockResolution {
                origin_kind: ConfigBlockSourceKind::OverrideReplace,
                workspace_profile: Some(profile_name.to_string()),
                mode: Some(override_block.mode.as_str().to_string()),
            },
        ),
        Some(override_block) => {
            let mut merged = inherited.cloned().unwrap_or_default();
            for (alias, intent) in normalize_routing_intents(&override_block.intents) {
                merged.intents.insert(alias, intent);
            }
            (
                merged,
                BlockResolution {
                    origin_kind: ConfigBlockSourceKind::OverrideMerge,
                    workspace_profile: Some(profile_name.to_string()),
                    mode: Some(override_block.mode.as_str().to_string()),
                },
            )
        }
        None => match inherited {
            Some(inherited) => (
                RoutingConfig {
                    intents: normalize_routing_intents(&inherited.intents),
                },
                BlockResolution {
                    origin_kind: ConfigBlockSourceKind::WorkspaceProfile,
                    workspace_profile: Some(profile_name.to_string()),
                    mode: None,
                },
            ),
            None => (
                RoutingConfig::default(),
                BlockResolution {
                    origin_kind: ConfigBlockSourceKind::Default,
                    workspace_profile: Some(profile_name.to_string()),
                    mode: None,
                },
            ),
        },
    }
}

fn normalize_catalog_config(catalog: &CatalogConfig) -> CatalogConfig {
    CatalogConfig {
        repos: catalog.repos.iter().map(normalize_catalog_repo).collect(),
    }
}

fn normalize_catalog_repo(repo: &CatalogRepo) -> CatalogRepo {
    CatalogRepo {
        id: repo.id.trim().to_string(),
        path: normalize_catalog_repo_path(&repo.path),
        canonical_repo: repo
            .canonical_repo
            .as_ref()
            .map(|value| value.trim().to_string()),
        entry_doc: repo
            .entry_doc
            .as_ref()
            .map(|value| normalize_path(value.trim())),
        branch_policy_doc: repo
            .branch_policy_doc
            .as_ref()
            .map(|value| normalize_path(value.trim())),
        workflow_docs: normalize_exact_doc_pointer_list(&repo.workflow_docs),
        integration_docs: normalize_exact_doc_pointer_list(&repo.integration_docs),
        workspace_integration_required: repo.workspace_integration_required,
    }
}

fn normalize_ownership_config(ownership: &OwnershipConfig) -> OwnershipConfig {
    OwnershipConfig {
        domains: ownership
            .domains
            .iter()
            .map(normalize_ownership_domain)
            .collect(),
    }
}

fn normalize_ownership_domain(domain: &OwnershipDomain) -> OwnershipDomain {
    OwnershipDomain {
        id: domain.id.trim().to_string(),
        paths: OwnershipPaths {
            include: sorted_unique_patterns(&domain.paths.include),
            exclude: sorted_unique_patterns(&domain.paths.exclude),
        },
        owner_repo: domain.owner_repo.trim().to_string(),
        non_owner_repos: domain
            .non_owner_repos
            .iter()
            .map(|repo| repo.trim().to_string())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    }
}

fn normalize_catalog_repo_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed == "." {
        ".".into()
    } else {
        normalize_path(trimmed)
    }
}

fn normalize_exact_doc_pointer_list(paths: &[String]) -> Vec<String> {
    paths
        .iter()
        .map(|path| normalize_path(path.trim()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn normalize_routing_intents(
    intents: &BTreeMap<String, RoutingIntent>,
) -> BTreeMap<String, RoutingIntent> {
    intents
        .iter()
        .map(|(alias, intent)| {
            (
                alias.trim().to_string(),
                RoutingIntent {
                    paths: sorted_unique_patterns(&intent.paths),
                },
            )
        })
        .collect()
}

fn sorted_unique_patterns(patterns: &[String]) -> Vec<String> {
    patterns
        .iter()
        .map(|pattern| normalize_path(pattern))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn merge_patterns(left: &[String], right: &[String]) -> Vec<String> {
    left.iter()
        .chain(right.iter())
        .map(|pattern| normalize_path(pattern))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn workspace_profile_rule_source(root_source: &str, profile_name: &str, rule_id: &str) -> String {
    format!("{root_source}#workspace.profiles.{profile_name}.rules.{rule_id}")
}

fn override_rule_source(config_source: &str, operation: &str, rule_id: &str) -> String {
    format!("{config_source}#overrides.rules.{operation}.{rule_id}")
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

pub fn validate_loaded_freshness_configs(
    loaded_configs: &[LoadedFreshnessConfig],
) -> Vec<ConfigValidationProblem> {
    let mut problems = Vec::new();

    for loaded in loaded_configs {
        if loaded.freshness.warn_after_commits == 0 {
            problems.push(ConfigValidationProblem {
                source: loaded.source.clone(),
                rule_id: None,
                message: "freshness.warn_after_commits must be greater than 0".into(),
            });
        }

        if loaded.freshness.warn_after_days == 0 {
            problems.push(ConfigValidationProblem {
                source: loaded.source.clone(),
                rule_id: None,
                message: "freshness.warn_after_days must be greater than 0".into(),
            });
        }

        if loaded.freshness.critical_after_days == 0 {
            problems.push(ConfigValidationProblem {
                source: loaded.source.clone(),
                rule_id: None,
                message: "freshness.critical_after_days must be greater than 0".into(),
            });
        }

        if loaded.freshness.critical_after_days < loaded.freshness.warn_after_days {
            problems.push(ConfigValidationProblem {
                source: loaded.source.clone(),
                rule_id: None,
                message:
                    "freshness.critical_after_days must be greater than or equal to freshness.warn_after_days"
                        .into(),
            });
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

pub fn validate_loaded_doc_inventory_configs(
    loaded_configs: &[LoadedDocInventoryConfig],
) -> Vec<ConfigValidationProblem> {
    let mut problems = Vec::new();

    for loaded in loaded_configs {
        for (index, pattern) in loaded.doc_inventory.include.iter().enumerate() {
            if let Some(message) = validate_trigger_path(pattern) {
                problems.push(ConfigValidationProblem {
                    source: loaded.source.clone(),
                    rule_id: None,
                    message: format!("docInventory.include[{index}] {message}"),
                });
            }
        }

        for (index, pattern) in loaded.doc_inventory.exclude.iter().enumerate() {
            if let Some(message) = validate_trigger_path(pattern) {
                problems.push(ConfigValidationProblem {
                    source: loaded.source.clone(),
                    rule_id: None,
                    message: format!("docInventory.exclude[{index}] {message}"),
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

pub fn validate_loaded_routing_configs(
    loaded_configs: &[LoadedRoutingConfig],
) -> Vec<ConfigValidationProblem> {
    let mut problems = Vec::new();
    let mut aliases = BTreeMap::<String, Vec<&LoadedRoutingConfig>>::new();

    for loaded in loaded_configs {
        for (alias, intent) in &loaded.routing.intents {
            aliases.entry(alias.clone()).or_default().push(loaded);

            if intent.paths.is_empty() {
                problems.push(ConfigValidationProblem {
                    source: loaded.source.clone(),
                    rule_id: None,
                    message: format!("routing.intents.{alias} must define at least one path"),
                });
            }

            for (index, pattern) in intent.paths.iter().enumerate() {
                if let Some(message) = validate_trigger_path(pattern) {
                    problems.push(ConfigValidationProblem {
                        source: loaded.source.clone(),
                        rule_id: None,
                        message: format!("routing.intents.{alias}.paths[{index}] {message}"),
                    });
                }
            }
        }
    }

    for (alias, entries) in aliases {
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
                rule_id: None,
                message: format!(
                    "routing intent alias `{alias}` is duplicated across configs: {other_sources}"
                ),
            });
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

pub fn validate_loaded_catalog_configs(
    loaded_configs: &[LoadedCatalogConfig],
) -> Vec<ConfigValidationProblem> {
    let mut problems = Vec::new();
    let mut id_sources = BTreeMap::<String, Vec<&LoadedCatalogConfig>>::new();
    let mut path_sources = BTreeMap::<String, Vec<&LoadedCatalogConfig>>::new();
    let mut repo_entries = Vec::<(&LoadedCatalogConfig, &CatalogRepo)>::new();

    for loaded in loaded_configs {
        for repo in &loaded.catalog.repos {
            if !repo.id.is_empty() {
                id_sources.entry(repo.id.clone()).or_default().push(loaded);
            }
            if !repo.path.is_empty() {
                path_sources
                    .entry(repo.path.clone())
                    .or_default()
                    .push(loaded);
            }
            repo_entries.push((loaded, repo));
        }
    }

    for (repo_id, entries) in id_sources {
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
            let other_sources = if other_sources.is_empty() {
                entry.source.clone()
            } else {
                other_sources
            };
            problems.push(ConfigValidationProblem {
                source: entry.source.clone(),
                rule_id: None,
                message: format!(
                    "catalog repo id `{repo_id}` is duplicated across configs: {other_sources}"
                ),
            });
        }
    }

    for (repo_path, entries) in path_sources {
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
            let other_sources = if other_sources.is_empty() {
                entry.source.clone()
            } else {
                other_sources
            };
            problems.push(ConfigValidationProblem {
                source: entry.source.clone(),
                rule_id: None,
                message: format!(
                    "catalog repo path `{repo_path}` is duplicated across configs: {other_sources}"
                ),
            });
        }
    }

    for left_index in 0..repo_entries.len() {
        for right_index in (left_index + 1)..repo_entries.len() {
            let (left_loaded, left_repo) = repo_entries[left_index];
            let (right_loaded, right_repo) = repo_entries[right_index];
            if left_repo.path == right_repo.path {
                continue;
            }
            if !catalog_repo_paths_overlap(&left_repo.path, &right_repo.path) {
                continue;
            }

            problems.push(ConfigValidationProblem {
                source: left_loaded.source.clone(),
                rule_id: None,
                message: format!(
                    "catalog repo path `{}` overlaps with `{}` from {}",
                    left_repo.path, right_repo.path, right_loaded.source
                ),
            });
            problems.push(ConfigValidationProblem {
                source: right_loaded.source.clone(),
                rule_id: None,
                message: format!(
                    "catalog repo path `{}` overlaps with `{}` from {}",
                    right_repo.path, left_repo.path, left_loaded.source
                ),
            });
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

pub fn validate_loaded_ownership_configs(
    loaded_configs: &[LoadedOwnershipConfig],
    catalog_configs: &[LoadedCatalogConfig],
) -> Vec<ConfigValidationProblem> {
    let mut problems = Vec::new();
    let mut domain_sources = BTreeMap::<String, Vec<&LoadedOwnershipConfig>>::new();
    let mut catalog_repo_ids = BTreeSet::<String>::new();

    for loaded_catalog in catalog_configs {
        for repo in &loaded_catalog.catalog.repos {
            if !repo.id.is_empty() {
                catalog_repo_ids.insert(repo.id.clone());
            }
        }
    }

    for loaded in loaded_configs {
        for domain in &loaded.ownership.domains {
            if !domain.id.is_empty() {
                domain_sources
                    .entry(domain.id.clone())
                    .or_default()
                    .push(loaded);
            }

            if !domain.owner_repo.is_empty() && !catalog_repo_ids.contains(&domain.owner_repo) {
                problems.push(ConfigValidationProblem {
                    source: loaded.source.clone(),
                    rule_id: None,
                    message: format!(
                        "ownership domain `{}` references unknown ownerRepo `{}`",
                        domain.id, domain.owner_repo
                    ),
                });
            }

            for repo_id in &domain.non_owner_repos {
                if !catalog_repo_ids.contains(repo_id) {
                    problems.push(ConfigValidationProblem {
                        source: loaded.source.clone(),
                        rule_id: None,
                        message: format!(
                            "ownership domain `{}` references unknown nonOwnerRepo `{}`",
                            domain.id, repo_id
                        ),
                    });
                }
            }
        }
    }

    for (domain_id, entries) in domain_sources {
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
            let other_sources = if other_sources.is_empty() {
                entry.source.clone()
            } else {
                other_sources
            };

            problems.push(ConfigValidationProblem {
                source: entry.source.clone(),
                rule_id: None,
                message: format!(
                    "ownership domain id `{domain_id}` is duplicated across configs: {other_sources}"
                ),
            });
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

pub fn analyze_ownership_paths(
    tracked_paths: &[String],
    loaded_configs: &[LoadedOwnershipConfig],
) -> OwnershipAnalysis {
    let mut analyses = Vec::new();
    let mut conflicts = Vec::new();
    let mut overlaps = Vec::new();

    for tracked_path in tracked_paths {
        let mut matches = Vec::<OwnershipDomainMatch>::new();

        for loaded in loaded_configs {
            for domain in &loaded.ownership.domains {
                let matched_include = domain
                    .paths
                    .include
                    .iter()
                    .map(|pattern| resolve_rule_path(&loaded.base_dir, pattern))
                    .filter(|pattern| crate::rules::matches_pattern(tracked_path, pattern))
                    .max_by(ownership_pattern_specificity_cmp);

                let Some(matched_include) = matched_include else {
                    continue;
                };

                let excluded = domain
                    .paths
                    .exclude
                    .iter()
                    .map(|pattern| resolve_rule_path(&loaded.base_dir, pattern))
                    .any(|pattern| crate::rules::matches_pattern(tracked_path, &pattern));
                if excluded {
                    continue;
                }

                let (static_prefix_len, wildcard_count) =
                    ownership_pattern_specificity(&matched_include);

                matches.push(OwnershipDomainMatch {
                    source: loaded.source.clone(),
                    base_dir: loaded.base_dir.clone(),
                    domain_id: domain.id.clone(),
                    owner_repo: domain.owner_repo.clone(),
                    non_owner_repos: domain.non_owner_repos.clone(),
                    matched_include,
                    static_prefix_len,
                    wildcard_count,
                });
            }
        }

        if matches.is_empty() {
            continue;
        }

        matches.sort_by(ownership_domain_match_cmp);
        let selected = matches[0].clone();
        let owner_repos = matches
            .iter()
            .map(|entry| entry.owner_repo.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let domain_ids = matches
            .iter()
            .map(|entry| entry.domain_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let has_conflict = owner_repos.len() > 1;
        let has_overlap = !has_conflict && matches.len() > 1;

        if has_conflict {
            conflicts.push(OwnershipConflict {
                path: tracked_path.clone(),
                owner_repos,
                domain_ids,
            });
        } else if has_overlap {
            overlaps.push(OwnershipOverlap {
                path: tracked_path.clone(),
                owner_repo: selected.owner_repo.clone(),
                domain_ids,
                selected_domain_id: selected.domain_id.clone(),
            });
        }

        analyses.push(OwnershipPathAnalysis {
            path: tracked_path.clone(),
            matches,
            selected,
            has_conflict,
            has_overlap,
        });
    }

    OwnershipAnalysis {
        paths: analyses,
        conflicts,
        overlaps,
    }
}

pub fn validate_ownership_path_conflicts(
    analysis: &OwnershipAnalysis,
) -> Vec<ConfigValidationProblem> {
    let mut problems = Vec::new();

    for path_analysis in &analysis.paths {
        if !path_analysis.has_conflict {
            continue;
        }

        let conflicting_owners = path_analysis
            .matches
            .iter()
            .map(|entry| entry.owner_repo.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ");

        for entry in &path_analysis.matches {
            problems.push(ConfigValidationProblem {
                source: entry.source.clone(),
                rule_id: None,
                message: format!(
                    "ownership conflict: tracked path `{}` matches domain `{}` (ownerRepo `{}`) but conflicting ownerRepos are present: {}",
                    path_analysis.path,
                    entry.domain_id,
                    entry.owner_repo,
                    conflicting_owners
                ),
            });
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

fn validate_parsed_configs(parsed_files: &[ParsedImpactFile]) -> Vec<ConfigValidationProblem> {
    let mut problems = Vec::new();

    let root_workspace_profiles = parsed_files
        .first()
        .and_then(|parsed| match parsed.parsed.layout {
            ConfigLayout::Workspace => parsed.parsed.workspace.as_ref(),
            ConfigLayout::Repo => None,
        })
        .map(|workspace| workspace.profiles.clone())
        .unwrap_or_default();

    for parsed in parsed_files {
        validate_top_level_blocks(parsed, &mut problems);
        match parsed.parsed.layout {
            ConfigLayout::Workspace => {
                validate_workspace_config(parsed, &mut problems);
            }
            ConfigLayout::Repo => {
                validate_repo_config(parsed, &root_workspace_profiles, &mut problems);
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

fn validate_top_level_blocks(
    parsed: &ParsedImpactFile,
    problems: &mut Vec<ConfigValidationProblem>,
) {
    if let Some(coverage) = parsed.parsed.coverage.as_ref() {
        validate_scoped_patterns(
            &parsed.descriptor.rel_path,
            "coverage",
            &coverage.include,
            &coverage.exclude,
            problems,
        );
    }

    if let Some(doc_inventory) = parsed.parsed.doc_inventory.as_ref() {
        validate_scoped_patterns(
            &parsed.descriptor.rel_path,
            "docInventory",
            &doc_inventory.include,
            &doc_inventory.exclude,
            problems,
        );
    }

    if let Some(freshness) = parsed.parsed.freshness.as_ref() {
        validate_freshness_block(&parsed.descriptor.rel_path, freshness, problems);
    }

    if let Some(routing) = parsed.parsed.routing.as_ref() {
        validate_routing_block(&parsed.descriptor.rel_path, &routing.intents, problems);
    }

    if let Some(catalog) = parsed.parsed.catalog.as_ref() {
        validate_catalog_block(&parsed.descriptor.rel_path, &catalog.repos, problems);
    }

    if let Some(ownership) = parsed.parsed.ownership.as_ref() {
        validate_ownership_block(&parsed.descriptor.rel_path, &ownership.domains, problems);
    }

    for rule in &parsed.parsed.rules {
        validate_single_rule(
            rule,
            &parsed.descriptor.rel_path,
            &parsed.descriptor.base_dir,
            problems,
        );
    }
}

fn validate_workspace_config(
    parsed: &ParsedImpactFile,
    problems: &mut Vec<ConfigValidationProblem>,
) {
    if parsed.parsed.inherit.is_some() {
        problems.push(ConfigValidationProblem {
            source: parsed.descriptor.rel_path.clone(),
            rule_id: None,
            message: "workspace root configs must not declare `inherit`".into(),
        });
    }
    if parsed.parsed.overrides.is_some() {
        problems.push(ConfigValidationProblem {
            source: parsed.descriptor.rel_path.clone(),
            rule_id: None,
            message: "workspace root configs must not declare `overrides`".into(),
        });
    }

    if let Some(workspace) = parsed.parsed.workspace.as_ref() {
        for (profile_name, profile) in &workspace.profiles {
            let profile_source = format!(
                "{}#workspace.profiles.{profile_name}",
                parsed.descriptor.rel_path
            );
            if let Some(coverage) = profile.coverage.as_ref() {
                validate_scoped_patterns(
                    &profile_source,
                    "coverage",
                    &coverage.include,
                    &coverage.exclude,
                    problems,
                );
            }
            if let Some(doc_inventory) = profile.doc_inventory.as_ref() {
                validate_scoped_patterns(
                    &profile_source,
                    "docInventory",
                    &doc_inventory.include,
                    &doc_inventory.exclude,
                    problems,
                );
            }
            if let Some(freshness) = profile.freshness.as_ref() {
                validate_freshness_block(&profile_source, freshness, problems);
            }
            if let Some(routing) = profile.routing.as_ref() {
                validate_routing_block(&profile_source, &routing.intents, problems);
            }
            for rule in &profile.rules {
                validate_single_rule(rule, &profile_source, &parsed.descriptor.base_dir, problems);
            }
        }
    }
}

fn validate_repo_config(
    parsed: &ParsedImpactFile,
    root_workspace_profiles: &BTreeMap<String, WorkspaceProfile>,
    problems: &mut Vec<ConfigValidationProblem>,
) {
    let Some(inherit) = parsed.parsed.inherit.as_ref() else {
        if parsed.parsed.overrides.is_some() {
            problems.push(ConfigValidationProblem {
                source: parsed.descriptor.rel_path.clone(),
                rule_id: None,
                message: "`overrides` requires `inherit.workspace_profile`".into(),
            });
        }
        return;
    };

    if parsed.parsed.coverage.is_some() {
        problems.push(ConfigValidationProblem {
            source: parsed.descriptor.rel_path.clone(),
            rule_id: None,
            message: "configs using workspace inheritance must not define top-level `coverage`; move it into `overrides.coverage`".into(),
        });
    }
    if parsed.parsed.doc_inventory.is_some() {
        problems.push(ConfigValidationProblem {
            source: parsed.descriptor.rel_path.clone(),
            rule_id: None,
            message: "configs using workspace inheritance must not define top-level `docInventory`; move it into `overrides.docInventory`".into(),
        });
    }
    if parsed.parsed.freshness.is_some() {
        problems.push(ConfigValidationProblem {
            source: parsed.descriptor.rel_path.clone(),
            rule_id: None,
            message: "configs using workspace inheritance must not define top-level `freshness`; move it into `overrides.freshness`".into(),
        });
    }
    if parsed.parsed.routing.is_some() {
        problems.push(ConfigValidationProblem {
            source: parsed.descriptor.rel_path.clone(),
            rule_id: None,
            message: "configs using workspace inheritance must not define top-level `routing`; move it into `overrides.routing`".into(),
        });
    }
    if !parsed.parsed.rules.is_empty() {
        problems.push(ConfigValidationProblem {
            source: parsed.descriptor.rel_path.clone(),
            rule_id: None,
            message: "configs using workspace inheritance must not define top-level `rules`; use `overrides.rules`".into(),
        });
    }

    let Some(profile) = root_workspace_profiles.get(&inherit.workspace_profile) else {
        problems.push(ConfigValidationProblem {
            source: parsed.descriptor.rel_path.clone(),
            rule_id: None,
            message: format!(
                "inherit.workspace_profile `{}` does not exist in the active workspace root",
                inherit.workspace_profile
            ),
        });
        return;
    };

    let overrides = parsed.parsed.overrides.clone().unwrap_or_default();
    if let Some(coverage) = overrides.coverage.as_ref() {
        validate_scoped_patterns(
            &format!("{}#overrides.coverage", parsed.descriptor.rel_path),
            "coverage",
            &coverage.include,
            &coverage.exclude,
            problems,
        );
    }
    if let Some(doc_inventory) = overrides.doc_inventory.as_ref() {
        validate_scoped_patterns(
            &format!("{}#overrides.docInventory", parsed.descriptor.rel_path),
            "docInventory",
            &doc_inventory.include,
            &doc_inventory.exclude,
            problems,
        );
    }
    if let Some(freshness) = overrides.freshness.as_ref() {
        if freshness.mode != OverrideMode::Replace {
            problems.push(ConfigValidationProblem {
                source: format!("{}#overrides.freshness", parsed.descriptor.rel_path),
                rule_id: None,
                message: "overrides.freshness only supports `mode: replace`".into(),
            });
        }
        validate_freshness_block(
            &format!("{}#overrides.freshness", parsed.descriptor.rel_path),
            &FreshnessConfig {
                warn_after_commits: freshness.warn_after_commits,
                warn_after_days: freshness.warn_after_days,
                critical_after_days: freshness.critical_after_days,
            },
            problems,
        );
    }
    if let Some(routing) = overrides.routing.as_ref() {
        validate_routing_block(
            &format!("{}#overrides.routing", parsed.descriptor.rel_path),
            &routing.intents,
            problems,
        );
    }

    for rule in &overrides.rules.add {
        validate_single_rule(
            rule,
            &format!("{}#overrides.rules.add", parsed.descriptor.rel_path),
            &parsed.descriptor.base_dir,
            problems,
        );
    }
    for rule in &overrides.rules.replace {
        validate_single_rule(
            rule,
            &format!("{}#overrides.rules.replace", parsed.descriptor.rel_path),
            &parsed.descriptor.base_dir,
            problems,
        );
    }

    let profile_rule_ids = profile
        .rules
        .iter()
        .map(|rule| rule.id.as_str())
        .collect::<BTreeSet<_>>();
    let replace_ids = overrides
        .rules
        .replace
        .iter()
        .map(|rule| rule.id.as_str())
        .collect::<BTreeSet<_>>();
    let add_ids = overrides
        .rules
        .add
        .iter()
        .map(|rule| rule.id.as_str())
        .collect::<BTreeSet<_>>();

    for rule in &overrides.rules.replace {
        if !profile_rule_ids.contains(rule.id.as_str()) {
            problems.push(ConfigValidationProblem {
                source: format!("{}#overrides.rules.replace", parsed.descriptor.rel_path),
                rule_id: Some(rule.id.clone()),
                message: format!(
                    "override replace rule `{}` does not match any inherited rule in workspace profile `{}`",
                    rule.id, inherit.workspace_profile
                ),
            });
        }
    }

    for disabled in &overrides.rules.disable {
        if !profile_rule_ids.contains(disabled.id.as_str()) {
            problems.push(ConfigValidationProblem {
                source: format!("{}#overrides.rules.disable", parsed.descriptor.rel_path),
                rule_id: Some(disabled.id.clone()),
                message: format!(
                    "override disable rule `{}` does not match any inherited rule in workspace profile `{}`",
                    disabled.id, inherit.workspace_profile
                ),
            });
        }
        if add_ids.contains(disabled.id.as_str()) && !replace_ids.contains(disabled.id.as_str()) {
            problems.push(ConfigValidationProblem {
                source: format!("{}#overrides.rules.disable", parsed.descriptor.rel_path),
                rule_id: Some(disabled.id.clone()),
                message: format!(
                    "override disable rule `{}` targets a locally added rule; only inherited rules may be disabled",
                    disabled.id
                ),
            });
        }
        if disabled.reason.trim().is_empty() {
            problems.push(ConfigValidationProblem {
                source: format!("{}#overrides.rules.disable", parsed.descriptor.rel_path),
                rule_id: Some(disabled.id.clone()),
                message: "override disable entries must include a non-empty reason".into(),
            });
        }
    }
}

fn validate_scoped_patterns(
    source: &str,
    block_name: &str,
    include: &[String],
    exclude: &[String],
    problems: &mut Vec<ConfigValidationProblem>,
) {
    for (index, pattern) in include.iter().enumerate() {
        if let Some(message) = validate_trigger_path(pattern) {
            problems.push(ConfigValidationProblem {
                source: source.into(),
                rule_id: None,
                message: format!("{block_name}.include[{index}] {message}"),
            });
        }
    }

    for (index, pattern) in exclude.iter().enumerate() {
        if let Some(message) = validate_trigger_path(pattern) {
            problems.push(ConfigValidationProblem {
                source: source.into(),
                rule_id: None,
                message: format!("{block_name}.exclude[{index}] {message}"),
            });
        }
    }
}

fn validate_freshness_block(
    source: &str,
    freshness: &FreshnessConfig,
    problems: &mut Vec<ConfigValidationProblem>,
) {
    if freshness.warn_after_commits == 0 {
        problems.push(ConfigValidationProblem {
            source: source.into(),
            rule_id: None,
            message: "freshness.warn_after_commits must be greater than 0".into(),
        });
    }
    if freshness.warn_after_days == 0 {
        problems.push(ConfigValidationProblem {
            source: source.into(),
            rule_id: None,
            message: "freshness.warn_after_days must be greater than 0".into(),
        });
    }
    if freshness.critical_after_days == 0 {
        problems.push(ConfigValidationProblem {
            source: source.into(),
            rule_id: None,
            message: "freshness.critical_after_days must be greater than 0".into(),
        });
    }
    if freshness.critical_after_days < freshness.warn_after_days {
        problems.push(ConfigValidationProblem {
            source: source.into(),
            rule_id: None,
            message:
                "freshness.critical_after_days must be greater than or equal to freshness.warn_after_days"
                    .into(),
        });
    }
}

fn validate_routing_block(
    source: &str,
    intents: &BTreeMap<String, RoutingIntent>,
    problems: &mut Vec<ConfigValidationProblem>,
) {
    for (alias, intent) in intents {
        if alias.trim().is_empty() {
            problems.push(ConfigValidationProblem {
                source: source.into(),
                rule_id: None,
                message: "routing intent alias must not be empty".into(),
            });
        }

        if intent.paths.is_empty() {
            problems.push(ConfigValidationProblem {
                source: source.into(),
                rule_id: None,
                message: format!("routing.intents.{alias} must define at least one path"),
            });
        }

        for (index, pattern) in intent.paths.iter().enumerate() {
            if let Some(message) = validate_trigger_path(pattern) {
                problems.push(ConfigValidationProblem {
                    source: source.into(),
                    rule_id: None,
                    message: format!("routing.intents.{alias}.paths[{index}] {message}"),
                });
            }
        }
    }
}

fn validate_catalog_block(
    source: &str,
    repos: &[CatalogRepo],
    problems: &mut Vec<ConfigValidationProblem>,
) {
    if repos.is_empty() {
        problems.push(ConfigValidationProblem {
            source: source.into(),
            rule_id: None,
            message: "catalog.repos must define at least one repo".into(),
        });
        return;
    }

    for (index, repo) in repos.iter().enumerate() {
        if repo.id.trim().is_empty() {
            problems.push(ConfigValidationProblem {
                source: source.into(),
                rule_id: None,
                message: format!("catalog.repos[{index}].id must not be empty"),
            });
        }

        if let Some(message) = validate_catalog_repo_path(&repo.path) {
            problems.push(ConfigValidationProblem {
                source: source.into(),
                rule_id: None,
                message: format!("catalog.repos[{index}].path {message}"),
            });
        }

        if let Some(canonical_repo) = repo.canonical_repo.as_ref()
            && canonical_repo.trim().is_empty()
        {
            problems.push(ConfigValidationProblem {
                source: source.into(),
                rule_id: None,
                message: format!(
                    "catalog.repos[{index}].canonicalRepo must not be empty when provided"
                ),
            });
        }

        if let Some(path) = repo.entry_doc.as_ref()
            && let Some(message) = validate_required_doc_path(path)
        {
            problems.push(ConfigValidationProblem {
                source: source.into(),
                rule_id: None,
                message: format!("catalog.repos[{index}].entryDoc {message}"),
            });
        }

        if let Some(path) = repo.branch_policy_doc.as_ref()
            && let Some(message) = validate_required_doc_path(path)
        {
            problems.push(ConfigValidationProblem {
                source: source.into(),
                rule_id: None,
                message: format!("catalog.repos[{index}].branchPolicyDoc {message}"),
            });
        }

        for (doc_index, path) in repo.workflow_docs.iter().enumerate() {
            if let Some(message) = validate_required_doc_path(path) {
                problems.push(ConfigValidationProblem {
                    source: source.into(),
                    rule_id: None,
                    message: format!("catalog.repos[{index}].workflowDocs[{doc_index}] {message}"),
                });
            }
        }

        for (doc_index, path) in repo.integration_docs.iter().enumerate() {
            if let Some(message) = validate_required_doc_path(path) {
                problems.push(ConfigValidationProblem {
                    source: source.into(),
                    rule_id: None,
                    message: format!(
                        "catalog.repos[{index}].integrationDocs[{doc_index}] {message}"
                    ),
                });
            }
        }
    }
}

fn validate_ownership_block(
    source: &str,
    domains: &[OwnershipDomain],
    problems: &mut Vec<ConfigValidationProblem>,
) {
    if domains.is_empty() {
        problems.push(ConfigValidationProblem {
            source: source.into(),
            rule_id: None,
            message: "ownership.domains must define at least one domain".into(),
        });
        return;
    }

    for (index, domain) in domains.iter().enumerate() {
        if domain.id.trim().is_empty() {
            problems.push(ConfigValidationProblem {
                source: source.into(),
                rule_id: None,
                message: format!("ownership.domains[{index}].id must not be empty"),
            });
        }

        if domain.paths.include.is_empty() {
            problems.push(ConfigValidationProblem {
                source: source.into(),
                rule_id: None,
                message: format!(
                    "ownership.domains[{index}].paths.include must define at least one path"
                ),
            });
        }

        for (path_index, pattern) in domain.paths.include.iter().enumerate() {
            if let Some(message) = validate_trigger_path(pattern) {
                problems.push(ConfigValidationProblem {
                    source: source.into(),
                    rule_id: None,
                    message: format!(
                        "ownership.domains[{index}].paths.include[{path_index}] {message}"
                    ),
                });
            }
        }

        for (path_index, pattern) in domain.paths.exclude.iter().enumerate() {
            if let Some(message) = validate_trigger_path(pattern) {
                problems.push(ConfigValidationProblem {
                    source: source.into(),
                    rule_id: None,
                    message: format!(
                        "ownership.domains[{index}].paths.exclude[{path_index}] {message}"
                    ),
                });
            }
        }

        if domain.owner_repo.trim().is_empty() {
            problems.push(ConfigValidationProblem {
                source: source.into(),
                rule_id: None,
                message: format!("ownership.domains[{index}].ownerRepo must not be empty"),
            });
        }

        if domain.non_owner_repos.contains(&domain.owner_repo) {
            problems.push(ConfigValidationProblem {
                source: source.into(),
                rule_id: None,
                message: format!(
                    "ownership domain `{}` must not list ownerRepo `{}` inside nonOwnerRepos",
                    domain.id, domain.owner_repo
                ),
            });
        }

        for (repo_index, repo_id) in domain.non_owner_repos.iter().enumerate() {
            if repo_id.trim().is_empty() {
                problems.push(ConfigValidationProblem {
                    source: source.into(),
                    rule_id: None,
                    message: format!(
                        "ownership.domains[{index}].nonOwnerRepos[{repo_index}] must not be empty"
                    ),
                });
            }
        }
    }
}

fn validate_single_rule(
    rule: &Rule,
    source: &str,
    base_dir: &str,
    problems: &mut Vec<ConfigValidationProblem>,
) {
    let loaded = LoadedRule {
        source: source.into(),
        config_source: source.into(),
        base_dir: base_dir.into(),
        provenance: RuleProvenance {
            config_source: source.into(),
            origin_kind: RuleOriginKind::RepoLocal,
            workspace_profile: None,
        },
        rule: rule.clone(),
    };
    validate_rule(&loaded, problems);
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

fn validate_catalog_repo_path(path: &str) -> Option<String> {
    let trimmed = path.trim();

    if trimmed == "." {
        return None;
    }

    validate_repo_relative_path(trimmed, false)
}

fn catalog_repo_paths_overlap(left: &str, right: &str) -> bool {
    if left == "." || right == "." {
        return true;
    }

    left == right
        || left
            .strip_prefix(right)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || right
            .strip_prefix(left)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn ownership_pattern_specificity(pattern: &str) -> (usize, usize) {
    let static_prefix_len = pattern
        .chars()
        .take_while(|ch| *ch != '*' && *ch != '?')
        .count();
    let wildcard_count = pattern.chars().filter(|ch| matches!(ch, '*' | '?')).count();
    (static_prefix_len, wildcard_count)
}

fn ownership_pattern_specificity_cmp(left: &String, right: &String) -> std::cmp::Ordering {
    let left_specificity = ownership_pattern_specificity(left);
    let right_specificity = ownership_pattern_specificity(right);

    right_specificity
        .0
        .cmp(&left_specificity.0)
        .then_with(|| left_specificity.1.cmp(&right_specificity.1))
        .then_with(|| left.cmp(right))
}

fn ownership_domain_match_cmp(
    left: &OwnershipDomainMatch,
    right: &OwnershipDomainMatch,
) -> std::cmp::Ordering {
    right
        .static_prefix_len
        .cmp(&left.static_prefix_len)
        .then_with(|| left.wildcard_count.cmp(&right.wildcard_count))
        .then_with(|| left.domain_id.cmp(&right.domain_id))
        .then_with(|| left.source.cmp(&right.source))
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
        CONFIG_FILE, DOC_ROOT_DIR, ImpactLayout, RuleOriginKind, analyze_ownership_paths,
        detect_impact_layout, load_catalog_configs, load_effective_configs, load_impact_files,
        load_ownership_configs, load_routing_configs, normalize_path, resolve_rule_path,
        root_dir_from_option, validate_config_graph, validate_loaded_catalog_configs,
        validate_loaded_ownership_configs, validate_loaded_routing_configs, validate_loaded_rules,
        validate_ownership_path_conflicts,
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
    fn workspace_inheritance_builds_effective_child_config() {
        let root = temp_dir("docpact-config-inherit");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("root doc dir");
        fs::create_dir_all(root.join(format!("sample-sdk/{DOC_ROOT_DIR}")))
            .expect("subrepo doc dir");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: workspace
workspace:
  name: demo
  profiles:
    default:
      coverage:
        include:
          - src/**
      docInventory:
        include:
          - docs/**
      freshness:
        warn_after_commits: 10
        warn_after_days: 20
        critical_after_days: 30
      routing:
        intents:
          shared:
            paths:
              - src/**
      rules:
        - id: inherited-rule
          scope: workspace
          repo: workspace
          triggers:
            - path: src/**
              kind: code
          requiredDocs:
            - path: docs/guide.md
              mode: review_or_update
          reason: inherited
rules:
  - id: root-only
    scope: workspace
    repo: workspace
    triggers:
      - path: AGENTS.md
        kind: doc
    requiredDocs:
      - path: .docpact/config.yaml
    reason: root
"#,
        )
        .expect("root config");

        fs::write(
            root.join(format!("sample-sdk/{CONFIG_FILE}")),
            r#"
version: 1
layout: repo
inherit:
  workspace_profile: default
overrides:
  rules:
    add:
      - id: local-extra
        scope: repo
        repo: sample-sdk
        triggers:
          - path: src/payments/**
            kind: code
        requiredDocs:
          - path: docs/payments.md
        reason: local
    replace:
      - id: inherited-rule
        scope: repo
        repo: sample-sdk
        triggers:
          - path: src/app/**
            kind: code
        requiredDocs:
          - path: docs/app.md
        reason: replaced
  coverage:
    mode: merge
    include:
      - tests/**
    exclude:
      - dist/**
  docInventory:
    mode: replace
    include:
      - README.md
  freshness:
    mode: replace
    warn_after_commits: 21
    warn_after_days: 34
    critical_after_days: 55
  routing:
    mode: merge
    intents:
      payments:
        paths:
          - src/payments/**
      shared:
        paths:
          - src/app/**
"#,
        )
        .expect("child config");

        let effective = load_effective_configs(&root, None).expect("effective configs");
        assert_eq!(effective.len(), 2);

        let child = effective
            .iter()
            .find(|entry| entry.base_dir == "sample-sdk")
            .expect("child config should exist");
        assert_eq!(
            child
                .inheritance
                .as_ref()
                .expect("inheritance should exist")
                .workspace_profile,
            "default"
        );
        assert_eq!(child.coverage.coverage.include, vec!["src/**", "tests/**"]);
        assert_eq!(child.coverage.coverage.exclude, vec!["dist/**"]);
        assert_eq!(
            child
                .routing
                .routing
                .intents
                .get("shared")
                .expect("shared intent")
                .paths,
            vec!["src/app/**"]
        );
        assert_eq!(
            child
                .routing
                .routing
                .intents
                .get("payments")
                .expect("payments intent")
                .paths,
            vec!["src/payments/**"]
        );
        assert_eq!(child.doc_inventory.doc_inventory.include, vec!["README.md"]);
        assert_eq!(child.freshness.freshness.warn_after_commits, 21);
        assert_eq!(child.rules.len(), 2);
        assert_eq!(child.rules[0].rule.id, "inherited-rule");
        assert_eq!(
            child.rules[0].provenance.origin_kind,
            RuleOriginKind::OverrideReplace
        );
        assert_eq!(child.rules[1].rule.id, "local-extra");
        assert_eq!(
            child.rules[1].provenance.origin_kind,
            RuleOriginKind::OverrideAdd
        );
        assert_eq!(
            child.rules[0].config_source,
            "sample-sdk/.docpact/config.yaml"
        );
        assert_eq!(child.rules[0].base_dir, "sample-sdk");
        assert_eq!(
            child
                .inheritance
                .as_ref()
                .expect("inheritance should exist")
                .routing_mode
                .as_deref(),
            Some("merge")
        );
    }

    #[test]
    fn workspace_child_without_inherit_remains_local_repo_config() {
        let root = temp_dir("docpact-config-local-child");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("root doc dir");
        fs::create_dir_all(root.join(format!("sample-sdk/{DOC_ROOT_DIR}")))
            .expect("subrepo doc dir");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: workspace
workspace:
  name: demo
rules: []
"#,
        )
        .expect("root config");

        fs::write(
            root.join(format!("sample-sdk/{CONFIG_FILE}")),
            r#"
version: 1
layout: repo
coverage:
  include:
    - src/**
rules:
  - id: repo-rule
    scope: repo
    repo: sample-sdk
    triggers:
      - path: src/**
        kind: code
    requiredDocs:
      - path: docs/guide.md
    reason: local
"#,
        )
        .expect("child config");

        let effective = load_effective_configs(&root, None).expect("effective configs");
        let child = effective
            .iter()
            .find(|entry| entry.base_dir == "sample-sdk")
            .expect("child config should exist");

        assert!(child.inheritance.is_none());
        assert_eq!(child.coverage.coverage.include, vec!["src/**"]);
        assert_eq!(child.rules[0].source, "sample-sdk/.docpact/config.yaml");
        assert_eq!(
            child.rules[0].provenance.origin_kind,
            RuleOriginKind::RepoLocal
        );
    }

    #[test]
    fn strict_validation_reports_invalid_inheritance_shapes() {
        let root = temp_dir("docpact-config-invalid-inherit");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("root doc dir");
        fs::create_dir_all(root.join(format!("sample-sdk/{DOC_ROOT_DIR}")))
            .expect("subrepo doc dir");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: workspace
workspace:
  name: demo
  profiles:
    default:
      rules:
        - id: inherited-rule
          scope: workspace
          repo: workspace
          triggers:
            - path: src/**
              kind: code
          requiredDocs:
            - path: docs/guide.md
          reason: inherited
rules: []
"#,
        )
        .expect("root config");

        fs::write(
            root.join(format!("sample-sdk/{CONFIG_FILE}")),
            r#"
version: 1
layout: repo
inherit:
  workspace_profile: default
coverage:
  include:
    - src/**
rules:
  - id: illegal-local
    scope: repo
    repo: sample-sdk
    triggers:
      - path: src/**
        kind: code
    requiredDocs:
      - path: docs/guide.md
    reason: local
overrides:
  freshness:
    mode: merge
"#,
        )
        .expect("child config");

        let problems = validate_config_graph(&root, None).expect("validation should work");
        let messages = problems
            .iter()
            .map(|problem| format!("{}: {}", problem.source, problem.message))
            .collect::<Vec<_>>();

        assert!(messages.iter().any(|message| {
            message
                .contains("must not define top-level `coverage`; move it into `overrides.coverage`")
        }));
        assert!(messages.iter().any(|message| {
            message.contains("must not define top-level `rules`; use `overrides.rules`")
        }));
        assert!(messages.iter().any(|message| {
            message.contains("overrides.freshness only supports `mode: replace`")
        }));
    }

    #[test]
    fn strict_validation_reports_duplicate_routing_intent_aliases() {
        let root = temp_dir("docpact-routing-duplicate-intents");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("doc root dir");
        fs::create_dir_all(root.join(format!("a/{DOC_ROOT_DIR}"))).expect("repo a dir");
        fs::create_dir_all(root.join(format!("b/{DOC_ROOT_DIR}"))).expect("repo b dir");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: workspace
rules: []
"#,
        )
        .expect("root config");
        fs::write(
            root.join(format!("a/{CONFIG_FILE}")),
            r#"
version: 1
layout: repo
routing:
  intents:
    payments:
      paths:
        - src/a/**
rules: []
"#,
        )
        .expect("repo a config");
        fs::write(
            root.join(format!("b/{CONFIG_FILE}")),
            r#"
version: 1
layout: repo
routing:
  intents:
    payments:
      paths:
        - src/b/**
rules: []
"#,
        )
        .expect("repo b config");

        let routing_configs = load_routing_configs(&root, None).expect("routing configs");
        let problems = validate_loaded_routing_configs(&routing_configs);
        let messages = problems
            .iter()
            .map(|problem| problem.message.as_str())
            .collect::<Vec<_>>();

        assert!(messages.iter().any(|message| {
            message.contains("routing intent alias `payments` is duplicated across configs")
        }));
    }

    #[test]
    fn load_catalog_configs_normalizes_doc_pointers_and_repo_root() {
        let root = temp_dir("docpact-catalog-normalize");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("doc root dir");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: repo
catalog:
  repos:
    - id: sample-sdk
      path: .
      canonicalRepo: "  Biaoo/sample-sdk  "
      entryDoc: ./README.md
      branchPolicyDoc: docs/branch-policy.md
      workflowDocs:
        - ./docs/workflows/local-dev.md
        - docs/workflows/local-dev.md
        - docs/workflows/release.md
      integrationDocs:
        - docs/workflows/workspace-integration.md
        - ./docs/workflows/workspace-integration.md
rules: []
"#,
        )
        .expect("repo config");

        let loaded = load_catalog_configs(&root, None).expect("catalog configs");
        let catalog = &loaded[0].catalog;
        let repo = &catalog.repos[0];

        assert_eq!(repo.path, ".");
        assert_eq!(repo.canonical_repo.as_deref(), Some("Biaoo/sample-sdk"));
        assert_eq!(repo.entry_doc.as_deref(), Some("README.md"));
        assert_eq!(
            repo.workflow_docs,
            vec!["docs/workflows/local-dev.md", "docs/workflows/release.md"]
        );
        assert_eq!(
            repo.integration_docs,
            vec!["docs/workflows/workspace-integration.md"]
        );
    }

    #[test]
    fn strict_validation_reports_invalid_catalog_pointers_and_overlapping_roots() {
        let root = temp_dir("docpact-catalog-invalid");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("doc root dir");
        fs::create_dir_all(root.join(format!("sample-sdk/{DOC_ROOT_DIR}"))).expect("child doc dir");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: workspace
catalog:
  repos:
    - id: app
      path: apps
      entryDoc: docs/*.md
    - id: nested
      path: apps/web
rules: []
"#,
        )
        .expect("workspace config");

        fs::write(
            root.join(format!("sample-sdk/{CONFIG_FILE}")),
            r#"
version: 1
layout: repo
catalog:
  repos:
    - id: app
      path: services/api
rules: []
"#,
        )
        .expect("child config");

        let graph_problems = validate_config_graph(&root, None).expect("graph problems");
        assert!(graph_problems.iter().any(|problem| {
            problem
                .message
                .contains("catalog.repos[0].entryDoc must be an exact document path, not a glob")
        }));

        let catalog_configs = load_catalog_configs(&root, None).expect("catalog configs");
        let loaded_problems = validate_loaded_catalog_configs(&catalog_configs);
        let messages = loaded_problems
            .iter()
            .map(|problem| problem.message.as_str())
            .collect::<Vec<_>>();

        assert!(messages.iter().any(|message| {
            message.contains("catalog repo id `app` is duplicated across configs")
        }));
        assert!(messages.iter().any(|message| {
            message.contains("catalog repo path `apps` overlaps with `apps/web`")
        }));
        assert!(messages.iter().any(|message| {
            message.contains("catalog repo path `apps/web` overlaps with `apps`")
        }));
    }

    #[test]
    fn load_ownership_configs_normalizes_patterns_and_repo_references() {
        let root = temp_dir("docpact-ownership-normalize");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("doc root dir");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: repo
ownership:
  domains:
    - id: " payments "
      paths:
        include:
          - ./src/payments/**
          - src/payments/**
        exclude:
          - ./src/payments/generated/**
      ownerRepo: " sample-sdk "
      nonOwnerRepos:
        - edge
        - edge
        - next
rules: []
"#,
        )
        .expect("repo config");

        let loaded = load_ownership_configs(&root, None).expect("ownership configs");
        let domain = &loaded[0].ownership.domains[0];

        assert_eq!(domain.id, "payments");
        assert_eq!(domain.paths.include, vec!["src/payments/**"]);
        assert_eq!(domain.paths.exclude, vec!["src/payments/generated/**"]);
        assert_eq!(domain.owner_repo, "sample-sdk");
        assert_eq!(domain.non_owner_repos, vec!["edge", "next"]);
    }

    #[test]
    fn strict_validation_reports_invalid_ownership_references_and_duplicate_ids() {
        let root = temp_dir("docpact-ownership-invalid");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("doc root dir");
        fs::create_dir_all(root.join(format!("sample-sdk/{DOC_ROOT_DIR}"))).expect("child doc dir");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: workspace
catalog:
  repos:
    - id: app
      path: apps/app
    - id: web
      path: apps/web
ownership:
  domains:
    - id: payments
      paths:
        include:
          - src/***
      ownerRepo: app
      nonOwnerRepos:
        - app
        - missing
    - id: checkout
      paths:
        include: []
      ownerRepo: missing
rules: []
"#,
        )
        .expect("workspace config");

        fs::write(
            root.join(format!("sample-sdk/{CONFIG_FILE}")),
            r#"
version: 1
layout: repo
ownership:
  domains:
    - id: payments
      paths:
        include:
          - src/web/**
      ownerRepo: web
rules: []
"#,
        )
        .expect("child config");

        let graph_problems = validate_config_graph(&root, None).expect("graph problems");
        assert!(graph_problems.iter().any(|problem| {
            problem
                .message
                .contains("ownership.domains[0].paths.include[0] contains malformed glob segment")
        }));
        assert!(graph_problems.iter().any(|problem| {
            problem.message.contains(
                "ownership domain `payments` must not list ownerRepo `app` inside nonOwnerRepos",
            )
        }));
        assert!(graph_problems.iter().any(|problem| {
            problem
                .message
                .contains("ownership.domains[1].paths.include must define at least one path")
        }));

        let ownership_configs = load_ownership_configs(&root, None).expect("ownership configs");
        let catalog_configs = load_catalog_configs(&root, None).expect("catalog configs");
        let loaded_problems =
            validate_loaded_ownership_configs(&ownership_configs, &catalog_configs);
        let messages = loaded_problems
            .iter()
            .map(|problem| problem.message.as_str())
            .collect::<Vec<_>>();

        assert!(messages.iter().any(|message| {
            message
                .contains("ownership domain `payments` references unknown nonOwnerRepo `missing`")
        }));
        assert!(messages.iter().any(|message| {
            message.contains("ownership domain `checkout` references unknown ownerRepo `missing`")
        }));
        assert!(messages.iter().any(|message| {
            message.contains("ownership domain id `payments` is duplicated across configs")
        }));
    }

    #[test]
    fn ownership_analysis_reports_conflicts_and_overlaps_and_selects_most_specific_domain() {
        let root = temp_dir("docpact-ownership-analysis");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("doc root dir");
        fs::create_dir_all(root.join(format!("service/{DOC_ROOT_DIR}"))).expect("service doc dir");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: workspace
ownership:
  domains:
    - id: broad-root
      paths:
        include:
          - service/src/**
      ownerRepo: app
    - id: payments-specific
      paths:
        include:
          - service/src/payments/**
      ownerRepo: app
    - id: conflicts
      paths:
        include:
          - service/src/conflict/**
      ownerRepo: edge
rules: []
"#,
        )
        .expect("root config");

        fs::write(
            root.join(format!("service/{CONFIG_FILE}")),
            r#"
version: 1
layout: repo
ownership:
  domains:
    - id: child-conflict
      paths:
        include:
          - src/conflict/**
      ownerRepo: web
rules: []
"#,
        )
        .expect("child config");

        let ownership_configs = load_ownership_configs(&root, None).expect("ownership configs");
        let analysis = analyze_ownership_paths(
            &[
                "service/src/payments/charge.ts".into(),
                "service/src/conflict/index.ts".into(),
            ],
            &ownership_configs,
        );

        assert_eq!(analysis.overlaps.len(), 1);
        assert_eq!(analysis.conflicts.len(), 1);
        assert_eq!(analysis.overlaps[0].selected_domain_id, "payments-specific");
        assert_eq!(
            analysis.conflicts[0].owner_repos,
            vec!["app", "edge", "web"]
        );

        let selected = analysis
            .paths
            .iter()
            .find(|entry| entry.path == "service/src/payments/charge.ts")
            .expect("payments path should exist");
        assert_eq!(selected.selected.domain_id, "payments-specific");

        let conflict_problems = validate_ownership_path_conflicts(&analysis);
        assert!(conflict_problems.iter().any(|problem| {
            problem
                .message
                .contains("tracked path `service/src/conflict/index.ts`")
        }));
        assert!(conflict_problems.iter().any(|problem| {
            problem
                .message
                .contains("conflicting ownerRepos are present: app, edge, web")
        }));
    }

    #[test]
    fn duplicate_effective_rule_ids_still_report_in_strict_validation() {
        let root = temp_dir("docpact-validate-duplicate-effective");
        fs::create_dir_all(root.join(DOC_ROOT_DIR)).expect("doc root dir");
        fs::create_dir_all(root.join(format!("subrepo/{DOC_ROOT_DIR}"))).expect("subrepo dir");

        fs::write(
            root.join(CONFIG_FILE),
            r#"
version: 1
layout: workspace
rules:
  - id: duplicate-rule
    scope: workspace
    repo: workspace
    triggers:
      - path: AGENTS.md
        kind: doc
    requiredDocs:
      - path: .docpact/config.yaml
    reason: root
"#,
        )
        .expect("root config");
        fs::write(
            root.join(format!("subrepo/{CONFIG_FILE}")),
            r#"
version: 1
layout: repo
rules:
  - id: duplicate-rule
    scope: repo
    repo: subrepo
    triggers:
      - path: src/**
        kind: code
    requiredDocs:
      - path: docs/guide.md
    reason: repo
"#,
        )
        .expect("subrepo config");

        let loaded = load_impact_files(&root, None).expect("rules should load");
        let problems = validate_loaded_rules(&loaded);
        assert!(problems.iter().any(|problem| {
            problem
                .message
                .contains("duplicate rule id `duplicate-rule`")
        }));
    }

    #[test]
    fn resolve_rule_path_uses_base_dir() {
        assert_eq!(
            resolve_rule_path("subrepo", ".docpact/config.yaml"),
            "subrepo/.docpact/config.yaml"
        );
    }

    #[test]
    fn root_dir_from_option_preserves_explicit_path() {
        let root = temp_dir("docpact-config-root-dir");
        let resolved = root_dir_from_option(Some(root.as_path())).expect("root dir");
        assert_eq!(resolved, root);
    }
}
