//! Canonical Facts, mirroring `docs/facts.md`. Where they disagree, this file is wrong.
//!
//! A fact enters `core` only through [`Facts`], whose `add_*` methods reject what makes a fact
//! meaningless (Q235). Add facts in reference order: ledger settings, then accounts and terms,
//! then counterparties, templates, documents, payments and applications.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{Month, NaiveDate};
use rust_decimal::Decimal;

use crate::money::{Currency, Money};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub String);

/// Deterministic, so a later sync of the same record keeps its id (Q221). No Transport.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FactId {
    pub entity: EntityId,
    pub provider: String,
    pub object_type: String,
    pub provider_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Provenance {
    Adapter {
        adapter: String,
        snapshot: String,
    },
    /// In M1, the Scenario that built the fact.
    Scenario(String),
}

/// Provider metadata for Classification only; Rules never read it directly (ADR-0013).
pub type Metadata = BTreeMap<String, String>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub id: FactId,
    pub provenance: Provenance,
    pub name: String,
    pub code: Option<String>,
    pub currency: Currency,
    pub metadata: Metadata,
    pub parent: Option<FactId>,
    pub active: bool,
}

/// A customer, vendor or tax authority, never merged (Q244).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Counterparty {
    pub id: FactId,
    pub provenance: Provenance,
    pub name: String,
    /// Rules that group by customer use the top-level parent (Q245).
    pub parent: Option<FactId>,
    pub currency: Option<Currency>,
    pub metadata: Metadata,
    /// Default terms as a customer and as a vendor; a Xero contact can carry both (Q255).
    pub receivable_terms: Option<FactId>,
    pub payable_terms: Option<FactId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DueRule {
    DaysAfterDate(u32),
    /// Xero's `DAYSAFTERBILLMONTH` (Q256).
    DaysAfterMonthEnd(u32),
    DayOfCurrentMonth(u32),
    DayOfFollowingMonth(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Discount {
    pub days: u32,
    pub percent: Decimal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaymentTerms {
    pub id: FactId,
    pub provenance: Provenance,
    pub due_rule: DueRule,
    pub discount: Option<Discount>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Receivable,
    Payable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentKind {
    Invoice,
    CreditNote,
}

/// Open, part-paid and paid are derived from the open amount, never stored (Q220, Q229).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentStatus {
    Draft,
    AwaitingApproval,
    Posted,
    Voided,
    Deleted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installment {
    pub due_date: NaiveDate,
    pub amount: Money,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    pub id: FactId,
    pub provenance: Provenance,
    pub side: Side,
    pub kind: DocumentKind,
    pub counterparty: FactId,
    pub number: Option<String>,
    pub vendor_reference: Option<String>,
    pub date: NaiveDate,
    pub status: DocumentStatus,
    /// Tax included.
    pub total: Money,
    pub tax: Money,
    /// Non-empty, summing to `total`.
    pub installments: Vec<Installment>,
    pub terms: Option<FactId>,
    pub expected_date: Option<NaiveDate>,
    pub template: Option<FactId>,
    pub control_account: FactId,
    /// The ledger's open amount, for the Adapter's check only; no Rule reads it (Q229).
    pub ledger_open: Money,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
}

/// The counterparty is required except for a tax authority (Q237).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaymentPurpose {
    Customer { counterparty: FactId },
    Vendor { counterparty: FactId },
    TaxAuthority { counterparty: Option<FactId> },
}

impl PaymentPurpose {
    pub fn counterparty(&self) -> Option<&FactId> {
        match self {
            Self::Customer { counterparty } | Self::Vendor { counterparty } => Some(counterparty),
            Self::TaxAuthority { counterparty } => counterparty.as_ref(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostingStatus {
    Posted,
    Deleted,
}

/// Includes overpayments and prepayments: a payment not yet applied (Q251).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Payment {
    pub id: FactId,
    pub provenance: Provenance,
    pub direction: Direction,
    pub purpose: PaymentPurpose,
    pub date: NaiveDate,
    pub amount: Money,
    /// Bank, card or clearing (ADR-0014).
    pub account: FactId,
    pub status: PostingStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplicationFrom {
    Payment(FactId),
    CreditNote(FactId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Application {
    pub id: FactId,
    pub provenance: Provenance,
    pub from: ApplicationFrom,
    pub to: FactId,
    pub date: NaiveDate,
    /// In the currency of the document applied to.
    pub amount: Money,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reconciliation {
    Reconciled,
    NotReconciled,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    pub record_type: String,
    pub id: String,
}

/// One transaction's posting to one account (Q217).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountLine {
    pub id: FactId,
    pub provenance: Provenance,
    pub account: FactId,
    pub date: NaiveDate,
    /// Signed, debit positive (Q231).
    pub amount: Money,
    pub counterparty: Option<FactId>,
    pub source: Source,
    pub reconciliation: Reconciliation,
    pub status: PostingStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountBalance {
    pub id: FactId,
    pub provenance: Provenance,
    pub account: FactId,
    pub as_of: NaiveDate,
    /// Signed, debit positive; book value, never revalued (Q240).
    pub amount: Money,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrequencyUnit {
    Day,
    Week,
    Month,
    Year,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frequency {
    pub unit: FrequencyUnit,
    pub interval: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemplateMode {
    Automatic,
    /// Generates draft documents; Xero only (Q254).
    AutomaticDraft,
    Reminder,
    Manual,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledTemplate {
    pub id: FactId,
    pub provenance: Provenance,
    pub side: Side,
    pub counterparty: FactId,
    pub amount: Money,
    pub frequency: Frequency,
    pub start: NaiveDate,
    /// Informational: Rules judge occurrences from documents.
    pub next: Option<NaiveDate>,
    pub end: Option<NaiveDate>,
    pub due_rule: DueRule,
    pub mode: TemplateMode,
    pub active: bool,
}

/// Purchase orders keep their own statuses (Q230).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurchaseOrderStatus {
    Draft,
    AwaitingApproval,
    Authorised,
    Billed,
    Closed,
    Deleted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PurchaseOrder {
    pub id: FactId,
    pub provenance: Provenance,
    pub vendor: FactId,
    pub status: PurchaseOrderStatus,
    pub delivery_date: Option<NaiveDate>,
    pub total: Money,
}

/// The tax accounting scheme a ledger holds (Q253).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SalesTaxBasis {
    Standard,
    Cash,
    FlatRate,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FiscalYearEnd {
    pub month: Month,
    pub day: u8,
}

/// One per Entity per sync (Q216).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerSettings {
    pub id: FactId,
    pub provenance: Provenance,
    pub home_currency: Currency,
    pub lock_date: Option<NaiveDate>,
    pub sales_tax_period: Option<String>,
    pub sales_tax_basis: Option<SalesTaxBasis>,
    pub fiscal_year_end: Option<FiscalYearEnd>,
}

/// Why a fact was refused entry to `core` (`docs/facts.md`, "What construction rejects").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejection {
    pub fact: FactId,
    pub reason: Reason,
}

impl Rejection {
    pub fn new(fact: FactId, reason: Reason) -> Self {
        Self { fact, reason }
    }
}

/// Numbered as in `docs/facts.md`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reason {
    /// 1. A reference that doesn't resolve in the same Entity.
    UnresolvedReference(Box<FactId>),
    /// 1. The fact's Entity has no ledger settings, so its Home Currency is unknown.
    UnknownEntity,
    /// 2.
    TransactionInHomeCurrency,
    /// 3.
    NegativeAmount,
    /// 4.
    ApplicationExceedsOpenAmount,
    /// 4.
    ApplicationCurrencyMismatch,
    /// 5.
    NoInstallments,
    /// 5.
    InstallmentsDoNotSumToTotal,
    /// 6.
    DuplicateId,
    /// One ledger settings fact per Entity (Q216).
    SecondLedgerSettings,
}

/// Every fact `core` reads, each already checked at construction.
#[derive(Clone, Debug, Default)]
pub struct Facts {
    ids: BTreeSet<FactId>,
    ledger_settings: BTreeMap<EntityId, LedgerSettings>,
    accounts: BTreeMap<FactId, Account>,
    counterparties: BTreeMap<FactId, Counterparty>,
    payment_terms: BTreeMap<FactId, PaymentTerms>,
    scheduled_templates: BTreeMap<FactId, ScheduledTemplate>,
    documents: BTreeMap<FactId, Document>,
    payments: BTreeMap<FactId, Payment>,
    applications: BTreeMap<FactId, Application>,
    account_lines: BTreeMap<FactId, AccountLine>,
    account_balances: BTreeMap<FactId, AccountBalance>,
    purchase_orders: BTreeMap<FactId, PurchaseOrder>,
}

fn resolve<'a, T>(
    map: &'a BTreeMap<FactId, T>,
    fact: &FactId,
    reference: &FactId,
) -> Result<&'a T, Rejection> {
    map.get(reference)
        .filter(|_| reference.entity == fact.entity)
        .ok_or_else(|| {
            Rejection::new(
                fact.clone(),
                Reason::UnresolvedReference(Box::new(reference.clone())),
            )
        })
}

fn resolve_optional<T>(
    map: &BTreeMap<FactId, T>,
    fact: &FactId,
    reference: Option<&FactId>,
) -> Result<(), Rejection> {
    reference.map_or(Ok(()), |r| resolve(map, fact, r).map(|_| ()))
}

fn check_money(
    fact: &FactId,
    money: &Money,
    home: Currency,
    non_negative: bool,
) -> Result<(), Rejection> {
    if money
        .transaction
        .as_ref()
        .is_some_and(|t| t.currency == home)
    {
        return Err(Rejection::new(
            fact.clone(),
            Reason::TransactionInHomeCurrency,
        ));
    }
    if non_negative && money.is_negative() {
        return Err(Rejection::new(fact.clone(), Reason::NegativeAmount));
    }
    Ok(())
}

/// `None` on overflow, so a sum never panics.
fn checked_sum(amounts: impl IntoIterator<Item = Decimal>) -> Option<Decimal> {
    amounts
        .into_iter()
        .try_fold(Decimal::ZERO, |sum, a| sum.checked_add(a))
}

fn check_installments(document: &Document) -> Result<(), Rejection> {
    let fact = || document.id.clone();
    if document.installments.is_empty() {
        return Err(Rejection::new(fact(), Reason::NoInstallments));
    }
    let (currency, total) = document.total.in_own_currency();
    let own = document
        .installments
        .iter()
        .map(|i| i.amount.in_own_currency());
    let sums = own.clone().all(|(c, _)| c == currency)
        && checked_sum(own.map(|(_, a)| a)) == Some(total)
        && checked_sum(document.installments.iter().map(|i| i.amount.home.0))
            == Some(document.total.home.0);
    if sums {
        Ok(())
    } else {
        Err(Rejection::new(fact(), Reason::InstallmentsDoNotSumToTotal))
    }
}

impl Facts {
    pub fn new() -> Self {
        Self::default()
    }

    /// Rejects a duplicate id, and returns the Home Currency of the fact's Entity.
    fn check_new(&self, fact: &FactId) -> Result<Currency, Rejection> {
        if self.ids.contains(fact) {
            return Err(Rejection::new(fact.clone(), Reason::DuplicateId));
        }
        self.ledger_settings
            .get(&fact.entity)
            .map(|s| s.home_currency)
            .ok_or_else(|| Rejection::new(fact.clone(), Reason::UnknownEntity))
    }

    pub fn add_ledger_settings(&mut self, settings: LedgerSettings) -> Result<(), Rejection> {
        let id = &settings.id;
        if self.ids.contains(id) {
            return Err(Rejection::new(id.clone(), Reason::DuplicateId));
        }
        if self.ledger_settings.contains_key(&id.entity) {
            return Err(Rejection::new(id.clone(), Reason::SecondLedgerSettings));
        }
        self.ids.insert(id.clone());
        self.ledger_settings.insert(id.entity.clone(), settings);
        Ok(())
    }

    pub fn add_account(&mut self, account: Account) -> Result<(), Rejection> {
        let id = &account.id;
        self.check_new(id)?;
        resolve_optional(&self.accounts, id, account.parent.as_ref())?;
        self.ids.insert(id.clone());
        self.accounts.insert(id.clone(), account);
        Ok(())
    }

    pub fn add_counterparty(&mut self, counterparty: Counterparty) -> Result<(), Rejection> {
        let id = &counterparty.id;
        self.check_new(id)?;
        resolve_optional(&self.counterparties, id, counterparty.parent.as_ref())?;
        resolve_optional(
            &self.payment_terms,
            id,
            counterparty.receivable_terms.as_ref(),
        )?;
        resolve_optional(&self.payment_terms, id, counterparty.payable_terms.as_ref())?;
        self.ids.insert(id.clone());
        self.counterparties.insert(id.clone(), counterparty);
        Ok(())
    }

    pub fn add_payment_terms(&mut self, terms: PaymentTerms) -> Result<(), Rejection> {
        let id = &terms.id;
        self.check_new(id)?;
        self.ids.insert(id.clone());
        self.payment_terms.insert(id.clone(), terms);
        Ok(())
    }

    pub fn add_scheduled_template(&mut self, template: ScheduledTemplate) -> Result<(), Rejection> {
        let id = &template.id;
        let home = self.check_new(id)?;
        resolve(&self.counterparties, id, &template.counterparty)?;
        check_money(id, &template.amount, home, false)?;
        self.ids.insert(id.clone());
        self.scheduled_templates.insert(id.clone(), template);
        Ok(())
    }

    pub fn add_document(&mut self, document: Document) -> Result<(), Rejection> {
        let id = &document.id;
        let home = self.check_new(id)?;
        resolve(&self.counterparties, id, &document.counterparty)?;
        resolve(&self.accounts, id, &document.control_account)?;
        resolve_optional(&self.payment_terms, id, document.terms.as_ref())?;
        resolve_optional(&self.scheduled_templates, id, document.template.as_ref())?;
        let amounts = [&document.total, &document.tax, &document.ledger_open];
        for money in amounts
            .into_iter()
            .chain(document.installments.iter().map(|i| &i.amount))
        {
            check_money(id, money, home, true)?;
        }
        check_installments(&document)?;
        self.ids.insert(id.clone());
        self.documents.insert(id.clone(), document);
        Ok(())
    }

    pub fn add_payment(&mut self, payment: Payment) -> Result<(), Rejection> {
        let id = &payment.id;
        let home = self.check_new(id)?;
        resolve_optional(&self.counterparties, id, payment.purpose.counterparty())?;
        resolve(&self.accounts, id, &payment.account)?;
        check_money(id, &payment.amount, home, true)?;
        self.ids.insert(id.clone());
        self.payments.insert(id.clone(), payment);
        Ok(())
    }

    pub fn add_application(&mut self, application: Application) -> Result<(), Rejection> {
        let id = &application.id;
        let home = self.check_new(id)?;
        match &application.from {
            ApplicationFrom::Payment(p) => resolve(&self.payments, id, p).map(|_| ())?,
            ApplicationFrom::CreditNote(c) => resolve(&self.documents, id, c).map(|_| ())?,
        }
        let document = resolve(&self.documents, id, &application.to)?;
        check_money(id, &application.amount, home, true)?;

        let (currency, total) = document.total.in_own_currency();
        let (applied_currency, amount) = application.amount.in_own_currency();
        if applied_currency != currency {
            return Err(Rejection::new(
                id.clone(),
                Reason::ApplicationCurrencyMismatch,
            ));
        }
        // ponytail: scans every application on each add, O(n²) per Entity; index by document if M2 volumes need it.
        let earlier = checked_sum(
            self.applications
                .values()
                .filter(|a| a.to == application.to)
                .map(|a| a.amount.in_own_currency().1),
        );
        let open = earlier.and_then(|e| total.checked_sub(e));
        if open.is_none_or(|open| amount > open) {
            return Err(Rejection::new(
                id.clone(),
                Reason::ApplicationExceedsOpenAmount,
            ));
        }
        self.ids.insert(id.clone());
        self.applications.insert(id.clone(), application);
        Ok(())
    }

    pub fn add_account_line(&mut self, line: AccountLine) -> Result<(), Rejection> {
        let id = &line.id;
        let home = self.check_new(id)?;
        resolve(&self.accounts, id, &line.account)?;
        resolve_optional(&self.counterparties, id, line.counterparty.as_ref())?;
        check_money(id, &line.amount, home, false)?;
        self.ids.insert(id.clone());
        self.account_lines.insert(id.clone(), line);
        Ok(())
    }

    pub fn add_account_balance(&mut self, balance: AccountBalance) -> Result<(), Rejection> {
        let id = &balance.id;
        let home = self.check_new(id)?;
        resolve(&self.accounts, id, &balance.account)?;
        check_money(id, &balance.amount, home, false)?;
        self.ids.insert(id.clone());
        self.account_balances.insert(id.clone(), balance);
        Ok(())
    }

    pub fn add_purchase_order(&mut self, order: PurchaseOrder) -> Result<(), Rejection> {
        let id = &order.id;
        let home = self.check_new(id)?;
        resolve(&self.counterparties, id, &order.vendor)?;
        check_money(id, &order.total, home, false)?;
        self.ids.insert(id.clone());
        self.purchase_orders.insert(id.clone(), order);
        Ok(())
    }

    pub fn ledger_settings(&self) -> &BTreeMap<EntityId, LedgerSettings> {
        &self.ledger_settings
    }

    /// The Scenario builder fills ledger-held tax fields after the Entity exists (GAP-TAX-03,
    /// Q253); construction already forbids a second `LedgerSettings` fact.
    pub fn ledger_settings_mut(&mut self, entity: &EntityId) -> Option<&mut LedgerSettings> {
        self.ledger_settings.get_mut(entity)
    }

    pub fn accounts(&self) -> &BTreeMap<FactId, Account> {
        &self.accounts
    }

    pub fn counterparties(&self) -> &BTreeMap<FactId, Counterparty> {
        &self.counterparties
    }

    pub fn payment_terms(&self) -> &BTreeMap<FactId, PaymentTerms> {
        &self.payment_terms
    }

    pub fn scheduled_templates(&self) -> &BTreeMap<FactId, ScheduledTemplate> {
        &self.scheduled_templates
    }

    pub fn documents(&self) -> &BTreeMap<FactId, Document> {
        &self.documents
    }

    pub fn payments(&self) -> &BTreeMap<FactId, Payment> {
        &self.payments
    }

    pub fn applications(&self) -> &BTreeMap<FactId, Application> {
        &self.applications
    }

    pub fn account_lines(&self) -> &BTreeMap<FactId, AccountLine> {
        &self.account_lines
    }

    pub fn account_balances(&self) -> &BTreeMap<FactId, AccountBalance> {
        &self.account_balances
    }

    pub fn purchase_orders(&self) -> &BTreeMap<FactId, PurchaseOrder> {
        &self.purchase_orders
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::{Quote, TransactionAmount, TransactionPart};

    fn id(entity: &str, object_type: &str, provider_id: &str) -> FactId {
        FactId {
            entity: EntityId(entity.into()),
            provider: "scenario".into(),
            object_type: object_type.into(),
            provider_id: provider_id.into(),
        }
    }

    fn cad(cents: i64) -> Money {
        Money::home(Decimal::new(cents, 2))
    }

    fn usd(cents: i64, home_cents: i64) -> Money {
        Money {
            home: crate::money::HomeAmount(Decimal::new(home_cents, 2)),
            transaction: Some(TransactionPart {
                amount: TransactionAmount(Decimal::new(cents, 2)),
                currency: Currency::new("USD").unwrap(),
                rate: Decimal::new(137, 2),
                quote: Quote::HomePerUnit,
                home_derived: false,
            }),
        }
    }

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 7).unwrap()
    }

    fn provenance() -> Provenance {
        Provenance::Scenario("test".into())
    }

    /// An Entity in CAD with an AR account, a customer, one CAD and one USD invoice, and a payment.
    fn entity() -> Facts {
        let mut facts = Facts::new();
        facts
            .add_ledger_settings(LedgerSettings {
                id: id("maple", "Preferences", "1"),
                provenance: provenance(),
                home_currency: Currency::new("CAD").unwrap(),
                lock_date: None,
                sales_tax_period: None,
                sales_tax_basis: None,
                fiscal_year_end: None,
            })
            .unwrap();
        facts
            .add_account(Account {
                id: id("maple", "Account", "ar"),
                provenance: provenance(),
                name: "Accounts Receivable".into(),
                code: None,
                currency: Currency::new("CAD").unwrap(),
                metadata: Metadata::new(),
                parent: None,
                active: true,
            })
            .unwrap();
        facts
            .add_counterparty(Counterparty {
                id: id("maple", "Customer", "harbourview"),
                provenance: provenance(),
                name: "Harbourview Strata Corp.".into(),
                parent: None,
                currency: None,
                metadata: Metadata::new(),
                receivable_terms: None,
                payable_terms: None,
            })
            .unwrap();
        facts.add_document(invoice("1001", cad(525_000))).unwrap();
        facts
            .add_document(invoice("1002", usd(100_000, 137_000)))
            .unwrap();
        facts.add_payment(payment(cad(525_000))).unwrap();
        facts
    }

    fn invoice(number: &str, total: Money) -> Document {
        Document {
            id: id("maple", "Invoice", number),
            provenance: provenance(),
            side: Side::Receivable,
            kind: DocumentKind::Invoice,
            counterparty: id("maple", "Customer", "harbourview"),
            number: Some(number.into()),
            vendor_reference: None,
            date: date(),
            status: DocumentStatus::Posted,
            tax: Money {
                home: crate::money::HomeAmount(Decimal::ZERO),
                transaction: total.transaction.clone(),
            },
            installments: vec![Installment {
                due_date: date(),
                amount: total.clone(),
            }],
            terms: None,
            expected_date: None,
            template: None,
            control_account: id("maple", "Account", "ar"),
            ledger_open: total.clone(),
            total,
        }
    }

    fn payment(amount: Money) -> Payment {
        Payment {
            id: id("maple", "Payment", "p1"),
            provenance: provenance(),
            direction: Direction::In,
            purpose: PaymentPurpose::Customer {
                counterparty: id("maple", "Customer", "harbourview"),
            },
            date: date(),
            amount,
            account: id("maple", "Account", "ar"),
            status: PostingStatus::Posted,
        }
    }

    fn application(provider_id: &str, to: &str, amount: Money) -> Application {
        Application {
            id: id("maple", "Application", provider_id),
            provenance: provenance(),
            from: ApplicationFrom::Payment(id("maple", "Payment", "p1")),
            to: id("maple", "Invoice", to),
            date: date(),
            amount,
        }
    }

    #[test]
    fn a_valid_entity_is_accepted() {
        let facts = entity();
        assert_eq!(facts.documents().len(), 2);
        assert_eq!(facts.payments().len(), 1);
    }

    #[test]
    fn construction_rejects_what_makes_a_fact_meaningless() {
        let mut facts = entity();
        let fact = |object_type, provider_id| id("maple", object_type, provider_id);

        // 1. Unresolved reference, including one into another Entity.
        let mut orphan = invoice("2001", cad(100));
        orphan.counterparty = id("other", "Customer", "harbourview");
        assert_eq!(
            facts.add_document(orphan),
            Err(Rejection::new(
                fact("Invoice", "2001"),
                Reason::UnresolvedReference(Box::new(id("other", "Customer", "harbourview")))
            ))
        );
        let mut stranger = invoice("2002", cad(100));
        stranger.id = id("other", "Invoice", "2002");
        assert_eq!(
            facts.add_document(stranger),
            Err(Rejection::new(
                id("other", "Invoice", "2002"),
                Reason::UnknownEntity
            ))
        );

        // 2. A transaction part in the home currency.
        let mut in_home = usd(100, 100);
        if let Some(t) = in_home.transaction.as_mut() {
            t.currency = Currency::new("CAD").unwrap();
        }
        assert_eq!(
            facts.add_document(invoice("2003", in_home)),
            Err(Rejection::new(
                fact("Invoice", "2003"),
                Reason::TransactionInHomeCurrency
            ))
        );

        // 3. A negative amount.
        assert_eq!(
            facts.add_document(invoice("2004", cad(-100))),
            Err(Rejection::new(
                fact("Invoice", "2004"),
                Reason::NegativeAmount
            ))
        );

        // 4. Applications: over the open amount, and in another currency.
        facts
            .add_application(application("a1", "1001", cad(500_000)))
            .unwrap();
        assert_eq!(
            facts.add_application(application("a2", "1001", cad(25_001))),
            Err(Rejection::new(
                fact("Application", "a2"),
                Reason::ApplicationExceedsOpenAmount
            ))
        );
        facts
            .add_application(application("a3", "1001", cad(25_000)))
            .unwrap();
        assert_eq!(
            facts.add_application(application("a4", "1002", cad(100))),
            Err(Rejection::new(
                fact("Application", "a4"),
                Reason::ApplicationCurrencyMismatch
            ))
        );

        // 5. Installments: none, or not summing in either currency.
        let mut none = invoice("2005", cad(100));
        none.installments.clear();
        assert_eq!(
            facts.add_document(none),
            Err(Rejection::new(
                fact("Invoice", "2005"),
                Reason::NoInstallments
            ))
        );
        let mut short = invoice("2006", usd(100_000, 137_000));
        short.installments[0].amount = usd(99_999, 137_000);
        assert_eq!(
            facts.add_document(short),
            Err(Rejection::new(
                fact("Invoice", "2006"),
                Reason::InstallmentsDoNotSumToTotal
            ))
        );

        // 6. A duplicate id.
        assert_eq!(
            facts.add_document(invoice("1001", cad(100))),
            Err(Rejection::new(fact("Invoice", "1001"), Reason::DuplicateId))
        );

        // Nothing rejected was kept.
        assert_eq!(facts.documents().len(), 2);
        assert_eq!(facts.applications().len(), 2);
    }
}
