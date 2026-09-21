//! The GAP Family: obligations that are not open bills. Two Rules arrive with CASH (Q275),
//! written ahead of the Family: GAP-CARD-01 and GAP-SCHED-01. `core/src/forecast.rs::run_forecast`
//! calls `gap::run` in the same per-Entity loop AR and AP run in, so their Placements are in the
//! run before CASH-ROLL-01 reads them. Nothing else here is built.

use chrono::{Datelike, Months, NaiveDate};

use crate::document::balance_at;
use crate::facts::{Direction, EntityId, Facts};
use crate::forecast::{
    Basis, Confidence, DecisionItem, Exclusion, ForecastRun, ItemKind, Outcome, Placement,
    ProvisionalReason, Role, RuleId, Severity, Subject,
};
use crate::money::HomeAmount;
use crate::schedule::occurrence;
use crate::settings::{AccountClass, Classifications, Settings};

pub(crate) const GAP_CARD_01: RuleId = RuleId("GAP-CARD-01");
pub(crate) const GAP_SCHED_01: RuleId = RuleId("GAP-SCHED-01");

/// The next date on or after the run date with this day of the month (Q121: the run-date balance,
/// paid on the card's payment day).
// ponytail: clamps to 28 so every month has the day; a calendar-aware "last valid day" belongs to
// Reference Data (ADR-0007) when a Scenario needs a day past it.
fn next_payment_day(as_of: NaiveDate, day: u32) -> NaiveDate {
    let day = day.clamp(1, 28);
    NaiveDate::from_ymd_opt(as_of.year(), as_of.month(), day)
        .filter(|on| *on >= as_of)
        .or_else(|| {
            as_of
                .with_day(1)?
                .checked_add_months(Months::new(1))?
                .with_day(day)
        })
        .unwrap_or(as_of)
}

/// GAP-CARD-01: each credit card's run-date balance is placed as one payment on the card's next
/// payment day. A card with no payment day set is excluded, and the run is Provisional.
fn cards(
    run: &mut ForecastRun,
    facts: &Facts,
    classifications: &Classifications,
    settings: &Settings,
    entity: &EntityId,
) {
    let as_of = run.as_of;
    for account in classifications.accounts_classed(entity, &AccountClass::CreditCard) {
        let Some(balance) = balance_at(facts, account, as_of) else {
            continue;
        };
        if balance.0.is_zero() {
            continue;
        }
        let amount = Some(HomeAmount(balance.0.abs()));
        match settings.card_payment_days.get(account) {
            Some(&day) => {
                let on = next_payment_day(as_of, day);
                run.placements.push(Placement {
                    subject: Subject::Fact(account.clone()),
                    rule: GAP_CARD_01,
                    rules: vec![GAP_CARD_01],
                    outcome: Outcome::placed_on(
                        run.horizon,
                        as_of,
                        on,
                        Direction::Out,
                        Basis::CardBalance,
                        Confidence::Firm,
                    ),
                    amount,
                    history: None,
                    foreign: None,
                    reductions: Vec::new(),
                    priority: None,
                });
            }
            None => {
                let subject = Subject::Fact(account.clone());
                run.placements.push(Placement {
                    subject: subject.clone(),
                    rule: GAP_CARD_01,
                    rules: vec![GAP_CARD_01],
                    outcome: Outcome::Excluded(Exclusion::NoCardPaymentDay),
                    amount,
                    history: None,
                    foreign: None,
                    reductions: Vec::new(),
                    priority: None,
                });
                run.decision_items.push(DecisionItem {
                    kind: ItemKind::SetCardPaymentDay,
                    subject: subject.clone(),
                    rule: GAP_CARD_01,
                    acted_on_by: Role::Owner,
                    severity: Severity::Action,
                    evidence: None,
                    draft: None,
                    due: None,
                    priority: None,
                });
                run.provisional.push(ProvisionalReason {
                    rule: GAP_CARD_01,
                    subject,
                });
            }
        }
    }
}

/// GAP-SCHED-01: one payment per occurrence of an owner-entered scheduled obligation that falls
/// inside the Horizon. GAP-SCHED-02's coverage test is not built; every occurrence is placed.
fn scheduled_obligations(run: &mut ForecastRun, settings: &Settings, entity: &EntityId) {
    let as_of = run.as_of;
    let end = run.horizon.end(as_of);
    for (id, obligation) in settings
        .scheduled_obligations
        .iter()
        .filter(|(id, _)| id.entity == *entity)
    {
        let within = |on: NaiveDate| obligation.end.is_none_or(|last| on <= last);
        let mut k = 0;
        while let Some(on) = occurrence(obligation.start, obligation.frequency, k) {
            k = k.saturating_add(1);
            if on >= end || !within(on) {
                break;
            }
            if on < as_of {
                continue;
            }
            run.placements.push(Placement {
                subject: Subject::Occurrence {
                    template: id.clone(),
                    date: on,
                },
                rule: GAP_SCHED_01,
                rules: vec![GAP_SCHED_01],
                outcome: Outcome::placed_on(
                    run.horizon,
                    as_of,
                    on,
                    Direction::Out,
                    Basis::ScheduledObligation,
                    Confidence::Firm,
                ),
                amount: Some(obligation.amount),
                history: None,
                foreign: None,
                reductions: Vec::new(),
                priority: None,
            });
        }
    }
}

/// Runs every GAP Rule built for one Entity.
pub fn run(
    run: &mut ForecastRun,
    facts: &Facts,
    settings: &Settings,
    classifications: &Classifications,
    entity: &EntityId,
) {
    cards(run, facts, classifications, settings, entity);
    scheduled_obligations(run, settings, entity);
}
