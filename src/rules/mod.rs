use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::config::{LoadedRule, normalize_path, resolve_rule_path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequiredDocMode {
    ReviewOrUpdate,
    MetadataRefreshRequired,
    BodyUpdateRequired,
    MustExist,
}

impl RequiredDocMode {
    pub fn from_option(value: Option<&str>) -> Self {
        match value {
            Some("metadata_refresh_required") => Self::MetadataRefreshRequired,
            Some("body_update_required") => Self::BodyUpdateRequired,
            Some("must_exist") => Self::MustExist,
            Some("review_or_update") | None | Some(_) => Self::ReviewOrUpdate,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReviewOrUpdate => "review_or_update",
            Self::MetadataRefreshRequired => "metadata_refresh_required",
            Self::BodyUpdateRequired => "body_update_required",
            Self::MustExist => "must_exist",
        }
    }
}

impl fmt::Display for RequiredDocMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedRule {
    pub changed_path: String,
    pub source: String,
    pub base_dir: String,
    pub rule: crate::config::Rule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedDoc {
    pub path: String,
    pub rules: BTreeSet<String>,
    pub changed_paths: BTreeSet<String>,
    pub modes: BTreeSet<RequiredDocMode>,
}

pub fn matches_pattern(file_path: &str, pattern: &str) -> bool {
    let file_path = normalize_path(file_path);
    let pattern = normalize_path(pattern);
    let file_bytes = file_path.as_bytes();
    let pattern_bytes = pattern.as_bytes();
    let mut memo = BTreeMap::new();
    matches_recursive(file_bytes, pattern_bytes, 0, 0, &mut memo)
}

fn matches_recursive(
    file: &[u8],
    pattern: &[u8],
    file_index: usize,
    pattern_index: usize,
    memo: &mut BTreeMap<(usize, usize), bool>,
) -> bool {
    if let Some(value) = memo.get(&(file_index, pattern_index)) {
        return *value;
    }

    let matched = if pattern_index == pattern.len() {
        file_index == file.len()
    } else {
        match pattern[pattern_index] {
            b'*' if pattern_index + 1 < pattern.len() && pattern[pattern_index + 1] == b'*' => {
                let next_pattern = pattern_index + 2;
                (file_index..=file.len()).any(|candidate| {
                    matches_recursive(file, pattern, candidate, next_pattern, memo)
                })
            }
            b'*' => {
                let next_pattern = pattern_index + 1;
                let mut candidate = file_index;
                loop {
                    if matches_recursive(file, pattern, candidate, next_pattern, memo) {
                        break true;
                    }
                    if candidate == file.len() || file[candidate] == b'/' {
                        break false;
                    }
                    candidate += 1;
                }
            }
            b'?' => {
                file_index < file.len()
                    && file[file_index] != b'/'
                    && matches_recursive(file, pattern, file_index + 1, pattern_index + 1, memo)
            }
            byte => {
                file_index < file.len()
                    && file[file_index] == byte
                    && matches_recursive(file, pattern, file_index + 1, pattern_index + 1, memo)
            }
        }
    };

    memo.insert((file_index, pattern_index), matched);
    matched
}

pub fn match_rules(changed_paths: &[String], loaded_rules: &[LoadedRule]) -> Vec<MatchedRule> {
    let mut matches = Vec::new();

    for changed_path in changed_paths {
        for loaded in loaded_rules {
            if loaded.rule.triggers.iter().any(|trigger| {
                matches_pattern(
                    changed_path,
                    &resolve_rule_path(&loaded.base_dir, &trigger.path),
                )
            }) {
                matches.push(MatchedRule {
                    changed_path: changed_path.clone(),
                    source: loaded.source.clone(),
                    base_dir: loaded.base_dir.clone(),
                    rule: loaded.rule.clone(),
                });
            }
        }
    }

    matches
}

pub fn collect_expected_docs(matches: &[MatchedRule]) -> BTreeMap<String, ExpectedDoc> {
    let mut expected = BTreeMap::new();

    for matched in matches {
        for doc in &matched.rule.required_docs {
            let full_path = resolve_rule_path(&matched.base_dir, &doc.path);
            let entry = expected
                .entry(full_path.clone())
                .or_insert_with(|| ExpectedDoc {
                    path: full_path,
                    rules: BTreeSet::new(),
                    changed_paths: BTreeSet::new(),
                    modes: BTreeSet::new(),
                });

            entry.rules.insert(matched.rule.id.clone());
            entry.changed_paths.insert(matched.changed_path.clone());
            entry
                .modes
                .insert(RequiredDocMode::from_option(doc.mode.as_deref()));
        }
    }

    expected
}

#[cfg(test)]
mod tests {
    use crate::config::{LoadedRule, RequiredDoc, Rule, Trigger};

    use super::{RequiredDocMode, collect_expected_docs, match_rules, matches_pattern};

    #[test]
    fn glob_matching_supports_repo_relative_paths() {
        assert!(matches_pattern(
            "tiangong-lca-next/config/routes.ts",
            "tiangong-lca-next/**"
        ));
        assert!(matches_pattern(
            ".docpact/quality-rubric.md",
            ".docpact/*.md"
        ));
        assert!(!matches_pattern(".docpact/nested/file.md", ".docpact/*.md"));
    }

    #[test]
    fn matching_and_expected_docs_resolve_repo_paths() {
        let loaded = vec![
            LoadedRule {
                source: ".docpact/config.yaml".into(),
                base_dir: String::new(),
                rule: Rule {
                    id: "root-rule".into(),
                    scope: "workspace".into(),
                    repo: "workspace".into(),
                    triggers: vec![Trigger {
                        path: "AGENTS.md".into(),
                        kind: Some("doc-contract".into()),
                    }],
                    required_docs: vec![RequiredDoc {
                        path: ".docpact/config.yaml".into(),
                        mode: Some("review_or_update".into()),
                    }],
                    reason: "root".into(),
                },
            },
            LoadedRule {
                source: "subrepo/.docpact/config.yaml".into(),
                base_dir: "subrepo".into(),
                rule: Rule {
                    id: "repo-rule".into(),
                    scope: "repo".into(),
                    repo: "subrepo".into(),
                    triggers: vec![Trigger {
                        path: "src/**".into(),
                        kind: Some("code".into()),
                    }],
                    required_docs: vec![RequiredDoc {
                        path: ".docpact/config.yaml".into(),
                        mode: Some("review_or_update".into()),
                    }],
                    reason: "repo".into(),
                },
            },
        ];

        let matches = match_rules(
            &["AGENTS.md".into(), "subrepo/src/index.ts".into()],
            &loaded,
        );
        let expected = collect_expected_docs(&matches);

        assert_eq!(matches.len(), 2);
        assert_eq!(
            expected.keys().cloned().collect::<Vec<_>>(),
            vec![
                ".docpact/config.yaml".to_string(),
                "subrepo/.docpact/config.yaml".to_string()
            ]
        );
        assert_eq!(
            expected[".docpact/config.yaml"].modes,
            [RequiredDocMode::ReviewOrUpdate].into_iter().collect()
        );
    }
}
