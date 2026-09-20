# CLOSE Scenarios

Status: **approved by the owner** (2026-09-14). Nothing here is built.

Each Scenario is a named general accounting concern, described as Canonical Facts and Settings,
with the outputs the CLOSE Rules must produce (ADR-0010, ADR-0016). Together these exercise every
Rule in `docs/rules/close.md`.

## Conventions

The conventions in `docs/scenarios/ar.md` apply: run date Wednesday 2026-10-07, the last completed
month September 2026, the month before last August 2026, Maple Ridge Landscaping Ltd. in CAD
unless stated, an empty Entity at the start of each Scenario, and exhaustive expected outputs for
this Family. Also:

- **The ledger is QBO** unless stated.
- **Transactions listed are all the account has.** None is deleted unless stated.
- **No tax-year start and no sales tax reporting period are set,** so CLOSE-LOCK-01 does nothing,
  unless stated.
- **CLOSE produces no Placements** (Q143 also rules out Draft Corrections), so each Scenario lists
  Decision Items and the run's Provisional state.

---

## CLOSE-S01 — Has last month been reconciled?

Concern: without a reconciliation, nothing shows that the bank's movements are all in the books,
so the forecast can't rest on them; outstanding items at month-end are normal and not the test.

**Facts**

| Account | Classification | September transactions | Notes |
|---|---|---|---|
| RBC Business Chequing | bank | 12: 9 reconciled; cheques #1101 (09-28), #1102 (09-29) and #1103 (09-30) unreconciled | |
| Scotiabank Operating | bank | 8, none reconciled | |
| RBC Visa Business | credit card | 15 charges, none reconciled | |
| Petty Cash | bank, marked not reconciled against statements | 6, none reconciled | |
| TD Business Savings | bank | 1, deleted | No August transactions |
| CIBC Business Operating | bank | 3 journal entries, reconciliation status unknown | The ledger's reconciliation source has no row matching these lines (Q180) |

**Expected**

| Decision Item | Subject | Acted on by | Rules |
|---|---|---|---|
| reconcile last month | Scotiabank Operating, September 2026 | bookkeeper | CLOSE-BANK-01 |
| reconcile last month | RBC Visa Business, September 2026 | bookkeeper | CLOSE-BANK-01 (Q135) |
| reconcile last month | CIBC Business Operating, September 2026 | bookkeeper | CLOSE-BANK-01 (Q172, Q173, Q180) |

- **RBC Business Chequing is reconciled:** three cheques outstanding at month-end are normal
  (Q134), and none is older than September, so CLOSE-BANK-02 has nothing.
- **Petty Cash is skipped;** the Entity doesn't reconcile it.
- **TD Business Savings has no September transactions,** because the only one is deleted.
- **CIBC Business Operating is not reconciled:** its journal entries count as transactions even
  though the reconciliation source can't say their status, and unknown counts as not reconciled.
- **Provisional:** yes. Reasons: CLOSE-BANK-01 on Scotiabank Operating; CLOSE-BANK-01 on RBC Visa
  Business; CLOSE-BANK-01 on CIBC Business Operating.

---

## CLOSE-S02 — Items outstanding through a full cycle, and an account gone quiet

Concern: an item still unreconciled after a later month has been reconciled is a stale cheque, a
duplicate or an error; an account that stopped recording anything usually means the books are
behind.

**Facts**

| Account | Classification | Transactions |
|---|---|---|
| RBC Business Chequing | bank | September: 20, 19 reconciled; cheque #1119 (09-29) unreconciled. Earlier, unreconciled: cheque #1043 (07-22), 1,250.00 to Western Turf Farms; deposit (08-31), 480.00. Earlier, status unknown: journal entry (08-20), 5,000.00 transfer |
| RBC Visa Business | credit card | September: 11, all reconciled. Earlier, unreconciled: charge (08-14), 96.00 to Microsoft Canada |
| Scotiabank Operating | bank | August: 4, all reconciled. September: none |
| TD Business Savings | bank | June: 2. Nothing since |
| Petty Cash | bank, marked not reconciled against statements | August: 3. September: none |

**Expected**

| Decision Item | Subject | Evidence | Acted on by | Rules |
|---|---|---|---|---|
| stale outstanding items | RBC Business Chequing | cheque #1043 (07-22) 1,250.00; deposit (08-31) 480.00 | bookkeeper | CLOSE-BANK-02 |
| stale outstanding items | RBC Visa Business | charge (08-14) 96.00 | bookkeeper | CLOSE-BANK-02 (Q135) |
| no activity recorded | Scotiabank Operating, September 2026 | 4 transactions in August, none in September | bookkeeper | CLOSE-BANK-03 |

- **Cheque #1119** was outstanding for less than a full cycle, so it isn't listed.
- **The 08-20 journal entry isn't listed:** its status is unknown, not unreconciled (Q178).
- **The 08-31 deposit is listed:** it is dated on the last day of August, so it had all of
  September to clear.
- **Scotiabank Operating raises no reconciliation item:** with no September transactions, there
  was nothing to reconcile.
- **TD Business Savings** went quiet before August, so it isn't flagged again.
- **Petty Cash is skipped.**
- **Provisional:** no.

---

## CLOSE-S03 — Possible duplicate bank transactions

Concern: a transaction entered twice misstates book cash; only exact, unreconciled matches are
flagged, because a reconciled transaction is backed by its own statement line.

**Facts.** Each account's September transactions are otherwise reconciled.

| # | Account | Date | Payee | Amount | Status |
|---|---|---|---|---|---|
| T1 | RBC Business Chequing | 10-02 | Telus Business | 450.00 | unreconciled |
| T2 | RBC Business Chequing | 10-02 | Telus Business | 450.00 | unreconciled |
| T3 | RBC Business Chequing | 09-18 | Home Depot Pro | 1,130.00 | reconciled |
| T4 | RBC Business Chequing | 09-18 | Home Depot Pro | 1,130.00 | unreconciled |
| T5 | RBC Business Chequing | 10-05 | Stihl Canada | 2,260.00 | unreconciled |
| T6 | RBC Business Chequing | 10-05 | Brandt Tractor Ltd. | 2,260.00 | unreconciled |
| T7 | RBC Business Chequing | 10-05 | Fraser Valley Properties Ltd. | 4,200.00 | unreconciled |
| T8 | RBC Business Chequing | 10-05 | Fraser Valley Properties Ltd. | 4,200.00 | unreconciled, deleted |
| T9 | RBC Visa Business | 10-03 | Microsoft Canada | 104.50 | unreconciled |
| T10 | RBC Visa Business | 10-03 | Microsoft Canada | 104.50 | unreconciled |
| T11 | Scotiabank Operating | 10-02 | Telus Business | 450.00 | unreconciled |
| T12 | Scotiabank Operating | 10-06 | Scotiabank | 35.00 | unknown |
| T13 | Scotiabank Operating | 10-06 | Scotiabank | 35.00 | unknown |

**Expected**

| Decision Item | Subject | Acted on by | Rules |
|---|---|---|---|
| possible duplicate bank transaction | T1 and T2 | bookkeeper | CLOSE-DUP-01 |
| possible duplicate bank transaction | T9 and T10 | bookkeeper | CLOSE-DUP-01 |

- **Not flagged:**
  - **T3 and T4:** T3 is reconciled.
  - **T5 and T6:** different payees.
  - **T7 and T8:** T8 is deleted.
  - **T11 and T1:** different accounts.
  - **T12 and T13:** status unknown, so neither is a candidate (Q178).
- **Provisional:** no.

---

## CLOSE-S04 — Suspense, uncategorized and reconciliation discrepancies

Concern: money parked in suspense can hide a customer or vendor payment, so a balance there
blocks; uncategorized income and expense, and forced reconciliation matches, need attention but
don't hide cash.

**Facts**

| Account | How it is classified (Q136) | Balance at 09-30 | September activity |
|---|---|---|---|
| Uncategorized Asset | uncategorized asset, by the default classifier on QBO's default name | 1,275.00 | 2 postings |
| Suspense – Unknown Deposits | suspense, by the Group's mapping | −350.00 | 1 posting |
| Customer Deposits Holding | suspense, by the Group's mapping | 0.00 | 3 postings, cleared within the month |
| Uncategorized Income | uncategorized income, by metadata | — | 3 postings, 2,340.00 |
| Uncategorized Expense | uncategorized expense, by metadata | — | none (410.00 posted in July) |
| Reconciliation Discrepancies | reconciliation discrepancies, by the default classifier on QBO's default name | — | 1 posting, 18.62 |

**Expected**

| Decision Item | Subject | Evidence | Acted on by | Rules |
|---|---|---|---|---|
| clear suspense balance | Uncategorized Asset, September 2026 | balance 1,275.00 | bookkeeper | CLOSE-SUSP-01 |
| clear suspense balance | Suspense – Unknown Deposits, September 2026 | balance −350.00 | bookkeeper | CLOSE-SUSP-01 |
| categorise transactions | Uncategorized Income, September 2026 | 3 postings, 2,340.00 | bookkeeper | CLOSE-SUSP-02 |
| review reconciliation adjustment | Reconciliation Discrepancies, September 2026 | 1 posting, 18.62 | accountant | CLOSE-DISC-01 |

- **A negative suspense balance** is non-zero too.
- **Customer Deposits Holding** ends the month at zero, so it raises nothing, however busy it was.
- **Uncategorized Expense** had no September activity. Its July posting belongs to an earlier
  close.
- **Provisional:** yes. Reasons: CLOSE-SUSP-01 on Uncategorized Asset; CLOSE-SUSP-01 on
  Suspense – Unknown Deposits.

---

## CLOSE-S05 — Money that hasn't reached the bank

Concern: receipts sitting in a clearing account should reach the bank within a month; one still
there after a full month may be lost, and a negative clearing balance is an error.

**Facts**

| Account | Classification | Balance at 08-31 | Moved out since 08-31 | Balance at 09-30 |
|---|---|---|---|---|
| Undeposited Funds | clearing, by QBO metadata | 3,400.00 (receipts 08-27 1,900.00; 08-31 1,500.00) | deposit 09-02, 1,900.00 | 2,100.00 (1,500.00 plus a 09-30 receipt of 600.00) |
| Stripe Clearing | clearing, by the Group's mapping | 820.00 | payouts 1,960.00 | −45.00 |
| Square Clearing | clearing, by the Group's mapping | 0.00 | none | 640.00 |

**Expected**

| Decision Item | Subject | Evidence | Acted on by | Rules |
|---|---|---|---|---|
| money not banked | Undeposited Funds, August 2026 | 3,400.00 at 08-31, less 1,900.00 moved out since: 1,500.00 still unmoved | bookkeeper | CLOSE-CLEAR-01 |
| negative clearing balance | Stripe Clearing, September 2026 | balance −45.00 at 09-30 | bookkeeper | CLOSE-CLEAR-01 |

- **Undeposited Funds' 09-30 receipt** is a normal month-end deposit and isn't flagged.
- **Stripe Clearing moved out more than it held at 08-31,** so nothing from August is unmoved.
  Its negative balance is the finding.
- **Square Clearing** was empty at 08-31.
- **The forecast is unchanged:** CASH-OPEN-03 still places each run-date balance in week 1.
- **Provisional:** no.

---

## CLOSE-S06 — Closed periods that aren't locked

Concern: once a return depends on a period's figures, the period should be locked; an unlocked
month isn't wrong, so only genuinely closed periods are checked.

**Facts.** The Group holds four Entities.

| Entity | Ledger | Tax-year start | Sales tax reporting period | Lock date |
|---|---|---|---|---|
| Maple Ridge Landscaping Ltd. | QBO | 01-01 | GST/HST quarterly | `BookCloseDate` 03-31 |
| Birch Hill Nursery Ltd. | Xero | 09-01 | GST/HST annual | `PeriodLockDate` 08-31; `EndOfYearLockDate` 2025-08-31 |
| Cascade Garden Supply Inc. (USD) | QBO | 01-01 | none | none |
| Alder Bay Properties Ltd. | QBO | none | none | none |

**Expected**

| Entity | Decision Item | Subject | Evidence | Acted on by | Rules |
|---|---|---|---|---|---|
| Maple Ridge | lock closed period | Maple Ridge Landscaping Ltd., through 06-30 | Most recent closed period ends 06-30 (April to June GST/HST, due 07-31); lock date 03-31 | accountant | CLOSE-LOCK-01 |
| Cascade | lock closed period | Cascade Garden Supply Inc., through 2025-12-31 | Most recent closed period is fiscal year-end 2025-12-31; no lock date | accountant | CLOSE-LOCK-01 |

**How the closed periods come out**

| Entity | Last fiscal year-end | Last tax period whose due date has passed | Most recent closed period | Lock covers it? |
|---|---|---|---|---|
| Maple Ridge | 2025-12-31 | April to June, ended 06-30 (July to September is due 10-31) | 06-30 | no |
| Birch Hill | 2026-08-31 | year ended 2025-08-31 (due 2025-11-30); the year ended 2026-08-31 is due 11-30 | 2026-08-31 | yes, by `PeriodLockDate` (Q164) |
| Cascade | 2025-12-31 | none set | 2025-12-31 | no |
| Alder Bay | none | none | cannot be worked out; the Rule does nothing | — |

- **Provisional:** no. A missing lock is never blocking (Q139).

---

## CLOSE-S07 — Conversion balances left over from setup

Concern: a balance left in the account opening balances were posted to says setup was never
finished; it needs the accountant but doesn't put cash at risk.

**Facts.** The Group holds two Entities.
- **Maple Ridge Landscaping Ltd.** (QBO): Opening Balance Equity (conversion, by metadata
  `OpeningBalanceEquity`) balance at 09-30 −2,480.00.
- **Birch Hill Nursery Ltd.** (Xero): Historical Adjustment (conversion, by metadata
  `SystemAccount: HISTORICAL`) balance at 09-30 0.00.

**Expected**

| Entity | Decision Item | Subject | Evidence | Acted on by | Rules |
|---|---|---|---|---|---|
| Maple Ridge | clear conversion balance | Opening Balance Equity, September 2026 | balance −2,480.00 | accountant | CLOSE-CONV-01 |

- **Birch Hill's Historical Adjustment is zero,** so it raises nothing.
- **Provisional:** no.

---

## Coverage

| Rule | Scenarios |
|---|---|
| CLOSE-BANK-01 | S01, S02 |
| CLOSE-BANK-02 | S01, S02 |
| CLOSE-BANK-03 | S01, S02 |
| CLOSE-DUP-01 | S03 |
| CLOSE-SUSP-01 | S04 |
| CLOSE-SUSP-02 | S04 |
| CLOSE-DISC-01 | S04 |
| CLOSE-CLEAR-01 | S05 |
| CLOSE-LOCK-01 | S06 |
| CLOSE-CONV-01 | S07 |

## Questions the Scenarios raised

Decided by the owner on 2026-09-14 and written into the Rules.

- **Q164 — Which Xero lock date counts** (CLOSE-LOCK-01). The later of `PeriodLockDate` and
  `EndOfYearLockDate`, because either stops the owner's staff changing the period (S06, Birch
  Hill). That `PeriodLockDate` exempts advisers is **UNVERIFIED**: it comes from Xero's settings
  screen, not the research notes, which record only that both fields exist.
