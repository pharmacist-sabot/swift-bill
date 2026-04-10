//! Excel export for the two main reports.
//!
//! Uses `rust_xlsxwriter` — pure Rust, no external process required.

use std::path::Path;

use rust_xlsxwriter::Workbook;

use crate::models::{InvoiceSubmissionRow, ReceivingSummaryRow};

// internal helper

/// Convert an `XlsxError` into a plain `String` so callers return `Result<_, String>`.
fn xl<T>(r: Result<T, rust_xlsxwriter::XlsxError>) -> Result<T, String> {
  r.map_err(|e| format!("Excel error: {e}"))
}

fn thai_month(m: u32) -> &'static str {
  match m {
    1 => "มกราคม",
    2 => "กุมภาพันธ์",
    3 => "มีนาคม",
    4 => "เมษายน",
    5 => "พฤษภาคม",
    6 => "มิถุนายน",
    7 => "กรกฎาคม",
    8 => "สิงหาคม",
    9 => "กันยายน",
    10 => "ตุลาคม",
    11 => "พฤศจิกายน",
    12 => "ธันวาคม",
    _ => "",
  }
}

// 1. ส่งหนี้เบิกยา (Invoice Submission List)

/// Generate an `.xlsx` file for **ส่งหนี้เบิกยา** (Invoice Submission List).
///
/// Columns:
///   ลำดับ | วันที่รับของ | เลขที่เอกสาร | เลขทะเบียนคุม | ลำดับ |
///   วัน/เดือน/ปีใบส่งของ | รหัสบริษัท | ค่าใช้จ่ายเรื่อง | จำนวนเงินรวม
pub fn generate_invoice_submission_excel(
  rows: &[InvoiceSubmissionRow],
  year: i32,
  month: u32,
  round: u32,
  output_dir: &str,
) -> Result<String, String> {
  let mut workbook = Workbook::new();
  let ws = workbook.add_worksheet();

  let month_name = thai_month(month);

  // Row 0: title
  xl(ws.write(
    0,
    0,
    format!(
      "ส่งรายการหนี้สินและเอกสารเบิกเงิน  เดือน{}  รอบ {}  ปีงบประมาณ {}",
      month_name, round, year
    ),
  ))?;

  // Row 1: column headers
  let headers: &[&str] = &[
    "ลำดับ",
    "วันที่รับของ",
    "เลขที่เอกสาร",
    "เลขทะเบียนคุม",
    "ลำดับ",
    "วัน/เดือน/ปีใบส่งของ",
    "รหัสบริษัท",
    "ค่าใช้จ่ายเรื่อง",
    "จำนวนเงินรวม",
  ];
  for (col, h) in headers.iter().enumerate() {
    xl(ws.write(1, col as u16, *h))?;
  }

  // Rows 2+: data
  let mut grand_total = 0.0_f64;
  for (i, row) in rows.iter().enumerate() {
    let r = (i as u32) + 2;
    xl(ws.write(r, 0, row.seq as f64))?;
    xl(ws.write(r, 1, row.receive_date.as_str()))?;
    xl(ws.write(r, 2, row.invoice_no.as_str()))?;
    xl(ws.write(r, 3, row.reg_no.as_str()))?;
    xl(ws.write(r, 4, row.running_in_reg as f64))?;
    xl(ws.write(r, 5, row.invoice_date.as_str()))?;
    xl(ws.write(r, 6, row.company_name.as_str()))?;
    xl(ws.write(r, 7, row.category.as_str()))?;
    xl(ws.write(r, 8, row.total_amount))?;
    grand_total += row.total_amount;
  }

  // Total row
  let total_r = (rows.len() as u32) + 2;
  xl(ws.write(total_r, 7, "รวมทั้งสิ้น"))?;
  xl(ws.write(total_r, 8, grand_total))?;

  // Save
  let filename = format!("ส่งหนี้เบิกยา_{year}_เดือน{month}_รอบ{round}.xlsx");
  let path = Path::new(output_dir).join(&filename);
  let path_str = path.to_string_lossy().to_string();
  workbook
    .save(&path_str)
    .map_err(|e| format!("บันทึก Excel ไม่สำเร็จ: {e}"))?;

  Ok(path_str)
}

// 2. สรุปรับยา (Receiving Summary)

/// Generate an `.xlsx` file for **สรุปรับยา** (Receiving Summary).
///
/// Columns:
///   วันที่ขออนุมัติ | วันที่สั่งซื้อ | วันที่รับของ | รหัสบริษัท |
///   จำนวนเงินรวม | รหัสลงรับยา | เลขทะเบียนคุม | ลำดับ | เลขที่ลงรับ |
///   ขอซื้อ (ลบ0033.302/) | รายงาน/อนุมัติ (ลบ0033.302/) | ใบสั่งซื้อ…/{year}
pub fn generate_receiving_summary_excel(
  rows: &[ReceivingSummaryRow],
  year: i32,
  month: u32,
  round: u32,
  output_dir: &str,
) -> Result<String, String> {
  let mut workbook = Workbook::new();
  let ws = workbook.add_worksheet();

  let month_name = thai_month(month);

  // Row 0: title
  xl(ws.write(
    0,
    0,
    format!(
      "สรุปรับยาเดือน{}  รอบ {}  ปีงบประมาณ {}",
      month_name, round, year
    ),
  ))?;

  // Row 1: column headers
  // Static headers (cols 0–10)
  let static_headers: &[&str] = &[
    "วันที่ขออนุมัติ",
    "วันที่สั่งซื้อ",
    "วันที่รับของ",
    "รหัสบริษัท",
    "จำนวนเงินรวม",
    "รหัสลงรับยา",
    "เลขทะเบียนคุม",
    "ลำดับ",
    "เลขที่ลงรับ",
    "ขอซื้อ (ลบ0033.302/)",
    "รายงาน/อนุมัติ (ลบ0033.302/)",
  ];
  for (col, h) in static_headers.iter().enumerate() {
    xl(ws.write(1, col as u16, *h))?;
  }
  // Col 11: dynamic header that includes the year
  let year_header = format!("ใบสั่งซื้อ…/{}", year);
  xl(ws.write(1, 11_u16, year_header.as_str()))?;

  // Rows 2+: data
  let mut grand_total = 0.0_f64;
  for (i, row) in rows.iter().enumerate() {
    let r = (i as u32) + 2;
    xl(ws.write(r, 0, row.approval_date.as_str()))?;
    xl(ws.write(r, 1, row.po_date.as_str()))?;
    xl(ws.write(r, 2, row.receive_date.as_str()))?;
    xl(ws.write(r, 3, row.company_code.as_str()))?;
    xl(ws.write(r, 4, row.total_amount))?;
    xl(ws.write(r, 5, row.receiving_code as f64))?;
    xl(ws.write(r, 6, row.reg_no.as_str()))?;
    xl(ws.write(r, 7, row.running_in_reg as f64))?;
    xl(ws.write(r, 8, row.invoice_no.as_str()))?;
    xl(ws.write(r, 9, row.request_no as f64))?;
    xl(ws.write(r, 10, row.report_no as f64))?;
    xl(ws.write(r, 11, row.po_no as f64))?;
    grand_total += row.total_amount;
  }

  // Total row
  let total_r = (rows.len() as u32) + 2;
  xl(ws.write(total_r, 3, "รวมทั้งสิ้น"))?;
  xl(ws.write(total_r, 4, grand_total))?;

  // Save
  let filename = format!("สรุปรับยา_{year}_เดือน{month}_รอบ{round}.xlsx");
  let path = Path::new(output_dir).join(&filename);
  let path_str = path.to_string_lossy().to_string();
  workbook
    .save(&path_str)
    .map_err(|e| format!("บันทึก Excel ไม่สำเร็จ: {e}"))?;

  Ok(path_str)
}
