//! The IC Family: cash moving between Entities in the same Group. Four Rules arrive with CASH
//! (Q275), written ahead of the Family: IC-MAP-01, IC-DOC-01, IC-ELIM-01 and IC-FUND-01.
//! `core/src/cash.rs::run` calls `ic::run` once, before its own per-Entity loop, so IC-DOC-01's
//! Placements are in the run before CASH-ROLL-01 reads them; it calls `ic::fund` once, after that
//! loop, because IC-FUND-01 reads every Entity's own CASH-ROLL-01 output and CASH-SHORT-01's
//! findings. Nothing else here is built: IC-MAP-01's own Decision Item for an unpaired
//! intercompany contact (Q162) is not raised, because no Scenario here has one.

use std::collections::BTreeSet;

use rust_decimal::Decimal;

use crate::document::{due_date, open_home};
use crate::facts::{
    Direction, Document, DocumentKind, DocumentStatus, EntityId, FactId, Facts, Side,
};
use crate::forecast::{
    Basis, Confidence, DecisionItem, Exclusion, ForecastRun, ItemKind, Outcome, Placement, Role,
    RuleId, Severity, Subject, Week,
};
use crate::money::HomeAmount;
use crate::settings::{Classifications, CounterpartyClass, Settings};

pub(crate) const IC_DOC_01: RuleId = RuleId("IC-DOC-01");
pub(crate) const IC_FUND_01: RuleId = RuleId("IC-FUND-01");

/// The Entity an intercompany-classified counterparty names (IC-MAP-01), when it names one.
fn names(classifications: &Classifications, counterparty: &FactId) -> Option<EntityId> {
    match classifications.counterparty(counterparty).class {
        CounterpartyClass::Intercompany(entity) => Some(entity),
        _ => None,
    }
}

/// An open, posted document of either side (IC-DOC-01 reads both an invoice and a bill the same
/// way AR and AP each read their own side).
fn open(facts: &Facts, document: &Document, as_of: chrono::NaiveDate) -> bool {
    document.kind == DocumentKind::Invoice
        && document.status == DocumentStatus::Posted
        && open_home(facts, document, as_of).0 > Decimal::ZERO
}

/// IC-MAP-01 and IC-DOC-01: an open invoice or bill between two connected Entities is forecast in
/// both — a receipt in the Entity it's owed to, a payment in the Entity that owes it, in the same
/// week, both citing the receivable so a reader sees they are the same transaction. The paying
/// Entity's own booking (read here, not cited) supplies its own dates and its own Home Currency
/// amount, at its own booked rate when the pair crosses currencies. AR-OPEN-03 and AP-OPEN-03
/// already excluded both documents as intercompany; that exclusion is replaced here, which is the
/// hole this Rule closes (`docs/plans/cash-build-brief.md`, "Built here").
pub fn run(run: &mut ForecastRun, facts: &Facts, classifications: &Classifications) {
    let as_of = run.as_of;
    let mut used: BTreeSet<FactId> = BTreeSet::new();
    let mut placed: Vec<(EntityId, FactId, HomeAmount, Direction, chrono::NaiveDate)> = Vec::new();

    for (rid, receivable) in facts.documents() {
        if receivable.side != Side::Receivable
            || used.contains(rid)
            || !open(facts, receivable, as_of)
        {
            continue;
        }
        let Some(payer) = names(classifications, &receivable.counterparty) else {
            continue;
        };
        if payer == rid.entity || !facts.ledger_settings().contains_key(&payer) {
            continue;
        }
        let Some((pid, payable)) = facts.documents().iter().find(|(pid, document)| {
            document.side == Side::Payable
                && pid.entity == payer
                && !used.contains(*pid)
                && open(facts, document, as_of)
                && names(classifications, &document.counterparty) == Some(rid.entity.clone())
        }) else {
            continue;
        };

        used.insert(rid.clone());
        used.insert(pid.clone());
        let date = payable
            .expected_date
            .or_else(|| due_date(payable))
            .or_else(|| due_date(receivable))
            .unwrap_or(as_of);
        placed.push((
            rid.entity.clone(),
            rid.clone(),
            open_home(facts, receivable, as_of),
            Direction::In,
            date,
        ));
        placed.push((
            payer,
            rid.clone(),
            open_home(facts, payable, as_of),
            Direction::Out,
            date,
        ));
    }

    if placed.is_empty() {
        return;
    }

    // AR-OPEN-03 and AP-OPEN-03 already excluded both legs before this Rule could pair them; that
    // dead end is replaced by the real Placements below.
    run.placements.retain(|p| {
        !(matches!(p.outcome, Outcome::Excluded(Exclusion::Intercompany))
            && matches!(&p.subject, Subject::Fact(id) if used.contains(id)))
    });

    for (entity, document, amount, direction, date) in placed {
        run.placements.push(Placement {
            subject: Subject::Intercompany { entity, document },
            rule: IC_DOC_01,
            rules: vec![IC_DOC_01],
            outcome: Outcome::placed_on(
                run.horizon,
                as_of,
                date,
                direction,
                Basis::IntercompanyDocument,
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

/// Whether an Entity's own weeks stay at or above its Minimum Cash Buffer (or zero, with none
/// set) from a week to the end of the horizon, after giving up `amount` in the transfer week
/// (IC-FUND-01). Same-currency only: a helper in a different Home Currency is IC-S07's, the
/// Scenario Q275 leaves this thin build to (`docs/rules/ic.md`, IC-FUND-01).
fn could_cover(
    forecast: &crate::forecast::EntityForecast,
    from: Week,
    amount: Decimal,
    buffer: Option<HomeAmount>,
) -> bool {
    let floor = buffer.map_or(Decimal::ZERO, |b| b.0);
    forecast
        .weeks
        .iter()
        .filter(|w| w.week >= from)
        .all(|w| w.closing.0.saturating_sub(amount) >= floor)
}

/// IC-FUND-01: for each Entity with a cash-shortfall item, name whether any other connected
/// Entity in the same Home Currency could transfer enough to cover the deepest week without
/// itself falling below zero or its own buffer, from that week to the end of the horizon. Built
/// thin (`docs/plans/cash-build-brief.md`, "Build thin, and revisit"): S09's own row asserts only
/// that the item exists, so only IC-S07 proves the substance — the caveats IC-FUND-01's own
/// catalogue entry lists, and a transfer across currencies.
pub fn fund(run: &mut ForecastRun, facts: &Facts, settings: &Settings) {
    let shortfalls: Vec<(EntityId, Week, Decimal)> = run
        .decision_items
        .iter()
        .filter_map(|item| match (&item.kind, &item.subject, &item.evidence) {
            (
                ItemKind::CashShortfall,
                Subject::Entity(entity),
                Some(crate::forecast::Evidence::CashStretches { stretches, .. }),
            ) => stretches
                .iter()
                .min_by(|a, b| a.lowest.0.cmp(&b.lowest.0))
                .map(|s| (entity.clone(), s.first, -s.lowest.0)),
            _ => None,
        })
        .collect();
    for (entity, week, needed) in shortfalls {
        let home = facts
            .ledger_settings()
            .get(&entity)
            .map(|s| s.home_currency);
        let can_help = run.entities.iter().any(|other| {
            other.entity != entity
                && facts
                    .ledger_settings()
                    .get(&other.entity)
                    .map(|s| s.home_currency)
                    == home
                && could_cover(
                    other,
                    week,
                    needed,
                    settings.minimum_cash_buffer.get(&other.entity).copied(),
                )
        });
        if can_help {
            run.decision_items.push(DecisionItem {
                kind: ItemKind::ConsiderIntercompanyFunding,
                subject: Subject::Entity(entity),
                rule: IC_FUND_01,
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
