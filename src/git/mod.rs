use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

use miette::{IntoDiagnostic, Result, bail, miette};

use crate::cli::CheckArgs;
use crate::config::normalize_path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSource {
    Files,
    Range,
    Staged,
    Worktree,
    MergeBase,
}

pub fn get_changed_paths(root_dir: &Path, args: &CheckArgs) -> Result<Vec<String>> {
    if let Some(files) = &args.files {
        let values = files
            .split(',')
            .map(|value| normalize_path(value.trim()))
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        return Ok(dedup(values));
    }

    if args.staged {
        return git_name_only(root_dir, &["diff", "--name-only", "--cached"]);
    }

    if args.worktree {
        return git_name_only(root_dir, &["diff", "--name-only"]);
    }

    if let Some(reference) = &args.merge_base {
        let merge_base = git_stdout(root_dir, &["merge-base", "HEAD", reference])?;
        return git_name_only(root_dir, &["diff", "--name-only", &format!("{merge_base}...HEAD")]);
    }

    if let (Some(base), Some(head)) = (&args.base, &args.head) {
        return git_name_only(root_dir, &["diff", "--name-only", &format!("{base}...{head}")]);
    }

    bail!("Pass either --files, --staged, --worktree, --merge-base <ref>, or both --base and --head.")
}

fn git_name_only(root_dir: &Path, args: &[&str]) -> Result<Vec<String>> {
    let output = git_stdout(root_dir, args)?;
    Ok(dedup(
        output
            .lines()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(normalize_path)
            .collect::<Vec<_>>(),
    ))
}

fn git_stdout(root_dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root_dir)
        .output()
        .into_diagnostic()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(miette!("git {} failed: {}", args.join(" "), stderr));
    }

    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_string())
        .map_err(|error| miette!("git output was not valid UTF-8: {error}"))
}

fn dedup(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    for value in values {
        if seen.insert(value.clone()) {
            result.push(value);
        }
    }

    result
}
