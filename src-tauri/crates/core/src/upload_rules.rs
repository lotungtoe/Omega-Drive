pub use omega_drive_gateway::upload::upload_rules::UploadProfileRule;

use crate::file_types::normalize_extension;

pub fn normalize_extensions(list: &[String]) -> Vec<String> {
    let mut out: Vec<String> = list.iter().filter_map(|ext| normalize_extension(ext)).collect();
    out.sort();
    out.dedup();
    out
}

pub fn verify_upload_rule(
    rule: &UploadProfileRule,
    file_type: crate::file_types::FileType,
    extension: Option<&str>,
    size_bytes: i64,
) -> Option<i64> {
    if let Some(rt) = rule.file_type {
        if rt != file_type {
            return None;
        }
    }

    if !rule.extensions.is_empty() {
        let ext = extension.unwrap_or("");
        if !rule.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
            return None;
        }
    }

    if let Some(min) = rule.min_size_bytes {
        if size_bytes < min {
            return None;
        }
    }
    if let Some(max) = rule.max_size_bytes {
        if size_bytes > max {
            return None;
        }
    }

    let mut specificity = 0i64;
    if !rule.extensions.is_empty() {
        specificity += 500;
    }
    if rule.file_type.is_some() {
        specificity += 200;
    }
    if rule.min_size_bytes.is_some() || rule.max_size_bytes.is_some() {
        specificity += 100;
    }
    Some(specificity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_types::FileType;

    #[test]
    fn ext_wins_when_priority_equal() {
        let rule_type = UploadProfileRule { id: Some(1), profile_id: 1, priority: 10, file_type: Some(FileType::Video), extensions: vec![], min_size_bytes: None, max_size_bytes: None };
        let rule_ext = UploadProfileRule { id: Some(2), profile_id: 2, priority: 10, file_type: None, extensions: vec!["mp4".to_string()], min_size_bytes: None, max_size_bytes: None };
        let spec_type = verify_upload_rule(&rule_type, FileType::Video, Some("mp4"), 100).unwrap();
        let spec_ext = verify_upload_rule(&rule_ext, FileType::Video, Some("mp4"), 100).unwrap();
        let score_type = rule_type.priority * 1000 + spec_type;
        let score_ext = rule_ext.priority * 1000 + spec_ext;
        assert!(score_ext > score_type);
    }
}
