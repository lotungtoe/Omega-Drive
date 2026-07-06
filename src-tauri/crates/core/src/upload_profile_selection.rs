use std::collections::HashSet;

use crate::file_types::FileType;
use crate::upload_rules::{normalize_extensions, verify_upload_rule, UploadProfileRule};

#[derive(Debug, Clone)]
pub struct UploadProfileCandidate {
    pub file_type: FileType,
    pub extension: Option<String>,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UploadProfileSelection {
    pub profile_id: Option<i64>,
    pub rule_id: Option<i64>,
}

pub fn prepare_upload_rules(
    rules: Vec<UploadProfileRule>,
    valid_profile_ids: &HashSet<i64>,
) -> Vec<UploadProfileRule> {
    let mut prepared = Vec::new();
    for mut rule in rules {
        if !valid_profile_ids.contains(&rule.profile_id) {
            continue;
        }
        rule.extensions = normalize_extensions(&rule.extensions);
        prepared.push(rule);
    }
    prepared
}

pub fn select_upload_profile(
    candidate: &UploadProfileCandidate,
    rules: &[UploadProfileRule],
    default_profile_id: Option<i64>,
) -> UploadProfileSelection {
    let mut best_score: Option<i64> = None;
    let mut best_rule_id: Option<i64> = None;
    let mut best_profile_id: Option<i64> = None;
    for rule in rules {
        if let Some(specificity) = verify_upload_rule(rule, candidate.file_type, candidate.extension.as_deref(), candidate.size_bytes) {
            let score = rule.priority * 1000 + specificity;
            if should_take_rule(score, rule.id, best_score, best_rule_id) {
                best_score = Some(score);
                best_rule_id = rule.id;
                best_profile_id = Some(rule.profile_id);
            }
        }
    }
    UploadProfileSelection {
        profile_id: best_profile_id.or(default_profile_id),
        rule_id: best_rule_id,
    }
}

fn should_take_rule(
    next_score: i64,
    next_rule_id: Option<i64>,
    best_score: Option<i64>,
    best_rule_id: Option<i64>,
) -> bool {
    match best_score {
        None => true,
        Some(current_score) => {
            if next_score > current_score {
                true
            } else if next_score == current_score {
                match (best_rule_id, next_rule_id) {
                    (Some(current_rule_id), Some(new_rule_id)) => new_rule_id < current_rule_id,
                    (None, Some(_)) => true,
                    _ => false,
                }
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use super::*;
    use crate::upload_rules::UploadProfileRule;

    fn sample_rule(id: Option<i64>, profile_id: i64, priority: i64, extensions: Vec<&str>) -> UploadProfileRule {
        UploadProfileRule {
            id,
            profile_id,
            priority,
            file_type: Some(FileType::Video),
            extensions: extensions.into_iter().map(str::to_string).collect(),
            min_size_bytes: None,
            max_size_bytes: None,
        }
    }

    #[test]
    fn prepare_upload_rules_filters_invalid_profiles_and_normalizes_extensions() {
        let rules = vec![
            sample_rule(Some(2), 10, 1, vec![".MP4", "mp4", ""]),
            sample_rule(Some(3), 99, 1, vec!["mkv"]),
        ];
        let valid_profile_ids = HashSet::from([10]);
        let prepared = prepare_upload_rules(rules, &valid_profile_ids);
        assert_eq!(prepared.len(), 1);
        assert_eq!(prepared[0].profile_id, 10);
        assert_eq!(prepared[0].extensions, vec!["mp4".to_string()]);
    }

    #[test]
    fn select_upload_profile_tie_breaks_on_lower_rule_id() {
        let candidate = UploadProfileCandidate { file_type: FileType::Video, extension: Some("mp4".to_string()), size_bytes: 1024 };
        let rules = vec![
            sample_rule(Some(5), 10, 1, vec!["mp4"]),
            sample_rule(Some(3), 20, 1, vec!["mp4"]),
        ];
        let selected = select_upload_profile(&candidate, &rules, None);
        assert_eq!(selected, UploadProfileSelection { profile_id: Some(20), rule_id: Some(3) });
    }

    #[test]
    fn select_upload_profile_falls_back_to_default_profile() {
        let candidate = UploadProfileCandidate { file_type: FileType::Unknown, extension: None, size_bytes: 512 };
        let selected = select_upload_profile(&candidate, &[], Some(77));
        assert_eq!(selected, UploadProfileSelection { profile_id: Some(77), rule_id: None });
    }
}
