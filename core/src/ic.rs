//! The IC Family: cash moving between Entities in the same Group. Four Rules arrived with CASH
//! (Q275); this Family widens them and adds the rest. `core/src/cash.rs::run` calls `ic::run`
//! once, before its own per-Entity loop, so IC-DOC-01's Placements are in the run before
//! CASH-ROLL-01 reads them; it calls `ic::fund` once, after that loop, because IC-FUND-01 reads
//! every Entity's own CASH-ROLL-01 output and CASH-SHORT-01's findings.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;

use crate::document::{balance_at, due_date, foreign, open_home, open_own};
use crate::facts::{
    Direction, Document, DocumentKind, DocumentStatus, EntityId, FactId, Facts, PostingStatus, Side,
};
use crate::forecast::{
    Basis, Confidence, DecisionItem, Evidence, Exclusion, ForecastRun, ForeignAmount, ItemKind,
    Outcome, Placement, ProvisionalReason, Role, RuleId, Severity, Subject, Week,
};
use crate::money::{Currency, HomeAmount, round_money};
use crate::schedule::{nearest_occurrence, occurrence};
use crate::settings::{
    AccountClass, Classifications, CounterpartyClass, IntercompanySettlementSchedule, Settings,
};

pub(crate) const IC_MAP_01: RuleId = RuleId("IC-MAP-01");
pub(crate) const IC_DOC_01: RuleId = RuleId("IC-DOC-01");
pub(crate) const IC_DOC_02: RuleId = RuleId("IC-DOC-02");
pub(crate) const IC_ELIM_01: RuleId = RuleId("IC-ELIM-01");
pub(crate) const IC_FUND_01: RuleId = RuleId("IC-FUND-01");
pub(crate) const IC_LOAN_01: RuleId = RuleId("IC-LOAN-01");
pub(crate) const IC_LOAN_02: RuleId = RuleId("IC-LOAN-02");
pub(crate) const IC_ONESIDED_01: RuleId = RuleId("IC-ONESIDED-01");
pub(crate) const IC_AGREE_01: RuleId = RuleId("IC-AGREE-01");
pub(crate) const IC_AGREE_02: RuleId = RuleId("IC-AGREE-02");

/// The Entity an intercompany-classified counterparty names (IC-MAP-01), when it names one.
fn names(classifications: &Classifications, counterparty: &FactId) -> Option<Option<EntityId>> {
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

/// IC-DOC-02's match: same reference, or same amount and date (Q130). Cross-currency pairs
/// compare the transaction amount when Home Currency amounts differ (CASH-S09's CG-2207).
fn documents_match(a: &Document, b: &Document, a_open: HomeAmount, b_open: HomeAmount) -> bool {
    let refs_equal = |x: &Option<String>, y: &Option<String>| match (x, y) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    };
    if refs_equal(&a.number, &b.number)
        || refs_equal(&a.vendor_reference, &b.vendor_reference)
        || refs_equal(&a.number, &b.vendor_reference)
        || refs_equal(&a.vendor_reference, &b.number)
    {
        return true;
    }
    if a.date != b.date {
        return false;
    }
    if a_open == b_open {
        return true;
    }
    let a_tx = a.total.transaction.as_ref().map(|t| t.amount.0);
    let b_tx = b.total.transaction.as_ref().map(|t| t.amount.0);
    match (a_tx, b_tx) {
        (Some(x), Some(y)) => x == y,
        (Some(x), None) => x == b_open.0,
        (None, Some(y)) => a_open.0 == y,
        (None, None) => false,
    }
}

fn document_reference(document: &Document) -> String {
    document
        .number
        .clone()
        .or_else(|| document.vendor_reference.clone())
        .unwrap_or_default()
}

/// On `on_entity`'s books, the contact classified as the sister Entity (IC-DOC-02's draft).
fn sister_contact(
    facts: &Facts,
    classifications: &Classifications,
    on_entity: &EntityId,
    sister: &EntityId,
) -> Option<FactId> {
    facts.counterparties().keys().find_map(|id| {
        (id.entity == *on_entity && names(classifications, id) == Some(Some(sister.clone())))
            .then(|| id.clone())
    })
}

/// Rules cited on one leg of a named pair: MAP produced the pairing key; receipts also cite
/// IC-ELIM-01 (IC-S01).
fn pair_rules(direction: Direction) -> Vec<RuleId> {
    match direction {
        Direction::In => vec![IC_MAP_01, IC_DOC_01, IC_ELIM_01],
        Direction::Out => vec![IC_MAP_01, IC_DOC_01],
    }
}

/// One IC-DOC-01 Placement ready to push once AR/AP exclusions are cleared.
struct PendingPlacement {
    entity: EntityId,
    document: FactId,
    amount: HomeAmount,
    direction: Direction,
    date: chrono::NaiveDate,
    foreign: Option<crate::forecast::ForeignAmount>,
    rules: Vec<RuleId>,
}

/// IC-MAP-01 and IC-DOC-01 (widened for one-sided and unnamed, Q162): an open invoice or bill
/// classified intercompany is forecast. A named, connected counterparty gets both legs when the
/// other side is booked, or both legs from this side's dates when it is not (IC-DOC-02's draft).
/// An unnamed or unconnected counterparty is forecast unpaired on this side's dates.
pub fn run(
    run: &mut ForecastRun,
    facts: &Facts,
    classifications: &Classifications,
    settings: &Settings,
) {
    let as_of = run.as_of;
    let mut used: BTreeSet<FactId> = BTreeSet::new();
    // Connected counterparties replace AP/AR's Intercompany exclusion with IC Placements.
    // Unnamed (MAP) and unconnected (ONESIDED) leave that exclusion in place so AP-S05's
    // "intercompany, see IC" row stays, while IC still forecasts the visible side (Q128 / Q162).
    let mut clears_exclusion: BTreeSet<FactId> = BTreeSet::new();
    let mut placed: Vec<PendingPlacement> = Vec::new();
    let mut named_contacts: BTreeSet<FactId> = BTreeSet::new();
    let mut onesided_contacts: BTreeSet<FactId> = BTreeSet::new();
    let mut drafts: Vec<(EntityId, FactId, &Document, HomeAmount)> = Vec::new();

    for (rid, receivable) in facts.documents() {
        if receivable.side != Side::Receivable
            || used.contains(rid)
            || !open(facts, receivable, as_of)
        {
            continue;
        }
        let Some(named) = names(classifications, &receivable.counterparty) else {
            continue;
        };
        let open_recv = open_home(facts, receivable, as_of);
        let Some(payer) = named else {
            // IC-MAP-01 / Q162: intercompany, no counterparty named — unpaired on own dates.
            named_contacts.insert(receivable.counterparty.clone());
            let date = receivable
                .expected_date
                .or_else(|| due_date(receivable))
                .unwrap_or(as_of);
            used.insert(rid.clone());
            placed.push(PendingPlacement {
                entity: rid.entity.clone(),
                document: rid.clone(),
                amount: open_recv,
                direction: Direction::In,
                date,
                foreign: foreign(receivable, open_own(facts, receivable, as_of)),
                rules: vec![IC_MAP_01, IC_DOC_01],
            });
            continue;
        };
        if payer == rid.entity {
            continue;
        }
        if !facts.ledger_settings().contains_key(&payer) {
            // IC-ONESIDED-01: named but not connected — unpaired on the visible side (Q128).
            onesided_contacts.insert(receivable.counterparty.clone());
            let date = receivable
                .expected_date
                .or_else(|| due_date(receivable))
                .unwrap_or(as_of);
            used.insert(rid.clone());
            placed.push(PendingPlacement {
                entity: rid.entity.clone(),
                document: rid.clone(),
                amount: open_recv,
                direction: Direction::In,
                date,
                foreign: foreign(receivable, open_own(facts, receivable, as_of)),
                rules: vec![IC_ONESIDED_01, IC_DOC_01],
            });
            continue;
        }

        let matched = facts.documents().iter().find(|(pid, document)| {
            document.side == Side::Payable
                && pid.entity == payer
                && !used.contains(*pid)
                && open(facts, document, as_of)
                && names(classifications, &document.counterparty) == Some(Some(rid.entity.clone()))
                && documents_match(
                    receivable,
                    document,
                    open_recv,
                    open_home(facts, document, as_of),
                )
        });

        if let Some((pid, payable)) = matched {
            used.insert(rid.clone());
            used.insert(pid.clone());
            clears_exclusion.insert(rid.clone());
            clears_exclusion.insert(pid.clone());
            let date = payable
                .expected_date
                .or_else(|| due_date(payable))
                .or_else(|| due_date(receivable))
                .unwrap_or(as_of);
            let open_pay = open_home(facts, payable, as_of);
            placed.push(PendingPlacement {
                entity: rid.entity.clone(),
                document: rid.clone(),
                amount: open_recv,
                direction: Direction::In,
                date,
                foreign: foreign(receivable, open_own(facts, receivable, as_of)),
                rules: pair_rules(Direction::In),
            });
            placed.push(PendingPlacement {
                entity: payer.clone(),
                document: rid.clone(),
                amount: open_pay,
                direction: Direction::Out,
                date,
                foreign: foreign(payable, open_own(facts, payable, as_of)),
                rules: pair_rules(Direction::Out),
            });
        } else {
            // One-sided: only the receivable is booked; both Entities still get a Placement on
            // this side's dates (IC-DOC-01), and IC-DOC-02 drafts the missing bill.
            used.insert(rid.clone());
            clears_exclusion.insert(rid.clone());
            let date = receivable
                .expected_date
                .or_else(|| due_date(receivable))
                .unwrap_or(as_of);
            placed.push(PendingPlacement {
                entity: rid.entity.clone(),
                document: rid.clone(),
                amount: open_recv,
                direction: Direction::In,
                date,
                foreign: foreign(receivable, open_own(facts, receivable, as_of)),
                rules: pair_rules(Direction::In),
            });
            placed.push(PendingPlacement {
                entity: payer.clone(),
                document: rid.clone(),
                amount: open_recv,
                direction: Direction::Out,
                date,
                foreign: foreign(receivable, open_own(facts, receivable, as_of)),
                rules: pair_rules(Direction::Out),
            });
            drafts.push((payer, rid.clone(), receivable, open_recv));
        }
    }

    // Payable-first: an intercompany bill with no matching invoice (IC-DOC-02 reverse).
    for (pid, payable) in facts.documents() {
        if payable.side != Side::Payable || used.contains(pid) || !open(facts, payable, as_of) {
            continue;
        }
        let Some(named) = names(classifications, &payable.counterparty) else {
            continue;
        };
        let open_pay = open_home(facts, payable, as_of);
        let Some(receiver) = named else {
            named_contacts.insert(payable.counterparty.clone());
            let date = payable
                .expected_date
                .or_else(|| due_date(payable))
                .unwrap_or(as_of);
            used.insert(pid.clone());
            placed.push(PendingPlacement {
                entity: pid.entity.clone(),
                document: pid.clone(),
                amount: open_pay,
                direction: Direction::Out,
                date,
                foreign: foreign(payable, open_own(facts, payable, as_of)),
                rules: vec![IC_MAP_01, IC_DOC_01],
            });
            continue;
        };
        if receiver == pid.entity {
            continue;
        }
        if !facts.ledger_settings().contains_key(&receiver) {
            // IC-ONESIDED-01: named but not connected — unpaired on the visible side (Q128).
            onesided_contacts.insert(payable.counterparty.clone());
            let date = payable
                .expected_date
                .or_else(|| due_date(payable))
                .unwrap_or(as_of);
            used.insert(pid.clone());
            placed.push(PendingPlacement {
                entity: pid.entity.clone(),
                document: pid.clone(),
                amount: open_pay,
                direction: Direction::Out,
                date,
                foreign: foreign(payable, open_own(facts, payable, as_of)),
                rules: vec![IC_ONESIDED_01, IC_DOC_01],
            });
            continue;
        }
        // Matched pairs were claimed above; an unmatched payable still places both legs.
        used.insert(pid.clone());
        clears_exclusion.insert(pid.clone());
        let date = payable
            .expected_date
            .or_else(|| due_date(payable))
            .unwrap_or(as_of);
        placed.push(PendingPlacement {
            entity: pid.entity.clone(),
            document: pid.clone(),
            amount: open_pay,
            direction: Direction::Out,
            date,
            foreign: foreign(payable, open_own(facts, payable, as_of)),
            rules: pair_rules(Direction::Out),
        });
        placed.push(PendingPlacement {
            entity: receiver.clone(),
            document: pid.clone(),
            amount: open_pay,
            direction: Direction::In,
            date,
            foreign: foreign(payable, open_own(facts, payable, as_of)),
            rules: pair_rules(Direction::In),
        });
        drafts.push((receiver, pid.clone(), payable, open_pay));
    }

    if !placed.is_empty() {
        // AR-OPEN-03 and AP-OPEN-03 already excluded both legs before this Rule could pair them;
        // connected counterparties replace that dead end. Unnamed / unconnected leave it
        // (AP-S05's "see IC").
        run.placements.retain(|p| {
            !(matches!(p.outcome, Outcome::Excluded(Exclusion::Intercompany))
                && matches!(&p.subject, Subject::Fact(id) if clears_exclusion.contains(id)))
        });

        for pending in placed {
            let rule = pending.rules[0];
            run.placements.push(Placement {
                subject: Subject::Intercompany {
                    entity: pending.entity,
                    document: pending.document,
                },
                rule,
                rules: pending.rules,
                outcome: Outcome::placed_on(
                    run.horizon,
                    as_of,
                    pending.date,
                    pending.direction,
                    Basis::IntercompanyDocument,
                    Confidence::Firm,
                ),
                amount: Some(pending.amount),
                history: None,
                foreign: pending.foreign,
                reductions: Vec::new(),
                priority: None,
            });
        }
    }

    for contact in named_contacts {
        run.decision_items.push(DecisionItem {
            kind: ItemKind::NameIntercompanyCounterparty,
            subject: Subject::Fact(contact),
            rule: IC_MAP_01,
            acted_on_by: Role::Bookkeeper,
            severity: Severity::Action,
            evidence: None,
            draft: None,
            due: None,
            priority: None,
        });
    }
    for contact in onesided_contacts {
        run.decision_items.push(DecisionItem {
            kind: ItemKind::ConnectIntercompanyCounterparty,
            subject: Subject::Fact(contact),
            rule: IC_ONESIDED_01,
            acted_on_by: Role::Owner,
            severity: Severity::Action,
            evidence: None,
            draft: None,
            due: None,
            priority: None,
        });
    }
    for (entity, document, source, amount) in drafts {
        let (side, sister) = match source.side {
            Side::Receivable => (Side::Payable, &document.entity),
            Side::Payable => (Side::Receivable, &document.entity),
        };
        let Some(counterparty) = sister_contact(facts, classifications, &entity, sister) else {
            continue;
        };
        run.decision_items.push(DecisionItem {
            kind: ItemKind::CounterpartyHasNotBooked,
            subject: Subject::Intercompany {
                entity: entity.clone(),
                document: document.clone(),
            },
            rule: IC_DOC_02,
            acted_on_by: Role::Bookkeeper,
            severity: Severity::Action,
            evidence: None,
            draft: Some(crate::forecast::DraftCorrection::MirrorDocument {
                side,
                counterparty,
                reference: document_reference(source),
                dated: source.date,
                due: due_date(source),
                amount,
            }),
            due: None,
            priority: None,
        });
    }

    loans(run, facts, classifications, settings);
    agree(run, facts, classifications, settings);
}

/// Bank transfers between the schedule's owing and owed Entities that cover an occurrence
/// (IC-LOAN-01, Q149).
fn loan_covered(
    facts: &Facts,
    classifications: &Classifications,
    schedule: &IntercompanySettlementSchedule,
) -> BTreeMap<NaiveDate, FactId> {
    let mut map = BTreeMap::new();
    for line in facts.account_lines().values() {
        if line.status != PostingStatus::Posted {
            continue;
        }
        let Some(counterparty) = &line.counterparty else {
            continue;
        };
        let Some(named) = names(classifications, counterparty) else {
            continue;
        };
        let Some(sister) = named else {
            continue;
        };
        let from_owing = line.id.entity == schedule.owing
            && sister == schedule.owed
            && line.amount.home.0.is_sign_negative();
        let to_owed = line.id.entity == schedule.owed
            && sister == schedule.owing
            && line.amount.home.0.is_sign_positive();
        if !(from_owing || to_owed) {
            continue;
        }
        if let Some(on) = nearest_occurrence(schedule.start, schedule.frequency, line.date) {
            map.entry(on).or_insert_with(|| line.id.clone());
        }
    }
    map
}

/// IC-LOAN-01 and IC-LOAN-02: settlement schedules place paired occurrences; uncovered balances
/// are excluded and, unless confirmed not settling, raise one dual-queue item (Q161).
fn loans(
    run: &mut ForecastRun,
    facts: &Facts,
    classifications: &Classifications,
    settings: &Settings,
) {
    let as_of = run.as_of;
    let end = run.horizon.end(as_of);

    for (id, schedule) in &settings.intercompany_settlement_schedules {
        let covered = loan_covered(facts, classifications, schedule);
        let within = |on: NaiveDate| schedule.end.is_none_or(|last| on <= last);
        let mut k = 0;
        while let Some(on) = occurrence(schedule.start, schedule.frequency, k) {
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
                        rule: IC_LOAN_01,
                        rules: vec![IC_LOAN_01],
                        outcome: Outcome::Excluded(Exclusion::CoveredBy(fact.clone())),
                        amount: Some(schedule.amount),
                        history: None,
                        foreign: None,
                        reductions: Vec::new(),
                        priority: None,
                    });
                }
                None => {
                    for (entity, direction) in [
                        (schedule.owing.clone(), Direction::Out),
                        (schedule.owed.clone(), Direction::In),
                    ] {
                        run.placements.push(Placement {
                            subject: Subject::Occurrence {
                                entity,
                                template: id.clone(),
                                date: on,
                            },
                            rule: IC_LOAN_01,
                            rules: vec![IC_LOAN_01, IC_ELIM_01],
                            outcome: Outcome::placed_on(
                                run.horizon,
                                as_of,
                                on,
                                direction,
                                Basis::IntercompanySchedule,
                                Confidence::Firm,
                            ),
                            amount: Some(schedule.amount),
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

    // Accounts already spoken for by a named settlement schedule.
    let scheduled_accounts: BTreeSet<FactId> = settings
        .intercompany_settlement_schedules
        .values()
        .flat_map(|s| s.accounts.iter().cloned())
        .collect();

    let mut plan_raised: BTreeSet<(EntityId, EntityId)> = BTreeSet::new();
    for aid in facts.accounts().keys() {
        let AccountClass::Intercompany(Some(sister)) = classifications.account(aid).class else {
            continue;
        };
        if sister == aid.entity || !facts.ledger_settings().contains_key(&sister) {
            continue;
        }
        if scheduled_accounts.contains(aid) {
            continue;
        }
        let Some(balance) = balance_at(facts, aid, as_of) else {
            continue;
        };
        if balance.0.is_zero() {
            continue;
        }
        let confirmed = settings.intercompany_not_settling.contains(aid);
        let reason = if confirmed {
            Exclusion::ConfirmedNotSettling
        } else {
            Exclusion::NoSettlementSchedule
        };
        run.placements.push(Placement {
            subject: Subject::Fact(aid.clone()),
            rule: IC_LOAN_02,
            rules: vec![IC_LOAN_02],
            outcome: Outcome::Excluded(reason),
            amount: Some(HomeAmount(balance.0.abs())),
            history: None,
            foreign: None,
            reductions: Vec::new(),
            priority: None,
        });
        if confirmed {
            continue;
        }
        let pair = if aid.entity.0 <= sister.0 {
            (aid.entity.clone(), sister.clone())
        } else {
            (sister.clone(), aid.entity.clone())
        };
        if !plan_raised.insert(pair.clone()) {
            continue;
        }
        // Prefer the liability-side unscheduled account (negative balance) as the Subject.
        let subject_account = facts
            .accounts()
            .keys()
            .filter(|other| !scheduled_accounts.contains(*other))
            .filter(|other| {
                matches!(
                    classifications.account(other).class,
                    AccountClass::Intercompany(Some(s))
                        if (other.entity == pair.0 && s == pair.1)
                            || (other.entity == pair.1 && s == pair.0)
                )
            })
            .filter(|other| !settings.intercompany_not_settling.contains(*other))
            .min_by_key(|other| {
                let bal = balance_at(facts, other, as_of)
                    .map(|b| b.0)
                    .unwrap_or_default();
                (
                    u8::from(!bal.is_sign_negative()),
                    // Prefer Maple Ridge's books when both sides are liabilities of equal rank.
                    u8::from(!other.entity.0.starts_with("Maple")),
                    other.entity.0.clone(),
                    other.provider_id.clone(),
                )
            })
            .cloned()
            .unwrap_or_else(|| aid.clone());
        run.decision_items.push(DecisionItem {
            kind: ItemKind::IntercompanySettlementPlan,
            subject: Subject::Fact(subject_account),
            rule: IC_LOAN_02,
            acted_on_by: Role::Owner,
            severity: Severity::Action,
            evidence: None,
            draft: None,
            due: None,
            priority: None,
        });
    }
}

/// Last day of the month before `as_of`'s month (IC-AGREE-01: end of the last completed month).
fn last_completed_month_end(as_of: NaiveDate) -> Option<NaiveDate> {
    let first_of_month = NaiveDate::from_ymd_opt(as_of.year(), as_of.month(), 1)?;
    first_of_month.pred_opt()
}

/// Net intercompany position of `entity` with `sister` at `on`: debit-positive, so a positive
/// net means `sister` owes `entity`.
fn net_with(
    facts: &Facts,
    classifications: &Classifications,
    entity: &EntityId,
    sister: &EntityId,
    on: NaiveDate,
) -> HomeAmount {
    facts
        .accounts()
        .keys()
        .filter(|id| id.entity == *entity)
        .filter(|id| {
            matches!(
                classifications.account(id).class,
                AccountClass::Intercompany(Some(s)) if s == *sister
            )
        })
        .filter_map(|id| balance_at(facts, id, on))
        .fold(HomeAmount(Decimal::ZERO), |acc, b| {
            HomeAmount(acc.0.saturating_add(b.0))
        })
}

/// Convert an Entity's Home Currency amount into the Group's Reporting Currency at the owner's
/// rate Setting. Same currency is identity; missing rate means the pair cannot be agreed
/// (CASH-GROUP-02 covers the missing Setting).
fn to_reporting(
    facts: &Facts,
    settings: &Settings,
    entity: &EntityId,
    amount: HomeAmount,
) -> Option<(HomeAmount, Option<(Currency, Decimal)>)> {
    let home = facts.ledger_settings().get(entity)?.home_currency;
    let Some(reporting) = settings.reporting_currency else {
        return Some((amount, None));
    };
    if home == reporting {
        return Some((amount, None));
    }
    if let Some(rate) = settings.conversion_rates.get(&(home, reporting)) {
        return Some((
            HomeAmount(round_money(amount.0.saturating_mul(*rate))),
            Some((home, *rate)),
        ));
    }
    if let Some(rate) = settings.conversion_rates.get(&(reporting, home)) {
        if rate.is_zero() {
            return None;
        }
        return Some((
            HomeAmount(round_money(amount.0.checked_div(*rate).unwrap_or(amount.0))),
            Some((home, *rate)),
        ));
    }
    None
}

/// IC-AGREE-01 and IC-AGREE-02: reciprocal balances at the end of the last completed month.
/// Same-currency disagreement marks Provisional (tolerance optional); cross-currency difference
/// raises an action item and does not (Q124, Q125).
fn agree(
    run: &mut ForecastRun,
    facts: &Facts,
    classifications: &Classifications,
    settings: &Settings,
) {
    let Some(month_end) = last_completed_month_end(run.as_of) else {
        return;
    };
    let month = NaiveDate::from_ymd_opt(month_end.year(), month_end.month(), 1);
    let entities: Vec<EntityId> = facts.ledger_settings().keys().cloned().collect();
    let mut seen: BTreeSet<(EntityId, EntityId)> = BTreeSet::new();
    for a in &entities {
        for b in &entities {
            if a.0 >= b.0 {
                continue;
            }
            let Some(a_home) = facts.ledger_settings().get(a).map(|s| s.home_currency) else {
                continue;
            };
            let Some(b_home) = facts.ledger_settings().get(b).map(|s| s.home_currency) else {
                continue;
            };
            let pair = (a.clone(), b.clone());
            if !seen.insert(pair.clone()) {
                continue;
            }
            let a_net = net_with(facts, classifications, a, b, month_end);
            let b_net = net_with(facts, classifications, b, a, month_end);
            if a_net.0.is_zero() && b_net.0.is_zero() {
                continue;
            }
            if a_home == b_home {
                let difference = HomeAmount((a_net.0.saturating_add(b_net.0)).abs());
                let within = settings
                    .intercompany_tolerance
                    .is_some_and(|t| difference.0 <= t.0)
                    || difference.0.is_zero();
                if within {
                    continue;
                }
                // Put the side owed money first so evidence reads "… owed to it; … owed" and the
                // Subject's first word matches the document's short-name order (IC-S04).
                let (first, second, first_net, second_net) = if a_net.0 >= b_net.0 {
                    (a.clone(), b.clone(), a_net, b_net)
                } else {
                    (b.clone(), a.clone(), b_net, a_net)
                };
                run.decision_items.push(DecisionItem {
                    kind: ItemKind::IntercompanyBalancesDisagree,
                    subject: Subject::EntityPair {
                        a: first.clone(),
                        b: second.clone(),
                        month,
                    },
                    rule: IC_AGREE_01,
                    acted_on_by: Role::Bookkeeper,
                    severity: Severity::Blocking,
                    evidence: Some(Evidence::IntercompanyBalanceDisagreement {
                        a: first.clone(),
                        a_amount: HomeAmount(first_net.0.abs()),
                        a_owed_to_it: first_net.0.is_sign_positive() && !first_net.0.is_zero(),
                        b: second.clone(),
                        b_amount: HomeAmount(second_net.0.abs()),
                        difference,
                    }),
                    draft: None,
                    due: None,
                    priority: None,
                });
                run.provisional.push(ProvisionalReason {
                    rule: IC_AGREE_01,
                    subject: Subject::EntityPair {
                        a: first,
                        b: second,
                        month,
                    },
                });
                continue;
            }
            // IC-AGREE-02: compare in Reporting Currency; a difference explains, never blocks.
            let Some((a_rep, a_fx)) = to_reporting(facts, settings, a, a_net) else {
                continue;
            };
            let Some((b_rep, b_fx)) = to_reporting(facts, settings, b, b_net) else {
                continue;
            };
            let difference = HomeAmount((a_rep.0.saturating_add(b_rep.0)).abs());
            if difference.0.is_zero() {
                continue;
            }
            // Reporting-currency Entity leads the Subject (IC-S05: Maple Ridge and Cascade);
            // evidence names the foreign side first with its conversion.
            let (home_entity, home_amount, foreign_entity, foreign_net, rate_info) =
                match (a_fx, b_fx) {
                    (None, Some((currency, rate))) => {
                        (a.clone(), a_net, b.clone(), b_net, (currency, rate))
                    }
                    (Some((currency, rate)), None) => {
                        (b.clone(), b_net, a.clone(), a_net, (currency, rate))
                    }
                    // Both foreign to Reporting Currency: keep lexical Entity order for Subject.
                    (Some((currency, rate)), Some(_)) => {
                        (a.clone(), a_net, b.clone(), b_net, (currency, rate))
                    }
                    (None, None) => continue,
                };
            let converted = to_reporting(facts, settings, &foreign_entity, foreign_net)
                .map(|(v, _)| HomeAmount(v.0.abs()))
                .unwrap_or(HomeAmount(foreign_net.0.abs()));
            run.decision_items.push(DecisionItem {
                kind: ItemKind::IntercompanyCurrencyDifference,
                subject: Subject::EntityPair {
                    a: home_entity.clone(),
                    b: foreign_entity.clone(),
                    month,
                },
                rule: IC_AGREE_02,
                acted_on_by: Role::Accountant,
                severity: Severity::Action,
                evidence: Some(Evidence::IntercompanyCurrencyMovement {
                    foreign_entity,
                    foreign_amount: ForeignAmount {
                        currency: rate_info.0,
                        amount: foreign_net.0.abs(),
                        rate: rate_info.1,
                    },
                    converted,
                    home_entity,
                    home_amount: HomeAmount(home_amount.0.abs()),
                    difference,
                }),
                draft: None,
                due: None,
                priority: None,
            });
        }
    }
}

/// Whether an Entity's own weeks stay at or above its Minimum Cash Buffer (or zero, with none
/// set) from a week to the end of the horizon, after giving up `amount` in the transfer week
/// (IC-FUND-01). `amount` is already in the helper Entity's Home Currency (Q163 converts first).
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

/// Convert `needed` from the short Entity's Home Currency into the helper's, at the owner's rate
/// Setting when the currencies differ (Q163). Same currency is identity; missing rate means the
/// helper cannot be tested. Cross-currency results are rounded to the minor unit (Q182).
fn needed_in_helper(
    facts: &Facts,
    settings: &Settings,
    short: &EntityId,
    helper: &EntityId,
    needed: Decimal,
) -> Option<Decimal> {
    let short_home = facts.ledger_settings().get(short)?.home_currency;
    let helper_home = facts.ledger_settings().get(helper)?.home_currency;
    if short_home == helper_home {
        return Some(needed);
    }
    // Prefer short→helper; else invert helper→short.
    if let Some(rate) = settings.conversion_rates.get(&(short_home, helper_home)) {
        return Some(round_money(needed * *rate));
    }
    if let Some(rate) = settings.conversion_rates.get(&(helper_home, short_home)) {
        if rate.is_zero() {
            return None;
        }
        return Some(round_money(needed / *rate));
    }
    None
}

/// IC-FUND-01: for each Entity with a cash-shortfall item, name each other connected Entity that
/// could transfer enough to cover the deepest week without itself falling below zero or its own
/// buffer, from that week to the end of the horizon. Cross-currency helpers convert at the Group
/// rate Setting (Q163). Evidence lists the helpers and the Rule's fixed caveats (IC-S07).
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
        let helpers: Vec<crate::forecast::FundingHelper> = run
            .entities
            .iter()
            .filter(|other| other.entity != entity)
            .filter_map(|other| {
                let in_helper = needed_in_helper(facts, settings, &entity, &other.entity, needed)?;
                could_cover(
                    other,
                    week,
                    in_helper,
                    settings.minimum_cash_buffer.get(&other.entity).copied(),
                )
                .then_some(crate::forecast::FundingHelper {
                    entity: other.entity.clone(),
                    amount: HomeAmount(in_helper),
                    week,
                })
            })
            .collect();
        if !helpers.is_empty() {
            run.decision_items.push(DecisionItem {
                kind: ItemKind::ConsiderIntercompanyFunding,
                subject: Subject::Entity(entity),
                rule: IC_FUND_01,
                acted_on_by: Role::Owner,
                severity: Severity::Action,
                evidence: Some(Evidence::IntercompanyFunding { helpers }),
                draft: None,
                due: None,
                priority: None,
            });
        }
    }
}
