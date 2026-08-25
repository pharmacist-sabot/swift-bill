//! Persistent number-lock storage.
//!
//! Locks are stored as `number_locks.json` in the per-user Tauri app
//! data directory. This module is Tauri-specific because it needs the
//! [`tauri::AppHandle`] to resolve the data dir; the lock entry schema
//! itself lives in `swift_bill_core::NumberLockEntry`.

use std::fs;
use tauri::Manager;

use swift_bill_core::register::{compute_reg_for_item, format_reg_no, parse_reg_no};
use swift_bill_core::{NumberLockBatchParams, NumberLockEntry, NumberLockStore};

pub fn load_number_locks(app: &tauri::AppHandle) -> Result<Vec<NumberLockEntry>, String> {
  let mut store = load_number_lock_store(app)?;
  sort_entries(&mut store.entries);
  Ok(store.entries)
}

pub fn create_number_locks(
  app: &tauri::AppHandle,
  params: NumberLockBatchParams,
) -> Result<Vec<NumberLockEntry>, String> {
  validate_batch_params(&params)?;

  let path = get_number_lock_path(app)?;
  let mut store = load_number_lock_store(app)?;
  let created_at = chrono::Utc::now().to_rfc3339();
  let mut created: Vec<NumberLockEntry> = Vec::with_capacity(params.count as usize);

  // A register lock (เลขทะเบียนคุม) locks one or more individual slots.
  if let (Some(reg_no), Some(running)) = (params.start_reg_no.clone(), params.running) {
    let (prefix, start_num) = parse_reg_no(&reg_no);
    for offset in 0..params.count {
      let (num, slot) = compute_reg_for_item(offset, running, start_num);
      let rno = format_reg_no(&prefix, num);
      if store.entries.iter().any(|e| {
        e.fiscal_year == params.fiscal_year
          && e.start_reg_no.as_deref() == Some(rno.as_str())
          && e.running == Some(slot)
      }) {
        return Err(format!(
          "เลขทะเบียนคุม {rno} ช่อง {slot} ถูกล็อกไว้แล้ว ({})",
          params.reason.trim()
        ));
      }
      created.push(NumberLockEntry {
        id: format!("{}-{}-{}", params.fiscal_year, rno, slot),
        fiscal_year: params.fiscal_year,
        request_no: 0,
        report_no: 0,
        purchase_no: 0,
        start_reg_no: Some(rno),
        running: Some(slot),
        reason: params.reason.trim().to_string(),
        note: params.note.trim().to_string(),
        created_at: created_at.clone(),
      });
    }
    store.entries.extend(created.iter().cloned());
    sort_entries(&mut store.entries);
    write_number_lock_store(path, &store)?;
    Ok(created)
  } else {
    for offset in 0..params.count {
      let request_no = params.start_request_no + offset * 2;
      let report_no = request_no + 1;
      let purchase_no = params.start_purchase_no + offset;

      if let Some(existing) = find_overlapping_entry(
        &store.entries,
        params.fiscal_year,
        request_no,
        report_no,
        purchase_no,
      ) {
        return Err(format!(
          "เลขชุด {request_no}/{report_no}/{purchase_no} ถูกล็อกไว้แล้ว ({})",
          existing.reason
        ));
      }

      created.push(NumberLockEntry {
        id: format!("{}-{}-{}", params.fiscal_year, request_no, purchase_no),
        fiscal_year: params.fiscal_year,
        request_no,
        report_no,
        purchase_no,
        start_reg_no: None,
        running: None,
        reason: params.reason.trim().to_string(),
        note: params.note.trim().to_string(),
        created_at: created_at.clone(),
      });
    }

    store.entries.extend(created.iter().cloned());
    sort_entries(&mut store.entries);
    write_number_lock_store(path, &store)?;
    Ok(created)
  }
}

pub fn delete_number_lock(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
  let path = get_number_lock_path(app)?;
  let mut store = load_number_lock_store(app)?;
  store.entries.retain(|entry| entry.id != id);
  sort_entries(&mut store.entries);
  write_number_lock_store(path, &store)
}

fn validate_batch_params(params: &NumberLockBatchParams) -> Result<(), String> {
  if params.fiscal_year <= 0 {
    return Err("ปีงบประมาณไม่ถูกต้อง".to_string());
  }
  if params.start_request_no == 0 {
    return Err("เลขขอซื้อเริ่มต้นต้องมากกว่า 0".to_string());
  }
  if params.start_purchase_no == 0 {
    return Err("เลขใบสั่งซื้อเริ่มต้นต้องมากกว่า 0".to_string());
  }
  if params.count == 0 {
    return Err("จำนวนชุดที่ต้องการล็อกต้องมากกว่า 0".to_string());
  }
  if params.reason.trim().is_empty() {
    return Err("กรุณาระบุเหตุผลในการล็อกเลข".to_string());
  }
  match (params.start_reg_no.as_ref(), params.running) {
    (Some(_), None) | (None, Some(_)) => {
      return Err("การล็อกเลขทะเบียนคุมต้องระบุทั้ง start_reg_no และ running".to_string());
    }
    (Some(_), Some(running)) if running > 9 => {
      return Err("running ต้องอยู่ในช่วง 0–9".to_string());
    }
    _ => {}
  }
  Ok(())
}

fn overlaps(
  entry: &NumberLockEntry,
  fiscal_year: i32,
  request_no: u32,
  report_no: u32,
  purchase_no: u32,
) -> bool {
  entry.fiscal_year == fiscal_year
    && (entry.request_no == request_no
      || entry.report_no == report_no
      || entry.purchase_no == purchase_no)
}

fn find_overlapping_entry(
  entries: &[NumberLockEntry],
  fiscal_year: i32,
  request_no: u32,
  report_no: u32,
  purchase_no: u32,
) -> Option<&NumberLockEntry> {
  entries
    .iter()
    .find(|entry| overlaps(entry, fiscal_year, request_no, report_no, purchase_no))
}

fn get_number_lock_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
  let data_dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Cannot resolve app data dir: {e}"))?;
  fs::create_dir_all(&data_dir).map_err(|e| format!("Cannot create data dir: {e}"))?;
  Ok(data_dir.join("number_locks.json"))
}

fn load_number_lock_store(app: &tauri::AppHandle) -> Result<NumberLockStore, String> {
  let path = get_number_lock_path(app)?;
  if !path.exists() {
    return Ok(NumberLockStore::default());
  }
  let content = fs::read_to_string(&path).map_err(|e| format!("Cannot read number locks: {e}"))?;
  serde_json::from_str(&content).map_err(|e| format!("Cannot parse number locks: {e}"))
}

fn write_number_lock_store(
  path: std::path::PathBuf,
  store: &NumberLockStore,
) -> Result<(), String> {
  let content = serde_json::to_string_pretty(store)
    .map_err(|e| format!("Cannot serialize number locks: {e}"))?;
  fs::write(&path, content).map_err(|e| format!("Cannot write number locks: {e}"))
}

fn sort_entries(entries: &mut [NumberLockEntry]) {
  entries.sort_by(|a, b| {
    b.fiscal_year
      .cmp(&a.fiscal_year)
      .then(a.request_no.cmp(&b.request_no))
      .then(a.purchase_no.cmp(&b.purchase_no))
  });
}
