# System Architecture & Agent Instructions — Swift Bill v0.3.2

## 1. Project Overview

Desktop application for **Sabot Hospital (โรงพยาบาลสระโบสถ์)** to automate generation of three pharmaceutical disbursement reports, replacing an error-prone manual Excel workflow.

The app connects directly to the hospital's legacy **INVS** SQL Server database (read-only), processes business logic in Rust, and outputs **PDF** files ready for printing.

---

## 2. Tech Stack

| Layer           | Technology                                        |
| --------------- | ------------------------------------------------- |
| Frontend        | Vue 3 (Composition API) + TypeScript + Vite       |
| Desktop shell   | Tauri 2 (Rust)                                    |
| Database driver | `tiberius` v0.12 — direct TDS, **no ODBC needed** |
| PDF generation  | `printpdf` v0.9 (Op-stream API)                   |
| Thai font       | CordiaNew TrueType, embedded via `include_bytes!` |

---

## 3. Round / Batch System (รอบ)

Within a single month/งวด (half), the user may process invoices in **multiple rounds** (รอบ).  
Example: Round 1 → 10 bills processed. Round 2 → 20 more bills added later.

Key invariants across rounds:

- **เลขทะเบียนคุม** (register numbers) continue sequentially from where the previous round ended.  
  The user supplies `start_reg_no` (e.g. `69ภ12`) and `start_running` (position 0–9 within the register).
- **เลขขอซื้อ / PO numbers** continue from the last value. The user supplies `start_po_no`.
- **ยอดงบประมาณคงเหลือ** (remaining budget) carries over. The user supplies `previous_balance`  
  (= the `remaining_balance` of the last page from the previous round).
- **รอบ** (`round: u32`) is stored in `GenerateParams` and appears in all PDF titles and filenames.

---

## 4. Output Files

### File 1 — ส่งหนี้เบิกยา (Invoice Submission List)

- **Layout:** A4 Landscape, single PDF, all rows on one sheet
- **Filename:** `ส่งหนี้เบิกยา_{year}_เดือน{month}_รอบ{round}.pdf`
- **Columns:** ลำดับ | วันที่รับของ | เลขที่เอกสาร | เลขทะเบียนคุม | ลำดับ | วัน/เดือน/ปีใบส่งของ | รหัสบริษัท | ค่าใช้จ่ายเรื่อง | จำนวนเงินรวม

### File 2 — สรุปรับยา (Receiving Summary)

- **Layout:** A4 Landscape, single PDF
- **Filename:** `สรุปรับยา_{year}_เดือน{month}_รอบ{round}.pdf`
- **Columns:** วันที่ขออนุมัติ | วันที่สั่งซื้อ | วันที่รับของ | รหัสบริษัท | จำนวนเงินรวม | รหัสลงรับยา | เลขทะเบียนคุม | ลำดับ | เลขที่ลงรับ | ขอซื้อ (ลบ0033.302/) | รายงาน/อนุมัติ (ลบ0033.302/) | ใบสั่งซื้อ…/{year}

### File 3 — เบิกยาปะหน้า (Disbursement Cover Letters)

- **Layout:** A4 Portrait, **one separate PDF per invoice**
- **Filename:** `เบิกยาปะหน้า_{year}_เดือน{month}_รอบ{round}_หน้า{NNN}.pdf`
- **Content:** Formal Thai government memo (บันทึกข้อความ) including:
  - Header: ส่วนราชการ, ที่/วันที่, เรื่อง, เรียน
  - Body: ด้วย…, ค่า{category}, ปีงบประมาณ, แผนงาน/ซื้อยาจาก{company}
  - **Budget table** (5 columns): ยอดเงินจัดสรร | ยอดเบิกจ่ายแล้ว | ยอดคงเหลือ | เบิกจ่ายครั้งนี้ | ยอดเงินคงเหลือ
  - Signature blocks: ผู้ขออนุมัติ / ผู้เห็นชอบ / ผู้อนุมัติ (ผอ. รพ.)

**Critical budget calculation** (eliminates the legacy `#REF!` Excel bug):

```
remaining_balance[i] = previous_balance[i] - current_amount[i]
previous_balance[i+1] = remaining_balance[i]
cumulative_spent[i+1] = cumulative_spent[i] + current_amount[i]
```

All values are static numbers in the PDF — no formulas.

---

## 5. Source File Map

```
app/src-tauri/src/
├── lib.rs        — Tauri commands: test_connection, preview_data, generate_reports
├── db.rs         — tiberius TCP connection helper
├── queries.rs    — fetch_invoices() SQL query against MS_IVO JOIN COMPANY
├── models.rs     — all Rust structs (GenerateParams has `round: u32`)
├── reports.rs    — business logic: process_invoice_submission, process_receiving_summary, process_cover_letters
├── pdf.rs        — PDF generation via printpdf 0.9 Op-stream API
├── THSarabun.ttf      — embedded Thai regular font (CordiaNew)
└── THSarabunBold.ttf  — embedded Thai bold font (CordiaNew Bold)

app/src/
└── App.vue       — Vue 3 UI: Settings tab | Generate tab | Preview tab
```

---

## 6. Database Schema (INVS — Read Only)

### MS_IVO (Invoices / Receiving)

| Column       | Type          | Notes                     |
| ------------ | ------------- | ------------------------- |
| INVOICE_NO   | String        | Document number           |
| VENDOR_CODE  | String        | FK → COMPANY.COMPANY_CODE |
| TOTAL_COST   | Numeric/Float | Invoice total             |
| RECEIVE_DATE | String        | Format `YYYYMMDD`         |

### COMPANY (Vendor Master)

| Column       | Type   | Notes             |
| ------------ | ------ | ----------------- |
| COMPANY_CODE | String | PK                |
| KEY_WORD     | String |                   |
| COMPANY_NAME | String | Full company name |

---

## 7. Key Implementation Details

### tiberius Numeric Extraction (queries.rs)

SQL Server can return FLOAT, REAL, or DECIMAL for the same column. Always use the multi-type helper:

```rust
fn get_f64(row: &Row, col: &str) -> Option<f64> {
    if let Ok(Some(v)) = row.try_get::<f64, _>(col) { return Some(v); }
    if let Ok(Some(v)) = row.try_get::<f32, _>(col) { return Some(f64::from(v)); }
    if let Ok(Some(v)) = row.try_get::<tiberius::numeric::Numeric, _>(col) {
        if let Ok(f) = format!("{v}").parse::<f64>() { return Some(f); }
    }
    None
}
```

### printpdf 0.9 API Pattern (pdf.rs)

v0.9 uses an **Op-stream** API (not the older layer-reference API):

```rust
let mut doc = PdfDocument::new("title");
let font_id = doc.add_font(&parsed_font);
let mut ops: Vec<Op> = vec![];

// Text
ops.push(Op::StartTextSection);
ops.push(Op::SetFont { font: PdfFontHandle::External(font_id.clone()), size: Pt(10.0) });
ops.push(Op::SetTextCursor { pos: Point { x: mm(20.0), y: mm(277.0) } });
ops.push(Op::ShowText { items: vec![TextItem::Text("สวัสดี".into())] });
ops.push(Op::EndTextSection);

// Save
let page = PdfPage::new(Mm(210.0), Mm(297.0), ops);
doc.with_pages(vec![page]);
let bytes = doc.save(&PdfSaveOptions::default(), &mut vec![]);
fs::write(path, &bytes)?;
```

### Register Number Logic (reports.rs)

- Each register book holds 10 slots (positions 0–9).
- `compute_reg_for_item(item_index, start_running, start_reg_num)`:
  - `absolute_pos = start_running + item_index`
  - `reg_number = start_reg_num + (absolute_pos / 10)`
  - `running_in_reg = absolute_pos % 10`

### PO Number Logic (reports.rs)

- `request_no` starts at `start_po_no`, increments by **+2** per row
- `report_no` starts at `start_po_no + 1`, increments by **+2** per row
- `po_no` starts at `start_po_no`, increments by **+1** per row

### Date Range Computation (lib.rs)

- งวด 1 → days 1–10
- งวด 2 → days 11–20
- งวด 3 → days 21–end-of-month
- Buddhist year → subtract 543 for SQL query

---

## 8. GenerateParams Fields

```rust
pub struct GenerateParams {
    pub db_config: DbConfig,
    pub year: i32,                     // Buddhist year e.g. 2568
    pub month: u32,                    // 1–12
    pub half: u32,                     // งวด: 1, 2, or 3
    pub round: u32,                    // รอบ within the งวด: 1, 2, 3, …
    pub budget_total: f64,             // Total allocated budget for fiscal year
    pub previous_balance: f64,         // Remaining balance BEFORE this round
    pub start_po_no: u32,              // First PO number for this round
    pub start_reg_no: String,          // First register string e.g. "69ภ12"
    pub start_running: u32,            // Starting slot (0–9) within first register
    pub output_dir: String,            // Directory to save PDFs
    pub approval_date: Option<String>, // Thai date string shown on cover letters
}
```

---

## 9. GenerateResult Fields

```rust
pub struct GenerateResult {
    pub files: Vec<String>, // Absolute paths to all generated PDF files
    pub total_rows: usize,
    pub total_amount: f64,
}
```

`files[0]` = ส่งหนี้เบิกยา PDF  
`files[1]` = สรุปรับยา PDF  
`files[2..N]` = เบิกยาปะหน้า PDFs (one per invoice)

---

## 10. Development Rules

1. **Rust for all logic** — Vue only handles UI state and user input.
2. **Type safety** — TypeScript interfaces must exactly match Rust structs.
3. **Error handling** — All Tauri commands return `Result<T, String>`. Errors are shown in the UI.
4. **Read-only DB** — Never write to or modify the INVS database.
5. **Embedded fonts** — Both Thai TTF files are compiled into the binary via `include_bytes!`. No system font dependency.
6. **Round continuity** — The user is responsible for entering the correct `start_po_no`, `start_reg_no`, `start_running`, and `previous_balance` for each new round. The app does not persist state between sessions.
