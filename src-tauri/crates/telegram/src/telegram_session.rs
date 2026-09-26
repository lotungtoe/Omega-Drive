use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
};

use anyhow::Context;
use futures_util::future::BoxFuture;
use grammers_session::{
    types::{
        ChannelState, DcOption, PeerId, PeerInfo, PeerKind, UpdateState,
        UpdatesState,
    },
    Session, SessionData,
};

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
        let state = load_state(&path).with_context(|| {
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

fn load_state(path: &Path) -> io::Result<RuntimeTelegramSessionState> {
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

    Ok(RuntimeTelegramSessionState::default())
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


