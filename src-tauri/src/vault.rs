//! OS-keychain-backed encryption for persisted credentials.
//!
//! Uses [`encryptman_keyring::Vault`], which stores a master key in the
//! operating system's native credential store (Windows Credential Manager,
//! macOS Keychain, Linux Secret Service) and delegates the actual cryptography
//! to [`encryptman`] (AES-256-GCM with HKDF-SHA256 key derivation).
//!
//! Every persisted user-entered value that leaves the process — the INVS
//! connection settings (host, port, database, username, password) — is written
//! to disk only as ciphertext. The master key itself never touches disk; it
//! lives in the OS keychain.

use encryptman::MasterKey;
use encryptman_keyring::Vault;

/// Service identifier for the OS keychain entry. Doubles as the HKDF context,
/// isolating Swift Bill's master key from any other app on the same machine.
const SERVICE: &str = "swift-bill";

/// Encrypt `plaintext` with an explicit master key (no OS keychain access).
///
/// Used by tests so the crypto path can be exercised in headless/CI
/// environments where no credential store is available.
pub fn encrypt_with_key(key: &MasterKey, plaintext: &str) -> Result<String, String> {
  encryptman::encrypt(key, plaintext).map_err(|e| e.to_string())
}

/// Decrypt `ciphertext` with an explicit master key (no OS keychain access).
pub fn decrypt_with_key(key: &MasterKey, ciphertext: &str) -> Result<String, String> {
  encryptman::decrypt(key, ciphertext).map_err(|e| e.to_string())
}

/// Acquire the OS-keychain-backed vault. Creates and persists the master key on
/// first use; loads the existing key on subsequent calls.
pub fn vault() -> Result<Vault, String> {
  Vault::new(SERVICE).map_err(|e| format!("Cannot access OS keychain: {e}"))
}

/// Encrypt a string using the OS-keychain-backed master key.
pub fn encrypt_value(plaintext: &str) -> Result<String, String> {
  vault()?.encrypt(plaintext).map_err(|e| e.to_string())
}

/// Decrypt a ciphertext string using the OS-keychain-backed master key.
pub fn decrypt_value(ciphertext: &str) -> Result<String, String> {
  vault()?.decrypt(ciphertext).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn keyring_free_round_trip() {
    let key = encryptman::generate_master_key().unwrap();
    let plaintext = "plaintext-sample-value";
    let enc = encrypt_with_key(&key, plaintext).unwrap();
    let dec = decrypt_with_key(&key, &enc).unwrap();
    assert_eq!(dec, plaintext);
  }

  #[test]
  fn same_plaintext_yields_different_ciphertext() {
    let key = encryptman::generate_master_key().unwrap();
    let a = encrypt_with_key(&key, "repeat").unwrap();
    let b = encrypt_with_key(&key, "repeat").unwrap();
    assert_ne!(a, b, "nonce must make ciphertexts unique");
    assert_eq!(
      decrypt_with_key(&key, &a).unwrap(),
      decrypt_with_key(&key, &b).unwrap()
    );
  }

  #[test]
  fn tampered_ciphertext_fails() {
    let key = encryptman::generate_master_key().unwrap();
    let mut enc = encrypt_with_key(&key, "payload").unwrap();
    // Flip the last base64 character to corrupt the ciphertext.
    if let Some(last) = enc.chars().last() {
      let replacement = if last == 'A' { 'B' } else { 'A' };
      enc.pop();
      enc.push(replacement);
    }
    assert!(decrypt_with_key(&key, &enc).is_err());
  }
}
