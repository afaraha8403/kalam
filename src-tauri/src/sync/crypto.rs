//! Encrypt `provider_keys` for sync using a key derived from the Pro license string (PBKDF2 + AES-256-GCM).

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

/// HKDF from the license string (deterministic per key; salt separates contexts).
fn derive_key(license_key: &str) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(b"kalam-sync-salt"), license_key.as_bytes());
    let mut out = [0u8; 32];
    hk.expand(b"provider-keys-v1", &mut out)
        .expect("hkdf expand to 32 bytes");
    out
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderKeysEnvelope {
    pub v: u32,
    /// ISO time for last-write-wins when merging pulled blobs.
    pub updated_at: String,
    pub nonce: String,
    pub ciphertext: String,
}

pub fn encrypt_provider_keys_json(
    license_key: &str,
    updated_at: &str,
    plaintext_json: &str,
) -> Result<String, String> {
    let key = derive_key(license_key);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ct = cipher
        .encrypt(&nonce, plaintext_json.as_bytes())
        .map_err(|e| e.to_string())?;
    let env = ProviderKeysEnvelope {
        v: 1,
        updated_at: updated_at.to_string(),
        nonce: B64.encode(nonce),
        ciphertext: B64.encode(ct),
    };
    serde_json::to_string(&env).map_err(|e| e.to_string())
}

pub fn decrypt_provider_keys_json(license_key: &str, payload: &str) -> Result<String, String> {
    let env: ProviderKeysEnvelope = serde_json::from_str(payload).map_err(|e| e.to_string())?;
    if env.v != 1 {
        return Err("unsupported_keys_envelope".to_string());
    }
    let nonce_bytes = B64
        .decode(env.nonce.as_bytes())
        .map_err(|e| e.to_string())?;
    let ct = B64
        .decode(env.ciphertext.as_bytes())
        .map_err(|e| e.to_string())?;
    let key = derive_key(license_key);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let nonce = Nonce::from_slice(nonce_bytes.as_slice());
    let pt = cipher
        .decrypt(nonce, ct.as_ref())
        .map_err(|_| "decrypt_failed".to_string())?;
    String::from_utf8(pt).map_err(|e| e.to_string())
}

pub fn envelope_updated_at(payload: &str) -> Result<String, String> {
    let env: ProviderKeysEnvelope = serde_json::from_str(payload).map_err(|e| e.to_string())?;
    Ok(env.updated_at)
}
