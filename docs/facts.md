# Canonical Facts

Status: **approved by the owner** (2026-09-16). Built: every fact kind here is a type in
`core/src/facts.rs`, and construction rejects what this document calls invalid. Only the AR Rules
read these facts yet, and Classification implements the mapping rung only — the `metadata` columns
below are carried on the facts and not read (Q258).

What this document is: the normative fact model (Q215). Every fact kind `core` reads is listed
here with its fields, what each field means, which Rules read it, and where QuickBooks Online
(QBO) and Xero supply it, citing sandbox evidence. The Rust types mirror this document; where they
disagree, the code is wrong. M2's database schema is derived from the same types (Q34).

Decisions behind it are Q215–Q250, recorded in `docs/plans/design-session-2026-09-16.md`, with
earlier ones cited by number. Four questions raised while writing it were decided as Q251–Q254.

**Evidence keys.** Research notes under `docs/research/`:

| Key | Note |
|---|---|
| `[ledger: Part n]` | `2026-09-13-ledger-document-model-qbo-xero.md` |
| `[write: n]` | `2026-09-14-sandbox-write-pass.md`, check n |
| `[read: n]` | `2026-09-15-sandbox-read-pass.md`, check n |
| `[records: n]` | `2026-09-16-sandbox-credit-and-intercompany-records.md`, check n |
| `[uk: n]` | `2026-09-16-sandbox-uk-pass.md`, check n |
| `[fxbank: n]` | `2026-09-16-sandbox-foreign-currency-bank.md`, check n |
| `[xlines: n]` | `2026-09-16-sandbox-xero-document-lines.md`, check n |

---

## Rules every fact follows

### What is a fact (Q216)

M1's fact kinds are exactly what the Rules read:

| Fact kind | Section |
|---|---|
| Account | [Account](#account) |
| Counterparty | [Counterparty](#counterparty) |
| Payment terms | [Payment terms](#payment-terms) |
| Document (invoices, bills and credit notes) | [Document](#document) |
| Payment | [Payment](#payment) |
| Application | [Application](#application) |
| Account line | [Account line](#account-line) |
| Account balance | [Account balance](#account-balance) |
| Scheduled template | [Scheduled template](#scheduled-template) |
| Purchase order | [Purchase order](#purchase-order) |
| Ledger settings | [Ledger settings](#ledger-settings) |

Nothing else enters `core`. Time entries belong to a future scenario; journal entries appear only
as account lines (Q216).

### Every fact carries

| Field | Meaning | Decision |
|---|---|---|
| `id` | Deterministic: Entity, provider, provider object type, provider id. A later sync of the same record keeps its id, so Decision Items survive across runs. The Transport is not part of it. | Q221, Q31, ADR-0005 |
| `entity` | The Entity the fact belongs to. Every reference a fact holds resolves within the same Entity. | ADR-0015, Q235 |
| `provenance` | The Adapter and Source Snapshot it came from, or in M1 the Scenario that built it. | Q221 |

### Money (Q182, Q202, Q209, Q219, Q231, Q242, Q243)

Every monetary field is one `Money` shape:

| Part | Meaning |
|---|---|
| `home` | Amount in the Entity's Home Currency. Always present. Exact decimal, never floating point. |
| `transaction` | Optional. Present only when the record's currency is not the home currency: `amount`, `currency`, `rate`, `quote` and `home_derived`. |
| `transaction.rate` | The rate exactly as the ledger gave it, never inverted. |
| `transaction.quote` | Which way the rate is quoted: `home_per_unit` (QBO: 1.37 CAD per USD `[records: 10]`) or `units_per_home` (Xero's documented `CurrencyRate`; unevidenced in a sandbox). |
| `transaction.home_derived` | True when the ledger gave no home figure and the Adapter computed it: `amount` × or ÷ `rate`, rounded once to the home currency's minor unit, half away from zero (Q182). QBO bills have no `HomeTotalAmt` `[records: 11]`, `[uk: 10]`. |

- **Home and reporting amounts are distinct types** in code (Q202); a fact never holds a reporting
  amount. Conversion to reporting currency needs the rate Setting (ADR-0019).
- **Signs (Q231).** Documents, payments and applications carry non-negative amounts; their
  direction comes from side, kind and purpose. Account lines and account balances are signed,
  **debit positive**.
- **Minor units** other than 2 are Reference Data when a Scenario needs one (Q182). GBP, CAD, USD
  and HKD are all 2 `[uk: 11]`.

### Dates (Q183)

Every date is a calendar date with no time or zone. Adapters convert ledger timestamps, including
Xero's `/Date(ms+offset)/` values `[read: 6]`. QBO's API dates are ISO whatever the company's
display format `[uk: 12]`.

### References and Classification (Q232, ADR-0013)

- **Facts reference accounts, counterparties, documents and terms by fact id,** never by name.
- **Classification is not a field on any fact.** It is computed per account or counterparty from
  an owner-confirmed mapping, then provider metadata, then default keyword matching, and records
  which produced it. The metadata it reads is carried on the Account and Counterparty facts.

### Status (Q220, Q230)

Documents carry `draft`, `awaiting_approval`, `posted`, `voided` or `deleted`. Open, part-paid and
paid are never stored; they are derived from the open amount (Q229). Voided and deleted facts are
kept, so "deleted items are ignored" is Rule behaviour a Scenario can test, not an Adapter filter.

### What construction rejects (Q208, Q235, Q243)

Construction rejects only what makes a fact meaningless:

1. A reference to an account, counterparty, document or terms that doesn't resolve in the same
   Entity.
2. A `transaction` part whose currency is the home currency.
3. A negative amount on a document, payment or application.
4. An application larger than the open amount of what it applies to, or in a different currency
   from it.
5. An empty installment list, or installments that don't sum to the document total.
6. A duplicate fact id.

Everything else is data, left to Rules: due before issue, a future-dated bill, an unusual status
history. In M1 a rejection fails the Scenario test. In M2 the record's Source Snapshot is kept, a
sync warning is raised, and every run for that Entity is Provisional until it is resolved, through
the CASH Rule "records the engine couldn't read", acted on by the bookkeeper (Q236, Q246).

---

## Account

An account in the Entity's chart of accounts. Classifications attach here.

| Field | Meaning | QBO | Xero | Read by |
|---|---|---|---|---|
| `name`, `code` | As the ledger shows them; `code` optional. | `Name`, `AcctNum` | `Name`, `Code` | keyword matching (ADR-0013); Scenario tables |
| `currency` | The account's currency. | `CurrencyRef` `[fxbank: 1]` | `CurrencyCode` (bank accounts) | Q234, CASH-OPEN-01 |
| `metadata` | Provider metadata for Classification: an open set of named values, never read by Rules directly. | `Classification`, `AccountType`, `AccountSubType` `[ledger: Part 3]` | `Class`, `Type`, `BankAccountType`, `SystemAccount`, `ReportingCode`, `EnablePaymentsToAccount` `[ledger: Part 3]` | Classification only |
| `parent` | Optional parent account. | `ParentRef` | none | display |
| `active` | Whether the account is in use. | `Active` | `Status` | CLOSE-BANK-03 (via Classification) |

**Metadata defaults ADR-0013 applies** (evidence `[ledger: Part 3]` unless noted):

| Classification | QBO | Xero |
|---|---|---|
| bank | `AccountType` Bank | `Type` BANK, `BankAccountType` BANK |
| credit card | `AccountType` Credit Card | `BankAccountType` CREDITCARD |
| AR control | `AccountType` Accounts Receivable | `SystemAccount` DEBTORS |
| AP control | `AccountType` Accounts Payable; **one per currency** ("Accounts Payable (A/P) - HKD") `[records: 11]`, `[uk: The company]` | `SystemAccount` CREDITORS |
| sales-tax liability | `AccountSubType` `GlobalTaxPayable`, `SalesTaxPayable`, **and `GlobalTaxSuspense`** (Q225, `[uk: 8]`) | `SystemAccount` GST |
| clearing | `AccountSubType` UndepositedFunds | none (mapping or keywords) |
| conversion | `AccountSubType` OpeningBalanceEquity | `SystemAccount` HISTORICAL |

Suspense, intercompany, related-party and usually payroll have no reliable metadata in either
ledger and come from mapping or keywords `[ledger: Part 3]`, `[records: 8]`.

## Counterparty

A customer, vendor or tax authority (Q237, Q244, Q245).

| Field | Meaning | QBO | Xero | Read by |
|---|---|---|---|---|
| `name` | Display name. | `DisplayName` (`FullyQualifiedName` for jobs) | `Name` | keyword matching; AP-DUP-01 (same vendor) |
| `parent` | Optional parent counterparty. **Rules that group by customer use the top-level parent** (Q245). | `ParentRef` on a sub-customer (job) `[uk: The company]` | always empty | AR-TIME-02, AR-UNAPPLIED-01, 02 |
| `currency` | Optional default currency. | `CurrencyRef` | `DefaultCurrency` | display |
| `metadata` | Provider metadata for Classification. | object type (Customer, Vendor) | `IsCustomer`, `IsSupplier`, `ContactGroups` | Classification only |
| `receivable_terms`, `payable_terms` | Optional references to the counterparty's default Payment terms as a customer and as a vendor (Q255). A Xero contact can carry both; a QBO customer or vendor carries only its own side's. | `SalesTermRef` on Customer; `TermRef` on Vendor | `PaymentTerms.Sales`, `PaymentTerms.Bills` on the Contact, else the organisation's | AP-PO-01 |

- **One kind, never merged (Q244).** A QBO business that is both customer and vendor is two facts,
  because the provider object type is part of the id; a Xero contact is one.
- **Nothing marks a counterparty as intercompany** in either ledger `[records: 8, 14]`; that is
  always a mapping.

## Payment terms

How a due date and an early-payment discount are worked out (Q233).

| Field | Meaning | QBO | Xero | Read by |
|---|---|---|---|---|
| `due_rule` | Days after the document date, days after the end of the document's month (Q256), or a day of the current or following month. | `Term` `STANDARD` (`DueDays`) or `DATE_DRIVEN` (`DayOfMonthDue`) `[ledger: Part 1]` | `PaymentTerms` `Day` + `Type` (`DAYSAFTERBILLDATE`, `DAYSAFTERBILLMONTH`, `OFCURRENTMONTH`, `OFFOLLOWINGMONTH`) `[ledger: Part 1]` | AP-PO-01 |
| `discount` | Optional: days and percent. | `DiscountDays`, `DiscountPercent` | never present | AR-DISC-01, AP-DISC-01, 02 |

- **Xero's `DAYSAFTERBILLMONTH`** is days after the end of the document's month (Q256); QBO has no
  equivalent. Terms of 10 on a document dated 10-10 fall due 11-10.
- **Where terms live differs.** QBO `Term` is its own entity, referenced from a document
  (`SalesTermRef`). Xero terms sit on the Contact and the Organisation, never on the document; the
  Adapter emits them as terms facts and references the contact's (else the organisation's) from
  each document.
- **Terms are unevidenced in both sandboxes beyond QBO `STANDARD`** `[ledger: Part 1]`; Xero
  contacts carried none.

## Document

An invoice or credit note, receivable or payable side (Q218).

| Field | Meaning | QBO | Xero | Read by |
|---|---|---|---|---|
| `side` | `receivable` or `payable`. | object type | `Type` ACCREC/ACCPAY, ACCRECCREDIT/ACCPAYCREDIT | every AR and AP Rule |
| `kind` | `invoice` or `credit_note`. An overpayment or prepayment is a Payment, not a document (Q251). | Invoice, Bill, CreditMemo, VendorCredit | `Invoice`, `CreditNote` | AR-OPEN-01, AP-OPEN-01, AR/AP-UNAPPLIED |
| `counterparty` | Customer or vendor. | `CustomerRef`, `VendorRef` | `Contact` | AR-UNAPPLIED, AP-DUP-01, AR/AP-SCHED, IC-DOC |
| `number` | Optional; may be absent. | `DocNumber` (null on API-created Canada invoices `[records: 15]`) | `InvoiceNumber`, `CreditNoteNumber` (absent on an AP credit note `[xlines: Method]`) | display, Scenario tables |
| `vendor_reference` | Optional; the supplier's own invoice number. | `DocNumber` on a Bill | `InvoiceNumber` on ACCPAY | AP-DUP-01 |
| `date` | Document date. | `TxnDate` | `Date` | AR-TIME-02, AR/AP-SCHED coverage, AP-DUP-01 |
| `status` | See Status above. | Derived: void recognised from a zeroed document whose `PrivateNote` starts "Voided - " `[write: 6]` (Q220); deleted from change data capture (M2); otherwise `posted`. QBO has no draft. | `Status` DRAFT → draft, SUBMITTED → awaiting_approval, AUTHORISED and PAID → posted, VOIDED, DELETED `[ledger: Part 1]` | AR-OPEN-01, 02, AP-OPEN-01, 02 |
| `total` | `Money`, tax included. | `TotalAmt`, `HomeTotalAmt` (absent on bills `[records: 11]`), `ExchangeRate`, `CurrencyRef` | `Total`, `CurrencyCode`, `CurrencyRate` | AR/AP-OPEN-04, AR/AP-FX-01 |
| `tax` | `Money`, the sales tax inside `total`. | `TxnTaxDetail.TotalTax` `[ledger: Part 1]`, `[uk: 9]` | `TotalTax` | AR-OPEN-04, AP-OPEN-04 |
| `installments` | Non-empty list of `{due_date, amount}` summing to `total`. One entry for QBO and Xero. | `DueDate` | `DueDate` | AR-TIME-02, AP-TIME-02, 03 |
| `terms` | Optional reference to Payment terms. | `SalesTermRef` | contact's or organisation's terms | AR-DISC-01, AP-DISC-01, 02 |
| `expected_date` | Optional date a person entered: expected payment (receivable) or planned payment (payable). | none `[ledger: Part 1]` | `ExpectedPaymentDate`, `PlannedPaymentDate` | AR-TIME-01, AP-TIME-01 |
| `template` | Optional reference to the Scheduled template that generated it. | none | `RepeatingInvoiceID` `[write: 1]` | AR-SCHED-01, 02, AP-SCHED-01 |
| `control_account` | The AR or AP account it posts to. | `ARAccountRef` / `APAccountRef` (one AP account per currency) | `DEBTORS` / `CREDITORS` system account | AR-TIE-01, AP-TIE-01 |
| `ledger_open` | The ledger's current open amount, **for the Adapter's check only**; no Rule reads it (Q229). | `Balance`/`HomeBalance`; credit memo `RemainingCredit`; vendor credit `Balance` `[records: 1, 3, 12]` | `AmountDue`; credit note `RemainingCredit` `[records: 4]` | Adapter sync warning |

**Derived, never stored:**

- **Open amount as of a date (Q229, Q243)** is `total` less applications dated on or before that
  date, worked out in the document's own currency, then converted at its booked rate. QBO confirms
  the open remainder stays at the booked rate after a payment at another rate `[records: 12]`.
- **For a credit note,** the same derivation is its unapplied remainder, read by AR-UNAPPLIED and
  AP-UNAPPLIED.

**Adapter notes.** Xero list endpoints must be read paged, or credit notes return no line items
`[xlines: 3]`. A QBO zero-total document that isn't a recognised void is `posted`, and Q145 ignores
it.

## Payment

Money received or paid (Q218, Q237, Q251), including overpayments and prepayments: a payment
not yet applied. ADR-0014's registered-not-posted state is an Adapter
capability that QBO and Xero never produce `[ledger: Part 2]`, so it is not a field in M1.

| Field | Meaning | QBO | Xero | Read by |
|---|---|---|---|---|
| `direction` | `in` or `out`. | object type | `PaymentType`, bank transaction `Type` | CASH, AR, AP |
| `purpose` | `customer`, `vendor` or `tax_authority`. Refunds are a direction and purpose pair, not a kind. | Payment → customer; BillPayment → vendor; **`TaxPayment` → tax_authority** `[uk: 7]` (Q224) | `ACCREC*` → customer; `ACCPAY*` → vendor; a bank transaction posting to a sales-tax liability account, else to a contact classified as a tax authority → tax_authority (Q252, unevidenced) | AR, AP, GAP-TAX-01, 02 |
| `counterparty` | Required, except for `tax_authority`. | `CustomerRef`, `VendorRef`; none on `TaxPayment` | `Contact` | AR-UNAPPLIED, AP-UNAPPLIED |
| `date` | Payment date. | `TxnDate`, `PaymentDate` | `Date` | AR-TIME-02 history, GAP-TAX |
| `amount` | `Money`. | `TotalAmt`, `ExchangeRate`; `TaxPayment.PaymentAmount` | `Amount` (document currency), `BankAmount`, `CurrencyRate` `[ledger: Part 2]`; an overpayment's or prepayment's `Total` `[records: 5]` | AR, AP, GAP-TAX |
| `account` | The account the money went to or came from: bank, card or clearing (ADR-0014). | `DepositToAccountRef` (Undeposited Funds if unset) `[ledger: Part 2]`, `[records: 2]`; `CheckPayment.BankAccountRef` / `CreditCardPayment.CCAccountRef`; `PaymentAccountRef` | `Account` | AP-OPEN-01 (card-paid), CASH-OPEN-03 |
| `status` | `posted` or `deleted`. | posted (deleted via change data capture, M2) | `Status` AUTHORISED, DELETED `[ledger: Part 2]`, `[xlines: 5]` | all (deleted ignored) |

**Derived:** a payment's **unapplied remainder** is its amount less its applications (Q229, Q251).
The Adapter checks it against QBO's `UnappliedAmt` `[records: 2]` and Xero's `RemainingCredit` on
an overpayment or prepayment `[records: 5]`. A Xero overpayment is one Payment fact, built from the
`Overpayment` object and its `RECEIVE-OVERPAYMENT` bank transaction together, never two.

## Application

A payment or credit note applied to a document (Q218, Q229, Q243).

| Field | Meaning | QBO | Xero | Read by |
|---|---|---|---|---|
| `from` | The payment or credit note applied. | the Payment or BillPayment; a CreditMemo line on a zero-value Payment `[ledger: Part 2]` | the Payment; `Allocations` on a CreditNote, Overpayment or Prepayment | AR-OPEN-01, AP-OPEN-01, AR/AP-UNAPPLIED |
| `to` | The document it applies to. | `Line[].LinkedTxn` | `Invoice` on a payment or allocation | same |
| `date` | Date applied. | the payment's `TxnDate` (QBO lines carry no date) | payment `Date`; allocation `Date` `[read: 10]` | open amount as of a date; AR-TIME-02 |
| `amount` | `Money` in the currency of the document applied to. | `Line[].Amount` | `Amount`; allocation `Amount` | same |

A QBO credit applied to an invoice appears as a zero-value payment with one invoice line and one
credit-memo line `[ledger: Part 2]`; the Adapter emits one application from the credit memo to the
invoice. How QBO applies a vendor credit is unevidenced `[ledger: Part 2]`.

## Account line

One transaction's posting to one account (Q217). Bank and card transactions are account lines in
bank- or card-classified accounts; account activity is a sum over them.

| Field | Meaning | QBO | Xero | Read by |
|---|---|---|---|---|
| `account` | The account posted to. | document line accounts; journal entry lines (`TotalAmt` is 0 on journals `[records: 6]`) | document line `AccountCode`, bank transaction `BankAccount`, payment `Account` `[xlines: Method]` | CLOSE, CASH, GAP-SCHED-02 |
| `date` | Posting date. | `TxnDate` | `Date` | CLOSE-BANK-01–03, CLOSE-SUSP-02, CLOSE-DISC-01 |
| `amount` | Signed `Money`, debit positive, with the account-currency part when the account's currency isn't home (Q234). | GeneralLedger `nat_foreign_amount`, `exch_rate` `[fxbank: 4]` | line amounts | CLOSE-CLEAR-01, CLOSE-DUP-01 |
| `counterparty` | Optional payee or payer. | `EntityRef`; blank on `TaxPayment` `[uk: 7]` | `Contact` | CLOSE-DUP-01, GAP-SCHED-02, AP-SCHED-01, CASH-SCHED-01 |
| `source` | Source record type and id. | entity type and `Id` | object type and id | coverage citations, Decision Item subjects |
| `reconciliation` | `reconciled`, `not_reconciled` or `unknown` (Q173). Only a completed reconciliation is reconciled (Q222). | GeneralLedger `is_cleared`, matched on account id, type and id: `R` reconciled; `C` and blank not reconciled `[records: 16–19]`, `[uk: 1–2]` (Q179, Q222); no match → unknown (Q180) | `IsReconciled` on `BankTransaction` and `Payment` `[read: 7]` | CLOSE-BANK-01, 02, CLOSE-DUP-01 |
| `status` | `posted` or `deleted`. | as the source record | `Status` DELETED on the source `[xlines: 5]` | all (deleted ignored) |

**Adapter notes.**

- **QBO:** which transactions an account has comes from documents (Q172); reconciliation status
  comes from the GeneralLedger report. Report columns are read by `ColKey`, and transaction-type
  labels are mapped per region ("Check" in the US, "Cheque" in the UK) (Q223, `[uk: 4]`). In a
  multicurrency company the amount keys are the home-currency ones (`subt_nat_amount_home_nt`,
  `debt_home_amt`, `credit_home_amt`); others are dropped without an error `[fxbank: 3–4]`. A
  missing `is_cleared` column is an Adapter error, never unknown status.
- **QBO `TaxPayment`** is read outside the US; the US company rejects the entity `[uk: 7]` (Q224).
- **Xero:** without the Journals endpoint (Advanced tier only, Q249–Q250), account lines are built
  from invoices, credit notes, payments, bank transactions, manual journals, overpayments and
  prepayments. Each month the Adapter compares them with the trial balance's movement per account;
  a disagreement is a sync warning. In August 2026, 17 of 20 Demo Company accounts matched to the
  cent and one 29.50 spend was unexplained `[xlines: 2, 4]`.

## Account balance

An account's balance as of a date (ADR-0014, ADR-0020).

| Field | Meaning | QBO | Xero | Read by |
|---|---|---|---|---|
| `account` | The account. | report row account id | report row `account` attribute | CASH-OPEN-01–04, CASH-HEAD-01, GAP, CLOSE-SUSP-01, CLOSE-CONV-01, AR-TIE-01, AP-TIE-01, IC-AGREE |
| `as_of` | The date. | report `end_date` | report `date` | same |
| `amount` | Signed `Money`, debit positive. `home` is the ledger's book value at the rates booked on each transaction, never revalued (Q240). The account-currency part, when present, is derived by the Adapter from the account's lines and marked derived (Q241). | TrialBalance or BalanceSheet (home only: 2,070.00 for USD 1,500.00 `[fxbank: 2]`); account currency from GeneralLedger `nat_foreign_amount` `[fxbank: 5]` | TrialBalance `YTD Debit`/`YTD Credit` at the date `[xlines: Method]` | same |

`Account.CurrentBalance` (QBO) is today's figure in the account's currency and never used
`[fxbank: 1]`.

## Scheduled template

A recurring invoice or bill template (Q239).

| Field | Meaning | QBO | Xero | Read by |
|---|---|---|---|---|
| `side` | `receivable` or `payable`. | the templated entity (Invoice, Bill) | `Type` ACCREC, ACCPAY | AR-SCHED-01, 02, AP-SCHED-01 |
| `counterparty` | Customer or vendor. | on the templated entity | `Contact` | coverage test (Q149) |
| `amount` | `Money`. | templated `TotalAmt` | `Total`, `CurrencyCode` | AR-SCHED-01, AP-SCHED-01 |
| `frequency` | Unit and interval. | `ScheduleInfo.IntervalType`, `NumInterval` | `Schedule.Unit`, `Period` | same |
| `start`, `next`, `end` | Dates. `next` is informational: neither ledger advances it reliably `[write: 1]`, so Rules judge occurrences from documents. | `ScheduleInfo.StartDate`, `NextDate`, `EndDate` | `Schedule.StartDate`, `NextScheduledDate`, `EndDate` | same |
| `due_rule` | How a generated document's due date is set. | templated terms | `Schedule.DueDate`, `DueDateType` | AR-SCHED-01, AP-SCHED-01 |
| `mode` | `automatic`, `automatic_draft` (generates draft documents; Xero only; Q254), `reminder` or `manual`; plus `active`. | `RecurType` Automated, Reminder, Unscheduled; `Active` | `Status` AUTHORISED, DRAFT `[write: 1]` | AR-SCHED-01, 02, AP-SCHED-01 |

## Purchase order

A committed purchase not yet billed (Q239).

| Field | Meaning | QBO | Xero | Read by |
|---|---|---|---|---|
| `vendor` | Counterparty. | `VendorRef` | `Contact` | AP-PO-01 |
| `status` | `draft`, `awaiting_approval`, `authorised`, `billed` or `closed`, `deleted`. Purchase orders keep their own statuses (Q230). | `POStatus` Open, Closed | `Status` DRAFT, SUBMITTED, AUTHORISED, BILLED, DELETED | AP-PO-01 |
| `delivery_date` | Optional. | none `[ledger: Part 1]` | `DeliveryDate` | AP-PO-01 |
| `total` | `Money`. | `TotalAmt`, `CurrencyRef` | `Total`, `CurrencyCode` | AP-PO-01 |

## Ledger settings

One per Entity per sync (Q216).

| Field | Meaning | QBO | Xero | Read by |
|---|---|---|---|---|
| `home_currency` | The Entity's Home Currency. | `CurrencyPrefs.HomeCurrency` `[uk: The company]` | `BaseCurrency` | every `Money` |
| `lock_date` | Optional. For Xero, the later of whichever of the two is set; none if neither (Q176). | `AccountingInfoPrefs.BookCloseDate` (unset in all sandboxes) `[uk: The company]` | `PeriodLockDate`, `EndOfYearLockDate` `[read: 6]` | CLOSE-LOCK-01 |
| `sales_tax_period` | Optional ledger-held reporting period, used with Confidence estimated until confirmed (GAP-TAX-03). | none `[uk: The company]` | `SalesTaxPeriod` (`3MONTHLY` in Demo Company) | GAP-TAX-03 |
| `sales_tax_basis` | Optional ledger-held tax accounting scheme, used with Confidence estimated until confirmed (Q253): `ACCRUAL`, `ACCRUALS`, `INVOICE` → standard; `CASH`, `PAYMENTS` → cash; `FLATRATEACCRUAL`, `FLATRATECASH` → flat rate; `NONE` → none. | none | `SalesTaxBasis` (`ACCRUALS` in Demo Company) | GAP-TAX-01, 02 |
| `fiscal_year_end` | Optional ledger-held year end, for display and M2 prefill only; Rules read the tax-year start Setting. | `FirstMonthOfFiscalYear` (and a separate `TaxYearMonth`, which can differ `[uk: The company]`) | `FinancialYearEndDay`, `FinancialYearEndMonth` `[read: 6]` | none in M1 |

