use serde::{Deserialize, Serialize};

pub const TENANT_SCOPE_MY: &str = "my";
pub const TENANT_SCOPE_SHARED: &str = "shared";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct TenantDescriptor {
    pub scope: String,
    pub discord_guild_id: Option<String>,
    pub telegram_group_id: Option<String>,
}

impl Default for TenantDescriptor {
    fn default() -> Self {
        Self::new(TENANT_SCOPE_MY, None, None)
    }
}

impl TenantDescriptor {
    pub fn new(
        scope: impl Into<String>,
        discord_guild_id: Option<String>,
        telegram_group_id: Option<String>,
    ) -> Self {
        Self {
            scope: normalize_scope(&scope.into()),
            discord_guild_id: normalize_id(discord_guild_id),
            telegram_group_id: normalize_id(telegram_group_id),
        }
    }

    pub fn key(&self) -> String {
        format!(
            "{}__{}__{}",
            self.scope,
            self.discord_segment(),
            self.telegram_segment()
        )
    }

    pub fn db_file_name(&self) -> String {
        format!("{}.db", self.key())
    }

    pub fn display_name(&self) -> String {
        format!(
            "{} / Discord {} / Telegram {}",
            self.scope,
            self.discord_segment(),
            self.telegram_segment()
        )
    }

    pub fn discord_segment(&self) -> String {
        self.discord_guild_id
            .clone()
            .unwrap_or_else(|| "0".to_string())
    }

    pub fn telegram_segment(&self) -> String {
        self.telegram_group_id
            .clone()
            .unwrap_or_else(|| "0".to_string())
    }

    pub fn from_db_file_name(name: &str) -> Option<Self> {
        let stem = name.strip_suffix(".db")?;
        let mut parts = stem.split("__");
        let scope = parts.next()?;
        let discord = parts.next()?;
        let telegram = parts.next()?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self::new(
            scope,
            denormalize_id(discord),
            denormalize_id(telegram),
        ))
    }
}

fn normalize_scope(scope: &str) -> String {
    match scope.trim().to_ascii_lowercase().as_str() {
        TENANT_SCOPE_SHARED => TENANT_SCOPE_SHARED.to_string(),
        _ => TENANT_SCOPE_MY.to_string(),
    }
}

fn normalize_id(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && value != "0")
}

fn denormalize_id(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "0" {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::TenantDescriptor;

    #[test]
    fn tenant_db_name_roundtrips() {
        let tenant = TenantDescriptor::new("shared", Some("123".into()), Some("456".into()));
        let file_name = tenant.db_file_name();
        let parsed = TenantDescriptor::from_db_file_name(&file_name).expect("parse tenant");
        assert_eq!(parsed, tenant);
    }

    #[test]
    fn zero_segments_become_none() {
        let parsed = TenantDescriptor::from_db_file_name("my__0__0.db").expect("parse tenant");
        assert_eq!(parsed.scope, "my");
        assert_eq!(parsed.discord_guild_id, None);
        assert_eq!(parsed.telegram_group_id, None);
    }
}
