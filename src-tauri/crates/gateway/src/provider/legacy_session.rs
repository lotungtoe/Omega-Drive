use std::path::Path;

#[derive(Debug, Clone)]
pub struct LegacyDcOption {
    pub id: i32,
    pub ipv4: String,
    pub ipv6: String,
    pub auth_key: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct LegacyPeerInfo {
    pub peer_id: i64,
    pub hash: Option<i64>,
    pub subtype: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct LegacyChannelState {
    pub id: i64,
    pub pts: i64,
}

#[derive(Debug, Clone)]
pub struct LegacySessionData {
    pub home_dc: i32,
    pub dc_options: Vec<LegacyDcOption>,
    pub peer_infos: Vec<LegacyPeerInfo>,
    pub pts: i64,
    pub qts: i64,
    pub date: i64,
    pub seq: i64,
    pub channels: Vec<LegacyChannelState>,
}

pub trait LegacySessionReader: Send + Sync {
    fn read_legacy_session(&self, path: &Path) -> Result<LegacySessionData, String>;
}
