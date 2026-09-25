//! The Forecast Run and what it holds: Placements, exclusions included (ADR-0011), Decision Items
//! and Draft Corrections (ADR-0019, ADR-0020). The shape is Q257.

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::facts::{Direction, EntityId, FactId, Facts};
use crate::money::{Currency, HomeAmount, ReportingAmount};
use crate::reference::{Jurisdiction, sales_tax_name};
use crate::settings::{SalesTaxReportingPeriod, Settings, classify};

/// A catalogue Rule ID (ADR-0009): the evidence vocabulary every output cites.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleId(pub &'static str);

impl RuleId {
    /// The `FAMILY` segment of `FAMILY-SUBJECT-NN`.
    pub fn family(self) -> &'static str {
        self.0.split('-').next().unwrap_or_default()
    }
}

/// Weekly buckets, each a consecutive 7-day period starting on the run date (ADR-0020).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Horizon {
    pub weeks: u32,
}

/// One bucket, numbered from 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Week(pub u32);

impl Horizon {
    /// The week a date falls in, or `None` when it lies beyond the Horizon. A date before the run
    /// date is week 1: cash expected already but not yet received is still expected.
    pub fn week_of(self, as_of: NaiveDate, date: NaiveDate) -> Option<Week> {
        let days = (date - as_of).num_days().max(0);
        let week = u32::try_from(days / 7).ok()?.checked_add(1)?;
        (week <= self.weeks).then_some(Week(week))
    }

    /// The day after the last bucket: the first date beyond the Horizon.
    pub fn end(self, as_of: NaiveDate) -> NaiveDate {
        as_of + chrono::Days::new(u64::from(self.weeks) * 7)
    }
}

impl Week {
    /// The first date of this bucket (CASH-SHORT-01, CASH-BUFFER-01: a stretch is due on the
    /// first day of its first week, not the date of the Placement that caused it).
    pub fn start(self, as_of: NaiveDate) -> NaiveDate {
        as_of + chrono::Days::new(u64::from(self.0.saturating_sub(1)) * 7)
    }
}

/// How firm the evidence behind a Placement is (ADR-0020). Missing evidence is not a level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confidence {
    Firm,
    Estimated,
}

/// What put a receipt in its week.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Basis {
    DueDate,
    ExpectedDate,
    EntityHistory,
    CustomerHistory,
    ScheduledInvoice,
    PlannedDate,
    Overdue,
    PayRun,
    ScheduledBill,
    PurchaseOrder,
    /// An owner-entered expected receipt (CASH-SCHED-01).
    ScheduledReceipt,
    /// An owner-entered scheduled obligation (GAP-SCHED-01).
    ScheduledObligation,
    /// A credit card's run-date balance, paid on its payment day (GAP-CARD-01).
    CardBalance,
    /// A clearing account's run-date balance, placed in week 1 (CASH-OPEN-03).
    ClearingBalance,
    /// An intercompany invoice or bill, timed on the paying Entity's own dates (IC-DOC-01).
    IntercompanyDocument,
    /// An owner-entered intercompany settlement schedule occurrence (IC-LOAN-01).
    IntercompanySchedule,
    /// An owner-entered payroll schedule pay date (GAP-PAYROLL-01).
    PayrollSchedule,
    /// A payroll or tax liability balance at a period end (GAP-PAYROLL-03, GAP-TAX-01).
    BookedLiability,
    /// The sales-tax liability balance at the end of a reporting period, less what has been
    /// remitted since (GAP-TAX-01, Q225). Named apart from `BookedLiability` because the Scenario
    /// documents do: a payroll remittance is `booked liability`, a sales tax one `booked tax
    /// liability`.
    BookedTaxLiability,
    /// The Entity's last remittance of the same length (GAP-PAYROLL-03, GAP-TAX-02, Q119).
    LastRemittance,
    /// An owner-entered corporate income tax instalment (GAP-INCOME-01).
    Instalment,
    /// A booked income-tax payable balance for a completed tax year (GAP-INCOME-01).
    BalanceDue,
    /// An accrued liability settled on an owner-entered date (GAP-ACCRUAL-01).
    Accrual,
}

/// Why a Placement has no bucket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Exclusion {
    NotIssued,
    Intercompany,
    BeyondHorizon(NaiveDate),
    HistoryUncollected,
    NoTimingEvidence,
    CoveredBy(FactId),
    TemplateNotAutomatic,
    OffsetByCredit(FactId),
    CreditBalance,
    CommittedPurchaseTimingUnknown,
    VendorCreditBalance,
    /// The Entity's Home Currency is not the Reporting Currency and the owner has set no rate
    /// for the pair, so it is left out of the Group totals (CASH-GROUP-02).
    NoConversionRate,
    /// A bank account the Entity maps as restricted (CASH-OPEN-02).
    Restricted,
    /// A credit card account, never cash however a ledger types it (CASH-OPEN-04).
    CreditCardBalance,
    /// A credit card with no payment day Setting (GAP-CARD-01).
    NoCardPaymentDay,
    /// A loan or lease liability with no covering schedule (GAP-LOAN-01).
    NoSchedule,
    /// Payroll schedule set but no remitter type (GAP-PAYROLL-04).
    NoRemitterType,
    /// A remittance with neither an expected amount nor a last actual (GAP-PAYROLL-03, Q119).
    RemittanceAmountUnknown,
    /// A sales-tax account in debit at the period end: a refund the authority decides, never a
    /// forecast receipt (GAP-TAX-01).
    SalesTaxRefundPosition,
    /// No reporting period is set and none is in the ledger, so nothing times the remittance
    /// (GAP-TAX-03).
    NoSalesTaxPeriod,
    /// A running period with no prior actual remittance to estimate from (GAP-TAX-02, Q119).
    SalesTaxEstimateUnavailable,
    /// The obligation's jurisdiction has no calendar in Reference Data (GAP-TAX-04, Q122, Q228).
    NoCalendar,
    /// An accrued liability with no expected settlement Setting (GAP-ACCRUAL-01).
    NoExpectedSettlement,
    /// An intercompany balance with no settlement schedule (IC-LOAN-02).
    NoSettlementSchedule,
    /// An intercompany balance the owner confirmed is not settling within the horizon (IC-LOAN-02).
    ConfirmedNotSettling,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// A position the run states as of the run date rather than placing in a week: Opening Cash
    /// (CASH-OPEN-01). It has no week, and the roll-forward starts from it instead of moving it.
    Stated {
        confidence: Confidence,
    },
    Placed {
        week: Week,
        /// The date the cash is expected on.
        date: NaiveDate,
        /// Which way the money moves. Set by the Rule that places it, because a Family is not
        /// the answer: IC-DOC-01 places one pair out of the paying Entity and in to the
        /// receiving one. CASH-ROLL-01 reads it, and CASH-CONF-01 counts a payment by its size
        /// without netting it against a receipt.
        direction: Direction,
        basis: Basis,
        confidence: Confidence,
    },
    Excluded(Exclusion),
    /// Shown beside the forecast, never counted in it (`CONTEXT.md`, Headroom): CASH-HEAD-01's
    /// undrawn credit. It has no week and rolls into nothing, the same way `Stated` and `Excluded`
    /// don't.
    Shown,
}

impl Outcome {
    /// A date's week in the Horizon, or an exclusion beyond it.
    pub fn placed_on(
        horizon: Horizon,
        as_of: NaiveDate,
        date: NaiveDate,
        direction: Direction,
        basis: Basis,
        confidence: Confidence,
    ) -> Outcome {
        match horizon.week_of(as_of, date) {
            Some(week) => Outcome::Placed {
                week,
                date,
                direction,
                basis,
                confidence,
            },
            None => Outcome::Excluded(Exclusion::BeyondHorizon(date)),
        }
    }
}

/// The collection history behind a Placement (AR-TIME-02, Q76, Q147).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct History {
    /// `None` when more than half the comparable invoices were never collected.
    pub median_days: Option<i64>,
    pub comparable: usize,
    /// Comparable invoices closed with no payment (AR-TIME-03, Q77).
    pub never_collected: usize,
}

/// A foreign-currency document's own amount and the rate booked on it (AR-FX-01).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForeignAmount {
    pub currency: Currency,
    pub amount: Decimal,
    pub rate: Decimal,
}

/// Unapplied credit netted against a Placement (AR-UNAPPLIED-01).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reduction {
    pub credit: FactId,
    /// In Home Currency.
    pub amount: Decimal,
    /// True when an earlier invoice already took part of this credit.
    pub remainder: bool,
}

/// What an output is about. Identity survives across runs (Q31, Q221).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Subject {
    Fact(FactId),
    /// One scheduled date of a template. `entity` is whose forecast the occurrence belongs to —
    /// the template's own Entity for GAP/AR/AP, and the owing or owed Entity for IC-LOAN-01.
    Occurrence {
        entity: EntityId,
        template: FactId,
        date: NaiveDate,
    },
    /// An account at the end of a month, given by its first day.
    AccountMonth {
        account: FactId,
        month: NaiveDate,
    },
    /// Two facts one finding is about, ordered by id (AP-DUP-01, AP-PAID-01).
    Pair(FactId, FactId),
    /// An Entity's own position, citing the accounts it was read from (CASH-OPEN-01).
    Position {
        entity: EntityId,
        accounts: Vec<FactId>,
    },
    /// The Group itself, which is what CASH-WEEK-01's horizon is set for. The only Subject with
    /// no Entity behind it.
    Group,
    /// The currency pair one Entity needs a rate for (CASH-GROUP-02). It names the Entity,
    /// because a Group finding joins the queue of the Entity it concerns (Q155).
    Conversion {
        entity: EntityId,
        from: Currency,
        to: Currency,
    },
    /// An Entity's own cash position, not any one account or document (CASH-SHORT-01,
    /// CASH-BUFFER-01): the finding is about the whole roll-forward, not a citation of what fed
    /// it, which is `Position`'s own meaning (CASH-OPEN-01).
    Entity(EntityId),
    /// A credit line named by a Setting but not yet mapped to an account (CASH-HEAD-01): the
    /// Entity whose queue the item joins, and the name the owner gave it.
    CreditLine {
        entity: EntityId,
        name: String,
    },
    /// One Entity's own leg of a paired intercompany Placement (IC-DOC-01): the Entity this leg
    /// belongs to, and the receivable document both legs are named by, so a reader sees they are
    /// the same transaction (CASH-S09) — even the paying Entity's own leg, whose amount comes
    /// from its own booking, not this one. IC-ELIM-01 groups by `document` to find the pair.
    Intercompany {
        entity: EntityId,
        document: FactId,
    },
    /// Two Entities one finding is about (Q161): IC-AGREE-01/02 and dual-queue settlement items.
    /// `month` is the first day of the agreement month when the finding is about one; `None`
    /// when the subject is the pair alone.
    EntityPair {
        a: EntityId,
        b: EntityId,
        month: Option<NaiveDate>,
    },
    /// A net-pay date from the payroll schedule (GAP-PAYROLL-01).
    PayrollPay {
        entity: EntityId,
        date: NaiveDate,
    },
    /// A source-deduction remittance for one remittance period (GAP-PAYROLL-03).
    PayrollRemittance {
        entity: EntityId,
        /// First day of the remittance period the remittance covers (calendar from remitter type).
        pay_month: NaiveDate,
        /// Last day of that remittance period (may be mid-month for Accelerated threshold 1).
        period_end: NaiveDate,
        /// Statutory due before weekend/holiday move.
        raw_due: NaiveDate,
        /// Due date after the calendar moves it.
        due: NaiveDate,
        /// Pay dates in the period, when the amount comes from the schedule (empty when booked).
        runs: Vec<NaiveDate>,
    },
    /// All source-deduction remittances when the remitter type is unset (GAP-PAYROLL-04).
    PayrollRemittances {
        entity: EntityId,
    },
    /// Remittances that lack an amount, listed together (GAP-PAYROLL-03, Q158).
    PayrollRemittancesUnknown {
        entity: EntityId,
        /// `(period first day, effective due)` for each excluded remittance.
        remittances: Vec<(NaiveDate, NaiveDate)>,
    },
    /// A sales tax remittance for one reporting period (GAP-TAX-01, GAP-TAX-02). It is not named
    /// by a liability account, because the balance is the sum over every account of the
    /// jurisdiction, including a tax suspense account (Q225).
    SalesTaxRemittance {
        entity: EntityId,
        jurisdiction: Jurisdiction,
        /// First and last day of the period the remittance covers.
        period: (NaiveDate, NaiveDate),
        /// Statutory due date, before the calendar moved it.
        raw_due: NaiveDate,
        due: NaiveDate,
    },
    /// One sales-tax liability account whose balance is excluded (GAP-TAX-03, GAP-TAX-04): the
    /// exclusion is about the account, since nothing times an obligation it could be part of.
    SalesTaxAccount {
        account: FactId,
        jurisdiction: Jurisdiction,
    },
    /// One Entity's sales tax in one jurisdiction, which is what asking for a reporting period or
    /// naming an unsupported jurisdiction is about (GAP-TAX-03, GAP-TAX-04) — not any one account.
    SalesTaxJurisdiction {
        entity: EntityId,
        jurisdiction: Jurisdiction,
    },
    /// A corporate income tax instalment on its statutory date (GAP-INCOME-01).
    CorporateInstalment {
        entity: EntityId,
        /// Statutory due before weekend/holiday move.
        raw_due: NaiveDate,
        due: NaiveDate,
    },
    /// A booked income-tax balance for a completed tax year (GAP-INCOME-01, Q160).
    CorporateBalanceDue {
        entity: EntityId,
        year_end: NaiveDate,
        raw_due: NaiveDate,
        due: NaiveDate,
    },
}

impl Subject {
    /// The Entity the output belongs to, or `None` for one about the Group.
    pub fn entity(&self) -> Option<&EntityId> {
        match self {
            Self::Fact(id)
            | Self::AccountMonth { account: id, .. }
            | Self::Pair(id, _)
            | Self::SalesTaxAccount { account: id, .. } => Some(&id.entity),
            Self::Occurrence { entity, .. }
            | Self::Position { entity, .. }
            | Self::Entity(entity)
            | Self::Conversion { entity, .. }
            | Self::CreditLine { entity, .. }
            | Self::Intercompany { entity, .. }
            | Self::PayrollPay { entity, .. }
            | Self::PayrollRemittance { entity, .. }
            | Self::PayrollRemittances { entity }
            | Self::PayrollRemittancesUnknown { entity, .. }
            | Self::SalesTaxRemittance { entity, .. }
            | Self::SalesTaxJurisdiction { entity, .. }
            | Self::CorporateInstalment { entity, .. }
            | Self::CorporateBalanceDue { entity, .. } => Some(entity),
            // Q161: one item in both queues; neither Entity alone owns it.
            Self::Group | Self::EntityPair { .. } => None,
        }
    }

    /// How a trust-marked Placement on this Subject is named where a shortfall's evidence lists
    /// what falls inside it (CASH-SHORT-01, Q282). A Rule-produced name, never a provider account
    /// title (ADR-0013); `None` where the Subject carries no name of its own, and the citing
    /// Rule's id is the only stable label left.
    pub fn trust_label(&self) -> Option<String> {
        match self {
            Self::Fact(id) => Some(id.provider_id.clone()),
            Self::Occurrence { template, date, .. } => {
                Some(format!("{} {date}", template.provider_id))
            }
            Self::SalesTaxRemittance { jurisdiction, .. } => {
                Some(format!("{} remittance", sales_tax_name(*jurisdiction)))
            }
            _ => None,
        }
    }
}

/// Why an obligation is named first when cash is short (AP-PRIORITY-01). Recorded here; ranked
/// by CASH-ORDER-01.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    GovernmentTrust,
    CriticalVendor,
    SecuredLender,
}

/// One Canonical Fact or Gap assigned by one Rule to one week, or explicitly to none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    pub subject: Subject,
    /// The Rule that decided the outcome.
    pub rule: RuleId,
    /// Every Rule that shaped the Placement, the deciding one first.
    pub rules: Vec<RuleId>,
    pub outcome: Outcome,
    /// In Home Currency; `None` when nothing is placed or excluded by amount.
    pub amount: Option<HomeAmount>,
    pub history: Option<History>,
    pub foreign: Option<ForeignAmount>,
    pub reductions: Vec<Reduction>,
    pub priority: Option<Priority>,
}

impl Placement {
    pub fn cites(&self, rule: RuleId) -> bool {
        self.rules.contains(&rule)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ItemKind {
    Collect,
    SetExpectedDate,
    ExpectedDatePassed,
    InvoiceMissed,
    ApplyCredit,
    RefundOrApplyCredit,
    ReviewForWriteOff,
    InvoiceAccruedRevenue,
    ReconcileAr,
    ApproveOrDeleteBill,
    PlannedDatePassed,
    OverdueBill,
    TakeDiscount,
    ApplyVendorCredit,
    ClaimRefundOrHoldCredit,
    PossibleDuplicateBill,
    BillMayAlreadyBePaid,
    ReconcileAp,
    SetHorizon,
    CashShortfall,
    BelowBuffer,
    SetConversionRate,
    /// GAP-CARD-01: no payment day set for a credit card.
    SetCardPaymentDay,
    /// GAP-LOAN-01: a loan or lease balance needs its payment schedule.
    EnterLoanSchedule,
    /// CASH-HEAD-01: a credit line limit Setting names an account nothing maps.
    MapCreditLineAccount,
    /// IC-FUND-01: a connected Entity could cover another's shortfall.
    ConsiderIntercompanyFunding,
    /// IC-MAP-01: an intercompany account or contact with no counterparty Entity named.
    NameIntercompanyCounterparty,
    /// IC-ONESIDED-01: the named counterparty Entity is not connected (no books in the run).
    ConnectIntercompanyCounterparty,
    /// IC-DOC-02: the other Entity has not booked the matching document.
    CounterpartyHasNotBooked,
    /// IC-LOAN-02: enter a settlement schedule or confirm not settling within the horizon.
    IntercompanySettlementPlan,
    /// IC-AGREE-01: same-currency reciprocal balances disagree past tolerance.
    IntercompanyBalancesDisagree,
    /// IC-AGREE-02: cross-currency reciprocal balances differ after translation.
    IntercompanyCurrencyDifference,
    /// GAP-PAYROLL-02: payroll activity with no schedule.
    SetUpPayrollSchedule,
    /// GAP-PAYROLL-04: schedule set but no remitter type.
    SetRemitterType,
    /// GAP-PAYROLL-03: remittances with no amount source (Q158).
    PayrollRemittanceAmountUnknown,
    /// GAP-TAX-01: a sales-tax account in debit at the period end.
    SalesTaxRefundPosition,
    /// GAP-TAX-01: the tax accounting scheme is unset, so the basis is assumed (Q226, Q253).
    ConfirmTaxAccountingScheme,
    /// GAP-TAX-02: a running period with no prior actual to estimate from (Q119).
    SalesTaxEstimateUnavailable,
    /// GAP-TAX-03: the reporting period came from the ledger and is unconfirmed (Q113, Q253).
    ConfirmSalesTaxPeriod,
    /// GAP-TAX-03: no reporting period anywhere.
    SetSalesTaxPeriod,
    /// GAP-TAX-04: the jurisdiction has no calendar in Reference Data (Q122, Q228).
    UnsupportedTaxJurisdiction,
    /// GAP-INCOME-01: whether the later balance-due date applies is unset (Q160).
    ConfirmIncomeTaxBalanceDueDate,
    /// GAP-ACCRUAL-01: an accrued liability has no expected settlement.
    WhenWillAccrualBePaid,
}

/// The role that usually acts on a Decision Item (ADR-0020).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Owner,
    Bookkeeper,
    Accountant,
}

/// A class, never a score (ADR-0020, Q92).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Critical,
    Blocking,
    Action,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Evidence {
    ExpectedDatePassed(NaiveDate),
    Balance(HomeAmount),
    ControlDifference {
        open: HomeAmount,
        control: HomeAmount,
        difference: HomeAmount,
    },
    /// The due date and, when one was entered and has passed, the planned date (AP-TIME-03).
    Overdue {
        due: NaiveDate,
        planned: Option<NaiveDate>,
    },
    PlannedDatePassed(NaiveDate),
    /// Pay this much by this date to save this much (AP-DISC-02).
    Discount {
        pay: HomeAmount,
        by: NaiveDate,
        saving: HomeAmount,
        /// A percentage, exact.
        implied_annual_rate: Decimal,
    },
    Duplicate(DuplicateMatch),
    /// A spend that looks like this bill already paid (AP-PAID-01).
    MaybePaid,
    /// The reporting period the ledger holds, which GAP-TAX-03 times remittances from until the
    /// accountant confirms it (Q113, Q253).
    LedgerSalesTaxPeriod(SalesTaxReportingPeriod),
    /// CASH-SHORT-01's and CASH-BUFFER-01's evidence: every continuous stretch, the Entity's
    /// buffer when the finding is CASH-BUFFER-01's, and the Headroom available. `trust_capable` is
    /// false for CASH-BUFFER-01 always, since its own catalogue entry never mentions trust (Q276).
    /// For CASH-SHORT-01, whether a stretch's trust status is named at all is Q279's reading:
    /// whenever the Entity carries a government-trust obligation somewhere in the run
    /// (`trust_present`), which is when "inside" is added to a stretch with none, or whenever any
    /// stretch runs more than one week or there is more than one of them — a sentence naming only
    /// a single one-week stretch and nothing else stays short (CASH-S04); one spanning to the end
    /// of the horizon says so even with no trust anywhere to name (CASH-S09).
    CashStretches {
        buffer: Option<HomeAmount>,
        stretches: Vec<Stretch>,
        trust_capable: bool,
        trust_present: bool,
        headroom: Option<HomeAmount>,
    },
    /// IC-FUND-01: each sister that could cover, and the Rule's fixed caveats (renderer lists them).
    IntercompanyFunding {
        helpers: Vec<FundingHelper>,
    },
    /// IC-AGREE-01: each side's reading of the pair and the difference.
    IntercompanyBalanceDisagreement {
        a: EntityId,
        a_amount: HomeAmount,
        /// True when `a_amount` is owed *to* `a` (due-from); false when `a` owes it (due-to).
        a_owed_to_it: bool,
        b: EntityId,
        b_amount: HomeAmount,
        difference: HomeAmount,
    },
    /// IC-AGREE-02: each side in Reporting Currency and the residual after conversion.
    IntercompanyCurrencyMovement {
        foreign_entity: EntityId,
        foreign_amount: ForeignAmount,
        converted: HomeAmount,
        home_entity: EntityId,
        home_amount: HomeAmount,
        difference: HomeAmount,
    },
}

/// One sister Entity that could fund a shortfall (IC-FUND-01, Q163).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FundingHelper {
    pub entity: EntityId,
    pub amount: HomeAmount,
    pub week: Week,
}

/// One continuous run of weeks in the same zone — below zero (CASH-SHORT-01) or at or above zero
/// but below the buffer (CASH-BUFFER-01) — with its lowest closing cash and, when relevant, the
/// government-trust-marked Placements landing inside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stretch {
    pub first: Week,
    pub last: Week,
    pub lowest: HomeAmount,
    /// Trust obligations inside this stretch, named with amount and placed week (Q282).
    pub trust: Vec<TrustInStretch>,
}

/// A government-trust Placement falling inside a CASH-SHORT-01 stretch (Q282).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustInStretch {
    /// Rule-produced name shown in evidence (e.g. `GST/HST remittance`); never a provider
    /// account title (ADR-0013).
    pub label: String,
    pub amount: HomeAmount,
    pub week: Week,
}

/// What two bills share (AP-DUP-01).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DuplicateMatch {
    SameReference,
    SameAmountAndDate,
}

/// A proposed ledger transaction in the form the ledger uses for the fix. An account the Entity
/// has not mapped is `None`: the draft is incomplete, never filled in by guess (ADR-0019).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DraftCorrection {
    CreditNote {
        invoice: FactId,
        amount: HomeAmount,
        account: Option<FactId>,
    },
    PaymentApplication {
        from: FactId,
        /// Each document and the Home Currency amount applied to it.
        to: Vec<(FactId, Decimal)>,
    },
    /// A draft bill or invoice mirroring the other side's intercompany document (IC-DOC-02).
    /// `Side::Payable` is a bill; `Side::Receivable` is an invoice.
    MirrorDocument {
        side: crate::facts::Side,
        /// The counterparty the draft is from / to.
        counterparty: FactId,
        reference: String,
        dated: NaiveDate,
        due: Option<NaiveDate>,
        amount: HomeAmount,
    },
}

impl DraftCorrection {
    pub fn incomplete(&self) -> bool {
        match self {
            Self::CreditNote { account, .. } => account.is_none(),
            Self::PaymentApplication { .. } | Self::MirrorDocument { .. } => false,
        }
    }
}

/// An evidence-backed unit of action or review. Status and owner are M2 state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionItem {
    pub kind: ItemKind,
    pub subject: Subject,
    pub rule: RuleId,
    pub acted_on_by: Role,
    pub severity: Severity,
    pub evidence: Option<Evidence>,
    pub draft: Option<DraftCorrection>,
    /// Only where the finding has one (ADR-0020, Q93).
    pub due: Option<NaiveDate>,
    pub priority: Option<Priority>,
}

/// Why a run is Provisional: evidence a Rule needed was missing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProvisionalReason {
    pub rule: RuleId,
    pub subject: Subject,
}

/// CASH-CONF-01: the share of a week's placed amount at each Confidence level, as percentages
/// that sum to 100. A payment counts by its size and is never netted against a receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfidenceShares {
    pub firm: Decimal,
    pub estimated: Decimal,
}

/// One week of one Entity's forecast, in the Entity's Home Currency (CASH-ROLL-01).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeekForecast {
    pub week: Week,
    pub opening: HomeAmount,
    pub receipts: HomeAmount,
    pub payments: HomeAmount,
    pub closing: HomeAmount,
    /// `None` for a week with nothing placed, which has no shares to show (Q153).
    pub confidence: Option<ConfidenceShares>,
}

/// CASH-LOW-01: an Entity's lowest weekly closing cash, and the week it falls in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LowPoint {
    pub week: Week,
    pub amount: HomeAmount,
}

/// Where one Entity's cash starts and what each week does to it (CASH-OPEN-01, CASH-ROLL-01).
/// `weeks` is empty when the Group has set no horizon, which is CASH-WEEK-01's "no weeks are
/// produced". Headroom and the Low Point arrive with CASH-HEAD-01 and CASH-LOW-01.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityForecast {
    pub entity: EntityId,
    pub opening: HomeAmount,
    pub weeks: Vec<WeekForecast>,
    /// `None` when there are no weeks (CASH-WEEK-01).
    pub low_point: Option<LowPoint>,
    /// The Rules that produced this forecast. A Placement cites its Rules and so does this, so a
    /// Scenario declaring CASH-ROLL-01 or CASH-LOW-01 is answered by an output, not by nothing.
    pub rules: Vec<RuleId>,
}

/// One week of the Group's totals, in its Reporting Currency (CASH-GROUP-01). Each figure is the
/// sum of the included Entities' own, converted at the owner's rate for that Entity's pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupWeek {
    pub week: Week,
    pub opening: ReportingAmount,
    /// Excludes each paired intercompany leg (IC-ELIM-01): `intercompany_difference` carries what
    /// a cross-currency pair leaves behind instead.
    pub receipts: ReportingAmount,
    pub payments: ReportingAmount,
    /// The part of a cross-currency intercompany pair that does not eliminate (IC-ELIM-01), zero
    /// for a same-currency pair or a week with no intercompany Placement.
    pub intercompany_difference: ReportingAmount,
    pub closing: ReportingAmount,
}

/// CASH-GROUP-01's Group view: a labelled total of separate Entities' cash, which never means
/// cash can move between them (Q87), and which raises no Decision Item of its own however low it
/// goes. An Entity with no rate is not in `entities` and is CASH-GROUP-02's exclusion instead.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupView {
    pub currency: Currency,
    /// The Entities the total covers, in the order the run holds them.
    pub entities: Vec<EntityId>,
    pub opening: ReportingAmount,
    pub weeks: Vec<GroupWeek>,
    /// The Rules that produced it, as `EntityForecast` carries its own.
    pub rules: Vec<RuleId>,
}

/// One reproducible execution over fixed facts and Settings. The Horizon is one of those Settings
/// (CASH-WEEK-01), and is zero weeks when the Group has set none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForecastRun {
    pub as_of: NaiveDate,
    pub horizon: Horizon,
    pub placements: Vec<Placement>,
    pub decision_items: Vec<DecisionItem>,
    pub provisional: Vec<ProvisionalReason>,
    /// One per Entity, in the order the Entities are held.
    pub entities: Vec<EntityForecast>,
    /// `None` when the Group has set no Reporting Currency, which no Rule asks it to (Q277).
    pub group: Option<GroupView>,
}

impl ForecastRun {
    pub fn is_provisional(&self) -> bool {
        !self.provisional.is_empty()
    }
}

/// The engine. It cannot fail: missing configuration is output (ADR-0019, Q208).
pub fn run_forecast(facts: &Facts, settings: &Settings, as_of: NaiveDate) -> ForecastRun {
    let classifications = classify(facts, settings);
    let mut run = ForecastRun {
        as_of,
        horizon: settings.horizon.unwrap_or(Horizon { weeks: 0 }),
        placements: Vec::new(),
        decision_items: Vec::new(),
        provisional: Vec::new(),
        entities: Vec::new(),
        group: None,
    };
    // With no horizon there is no week to place into, so no Family Rule runs (CASH-WEEK-01).
    // Excluding every document as "beyond horizon" would state a reason that is not the reason.
    if settings.horizon.is_some() {
        for entity in facts.ledger_settings().keys() {
            crate::ar::run(&mut run, facts, settings, &classifications, entity);
            crate::ap::run(&mut run, facts, settings, &classifications, entity);
            crate::gap::run(&mut run, facts, settings, &classifications, entity);
        }
    }
    // Once, after the loop: CASH reads what every other Family placed, and CASH-GROUP-01 and
    // CASH-ORDER-01 cross Entity boundaries.
    crate::cash::run(&mut run, facts, settings, &classifications);
    run
}
