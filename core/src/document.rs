//! Document arithmetic with no side in it: what a document is worth, when, and to whom (ADR-0022).

use chrono::{Datelike, NaiveDate};
use rust_decimal::{Decimal, RoundingStrategy};

use crate::facts::{ApplicationFrom, Document, DocumentKind, FactId, Facts};
use crate::forecast::ForeignAmount;
use crate::money::{HomeAmount, Quote};
use crate::settings::{Classifications, CounterpartyClass};

/// Days after the due date, negative before it.
pub fn age(as_of: NaiveDate, due: NaiveDate) -> i64 {
    (as_of - due).num_days()
}

/// The due date aging runs from: the first installment's (N=1 in QBO and Xero).
// ponytail: a document with several installments ages by the first, so the later ones read as
// current while one is already overdue; ADR-0014 makes aging per installment, owed to the first
// Scenario with two on either side.
pub fn due_date(document: &Document) -> Option<NaiveDate> {
    document.installments.first().map(|i| i.due_date)
}

/// Applications to a document dated on or before a date, in the document's currency.
pub(crate) fn applied_own(facts: &Facts, document: &FactId, as_of: NaiveDate) -> Decimal {
    facts
        .applications()
        .values()
        .filter(|a| a.to == *document && a.date <= as_of)
        .map(|a| a.amount.in_own_currency().1)
        .fold(Decimal::ZERO, Decimal::saturating_add)
}

/// The open amount as of a date in the document's own currency (Q229).
pub fn open_own(facts: &Facts, document: &Document, as_of: NaiveDate) -> Decimal {
    document
        .total
        .in_own_currency()
        .1
        .saturating_sub(applied_own(facts, &document.id, as_of))
}

/// An own-currency amount converted at the rate booked on the document, rounded once to the
/// minor unit, half away from zero (Q182, Q243). Home Currency amounts pass through.
pub fn to_home(document: &Document, own: Decimal) -> HomeAmount {
    let Some(part) = &document.total.transaction else {
        return HomeAmount(own);
    };
    let converted = match part.quote {
        Quote::HomePerUnit => own.checked_mul(part.rate),
        Quote::UnitsPerHome => own.checked_div(part.rate),
    };
    // ponytail: minor unit fixed at 2; Reference Data when a Scenario needs another (Q182).
    HomeAmount(
        converted
            .unwrap_or(document.total.home.0)
            .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero),
    )
}

/// The open amount as of a date in Home Currency (AR-OPEN-01).
pub fn open_home(facts: &Facts, document: &Document, as_of: NaiveDate) -> HomeAmount {
    to_home(document, open_own(facts, document, as_of))
}

/// Applications from a payment or credit note dated on or before a date, in Home Currency.
pub(crate) fn applied_from(facts: &Facts, credit: &FactId, as_of: NaiveDate) -> Decimal {
    facts
        .applications()
        .values()
        .filter(|a| a.date <= as_of)
        .filter(|a| match &a.from {
            ApplicationFrom::Payment(p) | ApplicationFrom::CreditNote(p) => p == credit,
        })
        .map(|a| a.amount.home.0)
        .fold(Decimal::ZERO, Decimal::saturating_add)
}

/// The unapplied remainder of a customer payment or credit note as of a date, in Home Currency
/// (Q251); `None` when the id names neither.
// ponytail: home amounts throughout; a foreign-currency credit gets Q243's treatment when a
// Scenario has one.
pub fn unapplied(facts: &Facts, credit: &FactId, as_of: NaiveDate) -> Option<HomeAmount> {
    let total = facts
        .payments()
        .get(credit)
        .map(|p| p.amount.home.0)
        .or_else(|| {
            facts
                .documents()
                .get(credit)
                .filter(|d| d.kind == DocumentKind::CreditNote)
                .map(|d| d.total.home.0)
        })?;
    Some(HomeAmount(
        total.saturating_sub(applied_from(facts, credit, as_of)),
    ))
}

pub fn foreign(document: &Document, open_own: Decimal) -> Option<ForeignAmount> {
    document.total.transaction.as_ref().map(|t| ForeignAmount {
        currency: t.currency,
        amount: open_own,
        rate: t.rate,
    })
}

/// The top-level parent a Rule groups a customer by (Q245).
pub fn top_level(facts: &Facts, counterparty: &FactId) -> FactId {
    let mut id = counterparty.clone();
    let mut depth = 0;
    while let Some(parent) = facts
        .counterparties()
        .get(&id)
        .and_then(|c| c.parent.clone())
    {
        depth += 1;
        if depth > 32 {
            break;
        }
        id = parent;
    }
    id
}

pub fn intercompany(
    facts: &Facts,
    classifications: &Classifications,
    counterparty: &FactId,
) -> bool {
    let mut id = counterparty.clone();
    let mut depth = 0;
    loop {
        if let CounterpartyClass::Intercompany(_) = classifications.counterparty(&id).class {
            return true;
        }
        match facts
            .counterparties()
            .get(&id)
            .and_then(|c| c.parent.clone())
        {
            Some(parent) if depth < 32 => {
                depth += 1;
                id = parent;
            }
            _ => return false,
        }
    }
}

/// The last completed month before a date: its first day and its last.
pub fn last_completed_month(as_of: NaiveDate) -> Option<(NaiveDate, NaiveDate)> {
    let end = as_of.with_day(1)?.pred_opt()?;
    Some((end.with_day(1)?, end))
}

/// An account's balance fact dated on a day, in Home Currency (ADR-0014, Q72).
pub fn balance_at(facts: &Facts, account: &FactId, on: NaiveDate) -> Option<HomeAmount> {
    facts
        .account_balances()
        .values()
        .find(|b| b.account == *account && b.as_of == on)
        .map(|b| b.amount.home)
}
