use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use anyhow::Context;
use futures_util::future::BoxFuture;
use grammers_session::{
    types::{
        ChannelKind, ChannelState, DcOption, PeerAuth, PeerId, PeerInfo, PeerKind, UpdateState,
        UpdatesState,
    },
    Session, SessionData,
};
use omega_drive_gateway::provider::legacy_session::LegacySessionReader;

static LEGACY_READER: OnceLock<Box<dyn LegacySessionReader + Send + Sync>> = OnceLock::new();

pub fn init_legacy_reader(reader: Box<dyn LegacySessionReader + Send + Sync>) {
    let _ = LEGACY_READER.set(reader);
}

pub(crate) const TELEGRAM_SESSION_FILE_NAME: &str = "tg.session.json";
pub(crate) const LEGACY_TELEGRAM_SESSION_FILE_NAME: &str = "tg.session";

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct PersistedTelegramSession {
    home_dc: i32,
    dc_options: Vec<DcOption>,
    peer_infos: Vec<PeerInfo>,
    updates_state: UpdatesState,
}

#[derive(Clone)]
struct RuntimeTelegramSessionState {
    home_dc: i32,
    dc_options: HashMap<i32, DcOption>,
    peer_infos: HashMap<PeerId, PeerInfo>,
    updates_state: UpdatesState,
}

impl Default for RuntimeTelegramSessionState {
    fn default() -> Self {
        let defaults = SessionData::default();
        Self {
            home_dc: defaults.home_dc,
            dc_options: defaults.dc_options,
            peer_infos: defaults.peer_infos,
            updates_state: defaults.updates_state,
        }
    }
}

impl From<&RuntimeTelegramSessionState> for PersistedTelegramSession {
    fn from(value: &RuntimeTelegramSessionState) -> Self {
        let mut dc_options: Vec<_> = value.dc_options.values().cloned().collect();
        dc_options.sort_by_key(|option| option.id);

        let mut peer_infos: Vec<_> = value.peer_infos.values().cloned().collect();
        peer_infos.sort_by_key(|info| info.id().bot_api_dialog_id());

        Self {
            home_dc: value.home_dc,
            dc_options,
            peer_infos,
            updates_state: value.updates_state.clone(),
        }
    }
}

impl From<PersistedTelegramSession> for RuntimeTelegramSessionState {
    fn from(value: PersistedTelegramSession) -> Self {
        Self {
            home_dc: value.home_dc,
            dc_options: value
                .dc_options
                .into_iter()
                .map(|option| (option.id, option))
                .collect(),
            peer_infos: value
                .peer_infos
                .into_iter()
                .map(|peer| (peer.id(), peer))
                .collect(),
            updates_state: value.updates_state,
        }
    }
}

pub(crate) struct FileTelegramSession {
    path: PathBuf,
    state: Mutex<RuntimeTelegramSessionState>,
}

pub fn telegram_session_path(base_dir: &Path) -> PathBuf {
    base_dir.join(TELEGRAM_SESSION_FILE_NAME)
}

pub fn legacy_telegram_session_path(base_dir: &Path) -> PathBuf {
    base_dir.join(LEGACY_TELEGRAM_SESSION_FILE_NAME)
}

impl FileTelegramSession {
    pub(crate) fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let state = load_or_migrate_state(&path).with_context(|| {
            format!(
                "Khong the khoi phuc session Telegram tai {}",
                path.display()
            )
        })?;
        Ok(Self {
            path,
            state: Mutex::new(state),
        })
    }

    fn persist_snapshot(path: &Path, snapshot: &RuntimeTelegramSessionState) {
        if let Err(err) = persist_snapshot(path, snapshot) {
            tracing::warn!(
                path = %path.display(),
                error = %err,
                "Khong the persist session Telegram"
            );
        }
    }
}

impl Session for FileTelegramSession {
    fn home_dc_id(&self) -> i32 {
        self.state.lock().expect("Mutex poisoned").home_dc
    }

    fn set_home_dc_id(&self, dc_id: i32) -> BoxFuture<'_, ()> {
        let snapshot = {
            let mut state = self.state.lock().expect("Mutex poisoned");
            state.home_dc = dc_id;
            state.clone()
        };
        let path = self.path.clone();
        Box::pin(async move {
            Self::persist_snapshot(&path, &snapshot);
        })
    }

    fn dc_option(&self, dc_id: i32) -> Option<DcOption> {
        self.state.lock().expect("Mutex poisoned").dc_options.get(&dc_id).cloned()
    }

    fn set_dc_option(&self, dc_option: &DcOption) -> BoxFuture<'_, ()> {
        let snapshot = {
            let mut state = self.state.lock().expect("Mutex poisoned");
            state.dc_options.insert(dc_option.id, dc_option.clone());
            state.clone()
        };
        let path = self.path.clone();
        Box::pin(async move {
            Self::persist_snapshot(&path, &snapshot);
        })
    }

    fn peer(&self, peer: PeerId) -> BoxFuture<'_, Option<PeerInfo>> {
        let result = match peer.kind() {
            PeerKind::UserSelf => self
                .state
                .lock()
                .expect("Mutex poisoned")
                .peer_infos
                .values()
                .find_map(|info| match info {
                    PeerInfo::User {
                        is_self: Some(true),
                        ..
                    } => Some(info.clone()),
                    _ => None,
                }),
            _ => self.state.lock().expect("Mutex poisoned").peer_infos.get(&peer).cloned(),
        };
        Box::pin(async move { result })
    }

    fn cache_peer(&self, peer: &PeerInfo) -> BoxFuture<'_, ()> {
        let snapshot = {
            let mut state = self.state.lock().expect("Mutex poisoned");
            state.peer_infos.insert(peer.id(), peer.clone());
            state.clone()
        };
        let path = self.path.clone();
        Box::pin(async move {
            Self::persist_snapshot(&path, &snapshot);
        })
    }

    fn updates_state(&self) -> BoxFuture<'_, UpdatesState> {
        let state = self.state.lock().expect("Mutex poisoned").updates_state.clone();
        Box::pin(async move { state })
    }

    fn set_update_state(&self, update: UpdateState) -> BoxFuture<'_, ()> {
        let snapshot = {
            let mut state = self.state.lock().expect("Mutex poisoned");
            match update {
                UpdateState::All(updates_state) => state.updates_state = updates_state,
                UpdateState::Primary { pts, date, seq } => {
                    state.updates_state.pts = pts;
                    state.updates_state.date = date;
                    state.updates_state.seq = seq;
                }
                UpdateState::Secondary { qts } => {
                    state.updates_state.qts = qts;
                }
                UpdateState::Channel { id, pts } => {
                    state
                        .updates_state
                        .channels
                        .retain(|channel| channel.id != id);
                    state.updates_state.channels.push(ChannelState { id, pts });
                }
            }
            state.clone()
        };
        let path = self.path.clone();
        Box::pin(async move {
            Self::persist_snapshot(&path, &snapshot);
        })
    }
}

fn load_or_migrate_state(path: &Path) -> io::Result<RuntimeTelegramSessionState> {
    if path.exists() {
        return load_json_state(path).or_else(|err| {
            tracing::warn!(
                path = %path.display(),
                error = %err,
                "Session Telegram JSON khong hop le, khoi tao session moi"
            );
            Ok(RuntimeTelegramSessionState::default())
        });
    }

    if let Some(legacy_path) = legacy_path_for_json(path) {
        if legacy_path.exists() {
            let migrated = try_migrate_legacy(&legacy_path).or_else(|err| {
                tracing::warn!(
                    path = %legacy_path.display(),
                    error = %err,
                    "Khong the migrate session Telegram SQLite cu, se dung session moi"
                );
                Ok::<RuntimeTelegramSessionState, io::Error>(RuntimeTelegramSessionState::default())
            })?;
            persist_snapshot(path, &migrated)?;
            return Ok(migrated);
        }
    }

    Ok(RuntimeTelegramSessionState::default())
}

fn try_migrate_legacy(path: &Path) -> io::Result<RuntimeTelegramSessionState> {
    let reader = LEGACY_READER.get().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "legacy session reader not initialized")
    })?;
    let data = reader.read_legacy_session(path).map_err(io::Error::other)?;
    let mut dc_options = HashMap::new();
    for opt in data.dc_options {
        let auth_key = opt.auth_key.and_then(|bytes| <[u8; 256]>::try_from(bytes).ok());
        dc_options.insert(
            opt.id,
            DcOption {
                id: opt.id,
                ipv4: opt.ipv4.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                ipv6: opt.ipv6.parse().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                auth_key,
            },
        );
    }
    let mut peer_infos = HashMap::new();
    for p in data.peer_infos {
        let peer_info = decode_legacy_peer_info(p.peer_id, p.hash, p.subtype)?;
        peer_infos.insert(peer_info.id(), peer_info);
    }
    let updates_state = UpdatesState {
        pts: data.pts as i32,
        qts: data.qts as i32,
        date: data.date as i32,
        seq: data.seq as i32,
        channels: data
            .channels
            .into_iter()
            .map(|c| ChannelState { id: c.id, pts: c.pts as i32 })
            .collect(),
    };
    Ok(RuntimeTelegramSessionState {
        home_dc: data.home_dc,
        dc_options,
        peer_infos,
        updates_state,
    })
}

fn load_json_state(path: &Path) -> io::Result<RuntimeTelegramSessionState> {
    let bytes = fs::read(path)?;
    let persisted: PersistedTelegramSession = serde_json::from_slice(&bytes)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    Ok(persisted.into())
}

fn persist_snapshot(path: &Path, snapshot: &RuntimeTelegramSessionState) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let persisted = PersistedTelegramSession::from(snapshot);
    let bytes = serde_json::to_vec(&persisted)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, bytes)?;
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    fs::rename(temp_path, path)
}

fn legacy_path_for_json(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    Some(parent.join(LEGACY_TELEGRAM_SESSION_FILE_NAME))
}



fn decode_legacy_peer_info(
    peer_id: i64,
    hash: Option<i64>,
    subtype: Option<i64>,
) -> io::Result<PeerInfo> {
    const USER_SELF: u8 = 1;
    const USER_BOT: u8 = 2;
    const MEGAGROUP: u8 = 4;
    const BROADCAST: u8 = 8;
    const GIGAGROUP: u8 = 12;

    let peer = decode_peer_id(peer_id)?;
    let subtype = subtype.map(|value| value as u8);
    Ok(match peer.kind() {
        PeerKind::User | PeerKind::UserSelf => PeerInfo::User {
            id: peer.bare_id(),
            auth: hash.map(PeerAuth::from_hash),
            bot: subtype.map(|value| value & USER_BOT != 0),
            is_self: subtype.map(|value| value & USER_SELF != 0),
        },
        PeerKind::Chat => PeerInfo::Chat { id: peer.bare_id() },
        PeerKind::Channel => PeerInfo::Channel {
            id: peer.bare_id(),
            auth: hash.map(PeerAuth::from_hash),
            kind: subtype.and_then(|value| {
                if (value & GIGAGROUP) == GIGAGROUP {
                    Some(ChannelKind::Gigagroup)
                } else if value & BROADCAST != 0 {
                    Some(ChannelKind::Broadcast)
                } else if value & MEGAGROUP != 0 {
                    Some(ChannelKind::Megagroup)
                } else {
                    None
                }
            }),
        },
    })
}

fn decode_peer_id(value: i64) -> io::Result<PeerId> {
    if value == (1_i64 << 40) {
        return Ok(PeerId::self_user());
    }
    if value > 0 {
        return PeerId::user(value).ok_or_else(|| invalid_peer_error(value));
    }
    if (-999_999_999_999..=-1).contains(&value) {
        return PeerId::chat(-value).ok_or_else(|| invalid_peer_error(value));
    }
    let channel_id = -value - 1_000_000_000_000;
    PeerId::channel(channel_id).ok_or_else(|| invalid_peer_error(value))
}

fn invalid_peer_error(value: i64) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid peer id {value}"),
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    struct MockLegacyReader;

    impl LegacySessionReader for MockLegacyReader {
        fn read_legacy_session(&self, _path: &Path) -> Result<LegacySessionData, String> {
            use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};
            Ok(LegacySessionData {
                home_dc: 2,
                dc_options: vec![LegacyDcOption {
                    id: 2,
                    ipv4: SocketAddrV4::new(Ipv4Addr::LOCALHOST, 443).to_string(),
                    ipv6: SocketAddrV6::new(Ipv6Addr::LOCALHOST, 443, 0, 0).to_string(),
                    auth_key: Some(vec![7_u8; 256]),
                }],
                peer_infos: vec![LegacyPeerInfo {
                    peer_id: 1,
                    hash: Some(99),
                    subtype: Some(3),
                }],
                pts: 10,
                qts: 11,
                date: 12,
                seq: 13,
                channels: vec![LegacyChannelState { id: 123, pts: 456 }],
            })
        }
    }

    #[test]
    fn migrates_legacy_state() {
        init_legacy_reader(Box::new(MockLegacyReader));

        let temp_root =
            std::env::temp_dir().join(format!("omega-drive-tg-session-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_root);
        fs::create_dir_all(&temp_root).unwrap();

        let legacy = legacy_telegram_session_path(&temp_root);
        fs::write(&legacy, b"placeholder").unwrap();

        let json = telegram_session_path(&temp_root);
        let session = FileTelegramSession::open(&json).unwrap();
        assert_eq!(session.home_dc_id(), 2);
        assert_eq!(session.dc_option(2).unwrap().auth_key, Some([7_u8; 256]));
        assert!(json.exists());

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let self_peer = runtime.block_on(session.peer(PeerId::self_user())).unwrap();
        match self_peer {
            PeerInfo::User {
                bot: Some(true),
                is_self: Some(true),
                ..
            } => {}
            other => panic!("unexpected peer info: {other:?}"),
        }
        let updates = runtime.block_on(session.updates_state());
        assert_eq!(updates.pts, 10);
        assert_eq!(updates.channels.len(), 1);

        let _ = fs::remove_dir_all(&temp_root);
    }
}
