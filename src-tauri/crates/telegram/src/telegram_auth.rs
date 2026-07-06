use std::{
    path::Path,
    sync::Arc,
};
use tokio::task::JoinHandle;

use omega_drive_gateway::core::types::ReachableDestination;

use crate::telegram_session::{
    telegram_session_path, FileTelegramSession,
};
use grammers_client::{
    client::{LoginToken, PasswordToken},
    Client as TelegramAuthClient, SenderPool, SignInError,
};

enum PendingTelegramAuth {
    AwaitingCode {
        client: TelegramAuthClient,
        login_token: LoginToken,
        runner: JoinHandle<()>,
    },
    AwaitingPassword {
        client: TelegramAuthClient,
        password_token: PasswordToken,
        runner: JoinHandle<()>,
    },
}

static PENDING_TELEGRAM_AUTH: tokio::sync::Mutex<Option<PendingTelegramAuth>> =
    tokio::sync::Mutex::const_new(None);

async fn connect_client(
    base_dir: &Path,
    api_id: i32,
) -> Result<(TelegramAuthClient, JoinHandle<()>), anyhow::Error> {
    let session_path = telegram_session_path(base_dir);
    let session = Arc::new(FileTelegramSession::open(&session_path)?);
    let SenderPool { runner, handle, .. } = SenderPool::new(session, api_id);
    let client = TelegramAuthClient::new(handle);
    let runner_handle = tokio::spawn(runner.run());
    Ok((client, runner_handle))
}

async fn replace_pending(next: Option<PendingTelegramAuth>) {
    let mut guard = PENDING_TELEGRAM_AUTH.lock().await;
    if let Some(old) = guard.take() {
        match old {
            PendingTelegramAuth::AwaitingCode { runner, .. }
            | PendingTelegramAuth::AwaitingPassword { runner, .. } => runner.abort(),
        }
    }
    *guard = next;
}

pub async fn current_step() -> (String, Option<String>) {
    let guard = PENDING_TELEGRAM_AUTH.lock().await;
    match guard.as_ref() {
        Some(PendingTelegramAuth::AwaitingCode { .. }) => ("code".to_string(), None),
        Some(PendingTelegramAuth::AwaitingPassword { password_token, .. }) => (
            "password".to_string(),
            password_token.hint().map(|value| value.to_string()),
        ),
        None => ("idle".to_string(), None),
    }
}

pub async fn start_login(
    base_dir: &Path,
    api_id: i32,
    api_hash: &str,
    phone: &str,
) -> Result<bool, anyhow::Error> {
    replace_pending(None).await;
    let (client, runner) = connect_client(base_dir, api_id).await?;

    if client.is_authorized().await? {
        runner.abort();
        return Ok(true);
    }

    let login_token = client
        .request_login_code(phone, api_hash)
        .await?;

    replace_pending(Some(PendingTelegramAuth::AwaitingCode {
        client,
        login_token,
        runner,
    }))
    .await;

    Ok(false)
}

pub async fn submit_code(code: &str) -> Result<bool, anyhow::Error> {
    let pending = PENDING_TELEGRAM_AUTH.lock().await.take();
    let Some(PendingTelegramAuth::AwaitingCode {
        client,
        login_token,
        runner,
    }) = pending
    else {
        return Err(anyhow::anyhow!("No pending login code request"));
    };

    let trimmed = code.trim().to_string();
    if trimmed.is_empty() {
        replace_pending(Some(PendingTelegramAuth::AwaitingCode {
            client,
            login_token,
            runner,
        }))
        .await;
        return Err(anyhow::anyhow!("Verification code is empty"));
    }

    match client.sign_in(&login_token, &trimmed).await {
        Ok(_) => {
            runner.abort();
            Ok(true) // authorized
        }
        Err(SignInError::PasswordRequired(password_token)) => {
            replace_pending(Some(PendingTelegramAuth::AwaitingPassword {
                client,
                password_token,
                runner,
            }))
            .await;
            Ok(false) // password needed
        }
        Err(SignInError::InvalidPassword(token)) => {
            replace_pending(Some(PendingTelegramAuth::AwaitingPassword {
                client,
                runner,
                password_token: token,
            }))
            .await;
            Err(anyhow::anyhow!("Invalid password"))
        }
        Err(SignInError::InvalidCode) => {
            replace_pending(Some(PendingTelegramAuth::AwaitingCode {
                client,
                login_token,
                runner,
            }))
            .await;
            Err(anyhow::anyhow!("Invalid verification code"))
        }
        Err(SignInError::SignUpRequired) => {
            runner.abort();
            Err(anyhow::anyhow!("Sign up required"))
        }
        Err(SignInError::Other(err)) => {
            runner.abort();
            Err(err.into())
        }
    }
}

pub async fn reset_state() {
    let mut guard = PENDING_TELEGRAM_AUTH.lock().await;
    if let Some(old) = guard.take() {
        match old {
            PendingTelegramAuth::AwaitingCode { runner, .. }
            | PendingTelegramAuth::AwaitingPassword { runner, .. } => runner.abort(),
        }
    }
}

pub async fn submit_password(password: &str) -> Result<(), anyhow::Error> {
    let pending = PENDING_TELEGRAM_AUTH.lock().await.take();
    let Some(PendingTelegramAuth::AwaitingPassword {
        client,
        password_token,
        runner,
    }) = pending
    else {
        return Err(anyhow::anyhow!("No pending password request"));
    };

    match client.check_password(password_token, password).await {
        Ok(_) => {
            runner.abort();
            Ok(())
        }
        Err(SignInError::InvalidPassword(password_token)) => {
            replace_pending(Some(PendingTelegramAuth::AwaitingPassword {
                client,
                password_token,
                runner,
            }))
            .await;
            Err(anyhow::anyhow!("Invalid password"))
        }
        Err(SignInError::Other(err)) => {
            runner.abort();
            Err(err.into())
        }
        Err(err) => {
            runner.abort();
            Err(err.into())
        }
    }
}

pub async fn list_reachable_groups(
    _base_dir: &Path,
    api_id: i32,
    api_hash: &str,
    phone: &str,
) -> Result<Vec<ReachableDestination>, anyhow::Error> {
    let (client, runner) = connect_client(_base_dir, api_id).await?;
    let result = list_groups_inner(&client, phone, api_hash).await;
    runner.abort();
    result
}

async fn list_groups_inner(
    client: &TelegramAuthClient,
    phone: &str,
    api_hash: &str,
) -> Result<Vec<ReachableDestination>, anyhow::Error> {
    if !client.is_authorized().await? {
        client.request_login_code(phone, api_hash).await?;
    }

    let mut groups = Vec::new();
    let mut dialogs = client.iter_dialogs();
    while let Some(dialog) = dialogs.next().await? {
        let peer = dialog.peer();
        let dialog_id = peer.id().bot_api_dialog_id();
        if dialog_id >= 0 {
            continue;
        }
        let name = peer.name().unwrap_or_default().trim().to_string();
        groups.push(ReachableDestination {
            id: dialog_id.to_string(),
            name: if name.is_empty() {
                dialog_id.to_string()
            } else {
                name
            },
        });
    }

    groups.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    groups.dedup_by(|left, right| left.id == right.id);

    Ok(groups)
}
