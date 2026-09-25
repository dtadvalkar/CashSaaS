//! The AR Family, mirroring `docs/rules/ar.md`. Where they disagree, this file is wrong.

use std::collections::BTreeMap;

use chrono::{Days, NaiveDate};
use rust_decimal::Decimal;

use crate::document::{
    age, balance_at, due_date, foreign, intercompany, last_completed_month, open_home, open_own,
    to_home, top_level, unapplied,
};
use crate::facts::{
    ApplicationFrom, Direction, Document, DocumentKind, DocumentStatus, EntityId, FactId, Facts,
    PaymentPurpose, PostingStatus, ScheduledTemplate, Side, TemplateMode,
};
use crate::forecast::{
    Basis, Confidence, DecisionItem, DraftCorrection, Evidence, Exclusion, ForecastRun, History,
    ItemKind, Outcome, Placement, ProvisionalReason, Reduction, Role, RuleId, Severity, Subject,
};
use crate::money::HomeAmount;
use crate::schedule::{due_by, index_at_or_before, nearest_occurrence, occurrence};
use crate::settings::{AccountClass, Classifications, Settings};

pub(crate) const AR_OPEN_01: RuleId = RuleId("AR-OPEN-01");
pub(crate) const AR_OPEN_02: RuleId = RuleId("AR-OPEN-02");
pub(crate) const AR_OPEN_03: RuleId = RuleId("AR-OPEN-03");
pub(crate) const AR_OPEN_04: RuleId = RuleId("AR-OPEN-04");
pub(crate) const AR_TIME_01: RuleId = RuleId("AR-TIME-01");
pub(crate) const AR_TIME_02: RuleId = RuleId("AR-TIME-02");
pub(crate) const AR_TIME_03: RuleId = RuleId("AR-TIME-03");
pub(crate) const AR_TIME_04: RuleId = RuleId("AR-TIME-04");
pub(crate) const AR_SCHED_01: RuleId = RuleId("AR-SCHED-01");
pub(crate) const AR_SCHED_02: RuleId = RuleId("AR-SCHED-02");
pub(crate) const AR_FX_01: RuleId = RuleId("AR-FX-01");
pub(crate) const AR_DISC_01: RuleId = RuleId("AR-DISC-01");
pub(crate) const AR_UNAPPLIED_01: RuleId = RuleId("AR-UNAPPLIED-01");
pub(crate) const AR_UNAPPLIED_02: RuleId = RuleId("AR-UNAPPLIED-02");
pub(crate) const AR_COLLECT_01: RuleId = RuleId("AR-COLLECT-01");
pub(crate) const AR_WRITEOFF_01: RuleId = RuleId("AR-WRITEOFF-01");
pub(crate) const AR_UNBILLED_01: RuleId = RuleId("AR-UNBILLED-01");
pub(crate) const AR_TIE_01: RuleId = RuleId("AR-TIE-01");

/// The date an invoice was collected: when its applications first covered its total, provided a
/// payment was among them. Closed by credit alone is never collected (AR-TIME-03, Q77).
pub fn collected_on(facts: &Facts, document: &Document, as_of: NaiveDate) -> Option<NaiveDate> {
    let mut applications: Vec<_> = facts
        .applications()
        .values()
        .filter(|a| a.to == document.id && a.date <= as_of)
        .collect();
    applications.sort_by_key(|a| a.date);
    let total = document.total.in_own_currency().1;
    let mut applied = Decimal::ZERO;
    let mut paid = false;
    for application in applications {
        applied = applied.saturating_add(application.amount.in_own_currency().1);
        paid |= matches!(application.from, ApplicationFrom::Payment(_));
        if applied >= total {
            return paid.then_some(application.date);
        }
    }
    None
}

/// Unapplied customer credit: a posted customer payment or credit note of the Entity with an
/// unapplied remainder on the run date (AR-UNAPPLIED-01, Q251).
struct Credit {
    id: FactId,
    customer: FactId,
    date: NaiveDate,
    remainder: Decimal,
}

fn credits(facts: &Facts, entity: &EntityId, as_of: NaiveDate) -> Vec<Credit> {
    let payments = facts
        .payments()
        .values()
        .filter(|p| p.id.entity == *entity && p.status == PostingStatus::Posted)
        .filter(|p| p.direction == Direction::In && p.date <= as_of)
        .filter_map(|p| match &p.purpose {
            PaymentPurpose::Customer { counterparty } => Some((&p.id, counterparty, p.date)),
            _ => None,
        });
    let notes = facts
        .documents()
        .values()
        .filter(|d| d.id.entity == *entity && d.side == Side::Receivable)
        .filter(|d| d.kind == DocumentKind::CreditNote && d.status == DocumentStatus::Posted)
        .filter(|d| d.date <= as_of)
        .map(|d| (&d.id, &d.counterparty, d.date));
    let mut credits: Vec<Credit> = payments
        .chain(notes)
        .filter_map(|(id, customer, date)| {
            let remainder = unapplied(facts, id, as_of)?.0;
            (remainder > Decimal::ZERO).then(|| Credit {
                id: id.clone(),
                customer: top_level(facts, customer),
                date,
                remainder,
            })
        })
        .collect();
    credits.sort_by(|a, b| (a.date, &a.id).cmp(&(b.date, &b.id)));
    credits
}

/// The scheduled dates the Entity's invoices cover, each with the invoice covering it (AR-SCHED-01,
/// Q144, Q149): invoices linked to the template where any are, otherwise unlinked invoices to the
/// same customer. An invoice that exists in any state but voided or deleted covers its occurrence.
fn covered(facts: &Facts, template: &ScheduledTemplate) -> BTreeMap<NaiveDate, FactId> {
    let invoices: Vec<&Document> = facts
        .documents()
        .values()
        .filter(|d| d.id.entity == template.id.entity && d.side == Side::Receivable)
        .filter(|d| d.kind == DocumentKind::Invoice)
        .filter(|d| !matches!(d.status, DocumentStatus::Voided | DocumentStatus::Deleted))
        .collect();
    let linked: Vec<&Document> = invoices
        .iter()
        .copied()
        .filter(|d| d.template.as_ref() == Some(&template.id))
        .collect();
    let covering = if linked.is_empty() {
        let customer = top_level(facts, &template.counterparty);
        invoices
            .into_iter()
            .filter(|d| d.template.is_none() && top_level(facts, &d.counterparty) == customer)
            .collect()
    } else {
        linked
    };
    let mut map = BTreeMap::new();
    for document in covering {
        if let Some(on) = nearest_occurrence(template.start, template.frequency, document.date) {
            map.entry(on).or_insert_with(|| document.id.clone());
        }
    }
    map
}

/// A receivable-side invoice with a non-zero total that is posted: everything the AR Rules read
/// as history or as a receivable (Q145).
fn counts(document: &Document) -> bool {
    document.side == Side::Receivable
        && document.kind == DocumentKind::Invoice
        && document.status == DocumentStatus::Posted
        && !document.total.in_own_currency().1.is_zero()
}

/// One invoice of the Entity's history: when it fell due and when, if ever, it was collected.
struct Historical {
    id: FactId,
    customer: FactId,
    due: NaiveDate,
    collected: Option<NaiveDate>,
    /// Closed with no payment, by credit alone: never collected (AR-TIME-03, Q77).
    never_collected: bool,
}

/// The median collection delay for an invoice at an age (AR-TIME-02, AR-TIME-03, Q147): the first
/// number of days after that age by which at least half the comparable invoices were collected.
/// An invoice still open counts as not collected at every point (Q147), but only one closed with
/// no payment was never collected. With no median and no never-collected majority, the history is
/// inconclusive and `None`, as when no invoice is comparable.
fn history_at(
    history: &[Historical],
    subject: Option<&FactId>,
    customer: Option<&FactId>,
    age: i64,
    as_of: NaiveDate,
) -> Option<History> {
    let mut delays = Vec::new();
    let mut comparable = 0;
    let mut never_collected = 0;
    for h in history {
        if subject == Some(&h.id) || customer.is_some_and(|c| *c != h.customer) {
            continue;
        }
        let reached = crate::document::age(as_of, h.due) >= age;
        let unpaid_at_age = h
            .collected
            .is_none_or(|c| crate::document::age(c, h.due) > age);
        if !(reached && unpaid_at_age) {
            continue;
        }
        comparable += 1;
        never_collected += usize::from(h.never_collected);
        if let Some(c) = h.collected {
            delays.push(crate::document::age(c, h.due) - age);
        }
    }
    delays.sort_unstable();
    let median_days = delays
        .iter()
        .enumerate()
        .find(|(i, _)| (i + 1) * 2 >= comparable)
        .map(|(_, d)| *d);
    if median_days.is_none() && never_collected * 2 <= comparable {
        return None;
    }
    Some(History {
        median_days,
        comparable,
        never_collected,
    })
}

/// An open, posted, non-intercompany invoice waiting to be timed.
struct Candidate<'a> {
    document: &'a Document,
    customer: FactId,
    due: NaiveDate,
    /// Net of the credit in `reductions`.
    open: HomeAmount,
    reductions: Vec<Reduction>,
}

fn subject(document: &Document) -> Subject {
    Subject::Fact(document.id.clone())
}

fn item(kind: ItemKind, subject: Subject, rule: RuleId, role: Role) -> DecisionItem {
    DecisionItem {
        kind,
        subject,
        rule,
        acted_on_by: role,
        severity: Severity::Action,
        evidence: None,
        draft: None,
        due: None,
        priority: None,
    }
}

/// Runs every AR Rule for one Entity.
pub fn run(
    run: &mut ForecastRun,
    facts: &Facts,
    settings: &Settings,
    classifications: &Classifications,
    entity: &EntityId,
) {
    let as_of = run.as_of;
    let documents = facts
        .documents()
        .values()
        .filter(|d| d.id.entity == *entity && d.side == Side::Receivable);

    let mut history = Vec::new();
    let mut candidates = Vec::new();
    for document in documents.clone() {
        if document.kind != DocumentKind::Invoice {
            continue;
        }
        let Some(due) = due_date(document) else {
            continue;
        };
        match document.status {
            DocumentStatus::Voided | DocumentStatus::Deleted => continue,
            DocumentStatus::Draft | DocumentStatus::AwaitingApproval => {
                run.placements.push(Placement {
                    subject: subject(document),
                    rule: AR_OPEN_02,
                    rules: vec![AR_OPEN_02],
                    outcome: Outcome::Excluded(Exclusion::NotIssued),
                    amount: Some(open_home(facts, document, as_of)),
                    history: None,
                    foreign: None,
                    reductions: Vec::new(),
                    priority: None,
                });
                continue;
            }
            DocumentStatus::Posted => {}
        }
        if !counts(document) {
            continue;
        }
        let own = open_own(facts, document, as_of);
        let collected = collected_on(facts, document, as_of);
        history.push(Historical {
            id: document.id.clone(),
            customer: top_level(facts, &document.counterparty),
            due,
            collected,
            never_collected: collected.is_none() && own <= Decimal::ZERO,
        });
        if own <= Decimal::ZERO {
            continue;
        }
        let open = to_home(document, own);
        if intercompany(facts, classifications, &document.counterparty) {
            run.placements.push(Placement {
                subject: subject(document),
                rule: AR_OPEN_03,
                rules: vec![AR_OPEN_03, AR_OPEN_01],
                outcome: Outcome::Excluded(Exclusion::Intercompany),
                amount: Some(open),
                history: None,
                foreign: foreign(document, own),
                reductions: Vec::new(),
                priority: None,
            });
            continue;
        }
        candidates.push(Candidate {
            document,
            customer: top_level(facts, &document.counterparty),
            due,
            open,
            reductions: Vec::new(),
        });
    }

    let candidates = net_credits(run, facts, entity, candidates);

    for candidate in &candidates {
        time(run, facts, settings, classifications, &history, candidate);
    }

    schedules(run, facts, settings, &history, entity);
    month_end(run, facts, classifications, entity);

    // AR-COLLECT-01: overdue invoices, ordered by open amount.
    let mut overdue: Vec<_> = candidates
        .iter()
        .filter(|c| age(as_of, c.due) > 0)
        .collect();
    overdue.sort_by_key(|c| std::cmp::Reverse(c.open));
    for candidate in overdue {
        run.decision_items.push(item(
            ItemKind::Collect,
            subject(candidate.document),
            AR_COLLECT_01,
            Role::Owner,
        ));
    }
}

/// AR-UNAPPLIED-01 and AR-UNAPPLIED-02: each credit, oldest first, reduces its customer's open
/// invoices, earliest due first: every Scenario states a due date, so no builder default decides
/// the order. An invoice reduced to nothing is excluded (Q148) and is no longer a candidate;
/// credit for a customer with no open invoice is a credit balance. Credit left over once a
/// customer's invoices are covered is not forecast and raises nothing.
fn net_credits<'a>(
    run: &mut ForecastRun,
    facts: &Facts,
    entity: &EntityId,
    mut candidates: Vec<Candidate<'a>>,
) -> Vec<Candidate<'a>> {
    let as_of = run.as_of;
    for credit in credits(facts, entity, as_of) {
        let mut order: Vec<usize> = (0..candidates.len())
            .filter(|&i| candidates[i].customer == credit.customer)
            .filter(|&i| candidates[i].open.0 > Decimal::ZERO)
            .collect();
        order.sort_by_key(|&i| (candidates[i].due, candidates[i].document.id.clone()));
        if order.is_empty() {
            run.placements.push(Placement {
                subject: Subject::Fact(credit.id.clone()),
                rule: AR_UNAPPLIED_02,
                rules: vec![AR_UNAPPLIED_02],
                outcome: Outcome::Excluded(Exclusion::CreditBalance),
                amount: Some(HomeAmount(credit.remainder)),
                history: None,
                foreign: None,
                reductions: Vec::new(),
                priority: None,
            });
            run.decision_items.push(item(
                ItemKind::RefundOrApplyCredit,
                Subject::Fact(credit.id),
                AR_UNAPPLIED_02,
                Role::Owner,
            ));
            continue;
        }
        let mut remaining = credit.remainder;
        let mut to = Vec::new();
        for i in order {
            if remaining <= Decimal::ZERO {
                break;
            }
            let candidate = &mut candidates[i];
            let take = remaining.min(candidate.open.0);
            candidate.open.0 = candidate.open.0.saturating_sub(take);
            candidate.reductions.push(Reduction {
                credit: credit.id.clone(),
                amount: take,
                remainder: !to.is_empty(),
            });
            to.push((candidate.document.id.clone(), take));
            remaining = remaining.saturating_sub(take);
        }
        let mut item = item(
            ItemKind::ApplyCredit,
            Subject::Fact(credit.id.clone()),
            AR_UNAPPLIED_01,
            Role::Bookkeeper,
        );
        item.draft = Some(DraftCorrection::PaymentApplication {
            from: credit.id,
            to,
        });
        run.decision_items.push(item);
    }

    let (offset, open): (Vec<_>, Vec<_>) = candidates
        .into_iter()
        .partition(|c| c.open.0 <= Decimal::ZERO);
    for candidate in offset {
        let Some(last) = candidate.reductions.last() else {
            continue;
        };
        let gross = candidate
            .reductions
            .iter()
            .fold(Decimal::ZERO, |g, r| g.saturating_add(r.amount));
        run.placements.push(Placement {
            subject: subject(candidate.document),
            rule: AR_UNAPPLIED_01,
            rules: vec![AR_UNAPPLIED_01, AR_OPEN_01],
            outcome: Outcome::Excluded(Exclusion::OffsetByCredit(last.credit.clone())),
            amount: Some(HomeAmount(gross)),
            history: None,
            foreign: None,
            reductions: Vec::new(),
            priority: None,
        });
    }
    open
}

/// AR-UNBILLED-01 and AR-TIE-01 at the end of the last completed month. Each reads a balance fact
/// dated on that day for an account classified for it; with no such account, or no balance fact
/// for it, there is nothing to read and the Rule is silent.
fn month_end(
    run: &mut ForecastRun,
    facts: &Facts,
    classifications: &Classifications,
    entity: &EntityId,
) {
    let Some((month, end)) = last_completed_month(run.as_of) else {
        return;
    };

    // AR-UNBILLED-01: a balance in an unbilled-receivable account is revenue to invoice.
    for account in classifications.accounts_classed(entity, &AccountClass::UnbilledReceivable) {
        let Some(balance) = balance_at(facts, account, end).filter(|b| !b.0.is_zero()) else {
            continue;
        };
        let mut item = item(
            ItemKind::InvoiceAccruedRevenue,
            Subject::AccountMonth {
                account: account.clone(),
                month,
            },
            AR_UNBILLED_01,
            Role::Bookkeeper,
        );
        item.evidence = Some(Evidence::Balance(balance));
        run.decision_items.push(item);
    }

    // AR-TIE-01: open invoices less unapplied credit, both at the month end, against the control
    // balance. Any difference is an item to reconcile and makes the run Provisional.
    let controls: Vec<(&FactId, HomeAmount)> = classifications
        .accounts_classed(entity, &AccountClass::ArControl)
        .filter_map(|a| balance_at(facts, a, end).map(|b| (a, b)))
        .collect();
    let Some((first, _)) = controls.first() else {
        return;
    };
    let control = controls
        .iter()
        .fold(Decimal::ZERO, |sum, (_, b)| sum.saturating_add(b.0));
    let open = facts
        .documents()
        .values()
        .filter(|d| d.id.entity == *entity && counts(d) && d.date <= end)
        .map(|d| open_home(facts, d, end).0)
        .fold(Decimal::ZERO, Decimal::saturating_add);
    let credit = credits(facts, entity, end)
        .iter()
        .fold(Decimal::ZERO, |sum, c| sum.saturating_add(c.remainder));
    let subledger = open.saturating_sub(credit);
    let difference = control.saturating_sub(subledger);
    if difference.is_zero() {
        return;
    }
    // ponytail: the item names the first control account; a Group with several gets one item.
    let subject = Subject::AccountMonth {
        account: (*first).clone(),
        month,
    };
    // Severity is CASH-ORDER-01's: it always derives every item's class, blocking included,
    // from what marked the run Provisional, so it is not set here (checkpoint B).
    let mut item = item(
        ItemKind::ReconcileAr,
        subject.clone(),
        AR_TIE_01,
        Role::Bookkeeper,
    );
    item.evidence = Some(Evidence::ControlDifference {
        open: HomeAmount(subledger),
        control: HomeAmount(control),
        difference: HomeAmount(difference),
    });
    run.decision_items.push(item);
    run.provisional.push(ProvisionalReason {
        rule: AR_TIE_01,
        subject,
    });
}

/// AR-SCHED-01 and AR-SCHED-02 for the Entity's scheduled invoice templates.
fn schedules(
    run: &mut ForecastRun,
    facts: &Facts,
    settings: &Settings,
    history: &[Historical],
    entity: &EntityId,
) {
    let as_of = run.as_of;
    let end = run.horizon.end(as_of);
    let templates = facts
        .scheduled_templates()
        .values()
        .filter(|t| t.id.entity == *entity && t.side == Side::Receivable);
    for template in templates {
        let not_automatic = |run: &mut ForecastRun| {
            run.placements.push(Placement {
                subject: Subject::Fact(template.id.clone()),
                rule: AR_SCHED_01,
                rules: vec![AR_SCHED_01],
                outcome: Outcome::Excluded(Exclusion::TemplateNotAutomatic),
                amount: None,
                history: None,
                foreign: None,
                reductions: Vec::new(),
                priority: None,
            });
        };
        if !template.active {
            not_automatic(run);
            continue;
        }
        let covered = covered(facts, template);
        let within = |on: NaiveDate| template.end.is_none_or(|e| on <= e);

        // AR-SCHED-02: the most recent scheduled date on or before the run date, once the schedule
        // has begun, is missed when no invoice covers it.
        let last =
            index_at_or_before(template.start, template.frequency, as_of).filter(|k| *k >= 0);
        if let Some(on) = last.and_then(|k| occurrence(template.start, template.frequency, k))
            && within(on)
            && !covered.contains_key(&on)
        {
            run.decision_items.push(item(
                ItemKind::InvoiceMissed,
                Subject::Occurrence {
                    entity: template.id.entity.clone(),
                    template: template.id.clone(),
                    date: on,
                },
                AR_SCHED_02,
                Role::Bookkeeper,
            ));
        }

        if template.mode != TemplateMode::Automatic {
            not_automatic(run);
            continue;
        }

        // AR-SCHED-01: one expected invoice per scheduled date after the run date to the end of
        // the Horizon, unless covered. The date on or before the run date is AR-SCHED-02's alone:
        // no receipt is forecast for it until its invoice exists.
        let mut k =
            index_at_or_before(template.start, template.frequency, as_of).map_or(0, |k| k + 1);
        k = k.max(0);
        while let Some(on) =
            occurrence(template.start, template.frequency, k).filter(|on| *on < end && within(*on))
        {
            let subject = Subject::Occurrence {
                entity: template.id.entity.clone(),
                template: template.id.clone(),
                date: on,
            };
            if let Some(invoice) = covered.get(&on) {
                run.placements.push(Placement {
                    subject,
                    rule: AR_SCHED_01,
                    rules: vec![AR_SCHED_01],
                    outcome: Outcome::Excluded(Exclusion::CoveredBy(invoice.clone())),
                    amount: Some(template.amount.home),
                    history: None,
                    foreign: None,
                    reductions: Vec::new(),
                    priority: None,
                });
            } else {
                time_occurrence(run, facts, settings, history, template, on, subject);
            }
            k += 1;
        }
    }
}

/// An uncovered occurrence, timed as if open on its scheduled invoice date (Q151): its age is
/// taken on that date and the median counted from it; with no history, its scheduled due date.
fn time_occurrence(
    run: &mut ForecastRun,
    facts: &Facts,
    settings: &Settings,
    history: &[Historical],
    template: &ScheduledTemplate,
    on: NaiveDate,
    subject: Subject,
) {
    let as_of = run.as_of;
    let Some(due) = due_by(template.due_rule, on) else {
        return;
    };
    let customer = top_level(facts, &template.counterparty);
    let customer = settings
        .distinct_customers
        .contains(&customer)
        .then_some(&customer);
    let observed = history_at(history, None, customer, age(on, due), as_of);
    let placement = |rules: Vec<RuleId>, outcome: Outcome, history: Option<History>| Placement {
        subject,
        rule: AR_SCHED_01,
        rules,
        outcome,
        amount: Some(template.amount.home),
        history,
        foreign: None,
        reductions: Vec::new(),
        priority: None,
    };
    let placed = |date: NaiveDate| {
        Outcome::placed_on(
            run.horizon,
            as_of,
            date,
            Direction::In,
            Basis::ScheduledInvoice,
            Confidence::Estimated,
        )
    };
    let placement = match observed {
        Some(
            observed @ History {
                median_days: Some(median),
                ..
            },
        ) => {
            let outcome = placed(on + Days::new(median.unsigned_abs()));
            let timing = if matches!(outcome, Outcome::Placed { .. }) {
                AR_TIME_02
            } else {
                AR_TIME_03
            };
            placement(vec![AR_SCHED_01, timing], outcome, Some(observed))
        }
        Some(uncollected) => placement(
            vec![AR_SCHED_01, AR_TIME_03],
            Outcome::Excluded(Exclusion::HistoryUncollected),
            Some(uncollected),
        ),
        None => {
            let outcome = placed(due);
            let rules = if matches!(outcome, Outcome::Placed { .. }) {
                vec![AR_SCHED_01]
            } else {
                vec![AR_SCHED_01, AR_TIME_03]
            };
            placement(rules, outcome, None)
        }
    };
    run.placements.push(placement);
}

/// AR-TIME-01 to AR-TIME-04 for one open invoice.
fn time(
    run: &mut ForecastRun,
    facts: &Facts,
    settings: &Settings,
    classifications: &Classifications,
    history: &[Historical],
    candidate: &Candidate<'_>,
) {
    let as_of = run.as_of;
    let document = candidate.document;
    let own = open_own(facts, document, as_of);
    let mut rules = vec![AR_OPEN_01, AR_OPEN_04];
    if document.total.transaction.is_some() {
        rules.push(AR_FX_01);
    }
    if !candidate.reductions.is_empty() {
        rules.push(AR_UNAPPLIED_01);
    }
    if document
        .terms
        .as_ref()
        .and_then(|t| facts.payment_terms().get(t))
        .is_some_and(|t| t.discount.is_some())
    {
        rules.push(AR_DISC_01);
    }
    let placement = |rule: RuleId, outcome: Outcome, history: Option<History>| Placement {
        subject: subject(document),
        rule,
        rules: std::iter::once(rule).chain(rules.iter().copied()).collect(),
        outcome,
        amount: Some(candidate.open),
        history,
        foreign: foreign(document, own),
        reductions: candidate.reductions.clone(),
        priority: None,
    };
    let placed = |date: NaiveDate, basis: Basis, confidence: Confidence| {
        Outcome::placed_on(run.horizon, as_of, date, Direction::In, basis, confidence)
    };

    // AR-TIME-01: an expected date on or after the run date places the invoice.
    let expected = document.expected_date;
    if let Some(date) = expected.filter(|d| *d >= as_of) {
        run.placements.push(placement(
            AR_TIME_01,
            placed(date, Basis::ExpectedDate, Confidence::Firm),
            None,
        ));
        return;
    }

    // AR-TIME-02: the Entity's own collection history, or the customer's where marked.
    let age = age(as_of, candidate.due);
    let customer = top_level(facts, &document.counterparty);
    let (basis, customer) = if settings.distinct_customers.contains(&customer) {
        (Basis::CustomerHistory, Some(&customer))
    } else {
        (Basis::EntityHistory, None)
    };
    let observed = history_at(history, Some(&document.id), customer, age, as_of);
    let subject = subject(document);
    match observed {
        Some(History {
            median_days: Some(median),
            comparable,
            never_collected,
        }) => {
            let date = as_of + chrono::Days::new(median.unsigned_abs());
            let outcome = placed(date, basis, Confidence::Estimated);
            let rule = if matches!(outcome, Outcome::Placed { .. }) {
                AR_TIME_02
            } else {
                AR_TIME_03
            };
            run.placements.push(placement(
                rule,
                outcome,
                Some(History {
                    median_days: Some(median),
                    comparable,
                    never_collected,
                }),
            ));
            if let Some(passed) = expected {
                let mut item = item(
                    ItemKind::ExpectedDatePassed,
                    subject,
                    AR_TIME_01,
                    Role::Owner,
                );
                item.evidence = Some(Evidence::ExpectedDatePassed(passed));
                run.decision_items.push(item);
            }
        }
        // AR-TIME-03: more than half the comparable invoices were never collected.
        // AR-WRITEOFF-01: propose the write-off; the account is never guessed (ADR-0019).
        Some(uncollected) => {
            run.placements.push(placement(
                AR_TIME_03,
                Outcome::Excluded(Exclusion::HistoryUncollected),
                Some(uncollected),
            ));
            let mut item = item(
                ItemKind::ReviewForWriteOff,
                subject,
                AR_WRITEOFF_01,
                Role::Accountant,
            );
            item.draft = Some(DraftCorrection::CreditNote {
                invoice: document.id.clone(),
                amount: candidate.open,
                account: classifications
                    .accounts_classed(&document.id.entity, &AccountClass::BadDebtWriteOff)
                    .next()
                    .cloned(),
            });
            run.decision_items.push(item);
        }
        None if age <= 0 => {
            run.placements.push(placement(
                AR_TIME_02,
                placed(candidate.due, Basis::DueDate, Confidence::Firm),
                None,
            ));
        }
        // AR-TIME-04: overdue with no evidence is not guessed.
        None => {
            run.placements.push(placement(
                AR_TIME_04,
                Outcome::Excluded(Exclusion::NoTimingEvidence),
                None,
            ));
            // Severity is CASH-ORDER-01's (checkpoint B): not set here.
            let mut item = item(
                ItemKind::SetExpectedDate,
                subject.clone(),
                AR_TIME_04,
                Role::Owner,
            );
            item.evidence = expected.map(Evidence::ExpectedDatePassed);
            run.decision_items.push(item);
            run.provisional.push(ProvisionalReason {
                rule: AR_TIME_04,
                subject,
            });
        }
    }
}
