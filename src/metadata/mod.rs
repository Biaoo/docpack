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

pub fn build_doc_problems(root_dir: &Path, changed_paths: &[String]) -> Result<Vec<Problem>> {
    let mut problems = Vec::new();

    for rel_path in changed_paths {
        let abs_path = root_dir.join(rel_path);
        if !abs_path.exists() {
            continue;
        }

        if is_key_markdown_doc(rel_path) {
            let text = fs::read_to_string(&abs_path).into_diagnostic()?;
            let missing = missing_markdown_metadata(&text);
            if !missing.is_empty() {
                problems.push(Problem::missing_metadata(
                    rel_path.clone(),
                    format!("Missing Markdown metadata keys: {}", missing.join(", ")),
                ));
            }
            continue;
        }

        if is_key_yaml_contract(rel_path) {
            let parsed = load_yaml_value(&abs_path, rel_path)?;
            let missing = missing_yaml_metadata_from_value(&parsed);
            if !missing.is_empty() {
                problems.push(Problem::missing_metadata(
                    rel_path.clone(),
                    format!("Missing YAML metadata keys: {}", missing.join(", ")),
                ));
            }
        }
    }

    Ok(problems)
}

#[cfg(test)]
mod tests {
    use yaml_serde::Value;

    use super::{
        is_key_markdown_doc, markdown_body, missing_markdown_metadata,
        missing_markdown_review_metadata, missing_yaml_metadata,
        missing_yaml_review_metadata_from_value, parse_frontmatter_scalar_values,
    };

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
}
