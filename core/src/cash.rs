//! The CASH Family: where each Entity's cash starts and what the weeks do to it. It reads every
//! other Family's Placements and produces the run's per-Entity forecast; no Family reads its
//! output. CASH-OPEN-01 and CASH-ROLL-01 arrived a Family early because an approved AP Scenario
//! names them as its input (Q265); AP-DISC-02 still reads `opening_cash` and `weeks` directly.

use std::cmp::Reverse;

use rust_decimal::Decimal;

use crate::document::balance_at;
use crate::facts::{Direction, EntityId, FactId, Facts};
use crate::forecast::{
    Basis, Confidence, ConfidenceShares, DecisionItem, EntityForecast, Evidence, Exclusion,
    ForecastRun, Horizon, ItemKind, LowPoint, Outcome, Placement, Priority, ProvisionalReason,
    Role, RuleId, Severity, Stretch, Subject, Week, WeekForecast,
};
use crate::money::HomeAmount;
use crate::schedule::{nearest_occurrence, occurrence};
use crate::settings::{AccountClass, Classifications, Settings};

const CASH_OPEN_01: RuleId = RuleId("CASH-OPEN-01");
const CASH_WEEK_01: RuleId = RuleId("CASH-WEEK-01");
const CASH_ROLL_01: RuleId = RuleId("CASH-ROLL-01");
const CASH_CONF_01: RuleId = RuleId("CASH-CONF-01");
const CASH_LOW_01: RuleId = RuleId("CASH-LOW-01");
const CASH_SCHED_01: RuleId = RuleId("CASH-SCHED-01");
const CASH_SHORT_01: RuleId = RuleId("CASH-SHORT-01");
const CASH_BUFFER_01: RuleId = RuleId("CASH-BUFFER-01");
const CASH_ORDER_01: RuleId = RuleId("CASH-ORDER-01");

/// CASH-OPEN-01: the cash a Forecast Run starts from — the book balance of the Entity's
/// bank-classed accounts on the run date. Restricted accounts, clearing accounts and credit cards
/// are not bank accounts however a ledger types them (`CONTEXT.md`), so only the `Bank` class
/// counts, and a balance dated any other day is not read (ADR-0014).
pub fn opening_cash(
    facts: &Facts,
    classifications: &Classifications,
    entity: &EntityId,
    as_of: chrono::NaiveDate,
) -> HomeAmount {
    HomeAmount(
        classifications
            .accounts_classed(entity, &AccountClass::Bank)
            .filter_map(|account| balance_at(facts, account, as_of))
            .fold(Decimal::ZERO, |sum, balance| sum.saturating_add(balance.0)),
    )
}

/// The bank accounts Opening Cash was read from, which the Placement cites. An account with no
/// balance on the run date contributed nothing, so it is not cited (ADR-0014).
fn opening_accounts(
    facts: &Facts,
    classifications: &Classifications,
    entity: &EntityId,
    as_of: chrono::NaiveDate,
) -> Vec<FactId> {
    classifications
        .accounts_classed(entity, &AccountClass::Bank)
        .filter(|account| balance_at(facts, account, as_of).is_some())
        .cloned()
        .collect()
}

/// CASH-ROLL-01: one row per week of the Horizon for one Entity, from its Opening Cash and the
/// run's Placements. Each week opens at the previous week's closing cash and closes at that plus
/// its receipts less its payments. An exclusion has no week and moves nothing (ADR-0011), and a
/// Placement belonging to another Entity is another Entity's roll: Group cash is not pooled (Q87).
/// Every figure is in the Entity's Home Currency; converting a week to the Group's Reporting
/// Currency is CASH-GROUP-01's, at the owner's rate Setting (Q273, Q30).
pub fn weeks(
    placements: &[Placement],
    entity: &EntityId,
    opening: HomeAmount,
    horizon: Horizon,
) -> Vec<WeekForecast> {
    let count = usize::try_from(horizon.weeks).unwrap_or_default();
    // Per week: receipts, payments, and the size placed at each Confidence level (CASH-CONF-01).
    let mut moved = vec![[Decimal::ZERO; 4]; count];
    for placement in placements
        .iter()
        .filter(|p| p.subject.entity() == Some(entity))
    {
        let Outcome::Placed {
            week,
            direction,
            confidence,
            ..
        } = placement.outcome
        else {
            continue;
        };
        let Some(amount) = placement.amount else {
            continue;
        };
        let index = usize::try_from(week.0)
            .unwrap_or_default()
            .saturating_sub(1);
        if let Some(week) = moved.get_mut(index) {
            match direction {
                Direction::In => week[0] = week[0].saturating_add(amount.0),
                Direction::Out => week[1] = week[1].saturating_add(amount.0),
            }
            // By size, so a large estimated receipt is not hidden by an equal firm payment.
            let at = match confidence {
                Confidence::Firm => 2,
                Confidence::Estimated => 3,
            };
            week[at] = week[at].saturating_add(amount.0.abs());
        }
    }
    let mut running = opening.0;
    moved
        .into_iter()
        .enumerate()
        .map(|(index, [receipts, payments, firm, estimated])| {
            let week = WeekForecast {
                week: Week(u32::try_from(index).unwrap_or_default().saturating_add(1)),
                opening: HomeAmount(running),
                receipts: HomeAmount(receipts),
                payments: HomeAmount(payments),
                closing: HomeAmount(running.saturating_add(receipts).saturating_sub(payments)),
                confidence: shares(firm, estimated),
            };
            running = week.closing.0;
            week
        })
        .collect()
}

/// CASH-CONF-01: each level's share of the week's placed size, as a percentage. A week with
/// nothing placed has no shares (Q153); 100% firm would claim evidence it does not have.
fn shares(firm: Decimal, estimated: Decimal) -> Option<ConfidenceShares> {
    let total = firm.saturating_add(estimated);
    if total.is_zero() {
        return None;
    }
    let hundred = Decimal::from(100);
    let share = |part: Decimal| {
        part.checked_mul(hundred)
            .and_then(|scaled| scaled.checked_div(total))
            .unwrap_or_default()
    };
    Some(ConfidenceShares {
        firm: share(firm),
        estimated: share(estimated),
    })
}

/// CASH-LOW-01: the lowest weekly closing cash, and the week it falls in. When weeks tie it is the
/// earliest, which is where the Entity first reaches that level (Q152).
fn low_point(weeks: &[WeekForecast]) -> Option<LowPoint> {
    weeks
        .iter()
        .min_by(|a, b| a.closing.0.cmp(&b.closing.0).then(a.week.cmp(&b.week)))
        .map(|week| LowPoint {
            week: week.week,
            amount: week.closing,
        })
}

/// Every continuous run of weeks whose closing cash satisfies `zone`, shared by CASH-SHORT-01
/// (below zero) and CASH-BUFFER-01 (at or above zero but below the buffer). A week outside the
/// zone ends the run in progress.
fn stretches(weeks: &[WeekForecast], zone: impl Fn(Decimal) -> bool) -> Vec<Stretch> {
    let mut out = Vec::new();
    let mut open: Option<(Week, Week, Decimal)> = None;
    for week in weeks {
        if zone(week.closing.0) {
            open = Some(match open {
                Some((first, _, lowest)) => (first, week.week, lowest.min(week.closing.0)),
                None => (week.week, week.week, week.closing.0),
            });
        } else if let Some((first, last, lowest)) = open.take() {
            out.push(Stretch {
                first,
                last,
                lowest: HomeAmount(lowest),
                trust: Vec::new(),
            });
        }
    }
    if let Some((first, last, lowest)) = open {
        out.push(Stretch {
            first,
            last,
            lowest: HomeAmount(lowest),
            trust: Vec::new(),
        });
    }
    out
}

/// The government-trust-marked Placements of this Entity landing inside a stretch, by Subject.
fn trust_inside(run: &ForecastRun, entity: &EntityId, stretch: &Stretch) -> Vec<FactId> {
    run.placements
        .iter()
        .filter(|p| {
            p.subject.entity() == Some(entity) && p.priority == Some(Priority::GovernmentTrust)
        })
        .filter_map(|p| match (&p.outcome, &p.subject) {
            (Outcome::Placed { week, .. }, Subject::Fact(id))
                if *week >= stretch.first && *week <= stretch.last =>
            {
                Some(id.clone())
            }
            _ => None,
        })
        .collect()
}

/// CASH-SHORT-01 and CASH-BUFFER-01: one "cash shortfall" item when any week closes below zero,
/// and, with a Minimum Cash Buffer set, one "below buffer" item for weeks at or above zero but
/// below it — a week below zero belongs to the shortfall item instead. Each item is due on the
/// first day of its first stretch (the reading CASH-S08 confirms: the due date of the stretch,
/// not of the Placement that caused it). Headroom is always none until CASH-HEAD-01 arrives at
/// checkpoint D; no credit line exists in `Settings` yet for it to read.
fn cash_findings(
    run: &mut ForecastRun,
    entity: &EntityId,
    weeks: &[WeekForecast],
    buffer: Option<HomeAmount>,
) {
    let headroom: Option<HomeAmount> = None;
    let trust_relevant = run.placements.iter().any(|p| {
        p.subject.entity() == Some(entity) && p.priority == Some(Priority::GovernmentTrust)
    });

    let below_zero = stretches(weeks, |c| c < Decimal::ZERO);
    if let Some(first) = below_zero.first() {
        let due = first.first.start(run.as_of);
        let stretches = below_zero
            .into_iter()
            .map(|s| Stretch {
                trust: trust_inside(run, entity, &s),
                ..s
            })
            .collect();
        run.decision_items.push(DecisionItem {
            kind: ItemKind::CashShortfall,
            subject: Subject::Entity(entity.clone()),
            rule: CASH_SHORT_01,
            acted_on_by: Role::Owner,
            severity: Severity::Critical,
            evidence: Some(Evidence::CashStretches {
                buffer: None,
                stretches,
                trust_relevant,
                headroom,
            }),
            draft: None,
            due: Some(due),
            priority: None,
        });
    }

    if let Some(buf) = buffer {
        let below_buffer = stretches(weeks, |c| c >= Decimal::ZERO && c < buf.0);
        if let Some(first) = below_buffer.first() {
            let due = first.first.start(run.as_of);
            run.decision_items.push(DecisionItem {
                kind: ItemKind::BelowBuffer,
                subject: Subject::Entity(entity.clone()),
                rule: CASH_BUFFER_01,
                acted_on_by: Role::Owner,
                severity: Severity::Critical,
                evidence: Some(Evidence::CashStretches {
                    buffer: Some(buf),
                    stretches: below_buffer,
                    // CASH-BUFFER-01's own catalogue entry never mentions trust (Q276).
                    trust_relevant: false,
                    headroom,
                }),
                draft: None,
                due: Some(due),
                priority: None,
            });
        }
    }
}

/// The total and the on-or-before-Low-Point-week size of the Placements a Decision Item
/// concerns, by matching Subject (CASH-ORDER-01).
fn item_amounts(
    placements: &[Placement],
    subject: &Subject,
    low: Option<Week>,
) -> (Decimal, Decimal) {
    let mut total = Decimal::ZERO;
    let mut landing = Decimal::ZERO;
    for p in placements.iter().filter(|p| &p.subject == subject) {
        let Outcome::Placed { week, .. } = p.outcome else {
            continue;
        };
        let Some(amount) = p.amount else { continue };
        let size = amount.0.abs();
        total = total.saturating_add(size);
        if low.is_some_and(|low| week <= low) {
            landing = landing.saturating_add(size);
        }
    }
    (total, landing)
}

/// Whether a Decision Item is blocking: produced by a Rule whose own instance also marked the
/// run Provisional over the same Subject (CASH-ORDER-01 reads "marks Provisional" per finding,
/// not per Rule, since a Rule can find something that does and something that doesn't).
fn is_blocking(item: &DecisionItem, provisional: &[ProvisionalReason]) -> bool {
    provisional
        .iter()
        .any(|reason| reason.rule == item.rule && reason.subject == item.subject)
}

/// CASH-ORDER-01: the severity class and queue order for every Decision Item from every Family.
/// Critical (cash shortfall, below buffer — shortfall always first, Q154), then blocking, then
/// action. Within a class: trust obligations first, then a vendor the owner marks critical or a
/// secured lender (AP-PRIORITY-01), then the rest, ordered by the cash they concern landing on or
/// before the Entity's Low Point week, then by total amount — a zero landing amount sorts last
/// within its group (the reading CASH-S08 confirms). Per-Entity grouping (Q155) is not needed
/// until a multi-Entity Scenario reads this queue.
fn order_queue(run: &mut ForecastRun) {
    let low_points: std::collections::BTreeMap<EntityId, Week> = run
        .entities
        .iter()
        .filter_map(|e| e.low_point.map(|lp| (e.entity.clone(), lp.week)))
        .collect();

    type Key = (u8, u8, u8, Reverse<Decimal>, Reverse<Decimal>);
    let keyed: Vec<(Severity, Key)> = run
        .decision_items
        .iter()
        .map(|item| {
            let severity = match item.kind {
                ItemKind::CashShortfall => Severity::Critical,
                ItemKind::BelowBuffer => Severity::Critical,
                _ if is_blocking(item, &run.provisional) => Severity::Blocking,
                _ => Severity::Action,
            };
            let severity_rank = match severity {
                Severity::Critical => 0,
                Severity::Blocking => 1,
                Severity::Action => 2,
            };
            let kind_rank = match item.kind {
                ItemKind::CashShortfall => 0,
                ItemKind::BelowBuffer => 1,
                _ => 0,
            };
            let priority_rank = match item.priority {
                Some(Priority::GovernmentTrust) => 0,
                Some(Priority::CriticalVendor | Priority::SecuredLender) => 1,
                None => 2,
            };
            let low = item
                .subject
                .entity()
                .and_then(|e| low_points.get(e).copied());
            let (total, landing) = item_amounts(&run.placements, &item.subject, low);
            (
                severity,
                (
                    severity_rank,
                    kind_rank,
                    priority_rank,
                    Reverse(landing),
                    Reverse(total),
                ),
            )
        })
        .collect();

    for (item, (severity, _)) in run.decision_items.iter_mut().zip(&keyed) {
        item.severity = *severity;
    }

    let mut order: Vec<usize> = (0..run.decision_items.len()).collect();
    order.sort_by_key(|&i| keyed[i].1);
    run.decision_items = order
        .into_iter()
        .map(|i| run.decision_items[i].clone())
        .collect();
}

/// CASH-SCHED-01: one receipt per occurrence of an owner-entered schedule that falls inside the
/// Horizon, unless a bank receipt from that payer already covers it. Coverage is by nearest
/// scheduled date, not by interval, so a receipt banked a few days early covers the occurrence it
/// was meant for and not the next one (Q149) — the reading that keeps 3,000.00 from going missing.
/// An occurrence before the run date is neither placed nor shown: it is already in Opening Cash.
fn scheduled_receipts(
    run: &mut ForecastRun,
    facts: &Facts,
    settings: &Settings,
    classifications: &Classifications,
    entity: &EntityId,
) {
    let as_of = run.as_of;
    let end = run.horizon.end(as_of);
    for (id, receipt) in settings
        .scheduled_receipts
        .iter()
        .filter(|(id, _)| id.entity == *entity)
    {
        let covered = covered_occurrences(facts, classifications, entity, receipt);
        let within = |on: chrono::NaiveDate| receipt.end.is_none_or(|last| on <= last);
        let mut k = 0;
        // The schedule extends before its entered date, so walk forward from the run date.
        while let Some(on) = occurrence(receipt.start, receipt.frequency, k) {
            k = k.saturating_add(1);
            if on >= end || !within(on) {
                break;
            }
            if on < as_of {
                continue;
            }
            let outcome = match covered.get(&on) {
                Some(line) => Outcome::Excluded(Exclusion::CoveredBy(line.clone())),
                None => Outcome::placed_on(
                    run.horizon,
                    as_of,
                    on,
                    Direction::In,
                    Basis::ScheduledReceipt,
                    Confidence::Firm,
                ),
            };
            run.placements.push(Placement {
                subject: Subject::Occurrence {
                    template: id.clone(),
                    date: on,
                },
                rule: CASH_SCHED_01,
                rules: vec![CASH_SCHED_01],
                outcome,
                amount: Some(receipt.amount),
                history: None,
                foreign: None,
                reductions: Vec::new(),
                priority: None,
            });
        }
    }
}

/// The scheduled dates a bank receipt from the payer already covers, each with the line covering
/// it. A receipt is money in to a bank account: a spend is the same fact the other way and covers
/// nothing here.
fn covered_occurrences(
    facts: &Facts,
    classifications: &Classifications,
    entity: &EntityId,
    receipt: &crate::settings::ScheduledReceipt,
) -> std::collections::BTreeMap<chrono::NaiveDate, FactId> {
    let mut map = std::collections::BTreeMap::new();
    for line in facts.account_lines().values() {
        let banked = classifications.account(&line.account).class == AccountClass::Bank;
        if line.id.entity != *entity
            || !banked
            || line.amount.home.0 <= Decimal::ZERO
            || line.counterparty.as_ref() != Some(&receipt.payer)
        {
            continue;
        }
        if let Some(on) = nearest_occurrence(receipt.start, receipt.frequency, line.date) {
            map.entry(on).or_insert_with(|| line.id.clone());
        }
    }
    map
}

/// The Family, run once after every other Family has placed (CASH-ROLL-01 reads their Placements,
/// and CASH-GROUP-01 and CASH-ORDER-01 cross Entity boundaries). The Rules that turn this into
/// Confidence shares, Headroom, a Low Point, cash findings, the queue and the Group view arrive at
/// their own checkpoints.
pub fn run(
    run: &mut ForecastRun,
    facts: &Facts,
    settings: &Settings,
    classifications: &Classifications,
) {
    // CASH-WEEK-01: the horizon is per Group and has no general default, so an unset one is asked
    // for once, not once per Entity. With none set no weeks are produced and nothing is bucketed.
    if settings.horizon.is_none() {
        run.decision_items.push(DecisionItem {
            kind: ItemKind::SetHorizon,
            subject: Subject::Group,
            rule: CASH_WEEK_01,
            acted_on_by: Role::Owner,
            severity: Severity::Action,
            evidence: None,
            draft: None,
            due: None,
            priority: None,
        });
    }
    for entity in facts.ledger_settings().keys().cloned().collect::<Vec<_>>() {
        let entity = &entity;
        scheduled_receipts(run, facts, settings, classifications, entity);
        let opening = opening_cash(facts, classifications, entity, run.as_of);
        run.placements.push(Placement {
            subject: Subject::Position {
                entity: entity.clone(),
                accounts: opening_accounts(facts, classifications, entity, run.as_of),
            },
            rule: CASH_OPEN_01,
            rules: vec![CASH_OPEN_01],
            outcome: Outcome::Stated {
                confidence: Confidence::Firm,
            },
            amount: Some(opening),
            history: None,
            foreign: None,
            reductions: Vec::new(),
            priority: None,
        });
        let weeks = weeks(&run.placements, entity, opening, run.horizon);
        let mut rules = Vec::new();
        if !weeks.is_empty() {
            rules.push(CASH_ROLL_01);
            rules.push(CASH_LOW_01);
        }
        if weeks.iter().any(|week| week.confidence.is_some()) {
            rules.push(CASH_CONF_01);
        }
        let buffer = settings.minimum_cash_buffer.get(entity).copied();
        cash_findings(run, entity, &weeks, buffer);
        // CASH-ORDER-01 always classes and orders this Entity's queue, even a queue of one.
        rules.push(CASH_ORDER_01);
        run.entities.push(EntityForecast {
            entity: entity.clone(),
            opening,
            low_point: low_point(&weeks),
            weeks,
            rules,
        });
    }
    order_queue(run);
}
