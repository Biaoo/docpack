use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use miette::{IntoDiagnostic, Result, bail, miette};
use serde::{Deserialize, Serialize};

use crate::AppExit;
use crate::baseline::{FindingFingerprint, fingerprint_for};
use crate::cli::{WaiverAddArgs, WaiverArgs, WaiverCommands, WaiverOutputFormat};
use crate::config::{normalize_path, root_dir_from_option};
use crate::diagnostics::read_diagnostics_artifact;
use crate::reporters::{
    DiagnosticRecord, DiagnosticsArtifact, ExpiredWaiver, refresh_finding_summaries,
};
use crate::rules::matches_pattern;

pub const WAIVER_SCHEMA_VERSION: &str = "docpact.waivers.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WaiverFile {
    pub schema_version: String,
    pub tool_name: String,
    pub tool_version: String,
    pub waiver_count: usize,
    pub waivers: Vec<WaiverRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WaiverRecord {
    pub fingerprint: FindingFingerprint,
    pub reason: String,
    pub owner: String,
    pub expires_at: String,
    #[serde(default, skip_serializing_if = "WaiverScope::is_empty")]
    pub scope: WaiverScope,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WaiverScope {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rule_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paths: Vec<String>,
}

impl WaiverScope {
    fn is_empty(&self) -> bool {
        self.rule_ids.is_empty() && self.paths.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct WaiverAddResult {
    status: String,
    created: bool,
    waiver_count: usize,
    waivers_path: String,
    waiver: WaiverRecord,
}

pub fn run(args: WaiverArgs) -> Result<AppExit> {
    match args.command {
        WaiverCommands::Add(args) => add(args),
    }
}

fn add(args: WaiverAddArgs) -> Result<AppExit> {
    let root_dir = root_dir_from_option(args.root.as_deref())?;
    let artifact = read_diagnostics_artifact(&args.report)?;
    let diagnostic = artifact
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.diagnostic_id == args.id)
        .ok_or_else(|| {
            miette!(
                "diagnostic `{}` was not found in report {}",
                args.id,
                args.report.display()
            )
        })?;

    let waiver = WaiverRecord {
        fingerprint: fingerprint_for(diagnostic),
        reason: non_empty_field("reason", &args.reason)?,
        owner: non_empty_field("owner", &args.owner)?,
        expires_at: args.expires_at,
        scope: WaiverScope {
            rule_ids: normalized_rule_ids(&args.scope_rule_ids),
            paths: normalized_scope_paths(&root_dir, &args.scope_paths)?,
        },
    };

    let mut file = read_waiver_file_or_default(&args.waivers)?;
    let created = if file.waivers.iter().any(|existing| existing == &waiver) {
        false
    } else {
        file.waivers.push(waiver.clone());
        file.waivers.sort_by(|left, right| {
            (
                &left.fingerprint.problem_type,
                &left.fingerprint.path,
                &left.fingerprint.rule_id,
                &left.expires_at,
                &left.owner,
                &left.reason,
            )
                .cmp(&(
                    &right.fingerprint.problem_type,
                    &right.fingerprint.path,
                    &right.fingerprint.rule_id,
                    &right.expires_at,
                    &right.owner,
                    &right.reason,
                ))
        });
        file.waiver_count = file.waivers.len();
        write_waiver_file(&args.waivers, &file)?;
        true
    };

    if !created {
        file.waiver_count = file.waivers.len();
    }

    emit_add_result(
        WaiverAddResult {
            status: "ok".into(),
            created,
            waiver_count: file.waivers.len(),
            waivers_path: display_path(&args.waivers)?,
            waiver,
        },
        args.format,
    );

    Ok(AppExit::Success)
}

pub fn read_waiver_file(path: &Path) -> Result<WaiverFile> {
    let text = fs::read_to_string(path).into_diagnostic()?;
    let file: WaiverFile = yaml_serde::from_str(&text)
        .into_diagnostic()
        .map_err(|error| {
            miette!(
                "{} is not a valid docpact waivers file. {error}",
                path.display()
            )
        })?;

    if file.schema_version != WAIVER_SCHEMA_VERSION {
        bail!(
            "unsupported waivers schema `{}` in {}",
            file.schema_version,
            path.display()
        );
    }

    Ok(file)
}

pub fn read_waiver_file_or_default(path: &Path) -> Result<WaiverFile> {
    if path.exists() {
        read_waiver_file(path)
    } else {
        Ok(WaiverFile {
            schema_version: WAIVER_SCHEMA_VERSION.into(),
            tool_name: env!("CARGO_PKG_NAME").into(),
            tool_version: env!("CARGO_PKG_VERSION").into(),
            waiver_count: 0,
            waivers: Vec::new(),
        })
    }
}

pub fn write_waiver_file(path: &Path, file: &WaiverFile) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| miette!("waivers path has no parent directory: {}", path.display()))?;
    fs::create_dir_all(parent).into_diagnostic()?;
    let text = yaml_serde::to_string(file).into_diagnostic()?;
    fs::write(path, text).into_diagnostic()?;
    Ok(())
}

pub fn apply_waivers(
    artifact: &mut DiagnosticsArtifact,
    waivers: &WaiverFile,
    current_date: &str,
) -> Result<()> {
    let mut expired_waivers = Vec::new();
    let mut active_waivers = Vec::new();
    for waiver in &waivers.waivers {
        if is_expired(current_date, &waiver.expires_at) {
            expired_waivers.push(ExpiredWaiver {
                problem_type: waiver.fingerprint.problem_type.clone(),
                path: waiver.fingerprint.path.clone(),
                rule_id: waiver.fingerprint.rule_id.clone(),
                owner: waiver.owner.clone(),
                reason: waiver.reason.clone(),
                expires_at: waiver.expires_at.clone(),
            });
        } else {
            active_waivers.push(waiver);
        }
    }

    let mut waived_count = 0usize;
    for diagnostic in &mut artifact.diagnostics {
        diagnostic.waiver_reason = None;
        diagnostic.waiver_owner = None;
        diagnostic.waiver_expires_at = None;

        if let Some(waiver) = active_waivers
            .iter()
            .find(|waiver| waiver_matches(diagnostic, waiver))
        {
            diagnostic.finding_state = "waived".into();
            diagnostic.waiver_reason = Some(waiver.reason.clone());
            diagnostic.waiver_owner = Some(waiver.owner.clone());
            diagnostic.waiver_expires_at = Some(waiver.expires_at.clone());
            waived_count += 1;
        }
    }

    artifact.expired_waivers = expired_waivers;
    artifact.waiver_status = match (waived_count > 0, !artifact.expired_waivers.is_empty()) {
        (false, false) => "applied-no-match".into(),
        (true, false) => "has-waived-findings".into(),
        (false, true) => "has-expired-waivers".into(),
        (true, true) => "has-waived-and-expired".into(),
    };
    refresh_finding_summaries(artifact, artifact.expired_waivers.len());
    Ok(())
}

pub fn current_local_date() -> Result<String> {
    let output = Command::new("date")
        .args(["+%F"])
        .output()
        .into_diagnostic()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!("date +%F failed: {stderr}");
    }

    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_string())
        .map_err(|error| miette!("date output was not valid UTF-8: {error}"))
}

fn is_expired(current_date: &str, expires_at: &str) -> bool {
    current_date > expires_at
}

fn waiver_matches(diagnostic: &DiagnosticRecord, waiver: &WaiverRecord) -> bool {
    if fingerprint_for(diagnostic) != waiver.fingerprint {
        return false;
    }

    if !waiver.scope.rule_ids.is_empty()
        && !waiver
            .scope
            .rule_ids
            .iter()
            .any(|rule_id| rule_id == &diagnostic.rule_id)
    {
        return false;
    }

    if !waiver.scope.paths.is_empty()
        && !waiver
            .scope
            .paths
            .iter()
            .any(|pattern| matches_pattern(&diagnostic.path, pattern))
    {
        return false;
    }

    true
}

fn normalized_scope_paths(root_dir: &Path, paths: &[String]) -> Result<Vec<String>> {
    let mut normalized = BTreeSet::new();
    for path in paths {
        let normalized_path = if Path::new(path).is_absolute() {
            let relative = Path::new(path).strip_prefix(root_dir).map_err(|_| {
                miette!("scope path {} is outside root {}", path, root_dir.display())
            })?;
            normalize_path(&relative.to_string_lossy())
        } else {
            normalize_path(path)
        };
        normalized.insert(normalized_path);
    }
    Ok(normalized.into_iter().collect())
}

fn normalized_rule_ids(rule_ids: &[String]) -> Vec<String> {
    let mut normalized = rule_ids
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    normalized.sort();
    normalized
}

fn non_empty_field(name: &str, value: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("--{name} cannot be empty");
    }
    Ok(trimmed.to_string())
}

fn emit_add_result(result: WaiverAddResult, format: WaiverOutputFormat) {
    match format {
        WaiverOutputFormat::Text => {
            println!(
                "Recorded waiver: created={} path={} waiver_count={}",
                result.created, result.waivers_path, result.waiver_count
            );
            println!(
                "Waivers are temporary exceptions and should remain rare. owner={} expires_at={} rule={} path={} reason={}",
                result.waiver.owner,
                result.waiver.expires_at,
                result.waiver.fingerprint.rule_id,
                result.waiver.fingerprint.path,
                result.waiver.reason
            );
        }
        WaiverOutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&result).expect("waiver result should serialize")
            );
        }
    }
}

fn display_path(path: &Path) -> Result<String> {
    let current_dir = std::env::current_dir().into_diagnostic()?;
    let display = match path.strip_prefix(&current_dir) {
        Ok(relative) => relative.to_path_buf(),
        Err(_) => path.to_path_buf(),
    };
    Ok(normalize_path(&display.to_string_lossy()))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        WAIVER_SCHEMA_VERSION, WaiverFile, WaiverRecord, WaiverScope, apply_waivers,
        read_waiver_file, read_waiver_file_or_default, write_waiver_file,
    };
    use crate::baseline::{apply_baseline, create_baseline_from_artifact, fingerprint_for};
    use crate::reporters::{
        DiagnosticRecord, DiagnosticsArtifact, Problem, build_diagnostics_artifact,
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

    fn sample_artifact() -> DiagnosticsArtifact {
        build_diagnostics_artifact(
            &[
                Problem::missing_review(
                    "docs/api.md".into(),
                    "repo-rule".into(),
                    ".docpact/config.yaml".into(),
                    "review_or_update".into(),
                    "required_doc_not_touched".into(),
                    "touch_required_doc".into(),
                    vec!["src/index.ts".into()],
                    "repo rationale".into(),
                    "missing review".into(),
                ),
                Problem::uncovered_change("src/payments/charge.ts".into()),
            ],
            &["src/index.ts".into(), "src/payments/charge.ts".into()],
            2,
        )
    }

    fn waiver_for(diagnostic: &DiagnosticRecord, expires_at: &str) -> WaiverRecord {
        WaiverRecord {
            fingerprint: fingerprint_for(diagnostic),
            reason: "legacy migration".into(),
            owner: "docs-team".into(),
            expires_at: expires_at.into(),
            scope: WaiverScope::default(),
        }
    }

    #[test]
    fn waiver_round_trip_preserves_schema() {
        let root = temp_dir("docpact-waiver-roundtrip");
        let path = root.join(".docpact/waivers.yaml");
        let artifact = sample_artifact();
        let file = WaiverFile {
            schema_version: WAIVER_SCHEMA_VERSION.into(),
            tool_name: "docpact".into(),
            tool_version: "0.1.0".into(),
            waiver_count: 1,
            waivers: vec![waiver_for(&artifact.diagnostics[0], "2026-05-01")],
        };

        write_waiver_file(&path, &file).expect("waiver file should write");
        let restored = read_waiver_file(&path).expect("waiver file should read");

        assert_eq!(restored, file);
    }

    #[test]
    fn read_waiver_file_or_default_creates_empty_file_model() {
        let root = temp_dir("docpact-waiver-default");
        let path = root.join(".docpact/waivers.yaml");
        let file = read_waiver_file_or_default(&path).expect("default waiver file should load");

        assert_eq!(file.schema_version, WAIVER_SCHEMA_VERSION);
        assert_eq!(file.waiver_count, 0);
        assert!(file.waivers.is_empty());
    }

    #[test]
    fn apply_waivers_marks_findings_and_reports_expired_entries() {
        let mut artifact = sample_artifact();
        let file = WaiverFile {
            schema_version: WAIVER_SCHEMA_VERSION.into(),
            tool_name: "docpact".into(),
            tool_version: "0.1.0".into(),
            waiver_count: 2,
            waivers: vec![
                waiver_for(&artifact.diagnostics[0], "2026-05-01"),
                waiver_for(&artifact.diagnostics[1], "2026-04-20"),
            ],
        };

        apply_waivers(&mut artifact, &file, "2026-04-21").expect("waivers should apply");

        assert_eq!(artifact.waiver_status, "has-waived-and-expired");
        assert_eq!(artifact.waiver_summary.waived_count, 1);
        assert_eq!(artifact.waiver_summary.expired_count, 1);
        assert_eq!(artifact.baseline_summary.active_count, 1);
        assert_eq!(artifact.diagnostics[0].finding_state, "waived");
        assert_eq!(
            artifact.diagnostics[0].waiver_owner.as_deref(),
            Some("docs-team")
        );
        assert_eq!(artifact.expired_waivers.len(), 1);
        assert_eq!(artifact.expired_waivers[0].path, "src/payments/charge.ts");
    }

    #[test]
    fn waiver_overrides_baseline_state() {
        let mut artifact = sample_artifact();
        let baseline = create_baseline_from_artifact(&artifact).expect("baseline should build");
        apply_baseline(&mut artifact, &baseline);
        let file = WaiverFile {
            schema_version: WAIVER_SCHEMA_VERSION.into(),
            tool_name: "docpact".into(),
            tool_version: "0.1.0".into(),
            waiver_count: 1,
            waivers: vec![waiver_for(&artifact.diagnostics[0], "2026-05-01")],
        };

        apply_waivers(&mut artifact, &file, "2026-04-21").expect("waivers should apply");

        assert_eq!(artifact.diagnostics[0].finding_state, "waived");
        assert_eq!(artifact.baseline_summary.suppressed_count, 1);
        assert_eq!(artifact.waiver_summary.waived_count, 1);
    }

    #[test]
    fn scope_does_not_expand_matches() {
        let mut artifact = sample_artifact();
        let mut waiver = waiver_for(&artifact.diagnostics[0], "2026-05-01");
        waiver.scope.paths = vec!["docs/other.md".into()];
        let file = WaiverFile {
            schema_version: WAIVER_SCHEMA_VERSION.into(),
            tool_name: "docpact".into(),
            tool_version: "0.1.0".into(),
            waiver_count: 1,
            waivers: vec![waiver],
        };

        apply_waivers(&mut artifact, &file, "2026-04-21").expect("waivers should apply");

        assert_eq!(artifact.waiver_status, "applied-no-match");
        assert_eq!(artifact.diagnostics[0].finding_state, "active");
    }
}
