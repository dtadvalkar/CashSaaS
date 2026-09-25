# AP Scenarios

Status: **approved by the owner** (2026-09-14). Being built: each Scenario here becomes a test in
`scenarios/tests/ap.rs` as the build reaches it, and `tools/mutate.py ap` sweeps this document for
figures a test would not catch. This document is fixed input: when a test disagrees with it, the
code is wrong (`README.md`).

Each Scenario is a named general accounting concern, described as Canonical Facts and Settings,
with the outputs the AP Rules must produce (ADR-0010, ADR-0016). Together these exercise every
Rule in `docs/rules/ap.md`.

## Conventions

The conventions in `docs/scenarios/ar.md` apply: run date Wednesday 2026-10-07, a 13-week
horizon (W1 is 10-07 to 10-13, W13 is 12-30 to 2027-01-05), Maple Ridge Landscaping Ltd. in
CAD unless stated, an empty Entity at the start of each Scenario, and exhaustive expected outputs
for this Family. Also:

- **Bills are posted,** with no planned payment date, no pay-run day set and no vendor
  credits, unless stated.
- **Amounts are payments,** so they are shown without a sign.
- **Bank and card transactions** are dated in October unless stated, so no CLOSE check reads
  them.

---

## AP-S01 — Open, draft, voided, zero-total, paid and card-paid bills

Concern: a bill is owed until it is paid from the bank; a draft bill is still likely owed, a void
is not, a card-paid bill has moved to the card, and payments include tax.

**Facts**

| Bill | Vendor | Status | Due | Total | Tax in total | Open | Notes |
|---|---|---|---|---|---|---|---|
| BILL-1001 | Brandt Tractor Ltd. | posted | 10-20 | 3,150.00 | 150.00 | 3,150.00 | |
| BILL-1002 | Western Turf Farms | draft | 10-30 | 1,575.00 | 75.00 | 1,575.00 | |
| BILL-1003 | Home Depot Pro | voided | 09-15 | 0.00 | 0.00 | 0.00 | Voided in September |
| BILL-1004 | Telus Business | posted | 10-15 | 450.00 | 21.43 | 0.00 | Paid in full from RBC Business Chequing, early |
| BILL-1005 | Nutrien Ag Solutions | posted | 10-25 | 840.00 | 40.00 | 0.00 | Paid in full by RBC Visa Business, early |
| BILL-1006 | Brandt Tractor Ltd. | posted | 10-09 | 0.00 | 0.00 | 0.00 | Warranty part, no charge |

BILL-1004 was paid on 10-01 and BILL-1005 charged on 10-02, both before the run date.

**Expected**

| Output | Bill | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|
| Placement | BILL-1001 | W2 | 3,150.00 | due date | firm | AP-OPEN-01, AP-OPEN-04, AP-TIME-02 |
| Placement | BILL-1002 | W4 | 1,575.00 | due date | estimated | AP-TIME-02, AP-OPEN-02, AP-OPEN-04 |

| Decision Item | Subject | Acted on by | Rules |
|---|---|---|---|
| approve or delete bill | BILL-1002 | bookkeeper | AP-OPEN-02 |

- **BILL-1003** is voided and **BILL-1006** has a zero total, so both are ignored (Q145). Read
  as an open bill, BILL-1003 would be an overdue payment in W1.
- **BILL-1005** is closed in AP. Its amount is inside the card balance GAP-CARD-01 pays (Q103).
- **Provisional:** no.

---

## AP-S02 — Owner-planned payment dates, current and stale

Concern: the owner decides when a bill is paid, so a stated plan places it, and a plan that has
passed is not trusted.

**Facts**

| Bill | Vendor | Due | Open | Planned payment date |
|---|---|---|---|---|
| BILL-2001 | Stihl Canada | 11-05 | 2,400.00 | 10-28 |
| BILL-2002 | Brandt Tractor Ltd. | 10-30 | 1,800.00 | 10-02 |
| BILL-2003 | Western Turf Farms | 09-25 | 1,250.00 | 10-05 |
| BILL-2004 | Home Depot Pro | 09-30 | 680.00 | — |

**Expected**

| Output | Bill | Week | Amount | Basis | Confidence | Rules |
|---|---|---|---|---|---|---|
| Placement | BILL-2001 | W4 (10-28) | 2,400.00 | planned date | firm | AP-TIME-01, AP-OPEN-01, AP-OPEN-04 |
| Placement | BILL-2002 | W4 (10-30) | 1,800.00 | due date | firm | AP-TIME-02, AP-OPEN-01, AP-OPEN-04 |
| Placement | BILL-2003 | W1 | 1,250.00 | overdue | firm | AP-TIME-03, AP-OPEN-01, AP-OPEN-04 |
| Placement | BILL-2004 | W1 | 680.00 | overdue | firm | AP-TIME-03, AP-OPEN-01, AP-OPEN-04 |

| Decision Item | Subject | Evidence | Acted on by | Rules |
|---|---|---|---|---|
| planned date passed | BILL-2002 | planned 10-02 | owner | AP-TIME-01 |
| overdue bill | BILL-2003 | due 09-25; planned 10-05, passed | owner | AP-TIME-03 (Q150) |
| overdue bill | BILL-2004 | due 09-30 | owner | AP-TIME-03 |

- **BILL-2003 gets one item, not two** (Q150).
- **Provisional:** no.

---

## AP-S03 — A weekly pay run

Concern: a business that pays bills on one weekday pays each bill in the last run before it falls
due, never late, and pays overdue bills in the next run. The same weekday moves scheduled bills.

**Facts.** Settings: pay-run weekday Thursday. Runs fall on 10-08, 10-15, 10-22, 10-29, 11-05 and
every 7 days after. BILL-3001 falls due on a Tuesday, BILL-3002 on a Thursday, BILL-3005 on a
Monday and BILL-3006 on the run date, a Wednesday; BILL-3004's planned date is a Friday. T1 is a
scheduled bill the pay run also moves.

| Bill | Vendor | Due | Open | Planned payment date |
|---|---|---|---|---|
| BILL-3001 | Stihl Canada | 10-20 | 2,260.00 | — |
| BILL-3002 | Brandt Tractor Ltd. | 10-22 | 3,900.00 | — |
| BILL-3003 | Telus Business | 10-01 | 450.00 | — |
| BILL-3004 | Western Turf Farms | 11-10 | 1,100.00 | 11-06 |
| BILL-3005 | Home Depot Pro | 10-12 | 915.00 | — |
| BILL-3006 | Nutrien Ag Solutions | 10-07 | 1,340.00 | — |

| Template | Vendor | Mode | Amount | Every | Starting | Due rule |
|---|---|---|---|---|---|---|
| T1 | Waste Connections of Canada | automatic | 610.00 | month | 11-08 | 10 days after bill date |

**Expected**

| Output | Bill | Week | Amount | Basis | Confidence | Rules |
|---|---|---|---|---|---|---|
| Placement | BILL-3001 | W2 (10-15) | 2,260.00 | pay run | firm | AP-TIME-02, AP-RUN-01, AP-OPEN-01, AP-OPEN-04 |
| Placement | BILL-3002 | W3 (10-22) | 3,900.00 | pay run | firm | AP-TIME-02, AP-RUN-01, AP-OPEN-01, AP-OPEN-04 |
| Placement | BILL-3003 | W1 (10-08) | 450.00 | pay run | firm | AP-TIME-03, AP-RUN-01, AP-OPEN-01, AP-OPEN-04 |
| Placement | BILL-3004 | W5 (11-06) | 1,100.00 | planned date | firm | AP-TIME-01, AP-OPEN-01, AP-OPEN-04 |
| Placement | BILL-3005 | W1 (10-08) | 915.00 | pay run | firm | AP-TIME-02, AP-RUN-01, AP-OPEN-01, AP-OPEN-04 |
| Placement | BILL-3006 | W1 (10-08) | 1,340.00 | pay run | firm | AP-TIME-02, AP-RUN-01, AP-OPEN-01, AP-OPEN-04 (Q156) |
| Placement | T1 occurrence 11-08 | W6 (due 11-12) | 610.00 | scheduled bill | estimated | AP-SCHED-01, AP-RUN-01 |
| Placement | T1 occurrence 12-08 | W11 (due 12-17) | 610.00 | scheduled bill | estimated | AP-SCHED-01, AP-RUN-01 |

| Week | Opens | Receipts | Payments | Closes | Firm | Estimated |
|---|---|---|---|---|---|---|
| W1 | 0.00 | — | 2,705.00 | −2,705.00 | 100% | 0% |
| W2 | −2,705.00 | — | 2,260.00 | −4,965.00 | 100% | 0% |
| W3 | −4,965.00 | — | 3,900.00 | −8,865.00 | 100% | 0% |
| W5 | −8,865.00 | — | 1,100.00 | −9,965.00 | 100% | 0% |
| W6 | −9,965.00 | — | 610.00 | −10,575.00 | 0% | 100% |
| W11 | −10,575.00 | — | 610.00 | −11,185.00 | 0% | 100% |

| Decision Item | Subject | Acted on by | Rules |
|---|---|---|---|
| overdue bill | BILL-3003 | owner | AP-TIME-03 |

- **BILL-3005** moves into an earlier week than its due date: the next Thursday run would be late.
- **BILL-3004's planned date** is not moved to a Thursday.
- **BILL-3006 is due today.** Its last run on or before the run date is already past, so it is
  paid in the first run from the run date (Q156). It isn't overdue, so it raises no item.
- **T1's due dates** follow its due rule; the pay run places them on the Thursdays in the Output
  table.
- **Provisional:** no.

---

## AP-S04 — Scheduled bills: generated, paid by card, still to come, and reminder-only

Concern: a bill schedule forecasts payments not yet billed, without paying twice for an
occurrence that is already a bill or already charged to a card.

**Facts.** T1 is a Xero repeating bill. T2 is a QBO automated recurring bill, and QBO records no
link from a generated bill to it. T3 is a QBO recurring bill for an equipment lease.

| Template | Vendor | Mode | Amount | Every | Starting | Due rule |
|---|---|---|---|---|---|---|
| T1 | Telus Business | automatic | 450.00 | month | 06-10 | 20 days after bill date |
| T2 | Waste Connections of Canada | automatic | 610.00 | month | 01-08 | 10 days after bill date |
| T3 | Kubota Canada Ltd. | reminder | 1,280.00 | month | 01-01 | 30 days after bill date |

BILL-4001 was generated ahead of its occurrence:

| Bill | Vendor | Status | Dated | Due | Total | Open | Template |
|---|---|---|---|---|---|---|---|
| BILL-4001 | Telus Business | posted | 10-10 | 10-30 | 450.00 | 450.00 | T1 |

The vendor's autopay, charged to the card:

| Transaction | Account | Payee | Dated | Amount |
|---|---|---|---|---|
| CARD-4101 | RBC Visa Business | Waste Connections of Canada | 10-06 | 610.00 |

**Expected**

| Output | Item | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|
| Placement | BILL-4001 | W4 | 450.00 | due date | firm | AP-TIME-02, AP-OPEN-01, AP-OPEN-04 |
| Exclusion | T1 occurrence 10-10 | — | 450.00 | covered by BILL-4001 | — | AP-SCHED-01 |
| Placement | T1 occurrence 11-10 | W8 (due 11-30) | 450.00 | scheduled bill | estimated | AP-SCHED-01 |
| Placement | T1 occurrence 12-10 | W13 (due 12-30) | 450.00 | scheduled bill | estimated | AP-SCHED-01 |
| Exclusion | T2 occurrence 10-08 | — | 610.00 | covered by card charge 10-06 | — | AP-SCHED-01 |
| Placement | T2 occurrence 11-08 | W7 (due 11-18) | 610.00 | scheduled bill | estimated | AP-SCHED-01 |
| Placement | T2 occurrence 12-08 | W11 (due 12-18) | 610.00 | scheduled bill | estimated | AP-SCHED-01 |
| Exclusion | T3 | — | — | template not automatic | — | AP-SCHED-01 |

| Week | Opens | Receipts | Payments | Closes | Firm | Estimated |
|---|---|---|---|---|---|---|
| W4 | 0.00 | — | 450.00 | −450.00 | 100% | 0% |
| W7 | −450.00 | — | 610.00 | −1,060.00 | 0% | 100% |
| W8 | −1,060.00 | — | 450.00 | −1,510.00 | 0% | 100% |
| W11 | −1,510.00 | — | 610.00 | −2,120.00 | 0% | 100% |
| W13 | −2,120.00 | — | 450.00 | −2,570.00 | 0% | 100% |

- **The card charge** is nearer T2's October occurrence than its September one, so it covers
  that occurrence (Q149). Its cash is in the card balance.
- **T1's and T2's January occurrences** fall after the horizon.
- **No Decision Items, not Provisional.**

---

## AP-S05 — A committed purchase, a foreign-currency bill and an intercompany bill

Concern: a purchase order drains cash only once its timing is known, a foreign bill is paid at
its booked rate, and a sister Entity's bill is not external cash.

**Facts.** The Group also holds Birch Hill Nursery Ltd. Its contact in Maple Ridge is classified
intercompany, naming Birch Hill. PO-5002 has no delivery date, as QBO holds none. Neither purchase
order is billed.

| Purchase order | Vendor | Status | Total | Delivery date | Vendor terms |
|---|---|---|---|---|---|
| PO-5001 | Brandt Tractor Ltd. | authorised | 18,500.00 | 10-20 | net 30 |
| PO-5002 | Stihl Canada | authorised | 4,200.00 | — | — |

| Bill | Vendor | Currency | Due | Open (document) | Booked rate | Open (CAD) |
|---|---|---|---|---|---|---|
| BILL-5101 | Pacific Growers Supply LLC | USD | 10-29 | 2,000.00 | 1.3700 | 2,740.00 |
| BILL-5102 | Birch Hill Nursery Ltd. | CAD | 10-16 | 3,000.00 | — | 3,000.00 |

**Expected**

| Output | Item | Week | Amount (CAD) | Basis / reason | Confidence | Recorded on Placement | Rules |
|---|---|---|---|---|---|---|---|
| Placement | PO-5001 | W7 (11-19) | 18,500.00 | purchase order | estimated | — | AP-PO-01 |
| Exclusion | PO-5002 | — | 4,200.00 | committed purchase, timing unknown | — | — | AP-PO-01 |
| Placement | BILL-5101 | W4 | 2,740.00 | due date | firm | USD 2,000.00 at 1.3700 | AP-TIME-02, AP-OPEN-01, AP-OPEN-04, AP-FX-01 |
| Exclusion | BILL-5102 | — | 3,000.00 | intercompany, see IC | — | — | AP-OPEN-03, AP-OPEN-01 |

| Week | Opens | Receipts | Payments | Closes | Firm | Estimated |
|---|---|---|---|---|---|---|
| W2 | 0.00 | — | 3,000.00 | −3,000.00 | 100% | 0% |
| W4 | −3,000.00 | — | 2,740.00 | −5,740.00 | 100% | 0% |
| W7 | −5,740.00 | — | 18,500.00 | −24,240.00 | 0% | 100% |

- **W2's payment** is IC-ONESIDED-01 placing BILL-5102 (AP excluded it; IC placed it).
- **W7's payment** is PO-5001 (AP-PO-01). W4 is BILL-5101.
- **No Decision Items, not Provisional.**

---

## AP-S06 — Early-payment discounts: assumed not taken, suggested when affordable

Concern: a discount is the owner's choice, so the forecast pays the full amount on time, but a
discount the cash can bear is pointed out with what skipping it costs.

**Facts.** Settings: Minimum Cash Buffer 5,000.00.

| Account | Classification | As of | Balance |
|---|---|---|---|
| RBC Business Chequing | bank | 10-07 | 25,000.00 |

| Bill | Vendor | Dated | Terms | Discount date | Due | Open |
|---|---|---|---|---|---|---|
| BILL-6101 | Nutrien Ag Solutions | 10-06 | 2% 10 days, net 30 | 10-16 | 11-05 | 10,000.00 |
| BILL-6102 | Brandt Tractor Ltd. | 10-02 | 1% 15 days, net 45 | 10-17 | 11-16 | 15,000.00 |
| BILL-6103 | Stihl Canada | 09-20 | 2% 10 days, net 30 | 09-30 | 10-20 | 1,000.00 |

One receipt, an open invoice:

| Invoice | Customer | Due | Open |
|---|---|---|---|
| INV-6001 | Harbourview Strata Corp. | 10-23 | 12,000.00 |

**Expected**

| Output | Bill | Week | Amount | Basis | Confidence | Rules |
|---|---|---|---|---|---|---|
| Placement | BILL-6101 | W5 | 10,000.00 | due date | firm | AP-TIME-02, AP-OPEN-01, AP-OPEN-04, AP-DISC-01 |
| Placement | BILL-6102 | W6 | 15,000.00 | due date | firm | AP-TIME-02, AP-OPEN-01, AP-OPEN-04, AP-DISC-01 |
| Placement | BILL-6103 | W2 | 1,000.00 | due date | firm | AP-TIME-02, AP-OPEN-01, AP-OPEN-04, AP-DISC-01 |

| Decision Item | Subject | Evidence | Acted on by | Rules |
|---|---|---|---|---|
| take discount | BILL-6101 | Pay 9,800.00 by 10-16; saving 200.00; implied annual rate of not taking it 37.24% | owner | AP-DISC-02 |

| Week | Opens | Receipts | Payments | Closes | Firm | Estimated |
|---|---|---|---|---|---|---|
| W2 | 25,000.00 | — | 1,000.00 | 24,000.00 | 100% | 0% |
| W3 | 24,000.00 | 12,000.00 | — | 36,000.00 | 100% | 0% |
| W5 | 36,000.00 | — | 10,000.00 | 26,000.00 | 100% | 0% |
| W6 | 26,000.00 | — | 15,000.00 | 11,000.00 | 100% | 0% |

**How affordability comes out.** Base closing cash is the weeks table (CASH-ROLL-01); lowest at
W6.
- **BILL-6101 first,** by the earlier discount date (Q157). Paying the discounted amount in the
  discount week instead of full on the due date keeps every week at or above the buffer, so the
  item is raised (Evidence).
- **BILL-6102 is then judged with BILL-6101's discount taken.** Paying both discounts in the same
  week breaches zero, so no item. Judged alone it would have passed.
- **BILL-6103's discount date** has passed, so no item.
- **The implied rate** is discount ÷ (1 − discount) × 365 ÷ (days between the discount date and
  the due date); Evidence carries the percentage.
- **Provisional:** no.

---

## AP-S07 — Unapplied vendor credit and a supplier prepayment

Concern: credit a vendor already owes the Entity reduces what the Entity will pay, and credit with
nothing to apply to is not a receipt.

**Facts**

| Bill | Vendor | Due | Open |
|---|---|---|---|
| BILL-7001 | Western Turf Farms | 10-12 | 900.00 |
| BILL-7002 | Western Turf Farms | 10-26 | 1,400.00 |

Vendor credit, none of it applied to anything:

| Item | Vendor | Kind | Dated | Unapplied |
|---|---|---|---|---|
| VC-7101 | Western Turf Farms | vendor credit issued | 09-28 | 1,200.00 |
| PP-7201 | Kubota Canada Ltd. | prepayment paid | 10-01 | 2,500.00 |

Kubota Canada Ltd. has no open bills.

**Expected**

| Output | Item | Week | Amount | Basis / reason | Confidence | Rules |
|---|---|---|---|---|---|---|
| Exclusion | BILL-7001 | — | 900.00 | offset by credit (VC-7101) | — | AP-UNAPPLIED-01, AP-OPEN-01 (Q148) |
| Placement | BILL-7002 | W3 | 1,100.00 (1,400.00 less VC-7101's remaining 300.00) | due date | firm | AP-TIME-02, AP-OPEN-01, AP-OPEN-04, AP-UNAPPLIED-01 |
| Exclusion | PP-7201 | — | 2,500.00 | vendor credit balance | — | AP-UNAPPLIED-02 |

| Decision Item | Subject | Acted on by | Draft Correction | Rules |
|---|---|---|---|---|
| apply vendor credit | VC-7101 | bookkeeper | Payment application: 900.00 of VC-7101 to BILL-7001, 300.00 to BILL-7002 | AP-UNAPPLIED-01 |
| claim refund or hold credit | PP-7201 | owner | — | AP-UNAPPLIED-02 |

- **Provisional:** no.

---

## AP-S08 — Government trust, critical vendors and secured lenders are marked

Concern: what is owed to a government, or to a vendor the business can't lose, is named so the
owner sees it first, without moving any payment.

**Facts.** The contact Receiver General for Canada is classified government trust. Settings:
Nutrien Ag Solutions marked critical vendor; Kubota Credit Corporation Canada marked secured
lender.

| Bill | Vendor | Due | Open |
|---|---|---|---|
| RG-0915 | Receiver General for Canada | 10-15 | 3,400.00 |
| NAS-8101 | Nutrien Ag Solutions | 09-30 | 1,200.00 |
| KCC-8102 | Kubota Credit Corporation Canada | 10-25 | 1,850.00 |
| HD-8103 | Home Depot Pro | 10-21 | 700.00 |

RG-0915 is September's source deductions; KCC-8102 is an equipment-financing instalment.

**Expected**

| Output | Bill | Week | Amount | Basis | Confidence | Priority mark | Rules |
|---|---|---|---|---|---|---|---|
| Placement | RG-0915 | W2 | 3,400.00 | due date | firm | government trust | AP-TIME-02, AP-OPEN-01, AP-OPEN-04, AP-PRIORITY-01 |
| Placement | NAS-8101 | W1 | 1,200.00 | overdue | firm | critical vendor | AP-TIME-03, AP-OPEN-01, AP-OPEN-04, AP-PRIORITY-01 |
| Placement | KCC-8102 | W3 | 1,850.00 | due date | firm | secured lender | AP-TIME-02, AP-OPEN-01, AP-OPEN-04, AP-PRIORITY-01 |
| Placement | HD-8103 | W3 | 700.00 | due date | firm | — | AP-TIME-02, AP-OPEN-01, AP-OPEN-04 |

| Decision Item | Subject | Priority mark | Acted on by | Rules |
|---|---|---|---|---|
| overdue bill | NAS-8101 | critical vendor | owner | AP-TIME-03, AP-PRIORITY-01 |

- **Every payment stays on its date;** the marks only change the queue (CASH-S08).
- **Provisional:** no.

---

## AP-S09 — Possible duplicates, and a bill possibly paid another way

Concern: a bill entered twice, or paid from the bank without being matched, overstates what is
owed; both are flagged and kept in the forecast until someone checks.

**Facts**

| Bill | Vendor | Vendor reference | Dated | Due | Open |
|---|---|---|---|---|---|
| BILL-9001 | Home Depot Pro | HD-55120 | 09-28 | 10-28 | 1,130.00 |
| BILL-9002 | Home Depot Pro | HD-55120 | 09-29 | 10-29 | 1,130.00 |
| BILL-9003 | Stihl Canada | ST-7781 | 10-01 | 10-31 | 2,260.00 |
| BILL-9004 | Stihl Canada | ST-7790 | 10-01 | 10-31 | 2,260.00 |
| BILL-9005 | Telus Business | TB-1025 | 09-25 | 10-25 | 450.00 |
| BILL-9006 | Brandt Tractor Ltd. | BT-6612 | 10-02 | 11-01 | 900.00 |
| BILL-9007 | Western Turf Farms | WT-3310 | 09-22 | 10-22 | 800.00 |

Bank spend transactions, none recorded as a bill payment:

| Transaction | Account | Payee | Dated | Amount |
|---|---|---|---|---|
| BANK-9101 | RBC Business Chequing | Telus Business | 10-02 | 450.00 |
| BANK-9102 | RBC Business Chequing | Brandt Tractor Ltd. | 10-01 | 900.00 |
| BANK-9103 | RBC Business Chequing | Western Turf Farms | 10-03 | 850.00 |

**Expected**

| Output | Bill | Week | Amount | Basis | Confidence | Rules |
|---|---|---|---|---|---|---|
| Placement | BILL-9001 | W4 | 1,130.00 | due date | firm | AP-TIME-02, AP-OPEN-01, AP-OPEN-04, AP-DUP-01 |
| Placement | BILL-9002 | W4 | 1,130.00 | due date | firm | AP-TIME-02, AP-OPEN-01, AP-OPEN-04, AP-DUP-01 |
| Placement | BILL-9003 | W4 | 2,260.00 | due date | firm | AP-TIME-02, AP-OPEN-01, AP-OPEN-04, AP-DUP-01 |
| Placement | BILL-9004 | W4 | 2,260.00 | due date | firm | AP-TIME-02, AP-OPEN-01, AP-OPEN-04, AP-DUP-01 |
| Placement | BILL-9005 | W3 | 450.00 | due date | firm | AP-TIME-02, AP-OPEN-01, AP-OPEN-04, AP-PAID-01 |
| Placement | BILL-9006 | W4 | 900.00 | due date | firm | AP-TIME-02, AP-OPEN-01, AP-OPEN-04 |
| Placement | BILL-9007 | W3 | 800.00 | due date | firm | AP-TIME-02, AP-OPEN-01, AP-OPEN-04 |

| Decision Item | Subject | Evidence | Acted on by | Rules |
|---|---|---|---|---|
| possible duplicate bill | BILL-9001, BILL-9002 | same vendor reference | bookkeeper | AP-DUP-01 |
| possible duplicate bill | BILL-9003, BILL-9004 | same amount and bill date | bookkeeper | AP-DUP-01 |
| bill may already be paid | BILL-9005, bank spend 10-02 | same vendor and amount, on or after the bill date | bookkeeper | AP-PAID-01 |

- **BILL-9006** is not flagged: the spend is before the bill date.
- **BILL-9007** is not flagged: the amounts differ. No near-match is assumed.
- **Provisional:** no.

---

## AP-S10 — Open AP that doesn't agree with the ledger

Concern: a control account that disagrees with the open bills means something went through AP
that the forecast can't see, so the forecast is Provisional.

**Facts**

| Bill | Vendor | Dated | Due | Total | Open | Paid |
|---|---|---|---|---|---|---|
| BILL-10001 | Brandt Tractor Ltd. | 09-10 | 10-15 | 4,000.00 | 4,000.00 | — |
| BILL-10002 | Stihl Canada | 09-22 | 10-22 | 2,500.00 | 2,500.00 | — |
| BILL-10003 | Home Depot Pro | 09-15 | 10-15 | 1,000.00 | 0.00 | 10-03 |

Account balance at the end of the last completed month:

| Account | Classification | As of | Balance |
|---|---|---|---|
| Accounts Payable | AP control | 09-30 | 7,850.00 |

**Expected**

| Output | Bill | Week | Amount | Basis | Confidence | Rules |
|---|---|---|---|---|---|---|
| Placement | BILL-10001 | W2 | 4,000.00 | due date | firm | AP-TIME-02, AP-OPEN-01, AP-OPEN-04 |
| Placement | BILL-10002 | W3 | 2,500.00 | due date | firm | AP-TIME-02, AP-OPEN-01, AP-OPEN-04 |

| Decision Item | Subject | Acted on by | Evidence | Rules |
|---|---|---|---|---|
| reconcile AP | Accounts Payable, September 2026 | bookkeeper | Open bills at 09-30 7,500.00 against control 7,850.00; difference 350.00 | AP-TIE-01 |

- **BILL-10003 counts at month-end** (open then) but is not a payment, because it was paid before
  the run date.
- **Provisional:** yes. Reason: AP-TIE-01.

---

## Coverage

| Rule | Scenarios |
|---|---|
| AP-OPEN-01 | S01, S04, S10 |
| AP-OPEN-02 | S01 |
| AP-OPEN-03 | S05 |
| AP-OPEN-04 | S01 |
| AP-TIME-01 | S02, S03 |
| AP-TIME-02 | S01–S10 |
| AP-TIME-03 | S02, S03, S08 |
| AP-RUN-01 | S03 |
| AP-SCHED-01 | S03, S04 |
| AP-PO-01 | S05 |
| AP-FX-01 | S05 |
| AP-DISC-01 | S06 |
| AP-DISC-02 | S06 |
| AP-UNAPPLIED-01 | S07 |
| AP-UNAPPLIED-02 | S07 |
| AP-PRIORITY-01 | S08 |
| AP-DUP-01 | S09 |
| AP-PAID-01 | S09 |
| AP-TIE-01 | S10 |

## Questions the Scenarios raised

Both were decided by the owner on 2026-09-14 as picked below and written into the Rules.

- **Q156 — A pay run that has already passed** (AP-RUN-01). A bill whose last run before its due
  date is past is paid in the first run on or after the run date, with no overdue item (S03,
  BILL-3006).
- **Q157 — Several affordable discounts** (AP-DISC-02). Judged in discount-date order, each with
  the earlier suggested discounts taken (ties: larger saving first), so the items can be acted on
  together (S06).
