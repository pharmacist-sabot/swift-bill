//! Round-continuity validation.
//!
//! Between rounds (รอบ) the hospital must carry the remaining budget balance
//! forward by hand. A typo here propagates through every page of every later
//! report, so this module derives the *expected* `previous_balance` from the
//! last recorded round and flags any mismatch before generation.

use crate::models::RoundHistory;
use serde::{Deserialize, Serialize};

/// Result of [`validate_budget_carry_forward`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetValidation {
  /// The `remaining_balance` recorded by the most recent prior round for the
  /// same fiscal year + month, if any.
  pub expected_previous_balance: Option<f64>,
  /// The `previous_balance` the user entered for the new round.
  pub entered_previous_balance: f64,
  /// `true` when an expected value exists and differs from the entered value
  /// beyond `tolerance`.
  pub mismatch: bool,
  /// Absolute tolerance used for the comparison (baht).
  pub tolerance: f64,
}

/// Return the `remaining_balance` recorded by the most recent prior round for
/// the same fiscal year + month, preferring entries that actually carry a
/// non-zero balance (i.e. a เบิกยาปะหน้า / cover-letter round). Returns `None`
/// when no comparable prior round exists.
#[must_use]
pub fn last_remaining_balance(fiscal_year: i32, month: u32, history: &RoundHistory) -> Option<f64> {
  history
    .entries
    .iter()
    .filter(|e| e.fiscal_year == fiscal_year && e.month == month && e.remaining_balance.abs() > 0.01)
    .max_by_key(|e| (e.round, e.created_at.clone()))
    .map(|e| e.remaining_balance)
}

/// Compare the user-entered `previous_balance` against the value implied by the
/// last recorded round. When no prior round exists the validation is neutral
/// (no expected value, no mismatch) -- first rounds have nothing to check
/// against.
#[must_use]
pub fn validate_budget_carry_forward(
  fiscal_year: i32,
  month: u32,
  entered_previous_balance: f64,
  history: &RoundHistory,
  tolerance: f64,
) -> BudgetValidation {
  let expected = last_remaining_balance(fiscal_year, month, history);
  let mismatch = expected
    .map_or(false, |exp| (exp - entered_previous_balance).abs() > tolerance);
  BudgetValidation {
    expected_previous_balance: expected,
    entered_previous_balance,
    mismatch,
    tolerance,
  }
}

/// Assert that every fetched invoice produced exactly one report row.
///
/// A mismatch means an invoice was silently dropped between the query and the
/// report -- the exact class of error the legacy Excel workflow produced. The
/// generate/preview commands call this as a guard before writing any output.
#[must_use]
pub fn reconcile_row_count(invoice_count: usize, row_count: usize) -> Result<(), String> {
  if invoice_count != row_count {
    return Err(format!(
      "จำนวนแถวไม่ตรงกัน: ดึงข้อมูล {invoice_count} ใบ แต่สร้างรายงานได้ {row_count} แถว (อาจมีบางรายการหายไป)"
    ));
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::RoundHistoryEntry;

  fn entry(round: u32, remaining: f64, created_at: &str) -> RoundHistoryEntry {
    RoundHistoryEntry {
      id: format!("e{round}"),
      label: format!("รอบ {round}"),
      fiscal_year: 2569,
      month: 1,
      round,
      date_from: "20260101".into(),
      date_to: "20260110".into(),
      next_reg_no: "69ภ13".into(),
      next_running: 0,
      next_po_no: 259,
      next_purchase_no: 256,
      remaining_balance: remaining,
      budget_total: 5_000_000.0,
      total_amount: 0.0,
      invoice_count: 3,
      created_at: created_at.into(),
      source_tab: "cover".into(),
    }
  }

  #[test]
  fn no_prior_round_is_neutral() {
    let history = RoundHistory { entries: vec![] };
    let v = validate_budget_carry_forward(2569, 1, 1_000_000.0, &history, 0.01);
    assert!(v.expected_previous_balance.is_none());
    assert!(!v.mismatch);
  }

  #[test]
  fn matching_balance_is_ok() {
    let history = RoundHistory {
      entries: vec![entry(1, 3_850_000.0, "2026-01-10T00:00:00Z")],
    };
    let v = validate_budget_carry_forward(2569, 1, 3_850_000.0, &history, 0.01);
    assert_eq!(v.expected_previous_balance, Some(3_850_000.0));
    assert!(!v.mismatch);
  }

  #[test]
  fn mismatch_is_flagged() {
    let history = RoundHistory {
      entries: vec![entry(1, 3_850_000.0, "2026-01-10T00:00:00Z")],
    };
    let v = validate_budget_carry_forward(2569, 1, 3_800_000.0, &history, 0.01);
    assert_eq!(v.expected_previous_balance, Some(3_850_000.0));
    assert!(v.mismatch);
  }

  #[test]
  fn within_tolerance_is_not_a_mismatch() {
    let history = RoundHistory {
      entries: vec![entry(1, 3_850_000.0, "2026-01-10T00:00:00Z")],
    };
    let v = validate_budget_carry_forward(2569, 1, 3_850_000.5, &history, 1.0);
    assert!(!v.mismatch);
  }

  #[test]
  fn ignores_other_fiscal_year_and_month() {
    let mut e = entry(1, 3_850_000.0, "2026-01-10T00:00:00Z");
    e.fiscal_year = 2568;
    let history = RoundHistory { entries: vec![e] };
    let v = validate_budget_carry_forward(2569, 1, 0.0, &history, 0.01);
    assert!(v.expected_previous_balance.is_none());
  }

  #[test]
  fn picks_latest_round_balance() {
    let history = RoundHistory {
      entries: vec![
        entry(1, 3_850_000.0, "2026-01-10T00:00:00Z"),
        entry(2, 3_724_569.5, "2026-01-20T00:00:00Z"),
      ],
    };
    let v = validate_budget_carry_forward(2569, 1, 3_724_569.5, &history, 0.01);
    assert_eq!(v.expected_previous_balance, Some(3_724_569.5));
    assert!(!v.mismatch);
  }

  #[test]
  fn ignores_zero_balance_entries() {
    // A receiving-summary round records remaining_balance = 0; it must not be
    // used as the expected carry-forward.
    let mut zero = entry(1, 0.0, "2026-01-10T00:00:00Z");
    zero.source_tab = "summary".into();
    let mut real = entry(2, 3_850_000.0, "2026-01-20T00:00:00Z");
    real.round = 1;
    real.created_at = "2026-01-05T00:00:00Z".into();
    let history = RoundHistory {
      entries: vec![zero, real],
    };
    let v = validate_budget_carry_forward(2569, 1, 3_850_000.0, &history, 0.01);
    assert_eq!(v.expected_previous_balance, Some(3_850_000.0));
  }

  #[test]
  fn reconcile_passes_when_counts_match() {
    assert!(reconcile_row_count(3, 3).is_ok());
  }

  #[test]
  fn reconcile_fails_when_an_invoice_is_dropped() {
    let err = reconcile_row_count(3, 2).unwrap_err();
    assert!(err.contains('3'));
    assert!(err.contains('2'));
  }
}
