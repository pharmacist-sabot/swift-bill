# Security Model — Swift Bill

Swift Bill is a desktop app for สระโบสถ์ Hospital that reads the legacy **INVS**
SQL Server (read-only) and prints three pharmaceutical disbursement reports.
This document describes how credentials are protected at rest.

## Credential storage

The INVS connection settings (`DbConfig`: host, port, database, username,
password) are **user-entered secrets**. They are persisted **only** as
ciphertext:

1. The settings are serialized to JSON in memory.
2. The JSON is encrypted with [`encryptman`](https://crates.io/crates/encryptman)
   — **AES-256-GCM** with **HKDF-SHA256** key derivation. A fresh random 12-byte
   nonce is used per encryption call, so identical plaintexts yield different
   ciphertexts.
3. The resulting blob is written to
   `<app-data-dir>/db_config.enc`.

The **master key is never written to disk**. It is generated once and stored in
the operating system's native credential store via
[`encryptman-keyring`](https://crates.io/crates/encryptman-keyring)
(`Vault::new("swift-bill")`):

| OS      | Keychain                       |
| ------- | ------------------------------ |
| Windows | Credential Manager             |
| macOS   | Keychain Services              |
| Linux   | Secret Service (D-Bus)         |

The string `"swift-bill"` is both the keychain service identifier **and** the
HKDF `info` context, isolating this app's key from any other on the machine.

## Data flow

```
User types credentials ─► App.vue (in-memory)
                            │
              save_db_config ─┤ serialize ─► encryptman::encrypt (master key from OS keychain)
                            │                         │
                            │                         ▼
                            │                  db_config.enc  (ciphertext only)
                            ▼
              load_db_config ─► encryptman::decrypt ─► parse ─► in-memory DbConfig
                            │
              test_connection / generate commands (local Tauri IPC, in-memory)
```

Plaintext exists only in process memory during a session; it is **never**
serialized to storage. The connection commands receive `DbConfig` over the
local Tauri IPC channel (same machine, no network), which is the intended
trust boundary — the at-rest guarantee is what this design enforces.

## Key lifecycle

- **First run:** `Vault::new` generates a master key and stores it in the OS
  keychain.
- **Subsequent runs:** the existing key is loaded from the keychain.
- **Delete:** `delete_db_config` removes the ciphertext blob. To also remove the
  key from the keychain, use `encryptman_keyring::Vault::delete("swift-bill")`.

There is no key escrow; losing OS-keychain access (e.g. different user account
or OS reinstall) means the saved config cannot be recovered — re-enter the
credentials. This is acceptable for a single-hospital desktop tool.

## What is NOT encrypted

- Report inputs entered per session that are **not** persisted (period, register
  numbers, budgets typed into the generate forms) live only in memory and are
  not stored.
- Round history and number locks (`round_history.json`, number-lock JSON) are
  unencrypted local files. They contain operational metadata (register numbers,
  running balances) but **no DB credentials**. Encrypting them is a future
  option (see ROADMAP Phase 5 "Structured round-history store").
- The INVS database connection itself is read-only by design (see `AGENTS.md`);
  Swift Bill never writes to INVS.

## Dependencies

- `encryptman` 0.3 — AES-256-GCM + HKDF-SHA256, string-oriented.
- `encryptman-keyring` 0.1 — OS-keychain-backed `Vault` for the master key.
