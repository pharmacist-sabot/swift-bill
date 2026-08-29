# Swift Bill

```
 ██████╗██╗    ██╗██╗███████╗████████╗██████╗ ██╗██╗     ██╗
██╔════╝██║    ██║██║██╔════╝╚══██╔══╝██╔══██╗██║██║     ██║
███████╗██║ █╗ ██║██║█████╗     ██║   ██████╔╝██║██║     ██║
╚════██║██║███╗██║██║██╔══╝     ██║   ██╔══██╗██║██║     ██║
██████╔╝╚███╔███╔╝██║██║        ██║   ██████╔╝██║███████╗███████╗
╚═════╝ ╚══╝╚══╝ ╚═╝╚═╝   ╚═╝╚═════╝╚═╝╚══════╝╚══════╝
```

---

## ◆ PULSE

A disbursement report assembled by hand is a report that arrives late,
and a late report is a missing claim. Swift Bill automates the
pharmaceutical disbursement report for the hospital that still lives in
Excel: connect to INVS over native TDS, process the round (รอบ) with
running balances and request numbers locked, and print the A4 PDF with
CordiaNew Thai typography embedded - the same page on every
workstation, every time. The Excel era's arithmetic is done; the
document's continuity is the machine's job now.

| PDF ▣ | Thai fonts ▣ | Rounds ▣ | Encrypted ▣ |
|---|---|---|---|

*P1's continuity foundation and P5's encrypted credentials are sealed;
the CI gate, round wizard, and the rest stand open.*

> Built with Tauri 2 + Vue 3, read from INVS by native TDS, drawn by
> `printpdf` - the report that used to take an afternoon now takes a
> click.
>
> **suradet-ps**, artifact keeper

---

## ◆ IGNITION

One runtime, two commands.

```
⟫ git clone https://github.com/suradet-ps/swift-bill.git
⟫ cd swift-bill
⟫ bun install
⟫ bun run tauri dev
```

The release artifact: `⟫ bun run tauri build` - native executables in
`src-tauri/target/release/bundle`.

<details>
<summary>Prerequisites</summary>

- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Bun](https://bun.sh/) (v1.3+)
- Tauri CLI dependencies for your OS
- An INVS MS SQL Server database reachable over TCP/IP

</details>

---

## ◆ ANATOMY

Four crates, one report, a vault for the credentials.

- **Connects** - `swift-bill-db` speaks to the legacy MS SQL Server
  over native TDS - no ODBC drivers, no middleware, just the protocol
  the database already speaks.
- **Computes** - `swift-bill-core` holds the business logic: rounds,
  running balances, request numbers, and document continuity - the
  arithmetic that Excel used to guess at.
- **Draws** - `swift-bill-pdf` renders print-ready A4 landscape and
  portrait reports through `printpdf`, with CordiaNew embedded so
  Thai text renders identically on any workstation.
- **Exports** - `swift-bill-excel` keeps the spreadsheet output alive
  for the workflows that still need it - the Excel door stays open
  while the PDF door opens.
- **Seals** - connection settings are encrypted at rest with
  `encryptman` and the OS keychain (Phase 5) - the INVS password is
  not a plaintext line in a config file anymore.

---

## ◆ RITUALS

**The core ceremony** - the monthly disbursement report:

1. Open Swift Bill and connect to INVS. One configuration, sealed and
   remembered.
2. Start the round (รอบ). Number locks guard the request numbers;
   the running balance is tracked from the first line.
3. Generate the report - PDF or Excel, landscape or portrait, Thai
   typography embedded.
4. Print, sign, and send. The report is complete because the machine
   carried the continuity, not the memory.

**The ceremony of the lock** - a round's request numbers and balances
are locked by the system, not trusted to the operator. Continuity is
guaranteed in code, where Excel left it to discipline.

**The ceremony of the same page** - CordiaNew ships inside the binary.
Whatever workstation prints the report prints the same letters - the
Thai page cannot quietly change fonts between rooms.

---

## ◆ ECHOES

**Where this artifact is heading**

```
P1 ▸ correctness & continuity: number locks, round state ───────────── ▸ forging
P2 ▸ CI quality gate ───────────────────────────────────────────────── ▸ open
P3 ▸ cover-letter parity, in-app preview ────────────────────────────── ▸ open
P4 ▸ round wizard, งวด UX ───────────────────────────────────────────── ▸ open
P5 ▸ secure settings: encrypted credentials ────────────────────────── ▸ sealed
P6-P9 ▸ frontend tests, reconciliation reports, performance, v1.0 ───── ▸ open
```

**Raising the artifact** - the honest plan lives in `docs/ROADMAP.md`;
the design language in `docs/DESIGN.md`; the contribution rules in
`docs/CONTRIBUTING.md`; the security posture in `docs/security.md`.
Open an issue first to discuss a change.

**Status** - releases build from tags; the CI quality gate is on the
roadmap (P2). [Watch the workflows](.github/workflows).

---

```
  ─────────────────────────────────────────
   An Excel report is a promise
   that someone will not slip.
  ─────────────────────────────────────────
```

Licensed under the [MIT License](LICENSE).