//! E2E DM crypto — vodozemac Olm with X3DH-style bundles, persisted per
//! user under `<app_data>/crypto/<user_id>/`.
//!
//! Day-2/3 scope: identity + signed (fallback) prekey + one-time prekeys +
//! per-peer 1:1 sessions. Multiple sessions per peer to survive the
//! simultaneous-first-message race. Per-user namespacing so logging out
//! and back in as a different user (or the same user) doesn't trash the
//! other accounts' keys. No multi-device, no key rotation, no prekey
//! replenishment.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use vodozemac::olm::{
    Account, AccountPickle, InboundCreationResult, OlmMessage, Session, SessionConfig,
    SessionPickle,
};
use vodozemac::{Curve25519PublicKey, Ed25519PublicKey, Ed25519Signature};

const SIGNED_PREKEY_ID: i64 = 0;
const INITIAL_ONE_TIME_KEY_COUNT: usize = 50;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("bad key: {0}")]
    BadKey(String),
    #[error("bad signature: {0}")]
    BadSignature(String),
    #[error("signature verification failed: {0}")]
    SignatureInvalid(String),
    #[error("bad message: {0}")]
    BadMessage(String),
    #[error("no session with {0}")]
    NoSession(String),
    #[error("session creation: {0}")]
    SessionCreation(String),
    #[error("encryption: {0}")]
    Encryption(String),
    #[error("decryption: {0}")]
    Decryption(String),
    #[error("plaintext not utf-8")]
    PlaintextNotUtf8,
    #[error("no fallback key present")]
    MissingFallback,
    #[error("crypto not attached to a user — call crypto_attach first")]
    NotAttached,
    #[error("mutex poisoned")]
    Poisoned,
}

impl From<CryptoError> for String {
    fn from(e: CryptoError) -> Self {
        e.to_string()
    }
}

#[derive(Serialize, Deserialize)]
struct StoredAccount {
    pickle: AccountPickle,
    keys_published: bool,
}

pub struct CryptoState {
    inner: Mutex<Option<Inner>>,
    base_dir: PathBuf,
}

struct Inner {
    account: Account,
    sessions: HashMap<String, Vec<Session>>,
    keys_published: bool,
    user_dir: PathBuf,
}

impl CryptoState {
    /// Initialize the empty state container at app startup. No identity is
    /// loaded until `crypto_attach` is called for a specific user.
    pub fn load_or_init(app_data_dir: PathBuf) -> Result<Self, CryptoError> {
        fs::create_dir_all(app_data_dir.join("crypto"))?;
        Ok(Self {
            inner: Mutex::new(None),
            base_dir: app_data_dir,
        })
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), CryptoError> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

fn write_account(user_dir: &Path, stored: &StoredAccount) -> Result<(), CryptoError> {
    let path = user_dir.join("account.json");
    let bytes = serde_json::to_vec(stored)?;
    write_atomic(&path, &bytes)
}

fn write_sessions(
    user_dir: &Path,
    peer: &str,
    pickles: &[SessionPickle],
) -> Result<(), CryptoError> {
    let path = user_dir.join("sessions").join(format!("{peer}.json"));
    let bytes = serde_json::to_vec(pickles)?;
    write_atomic(&path, &bytes)
}

fn parse_curve(bytes: &[u8]) -> Result<Curve25519PublicKey, CryptoError> {
    Curve25519PublicKey::from_slice(bytes).map_err(|e| CryptoError::BadKey(e.to_string()))
}

fn parse_ed25519(bytes: &[u8]) -> Result<Ed25519PublicKey, CryptoError> {
    let arr: &[u8; 32] = bytes
        .try_into()
        .map_err(|_| CryptoError::BadKey(format!("expected 32 bytes, got {}", bytes.len())))?;
    Ed25519PublicKey::from_slice(arr).map_err(|e| CryptoError::BadKey(e.to_string()))
}

fn parse_signature(bytes: &[u8]) -> Result<Ed25519Signature, CryptoError> {
    Ed25519Signature::from_slice(bytes).map_err(|e| CryptoError::BadSignature(e.to_string()))
}

#[derive(Serialize)]
pub struct IdentityDto {
    pub curve25519: Vec<u8>,
    pub ed25519: Vec<u8>,
    pub keys_published: bool,
}

#[derive(Serialize)]
pub struct OneTimePrekeyDto {
    pub prekey_id: i64,
    pub public_key: Vec<u8>,
}

#[derive(Serialize)]
pub struct BundleUploadDto {
    pub identity_key: Vec<u8>,
    pub signing_key: Vec<u8>,
    pub signed_prekey: Vec<u8>,
    pub signed_prekey_signature: Vec<u8>,
    pub signed_prekey_id: i64,
    pub one_time_prekeys: Vec<OneTimePrekeyDto>,
}

#[derive(Serialize)]
pub struct EncryptedMessageDto {
    pub message_type: u8,
    pub ciphertext: Vec<u8>,
}

// ─── commands ────────────────────────────────────────────────────────

/// Load (or create) crypto state for `user_id`. Each user has their own
/// subdirectory at `<app_data>/crypto/<user_id>/`. Idempotent: a no-op if
/// we're already attached to this user.
#[tauri::command]
pub fn crypto_attach(state: tauri::State<'_, CryptoState>, user_id: String) -> Result<(), String> {
    let user_dir = state.base_dir.join("crypto").join(&user_id);

    // Fast path: already attached to this user.
    {
        let guard = state.inner.lock().map_err(|_| CryptoError::Poisoned)?;
        if let Some(i) = guard.as_ref() {
            if i.user_dir == user_dir {
                return Ok(());
            }
        }
    }

    let sessions_dir = user_dir.join("sessions");
    fs::create_dir_all(&sessions_dir).map_err(CryptoError::from)?;

    let account_path = user_dir.join("account.json");
    let (mut account, keys_published, fresh) = if account_path.exists() {
        let bytes = fs::read(&account_path).map_err(CryptoError::from)?;
        let stored: StoredAccount = serde_json::from_slice(&bytes).map_err(CryptoError::from)?;
        (Account::from(stored.pickle), stored.keys_published, false)
    } else {
        (Account::new(), false, true)
    };

    if fresh {
        account.generate_fallback_key();
        account.generate_one_time_keys(INITIAL_ONE_TIME_KEY_COUNT);
    }

    let mut sessions: HashMap<String, Vec<Session>> = HashMap::new();
    for entry in fs::read_dir(&sessions_dir).map_err(CryptoError::from)? {
        let entry = entry.map_err(CryptoError::from)?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let bytes = fs::read(&path).map_err(CryptoError::from)?;
        let pickles: Vec<SessionPickle> = match serde_json::from_slice::<Vec<SessionPickle>>(&bytes)
        {
            Ok(v) => v,
            Err(_) => match serde_json::from_slice::<SessionPickle>(&bytes) {
                Ok(p) => vec![p],
                Err(e) => return Err(CryptoError::Serde(e).into()),
            },
        };
        sessions.insert(
            stem.to_string(),
            pickles.into_iter().map(Session::from).collect(),
        );
    }

    if fresh {
        let stored = StoredAccount {
            pickle: account.pickle(),
            keys_published: false,
        };
        write_account(&user_dir, &stored)?;
    }

    let new_inner = Inner {
        account,
        sessions,
        keys_published,
        user_dir,
    };

    let mut guard = state.inner.lock().map_err(|_| CryptoError::Poisoned)?;
    *guard = Some(new_inner);

    Ok(())
}

/// Drop the in-memory state. On-disk state for each user is preserved.
/// Call on logout.
#[tauri::command]
pub fn crypto_detach(state: tauri::State<'_, CryptoState>) -> Result<(), String> {
    let mut guard = state.inner.lock().map_err(|_| CryptoError::Poisoned)?;
    *guard = None;
    Ok(())
}

/// Wipe the current user's on-disk state and regenerate from scratch. Use
/// this only when the server has lost the bundle (e.g. DB was dropped)
/// and you need a fresh identity. Destroys all sessions with peers.
#[tauri::command]
pub fn crypto_regenerate(state: tauri::State<'_, CryptoState>) -> Result<(), String> {
    let user_dir = {
        let mut guard = state.inner.lock().map_err(|_| CryptoError::Poisoned)?;
        let inner = guard.as_mut().ok_or(CryptoError::NotAttached)?;

        let mut account = Account::new();
        account.generate_fallback_key();
        account.generate_one_time_keys(INITIAL_ONE_TIME_KEY_COUNT);

        inner.account = account;
        inner.sessions.clear();
        inner.keys_published = false;

        let stored = StoredAccount {
            pickle: inner.account.pickle(),
            keys_published: false,
        };
        let user_dir = inner.user_dir.clone();
        write_account(&user_dir, &stored)?;
        user_dir
    };

    let sessions_dir = user_dir.join("sessions");
    if sessions_dir.exists() {
        for entry in fs::read_dir(&sessions_dir).map_err(CryptoError::from)? {
            let entry = entry.map_err(CryptoError::from)?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                fs::remove_file(&path).map_err(CryptoError::from)?;
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub fn crypto_get_identity(state: tauri::State<'_, CryptoState>) -> Result<IdentityDto, String> {
    let guard = state.inner.lock().map_err(|_| CryptoError::Poisoned)?;
    let inner = guard.as_ref().ok_or(CryptoError::NotAttached)?;
    let keys = inner.account.identity_keys();
    Ok(IdentityDto {
        curve25519: keys.curve25519.to_bytes().to_vec(),
        ed25519: keys.ed25519.as_bytes().to_vec(),
        keys_published: inner.keys_published,
    })
}

#[tauri::command]
pub fn crypto_get_bundle_for_upload(
    state: tauri::State<'_, CryptoState>,
) -> Result<BundleUploadDto, String> {
    let guard = state.inner.lock().map_err(|_| CryptoError::Poisoned)?;
    let inner = guard.as_ref().ok_or(CryptoError::NotAttached)?;

    let identity = inner.account.identity_keys();
    let identity_key = identity.curve25519.to_bytes().to_vec();
    let signing_key = identity.ed25519.as_bytes().to_vec();

    let fallback = inner.account.fallback_key();
    let (_kid, signed_prekey_pub) = fallback
        .into_iter()
        .next()
        .ok_or(CryptoError::MissingFallback)?;
    let signed_prekey = signed_prekey_pub.to_bytes().to_vec();

    let signature = inner.account.sign(signed_prekey_pub.to_base64().as_bytes());
    let signed_prekey_signature = signature.to_bytes().to_vec();

    let one_time_prekeys: Vec<OneTimePrekeyDto> = inner
        .account
        .one_time_keys()
        .into_values()
        .enumerate()
        .map(|(idx, key)| OneTimePrekeyDto {
            prekey_id: idx as i64,
            public_key: key.to_bytes().to_vec(),
        })
        .collect();

    Ok(BundleUploadDto {
        identity_key,
        signing_key,
        signed_prekey,
        signed_prekey_signature,
        signed_prekey_id: SIGNED_PREKEY_ID,
        one_time_prekeys,
    })
}

#[tauri::command]
pub fn crypto_mark_keys_published(state: tauri::State<'_, CryptoState>) -> Result<(), String> {
    let (stored, user_dir) = {
        let mut guard = state.inner.lock().map_err(|_| CryptoError::Poisoned)?;
        let inner = guard.as_mut().ok_or(CryptoError::NotAttached)?;
        inner.account.mark_keys_as_published();
        inner.keys_published = true;
        let stored = StoredAccount {
            pickle: inner.account.pickle(),
            keys_published: true,
        };
        (stored, inner.user_dir.clone())
    };
    write_account(&user_dir, &stored)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn crypto_establish_session(
    state: tauri::State<'_, CryptoState>,
    peer_user_id: String,
    peer_identity_key: Vec<u8>,
    peer_signing_key: Vec<u8>,
    peer_signed_prekey: Vec<u8>,
    peer_signed_prekey_signature: Vec<u8>,
    peer_one_time_prekey: Option<Vec<u8>>,
) -> Result<(), String> {
    let identity = parse_curve(&peer_identity_key)?;
    let signing = parse_ed25519(&peer_signing_key)?;
    let signed_prekey = parse_curve(&peer_signed_prekey)?;
    let signature = parse_signature(&peer_signed_prekey_signature)?;

    signing
        .verify(signed_prekey.to_base64().as_bytes(), &signature)
        .map_err(|e| CryptoError::SignatureInvalid(e.to_string()))?;

    let session_key = if let Some(otk_bytes) = peer_one_time_prekey {
        parse_curve(&otk_bytes)?
    } else {
        signed_prekey
    };

    let (pickles, user_dir) = {
        let mut guard = state.inner.lock().map_err(|_| CryptoError::Poisoned)?;
        let inner = guard.as_mut().ok_or(CryptoError::NotAttached)?;
        let session = inner
            .account
            .create_outbound_session(SessionConfig::version_1(), identity, session_key)
            .map_err(|e| CryptoError::SessionCreation(e.to_string()))?;
        let entry = inner.sessions.entry(peer_user_id.clone()).or_default();
        entry.push(session);
        let pickles: Vec<SessionPickle> = entry.iter().map(|s| s.pickle()).collect();
        (pickles, inner.user_dir.clone())
    };
    write_sessions(&user_dir, &peer_user_id, &pickles)?;
    Ok(())
}

#[tauri::command]
pub fn crypto_has_session(
    state: tauri::State<'_, CryptoState>,
    peer_user_id: String,
) -> Result<bool, String> {
    let guard = state.inner.lock().map_err(|_| CryptoError::Poisoned)?;
    let inner = guard.as_ref().ok_or(CryptoError::NotAttached)?;
    Ok(inner
        .sessions
        .get(&peer_user_id)
        .map(|v| !v.is_empty())
        .unwrap_or(false))
}

#[tauri::command]
pub fn crypto_encrypt(
    state: tauri::State<'_, CryptoState>,
    peer_user_id: String,
    plaintext: String,
) -> Result<EncryptedMessageDto, String> {
    let (msg_type, ciphertext, pickles, user_dir) = {
        let mut guard = state.inner.lock().map_err(|_| CryptoError::Poisoned)?;
        let inner = guard.as_mut().ok_or(CryptoError::NotAttached)?;
        let sessions = inner
            .sessions
            .get_mut(&peer_user_id)
            .ok_or_else(|| CryptoError::NoSession(peer_user_id.clone()))?;
        let session = sessions
            .first_mut()
            .ok_or_else(|| CryptoError::NoSession(peer_user_id.clone()))?;
        let msg = session
            .encrypt(plaintext.as_bytes())
            .map_err(|e| CryptoError::Encryption(e.to_string()))?;
        let (msg_type, ciphertext) = msg.to_parts();
        let pickles: Vec<SessionPickle> = sessions.iter().map(|s| s.pickle()).collect();
        (msg_type as u8, ciphertext, pickles, inner.user_dir.clone())
    };
    write_sessions(&user_dir, &peer_user_id, &pickles)?;
    Ok(EncryptedMessageDto {
        message_type: msg_type,
        ciphertext,
    })
}

#[tauri::command]
pub fn crypto_decrypt(
    state: tauri::State<'_, CryptoState>,
    peer_user_id: String,
    peer_identity_key: Vec<u8>,
    message_type: u8,
    ciphertext: Vec<u8>,
) -> Result<String, String> {
    let msg = OlmMessage::from_parts(message_type as usize, &ciphertext)
        .map_err(|e| CryptoError::BadMessage(e.to_string()))?;

    let (plaintext, session_pickles, account_pickle, user_dir) = {
        let mut guard = state.inner.lock().map_err(|_| CryptoError::Poisoned)?;
        let inner = guard.as_mut().ok_or(CryptoError::NotAttached)?;

        // Try every existing session; if any decrypts, take it.
        let mut decrypted: Option<Vec<u8>> = None;
        if let Some(sessions) = inner.sessions.get_mut(&peer_user_id) {
            for session in sessions.iter_mut() {
                if let Ok(bytes) = session.decrypt(&msg) {
                    decrypted = Some(bytes);
                    break;
                }
            }
        }

        let (plaintext, persist_account) = if let Some(plaintext) = decrypted {
            (plaintext, false)
        } else {
            // No existing session matched. New PreKey messages spawn a
            // fresh inbound session; Normal messages with no match are
            // unrecoverable.
            let prekey = match msg {
                OlmMessage::PreKey(m) => m,
                OlmMessage::Normal(_) => {
                    return Err(CryptoError::NoSession(peer_user_id).into());
                }
            };
            let identity = parse_curve(&peer_identity_key)?;
            let InboundCreationResult { session, plaintext } = inner
                .account
                .create_inbound_session(SessionConfig::version_1(), identity, &prekey)
                .map_err(|e| CryptoError::SessionCreation(e.to_string()))?;
            inner
                .sessions
                .entry(peer_user_id.clone())
                .or_default()
                .push(session);
            (plaintext, true)
        };

        let session_pickles: Vec<SessionPickle> = inner
            .sessions
            .get(&peer_user_id)
            .unwrap()
            .iter()
            .map(|s| s.pickle())
            .collect();
        let account_pickle = if persist_account {
            Some(StoredAccount {
                pickle: inner.account.pickle(),
                keys_published: inner.keys_published,
            })
        } else {
            None
        };
        (
            plaintext,
            session_pickles,
            account_pickle,
            inner.user_dir.clone(),
        )
    };

    write_sessions(&user_dir, &peer_user_id, &session_pickles)?;
    if let Some(ap) = account_pickle {
        write_account(&user_dir, &ap)?;
    }

    String::from_utf8(plaintext).map_err(|_| CryptoError::PlaintextNotUtf8.into())
}
