//! Encrypted persistence for the INVS connection settings (`DbConfig`).
//!
//! The connection settings (host, port, database, username, password) are
//! user-entered secrets. They are written to disk **only** as `encryptman`
//! ciphertext (AES-256-GCM) whose master key is held in the OS keychain via
//! `encryptman-keyring`. Plaintext is never serialized to storage.
//!
//! This replaces the previous `localStorage` cleartext persistence.

use std::fs;
use tauri::Manager;

use swift_bill_core::DbConfig;

fn db_config_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
  let data_dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Cannot resolve app data dir: {e}"))?;
  fs::create_dir_all(&data_dir).map_err(|e| format!("Cannot create data dir: {e}"))?;
  Ok(data_dir.join("db_config.enc"))
}

/// Persist the connection settings encrypted at rest.
pub fn save_db_config(app: &tauri::AppHandle, config: DbConfig) -> Result<(), String> {
  let path = db_config_path(app)?;
  let plaintext =
    serde_json::to_string(&config).map_err(|e| format!("Cannot serialize config: {e}"))?;
  let ciphertext = crate::vault::encrypt_value(&plaintext)?;
  fs::write(&path, ciphertext).map_err(|e| format!("Cannot write config: {e}"))
}

/// Load the connection settings, decrypting from storage.
///
/// Returns `None` when no config has been saved yet. A legacy cleartext file
/// (if ever present) is transparently parsed and treated as the config.
pub fn load_db_config(app: &tauri::AppHandle) -> Result<Option<DbConfig>, String> {
  let path = db_config_path(app)?;
  if !path.exists() {
    return Ok(None);
  }
  let content = fs::read_to_string(&path).map_err(|e| format!("Cannot read config: {e}"))?;

  // Fast path: stored as ciphertext.
  if let Ok(plaintext) = crate::vault::decrypt_value(&content) {
    return parse_config(&plaintext).map(Some);
  }

  // Legacy fallback: a cleartext JSON file from an older build.
  if let Ok(cfg) = parse_config(&content) {
    // Re-encrypt on next save by writing through the encrypted path.
    let _ = save_db_config(app, cfg.clone());
    return Ok(Some(cfg));
  }

  Err("Saved configuration is corrupted and cannot be read".to_string())
}

/// Remove the persisted connection settings from disk.
pub fn delete_db_config(app: &tauri::AppHandle) -> Result<(), String> {
  let path = db_config_path(app)?;
  if path.exists() {
    fs::remove_file(&path).map_err(|e| format!("Cannot delete config: {e}"))?;
  }
  Ok(())
}

fn parse_config(plaintext: &str) -> Result<DbConfig, String> {
  serde_json::from_str::<DbConfig>(plaintext).map_err(|e| format!("Cannot parse config: {e}"))
}

#[cfg(test)]
mod tests {
  use super::*;

  // These tests use an in-memory keychain-free path via the vault helpers.
  // They validate that the serialized config round-trips and that the on-disk
  // blob never contains the plaintext secrets.

  fn sample_config() -> DbConfig {
    DbConfig {
      host: "localhost".into(),
      port: 1433,
      database: "INVS".into(),
      username: "sa".into(),
      password: String::new(),
    }
  }

  #[test]
  fn encrypted_blob_contains_no_plaintext() {
    let cfg = sample_config();
    let plaintext = serde_json::to_string(&cfg).unwrap();
    let key = encryptman::generate_master_key().unwrap();
    let blob = crate::vault::encrypt_with_key(&key, &plaintext).unwrap();

    // None of the stored values may appear in the ciphertext blob.
    assert!(!blob.contains("localhost"));
    assert!(!blob.contains("INVS"));
    assert!(!blob.contains("sa"));
    // The blob is base64 and not valid JSON plaintext.
    assert!(
      blob.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
    );
    assert!(serde_json::from_str::<DbConfig>(&blob).is_err());
  }

  #[test]
  fn config_round_trips_through_vault_helpers() {
    let cfg = sample_config();
    let plaintext = serde_json::to_string(&cfg).unwrap();
    let key = encryptman::generate_master_key().unwrap();
    let blob = crate::vault::encrypt_with_key(&key, &plaintext).unwrap();
    let recovered = crate::vault::decrypt_with_key(&key, &blob).unwrap();
    let parsed = parse_config(&recovered).unwrap();
    assert_eq!(parsed, cfg);
  }
}
