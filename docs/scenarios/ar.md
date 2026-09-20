# AR Scenarios

Status: **approved by the owner** (2026-09-14). Built: every Scenario here passes as a test in
`scenarios/tests/ar.rs`, and `tools/mutate.py ar` sweeps this document for figures a test would
not catch. This document is fixed input: when a test disagrees with it, the code is wrong
(`README.md`).

Each Scenario is a named general accounting concern, described as Canonical Facts and Settings,
with the outputs the AR Rules must produce (ADR-0010, ADR-0016). M1 is done when every Scenario
passes as a test. Together these exercise every Rule in `docs/rules/ar.md`.

## Conventions (all Families)

- **Run date:** Wednesday 2026-10-07. The last completed month is September 2026; the month
  before last is August 2026.
- **Horizon:** 13 weeks, each a 7-day period from the run date (ADR-0020):

  | Week | Dates | Week | Dates |
  |---|---|---|---|
  | W1 | 10-07 to 10-13 | W8 | 11-25 to 12-01 |
  | W2 | 10-14 to 10-20 | W9 | 12-02 to 12-08 |
  | W3 | 10-21 to 10-27 | W10 | 12-09 to 12-15 |
  | W4 | 10-28 to 11-03 | W11 | 12-16 to 12-22 |
  | W5 | 11-04 to 11-10 | W12 | 12-23 to 12-29 |
  | W6 | 11-11 to 11-17 | W13 | 12-30 to 2027-01-05 |
  | W7 | 11-18 to 11-24 | | |

- **Entity:** Maple Ridge Landscaping Ltd., Home Currency CAD, unless a Scenario says otherwise.
  Every Scenario starts from an empty Entity; nothing carries over between Scenarios.
- **Facts** are Canonical Facts, never provider payloads (ADR-0010). Names are synthetic and
  realistic. Dates are 2026 unless shown. "Age" is days after the due date (negative before it).
- **Expected outputs are exhaustive for this Family:** a test fails on any AR output not listed.
  Other Families' outputs are not listed.

---

## AR-S01 — Open, draft, voided, zero-total and paid invoices

Concern: only issued, unpaid invoices are receivables, a void is not a collection, and receipts
include tax.

**Facts**

| Invoice | Customer | Status | Due | Total | Tax in total | Open | Notes |
|---|---|---|---|---|---|---|---|
| INV-1001 | Harbourview Strata Corp. | posted | 10-20 | 5,250.00 | 250.00 | 5,250.00 | |
| INV-1002 | Harbourview Strata Corp. | draft | 11-05 | 2,100.00 | 100.00 | 2,100.00 | |
| INV-1003 | Harbourview Strata Corp. | voided | 09-01 | 0.00 | 0.00 | 0.00 | Voided in September |
| INV-1004 | Harbourview Strata Corp. | posted | 10-15 | 1,050.00 | 50.00 | 0.00 | Paid in full, early |
| INV-1005 | Harbourview Strata Corp. | posted | 10-20 | 0.00 | 0.00 | 0.00 | No-charge warranty visit |

**Expected**

| Output | Invoice | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|
| Placement | INV-1001 | W2 | 5,250.00 | due date | firm | AR-OPEN-01, AR-OPEN-04, AR-TIME-02 |
| Exclusion | INV-1002 | — | 2,100.00 | not issued | — | AR-OPEN-02 |

- **No history applies to INV-1001** (age −13). INV-1004 was already paid by age −13, so it is
  not comparable. INV-1003 is voided and INV-1005 has a zero total, so both are ignored (Q145).
  INV-1003 counted as paid on 09-25 (age +24) would wrongly place INV-1001 at 11-13 (W6).
  INV-1005 is never a receivable and never a comparable invoice.
- **No other outputs:** no Decision Items, and the run is not Provisional.

---

## AR-S02 — Owner-entered expected dates, current and stale

Concern: a person's stated expectation places an invoice; a passed one is not trusted.

**Facts.** No paid invoices in the Entity's history.

| Invoice | Customer | Due | Open | Expected payment date |
|---|---|---|---|---|
| INV-2001 | Alder Creek Homes | 09-15 | 3,150.00 | 10-30 |
| INV-2002 | Alder Creek Homes | 09-01 | 1,575.00 | 10-01 |

**Expected**

| Output | Invoice | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|
| Placement | INV-2001 | W4 | 3,150.00 | expected date | firm | AR-TIME-01 |
| Exclusion | INV-2002 | — | 1,575.00 | no timing evidence | — | AR-TIME-04 |

| Decision Item | Subject | Acted on by | Rules |
|---|---|---|---|
| collect | INV-2001 | owner | AR-COLLECT-01 (first: larger open amount) |
| collect | INV-2002 | owner | AR-COLLECT-01 |
| set expected date (cites expected date 10-01, passed) | INV-2002 | owner | AR-TIME-04 (Q150) |

- **Provisional:** yes. Reason: AR-TIME-04 on INV-2002.

---

## AR-S03 — The Entity's collection history times overdue and not-yet-due invoices

Concern: timing comes from how this Entity's customers actually pay, with no day thresholds.

**Facts.** Paid history, each invoice 2,000.00 and paid in full in one payment:

| Invoice | Customer | Due | Paid | Age at payment |
|---|---|---|---|---|
| H-3101 | Cedar Point Property Mgmt. | 06-01 | 06-06 | +5 |
| H-3102 | Cedar Point Property Mgmt. | 06-15 | 06-25 | +10 |
| H-3103 | Westgate Business Park | 07-01 | 07-21 | +20 |
| H-3104 | Westgate Business Park | 07-15 | 08-14 | +30 |
| H-3105 | Riverbend Elementary PAC | 08-01 | 09-10 | +40 |

Open invoices:

| Invoice | Customer | Due | Open | Age at run |
|---|---|---|---|---|
| INV-3001 | Cedar Point Property Mgmt. | 10-17 | 4,200.00 | −10 |
| INV-3002 | Westgate Business Park | 09-22 | 2,625.00 | +15 |
| INV-3003 | Riverbend Elementary PAC | 09-02 | 1,050.00 | +35 |

**Expected**

| Output | Invoice | Week | Amount | Basis | Median | Comparable | Confidence | Rules |
|---|---|---|---|---|---|---|---|---|
| Placement | INV-3003 | W1 (10-12) | 1,050.00 | Entity history | 5 days | 1 | estimated | AR-TIME-02 |
| Placement | INV-3002 | W3 (10-22) | 2,625.00 | Entity history | 15 days | 4 | estimated | AR-TIME-02 |
| Placement | INV-3001 | W6 (11-16) | 4,200.00 | Entity history | 40 days | 7 | estimated | AR-TIME-02 |

| Decision Item | Subject | Acted on by | Rules |
|---|---|---|---|
| collect | INV-3002 | owner | AR-COLLECT-01 (first) |
| collect | INV-3003 | owner | AR-COLLECT-01 |

**How the medians come out.** These use the reading in Q147.
- **INV-3003 (age +35):** the comparable set, invoices unpaid at +35, is H-3105 alone, paid 5 days
  later. Median 5 days, placed 10-12.
- **INV-3002 (age +15):** the comparable set is H-3103, H-3104 and H-3105 (paid 5, 15 and 25
  days later) plus INV-3003, still open, which counts as not collected. The first point at
  which at least half (2 of 4) were collected is 15 days, so 10-22.
- **INV-3001 (age −10):** all five H invoices (15, 20, 30, 40 and 50 days later) plus INV-3002
  and INV-3003, still open. At least half (4 of 7) collected first at 40 days, so 11-16.

- **Provisional:** no.

---

## AR-S04 — History says invoices this old are not collected

Concern: when most comparable invoices were never collected, the invoice leaves the forecast
and a write-off review is raised; the decision stays with the Entity.

**Facts.** H-4101 to H-4103 were each closed by a credit note with no payment — on 07-14, 08-14
and 09-11, all at age +180 — so none was ever collected. No account is mapped for bad-debt
write-offs.

| Invoice | Customer | Due | Total | Open | Paid | Age at payment |
|---|---|---|---|---|---|---|
| H-4101 | Pinecrest Builders Inc. | 01-15 | 1,500.00 | 0.00 | — | — |
| H-4102 | Pinecrest Builders Inc. | 02-15 | 1,800.00 | 0.00 | — | — |
| H-4103 | Pinecrest Builders Inc. | 03-15 | 1,200.00 | 0.00 | — | — |
| H-4104 | Sandpiper Motel | 04-01 | 900.00 | 0.00 | 04-11 | +10 |
| H-4105 | Sandpiper Motel | 05-01 | 900.00 | 0.00 | 05-21 | +20 |

Open invoices:

| Invoice | Customer | Due | Open | Age at run |
|---|---|---|---|---|
| INV-4001 | Pinecrest Builders Inc. | 07-09 | 2,310.00 | +90 |

**Expected**

| Output | Invoice | Amount | Reason | Comparable | Rules |
|---|---|---|---|---|---|
| Exclusion | INV-4001 | 2,310.00 | history says uncollected | 3 (all never collected) | AR-TIME-03 |

| Decision Item | Subject | Acted on by | Draft Correction | Rules |
|---|---|---|---|---|
| collect | INV-4001 | owner | — | AR-COLLECT-01 |
| review for write-off | INV-4001 | accountant | Credit note for 2,310.00 against INV-4001; **incomplete**: no bad-debt account mapped | AR-WRITEOFF-01 |

- **Comparable set at +90:** H-4101 to H-4103, unpaid at +90 and never collected. H-4104 and
  H-4105 were paid before +90.
- **Provisional:** no.

---

## AR-S05 — History says collection falls beyond the horizon

Concern: an invoice expected after the horizon is left out, but that alone is no write-off
signal.

**Facts.** Paid history, each invoice paid in full in one payment:

| Invoice | Customer | Due | Total | Paid | Age at payment |
|---|---|---|---|---|---|
| H-5101 | Oakview Senior Living | 01-10 | 2,400.00 | 05-10 | +120 |
| H-5102 | Oakview Senior Living | 02-10 | 2,400.00 | 06-20 | +130 |
| H-5103 | Oakview Senior Living | 03-10 | 2,400.00 | 07-28 | +140 |

Open invoices:

| Invoice | Customer | Due | Open | Age at run |
|---|---|---|---|---|
| INV-5001 | Oakview Senior Living | 09-27 | 2,400.00 | +10 |

**Expected**

| Output | Invoice | Amount | Reason | Median | Comparable | Rules |
|---|---|---|---|---|---|---|
| Exclusion | INV-5001 | 2,400.00 | beyond horizon (2027-02-04) | 120 days | 3 | AR-TIME-03 |

| Decision Item | Subject | Acted on by | Rules |
|---|---|---|---|
| collect | INV-5001 | owner | AR-COLLECT-01 |

- **No write-off review:** none of the comparable invoices went uncollected.
- **Provisional:** no.

---

## AR-S06 — Intercompany, foreign currency and early-payment discounts

Concern: a sister Entity's invoice isn't external cash, a foreign receipt keeps its booked rate,
and a discount is not assumed.

**Facts.** The Group also holds Birch Hill Nursery Ltd. Its contact in Maple Ridge is classified
intercompany, naming Birch Hill. No paid history.

| Invoice | Customer | Currency | Dated | Due | Open (document) | Booked rate | Open (CAD) | Terms |
|---|---|---|---|---|---|---|---|---|
| INV-6001 | Birch Hill Nursery Ltd. | CAD | 09-16 | 10-16 | 2,000.00 | — | 2,000.00 | net 30 |
| INV-6002 | Summit Outdoor Supply Inc. | USD | 09-27 | 10-27 | 1,000.00 | 1.3600 | 1,360.00 | net 30 |
| INV-6003 | Lakeshore Condominium Assoc. | CAD | 10-06 | 11-05 | 3,000.00 | — | 3,000.00 | 2% 10 days, net 30 |

**Expected**

| Output | Invoice | Week | Amount (CAD) | Basis / reason | Confidence | Recorded on Placement | Rules |
|---|---|---|---|---|---|---|---|
| Exclusion | INV-6001 | — | 2,000.00 | intercompany, see IC | — | — | AR-OPEN-03 |
| Placement | INV-6002 | W3 | 1,360.00 | due date | firm | USD 1,000.00 at 1.3600 | AR-FX-01, AR-TIME-02 |
| Placement | INV-6003 | W5 | 3,000.00 | due date | firm | — | AR-DISC-01, AR-TIME-02 |

- **No other outputs:** no Decision Items, and the run is not Provisional.

---

## AR-S07 — Unapplied credit and a customer credit balance

Concern: credit a customer already paid or was given reduces what they will pay, and credit with
nothing to apply to is owed back, not received.

**Facts.** No paid history.

| Invoice | Customer | Due | Open |
|---|---|---|---|
| INV-7001 | Cypress Bay Dental | 10-10 | 1,000.00 |
| INV-7002 | Cypress Bay Dental | 10-24 | 1,500.00 |

Customer credit, none of it applied to anything:

| Item | Customer | Kind | Dated | Unapplied |
|---|---|---|---|---|
| OP-7001 | Cypress Bay Dental | overpayment received | 09-29 | 400.00 |
| CN-7101 | Fernwood Café | credit note issued | 09-18 | 300.00 |

Fernwood Café has no open invoices.

**Expected**

| Output | Item | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|
| Placement | INV-7001 | W1 | 600.00 (1,000.00 less OP-7001 400.00) | due date | firm | AR-UNAPPLIED-01, AR-TIME-02 |
| Placement | INV-7002 | W3 | 1,500.00 | due date | firm | AR-TIME-02 |
| Exclusion | CN-7101 | — | 300.00 | credit balance | — | AR-UNAPPLIED-02 |

| Decision Item | Subject | Acted on by | Draft Correction | Rules |
|---|---|---|---|---|
| apply credit | OP-7001 | bookkeeper | Payment application: 400.00 of OP-7001 to INV-7001 | AR-UNAPPLIED-01 |
| refund or apply credit | CN-7101 | owner | — | AR-UNAPPLIED-02 |

- **Provisional:** no.

---

## AR-S08 — Scheduled invoices: already generated, still to come, and missed

Concern: an invoice schedule forecasts future billing without counting an invoice twice, and a
schedule nobody acted on is noticed.

**Facts.** No paid history.

| Template | Customer | Mode | Amount | Every | Starting | Due rule |
|---|---|---|---|---|---|---|
| T1 | Granite Peak Fitness | automatic | 840.00 | month | 10-15 | 15 days after invoice date |
| T2 | Northshore Rowing Club | reminder | 525.00 | month | 08-01 | 30 days after invoice date |

INV-8001 was generated ahead of its occurrence:

| Invoice | Customer | Status | Dated | Due | Total | Open | Template |
|---|---|---|---|---|---|---|---|
| INV-8001 | Granite Peak Fitness | posted | 10-15 | 10-30 | 840.00 | 840.00 | T1 |

No invoice to Northshore Rowing Club is dated after 09-01.

**Expected**

| Output | Item | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|
| Placement | INV-8001 | W4 | 840.00 | due date | firm | AR-OPEN-01, AR-TIME-02 |
| Exclusion | T1 occurrence 10-15 | — | 840.00 | covered by INV-8001 | — | AR-SCHED-01 |
| Placement | T1 occurrence 11-15 | W8 (due 11-30) | 840.00 | scheduled invoice | estimated | AR-SCHED-01 |
| Placement | T1 occurrence 12-15 | W13 (due 12-30) | 840.00 | scheduled invoice | estimated | AR-SCHED-01 |
| Exclusion | T2 | — | — | template not automatic | — | AR-SCHED-01 |

| Decision Item | Subject | Acted on by | Rules |
|---|---|---|---|
| invoice missed | T2, occurrence 10-01 | bookkeeper | AR-SCHED-02 |

- **T1's first occurrence is 10-15,** so it has no past occurrence to be missed.
- **T1's 2027-01-15 occurrence** falls after the horizon.
- **Provisional:** no.

---

## AR-S09 — Accrued revenue and AR that doesn't agree with the ledger

Concern: revenue earned but not billed is pointed out, and a control account that disagrees
with the open invoices makes the forecast Provisional.

**Facts.** No paid history.

| Invoice | Customer | Dated | Due | Open |
|---|---|---|---|---|
| INV-9001 | Kingsway Medical Clinic | 09-19 | 10-19 | 6,000.00 |
| INV-9002 | Kingsway Medical Clinic | 09-26 | 10-26 | 6,000.00 |

Account balances at the end of the last completed month:

| Account | Classification | As of | Balance |
|---|---|---|---|
| Accounts Receivable | AR control | 09-30 | 12,350.00 |
| Unbilled Revenue | unbilled receivable | 09-30 | 4,200.00 |

**Expected**

| Output | Invoice | Week | Amount | Basis | Confidence | Rules |
|---|---|---|---|---|---|---|
| Placement | INV-9001 | W2 | 6,000.00 | due date | firm | AR-TIME-02 |
| Placement | INV-9002 | W3 | 6,000.00 | due date | firm | AR-TIME-02 |

| Decision Item | Subject | Acted on by | Evidence | Rules |
|---|---|---|---|---|
| reconcile AR | Accounts Receivable, September 2026 | bookkeeper | Open invoices 12,000.00 against control 12,350.00; difference 350.00 | AR-TIE-01 |
| invoice accrued revenue | Unbilled Revenue, September 2026 | bookkeeper | Balance 4,200.00 | AR-UNBILLED-01 |

- **Provisional:** yes. Reason: AR-TIE-01.

---

## AR-S10 — An invoice fully offset by credit

Concern: a credit that covers a whole invoice leaves nothing to collect on it, and the rest of the
credit reduces the customer's next invoice.

**Facts.** No paid history.

| Invoice | Customer | Due | Open |
|---|---|---|---|
| INV-10001 | Cypress Bay Dental | 10-09 | 700.00 |
| INV-10002 | Cypress Bay Dental | 10-23 | 1,200.00 |

Customer credit, none of it applied to anything:

| Item | Customer | Kind | Dated | Unapplied |
|---|---|---|---|---|
| CN-10101 | Cypress Bay Dental | credit note issued | 09-30 | 1,000.00 |

**Expected**

| Output | Item | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|
| Exclusion | INV-10001 | — | 700.00 | offset by credit (CN-10101) | — | AR-UNAPPLIED-01 (Q148) |
| Placement | INV-10002 | W3 | 900.00 (1,200.00 less CN-10101's remaining 300.00) | due date | firm | AR-UNAPPLIED-01, AR-TIME-02 |

| Decision Item | Subject | Acted on by | Draft Correction | Rules |
|---|---|---|---|---|
| apply credit | CN-10101 | bookkeeper | Payment application: 700.00 of CN-10101 to INV-10001, 300.00 to INV-10002 | AR-UNAPPLIED-01 |

- **INV-10001 is not a zero-amount Placement** (Q148). The exclusion keeps it visible with the
  credit that offsets it.
- **Provisional:** no.

---

## AR-S11 — A scheduled invoice timed by collection history

Concern: an invoice the Entity hasn't issued yet is timed by how its customers pay, counted from
the day it will be issued, since it has no age on the run date.

**Facts.** Paid history, each invoice 1,000.00 on net 30 terms, paid in full in one payment:

| Invoice | Customer | Dated | Due | Paid | Days after invoice date |
|---|---|---|---|---|---|
| H-11101 | Cedar Point Property Mgmt. | 05-31 | 06-30 | 07-10 | 40 |
| H-11102 | Westgate Business Park | 07-01 | 07-31 | 08-20 | 50 |
| H-11103 | Granite Peak Fitness | 08-01 | 08-31 | 09-05 | 35 |
| H-11104 | Riverbend Elementary PAC | 08-16 | 09-15 | 10-05 | 50 |

No open invoices.

| Template | Customer | Mode | Amount | Every | Starting | Due rule |
|---|---|---|---|---|---|---|
| T1 | Granite Peak Fitness | automatic | 1,000.00 | month | 10-20 | 30 days after invoice date |

No invoice is linked to T1, and none to Granite Peak Fitness is dated after 08-01.

**Expected**

| Output | Item | Week | Amount | Basis / reason | Median | Comparable | Confidence | Rules |
|---|---|---|---|---|---|---|---|---|
| Placement | T1 occurrence 10-20 | W8 (11-29) | 1,000.00 | scheduled invoice | 40 days | 4 | estimated | AR-SCHED-01, AR-TIME-02 (Q151) |
| Placement | T1 occurrence 11-20 | W13 (12-30) | 1,000.00 | scheduled invoice | 40 days | 4 | estimated | AR-SCHED-01, AR-TIME-02 (Q151) |
| Exclusion | T1 occurrence 12-20 | — | 1,000.00 | beyond horizon (2027-01-29) | 40 days | 4 | — | AR-SCHED-01, AR-TIME-03 (Q151) |

**How the median comes out.** Each occurrence is taken as open on its scheduled invoice date, at
age −30. All four H invoices were unpaid at −30 and were collected 35, 40, 50 and 50 days later.
At least half (2 of 4) were collected first at 40 days (Q147), so each occurrence is placed 40 days
after its invoice date.
- **With no history** (AR-S08), the occurrences would be placed at their due dates: 11-19 (W7),
  12-20 (W11), and 2027-01-19, after the horizon.
- **No Decision Items:** T1's first date is 10-20, so no past occurrence can be missed.
- **Provisional:** no.

---

## Coverage

| Rule | Scenarios |
|---|---|
| AR-OPEN-01 | S01, S08 |
| AR-OPEN-02 | S01 |
| AR-OPEN-03 | S06 |
| AR-OPEN-04 | S01 |
| AR-TIME-01 | S02 |
| AR-TIME-02 | S01, S03, S06, S07, S08, S09, S10, S11 |
| AR-TIME-03 | S04, S05, S11 |
| AR-TIME-04 | S02 |
| AR-SCHED-01 | S08, S11 |
| AR-SCHED-02 | S08 |
| AR-FX-01 | S06 |
| AR-DISC-01 | S06 |
| AR-UNAPPLIED-01 | S07, S10 |
| AR-UNAPPLIED-02 | S07 |
| AR-COLLECT-01 | S02, S03, S04, S05 |
| AR-WRITEOFF-01 | S04 |
| AR-UNBILLED-01 | S09 |
| AR-TIE-01 | S09 |

## Questions the Scenarios raised

Writing exact expectations exposed Rule wording that allowed more than one answer. All five were
decided by the owner on 2026-09-14 and written into the Rules.

- **Q147 — The median, precisely** (AR-TIME-02, AR-TIME-03). The median is the first number of
  days by which at least half the comparable invoices were collected (the lower middle for an
  even count), and an invoice still open counts as not collected at every point. Rejected: a
  survival-curve estimate, which is harder to explain and times receipts earlier.
- **Q148 — An invoice fully offset by credit** (AR-UNAPPLIED-01, and AP-UNAPPLIED-01 to match).
  An exclusion, reason "offset by credit", citing the credit; the "apply credit" item still
  stands.
- **Q149 — Which occurrence a document covers** (AR-SCHED-01, AP-SCHED-01, GAP-SCHED-02,
  CASH-SCHED-01). The nearest scheduled date, with the frequency extended before the first date
  and past the end date; exactly halfway covers the earlier one. An invoice issued a day late
  covers its own occurrence, not the next.
- **Q150 — Two items for one stale expected date** (AR-TIME-01, AR-TIME-04, and AP-TIME-01 with
  AP-TIME-03 to match). One item: "set expected date", citing the passed date. S02 amended.
- **Q151 — Timing a scheduled invoice not yet issued** (AR-SCHED-01 with AR-TIME-02). Timed as if
  open on its scheduled invoice date, with its age taken on that date; with no history, at its
  scheduled due date.

Q148 is exercised by S10, Q151 by S11, and Q149 by CASH-S05 and GAP-S01.
