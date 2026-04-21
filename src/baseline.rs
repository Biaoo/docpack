use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use miette::{IntoDiagnostic, Result, bail, miette};
use serde::{Deserialize, Serialize};

use crate::AppExit;
use crate::cli::{BaselineArgs, BaselineCommands, BaselineCreateArgs};
use crate::diagnostics::read_diagnostics_artifact;
use crate::reporters::{BaselineSummary, DiagnosticRecord, DiagnosticsArtifact};

pub const BASELINE_SCHEMA_VERSION: &str = "docpact.baseline.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaselineFile {
    pub schema_version: String,
    pub tool_name: String,
    pub tool_version: String,
    pub generated_at: String,
    pub fingerprint_count: usize,
    pub fingerprints: Vec<BaselineFingerprint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct BaselineFingerprint {
    #[serde(rename = "type")]
    pub problem_type: String,
    pub path: String,
    pub rule_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_mode: Option<String>,
    pub failure_reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_source: Option<String>,
}

pub fn run(args: BaselineArgs) -> Result<AppExit> {
    match args.command {
        BaselineCommands::Create(args) => create(args),
    }
}

fn create(args: BaselineCreateArgs) -> Result<AppExit> {
    let artifact = read_diagnostics_artifact(&args.report)?;
    let baseline = create_baseline_from_artifact(&artifact)?;
    write_baseline_file(&args.output, &baseline)?;
    println!(
        "Docpact baseline created: path={} fingerprint_count={}",
        display_path(&args.output)?,
        baseline.fingerprint_count,
    );
    Ok(AppExit::Success)
}

pub fn create_baseline_from_artifact(artifact: &DiagnosticsArtifact) -> Result<BaselineFile> {
    let mut fingerprints = artifact
        .diagnostics
        .iter()
        .map(fingerprint_for)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    fingerprints.sort();

    Ok(BaselineFile {
        schema_version: BASELINE_SCHEMA_VERSION.into(),
        tool_name: env!("CARGO_PKG_NAME").into(),
        tool_version: env!("CARGO_PKG_VERSION").into(),
        generated_at: generated_at_string()?,
        fingerprint_count: fingerprints.len(),
        fingerprints,
    })
}

pub fn fingerprint_for(diagnostic: &DiagnosticRecord) -> BaselineFingerprint {
    BaselineFingerprint {
        problem_type: diagnostic.problem_type.clone(),
        path: diagnostic.path.clone(),
        rule_id: diagnostic.rule_id.clone(),
        required_mode: diagnostic.required_mode.clone(),
        failure_reason: diagnostic.failure_reason.clone(),
        rule_source: diagnostic.rule_source.clone(),
    }
}

pub fn read_baseline_file(path: &Path) -> Result<BaselineFile> {
    let text = fs::read_to_string(path).into_diagnostic()?;
    let baseline: BaselineFile =
        serde_json::from_str(&text)
            .into_diagnostic()
            .map_err(|error| {
                miette!(
                    "{} is not a valid docpact baseline file. {error}",
                    path.display()
                )
            })?;

    if baseline.schema_version != BASELINE_SCHEMA_VERSION {
        bail!(
            "unsupported baseline schema `{}` in {}",
            baseline.schema_version,
            path.display()
        );
    }

    Ok(baseline)
}

pub fn write_baseline_file(path: &Path, baseline: &BaselineFile) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| miette!("baseline path has no parent directory: {}", path.display()))?;
    fs::create_dir_all(parent).into_diagnostic()?;
    fs::write(
        path,
        serde_json::to_string_pretty(baseline).expect("baseline should serialize"),
    )
    .into_diagnostic()?;
    Ok(())
}

pub fn apply_baseline(artifact: &mut DiagnosticsArtifact, baseline: &BaselineFile) {
    let fingerprints = baseline
        .fingerprints
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut suppressed_count = 0usize;
    for diagnostic in &mut artifact.diagnostics {
        if fingerprints.contains(&fingerprint_for(diagnostic)) {
            diagnostic.baseline_state = "suppressed_by_baseline".into();
            suppressed_count += 1;
        } else {
            diagnostic.baseline_state = "active".into();
        }
    }

    let active_count = artifact.diagnostics.len().saturating_sub(suppressed_count);
    artifact.baseline_summary = BaselineSummary {
        active_count,
        suppressed_count,
    };
    artifact.baseline_status = if suppressed_count == 0 {
        "applied-no-match".into()
    } else {
        "has-suppressed-findings".into()
    };
    artifact.status = if active_count == 0 {
        "ok".into()
    } else {
        "fail".into()
    };
}

fn generated_at_string() -> Result<String> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .into_diagnostic()?
        .as_secs();
    Ok(seconds.to_string())
}

fn display_path(path: &Path) -> Result<String> {
    let current_dir = std::env::current_dir().into_diagnostic()?;
    let display = match path.strip_prefix(&current_dir) {
        Ok(relative) => relative.to_path_buf(),
        Err(_) => path.to_path_buf(),
    };
    Ok(crate::config::normalize_path(&display.to_string_lossy()))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        BASELINE_SCHEMA_VERSION, apply_baseline, create_baseline_from_artifact, fingerprint_for,
        read_baseline_file, write_baseline_file,
    };
    use crate::reporters::{DiagnosticsArtifact, Problem, build_diagnostics_artifact};

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

    #[test]
    fn create_baseline_deduplicates_and_sorts_fingerprints() {
        let artifact = sample_artifact();
        let baseline = create_baseline_from_artifact(&artifact).expect("baseline should build");

        assert_eq!(baseline.schema_version, BASELINE_SCHEMA_VERSION);
        assert_eq!(baseline.fingerprint_count, 2);
        assert_eq!(baseline.fingerprints[0].path, "docs/api.md");
        assert_eq!(baseline.fingerprints[1].path, "src/payments/charge.ts");
    }

    #[test]
    fn baseline_round_trip_preserves_schema() {
        let root = temp_dir("docpact-baseline-roundtrip");
        let path = root.join(".docpact/baseline.json");
        let artifact = sample_artifact();
        let baseline = create_baseline_from_artifact(&artifact).expect("baseline should build");

        write_baseline_file(&path, &baseline).expect("baseline should write");
        let restored = read_baseline_file(&path).expect("baseline should read");

        assert_eq!(restored, baseline);
    }

    #[test]
    fn apply_baseline_marks_suppressed_findings_without_erasing_them() {
        let mut artifact = sample_artifact();
        let baseline = create_baseline_from_artifact(&artifact).expect("baseline should build");

        artifact.diagnostics.pop();
        let unmatched = crate::reporters::DiagnosticRecord {
            diagnostic_id: "d999".into(),
            problem_type: "missing-review".into(),
            path: "docs/extra.md".into(),
            message: "extra".into(),
            rule_id: "repo-rule".into(),
            required_mode: Some("review_or_update".into()),
            failure_reason: "required_doc_not_touched".into(),
            suggested_action: "touch_required_doc".into(),
            baseline_state: "active".into(),
            rule_source: Some(".docpact/config.yaml".into()),
            trigger_paths: vec!["src/extra.ts".into()],
            rule_reason: Some("repo rationale".into()),
        };
        artifact.diagnostics.push(unmatched.clone());

        apply_baseline(&mut artifact, &baseline);

        assert_eq!(artifact.baseline_status, "has-suppressed-findings");
        assert_eq!(artifact.baseline_summary.suppressed_count, 1);
        assert_eq!(artifact.baseline_summary.active_count, 1);
        assert_eq!(artifact.diagnostics.len(), 2);
        assert_eq!(
            artifact.diagnostics[0].baseline_state,
            "suppressed_by_baseline"
        );
        assert_eq!(artifact.diagnostics[1].baseline_state, "active");
        assert_eq!(artifact.status, "fail");
        assert_eq!(
            fingerprint_for(&artifact.diagnostics[1]),
            fingerprint_for(&unmatched)
        );
    }
}
