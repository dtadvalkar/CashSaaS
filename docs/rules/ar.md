# AR — Accounts receivable

Status: **approved by the owner** (2026-09-14). Built: every Rule here is implemented in
`core/src/ar.rs` and covered by a Scenario test, with coverage gated against the table below
(`scenarios/tests/coverage.rs`).

What this Family does: turns an Entity's open customer documents into expected receipts in the
Forecast Run, explains what it left out, and raises the AR findings a controller would raise
before trusting the forecast.

Evidence: `docs/research/2026-09-13-ar-general-practice.md` (cited `[ar: Part n]`) and
`docs/research/2026-09-13-ledger-document-model-qbo-xero.md` (cited `[ledger: Part n]`).
Entry format follows ADR-0009, including each Placement's Confidence level and the role that
acts on each Decision Item (ADR-0020). Every Rule here is Universal (ADR-0017).

## Decisions behind these Rules

Decided by the owner on 2026-09-13 and recorded in `docs/plans/design-session-2026-09-13.md`.

| # | Question | Decision | Rules |
|---|---|---|---|
| Q73 | Order of timing evidence | Owner-entered expected date, then the Entity's collection history, then the due date. History may override contractual terms, because this is a cash forecast, not a legal entitlement. | AR-TIME-01, 02 |
| Q74 | Overdue invoices in the forecast | Place at the median collection date from the Entity's own history for invoices at the same age. No fixed day thresholds and no haircut in the base forecast. | AR-TIME-02, 03 |
| Q75 | IFRS 9's 30/90-day presumptions as defaults | No. They stage credit risk under IFRS; they are not cash-timing evidence, and most target Entities do not report under IFRS. | AR-TIME-02 |
| Q76 | How much history, and at what level | Pool all of the Entity's invoice history. Use customer-level history only where the owner marks a customer as behaving differently. Record the observation count on each Placement; no minimum-count Setting in M1. | AR-TIME-02 |
| Q77 | When an invoice leaves the forecast | When more than half of comparable invoices were never collected, or the median date is beyond the horizon. Proposing a write-off follows the first case; deciding one stays the Entity's policy. | AR-TIME-03, AR-WRITEOFF-01 |
| Q78 | Invoices not yet issued | Active scheduled templates (Xero repeating invoices, QBO automated recurring invoices) are in AR. Other future sales are out of M1. | AR-SCHED-01 |
| Q79 | Rate for foreign-currency receipts | The rate booked on the document, recorded on the Placement, until date-aware rates arrive with the full close. | AR-FX-01 |
| Q80 | Early-payment discounts | Assume not taken; place the full open amount. | AR-DISC-01 |
| Q81 | Unapplied credits and overpayments | Net against the same customer's open invoices, oldest first. A customer with credit and no open invoices gets a Decision Item, not a receipt. | AR-UNAPPLIED-01, 02 |
| Q83 | Unbilled revenue in M1 | Two cheap signals only: an unbilled-receivable or contract-asset balance, and a scheduled invoice that did not go out. Other signals are future scenarios (`docs/plans/future-product-scenarios.md`). | AR-SCHED-02, AR-UNBILLED-01 |
| Q144 | Scheduled templates after the write pass | Occurrences are judged by covering invoices (template link, else same customer within the period), not by the template's next date; covered occurrences are not forecast again. Decided 2026-09-14. | AR-SCHED-01, 02 |
| Q230 | Status words | Documents are draft, awaiting approval, posted, voided or deleted (Q220); "authorised" was Xero's word. The Adapter recognises a QBO void, so a Rule never sees QBO's zero document. Decided 2026-09-16. | AR-OPEN-01 |
| Q243 | Open amount in a foreign currency | Derived in the document's currency, then converted at its booked rate, so a paid document has no home residual from applications at other rates. Decided 2026-09-16. | AR-OPEN-01, AR-FX-01 |
| Q245 | Jobs under a customer | A counterparty may have a parent; Rules that group by customer use the top-level parent. Decided 2026-09-16. | AR-TIME-02, AR-UNAPPLIED-01, 02 |
| Q251 | Overpayments and prepayments | Payments with an unapplied remainder, not documents; the remainder is the payment less its applications. Decided 2026-09-16. | AR-UNAPPLIED-01, 02 |
| Q145 | Voided and zero-total documents | Ignored by every AR Rule, including collection history. Decided 2026-09-14. | AR-OPEN-01, AR-TIME-02 |
| Q82 | Form of an AR fix | A Draft Correction: a journal entry, credit note or payment application, in the form the ledger uses. Bad-debt tax relief belongs to GAP. | AR-UNAPPLIED-01, AR-WRITEOFF-01 |

---

## What counts as open AR

### AR-OPEN-01 — A posted customer invoice with an open amount is a receivable

- **Scope:** Universal · **Status:** active
- **Statement:** A customer invoice is a receivable when it is posted (not draft, voided or
  deleted) and its open amount is greater than zero. The open amount is the invoice total less
  payments and credits applied on or before the run date, worked out in the invoice's currency
  and converted to Home Currency at the rate booked on the invoice (Q229, Q243). A
  document with a zero total is ignored by every AR Rule, including collection history (Q145).
  QBO returns a voided invoice as a zero document with no status; the Adapter recognises it as
  voided (Q220, Q230).
- **Justification:** A receivable is an unconditional right to consideration, needing only
  the passage of time `[ar: Part 3, IFRS 15.108]`. Aging is computed from documents and their
  applications, never from aging reports (ADR-0014). Both ledgers expose the open amount and
  the application records `[ledger: Parts 1–2]`.
- **Reads:** Canonical Facts: invoice, payment, credit note, payment application.
- **Produces:** the open amount every other AR Rule works from. Testable through their
  Placements.
- **Marks Provisional:** no.

### AR-OPEN-02 — A draft invoice is not a receivable

- **Scope:** Universal · **Status:** active
- **Statement:** An invoice that has not been issued (a draft, or submitted for approval) is
  not a receivable. It is left out of expected receipts and shown as an exclusion, so an
  owner can see work billed but not yet sent.
- **Justification:** No unconditional right exists until the customer has been invoiced
  `[ar: Part 3]`. Xero marks these `DRAFT` and `SUBMITTED` `[ledger: Part 1]`.
- **Reads:** Canonical Facts: invoice (status).
- **Produces:** exclusion Placement, reason "not issued".
- **Marks Provisional:** no.

### AR-OPEN-03 — An intercompany receivable is not an external receipt

- **Scope:** Universal · **Status:** active
- **Statement:** An invoice to a counterparty classified intercompany is left out of AR's
  expected receipts and shown as an exclusion. IC Rules handle it. A related party outside the
  Group is treated like any other customer.
- **Justification:** Consolidation eliminates intragroup balances and cash flows in full
  `[ar: Part 6, IFRS 10 B86(c)]`, so at Group level this is not external cash. Related
  parties outside the Group are not eliminated `[ar: Part 6]`.
- **Reads:** Classifications: intercompany (counterparty).
- **Produces:** exclusion Placement, reason "intercompany", citing the IC Family.
- **Marks Provisional:** no.

### AR-OPEN-04 — Expected receipts include the sales tax charged

- **Scope:** Universal · **Status:** active
- **Statement:** An expected receipt is the full open amount, including sales tax charged on
  the invoice. Remitting that tax is scheduled by the GAP Family on its statutory calendar.
- **Justification:** The cash a customer pays includes tax collected for the authority
  `[ar: Part 6]`, `[ledger: Part 1]`. Remittance follows statutory calendars (ADR-0007).
- **Reads:** Canonical Facts: invoice (total, tax).
- **Produces:** Placement amount gross of tax.
- **Marks Provisional:** no.

---

## Timing

### AR-TIME-01 — An owner-entered expected date places the invoice (Q73)

- **Scope:** Universal · **Status:** active
- **Statement:** When an open invoice carries an expected payment date entered by a person,
  on or after the run date, the expected receipt is placed in that date's week. If the date
  has passed and the invoice is still open, the date is ignored, AR-TIME-02 applies, and a
  Decision Item asks for the date to be updated. When AR-TIME-04 applies instead, its "set
  expected date" item cites the passed date and this Rule raises no item (Q150).
- **Justification:** A collector's or owner's stated expectation is direct evidence about
  timing. Xero stores it (`ExpectedPaymentDate`) and asks users to set it on overdue invoices
  to improve its own cash projections `[ar: Part 1]`. QBO has no such field `[ledger: Part 1]`.
- **Reads:** Canonical Facts: invoice (expected payment date).
- **Produces:** Placement with basis "expected date", Confidence firm; Decision Item kind
  "expected date passed".
- **Acted on by:** owner.
- **Marks Provisional:** no.

### AR-TIME-02 — Otherwise, the Entity's own collection history places the invoice (Q73–Q76)

- **Scope:** Universal · **Status:** active
- **Statement:** An open invoice without a usable expected date is placed at the median
  collection date observed for the Entity's own invoices at the same age, age being days
  before or after the due date. Comparable invoices are those with a non-zero total that were
  still unpaid at that age (Q145). The median is the first number of days after that age by
  which at least half of the comparable invoices had been collected; with an even count that
  is the lower middle, never an average. An invoice still open counts as not collected at
  every point, even beyond its current age (Q147). Where the owner
  has marked a customer as behaving differently, only that customer's history is used, its
  jobs' included (Q245). An invoice not yet due, with no comparable history, is placed at its
  due date.
- **Justification:** IFRS 9 and ASC 326 both rest collectability on the entity's own
  historical experience, grouped by shared characteristics, and prescribe no fixed day
  thresholds `[ar: Part 2]`. IFRS 9 treats late payment as a loss even when paid in full, so
  delay and non-payment form one continuum `[ar: Summary 2]`. A collection curve from the
  Entity's own documents is the timing counterpart of the roll-rate and aging methods ASC 326
  names `[ar: Part 2]`. IFRS 9's 30- and 90-day presumptions are credit-risk staging, not
  timing evidence, and are not used (Q75).
- **Reads:** Canonical Facts: invoice (due date, open amount), historical invoices and their
  payment applications. Settings: customers marked as behaving differently (optional, no
  default needed).
- **Produces:** Placement with basis "Entity history" or "customer history", Confidence
  estimated, carrying the median used and the number of comparable invoices; basis "due date",
  Confidence firm, when not yet due with no history.
- **Marks Provisional:** no.

### AR-TIME-03 — An invoice unlikely to be collected within the horizon is excluded (Q74, Q77)

- **Scope:** Universal · **Status:** active
- **Statement:** When more than half of comparable invoices at the same age were never
  collected, or the median collection date lies beyond the horizon, the invoice is left out of
  the horizon and shown as an exclusion with that reason.
- **Justification:** It follows from AR-TIME-02: the median is the point at which collection
  becomes more likely than not. No threshold is added `[ar: Part 2]`.
- **Reads:** the same as AR-TIME-02.
- **Produces:** exclusion Placement, reason "beyond horizon" or "history says uncollected",
  carrying the comparable-invoice count.
- **Marks Provisional:** no.

### AR-TIME-04 — An overdue invoice with no timing evidence is not guessed

- **Scope:** Universal · **Status:** active
- **Statement:** An overdue invoice with no usable expected date and no comparable history is
  left out of expected receipts. A Decision Item asks for an expected date, and the run is
  marked Provisional.
- **Justification:** Missing evidence is surfaced, never filled by a default (ADR-0019). An
  overdue invoice's due date is known not to be when the cash arrives.
- **Reads:** Canonical Facts: invoice; the history AR-TIME-02 reads.
- **Produces:** exclusion Placement, reason "no timing evidence"; Decision Item kind "set
  expected date".
- **Acted on by:** owner.
- **Marks Provisional:** yes.

### AR-SCHED-01 — An active scheduled invoice produces an expected receipt (Q78, Q144)

- **Scope:** Universal · **Status:** active
- **Statement:** An active invoice template scheduled to generate invoices automatically
  produces one expected invoice per scheduled date within the horizon, unless that occurrence
  is covered. Each uncovered occurrence is timed by AR-TIME-02 and AR-TIME-03 as if it were
  open on its scheduled invoice date: its age is taken on that date, and the median is counted
  from that date. With no comparable history it is placed at its scheduled due date (Q151). An
  occurrence is covered by an invoice linked to the template, where the ledger links them (Xero
  `RepeatingInvoiceID`), otherwise by an invoice to the same customer
  dated nearer this scheduled date than either neighbouring one (Q149). For this test
  the schedule's frequency extends before its first date and past its end date, and a
  document exactly halfway covers the earlier occurrence; a covered occurrence is an exclusion citing the
  invoice. Draft, inactive, reminder-only and manual templates are shown as exclusions instead.
- **Justification:** A forecast's receipts include inflows beyond existing AR `[ar: Part 5]`.
  An automatic schedule is the Entity's own evidence of invoices it will issue. Both ledgers
  store schedules: Xero `RepeatingInvoice`, QBO `RecurringTransaction` `[ar: Part 5]`. Neither
  ledger's next-date field reliably tracks what has been generated, and Xero holds invoices
  already generated for future occurrences
  (`docs/research/2026-09-14-sandbox-write-pass.md`, check 1), so without the coverage test an
  occurrence and its open invoice would both be counted. The test is GAP-SCHED-02's (Q118). In
  QBO, where no link exists, two invoices to one customer in a period can mask a missed one;
  that errs toward silence, not a false receipt.
- **Reads:** Canonical Facts: scheduled invoice template, invoice (template link, customer,
  date).
- **Produces:** Placement with basis "scheduled invoice", Confidence estimated; exclusion
  Placement, reason "covered by", for covered occurrences; exclusion Placement for templates
  that are not automatic.
- **Marks Provisional:** no.

### AR-SCHED-02 — A scheduled invoice that did not go out raises an item (Q83, Q144)

- **Scope:** Universal · **Status:** active
- **Statement:** The most recent scheduled date on or before the run date, for any active
  invoice template (automatic, reminder-only or manual), is missed when AR-SCHED-01's coverage
  test finds no invoice covering it. A missed occurrence raises a Decision Item asking for the
  invoice to be issued. No receipt is forecast for it until the invoice exists.
- **Justification:** A schedule is the Entity's own statement that it bills on those dates.
  Whether the invoice went out is judged from invoices, not from the template's next-date field,
  which neither ledger keeps reliably (`docs/research/2026-09-14-sandbox-write-pass.md`,
  check 1). Checking only the most recent past occurrence keeps one stale template from raising
  an item for every month since it was set up.
- **Reads:** Canonical Facts: scheduled invoice template (schedule, type), invoice (template
  link, customer, date).
- **Produces:** Decision Item kind "invoice missed", subject the template and scheduled date.
- **Acted on by:** bookkeeper.
- **Marks Provisional:** no.

---

## Amounts

### AR-FX-01 — A foreign-currency receipt uses the rate booked on the invoice (Q79)

- **Scope:** Universal · **Status:** active
- **Statement:** An open invoice in a currency other than the Entity's Home Currency is placed
  at its Home Currency amount, using the exchange rate booked on the document. The Placement
  records the document currency, amount and rate.
- **Justification:** Canonical Fact amounts are in Home Currency. Date-aware rates are
  deferred to the full close (ADR-0018), and no source prescribes a rate for expected receipts
  `[ar: Part 6]`. Recording the rate keeps the choice visible.
- **Reads:** Canonical Facts: invoice (currency, rate, Home Currency amount).
- **Produces:** Placement carrying currency, document amount and rate.
- **Marks Provisional:** no.

### AR-DISC-01 — An early-payment discount is assumed not taken (Q80)

- **Scope:** Universal · **Status:** active
- **Statement:** Where payment terms offer an early-payment discount, the expected receipt is
  the full open amount.
- **Justification:** Whether a customer takes a discount is uncertain and not recorded until
  payment. QBO terms carry `DiscountPercent`; Xero terms have no settlement discount
  `[ar: Part 6]`.
- **Reads:** Canonical Facts: invoice, payment terms.
- **Produces:** Placement at the full open amount.
- **Marks Provisional:** no.

### AR-UNAPPLIED-01 — Unapplied customer credit reduces that customer's expected receipts (Q81, Q82, Q245)

- **Scope:** Universal · **Status:** active
- **Statement:** Unapplied customer payments, overpayments, prepayments and credit notes
  reduce the same customer's expected receipts, earliest due date first. A customer is its
  top-level parent, so a parent's credit reduces its jobs' invoices (Q245). An invoice reduced
  to nothing is an exclusion, reason "offset by credit", citing the credit (Q148). A Decision
  Item proposes applying the credit, with a Draft Correction in the form of a payment
  application.
- **Justification:** Unapplied payments distort AR, and the fix is to apply them
  `[ar: Part 3]`. Both ledgers record unapplied amounts and fix them by application, not by
  journal `[ar: Part 4]`, `[ledger: Part 2]`.
- **Reads:** Canonical Facts: payment and credit note (unapplied remainder, derived from their
  applications; an overpayment or prepayment is a payment, Q251), invoice.
- **Produces:** reduced Placement amounts citing the credit; exclusion Placement, reason
  "offset by credit"; Decision Item kind "apply credit" with a Draft Correction (payment
  application).
- **Acted on by:** bookkeeper.
- **Marks Provisional:** no.

### AR-UNAPPLIED-02 — A customer credit with nothing to apply to is not a receipt (Q81, Q245)

- **Scope:** Universal · **Status:** active
- **Statement:** Unapplied credit for a customer with no open invoices, its jobs' included
  (Q245), produces no receipt.
  The credit is shown as an exclusion, and a Decision Item asks whether to refund it or hold
  it for future invoices.
- **Justification:** A customer's credit balance is an amount owed to the customer, a
  liability rather than a receivable `[ar: Part 3]`. Whether it is refunded is the owner's
  call, so no refund is forecast.
- **Reads:** Canonical Facts: payment and credit note (unapplied remainder; Q251), invoice.
- **Produces:** exclusion Placement, reason "credit balance"; Decision Item kind "refund or
  apply credit".
- **Acted on by:** owner.
- **Marks Provisional:** no.

---

## Findings for the owner

### AR-COLLECT-01 — An overdue invoice raises a collection item

- **Scope:** Universal · **Status:** active
- **Statement:** Each open invoice past its due date, and not excluded as intercompany,
  produces a Decision Item to follow up with the customer. Items are ordered by open amount.
- **Justification:** Following up after the due date is standard collection practice. No
  threshold decides which overdue invoices qualify, because any threshold would be arbitrary
  (ADR-0019), so ordering carries the priority instead.
- **Reads:** Canonical Facts: invoice. Classifications: intercompany (counterparty).
- **Produces:** Decision Item kind "collect", subject the invoice.
- **Acted on by:** owner.
- **Marks Provisional:** no.

### AR-WRITEOFF-01 — History that says "uncollected" raises a write-off review (Q77, Q82)

- **Scope:** Universal · **Status:** active
- **Statement:** When AR-TIME-03 excludes an invoice because most comparable invoices were
  never collected, a Decision Item proposes reviewing it for write-off, with a Draft
  Correction in the form of a credit note to the account the Entity uses for bad-debt write-offs. If that account is not mapped,
  the draft is incomplete. Any sales-tax relief on the write-off is left to GAP.
- **Justification:** Write-off happens when there is no reasonable expectation of recovery
  `[ar: Part 2, IFRS 9 5.4.4]`. Whether the Entity writes off against an allowance or directly
  is its own policy `[ar: Part 3]`. Both ledgers record write-offs as credit notes or memos
  applied to the invoice, not as journals `[ar: Part 4]`. No account is guessed (ADR-0019).
- **Reads:** AR-TIME-03's outcome. Classifications: bad-debt write-off account.
- **Produces:** Decision Item kind "review for write-off" with a Draft Correction, credit note (incomplete
  when unmapped).
- **Acted on by:** accountant.
- **Marks Provisional:** no.

### AR-UNBILLED-01 — Revenue accrued but not invoiced raises an item (Q83)

- **Scope:** Universal · **Status:** active
- **Statement:** At the end of the last completed month, a balance in an account the Entity
  maps as unbilled receivable or contract asset produces a Decision Item to invoice the accrued
  revenue. It is not forecast as a receipt: no cash is expected until an invoice exists. An
  Entity with no such account mapped has nothing for this Rule to read, and the Rule is
  inactive. That is not missing configuration, because many Entities never accrue unbilled
  revenue.
- **Justification:** Revenue earned before the right to payment is unconditional is presented
  as a contract asset; a receivable arises once only the passage of time is needed
  `[ar: Parts 3 and 6, IFRS 15.105, 15.108]`. A balance there is revenue the business could
  bill. Neither ledger identifies such an account from metadata, so it needs a per-Group
  mapping (Q68) `[ledger: Part 3]`. The balance comes from the trial balance (ADR-0014, Q72).
- **Reads:** Canonical Facts: account balance as of a date. Classifications: unbilled
  receivable or contract asset.
- **Produces:** Decision Item kind "invoice accrued revenue", subject the account and month.
- **Acted on by:** bookkeeper.
- **Marks Provisional:** no.

### AR-TIE-01 — Open AR agrees with the AR control account at month-end

- **Scope:** Universal · **Status:** active
- **Statement:** At the end of the last completed month, open invoices less unapplied
  customer credits, in Home Currency, must equal the balance of the AR control accounts. Any
  difference produces a Decision Item to reconcile AR and marks the run Provisional.
- **Justification:** Subledger-to-control agreement is a standard close check. Differences
  come from postings through AR to other balance-sheet accounts, unapplied payments, and
  currency (the last **UNVERIFIED**) `[ar: Part 3]`. Both ledgers tie AR postings to a
  customer, so a structural difference should be rare and is worth attention when it
  appears. The control balance is a Canonical Fact from the trial balance (ADR-0014, Q72).
- **Reads:** Canonical Facts: invoice, payment applications, credits, account balance as of a
  date. Classifications: AR control.
- **Produces:** Decision Item kind "reconcile AR", subject the AR control account and month.
- **Acted on by:** bookkeeper.
- **Marks Provisional:** yes.
