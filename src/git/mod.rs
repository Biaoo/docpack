use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use miette::{IntoDiagnostic, Result, bail, miette};

use crate::cli::LintArgs;
use crate::config::normalize_path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSource {
    Files,
    Range,
    Staged,
    Worktree,
    MergeBase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileComparison {
    pub previous: Option<String>,
    pub current: Option<String>,
}

pub fn get_changed_paths(root_dir: &Path, args: &LintArgs) -> Result<Vec<String>> {
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
        return git_name_only(
            root_dir,
            &["diff", "--name-only", &format!("{merge_base}...HEAD")],
        );
    }

    if let (Some(base), Some(head)) = (&args.base, &args.head) {
        return git_name_only(
            root_dir,
            &["diff", "--name-only", &format!("{base}...{head}")],
        );
    }

    bail!(
        "Pass either --files, --staged, --worktree, --merge-base <ref>, or both --base and --head."
    )
}

pub fn get_head_commit(root_dir: &Path) -> Result<String> {
    git_stdout(root_dir, &["rev-parse", "HEAD"])
}

pub fn get_tracked_paths(root_dir: &Path) -> Result<Vec<String>> {
    git_name_only(root_dir, &["ls-files"])
}

pub fn get_file_comparison(
    root_dir: &Path,
    args: &LintArgs,
    rel_path: &str,
) -> Result<FileComparison> {
    let rel_path = normalize_path(rel_path);

    if args.staged {
        return Ok(FileComparison {
            previous: git_show_revision_path(root_dir, "HEAD", &rel_path)?,
            current: git_show_index_path(root_dir, &rel_path)?,
        });
    }

    if args.worktree || args.files.is_some() {
        return Ok(FileComparison {
            previous: git_show_revision_path(root_dir, "HEAD", &rel_path)?,
            current: read_worktree_path(root_dir, &rel_path)?,
        });
    }

    if let Some(reference) = &args.merge_base {
        let merge_base = git_stdout(root_dir, &["merge-base", "HEAD", reference])?;
        return Ok(FileComparison {
            previous: git_show_revision_path(root_dir, &merge_base, &rel_path)?,
            current: git_show_revision_path(root_dir, "HEAD", &rel_path)?,
        });
    }

    if let (Some(base), Some(head)) = (&args.base, &args.head) {
        let merge_base = git_stdout(root_dir, &["merge-base", base, head])?;
        return Ok(FileComparison {
            previous: git_show_revision_path(root_dir, &merge_base, &rel_path)?,
            current: git_show_revision_path(root_dir, head, &rel_path)?,
        });
    }

    bail!(
        "Pass either --files, --staged, --worktree, --merge-base <ref>, or both --base and --head."
    )
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

fn git_show_revision_path(
    root_dir: &Path,
    revision: &str,
    rel_path: &str,
) -> Result<Option<String>> {
    git_show_spec(root_dir, &format!("{revision}:{rel_path}"))
}

fn git_show_index_path(root_dir: &Path, rel_path: &str) -> Result<Option<String>> {
    git_show_spec(root_dir, &format!(":{rel_path}"))
}

fn git_show_spec(root_dir: &Path, spec: &str) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["show", spec])
        .current_dir(root_dir)
        .output()
        .into_diagnostic()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if is_missing_path_error(&stderr) {
            return Ok(None);
        }
        return Err(miette!("git show {} failed: {}", spec, stderr));
    }

    String::from_utf8(output.stdout)
        .map(Some)
        .map_err(|error| miette!("git output was not valid UTF-8: {error}"))
}

fn read_worktree_path(root_dir: &Path, rel_path: &str) -> Result<Option<String>> {
    let abs_path = root_dir.join(rel_path);
    if !abs_path.exists() {
        return Ok(None);
    }

    fs::read_to_string(abs_path).map(Some).into_diagnostic()
}

fn is_missing_path_error(stderr: &str) -> bool {
    stderr.contains("does not exist in")
        || stderr.contains("exists on disk, but not in")
        || stderr.contains("path '") && stderr.contains("not in the index")
        || stderr.contains("unknown revision or path not in the working tree")
        || stderr.contains("bad revision")
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
