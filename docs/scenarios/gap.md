# GAP Scenarios

Status: **approved by the owner** (2026-09-14). Built: each Scenario here is a test in
`scenarios/tests/gap.rs`, and `tools/mutate.py gap` sweeps this document for figures a test
would not catch. This document is fixed input: when a test disagrees with it, the code is wrong
(`README.md`).

Each Scenario is a named general accounting concern, described as Canonical Facts and Settings,
with the outputs the GAP Rules must produce (ADR-0010, ADR-0016). Together these exercise every
Rule in `docs/rules/gap.md`.

## Conventions

The conventions in `docs/scenarios/ar.md` apply: run date Wednesday 2026-10-07, a 13-week
horizon (W1 is 10-07 to 10-13, W13 is 12-30 to 2027-01-05), Maple Ridge Landscaping Ltd. in
CAD unless stated, an empty Entity at the start of each Scenario, and exhaustive expected outputs
for this Family. Also:

- **Jurisdiction:** Canada federal (CRA), fiscal and calendar year-end 12-31, unless stated.
- **Reference Data** holds the CRA calendars and holiday list. A statutory due date on a
  Saturday, Sunday or CRA-recognised holiday moves to the next business day `[gap: Part 8]`.
  Dates the owner enters are used as entered.
- **Amounts are payments,** shown without a sign. Every remittance to a government is
  trust-marked.
- **September bank transactions are reconciled,** so no CLOSE check blocks the run.

---

## GAP-S01 — Scheduled obligations, and what covers one

Concern: rent, insurance, subscriptions and loan payments the ledger has no bill for are forecast
from the owner's schedule, and not again once the ledger shows them.

**Facts.** Settings, scheduled obligations:

| Obligation | Payee | Amount | Frequency | Next date | End date |
|---|---|---|---|---|---|
| O1 | Fraser Valley Properties Ltd. | 4,200.00 | monthly | 11-01 | none |
| O2 | Intact Insurance | 385.00 | monthly | 10-20 | 11-20 |
| O3 | Microsoft Canada | 96.00 | monthly | 10-10 | none |
| O4 | Kubota Credit Corporation Canada | 1,850.00 | monthly | 10-09 | none |

Documents and transactions:
- **Bill FVP-1101:** Fraser Valley Properties Ltd., November rent, dated 10-28, due 11-01,
  4,200.00.
- **Bank spend 10-05:** RBC Business Chequing, 412.00 to Intact Insurance.
- **Card charge 10-06:** RBC Visa Business, 104.50 to Microsoft Canada.
- **Bank spend 10-06:** RBC Business Chequing, 1,850.00, no payee, memo "Kubota loan" (a QBO
  purchase).

**Expected**

| Output | Item | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|
| Exclusion | O1 occurrence 11-01 | — | 4,200.00 | covered by FVP-1101 | — | GAP-SCHED-02 |
| Placement | O1 occurrence 12-01 | W8 | 4,200.00 | scheduled obligation | firm | GAP-SCHED-01 |
| Placement | O1 occurrence 2027-01-01 | W13 | 4,200.00 | scheduled obligation | firm | GAP-SCHED-01 |
| Placement | O2 occurrence 10-20 | W2 | 385.00 | scheduled obligation | firm | GAP-SCHED-01 |
| Placement | O2 occurrence 11-20 | W7 | 385.00 | scheduled obligation | firm | GAP-SCHED-01 |
| Exclusion | O3 occurrence 10-10 | — | 96.00 | covered by card charge 10-06 | — | GAP-SCHED-02 |
| Placement | O3 occurrence 11-10 | W5 | 96.00 | scheduled obligation | firm | GAP-SCHED-01 |
| Placement | O3 occurrence 12-10 | W10 | 96.00 | scheduled obligation | firm | GAP-SCHED-01 |
| Placement | O4 occurrence 10-09 | W1 | 1,850.00 | scheduled obligation | firm | GAP-SCHED-01 |
| Placement | O4 occurrence 11-09 | W5 | 1,850.00 | scheduled obligation | firm | GAP-SCHED-01 |
| Placement | O4 occurrence 12-09 | W10 | 1,850.00 | scheduled obligation | firm | GAP-SCHED-01 |

- **Coverage (Q149, Q118):**
  - **FVP-1101** is nearer the November occurrence than the October one, so it covers November.
    AP pays it.
  - **The Intact spend** is exactly halfway between two O2 occurrences, so it covers the earlier
    one, which is before the run date. O2's later occurrence stays.
  - **The Microsoft charge** covers the October O3 occurrence though the amount differs: the
    actual replaces the estimate.
  - **The spend with no payee** can't cover O4's October occurrence, so that occurrence is still
    forecast. This overstates outflows rather than understating them.
- **No Decision Items, not Provisional.**

---

## GAP-S02 — Loans, leases and credit cards

Concern: debt service is forecast only from a schedule, and a card balance only from a payment
day; a balance with neither is excluded, and the run says it is incomplete.

**Facts.** Balances at 10-07:

| Account | Classification | Balance | Settings |
|---|---|---|---|
| BDC Term Loan | loan | 48,000.00 | no scheduled obligation |
| Kubota Equipment Lease | lease liability | 22,500.00 | scheduled obligation: Kubota Canada Ltd., 1,280.00 monthly, next 10-16, for this account |
| RBC Visa Business | credit card | 3,480.00 owed | card payment day the 22nd |
| Amex Business Gold | credit card | 1,960.00 owed | no card payment day |

**Expected**

| Output | Item | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|
| Exclusion | BDC Term Loan | — | 48,000.00 | no schedule | — | GAP-LOAN-01 |
| Placement | Kubota Equipment Lease 10-16 | W2 | 1,280.00 | scheduled obligation | firm | GAP-SCHED-01 |
| Placement | Kubota Equipment Lease 11-16 | W6 | 1,280.00 | scheduled obligation | firm | GAP-SCHED-01 |
| Placement | Kubota Equipment Lease 12-16 | W11 | 1,280.00 | scheduled obligation | firm | GAP-SCHED-01 |
| Placement | RBC Visa Business | W3 (10-22) | 3,480.00 | card balance | firm | GAP-CARD-01 |
| Exclusion | Amex Business Gold | — | 1,960.00 | no payment day | — | GAP-CARD-01 |

| Decision Item | Subject | Acted on by | Rules |
|---|---|---|---|
| enter loan schedule | BDC Term Loan | owner | GAP-LOAN-01 |
| set card payment day | Amex Business Gold | owner | GAP-CARD-01 |

- **The card is paid once, in full,** at its run-date balance (Q121); later charges aren't
  forecast.
- **Provisional:** yes. Reasons: GAP-LOAN-01 on BDC Term Loan; GAP-CARD-01 on Amex Business Gold.

---

## GAP-S03 — Payroll: net pay and source deductions for a regular remitter

Concern: payroll, often the largest outflow, is forecast from the owner's schedule; remittances
already booked come from the liability, later ones from the expected amount.

**Facts.** Settings:
- **Payroll schedule:** biweekly, next pay date 10-08 (Thursday), expected net pay 18,400.00,
  expected remittance 7,600.00 per run, paid through Wagepoint.
- **Remitter type:** regular (15th of the month after pay) `[gap: Part 1]`.

| Account | Classification | As of | Balance |
|---|---|---|---|
| Payroll Liabilities | payroll liability | 09-30 | 15,200.00 |

| Transaction | Account | Payee | Dated | Amount |
|---|---|---|---|---|
| BANK-RG-1002 | Business Chequing | Receiver General for Canada | 10-02 | 5,000.00 |
| BANK-WP-1007 | Business Chequing | Wagepoint | 10-07 | 18,212.55 |

**Expected**

| Output | Item | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|
| Exclusion | Net pay 10-08 | — | 18,400.00 | covered by Wagepoint spend 10-07 | — | GAP-PAYROLL-01, GAP-SCHED-02 |
| Placement | Net pay 10-22 | W3 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 11-05 | W5 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 11-19 | W7 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 12-03 | W9 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 12-17 | W11 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 12-31 | W13 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Remittance for September, due 10-15 | W2 | 10,200.00 | booked liability | firm, trust | GAP-PAYROLL-03 |
| Placement | Remittance for October (runs 10-08, 10-22), due 11-15, moved to 11-16 | W6 | 15,200.00 | payroll schedule | firm, trust | GAP-PAYROLL-03 |
| Placement | Remittance for November (runs 11-05, 11-19), due 12-15 | W10 | 15,200.00 | payroll schedule | firm, trust | GAP-PAYROLL-03 |

- **September:** booked liability at month-end, less what was remitted after month-end; the
  Output table carries the net.
- **October's remittance counts both runs,** including the run already paid through Wagepoint:
  covering net pay doesn't cover its deductions.
- **December's runs** are remitted after the horizon.
- **No Decision Items, not Provisional.**

---

## GAP-S04 — Payroll with a missing Setting

Concern: an Entity that clearly runs payroll is never forecast as if it didn't.

**Facts.** The Group holds two Entities.
- **Birch Hill Nursery Ltd.:** "Wages and Salaries" (wages) has 14,000.00 of September activity.
  No payroll schedule.
- **Maple Ridge Landscaping Ltd.:** payroll schedule monthly, next pay date 10-30, expected net
  pay 9,800.00, expected remittance 3,900.00 per run. No remitter type.

| Entity | Account | Classification |
|---|---|---|
| Birch Hill Nursery Ltd. | Wages and Salaries | wages |
| Maple Ridge Landscaping Ltd. | Payroll Liabilities | payroll liability |

| Account | Classification | As of | Balance |
|---|---|---|---|
| Payroll Liabilities | payroll liability | 09-30 | 0.00 |

**Expected**

| Entity | Output | Item | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|---|
| Maple Ridge | Placement | Net pay 10-30 | W4 | 9,800.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Maple Ridge | Placement | Net pay 11-30 | W8 | 9,800.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Maple Ridge | Placement | Net pay 12-30 | W13 | 9,800.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Maple Ridge | Exclusion | Source deduction remittances | — | — | no remitter type | — | GAP-PAYROLL-04 |

| Entity | Decision Item | Subject | Acted on by | Rules |
|---|---|---|---|---|
| Birch Hill | set up payroll schedule | Birch Hill Nursery Ltd. | owner | GAP-PAYROLL-02 |
| Maple Ridge | set remitter type | Maple Ridge Landscaping Ltd. | accountant | GAP-PAYROLL-04 |

- **Maple Ridge's remittances carry no amount or week:** without a remitter type, the calendar
  that dates them is unknown.
- **Provisional:** yes. Reasons: GAP-PAYROLL-02 on Birch Hill; GAP-PAYROLL-04 on Maple Ridge.

---

## GAP-S05 — Remittance amounts from the last actual, or not at all

Concern: when the owner hasn't said what a future remittance will be, the Entity's own last
remittance is the estimate; with no remittance history, the amount is not guessed.

**Facts.** Both Entities: payroll schedule monthly, next pay date 10-30, no expected remittance;
remitter type regular. Maple Ridge expected net pay 9,800.00; Birch Hill 6,200.00 (first payroll
in October). Maple Ridge remitted 3,950.00 on 09-15 for August.

| Entity | Account | Classification | As of | Balance |
|---|---|---|---|---|
| Maple Ridge Landscaping Ltd. | Payroll Liabilities | payroll liability | 09-30 | 4,100.00 |
| Birch Hill Nursery Ltd. | Payroll Liabilities | payroll liability | 09-30 | 0.00 |

| Transaction | Account | Payee | Dated | Amount |
|---|---|---|---|---|
| BANK-RG-0915 | Business Chequing | Receiver General for Canada | 09-15 | 3,950.00 |

**Expected**

| Entity | Output | Item | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|---|
| Maple Ridge | Placement | Net pay 10-30 | W4 | 9,800.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Maple Ridge | Placement | Net pay 11-30 | W8 | 9,800.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Maple Ridge | Placement | Net pay 12-30 | W13 | 9,800.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Maple Ridge | Placement | Remittance for September, due 10-15 | W2 | 4,100.00 | booked liability | firm, trust | GAP-PAYROLL-03 |
| Maple Ridge | Placement | Remittance for October, due 11-15, moved to 11-16 | W6 | 3,950.00 | last remittance | estimated, trust | GAP-PAYROLL-03 |
| Maple Ridge | Placement | Remittance for November, due 12-15 | W10 | 3,950.00 | last remittance | estimated, trust | GAP-PAYROLL-03 |
| Birch Hill | Placement | Net pay 10-30 | W4 | 6,200.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Birch Hill | Placement | Net pay 11-30 | W8 | 6,200.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Birch Hill | Placement | Net pay 12-30 | W13 | 6,200.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Birch Hill | Exclusion | Remittances for October (due 11-16) and November (due 12-15) | — | — | remittance amount unknown | — | GAP-PAYROLL-03 |

| Entity | Decision Item | Subject | Acted on by | Rules |
|---|---|---|---|---|
| Birch Hill | payroll remittance amount unknown | Birch Hill Nursery Ltd., remittances due 11-16 and 12-15 | owner | GAP-PAYROLL-03 (Q158) |

- **Birch Hill's September balance is zero,** so nothing is due for that month.
- **One item for Birch Hill, not two:** entering an expected remittance fixes both (Q158).
- **Provisional:** yes. Reason: GAP-PAYROLL-03 on Birch Hill.

---

## GAP-S06 — GST/HST: a booked period, running periods and a refund position

Concern: tax already collected is remitted from the booked liability, a period still running is
estimated from the last remittance, and a refund is never forecast as cash.

**Facts.** The Group holds two Entities. Tax accounting scheme is unset on both (Q284 Pick A).
- **Maple Ridge Landscaping Ltd.:** GST/HST reporting period monthly (set). August's remittance
  was paid 09-29. No payment since.
- **Birch Hill Nursery Ltd.:** reporting period quarterly (set). No remittance history.

| Entity | Account | Classification | As of | Balance |
|---|---|---|---|---|
| Maple Ridge Landscaping Ltd. | GST/HST Payable | sales-tax liability | 09-30 | 2,150.00 |
| Birch Hill Nursery Ltd. | GST/HST Payable | sales-tax liability | 09-30 | −1,240.00 |

| Entity | Transaction | Account | Kind | Dated | Amount |
|---|---|---|---|---|---|
| Maple Ridge Landscaping Ltd. | TAX-AUG | Business Chequing | tax remittance | 09-29 | 1,980.00 |

**Expected**

| Entity | Output | Item | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|---|
| Maple Ridge | Placement | September, due 10-31, moved to 11-02 | W4 | 2,150.00 | booked tax liability | firm, trust | GAP-TAX-01 |
| Maple Ridge | Placement | October, due 11-30 | W8 | 1,980.00 | last remittance | estimated, trust | GAP-TAX-02 |
| Maple Ridge | Placement | November, due 12-31 | W13 | 1,980.00 | last remittance | estimated, trust | GAP-TAX-02 |
| Birch Hill | Exclusion | July to September | — | 1,240.00 | refund position | — | GAP-TAX-01 |

| Entity | Decision Item | Subject | Acted on by | Rules |
|---|---|---|---|---|
| Birch Hill | sales tax refund position | GST/HST, July to September | accountant | GAP-TAX-01 |

- **Due dates:** one month after each monthly or quarterly period `[gap: Part 2]`.
- **Remittances due after the horizon are not estimated** (Q159): Maple Ridge's December and
  Birch Hill's October to December remittances fall after the horizon. So Birch Hill's lack of
  history raises nothing yet.
- **Provisional:** no.

---

## GAP-S07 — A reporting period from the ledger, none at all, and no calendar

Concern: which schedule applies is the authority's decision; a value in the ledger is used but
confirmed, no value at all is not guessed, and a jurisdiction with no calendar is never skipped.

**Facts.** The Group holds two Entities. Tax accounting scheme Setting is unset on both.
- **Maple Ridge Landscaping Ltd.** (Xero): no reporting period Setting; Xero's `SalesTaxPeriod`
  is `3MONTHLY`, periods ending March, June, September and December. Xero's `SalesTaxBasis` is
  `ACCRUALS` (standard), used unconfirmed (Q253). No payment since. PST jurisdiction British
  Columbia.
- **Birch Hill Nursery Ltd.** (QBO): no reporting period Setting, and none in the ledger. No
  ledger-held tax basis (Q284 Pick A).

| Entity | Account | Classification | As of | Balance |
|---|---|---|---|---|
| Maple Ridge Landscaping Ltd. | GST/HST Payable | sales-tax liability | 09-30 | 5,400.00 |
| Maple Ridge Landscaping Ltd. | PST Payable | sales-tax liability | 09-30 | 1,800.00 |
| Birch Hill Nursery Ltd. | GST/HST Payable | sales-tax liability | 09-30 | 3,100.00 |

**Expected**

| Entity | Output | Item | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|---|
| Maple Ridge | Placement | GST/HST July to September, due 10-31, moved to 11-02 | W4 | 5,400.00 | booked tax liability | estimated, trust | GAP-TAX-01, GAP-TAX-03 |
| Maple Ridge | Exclusion | PST Payable | — | 1,800.00 | no calendar | — | GAP-TAX-04 |
| Birch Hill | Exclusion | GST/HST Payable | — | 3,100.00 | no sales tax period | — | GAP-TAX-03 |

| Entity | Decision Item | Subject | Acted on by | Rules |
|---|---|---|---|---|
| Maple Ridge | confirm sales tax period | GST/HST, quarterly from the ledger | accountant | GAP-TAX-03 |
| Maple Ridge | confirm tax accounting scheme | GST/HST | accountant | GAP-TAX-01 |
| Maple Ridge | unsupported tax jurisdiction | British Columbia PST | accountant | GAP-TAX-04 |
| Birch Hill | set sales tax period | GST/HST | accountant | GAP-TAX-03 |

- **Maple Ridge's GST/HST is estimated** because its timing rests on an unconfirmed ledger period
  and its scheme on an unconfirmed ledger basis (Q253); the amount is booked. Blank scheme with
  no ledger basis alone does not soften a remittance (Q284 Pick A — Birch Hill's path if it had
  a period).
- **Provisional:** yes. Reasons: GAP-TAX-04 on Maple Ridge (British Columbia PST); GAP-TAX-03 on
  Birch Hill. Maple Ridge's unconfirmed period and scheme are not reasons.

---

## GAP-S08 — Corporate income tax instalments and a balance due

Concern: instalments the owner has entered are paid on statutory dates from the tax-year start,
and a booked balance for a finished year is paid when it falls due.

**Facts.** The Group holds two Entities.
- **Maple Ridge Landscaping Ltd.:** tax-year start 01-01; monthly instalments of 1,500.00 entered.
- **Birch Hill Nursery Ltd.:** tax-year start 09-01, so its last year ended 08-31. No instalments
  entered. "Income Tax Payable" (income tax payable) balance at 08-31 7,200.00 for that year, no
  payment since. Whether it qualifies for the 3-month balance-due date is not set.

| Entity | Account | Classification | As of | Balance |
|---|---|---|---|---|
| Birch Hill Nursery Ltd. | Income Tax Payable | income tax payable | 08-31 | 7,200.00 |

**Expected**

| Entity | Output | Item | Week | Amount | Basis | Confidence | Rules |
|---|---|---|---|---|---|---|---|
| Maple Ridge | Placement | Instalment due 10-31, moved to 11-02 | W4 | 1,500.00 | instalment | firm | GAP-INCOME-01 |
| Maple Ridge | Placement | Instalment due 11-30 | W8 | 1,500.00 | instalment | firm | GAP-INCOME-01 |
| Maple Ridge | Placement | Instalment due 12-31 | W13 | 1,500.00 | instalment | firm | GAP-INCOME-01 |
| Birch Hill | Placement | Balance for year ended 08-31, due 10-31, moved to 11-02 | W4 | 7,200.00 | balance due | estimated | GAP-INCOME-01 (Q160) |

| Entity | Decision Item | Subject | Acted on by | Rules |
|---|---|---|---|---|
| Birch Hill | confirm income tax balance-due date | Year ended 08-31 | accountant | GAP-INCOME-01 (Q160) |

- **Instalment dates:** one month less a day from the tax-year start, then the same day each
  month, so the last day of each month here `[gap: Part 3]`.
- **Birch Hill's balance is placed at the earlier date,** months after year-end as the Output
  table shows, until someone confirms whether the longer extension applies (Q160).
- **Birch Hill has no instalments entered,** so none are forecast and no item asks for them.
- **Provisional:** no.

---

## GAP-S09 — Accruals

Concern: an accrued liability is money owed for something received but not billed; it is paid
when someone who knows says so, and not guessed otherwise.

**Facts.** Balances at 10-07, accounts classified accrued liabilities:

| Account | Balance | Settings: expected settlement |
|---|---|---|
| Accrued Liabilities | 3,600.00 | 15 days after month-end |
| Accrued Bonuses | 12,000.00 | 12-18 |
| Accrued Professional Fees | 5,500.00 | none |

**Expected**

| Output | Account | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|
| Placement | Accrued Liabilities | W2 (10-15) | 3,600.00 | accrual | estimated | GAP-ACCRUAL-01 |
| Placement | Accrued Bonuses | W11 (12-18) | 12,000.00 | accrual | estimated | GAP-ACCRUAL-01 |
| Exclusion | Accrued Professional Fees | — | 5,500.00 | no expected settlement | — | GAP-ACCRUAL-01 |

| Decision Item | Subject | Acted on by | Rules |
|---|---|---|---|
| when will this accrual be paid | Accrued Professional Fees | accountant | GAP-ACCRUAL-01 |

- **"15 days after month-end"** counts from the last completed month-end.
- **Provisional:** yes. Reason: GAP-ACCRUAL-01 on Accrued Professional Fees.

---

## GAP-S10 — Accelerated threshold 1 (half-month remittance periods)

Concern: when CRA assigns Accelerated threshold 1, remittances cover 1st–15th and 16th–end, not
the whole calendar month, and due dates are the 25th / 10th (moved for weekends and CRA holidays).

**Facts.** Settings:
- **Payroll schedule:** biweekly, next pay date 10-08 (Thursday), expected net pay 18,400.00,
  expected remittance 7,600.00 per run, paid through Wagepoint.
- **Remitter type:** Accelerated threshold 1 `[gap: Part 1]`.

| Account | Classification | As of | Balance |
|---|---|---|---|
| Payroll Liabilities | payroll liability | 09-30 | 15,200.00 |

| Transaction | Account | Payee | Dated | Amount |
|---|---|---|---|---|
| BANK-RG-1002 | Business Chequing | Receiver General for Canada | 10-02 | 5,000.00 |
| BANK-WP-1007 | Business Chequing | Wagepoint | 10-07 | 18,212.55 |

**Expected**

| Output | Item | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|
| Exclusion | Net pay 10-08 | — | 18,400.00 | covered by Wagepoint spend 10-07 | — | GAP-PAYROLL-01, GAP-SCHED-02 |
| Placement | Net pay 10-22 | W3 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 11-05 | W5 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 11-19 | W7 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 12-03 | W9 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 12-17 | W11 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 12-31 | W13 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Remittance for 16–30 September, due 10-10, moved to 10-13 | W1 | 10,200.00 | booked liability | firm, trust | GAP-PAYROLL-03 |
| Placement | Remittance for 1–15 October (runs 10-08), due 10-25, moved to 10-26 | W3 | 7,600.00 | payroll schedule | firm, trust | GAP-PAYROLL-03 |
| Placement | Remittance for 16–31 October (runs 10-22), due 11-10 | W5 | 7,600.00 | payroll schedule | firm, trust | GAP-PAYROLL-03 |
| Placement | Remittance for 1–15 November (runs 11-05), due 11-25 | W8 | 7,600.00 | payroll schedule | firm, trust | GAP-PAYROLL-03 |
| Placement | Remittance for 16–30 November (runs 11-19), due 12-10 | W10 | 7,600.00 | payroll schedule | firm, trust | GAP-PAYROLL-03 |
| Placement | Remittance for 1–15 December (runs 12-03), due 12-25, moved to 12-28 | W12 | 7,600.00 | payroll schedule | firm, trust | GAP-PAYROLL-03 |

- **September’s second half** is the ended band that owns the month-end liability (booked less
  remitted after month-end). Statutory due falls on a Saturday; Thanksgiving moves cash as in the
  Output table.
- **Each half-month band with a pay run** remits the per-run amount in Facts. The December second
  half is due after the horizon.
- **No Decision Items, not Provisional.**

---

## GAP-S11 — Accelerated threshold 2 (weekly remittance bands)

Concern: Accelerated threshold 2 remits four times a month; each band ends on the 7th, 14th,
21st or month-end and is due the third working day after that end.

**Facts.** Settings:
- **Payroll schedule:** biweekly, next pay date 10-08 (Thursday), expected net pay 18,400.00,
  expected remittance 7,600.00 per run, paid through Wagepoint.
- **Remitter type:** Accelerated threshold 2 `[gap: Part 1]`.

| Account | Classification | As of | Balance |
|---|---|---|---|
| Payroll Liabilities | payroll liability | 09-30 | 15,200.00 |

| Transaction | Account | Payee | Dated | Amount |
|---|---|---|---|---|
| BANK-RG-1002 | Business Chequing | Receiver General for Canada | 10-02 | 5,000.00 |
| BANK-WP-1007 | Business Chequing | Wagepoint | 10-07 | 18,212.55 |

**Expected**

| Output | Item | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|
| Exclusion | Net pay 10-08 | — | 18,400.00 | covered by Wagepoint spend 10-07 | — | GAP-PAYROLL-01, GAP-SCHED-02 |
| Placement | Net pay 10-22 | W3 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 11-05 | W5 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 11-19 | W7 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 12-03 | W9 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 12-17 | W11 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 12-31 | W13 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Remittance for 22–30 September, due 10-05 | W1 | 10,200.00 | booked liability | firm, trust | GAP-PAYROLL-03 |
| Placement | Remittance for 8–14 October (runs 10-08), due 10-19 | W2 | 7,600.00 | payroll schedule | firm, trust | GAP-PAYROLL-03 |
| Placement | Remittance for 22–31 October (runs 10-22), due 11-04 | W5 | 7,600.00 | payroll schedule | firm, trust | GAP-PAYROLL-03 |
| Placement | Remittance for 1–7 November (runs 11-05), due 11-12 | W6 | 7,600.00 | payroll schedule | firm, trust | GAP-PAYROLL-03 |
| Placement | Remittance for 15–21 November (runs 11-19), due 11-25 | W8 | 7,600.00 | payroll schedule | firm, trust | GAP-PAYROLL-03 |
| Placement | Remittance for 1–7 December (runs 12-03), due 12-10 | W10 | 7,600.00 | payroll schedule | firm, trust | GAP-PAYROLL-03 |
| Placement | Remittance for 15–21 December (runs 12-17), due 12-24 | W12 | 7,600.00 | payroll schedule | firm, trust | GAP-PAYROLL-03 |

- **September’s last band** owns the booked liability (same net as the Output remittance row);
  due is already a working-day count, so no separate “moved to”.
- **Bands with no pay run** produce no remittance.
- **December’s last band** is due the third working day after month-end; with New Year’s Day on
  the holiday list that falls after the horizon.
- **No Decision Items, not Provisional.**

---

## GAP-S12 — Quarterly remitter

Concern: a small-employer quarterly remitter remits once per calendar quarter, due the 15th of
the month after quarter-end (15 Apr / Jul / Oct / Jan), moved for weekends and CRA holidays.

**Facts.** Settings:
- **Payroll schedule:** biweekly, next pay date 10-08 (Thursday), expected net pay 18,400.00,
  expected remittance 7,600.00 per run, paid through Wagepoint.
- **Remitter type:** Quarterly `[gap: Part 1]`.

| Account | Classification | As of | Balance |
|---|---|---|---|
| Payroll Liabilities | payroll liability | 09-30 | 15,200.00 |

| Transaction | Account | Payee | Dated | Amount |
|---|---|---|---|---|
| BANK-RG-1002 | Business Chequing | Receiver General for Canada | 10-02 | 5,000.00 |
| BANK-WP-1007 | Business Chequing | Wagepoint | 10-07 | 18,212.55 |

**Expected**

| Output | Item | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|
| Exclusion | Net pay 10-08 | — | 18,400.00 | covered by Wagepoint spend 10-07 | — | GAP-PAYROLL-01, GAP-SCHED-02 |
| Placement | Net pay 10-22 | W3 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 11-05 | W5 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 11-19 | W7 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 12-03 | W9 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 12-17 | W11 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Net pay 12-31 | W13 | 18,400.00 | payroll schedule | firm | GAP-PAYROLL-01 |
| Placement | Remittance for July–September, due 10-15 | W2 | 10,200.00 | booked liability | firm, trust | GAP-PAYROLL-03 |

- **Q3 (July–September)** is the ended quarter that owns the month-end liability (booked less
  remitted after month-end). Due falls in the horizon as in the Output table.
- **Q4 (October–December)** — every pay run in the horizon — remits after the horizon, so no
  estimated remittance rows (same posture as S03’s December under Regular / Q159).
- **No Decision Items, not Provisional.**

---

## Coverage

| Rule | Scenarios |
|---|---|
| GAP-SCHED-01 | S01, S02 |
| GAP-SCHED-02 | S01, S03, S10, S11, S12 |
| GAP-LOAN-01 | S02 |
| GAP-PAYROLL-01 | S03, S04, S05, S10, S11, S12 |
| GAP-PAYROLL-02 | S04 |
| GAP-PAYROLL-03 | S03, S05, S10, S11, S12 |
| GAP-PAYROLL-04 | S04 |
| GAP-TAX-01 | S06, S07 |
| GAP-TAX-02 | S06 |
| GAP-TAX-03 | S07 |
| GAP-TAX-04 | S07 |
| GAP-INCOME-01 | S08 |
| GAP-CARD-01 | S02 |
| GAP-ACCRUAL-01 | S09 |

## Questions the Scenarios raised

All three were decided by the owner on 2026-09-14 as picked below and written into the Rules.

- **Q158 — One item or one per remittance** (GAP-PAYROLL-03). One item per Entity listing every
  remittance without an amount, because one expected amount fixes them all (S05).
- **Q159 — Running periods whose remittance falls after the horizon** (GAP-TAX-02). Only
  remittances due within the horizon are estimated or excluded (S06).
- **Q160 — The corporate balance-due date when the 3-month extension is unknown**
  (GAP-INCOME-01). The earlier date, Confidence estimated, with an item for the accountant to
  confirm; not Provisional, because the amount is booked and the earlier date never understates
  near-term outflows (S08).
