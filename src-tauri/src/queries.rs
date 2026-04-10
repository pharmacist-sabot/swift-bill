use chrono::NaiveDate;
use tiberius::Row;

use crate::db;
use crate::models::{DbConfig, InvoiceRow};

fn get_f64(row: &Row, col: &str) -> Option<f64> {
  if let Ok(Some(v)) = row.try_get::<f64, _>(col) {
    return Some(v);
  }
  if let Ok(Some(v)) = row.try_get::<f32, _>(col) {
    return Some(f64::from(v));
  }
  if let Ok(Some(v)) = row.try_get::<tiberius::numeric::Numeric, _>(col) {
    let s = format!("{v}");
    if let Ok(f) = s.parse::<f64>() {
      return Some(f);
    }
  }
  None
}

fn get_string(row: &Row, col: &str) -> String {
  row
    .try_get::<&str, _>(col)
    .ok()
    .flatten()
    .unwrap_or("")
    .to_string()
}

fn parse_receive_date(raw: &str) -> Option<NaiveDate> {
  let trimmed = raw.trim();
  if trimmed.len() == 8 {
    NaiveDate::parse_from_str(trimmed, "%Y%m%d").ok()
  } else if trimmed.len() == 10 {
    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").ok()
  } else {
    None
  }
}

fn determine_category(keyword: &str, vendor_code: &str) -> String {
  let kw_upper = keyword.to_uppercase();
  let vc_upper = vendor_code.to_uppercase();
  if kw_upper.starts_with("PS") || vc_upper.starts_with("PS") {
    "วัสดุเภสัชกรรม".to_string()
  } else {
    "ยา".to_string()
  }
}

pub async fn fetch_invoices(
  config: &DbConfig,
  date_from: &str,
  date_to: &str,
) -> Result<Vec<InvoiceRow>, String> {
  let mut client = db::connect(config).await?;

  let query = r#"
        SELECT
            i.INVOICE_NO,
            i.VENDOR_CODE,
            c.COMPANY_NAME,
            c.KEY_WORD,
            i.TOTAL_COST,
            i.RECEIVE_DATE
        FROM MS_IVO i
        LEFT JOIN COMPANY c ON i.VENDOR_CODE = c.COMPANY_CODE
        WHERE i.RECEIVE_DATE >= @P1
          AND i.RECEIVE_DATE <= @P2
        ORDER BY i.PO_NO ASC, i.RECEIVE_DATE, i.VENDOR_CODE
    "#;

  let result = client
    .query(query, &[&date_from, &date_to])
    .await
    .map_err(|e| format!("Query execution failed: {e}"))?;

  let rows = result
    .into_results()
    .await
    .map_err(|e| format!("Failed to fetch results: {e}"))?;

  let mut invoices: Vec<InvoiceRow> = Vec::new();

  for result_set in &rows {
    for row in result_set {
      let invoice_no = get_string(row, "INVOICE_NO");
      let vendor_code = get_string(row, "VENDOR_CODE");
      let company_name = get_string(row, "COMPANY_NAME");
      let company_keyword = get_string(row, "KEY_WORD");
      let total_cost = get_f64(row, "TOTAL_COST").unwrap_or(0.0);
      let receive_date_raw = get_string(row, "RECEIVE_DATE");

      let receive_date = match parse_receive_date(&receive_date_raw) {
        Some(d) => d,
        None => {
          // Skip rows with unparseable dates
          continue;
        }
      };

      let category = determine_category(&company_keyword, &vendor_code);

      invoices.push(InvoiceRow {
        invoice_no,
        vendor_code,
        company_name,
        company_keyword,
        total_cost,
        receive_date,
        category,
      });
    }
  }

  Ok(invoices)
}
