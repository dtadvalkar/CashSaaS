//! The GAP Family: obligations that are not open bills. GAP-CARD-01 and GAP-SCHED-01 arrived with
//! CASH (Q275); GAP-SCHED-02, GAP-LOAN-01, GAP-PAYROLL-01–04, GAP-TAX-01–04, GAP-INCOME-01 and
//! GAP-ACCRUAL-01 land with the Family's own Scenarios. `core/src/forecast.rs::run_forecast` calls
//! `gap::run` in the same per-Entity loop AR and AP run in, so their Placements are in the run
//! before CASH-ROLL-01 reads them.

use std::collections::BTreeMap;

use chrono::{Datelike, Days, Months, NaiveDate};

use crate::document::{balance_at, last_completed_month, top_level};
use crate::facts::{
    Direction, DocumentKind, DocumentStatus, EntityId, FactId, Facts, Frequency, PaymentPurpose,
    PostingStatus, SalesTaxBasis, Side,
};
use crate::forecast::{
    Basis, Confidence, DecisionItem, Evidence, Exclusion, ForecastRun, ItemKind, Outcome,
    Placement, Priority, ProvisionalReason, Role, RuleId, Severity, Subject,
};
use crate::money::HomeAmount;
use crate::reference::{
    GstHstPeriod, InstalmentCadence, Jurisdiction, RemitterType, corporate_balance_due_raw,
    corporate_instalment_raw, has_calendar, move_due_date, payroll_remittance_due,
    payroll_remittance_due_raw, payroll_remittance_period, sales_tax_due_date, sales_tax_period,
};
use crate::schedule::{index_at_or_before, nearest_occurrence, occurrence};
use crate::settings::{
    AccountClass, AccrualSettlement, Classifications, CounterpartyClass, PayrollSchedule,
    SalesTaxReportingPeriod, ScheduledObligation, Settings, TaxAccountingScheme,
};

pub(crate) const GAP_CARD_01: RuleId = RuleId("GAP-CARD-01");
pub(crate) const GAP_SCHED_01: RuleId = RuleId("GAP-SCHED-01");
pub(crate) const GAP_SCHED_02: RuleId = RuleId("GAP-SCHED-02");
pub(crate) const GAP_LOAN_01: RuleId = RuleId("GAP-LOAN-01");
pub(crate) const GAP_PAYROLL_01: RuleId = RuleId("GAP-PAYROLL-01");
pub(crate) const GAP_PAYROLL_02: RuleId = RuleId("GAP-PAYROLL-02");
pub(crate) const GAP_PAYROLL_03: RuleId = RuleId("GAP-PAYROLL-03");
pub(crate) const GAP_PAYROLL_04: RuleId = RuleId("GAP-PAYROLL-04");
pub(crate) const GAP_TAX_01: RuleId = RuleId("GAP-TAX-01");
pub(crate) const GAP_TAX_02: RuleId = RuleId("GAP-TAX-02");
pub(crate) const GAP_TAX_03: RuleId = RuleId("GAP-TAX-03");
pub(crate) const GAP_TAX_04: RuleId = RuleId("GAP-TAX-04");
pub(crate) const GAP_INCOME_01: RuleId = RuleId("GAP-INCOME-01");
pub(crate) const GAP_ACCRUAL_01: RuleId = RuleId("GAP-ACCRUAL-01");

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

/// GAP-SCHED-02 (Q118, Q149): bills, card charges and bank spends that cover an occurrence of an
/// owner-entered schedule. Same nearest-occurrence test AP-SCHED-01 and CASH-SCHED-01 use; a spend
/// without a payee covers nothing.
fn covered(
    facts: &Facts,
    classifications: &Classifications,
    entity: &EntityId,
    obligation: &ScheduledObligation,
) -> BTreeMap<NaiveDate, FactId> {
    let payee = top_level(facts, &obligation.payee);
    let bills = facts
        .documents()
        .values()
        .filter(|d| d.id.entity == *entity && d.side == Side::Payable)
        .filter(|d| d.kind == DocumentKind::Invoice)
        .filter(|d| !matches!(d.status, DocumentStatus::Voided | DocumentStatus::Deleted))
        .filter(|d| top_level(facts, &d.counterparty) == payee)
        .map(|d| (&d.id, d.date));
    let spends = facts
        .account_lines()
        .values()
        .filter(|l| l.id.entity == *entity && l.status == PostingStatus::Posted)
        .filter(|l| {
            matches!(
                classifications.account(&l.account).class,
                AccountClass::Bank | AccountClass::CreditCard
            )
        })
        .filter(|l| {
            l.counterparty
                .as_ref()
                .is_some_and(|c| top_level(facts, c) == payee)
        })
        .map(|l| (&l.id, l.date));
    let mut map = BTreeMap::new();
    for (id, date) in bills.chain(spends) {
        if let Some(on) = nearest_occurrence(obligation.start, obligation.frequency, date) {
            map.entry(on).or_insert_with(|| id.clone());
        }
    }
    map
}

/// GAP-SCHED-01 and GAP-SCHED-02: one payment per occurrence inside the Horizon, or an exclusion
/// citing what already covers it.
fn scheduled_obligations(
    run: &mut ForecastRun,
    facts: &Facts,
    settings: &Settings,
    classifications: &Classifications,
    entity: &EntityId,
) {
    let as_of = run.as_of;
    let end = run.horizon.end(as_of);
    for (id, obligation) in settings
        .scheduled_obligations
        .iter()
        .filter(|(id, _)| id.entity == *entity)
    {
        let covered = covered(facts, classifications, entity, obligation);
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
            let subject = Subject::Occurrence {
                entity: id.entity.clone(),
                template: id.clone(),
                date: on,
            };
            match covered.get(&on) {
                Some(fact) => {
                    run.placements.push(Placement {
                        subject,
                        rule: GAP_SCHED_02,
                        rules: vec![GAP_SCHED_02],
                        outcome: Outcome::Excluded(Exclusion::CoveredBy(fact.clone())),
                        amount: Some(obligation.amount),
                        history: None,
                        foreign: None,
                        reductions: Vec::new(),
                        priority: None,
                    });
                }
                None => {
                    run.placements.push(Placement {
                        subject,
                        rule: GAP_SCHED_01,
                        // SCHED-02 produces exclusions only; uncovered occurrences cite SCHED-01
                        // alone (same as GAP-PAYROLL-01's uncovered net-pay rows).
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
    }
}

/// GAP-LOAN-01: a loan or lease liability with a balance and no covering schedule is excluded,
/// raises an item, and makes the run Provisional.
fn loans(
    run: &mut ForecastRun,
    facts: &Facts,
    classifications: &Classifications,
    settings: &Settings,
    entity: &EntityId,
) {
    let as_of = run.as_of;
    let classes = [AccountClass::Loan, AccountClass::LeaseLiability];
    for class in classes {
        for account in classifications.accounts_classed(entity, &class) {
            let Some(balance) = balance_at(facts, account, as_of) else {
                continue;
            };
            if balance.0.is_zero() {
                continue;
            }
            let covered = settings
                .scheduled_obligations
                .values()
                .any(|o| o.covers_account.as_ref() == Some(account));
            if covered {
                continue;
            }
            let subject = Subject::Fact(account.clone());
            let amount = Some(HomeAmount(balance.0.abs()));
            run.placements.push(Placement {
                subject: subject.clone(),
                rule: GAP_LOAN_01,
                rules: vec![GAP_LOAN_01],
                outcome: Outcome::Excluded(Exclusion::NoSchedule),
                amount,
                history: None,
                foreign: None,
                reductions: Vec::new(),
                priority: None,
            });
            run.decision_items.push(DecisionItem {
                kind: ItemKind::EnterLoanSchedule,
                subject: subject.clone(),
                rule: GAP_LOAN_01,
                acted_on_by: Role::Owner,
                severity: Severity::Action,
                evidence: None,
                draft: None,
                due: None,
                priority: None,
            });
            run.provisional.push(ProvisionalReason {
                rule: GAP_LOAN_01,
                subject,
            });
        }
    }
}

/// Pay dates of a schedule that fall inside a remittance period, walking the cadence both ways
/// from `next_pay_date` so a mid-period start still counts earlier runs in that period.
fn pay_dates_in_period(
    start: NaiveDate,
    frequency: Frequency,
    period_start: NaiveDate,
    period_end: NaiveDate,
) -> Vec<NaiveDate> {
    let anchor = index_at_or_before(start, frequency, period_end).unwrap_or(0);
    let mut dates = Vec::new();
    for k in (anchor - 8)..=(anchor + 8) {
        if let Some(on) = occurrence(start, frequency, k)
            && on >= period_start
            && on <= period_end
        {
            dates.push(on);
        }
    }
    dates.sort_unstable();
    dates.dedup();
    dates
}

/// Bank or card spends to the payroll payee that cover a pay date (GAP-PAYROLL-01 / GAP-SCHED-02).
fn payroll_covered(
    facts: &Facts,
    classifications: &Classifications,
    entity: &EntityId,
    schedule: &PayrollSchedule,
) -> BTreeMap<NaiveDate, FactId> {
    let Some(payee) = schedule.paid_through.as_ref() else {
        return BTreeMap::new();
    };
    let payee = top_level(facts, payee);
    let mut map = BTreeMap::new();
    for line in facts.account_lines().values() {
        if line.id.entity != *entity || line.status != PostingStatus::Posted {
            continue;
        }
        if !matches!(
            classifications.account(&line.account).class,
            AccountClass::Bank | AccountClass::CreditCard
        ) {
            continue;
        }
        let Some(counterparty) = line.counterparty.as_ref() else {
            continue;
        };
        if top_level(facts, counterparty) != payee {
            continue;
        }
        if let Some(on) = nearest_occurrence(schedule.next_pay_date, schedule.frequency, line.date)
        {
            map.entry(on).or_insert_with(|| line.id.clone());
        }
    }
    map
}

fn is_government_trust(facts: &Facts, classifications: &Classifications, id: &FactId) -> bool {
    let top = top_level(facts, id);
    classifications.counterparty(id).class == CounterpartyClass::GovernmentTrust
        || classifications.counterparty(&top).class == CounterpartyClass::GovernmentTrust
}

/// Outflows to a government-trust counterparty (GAP-PAYROLL-03).
fn authority_remittances<'a>(
    facts: &'a Facts,
    classifications: &Classifications,
    entity: &EntityId,
) -> Vec<(&'a FactId, NaiveDate, HomeAmount)> {
    facts
        .account_lines()
        .values()
        .filter(|l| l.id.entity == *entity && l.status == PostingStatus::Posted)
        .filter(|l| l.amount.home.0.is_sign_negative() && !l.amount.home.0.is_zero())
        .filter(|l| {
            l.counterparty
                .as_ref()
                .is_some_and(|c| is_government_trust(facts, classifications, c))
        })
        .map(|l| (&l.id, l.date, HomeAmount(l.amount.home.0.abs())))
        .collect()
}

fn payroll_liability_at(
    facts: &Facts,
    classifications: &Classifications,
    entity: &EntityId,
    on: NaiveDate,
) -> HomeAmount {
    let mut total = rust_decimal::Decimal::ZERO;
    for account in classifications.accounts_classed(entity, &AccountClass::PayrollLiability) {
        if let Some(balance) = balance_at(facts, account, on) {
            total += balance.0.abs();
        }
    }
    HomeAmount(total)
}

/// GAP-PAYROLL-02: wages or payroll-liability activity without a schedule is never guessed.
fn missing_payroll_schedule(
    run: &mut ForecastRun,
    classifications: &Classifications,
    settings: &Settings,
    entity: &EntityId,
) {
    if settings.payroll_schedules.contains_key(entity) {
        return;
    }
    let runs_payroll = classifications
        .accounts_classed(entity, &AccountClass::PayrollLiability)
        .next()
        .is_some()
        || classifications
            .accounts_classed(entity, &AccountClass::Wages)
            .next()
            .is_some();
    if !runs_payroll {
        return;
    }
    let subject = Subject::Entity(entity.clone());
    run.decision_items.push(DecisionItem {
        kind: ItemKind::SetUpPayrollSchedule,
        subject: subject.clone(),
        rule: GAP_PAYROLL_02,
        acted_on_by: Role::Owner,
        severity: Severity::Action,
        evidence: None,
        draft: None,
        due: None,
        priority: None,
    });
    run.provisional.push(ProvisionalReason {
        rule: GAP_PAYROLL_02,
        subject,
    });
}

/// GAP-PAYROLL-01: net pay on each pay date, with GAP-SCHED-02's coverage test.
fn net_pay(
    run: &mut ForecastRun,
    facts: &Facts,
    classifications: &Classifications,
    schedule: &PayrollSchedule,
    entity: &EntityId,
) {
    let as_of = run.as_of;
    let end = run.horizon.end(as_of);
    let covered = payroll_covered(facts, classifications, entity, schedule);
    let mut k = 0i64;
    while let Some(on) = occurrence(schedule.next_pay_date, schedule.frequency, k) {
        k = k.saturating_add(1);
        if on >= end {
            break;
        }
        if on < as_of {
            continue;
        }
        let subject = Subject::PayrollPay {
            entity: entity.clone(),
            date: on,
        };
        match covered.get(&on) {
            Some(fact) => {
                run.placements.push(Placement {
                    subject,
                    rule: GAP_PAYROLL_01,
                    rules: vec![GAP_PAYROLL_01, GAP_SCHED_02],
                    outcome: Outcome::Excluded(Exclusion::CoveredBy(fact.clone())),
                    amount: Some(schedule.expected_net_pay),
                    history: None,
                    foreign: None,
                    reductions: Vec::new(),
                    priority: None,
                });
            }
            None => {
                run.placements.push(Placement {
                    subject,
                    rule: GAP_PAYROLL_01,
                    rules: vec![GAP_PAYROLL_01],
                    outcome: Outcome::placed_on(
                        run.horizon,
                        as_of,
                        on,
                        Direction::Out,
                        Basis::PayrollSchedule,
                        Confidence::Firm,
                    ),
                    amount: Some(schedule.expected_net_pay),
                    history: None,
                    foreign: None,
                    reductions: Vec::new(),
                    priority: None,
                });
            }
        }
    }
}

/// GAP-PAYROLL-04: schedule without remitter type excludes remittances.
fn missing_remitter(run: &mut ForecastRun, entity: &EntityId) {
    let subject = Subject::PayrollRemittances {
        entity: entity.clone(),
    };
    run.placements.push(Placement {
        subject: subject.clone(),
        rule: GAP_PAYROLL_04,
        rules: vec![GAP_PAYROLL_04],
        outcome: Outcome::Excluded(Exclusion::NoRemitterType),
        amount: None,
        history: None,
        foreign: None,
        reductions: Vec::new(),
        priority: None,
    });
    run.decision_items.push(DecisionItem {
        kind: ItemKind::SetRemitterType,
        subject: Subject::Entity(entity.clone()),
        rule: GAP_PAYROLL_04,
        acted_on_by: Role::Accountant,
        severity: Severity::Action,
        evidence: None,
        draft: None,
        due: None,
        priority: None,
    });
    run.provisional.push(ProvisionalReason {
        rule: GAP_PAYROLL_04,
        subject: Subject::Entity(entity.clone()),
    });
}

/// GAP-PAYROLL-03: remittances on the statutory calendar for the remitter type. Period start and
/// end come from Reference Data (Q227), so Accelerated threshold 1 can end on the 15th.
fn remittances(
    run: &mut ForecastRun,
    facts: &Facts,
    classifications: &Classifications,
    schedule: &PayrollSchedule,
    remitter: RemitterType,
    entity: &EntityId,
) {
    let as_of = run.as_of;
    let horizon_end = run.horizon.end(as_of);
    let paid = authority_remittances(facts, classifications, entity);
    let last_actual = paid
        .iter()
        .filter(|(_, on, _)| *on <= as_of)
        .max_by_key(|(_, on, _)| *on)
        .map(|(_, _, amount)| *amount);

    // Walk every remittance period that can still fall due inside the horizon, starting two
    // calendar months before as_of so a booked prior period is not missed.
    let mut cursor = as_of
        .with_day(1)
        .and_then(|d| d.checked_sub_months(Months::new(2)))
        .unwrap_or(as_of);
    let mut unknown: Vec<(NaiveDate, NaiveDate)> = Vec::new();

    while cursor < horizon_end {
        let Some((period_start, period_end)) = payroll_remittance_period(remitter, cursor) else {
            break;
        };
        let Some(due) = payroll_remittance_due(remitter, period_end) else {
            break;
        };
        let Some(raw_due) = payroll_remittance_due_raw(remitter, period_end) else {
            break;
        };
        let next = match period_end.succ_opt() {
            Some(d) => d,
            None => break,
        };
        if run.horizon.week_of(as_of, due).is_none() {
            cursor = next;
            continue;
        }

        let ended = period_end < as_of;
        let amount_basis = if ended {
            let liability = payroll_liability_at(facts, classifications, entity, period_end);
            let remitted: rust_decimal::Decimal = paid
                .iter()
                .filter(|(_, on, _)| *on > period_end && *on <= as_of)
                .map(|(_, _, a)| a.0)
                .sum();
            let owed = liability.0 - remitted;
            if owed.is_sign_positive() && !owed.is_zero() {
                Some((
                    HomeAmount(owed),
                    Basis::BookedLiability,
                    Confidence::Firm,
                    Vec::new(),
                ))
            } else {
                None
            }
        } else if let Some(per_run) = schedule.expected_remittance {
            let runs = pay_dates_in_period(
                schedule.next_pay_date,
                schedule.frequency,
                period_start,
                period_end,
            );
            let n = rust_decimal::Decimal::from(runs.len());
            let total = per_run.0 * n;
            if total.is_zero() {
                None
            } else {
                Some((
                    HomeAmount(total),
                    Basis::PayrollSchedule,
                    Confidence::Firm,
                    runs,
                ))
            }
        } else if let Some(last) = last_actual {
            Some((
                last,
                Basis::LastRemittance,
                Confidence::Estimated,
                Vec::new(),
            ))
        } else {
            unknown.push((period_start, due));
            None
        };

        if let Some((amount, basis, confidence, runs)) = amount_basis {
            let subject = Subject::PayrollRemittance {
                entity: entity.clone(),
                pay_month: period_start,
                period_end,
                raw_due,
                due,
                runs,
            };
            run.placements.push(Placement {
                subject,
                rule: GAP_PAYROLL_03,
                rules: vec![GAP_PAYROLL_03],
                outcome: Outcome::placed_on(
                    run.horizon,
                    as_of,
                    due,
                    Direction::Out,
                    basis,
                    confidence,
                ),
                amount: Some(amount),
                history: None,
                foreign: None,
                reductions: Vec::new(),
                priority: Some(Priority::GovernmentTrust),
            });
        }

        cursor = next;
    }

    if !unknown.is_empty() {
        let subject = Subject::PayrollRemittancesUnknown {
            entity: entity.clone(),
            remittances: unknown.clone(),
        };
        run.placements.push(Placement {
            subject: subject.clone(),
            rule: GAP_PAYROLL_03,
            rules: vec![GAP_PAYROLL_03],
            outcome: Outcome::Excluded(Exclusion::RemittanceAmountUnknown),
            amount: None,
            history: None,
            foreign: None,
            reductions: Vec::new(),
            priority: None,
        });
        run.decision_items.push(DecisionItem {
            kind: ItemKind::PayrollRemittanceAmountUnknown,
            subject: subject.clone(),
            rule: GAP_PAYROLL_03,
            acted_on_by: Role::Owner,
            severity: Severity::Action,
            evidence: None,
            draft: None,
            due: None,
            priority: None,
        });
        run.provisional.push(ProvisionalReason {
            rule: GAP_PAYROLL_03,
            subject,
        });
    }
}

/// GAP-PAYROLL-01 through 04 for one Entity.
fn payroll(
    run: &mut ForecastRun,
    facts: &Facts,
    classifications: &Classifications,
    settings: &Settings,
    entity: &EntityId,
) {
    missing_payroll_schedule(run, classifications, settings, entity);
    let Some(schedule) = settings.payroll_schedules.get(entity) else {
        return;
    };
    net_pay(run, facts, classifications, schedule, entity);
    match settings.remitter_types.get(entity).copied() {
        None => missing_remitter(run, entity),
        Some(remitter) => {
            remittances(run, facts, classifications, schedule, remitter, entity);
        }
    }
}

/// The sales-tax liability accounts of one Entity, grouped by the jurisdiction whose calendar
/// times them. An account with no jurisdiction Setting is Canada federal, which is what the
/// Entities of a Canadian Group are (GAP-TAX-04, Q122).
fn sales_tax_accounts(
    classifications: &Classifications,
    settings: &Settings,
    entity: &EntityId,
) -> BTreeMap<Jurisdiction, Vec<FactId>> {
    let mut groups: BTreeMap<Jurisdiction, Vec<FactId>> = BTreeMap::new();
    for account in classifications.accounts_classed(entity, &AccountClass::SalesTaxLiability) {
        let jurisdiction = settings
            .tax_jurisdictions
            .get(account)
            .copied()
            .unwrap_or(Jurisdiction::CanadaFederal);
        groups
            .entry(jurisdiction)
            .or_default()
            .push(account.clone());
    }
    groups
}

/// The most recent balance the Entity has stated for an account on or before a date (ADR-0014):
/// what a sales-tax account holds when no reporting period says which date to read it at.
fn stated_balance(facts: &Facts, account: &FactId, on: NaiveDate) -> Option<HomeAmount> {
    facts
        .account_balances()
        .values()
        .filter(|balance| balance.account == *account && balance.as_of <= on)
        .max_by_key(|balance| balance.as_of)
        .map(|balance| balance.amount.home)
}

/// Payments the ledger marks as tax remittances, whatever their payee (Q224): the only way a Rule
/// finds money already sent to the authority, since a QBO `TaxPayment` carries no payee at all.
fn tax_remittances(facts: &Facts, entity: &EntityId) -> Vec<(NaiveDate, HomeAmount)> {
    facts
        .payments()
        .values()
        .filter(|payment| payment.id.entity == *entity && payment.status == PostingStatus::Posted)
        .filter(|payment| payment.direction == Direction::Out)
        .filter(|payment| matches!(payment.purpose, PaymentPurpose::TaxAuthority { .. }))
        .map(|payment| (payment.date, HomeAmount(payment.amount.home.0.abs())))
        .collect()
}

/// Xero's `SalesTaxPeriod` values M1 reads (`docs/facts.md`, Q253). Anything else names no period a
/// remittance can be timed from, which is GAP-TAX-03's "with neither".
fn ledger_period(value: &str) -> Option<SalesTaxReportingPeriod> {
    match value {
        "MONTHLY" => Some(SalesTaxReportingPeriod::Monthly),
        "3MONTHLY" => Some(SalesTaxReportingPeriod::Quarterly),
        "ANNUALLY" => Some(SalesTaxReportingPeriod::Annual),
        _ => None,
    }
}

/// Which reporting period applies, and whether it is the owner's own (GAP-TAX-03, Q113, Q253): the
/// Setting first, then the value the ledger holds, which is used but not trusted.
fn reporting_period(
    facts: &Facts,
    settings: &Settings,
    entity: &EntityId,
) -> Option<(SalesTaxReportingPeriod, bool)> {
    if let Some(period) = settings.sales_tax_periods.get(entity).copied() {
        return Some((period, true));
    }
    let held = facts
        .ledger_settings()
        .get(entity)
        .and_then(|ledger| ledger.sales_tax_period.as_deref())
        .and_then(ledger_period)?;
    Some((held, false))
}

/// The length of the period, as Reference Data counts due dates from it.
fn period_length(period: SalesTaxReportingPeriod) -> GstHstPeriod {
    match period {
        SalesTaxReportingPeriod::Monthly => GstHstPeriod::Monthly,
        SalesTaxReportingPeriod::Quarterly => GstHstPeriod::Quarterly,
        SalesTaxReportingPeriod::Annual => GstHstPeriod::Annual,
    }
}

/// Whether the booked balance is the return, and whether the scheme behind that is the owner's own
/// (GAP-TAX-01, Q226, Q253, Q284). Standard Setting: the booked balance, confirmed. Any other
/// Setting: estimated as GAP-TAX-02 estimates it. Unset with no ledger basis: standard firm, no
/// confirm item (Q284 Pick A). Unset with a ledger-held basis: that basis, unconfirmed (Q253).
fn accounting_scheme(facts: &Facts, settings: &Settings, entity: &EntityId) -> (bool, bool) {
    match settings.tax_accounting_schemes.get(entity).copied() {
        Some(TaxAccountingScheme::Standard) => (true, true),
        Some(
            TaxAccountingScheme::Cash | TaxAccountingScheme::FlatRate | TaxAccountingScheme::Other,
        ) => (false, true),
        None => match facts
            .ledger_settings()
            .get(entity)
            .and_then(|ledger| ledger.sales_tax_basis)
        {
            Some(SalesTaxBasis::Cash | SalesTaxBasis::FlatRate) => (false, false),
            Some(SalesTaxBasis::Standard) => (true, false),
            Some(SalesTaxBasis::None) | None => (true, true),
        },
    }
}

/// One Entity's sales tax in one jurisdiction: the accounts it is read from, the period that times
/// it, and what the Entity's scheme says the amount is.
struct SalesTax<'a> {
    entity: &'a EntityId,
    jurisdiction: Jurisdiction,
    accounts: Vec<FactId>,
    period: SalesTaxReportingPeriod,
    /// False for a period taken from the ledger and not yet confirmed (GAP-TAX-03).
    period_confirmed: bool,
    /// Whether the booked balance is the return (GAP-TAX-01) rather than an estimate (GAP-TAX-02).
    booked: bool,
    /// False when a ledger-held scheme is used without an owner Setting (Q253); blank unset is
    /// confirmed standard under Q284 Pick A.
    scheme_confirmed: bool,
}

impl SalesTax<'_> {
    /// Every Rule that shaped a Placement, the deciding one first: a period taken from the ledger
    /// is GAP-TAX-03's doing as much as the amount is GAP-TAX-01's (Q253).
    fn rules(&self, deciding: RuleId) -> Vec<RuleId> {
        let mut rules = vec![deciding];
        if !self.period_confirmed {
            rules.push(GAP_TAX_03);
        }
        rules
    }

    /// A booked remittance is firm only where both the period and the scheme are the owner's own;
    /// either one assumed makes the figure estimated (Q226, Q253).
    fn booked_confidence(&self) -> Confidence {
        if self.period_confirmed && self.scheme_confirmed {
            Confidence::Firm
        } else {
            Confidence::Estimated
        }
    }
}

/// One reporting period and the dates its remittance falls due on, before and after the calendar
/// moves it (Q227).
struct Remittance {
    period: (NaiveDate, NaiveDate),
    raw_due: NaiveDate,
    due: NaiveDate,
}

/// The sales-tax liability at a period end less remittances paid since (GAP-TAX-01): the sum over
/// every account of the jurisdiction, a tax suspense account holding a filed return included
/// (Q225). Negative where the accounts are in debit, which is a refund position.
fn booked_balance(
    facts: &Facts,
    tax: &SalesTax,
    end: NaiveDate,
    as_of: NaiveDate,
) -> rust_decimal::Decimal {
    let booked: rust_decimal::Decimal = tax
        .accounts
        .iter()
        .filter_map(|account| balance_at(facts, account, end))
        .map(|balance| balance.0)
        .sum();
    let paid_since: rust_decimal::Decimal = tax_remittances(facts, tax.entity)
        .iter()
        .filter(|(on, _)| *on > end && *on <= as_of)
        .map(|(_, amount)| amount.0)
        .sum();
    booked - paid_since
}

/// GAP-TAX-01 and GAP-TAX-02 for one period: the booked balance for a period that has ended under
/// the standard scheme, the last actual remittance otherwise, and a refund position shown as an
/// exclusion rather than forecast as a receipt.
fn place_remittance(
    run: &mut ForecastRun,
    facts: &Facts,
    tax: &SalesTax,
    at: &Remittance,
    last_actual: Option<HomeAmount>,
) {
    let as_of = run.as_of;
    let subject = Subject::SalesTaxRemittance {
        entity: tax.entity.clone(),
        jurisdiction: tax.jurisdiction,
        period: at.period,
        raw_due: at.raw_due,
        due: at.due,
    };
    let ended = at.period.1 < as_of;
    if ended && tax.booked {
        let owed = booked_balance(facts, tax, at.period.1, as_of);
        if owed.is_zero() {
            return;
        }
        if owed.is_sign_negative() {
            run.placements.push(Placement {
                subject: subject.clone(),
                rule: GAP_TAX_01,
                rules: tax.rules(GAP_TAX_01),
                outcome: Outcome::Excluded(Exclusion::SalesTaxRefundPosition),
                amount: Some(HomeAmount(owed.abs())),
                history: None,
                foreign: None,
                reductions: Vec::new(),
                priority: None,
            });
            run.decision_items.push(DecisionItem {
                kind: ItemKind::SalesTaxRefundPosition,
                subject,
                rule: GAP_TAX_01,
                acted_on_by: Role::Accountant,
                severity: Severity::Action,
                evidence: None,
                draft: None,
                due: None,
                priority: None,
            });
            return;
        }
        run.placements.push(Placement {
            subject,
            rule: GAP_TAX_01,
            rules: tax.rules(GAP_TAX_01),
            outcome: Outcome::placed_on(
                run.horizon,
                as_of,
                at.due,
                Direction::Out,
                Basis::BookedTaxLiability,
                tax.booked_confidence(),
            ),
            amount: Some(HomeAmount(owed)),
            history: None,
            foreign: None,
            reductions: Vec::new(),
            priority: Some(Priority::GovernmentTrust),
        });
        return;
    }
    // A period still running, or an ended one whose scheme means the booked balance is not the
    // return (Q226): the Entity's own last remittance is the least arbitrary estimate available.
    match last_actual {
        Some(amount) => run.placements.push(Placement {
            subject,
            rule: GAP_TAX_02,
            rules: tax.rules(GAP_TAX_02),
            outcome: Outcome::placed_on(
                run.horizon,
                as_of,
                at.due,
                Direction::Out,
                Basis::LastRemittance,
                Confidence::Estimated,
            ),
            amount: Some(amount),
            history: None,
            foreign: None,
            reductions: Vec::new(),
            priority: Some(Priority::GovernmentTrust),
        }),
        None => {
            run.placements.push(Placement {
                subject: subject.clone(),
                rule: GAP_TAX_02,
                rules: tax.rules(GAP_TAX_02),
                outcome: Outcome::Excluded(Exclusion::SalesTaxEstimateUnavailable),
                amount: None,
                history: None,
                foreign: None,
                reductions: Vec::new(),
                priority: None,
            });
            run.decision_items.push(DecisionItem {
                kind: ItemKind::SalesTaxEstimateUnavailable,
                subject: subject.clone(),
                rule: GAP_TAX_02,
                acted_on_by: Role::Accountant,
                severity: Severity::Action,
                evidence: None,
                draft: None,
                due: None,
                priority: None,
            });
            run.provisional.push(ProvisionalReason {
                rule: GAP_TAX_02,
                subject,
            });
        }
    }
}

/// GAP-TAX-01 and GAP-TAX-02: one remittance per reporting period whose payment falls due inside
/// the Horizon. A remittance due after it is neither estimated nor excluded (Q159), and one that
/// fell due before the run date was already the previous run's business.
fn sales_tax_remittances(run: &mut ForecastRun, facts: &Facts, tax: &SalesTax) {
    let as_of = run.as_of;
    let length = period_length(tax.period);
    let paid = tax_remittances(facts, tax.entity);
    let last_actual = paid
        .iter()
        .filter(|(on, _)| *on <= as_of)
        .max_by_key(|(on, _)| *on)
        .map(|(_, amount)| *amount);
    let Some(current) = sales_tax_period(as_of, length) else {
        return;
    };
    // The period before the run date's own has ended and may still be unremitted.
    let mut window = current
        .0
        .pred_opt()
        .and_then(|before| sales_tax_period(before, length))
        .unwrap_or(current);
    while let Some(raw_due) = sales_tax_due_date(window.1, length) {
        let Some(due) = move_due_date(raw_due, tax.jurisdiction) else {
            break;
        };
        if run.horizon.week_of(as_of, due).is_none() {
            break;
        }
        if due >= as_of {
            place_remittance(
                run,
                facts,
                tax,
                &Remittance {
                    period: window,
                    raw_due,
                    due,
                },
                last_actual,
            );
        }
        let Some(next) = window
            .1
            .succ_opt()
            .and_then(|after| sales_tax_period(after, length))
        else {
            break;
        };
        window = next;
    }
}

/// The balances of a jurisdiction nothing can time, excluded one account at a time so the figure
/// left out of the forecast is named (ADR-0011).
fn exclude_sales_tax_accounts(
    run: &mut ForecastRun,
    facts: &Facts,
    accounts: &[FactId],
    jurisdiction: Jurisdiction,
    rule: RuleId,
    reason: &Exclusion,
) {
    let as_of = run.as_of;
    for account in accounts {
        run.placements.push(Placement {
            subject: Subject::SalesTaxAccount {
                account: account.clone(),
                jurisdiction,
            },
            rule,
            rules: vec![rule],
            outcome: Outcome::Excluded(reason.clone()),
            amount: stated_balance(facts, account, as_of)
                .map(|balance| HomeAmount(balance.0.abs())),
            history: None,
            foreign: None,
            reductions: Vec::new(),
            priority: None,
        });
    }
}

/// GAP-TAX-04: an obligation whose jurisdiction has no calendar in Reference Data is excluded and
/// named, never silently left out (ADR-0007), and the run is Provisional.
fn no_calendar(
    run: &mut ForecastRun,
    facts: &Facts,
    accounts: &[FactId],
    jurisdiction: Jurisdiction,
    entity: &EntityId,
) {
    exclude_sales_tax_accounts(
        run,
        facts,
        accounts,
        jurisdiction,
        GAP_TAX_04,
        &Exclusion::NoCalendar,
    );
    let subject = Subject::SalesTaxJurisdiction {
        entity: entity.clone(),
        jurisdiction,
    };
    run.decision_items.push(DecisionItem {
        kind: ItemKind::UnsupportedTaxJurisdiction,
        subject: subject.clone(),
        rule: GAP_TAX_04,
        acted_on_by: Role::Accountant,
        severity: Severity::Action,
        evidence: None,
        draft: None,
        due: None,
        priority: None,
    });
    run.provisional.push(ProvisionalReason {
        rule: GAP_TAX_04,
        subject,
    });
}

/// GAP-TAX-03: with no reporting period in the Settings and none in the ledger, the timing is not
/// guessed — the balances are excluded, the accountant is asked to set the period, and the run is
/// Provisional.
fn no_sales_tax_period(
    run: &mut ForecastRun,
    facts: &Facts,
    accounts: &[FactId],
    jurisdiction: Jurisdiction,
    entity: &EntityId,
) {
    exclude_sales_tax_accounts(
        run,
        facts,
        accounts,
        jurisdiction,
        GAP_TAX_03,
        &Exclusion::NoSalesTaxPeriod,
    );
    let subject = Subject::SalesTaxJurisdiction {
        entity: entity.clone(),
        jurisdiction,
    };
    run.decision_items.push(DecisionItem {
        kind: ItemKind::SetSalesTaxPeriod,
        subject: subject.clone(),
        rule: GAP_TAX_03,
        acted_on_by: Role::Accountant,
        severity: Severity::Action,
        evidence: None,
        draft: None,
        due: None,
        priority: None,
    });
    run.provisional.push(ProvisionalReason {
        rule: GAP_TAX_03,
        subject,
    });
}

/// GAP-TAX-01 through 04 for one Entity: one obligation per jurisdiction, each timed on that
/// jurisdiction's own calendar.
fn sales_tax(
    run: &mut ForecastRun,
    facts: &Facts,
    classifications: &Classifications,
    settings: &Settings,
    entity: &EntityId,
) {
    for (jurisdiction, accounts) in sales_tax_accounts(classifications, settings, entity) {
        if !has_calendar(jurisdiction) {
            no_calendar(run, facts, &accounts, jurisdiction, entity);
            continue;
        }
        let Some((period, period_confirmed)) = reporting_period(facts, settings, entity) else {
            no_sales_tax_period(run, facts, &accounts, jurisdiction, entity);
            continue;
        };
        let subject = Subject::SalesTaxJurisdiction {
            entity: entity.clone(),
            jurisdiction,
        };
        if !period_confirmed {
            run.decision_items.push(DecisionItem {
                kind: ItemKind::ConfirmSalesTaxPeriod,
                subject: subject.clone(),
                rule: GAP_TAX_03,
                acted_on_by: Role::Accountant,
                severity: Severity::Action,
                evidence: Some(Evidence::LedgerSalesTaxPeriod(period)),
                draft: None,
                due: None,
                priority: None,
            });
        }
        let (booked, scheme_confirmed) = accounting_scheme(facts, settings, entity);
        if !scheme_confirmed {
            // Q253: a ledger-held basis without an owner Setting is used but confirmed. Blank
            // unset is Q284 Pick A (firm, no item) and never reaches here.
            run.decision_items.push(DecisionItem {
                kind: ItemKind::ConfirmTaxAccountingScheme,
                subject,
                rule: GAP_TAX_01,
                acted_on_by: Role::Accountant,
                severity: Severity::Action,
                evidence: None,
                draft: None,
                due: None,
                priority: None,
            });
        }
        sales_tax_remittances(
            run,
            facts,
            &SalesTax {
                entity,
                jurisdiction,
                accounts,
                period,
                period_confirmed,
                booked,
                scheme_confirmed,
            },
        );
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
    loans(run, facts, classifications, settings, entity);
    scheduled_obligations(run, facts, settings, classifications, entity);
    payroll(run, facts, classifications, settings, entity);
    sales_tax(run, facts, classifications, settings, entity);
    income_tax(run, facts, classifications, settings, entity);
    accruals(run, facts, classifications, settings, entity);
}

/// GAP-INCOME-01: owner-entered instalments on statutory dates from the tax-year start, and a
/// booked income-tax payable balance for a completed year on its balance-due date (Q160).
fn income_tax(
    run: &mut ForecastRun,
    facts: &Facts,
    classifications: &Classifications,
    settings: &Settings,
    entity: &EntityId,
) {
    let as_of = run.as_of;
    let Some(tax) = settings.corporate_tax.get(entity) else {
        return;
    };
    if let Some(amount) = tax.monthly_instalment {
        for k in 0.. {
            let Some(raw_due) =
                corporate_instalment_raw(tax.tax_year_start, InstalmentCadence::Monthly, k)
            else {
                break;
            };
            let Some(due) = move_due_date(raw_due, Jurisdiction::CanadaFederal) else {
                break;
            };
            if run.horizon.week_of(as_of, due).is_none() {
                break;
            }
            if due < as_of {
                continue;
            }
            run.placements.push(Placement {
                subject: Subject::CorporateInstalment {
                    entity: entity.clone(),
                    raw_due,
                    due,
                },
                rule: GAP_INCOME_01,
                rules: vec![GAP_INCOME_01],
                outcome: Outcome::placed_on(
                    run.horizon,
                    as_of,
                    due,
                    Direction::Out,
                    Basis::Instalment,
                    Confidence::Firm,
                ),
                amount: Some(amount),
                history: None,
                foreign: None,
                reductions: Vec::new(),
                priority: None,
            });
        }
    }
    let Some(year_end) = tax.tax_year_start.pred_opt() else {
        return;
    };
    let months_after = match tax.balance_due_extension {
        Some(true) => 3,
        Some(false) | None => 2,
    };
    let Some(raw_due) = corporate_balance_due_raw(year_end, months_after) else {
        return;
    };
    let Some(due) = move_due_date(raw_due, Jurisdiction::CanadaFederal) else {
        return;
    };
    if due < as_of || run.horizon.week_of(as_of, due).is_none() {
        return;
    }
    for account in classifications.accounts_classed(entity, &AccountClass::IncomeTaxPayable) {
        let Some(balance) = balance_at(facts, account, year_end) else {
            continue;
        };
        if balance.0.is_zero() {
            continue;
        }
        let estimated = tax.balance_due_extension.is_none();
        let subject = Subject::CorporateBalanceDue {
            entity: entity.clone(),
            year_end,
            raw_due,
            due,
        };
        run.placements.push(Placement {
            subject: subject.clone(),
            rule: GAP_INCOME_01,
            rules: vec![GAP_INCOME_01],
            outcome: Outcome::placed_on(
                run.horizon,
                as_of,
                due,
                Direction::Out,
                Basis::BalanceDue,
                if estimated {
                    Confidence::Estimated
                } else {
                    Confidence::Firm
                },
            ),
            amount: Some(HomeAmount(balance.0.abs())),
            history: None,
            foreign: None,
            reductions: Vec::new(),
            priority: None,
        });
        if estimated {
            run.decision_items.push(DecisionItem {
                kind: ItemKind::ConfirmIncomeTaxBalanceDueDate,
                subject,
                rule: GAP_INCOME_01,
                acted_on_by: Role::Accountant,
                severity: Severity::Action,
                evidence: None,
                draft: None,
                due: None,
                priority: None,
            });
        }
        break;
    }
}

/// GAP-ACCRUAL-01: an accrued liability settles when the owner says; unset means excluded and
/// Provisional (Q120).
fn accruals(
    run: &mut ForecastRun,
    facts: &Facts,
    classifications: &Classifications,
    settings: &Settings,
    entity: &EntityId,
) {
    let as_of = run.as_of;
    for account in classifications.accounts_classed(entity, &AccountClass::AccruedLiabilities) {
        let Some(balance) = balance_at(facts, account, as_of) else {
            continue;
        };
        if balance.0.is_zero() {
            continue;
        }
        let amount = Some(HomeAmount(balance.0.abs()));
        let subject = Subject::Fact(account.clone());
        match settings.accrual_settlements.get(account) {
            Some(AccrualSettlement::OnDate(on)) => {
                run.placements.push(Placement {
                    subject,
                    rule: GAP_ACCRUAL_01,
                    rules: vec![GAP_ACCRUAL_01],
                    outcome: Outcome::placed_on(
                        run.horizon,
                        as_of,
                        *on,
                        Direction::Out,
                        Basis::Accrual,
                        Confidence::Estimated,
                    ),
                    amount,
                    history: None,
                    foreign: None,
                    reductions: Vec::new(),
                    priority: None,
                });
            }
            Some(AccrualSettlement::DaysAfterMonthEnd(days)) => {
                let Some((_, month_end)) = last_completed_month(as_of) else {
                    continue;
                };
                let Some(on) = month_end.checked_add_days(Days::new(u64::from(*days))) else {
                    continue;
                };
                run.placements.push(Placement {
                    subject,
                    rule: GAP_ACCRUAL_01,
                    rules: vec![GAP_ACCRUAL_01],
                    outcome: Outcome::placed_on(
                        run.horizon,
                        as_of,
                        on,
                        Direction::Out,
                        Basis::Accrual,
                        Confidence::Estimated,
                    ),
                    amount,
                    history: None,
                    foreign: None,
                    reductions: Vec::new(),
                    priority: None,
                });
            }
            None => {
                run.placements.push(Placement {
                    subject: subject.clone(),
                    rule: GAP_ACCRUAL_01,
                    rules: vec![GAP_ACCRUAL_01],
                    outcome: Outcome::Excluded(Exclusion::NoExpectedSettlement),
                    amount,
                    history: None,
                    foreign: None,
                    reductions: Vec::new(),
                    priority: None,
                });
                run.decision_items.push(DecisionItem {
                    kind: ItemKind::WhenWillAccrualBePaid,
                    subject: subject.clone(),
                    rule: GAP_ACCRUAL_01,
                    acted_on_by: Role::Accountant,
                    severity: Severity::Action,
                    evidence: None,
                    draft: None,
                    due: None,
                    priority: None,
                });
                run.provisional.push(ProvisionalReason {
                    rule: GAP_ACCRUAL_01,
                    subject,
                });
            }
        }
    }
}
