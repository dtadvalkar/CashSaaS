# CLOSE — Whether the books are fit to forecast from

Status: **approved by the owner** (2026-09-14). Nothing here is built.

What this Family does: checks the parts of an Entity's books that no other Family owns (bank and
card reconciliation, suspense and uncategorized balances, clearing accounts, the period lock,
conversion accounts) and says whether the forecast can rest on them. Subledger agreement
(AR-TIE-01, AP-TIE-01), unapplied payments (AR-UNAPPLIED) and intercompany agreement (IC-AGREE)
live in their own Families (ADR-0018).

Evidence: `docs/research/2026-09-13-close-general-practice.md` (cited `[close: Part n]`), with the
ledger, AP and IC research notes of the same date. Entry format follows ADR-0009, including the
role that acts on each Decision Item (ADR-0020). Every Rule here is Universal (ADR-0017). No Rule
here produces a Placement, so none carries a Confidence level.

**The shape of this Family.** Every check is about the last completed month (design-session
2026-09-13 Q58), and every check is read from ledger data alone. Neither API exposes a bank
statement balance or a reconciliation record `[close: Summary 3]`, so reconciliation is judged
from per-transaction status. A check is blocking, marking the run Provisional, only when the
problem can hide cash, AR or AP movements; the rest are action items (Q92, Q123).

## Decisions behind these Rules

Decided by the owner on 2026-09-14 and recorded in `docs/plans/design-session-2026-09-13.md`.
Q134, Q138 and Q139 were revised from the unattended draft before the owner decided. Q172, Q173
and Q176 were decided on 2026-09-15, and Q177–Q180 on 2026-09-16, from sandbox evidence (`docs/research/2026-09-15-sandbox-read-pass.md`)
and are recorded in `docs/plans/design-session-2026-09-15.md` and `design-session-2026-09-16.md`.
Q222, Q223 and Q225 were decided on 2026-09-16 from the UK pass
(`docs/research/2026-09-16-sandbox-uk-pass.md`), recorded in the same session file.

| # | Question | Decision | Rules |
|---|---|---|---|
| Q134 | The bank reconciliation test | Replaces Q70's "no unreconciled transactions dated on or before month-end", which flags normal outstanding cheques and deposits in transit, with three tests. **Blocking:** an account with transactions in the last completed month has none of that month's transactions reconciled, meaning reconciliation was not done. **Action:** transactions still unreconciled from before that month, in an account whose last month *is* reconciled, meaning items outstanding through a full cycle. **Action:** an account with transactions in the month before last and none in the last completed month, the usual sign the books weren't updated. Amends Q70 and ADR-0018. | CLOSE-BANK-01, 02, 03 |
| Q135 | Credit cards | The same tests as bank accounts. Both ledgers reconcile cards against statements, and Xero types them as bank accounts. | CLOSE-BANK-01, 02 |
| Q136 | Finding suspense and uncategorized accounts | Metadata where it exists, then ADR-0013's default keyword matching on the ledgers' default names (QBO Uncategorized Asset, Income and Expense, Reconciliation Discrepancies; Xero Suspense), then the per-Group mapping. No separate confirmation step. | CLOSE-SUSP-01, 02, CLOSE-DISC-01 |
| Q137 | Which checks are blocking | Two: reconciliation not done (CLOSE-BANK-01), and a non-zero balance-sheet suspense or uncategorized account (CLOSE-SUSP-01). Both can hide cash, AR or AP movements. Everything else is an action item. | all |
| Q138 | A stale clearing balance | No day count. Money in a clearing account at the end of the month before last that still hasn't moved out is a finding, so normal month-end deposits are never flagged by a run early in the month. A negative clearing balance at the last month-end is a finding immediately. The forecast is unchanged (CASH-OPEN-03 still places the run-date balance in week 1); the trade-off is that a lost cheque can sit in week 1 until the bookkeeper acts. | CLOSE-CLEAR-01 |
| Q139 | The lock check | An action item, not blocking, when the lock date is missing or falls before a period that is genuinely closed: the last fiscal year-end, or the end of the last sales tax period whose due date has passed. Many small businesses lock only at those points, so a monthly test would nag them. Amends ADR-0018's list, where the lock was a monthly fitness check. | CLOSE-LOCK-01 |
| Q140 | Changes to closed periods | Defer to M2. Detecting a back-dated edit needs the previous run's view (QBO change data capture, Xero History), and M1 persists no runs. | — |
| Q141 | Conversion accounts | A non-zero Opening Balance Equity or Historical Adjustment at the last month-end is an action item for the accountant. It signals setup left incomplete, not a cash risk. | CLOSE-CONV-01 |
| Q142 | Duplicate bank transactions | Flag only unreconciled exact matches (same account, date, amount and payee). A reconciled transaction is backed by its own statement line. | CLOSE-DUP-01 |
| Q143 | Draft Corrections in CLOSE | None in M1. Each fix needs a destination account, a transaction to edit, or an actual deposit that the finding can't supply, and a guessed account would breach ADR-0019. Decision Items only. | all |
| Q172 | Where the reconciliation test finds its transactions | Two sources. Transactions come from the ledger's documents, which always name the account; reconciliation status is joined on separately. QBO's TransactionList report, its only source of `Clr`, leaves journal entries unattributed to an account, so reading transactions from the report would make a month of journal-entry-only bank activity look empty and silently pass a blocking check. A transaction whose status is unknown counts as not reconciled. | CLOSE-BANK-01 |
| Q173 | Reconciliation status | Three states: reconciled, not reconciled, unknown. Xero's `IsReconciled` is a boolean on the transaction; QBO's report join can be silent. Unknown collapsed into not reconciled would be a false finding; collapsed into reconciled it would disarm a blocking Rule. | CLOSE-BANK-01, 02, CLOSE-DUP-01 |
| Q177 | Joining QBO reconciliation status | **Superseded by Q179.** Decided a TransactionList join on transaction type and id. After a reconciliation in the sandbox, TransactionList still showed reconciled journal entries as not reconciled, so the join would have produced false findings. | — |
| Q179 | Where QBO reconciliation status comes from | The GeneralLedger report, matched on account id, transaction type and transaction id. It returns one row per account line with its own `Clr`, so every line is attributed to its account, including a journal entry between two bank accounts. Q172 is unchanged: which transactions an account has still comes from documents. | CLOSE-BANK-01, 02, CLOSE-DUP-01 |
| Q180 | Keeping the unknown state | Kept, with Q178. With Q179, unknown arises only when a document line has no matching GeneralLedger row, which the sandbox never showed. It costs nothing, and without it a missed match would silently pass the blocking check. | CLOSE-BANK-01, 02, CLOSE-DUP-01 |
| Q178 | Unknown status in the action-item checks | Excluded. CLOSE-BANK-02 and CLOSE-DUP-01 assert a transaction is still unreconciled, which unknown can't support, and counting it would list the same transactions on every run. Only the blocking check (CLOSE-BANK-01) errs toward Provisional. The trade-off: a duplicate whose status is unknown isn't flagged. | CLOSE-BANK-02, CLOSE-DUP-01 |
| Q222 | What counts as reconciled | Only a transaction matched in a completed reconciliation. QBO `R` is reconciled; `C` (cleared, not reconciled), first seen in the UK sandbox, is not reconciled. | CLOSE-BANK-01, 02, CLOSE-DUP-01 |
| Q223 | Reading QBO reconciliation status reliably | The Adapter reads GeneralLedger columns by `ColKey`, not title, and maps transaction-type labels to record types per region ("Cheque" in the UK, "Check" in the US). A missing `is_cleared` column is an Adapter error, never unknown status, or a whole company would read as unknown and block every month. Amends Q179's join key. | CLOSE-BANK-01, 02, CLOSE-DUP-01 |
| Q225 | The tax suspense account | QBO's `GlobalTaxSuspense` account ("VAT Suspense", "GST/HST Suspense") is sales-tax liability, never suspense. It holds a filed return until the tax payment clears it, so a balance there is ordinary; classified by keyword it would mark every run Provisional between filing and payment. | CLOSE-SUSP-01 |
| Q176 | A Xero lock date that isn't set | Amends Q164. The lock date is the later of whichever of `PeriodLockDate` and `EndOfYearLockDate` is set; if neither is, there is no lock date. `EndOfYearLockDate` null with `PeriodLockDate` set is the ordinary case. Xero's `/Date(ms+offset)/` values are parsed by the Adapter, never by the engine. | CLOSE-LOCK-01 |

---

## Bank and card reconciliation

### CLOSE-BANK-01 — Last month's reconciliation has been done (Q134, Q135, Q137, Q172, Q173, Q222)

- **Scope:** Universal · **Status:** active
- **Statement:** For each account classified as bank or credit card, if the account has
  transactions dated in the last completed month and none of those transactions is reconciled
  (matched in a completed reconciliation; QBO `C`, cleared, is not reconciled), the Entity has a Decision Item saying that month has not been reconciled for that account, and
  the run is Provisional. A transaction whose reconciliation status is unknown counts as not
  reconciled. Deleted transactions are ignored. An account the Entity marks as not reconciled
  against statements (petty cash, for example) is skipped.
- **Justification:** Reconciling each month is the core close step `[close: Part 1]`. Without it,
  nothing shows that bank movements are all in the books, so Opening Cash can't be trusted
  (ADR-0018). Unreconciled items at a month-end are often legitimate outstanding cheques and
  deposits in transit `[close: Part 2]`, so the test asks whether reconciliation happened at all,
  not whether every item cleared. Reconciliation status is per transaction in both ledgers
  `[close: Part 2]`. Which transactions an account has is taken from documents, not from the
  reconciliation source: QBO's TransactionList report omits the account on journal entries, so a
  month of journal-entry-only activity would otherwise look empty and pass (Q172). QBO status comes
  from the GeneralLedger report, which attributes every line to its account (Q179), so unknown
  means only a missed match (Q180); it errs toward Provisional because this Rule is the one that
  can block. Report labels are localised, so the join maps them per region (Q223). A transaction
  marked cleared has been ticked but not reconciled against a statement, which is what this Rule
  asks about (Q222). The account's transactions include tax payments, which QBO keeps as their
  own record type outside the US (Q224).
- **Reads:** Canonical Facts: bank and card transactions with reconciliation status (reconciled,
  not reconciled or unknown; Q173, Q222). Classifications: bank, credit card. Settings: accounts not
  reconciled against statements (optional, no default).
- **Produces:** Decision Item kind "reconcile last month", subject the account and month.
- **Acted on by:** bookkeeper.
- **Marks Provisional:** yes.

### CLOSE-BANK-02 — Items outstanding through a full cycle are flagged (Q134, Q135, Q178, Q222)

- **Scope:** Universal · **Status:** active
- **Statement:** In a bank or card account whose last completed month has reconciled
  transactions, transactions dated on or before the end of the month before that and still
  unreconciled raise one Decision Item listing them. Transactions whose reconciliation status is
  unknown are left out.
- **Justification:** An item still unreconciled after a later period has been reconciled is a
  stale cheque, a duplicate or an error `[close: Part 2]`. Anchoring on reconciled periods rather
  than a day count keeps the test free of thresholds (ADR-0019).
- **Reads:** Canonical Facts: bank and card transactions with reconciliation status (reconciled,
  not reconciled or unknown; Q173, Q222). Classifications: bank, credit card.
- **Produces:** Decision Item kind "stale outstanding items", subject the account.
- **Acted on by:** bookkeeper.
- **Marks Provisional:** no.

### CLOSE-BANK-03 — An account with nothing recorded last month is flagged (Q134)

- **Scope:** Universal · **Status:** active
- **Statement:** A bank or card account with transactions dated in the month before last, and
  none dated in the last completed month, raises a Decision Item asking whether the account has
  gone quiet or the books are behind. Accounts the Entity marks as not reconciled against
  statements are skipped.
- **Justification:** Books not kept up to date are the plainest reason cash data can't be
  trusted, and neither API shows the bank's own activity to compare against `[close: Part 2]`.
  A quiet account is legitimate, so this is an action item, not a block. Anchoring on the month
  before last, rather than a count of idle days, keeps it free of thresholds (ADR-0019) and
  stops long-closed accounts from being flagged forever.
- **Reads:** Canonical Facts: bank and card transactions. Classifications: bank, credit card.
  Settings: accounts not reconciled against statements (optional, no default).
- **Produces:** Decision Item kind "no activity recorded", subject the account and month.
- **Acted on by:** bookkeeper.
- **Marks Provisional:** no.

### CLOSE-DUP-01 — Unreconciled exact duplicates are flagged (Q142, Q178, Q222)

- **Scope:** Universal · **Status:** active
- **Statement:** Two unreconciled, non-deleted transactions in the same bank or card account with
  the same date, amount and payee raise a Decision Item to check for a duplicate. A transaction
  whose reconciliation status is unknown is not a candidate.
- **Justification:** Duplicates understate or overstate book cash. Only vendor material names
  the check, but QBO's matching guidance warns about duplicates `[close: Part 7]`. A reconciled
  transaction is backed by its own statement line, so only unreconciled matches are at risk.
  Exact matches need no threshold.
- **Reads:** Canonical Facts: bank and card transactions with reconciliation status (reconciled,
  not reconciled or unknown; Q173, Q222).
- **Produces:** Decision Item kind "possible duplicate bank transaction", subject both
  transactions.
- **Acted on by:** bookkeeper.
- **Marks Provisional:** no.

---

## Suspense, uncategorized and clearing

### CLOSE-SUSP-01 — Balance-sheet suspense and uncategorized accounts are empty at month-end (Q136, Q137, Q225)

- **Scope:** Universal · **Status:** active
- **Statement:** An account classified as suspense or uncategorized asset with a non-zero balance
  at the end of the last completed month raises a Decision Item to clear it, and the run is
  Provisional. A sales-tax suspense account (QBO `GlobalTaxSuspense`) is sales-tax liability,
  not suspense, and is never read here (Q225).
- **Justification:** These accounts hold movements that still need classifying `[close: Part 3]`.
  On the balance sheet they can hide a customer payment, a vendor payment or a bank movement that
  AR, AP or CASH would otherwise see. They are identified by metadata where it exists, else by
  the ledgers' default names through the default classifier (ADR-0013), else by mapping
  `[close: Part 3]`. QBO's tax suspense account holds a filed return until it is paid, so its
  balance is an ordinary liability; its name contains "Suspense", so provider metadata must
  classify it before keyword matching can (UK pass, check 8).
- **Reads:** Canonical Facts: account balance as of a date. Classifications: suspense,
  uncategorized asset.
- **Produces:** Decision Item kind "clear suspense balance", subject the account and month.
- **Acted on by:** bookkeeper.
- **Marks Provisional:** yes.

### CLOSE-SUSP-02 — Uncategorized income and expense are flagged (Q136, Q137)

- **Scope:** Universal · **Status:** active
- **Statement:** An income or expense account classified as uncategorized with any activity in
  the last completed month raises a Decision Item to categorise it.
- **Justification:** Uncategorized income and expense still need categorising `[close: Part 3]`.
  They don't hide cash, which has already moved through a bank account. They can hide a payroll
  or tax payment that GAP estimates rely on, so they are worth an action item, not a block.
- **Reads:** Canonical Facts: account activity for the month. Classifications: uncategorized
  income, uncategorized expense.
- **Produces:** Decision Item kind "categorise transactions", subject the account and month.
- **Acted on by:** bookkeeper.
- **Marks Provisional:** no.

### CLOSE-DISC-01 — Postings to reconciliation discrepancies are flagged (Q136)

- **Scope:** Universal · **Status:** active
- **Statement:** Any posting in the last completed month to an account classified as
  reconciliation discrepancies raises a Decision Item for the accountant.
- **Justification:** A reconciliation adjustment forces a match without correcting the underlying
  error, and recurring differences should go to an accountant `[close: Part 2]`.
- **Reads:** Canonical Facts: account activity for the month. Classifications: reconciliation
  discrepancies.
- **Produces:** Decision Item kind "review reconciliation adjustment", subject the account and
  month.
- **Acted on by:** accountant.
- **Marks Provisional:** no.

### CLOSE-CLEAR-01 — Money left in clearing through a full month is flagged (Q138)

- **Scope:** Universal · **Status:** active
- **Statement:** For each account classified as clearing, the amount still unmoved is its balance
  at the end of the month before last, less money moved out of it since. If that amount is
  positive, a Decision Item says money that was in clearing more than a month ago has not reached
  a bank. A negative clearing balance at the end of the last completed month raises its own item.
  The forecast is unchanged.
- **Justification:** Clearing accounts clear when the money moves on, not by a deadline, and
  deposits in transit are normal at a cut-off `[close: Part 4]`. Anchoring on the month before
  last gives every item a full month to clear, so a run early in a month never flags that
  month-end's normal deposits, and it needs only a dated balance and later movements, not a day
  count (ADR-0019). A negative balance means more left than arrived, which is an error by
  construction `[close: Part 4]`.
- **Reads:** Canonical Facts: account balance as of a date, movements out of the account since.
  Classifications: clearing.
- **Produces:** Decision Item kind "money not banked" or "negative clearing balance", subject the
  account and month.
- **Acted on by:** bookkeeper.
- **Marks Provisional:** no.

---

## Lock and setup

### CLOSE-LOCK-01 — Closed periods are locked (Q139, Q176)

- **Scope:** Universal · **Status:** active
- **Statement:** The most recent closed period is the later of two dates: the last fiscal
  year-end, from the Entity's tax-year start, and the end of the last sales tax reporting period
  whose remittance due date has passed. When the ledger has no lock date, or its lock date falls
  before that period's end, a Decision Item suggests locking through it. For Xero, the lock date
  is the later of whichever of `PeriodLockDate` and `EndOfYearLockDate` is set, and there is none
  if neither is (Q164, Q176). If neither closed-period date can be
  worked out (no tax-year start and no reporting period set), the Rule does nothing. The run is
  not Provisional.
- **Justification:** Locking stops changes to past transactions `[close: Part 5]`, and lock
  dates are commonly set at year-end or tax-period end `[close: Part 5]`, once a return depends
  on the figures. An unlocked month isn't wrong, only changeable, so testing monthly would nag
  businesses following that practice. QBO exposes `BookCloseDate`; Xero exposes
  `PeriodLockDate` and `EndOfYearLockDate` `[close: Part 5]`. The period dates come from
  Settings GAP already uses (GAP-INCOME-01, GAP-TAX-03) and statutory calendars (ADR-0007).
- **Reads:** Canonical Facts: ledger lock date. Settings: tax-year start, sales tax reporting
  period. Reference Data: sales tax calendars.
- **Produces:** Decision Item kind "lock closed period", subject the Entity and period end.
- **Acted on by:** accountant.
- **Marks Provisional:** no.

### CLOSE-CONV-01 — Conversion accounts are empty (Q141)

- **Scope:** Universal · **Status:** active
- **Statement:** An account classified as conversion (QBO Opening Balance Equity, Xero Historical
  Adjustment) with a non-zero balance at the end of the last completed month raises a Decision
  Item for the accountant.
- **Justification:** Opening balances post to these accounts during setup, and Intuit community
  guidance says Opening Balance Equity should end at zero `[close: Part 6]` (not stated in an
  official article). A balance signals setup left incomplete rather than a cash risk. Both are
  identifiable from metadata: `OpeningBalanceEquity` and `SystemAccount: HISTORICAL`
  `[close: Part 6]`.
- **Reads:** Canonical Facts: account balance as of a date. Classifications: conversion.
- **Produces:** Decision Item kind "clear conversion balance", subject the account and month.
- **Acted on by:** accountant.
- **Marks Provisional:** no.
