use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use miette::{IntoDiagnostic, Result, bail, miette};

use crate::AppExit;
use crate::cli::{DiagnosticsArgs, DiagnosticsCommands, DiagnosticsShowArgs};
use crate::config::normalize_path;
use crate::reporters::{DIAGNOSTICS_SCHEMA_VERSION, DiagnosticsArtifact, emit_diagnostic_show};

pub fn run(args: DiagnosticsArgs) -> Result<AppExit> {
    match args.command {
        DiagnosticsCommands::Show(args) => show(args),
    }
}

fn show(args: DiagnosticsShowArgs) -> Result<AppExit> {
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

    emit_diagnostic_show(diagnostic, args.format);
    Ok(AppExit::Success)
}

pub fn resolve_report_output_path(root_dir: &Path, output: Option<&Path>) -> Result<PathBuf> {
    match output {
        Some(path) if path.is_absolute() => Ok(path.to_path_buf()),
        Some(path) => Ok(std::env::current_dir().into_diagnostic()?.join(path)),
        None => {
            let millis = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .into_diagnostic()?
                .as_millis();
            Ok(root_dir
                .join(".docpact")
                .join("runs")
                .join(format!("{millis}.json")))
        }
    }
}

pub fn write_diagnostics_artifact(path: &Path, artifact: &DiagnosticsArtifact) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| miette!("report path has no parent directory: {}", path.display()))?;
    fs::create_dir_all(parent).into_diagnostic()?;
    fs::write(
        path,
        serde_json::to_string_pretty(artifact).expect("artifact should serialize"),
    )
    .into_diagnostic()?;
    Ok(())
}

pub fn read_diagnostics_artifact(path: &Path) -> Result<DiagnosticsArtifact> {
    let text = fs::read_to_string(path).into_diagnostic()?;
    let artifact: DiagnosticsArtifact =
        serde_json::from_str(&text)
            .into_diagnostic()
            .map_err(|error| {
                miette!(
                    "{} is not a valid docpact diagnostics report. {error}",
                    path.display()
                )
            })?;

    if artifact.schema_version != DIAGNOSTICS_SCHEMA_VERSION {
        bail!(
            "unsupported diagnostics report schema `{}` in {}",
            artifact.schema_version,
            path.display()
        );
    }

    Ok(artifact)
}

pub fn display_report_path(path: &Path) -> Result<String> {
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
        DiagnosticsArtifact, display_report_path, read_diagnostics_artifact,
        resolve_report_output_path, write_diagnostics_artifact,
    };
    use crate::reporters::build_diagnostics_artifact;

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
            &[crate::reporters::Problem::missing_review(
                "docs/api.md".into(),
                "repo-rule".into(),
                ".docpact/config.yaml".into(),
                "review_or_update".into(),
                "required_doc_not_touched".into(),
                "touch_required_doc".into(),
                vec!["src/index.ts".into()],
                "repo rationale".into(),
                "missing review".into(),
            )],
            &["src/index.ts".into()],
            1,
        )
    }

    #[test]
    fn resolve_report_output_path_defaults_under_docpact_runs() {
        let root = temp_dir("docpact-diagnostics-path");
        let path = resolve_report_output_path(&root, None).expect("default path should resolve");
        assert!(path.starts_with(root.join(".docpact").join("runs")));
        assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("json"));
    }

    #[test]
    fn write_and_read_diagnostics_artifact_round_trip() {
        let root = temp_dir("docpact-diagnostics-roundtrip");
        let path = root.join(".docpact/runs/latest.json");
        let artifact = sample_artifact();

        write_diagnostics_artifact(&path, &artifact).expect("artifact should be written");
        let restored = read_diagnostics_artifact(&path).expect("artifact should be readable");

        assert_eq!(restored, artifact);
    }

    #[test]
    fn display_report_path_prefers_current_dir_relative_paths() {
        let current_dir = std::env::current_dir().expect("cwd should resolve");
        let nested = current_dir.join(".docpact/runs/latest.json");
        let display = display_report_path(&nested).expect("display path should resolve");
        assert_eq!(display, ".docpact/runs/latest.json");
    }
}
