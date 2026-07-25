//! Offline license validation and encrypted local license storage.

use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

type HmacSha256 = Hmac<Sha256>;

const LICENSE_FILE: &str = "license.enc";
const LICENSE_PREFIX: &str = "CLIPBOARD_LITE_LICENSE_V1.";
const ACTIVATION_PREFIX: &str = "CLIP1";

// MVP offline signing key. Production builds should inject this at compile time
// from a private release process and keep the generator outside the open repo.
const ACTIVATION_HMAC_KEY: &str = "clipboard-lite-mvp-offline-license-key-change-for-release";
const LOCAL_LICENSE_SALT: &str = "clipboard-lite-local-license-aes-key-v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationPayload {
    pub product: String,
    pub seats: u8,
    pub issued_at: i64,
    pub expires_at: Option<i64>,
    pub device_ids: Vec<String>,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredLicense {
    payload: ActivationPayload,
    activation_code: String,
    bound_devices: Vec<String>,
    activated_at: i64,
}

/// Check whether Pro license is valid on this device.
pub fn is_pro_licensed(app: &AppHandle) -> Result<bool, String> {
    let Some(license) = load_license(app)? else {
        return Ok(false);
    };
    Ok(validate_stored_license(&license, &device_fingerprint()))
}

/// Validate activation code and persist encrypted license file.
pub fn activate(app: &AppHandle, activation_code: &str) -> Result<bool, String> {
    let activation_code = activation_code.trim();
    let payload = match verify_activation_code(activation_code) {
        Ok(payload) => payload,
        Err(err) => {
            log::warn!("activation rejected: {err}");
            return Ok(false);
        }
    };

    let current_device = device_fingerprint();
    if !payload_allows_device(&payload, &current_device) {
        return Ok(false);
    }

    let mut stored = load_license(app)?.unwrap_or_else(|| StoredLicense {
        payload: payload.clone(),
        activation_code: activation_code.to_string(),
        bound_devices: Vec::new(),
        activated_at: Utc::now().timestamp_millis(),
    });

    stored.payload = payload;
    stored.activation_code = activation_code.to_string();

    if !stored.bound_devices.iter().any(|id| id == &current_device) {
        let seats = stored.payload.seats.clamp(1, 2) as usize;
        if stored.bound_devices.len() >= seats {
            return Ok(false);
        }
        stored.bound_devices.push(current_device);
    }

    save_license(app, &stored)?;
    Ok(true)
}

fn validate_stored_license(license: &StoredLicense, current_device: &str) -> bool {
    if license.payload.product != "clipboard-lite-pro" {
        return false;
    }
    if let Some(expires_at) = license.payload.expires_at {
        if expires_at < Utc::now().timestamp_millis() {
            return false;
        }
    }
    if !payload_allows_device(&license.payload, current_device) {
        return false;
    }

    let seats = license.payload.seats.clamp(1, 2) as usize;
    license.bound_devices.len() <= seats
        && license
            .bound_devices
            .iter()
            .any(|device| device == current_device)
}

fn payload_allows_device(payload: &ActivationPayload, current_device: &str) -> bool {
    payload.device_ids.is_empty()
        || payload.device_ids.iter().any(|id| {
            let id = id.trim();
            id == "*" || id.eq_ignore_ascii_case(current_device)
        })
}

fn verify_activation_code(code: &str) -> Result<ActivationPayload, String> {
    let parts = code.split('.').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0] != ACTIVATION_PREFIX {
        return Err("invalid activation code format".to_string());
    }

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|e| e.to_string())?;
    let signature = URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|e| e.to_string())?;

    let mut mac = <HmacSha256 as Mac>::new_from_slice(ACTIVATION_HMAC_KEY.as_bytes())
        .map_err(|e| e.to_string())?;
    mac.update(&payload_bytes);
    mac.verify_slice(&signature)
        .map_err(|_| "activation signature mismatch".to_string())?;

    let payload: ActivationPayload =
        serde_json::from_slice(&payload_bytes).map_err(|e| e.to_string())?;
    if payload.product != "clipboard-lite-pro" {
        return Err("activation product mismatch".to_string());
    }
    if payload.seats == 0 || payload.seats > 2 {
        return Err("activation seats must be 1 or 2".to_string());
    }
    Ok(payload)
}

fn load_license(app: &AppHandle) -> Result<Option<StoredLicense>, String> {
    let path = license_path(app)?;
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let encrypted = raw
        .strip_prefix(LICENSE_PREFIX)
        .ok_or_else(|| "unsupported license file format".to_string())?;
    let bytes = URL_SAFE_NO_PAD
        .decode(encrypted.trim())
        .map_err(|e| e.to_string())?;
    if bytes.len() <= 12 {
        return Err("license file is truncated".to_string());
    }

    let (nonce, ciphertext) = bytes.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(&local_license_key()).map_err(|e| e.to_string())?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|e| e.to_string())?;
    serde_json::from_slice(&plaintext)
        .map(Some)
        .map_err(|e| e.to_string())
}

fn save_license(app: &AppHandle, license: &StoredLicense) -> Result<(), String> {
    let path = license_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let plaintext = serde_json::to_vec(license).map_err(|e| e.to_string())?;
    let cipher = Aes256Gcm::new_from_slice(&local_license_key()).map_err(|e| e.to_string())?;
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext.as_ref())
        .map_err(|e| e.to_string())?;

    let mut out = nonce.to_vec();
    out.extend(ciphertext);
    fs::write(
        path,
        format!("{LICENSE_PREFIX}{}", URL_SAFE_NO_PAD.encode(out)),
    )
    .map_err(|e| e.to_string())
}

fn license_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.join(LICENSE_FILE))
        .map_err(|e| e.to_string())
}

fn local_license_key() -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(LOCAL_LICENSE_SALT.as_bytes());
    hasher.update(device_fingerprint().as_bytes());
    let digest = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&digest);
    key
}

pub fn device_fingerprint() -> String {
    let mut hasher = Sha256::new();
    hasher.update(std::env::consts::OS.as_bytes());
    hasher.update(std::env::consts::ARCH.as_bytes());

    for key in [
        "COMPUTERNAME",
        "USERNAME",
        "USERDOMAIN",
        "PROCESSOR_IDENTIFIER",
        "PROCESSOR_ARCHITECTURE",
        "NUMBER_OF_PROCESSORS",
    ] {
        if let Ok(value) = std::env::var(key) {
            hasher.update(key.as_bytes());
            hasher.update(value.as_bytes());
        }
    }

    hex::encode(hasher.finalize())
}
