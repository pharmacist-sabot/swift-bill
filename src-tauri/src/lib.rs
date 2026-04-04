mod db;
mod excel;
mod history;
mod models;
mod pdf;
mod queries;
mod reports;

use models::{
    CarryForward, CoverLettersParams, DbConfig, GenerateResult, InvoiceSubmissionExcelParams,
    InvoiceSubmissionParams, InvoiceSubmissionPreview, PreviewData, ReceivingSummaryExcelParams,
    ReceivingSummaryParams, ReceivingSummaryPreview, RoundHistoryEntry,
};

// ── Helper ────────────────────────────────────────────────────────────────

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

// ── Tauri commands — connection & preview ─────────────────────────────────

#[tauri::command]
async fn test_connection(config: DbConfig) -> Result<String, String> {
    let mut client = db::connect(&config).await?;
    client
        .simple_query("SELECT 1")
        .await
        .map_err(|e| format!("Query failed: {e}"))?;
    Ok("เชื่อมต่อฐานข้อมูลสำเร็จ".to_string())
}

#[tauri::command]
async fn fetch_preview(
    config: DbConfig,
    date_from: String,
    date_to: String,
) -> Result<PreviewData, String> {
    let invoices = queries::fetch_invoices(&config, &date_from, &date_to).await?;
    let total_amount: f64 = invoices.iter().map(|i| i.total_cost).sum();
    let row_count = invoices.len();
    Ok(PreviewData {
        invoices,
        total_amount,
        row_count,
    })
}

// ── Tauri commands — ส่งหนี้เบิกยา preview + Excel export ────────────────

#[tauri::command]
async fn preview_invoice_submission(
    params: InvoiceSubmissionParams,
) -> Result<InvoiceSubmissionPreview, String> {
    let invoices =
        queries::fetch_invoices(&params.db_config, &params.date_from, &params.date_to).await?;
    if invoices.is_empty() {
        return Err("ไม่พบข้อมูลในช่วงวันที่ที่เลือก".to_string());
    }

    let n = invoices.len() as u32;
    let (next_reg_no, next_running) =
        reports::compute_next_reg(&params.start_reg_no, params.start_running, n);

    let rows = reports::process_invoice_submission(&invoices, &params);
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
        reports::compute_next_reg(&params.start_reg_no, params.start_running, n);

    let total_amount: f64 = params.rows.iter().map(|r| r.total_amount).sum();
    let total_rows = params.rows.len();

    let output_dir = prepare_output_dir(&params.output_dir)?;
    let file = excel::generate_invoice_submission_excel(
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

// ── Tauri commands — สรุปรับยา preview + Excel export ────────────────────

#[tauri::command]
async fn preview_receiving_summary(
    params: ReceivingSummaryParams,
) -> Result<ReceivingSummaryPreview, String> {
    let invoices =
        queries::fetch_invoices(&params.db_config, &params.date_from, &params.date_to).await?;
    if invoices.is_empty() {
        return Err("ไม่พบข้อมูลในช่วงวันที่ที่เลือก".to_string());
    }

    let n = invoices.len() as u32;
    let (next_reg_no, next_running) =
        reports::compute_next_reg(&params.start_reg_no, params.start_running, n);
    // request_no increments +2 per row, so the next batch must start at start_po_no + n*2
    let next_po_no = params.start_po_no + n * 2;

    let rows = reports::process_receiving_summary(&invoices, &params);
    let total_amount: f64 = invoices.iter().map(|i| i.total_cost).sum();

    Ok(ReceivingSummaryPreview {
        rows,
        carry_forward: CarryForward {
            next_reg_no,
            next_running,
            next_po_no,
            next_purchase_no: params.start_purchase_no + n,
            remaining_balance: 0.0,
        },
        total_rows: invoices.len(),
        total_amount,
    })
}

#[tauri::command]
async fn export_receiving_summary_excel(
    params: ReceivingSummaryExcelParams,
) -> Result<GenerateResult, String> {
    let n = params.rows.len() as u32;
    let (next_reg_no, next_running) =
        reports::compute_next_reg(&params.start_reg_no, params.start_running, n);
    // request_no increments +2 per row, so the next batch must start at start_po_no + n*2
    let next_po_no = params.start_po_no + n * 2;

    let total_amount: f64 = params.rows.iter().map(|r| r.total_amount).sum();
    let total_rows = params.rows.len();

    let output_dir = prepare_output_dir(&params.output_dir)?;
    let file = excel::generate_receiving_summary_excel(
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
            next_po_no,
            next_purchase_no: params.start_purchase_no + n,
            remaining_balance: 0.0,
        },
    })
}

// ── Tauri commands — เบิกยาปะหน้า (PDF, unchanged) ───────────────────────

#[tauri::command]
async fn generate_cover_letters(params: CoverLettersParams) -> Result<GenerateResult, String> {
    let invoices =
        queries::fetch_invoices(&params.db_config, &params.date_from, &params.date_to).await?;
    if invoices.is_empty() {
        return Err("ไม่พบข้อมูลในช่วงวันที่ที่เลือก".to_string());
    }

    let pages = reports::process_cover_letters(&invoices, &params);
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
    let file = pdf::generate_cover_letters_pdf(&pages, &params_out)?;

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

// ── History commands ──────────────────────────────────────────────────────

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

// ── App entrypoint ────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
