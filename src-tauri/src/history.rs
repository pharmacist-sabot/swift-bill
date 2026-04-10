use std::fs;
use tauri::Manager;

use crate::models::{RoundHistory, RoundHistoryEntry};

pub fn load_history(app: &tauri::AppHandle) -> Result<Vec<RoundHistoryEntry>, String> {
  let path = get_history_path(app)?;
  if !path.exists() {
    return Ok(Vec::new());
  }
  let content = fs::read_to_string(&path).map_err(|e| format!("Cannot read history: {e}"))?;
  let history: RoundHistory =
    serde_json::from_str(&content).map_err(|e| format!("Cannot parse history: {e}"))?;
  Ok(history.entries)
}

pub fn upsert_entry(app: &tauri::AppHandle, entry: RoundHistoryEntry) -> Result<(), String> {
  let path = get_history_path(app)?;
  let mut history = load_history_full(app)?;
  history.entries.retain(|e| e.id != entry.id);
  history.entries.insert(0, entry); // newest first
  write_history(path, &history)
}

pub fn delete_entry(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
  let path = get_history_path(app)?;
  let mut history = load_history_full(app)?;
  history.entries.retain(|e| e.id != id);
  write_history(path, &history)
}

fn get_history_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
  let data_dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Cannot resolve app data dir: {e}"))?;
  fs::create_dir_all(&data_dir).map_err(|e| format!("Cannot create data dir: {e}"))?;
  Ok(data_dir.join("round_history.json"))
}

fn load_history_full(app: &tauri::AppHandle) -> Result<RoundHistory, String> {
  let path = get_history_path(app)?;
  if !path.exists() {
    return Ok(RoundHistory::default());
  }
  let content = fs::read_to_string(&path).map_err(|e| format!("Cannot read history: {e}"))?;
  serde_json::from_str(&content).map_err(|e| format!("Cannot parse history: {e}"))
}

fn write_history(path: std::path::PathBuf, history: &RoundHistory) -> Result<(), String> {
  let content =
    serde_json::to_string_pretty(history).map_err(|e| format!("Cannot serialize history: {e}"))?;
  fs::write(&path, content).map_err(|e| format!("Cannot write history: {e}"))
}
