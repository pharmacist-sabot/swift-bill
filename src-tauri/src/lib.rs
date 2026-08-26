//! Tauri shell for Swift Bill.
//!
//! This module is a *thin* wrapper. It contains only:
//! * Tauri command handlers that delegate to the pure-Rust workspace crates
//!   (`swift-bill-core`, `swift-bill-db`, `swift-bill-excel`, `swift-bill-pdf`).
//! * Tauri-specific persistence (`history`, `number_locks`) that needs
//!   access to the [`tauri::AppHandle`] for resolving the per-user data dir.
//! * Tauri plugin setup and command registration in [`run`].
//!
//! No business logic, no report algorithms, no PDF rendering, no SQL —
//! those live in the workspace crates where they are independently
//! testable and free of any Tauri / GUI dependency.

mod history;
mod number_locks;
mod settings;
mod vault;

use swift_bill_core::{
  CarryForward, CoverLettersParams, DbConfig, GenerateResult, InvoiceSubmissionExcelParams,
  InvoiceSubmissionParams, InvoiceSubmissionPreview, NormalizeReceivingStartParams,
  NumberLockBatchParams, NumberLockEntry, PreviewData, ReceivingSummaryExcelParams,
  ReceivingSummaryGenerateResult, ReceivingSummaryParams, ReceivingSummaryPreview, RoundHistory,
  RoundHistoryEntry, RoundValidation, ValidateRoundParams,
};
use swift_bill_db::fetch_invoices;
use swift_bill_excel::{generate_invoice_submission_excel, generate_receiving_summary_excel};
use swift_bill_pdf::generate_cover_letters_pdf_template as generate_cover_letters_pdf;

// Helpers

fn prepare_output_dir(raw: &str) -> Result<String, String> {
  let base = if raw.trim().is_empty() {
    "."
  } else {
    raw.trim()
  };
  let dir = std::path::Path::new(base).join("output");
  std::fs::create_dir_all(&dir).map_err(|e| format!("ไม่สามารถสร้างโฟลเดอร์ output: {e}"))?;
  Ok(dir.to_string_lossy().to_string())
}

// Tauri commands: Connection & Preview

#[tauri::command]
async fn test_connection(config: DbConfig) -> Result<String, String> {
  let mut client = swift_bill_db::connect(&config)
    .await
    .map_err(|e| e.to_string())?;
  client
    .simple_query("SELECT 1")
    .await
    .map_err(|e| format!("Query failed: {e}"))?;
  Ok("เชื่อมต่อฐานข้อมูลสำเร็จ".to_string())
}

// Encrypted connection-settings persistence (Phase 5: credential store)

#[tauri::command]
async fn save_db_config(app: tauri::AppHandle, config: DbConfig) -> Result<(), String> {
  settings::save_db_config(&app, config)
}

#[tauri::command]
async fn load_db_config(app: tauri::AppHandle) -> Result<Option<DbConfig>, String> {
  settings::load_db_config(&app)
}

#[tauri::command]
async fn delete_db_config(app: tauri::AppHandle) -> Result<(), String> {
  settings::delete_db_config(&app)
}

#[tauri::command]
async fn fetch_preview(
  config: DbConfig,
  date_from: String,
  date_to: String,
) -> Result<PreviewData, String> {
  let invoices = fetch_invoices(&config, &date_from, &date_to)
    .await
    .map_err(|e| e.to_string())?;
  let total_amount: f64 = invoices.iter().map(|i| i.total_cost).sum();
  let row_count = invoices.len();
  Ok(PreviewData {
    invoices,
    total_amount,
    row_count,
  })
}

// Tauri commands: Invoice Submission (ส่งหนี้เบิกยา) Preview + Excel export

#[tauri::command]
async fn preview_invoice_submission(
  params: InvoiceSubmissionParams,
) -> Result<InvoiceSubmissionPreview, String> {
  let invoices = fetch_invoices(&params.db_config, &params.date_from, &params.date_to)
    .await
    .map_err(|e| e.to_string())?;
  if invoices.is_empty() {
    return Err("ไม่พบข้อมูลในช่วงวันที่ที่เลือก".to_string());
  }

  let n = invoices.len() as u32;
  let (next_reg_no, next_running) =
    swift_bill_core::reports::compute_next_reg(&params.start_reg_no, params.start_running, n);

  let rows = swift_bill_core::reports::process_invoice_submission(&invoices, &params);
  swift_bill_core::continuity::reconcile_row_count(invoices.len(), rows.len())?;
  let total_amount: f64 = invoices.iter().map(|i| i.total_cost).sum();

  Ok(InvoiceSubmissionPreview {
    rows,
    carry_forward: CarryForward {
      next_reg_no,
      next_running,
      next_po_no: 0,
      next_purchase_no: 0,
      remaining_balance: 0.0,
    },
    total_rows: invoices.len(),
    total_amount,
  })
}

#[tauri::command]
async fn export_invoice_submission_excel(
  params: InvoiceSubmissionExcelParams,
) -> Result<GenerateResult, String> {
  let n = params.rows.len() as u32;
  let (next_reg_no, next_running) =
    swift_bill_core::reports::compute_next_reg(&params.start_reg_no, params.start_running, n);

  let total_amount: f64 = params.rows.iter().map(|r| r.total_amount).sum();
  let total_rows = params.rows.len();

  let output_dir = prepare_output_dir(&params.output_dir)?;
  let file = generate_invoice_submission_excel(
    &params.rows,
    params.year,
    params.month,
    params.round,
    &output_dir,
  )?;

  Ok(GenerateResult {
    files: vec![file],
    total_rows,
    total_amount,
    carry_forward: CarryForward {
      next_reg_no,
      next_running,
      next_po_no: 0,
      next_purchase_no: 0,
      remaining_balance: 0.0,
    },
  })
}

// Tauri commands: Receiving Summary (สรุปรับยา) Preview + Excel export

#[tauri::command]
async fn preview_receiving_summary(
  app: tauri::AppHandle,
  params: ReceivingSummaryParams,
) -> Result<ReceivingSummaryPreview, String> {
  let invoices = fetch_invoices(&params.db_config, &params.date_from, &params.date_to)
    .await
    .map_err(|e| e.to_string())?;
  if invoices.is_empty() {
    return Err("ไม่พบข้อมูลในช่วงวันที่ที่เลือก".to_string());
  }

  let n = invoices.len() as u32;
  let (next_reg_no, next_running) =
    swift_bill_core::reports::compute_next_reg(&params.start_reg_no, params.start_running, n);
  let locks = number_locks::load_number_locks(&app)?;
  let allocation = swift_bill_core::numbering::allocate_receiving_numbers(
    params.year,
    params.start_po_no,
    params.start_purchase_no,
    n,
    &locks,
  );
  let rows = swift_bill_core::reports::process_receiving_summary_with_numbers(
    &invoices,
    &params,
    &allocation.assignments,
  );
  swift_bill_core::continuity::reconcile_row_count(invoices.len(), rows.len())?;
  let total_amount: f64 = invoices.iter().map(|i| i.total_cost).sum();

  Ok(ReceivingSummaryPreview {
    rows,
    carry_forward: CarryForward {
      next_reg_no,
      next_running,
      next_po_no: allocation.next_po_no,
      next_purchase_no: allocation.next_purchase_no,
      remaining_balance: 0.0,
    },
    total_rows: invoices.len(),
    total_amount,
    numbering_info: allocation.numbering_info,
  })
}

#[tauri::command]
async fn export_receiving_summary_excel(
  app: tauri::AppHandle,
  params: ReceivingSummaryExcelParams,
) -> Result<ReceivingSummaryGenerateResult, String> {
  let n = params.rows.len() as u32;
  let (next_reg_no, next_running) =
    swift_bill_core::reports::compute_next_reg(&params.start_reg_no, params.start_running, n);
  let locks = number_locks::load_number_locks(&app)?;
  let allocation = swift_bill_core::numbering::allocate_receiving_numbers(
    params.year,
    params.start_po_no,
    params.start_purchase_no,
    n,
    &locks,
  );

  let total_amount: f64 = params.rows.iter().map(|r| r.total_amount).sum();
  let total_rows = params.rows.len();

  let output_dir = prepare_output_dir(&params.output_dir)?;
  let file = generate_receiving_summary_excel(
    &params.rows,
    params.year,
    params.month,
    params.round,
    &output_dir,
  )?;

  Ok(ReceivingSummaryGenerateResult {
    files: vec![file],
    total_rows,
    total_amount,
    carry_forward: CarryForward {
      next_reg_no,
      next_running,
      next_po_no: allocation.next_po_no,
      next_purchase_no: allocation.next_purchase_no,
      remaining_balance: 0.0,
    },
    numbering_info: allocation.numbering_info,
  })
}

// Tauri commands: Cover Letters (เบิกยาปะหน้า)

#[tauri::command]
async fn generate_cover_letters(params: CoverLettersParams) -> Result<GenerateResult, String> {
  let invoices = fetch_invoices(&params.db_config, &params.date_from, &params.date_to)
    .await
    .map_err(|e| e.to_string())?;
  if invoices.is_empty() {
    return Err("ไม่พบข้อมูลในช่วงวันที่ที่เลือก".to_string());
  }

  let pages = swift_bill_core::reports::process_cover_letters(&invoices, &params);
  swift_bill_core::continuity::reconcile_row_count(invoices.len(), pages.len())?;
  let remaining_balance = pages
    .last()
    .map(|p| p.remaining_balance)
    .unwrap_or(params.previous_balance);
  let total_amount: f64 = invoices.iter().map(|i| i.total_cost).sum();

  let output_dir = prepare_output_dir(&params.output_dir)?;
  let params_out = CoverLettersParams {
    output_dir,
    ..params
  };
  let file = generate_cover_letters_pdf(&pages, &params_out)?;

  Ok(GenerateResult {
    files: vec![file],
    total_rows: invoices.len(),
    total_amount,
    carry_forward: CarryForward {
      next_reg_no: String::new(),
      next_running: 0,
      next_po_no: 0,
      next_purchase_no: 0,
      remaining_balance,
    },
  })
}

// Round validation command

#[tauri::command]
async fn validate_round_params(
  app: tauri::AppHandle,
  params: ValidateRoundParams,
) -> Result<RoundValidation, String> {
  let locks = number_locks::load_number_locks(&app)?;
  let entries = history::load_history(&app)?;
  let history_store = RoundHistory { entries };

  // Register-number (เลขทะเบียนคุม) conflicts + normalized start.
  let register_conflicts = swift_bill_core::numbering::find_register_conflicts(
    params.fiscal_year,
    &params.start_reg_no,
    params.start_running,
    params.invoice_count,
    &locks,
  );
  let register_numbering_info = swift_bill_core::numbering::normalize_register_start_numbers(
    params.fiscal_year,
    &params.start_reg_no,
    params.start_running,
    &locks,
  );

  // Receiving-number (request/report/purchase) allocation, honouring locks.
  let receiving_allocation = swift_bill_core::numbering::allocate_receiving_numbers(
    params.fiscal_year,
    params.start_po_no,
    params.start_purchase_no,
    params.invoice_count,
    &locks,
  );

  // Budget carry-forward check against the last recorded round.
  let budget = swift_bill_core::continuity::validate_budget_carry_forward(
    params.fiscal_year,
    params.month,
    params.previous_balance,
    &history_store,
    0.01,
  );

  // Projected next values for the continuity preview card.
  let (next_reg_no, next_running) = swift_bill_core::reports::compute_next_reg(
    &params.start_reg_no,
    params.start_running,
    params.invoice_count,
  );

  let carry_forward = CarryForward {
    next_reg_no,
    next_running,
    next_po_no: receiving_allocation.next_po_no,
    next_purchase_no: receiving_allocation.next_purchase_no,
    remaining_balance: params.previous_balance,
  };

  Ok(RoundValidation {
    register_conflicts,
    register_numbering_info,
    receiving_numbering_info: receiving_allocation.numbering_info,
    budget,
    carry_forward,
  })
}

// History commands

#[tauri::command]
async fn load_round_history(app: tauri::AppHandle) -> Result<Vec<RoundHistoryEntry>, String> {
  history::load_history(&app)
}

#[tauri::command]
async fn save_round_entry(app: tauri::AppHandle, entry: RoundHistoryEntry) -> Result<(), String> {
  history::upsert_entry(&app, entry)
}

#[tauri::command]
async fn delete_round_entry(app: tauri::AppHandle, id: String) -> Result<(), String> {
  history::delete_entry(&app, &id)
}

#[tauri::command]
async fn load_number_locks(app: tauri::AppHandle) -> Result<Vec<NumberLockEntry>, String> {
  number_locks::load_number_locks(&app)
}

#[tauri::command]
async fn create_number_locks(
  app: tauri::AppHandle,
  params: NumberLockBatchParams,
) -> Result<Vec<NumberLockEntry>, String> {
  number_locks::create_number_locks(&app, params)
}

#[tauri::command]
async fn delete_number_lock(app: tauri::AppHandle, id: String) -> Result<(), String> {
  number_locks::delete_number_lock(&app, &id)
}

#[tauri::command]
async fn normalize_receiving_start(
  app: tauri::AppHandle,
  params: NormalizeReceivingStartParams,
) -> Result<swift_bill_core::ReceivingNumberingInfo, String> {
  let locks = number_locks::load_number_locks(&app)?;
  Ok(
    swift_bill_core::numbering::normalize_receiving_start_numbers(
      params.fiscal_year,
      params.start_po_no,
      params.start_purchase_no,
      &locks,
    ),
  )
}

// App entrypoint

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_dialog::init())
    .invoke_handler(tauri::generate_handler![
      test_connection,
      fetch_preview,
      preview_invoice_submission,
      export_invoice_submission_excel,
      preview_receiving_summary,
      export_receiving_summary_excel,
      generate_cover_letters,
      load_round_history,
      save_round_entry,
      delete_round_entry,
      load_number_locks,
      create_number_locks,
      delete_number_lock,
      normalize_receiving_start,
      validate_round_params,
      save_db_config,
      load_db_config,
      delete_db_config,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
