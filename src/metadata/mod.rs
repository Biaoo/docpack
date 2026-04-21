use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use miette::{IntoDiagnostic, Result};
use yaml_serde::Value;

use crate::config::{load_yaml_value, normalize_path};
use crate::reporters::Problem;

pub const DEFAULT_MARKDOWN_METADATA_KEYS: &[&str] = &[
    "docType",
    "scope",
    "status",
    "authoritative",
    "owner",
    "language",
    "whenToUse",
    "whenToUpdate",
    "checkPaths",
    "lastReviewedAt",
    "lastReviewedCommit",
];

pub const DEFAULT_YAML_METADATA_KEYS: &[&str] = &["lastReviewedAt", "lastReviewedCommit"];
pub const REVIEW_METADATA_KEYS: &[&str] = &["lastReviewedAt", "lastReviewedCommit"];

fn is_under_doc_root(normalized: &str) -> bool {
    normalized.starts_with(".docpact/") || normalized.contains("/.docpact/")
}

pub fn is_key_markdown_doc(rel_path: &str) -> bool {
    let normalized = normalize_path(rel_path);
    normalized.rsplit('/').next() == Some("AGENTS.md")
        || (is_under_doc_root(&normalized) && normalized.ends_with(".md"))
}

pub fn is_key_yaml_contract(rel_path: &str) -> bool {
    let normalized = normalize_path(rel_path);
    normalized.ends_with(".yaml") && is_under_doc_root(&normalized)
}

pub fn parse_frontmatter_keys(text: &str) -> BTreeSet<String> {
    let normalized = text.replace("\r\n", "\n");
    let mut lines = normalized.lines();
    if lines.next() != Some("---") {
        return BTreeSet::new();
    }

    let mut keys = BTreeSet::new();
    for line in lines {
        if line == "---" {
            break;
        }

        let trimmed = line.trim_end();
        let mut chars = trimmed.chars().peekable();
        let Some(first) = chars.peek().copied() else {
            continue;
        };
        if !first.is_ascii_alphabetic() {
            continue;
        }

        let mut key = String::new();
        while let Some(ch) = chars.peek().copied() {
            if ch.is_ascii_alphanumeric() {
                key.push(ch);
                chars.next();
            } else {
                break;
            }
        }

        if key.is_empty() {
            continue;
        }

        while let Some(' ') = chars.peek().copied() {
            chars.next();
        }

        if chars.next() == Some(':') {
            keys.insert(key);
        }
    }

    keys
}

pub fn parse_frontmatter_scalar_values(text: &str) -> BTreeMap<String, String> {
    let normalized = text.replace("\r\n", "\n");
    let mut lines = normalized.lines();
    if lines.next() != Some("---") {
        return BTreeMap::new();
    }

    let mut values = BTreeMap::new();
    for line in lines {
        if line == "---" {
            break;
        }

        let trimmed = line.trim_end();
        let mut chars = trimmed.chars().peekable();
        let Some(first) = chars.peek().copied() else {
            continue;
        };
        if !first.is_ascii_alphabetic() {
            continue;
        }

        let mut key = String::new();
        while let Some(ch) = chars.peek().copied() {
            if ch.is_ascii_alphanumeric() {
                key.push(ch);
                chars.next();
            } else {
                break;
            }
        }

        if key.is_empty() {
            continue;
        }

        while let Some(' ') = chars.peek().copied() {
            chars.next();
        }

        if chars.next() != Some(':') {
            continue;
        }

        let value = chars.collect::<String>().trim().to_string();
        values.insert(key, value);
    }

    values
}

pub fn missing_markdown_metadata(text: &str) -> Vec<String> {
    let keys = parse_frontmatter_keys(text);
    DEFAULT_MARKDOWN_METADATA_KEYS
        .iter()
        .filter(|key| !keys.contains(**key))
        .map(|key| (*key).to_string())
        .collect()
}

pub fn missing_markdown_review_metadata(text: &str) -> Vec<String> {
    let keys = parse_frontmatter_keys(text);
    REVIEW_METADATA_KEYS
        .iter()
        .filter(|key| !keys.contains(**key))
        .map(|key| (*key).to_string())
        .collect()
}

pub fn apply_review_metadata_to_markdown(
    text: &str,
    reviewed_at: &str,
    reviewed_commit: &str,
) -> String {
    let normalized = text.replace("\r\n", "\n");
    let newline = preferred_newline(text);

    if let Some((frontmatter, body)) = split_markdown_frontmatter(&normalized) {
        let updated = upsert_review_scalar_lines(
            frontmatter,
            reviewed_at,
            reviewed_commit,
            ValueStyle::Plain,
        );
        let mut output = String::from("---\n");
        output.push_str(&updated.join("\n"));
        output.push_str("\n---");
        if body.is_empty() {
            output.push('\n');
        } else {
            output.push('\n');
            output.push_str(&body);
        }
        return restore_newlines(output, newline);
    }

    let mut output =
        format!("---\nlastReviewedAt: {reviewed_at}\nlastReviewedCommit: {reviewed_commit}\n---");
    if normalized.is_empty() {
        output.push('\n');
    } else {
        output.push_str("\n\n");
        output.push_str(&normalized);
    }
    restore_newlines(output, newline)
}

pub fn apply_review_metadata_to_yaml(
    text: &str,
    reviewed_at: &str,
    reviewed_commit: &str,
) -> String {
    let normalized = text.replace("\r\n", "\n");
    let newline = preferred_newline(text);
    let lines = normalized.lines().map(str::to_string).collect::<Vec<_>>();
    let updated =
        upsert_review_scalar_lines(lines, reviewed_at, reviewed_commit, ValueStyle::YamlQuoted);
    let mut output = updated.join("\n");
    if text.ends_with('\n') || text.ends_with("\r\n") || output.is_empty() {
        output.push('\n');
    }
    restore_newlines(output, newline)
}

pub fn missing_yaml_metadata(text: &str, source_label: &str) -> Result<Vec<String>> {
    let parsed = crate::config::parse_yaml_value(text, source_label)?;
    Ok(missing_yaml_metadata_from_value(&parsed))
}

fn missing_yaml_metadata_from_value(value: &Value) -> Vec<String> {
    let mapping = match value {
        Value::Mapping(mapping) => mapping,
        _ => {
            return DEFAULT_YAML_METADATA_KEYS
                .iter()
                .map(|key| (*key).to_string())
                .collect();
        }
    };

    DEFAULT_YAML_METADATA_KEYS
        .iter()
        .filter(|key| !mapping.contains_key(Value::String((*key).to_string())))
        .map(|key| (*key).to_string())
        .collect()
}

pub fn missing_yaml_review_metadata_from_value(value: &Value) -> Vec<String> {
    let mapping = match value {
        Value::Mapping(mapping) => mapping,
        _ => {
            return REVIEW_METADATA_KEYS
                .iter()
                .map(|key| (*key).to_string())
                .collect();
        }
    };

    REVIEW_METADATA_KEYS
        .iter()
        .filter(|key| !mapping.contains_key(Value::String((*key).to_string())))
        .map(|key| (*key).to_string())
        .collect()
}

pub fn markdown_body(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n");
    let mut lines = normalized.lines();
    if lines.next() != Some("---") {
        return normalized;
    }

    let mut offset = 4usize;
    for line in lines {
        offset += line.len() + 1;
        if line == "---" {
            return normalized[offset..].to_string();
        }
    }

    normalized
}

pub fn build_doc_problems(
    root_dir: &Path,
    changed_paths: &[String],
    governed_required_docs: &BTreeSet<String>,
) -> Result<Vec<Problem>> {
    let mut problems = Vec::new();

    for rel_path in changed_paths {
        if !governed_required_docs.contains(rel_path) {
            continue;
        }

        let abs_path = root_dir.join(rel_path);
        if !abs_path.exists() {
            continue;
        }

        if rel_path.ends_with(".md") {
            let text = fs::read_to_string(&abs_path).into_diagnostic()?;
            let missing = missing_markdown_metadata(&text);
            if !missing.is_empty() {
                problems.push(Problem::missing_metadata(
                    rel_path.clone(),
                    format!(
                        "Touched required doc is missing Markdown metadata keys: {}",
                        missing.join(", ")
                    ),
                ));
            }
            continue;
        }

        if rel_path.ends_with(".yaml") || rel_path.ends_with(".yml") {
            let parsed = load_yaml_value(&abs_path, rel_path)?;
            let missing = missing_yaml_metadata_from_value(&parsed);
            if !missing.is_empty() {
                problems.push(Problem::missing_metadata(
                    rel_path.clone(),
                    format!(
                        "Touched required doc is missing YAML metadata keys: {}",
                        missing.join(", ")
                    ),
                ));
            }
        }
    }

    Ok(problems)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueStyle {
    Plain,
    YamlQuoted,
}

fn preferred_newline(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

fn restore_newlines(text: String, newline: &str) -> String {
    if newline == "\n" {
        text
    } else {
        text.replace('\n', newline)
    }
}

fn split_markdown_frontmatter(normalized: &str) -> Option<(Vec<String>, String)> {
    if !normalized.starts_with("---\n") {
        return None;
    }

    let lines = normalized.lines().collect::<Vec<_>>();
    let end = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (*line == "---").then_some(index))?;

    let frontmatter = lines[1..end]
        .iter()
        .map(|line| (*line).to_string())
        .collect();
    let body = lines[end + 1..].join("\n");
    let body = if normalized.ends_with('\n') && !body.is_empty() {
        format!("{body}\n")
    } else {
        body
    };
    Some((frontmatter, body))
}

fn upsert_review_scalar_lines(
    mut lines: Vec<String>,
    reviewed_at: &str,
    reviewed_commit: &str,
    value_style: ValueStyle,
) -> Vec<String> {
    let mut saw_review_line = false;
    let mut last_review_line = None;

    for (index, line) in lines.iter_mut().enumerate() {
        if matches_top_level_key(line, "lastReviewedAt") {
            *line = format_review_scalar_line("lastReviewedAt", reviewed_at, value_style);
            saw_review_line = true;
            last_review_line = Some(index);
        } else if matches_top_level_key(line, "lastReviewedCommit") {
            *line = format_review_scalar_line("lastReviewedCommit", reviewed_commit, value_style);
            saw_review_line = true;
            last_review_line = Some(index);
        }
    }

    let has_reviewed_at = lines
        .iter()
        .any(|line| matches_top_level_key(line, "lastReviewedAt"));
    let has_reviewed_commit = lines
        .iter()
        .any(|line| matches_top_level_key(line, "lastReviewedCommit"));

    let mut missing = Vec::new();
    if !has_reviewed_at {
        missing.push(format_review_scalar_line(
            "lastReviewedAt",
            reviewed_at,
            value_style,
        ));
    }
    if !has_reviewed_commit {
        missing.push(format_review_scalar_line(
            "lastReviewedCommit",
            reviewed_commit,
            value_style,
        ));
    }

    if missing.is_empty() {
        return lines;
    }

    let insert_at = if saw_review_line {
        last_review_line.expect("review line index should exist") + 1
    } else {
        leading_yaml_header_block_len(&lines)
    };

    for (offset, line) in missing.into_iter().enumerate() {
        lines.insert(insert_at + offset, line);
    }

    lines
}

fn leading_yaml_header_block_len(lines: &[String]) -> usize {
    let mut index = 0usize;
    while index < lines.len() {
        let trimmed = lines[index].trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            index += 1;
        } else {
            break;
        }
    }
    index
}

fn matches_top_level_key(line: &str, key: &str) -> bool {
    let trimmed = line.trim_end();
    let Some(rest) = trimmed.strip_prefix(key) else {
        return false;
    };

    let rest = rest.trim_start();
    rest.starts_with(':')
}

fn format_review_scalar_line(key: &str, value: &str, value_style: ValueStyle) -> String {
    match value_style {
        ValueStyle::Plain => format!("{key}: {value}"),
        ValueStyle::YamlQuoted => format!("{key}: \"{value}\""),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use yaml_serde::Value;

    use super::{
        build_doc_problems, is_key_markdown_doc, markdown_body, missing_markdown_metadata,
        missing_markdown_review_metadata, missing_yaml_metadata,
        missing_yaml_review_metadata_from_value, parse_frontmatter_scalar_values,
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
    fn key_markdown_doc_excludes_yaml_contract_files() {
        assert!(is_key_markdown_doc(".docpact/quality-rubric.md"));
        assert!(is_key_markdown_doc("AGENTS.md"));
        assert!(!is_key_markdown_doc(".docpact/config.yaml"));
    }

    #[test]
    fn missing_markdown_metadata_detects_absent_frontmatter_keys() {
        let text = r#"---
title: Example
docType: contract
scope: workspace
status: draft
authoritative: false
owner: sample-workspace
language: en
whenToUse:
  - x
whenToUpdate:
  - y
checkPaths:
  - .docpact/**
lastReviewedAt: 2026-04-18
---

# Example
"#;

        assert_eq!(
            missing_markdown_metadata(text),
            vec!["lastReviewedCommit".to_string()]
        );
    }

    #[test]
    fn missing_yaml_metadata_detects_absent_top_level_review_fields() {
        let text = r#"version: 1
lastReviewedAt: "2026-04-18"
"#;
        let missing =
            missing_yaml_metadata(text, ".docpact/example.yaml").expect("yaml should parse");
        assert_eq!(missing, vec!["lastReviewedCommit".to_string()]);
    }

    #[test]
    fn parse_frontmatter_scalar_values_extracts_review_fields() {
        let text = r#"---
title: Example
lastReviewedAt: 2026-04-18
lastReviewedCommit: abc123
---

# Example
"#;

        let values = parse_frontmatter_scalar_values(text);
        assert_eq!(
            values.get("lastReviewedAt"),
            Some(&"2026-04-18".to_string())
        );
        assert_eq!(
            values.get("lastReviewedCommit"),
            Some(&"abc123".to_string())
        );
    }

    #[test]
    fn markdown_body_excludes_frontmatter() {
        let text = r#"---
title: Example
lastReviewedAt: 2026-04-18
lastReviewedCommit: abc123
---

# Example
"#;

        assert_eq!(markdown_body(text), "\n# Example\n".to_string());
    }

    #[test]
    fn missing_markdown_review_metadata_checks_only_review_keys() {
        let text = r#"---
title: Example
lastReviewedAt: 2026-04-18
---

# Example
"#;

        assert_eq!(
            missing_markdown_review_metadata(text),
            vec!["lastReviewedCommit".to_string()]
        );
    }

    #[test]
    fn missing_yaml_review_metadata_checks_only_review_keys() {
        let value = yaml_serde::from_str::<Value>("version: 1\nlastReviewedAt: 2026-04-18\n")
            .expect("yaml should parse");
        assert_eq!(
            missing_yaml_review_metadata_from_value(&value),
            vec!["lastReviewedCommit".to_string()]
        );
    }

    #[test]
    fn build_doc_problems_only_checks_governed_required_docs() {
        let root = temp_dir("docpact-metadata-governed");
        fs::create_dir_all(root.join("docs")).expect("docs dir");
        fs::write(root.join("docs/api.md"), "# API\n").expect("api doc");
        fs::write(root.join("docs/freeform.md"), "# Freeform\n").expect("freeform doc");

        let changed_paths = vec!["docs/api.md".into(), "docs/freeform.md".into()];
        let governed_required_docs = BTreeSet::from(["docs/api.md".to_string()]);

        let problems = build_doc_problems(&root, &changed_paths, &governed_required_docs)
            .expect("metadata check should succeed");

        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].path, "docs/api.md");
        assert!(problems[0].message.contains("Touched required doc"));
    }

    #[test]
    fn build_doc_problems_ignores_non_markdown_yaml_required_docs() {
        let root = temp_dir("docpact-metadata-binary");
        fs::create_dir_all(root.join("docs")).expect("docs dir");
        fs::write(root.join("docs/spec.json"), "{}\n").expect("json doc");

        let changed_paths = vec!["docs/spec.json".into()];
        let governed_required_docs = BTreeSet::from(["docs/spec.json".to_string()]);

        let problems = build_doc_problems(&root, &changed_paths, &governed_required_docs)
            .expect("metadata check should succeed");

        assert!(problems.is_empty());
    }
}
