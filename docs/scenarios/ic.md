# IC Scenarios

Status: **approved by the owner** (2026-09-14). Built: every Scenario here passes as a test in
`scenarios/tests/ic.rs`, and `tools/mutate.py ic` sweeps this document for figures a test
would not catch. This document is fixed input: when a test disagrees with it, the code is wrong
(`README.md`).

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

| Document | Entity | Counterparty | Dated | Due | Open | Planned payment date |
|---|---|---|---|---|---|---|
| BH-501 | Birch Hill Nursery Ltd. | Maple Ridge Landscaping Ltd. | 10-01 | 10-31 | 4,000.00 | — |
| BH-501 | Maple Ridge Landscaping Ltd. | Birch Hill Nursery Ltd. | 10-01 | 10-31 | 4,000.00 | 10-21 |
| MR-M09 | Maple Ridge Landscaping Ltd. | Birch Hill Nursery Ltd. | 09-15 | 09-30 | 1,500.00 | — |
| MGMT-SEP | Birch Hill Nursery Ltd. | Maple Ridge Landscaping Ltd. | 09-15 | 09-30 | 1,500.00 | — |
| BH-502 | Birch Hill Nursery Ltd. | Maple Ridge Landscaping Ltd. | 10-05 | 11-04 | 2,200.00 | — |

At 09-30, MR-M09 is the only open intercompany document, and both sides show 1,500.00.

**Expected**

| Output | Entity | Document | Week | Amount | Basis | Confidence | Pair | Rules |
|---|---|---|---|---|---|---|---|---|
| Placement, payment | Maple Ridge | BH-501 | W3 (10-21) | 4,000.00 | intercompany document | firm | P1 | IC-MAP-01, IC-DOC-01 |
| Placement, receipt | Birch Hill | BH-501 | W3 (10-21) | 4,000.00 | intercompany document | firm | P1 | IC-MAP-01, IC-DOC-01, IC-ELIM-01 |
| Placement, payment | Maple Ridge | BH-502 | W5 (11-04) | 2,200.00 | intercompany document | firm | P2 | IC-MAP-01, IC-DOC-01 |
| Placement, receipt | Birch Hill | BH-502 | W5 (11-04) | 2,200.00 | intercompany document | firm | P2 | IC-MAP-01, IC-DOC-01, IC-ELIM-01 |
| Placement, payment | Birch Hill | MR-M09 | W1 | 1,500.00 | intercompany document | firm | P3 | IC-MAP-01, IC-DOC-01 |
| Placement, receipt | Maple Ridge | MR-M09 | W1 | 1,500.00 | intercompany document | firm | P3 | IC-MAP-01, IC-DOC-01, IC-ELIM-01 |

| Entity | Decision Item | Subject | Acted on by | Draft Correction | Rules |
|---|---|---|---|---|---|
| Maple Ridge | counterparty has not booked | BH-502 | bookkeeper | Bill from Birch Hill Nursery Ltd., reference BH-502, dated 10-05, due 11-04, 2,200.00 | IC-DOC-02 |

- **BH-501 lands in W3 on both sides,** at the payer's planned date, not Birch Hill's due date.
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

**Facts.** Maple Ridge and Birch Hill only. Balances at 09-30 and 10-07, agreeing on both sides.
Debit-positive (liability balances are stored negative; the table shows the magnitude).

| Entity | Account | Classification | As of | Balance | Settings |
|---|---|---|---|---|---|
| Maple Ridge Landscaping Ltd. | Loan to Birch Hill | intercompany, Birch Hill Nursery Ltd. | 09-30 | 30,000.00 | settlement schedule L1: Birch Hill Nursery Ltd. pays Maple Ridge Landscaping Ltd. 2,500.00 monthly, next date 10-15 |
| Birch Hill Nursery Ltd. | Loan from Maple Ridge | intercompany, Maple Ridge Landscaping Ltd. | 09-30 | −30,000.00 | settlement schedule L1: Birch Hill Nursery Ltd. pays Maple Ridge Landscaping Ltd. 2,500.00 monthly, next date 10-15 |
| Maple Ridge Landscaping Ltd. | Loan to Birch Hill | intercompany, Birch Hill Nursery Ltd. | 10-07 | 27,500.00 | settlement schedule L1: Birch Hill Nursery Ltd. pays Maple Ridge Landscaping Ltd. 2,500.00 monthly, next date 10-15 |
| Birch Hill Nursery Ltd. | Loan from Maple Ridge | intercompany, Maple Ridge Landscaping Ltd. | 10-07 | −27,500.00 | settlement schedule L1: Birch Hill Nursery Ltd. pays Maple Ridge Landscaping Ltd. 2,500.00 monthly, next date 10-15 |
| Maple Ridge Landscaping Ltd. | Due to Birch Hill | intercompany, Birch Hill Nursery Ltd. | 09-30 | −8,000.00 | none |
| Birch Hill Nursery Ltd. | Due from Maple Ridge | intercompany, Maple Ridge Landscaping Ltd. | 09-30 | 8,000.00 | none |
| Maple Ridge Landscaping Ltd. | Due to Birch Hill | intercompany, Birch Hill Nursery Ltd. | 10-07 | −8,000.00 | none |
| Birch Hill Nursery Ltd. | Due from Maple Ridge | intercompany, Maple Ridge Landscaping Ltd. | 10-07 | 8,000.00 | none |
| Maple Ridge Landscaping Ltd. | Long-term Advance to Birch Hill | intercompany, Birch Hill Nursery Ltd. | 09-30 | 50,000.00 | confirmed not settling within the horizon |
| Birch Hill Nursery Ltd. | Long-term Advance from Maple Ridge | intercompany, Maple Ridge Landscaping Ltd. | 09-30 | −50,000.00 | confirmed not settling within the horizon |
| Maple Ridge Landscaping Ltd. | Long-term Advance to Birch Hill | intercompany, Birch Hill Nursery Ltd. | 10-07 | 50,000.00 | confirmed not settling within the horizon |
| Birch Hill Nursery Ltd. | Long-term Advance from Maple Ridge | intercompany, Maple Ridge Landscaping Ltd. | 10-07 | −50,000.00 | confirmed not settling within the horizon |

| Entity | Transaction | Account | Counterparty | Dated | Amount |
|---|---|---|---|---|---|
| Birch Hill Nursery Ltd. | TX-L1 | RBC Business Chequing | Maple Ridge Landscaping Ltd. | 10-05 | 2,500.00 |
| Maple Ridge Landscaping Ltd. | TX-L1-IN | RBC Business Chequing | Birch Hill Nursery Ltd. | 10-05 | 2,500.00 |

**Expected**

| Output | Entity | Item | Week | Amount | Basis / reason | Confidence | Pair | Rules |
|---|---|---|---|---|---|---|---|---|
| Exclusion | both | L1 occurrence 10-15 | — | 2,500.00 | covered by transfer 10-05 | — | — | IC-LOAN-01 |
| Placement, payment | Birch Hill | L1 occurrence 11-15 | W6 | 2,500.00 | intercompany schedule | firm | P1 | IC-LOAN-01, IC-ELIM-01 |
| Placement, receipt | Maple Ridge | L1 occurrence 11-15 | W6 | 2,500.00 | intercompany schedule | firm | P1 | IC-LOAN-01, IC-ELIM-01 |
| Placement, payment | Birch Hill | L1 occurrence 12-15 | W10 | 2,500.00 | intercompany schedule | firm | P2 | IC-LOAN-01, IC-ELIM-01 |
| Placement, receipt | Maple Ridge | L1 occurrence 12-15 | W10 | 2,500.00 | intercompany schedule | firm | P2 | IC-LOAN-01, IC-ELIM-01 |
| Exclusion | Maple Ridge | Due to Birch Hill | — | 8,000.00 | no settlement schedule | — | — | IC-LOAN-02 |
| Exclusion | Birch Hill | Due from Maple Ridge | — | 8,000.00 | no settlement schedule | — | — | IC-LOAN-02 |
| Exclusion | Maple Ridge | Long-term Advance to Birch Hill | — | 50,000.00 | confirmed not settling within the horizon | — | — | IC-LOAN-02 |
| Exclusion | Birch Hill | Long-term Advance from Maple Ridge | — | 50,000.00 | confirmed not settling within the horizon | — | — | IC-LOAN-02 |

| Entity | Decision Item | Subject | Acted on by | Rules |
|---|---|---|---|---|
| Maple Ridge and Birch Hill (one item) | intercompany settlement plan | Due to Birch Hill, 8,000.00 | owner | IC-LOAN-02 (Q161) |

- **The mid-month transfer** is nearer the mid-month occurrence than the prior one, so it covers
  that occurrence (Q149).
- **L3 raises no item:** confirming closed it.
- **Agreement:** at month-end each Entity's intercompany balances net the same way (Facts), so
  IC-AGREE-01 raises nothing.
- **Provisional:** no.

---

## IC-S03 — An unnamed counterparty, and one that isn't connected

Concern: intercompany cash is still forecast when the other side can't be paired, and the owner
is asked to fix what stops the pairing.

**Facts.** Maple Ridge and Birch Hill are connected; Cascade is in the Group but not connected.

| Document | Entity | Counterparty | Classification | Due | Open |
|---|---|---|---|---|---|
| LFH-0930 | Maple Ridge Landscaping Ltd. | Lee Family Holdings Inc. | intercompany, no counterparty named | 10-20 | 3,000.00 |
| MR-C12 | Maple Ridge Landscaping Ltd. | Cascade Garden Supply Inc. | intercompany, names Cascade | 10-28 | 2,400.00 |

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
  forecast by IC, a real payment of that size would vanish.
- **Neither Placement eliminates** in the Group view, and neither balance can be agreed.
- **Provisional:** no.

---

## IC-S04 — Same-currency balances that don't agree

Concern: two Entities that disagree about what one owes the other can't both be right, so the
forecast is Provisional until someone investigates; no journal is guessed.

**Facts.** Maple Ridge and Birch Hill only. No tolerance. Balances at 09-30 and 10-07, confirmed
"not settling within the horizon". Debit-positive (liability balances are stored negative; the
table shows the signed figure).

| Entity | Account | Classification | As of | Balance | Settings |
|---|---|---|---|---|---|
| Maple Ridge Landscaping Ltd. | Advance to Birch Hill | intercompany, Birch Hill Nursery Ltd. | 09-30 | 12,400.00 | confirmed not settling within the horizon |
| Birch Hill Nursery Ltd. | Advance from Maple Ridge | intercompany, Maple Ridge Landscaping Ltd. | 09-30 | −12,000.00 | confirmed not settling within the horizon |
| Maple Ridge Landscaping Ltd. | Advance to Birch Hill | intercompany, Birch Hill Nursery Ltd. | 10-07 | 12,400.00 | confirmed not settling within the horizon |
| Birch Hill Nursery Ltd. | Advance from Maple Ridge | intercompany, Maple Ridge Landscaping Ltd. | 10-07 | −12,000.00 | confirmed not settling within the horizon |

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
1.3600; Reporting Currency CAD. Balances at 09-30 and 10-07, each confirmed "not settling within
the horizon". Debit-positive (liability balances are stored negative; the table shows the signed
figure).

| Entity | Account | Classification | As of | Balance | Settings |
|---|---|---|---|---|---|
| Maple Ridge Landscaping Ltd. | Advance to Birch Hill | intercompany, Birch Hill Nursery Ltd. | 09-30 | 12,025.00 | confirmed not settling within the horizon |
| Birch Hill Nursery Ltd. | Advance from Maple Ridge | intercompany, Maple Ridge Landscaping Ltd. | 09-30 | −12,000.00 | confirmed not settling within the horizon |
| Maple Ridge Landscaping Ltd. | Advance to Birch Hill | intercompany, Birch Hill Nursery Ltd. | 10-07 | 12,025.00 | confirmed not settling within the horizon |
| Birch Hill Nursery Ltd. | Advance from Maple Ridge | intercompany, Maple Ridge Landscaping Ltd. | 10-07 | −12,000.00 | confirmed not settling within the horizon |
| Maple Ridge Landscaping Ltd. | Due to Cascade Garden Supply | intercompany, Cascade Garden Supply Inc. | 09-30 | −6,700.00 | confirmed not settling within the horizon |
| Cascade Garden Supply Inc. | Due from Maple Ridge | intercompany, Maple Ridge Landscaping Ltd. | 09-30 | 5,000.00 | confirmed not settling within the horizon |
| Maple Ridge Landscaping Ltd. | Due to Cascade Garden Supply | intercompany, Cascade Garden Supply Inc. | 10-07 | −6,700.00 | confirmed not settling within the horizon |
| Cascade Garden Supply Inc. | Due from Maple Ridge | intercompany, Maple Ridge Landscaping Ltd. | 10-07 | 5,000.00 | confirmed not settling within the horizon |

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

- **Maple Ridge and Birch Hill** differ within the intercompany tolerance (Facts), so
  IC-AGREE-01 raises nothing.
- **Provisional:** no. A cross-currency difference never blocks (Q125).

---

## IC-S06 — Recharges across currencies

Concern: a recharge between Entities in different currencies eliminates at the Group rate, and
what doesn't eliminate is shown as currency difference, not hidden.

**Facts.** Maple Ridge and Cascade. Settings: conversion rate USD to CAD 1.3600; Reporting
Currency CAD. `Open (document)` is in the document's own currency; `Total` is Home Currency.

| Document | Entity | Counterparty | Dated | Due | Currency | Open (document) | Booked rate | Total |
|---|---|---|---|---|---|---|---|---|
| CG-2207 | Cascade Garden Supply Inc. | Maple Ridge Landscaping Ltd. | 10-01 | 10-28 | USD | 1,000.00 | — | 1,000.00 |
| CG-2207 | Maple Ridge Landscaping Ltd. | Cascade Garden Supply Inc. | 10-01 | 10-28 | USD | 1,000.00 | 1.3500 | 1,350.00 |
| MR-C20 | Maple Ridge Landscaping Ltd. | Cascade Garden Supply Inc. | 10-06 | 11-05 | CAD | 2,000.00 | — | 2,000.00 |
| MR-C20 | Cascade Garden Supply Inc. | Maple Ridge Landscaping Ltd. | 10-06 | 11-05 | CAD | 2,000.00 | 0.7400 | 1,480.00 |

**Expected**

| Output | Entity | Document | Week | Amount (Home Currency) | Recorded on Placement | Confidence | Pair | Rules |
|---|---|---|---|---|---|---|---|---|
| Placement, payment | Maple Ridge | CG-2207 | W4 | 1,350.00 | USD 1,000.00 at 1.3500 | firm | P1 | IC-MAP-01, IC-DOC-01 |
| Placement, receipt | Cascade | CG-2207 | W4 | USD 1,000.00 | — | firm | P1 | IC-MAP-01, IC-DOC-01, IC-ELIM-01 |
| Placement, receipt | Maple Ridge | MR-C20 | W5 | 2,000.00 | — | firm | P2 | IC-MAP-01, IC-DOC-01, IC-ELIM-01 |
| Placement, payment | Cascade | MR-C20 | W5 | USD 1,480.00 | CAD 2,000.00 at 0.7400 | firm | P2 | IC-MAP-01, IC-DOC-01 |

| Group-view line | Week | Amount (CAD) | Working | Rules |
|---|---|---|---|---|
| intercompany currency difference, P1 | W4 | +10.00 | Cascade receives 1,000.00 × 1.3600 = 1,360.00; Maple Ridge pays 1,350.00 | IC-ELIM-01 |
| intercompany currency difference, P2 | W5 | −12.80 | Maple Ridge receives 2,000.00; Cascade pays 1,480.00 × 1.3600 = 2,012.80 | IC-ELIM-01 |

- **No Decision Items, not Provisional.** Nothing was open at the last completed month-end, so
  there's nothing to agree.

---

## IC-S07 — A sister Entity that could cover a shortfall

Concern: when one Entity runs short and another could lend without falling below zero or its own
buffer, the owner is told, with what such a transfer would mean; nothing is moved.

**Facts.** All three Entities. Settings: conversion rate USD to CAD 1.3600; Reporting Currency CAD;
Minimum Cash Buffer Maple Ridge 15,000.00, Cascade USD 5,000.00; Birch Hill none. No intercompany
documents or balances. Placements from AR and AP land on the due dates below.

| Entity | Account | Classification | As of | Balance |
|---|---|---|---|---|
| Birch Hill Nursery Ltd. | RBC Business Chequing | bank | 10-07 | 5,000.00 |
| Maple Ridge Landscaping Ltd. | RBC Business Chequing | bank | 10-07 | 30,000.00 |
| Cascade Garden Supply Inc. | Chase Business Checking | bank | 10-07 | 9,000.00 |

| Document | Entity | Counterparty | Due | Open |
|---|---|---|---|---|
| BH-W6 | Birch Hill Nursery Ltd. | Western Turf Farms | 11-12 | 8,000.00 |
| BH-W9 | Birch Hill Nursery Ltd. | Brandt Tractor Ltd. | 12-03 | 1,500.00 |
| BH-W12 | Birch Hill Nursery Ltd. | Harbourview Strata Corp. | 12-24 | 6,000.00 |
| MR-W3 | Maple Ridge Landscaping Ltd. | Nutrien Ag Solutions | 10-22 | 6,000.00 |
| MR-W8 | Maple Ridge Landscaping Ltd. | Telus Business | 11-26 | 3,000.00 |
| MR-W10 | Maple Ridge Landscaping Ltd. | Oakview Senior Living | 12-10 | 5,000.00 |
| CG-W7 | Cascade Garden Supply Inc. | Pacific Growers Supply LLC | 11-19 | 2,000.00 |

Birch Hill's closing cash: W6 −3,000.00, W9 −4,500.00, W12 1,500.00. Its CASH-SHORT-01 item has
one stretch, W6 to W11, lowest −4,500.00 in W9.

**Expected**

| Entity | Opening Cash | Week closes | Low Point |
|---|---|---|---|
| Birch Hill | 5,000.00 | W6 −3,000.00; W9 −4,500.00; W12 1,500.00 | −4,500.00 (W9) |
| Maple Ridge | 30,000.00 | W3 24,000.00; W8 21,000.00; W10 26,000.00 | 21,000.00 (W8) |
| Cascade | USD 9,000.00 | W7 USD 7,000.00 | USD 7,000.00 (W7) |

| Entity | Decision Item | Subject | Acted on by | Evidence | Rules |
|---|---|---|---|---|---|
| Birch Hill | consider intercompany funding | Birch Hill's cash shortfall | owner | Maple Ridge could transfer 4,500.00 in W6. Caveats: characterise it as a loan, distribution or capital contribution; a loan needs documented terms, and arm's-length interest may apply; a distribution must pass the lender's solvency test; shareholder-loan rules can tax a loan to an individual owner; lender covenants may restrict it. | IC-FUND-01 (Q163) |

**How the test comes out** (Q163: deepest shortfall, moved in the first week of the shortfall;
Evidence names the helper and amount).
- **Maple Ridge qualifies.** After the suggested transfer, every week of its closes stays at or
  above its buffer (Entity table is before the transfer; buffer is in Facts).
- **Cascade doesn't.** Converted at the Group rate, the same transfer would leave Cascade below
  its buffer after its next payment week.
- **Nothing is forecast:** no Placement for the transfer, in either Entity.
- **Provisional:** no.

---

## Coverage

| Rule | Scenarios |
|---|---|
| IC-MAP-01 | S01, S03, S06; S03 also covers unnamed (Q162) |
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
