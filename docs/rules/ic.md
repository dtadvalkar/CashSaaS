# IC — Intercompany

Status: **approved by the owner** (2026-09-14). Built: every Rule here is implemented in
`core/src/ic.rs` and covered by a Scenario test, with coverage gated against the table in
`docs/scenarios/ic.md` (`scenarios/tests/coverage.rs`). IC-MAP-01, IC-DOC-01, IC-ELIM-01 and
IC-FUND-01 were written a Family early for CASH (Q275); this Family's tests now gate them too.

What this Family does: forecasts cash moving between Entities in the same Group, pairs both
sides so the Group view can eliminate them, checks that reciprocal balances agree, and points
out when one Entity could fund another's shortfall and what that would mean.

Evidence: `docs/research/2026-09-13-ic-general-practice.md` (cited `[ic: Part n]`), with the
ledger, AR, AP and GAP research notes of the same date. Entry format follows ADR-0009, including
Confidence and the role that acts on each Decision Item (ADR-0020). Every Rule here is Universal
(ADR-0017).

**The shape of this Family.** Neither ledger's standard product has an intercompany feature or
checks that the two sides agree `[ic: Summary 2]`. Each Entity books its own side as ordinary
accounts and contacts, so IC depends on the Group's mapping saying which account or contact in
one Entity *is* another Entity. Settlements are real cash for each Entity and eliminate in the
Group view (Q87), except for currency differences, which are real exposure `[ic: Summary 1]`.

## Decisions behind these Rules

Decided by the owner on 2026-09-14 and recorded in `docs/plans/design-session-2026-09-13.md`.

| # | Question | Decision | Rules |
|---|---|---|---|
| Q124 | Agreement test for same-currency pairs | Exact agreement by default, with an optional per-Group tolerance Setting (no default). A disagreement is a blocking finding and marks the run Provisional, as ADR-0018 set. | IC-AGREE-01 |
| Q125 | Agreement across currencies | Compare at the owner's conversion-rate Setting (Q30). A difference raises an action item stating that currency movement is expected, and does not mark the run Provisional. Only same-currency disagreement blocks. | IC-AGREE-02 |
| Q126 | Balances with no due date | An owner-entered settlement schedule places the payment on both Entities. With none, both sides are excluded, with an item to enter a schedule or confirm "not settling within the horizon". Not Provisional: standing intercompany balances often have no intention to settle soon. | IC-LOAN-01, 02 |
| Q127 | Which side's dates win | Invoiced recharges are timed by the paying Entity's planned date, else its due date, on both sides, so the pair lands in the same week. If only one side is booked, that side's dates. | IC-DOC-01 |
| Q128 | A counterparty Entity not connected | Forecast the visible side as normal. It is not eliminated in the Group view, because the other side isn't in it. An item asks to connect the Entity or obtain a balance confirmation. Not Provisional. | IC-ONESIDED-01 |
| Q129 | Suggesting transfers | Yes, as an item beside a shortfall, naming each Entity that could lend and still stay at or above zero and its buffer in every week. It carries caveats (characterise it as a loan, distribution or contribution; document terms; solvency; shareholder-loan and interest rules; lender covenants) and is never forecast automatically. | IC-FUND-01 |
| Q130 | Who books the true-up | A Draft Correction only where the fix is evident: an intercompany invoice with no matching bill on the other side gets a draft bill for that side. A balance-only disagreement gets an item, not a guessed journal. | IC-DOC-02, IC-AGREE-01 |
| Q131 | Mapping granularity | The intercompany Classification names the counterparty Entity, per account and per contact. An intercompany account or contact with no counterparty named is unpaired, and raises an item. | IC-MAP-01 |
| Q132 | Owner flows and scheduled inflows | Owner drawings, funds introduced and shareholder loans are forecast only when the owner enters them. A CASH Rule forecasts owner-entered scheduled receipts (injections, loan proceeds, refunds), the inflow counterpart of GAP-SCHED-01. | CASH-SCHED-01 |
| Q133 | GST/HST on recharges | No IC handling. Tax on intercompany recharges is part of the booked tax liability GAP already remits. | — |

---

## Pairing

### IC-MAP-01 — Every intercompany account and contact names its counterparty Entity (Q131)

- **Scope:** Universal · **Status:** active
- **Statement:** An account or contact classified as intercompany must name which Entity in the
  Group it represents. One that does not is still treated as intercompany, so it is left out of
  AR and AP, but it cannot be paired. Its documents are still forecast by IC-DOC-01 on their own
  dates, unpaired, so no payment is dropped (Q162). It raises a Decision Item to name the
  counterparty.
- **Justification:** Pairing needs to know which account or contact in one Entity is another
  Entity, and neither ledger's metadata says so `[ic: Part 1, Part 7]`. The mapping is per Group
  (Q68).
- **Reads:** Classifications: intercompany, with counterparty Entity.
- **Produces:** the pairing key used by every other IC Rule; Decision Item kind "name
  intercompany counterparty".
- **Acted on by:** bookkeeper.
- **Marks Provisional:** no.

### IC-ELIM-01 — Paired Placements eliminate in the Group view; currency differences don't

- **Scope:** Universal · **Status:** active
- **Statement:** Each IC Placement carries a pair reference. CASH-GROUP-01 eliminates both
  Placements of a pair from Group totals. When the pair is in different currencies, the part that
  does not eliminate at the conversion rate is shown as an intercompany currency difference, not
  hidden. An unpaired Placement is not eliminated.
- **Justification:** Consolidation eliminates intragroup flows in full `[ic: Part 1]`, but a
  currency difference on an intragroup monetary item survives, because it is real exposure
  (IAS 21.45) `[ic: Part 3]`.
- **Reads:** IC Placements and pair references. Settings: conversion rate per currency pair.
- **Produces:** elimination pairs for CASH-GROUP-01; a Group-view line "intercompany currency
  difference".
- **Marks Provisional:** no.

---

## Settlements

### IC-DOC-01 — An intercompany invoice is paid on the paying Entity's dates (Q127)

- **Scope:** Universal · **Status:** active
- **Statement:** An open invoice or bill between two Entities in the Group is forecast in both
  Entities: a payment in the paying Entity and a receipt in the receiving one, in the same week.
  The date is the paying Entity's planned payment date, else its due date. An overdue one is
  placed in week 1. If only one side is booked, that side's dates are used. Both Placements are
  paired.
- **Justification:** A recharge or management fee has a document with a due date `[ic: Part 1]`.
  The Group controls both sides, so the payer's decision sets the timing, as for AP (Q97).
  Timing both sides the same week lets the Group view eliminate them cleanly.
- **Reads:** Canonical Facts: invoice, bill. Classifications: intercompany with counterparty.
- **Produces:** paired Placements, basis "intercompany document", Confidence firm.
- **Marks Provisional:** no.

### IC-DOC-02 — An intercompany invoice the other side hasn't booked gets a draft bill (Q130)

- **Scope:** Universal · **Status:** active
- **Statement:** An intercompany invoice with no matching bill in the counterparty Entity (the
  same reference, or the same amount and date) raises a Decision Item for the counterparty, with
  a Draft Correction in the form of a bill mirroring the invoice. The reverse applies to a bill
  with no matching invoice.
- **Justification:** One side unbooked is a named cause of intercompany mismatch
  `[ic: Part 2]`. Here the fix is evident from the other side's document, so a draft is useful.
  Exact matches need no threshold (ADR-0019).
- **Reads:** Canonical Facts: invoice, bill, in both Entities. Classifications: intercompany with
  counterparty.
- **Produces:** Decision Item kind "counterparty has not booked", with a Draft Correction (bill or
  invoice).
- **Acted on by:** bookkeeper.
- **Marks Provisional:** no.

### IC-LOAN-01 — An intercompany balance with a settlement schedule is paid on its dates (Q126)

- **Scope:** Universal · **Status:** active
- **Statement:** An owner-entered settlement schedule for an intercompany balance (the two
  Entities, amount, frequency, next date, end date) places a payment in the owing Entity and a
  receipt in the other for each occurrence within the horizon, paired. A transfer booked between
  the two Entities within the occurrence's period covers it, as in GAP-SCHED-02.
- **Justification:** Due-to and due-from balances carry no terms in either ledger `[ic: Summary 4]`.
  The owner's schedule is the evidence, as for external loans (Q116).
- **Reads:** Settings: intercompany settlement schedule. Canonical Facts: bank transactions in
  both Entities.
- **Produces:** paired Placements, basis "intercompany schedule", Confidence firm.
- **Marks Provisional:** no.

### IC-LOAN-02 — An intercompany balance with no schedule is shown, not guessed (Q126)

- **Scope:** Universal · **Status:** active
- **Statement:** An intercompany balance with no settlement schedule is left out of both
  Entities' forecasts and shown as an exclusion. A Decision Item asks the owner either to enter a
  schedule or to confirm it is not settling within the horizon. Confirming closes the item. The
  item is one item, shown in both Entities' queues with one status (Q161).
- **Justification:** Placing it anywhere would guess at a decision nobody has made (ADR-0019). The
  run is not Provisional, because many standing intercompany balances are not expected to settle
  soon, and marking every such Group Provisional would drown the signal.
- **Reads:** Canonical Facts: account balance as of a date. Classifications: intercompany with
  counterparty. Settings: intercompany settlement schedule, "not settling" confirmation.
- **Produces:** exclusion Placement, reason "no settlement schedule"; Decision Item kind
  "intercompany settlement plan".
- **Acted on by:** owner.
- **Marks Provisional:** no.

### IC-ONESIDED-01 — A counterparty Entity that isn't connected (Q128)

- **Scope:** Universal · **Status:** active
- **Statement:** When the counterparty named by IC-MAP-01 is not a connected Entity, the visible
  side is forecast by IC-DOC-01 or IC-LOAN-01 as usual, is not eliminated, and cannot be agreed.
  A Decision Item asks to connect the Entity or record a balance confirmation from it.
- **Justification:** Only one side is visible, so agreement is impossible, and the Group view
  holds only what it can see `[ic: Part 4]`. The visible Entity's own documents and balances are
  still valid evidence for its own forecast.
- **Reads:** Classifications: intercompany with counterparty. Connected Entities in the Group.
- **Produces:** unpaired Placements; Decision Item kind "connect intercompany counterparty".
- **Acted on by:** owner.
- **Marks Provisional:** no.

---

## Agreement

### IC-AGREE-01 — Same-currency reciprocal balances agree at month-end (Q124, Q130)

- **Scope:** Universal · **Status:** active
- **Statement:** At the end of the last completed month, for each pair of connected Entities with
  the same Home Currency, what Entity A shows as owed by or to Entity B must equal, with opposite
  sign, what Entity B shows for Entity A. They must agree exactly, unless the Group has set a
  tolerance. A disagreement raises a Decision Item for both Entities and marks the run
  Provisional. No journal is proposed, because which side is wrong needs investigation.
- **Justification:** Agreeing reciprocal balances is the standard intercompany control, and the
  ledgers don't do it `[ic: Parts 2, 7]`. No standard sets a tolerance, so any tolerance is the
  Group's own policy, optional with no default (ADR-0019). ADR-0018 makes this a fitness-to-forecast
  check.
- **Reads:** Canonical Facts: account balances as of a date in both Entities, open intercompany
  documents. Classifications: intercompany with counterparty. Settings: intercompany tolerance
  (optional, no default).
- **Produces:** Decision Item kind "intercompany balances disagree", subject the pair and month.
- **Acted on by:** bookkeeper.
- **Marks Provisional:** yes.

### IC-AGREE-02 — Cross-currency reciprocal balances are compared, and differences explained (Q125)

- **Scope:** Universal · **Status:** active
- **Statement:** For a pair of connected Entities with different Home Currencies, the two sides
  are compared in the Reporting Currency at the conversion rate Setting. A difference raises an
  action item saying currency movement is expected and showing the amount, and does not mark the
  run Provisional. The item is shown in both Entities' queues with one status (Q161). With no rate set, CASH-GROUP-02 applies.
- **Justification:** Reciprocal balances in different currencies legitimately differ after
  translation. The difference is real exposure, not necessarily an error (IAS 21.45)
  `[ic: Part 3]`. The only rate available in M1 is the owner's Setting (design-session-2026-09-11
  Q30), not a closing rate, so the comparison can suggest but not prove an error.
- **Reads:** Canonical Facts: account balances as of a date in both Entities. Settings: conversion
  rate per currency pair.
- **Produces:** Decision Item kind "intercompany currency difference", subject the pair and month.
- **Acted on by:** accountant.
- **Marks Provisional:** no.

---

## Funding

### IC-FUND-01 — A shortfall that a sister Entity could cover is pointed out, with its consequences (Q129)

- **Scope:** Universal · **Status:** active
- **Statement:** When an Entity has a cash shortfall (CASH-SHORT-01), each connected Entity in the
  Group that could transfer enough to cover the deepest week is named, provided its own closing
  cash would stay at or above zero and its Minimum Cash Buffer in every week afterwards. The
  transfer tested is the deepest shortfall, moved in the first week of the shortfall, converted at
  the conversion rate Setting when currencies differ; "afterwards" runs from that week to the end
  of the horizon (Q163). The item
  states the amount and the week, and carries these caveats:
  - the transfer must be characterised as a loan, a distribution or a capital contribution;
  - a loan needs documented terms, and arm's-length interest may apply;
  - a distribution must pass the lender's solvency test;
  - shareholder-loan rules can tax a loan to an individual owner;
  - lender covenants may restrict it.

  Nothing is forecast until a transfer is booked or scheduled.
- **Justification:** Common ownership doesn't make cash shared (Q87); moving it is a legal act whose
  consequences depend on what it is `[ic: Part 5]`, for example CBCA s.42's solvency test, ITA
  s.15(2) on shareholder loans, and arm's-length interest on intercompany loans. The owner decides,
  ideally with the accountant, so the engine suggests and never assumes.
- **Reads:** CASH-SHORT-01, every Entity's CASH-ROLL-01 output. Settings: Minimum Cash Buffer.
- **Produces:** Decision Item kind "consider intercompany funding", linked to the shortfall item.
- **Acted on by:** owner.
- **Marks Provisional:** no.
