//! Receiving-number allocation with lock support.
//!
//! The hospital issues three coupled numbers per invoice:
//! * `request_no` – the "ขอซื้อ" (purchase request) number, increments by 2
//! * `report_no`  – the "รายงาน/อนุมัติ" number, always `request_no + 1`
//! * `purchase_no` – the "ใบสั่งซื้อ" number, increments by 1
//!
//! Some sets of numbers may be manually locked (e.g. numbers used in an
//! earlier round that the user forgot to import). This module skips locked
//! sets when allocating new numbers.
//!
//! The same lock mechanism also protects **register numbers**
//! (เลขทะเบียนคุม): a lock entry may carry an optional `start_reg_no` +
//! `running` slot, and [`find_register_conflicts`] reports any slot a planned
//! round would collide with before anything is generated.

use crate::models::{
  NumberLockEntry, ReceivingNumberAssignment, ReceivingNumberingInfo, RegisterConflict,
  RegisterNumberingInfo, SkippedLockedNumberSet,
};
use crate::register::{compute_reg_for_item, format_reg_no, parse_reg_no};

/// Result of [`allocate_receiving_numbers`].
pub struct ReceivingNumberAllocation {
  /// One assignment per invoice in the batch.
  pub assignments: Vec<ReceivingNumberAssignment>,
  /// Diagnostic info for the UI (locks that were skipped, etc.).
  pub numbering_info: ReceivingNumberingInfo,
  /// Next request_no to use in a subsequent batch.
  pub next_po_no: u32,
  /// Next purchase_no to use in a subsequent batch.
  pub next_purchase_no: u32,
}

/// Walk `start_po_no` and `start_purchase_no` forward until they point at
/// the first non-locked set. Returns the normalized starting numbers and
/// the set of locked sets that were skipped.
#[must_use]
pub fn normalize_receiving_start_numbers(
  fiscal_year: i32,
  start_po_no: u32,
  start_purchase_no: u32,
  locks: &[NumberLockEntry],
) -> ReceivingNumberingInfo {
  let mut current_po_no = start_po_no;
  let mut current_purchase_no = start_purchase_no;
  let mut skipped_locked_sets = Vec::new();

  while let Some(lock) = find_matching_lock(fiscal_year, current_po_no, current_purchase_no, locks)
  {
    skipped_locked_sets.push(to_skipped_locked_set(lock));
    current_po_no += 2;
    current_purchase_no += 1;
  }

  ReceivingNumberingInfo {
    start_po_no: current_po_no,
    start_purchase_no: current_purchase_no,
    skipped_locked_sets,
  }
}

/// Allocate `count` receiving number assignments, skipping any locked sets.
#[must_use]
pub fn allocate_receiving_numbers(
  fiscal_year: i32,
  start_po_no: u32,
  start_purchase_no: u32,
  count: u32,
  locks: &[NumberLockEntry],
) -> ReceivingNumberAllocation {
  let normalized =
    normalize_receiving_start_numbers(fiscal_year, start_po_no, start_purchase_no, locks);
  let mut current_po_no = normalized.start_po_no;
  let mut current_purchase_no = normalized.start_purchase_no;
  let mut skipped_locked_sets = normalized.skipped_locked_sets;
  let mut assignments = Vec::with_capacity(count as usize);

  while assignments.len() < count as usize {
    if let Some(lock) = find_matching_lock(fiscal_year, current_po_no, current_purchase_no, locks) {
      skipped_locked_sets.push(to_skipped_locked_set(lock));
      current_po_no += 2;
      current_purchase_no += 1;
      continue;
    }

    assignments.push(ReceivingNumberAssignment {
      request_no: current_po_no,
      report_no: current_po_no + 1,
      purchase_no: current_purchase_no,
    });
    current_po_no += 2;
    current_purchase_no += 1;
  }

  ReceivingNumberAllocation {
    assignments,
    numbering_info: ReceivingNumberingInfo {
      start_po_no: normalized.start_po_no,
      start_purchase_no: normalized.start_purchase_no,
      skipped_locked_sets,
    },
    next_po_no: current_po_no,
    next_purchase_no: current_purchase_no,
  }
}

fn find_matching_lock(
  fiscal_year: i32,
  request_no: u32,
  purchase_no: u32,
  locks: &[NumberLockEntry],
) -> Option<&NumberLockEntry> {
  let report_no = request_no + 1;
  locks.iter().find(|lock| {
    lock.fiscal_year == fiscal_year
      && (lock.request_no == request_no
        || lock.report_no == report_no
        || lock.purchase_no == purchase_no)
  })
}

fn to_skipped_locked_set(lock: &NumberLockEntry) -> SkippedLockedNumberSet {
  SkippedLockedNumberSet {
    request_no: lock.request_no,
    report_no: lock.report_no,
    purchase_no: lock.purchase_no,
    reason: lock.reason.clone(),
    note: lock.note.clone(),
  }
}

// ---------------------------------------------------------------------------
// Register-number (เลขทะเบียนคุม) lock support
// ---------------------------------------------------------------------------

/// One register assignment produced for an invoice in a batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterAssignment {
  pub reg_no: String,
  pub running_in_reg: u32,
}

/// Result of [`allocate_register_numbers`].
pub struct RegisterNumberAllocation {
  pub assignments: Vec<RegisterAssignment>,
  pub numbering_info: RegisterNumberingInfo,
  pub next_reg_no: String,
  pub next_running: u32,
}

/// Return the lock entry (if any) whose `(start_reg_no, running)` exactly
/// matches the given register slot in the same fiscal year.
fn find_matching_register_lock<'a>(
  fiscal_year: i32,
  reg_no: &str,
  running: u32,
  locks: &'a [NumberLockEntry],
) -> Option<&'a NumberLockEntry> {
  locks.iter().find(|lock| {
    lock.fiscal_year == fiscal_year
      && lock.start_reg_no.as_deref() == Some(reg_no)
      && lock.running == Some(running)
  })
}

/// Report every register slot a planned batch would collide with.
///
/// The batch plans `count` invoices starting at `(start_reg_no,
/// start_running)`, using the standard 10-slot-per-register layout. Any slot
/// that matches a locked register entry in the same fiscal year is returned.
#[must_use]
pub fn find_register_conflicts(
  fiscal_year: i32,
  start_reg_no: &str,
  start_running: u32,
  count: u32,
  locks: &[NumberLockEntry],
) -> Vec<RegisterConflict> {
  let (prefix, start_num) = parse_reg_no(start_reg_no);
  (0..count)
    .map(|i| {
      let (num, running) = compute_reg_for_item(i, start_running, start_num);
      (format_reg_no(&prefix, num), running)
    })
    .filter_map(|(reg_no, running)| {
      find_matching_register_lock(fiscal_year, &reg_no, running, locks).map(|lock| {
        RegisterConflict {
          reg_no,
          running_in_reg: running,
          reason: lock.reason.clone(),
        }
      })
    })
    .collect()
}

/// Walk `(start_reg_no, start_running)` forward until the first slot that does
/// not collide with a locked register entry. Returns the normalized start plus
/// every skipped conflict.
#[must_use]
pub fn normalize_register_start_numbers(
  fiscal_year: i32,
  start_reg_no: &str,
  start_running: u32,
  locks: &[NumberLockEntry],
) -> RegisterNumberingInfo {
  let (prefix, start_num) = parse_reg_no(start_reg_no);
  let mut current_num = start_num;
  let mut current_running = start_running;
  let mut skipped = Vec::new();

  while find_matching_register_lock(
    fiscal_year,
    &format_reg_no(&prefix, current_num),
    current_running,
    locks,
  )
  .is_some()
  {
    skipped.push(RegisterConflict {
      reg_no: format_reg_no(&prefix, current_num),
      running_in_reg: current_running,
      reason: find_matching_register_lock(
        fiscal_year,
        &format_reg_no(&prefix, current_num),
        current_running,
        locks,
      )
      .unwrap()
      .reason
      .clone(),
    });
    current_running += 1;
    if current_running >= 10 {
      current_running = 0;
      current_num += 1;
    }
  }

  RegisterNumberingInfo {
    start_reg_no: format_reg_no(&prefix, current_num),
    start_running: current_running,
    skipped_conflicts: skipped,
  }
}

/// Allocate `count` register assignments, skipping any locked slot (advancing
/// one position at a time, rolling over to the next register every 10 slots).
#[must_use]
pub fn allocate_register_numbers(
  fiscal_year: i32,
  start_reg_no: &str,
  start_running: u32,
  count: u32,
  locks: &[NumberLockEntry],
) -> RegisterNumberAllocation {
  let normalized =
    normalize_register_start_numbers(fiscal_year, start_reg_no, start_running, locks);
  let (prefix, start_num) = parse_reg_no(&normalized.start_reg_no);
  let mut current_num = start_num;
  let mut current_running = normalized.start_running;
  let mut skipped = normalized.skipped_conflicts;
  let mut assignments = Vec::with_capacity(count as usize);

  while assignments.len() < count as usize {
    let reg_no = format_reg_no(&prefix, current_num);
    if find_matching_register_lock(fiscal_year, &reg_no, current_running, locks).is_some() {
      skipped.push(RegisterConflict {
        reg_no: reg_no.clone(),
        running_in_reg: current_running,
        reason: find_matching_register_lock(fiscal_year, &reg_no, current_running, locks)
          .unwrap()
          .reason
          .clone(),
      });
      current_running += 1;
      if current_running >= 10 {
        current_running = 0;
        current_num += 1;
      }
      continue;
    }

    assignments.push(RegisterAssignment {
      reg_no: reg_no.clone(),
      running_in_reg: current_running,
    });
    current_running += 1;
    if current_running >= 10 {
      current_running = 0;
      current_num += 1;
    }
  }

  let next_reg_no = format_reg_no(&prefix, current_num);
  RegisterNumberAllocation {
    assignments,
    numbering_info: RegisterNumberingInfo {
      start_reg_no: normalized.start_reg_no,
      start_running: normalized.start_running,
      skipped_conflicts: skipped,
    },
    next_reg_no,
    next_running: current_running,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::models::NumberLockEntry;

  fn sample_lock(
    fiscal_year: i32,
    request_no: u32,
    purchase_no: u32,
    reason: &str,
  ) -> NumberLockEntry {
    NumberLockEntry {
      id: format!("{fiscal_year}-{request_no}-{purchase_no}"),
      fiscal_year,
      request_no,
      report_no: request_no + 1,
      purchase_no,
      start_reg_no: None,
      running: None,
      reason: reason.to_string(),
      note: String::new(),
      created_at: "2026-01-01T00:00:00Z".to_string(),
    }
  }

  #[test]
  fn allocate_without_locks_matches_legacy_sequence() {
    let allocation = allocate_receiving_numbers(2569, 253, 253, 3, &[]);
    assert_eq!(allocation.assignments.len(), 3);
    assert_eq!(allocation.assignments[0].request_no, 253);
    assert_eq!(allocation.assignments[1].request_no, 255);
    assert_eq!(allocation.assignments[2].request_no, 257);
    assert_eq!(allocation.assignments[0].purchase_no, 253);
    assert_eq!(allocation.assignments[2].purchase_no, 255);
    assert_eq!(allocation.next_po_no, 259);
    assert_eq!(allocation.next_purchase_no, 256);
    assert!(allocation.numbering_info.skipped_locked_sets.is_empty());
  }

  #[test]
  fn normalize_skips_locked_start_set() {
    let locks = vec![sample_lock(2569, 253, 253, "ใช้ไปแล้ว")];
    let normalized = normalize_receiving_start_numbers(2569, 253, 253, &locks);
    assert_eq!(normalized.start_po_no, 255);
    assert_eq!(normalized.start_purchase_no, 254);
    assert_eq!(normalized.skipped_locked_sets.len(), 1);
    assert_eq!(normalized.skipped_locked_sets[0].request_no, 253);
  }

  #[test]
  fn allocate_skips_locked_set_in_middle() {
    let locks = vec![sample_lock(2569, 255, 254, "ล็อกกลางชุด")];
    let allocation = allocate_receiving_numbers(2569, 253, 253, 3, &locks);
    assert_eq!(allocation.assignments[0].request_no, 253);
    assert_eq!(allocation.assignments[1].request_no, 257);
    assert_eq!(allocation.assignments[2].request_no, 259);
    assert_eq!(allocation.assignments[0].purchase_no, 253);
    assert_eq!(allocation.assignments[1].purchase_no, 255);
    assert_eq!(allocation.assignments[2].purchase_no, 256);
    assert_eq!(allocation.next_po_no, 261);
    assert_eq!(allocation.next_purchase_no, 257);
    assert_eq!(allocation.numbering_info.skipped_locked_sets.len(), 1);
    assert_eq!(
      allocation.numbering_info.skipped_locked_sets[0].purchase_no,
      254
    );
  }

  #[test]
  fn normalize_ignores_other_fiscal_years() {
    let locks = vec![sample_lock(2568, 253, 253, "ปีก่อน")];
    let normalized = normalize_receiving_start_numbers(2569, 253, 253, &locks);
    assert_eq!(normalized.start_po_no, 253);
    assert_eq!(normalized.start_purchase_no, 253);
    assert!(normalized.skipped_locked_sets.is_empty());
  }

  #[test]
  fn carry_forward_advances_past_skipped_sets() {
    let locks = vec![
      sample_lock(2569, 255, 254, "ชุดที่ 2"),
      sample_lock(2569, 259, 256, "ชุดที่ 4"),
    ];
    let allocation = allocate_receiving_numbers(2569, 253, 253, 3, &locks);
    assert_eq!(allocation.assignments[0].request_no, 253);
    assert_eq!(allocation.assignments[1].request_no, 257);
    assert_eq!(allocation.assignments[2].request_no, 261);
    assert_eq!(allocation.next_po_no, 263);
    assert_eq!(allocation.next_purchase_no, 258);
    assert_eq!(allocation.numbering_info.skipped_locked_sets.len(), 2);
  }

  // --- Register-number (เลขทะเบียนคุม) lock tests ---

  fn sample_register_lock(
    fiscal_year: i32,
    reg_no: &str,
    running: u32,
    reason: &str,
  ) -> NumberLockEntry {
    NumberLockEntry {
      id: format!("{fiscal_year}-{reg_no}-{running}"),
      fiscal_year,
      request_no: 0,
      report_no: 1,
      purchase_no: 0,
      start_reg_no: Some(reg_no.to_string()),
      running: Some(running),
      reason: reason.to_string(),
      note: String::new(),
      created_at: "2026-01-01T00:00:00Z".to_string(),
    }
  }

  #[test]
  fn no_register_conflicts_without_locks() {
    let conflicts = find_register_conflicts(2569, "69ภ12", 3, 5, &[]);
    assert!(conflicts.is_empty());
  }

  #[test]
  fn detects_register_conflict_in_batch() {
    // Batch of 3 starting at pos 3 in reg 12 occupies slots 3,4,5.
    let locks = vec![sample_register_lock(2569, "69ภ12", 4, "ใช้ไปแล้ว")];
    let conflicts = find_register_conflicts(2569, "69ภ12", 3, 3, &locks);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].reg_no, "69ภ12");
    assert_eq!(conflicts[0].running_in_reg, 4);
    assert_eq!(conflicts[0].reason, "ใช้ไปแล้ว");
  }

  #[test]
  fn register_conflict_rolls_into_next_register() {
    // Slot 9 in reg 12 and slot 0 in reg 13 are both locked.
    let locks = vec![
      sample_register_lock(2569, "69ภ12", 9, "ช่อง 9"),
      sample_register_lock(2569, "69ภ13", 0, "ช่อง 0"),
    ];
    // Batch of 4 from pos 8: slots 8,9,(13,0),1 → two conflicts.
    let conflicts = find_register_conflicts(2569, "69ภ12", 8, 4, &locks);
    assert_eq!(conflicts.len(), 2);
    assert_eq!(conflicts[0].reg_no, "69ภ12");
    assert_eq!(conflicts[0].running_in_reg, 9);
    assert_eq!(conflicts[1].reg_no, "69ภ13");
    assert_eq!(conflicts[1].running_in_reg, 0);
  }

  #[test]
  fn normalize_register_start_skips_locked_slot() {
    let locks = vec![sample_register_lock(2569, "69ภ12", 3, "ล็อก")];
    let info = normalize_register_start_numbers(2569, "69ภ12", 3, &locks);
    assert_eq!(info.start_reg_no, "69ภ12");
    assert_eq!(info.start_running, 4);
    assert_eq!(info.skipped_conflicts.len(), 1);
  }

  #[test]
  fn allocate_register_skips_conflicts_and_rolls_over() {
    let locks = vec![
      sample_register_lock(2569, "69ภ12", 9, "ช่อง 9"),
      sample_register_lock(2569, "69ภ13", 0, "ช่อง 0"),
    ];
    // 4 assignments from pos 8 should skip 9 and (13,0), yielding 8,(13,1),(13,2),(13,3).
    let alloc = allocate_register_numbers(2569, "69ภ12", 8, 4, &locks);
    assert_eq!(alloc.assignments.len(), 4);
    assert_eq!(
      alloc.assignments[0],
      RegisterAssignment {
        reg_no: "69ภ12".into(),
        running_in_reg: 8
      }
    );
    assert_eq!(
      alloc.assignments[1],
      RegisterAssignment {
        reg_no: "69ภ13".into(),
        running_in_reg: 1
      }
    );
    assert_eq!(
      alloc.assignments[2],
      RegisterAssignment {
        reg_no: "69ภ13".into(),
        running_in_reg: 2
      }
    );
    assert_eq!(
      alloc.assignments[3],
      RegisterAssignment {
        reg_no: "69ภ13".into(),
        running_in_reg: 3
      }
    );
    assert_eq!(alloc.next_reg_no, "69ภ13");
    assert_eq!(alloc.next_running, 4);
  }

  #[test]
  fn register_conflicts_ignore_other_fiscal_year() {
    let locks = vec![sample_register_lock(2568, "69ภ12", 4, "ปีก่อน")];
    let conflicts = find_register_conflicts(2569, "69ภ12", 3, 3, &locks);
    assert!(conflicts.is_empty());
  }
}
