# IC Scenarios

Status: **approved by the owner** (2026-09-14). Nothing here is built. IC-S07 is the Scenario that
first proves IC-FUND-01, which the CASH build writes ahead of this Family (Q275).

Each Scenario is a named general accounting concern, described as Canonical Facts and Settings,
with the outputs the IC Rules must produce (ADR-0010, ADR-0016). Together these exercise every
Rule in `docs/rules/ic.md`.

## Conventions

The conventions in `docs/scenarios/ar.md` apply: run date Wednesday 2026-10-07, a 13-week
horizon (W1 is 10-07 to 10-13, W13 is 12-30 to 2027-01-05), an empty Group at the start of each
Scenario, and exhaustive expected outputs for this Family. Also:

- **Group:** Lee Family Holdings, Reporting Currency CAD. Its Entities, all connected unless
  stated:

  | Entity | Short name | Home Currency |
  |---|---|---|
  | Maple Ridge Landscaping Ltd. | Maple Ridge | CAD |
  | Birch Hill Nursery Ltd. | Birch Hill | CAD |
  | Cascade Garden Supply Inc. | Cascade | USD |

- **Every intercompany account and contact names its counterparty Entity,** unless stated.
- **No intercompany tolerance and no conversion rate** are set, unless stated.
- **Pairs** are labelled P1, P2, and so on. A paired Placement appears once per Entity.
- **Dates the owner enters are used as entered.**

---

## IC-S01 — Recharges between two Entities, booked on one side or both

Concern: an intercompany invoice is cash for each Entity, timed by the Entity that pays it, and
paired so the Group view cancels it; a side not booked gets a draft.

**Facts.** Maple Ridge and Birch Hill only.

| Document | Issued by | Paid by | Dated | Due | Amount | Other side |
|---|---|---|---|---|---|---|
| Invoice BH-501 (nursery stock) | Birch Hill | Maple Ridge | 10-01 | 10-31 | 4,000.00 | Maple Ridge bill, reference BH-501, planned payment date 10-21 |
| Invoice MR-M09 (September management fee) | Maple Ridge | Birch Hill | 09-15 | 09-30 | 1,500.00 | Birch Hill bill, reference MGMT-SEP, dated 09-15, 1,500.00, no planned date |
| Invoice BH-502 (nursery stock) | Birch Hill | Maple Ridge | 10-05 | 11-04 | 2,200.00 | none booked in Maple Ridge |

At 09-30, MR-M09 is the only open intercompany document, and both sides show 1,500.00.

**Expected**

| Output | Entity | Document | Week | Amount | Basis | Confidence | Pair | Rules |
|---|---|---|---|---|---|---|---|---|
| Placement, payment | Maple Ridge | BH-501 | W3 (10-21) | 4,000.00 | intercompany document | firm | P1 | IC-DOC-01 |
| Placement, receipt | Birch Hill | BH-501 | W3 (10-21) | 4,000.00 | intercompany document | firm | P1 | IC-DOC-01, IC-ELIM-01 |
| Placement, payment | Birch Hill | MR-M09 | W1 | 1,500.00 | intercompany document | firm | P2 | IC-DOC-01 |
| Placement, receipt | Maple Ridge | MR-M09 | W1 | 1,500.00 | intercompany document | firm | P2 | IC-DOC-01, IC-ELIM-01 |
| Placement, payment | Maple Ridge | BH-502 | W5 (11-04) | 2,200.00 | intercompany document | firm | P3 | IC-DOC-01 |
| Placement, receipt | Birch Hill | BH-502 | W5 (11-04) | 2,200.00 | intercompany document | firm | P3 | IC-DOC-01, IC-ELIM-01 |

| Entity | Decision Item | Subject | Acted on by | Draft Correction | Rules |
|---|---|---|---|---|---|
| Maple Ridge | counterparty has not booked | BH-502 | bookkeeper | Bill from Birch Hill Nursery Ltd., reference BH-502, dated 10-05, due 11-04, 2,200.00 | IC-DOC-02 |

- **BH-501 lands in W3 on both sides,** at the payer's planned date, not Birch Hill's due date
  (10-31, W4).
- **MR-M09 is overdue,** so both sides land in W1.
- **BH-502 is matched by nothing:** Maple Ridge has no bill with that reference, or with that
  amount and date. Only Birch Hill's side is booked, so its dates are used, and Maple Ridge still
  forecasts the payment.
- **MR-M09's bill matches** by amount and date, though the references differ.
- **Group view:** P1, P2 and P3 eliminate in full; no currency difference.
- **Provisional:** no.

---

## IC-S02 — Intercompany loans: scheduled, unscheduled and not settling

Concern: a balance between Entities has no due date, so it is forecast only from the owner's
schedule; without one it is shown and the owner is asked, and the run stays settled.

**Facts.** Maple Ridge and Birch Hill only. Balances at 09-30 and 10-07, agreeing on both sides:

| Balance | Maple Ridge account | Birch Hill account | Amount | Settings |
|---|---|---|---|---|
| L1 | Loan to Birch Hill | Loan from Maple Ridge | 30,000.00 at 09-30; 27,500.00 at 10-07 | Settlement schedule: Birch Hill pays Maple Ridge 2,500.00 monthly, next date 10-15 |
| L2 | Due to Birch Hill | Due from Maple Ridge | 8,000.00 | none |
| L3 | Long-term Advance to Birch Hill | Long-term Advance from Maple Ridge | 50,000.00 | confirmed "not settling within the horizon" |

Transfer 10-05: 2,500.00 from Birch Hill's RBC Business Chequing to Maple Ridge's, booked in both
Entities against L1.

**Expected**

| Output | Entity | Item | Week | Amount | Basis / reason | Confidence | Pair | Rules |
|---|---|---|---|---|---|---|---|---|
| Exclusion | both | L1 occurrence 10-15 | — | 2,500.00 | covered by transfer 10-05 | — | — | IC-LOAN-01 |
| Placement, payment | Birch Hill | L1 occurrence 11-15 | W6 | 2,500.00 | intercompany schedule | firm | P1 | IC-LOAN-01 |
| Placement, receipt | Maple Ridge | L1 occurrence 11-15 | W6 | 2,500.00 | intercompany schedule | firm | P1 | IC-LOAN-01 |
| Placement, payment | Birch Hill | L1 occurrence 12-15 | W10 | 2,500.00 | intercompany schedule | firm | P2 | IC-LOAN-01 |
| Placement, receipt | Maple Ridge | L1 occurrence 12-15 | W10 | 2,500.00 | intercompany schedule | firm | P2 | IC-LOAN-01 |
| Exclusion | Maple Ridge | L2 | — | 8,000.00 | no settlement schedule | — | — | IC-LOAN-02 |
| Exclusion | Birch Hill | L2 | — | 8,000.00 | no settlement schedule | — | — | IC-LOAN-02 |
| Exclusion | Maple Ridge | L3 | — | 50,000.00 | confirmed not settling within the horizon | — | — | IC-LOAN-02 |
| Exclusion | Birch Hill | L3 | — | 50,000.00 | confirmed not settling within the horizon | — | — | IC-LOAN-02 |

| Entity | Decision Item | Subject | Acted on by | Rules |
|---|---|---|---|---|
| Maple Ridge and Birch Hill (one item) | intercompany settlement plan | L2, 8,000.00 | owner | IC-LOAN-02 (Q161) |

- **The 10-05 transfer** is nearer 10-15 than 09-15, so it covers 10-15 (Q149).
- **L3 raises no item:** confirming closed it.
- **Agreement:** at 09-30 each Entity shows 72,000.00 net owed by Birch Hill to Maple Ridge
  (30,000.00 − 8,000.00 + 50,000.00), so IC-AGREE-01 raises nothing.
- **Provisional:** no.

---

## IC-S03 — An unnamed counterparty, and one that isn't connected

Concern: intercompany cash is still forecast when the other side can't be paired, and the owner
is asked to fix what stops the pairing.

**Facts.** Maple Ridge and Birch Hill are connected; Cascade is in the Group but not connected.

| Document | Maple Ridge contact | Classification | Due | Amount |
|---|---|---|---|---|
| Bill LFH-0930 (management fee) | Lee Family Holdings Inc. | intercompany, no counterparty named | 10-20 | 3,000.00 |
| Invoice MR-C12 (landscaping for Cascade's retail yard) | Cascade Garden Supply Inc. | intercompany, names Cascade | 10-28 | 2,400.00 |

**Expected**

| Output | Entity | Document | Week | Amount | Basis | Confidence | Pair | Rules |
|---|---|---|---|---|---|---|---|---|
| Placement, payment | Maple Ridge | LFH-0930 | W2 | 3,000.00 | intercompany document | firm | unpaired | IC-MAP-01, IC-DOC-01 (Q162) |
| Placement, receipt | Maple Ridge | MR-C12 | W4 | 2,400.00 | intercompany document | firm | unpaired | IC-ONESIDED-01, IC-DOC-01 |

| Entity | Decision Item | Subject | Acted on by | Rules |
|---|---|---|---|---|
| Maple Ridge | name intercompany counterparty | contact Lee Family Holdings Inc. | bookkeeper | IC-MAP-01 |
| Maple Ridge | connect intercompany counterparty | Cascade Garden Supply Inc. | owner | IC-ONESIDED-01 |

- **LFH-0930 stays in the forecast** on its own dates, unpaired (Q162). Left out of AP and not
  forecast by IC, a real 3,000.00 payment would vanish.
- **Neither Placement eliminates** in the Group view, and neither balance can be agreed.
- **Provisional:** no.

---

## IC-S04 — Same-currency balances that don't agree

Concern: two Entities that disagree about what one owes the other can't both be right, so the
forecast is Provisional until someone investigates; no journal is guessed.

**Facts.** Maple Ridge and Birch Hill only. No tolerance. Balances at 09-30 and 10-07, confirmed
"not settling within the horizon":

| Maple Ridge account | Amount | Birch Hill account | Amount |
|---|---|---|---|
| Advance to Birch Hill | 12,400.00 | Advance from Maple Ridge | 12,000.00 |

**Expected**

| Output | Entity | Item | Amount | Reason | Rules |
|---|---|---|---|---|---|
| Exclusion | Maple Ridge | Advance to Birch Hill | 12,400.00 | confirmed not settling within the horizon | IC-LOAN-02 |
| Exclusion | Birch Hill | Advance from Maple Ridge | 12,000.00 | confirmed not settling within the horizon | IC-LOAN-02 |

| Entity | Decision Item | Subject | Acted on by | Evidence | Draft Correction | Rules |
|---|---|---|---|---|---|---|
| Maple Ridge and Birch Hill (one item) | intercompany balances disagree | Maple Ridge and Birch Hill, September 2026 | bookkeeper | Maple Ridge 12,400.00 owed to it; Birch Hill 12,000.00 owed; difference 400.00 | — | IC-AGREE-01 |

- **No journal is proposed:** which side is wrong needs investigation (Q130).
- **Provisional:** yes. Reason: IC-AGREE-01 on Maple Ridge and Birch Hill.

---

## IC-S05 — A tolerance, and balances in two currencies

Concern: a Group may accept small differences, and balances in different currencies are expected
to differ after translation, so the difference is explained rather than blocking.

**Facts.** All three Entities. Settings: intercompany tolerance 50.00; conversion rate USD to CAD
1.3600. Balances at 09-30 and 10-07, each confirmed "not settling within the horizon":

| Pair | First side | Second side |
|---|---|---|
| Maple Ridge and Birch Hill | Maple Ridge, Advance to Birch Hill, 12,025.00 | Birch Hill, Advance from Maple Ridge, 12,000.00 |
| Maple Ridge and Cascade | Maple Ridge, Due to Cascade Garden Supply, 6,700.00 | Cascade, Due from Maple Ridge, USD 5,000.00 |

**Expected**

| Output | Entity | Item | Amount (Home Currency) | Reason | Rules |
|---|---|---|---|---|---|
| Exclusion | Maple Ridge | Advance to Birch Hill | 12,025.00 | confirmed not settling within the horizon | IC-LOAN-02 |
| Exclusion | Birch Hill | Advance from Maple Ridge | 12,000.00 | confirmed not settling within the horizon | IC-LOAN-02 |
| Exclusion | Maple Ridge | Due to Cascade Garden Supply | 6,700.00 | confirmed not settling within the horizon | IC-LOAN-02 |
| Exclusion | Cascade | Due from Maple Ridge | USD 5,000.00 | confirmed not settling within the horizon | IC-LOAN-02 |

| Entity | Decision Item | Subject | Acted on by | Evidence | Rules |
|---|---|---|---|---|---|
| Maple Ridge and Cascade (one item) | intercompany currency difference | Maple Ridge and Cascade, September 2026 | accountant | Cascade USD 5,000.00 at 1.3600 = 6,800.00; Maple Ridge 6,700.00; difference 100.00; currency movement is expected | IC-AGREE-02 |

- **Maple Ridge and Birch Hill differ by 25.00,** within the 50.00 tolerance, so IC-AGREE-01
  raises nothing.
- **Provisional:** no. A cross-currency difference never blocks (Q125).

---

## IC-S06 — Recharges across currencies

Concern: a recharge between Entities in different currencies eliminates at the Group rate, and
what doesn't eliminate is shown as currency difference, not hidden.

**Facts.** Maple Ridge and Cascade. Settings: conversion rate USD to CAD 1.3600.

| Document | Issued by | Paid by | Dated | Due | Issuer's side | Payer's side |
|---|---|---|---|---|---|---|
| Invoice CG-2207 | Cascade | Maple Ridge | 10-01 | 10-28 | USD 1,000.00 | bill USD 1,000.00 booked at 1.3500 = CAD 1,350.00 |
| Invoice MR-C20 | Maple Ridge | Cascade | 10-06 | 11-05 | CAD 2,000.00 | bill CAD 2,000.00 booked at 0.7400 = USD 1,480.00 |

**Expected**

| Output | Entity | Document | Week | Amount (Home Currency) | Recorded on Placement | Confidence | Pair | Rules |
|---|---|---|---|---|---|---|---|---|
| Placement, payment | Maple Ridge | CG-2207 | W4 | 1,350.00 | USD 1,000.00 at 1.3500 | firm | P1 | IC-DOC-01 |
| Placement, receipt | Cascade | CG-2207 | W4 | USD 1,000.00 | — | firm | P1 | IC-DOC-01 |
| Placement, receipt | Maple Ridge | MR-C20 | W5 | 2,000.00 | — | firm | P2 | IC-DOC-01 |
| Placement, payment | Cascade | MR-C20 | W5 | USD 1,480.00 | CAD 2,000.00 at 0.7400 | firm | P2 | IC-DOC-01 |

| Group-view line | Week | Amount (CAD) | Working | Rules |
|---|---|---|---|---|
| intercompany currency difference, P1 | W4 | +10.00 | Cascade receives 1,000.00 × 1.3600 = 1,360.00; Maple Ridge pays 1,350.00 | IC-ELIM-01 |
| intercompany currency difference, P2 | W5 | −12.80 | Maple Ridge receives 2,000.00; Cascade pays 1,480.00 × 1.3600 = 2,012.80 | IC-ELIM-01 |

- **No Decision Items, not Provisional.** Nothing was open at 09-30, so there's nothing to agree.

---

## IC-S07 — A sister Entity that could cover a shortfall

Concern: when one Entity runs short and another could lend without falling below zero or its own
buffer, the owner is told, with what such a transfer would mean; nothing is moved.

**Facts.** All three Entities. Settings: conversion rate USD to CAD 1.3600. No intercompany
documents or balances.

| Entity | Opening Cash | Minimum Cash Buffer | Placements from AR and AP |
|---|---|---|---|
| Birch Hill | 5,000.00 | none | W6 −8,000.00; W9 −1,500.00; W12 +6,000.00 |
| Maple Ridge | 30,000.00 | 15,000.00 | W3 −6,000.00; W8 −3,000.00; W10 +5,000.00 |
| Cascade | USD 9,000.00 | USD 5,000.00 | W7 −USD 2,000.00 |

Birch Hill's closing cash: W6 −3,000.00, W9 −4,500.00, W12 1,500.00. Its CASH-SHORT-01 item has
one stretch, W6 to W11, lowest −4,500.00 in W9.

**Expected**

| Entity | Decision Item | Subject | Acted on by | Evidence | Rules |
|---|---|---|---|---|---|
| Birch Hill | consider intercompany funding | Birch Hill's cash shortfall | owner | Maple Ridge could transfer 4,500.00 in W6. Caveats: characterise it as a loan, distribution or capital contribution; a loan needs documented terms, and arm's-length interest may apply; a distribution must pass the lender's solvency test; shareholder-loan rules can tax a loan to an individual owner; lender covenants may restrict it. | IC-FUND-01 (Q163) |

**How the test comes out** (Q163: 4,500.00, the deepest shortfall, moved in W6, the first week
of the shortfall).
- **Maple Ridge qualifies.** Closing cash from W6 falls from 24,000.00 to 19,500.00, from W8 from
  21,000.00 to 16,500.00, and from W10 from 26,000.00 to 21,500.00. Every week stays at or above
  its 15,000.00 buffer.
- **Cascade doesn't.** 4,500.00 is USD 3,308.82 at 1.3600. Cascade's W6 close would be
  USD 5,691.18, above its buffer, but after W7 it would be USD 3,691.18, below USD 5,000.00.
- **Nothing is forecast:** no Placement for the transfer, in either Entity.
- **Provisional:** no.

---

## Coverage

| Rule | Scenarios |
|---|---|
| IC-MAP-01 | S01–S07 (named), S03 (unnamed) |
| IC-ELIM-01 | S01, S02, S06 |
| IC-DOC-01 | S01, S03, S06 |
| IC-DOC-02 | S01 |
| IC-LOAN-01 | S02 |
| IC-LOAN-02 | S02, S04, S05 |
| IC-ONESIDED-01 | S03 |
| IC-AGREE-01 | S02, S04, S05 |
| IC-AGREE-02 | S05 |
| IC-FUND-01 | S07 |

## Questions the Scenarios raised

All three were decided by the owner on 2026-09-14 as picked below and written into the Rules.

- **Q161 — Whose queue an item about a pair joins** (IC-LOAN-02, IC-AGREE-01, IC-AGREE-02,
  CASH-ORDER-01). One item, shown in both Entities' queues with one status (S02, S04, S05).
  IC-DOC-02's item stays with the Entity that must book.
- **Q162 — Intercompany documents with no counterparty named** (IC-MAP-01, IC-DOC-01). IC-DOC-01
  forecasts the visible side on its own dates, unpaired, so the payment isn't dropped (S03).
- **Q163 — The size and week of a suggested transfer** (IC-FUND-01). The deepest shortfall, moved
  in the first week of the shortfall, converted at the conversion rate Setting when currencies
  differ; the lender stays at or above zero and its buffer from that week to the end of the
  horizon (S07).
