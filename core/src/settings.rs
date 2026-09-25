//! Settings the Rules read, and the Classifications computed from them (ADR-0013). Q258: M1
//! carries what AR reads; each later Family adds its classes and its rung of the ladder.

use std::collections::{BTreeMap, BTreeSet};

use chrono::Weekday;
use rust_decimal::Decimal;

use crate::facts::{EntityId, FactId, Facts, Frequency};
use crate::forecast::Horizon;
use crate::money::{Currency, HomeAmount};
use crate::reference::{Jurisdiction, RemitterType};
use chrono::NaiveDate;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AccountClass {
    ArControl,
    /// Unbilled receivable or contract asset (AR-UNBILLED-01).
    UnbilledReceivable,
    /// The account the Entity writes bad debts off to (AR-WRITEOFF-01).
    BadDebtWriteOff,
    ApControl,
    /// Cash the Entity can spend (CASH-OPEN-01); card and bank spends are read by AP-PAID-01.
    Bank,
    CreditCard,
    /// Trust, escrow or a security deposit: shown, not counted (CASH-OPEN-02).
    Restricted,
    /// Undeposited receipts, placed in week 1 rather than counted as cash on hand (CASH-OPEN-03).
    Clearing,
    /// A line of credit: undrawn is Headroom, drawn is a Gap (CASH-HEAD-01).
    CreditLine,
    /// Payroll source deductions held for remittance (GAP-PAYROLL-03).
    PayrollLiability,
    /// Wages payable / net pay clearing (GAP-PAYROLL-01).
    Wages,
    /// GST/HST or similar sales tax collected, including tax suspense (GAP-TAX-01, Q225).
    SalesTaxLiability,
    /// Accrued liabilities awaiting a bill (GAP-ACCRUAL-01).
    AccruedLiabilities,
    /// Corporate income tax payable (GAP-INCOME-01).
    IncomeTaxPayable,
    /// A loan balance that needs a payment schedule (GAP-LOAN-01).
    Loan,
    /// A lease liability that needs a payment schedule (GAP-LOAN-01).
    LeaseLiability,
    /// An intercompany due-to / due-from / loan account; `None` when the counterparty Entity is
    /// not named (IC-MAP-01, Q131 / Q162).
    Intercompany(Option<EntityId>),
    Unclassified,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CounterpartyClass {
    /// Names which Entity in the Group the counterparty represents; `None` when the mapping says
    /// intercompany but has not named one (IC-MAP-01, Q131 / Q162).
    Intercompany(Option<EntityId>),
    /// Owed amounts held in trust for a government (AP-PRIORITY-01).
    GovernmentTrust,
    Unclassified,
}

/// Which rung of the ladder produced a Classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassificationSource {
    Mapping,
    Metadata,
    Keyword,
    /// An explicit `unclassified`: nothing matched.
    NoMatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Classification<C> {
    pub class: C,
    pub source: ClassificationSource,
}

/// An expected receipt the owner enters because no ledger schedules it (CASH-SCHED-01, Q132): an
/// owner injection, loan proceeds, a grant, an expected tax refund. Owner drawings, funds
/// introduced and shareholder loans are forecast only this way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledReceipt {
    pub payer: FactId,
    pub amount: HomeAmount,
    pub frequency: Frequency,
    /// The next date the owner entered. The schedule extends before it and past its end (Q149).
    pub start: NaiveDate,
    pub end: Option<NaiveDate>,
}

/// An owner-entered obligation no ledger schedules (GAP-SCHED-01): a loan, lease, rent,
/// insurance, subscription or owner-level tax the Entity funds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledObligation {
    pub payee: FactId,
    pub amount: HomeAmount,
    pub frequency: Frequency,
    pub start: NaiveDate,
    pub end: Option<NaiveDate>,
    /// When set, GAP-LOAN-01 treats this schedule as covering that loan/lease account.
    pub covers_account: Option<FactId>,
}

/// Owner-entered payroll cadence (GAP-PAYROLL-01, Q111).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PayrollSchedule {
    pub frequency: Frequency,
    pub next_pay_date: NaiveDate,
    pub expected_net_pay: HomeAmount,
    /// Per-run source-deduction remittance; unset is Q119's "neither" path (GAP-PAYROLL-03).
    pub expected_remittance: Option<HomeAmount>,
    /// The processor or payee coverage tests match against (GAP-SCHED-02), when known.
    pub paid_through: Option<FactId>,
}

/// Sales tax reporting period the owner confirms (GAP-TAX-03); selection of a calendar (ADR-0007).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SalesTaxReportingPeriod {
    Monthly,
    Quarterly,
    Annual,
}

/// Tax accounting scheme the owner confirms (GAP-TAX-01, Q226). Distinct from the ledger-held
/// `SalesTaxBasis` fact, which may prefill this with Confidence estimated (Q253).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaxAccountingScheme {
    Standard,
    Cash,
    FlatRate,
    /// Canada's Quick Method and any other scheme where the booked balance is not the return.
    Other,
}

/// When an accrued liability is expected to settle (GAP-ACCRUAL-01, Q120).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccrualSettlement {
    DaysAfterMonthEnd(u32),
    OnDate(NaiveDate),
}

/// Corporate income tax timing inputs (GAP-INCOME-01).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorporateTax {
    /// Start of the current tax year (day and month recur).
    pub tax_year_start: NaiveDate,
    /// Owner-entered monthly instalment amount; unset means no instalments are forecast.
    pub monthly_instalment: Option<HomeAmount>,
    /// Whether the Entity qualifies for the later balance-due date; unset is Q160's earlier date.
    pub balance_due_extension: Option<bool>,
}

/// An owner-entered settlement schedule for an intercompany balance (IC-LOAN-01, Q126): the
/// Entity that pays, the Entity that receives, amount, frequency, next date and optional end.
/// `accounts` are the balance accounts this schedule settles (IC-S02's L1 loans); empty means
/// every intercompany account between the pair, which no Scenario uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntercompanySettlementSchedule {
    pub owing: EntityId,
    pub owed: EntityId,
    pub amount: HomeAmount,
    pub frequency: Frequency,
    pub start: NaiveDate,
    pub end: Option<NaiveDate>,
    pub accounts: Vec<FactId>,
}

/// Tenant-owned values that change what the engine computes. Versioning is M2 persistence.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    /// Owner-confirmed account mappings, the first rung of ADR-0013's ladder.
    pub account_mappings: BTreeMap<FactId, AccountClass>,
    pub counterparty_mappings: BTreeMap<FactId, CounterpartyClass>,
    /// Customers the owner marks as behaving differently, by top-level parent (AR-TIME-02, Q76).
    pub distinct_customers: BTreeSet<FactId>,
    /// The weekday an Entity pays its bills on (AP-RUN-01); optional, no default.
    pub pay_run_weekday: BTreeMap<EntityId, Weekday>,
    /// Vendors the owner marks (AP-PRIORITY-01), by top-level parent.
    pub critical_vendors: BTreeSet<FactId>,
    pub secured_lenders: BTreeSet<FactId>,
    /// The closing cash an owner chooses never to fall below (AP-DISC-02, CASH-BUFFER-01),
    /// in the Entity's Home Currency (Q273).
    pub minimum_cash_buffer: BTreeMap<EntityId, HomeAmount>,
    /// How many weeks the forecast runs for, per Group (CASH-WEEK-01); optional, no general
    /// default, because unset is what the Decision Item asks the owner to fill in.
    pub horizon: Option<Horizon>,
    /// Receipts the owner expects that no ledger holds (CASH-SCHED-01), by the id they are
    /// entered under, which is what an occurrence is named by.
    pub scheduled_receipts: BTreeMap<FactId, ScheduledReceipt>,
    /// The currency the Group consolidates in (CASH-GROUP-01), per Group; optional, because no
    /// Rule asks for one and a Group of one Entity never consolidates (Q277).
    pub reporting_currency: Option<Currency>,
    /// The owner's rate from one currency to another — their current belief, never a market fact
    /// (`CONTEXT.md`, Q30). No default: a pair with no rate is CASH-GROUP-02's exclusion.
    pub conversion_rates: BTreeMap<(Currency, Currency), Decimal>,
    /// The day of the month a credit card is paid on (GAP-CARD-01), by the card's account; no
    /// default, since neither ledger holds a statement date.
    pub card_payment_days: BTreeMap<FactId, u32>,
    /// A credit line's limit (CASH-HEAD-01), named rather than keyed by account: the Setting can
    /// name a line before any account is mapped to it, which is what raises the item to map one.
    pub credit_line_limits: BTreeMap<(EntityId, String), HomeAmount>,
    /// Obligations the owner enters that no ledger schedules (GAP-SCHED-01), by the id they are
    /// entered under, as `scheduled_receipts` is.
    pub scheduled_obligations: BTreeMap<FactId, ScheduledObligation>,
    /// Payroll cadence per Entity (GAP-PAYROLL-01); optional, no default (ADR-0019).
    pub payroll_schedules: BTreeMap<EntityId, PayrollSchedule>,
    /// CRA remitter type or US deposit schedule selection (GAP-PAYROLL-04, ADR-0007).
    pub remitter_types: BTreeMap<EntityId, RemitterType>,
    /// Confirmed sales tax reporting period (GAP-TAX-03).
    pub sales_tax_periods: BTreeMap<EntityId, SalesTaxReportingPeriod>,
    /// Confirmed tax accounting scheme (GAP-TAX-01, Q226).
    pub tax_accounting_schemes: BTreeMap<EntityId, TaxAccountingScheme>,
    /// Expected settlement of an accrued-liability account (GAP-ACCRUAL-01).
    pub accrual_settlements: BTreeMap<FactId, AccrualSettlement>,
    /// Corporate tax-year start and instalments (GAP-INCOME-01).
    pub corporate_tax: BTreeMap<EntityId, CorporateTax>,
    /// Jurisdiction of a sales-tax or remittance obligation account (GAP-TAX-04); unset means
    /// Canada federal when the Entity is otherwise Canadian in the Scenario.
    pub tax_jurisdictions: BTreeMap<FactId, Jurisdiction>,
    /// Settlement schedules for intercompany balances (IC-LOAN-01), keyed by the id they are
    /// entered under (the Scenario's L1, L2, …).
    pub intercompany_settlement_schedules: BTreeMap<FactId, IntercompanySettlementSchedule>,
    /// Optional per-Group tolerance for same-currency reciprocal balances (IC-AGREE-01, Q124);
    /// unset means exact agreement.
    pub intercompany_tolerance: Option<HomeAmount>,
    /// Intercompany accounts the owner has confirmed are not settling within the horizon
    /// (IC-LOAN-02, Q126). Confirmation closes the settlement-plan item.
    pub intercompany_not_settling: BTreeSet<FactId>,
}

/// Every account's and counterparty's Classification, computed before the Rules run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Classifications {
    accounts: BTreeMap<FactId, Classification<AccountClass>>,
    counterparties: BTreeMap<FactId, Classification<CounterpartyClass>>,
}

impl Classifications {
    pub fn account(&self, id: &FactId) -> Classification<AccountClass> {
        self.accounts.get(id).cloned().unwrap_or(Classification {
            class: AccountClass::Unclassified,
            source: ClassificationSource::NoMatch,
        })
    }

    pub fn counterparty(&self, id: &FactId) -> Classification<CounterpartyClass> {
        self.counterparties
            .get(id)
            .cloned()
            .unwrap_or(Classification {
                class: CounterpartyClass::Unclassified,
                source: ClassificationSource::NoMatch,
            })
    }

    /// The accounts of one Entity carrying a class.
    pub fn accounts_classed<'a>(
        &'a self,
        entity: &'a EntityId,
        class: &'a AccountClass,
    ) -> impl Iterator<Item = &'a FactId> + 'a {
        self.accounts
            .iter()
            .filter(move |(id, c)| id.entity == *entity && c.class == *class)
            .map(|(id, _)| id)
    }
}

/// Mapping first; provider metadata and keyword matching arrive with the Families that read them.
pub fn classify(facts: &Facts, settings: &Settings) -> Classifications {
    let mut classifications = Classifications::default();
    for id in facts.accounts().keys() {
        if let Some(class) = settings.account_mappings.get(id) {
            classifications.accounts.insert(
                id.clone(),
                Classification {
                    class: class.clone(),
                    source: ClassificationSource::Mapping,
                },
            );
        }
    }
    for id in facts.counterparties().keys() {
        if let Some(class) = settings.counterparty_mappings.get(id) {
            classifications.counterparties.insert(
                id.clone(),
                Classification {
                    class: class.clone(),
                    source: ClassificationSource::Mapping,
                },
            );
        }
    }
    classifications
}
