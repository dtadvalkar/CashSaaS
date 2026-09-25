//! The CASH Family: where each Entity's cash starts and what the weeks do to it. It reads every
//! other Family's Placements and produces the run's per-Entity forecast; no Family reads its
//! output. CASH-OPEN-01 and CASH-ROLL-01 arrived a Family early because an approved AP Scenario
//! names them as its input (Q265); AP-DISC-02 still reads `opening_cash` and `weeks` directly.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use rust_decimal::Decimal;

use crate::document::balance_at;
use crate::facts::{Direction, EntityId, FactId, Facts};
use crate::forecast::{
    Basis, Confidence, ConfidenceShares, DecisionItem, EntityForecast, Evidence, Exclusion,
    ForecastRun, GroupView, GroupWeek, Horizon, ItemKind, LowPoint, Outcome, Placement, Priority,
    ProvisionalReason, Role, RuleId, Severity, Stretch, Subject, TrustInStretch, Week,
    WeekForecast,
};
use crate::ic;
use crate::money::{Currency, HomeAmount, ReportingAmount};
use crate::schedule::{nearest_occurrence, occurrence};
use crate::settings::{AccountClass, Classifications, Settings};

const CASH_OPEN_01: RuleId = RuleId("CASH-OPEN-01");
const CASH_OPEN_02: RuleId = RuleId("CASH-OPEN-02");
const CASH_OPEN_03: RuleId = RuleId("CASH-OPEN-03");
const CASH_OPEN_04: RuleId = RuleId("CASH-OPEN-04");
const CASH_HEAD_01: RuleId = RuleId("CASH-HEAD-01");
const CASH_WEEK_01: RuleId = RuleId("CASH-WEEK-01");
const CASH_ROLL_01: RuleId = RuleId("CASH-ROLL-01");
const CASH_CONF_01: RuleId = RuleId("CASH-CONF-01");
const CASH_LOW_01: RuleId = RuleId("CASH-LOW-01");
const CASH_SCHED_01: RuleId = RuleId("CASH-SCHED-01");
const CASH_SHORT_01: RuleId = RuleId("CASH-SHORT-01");
const CASH_BUFFER_01: RuleId = RuleId("CASH-BUFFER-01");
const CASH_ORDER_01: RuleId = RuleId("CASH-ORDER-01");
const CASH_GROUP_01: RuleId = RuleId("CASH-GROUP-01");
const CASH_GROUP_02: RuleId = RuleId("CASH-GROUP-02");

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

/// CASH-OPEN-02, 03 and 04: what happens to every account CASH-OPEN-01 leaves out of Opening
/// Cash. Restricted money is shown, not counted; a clearing balance is a week-1 receipt; a credit
/// card is never cash, and its balance is the GAP Family's to schedule (GAP-CARD-01).
fn other_accounts(
    run: &mut ForecastRun,
    facts: &Facts,
    classifications: &Classifications,
    entity: &EntityId,
    as_of: chrono::NaiveDate,
) {
    for account in classifications.accounts_classed(entity, &AccountClass::Restricted) {
        let Some(balance) = balance_at(facts, account, as_of) else {
            continue;
        };
        run.placements.push(Placement {
            subject: Subject::Fact(account.clone()),
            rule: CASH_OPEN_02,
            rules: vec![CASH_OPEN_02],
            outcome: Outcome::Excluded(Exclusion::Restricted),
            amount: Some(balance),
            history: None,
            foreign: None,
            reductions: Vec::new(),
            priority: None,
        });
    }
    for account in classifications.accounts_classed(entity, &AccountClass::Clearing) {
        let Some(balance) = balance_at(facts, account, as_of) else {
            continue;
        };
        run.placements.push(Placement {
            subject: Subject::Fact(account.clone()),
            rule: CASH_OPEN_03,
            rules: vec![CASH_OPEN_03],
            outcome: Outcome::placed_on(
                run.horizon,
                as_of,
                as_of,
                Direction::In,
                Basis::ClearingBalance,
                Confidence::Firm,
            ),
            amount: Some(balance),
            history: None,
            foreign: None,
            reductions: Vec::new(),
            priority: None,
        });
    }
    for account in classifications.accounts_classed(entity, &AccountClass::CreditCard) {
        let Some(balance) = balance_at(facts, account, as_of) else {
            continue;
        };
        if balance.0.is_zero() {
            continue;
        }
        run.placements.push(Placement {
            subject: Subject::Fact(account.clone()),
            rule: CASH_OPEN_04,
            rules: vec![CASH_OPEN_04],
            outcome: Outcome::Excluded(Exclusion::CreditCardBalance),
            amount: Some(HomeAmount(balance.0.abs())),
            history: None,
            foreign: None,
            reductions: Vec::new(),
            priority: None,
        });
    }
}

/// CASH-HEAD-01: for each credit line with a limit Setting, Headroom is the limit less the drawn
/// balance on the run date, shown beside the forecast and never counted in it. A limit set for a
/// line no account is mapped to raises a Decision Item to map one.
fn headroom(
    run: &mut ForecastRun,
    facts: &Facts,
    classifications: &Classifications,
    settings: &Settings,
    entity: &EntityId,
) {
    let as_of = run.as_of;
    for ((line_entity, name), limit) in &settings.credit_line_limits {
        if line_entity != entity {
            continue;
        }
        let account = facts
            .accounts()
            .values()
            .find(|a| a.id.entity == *entity && a.name == *name)
            .filter(|a| classifications.account(&a.id).class == AccountClass::CreditLine);
        match account {
            Some(account) => {
                let drawn =
                    balance_at(facts, &account.id, as_of).map_or(Decimal::ZERO, |b| b.0.abs());
                run.placements.push(Placement {
                    subject: Subject::Fact(account.id.clone()),
                    rule: CASH_HEAD_01,
                    rules: vec![CASH_HEAD_01],
                    outcome: Outcome::Shown,
                    amount: Some(HomeAmount((limit.0 - drawn).max(Decimal::ZERO))),
                    history: None,
                    foreign: None,
                    reductions: Vec::new(),
                    priority: None,
                });
            }
            None => {
                run.decision_items.push(DecisionItem {
                    kind: ItemKind::MapCreditLineAccount,
                    subject: Subject::CreditLine {
                        entity: entity.clone(),
                        name: name.clone(),
                    },
                    rule: CASH_HEAD_01,
                    acted_on_by: Role::Owner,
                    severity: Severity::Action,
                    evidence: None,
                    draft: None,
                    due: None,
                    priority: None,
                });
            }
        }
    }
}

/// The Headroom CASH-HEAD-01 has already shown for this Entity, summed: what CASH-SHORT-01 and
/// CASH-BUFFER-01 name beside a stretch.
fn entity_headroom(run: &ForecastRun, entity: &EntityId) -> Option<HomeAmount> {
    let shown: Vec<Decimal> = run
        .placements
        .iter()
        .filter(|p| p.subject.entity() == Some(entity) && matches!(p.outcome, Outcome::Shown))
        .filter_map(|p| p.amount.map(|a| a.0))
        .collect();
    (!shown.is_empty()).then(|| {
        HomeAmount(
            shown
                .iter()
                .fold(Decimal::ZERO, |sum, v| sum.saturating_add(*v)),
        )
    })
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

/// The government-trust-marked Placements of this Entity landing inside a stretch, named with
/// amount and placed week (Q282). Sorted by week, then amount. The name is the Subject's own
/// (`Subject::trust_label`) — `GST/HST remittance` for GAP-TAX-01's, a document number for a bill
/// AP-PRIORITY-01 marked — and the citing Rule's id where a Subject has no name of its own.
fn trust_inside(run: &ForecastRun, entity: &EntityId, stretch: &Stretch) -> Vec<TrustInStretch> {
    let mut out: Vec<TrustInStretch> = run
        .placements
        .iter()
        .filter(|p| {
            p.subject.entity() == Some(entity) && p.priority == Some(Priority::GovernmentTrust)
        })
        .filter_map(|p| {
            let Outcome::Placed { week, .. } = p.outcome else {
                return None;
            };
            if week < stretch.first || week > stretch.last {
                return None;
            }
            let amount = p.amount?;
            let label = p
                .subject
                .trust_label()
                .unwrap_or_else(|| p.rule.0.to_owned());
            Some(TrustInStretch {
                label,
                amount,
                week,
            })
        })
        .collect();
    out.sort_by(|a, b| {
        a.week
            .cmp(&b.week)
            .then_with(|| a.amount.0.cmp(&b.amount.0))
    });
    out
}

/// CASH-SHORT-01 and CASH-BUFFER-01: one "cash shortfall" item when any week closes below zero,
/// and, with a Minimum Cash Buffer set, one "below buffer" item for weeks at or above zero but
/// below it — a week below zero belongs to the shortfall item instead. Each item is due on the
/// first day of its first stretch (the reading CASH-S08 confirms: the due date of the stretch,
/// not of the Placement that caused it). Headroom is CASH-HEAD-01's own Shown Placements for this
/// Entity, summed; still none for any Scenario built so far, since none combines a shortfall or a
/// below-buffer week with a mapped credit line.
fn cash_findings(
    run: &mut ForecastRun,
    entity: &EntityId,
    weeks: &[WeekForecast],
    buffer: Option<HomeAmount>,
) {
    let headroom = entity_headroom(run, entity);
    let trust_present = run.placements.iter().any(|p| {
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
            severity: Severity::Action,
            evidence: Some(Evidence::CashStretches {
                buffer: None,
                stretches,
                trust_capable: true,
                trust_present,
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
                severity: Severity::Action,
                evidence: Some(Evidence::CashStretches {
                    buffer: Some(buf),
                    stretches: below_buffer,
                    // CASH-BUFFER-01's own catalogue entry never mentions trust (Q276).
                    trust_capable: false,
                    trust_present: false,
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
/// concerns, by matching Subject (CASH-ORDER-01). The total counts an excluded amount too — a
/// card excluded for want of a payment day still has a size to rank by — but more than one Rule
/// can place the same Subject (CASH-OPEN-04 and GAP-CARD-01 both cite a card's own account), and
/// they always state the same underlying figure, so the largest is taken rather than summed.
fn item_amounts(
    placements: &[Placement],
    subject: &Subject,
    low: Option<Week>,
) -> (Decimal, Decimal) {
    let mut total = Decimal::ZERO;
    let mut landing = Decimal::ZERO;
    for p in placements.iter().filter(|p| &p.subject == subject) {
        let Some(amount) = p.amount else { continue };
        let size = amount.0.abs();
        total = total.max(size);
        if let Outcome::Placed { week, .. } = p.outcome
            && low.is_some_and(|low| week <= low)
        {
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
                    entity: id.entity.clone(),
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

/// CASH-GROUP-01 and CASH-GROUP-02: the Group's weekly totals in its Reporting Currency, and the
/// Entities left out of them. Each Entity converts at the owner's rate for its own pair, and an
/// Entity whose Home Currency is the Reporting Currency converts at one. An Entity the owner has
/// set no rate for is excluded, asked about and makes the run Provisional — never converted at a
/// guessed rate (ADR-0019, Q64). The total is a label over a sum: it raises no Decision Item of
/// its own however low it goes, and each Entity's own forecast is untouched by any of this.
/// Receipts and payments exclude each paired intercompany leg (IC-ELIM-01); `intercompany_difference`
/// carries what a cross-currency pair leaves behind. Closing stays the sum of the Entities' own
/// closings, which already carry the full effect of their own leg.
fn group_view(run: &mut ForecastRun, facts: &Facts, settings: &Settings) -> Option<GroupView> {
    let reporting = settings.reporting_currency?;
    let mut rates: Vec<(EntityId, Decimal)> = Vec::new();
    for (entity, ledger) in facts.ledger_settings() {
        let home = ledger.home_currency;
        // The Reporting Currency needs no rate to state itself; every other pair needs the
        // owner's, and has no default (Q30).
        let rate = if home == reporting {
            Some(Decimal::ONE)
        } else {
            settings.conversion_rates.get(&(home, reporting)).copied()
        };
        match rate {
            Some(rate) => rates.push((entity.clone(), rate)),
            None => no_conversion_rate(run, entity, home, reporting),
        }
    }
    let included: Vec<(&EntityForecast, Decimal)> = rates
        .iter()
        .filter_map(|(entity, rate)| {
            Some((run.entities.iter().find(|e| &e.entity == entity)?, *rate))
        })
        .collect();
    let weeks = (0..usize::try_from(run.horizon.weeks).unwrap_or_default())
        .map(|index| {
            let week = Week(u32::try_from(index).unwrap_or_default().saturating_add(1));
            let at = |pick: fn(&WeekForecast) -> HomeAmount| {
                total(&included, |forecast| Some(pick(forecast.weeks.get(index)?)))
            };
            GroupWeek {
                week,
                opening: at(|w| w.opening),
                receipts: external_movement(run, &rates, week, Direction::In),
                payments: external_movement(run, &rates, week, Direction::Out),
                intercompany_difference: intercompany_difference(run, &rates, week),
                closing: at(|w| w.closing),
            }
        })
        .collect();
    Some(GroupView {
        currency: reporting,
        opening: total(&included, |forecast| Some(forecast.opening)),
        entities: rates.into_iter().map(|(entity, _)| entity).collect(),
        weeks,
        rules: vec![CASH_GROUP_01],
    })
}

/// One figure of the Group's total: each included Entity's own, converted at that Entity's rate
/// and then summed. CASH-GROUP-01 sums Entities' cash; it never converts a pooled figure.
fn total(
    included: &[(&EntityForecast, Decimal)],
    pick: impl Fn(&EntityForecast) -> Option<HomeAmount>,
) -> ReportingAmount {
    included
        .iter()
        .filter_map(|(forecast, rate)| Some(ReportingAmount::convert(pick(forecast)?, *rate)))
        .fold(ReportingAmount::ZERO, ReportingAmount::saturating_add)
}

/// The rate an included Entity converts at, or `None` when it is CASH-GROUP-02's exclusion.
fn rate_of(rates: &[(EntityId, Decimal)], entity: &EntityId) -> Option<Decimal> {
    rates.iter().find(|(id, _)| id == entity).map(|(_, r)| *r)
}

/// The Group's receipts or payments for a week, converted and summed across included Entities,
/// with every Placement of a paired intercompany document left out (IC-ELIM-01: those eliminate
/// instead, into `intercompany_difference`). An unpaired intercompany Placement stays in the
/// totals — it has no other side to cancel against.
fn external_movement(
    run: &ForecastRun,
    rates: &[(EntityId, Decimal)],
    week: Week,
    direction: Direction,
) -> ReportingAmount {
    let paired = paired_documents(run);
    let paired_occs = paired_ic_occurrences(run);
    run.placements
        .iter()
        .filter(|p| match &p.subject {
            Subject::Intercompany { document, .. } => !paired.contains(document),
            Subject::Occurrence {
                template, date: on, ..
            } if p.rule.family() == "IC" => !paired_occs.contains(&(template, *on)),
            _ => true,
        })
        .filter_map(|p| {
            let Outcome::Placed {
                week: w,
                direction: d,
                ..
            } = p.outcome
            else {
                return None;
            };
            if w != week || d != direction {
                return None;
            }
            let entity = p.subject.entity()?;
            let rate = rate_of(rates, entity)?;
            Some(ReportingAmount::convert(p.amount?, rate))
        })
        .fold(ReportingAmount::ZERO, ReportingAmount::saturating_add)
}

/// Documents that have exactly two Intercompany Placement legs (IC-ELIM-01). One leg is unpaired
/// and is not eliminated; three would be a construction error, treated as unpaired.
fn paired_documents(run: &ForecastRun) -> BTreeSet<&FactId> {
    let mut counts: BTreeMap<&FactId, usize> = BTreeMap::new();
    for p in &run.placements {
        if let Subject::Intercompany { document, .. } = &p.subject
            && matches!(p.outcome, Outcome::Placed { .. })
        {
            *counts.entry(document).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .filter(|(_, n)| *n == 2)
        .map(|(d, _)| d)
        .collect()
}

/// IC-LOAN-01 schedule occurrences with exactly two legs (IC-ELIM-01).
fn paired_ic_occurrences(run: &ForecastRun) -> BTreeSet<(&FactId, chrono::NaiveDate)> {
    let mut counts: BTreeMap<(&FactId, chrono::NaiveDate), usize> = BTreeMap::new();
    for p in &run.placements {
        if let Subject::Occurrence {
            template, date: on, ..
        } = &p.subject
            && matches!(p.outcome, Outcome::Placed { .. })
            && p.rule.family() == "IC"
        {
            *counts.entry((template, *on)).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .filter(|(_, n)| *n == 2)
        .map(|(k, _)| k)
        .collect()
}

/// IC-ELIM-01: the part of each intercompany pair placed this week that does not eliminate at the
/// conversion rate — zero for a same-currency pair, the residual for a cross-currency one. A leg
/// whose Entity has no conversion rate (CASH-GROUP-02) is left out of the pair entirely, as the
/// Group total already leaves that Entity out. Unpaired Placements do not contribute.
fn intercompany_difference(
    run: &ForecastRun,
    rates: &[(EntityId, Decimal)],
    week: Week,
) -> ReportingAmount {
    let paired = paired_documents(run);
    let paired_occs = paired_ic_occurrences(run);
    let mut by_document: BTreeMap<&FactId, ReportingAmount> = BTreeMap::new();
    let mut by_occurrence: BTreeMap<(&FactId, chrono::NaiveDate), ReportingAmount> =
        BTreeMap::new();
    for p in &run.placements {
        let Outcome::Placed {
            week: w, direction, ..
        } = p.outcome
        else {
            continue;
        };
        if w != week {
            continue;
        }
        let Some(entity) = p.subject.entity() else {
            continue;
        };
        let Some(rate) = rate_of(rates, entity) else {
            continue;
        };
        let Some(amount) = p.amount else { continue };
        let signed = match direction {
            Direction::In => ReportingAmount::convert(amount, rate),
            Direction::Out => ReportingAmount::convert(HomeAmount(-amount.0), rate),
        };
        match &p.subject {
            Subject::Intercompany { document, .. } if paired.contains(document) => {
                let entry = by_document.entry(document).or_insert(ReportingAmount::ZERO);
                *entry = entry.saturating_add(signed);
            }
            Subject::Occurrence {
                template, date: on, ..
            } if p.rule.family() == "IC" && paired_occs.contains(&(template, *on)) => {
                let entry = by_occurrence
                    .entry((template, *on))
                    .or_insert(ReportingAmount::ZERO);
                *entry = entry.saturating_add(signed);
            }
            _ => {}
        }
    }
    by_document
        .into_values()
        .chain(by_occurrence.into_values())
        .fold(ReportingAmount::ZERO, ReportingAmount::saturating_add)
}

/// CASH-GROUP-02's three outputs for one Entity: the exclusion, the item asking the owner for the
/// rate, and the Provisional reason. The item's Subject names the Entity as well as the pair, so
/// it joins that Entity's queue (Q155) and `is_blocking` matches it to the reason.
fn no_conversion_rate(run: &mut ForecastRun, entity: &EntityId, home: Currency, to: Currency) {
    let subject = Subject::Conversion {
        entity: entity.clone(),
        from: home,
        to,
    };
    run.placements.push(Placement {
        subject: Subject::Entity(entity.clone()),
        rule: CASH_GROUP_02,
        rules: vec![CASH_GROUP_02],
        outcome: Outcome::Excluded(Exclusion::NoConversionRate),
        amount: None,
        history: None,
        foreign: None,
        reductions: Vec::new(),
        priority: None,
    });
    run.decision_items.push(DecisionItem {
        kind: ItemKind::SetConversionRate,
        subject: subject.clone(),
        rule: CASH_GROUP_02,
        acted_on_by: Role::Owner,
        severity: Severity::Action,
        evidence: None,
        draft: None,
        due: None,
        priority: None,
    });
    run.provisional.push(ProvisionalReason {
        rule: CASH_GROUP_02,
        subject,
    });
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
    // IC-MAP-01 and IC-DOC-01: cross-Entity, so it runs before any Entity's own roll, on the
    // Placements every other Family has already made — AR-OPEN-03 and AP-OPEN-03 excluded both
    // legs of a paired intercompany document, which this replaces with the real pair.
    ic::run(run, facts, classifications, settings);
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
        other_accounts(run, facts, classifications, entity, run.as_of);
        headroom(run, facts, classifications, settings, entity);
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
    // IC-FUND-01: needs every Entity's own CASH-ROLL-01 output and CASH-SHORT-01's own findings,
    // so it runs after the per-Entity loop, not inside it.
    ic::fund(run, facts, settings);
    run.group = group_view(run, facts, settings);
    order_queue(run);
}
