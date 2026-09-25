//! The Scenario builder, also the demo seeder (ADR-0010). One test file per Family in `tests/`.
//!
//! A Scenario is a named synthetic Group: Entities, Canonical Facts and Settings. Facts are
//! added one row of builder code per row of the Scenario document, with the document's own
//! spellings for dates (`10-20`) and amounts (`5,250.00`). A fact the engine rejects fails the
//! Scenario (Q235).

pub mod render;
pub mod tables;

use std::collections::BTreeMap;
use std::str::FromStr;

use cashsaas_core::facts::{
    Account, AccountBalance, AccountLine, Application, ApplicationFrom, Counterparty, Direction,
    Discount, Document, DocumentKind, DocumentStatus, DueRule, EntityId, FactId, Facts, Frequency,
    FrequencyUnit, Installment, LedgerSettings, Metadata, Payment, PaymentPurpose, PaymentTerms,
    PostingStatus, Provenance, PurchaseOrder, PurchaseOrderStatus, Reconciliation, Rejection,
    SalesTaxBasis, ScheduledTemplate, Side, Source, TemplateMode,
};
use cashsaas_core::forecast::{ForecastRun, Horizon, run_forecast};
use cashsaas_core::money::{
    Currency, HomeAmount, Money, Quote, TransactionAmount, TransactionPart,
};
use cashsaas_core::reference::{Jurisdiction, RemitterType};
use cashsaas_core::settings::{
    AccountClass, AccrualSettlement, CorporateTax, CounterpartyClass,
    IntercompanySettlementSchedule, PayrollSchedule, SalesTaxReportingPeriod, ScheduledObligation,
    ScheduledReceipt, Settings,
};
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;

/// The conventions every Family's Scenarios share (`docs/scenarios/*.md`, Conventions).
pub const RUN_DATE: (i32, u32, u32) = (2026, 10, 7);
pub const HORIZON_WEEKS: u32 = 13;

/// A `MM-DD` date in the Scenario's year, or a full `YYYY-MM-DD` date.
pub fn date(year: i32, text: &str) -> NaiveDate {
    let parsed = match text.len() {
        5 => NaiveDate::parse_from_str(&format!("{year}-{text}"), "%Y-%m-%d"),
        _ => NaiveDate::parse_from_str(text, "%Y-%m-%d"),
    };
    parsed.unwrap_or_else(|_| panic!("not a Scenario date: {text}"))
}

/// An amount as the Scenario document writes it: `5,250.00`.
pub fn amount(text: &str) -> Decimal {
    Decimal::from_str(&text.replace(',', ""))
        .unwrap_or_else(|_| panic!("not a Scenario amount: {text}"))
}

#[derive(Clone, Debug)]
pub struct Scenario {
    pub name: String,
    pub as_of: NaiveDate,
    facts: Facts,
    settings: Settings,
    /// The Entity facts are being added to.
    current: Option<EntityId>,
    /// Realistic names to the fact ids they were given, per Entity.
    names: BTreeMap<(EntityId, &'static str, String), FactId>,
    /// A vendor's default payment terms, stated before the contact is added (AP-PO-01).
    payable_terms: BTreeMap<String, FactId>,
}

impl Scenario {
    pub fn new(name: &str) -> Self {
        let (y, m, d) = RUN_DATE;
        Self {
            name: name.to_owned(),
            as_of: NaiveDate::from_ymd_opt(y, m, d).unwrap_or_default(),
            facts: Facts::new(),
            settings: Settings {
                horizon: Some(Horizon {
                    weeks: HORIZON_WEEKS,
                }),
                ..Settings::default()
            },
            current: None,
            names: BTreeMap::new(),
            payable_terms: BTreeMap::new(),
        }
    }

    /// The Group has set no horizon, which CASH-WEEK-01 asks for rather than assuming (Q10).
    pub fn no_horizon(&mut self) -> &mut Self {
        self.settings.horizon = None;
        self
    }

    pub fn facts(&self) -> &Facts {
        &self.facts
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn year(&self) -> i32 {
        self.as_of.year()
    }

    pub fn date(&self, text: &str) -> NaiveDate {
        date(self.year(), text)
    }

    pub fn run(&self) -> ForecastRun {
        run_forecast(&self.facts, &self.settings, self.as_of)
    }

    fn provenance(&self) -> Provenance {
        Provenance::Scenario(self.name.clone())
    }

    fn entity_id(&self) -> EntityId {
        self.current
            .clone()
            .unwrap_or_else(|| panic!("{}: add an Entity before its facts", self.name))
    }

    fn id(&self, object_type: &str, provider_id: &str) -> FactId {
        FactId {
            entity: self.entity_id(),
            provider: "scenario".to_owned(),
            object_type: object_type.to_owned(),
            provider_id: provider_id.to_owned(),
        }
    }

    fn accept(&self, result: Result<(), Rejection>) {
        if let Err(rejection) = result {
            panic!("{}: fact rejected: {rejection:?}", self.name);
        }
    }

    fn home(&self) -> Currency {
        self.facts
            .ledger_settings()
            .get(&self.entity_id())
            .map(|s| s.home_currency)
            .unwrap_or_else(|| panic!("{}: Entity has no ledger settings", self.name))
    }

    /// Adds an Entity and makes it current. Every Entity gets its AR control account and a
    /// bank account, named as a small business names them.
    pub fn entity(&mut self, name: &str, home_currency: &str) -> &mut Self {
        let entity = EntityId(name.to_owned());
        let home = Currency::new(home_currency)
            .unwrap_or_else(|| panic!("{}: not a currency: {home_currency}", self.name));
        self.current = Some(entity.clone());
        let settings = LedgerSettings {
            id: self.id("Preferences", "1"),
            provenance: self.provenance(),
            home_currency: home,
            lock_date: None,
            sales_tax_period: None,
            sales_tax_basis: None,
            fiscal_year_end: None,
        };
        let result = self.facts.add_ledger_settings(settings);
        self.accept(result);
        self.account("Accounts Receivable");
        self.account("Business Chequing");
        self
    }

    fn named(&mut self, object_type: &'static str, name: &str) -> Option<FactId> {
        self.names
            .get(&(self.entity_id(), object_type, name.to_owned()))
            .cloned()
    }

    fn remember(&mut self, object_type: &'static str, name: &str, id: FactId) {
        self.names
            .insert((self.entity_id(), object_type, name.to_owned()), id);
    }

    /// An account by name, added on first use.
    pub fn account(&mut self, name: &str) -> FactId {
        if let Some(id) = self.named("Account", name) {
            return id;
        }
        let id = self.id("Account", name);
        let account = Account {
            id: id.clone(),
            provenance: self.provenance(),
            name: name.to_owned(),
            code: None,
            currency: self.home(),
            metadata: Metadata::new(),
            parent: None,
            active: true,
        };
        let result = self.facts.add_account(account);
        self.accept(result);
        self.remember("Account", name, id.clone());
        id
    }

    /// The owner's mapping of an account to a Classification (ADR-0013).
    pub fn map_account(&mut self, name: &str, class: AccountClass) -> &mut Self {
        let id = self.account(name);
        self.settings.account_mappings.insert(id, class);
        self
    }

    /// An account's balance as of a date, as the trial balance states it (ADR-0014, Q72).
    pub fn balance(&mut self, account: &str, as_of: &str, total: &str) {
        let id = self.id("Balance", &format!("{account} {as_of}"));
        let account = self.account(account);
        let balance = AccountBalance {
            id,
            provenance: self.provenance(),
            account,
            as_of: self.date(as_of),
            amount: Money::home(amount(total)),
        };
        let result = self.facts.add_account_balance(balance);
        self.accept(result);
    }

    /// The account with a name, in any Entity.
    pub fn account_named(&self, name: &str) -> Option<&Account> {
        self.facts.accounts().values().find(|a| a.name == name)
    }

    /// The account with a name, in one Entity (CASH-S09: two Entities can bank at the same name,
    /// "RBC Business Chequing").
    pub fn account_named_in(&self, entity: &str, name: &str) -> Option<&Account> {
        self.facts
            .accounts()
            .values()
            .find(|a| a.id.entity.0 == entity && a.name == name)
    }

    /// A customer by name, added on first use.
    pub fn customer(&mut self, name: &str) -> FactId {
        self.contact("Customer", name)
    }

    /// A counterparty by name. One contact serves both sides (Q255), so a name already added as a
    /// customer is that same counterparty when a bill names it as a vendor, and the other way
    /// round.
    fn contact(&mut self, object_type: &'static str, name: &str) -> FactId {
        if let Some(id) = self.named("Customer", name) {
            return id;
        }
        if let Some(id) = self.named("Vendor", name) {
            return id;
        }
        let payable_terms = self.payable_terms.get(name).cloned();
        let id = self.id(object_type, name);
        let counterparty = Counterparty {
            id: id.clone(),
            provenance: self.provenance(),
            name: name.to_owned(),
            parent: None,
            currency: None,
            metadata: Metadata::new(),
            receivable_terms: None,
            payable_terms,
        };
        let result = self.facts.add_counterparty(counterparty);
        self.accept(result);
        self.remember(object_type, name, id.clone());
        id
    }

    /// The vendor's default payment terms, as a contact carries them (Q255): AP-PO-01 reads them
    /// to time a purchase order. A fact is never edited once added, so a vendor's terms are
    /// stated before its first document.
    pub fn vendor_terms(&mut self, vendor: &str, text: &str) -> &mut Self {
        let terms = self.terms(text);
        assert!(
            self.named("Customer", vendor).is_none() && self.named("Vendor", vendor).is_none(),
            "{}: state {vendor}'s terms before its first document",
            self.name
        );
        self.payable_terms.insert(vendor.to_owned(), terms);
        self
    }

    /// Payment terms as the document writes them, `net 30` or `2% 10 days, net 30`, added on
    /// first use.
    pub fn terms(&mut self, text: &str) -> FactId {
        if let Some(id) = self.named("Terms", text) {
            return id;
        }
        let bad = || -> ! { panic!("{}: not Scenario terms: {text}", self.name) };
        let (discount, net) = text
            .rsplit_once(", ")
            .map_or((None, text), |(d, n)| (Some(d), n));
        let days = net
            .strip_prefix("net ")
            .and_then(|n| n.parse().ok())
            .unwrap_or_else(|| bad());
        let discount = discount.map(|d| {
            let (percent, days) = d.split_once("% ").unwrap_or_else(|| bad());
            Discount {
                days: days
                    .strip_suffix(" days")
                    .and_then(|n| n.parse().ok())
                    .unwrap_or_else(|| bad()),
                percent: amount(percent),
            }
        });
        let id = self.id("Terms", text);
        let terms = PaymentTerms {
            id: id.clone(),
            provenance: self.provenance(),
            due_rule: DueRule::DaysAfterDate(days),
            discount,
        };
        let result = self.facts.add_payment_terms(terms);
        self.accept(result);
        self.remember("Terms", text, id.clone());
        id
    }

    /// The closing cash the owner chooses never to fall below (AP-DISC-02, CASH-BUFFER-01).
    /// The currency the Group consolidates in (CASH-GROUP-01). A Scenario with one Entity never
    /// consolidates, so only a Group Scenario sets it.
    pub fn reporting_currency(&mut self, code: &str) -> &mut Self {
        self.settings.reporting_currency = Some(
            Currency::new(code).unwrap_or_else(|| panic!("{}: not a currency: {code}", self.name)),
        );
        self
    }

    /// The owner's rate from one currency to another (CASH-GROUP-01, IC-ELIM-01, Q30).
    pub fn conversion_rate(&mut self, from: &str, to: &str, rate: &str) -> &mut Self {
        let bad = |code: &str| -> ! { panic!("{}: not a currency: {code}", self.name) };
        let from = Currency::new(from).unwrap_or_else(|| bad(from));
        let to = Currency::new(to).unwrap_or_else(|| bad(to));
        self.settings
            .conversion_rates
            .insert((from, to), amount(rate));
        self
    }

    /// The day of the month a credit card is paid on (GAP-CARD-01).
    pub fn card_payment_day(&mut self, account: &str, day: u32) -> &mut Self {
        let id = self.account(account);
        self.settings.card_payment_days.insert(id, day);
        self
    }

    /// A credit line's limit, named rather than an account added on first use (CASH-HEAD-01): the
    /// name can be a line no account is mapped to, which is what raises the item to map one.
    pub fn credit_line_limit(&mut self, name: &str, limit: &str) -> &mut Self {
        let entity = self.entity_id();
        self.settings
            .credit_line_limits
            .insert((entity, name.to_owned()), HomeAmount(amount(limit)));
        self
    }

    pub fn cash_buffer(&mut self, total: &str) -> &mut Self {
        let entity = self.entity_id();
        self.settings
            .minimum_cash_buffer
            .insert(entity, HomeAmount(amount(total)));
        self
    }

    /// The Entity's pay-run weekday (AP-RUN-01).
    pub fn pay_run(&mut self, weekday: chrono::Weekday) -> &mut Self {
        let entity = self.entity_id();
        self.settings.pay_run_weekday.insert(entity, weekday);
        self
    }

    /// The owner's mapping of a counterparty as holding amounts in trust for a government
    /// (AP-PRIORITY-01).
    pub fn government_trust(&mut self, vendor: &str) -> &mut Self {
        let id = self.vendor(vendor);
        self.settings
            .counterparty_mappings
            .insert(id, CounterpartyClass::GovernmentTrust);
        self
    }

    /// A vendor the owner marks as one the business cannot lose (AP-PRIORITY-01).
    pub fn critical_vendor(&mut self, vendor: &str) -> &mut Self {
        let id = self.vendor(vendor);
        self.settings.critical_vendors.insert(id);
        self
    }

    /// A vendor the owner marks as a secured lender (AP-PRIORITY-01).
    pub fn secured_lender(&mut self, vendor: &str) -> &mut Self {
        let id = self.vendor(vendor);
        self.settings.secured_lenders.insert(id);
        self
    }

    /// The owner's mapping of a customer or vendor as intercompany, naming the sister Entity.
    pub fn intercompany(&mut self, contact: &str, entity: &str) -> &mut Self {
        let id = self.customer(contact);
        self.settings.counterparty_mappings.insert(
            id,
            CounterpartyClass::Intercompany(Some(EntityId(entity.to_owned()))),
        );
        self
    }

    /// An intercompany contact with no counterparty Entity named (IC-MAP-01, Q162).
    pub fn intercompany_unnamed(&mut self, contact: &str) -> &mut Self {
        let id = self.customer(contact);
        self.settings
            .counterparty_mappings
            .insert(id, CounterpartyClass::Intercompany(None));
        self
    }

    /// An account mapped as intercompany, naming the sister Entity (IC-LOAN-*).
    pub fn intercompany_account(&mut self, account: &str, entity: &str) -> &mut Self {
        let id = self.account(account);
        self.settings.account_mappings.insert(
            id,
            AccountClass::Intercompany(Some(EntityId(entity.to_owned()))),
        );
        self
    }

    /// Optional per-Group tolerance for same-currency reciprocal balances (IC-AGREE-01).
    pub fn intercompany_tolerance(&mut self, total: &str) -> &mut Self {
        self.settings.intercompany_tolerance = Some(HomeAmount(amount(total)));
        self
    }

    /// An intercompany settlement schedule (IC-LOAN-01), entered under `id`. `owing_owed` is
    /// (Entity that pays, Entity that receives). Call `settlement_covers` to name the accounts.
    pub fn intercompany_settlement(
        &mut self,
        id: &str,
        owing_owed: (&str, &str),
        total: &str,
        unit: FrequencyUnit,
        start: &str,
        end: Option<&str>,
    ) -> &mut Self {
        let fact = self.id("IntercompanySettlement", id);
        self.settings.intercompany_settlement_schedules.insert(
            fact,
            IntercompanySettlementSchedule {
                owing: EntityId(owing_owed.0.to_owned()),
                owed: EntityId(owing_owed.1.to_owned()),
                amount: HomeAmount(amount(total)),
                frequency: Frequency { unit, interval: 1 },
                start: self.date(start),
                end: end.map(|d| self.date(d)),
                accounts: Vec::new(),
            },
        );
        self
    }

    /// The balance accounts an intercompany settlement schedule settles (IC-S02's L1 loans).
    pub fn settlement_covers(&mut self, id: &str, accounts: &[&str]) -> &mut Self {
        let fact = self.id("IntercompanySettlement", id);
        let resolved = accounts
            .iter()
            .map(|name| {
                self.facts()
                    .accounts()
                    .values()
                    .find(|a| a.name == *name)
                    .map(|a| a.id.clone())
                    .unwrap_or_else(|| {
                        panic!(
                            "{}: settlement_covers account `{name}` must exist before the schedule",
                            self.name
                        )
                    })
            })
            .collect();
        self.settings
            .intercompany_settlement_schedules
            .get_mut(&fact)
            .unwrap_or_else(|| {
                panic!(
                    "{}: settlement_covers `{id}` — call intercompany_settlement first",
                    self.name
                )
            })
            .accounts = resolved;
        self
    }

    /// Confirm an intercompany account is not settling within the horizon (IC-LOAN-02).
    pub fn intercompany_not_settling(&mut self, account: &str) -> &mut Self {
        let id = self.account(account);
        self.settings.intercompany_not_settling.insert(id);
        self
    }

    /// A vendor by name, added on first use.
    pub fn vendor(&mut self, name: &str) -> FactId {
        self.contact("Vendor", name)
    }

    /// An authorised purchase order, not yet billed.
    pub fn purchase_order(&mut self, number: &str, vendor: &str) -> PurchaseOrderBuilder<'_> {
        let vendor = self.vendor(vendor);
        PurchaseOrderBuilder {
            scenario: self,
            number: number.to_owned(),
            vendor,
            total: Decimal::ZERO,
            delivery: None,
        }
    }

    /// The purchase order with a provider id, in any Entity.
    pub fn purchase_order_numbered(&self, number: &str) -> Option<&PurchaseOrder> {
        self.facts
            .purchase_orders()
            .values()
            .find(|o| o.id.provider_id == number)
    }

    /// A customer invoice. Dated 30 days before it is due unless `dated` says otherwise.
    pub fn invoice(&mut self, number: &str, customer: &str) -> DocumentBuilder<'_> {
        let customer = self.customer(customer);
        self.document(Side::Receivable, number, customer)
    }

    /// A vendor bill, built as an invoice is.
    pub fn bill(&mut self, number: &str, vendor: &str) -> DocumentBuilder<'_> {
        let vendor = self.vendor(vendor);
        self.document(Side::Payable, number, vendor)
    }

    fn document(&mut self, side: Side, number: &str, counterparty: FactId) -> DocumentBuilder<'_> {
        DocumentBuilder {
            scenario: self,
            side,
            number: number.to_owned(),
            counterparty,
            paid_from: "Business Chequing",
            reference: None,
            dated: None,
            due: None,
            total: Decimal::ZERO,
            tax: Decimal::ZERO,
            status: DocumentStatus::Posted,
            foreign: None,
            terms: None,
            template: None,
            expected: None,
            paid: None,
            credited: None,
        }
    }

    /// A customer's payment with nothing applied to it: an overpayment or prepayment (Q251).
    pub fn overpayment(&mut self, number: &str, customer: &str, dated: &str, total: &str) {
        let customer = self.customer(customer);
        let purpose = PaymentPurpose::Customer {
            counterparty: customer,
        };
        self.unapplied_payment(Direction::In, number, purpose, dated, total);
    }

    /// A payment to a vendor with nothing applied to it: a supplier prepayment (Q251).
    pub fn prepayment(&mut self, number: &str, vendor: &str, dated: &str, total: &str) {
        let vendor = self.vendor(vendor);
        let purpose = PaymentPurpose::Vendor {
            counterparty: vendor,
        };
        self.unapplied_payment(Direction::Out, number, purpose, dated, total);
    }

    fn unapplied_payment(
        &mut self,
        direction: Direction,
        number: &str,
        purpose: PaymentPurpose,
        dated: &str,
        total: &str,
    ) {
        let payment = Payment {
            id: self.id("Payment", number),
            provenance: self.provenance(),
            direction,
            purpose,
            date: self.date(dated),
            amount: Money::home(amount(total)),
            account: self.account("Business Chequing"),
            status: PostingStatus::Posted,
        };
        let result = self.facts.add_payment(payment);
        self.accept(result);
    }

    /// A credit note issued to a customer with nothing applied to it.
    pub fn credit_note(&mut self, number: &str, customer: &str, dated: &str, total: &str) {
        let customer = self.customer(customer);
        self.unapplied_credit(Side::Receivable, number, customer, dated, total);
    }

    /// A credit note a vendor issued to the Entity, with nothing applied to it.
    pub fn vendor_credit(&mut self, number: &str, vendor: &str, dated: &str, total: &str) {
        let vendor = self.vendor(vendor);
        self.unapplied_credit(Side::Payable, number, vendor, dated, total);
    }

    fn unapplied_credit(
        &mut self,
        side: Side,
        number: &str,
        counterparty: FactId,
        dated: &str,
        total: &str,
    ) {
        let control = match side {
            Side::Receivable => "Accounts Receivable",
            Side::Payable => "Accounts Payable",
        };
        let total = Money::home(amount(total));
        let credit = Document {
            id: self.id("CreditNote", number),
            provenance: self.provenance(),
            side,
            kind: DocumentKind::CreditNote,
            counterparty,
            number: Some(number.to_owned()),
            vendor_reference: None,
            date: self.date(dated),
            status: DocumentStatus::Posted,
            total: total.clone(),
            tax: Money::home(Decimal::ZERO),
            installments: vec![Installment {
                due_date: self.date(dated),
                amount: total.clone(),
            }],
            terms: None,
            expected_date: None,
            template: None,
            control_account: self.account(control),
            ledger_open: total,
        };
        let result = self.facts.add_document(credit);
        self.accept(result);
    }

    /// A scheduled invoice template: automatic and monthly unless the builder says otherwise.
    pub fn template(&mut self, name: &str, customer: &str) -> TemplateBuilder<'_> {
        let customer = self.customer(customer);
        self.scheduled(Side::Receivable, name, customer)
    }

    /// A scheduled bill template, built as an invoice template is.
    pub fn bill_template(&mut self, name: &str, vendor: &str) -> TemplateBuilder<'_> {
        let vendor = self.vendor(vendor);
        self.scheduled(Side::Payable, name, vendor)
    }

    fn scheduled(&mut self, side: Side, name: &str, counterparty: FactId) -> TemplateBuilder<'_> {
        TemplateBuilder {
            scenario: self,
            side,
            name: name.to_owned(),
            counterparty,
            mode: TemplateMode::Automatic,
            amount: Decimal::ZERO,
            start: None,
            due_after: 0,
        }
    }

    /// Money leaving a bank or card account, recorded as its own transaction and not as a bill
    /// payment: the card charges and bank spends AP-SCHED-01 and AP-PAID-01 read. Stored as the
    /// credit it is (Q231); the Scenario documents show payments without a sign.
    pub fn spend(&mut self, number: &str, account: &str, payee: &str, dated: &str, total: &str) {
        let account = self.account(account);
        let payee = self.vendor(payee);
        let line = AccountLine {
            id: self.id("AccountLine", number),
            provenance: self.provenance(),
            account,
            date: self.date(dated),
            amount: Money::home(-amount(total)),
            counterparty: Some(payee),
            source: Source {
                record_type: "Expense".to_owned(),
                id: number.to_owned(),
            },
            reconciliation: Reconciliation::Reconciled,
            status: PostingStatus::Posted,
        };
        let result = self.facts.add_account_line(line);
        self.accept(result);
    }

    /// Money in to a bank account: the receipt CASH-SCHED-01 reads to see an occurrence already
    /// banked. `spend`'s mirror, stored as the debit it is (Q231).
    pub fn bank_receipt(
        &mut self,
        number: &str,
        account: &str,
        payer: &str,
        dated: &str,
        total: &str,
    ) {
        let account = self.account(account);
        let payer = self.customer(payer);
        let line = AccountLine {
            id: self.id("AccountLine", number),
            provenance: self.provenance(),
            account,
            date: self.date(dated),
            amount: Money::home(amount(total)),
            counterparty: Some(payer),
            source: Source {
                record_type: "Deposit".to_owned(),
                id: number.to_owned(),
            },
            reconciliation: Reconciliation::Reconciled,
            status: PostingStatus::Posted,
        };
        let result = self.facts.add_account_line(line);
        self.accept(result);
    }

    /// A receipt the owner expects that no ledger schedules (CASH-SCHED-01, Q132).
    pub fn scheduled_receipt(
        &mut self,
        id: &str,
        payer: &str,
        total: &str,
        every: FrequencyUnit,
        start: &str,
        end: Option<&str>,
    ) -> &mut Self {
        let payer = self.customer(payer);
        let receipt = ScheduledReceipt {
            payer,
            amount: HomeAmount(amount(total)),
            frequency: Frequency {
                unit: every,
                interval: 1,
            },
            start: self.date(start),
            end: end.map(|on| self.date(on)),
        };
        let key = self.id("ScheduledReceipt", id);
        self.settings.scheduled_receipts.insert(key, receipt);
        self
    }

    /// An obligation the owner expects that no ledger schedules (GAP-SCHED-01).
    pub fn scheduled_obligation(
        &mut self,
        id: &str,
        payee: &str,
        total: &str,
        every: FrequencyUnit,
        start: &str,
        end: Option<&str>,
    ) -> &mut Self {
        let payee = self.vendor(payee);
        let obligation = ScheduledObligation {
            payee,
            amount: HomeAmount(amount(total)),
            frequency: Frequency {
                unit: every,
                interval: 1,
            },
            start: self.date(start),
            end: end.map(|on| self.date(on)),
            covers_account: None,
        };
        let key = self.id("ScheduledObligation", id);
        self.settings.scheduled_obligations.insert(key, obligation);
        self
    }

    /// Marks an owner-entered schedule as covering a loan or lease account (GAP-LOAN-01).
    pub fn obligation_covers(&mut self, id: &str, account: &str) -> &mut Self {
        let account = self.account(account);
        let key = self.id("ScheduledObligation", id);
        self.settings
            .scheduled_obligations
            .get_mut(&key)
            .unwrap_or_else(|| panic!("{}: no scheduled obligation {id}", self.name))
            .covers_account = Some(account);
        self
    }

    /// Owner-entered payroll cadence (GAP-PAYROLL-01). `interval` is 1 for monthly/weekly and 2
    /// for biweekly. `expected_remittance` is optional (Q119).
    pub fn payroll_schedule(
        &mut self,
        every: FrequencyUnit,
        interval: u32,
        next_pay_date: &str,
        expected_net_pay: &str,
        expected_remittance: Option<&str>,
        paid_through: Option<&str>,
    ) -> &mut Self {
        let entity = self.entity_id();
        let paid_through = paid_through.map(|name| self.vendor(name));
        self.settings.payroll_schedules.insert(
            entity,
            PayrollSchedule {
                frequency: Frequency {
                    unit: every,
                    interval,
                },
                next_pay_date: self.date(next_pay_date),
                expected_net_pay: HomeAmount(amount(expected_net_pay)),
                expected_remittance: expected_remittance.map(|v| HomeAmount(amount(v))),
                paid_through,
            },
        );
        self
    }

    /// CRA remitter type (GAP-PAYROLL-04, ADR-0007).
    pub fn remitter_type(&mut self, remitter: RemitterType) -> &mut Self {
        let entity = self.entity_id();
        self.settings.remitter_types.insert(entity, remitter);
        self
    }

    /// Owner-confirmed sales tax reporting period (GAP-TAX-03).
    pub fn sales_tax_period(&mut self, period: SalesTaxReportingPeriod) -> &mut Self {
        let entity = self.entity_id();
        self.settings.sales_tax_periods.insert(entity, period);
        self
    }

    /// Jurisdiction of a sales-tax liability account (GAP-TAX-04); unset means Canada federal.
    pub fn tax_jurisdiction(&mut self, account: &str, jurisdiction: Jurisdiction) -> &mut Self {
        let id = self.account(account);
        self.settings.tax_jurisdictions.insert(id, jurisdiction);
        self
    }

    /// Corporate income tax timing (GAP-INCOME-01). `balance_due_extension` unset is Q160.
    pub fn corporate_tax(
        &mut self,
        tax_year_start: &str,
        monthly_instalment: Option<&str>,
        balance_due_extension: Option<bool>,
    ) -> &mut Self {
        let entity = self.entity_id();
        self.settings.corporate_tax.insert(
            entity,
            CorporateTax {
                tax_year_start: self.date(tax_year_start),
                monthly_instalment: monthly_instalment.map(|v| HomeAmount(amount(v))),
                balance_due_extension,
            },
        );
        self
    }

    /// Expected settlement for an accrued liability (GAP-ACCRUAL-01, Q120).
    pub fn accrual_settlement(
        &mut self,
        account: &str,
        settlement: AccrualSettlement,
    ) -> &mut Self {
        let id = self.account(account);
        self.settings.accrual_settlements.insert(id, settlement);
        self
    }

    /// Xero `SalesTaxPeriod` on the Entity's ledger settings (GAP-TAX-03, Q253).
    pub fn ledger_sales_tax_period(&mut self, value: &str) -> &mut Self {
        let entity = self.entity_id();
        self.facts
            .ledger_settings_mut(&entity)
            .unwrap_or_else(|| panic!("{}: Entity has no ledger settings", self.name))
            .sales_tax_period = Some(value.to_owned());
        self
    }

    /// Xero `SalesTaxBasis` on the Entity's ledger settings (GAP-TAX-01, Q253).
    pub fn ledger_sales_tax_basis(&mut self, basis: SalesTaxBasis) -> &mut Self {
        let entity = self.entity_id();
        self.facts
            .ledger_settings_mut(&entity)
            .unwrap_or_else(|| panic!("{}: Entity has no ledger settings", self.name))
            .sales_tax_basis = Some(basis);
        self
    }

    /// Lock date on the Entity's ledger settings (CASH-S03).
    pub fn lock_date(&mut self, on: &str) -> &mut Self {
        let entity = self.entity_id();
        let on = self.date(on);
        self.facts
            .ledger_settings_mut(&entity)
            .unwrap_or_else(|| panic!("{}: Entity has no ledger settings", self.name))
            .lock_date = Some(on);
        self
    }

    /// A payment marked as a tax remittance, with no payee (Q224): what GAP-TAX-01 and GAP-TAX-02
    /// read as money already sent to the authority.
    pub fn tax_remittance(
        &mut self,
        number: &str,
        account: &str,
        dated: &str,
        total: &str,
    ) -> &mut Self {
        let payment = Payment {
            id: self.id("Payment", number),
            provenance: self.provenance(),
            direction: Direction::Out,
            purpose: PaymentPurpose::TaxAuthority { counterparty: None },
            date: self.date(dated),
            amount: Money::home(amount(total)),
            account: self.account(account),
            status: PostingStatus::Posted,
        };
        let result = self.facts.add_payment(payment);
        self.accept(result);
        self
    }

    /// Money leaving a bank or card account with no payee (a QBO purchase). GAP-SCHED-02 cannot
    /// match it to an obligation, so the occurrence stays forecast.
    pub fn spend_no_payee(&mut self, number: &str, account: &str, dated: &str, total: &str) {
        let account = self.account(account);
        let line = AccountLine {
            id: self.id("AccountLine", number),
            provenance: self.provenance(),
            account,
            date: self.date(dated),
            amount: Money::home(-amount(total)),
            counterparty: None,
            source: Source {
                record_type: "Expense".to_owned(),
                id: number.to_owned(),
            },
            reconciliation: Reconciliation::Reconciled,
            status: PostingStatus::Posted,
        };
        let result = self.facts.add_account_line(line);
        self.accept(result);
    }

    /// The scheduled receipt entered under an id (CASH-SCHED-01).
    pub fn scheduled_receipt_named(&self, id: &str) -> Option<&ScheduledReceipt> {
        self.settings
            .scheduled_receipts
            .iter()
            .find(|(key, _)| key.provider_id == id)
            .map(|(_, receipt)| receipt)
    }

    /// The scheduled obligation entered under an id (GAP-SCHED-01).
    pub fn scheduled_obligation_named(&self, id: &str) -> Option<&ScheduledObligation> {
        self.settings
            .scheduled_obligations
            .iter()
            .find(|(key, _)| key.provider_id == id)
            .map(|(_, obligation)| obligation)
    }

    /// The account line with a provider id, in any Entity.
    pub fn account_line(&self, number: &str) -> Option<&AccountLine> {
        self.facts
            .account_lines()
            .values()
            .find(|l| l.id.provider_id == number)
    }

    /// The scheduled template with a provider id, in any Entity.
    pub fn scheduled_template(&self, name: &str) -> Option<&ScheduledTemplate> {
        self.facts
            .scheduled_templates()
            .values()
            .find(|t| t.id.provider_id == name)
    }

    /// The payment with a provider id, in any Entity.
    pub fn payment(&self, number: &str) -> Option<&Payment> {
        self.facts
            .payments()
            .values()
            .find(|p| p.id.provider_id == number)
    }

    /// The document with a number, in any Entity.
    pub fn document_numbered(&self, number: &str) -> Option<&Document> {
        self.facts
            .documents()
            .values()
            .find(|d| d.number.as_deref() == Some(number))
    }

    /// The document with a number in one Entity (IC-S01: both sides of a pair can share a number).
    pub fn document_numbered_in(&self, entity: &str, number: &str) -> Option<&Document> {
        self.facts.documents().values().find(|d| {
            d.number.as_deref() == Some(number)
                && (d.id.entity.0 == entity
                    || d.id
                        .entity
                        .0
                        .split_whitespace()
                        .next()
                        .is_some_and(|short| short == entity))
        })
    }

    pub fn counterparty_name(&self, id: &FactId) -> String {
        self.facts
            .counterparties()
            .get(id)
            .map(|c| c.name.clone())
            .unwrap_or_default()
    }

    fn money(&self, own: Decimal, foreign: Option<&(Currency, Decimal)>) -> Money {
        match foreign {
            None => Money::home(own),
            Some((currency, rate)) => Money {
                home: HomeAmount(own.saturating_mul(*rate).round_dp_with_strategy(
                    2,
                    rust_decimal::RoundingStrategy::MidpointAwayFromZero,
                )),
                transaction: Some(TransactionPart {
                    amount: TransactionAmount(own),
                    currency: *currency,
                    rate: *rate,
                    quote: Quote::HomePerUnit,
                    home_derived: false,
                }),
            },
        }
    }
}

/// One purchase-order row of a Scenario document.
pub struct PurchaseOrderBuilder<'a> {
    scenario: &'a mut Scenario,
    number: String,
    vendor: FactId,
    total: Decimal,
    delivery: Option<NaiveDate>,
}

impl PurchaseOrderBuilder<'_> {
    pub fn total(mut self, text: &str) -> Self {
        self.total = amount(text);
        self
    }

    /// The date the goods are expected, which AP-PO-01 times from.
    pub fn delivery(mut self, text: &str) -> Self {
        self.delivery = Some(self.scenario.date(text));
        self
    }

    pub fn add(self) {
        let s = self.scenario;
        let order = PurchaseOrder {
            id: s.id("PurchaseOrder", &self.number),
            provenance: s.provenance(),
            vendor: self.vendor,
            status: PurchaseOrderStatus::Authorised,
            delivery_date: self.delivery,
            total: Money::home(self.total),
        };
        let result = s.facts.add_purchase_order(order);
        s.accept(result);
    }
}

/// One invoice or bill row of a Scenario document.
pub struct DocumentBuilder<'a> {
    scenario: &'a mut Scenario,
    side: Side,
    number: String,
    counterparty: FactId,
    /// The account a `paid` settlement comes from.
    paid_from: &'static str,
    reference: Option<String>,
    dated: Option<NaiveDate>,
    due: Option<NaiveDate>,
    total: Decimal,
    tax: Decimal,
    status: DocumentStatus,
    /// Document currency and the booked rate, home per unit.
    foreign: Option<(Currency, Decimal)>,
    terms: Option<FactId>,
    template: Option<FactId>,
    expected: Option<NaiveDate>,
    paid: Option<NaiveDate>,
    credited: Option<NaiveDate>,
}

impl DocumentBuilder<'_> {
    pub fn dated(mut self, text: &str) -> Self {
        self.dated = Some(self.scenario.date(text));
        self
    }

    pub fn due(mut self, text: &str) -> Self {
        self.due = Some(self.scenario.date(text));
        self
    }

    pub fn total(mut self, text: &str) -> Self {
        self.total = amount(text);
        self
    }

    pub fn tax(mut self, text: &str) -> Self {
        self.tax = amount(text);
        self
    }

    pub fn draft(mut self) -> Self {
        self.status = DocumentStatus::Draft;
        self
    }

    pub fn voided(mut self) -> Self {
        self.status = DocumentStatus::Voided;
        self
    }

    /// In another currency at the rate booked on the document, Home Currency per unit.
    pub fn currency(mut self, code: &str, rate: &str) -> Self {
        let currency = Currency::new(code)
            .unwrap_or_else(|| panic!("{}: not a currency: {code}", self.scenario.name));
        self.foreign = Some((currency, amount(rate)));
        self
    }

    /// Payment terms as the document writes them: `net 30`, `2% 10 days, net 30`.
    pub fn terms(mut self, text: &str) -> Self {
        self.terms = Some(self.scenario.terms(text));
        self
    }

    /// The vendor's own number for a bill, which AP-DUP-01 matches on.
    pub fn reference(mut self, text: &str) -> Self {
        self.reference = Some(text.to_owned());
        self
    }

    /// Linked to a template already added, as Xero links a generated invoice.
    pub fn template(mut self, name: &str) -> Self {
        self.template = Some(self.scenario.id("Template", name));
        self
    }

    /// An expected payment date a person entered.
    pub fn expected(mut self, text: &str) -> Self {
        self.expected = Some(self.scenario.date(text));
        self
    }

    /// A planned payment date the owner entered on a bill: the same field (`docs/facts.md`).
    pub fn planned(self, text: &str) -> Self {
        self.expected(text)
    }

    /// Paid in full, in one payment, on a date.
    pub fn paid(mut self, text: &str) -> Self {
        self.paid = Some(self.scenario.date(text));
        self
    }

    /// Paid in full by credit card on a date: closed in AP, the obligation now on the card (Q103).
    pub fn paid_by_card(mut self, text: &str) -> Self {
        self.paid_from = "Business Visa";
        self.paid(text)
    }

    /// Closed in full by a credit note on a date, with no payment.
    pub fn credited(mut self, text: &str) -> Self {
        self.credited = Some(self.scenario.date(text));
        self
    }

    /// Adds the invoice, and the payment or credit note that settled it.
    pub fn add(self) {
        let s = self.scenario;
        let due = self
            .due
            .unwrap_or_else(|| panic!("{}: {} has no due date", s.name, self.number));
        let dated = self.dated.unwrap_or(due - chrono::Days::new(30));
        let total = s.money(self.total, self.foreign.as_ref());
        let (object_type, control, direction) = match self.side {
            Side::Receivable => ("Invoice", "Accounts Receivable", Direction::In),
            Side::Payable => ("Bill", "Accounts Payable", Direction::Out),
        };
        let purpose = match self.side {
            Side::Receivable => PaymentPurpose::Customer {
                counterparty: self.counterparty.clone(),
            },
            Side::Payable => PaymentPurpose::Vendor {
                counterparty: self.counterparty.clone(),
            },
        };
        let id = s.id(object_type, &self.number);
        let document = Document {
            id: id.clone(),
            provenance: s.provenance(),
            side: self.side,
            kind: DocumentKind::Invoice,
            counterparty: self.counterparty.clone(),
            number: Some(self.number.clone()),
            vendor_reference: self.reference.clone(),
            date: dated,
            status: self.status,
            total: total.clone(),
            tax: s.money(self.tax, self.foreign.as_ref()),
            installments: vec![Installment {
                due_date: due,
                amount: total.clone(),
            }],
            terms: self.terms,
            expected_date: self.expected,
            template: self.template,
            control_account: s.account(control),
            ledger_open: total.clone(),
        };
        let result = s.facts.add_document(document);
        s.accept(result);

        if let Some(on) = self.paid {
            let payment_id = s.id("Payment", &format!("{} payment", self.number));
            let payment = Payment {
                id: payment_id.clone(),
                provenance: s.provenance(),
                direction,
                purpose,
                date: on,
                amount: total.clone(),
                account: s.account(self.paid_from),
                status: PostingStatus::Posted,
            };
            let result = s.facts.add_payment(payment);
            s.accept(result);
            let application = Application {
                id: s.id("Application", &format!("{} payment", self.number)),
                provenance: s.provenance(),
                from: ApplicationFrom::Payment(payment_id),
                to: id.clone(),
                date: on,
                amount: total.clone(),
            };
            let result = s.facts.add_application(application);
            s.accept(result);
        }

        if let Some(on) = self.credited {
            let credit_number = format!("CN for {}", self.number);
            let credit_id = s.id("CreditNote", &credit_number);
            let credit = Document {
                id: credit_id.clone(),
                provenance: s.provenance(),
                side: self.side,
                kind: DocumentKind::CreditNote,
                counterparty: self.counterparty.clone(),
                number: Some(credit_number),
                vendor_reference: None,
                date: on,
                status: DocumentStatus::Posted,
                total: total.clone(),
                tax: s.money(self.tax, self.foreign.as_ref()),
                installments: vec![Installment {
                    due_date: on,
                    amount: total.clone(),
                }],
                terms: None,
                expected_date: None,
                template: None,
                control_account: s.account(control),
                ledger_open: total.clone(),
            };
            let result = s.facts.add_document(credit);
            s.accept(result);
            let application = Application {
                id: s.id("Application", &format!("{} credit", self.number)),
                provenance: s.provenance(),
                from: ApplicationFrom::CreditNote(credit_id),
                to: id,
                date: on,
                amount: total,
            };
            let result = s.facts.add_application(application);
            s.accept(result);
        }
    }
}

/// One template row of a Scenario document.
pub struct TemplateBuilder<'a> {
    scenario: &'a mut Scenario,
    side: Side,
    name: String,
    counterparty: FactId,
    mode: TemplateMode,
    amount: Decimal,
    start: Option<NaiveDate>,
    due_after: u32,
}

impl TemplateBuilder<'_> {
    pub fn amount(mut self, text: &str) -> Self {
        self.amount = amount(text);
        self
    }

    /// The first scheduled invoice date.
    pub fn starting(mut self, text: &str) -> Self {
        self.start = Some(self.scenario.date(text));
        self
    }

    /// Due this many days after the invoice or bill date.
    pub fn due_after(mut self, days: u32) -> Self {
        self.due_after = days;
        self
    }

    /// Reminds rather than generates: not automatic.
    pub fn reminder(mut self) -> Self {
        self.mode = TemplateMode::Reminder;
        self
    }

    pub fn add(self) {
        let s = self.scenario;
        let start = self
            .start
            .unwrap_or_else(|| panic!("{}: {} has no starting date", s.name, self.name));
        let template = ScheduledTemplate {
            id: s.id("Template", &self.name),
            provenance: s.provenance(),
            side: self.side,
            counterparty: self.counterparty,
            amount: Money::home(self.amount),
            // ponytail: every Scenario template is monthly; other frequencies when one is not.
            frequency: Frequency {
                unit: FrequencyUnit::Month,
                interval: 1,
            },
            start,
            next: None,
            end: None,
            due_rule: DueRule::DaysAfterDate(self.due_after),
            mode: self.mode,
            active: true,
        };
        let result = s.facts.add_scheduled_template(template);
        s.accept(result);
    }
}
