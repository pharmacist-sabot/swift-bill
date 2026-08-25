# Swift Bill Roadmap

This roadmap describes what Swift Bill is, honestly, from reading its own
code -- and where it should end up. It follows the architecture in
[../AGENTS.md](../AGENTS.md), the conventions in
[CONTRIBUTING.md](CONTRIBUTING.md), the safety standards in
[AGENTS-RUST.md](AGENTS-RUST.md), and the design system in
[DESIGN.md](DESIGN.md).

> **What Swift Bill is.** A *quiet, precise* desktop tool for hospital
> pharmaceutical disbursement: one hospital (Sabot Hospital / โรงพยาบาลสระโบสถ์),
> their legacy INVS SQL Server, their invoices, and three statutory
> disbursement reports -- generated correctly, every round, without the
> `#REF!` errors of the old manual Excel workflow. You connect read-only to
> INVS, pick a date range and a processing round (รอบ), and the app renders
> **ส่งหนี้เบิกยา** (invoice submission list), **สรุปรับยา** (receiving summary),
> and **เบิกยาปะหน้า** (disbursement cover letters) as print-ready PDFs --
> with register numbers, request/purchase numbers, and the running budget
> balance carried continuously across rounds.
>
> **What Swift Bill is not.** Not a procurement system, not an ERP, not a
> writer to INVS (the source system is read-only and stays that way), not an
> AI report generator, and not a cloud/SaaS product. It is a specialized
> reporter for one clinical-workflow niche: turning INVS invoice rows into
> accurate, auditable Thai government forms. Features that break that focus
> -- or that cross the line from "generate the form" into "make the financial
> decision" -- are listed under "Out of Scope" so the line is drawn on purpose.

Nothing here is called "done" on intent alone. The repo already builds on
three platforms via Tauri, but the current CI does **not** run the Rust test
suite, Clippy, or `cargo fmt --check` (see "Current State" below). Every
phase's acceptance is checked against a hardened CI gate once Phase 2 lands.

---

## Design Principles

Every feature in Swift Bill should reinforce one or more of these
principles. When a new feature is proposed, ask: "which principle does it
serve, and does it violate any other?"

1. **Numeric accuracy before convenience.** A wrong baht, a wrong register
   number, or a dropped invoice row is an audit failure. If a shortcut
   compromises correctness, it is not a shortcut -- it is a liability. The
   legacy Excel `#REF!` bug must never return in any form.
2. **Deterministic, reproducible output.** Same inputs (invoices + round
   parameters) must always produce the same reports. No timestamps inside
   report content except the user-supplied approval date. No hidden formula
   drift between the preview, the Excel, and the PDF.
3. **Round continuity is first-class.** Register numbers (เลขทะเบียนคุม),
   request/purchase numbers (เลขขอซื้อ/PO), and the remaining budget balance
   carry across rounds (รอบ) without manual re-entry or risk of collision.
   The app exists to remove the human error the manual workflow introduced.
4. **Pure business logic separated from infrastructure.** Report algorithms,
   register math, and number allocation live in `swift-bill-core` -- no I/O,
   no Tauri, no `tiberius`, no `printpdf`. Fully unit-testable, fully
   deterministic.
5. **Read-only respect for the source system.** INVS is never written to.
   Swift Bill is a reporter, not a mutator. A bug in Swift Bill must never
   corrupt hospital data.
6. **Local-first operation.** The app runs on a clinic PC on the hospital
   LAN. It must work without internet. The INVS connection is LAN-only by
   design.
7. **Auditability for every generated round.** Who generated which report,
   for which date range and round, with which starting parameters -- logged
   in round history and reproducible from its stored parameters.

---

## Accuracy Goals

Swift Bill exists to replace an error-prone manual Excel workflow with
output that is correct to the baht and to the register slot.

The software should help the hospital:

- Generate the three statutory reports with totals that reconcile to the
  fetched INVS invoice sum, every time.
- Carry register numbers, PO numbers, and the budget balance continuously
  across rounds without collisions or silent resets.
- Surface, before generation, any parameter conflict (e.g. a starting
  register number that would collide with a locked one).
- Keep the preview, the Excel export, and the PDF visually and numerically
  identical for the same inputs.
- Record every generated round so a past month can be reproduced exactly.

It should **never** silently drop an invoice, silently change a number, or
write to INVS. The app generates the form; the clinician signs it.

---

## Engineering Goals

- Business rules stay inside `swift-bill-core` -- pure Rust, no I/O.
- UI (Vue) contains no financial logic -- it renders what the core computes
  and validates user input only.
- Database layer (`swift-bill-db`) contains no business decisions -- it
  fetches and returns.
- Tauri commands remain thin adapters -- they wire core to IPC and
  persistence, nothing more.
- Every calculation is unit-tested and deterministically reproducible.
- Supply chain is auditable -- `cargo-deny`, pinned Actions,
  `#![deny(unsafe_code)]` at the workspace level.
- CI fails the build on any test failure, Clippy warning, or fmt diff.

---

## Current State (verified against the repo, not assumed)

- **Stack**: Tauri 2 (tauri 2, `tauri-build` 2, `@tauri-apps/api` 2.10.1) +
  Vue 3.5 (Composition API, `<script setup>`) + TypeScript 6 + Vite 8 +
  Bun 1.3.1, Rust 2024 edition backend. Version `0.3.6` in `package.json`
  and `src-tauri/Cargo.toml`. Deployed as a native desktop app (Windows,
  Linux, macOS aarch64) via `tauri-apps/tauri-action`.
- **Workspace layout** (5 members): `swift-bill-core` (pure domain +
  algorithms), `swift-bill-db` (tiberius TDS, read-only INVS),
  `swift-bill-excel` (report 1 & 2 Excel export), `swift-bill-pdf`
  (printpdf 0.12 + lopdf 0.44 overlay), `src-tauri` (thin Tauri shell).
- **Security model**: INVS accessed read-only over native TDS via `tiberius`
  (no ODBC). Thai fonts (CordiaNew / THSarabun) embedded in the binary via
  `include_bytes!` -- no system-font dependency. `#![deny(unsafe_code)]` at
  the workspace level. `tauri-plugin-dialog` / `tauri-plugin-opener` for file
  picking and opening output. Currently no encrypted credential store -- the
  DB password is handled as plain `DbConfig` (see gaps).
- **Data source**: Legacy INVS SQL Server `MS_IVO` (invoices) joined to
  `COMPANY` (vendor master), read-only. Multi-type numeric extraction
  (FLOAT / REAL / DECIMAL via `tiberius::numeric::Numeric`).
- **Core logic** (`crates/swift-bill-core`): pure functions for report
  processing (`process_invoice_submission`, `process_receiving_summary_*`,
  `process_cover_letters`), register-number math (`compute_reg_for_item`,
  `compute_next_reg`), round/งวด date computation, and receiving-number
  allocation with lock awareness (`allocate_receiving_numbers`,
  `normalize_receiving_start_numbers`). Has unit tests.
- **PDF/Excel layer** (`swift-bill-pdf`, `swift-bill-excel`):
  - Report 1 **ส่งหนี้เบิกยา** -- A4 landscape, single PDF (printpdf
    op-stream) **and** Excel export.
  - Report 2 **สรุปรับยา** -- A4 landscape, single PDF (printpdf) **and**
    Excel export.
  - Report 3 **เบิกยาปะหน้า** -- A4 portrait, **one PDF per invoice** via a
    pre-built template + `lopdf` overlay (embedded THSarabun Type0 font).
    PDF only -- no Excel, no preview command.
  - `swift-bill-pdf` has 21 unit tests (template/overlay round-trip, W-array
    widths, budget formatting).
- **Backend** (`src-tauri`, 14 commands): `test_connection`, `fetch_preview`,
  `preview_invoice_submission`, `export_invoice_submission_excel`,
  `preview_receiving_summary`, `export_receiving_summary_excel`,
  `generate_cover_letters`, `load_round_history`, `save_round_entry`,
  `delete_round_entry`, `load_number_locks`, `create_number_locks`,
  `delete_number_lock`, `normalize_receiving_start`. Thin wrappers over core
  + db; persistence (`history.rs`, `number_locks.rs`) uses the Tauri app-data
  dir as JSON.
- **Frontend** (`src/`): tabbed UI -- `TabQuery`, `TabReport1`, `TabReport2`,
  `TabReport3`, `TabSettings`, `TabHistory`, `TabNumberLocks`, plus
  `ToastContainer` / `useToast`. No Pinia (local component state +
  composables). Icons via `lucide-vue-next`; build icons via
  `scripts/gen-icons.cjs` (`@resvg/resvg-js`).
- **Round system**: number locks (`number_locks.json`) cover request /
  report / purchase numbers; round history (`history.rs`) persists each
  generated round with its starting parameters and resulting
  `remaining_balance`. `carry_forward` is computed for the next round but the
  *starting* values (start_reg_no, start_running, start_po_no,
  previous_balance) are still typed in by hand each round.
- **CI** (2 workflows): `test-build.yml` builds the Tauri app on
  `ubuntu-24.04` on push to `main` (paths-gated) -- **it only builds, it does
  not run `cargo test`, `cargo clippy`, or `fmt --check`**. `release.yml`
  builds Windows / Linux / macOS(aarch64) on `v*` tags. Both pin Actions by
  SHA. There is **no Rust test gate and no `cargo-deny`** in CI today.

### Gaps found while reading the repo (these shape the phases below)

1. **No Rust quality gate in CI.** `test-build.yml` calls `bun run tauri
   build` and stops. `cargo test`, `cargo clippy`, and `cargo fmt --check`
   never run in CI, so a regression in `swift-bill-core` math or a broken
   `lopdf`/PDF path would ship silently. For a financial/audit tool this is
   the single most dangerous gap. (Phase 2.)
2. **Report 3 is second-class.** `เบิกยาปะหน้า` has no `preview_*` command and
   no Excel export, while reports 1 & 2 have both. There is no in-app preview
   for any report -- the user generates a PDF and opens it in the OS viewer,
   hoping the numbers are right. (Phase 3.)
3. **Register-number continuity has no lock.** `number_locks` protects
   request / report / purchase numbers, but **เลขทะเบียนคุม** (register
   numbers) are entered by hand each round with no collision check. Two
   rounds can silently reuse the same register slot -- exactly the kind of
   error the app was built to prevent. (Phase 1.)
4. **Budget carry-forward is manual and unchecked.** The user retypes
   `previous_balance` each round; nothing validates it against the
   `remaining_balance` stored in the last round-history entry. A typo here
   propagates through every page of every subsequent report. (Phase 1.)
5. **No งวด date-range helper.** The งวด → day-range mapping (1–10 / 11–20 /
   21–end) lives in `lib.rs` logic but is never surfaced in the UI; the user
   types `date_from` / `date_to` by hand, inviting off-by-a-few-days errors.
   (Phase 4.)
6. **No credential protection.** `DbConfig` carries the INVS password in
   plain text through settings; there is no OS-keychain / encrypted store.
   (Phase 5.)
7. **No frontend tests.** The Vue layer has zero automated tests. A
   regression in a tab's validation or the number-lock UI would go
   unnoticed. (Phase 6.)
8. **No cross-report reconciliation in CI.** Nothing asserts that the PDF/Excel
   row count equals the fetched invoice count, or that preview totals equal
   generated totals, on every build. (Phase 1.)

---

## Phase 1: Correctness & Continuity Guarantees (the safety net)

The things that prevent an *incorrect* report come first. A register-number
collision or a wrong `previous_balance` is an audit failure, not a feature
request. These are the minimum responsible state for a financial tool.

### Register-number lock + carry-forward validation

The continuity values must be protected like the receiving numbers already
are.

- [ ] **Register-number lock.** Extend `number_locks` (and
  `allocate_receiving_numbers` / a new `allocate_register_numbers`) to cover
  เลขทะเบียนคุม: store the last used register string + running slot per
  fiscal year, and refuse (or warn) when a new round's `start_reg_no` /
  `start_running` would collide with a locked range. Pure logic in
  `swift-bill-core::numbering`.
- [ ] **Budget carry-forward validation.** When a round is configured, compare
  the entered `previous_balance` against the `remaining_balance` recorded in
  the last round-history entry for the same fiscal year/month. On mismatch,
  surface a blocking confirmation ("last round ended at 3,850,000.00; you
  entered 3,800,000.00 -- continue?"). Store the validated value.
- [ ] **PO/register next-value preview.** Before generate, show the computed
  next register number, next running slot, next PO number, and ending
  `remaining_balance` (the `carry_forward` struct already exists -- surface
  it in the UI, not just in the result).

### Reconciliation assertions

- [ ] **Row-count reconciliation.** Every generate path asserts
  `total_rows == fetched invoice count`; return an error (not a partial PDF)
  if they differ. Add a core unit test that feeds a known invoice set and
  asserts the report row count and grand total.
- [ ] **Golden-file snapshot tests.** For all three reports, render against a
  fixed invoice fixture + fixed round parameters and assert deterministic
  output (numeric totals, register/PO sequences). PDF content can be asserted
  via the parsed text/layout; this catches silent `#REF!`-style regressions.
- [ ] **Preview == generate parity test.** Assert that `preview_*` totals and
  sequences equal the `export_*` / `generate_*` results for identical inputs
  (closes the gap where report 3 has no preview yet).

**Acceptance:** register numbers cannot collide across rounds; a wrong
`previous_balance` is caught before generation; a generate call with a
missing/dropped invoice fails loudly; golden-file tests exist for all three
reports and pass in CI; preview and generate agree numerically.

**Status:** foundation exists (number_locks for receiving numbers, round
history, `carry_forward`); register lock, budget validation, and
reconciliation assertions are NOT yet implemented.

---

## Phase 2: CI Quality Gate

Today CI only *builds*. A financial tool needs a real gate.

- [ ] **Rust test + lint job.** Add a `ci.yml` (or extend `test-build.yml`)
  that runs, on every PR and push to `main`:
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `cargo build --workspace`
  on `ubuntu-24.04` with pinned Rust stable (SHA-pinned `dtolnay/rust-toolchain`).
- [ ] **Frontend type-check job.** Explicit `vue-tsc --noEmit` (the build
  script already runs it, but make it a visible, fail-on-warning gate).
- [ ] **Supply-chain job.** Add `cargo-deny` (advisory + license) mirroring
  the warfarin-care `rust-safety.yml` posture; pin GitHub Actions by SHA
  (already done in existing workflows).
- [ ] **Make the build fail on regressions.** `test-build.yml` must not mark
  success unless the new test job passes.

**Acceptance:** a PR that breaks a core math test, introduces a Clippy
warning, or drifts `fmt` fails CI; `cargo-deny` blocks a bad/advisory
dependency; Actions are SHA-pinned.

**Status:** NOT started. `test-build.yml` currently builds only.

---

## Phase 3: Cover-Letter Parity & In-App Preview

Report 3 must stop being second-class, and the user must see the numbers
before committing to paper.

- [ ] **`preview_cover_letters` command.** Add a preview command mirroring
  `preview_receiving_summary` (compute pages, return totals + first-page
  fields) so `TabReport3` can show a preview grid.
- [ ] **Cover-letter Excel export.** Add `export_cover_letters_excel` in
  `swift-bill-excel` to match reports 1 & 2 (one row per invoice with the
  same budget table), closing the export-parity gap.
- [ ] **In-app print preview.** Render the generated PDF inside the app
  (embedded viewer or a generated preview image via `resvg`/PDF raster) for
  all three reports, so the user verifies layout before opening the OS
  viewer. Keep the OS "open" action via `tauri-plugin-opener`.
- [ ] **One-click generate-all.** From a single date range + round, generate
  reports 1, 2, and 3 together into `output/`, returning all file paths --
  matching the three-statutory-reports workflow in one action.

**Acceptance:** report 3 has a preview and an Excel export identical in
numbers to its PDF; all three reports preview in-app; generate-all produces
the three expected files with matching totals.

**Status:** NOT started. Reports 1 & 2 already have preview + Excel.

---

## Phase 4: Round Wizard & งวด UX

Make round continuity something the app *manages*, not something the user
memorizes.

- [ ] **งวด date-range helper.** In `TabQuery` / settings, compute
  `date_from` / `date_to` from `year` + `month` + `งวด` (1→1–10, 2→11–20,
  3→21–end) and prefill the inputs; Buddhist→Gregorian year conversion already
  exists in `swift-bill-core::date`.
- [ ] **Round wizard prefill.** On "new round", prefill `start_reg_no`,
  `start_running`, `start_po_no`, and `previous_balance` from the last
  round-history entry for the same fiscal year/month (overridable). This is
  the safe, validated path that Phase 1 makes possible.
- [ ] **Continuity summary card.** Before generate, show a card: starting
  register, computed next register, starting PO, computed next PO, starting
  balance, computed ending balance, and any collision warnings from the locks.
- [ ] **Round-history browser.** `TabHistory` already lists rounds; add
  "re-open parameters" (load a past round's exact `GenerateParams` back into
  the form) and "reproduce" (regenerate from stored params).

**Acceptance:** a new round can be started from the last one in two clicks
with continuity values pre-filled and validated; งวด date ranges are computed,
not typed; the continuity card shows all next-values and any warnings.

**Status:** round history + number locks exist; the wizard, งวด helper, and
prefill are NOT implemented.

---

## Phase 5: Secure Settings & Persistence

- [ ] **Encrypted credential store.** Move `DbConfig` password out of plain
  settings into the OS keychain (or an AES-256-GCM blob keyed by a
  keychain-stored key), mirroring warfarin-care's credential posture. The
  connection test must work without the password ever being written to disk
  in cleartext.
- [ ] **Structured round-history store.** Promote `history.rs`'s JSON to a
  versioned, backed-up store (with export/import) so round parameters survive
  app reinstall and can be audited.
- [ ] **Connection profiles.** Allow saving multiple read-only INVS
  connection profiles (e.g. test vs production) selectable in settings, each
  with its own keychain credential.

**Acceptance:** the INVS password is never stored in cleartext; round history
has export/import and survives reinstall; multiple profiles can be saved.

**Status:** NOT started. Settings currently hold `DbConfig` in plain form.

---

## Phase 6: Frontend Tests & Accessibility

The Vue layer has zero tests and no accessibility pass.

- [ ] **Vitest unit tests for tab logic.** Test the components/composition
  that hold validation or transformation:
  - `TabReport3.vue` -- cover-letter field mapping
  - `TabNumberLocks.vue` -- batch lock create/overlap detection
  - `TabQuery.vue` -- งวด → date-range mapping (once Phase 4 lands)
  - `useToast.ts` -- queue/dismiss behavior
- [ ] **Preview/result parity e2e.** A lightweight Playwright or component
  test that drives query → preview → generate for report 1 and asserts the
  shown total equals the generated file's total.
- [ ] **Keyboard navigation + Thai a11y.** All tab actions completable via
  keyboard; `aria-label` on interactive elements (including Thai labels);
  visible `:focus-visible` ring; minimum 44px touch targets; contrast >= 4.5:1
  for normal text. Log a screen-reader pass in `docs/a11y-notes.md`.

**Acceptance:** Vitest runs in CI and passes; critical preview→generate path
is covered; keyboard-only navigation works across all tabs; ARIA labels
present; contrast passes WCAG AA.

**Status:** NOT started.

---

## Phase 7: Reconciliation & Analytics Reports

Beyond the three statutory forms, the hospital needs to trust and explain the
numbers.

- [ ] **Monthly reconciliation report.** For a fiscal year/month: fetched
  invoice sum vs sum of all three reports' totals; per-round breakdown;
  variance vs budget (`budget_total` - cumulative spent). Export PDF + Excel.
- [ ] **Budget variance report.** Shows allocated / spent / remaining per
  round, cumulative, with the exact `remaining_balance` chain used in cover
  letters -- the audit trail for the budget math.
- [ ] **Variance guard.** Flag any round whose computed `remaining_balance`
  disagrees with the next round's `previous_balance` (catches the manual
  typo gap from Phase 1 even after validation).
- [ ] **Per-vendor summary.** Group invoices by `COMPANY_NAME` for the period
  (already available from `COMPANY` join) -- useful for pharmacy committee
  review.

**Acceptance:** reconciliation report totals tie out to the fetched invoice
sum; budget variance report reproduces the cover-letter balance chain;
variance guard flags discontinuities; all exportable as PDF/Excel.

**Status:** NOT started. Core already computes per-round `remaining_balance`.

---

## Phase 8: Performance & Reliability

A pharmacy clerk will not wait on a slow report when the delivery truck is
at the door.

- [ ] **Baseline measurement.** Document in `docs/perf-baseline.md`: cold start,
  query response for a 1,000+ invoice range, preview latency, PDF generation
  time for a 50-page cover-letter batch, on a mid-range clinic PC.
- [ ] **Large-range handling.** Profile `fetch_invoices` + report processing
  for wide date ranges; stream/limit where needed so a full-month pull does
  not block the UI. The Tauri commands are already `async`; ensure the core
  processing does not allocate pathological intermediates.
- [ ] **INVS failure UX.** A clear Thai error (not a raw TDS stack trace)
  when the LAN DB is unreachable; the app stays usable for previously
  generated outputs.
- [ ] **PDF generation resilience.** If the embedded template or a font fails
  to load, return a actionable Thai error instead of a panic; optionally ship
  a fallback path.

**Acceptance:** baseline document exists; a 1,000-invoice month previews and
generates in seconds; INVS outage shows a clear Thai message; no panic on
asset load failure.

**Status:** NOT started.

---

## Phase 9: Validation against Legacy Excel (v1.0 gate)

Unit tests prove the code does what it says. A side-by-side against the real
legacy Excel proves the *output* matches what the hospital already trusted.

- [ ] **Reconciliation sample.** For a real past month, run Swift Bill and the
  legacy Excel for all three reports; diff register numbers, PO numbers, and
  grand totals. Document agreement (target: 0 discrepancies on totals; any
  register/PO difference root-caused).
- [ ] **Known-differences document.** In `docs/known-limitations.md`, record
  where Swift Bill intentionally differs from the old Excel (e.g. rounding
  rules, zero-fill of register slots) and why.
- [ ] **Sign-off.** Obtain pharmacist/clinic lead confirmation that the three
  reports are accepted for submission. Compile into
  `docs/validation-report.md`.

**Acceptance:** Swift Bill's three reports reconcile to the legacy Excel for a
real month with documented, root-caused differences only; clinical sign-off
obtained; validation report published. This is the gate before `v1.0.0`.

**Status:** NOT started.

---

## How the phases relate

```
Phase 1 (Correctness & Continuity)  -- foundation -- do first
Phase 2 (CI Quality Gate)           -- makes Phase 1's tests actually run
         |
         +---> Phase 3 (Cover-Letter Parity + Preview) -- independent of 2
         +---> Phase 4 (Round Wizard + งวด UX)         -- needs Phase 1 locks/validation
         +---> Phase 5 (Secure Settings)              -- independent
         +---> Phase 6 (Frontend Tests + a11y)        -- parallel, any time
         +---> Phase 7 (Reconciliation & Analytics)   -- needs Phase 1 totals
         +---> Phase 8 (Performance)                  -- needs features to measure
         |
         v
Phase 9 (Validation vs Legacy Excel) -- needs all reports + Phase 2 CI
         |
         v
     v1.0.0
```

Phase 1 comes first on purpose: a register-number collision or a wrong
`previous_balance` is an audit failure, not a feature. Phase 2 comes next
because the safeguards in Phase 1 are worthless if CI never runs them.
Everything after deepens the clinical-workflow correctness that Phases 1–2
make enforceable. Phase 9 is the gate before `v1.0.0`: the reports must
reconcile to the legacy Excel the hospital already trusts.

---

## Out of Scope (drawn on purpose, to stay a focused reporter)

Each of these is valuable *for a different product*. Swift Bill stays focused
on generating the three statutory disbursement reports:

- **Writing to INVS / becoming a mutator** -- INVS is read-only; Swift Bill
  never alters hospital data. This is a hard line.
- **Procurement / ERP system** -- No PO lifecycle, no goods receipt, no
  inventory. The app reads invoices; it does not manage purchasing.
- **AI/LLM report generation** -- Accuracy and auditability risk; hallucinated
  numbers in a financial form are unacceptable. Rule-based logic only.
- **Cloud / SaaS / multi-hospital hosting** -- Swift Bill is a LAN desktop
  app. A hosted version changes the deployment, security, and audit posture.
  Not today.
- **Patient / clinical data** -- Swift Bill processes invoices and vendors,
  not patients. No PHI handling.
- **Mobile app** -- Desktop only; the clinic context demands a printer and a
  large screen.
- **Multi-language (i18n)** -- Thai-only for now; the statutory forms and the
  users demand it.
- **Encrypted PDF / digital signature** -- The printed form is signed by hand
  by the ผอ. รพ. A cryptographic signature is a separate compliance initiative.
- **Automated financial decisioning** -- The app computes and lays out the
  numbers; the clinician/pharmacist verifies and signs. Never remove the
  human from the loop.

## Documentation

Every significant design decision should be documented. The `docs/` directory
should grow with the project:

| Document | Content | When |
|----------|---------|------|
| `../AGENTS.md` | Architecture, schema, modules, commands | Already exists |
| `AGENTS-RUST.md` | Rust workspace rules + project overrides | Already exists |
| `CONTRIBUTING.md` | Developer workflow, code style | Already exists |
| `DESIGN.md` | Design system, tokens, components | Already exists |
| `ROADMAP.md` | This document | Now |
| `architecture.md` | Detailed module dependencies, data flow, INVS query plan | Phase 1 |
| `security.md` | Credential model, keychain plan, read-only posture | Phase 5 |
| `validation-report.md` | Legacy-Excel reconciliation results, sign-off | Phase 9 |
| `known-limitations.md` | Intentional differences from legacy Excel | Phase 9 |
| `a11y-notes.md` | Accessibility audit results, screen-reader log | Phase 6 |
| `perf-baseline.md` | Performance measurements, budgets | Phase 8 |

## Future / Ecosystem (post-1.0, if they stay focused)

- **Template editor** -- let the hospital adjust the cover-letter template
  (positions, wording) without recompiling, while keeping the embedded-font
  guarantee.
- **Batch round automation** -- schedule end-of-งวด generation so the three
  reports are produced automatically from the last validated round.
- **Reconciliation dashboard** -- a standing view of budget vs spent per
  fiscal year, with the variance guard from Phase 7.
- **Multiple hospital profiles** -- the connection-profile groundwork is in
  Phase 5; a multi-site deployment would be a separate config + audit scope.
- **Print-queue integration** -- direct to the hospital's label/PDF printer
  instead of the OS "open" dialog.
