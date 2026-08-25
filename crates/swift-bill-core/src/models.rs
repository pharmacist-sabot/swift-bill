use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Database connection parameters for the INVS SQL Server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConfig {
  pub host: String,
  pub port: u16,
  pub database: String,
  pub username: String,
  pub password: String,
}

/// A normalized invoice row produced by [`crate::reports`] consumers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceRow {
  pub invoice_no: String,
  pub vendor_code: String,
  pub company_name: String,
  pub company_keyword: String,
  pub total_cost: f64,
  pub receive_date: NaiveDate,
  /// Must be either "ยา" or "วัสดุเภสัชกรรม"
  pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewData {
  pub invoices: Vec<InvoiceRow>,
  pub total_amount: f64,
  pub row_count: usize,
}

/// Carry-forward values for the next round processing.
/// Included in every `GenerateResult`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarryForward {
  /// Format: e.g., "69ภ13"
  pub next_reg_no: String,
  /// Next available slot (0-9) within the register
  pub next_running: u32,
  /// Next request/report start number for ReceivingSummary
  pub next_po_no: u32,
  /// Next purchase order start number for ReceivingSummary
  pub next_purchase_no: u32,
  /// Applicable only for CoverLetters
  pub remaining_balance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResult {
  pub files: Vec<String>,
  pub total_rows: usize,
  pub total_amount: f64,
  pub carry_forward: CarryForward,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkippedLockedNumberSet {
  pub request_no: u32,
  pub report_no: u32,
  pub purchase_no: u32,
  pub reason: String,
  pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivingNumberingInfo {
  pub start_po_no: u32,
  pub start_purchase_no: u32,
  pub skipped_locked_sets: Vec<SkippedLockedNumberSet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivingSummaryGenerateResult {
  pub files: Vec<String>,
  pub total_rows: usize,
  pub total_amount: f64,
  pub carry_forward: CarryForward,
  pub numbering_info: ReceivingNumberingInfo,
}

// Intermediate row types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceSubmissionRow {
  pub seq: u32,
  pub receive_date: String,
  pub invoice_no: String,
  pub reg_no: String,
  pub running_in_reg: u32,
  pub invoice_date: String,
  pub company_name: String,
  pub category: String,
  pub total_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivingSummaryRow {
  pub approval_date: String,
  pub po_date: String,
  pub receive_date: String,
  pub company_code: String,
  pub total_amount: f64,
  pub receiving_code: u32,
  pub reg_no: String,
  pub running_in_reg: u32,
  pub invoice_no: String,
  pub request_no: u32,
  pub report_no: u32,
  pub po_no: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivingNumberAssignment {
  pub request_no: u32,
  pub report_no: u32,
  pub purchase_no: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverLetterPage {
  pub company_name: String,
  pub category: String,
  pub budget_total: f64,
  pub previous_spent: f64,
  pub previous_balance: f64,
  pub current_amount: f64,
  pub remaining_balance: f64,
  pub fiscal_year: String,
  pub date_text: String,
}

// Per-report parameters

/// Parameters for Invoice Submission (ส่งหนี้เบิกยา)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceSubmissionParams {
  pub db_config: DbConfig,
  /// Date in YYYYMMDD (Gregorian) format
  pub date_from: String,
  /// Date in YYYYMMDD (Gregorian) format
  pub date_to: String,
  /// Buddhist year for display and filename (e.g., 2568)
  pub year: i32,
  pub month: u32,
  pub round: u32,
  /// Format: e.g., "69ภ12"
  pub start_reg_no: String,
  /// Available slot (0-9)
  pub start_running: u32,
  pub output_dir: String,
}

/// Parameters for Receiving Summary (สรุปรับยา)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivingSummaryParams {
  pub db_config: DbConfig,
  pub date_from: String,
  pub date_to: String,
  pub year: i32,
  pub month: u32,
  pub round: u32,
  /// Starting number for request and approval reports
  pub start_po_no: u32,
  /// Starting number for purchase orders (independent counter)
  pub start_purchase_no: u32,
  pub start_reg_no: String,
  pub start_running: u32,
  pub approval_date: Option<String>,
  pub output_dir: String,
}

/// Parameters for Cover Letters (เบิกยาปะหน้า)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverLettersParams {
  pub db_config: DbConfig,
  pub date_from: String,
  pub date_to: String,
  pub year: i32,
  pub month: u32,
  pub round: u32,
  pub budget_total: f64,
  pub previous_balance: f64,
  pub approval_date: Option<String>,
  pub output_dir: String,
}

// Round history

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundHistoryEntry {
  /// Timestamp-based unique identifier
  pub id: String,
  /// Descriptive label (e.g., "ม.ค. 2568 รอบ 1")
  pub label: String,
  /// Buddhist year
  pub fiscal_year: i32,
  pub month: u32,
  pub round: u32,
  /// Date in YYYYMMDD
  pub date_from: String,
  /// Date in YYYYMMDD
  pub date_to: String,
  pub next_reg_no: String,
  pub next_running: u32,
  pub next_po_no: u32,
  /// Independent counter for purchase orders. Defaults to 0 for older entries.
  #[serde(default)]
  pub next_purchase_no: u32,
  pub remaining_balance: f64,
  pub budget_total: f64,
  pub total_amount: f64,
  pub invoice_count: u32,
  /// ISO 8601 datetime
  pub created_at: String,
  #[serde(default)]
  pub source_tab: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoundHistory {
  pub entries: Vec<RoundHistoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumberLockEntry {
  pub id: String,
  pub fiscal_year: i32,
  pub request_no: u32,
  pub report_no: u32,
  pub purchase_no: u32,
  /// Optional locked register starting point (เลขทะเบียนคุม), e.g. `"69ภ12"`.
  /// When present, the locked slot is `(start_reg_no, running)`.
  #[serde(default)]
  pub start_reg_no: Option<String>,
  /// Running slot (0–9) within the locked register, required when
  /// `start_reg_no` is set.
  #[serde(default)]
  pub running: Option<u32>,
  pub reason: String,
  #[serde(default)]
  pub note: String,
  pub created_at: String,
}

/// A register slot (เลขทะเบียนคุม) that collides with a locked entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterConflict {
  /// Register number string, e.g. `"69ภ12"`.
  pub reg_no: String,
  /// Running position (0–9) within the register.
  pub running_in_reg: u32,
  /// Reason copied from the matching lock entry.
  pub reason: String,
}

/// Diagnostic result of a register-number allocation that skips locked slots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterNumberingInfo {
  pub start_reg_no: String,
  pub start_running: u32,
  pub skipped_conflicts: Vec<RegisterConflict>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NumberLockStore {
  pub entries: Vec<NumberLockEntry>,
}

// Preview results

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceSubmissionPreview {
  pub rows: Vec<InvoiceSubmissionRow>,
  pub carry_forward: CarryForward,
  pub total_rows: usize,
  pub total_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivingSummaryPreview {
  pub rows: Vec<ReceivingSummaryRow>,
  pub carry_forward: CarryForward,
  pub total_rows: usize,
  pub total_amount: f64,
  pub numbering_info: ReceivingNumberingInfo,
}

// Excel export parameters

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceSubmissionExcelParams {
  pub rows: Vec<InvoiceSubmissionRow>,
  pub year: i32,
  pub month: u32,
  pub round: u32,
  pub start_reg_no: String,
  pub start_running: u32,
  pub output_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceivingSummaryExcelParams {
  pub rows: Vec<ReceivingSummaryRow>,
  pub year: i32,
  pub month: u32,
  pub round: u32,
  pub start_po_no: u32,
  pub start_purchase_no: u32,
  pub start_reg_no: String,
  pub start_running: u32,
  pub output_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumberLockBatchParams {
  pub fiscal_year: i32,
  pub start_request_no: u32,
  pub start_purchase_no: u32,
  pub count: u32,
  pub reason: String,
  #[serde(default)]
  pub note: String,
  /// Optional register starting point (เลขทะเบียนคุม) to lock, e.g. `"69ภ12"`.
  #[serde(default)]
  pub start_reg_no: Option<String>,
  /// Running slot (0–9) within the locked register, required when
  /// `start_reg_no` is set.
  #[serde(default)]
  pub running: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizeReceivingStartParams {
  pub fiscal_year: i32,
  pub start_po_no: u32,
  pub start_purchase_no: u32,
}

/// Parameters for the round pre-generation validation command.
///
/// Carries only the values the user would enter for a new round plus the
/// planned invoice count (already known from the query preview), so validation
/// is a pure check that needs no database access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateRoundParams {
  pub fiscal_year: i32,
  pub month: u32,
  pub round: u32,
  /// Format: e.g., "69ภ12"
  pub start_reg_no: String,
  /// Available slot (0–9)
  pub start_running: u32,
  /// Starting number for request and approval reports
  pub start_po_no: u32,
  /// Starting number for purchase orders (independent counter)
  pub start_purchase_no: u32,
  /// Entered remaining budget (previous_balance) for the new round
  pub previous_balance: f64,
  /// Number of invoices planned for this round
  pub invoice_count: u32,
}

/// Result of a round pre-generation validation: every continuity check the
/// app can perform before generating a single PDF.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundValidation {
  /// Register slots (เลขทะเบียนคุม) that would collide with a locked entry.
  pub register_conflicts: Vec<RegisterConflict>,
  /// Diagnostic info for the register-number start (after skipping conflicts).
  pub register_numbering_info: RegisterNumberingInfo,
  /// Diagnostic info for the receiving-number start (after skipping locks).
  pub receiving_numbering_info: ReceivingNumberingInfo,
  /// Budget carry-forward check against the last recorded round.
  pub budget: crate::continuity::BudgetValidation,
  /// Projected next values for the round, for the continuity preview card.
  pub carry_forward: CarryForward,
}
