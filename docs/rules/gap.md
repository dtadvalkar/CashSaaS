# GAP — Obligations that are not open bills

Status: **approved by the owner** (2026-09-14). Built: every Rule here is implemented in
`core/src/gap.rs` and covered by a Scenario test, with coverage gated against the table in
`docs/scenarios/gap.md` (`scenarios/tests/coverage.rs`). GAP-CARD-01 and GAP-SCHED-01 were
written a Family early for CASH (Q275); this Family's tests now gate them too.

What this Family does: forecasts the cash obligations that are not open bills: payroll, tax
remittances and instalments, loan and lease payments, rent and subscriptions paid without a
bill, card balances and accruals. It also explains what it could not forecast and why.

Evidence: `docs/research/2026-09-13-gap-general-practice.md` (cited `[gap: Part n]`), with the
AP (`[ap: ...]`) and ledger (`[ledger: ...]`) research notes of the same date. Entry format
follows ADR-0009, including Confidence and the role that acts on each Decision Item (ADR-0020).
Every Rule here is Universal (ADR-0017).

**The shape of this Family.** The ledgers hold the *liability* but never the *schedule*
`[gap: Summary 3–4]`. So obligations come from three sources:
- **Booked balances:** tax liabilities, card balances and accruals, as of a date (ADR-0014).
- **Calendars:** statutory Reference Data, with which calendar applies selected per Entity
  (ADR-0007).
- **Owner-entered schedules:** payroll, loans, leases, rent and subscriptions.

Where a needed Setting is missing, the obligation is excluded and the run marked Provisional,
never guessed (ADR-0019).

## Decisions behind these Rules

Decided by the owner on 2026-09-13 and recorded in `docs/plans/design-session-2026-09-13.md`.
Q224–Q228 were decided on 2026-09-16 from the UK pass (`docs/research/2026-09-16-sandbox-uk-pass.md`,
cited `[uk: ...]`) and are recorded in `docs/plans/design-session-2026-09-16.md`. Q285 (CRA
remitter calendars complete in M1) was decided on 2026-09-23 after the model-quality Assessor
found Regular-only Reference Data under-scoped ADR-0007; recorded in
`docs/plans/design-session-2026-09-23.md`.

| # | Question | Decision | Rules |
|---|---|---|---|
| Q110 | What a Gap is | Broaden it: any cash obligation the forecast must pay that is not an open bill, recurring or not, whether booked as a liability (cards, tax, accruals, loans) or not yet booked (the next payroll, the unbooked part of a tax period, a subscription paid by card). | all |
| Q111 | Where payroll comes from | Owner-entered payroll Settings (pay frequency, next pay date, expected net pay, expected remittance per run). Inferring them from bank and journal history belongs to the M2 onboarding proposer. | GAP-PAYROLL-01, 02 |
| Q112 | Remittance amounts | For a period that has ended: the liability balance at period end less payments made to the authority since. For a period still running: an estimate (Q119). | GAP-PAYROLL-03, GAP-TAX-01, 02 |
| Q113 | Which schedule applies | A Setting per Entity (remitter type, GST/HST period, state frequency, instalment basis). A value the ledger holds, such as Xero's `SalesTaxPeriod`, is used with Confidence estimated plus a "confirm" item until confirmed. | GAP-PAYROLL-04, GAP-TAX-03 |
| Q114 | Recoverable tax | No tax-rate mapping in M1. A booked GST/HST liability already nets recoverable tax as the ledger records it; future-period estimates use actual remittances. | GAP-TAX-01, 02 |
| Q115 | Tax that falls on the owner | Out of the Entity forecast, unless the owner enters it as a scheduled obligation because the Entity funds it. | GAP-SCHED-01 |
| Q116 | Loans and leases | A scheduled-obligation Setting (payee, amount, frequency, next date, end date). A loan balance with no schedule is excluded with an item, and the run is Provisional. | GAP-SCHED-01, GAP-LOAN-01 |
| Q117 | Recurring charges with no bill | M1 forecasts only owner-entered scheduled obligations and ledger templates. Detecting candidates from bank history is the M2 onboarding proposer's job, so no "how many occurrences" threshold enters the engine. | GAP-SCHED-01 |
| Q118 | When a Gap is covered | By a template link where one exists. Otherwise by a booked bill, card charge or bank spend to the obligation's payee within that occurrence's period, whatever the amount (the actual replaces the estimate). Near-matches are not assumed. | GAP-SCHED-02 |
| Q119 | Amount of an unbooked estimate | The owner-entered amount (Confidence firm), or else the last actual for a period of the same length (Confidence estimated). With neither, the obligation is excluded with an item and the run is Provisional. | GAP-TAX-02, GAP-PAYROLL-03 |
| Q120 | Timing of accruals | An optional per-account Setting for expected settlement. Unset: excluded with an item, and the run is Provisional, because leaving it out understates outflows. | GAP-ACCRUAL-01 |
| Q121 | Card payment amount | The card balance on the run date, paid on the card's payment day (Q85). The ledger holds no statement balance, and paying in full is the prudent assumption. | GAP-CARD-01 |
| Q122 | Calendar coverage in M1 | Canada federal (CRA payroll remittances for every remitter type — Q285 — GST/HST, corporate instalments) and US federal (IRS deposits, FUTA, corporate estimated tax), with each country's holiday list. Provinces and states use ADR-0007's "no calendar" path until added. | GAP-TAX-04, GAP-PAYROLL-03 |
| Q224 | Finding payments to the tax authority | QBO outside the US keeps VAT and GST/HST payments as their own record type (`TaxPayment`) with no payee. The Adapter reads them as payment facts marked as tax remittances, and Rules find payments to the authority by that mark, never by payee. | GAP-TAX-01, 02 |
| Q225 | Where the sales-tax liability sits | In two QBO accounts: `GlobalTaxPayable` for the running period and `GlobalTaxSuspense` for a filed return not yet paid. Both are classified as sales-tax liability and read together. | GAP-TAX-01 |
| Q226 | Tax accounting schemes | A Setting per Entity. Standard: the booked balance, Confidence firm. Any other scheme (cash accounting, flat rate, Canada's Quick Method), where the booked balance isn't the return: the last actual remittance for a period of the same length, Confidence estimated. Unset with no ledger basis: the standard basis, Confidence firm, no confirm item (Q284 Pick A). Unset when the ledger holds a basis: Q253. Not Provisional. | GAP-TAX-01, 02 |
| Q253 | A ledger-held tax basis | Xero's `SalesTaxBasis` prefills the tax accounting scheme with Confidence estimated and a "confirm" item, as `SalesTaxPeriod` prefills the period. Amends Q226 for Xero. | GAP-TAX-01, 02, 03 |
| Q227 | Calendar shape | Each calendar carries its weekend and holiday direction (HMRC VAT moves earlier; CRA and IRS move later) and its period boundaries (UK PAYE tax months run 6th to 5th; CRA Accelerated remitter periods end mid-month). | GAP-PAYROLL-03, GAP-TAX-01, 02, 04 |
| Q228 | UK calendars | Not in M1. UK obligations take GAP-TAX-04's "no calendar" path; UK VAT and PAYE are the next jurisdiction to add. Q122 unchanged. | GAP-TAX-04 |
| Q285 | CRA remitter calendars complete in M1 | Canada federal payroll Reference Data holds Regular, Accelerated threshold 1, Accelerated threshold 2, and Quarterly. Which applies is the remitter-type Setting. Amends Q122 and ADR-0007; see `docs/plans/design-session-2026-09-23.md`. | GAP-PAYROLL-03, 04 |
| Q123 | What Provisional and "assumed" mean | A run is Provisional when evidence it needs is missing (an obligation excluded for want of a Setting, or a blocking check). Estimates from the Entity's own actuals are Confidence estimated and do not make a run Provisional; otherwise every forecast would be Provisional. Retire the "assumed" level, which nothing produces. Amends Q88, ADR-0020, CASH-CONF-01 and CASH-PROV-01. | all |

---

## Scheduled obligations

### GAP-SCHED-01 — An owner-entered scheduled obligation is paid on its dates (Q115–Q117)

- **Scope:** Universal · **Status:** active
- **Statement:** A scheduled obligation the owner has entered (payee, amount, frequency, next
  date, optional end date) produces a payment for each occurrence within the horizon. This
  covers loans, leases, rent, insurance, subscriptions and owner-level tax the Entity funds.
- **Justification:** Neither ledger holds a schedule for loans, leases or charges paid without a
  bill; the agreement does `[gap: Parts 4–5]`. The owner's entry is the evidence. Detecting
  candidates from bank history has no general rule for what makes a pattern `[gap: Part 5]`, so
  it is left to the M2 onboarding proposer, where a person confirms each one
  (design-session-2026-09-11 Q37).
- **Reads:** Settings: scheduled obligation (payee, amount, frequency, next date, end date).
- **Produces:** Placement per occurrence, basis "scheduled obligation", Confidence firm.
- **Marks Provisional:** no.

### GAP-SCHED-02 — A Gap covered by a booked document is not forecast again (Q118)

- **Scope:** Universal · **Status:** active
- **Statement:** An occurrence of a scheduled obligation is covered when the ledger holds a
  bill, card charge or bank spend for it. (Ledger bill templates are AP's, under AP-SCHED-01,
  which applies the same test.) Evidence of that is a
  template link where one exists (Xero `RepeatingInvoiceID`); otherwise the same payee,
  dated nearer this scheduled date than either neighbouring one (Q149). For this test
  the schedule's frequency extends before its first date and past its end date, and a
  document exactly halfway covers the earlier occurrence. The amount doesn't need to match, because the
  actual replaces the estimate. A covered occurrence becomes an exclusion citing what covers it.
- **Justification:** Once recorded, the payment is an open bill (AP), a card charge (in the card
  balance) or a bank spend (in Opening Cash); forecasting the Gap too counts the cash twice
  `[gap: Part 7]`. Only template-generated Xero bills carry a link; everything else must match
  by payee and period, and some QBO purchases have no payee `[gap: Part 7]`. A spend without a
  payee cannot cover a Gap, which overstates outflows rather than understating them.
- **Reads:** Canonical Facts: bill, card transaction, bank transaction, template link. Settings:
  scheduled obligation.
- **Produces:** exclusion Placement, reason "covered by", citing the covering document.
- **Marks Provisional:** no.

### GAP-LOAN-01 — A loan or lease balance with no schedule is not guessed (Q116)

- **Scope:** Universal · **Status:** active
- **Statement:** An account classified as a loan or lease liability, with a balance on the run
  date and no scheduled obligation entered for it, is shown as an exclusion. A Decision Item asks
  for the payment schedule, and the run is Provisional.
- **Justification:** The ledger holds the balance, not the terms `[gap: Part 4]`. Leaving debt
  service out understates outflows, so the run must say it is incomplete (ADR-0019).
- **Reads:** Canonical Facts: account balance as of a date. Classifications: loan, lease
  liability. Settings: scheduled obligation.
- **Produces:** exclusion Placement, reason "no schedule"; Decision Item kind "enter loan
  schedule".
- **Acted on by:** owner.
- **Marks Provisional:** yes.

---

## Payroll

### GAP-PAYROLL-01 — Net pay is paid on each pay date (Q111)

- **Scope:** Universal · **Status:** active
- **Statement:** For an Entity with a payroll schedule set (pay frequency, next pay date,
  expected net pay), each pay date within the horizon produces a net pay payment. GAP-SCHED-02's
  coverage test applies, with the payroll provider or employees as payee.
- **Justification:** Neither ledger exposes Canadian or US payroll data through the APIs read,
  and pay frequency and pay dates are held nowhere in the ledger `[gap: Part 1]`.
- **Reads:** Settings: payroll schedule.
- **Produces:** Placement per pay date, basis "payroll schedule", Confidence firm.
- **Marks Provisional:** no.

### GAP-PAYROLL-02 — An Entity that runs payroll but has no schedule is not guessed (Q111)

- **Scope:** Universal · **Status:** active
- **Statement:** An Entity with an account classified as payroll liability or wages, and no
  payroll schedule set, raises a Decision Item to set one, and the run is Provisional.
- **Justification:** Payroll is usually among the largest outflows. Leaving it out silently
  would make the forecast look healthy while being wrong (ADR-0019).
- **Reads:** Classifications: payroll liability, wages. Settings: payroll schedule.
- **Produces:** Decision Item kind "set up payroll schedule".
- **Acted on by:** owner.
- **Marks Provisional:** yes.

### GAP-PAYROLL-03 — Source deductions are remitted on the statutory calendar (Q112, Q119, Q227, Q285)

- **Scope:** Universal · **Status:** active
- **Statement:** Each remittance due within the horizon is placed on its due date from the
  calendar for the Entity's remitter type or deposit schedule.
  - **Pay periods already booked:** the amount is the payroll liability balance at the end of
    the period the remittance covers, less remittances made since. Confidence firm. The period
    and its end come from the calendar, and need not end at a month-end (Q227).
  - **Pay periods not yet booked:** the amount is the expected remittance from the payroll
    schedule (Confidence firm), or else the last actual remittance for a period of the same
    length (Confidence estimated).
  - **Neither:** that remittance is excluded, and the run is Provisional. One item per Entity
    lists every such remittance, because one expected amount fixes them all (Q158).

  Every remittance is marked as held in trust for a government.
- **Justification:**
  - **Due dates are statutory,** set by remitter type in Canada and by deposit schedule in the
    US `[gap: Part 1]`. Canada federal Reference Data holds Regular (month of pay, due the 15th
    of the next month), Accelerated threshold 1 (1st–15th due the 25th; 16th–end due the 10th of
    the next month), Accelerated threshold 2 (1–7 / 8–14 / 15–21 / 22–end, due the third working
    day after each period end), and Quarterly (small employer; remittance due 15 April, 15 July,
    15 October, 15 January) (Q285, ADR-0007).
  - **Booked balances cannot double-count** what has already been recorded `[gap: Part 7]`.
  - **Trust priority:** withheld amounts are held in trust, and directors can be personally
    liable for them `[ap: Part 2]`, so they rank first in the queue (AP-PRIORITY-01).
- **Reads:** Canonical Facts: account balance as of a date, bank transactions to the tax
  authority. Classifications: payroll liability, government trust. Settings: payroll schedule,
  remitter type. Reference Data: payroll remittance calendars.
- **Produces:** Placement per remittance, trust-marked; exclusion Placement and Decision Item
  kind "payroll remittance amount unknown" when neither amount source exists.
- **Acted on by:** owner.
- **Marks Provisional:** yes, only when a remittance is excluded.

### GAP-PAYROLL-04 — The remitter type is a Setting (Q113)

- **Scope:** Universal · **Status:** active
- **Statement:** An Entity with a payroll schedule and no remitter type or deposit schedule set
  raises a Decision Item to set it. Its remittances are excluded, and the run is Provisional.
- **Justification:** The tax authority assigns the remitter type from past withholding, and
  neither ledger holds it `[gap: Part 1]`. Which calendar applies is a Setting (ADR-0007).
- **Reads:** Settings: remitter type or deposit schedule.
- **Produces:** exclusion Placements for remittances; Decision Item kind "set remitter type".
- **Acted on by:** accountant.
- **Marks Provisional:** yes.

---

## Sales tax

### GAP-TAX-01 — Sales tax for an ended period is remitted from the booked balance (Q112, Q114, Q224–Q227)

- **Scope:** Universal · **Status:** active
- **Statement:** For each reporting period that has ended and whose payment falls due within the
  horizon, the remittance is the sales-tax liability balance at the period's end less payments
  made to the authority since. The balance is the sum over every account classified as sales-tax
  liability, including a tax suspense account holding a filed return (Q225). Payments to the
  authority are payments marked as tax remittances, whatever their payee (Q224). It is placed on
  the due date from the calendar for the Entity's reporting period, moved for weekends and
  holidays in the direction that calendar sets (Q227), and marked as held in trust for a
  government. A refund position (a debit balance) is shown as an exclusion with an item, not a
  forecast receipt.
  - **Tax accounting scheme (Q226, Q284):** this basis applies when the Entity's scheme is
    standard. Under any other scheme the remittance is estimated as GAP-TAX-02 estimates it. With
    no scheme set and no ledger basis, the standard basis applies, Confidence firm, and no
    confirm item (Q284 Pick A — small-business default). With no scheme set but a ledger-held
    basis (Xero `SalesTaxBasis`, Q253), that basis is used, Confidence estimated, and a Decision
    Item asks the accountant to confirm the scheme.
- **Justification:**
  - **Due dates:** GST/HST is due one month after a monthly or quarterly period, and three
    months after year-end for annual filers `[gap: Part 2]`. UK VAT is due one calendar month and
    7 days after the period, and must arrive by then even on a weekend or bank holiday
    `[uk: S1]`, where CRA and IRS move the date later `[gap: Part 8]`.
  - **Two liability accounts, payments with no payee:** QBO moves a filed return from the tax
    payable account to a tax suspense account, and records the payment as its own record type
    with no payee, in the UK and Canada `[uk: checks 7–8]`.
  - **The booked balance is the return only under the standard scheme:** QBO books tax on the
    document date `[uk: check 9]`, while cash accounting owes it when customers pay `[uk: S3]`,
    and only Xero records a basis (`SalesTaxBasis`: standard, cash or flat rate; Q253).
  - **Recoverable tax is already netted:** the booked liability nets input tax credits and any
    bad-debt adjustment as the ledger recorded them, so no tax-rate mapping is needed for
    booked periods `[gap: Parts 2, 7]`.
  - **Trust priority:** tax collected is not the business's money `[ap: Part 2]`.
  - **No refund is forecast:** whether a refund is paid is the authority's decision.
- **Reads:** Canonical Facts: account balance as of a date, payments marked as tax remittances.
  Classifications: sales-tax liability, government trust. Settings: reporting period, tax
  accounting scheme. Reference Data: sales tax calendars.
- **Produces:** Placement per remittance, basis "booked tax liability", Confidence firm (estimated
  when the period or a ledger-held scheme is unconfirmed), trust-marked; exclusion and Decision
  Item kind "sales tax refund position" for a debit balance; Decision Item kind "confirm tax
  accounting scheme" when a ledger-held basis is used without an owner Setting (Q253), not when
  both are blank (Q284).
- **Acted on by:** accountant.
- **Marks Provisional:** no.

### GAP-TAX-02 — Sales tax for a period still running is estimated from actuals (Q112, Q119, Q224, Q226)

- **Scope:** Universal · **Status:** active
- **Statement:** For a reporting period that ends within the horizon, or has not yet ended, the
  remittance is estimated as the last actual remittance for a period of the same length. It is
  placed on the calendar due date, with Confidence estimated, and trust-marked. The same estimate
  covers an ended period when the Entity's tax accounting scheme isn't standard (Q226). Actual
  remittances are payments marked as tax remittances, whatever their payee (Q224). With no prior
  actual, it is excluded with an item and the run is Provisional. Only remittances due within
  the horizon are estimated or excluded (Q159).
- **Justification:** The unbooked part of a period has no liability yet. The Entity's own last
  remittance is the least arbitrary basis available `[gap: Part 2]`, and no rate is guessed
  (ADR-0019).
- **Reads:** Canonical Facts: payments marked as tax remittances. Settings: reporting period, tax
  accounting scheme. Reference Data: sales tax calendars.
- **Produces:** Placement, basis "last remittance", Confidence estimated, trust-marked; or
  exclusion and Decision Item kind "sales tax estimate unavailable".
- **Acted on by:** accountant.
- **Marks Provisional:** yes, only when excluded.

### GAP-TAX-03 — The reporting period is a Setting, prefilled from the ledger (Q113, Q253)

- **Scope:** Universal · **Status:** active
- **Statement:** An Entity's sales tax reporting period is a Setting. When it is unset but the
  ledger holds one (Xero `SalesTaxPeriod`), that value is used, remittances timed from it carry
  Confidence estimated, and a Decision Item asks the accountant to confirm it. With neither,
  sales tax remittances are excluded with an item, and the run is Provisional. The tax
  accounting scheme follows the same pattern from Xero `SalesTaxBasis`, under GAP-TAX-01 (Q253).
- **Justification:** The authority assigns the period from revenue thresholds or elections
  `[gap: Part 2]`. Only Xero records it `[gap: Summary 1]`. A ledger value is evidence rather
  than a guess, but it may be stale.
- **Reads:** Settings: reporting period. Canonical Facts: organisation tax settings.
- **Produces:** Decision Item kind "confirm sales tax period" or "set sales tax period".
- **Acted on by:** accountant.
- **Marks Provisional:** yes, only when no period is known.

### GAP-TAX-04 — A jurisdiction with no calendar is flagged, never skipped (Q122, Q228)

- **Scope:** Universal · **Status:** active
- **Statement:** An obligation whose jurisdiction has no calendar in Reference Data (in M1,
  anything other than Canada federal and US federal) is excluded as "no calendar,
  unschedulable". A Decision Item names the jurisdiction, and the run is Provisional.
- **Justification:** ADR-0007 requires an explicit Gap instead of silent omission. Calendars
  need holiday lists and working-day counting, and they change `[gap: Part 8]`, so coverage is
  added one jurisdiction at a time. The UK is next (Q228). Because each calendar carries its own
  weekend direction and period boundaries (Q227), adding one changes no Rule.
- **Reads:** Settings: jurisdiction per obligation. Reference Data: calendar coverage.
- **Produces:** exclusion Placement, reason "no calendar"; Decision Item kind "unsupported tax
  jurisdiction".
- **Acted on by:** accountant.
- **Marks Provisional:** yes.

### GAP-INCOME-01 — Corporate income tax instalments and balances are paid on their dates

- **Scope:** Universal · **Status:** active
- **Statement:** Where the owner has entered corporate instalment amounts, each instalment within
  the horizon is placed on its statutory date, computed from the Entity's tax-year start. A
  booked income-tax payable balance for a completed tax year is placed on the balance-due date.
  Where it is not set whether the Entity qualifies for the later balance-due date (3 months
  after year-end rather than 2, in Canada), the balance is placed at the earlier date with
  Confidence estimated, and a Decision Item "confirm income tax balance-due date" goes to the
  accountant. The run is not Provisional (Q160).
  Tax that falls on the owner personally is not forecast unless entered under GAP-SCHED-01.
- **Justification:** Instalment and balance-due dates are statutory in both countries
  `[gap: Part 3]`. Instalment amounts are not in either ledger. Pass-through and
  sole-proprietor tax leaves the owner, not the Entity (Q115).
- **Reads:** Canonical Facts: account balance as of a date. Classifications: income tax payable.
  Settings: instalment amounts (optional), tax-year start. Reference Data: instalment calendars.
- **Produces:** Placement per instalment and balance due, Confidence firm.
- **Marks Provisional:** no.

---

## Balances with no bill

### GAP-CARD-01 — A card balance is paid in full on the card's payment day (Q85, Q121)

- **Scope:** Universal · **Status:** active
- **Statement:** Each credit card's balance on the run date is placed as one payment on the
  card's next payment day. A card with no payment day set is excluded with an item, and the run
  is Provisional.
- **Justification:** Card balances are credit used, not cash (ADR-0020). Neither ledger holds a
  statement date, statement balance or due date `[gap: Part 9]`. Paying the run-date balance in
  full is the prudent reading.
- **Reads:** Canonical Facts: account balance as of a date. Classifications: credit card.
  Settings: card payment day (required).
- **Produces:** Placement, basis "card balance", Confidence firm; or exclusion and Decision Item
  kind "set card payment day".
- **Acted on by:** owner.
- **Marks Provisional:** yes, only when excluded.

### GAP-ACCRUAL-01 — An accrued liability is paid when the owner says (Q120)

- **Scope:** Universal · **Status:** active
- **Statement:** The balance of an account classified as accrued liabilities, on the run date,
  is placed on the settlement date set for that account, as a date or as days after month-end.
  With none set, it is excluded with an item, and the run is Provisional.
- **Justification:** An accrual is owed for goods or services received and not yet billed
  `[gap: Part 6, IAS 37.11]`. The balance says what, not when. When the bill arrives, AP takes
  over and the accrual reverses in the ledger.
- **Reads:** Canonical Facts: account balance as of a date. Classifications: accrued
  liabilities. Settings: expected settlement per account (optional).
- **Produces:** Placement, basis "accrual", Confidence estimated; or exclusion and Decision Item
  kind "when will this accrual be paid".
- **Acted on by:** accountant.
- **Marks Provisional:** yes, only when excluded.
