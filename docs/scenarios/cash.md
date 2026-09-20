# CASH Scenarios

Status: **approved by the owner** (2026-09-14). Being built: each Scenario here becomes a test in
`scenarios/tests/cash.rs` as the build reaches it, and `tools/mutate.py cash` sweeps this document
for figures a test would not catch. This document is fixed input: when a test disagrees with it,
the code is wrong (`README.md`). CASH-S03 is the exception, built at the GAP build because it
needs GAP-TAX-01 (Q275); until then it is the one entry in `PENDING`
(`scenarios/tests/coverage.rs`).

Each Scenario is a named general accounting concern, described as Canonical Facts and Settings,
with the outputs the CASH Rules must produce (ADR-0010, ADR-0016). Together these exercise every
Rule in `docs/rules/cash.md`.

## Conventions

The conventions in `docs/scenarios/ar.md` apply: run date Wednesday 2026-10-07, a 13-week
horizon (W1 is 10-07 to 10-13, W13 is 12-30 to 2027-01-05), Maple Ridge Landscaping Ltd. in
CAD unless stated, an empty Entity at the start of each Scenario, and exhaustive expected outputs
for this Family. CASH also needs these:

- **Inputs from other Families** are the Placements their Rules make from the facts. They are
  listed so the roll-forward can be checked, and are asserted in those Families' own Scenarios,
  not here. Every document here has no payment history, planned or expected date, so AR and AP
  place it at its due date, Confidence firm, unless stated.
- **The weeks table lists only weeks with Placements.** An unlisted week opens and closes at the
  previous week's closing cash and has no Confidence shares (Q153).
- **The queue lists every Decision Item from every Family,** because CASH-ORDER-01 orders them all.
  It is per Entity (Q155). Only CASH's own items and the ordering are asserted here.

---

## CASH-S01 — What Opening Cash includes

Concern: the forecast starts from money the Entity can spend today, so overdrawn accounts count,
and restricted money, money not yet banked and card debt don't.

**Facts.** The Entity uses Xero, which types the card as a bank account (`CREDITCARD`). The
Holdback Trust Account is mapped restricted. Scotiabank Operating is overdrawn and the card
balance is owed, both negative under `docs/facts.md`. Settings: card payment day for RBC Visa
Business, the 22nd.

| Account | Classification | As of | Balance |
|---|---|---|---|
| RBC Business Chequing | bank | 10-07 | 18,400.00 |
| RBC Business Savings | bank | 10-07 | 25,000.00 |
| Scotiabank Operating | bank | 10-07 | −1,250.00 |
| Holdback Trust Account | restricted | 10-07 | 12,000.00 |
| Customer Receipts Clearing | clearing | 10-07 | 2,300.00 |
| RBC Visa Business | credit card | 10-07 | −3,480.00 |

**Inputs from other Families**

| Placement | Week | Amount | Rules |
|---|---|---|---|
| RBC Visa Business | W3 | −3,480.00 | GAP-CARD-01 |

**Expected**

| Output | Item | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|
| Opening Cash | RBC Business Chequing, RBC Business Savings, Scotiabank Operating | — | 42,150.00 | book balance at 10-07 | firm | CASH-OPEN-01 |
| Exclusion | Holdback Trust Account | — | 12,000.00 | restricted | — | CASH-OPEN-02 |
| Placement | Customer Receipts Clearing | W1 | 2,300.00 | clearing balance | firm | CASH-OPEN-03 |
| Exclusion | RBC Visa Business | — | 3,480.00 | credit card, see GAP | — | CASH-OPEN-04 |

| Week | Opens | Receipts | Payments | Closes | Firm | Estimated |
|---|---|---|---|---|---|---|
| W1 | 42,150.00 | 2,300.00 | — | 44,450.00 | 100% | 0% |
| W3 | 44,450.00 | — | 3,480.00 | 40,970.00 | 100% | 0% |

- **Weeks:** 13 buckets, W1 to W13 (CASH-WEEK-01). W2 closes at 44,450.00; W4 to W13 at
  40,970.00 (CASH-ROLL-01).
- **Low Point:** 40,970.00 in W3, the first of the weeks at that level (CASH-LOW-01, Q152).
- **Without CASH-OPEN-04,** Opening Cash would be 38,670.00, netting card debt against cash.
- **No Headroom, no Decision Items, not Provisional.**

---

## CASH-S02 — Undrawn credit is Headroom, never cash

Concern: an undrawn line tells the owner what they could borrow, but counting it as cash hides a
crunch.

**Facts.** A drawn credit line is money owed, so its balance is negative (`docs/facts.md`). No
account is mapped to the RBC Royal Line of Credit, and the TD Equipment Line has no limit set.

| Account | Classification | As of | Balance |
|---|---|---|---|
| RBC Business Chequing | bank | 10-07 | 9,000.00 |
| BDC Operating Line | credit line | 10-07 | −15,000.00 |
| TD Equipment Line | credit line | 10-07 | −4,000.00 |

Settings, credit line limits:

| Credit line | Limit |
|---|---|
| BDC Operating Line | 50,000.00 |
| RBC Royal Line of Credit | 20,000.00 |

Settings, scheduled obligations for the interest on each drawn line:

| Obligation | Payee | Amount | Every | Starting |
|---|---|---|---|---|
| SO1 | BDC Operating Line | 110.00 | month | 10-25 |
| SO2 | TD Equipment Line | 350.00 | month | 10-20 |

**Inputs from other Families**

| Placement | Week | Amount | Rules |
|---|---|---|---|
| SO2 occurrence 10-20 | W2 | −350.00 | GAP-SCHED-01 |
| SO1 occurrence 10-25 | W3 | −110.00 | GAP-SCHED-01 |
| SO2 occurrence 11-20 | W7 | −350.00 | GAP-SCHED-01 |
| SO1 occurrence 11-25 | W8 | −110.00 | GAP-SCHED-01 |
| SO2 occurrence 12-20 | W11 | −350.00 | GAP-SCHED-01 |
| SO1 occurrence 12-25 | W12 | −110.00 | GAP-SCHED-01 |

**Expected**

| Output | Item | Amount | Confidence | Rules |
|---|---|---|---|---|
| Opening Cash | RBC Business Chequing | 9,000.00 | firm | CASH-OPEN-01 |
| Headroom | BDC Operating Line | 35,000.00 | — | CASH-HEAD-01 |

| Week | Opens | Receipts | Payments | Closes | Firm | Estimated |
|---|---|---|---|---|---|---|
| W2 | 9,000.00 | — | 350.00 | 8,650.00 | 100% | 0% |
| W3 | 8,650.00 | — | 110.00 | 8,540.00 | 100% | 0% |
| W7 | 8,540.00 | — | 350.00 | 8,190.00 | 100% | 0% |
| W8 | 8,190.00 | — | 110.00 | 8,080.00 | 100% | 0% |
| W11 | 8,080.00 | — | 350.00 | 7,730.00 | 100% | 0% |
| W12 | 7,730.00 | — | 110.00 | 7,620.00 | 100% | 0% |

- **Low Point:** 7,620.00 in W12.
- **No Headroom for TD Equipment Line** (no limit) or RBC Royal Line of Credit (no account). No
  drawn amount is added to cash.

| # | Class | Decision Item | Subject | Acted on by | Due | Rules |
|---|---|---|---|---|---|---|
| 1 | action | map credit line account | RBC Royal Line of Credit | owner | — | CASH-HEAD-01 |

- **Provisional:** no.

---

## CASH-S03 — Two stretches below zero make one shortfall item

Concern: running out of cash is always raised, once per Entity however many weeks it lasts, with
what could cover it and which trust payments fall inside.

**Facts.**

| Account | Classification | As of | Balance |
|---|---|---|---|
| RBC Business Chequing | bank | 10-07 | 6,000.00 |
| BDC Operating Line | credit line | 10-07 | 0.00 |

Settings, credit line limits:

| Credit line | Limit |
|---|---|
| BDC Operating Line | 25,000.00 |

| Document | Counterparty | Due | Open |
|---|---|---|---|
| BT-3301 | Brandt Tractor Ltd. | 10-16 | 9,500.00 |
| ST-3302 | Stihl Canada | 12-04 | 12,000.00 |
| INV-3101 | Harbourview Strata Corp. | 10-28 | 7,000.00 |
| INV-3102 | Alder Creek Homes | 11-06 | 5,000.00 |
| INV-3103 | Kingsway Medical Clinic | 12-18 | 9,000.00 |

GST/HST: reporting period quarterly (set), Canada federal. "GST/HST Payable" (sales-tax
liability, government trust) balance at 09-30 is 4,200.00, with no payment since. The April to
June remittance, 3,900.00, was paid 07-31. The ledger lock date is 09-30.

**Inputs from other Families**

| Placement | Week | Amount | Rules |
|---|---|---|---|
| BT-3301 | W2 | −9,500.00 | AP-TIME-02 |
| INV-3101 | W4 | +7,000.00 | AR-TIME-02 |
| GST/HST remittance for July to September, due 10-31, trust-marked | W4 | −4,200.00 | GAP-TAX-01 |
| INV-3102 | W5 | +5,000.00 | AR-TIME-02 |
| ST-3302 | W9 | −12,000.00 | AP-TIME-02 |
| INV-3103 | W11 | +9,000.00 | AR-TIME-02 |

The October to December remittance falls due after the horizon.

**Expected**

| Output | Item | Amount | Confidence | Rules |
|---|---|---|---|---|
| Opening Cash | RBC Business Chequing | 6,000.00 | firm | CASH-OPEN-01 |
| Headroom | BDC Operating Line | 25,000.00 | — | CASH-HEAD-01 |

| Week | Opens | Receipts | Payments | Closes | Firm | Estimated |
|---|---|---|---|---|---|---|
| W2 | 6,000.00 | — | 9,500.00 | −3,500.00 | 100% | 0% |
| W4 | −3,500.00 | 7,000.00 | 4,200.00 | −700.00 | 100% | 0% |
| W5 | −700.00 | 5,000.00 | — | 4,300.00 | 100% | 0% |
| W9 | 4,300.00 | — | 12,000.00 | −7,700.00 | 100% | 0% |
| W11 | −7,700.00 | 9,000.00 | — | 1,300.00 | 100% | 0% |

- **Low Point:** −7,700.00 in W9.

| # | Class | Decision Item | Subject | Acted on by | Due | Evidence | Rules |
|---|---|---|---|---|---|---|---|
| 1 | critical | cash shortfall | Maple Ridge Landscaping Ltd. | owner | 10-14 | Stretch W2–W4, lowest −3,500.00 (W2), trust obligations inside: GST/HST remittance 4,200.00 (W4). Stretch W9–W10, lowest −7,700.00 (W9), no trust obligations. Headroom 25,000.00. | CASH-SHORT-01, CASH-ORDER-01 |

- **One item, not two:** both stretches are evidence on the same "cash shortfall" item.
- **Not Provisional,** so the item carries no Provisional note. No other Family raises an item:
  nothing is overdue, and the lock date covers the last closed tax period (ended 06-30).

---

## CASH-S04 — Below the buffer, and below zero

Concern: the owner's buffer is watched only when set, and weeks below zero are reported once, as a
shortfall, not again as below buffer.

**Facts.** Settings: Minimum Cash Buffer 10,000.00. No credit line.

| Account | Classification | As of | Balance |
|---|---|---|---|
| RBC Business Chequing | bank | 10-07 | 14,000.00 |

| Document | Counterparty | Due | Open |
|---|---|---|---|
| Bill WT-4401 | Western Turf Farms | 10-15 | 6,000.00 |
| Bill BT-4402 | Brandt Tractor Ltd. | 11-12 | 16,000.00 |
| Invoice INV-4101 | Harbourview Strata Corp. | 10-23 | 5,000.00 |
| Invoice INV-4102 | Oakview Senior Living | 11-20 | 8,000.00 |
| Invoice INV-4103 | Cedar Point Property Mgmt. | 11-27 | 7,000.00 |

**Expected**

| Output | Item | Amount | Confidence | Rules |
|---|---|---|---|---|
| Opening Cash | RBC Business Chequing | 14,000.00 | firm | CASH-OPEN-01 |

| Week | Opens | Receipts | Payments | Closes | Firm | Estimated |
|---|---|---|---|---|---|---|
| W2 | 14,000.00 | — | 6,000.00 | 8,000.00 | 100% | 0% |
| W3 | 8,000.00 | 5,000.00 | — | 13,000.00 | 100% | 0% |
| W6 | 13,000.00 | — | 16,000.00 | −3,000.00 | 100% | 0% |
| W7 | −3,000.00 | 8,000.00 | — | 5,000.00 | 100% | 0% |
| W8 | 5,000.00 | 7,000.00 | — | 12,000.00 | 100% | 0% |

- **Low Point:** −3,000.00 in W6.

| # | Class | Decision Item | Subject | Acted on by | Due | Evidence | Rules |
|---|---|---|---|---|---|---|---|
| 1 | critical | cash shortfall | Maple Ridge Landscaping Ltd. | owner | 11-11 | Stretch W6, lowest −3,000.00. No Headroom. | CASH-SHORT-01 |
| 2 | critical | below buffer | Maple Ridge Landscaping Ltd. | owner | 10-14 | Buffer 10,000.00. Stretch W2, lowest 8,000.00. Stretch W7, lowest 5,000.00. No Headroom. | CASH-BUFFER-01, CASH-ORDER-01 (Q154) |

- **W6 is not a below-buffer week:** it is below zero, so it belongs to the shortfall item.
- **With no buffer set,** CASH-BUFFER-01 does nothing: see CASH-S03, where weeks closing between zero
  and 10,000.00 raise no item.
- **Provisional:** no.

---

## CASH-S05 — Owner-entered scheduled receipts, one covered

Concern: money the owner expects from outside the ledger (funds introduced, loan proceeds) is
forecast when entered, and a receipt already banked is not forecast again.

**Facts.** The opening balance includes both October receipts. Jordan Lee is a shareholder and SR1
is funds introduced; SR2 is loan proceeds.

| Account | Classification | As of | Balance |
|---|---|---|---|
| RBC Business Chequing | bank | 10-07 | 31,000.00 |

The two receipts already banked:

| Transaction | Account | Payer | Dated | Amount |
|---|---|---|---|---|
| BANK-5101 | RBC Business Chequing | Jordan Lee | 10-02 | 3,000.00 |
| BANK-5102 | RBC Business Chequing | Farm Credit Canada | 10-06 | 25,000.00 |

Settings, scheduled receipts entered 09-20:

| Receipt | Payer | Amount | Every | Starting | End date |
|---|---|---|---|---|---|
| SR1 | Jordan Lee | 3,000.00 | month | 10-01 | — |
| SR2 | Farm Credit Canada | 25,000.00 | month | 10-15 | 11-15 |

**Expected**

| Output | Item | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|
| Opening Cash | RBC Business Chequing | — | 31,000.00 | book balance at 10-07 | firm | CASH-OPEN-01 |
| Exclusion | SR2 occurrence 10-15 | — | 25,000.00 | covered by bank receipt 10-06 | — | CASH-SCHED-01 |
| Placement | SR1 occurrence 11-01 | W4 | 3,000.00 | scheduled receipt | firm | CASH-SCHED-01 |
| Placement | SR2 occurrence 11-15 | W6 | 25,000.00 | scheduled receipt | firm | CASH-SCHED-01 |
| Placement | SR1 occurrence 12-01 | W8 | 3,000.00 | scheduled receipt | firm | CASH-SCHED-01 |
| Placement | SR1 occurrence 2027-01-01 | W13 | 3,000.00 | scheduled receipt | firm | CASH-SCHED-01 |

| Week | Opens | Receipts | Payments | Closes | Firm | Estimated |
|---|---|---|---|---|---|---|
| W4 | 31,000.00 | 3,000.00 | — | 34,000.00 | 100% | 0% |
| W6 | 34,000.00 | 25,000.00 | — | 59,000.00 | 100% | 0% |
| W8 | 59,000.00 | 3,000.00 | — | 62,000.00 | 100% | 0% |
| W13 | 62,000.00 | 3,000.00 | — | 65,000.00 | 100% | 0% |

- **Coverage (Q149):**
  - **The 10-02 receipt** is nearer SR1's 10-01 occurrence than its 11-01 one, so it covers
    10-01, which is before the run date. The 11-01 occurrence stays. Under the wording Q149
    replaced ("between the previous occurrence and this one") it would have covered 11-01, and
    3,000.00 would have gone missing.
  - **The 10-06 receipt** is nearer 10-15 than 09-15, SR2's frequency extended before its first
    date, so it covers 10-15.
- **SR1's 10-01 occurrence** falls before the run date, so it is neither placed nor shown.
- **Low Point:** 31,000.00 in W1.
- **No Decision Items, not Provisional.**

---

## CASH-S06 — No horizon set

Concern: the length of a forecast has no general default, so it is asked for, not assumed.

**Facts.** The Group has no horizon set.

| Account | Classification | As of | Balance |
|---|---|---|---|
| RBC Business Chequing | bank | 10-07 | 12,000.00 |

| Document | Counterparty | Due | Open |
|---|---|---|---|
| INV-6101 | Harbourview Strata Corp. | 10-20 | 4,000.00 |

**Expected**

| Output | Item | Amount | Confidence | Rules |
|---|---|---|---|---|
| Opening Cash | RBC Business Chequing | 12,000.00 | firm | CASH-OPEN-01 |

| # | Class | Decision Item | Subject | Acted on by | Due | Rules |
|---|---|---|---|---|---|---|
| 1 | action | set horizon | Group | owner | — | CASH-WEEK-01 |

- **No weeks,** so no roll-forward, Confidence, Low Point, shortfall or buffer check. INV-6101 is
  not bucketed.
- **Provisional:** no, as CASH-WEEK-01 marks.

---

## CASH-S07 — Confidence by size, and what makes a run Provisional

Concern: a week's Confidence shows how much of its money rests on estimates, and a run is
Provisional because evidence is missing, never because something is estimated.

**Facts.** No card payment day is set for RBC Visa Business. Its balance is owed, so it is
negative (`docs/facts.md`).

| Account | Classification | As of | Balance |
|---|---|---|---|
| RBC Business Chequing | bank | 10-07 | 20,000.00 |
| RBC Visa Business | credit card | 10-07 | −2,500.00 |

| Document | Counterparty | Status | Due | Open |
|---|---|---|---|---|
| INV-7101 | Harbourview Strata Corp. | posted | 10-16 | 6,000.00 |
| INV-7102 | Alder Creek Homes | posted | 09-10 | 1,800.00 |
| WT-7401 | Western Turf Farms | draft | 10-19 | 4,000.00 |

**Inputs from other Families**

| Output | Item | Week | Amount | Confidence | Rules |
|---|---|---|---|---|---|
| Placement | INV-7101 | W2 | +6,000.00 | firm | AR-TIME-02 |
| Placement | WT-7401 | W2 | −4,000.00 | estimated | AP-OPEN-02 |
| Exclusion | INV-7102, no timing evidence | — | 1,800.00 | — | AR-TIME-04 |
| Exclusion | RBC Visa Business, no payment day | — | 2,500.00 | — | GAP-CARD-01 |

**Expected**

| Output | Item | Amount | Confidence | Reason | Rules |
|---|---|---|---|---|---|
| Opening Cash | RBC Business Chequing | 20,000.00 | firm | — | CASH-OPEN-01 |
| Exclusion | RBC Visa Business | 2,500.00 | — | credit card, see GAP | CASH-OPEN-04 |

| Week | Opens | Receipts | Payments | Closes | Firm | Estimated |
|---|---|---|---|---|---|---|
| W2 | 20,000.00 | 6,000.00 | 4,000.00 | 22,000.00 | 60% | 40% |

- **Confidence counts size:** 6,000.00 firm and 4,000.00 estimated, out of 10,000.00 moved. By net
  amount it would read as 2,000.00 of movement, hiding the estimate (CASH-CONF-01).
- **Low Point:** 20,000.00 in W1.

| # | Class | Decision Item | Subject | Acted on by | Due | Rules |
|---|---|---|---|---|---|---|
| 1 | blocking | set card payment day | RBC Visa Business (2,500.00) | owner | — | GAP-CARD-01 |
| 2 | blocking | set expected date | INV-7102 (1,800.00) | owner | — | AR-TIME-04 |
| 3 | action | approve or delete bill | WT-7401 (4,000.00) | bookkeeper | — | AP-OPEN-02 |
| 4 | action | collect | INV-7102 (1,800.00) | owner | — | AR-COLLECT-01 |

- **Ordering (CASH-ORDER-01):**
  - **Blocking:** neither item's cash lands in the horizon, so the larger total goes first.
  - **Action:** WT-7401's cash lands in W2, after the Low Point week, so it counts nothing there
    either, and the larger total again goes first.
- **Provisional:** yes (CASH-PROV-01). Reasons: AR-TIME-04 on INV-7102; GAP-CARD-01 on RBC Visa
  Business. WT-7401's estimated Placement is not a reason.

---

## CASH-S08 — The order of the queue

Concern: the owner sees first what could run out the business's cash, then what is owed to a
government, then vendors they can't lose, then what matters before the Low Point.

**Facts.** No credit line. RG-0815 is August's source deductions. Settings: Nutrien Ag Solutions
marked as a critical vendor. The contact Receiver General for Canada is classified government
trust.

| Account | Classification | As of | Balance |
|---|---|---|---|
| RBC Business Chequing | bank | 10-07 | 8,000.00 |

| Document | Counterparty | Status | Due | Open |
|---|---|---|---|---|
| RG-0815 | Receiver General for Canada | posted | 09-15 | 2,600.00 |
| NAS-5512 | Nutrien Ag Solutions | posted | 09-30 | 1,200.00 |
| HD-88410 | Home Depot Pro | posted | 10-02 | 3,100.00 |
| TEL-1009 | Telus Business | posted | 10-05 | 450.00 |
| BT-2210 | Brandt Tractor Ltd. | posted | 10-23 | 6,000.00 |
| WT-0417 | Western Turf Farms | draft | 12-01 | 5,000.00 |
| INV-8101 | Harbourview Strata Corp. | posted | 11-20 | 12,000.00 |

**Inputs from other Families**

| Placement | Week | Amount | Confidence | Rules |
|---|---|---|---|---|
| RG-0815 | W1 | −2,600.00 | firm | AP-TIME-03 |
| NAS-5512 | W1 | −1,200.00 | firm | AP-TIME-03 |
| HD-88410 | W1 | −3,100.00 | firm | AP-TIME-03 |
| TEL-1009 | W1 | −450.00 | firm | AP-TIME-03 |
| BT-2210 | W3 | −6,000.00 | firm | AP-TIME-02 |
| INV-8101 | W7 | +12,000.00 | firm | AR-TIME-02 |
| WT-0417 | W8 | −5,000.00 | estimated | AP-OPEN-02 |

**Expected**

| Output | Item | Amount | Confidence | Rules |
|---|---|---|---|---|
| Opening Cash | RBC Business Chequing | 8,000.00 | firm | CASH-OPEN-01 |

| Week | Opens | Receipts | Payments | Closes | Firm | Estimated |
|---|---|---|---|---|---|---|
| W1 | 8,000.00 | — | 7,350.00 | 650.00 | 100% | 0% |
| W3 | 650.00 | — | 6,000.00 | −5,350.00 | 100% | 0% |
| W7 | −5,350.00 | 12,000.00 | — | 6,650.00 | 100% | 0% |
| W8 | 6,650.00 | — | 5,000.00 | 1,650.00 | 0% | 100% |

- **Low Point:** −5,350.00 in W3.

| # | Class | Decision Item | Subject | Acted on by | Due | Why here | Rules |
|---|---|---|---|---|---|---|---|
| 1 | critical | cash shortfall | Maple Ridge Landscaping Ltd. | owner | 10-21 | Stretch W3–W6, lowest −5,350.00; no trust obligations inside; no Headroom | CASH-SHORT-01 |
| 2 | action | overdue bill | RG-0815 | owner | — | held in trust for a government | AP-TIME-03 |
| 3 | action | overdue bill | NAS-5512 | owner | — | critical vendor | AP-TIME-03 |
| 4 | action | overdue bill | HD-88410 | owner | — | 3,100.00 lands on or before W3 | AP-TIME-03 |
| 5 | action | overdue bill | TEL-1009 | owner | — | 450.00 lands on or before W3 | AP-TIME-03 |
| 6 | action | approve or delete bill | WT-0417 | bookkeeper | — | nothing lands by W3; total 5,000.00 | AP-OPEN-02 |

- **Cash before the Low Point outranks size:** WT-0417 is the largest action item but comes last.
- **Only the shortfall has a due date** (Q93).
- **Provisional:** no.

---

## CASH-S09 — The Group view: a labelled total, intercompany eliminated

Concern: Group totals add up separate businesses without pretending their cash is shared,
intercompany payments cancel, and a currency difference on them stays visible.

**Facts.** Group Lee Family Holdings, Reporting Currency CAD, conversion rate USD to CAD 1.3600.
No intercompany balances at 09-30. Each intercompany contact names its counterparty Entity.

| Entity | Home Currency | Account | Classification | As of | Balance |
|---|---|---|---|---|---|
| Maple Ridge Landscaping Ltd. | CAD | RBC Business Chequing | bank | 10-07 | 30,000.00 |
| Birch Hill Nursery Ltd. | CAD | RBC Business Chequing | bank | 10-07 | 5,000.00 |
| Cascade Garden Supply Inc. | USD | Chase Business Checking | bank | 10-07 | 7,000.00 |

BH-501 and MR-9001 are one intercompany pair, CG-2207 and MR-9002 the other. `Total` is in the
Entity's Home Currency; `Open (document)` is in the document's own currency.

| Document | Entity | Counterparty | Dated | Due | Currency | Open (document) | Booked rate | Total |
|---|---|---|---|---|---|---|---|---|
| BH-501 | Birch Hill Nursery Ltd. | Maple Ridge Landscaping Ltd. | 10-01 | 10-21 | CAD | 4,000.00 | — | 4,000.00 |
| MR-9001 | Maple Ridge Landscaping Ltd. | Birch Hill Nursery Ltd. | 10-01 | 10-21 | CAD | 4,000.00 | — | 4,000.00 |
| CG-2207 | Cascade Garden Supply Inc. | Maple Ridge Landscaping Ltd. | 10-01 | 10-28 | USD | 1,000.00 | — | 1,000.00 |
| MR-9002 | Maple Ridge Landscaping Ltd. | Cascade Garden Supply Inc. | 10-01 | 10-28 | USD | 1,000.00 | 1.3500 | 1,350.00 |
| INV-9101 | Maple Ridge Landscaping Ltd. | Kingsway Medical Clinic | 10-07 | 11-06 | CAD | 8,000.00 | — | 8,000.00 |
| WT-9401 | Birch Hill Nursery Ltd. | Western Turf Farms | 10-07 | 11-13 | CAD | 12,000.00 | — | 12,000.00 |
| PG-9402 | Cascade Garden Supply Inc. | Pacific Growers Supply LLC | 10-07 | 12-04 | USD | 2,500.00 | — | 2,500.00 |

**Inputs from other Families**

| Entity | Placement | Week | Amount (Home Currency) | Pair | Rules |
|---|---|---|---|---|---|
| Maple Ridge | BH-501 | W3 | −4,000.00 | P1 | IC-DOC-01 |
| Birch Hill | BH-501 | W3 | +4,000.00 | P1 | IC-DOC-01 |
| Maple Ridge | CG-2207 | W4 | −1,350.00 | P2 | IC-DOC-01 |
| Cascade | CG-2207 | W4 | +USD 1,000.00 | P2 | IC-DOC-01 |
| Maple Ridge | INV-9101 | W5 | +8,000.00 | — | AR-TIME-02 |
| Birch Hill | WT-9401 | W6 | −12,000.00 | — | AP-TIME-02 |
| Cascade | PG-9402 | W9 | −USD 2,500.00 | — | AP-TIME-02 |

**Expected: each Entity**

| Entity | Opening Cash | Week closes | Low Point |
|---|---|---|---|
| Maple Ridge | 30,000.00 | W3 26,000.00; W4 24,650.00; W5 32,650.00 | 24,650.00 (W4) |
| Birch Hill | 5,000.00 | W3 9,000.00; W6 −3,000.00 | −3,000.00 (W6) |
| Cascade | USD 7,000.00 | W4 USD 8,000.00; W9 USD 5,500.00 | USD 5,500.00 (W9) |

All Placements are firm, so every week with movement is 100% firm.

**Expected: Group view (CAD)**

| Week | Opens | Receipts | Payments | Intercompany currency difference | Closes |
|---|---|---|---|---|---|
| W3 | 44,520.00 | — (P1 eliminated) | — (P1 eliminated) | — | 44,520.00 |
| W4 | 44,520.00 | — (P2 eliminated) | — (P2 eliminated) | +10.00 | 44,530.00 |
| W5 | 44,530.00 | 8,000.00 | — | — | 52,530.00 |
| W6 | 52,530.00 | — | 12,000.00 | — | 40,530.00 |
| W9 | 40,530.00 | — | 3,400.00 | — | 37,130.00 |

- **Label:** a total of separate Entities' cash, which does not mean cash can move between them
  (CASH-GROUP-01).
- **Opening:** 30,000.00 + 5,000.00 + USD 7,000.00 × 1.3600 (9,520.00).
- **P2's difference:** Cascade receives USD 1,000.00 = 1,360.00 at the Group rate; Maple Ridge
  pays 1,350.00 at its booked rate. The 10.00 that doesn't eliminate is shown.
- **The Group never goes below zero, and Birch Hill still does.** The Group total raises nothing;
  Birch Hill's shortfall is raised on Birch Hill.

| Entity | # | Class | Decision Item | Acted on by | Due | Evidence | Rules |
|---|---|---|---|---|---|---|---|
| Birch Hill | 1 | critical | cash shortfall | owner | 11-11 | Stretch W6–W13, lowest −3,000.00; no trust obligations; no Headroom | CASH-SHORT-01 |
| Birch Hill | 2 | action | consider intercompany funding | owner | — | — | IC-FUND-01 |

- **Maple Ridge and Cascade:** no Decision Items.
- **Provisional:** no.

---

## CASH-S10 — An Entity with no conversion rate

Concern: a Group total is not built on a guessed rate, and one Entity's missing rate doesn't
touch its own forecast.

**Facts.** Group Lee Family Holdings, Reporting Currency CAD, no conversion rate set for USD.

| Entity | Home Currency | Account | Classification | As of | Balance |
|---|---|---|---|---|---|
| Maple Ridge Landscaping Ltd. | CAD | RBC Business Chequing | bank | 10-07 | 15,000.00 |
| Cascade Garden Supply Inc. | USD | Chase Business Checking | bank | 10-07 | 4,000.00 |

| Document | Entity | Counterparty | Due | Open |
|---|---|---|---|---|
| INV-10101 | Maple Ridge Landscaping Ltd. | Harbourview Strata Corp. | 10-16 | 2,000.00 |
| PG-10401 | Cascade Garden Supply Inc. | Pacific Growers Supply LLC | 10-23 | 1,500.00 |

**Expected: each Entity**

| Entity | Opening Cash | Week closes | Low Point |
|---|---|---|---|
| Maple Ridge | 15,000.00 | W2 17,000.00 | 15,000.00 (W1) |
| Cascade | USD 4,000.00 | W3 USD 2,500.00 | USD 2,500.00 (W3) |

**Expected: Group view (CAD)**

| Output | Detail | Rules |
|---|---|---|
| Exclusion | Cascade Garden Supply Inc., reason "no conversion rate" | CASH-GROUP-02 |
| Weekly totals | Maple Ridge only: opens 15,000.00; W2 receipts 2,000.00, closes 17,000.00 | CASH-GROUP-01 |

| Entity | # | Class | Decision Item | Subject | Acted on by | Due | Rules |
|---|---|---|---|---|---|---|---|
| Cascade | 1 | blocking | set conversion rate | USD to CAD | owner | — | CASH-GROUP-02 (Q155) |

- **Provisional:** yes. Reason: CASH-GROUP-02 on Cascade Garden Supply Inc.

---

## Coverage

| Rule | Scenarios |
|---|---|
| CASH-OPEN-01 | all |
| CASH-OPEN-02 | S01 |
| CASH-OPEN-03 | S01 |
| CASH-OPEN-04 | S01, S07 |
| CASH-HEAD-01 | S02, S03 |
| CASH-SCHED-01 | S05 |
| CASH-WEEK-01 | S06; every other Scenario has 13 weeks |
| CASH-ROLL-01 | all but S06 |
| CASH-CONF-01 | S07, S08 |
| CASH-PROV-01 | S07, S10 |
| CASH-LOW-01 | all but S06 |
| CASH-SHORT-01 | S03, S04, S08, S09 |
| CASH-BUFFER-01 | S04 |
| CASH-ORDER-01 | S04, S07, S08, S09 |
| CASH-GROUP-01 | S09, S10 |
| CASH-GROUP-02 | S10 |

## Questions the Scenarios raised

All four were decided by the owner on 2026-09-14 as picked below and written into the Rules.

- **Q152 — Which week is the Low Point when weeks tie** (CASH-LOW-01). The earliest, when the
  Entity first reaches that level.
- **Q153 — Confidence for a week with nothing placed** (CASH-CONF-01). No shares shown; 100% firm
  would claim evidence the week doesn't have.
- **Q154 — Order between a shortfall and a below-buffer item** (CASH-ORDER-01). The shortfall
  always comes first, even when the buffer stretch starts earlier (S04).
- **Q155 — Whose queue a Group item joins** (CASH-GROUP-02, CASH-ORDER-01). The queue is per
  Entity; a Group item joins the queue of the Entity it concerns.
