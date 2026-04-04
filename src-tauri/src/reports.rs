use chrono::{Datelike, NaiveDate};

use crate::models::{
    CoverLetterPage, CoverLettersParams, InvoiceRow, InvoiceSubmissionParams, InvoiceSubmissionRow,
    ReceivingSummaryParams, ReceivingSummaryRow,
};

/// Format a NaiveDate as a Thai short date string, e.g. "5 ม.ค. 69"
fn format_thai_date(date: &NaiveDate) -> String {
    let thai_months = [
        "ม.ค.",
        "ก.พ.",
        "มี.ค.",
        "เม.ย.",
        "พ.ค.",
        "มิ.ย.",
        "ก.ค.",
        "ส.ค.",
        "ก.ย.",
        "ต.ค.",
        "พ.ย.",
        "ธ.ค.",
    ];
    let buddhist_year = date.year() + 543;
    let short_year = buddhist_year % 100;
    format!(
        "{} {} {}",
        date.day(),
        thai_months[date.month0() as usize],
        short_year
    )
}

/// Parse a register number string like "69ภ12" into (prefix, number).
fn parse_reg_no(reg: &str) -> (String, u32) {
    let last_non_digit = reg.char_indices().rev().find(|(_, c)| !c.is_ascii_digit());
    match last_non_digit {
        Some((idx, ch)) => {
            let prefix_end = idx + ch.len_utf8();
            let prefix = &reg[..prefix_end];
            let num_str = &reg[prefix_end..];
            let num = num_str.parse::<u32>().unwrap_or(0);
            (prefix.to_string(), num)
        }
        None => {
            let num = reg.parse::<u32>().unwrap_or(0);
            (String::new(), num)
        }
    }
}

/// Format a register string from prefix + number, e.g. ("69ภ", 12) → "69ภ12"
fn format_reg_no(prefix: &str, number: u32) -> String {
    format!("{}{}", prefix, number)
}

/// Compute register number and running position within the register.
///
/// Each register holds 10 items (positions 0–9). When the position reaches 10,
/// the register number increments and the position resets to 0.
///
/// `item_index`   – zero-based index of the current item in this batch.
/// `start_running`– starting position within the first register (0–9).
/// `start_reg_num`– numeric portion of the first register number.
fn compute_reg_for_item(item_index: u32, start_running: u32, start_reg_num: u32) -> (u32, u32) {
    let absolute_pos = start_running + item_index;
    let reg_offset = absolute_pos / 10;
    let running_in_reg = absolute_pos % 10;
    (start_reg_num + reg_offset, running_in_reg)
}

// ---------------------------------------------------------------------------
// 1. Invoice Submission List  (ส่งหนี้เบิกยา)
// ---------------------------------------------------------------------------

pub fn process_invoice_submission(
    invoices: &[InvoiceRow],
    params: &InvoiceSubmissionParams,
) -> Vec<InvoiceSubmissionRow> {
    let (reg_prefix, reg_start_num) = parse_reg_no(&params.start_reg_no);
    let mut rows: Vec<InvoiceSubmissionRow> = Vec::with_capacity(invoices.len());

    for (i, inv) in invoices.iter().enumerate() {
        let seq = (i as u32) + 1;
        let (reg_num, running) =
            compute_reg_for_item(i as u32, params.start_running, reg_start_num);
        let reg_no = format_reg_no(&reg_prefix, reg_num);

        let receive_date_str = format_thai_date(&inv.receive_date);
        let invoice_date_str = receive_date_str.clone();

        rows.push(InvoiceSubmissionRow {
            seq,
            receive_date: receive_date_str,
            invoice_no: inv.invoice_no.clone(),
            reg_no,
            running_in_reg: running,
            invoice_date: invoice_date_str,
            company_name: inv.company_name.clone(),
            category: inv.category.clone(),
            total_amount: inv.total_cost,
        });
    }

    rows
}

// ---------------------------------------------------------------------------
// 2. Receiving Summary  (สรุปรับยา)
// ---------------------------------------------------------------------------

pub fn process_receiving_summary(
    invoices: &[InvoiceRow],
    params: &ReceivingSummaryParams,
) -> Vec<ReceivingSummaryRow> {
    let (reg_prefix, reg_start_num) = parse_reg_no(&params.start_reg_no);
    let mut rows: Vec<ReceivingSummaryRow> = Vec::with_capacity(invoices.len());

    let mut request_no = params.start_po_no;
    let mut report_no = params.start_po_no + 1;
    let mut po_no = params.start_purchase_no;

    // If approval_date is absent or blank, fall back to the first invoice's
    // receive_date — matching the UI hint "ปล่อยว่าง = ใช้วันที่รับของจากบิลแรก"
    // and the same pattern used in process_cover_letters.
    let approval_date_str = params
        .approval_date
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .unwrap_or_else(|| {
            invoices
                .first()
                .map(|inv| format_thai_date(&inv.receive_date))
                .unwrap_or_default()
        });

    for (i, inv) in invoices.iter().enumerate() {
        let (reg_num, running) =
            compute_reg_for_item(i as u32, params.start_running, reg_start_num);
        let reg_no = format_reg_no(&reg_prefix, reg_num);

        let receive_date_str = format_thai_date(&inv.receive_date);
        let po_date_str = receive_date_str.clone();

        let receiving_code = (i as u32) + 1;

        rows.push(ReceivingSummaryRow {
            approval_date: approval_date_str.clone(),
            po_date: po_date_str,
            receive_date: receive_date_str,
            company_code: inv.company_keyword.clone(),
            total_amount: inv.total_cost,
            receiving_code,
            reg_no,
            running_in_reg: running,
            invoice_no: inv.invoice_no.clone(),
            request_no,
            report_no,
            po_no,
        });

        request_no += 2;
        report_no += 2;
        po_no += 1;
    }

    rows
}

// ---------------------------------------------------------------------------
// 3. Cover Letters  (เบิกยาปะหน้า)
// ---------------------------------------------------------------------------

pub fn process_cover_letters(
    invoices: &[InvoiceRow],
    params: &CoverLettersParams,
) -> Vec<CoverLetterPage> {
    // fiscal_year is already a Buddhist year supplied by the frontend
    let fiscal_year = format!("{}", params.year);

    let date_text = if let Some(ref dt) = params.approval_date {
        dt.clone()
    } else {
        invoices
            .first()
            .map(|inv| format_thai_date(&inv.receive_date))
            .unwrap_or_default()
    };

    let mut pages: Vec<CoverLetterPage> = Vec::with_capacity(invoices.len());

    let mut running_balance = params.previous_balance;
    let total_budget = params.budget_total;
    let mut cumulative_spent = total_budget - params.previous_balance;

    for inv in invoices.iter() {
        let previous_balance = running_balance;
        let current_amount = inv.total_cost;
        let remaining_balance = previous_balance - current_amount;

        pages.push(CoverLetterPage {
            company_name: inv.company_name.clone(),
            category: inv.category.clone(),
            budget_total: total_budget,
            previous_spent: cumulative_spent,
            previous_balance,
            current_amount,
            remaining_balance,
            fiscal_year: fiscal_year.clone(),
            date_text: date_text.clone(),
        });

        running_balance = remaining_balance;
        cumulative_spent += current_amount;
    }

    pages
}

// ---------------------------------------------------------------------------
// Compute next reg_no and running slot after processing `count` items.
// ---------------------------------------------------------------------------

/// Returns `(next_reg_no_string, next_running_slot)` — the register number and
/// position that the *next* batch should start from.
pub fn compute_next_reg(start_reg_no: &str, start_running: u32, count: u32) -> (String, u32) {
    let (prefix, start_num) = parse_reg_no(start_reg_no);
    let next_absolute = start_running + count;
    let reg_offset = next_absolute / 10;
    let next_running = next_absolute % 10;
    let next_reg_no = format_reg_no(&prefix, start_num + reg_offset);
    (next_reg_no, next_running)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        CoverLettersParams, DbConfig, InvoiceSubmissionParams, ReceivingSummaryParams,
    };

    fn sample_invoices() -> Vec<InvoiceRow> {
        vec![
            InvoiceRow {
                invoice_no: "INV001".to_string(),
                vendor_code: "V001".to_string(),
                company_name: "บริษัท ก".to_string(),
                company_keyword: "ABC".to_string(),
                total_cost: 10000.0,
                receive_date: NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(),
                category: "ยา".to_string(),
            },
            InvoiceRow {
                invoice_no: "INV002".to_string(),
                vendor_code: "V002".to_string(),
                company_name: "บริษัท ข".to_string(),
                company_keyword: "DEF".to_string(),
                total_cost: 20000.0,
                receive_date: NaiveDate::from_ymd_opt(2026, 1, 7).unwrap(),
                category: "ยา".to_string(),
            },
            InvoiceRow {
                invoice_no: "INV003".to_string(),
                vendor_code: "PS001".to_string(),
                company_name: "พลาสติก สโตร์".to_string(),
                company_keyword: "PS001".to_string(),
                total_cost: 5000.0,
                receive_date: NaiveDate::from_ymd_opt(2026, 1, 9).unwrap(),
                category: "วัสดุเภสัชกรรม".to_string(),
            },
        ]
    }

    fn sample_submission_params() -> InvoiceSubmissionParams {
        InvoiceSubmissionParams {
            db_config: DbConfig {
                host: "localhost".into(),
                port: 1433,
                database: "test".into(),
                username: "sa".into(),
                password: "pass".into(),
            },
            date_from: "20260101".into(),
            date_to: "20260110".into(),
            year: 2569,
            month: 1,
            round: 1,
            start_reg_no: "69ภ12".into(),
            start_running: 3,
            output_dir: "/tmp".into(),
        }
    }

    fn sample_summary_params() -> ReceivingSummaryParams {
        ReceivingSummaryParams {
            db_config: DbConfig {
                host: "localhost".into(),
                port: 1433,
                database: "test".into(),
                username: "sa".into(),
                password: "pass".into(),
            },
            date_from: "20260101".into(),
            date_to: "20260110".into(),
            year: 2569,
            month: 1,
            round: 1,
            start_po_no: 253,
            start_purchase_no: 253,
            start_reg_no: "69ภ12".into(),
            start_running: 3,
            approval_date: Some("15 ม.ค. 69".into()),
            output_dir: "/tmp".into(),
        }
    }

    fn sample_cover_params() -> CoverLettersParams {
        CoverLettersParams {
            db_config: DbConfig {
                host: "localhost".into(),
                port: 1433,
                database: "test".into(),
                username: "sa".into(),
                password: "pass".into(),
            },
            date_from: "20260101".into(),
            date_to: "20260110".into(),
            year: 2569,
            month: 1,
            round: 1,
            budget_total: 5_843_812.60,
            previous_balance: 1_000_000.0,
            approval_date: Some("15 ม.ค. 69".into()),
            output_dir: "/tmp".into(),
        }
    }

    #[test]
    fn test_parse_reg_no() {
        let (prefix, num) = parse_reg_no("69ภ12");
        assert_eq!(prefix, "69ภ");
        assert_eq!(num, 12);
    }

    #[test]
    fn test_format_reg_no() {
        assert_eq!(format_reg_no("69ภ", 12), "69ภ12");
        assert_eq!(format_reg_no("69ภ", 13), "69ภ13");
    }

    #[test]
    fn test_compute_reg_for_item() {
        // Starting at position 3 in register 12
        assert_eq!(compute_reg_for_item(0, 3, 12), (12, 3));
        assert_eq!(compute_reg_for_item(6, 3, 12), (12, 9));
        // Overflow into next register
        assert_eq!(compute_reg_for_item(7, 3, 12), (13, 0));
    }

    #[test]
    fn test_format_thai_date() {
        let d = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        assert_eq!(format_thai_date(&d), "5 ม.ค. 69");

        let d2 = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        assert_eq!(format_thai_date(&d2), "31 ธ.ค. 69");
    }

    #[test]
    fn test_invoice_submission() {
        let invoices = sample_invoices();
        let params = sample_submission_params();
        let rows = process_invoice_submission(&invoices, &params);

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].seq, 1);
        assert_eq!(rows[0].reg_no, "69ภ12");
        assert_eq!(rows[0].running_in_reg, 3);
        assert_eq!(rows[1].seq, 2);
        assert_eq!(rows[1].running_in_reg, 4);
        assert_eq!(rows[2].seq, 3);
        assert_eq!(rows[2].running_in_reg, 5);
    }

    #[test]
    fn test_receiving_summary_counters() {
        let invoices = sample_invoices();
        let params = sample_summary_params();
        let rows = process_receiving_summary(&invoices, &params);

        assert_eq!(rows.len(), 3);
        // request_no: 253, 255, 257
        assert_eq!(rows[0].request_no, 253);
        assert_eq!(rows[1].request_no, 255);
        assert_eq!(rows[2].request_no, 257);
        // report_no: 254, 256, 258
        assert_eq!(rows[0].report_no, 254);
        assert_eq!(rows[1].report_no, 256);
        assert_eq!(rows[2].report_no, 258);
        // po_no: 253, 254, 255
        assert_eq!(rows[0].po_no, 253);
        assert_eq!(rows[1].po_no, 254);
        assert_eq!(rows[2].po_no, 255);
    }

    #[test]
    fn test_cover_letters_running_balance() {
        let invoices = sample_invoices();
        let params = sample_cover_params();
        let pages = process_cover_letters(&invoices, &params);

        assert_eq!(pages.len(), 3);

        // Page 1: balance 1,000,000 - 10,000 = 990,000
        assert!((pages[0].previous_balance - 1_000_000.0).abs() < 0.01);
        assert!((pages[0].current_amount - 10_000.0).abs() < 0.01);
        assert!((pages[0].remaining_balance - 990_000.0).abs() < 0.01);

        // Page 2: balance 990,000 - 20,000 = 970,000
        assert!((pages[1].previous_balance - 990_000.0).abs() < 0.01);
        assert!((pages[1].current_amount - 20_000.0).abs() < 0.01);
        assert!((pages[1].remaining_balance - 970_000.0).abs() < 0.01);

        // Page 3: balance 970,000 - 5,000 = 965,000
        assert!((pages[2].previous_balance - 970_000.0).abs() < 0.01);
        assert!((pages[2].current_amount - 5_000.0).abs() < 0.01);
        assert!((pages[2].remaining_balance - 965_000.0).abs() < 0.01);
    }

    #[test]
    fn test_cover_letters_cumulative_spent() {
        let invoices = sample_invoices();
        let params = sample_cover_params();
        let pages = process_cover_letters(&invoices, &params);

        let initial_spent = params.budget_total - params.previous_balance;
        // 5,843,812.60 - 1,000,000 = 4,843,812.60
        assert!((pages[0].previous_spent - initial_spent).abs() < 0.01);
        assert!((pages[1].previous_spent - (initial_spent + 10_000.0)).abs() < 0.01);
        assert!((pages[2].previous_spent - (initial_spent + 30_000.0)).abs() < 0.01);
    }

    #[test]
    fn test_cover_letters_fiscal_year() {
        let invoices = sample_invoices();
        let params = sample_cover_params();
        let pages = process_cover_letters(&invoices, &params);
        // year = 2569 (Buddhist, passed directly)
        assert_eq!(pages[0].fiscal_year, "2569");
    }

    #[test]
    fn test_register_overflow() {
        let mut invoices = Vec::new();
        for i in 0..5 {
            invoices.push(InvoiceRow {
                invoice_no: format!("INV{:03}", i + 1),
                vendor_code: "V001".to_string(),
                company_name: "Test Co".to_string(),
                company_keyword: "TC".to_string(),
                total_cost: 1000.0,
                receive_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                category: "ยา".to_string(),
            });
        }

        let mut params = sample_submission_params();
        params.start_running = 8;
        params.start_reg_no = "69ภ12".to_string();

        let rows = process_invoice_submission(&invoices, &params);
        // Item 0: reg 12 pos 8
        assert_eq!(rows[0].reg_no, "69ภ12");
        assert_eq!(rows[0].running_in_reg, 8);
        // Item 1: reg 12 pos 9
        assert_eq!(rows[1].reg_no, "69ภ12");
        assert_eq!(rows[1].running_in_reg, 9);
        // Item 2: reg 13 pos 0 (overflow!)
        assert_eq!(rows[2].reg_no, "69ภ13");
        assert_eq!(rows[2].running_in_reg, 0);
        // Item 3: reg 13 pos 1
        assert_eq!(rows[3].reg_no, "69ภ13");
        assert_eq!(rows[3].running_in_reg, 1);
        // Item 4: reg 13 pos 2
        assert_eq!(rows[4].reg_no, "69ภ13");
        assert_eq!(rows[4].running_in_reg, 2);
    }

    #[test]
    fn test_round2_po_numbering() {
        let invoices = sample_invoices();
        let mut params = sample_summary_params();
        params.round = 2;
        params.start_po_no = 256;
        params.start_purchase_no = 256;
        params.start_running = 3;

        let rows = process_receiving_summary(&invoices, &params);
        assert_eq!(rows[0].po_no, 256);
        assert_eq!(rows[0].request_no, 256);
        assert_eq!(rows[0].report_no, 257);
        assert_eq!(rows[1].po_no, 257);
        assert_eq!(rows[2].po_no, 258);
    }

    #[test]
    fn test_compute_next_reg_no_overflow() {
        // 3 items starting at pos 3 in reg 12 → next is pos 6 in reg 12
        let (next_reg, next_run) = compute_next_reg("69ภ12", 3, 3);
        assert_eq!(next_reg, "69ภ12");
        assert_eq!(next_run, 6);
    }

    #[test]
    fn test_compute_next_reg_with_overflow() {
        // 5 items starting at pos 8 in reg 12 → last at pos 2 in reg 13, next is pos 3
        let (next_reg, next_run) = compute_next_reg("69ภ12", 8, 5);
        assert_eq!(next_reg, "69ภ13");
        assert_eq!(next_run, 3);
    }

    #[test]
    fn test_compute_next_reg_exact_boundary() {
        // 2 items starting at pos 8 → fills pos 8,9 → next is pos 0 in reg 13
        let (next_reg, next_run) = compute_next_reg("69ภ12", 8, 2);
        assert_eq!(next_reg, "69ภ13");
        assert_eq!(next_run, 0);
    }

    // ── next_po_no carry-forward: must advance by n*2 not n ──────────────

    /// After processing n=3 invoices starting at start_po_no=253,
    /// the last request_no used is 253 + (3-1)*2 = 257.
    /// Therefore the next batch must start at 253 + 3*2 = 259.
    #[test]
    fn test_next_po_no_carry_forward_correct() {
        let invoices = sample_invoices(); // 3 invoices
        let params = sample_summary_params(); // start_po_no = 253
        let rows = process_receiving_summary(&invoices, &params);

        assert_eq!(rows.len(), 3);

        // Last request_no in this batch is 257 (253 + 2*2)
        assert_eq!(rows[2].request_no, 257);

        // The next batch must start at 259 (253 + 3*2), NOT 256 (253 + 3)
        let n = rows.len() as u32;
        let expected_next_po_no = params.start_po_no + n * 2;
        assert_eq!(
            expected_next_po_no, 259,
            "next_po_no must be start_po_no + n*2 so the next batch starts after the last used pair"
        );

        // Confirm the wrong value (n instead of n*2) would collide with existing numbers
        let wrong_next_po_no = params.start_po_no + n; // 256 — already used as report_no
        assert_eq!(rows[1].report_no, 256);
        assert_ne!(
            wrong_next_po_no, expected_next_po_no,
            "start_po_no + n would collide with an already-issued report_no"
        );
    }

    /// Round-trip: second batch starts at 259 and its numbers do not overlap
    /// with the first batch's numbers (253–258).
    #[test]
    fn test_next_po_no_no_overlap_between_rounds() {
        let invoices = sample_invoices(); // 3 invoices
        let mut params = sample_summary_params(); // start_po_no = 253
        let n = invoices.len() as u32;
        let next_po_no = params.start_po_no + n * 2; // 259

        // Simulate round 2 starting where round 1 left off
        params.start_po_no = next_po_no;
        params.start_purchase_no = params.start_purchase_no + n; // 256
        params.round = 2;

        let rows2 = process_receiving_summary(&invoices, &params);

        // Round 2 request numbers: 259, 261, 263
        assert_eq!(rows2[0].request_no, 259);
        assert_eq!(rows2[1].request_no, 261);
        assert_eq!(rows2[2].request_no, 263);

        // Round 2 report numbers: 260, 262, 264
        assert_eq!(rows2[0].report_no, 260);
        assert_eq!(rows2[1].report_no, 262);
        assert_eq!(rows2[2].report_no, 264);

        // Round 2 po_no (purchase): 256, 257, 258
        assert_eq!(rows2[0].po_no, 256);
        assert_eq!(rows2[1].po_no, 257);
        assert_eq!(rows2[2].po_no, 258);
    }

    // ── approval_date fallback ────────────────────────────────────────────

    /// When approval_date is None the field should fall back to the first
    /// invoice's receive_date in Thai short format — same as cover letters.
    #[test]
    fn test_approval_date_fallback_to_first_receive_date() {
        let invoices = sample_invoices();
        let mut params = sample_summary_params();
        params.approval_date = None; // explicit None

        let rows = process_receiving_summary(&invoices, &params);

        // First invoice receive_date is 2026-01-05 → Thai "5 ม.ค. 69"
        let expected = "5 ม.ค. 69";
        for row in &rows {
            assert_eq!(
                row.approval_date, expected,
                "all rows should carry the fallback date when approval_date is None"
            );
        }
    }

    /// When approval_date is Some("") (empty string) it should also fall back.
    #[test]
    fn test_approval_date_fallback_when_empty_string() {
        let invoices = sample_invoices();
        let mut params = sample_summary_params();
        params.approval_date = Some(String::new()); // empty string

        let rows = process_receiving_summary(&invoices, &params);

        let expected = "5 ม.ค. 69";
        assert_eq!(
            rows[0].approval_date, expected,
            "empty-string approval_date must fall back to first invoice receive_date"
        );
    }

    /// When approval_date is explicitly provided it must be used as-is.
    #[test]
    fn test_approval_date_explicit_value_used() {
        let invoices = sample_invoices();
        let mut params = sample_summary_params();
        params.approval_date = Some("20 ก.พ. 69".to_string());

        let rows = process_receiving_summary(&invoices, &params);

        for row in &rows {
            assert_eq!(
                row.approval_date, "20 ก.พ. 69",
                "explicitly provided approval_date must appear on every row unchanged"
            );
        }
    }
}
