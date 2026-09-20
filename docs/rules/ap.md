# AP — Accounts payable

Status: **approved by the owner** (2026-09-14). Built: every Rule here is implemented in
`core/src/ap.rs` and covered by a Scenario test, with coverage gated against the table below
(`scenarios/tests/coverage.rs`).

What this Family does: turns an Entity's open bills and committed purchases into expected
payments in the Forecast Run, explains what it left out, and raises the AP findings a
controller would raise before trusting the forecast.

Evidence: `docs/research/2026-09-13-ap-general-practice.md` (cited `[ap: Part n]`),
`docs/research/2026-09-13-ar-general-practice.md` (`[ar: Part n]`) and
`docs/research/2026-09-13-ledger-document-model-qbo-xero.md` (`[ledger: Part n]`). Entry
format follows ADR-0009, including Confidence and the role that acts on each Decision Item
(ADR-0020). Every Rule here is Universal (ADR-0017).

**One asymmetry with AR shapes this Family.** A customer decides when AR is paid; the Entity
decides when AP is paid. Payment history is evidence about a customer, but a record of the
Entity's own past choices is not evidence of what the owner will choose next, and following a
habit of paying late would flatter the forecast `[ap: Part 1]`.

## Decisions behind these Rules

Decided by the owner on 2026-09-13 and recorded in `docs/plans/design-session-2026-09-13.md`.

| # | Question | Decision | Rules |
|---|---|---|---|
| Q97 | Order of timing evidence for bills | Owner-planned payment date, then the due date. No payment history for bills, because the owner controls AP timing and history would flatter the forecast. | AP-TIME-01, 02 |
| Q98 | Overdue bills | Place in week 1 (or the first pay run), Confidence firm, with a Decision Item to pay or plan a date. | AP-TIME-03 |
| Q99 | Payment runs | Optional pay-run weekday per Entity, no default. When set, a bill is paid in the last run on or before its due date; overdue bills in the first run. A planned date still wins. | AP-RUN-01 |
| Q100 | Priority when cash is short | Don't reorder payments in the forecast. Amend CASH-ORDER-01 so that, within a severity class, items concerning amounts held in trust for a government come first, then owner-marked critical vendors and secured lenders, then cash impact. A shortfall item lists the trust obligations inside each stretch. | AP-PRIORITY-01; amends CASH-ORDER-01, CASH-SHORT-01 |
| Q101 | Early-payment discounts on bills | Assume not taken (mirrors AR Q80). Raise an item showing the saving and implied annual rate only when paying early keeps every week at or above zero and any buffer. | AP-DISC-01, 02 |
| Q102 | Purchase orders and recurring bills | Automatic recurring bill templates are expected payments. An authorised PO is placed only when its delivery date and the vendor's payment terms are both known; otherwise it is shown as a committed purchase with unknown timing. | AP-SCHED-01, AP-PO-01 |
| Q103 | Bills paid by credit card | Confirm: a card-paid bill is closed in AP, and the card balance carries the obligation through GAP (CASH-OPEN-04). Nothing is counted twice. | AP-OPEN-01 |
| Q104 | Unapplied vendor credits and debit balances | Net against the same vendor's open bills, oldest first, with an "apply vendor credit" item. A vendor credit with nothing to apply to gets a "claim refund or hold" item, never a forecast receipt. | AP-UNAPPLIED-01, 02 |
| Q105 | Suspected duplicate bills | Flag exact matches and keep both in the forecast until resolved, overstating outflows rather than understating them. | AP-DUP-01 |
| Q106 | Bills possibly paid outside the bill flow | Flag an open bill beside a bank or card spend to the same vendor for the same amount on or after the bill date; keep the bill in the forecast until resolved. | AP-PAID-01 |
| Q107 | Received but not billed | Belongs to GAP, which handles obligations the ledger has not booked as bills. Revisit the Gap definition in the GAP cycle. | — |
| Q108 | Rate for foreign-currency payables | The booked rate, recorded on the Placement (mirrors AR Q79). | AP-FX-01 |
| Q144 | Scheduled bill templates after the write pass | Occurrences covered by a linked bill, or by a bill, card charge or bank spend to the same vendor within the period, are not forecast again. Decided 2026-09-14. | AP-SCHED-01 |
| Q230 | Status words | Documents are draft, awaiting approval, posted, voided or deleted (Q220); "authorised" was Xero's word. Purchase orders keep their own statuses. Decided 2026-09-16. | AP-OPEN-01, 02 |
| Q243 | Open amount in a foreign currency | Derived in the bill's currency, then converted at its booked rate. Decided 2026-09-16. | AP-OPEN-01, AP-FX-01 |
| Q251 | Supplier prepayments | Payments with an unapplied remainder, not documents. Decided 2026-09-16. | AP-UNAPPLIED-01, 02 |
| Q254 | Xero draft repeating bills | A draft template generates draft bills, so its occurrences are placed like draft bills (Confidence estimated). Decided 2026-09-16. | AP-SCHED-01 |
| Q145 | Voided and zero-total documents | Ignored by every AP Rule. Decided 2026-09-14. | AP-OPEN-01 |
| Q109 | Draft and awaiting-approval bills | Include at their due date with Confidence estimated, plus an "approve or delete bill" item. A draft bill usually means an invoice already received, so leaving it out understates outflows. | AP-OPEN-02 |

---

## What counts as open AP

### AP-OPEN-01 — A posted bill with an open amount is a payable

- **Scope:** Universal · **Status:** active
- **Statement:** A bill is a payable when it is posted (not voided or deleted) and its
  open amount is greater than zero. The open amount is the bill total less payments and vendor
  credits applied on or before the run date, worked out in the bill's currency and converted to
  Home Currency at the rate booked on the bill (Q229, Q243). A bill paid by credit card is
  closed: the obligation now sits on the card, which the GAP Family schedules. A document with a
  zero total is ignored by every AP Rule (Q145).
- **Justification:** Trade payables are amounts invoiced or formally agreed with a supplier
  `[ap: Part 4, IAS 37.11]`. Open amounts come from documents and their applications, never
  aging reports (ADR-0014). A card payment names a card account, not a bank account
  `[ap: Summary 4]`, and card balances are Gaps (ADR-0020, CASH-OPEN-04).
- **Reads:** Canonical Facts: bill, bill payment, vendor credit, payment application.
- **Produces:** the open amount every other AP Rule works from.
- **Marks Provisional:** no.

### AP-OPEN-02 — A draft bill is a likely obligation (Q109)

- **Scope:** Universal · **Status:** active
- **Statement:** A bill that is a draft or awaiting approval is placed like a posted bill,
  with Confidence estimated, and raises a Decision Item to approve or delete it.
- **Justification:** Unlike a draft customer invoice, which has not been sent, a draft bill
  usually records a vendor invoice already received. Leaving it out would understate outflows.
  Xero marks these `DRAFT` and `SUBMITTED` `[ledger: Part 1]`.
- **Reads:** Canonical Facts: bill (status).
- **Produces:** Placement as for a posted bill, Confidence estimated; Decision Item kind
  "approve or delete bill".
- **Acted on by:** bookkeeper.
- **Marks Provisional:** no.

### AP-OPEN-03 — An intercompany payable is not an external payment

- **Scope:** Universal · **Status:** active
- **Statement:** A bill from a counterparty classified intercompany is left out of AP's expected
  payments and shown as an exclusion. IC Rules handle it. A related party outside the Group is
  treated like any other vendor.
- **Justification:** Intragroup balances and flows are eliminated on consolidation
  `[ap: Part 7]`, `[ar: Part 6]`.
- **Reads:** Classifications: intercompany (counterparty).
- **Produces:** exclusion Placement, reason "intercompany", citing the IC Family.
- **Marks Provisional:** no.

### AP-OPEN-04 — Expected payments include the sales tax on the bill

- **Scope:** Universal · **Status:** active
- **Statement:** An expected payment is the full open amount, including tax charged by the
  vendor. Where that tax is recoverable, the recovery reduces a remittance scheduled by the GAP
  Family.
- **Justification:** The vendor is paid the gross amount. In Canada, input tax credits reduce
  the net tax remitted `[ap: Part 7]`; in the US, sales tax on purchases is generally a cost
  (**UNVERIFIED**) `[ap: Part 7]`. Either way AP pays gross.
- **Reads:** Canonical Facts: bill (total, tax).
- **Produces:** Placement amount gross of tax.
- **Marks Provisional:** no.

---

## Timing

### AP-TIME-01 — An owner-planned payment date places the bill (Q97)

- **Scope:** Universal · **Status:** active
- **Statement:** When an open bill carries a planned payment date entered by a person, on or
  after the run date, the payment is placed in that date's week. A planned date that has passed
  while the bill is still open is ignored, the later AP-TIME Rules apply, and a Decision Item
  asks for the date to be updated. When the bill is overdue, AP-TIME-03's "overdue bill" item
  cites the passed date and this Rule raises no item (Q150).
- **Justification:** Paying a bill is the owner's decision, so a stated plan is the best
  evidence. Xero stores `PlannedPaymentDate` `[ap: Part 1]`; QBO has no such field
  `[ledger: Part 1]`.
- **Reads:** Canonical Facts: bill (planned payment date).
- **Produces:** Placement with basis "planned date", Confidence firm; Decision Item kind
  "planned date passed".
- **Acted on by:** owner.
- **Marks Provisional:** no.

### AP-TIME-02 — Otherwise, a bill not yet due is paid on its due date (Q97)

- **Scope:** Universal · **Status:** active
- **Statement:** An open bill without a usable planned date, and not yet due, is placed on its
  due date, moved to the pay-run day when AP-RUN-01 applies.
- **Justification:** The due date is the contractual obligation. The Entity's own payment
  history is deliberately not used: it records past choices, not future ones, and following a
  habit of paying late would make cash look better than the obligations allow `[ap: Part 1]`.
- **Reads:** Canonical Facts: bill (due date).
- **Produces:** Placement with basis "due date" or "pay run", Confidence firm.
- **Marks Provisional:** no.

### AP-TIME-03 — An overdue bill is payable now (Q98)

- **Scope:** Universal · **Status:** active
- **Statement:** An open bill past its due date, without a usable planned date, is placed in
  week 1, or in the first pay run when AP-RUN-01 applies. It raises a Decision Item to pay it or
  set a planned date.
- **Justification:** An overdue bill is owed now. Placing it later would assume a decision the
  owner hasn't made, and it is usually the vendor, not the Entity, who ends the delay.
- **Reads:** Canonical Facts: bill (due date).
- **Produces:** Placement with basis "overdue", Confidence firm; Decision Item kind "overdue
  bill".
- **Acted on by:** owner.
- **Marks Provisional:** no.

### AP-RUN-01 — An Entity's pay-run day moves payments to that weekday (Q99)

- **Scope:** Universal · **Status:** active
- **Statement:** When the Entity has a pay-run weekday set, a bill timed by its due date is
  paid in the last pay run on or before that date, and an overdue bill in the first pay run on
  or after the run date. When the last run on or before the due date is already past, the bill
  is paid in the first run on or after the run date, and it is not overdue (Q156). A planned payment date is not moved. With no pay-run day set, this
  Rule does nothing.
- **Justification:** Many businesses pay bills in fixed runs, but neither ledger stores a
  pay-run calendar `[ap: Part 1]`, so it is an optional Setting with no default (ADR-0019).
  Paying in the last run before the due date keeps payments on time.
- **Reads:** Settings: pay-run weekday (optional, no default).
- **Produces:** Placement basis "pay run" on the adjusted date.
- **Marks Provisional:** no.

### AP-SCHED-01 — An active scheduled bill produces an expected payment (Q102, Q144, Q254)

- **Scope:** Universal · **Status:** active
- **Statement:** An active bill template scheduled to generate bills automatically produces
  one expected payment per scheduled date within the horizon, placed on the scheduled bill's
  due date (moved by AP-RUN-01 when set), unless that occurrence is covered. An occurrence is
  covered by a bill linked to the template where the ledger links them, otherwise by a bill,
  card charge or bank spend to the same vendor
  dated nearer this scheduled date than either neighbouring one (Q149). For this test
  the schedule's frequency extends before its first date and past its end date, and a
  document exactly halfway covers the earlier occurrence; a covered occurrence is an exclusion citing it. A template that
  generates draft bills automatically (Xero's draft repeating bill) is placed the same way with
  Confidence estimated, as the draft bills it creates are (AP-OPEN-02, Q254). Inactive,
  reminder-only and manual templates are shown as exclusions.
- **Justification:** A schedule is the Entity's own evidence of an obligation it expects.
  Both ledgers store bill schedules `[ap: Part 6]`. Neither ledger's next-date field reliably
  tracks what has been generated, and Xero holds bills already generated for future
  occurrences (`docs/research/2026-09-14-sandbox-write-pass.md`, check 1), so without the
  coverage test a scheduled bill and its open bill would both be paid in the forecast. The test
  is GAP-SCHED-02's (Q118).
- **Reads:** Canonical Facts: scheduled bill template, bill, card transaction, bank transaction.
- **Produces:** Placement with basis "scheduled bill", Confidence estimated; exclusion
  Placement, reason "covered by", for covered occurrences; exclusion Placement for templates
  that generate nothing automatically.
- **Marks Provisional:** no.

### AP-PO-01 — A committed purchase is placed only when its timing is known (Q102)

- **Scope:** Universal · **Status:** active
- **Statement:** An authorised purchase order not yet billed is placed at its delivery date
  plus the vendor's payment terms when both are known, with Confidence estimated. Otherwise it
  is shown as an exclusion, "committed purchase, timing unknown", with its amount.
- **Justification:** A purchase order is a commitment, not a liability, and does not post to the
  ledger `[ap: Part 6]`. A large commitment can still drain cash. Xero POs carry a delivery date
  and contacts can carry payment terms; QBO POs have neither in the data read `[ap: Part 6]`,
  `[ledger: Part 1]`. No timing is guessed (ADR-0019).
- **Reads:** Canonical Facts: purchase order (status, delivery date, total), vendor payment
  terms.
- **Produces:** Placement with basis "purchase order", Confidence estimated; or exclusion
  Placement, reason "committed purchase, timing unknown".
- **Marks Provisional:** no.

---

## Amounts

### AP-FX-01 — A foreign-currency payment uses the rate booked on the bill (Q108)

- **Scope:** Universal · **Status:** active
- **Statement:** An open bill in a currency other than the Entity's Home Currency is placed at
  its Home Currency amount, using the rate booked on the document. The Placement records the
  document currency, amount and rate.
- **Justification:** Mirrors AR-FX-01 (design-session-2026-09-13 Q79). No source prescribes a
  rate for expected payments `[ap: Part 7]`.
- **Reads:** Canonical Facts: bill (currency, rate, Home Currency amount).
- **Produces:** Placement carrying currency, document amount and rate.
- **Marks Provisional:** no.

### AP-DISC-01 — An early-payment discount is assumed not taken (Q101)

- **Scope:** Universal · **Status:** active
- **Statement:** Where a bill's payment terms offer an early-payment discount, the expected
  payment is the full open amount on the normal timing.
- **Justification:** Whether a discount is taken is the owner's choice. This mirrors AR-DISC-01.
  QBO terms carry discounts; Xero terms do not `[ap: Part 3]`.
- **Reads:** Canonical Facts: bill, payment terms.
- **Produces:** Placement at the full open amount.
- **Marks Provisional:** no.

### AP-DISC-02 — An affordable early-payment discount raises an item (Q101)

- **Scope:** Universal · **Status:** active
- **Statement:** When a bill's discount date is on or after the run date, and paying the
  discounted amount on that date would keep every week's closing cash at or above zero and any
  Minimum Cash Buffer, a Decision Item shows the saving and the implied annual rate of not
  taking it. Discounts are judged in discount-date order (larger saving first on a tie), each
  with the earlier suggested discounts taken, so every item raised can be acted on together
  (Q157). The implied rate is discount ÷ (1 − discount) × 365 ÷ days from the discount date to
  the due date.
- **Justification:** Forgoing a discount is expensive borrowing: 2/10 net 30 is about 37% a
  year simple `[ap: Part 3]`. That arithmetic needs no cost-of-funds Setting. Raising the item
  only when the forecast can absorb the earlier payment avoids suggesting a saving that causes a
  shortfall.
- **Reads:** Canonical Facts: bill, payment terms. CASH-ROLL-01; Settings: Minimum Cash Buffer.
- **Produces:** Decision Item kind "take discount", with saving and implied annual rate.
- **Acted on by:** owner.
- **Marks Provisional:** no.

### AP-UNAPPLIED-01 — Unapplied vendor credit reduces that vendor's expected payments (Q104)

- **Scope:** Universal · **Status:** active
- **Statement:** Unapplied vendor credits and supplier prepayments reduce the same vendor's
  expected payments, earliest due date first. A bill reduced to nothing is an exclusion, reason
  "offset by credit", citing the credit (Q148). A Decision Item proposes applying the credit,
  with a Draft Correction (payment application).
- **Justification:** Mirrors AR-UNAPPLIED-01. Both ledgers apply vendor credits by
  application, not journal `[ap: Parts 4–5]`.
- **Reads:** Canonical Facts: vendor credit and payment (unapplied remainder; a supplier
  prepayment is a payment, Q251), bill.
- **Produces:** reduced Placement amounts citing the credit; exclusion Placement, reason
  "offset by credit"; Decision Item kind "apply vendor credit" with a Draft Correction (payment
  application).
- **Acted on by:** bookkeeper.
- **Marks Provisional:** no.

### AP-UNAPPLIED-02 — A vendor credit with nothing to apply to is not a receipt (Q104)

- **Scope:** Universal · **Status:** active
- **Statement:** Unapplied credit from a vendor with no open bills produces no receipt. It is
  shown as an exclusion, and a Decision Item asks whether to claim a refund or hold it for
  future bills.
- **Justification:** A vendor that owes the Entity is an asset, not a negative payable
  `[ap: Part 4]`. Whether a refund is pursued is the owner's call, so none is forecast.
- **Reads:** Canonical Facts: vendor credit and payment (unapplied remainder; a supplier
  prepayment is a payment, Q251), bill.
- **Produces:** exclusion Placement, reason "vendor credit balance"; Decision Item kind "claim
  refund or hold credit".
- **Acted on by:** owner.
- **Marks Provisional:** no.

---

## Findings for the owner

### AP-PRIORITY-01 — What is owed to a government, or marked critical, is named first (Q100)

- **Scope:** Universal · **Status:** active
- **Statement:** A vendor or obligation classified as held in trust for a government (such as
  withheld source deductions or sales tax collected), or marked by the owner as critical or as a
  secured lender, is recorded on the Placements and Decision Items that concern it. CASH-ORDER-01
  uses the mark to rank items, and CASH-SHORT-01 lists trust obligations inside each shortfall
  stretch. The forecast itself is not reordered: no payment is moved or dropped.
- **Justification:** Amounts collected or withheld for a government are not the business's
  money. In Canada directors are personally liable for unremitted source deductions and GST/HST
  (ITA s. 227.1, ETA s. 323); in the US, paying other creditors instead is an indication of
  willfulness for the Trust Fund Recovery Penalty `[ap: Part 2]`. Which suppliers are critical is
  the owner's knowledge `[ap: Part 2]`. Choosing whom not to pay is a decision for the owner, not
  the engine.
- **Reads:** Classifications: government trust. Settings: critical vendor, secured lender (both
  optional, no default).
- **Produces:** a priority mark on Placements and Decision Items.
- **Marks Provisional:** no.

### AP-DUP-01 — A possible duplicate bill is flagged, and both stay in the forecast (Q105)

- **Scope:** Universal · **Status:** active
- **Statement:** Two open bills from the same vendor with the same vendor reference, or with the
  same amount and bill date, raise a Decision Item to check for a duplicate. Both stay in the
  forecast until one is voided or deleted.
- **Justification:** Duplicate bills are a recognised AP control risk (**UNVERIFIED** against a
  professional body) `[ap: Part 4]`. Keeping both overstates outflows until resolved, which is the
  safer error. Exact matches need no threshold.
- **Reads:** Canonical Facts: bill (vendor, reference, amount, date).
- **Produces:** Decision Item kind "possible duplicate bill", subject both bills.
- **Acted on by:** bookkeeper.
- **Marks Provisional:** no.

### AP-PAID-01 — A bill possibly paid outside the bill flow is flagged (Q106)

- **Scope:** Universal · **Status:** active
- **Statement:** An open bill raises a Decision Item when a bank or card spend transaction that
  is not a bill payment is recorded to the same vendor, for the same amount, on or after the bill
  date. The bill stays in the forecast until the bookkeeper matches the payment or confirms the
  bill is still owed.
- **Justification:** Recording a bank transaction as a new expense instead of matching it to
  the bill leaves the bill open and books the cost twice `[ap: Part 4]`. Keeping the bill until
  resolved overstates outflows, the safer error.
- **Reads:** Canonical Facts: bill, bank transaction, card transaction. Classifications: bank,
  credit card.
- **Produces:** Decision Item kind "bill may already be paid", subject the bill and the
  transaction.
- **Acted on by:** bookkeeper.
- **Marks Provisional:** no.

### AP-TIE-01 — Open AP agrees with the AP control account at month-end

- **Scope:** Universal · **Status:** active
- **Statement:** At the end of the last completed month, open bills less unapplied vendor
  credits, in Home Currency, must equal the balance of the AP control accounts. Any difference
  produces a Decision Item to reconcile AP and marks the run Provisional.
- **Justification:** Subledger-to-control agreement is a standard close check; differences come
  from postings through AP to other balance-sheet accounts and unapplied payments `[ap: Part 4]`.
  Xero rejects manual journals to the AP system account, so differences there should be rare
  (confirmed in `docs/research/2026-09-14-sandbox-write-pass.md`, check 3). The control balance is the dated balance fact (ADR-0014).
- **Reads:** Canonical Facts: bill, payment applications, vendor credits, account balance as of
  a date. Classifications: AP control.
- **Produces:** Decision Item kind "reconcile AP", subject the AP control account and month.
- **Acted on by:** bookkeeper.
- **Marks Provisional:** yes.
