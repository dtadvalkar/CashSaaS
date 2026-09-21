//! The AP Family, mirroring `docs/rules/ap.md`. Where they disagree, this file is wrong.
//!
//! Written from `ar.rs` (ADR-0022): the side-neutral arithmetic is shared through `document` and
//! `schedule`, and every function that reads a side is AP's own, copied from AR's where the
//! catalogues mirror each other.

use std::collections::BTreeMap;

use chrono::{Datelike, Days, NaiveDate, Weekday};
use rust_decimal::Decimal;

use crate::document::{
    age, balance_at, due_date, foreign, intercompany, last_completed_month, open_home, open_own,
    to_home, top_level, unapplied,
};
use crate::facts::{
    Direction, Document, DocumentKind, DocumentStatus, EntityId, FactId, Facts, PaymentPurpose,
    PostingStatus, PurchaseOrderStatus, ScheduledTemplate, Side, TemplateMode,
};
use crate::forecast::{
    Basis, Confidence, DecisionItem, DraftCorrection, DuplicateMatch, Evidence, Exclusion,
    ForecastRun, ItemKind, Outcome, Placement, Priority, ProvisionalReason, Reduction, Role,
    RuleId, Severity, Subject, Week,
};
use crate::money::{HomeAmount, round_money};
use crate::schedule::{due_by, index_at_or_before, nearest_occurrence, occurrence};
use crate::settings::{AccountClass, Classifications, CounterpartyClass, Settings};

pub(crate) const AP_OPEN_01: RuleId = RuleId("AP-OPEN-01");
pub(crate) const AP_OPEN_02: RuleId = RuleId("AP-OPEN-02");
pub(crate) const AP_OPEN_03: RuleId = RuleId("AP-OPEN-03");
pub(crate) const AP_OPEN_04: RuleId = RuleId("AP-OPEN-04");
pub(crate) const AP_TIME_01: RuleId = RuleId("AP-TIME-01");
pub(crate) const AP_TIME_02: RuleId = RuleId("AP-TIME-02");
pub(crate) const AP_TIME_03: RuleId = RuleId("AP-TIME-03");
pub(crate) const AP_RUN_01: RuleId = RuleId("AP-RUN-01");
pub(crate) const AP_SCHED_01: RuleId = RuleId("AP-SCHED-01");
pub(crate) const AP_PO_01: RuleId = RuleId("AP-PO-01");
pub(crate) const AP_FX_01: RuleId = RuleId("AP-FX-01");
pub(crate) const AP_DISC_01: RuleId = RuleId("AP-DISC-01");
pub(crate) const AP_DISC_02: RuleId = RuleId("AP-DISC-02");
pub(crate) const AP_UNAPPLIED_01: RuleId = RuleId("AP-UNAPPLIED-01");
pub(crate) const AP_UNAPPLIED_02: RuleId = RuleId("AP-UNAPPLIED-02");
pub const AP_PRIORITY_01: RuleId = RuleId("AP-PRIORITY-01");
pub(crate) const AP_DUP_01: RuleId = RuleId("AP-DUP-01");
pub(crate) const AP_PAID_01: RuleId = RuleId("AP-PAID-01");
pub(crate) const AP_TIE_01: RuleId = RuleId("AP-TIE-01");

/// A payable-side bill with a non-zero total that is not voided or deleted: posted (AP-OPEN-01)
/// or a draft (AP-OPEN-02). Copied from `ar::counts`, admitting drafts (Q145).
fn counts(document: &Document) -> bool {
    document.side == Side::Payable
        && document.kind == DocumentKind::Invoice
        && !matches!(
            document.status,
            DocumentStatus::Voided | DocumentStatus::Deleted
        )
        && !document.total.in_own_currency().1.is_zero()
}

/// An open, non-intercompany bill waiting to be timed.
struct Candidate<'a> {
    document: &'a Document,
    vendor: FactId,
    due: NaiveDate,
    /// Net of the credit in `reductions`.
    open: HomeAmount,
    reductions: Vec<Reduction>,
    /// A draft or awaiting approval: placed at Confidence estimated (AP-OPEN-02).
    draft: bool,
    priority: Option<Priority>,
    /// Rules that found something about this bill without changing where it is paid, so its
    /// Placement cites them: AP-DUP-01 and AP-PAID-01.
    findings: Vec<RuleId>,
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

/// AP-PRIORITY-01: the mark a vendor carries, government trust first, by top-level parent.
fn priority(
    facts: &Facts,
    settings: &Settings,
    classifications: &Classifications,
    vendor: &FactId,
) -> Option<Priority> {
    let top = top_level(facts, vendor);
    let trust =
        |id: &FactId| classifications.counterparty(id).class == CounterpartyClass::GovernmentTrust;
    if trust(vendor) || trust(&top) {
        Some(Priority::GovernmentTrust)
    } else if settings.critical_vendors.contains(&top) {
        Some(Priority::CriticalVendor)
    } else if settings.secured_lenders.contains(&top) {
        Some(Priority::SecuredLender)
    } else {
        None
    }
}

/// AP-RUN-01: the pay run a bill due on a date is paid in — the last run on or before it, or the
/// first run from the run date when that one is already past (Q156). `None` with no pay-run day
/// set, when the Rule does nothing.
fn run_day(pay_run: Option<Weekday>, as_of: NaiveDate, due: NaiveDate) -> Option<NaiveDate> {
    let weekday = pay_run?;
    let last = last_run(weekday, due);
    Some(if last >= as_of {
        last
    } else {
        next_run(weekday, as_of)
    })
}

/// The first pay run on or after a date.
fn next_run(weekday: Weekday, date: NaiveDate) -> NaiveDate {
    let ahead = (7 + weekday.num_days_from_monday() - date.weekday().num_days_from_monday()) % 7;
    date + Days::new(u64::from(ahead))
}

/// The last pay run on or before a date.
fn last_run(weekday: Weekday, date: NaiveDate) -> NaiveDate {
    let back = (7 + date.weekday().num_days_from_monday() - weekday.num_days_from_monday()) % 7;
    date - Days::new(u64::from(back))
}

/// Runs every AP Rule for one Entity.
pub fn run(
    run: &mut ForecastRun,
    facts: &Facts,
    settings: &Settings,
    classifications: &Classifications,
    entity: &EntityId,
) {
    let as_of = run.as_of;
    let mut candidates = Vec::new();
    for document in facts
        .documents()
        .values()
        .filter(|d| d.id.entity == *entity && counts(d))
    {
        let Some(due) = due_date(document) else {
            continue;
        };
        let own = open_own(facts, document, as_of);
        if own <= Decimal::ZERO {
            continue;
        }
        let open = to_home(document, own);
        if intercompany(facts, classifications, &document.counterparty) {
            run.placements.push(Placement {
                subject: subject(document),
                rule: AP_OPEN_03,
                rules: vec![AP_OPEN_03, AP_OPEN_01],
                outcome: Outcome::Excluded(Exclusion::Intercompany),
                amount: Some(open),
                history: None,
                foreign: foreign(document, own),
                reductions: Vec::new(),
                priority: None,
            });
            continue;
        }
        let draft = matches!(
            document.status,
            DocumentStatus::Draft | DocumentStatus::AwaitingApproval
        );
        if draft {
            run.decision_items.push(item(
                ItemKind::ApproveOrDeleteBill,
                subject(document),
                AP_OPEN_02,
                Role::Bookkeeper,
            ));
        }
        candidates.push(Candidate {
            document,
            vendor: top_level(facts, &document.counterparty),
            due,
            open,
            reductions: Vec::new(),
            draft,
            priority: priority(facts, settings, classifications, &document.counterparty),
            findings: Vec::new(),
        });
    }

    let mut candidates = net_credits(run, facts, entity, candidates);
    duplicates(run, &mut candidates);
    maybe_paid(run, facts, classifications, &mut candidates);

    for candidate in &candidates {
        time(run, facts, settings, entity, candidate);
    }

    discounts(run, facts, settings, classifications, entity, &candidates);
    purchases(run, facts, settings, classifications, entity);
    schedules(run, facts, settings, classifications, entity);
    month_end(run, facts, classifications, entity);
}

/// The scheduled dates the Entity's own records already cover, each with the fact covering it
/// (AP-SCHED-01, Q144, Q149): bills linked to the template where any are, otherwise bills, card
/// charges and bank spends to the same vendor. A copy of `ar::covered` with the side filter
/// changed and extended to account lines, which AR never reads. A bill in any state but voided or
/// deleted covers its occurrence.
fn covered(
    facts: &Facts,
    classifications: &Classifications,
    template: &ScheduledTemplate,
) -> BTreeMap<NaiveDate, FactId> {
    let entity = &template.id.entity;
    let bills: Vec<&Document> = facts
        .documents()
        .values()
        .filter(|d| d.id.entity == *entity && d.side == Side::Payable)
        .filter(|d| d.kind == DocumentKind::Invoice)
        .filter(|d| !matches!(d.status, DocumentStatus::Voided | DocumentStatus::Deleted))
        .collect();
    let linked: Vec<(&FactId, NaiveDate)> = bills
        .iter()
        .filter(|d| d.template.as_ref() == Some(&template.id))
        .map(|d| (&d.id, d.date))
        .collect();
    let vendor = top_level(facts, &template.counterparty);
    let covering: Vec<(&FactId, NaiveDate)> = if linked.is_empty() {
        let unlinked = bills
            .iter()
            .filter(|d| d.template.is_none() && top_level(facts, &d.counterparty) == vendor)
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
                    .is_some_and(|c| top_level(facts, c) == vendor)
            })
            .map(|l| (&l.id, l.date));
        unlinked.chain(spends).collect()
    } else {
        linked
    };
    let mut map = BTreeMap::new();
    for (id, date) in covering {
        if let Some(on) = nearest_occurrence(template.start, template.frequency, date) {
            map.entry(on).or_insert_with(|| id.clone());
        }
    }
    map
}

/// AP-SCHED-01 for the Entity's scheduled bill templates: one expected payment per scheduled date
/// within the Horizon, at that bill's due date, unless the occurrence is already covered. A
/// template that generates nothing automatically is one exclusion. AR-SCHED-02's missed
/// occurrence has no AP twin, so nothing is raised for a date already past.
fn schedules(
    run: &mut ForecastRun,
    facts: &Facts,
    settings: &Settings,
    classifications: &Classifications,
    entity: &EntityId,
) {
    let as_of = run.as_of;
    let end = run.horizon.end(as_of);
    let pay_run = settings.pay_run_weekday.get(entity).copied();
    let templates = facts
        .scheduled_templates()
        .values()
        .filter(|t| t.id.entity == *entity && t.side == Side::Payable);
    for template in templates {
        let automatic = template.active
            && matches!(
                template.mode,
                TemplateMode::Automatic | TemplateMode::AutomaticDraft
            );
        if !automatic {
            run.placements.push(Placement {
                subject: Subject::Fact(template.id.clone()),
                rule: AP_SCHED_01,
                rules: vec![AP_SCHED_01],
                outcome: Outcome::Excluded(Exclusion::TemplateNotAutomatic),
                amount: None,
                history: None,
                foreign: None,
                reductions: Vec::new(),
                priority: None,
            });
            continue;
        }
        let covered = covered(facts, classifications, template);
        let within = |on: NaiveDate| template.end.is_none_or(|e| on <= e);
        let mark = priority(facts, settings, classifications, &template.counterparty);

        // Occurrences after the run date to the end of the Horizon. The one on or before the run
        // date is not forecast: its bill either exists already or the schedule has not produced it.
        let mut k =
            index_at_or_before(template.start, template.frequency, as_of).map_or(0, |k| k + 1);
        k = k.max(0);
        while let Some(on) =
            occurrence(template.start, template.frequency, k).filter(|on| *on < end && within(*on))
        {
            let subject = Subject::Occurrence {
                template: template.id.clone(),
                date: on,
            };
            let (outcome, rules) = match covered.get(&on) {
                Some(fact) => (
                    Outcome::Excluded(Exclusion::CoveredBy(fact.clone())),
                    vec![AP_SCHED_01],
                ),
                None => match due_by(template.due_rule, on) {
                    Some(due) => match run_day(pay_run, as_of, due) {
                        Some(paid) => (
                            Outcome::placed_on(
                                run.horizon,
                                as_of,
                                paid,
                                Direction::Out,
                                Basis::ScheduledBill,
                                Confidence::Estimated,
                            ),
                            vec![AP_SCHED_01, AP_RUN_01],
                        ),
                        None => (
                            Outcome::placed_on(
                                run.horizon,
                                as_of,
                                due,
                                Direction::Out,
                                Basis::ScheduledBill,
                                Confidence::Estimated,
                            ),
                            vec![AP_SCHED_01],
                        ),
                    },
                    None => (
                        Outcome::Excluded(Exclusion::CommittedPurchaseTimingUnknown),
                        vec![AP_SCHED_01],
                    ),
                },
            };
            run.placements.push(Placement {
                subject,
                rule: AP_SCHED_01,
                rules,
                outcome,
                amount: Some(template.amount.home),
                history: None,
                foreign: None,
                reductions: Vec::new(),
                priority: mark,
            });
            k += 1;
        }
    }
}

/// AP-TIE-01 at the end of the last completed month: open bills less unapplied vendor credits,
/// both at that day, against the balance of the AP control accounts. With no account classified
/// AP control, or no balance fact dated on the month end, there is nothing to read and the Rule is
/// silent. A copy of `ar::month_end`'s AR-TIE-01 half with the class, the item kind and the Rule
/// ID changed (ADR-0022); AR-UNBILLED-01 has no AP twin. Open bills here are posted bills only: a
/// draft bill is not in the ledger's control account, so counting it would create a difference the
/// bookkeeper cannot reconcile.
fn month_end(
    run: &mut ForecastRun,
    facts: &Facts,
    classifications: &Classifications,
    entity: &EntityId,
) {
    let Some((month, end)) = last_completed_month(run.as_of) else {
        return;
    };
    let controls: Vec<(&FactId, HomeAmount)> = classifications
        .accounts_classed(entity, &AccountClass::ApControl)
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
        .filter(|d| d.status == DocumentStatus::Posted)
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
    // Severity is CASH-ORDER-01's (checkpoint B): not set here.
    let mut item = item(
        ItemKind::ReconcileAp,
        subject.clone(),
        AP_TIE_01,
        Role::Bookkeeper,
    );
    item.evidence = Some(Evidence::ControlDifference {
        open: HomeAmount(subledger),
        control: HomeAmount(control),
        difference: HomeAmount(difference),
    });
    run.decision_items.push(item);
    run.provisional.push(ProvisionalReason {
        rule: AP_TIE_01,
        subject,
    });
}

/// One bill's early-payment discount, once its arithmetic is known (AP-DISC-02).
struct Offer {
    bill: FactId,
    /// The last day the discount can be taken, and the week it falls in.
    on: NaiveDate,
    at: usize,
    /// The week the bill is otherwise paid in.
    instead_of: usize,
    /// The full open amount, the discounted amount, and the difference.
    full: Decimal,
    pay: Decimal,
    saving: Decimal,
    /// A percentage, exact until it is shown.
    rate: Decimal,
}

/// AP-DISC-02: a discount the forecast can absorb is pointed out with what skipping it costs.
/// Offers are judged in discount-date order, each with the earlier suggested discounts taken, so
/// every item raised can be acted on together (Q157); a tie goes to the larger saving. Affording
/// one means every week's closing cash (CASH-ROLL-01) stays at or above zero and any Minimum Cash
/// Buffer once the payment moves to the discount date.
fn discounts(
    run: &mut ForecastRun,
    facts: &Facts,
    settings: &Settings,
    classifications: &Classifications,
    entity: &EntityId,
    candidates: &[Candidate<'_>],
) {
    let as_of = run.as_of;
    let floor = settings
        .minimum_cash_buffer
        .get(entity)
        .map_or(Decimal::ZERO, |buffer| buffer.0)
        .max(Decimal::ZERO);
    let opening = crate::cash::opening_cash(facts, classifications, entity, as_of);
    let base: Vec<HomeAmount> = crate::cash::weeks(&run.placements, entity, opening, run.horizon)
        .into_iter()
        .map(|week| week.closing)
        .collect();
    if base.is_empty() {
        return;
    }
    let hundred = Decimal::from(100);

    let mut offers: Vec<Offer> = Vec::new();
    for candidate in candidates {
        let document = candidate.document;
        let Some(discount) = document
            .terms
            .as_ref()
            .and_then(|t| facts.payment_terms().get(t))
            .and_then(|t| t.discount.as_ref())
        else {
            continue;
        };
        let Some(on) = document
            .date
            .checked_add_days(Days::new(u64::from(discount.days)))
        else {
            continue;
        };
        if on < as_of {
            continue;
        }
        let Some(at) = run.horizon.week_of(as_of, on).map(week_index) else {
            continue;
        };
        // ponytail: the bill's own Placement is found by walking the run's; a Scenario has tens of
        // Placements, and an index is worth building when a Group has thousands.
        let Some(instead_of) = run
            .placements
            .iter()
            .filter(|p| p.rule.family() == "AP" && p.subject == subject(document))
            .find_map(|p| match p.outcome {
                Outcome::Placed { week, .. } => Some(week_index(week)),
                Outcome::Stated { .. } | Outcome::Excluded(_) | Outcome::Shown => None,
            })
        else {
            continue;
        };
        let full = candidate.open.0;
        let Some(fraction) = discount.percent.checked_div(hundred) else {
            continue;
        };
        let Some(saving) = full.checked_mul(fraction).map(round_money) else {
            continue;
        };
        let days = Decimal::from(age(candidate.due, on));
        let Some(rate) = Decimal::ONE
            .checked_sub(fraction)
            .and_then(|net| fraction.checked_div(net))
            .and_then(|r| r.checked_mul(Decimal::from(365)))
            .and_then(|r| r.checked_div(days))
            .and_then(|r| r.checked_mul(hundred))
        else {
            continue;
        };
        offers.push(Offer {
            bill: document.id.clone(),
            on,
            at,
            instead_of,
            full,
            pay: full.saturating_sub(saving),
            saving,
            rate,
        });
    }
    offers.sort_by(|a, b| {
        (a.on, std::cmp::Reverse(a.saving), &a.bill).cmp(&(
            b.on,
            std::cmp::Reverse(b.saving),
            &b.bill,
        ))
    });

    // Moving a payment out of one week and into an earlier one shifts every week from each of the
    // two onwards, so the effect on the roll is an adjustment to the closing figures rather than a
    // second roll. Adjustments accumulate as offers are accepted.
    let mut taken = vec![Decimal::ZERO; base.len()];
    for offer in offers {
        let mut trial = taken.clone();
        for week in trial.iter_mut().skip(offer.instead_of) {
            *week = week.saturating_add(offer.full);
        }
        for week in trial.iter_mut().skip(offer.at) {
            *week = week.saturating_sub(offer.pay);
        }
        if base
            .iter()
            .zip(&trial)
            .any(|(closing, adjustment)| closing.0.saturating_add(*adjustment) < floor)
        {
            continue;
        }
        taken = trial;
        let mut finding = item(
            ItemKind::TakeDiscount,
            Subject::Fact(offer.bill.clone()),
            AP_DISC_02,
            Role::Owner,
        );
        finding.evidence = Some(Evidence::Discount {
            pay: HomeAmount(offer.pay),
            by: offer.on,
            saving: HomeAmount(offer.saving),
            implied_annual_rate: offer.rate,
        });
        run.decision_items.push(finding);
    }
}

fn week_index(week: Week) -> usize {
    usize::try_from(week.0)
        .unwrap_or_default()
        .saturating_sub(1)
}

/// AP-DUP-01: two open bills from the same vendor with the same vendor reference, or with the same
/// amount and bill date, raise one item to check for a duplicate. Both stay in the forecast, which
/// overstates outflows rather than understating them. A bill in two pairs is in two items. Posted
/// bills only: a draft already has AP-OPEN-02's approve-or-delete item. A pair matching on both
/// tests raises one item, on the reference, which is the stronger evidence.
fn duplicates(run: &mut ForecastRun, candidates: &mut [Candidate<'_>]) {
    for i in 0..candidates.len() {
        for j in (i + 1)..candidates.len() {
            if candidates[i].vendor != candidates[j].vendor
                || candidates[i].draft
                || candidates[j].draft
            {
                continue;
            }
            let (a, b) = (candidates[i].document, candidates[j].document);
            let same_reference =
                a.vendor_reference.is_some() && a.vendor_reference == b.vendor_reference;
            let matched = if same_reference {
                DuplicateMatch::SameReference
            } else if a.date == b.date && a.total.in_own_currency().1 == b.total.in_own_currency().1
            {
                DuplicateMatch::SameAmountAndDate
            } else {
                continue;
            };
            let (first, second) = if a.id <= b.id {
                (a.id.clone(), b.id.clone())
            } else {
                (b.id.clone(), a.id.clone())
            };
            let mut finding = item(
                ItemKind::PossibleDuplicateBill,
                Subject::Pair(first, second),
                AP_DUP_01,
                Role::Bookkeeper,
            );
            finding.evidence = Some(Evidence::Duplicate(matched));
            run.decision_items.push(finding);
            for k in [i, j] {
                if !candidates[k].findings.contains(&AP_DUP_01) {
                    candidates[k].findings.push(AP_DUP_01);
                }
            }
        }
    }
}

/// AP-PAID-01: an open bill beside a bank or card spend that is not a bill payment, to the same
/// vendor, for the same amount, on or after the bill date. The bill stays in the forecast until
/// the bookkeeper matches the payment or confirms the bill is still owed. A spend is a bill
/// payment when its source names a `Payment` fact of the Entity; no near match is assumed.
fn maybe_paid(
    run: &mut ForecastRun,
    facts: &Facts,
    classifications: &Classifications,
    candidates: &mut [Candidate<'_>],
) {
    for candidate in candidates.iter_mut() {
        let bill = candidate.document;
        let vendor = candidate.vendor.clone();
        let open = candidate.open.0;
        let spends: Vec<FactId> = facts
            .account_lines()
            .values()
            .filter(|l| l.id.entity == bill.id.entity && l.status == PostingStatus::Posted)
            .filter(|l| {
                matches!(
                    classifications.account(&l.account).class,
                    AccountClass::Bank | AccountClass::CreditCard
                )
            })
            .filter(|l| l.date >= bill.date && l.amount.home.0.abs() == open)
            .filter(|l| {
                l.counterparty
                    .as_ref()
                    .is_some_and(|c| top_level(facts, c) == vendor)
            })
            .filter(|l| {
                !facts
                    .payments()
                    .values()
                    .any(|p| p.id.entity == l.id.entity && p.id.provider_id == l.source.id)
            })
            .map(|l| l.id.clone())
            .collect();
        for spend in spends {
            // The bill comes first: the pair is a bill and the transaction that may have paid it,
            // which is not the id order AP-DUP-01's pair of two bills takes.
            let mut finding = item(
                ItemKind::BillMayAlreadyBePaid,
                Subject::Pair(bill.id.clone(), spend),
                AP_PAID_01,
                Role::Bookkeeper,
            );
            finding.evidence = Some(Evidence::MaybePaid);
            run.decision_items.push(finding);
            if !candidate.findings.contains(&AP_PAID_01) {
                candidate.findings.push(AP_PAID_01);
            }
        }
    }
}

/// Unapplied vendor credit: a posted vendor payment or payable-side credit note of the Entity with
/// an unapplied remainder on the run date (AP-UNAPPLIED-01, Q251). A copy of `ar::credits` with
/// the side filters changed (ADR-0022).
struct Credit {
    id: FactId,
    vendor: FactId,
    date: NaiveDate,
    remainder: Decimal,
}

fn credits(facts: &Facts, entity: &EntityId, as_of: NaiveDate) -> Vec<Credit> {
    let payments = facts
        .payments()
        .values()
        .filter(|p| p.id.entity == *entity && p.status == PostingStatus::Posted)
        .filter(|p| p.direction == Direction::Out && p.date <= as_of)
        .filter_map(|p| match &p.purpose {
            PaymentPurpose::Vendor { counterparty } => Some((&p.id, counterparty, p.date)),
            _ => None,
        });
    let notes = facts
        .documents()
        .values()
        .filter(|d| d.id.entity == *entity && d.side == Side::Payable)
        .filter(|d| d.kind == DocumentKind::CreditNote && d.status == DocumentStatus::Posted)
        .filter(|d| d.date <= as_of)
        .map(|d| (&d.id, &d.counterparty, d.date));
    let mut credits: Vec<Credit> = payments
        .chain(notes)
        .filter_map(|(id, vendor, date)| {
            let remainder = unapplied(facts, id, as_of)?.0;
            (remainder > Decimal::ZERO).then(|| Credit {
                id: id.clone(),
                vendor: top_level(facts, vendor),
                date,
                remainder,
            })
        })
        .collect();
    credits.sort_by(|a, b| (a.date, &a.id).cmp(&(b.date, &b.id)));
    credits
}

/// AP-UNAPPLIED-01 and AP-UNAPPLIED-02: each credit, oldest first, reduces its vendor's open
/// bills, earliest due first. A bill reduced to nothing is excluded (Q148) and is no longer a
/// candidate; credit for a vendor with no open bill is a vendor credit balance. A copy of
/// `ar::net_credits` with the side filters and output tokens changed (ADR-0022).
fn net_credits<'a>(
    run: &mut ForecastRun,
    facts: &Facts,
    entity: &EntityId,
    mut candidates: Vec<Candidate<'a>>,
) -> Vec<Candidate<'a>> {
    let as_of = run.as_of;
    for credit in credits(facts, entity, as_of) {
        let mut order: Vec<usize> = (0..candidates.len())
            .filter(|&i| candidates[i].vendor == credit.vendor)
            .filter(|&i| candidates[i].open.0 > Decimal::ZERO)
            .collect();
        order.sort_by_key(|&i| (candidates[i].due, candidates[i].document.id.clone()));
        if order.is_empty() {
            run.placements.push(Placement {
                subject: Subject::Fact(credit.id.clone()),
                rule: AP_UNAPPLIED_02,
                rules: vec![AP_UNAPPLIED_02],
                outcome: Outcome::Excluded(Exclusion::VendorCreditBalance),
                amount: Some(HomeAmount(credit.remainder)),
                history: None,
                foreign: None,
                reductions: Vec::new(),
                priority: None,
            });
            run.decision_items.push(item(
                ItemKind::ClaimRefundOrHoldCredit,
                Subject::Fact(credit.id),
                AP_UNAPPLIED_02,
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
            ItemKind::ApplyVendorCredit,
            Subject::Fact(credit.id.clone()),
            AP_UNAPPLIED_01,
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
            rule: AP_UNAPPLIED_01,
            rules: vec![AP_UNAPPLIED_01, AP_OPEN_01],
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

/// AP-PO-01: an authorised purchase order not yet billed is placed at its delivery date plus the
/// vendor's payment terms when both are known, and is otherwise a committed purchase whose timing
/// is unknown. No timing is guessed (ADR-0019).
fn purchases(
    run: &mut ForecastRun,
    facts: &Facts,
    settings: &Settings,
    classifications: &Classifications,
    entity: &EntityId,
) {
    let as_of = run.as_of;
    for order in facts
        .purchase_orders()
        .values()
        .filter(|o| o.id.entity == *entity && o.status == PurchaseOrderStatus::Authorised)
    {
        let terms = facts
            .counterparties()
            .get(&order.vendor)
            .and_then(|c| c.payable_terms.as_ref())
            .and_then(|t| facts.payment_terms().get(t));
        let due = order
            .delivery_date
            .zip(terms)
            .and_then(|(delivery, terms)| due_by(terms.due_rule, delivery));
        let outcome = match due {
            Some(date) => Outcome::placed_on(
                run.horizon,
                as_of,
                date,
                Direction::Out,
                Basis::PurchaseOrder,
                Confidence::Estimated,
            ),
            None => Outcome::Excluded(Exclusion::CommittedPurchaseTimingUnknown),
        };
        let mark = priority(facts, settings, classifications, &order.vendor);
        run.placements.push(Placement {
            subject: Subject::Fact(order.id.clone()),
            rule: AP_PO_01,
            rules: if mark.is_some() {
                vec![AP_PO_01, AP_PRIORITY_01]
            } else {
                vec![AP_PO_01]
            },
            outcome,
            amount: Some(order.total.home),
            history: None,
            foreign: None,
            reductions: Vec::new(),
            priority: mark,
        });
    }
}

/// AP-TIME-01 to AP-TIME-03 and AP-RUN-01 for one open bill. The planned-date branch is copied
/// from `ar::time` (Q150); the rest is AP's own, since bills have no history (Q97).
fn time(
    run: &mut ForecastRun,
    facts: &Facts,
    settings: &Settings,
    entity: &EntityId,
    candidate: &Candidate<'_>,
) {
    let as_of = run.as_of;
    let document = candidate.document;
    let own = open_own(facts, document, as_of);
    let mut rules = vec![
        if candidate.draft {
            AP_OPEN_02
        } else {
            AP_OPEN_01
        },
        AP_OPEN_04,
    ];
    if document.total.transaction.is_some() {
        rules.push(AP_FX_01);
    }
    if !candidate.reductions.is_empty() {
        rules.push(AP_UNAPPLIED_01);
    }
    if document
        .terms
        .as_ref()
        .and_then(|t| facts.payment_terms().get(t))
        .is_some_and(|t| t.discount.is_some())
    {
        rules.push(AP_DISC_01);
    }
    if candidate.priority.is_some() {
        rules.push(AP_PRIORITY_01);
    }
    rules.extend(candidate.findings.iter().copied());
    let placement = |rules: Vec<RuleId>, outcome: Outcome| Placement {
        subject: subject(document),
        rule: rules[0],
        rules,
        outcome,
        amount: Some(candidate.open),
        history: None,
        foreign: foreign(document, own),
        reductions: candidate.reductions.clone(),
        priority: candidate.priority,
    };
    let confidence = if candidate.draft {
        Confidence::Estimated
    } else {
        Confidence::Firm
    };
    let placed = |date: NaiveDate, basis: Basis| {
        Outcome::placed_on(run.horizon, as_of, date, Direction::Out, basis, confidence)
    };
    let cite = |timing: &[RuleId]| timing.iter().chain(rules.iter()).copied().collect();

    // AP-TIME-01: a planned date on or after the run date places the bill.
    let planned = document.expected_date;
    if let Some(date) = planned.filter(|d| *d >= as_of) {
        run.placements.push(placement(
            cite(&[AP_TIME_01]),
            placed(date, Basis::PlannedDate),
        ));
        return;
    }

    let pay_run = settings.pay_run_weekday.get(entity).copied();

    // AP-TIME-03: an overdue bill is paid now, or in the first pay run (Q98).
    if age(as_of, candidate.due) > 0 {
        let outcome = match pay_run {
            Some(weekday) => placed(next_run(weekday, as_of), Basis::PayRun),
            None => placed(as_of, Basis::Overdue),
        };
        let timing: &[RuleId] = if pay_run.is_some() {
            &[AP_TIME_03, AP_RUN_01]
        } else {
            &[AP_TIME_03]
        };
        run.placements.push(placement(cite(timing), outcome));
        let mut overdue = item(
            ItemKind::OverdueBill,
            subject(document),
            AP_TIME_03,
            Role::Owner,
        );
        overdue.evidence = Some(Evidence::Overdue {
            due: candidate.due,
            planned,
        });
        overdue.priority = candidate.priority;
        run.decision_items.push(overdue);
        return;
    }

    // AP-TIME-02: the due date, moved to the pay run that pays it (AP-RUN-01, Q156).
    let (outcome, timing): (Outcome, &[RuleId]) = match run_day(pay_run, as_of, candidate.due) {
        Some(date) => (placed(date, Basis::PayRun), &[AP_TIME_02, AP_RUN_01]),
        None => (placed(candidate.due, Basis::DueDate), &[AP_TIME_02]),
    };
    run.placements.push(placement(cite(timing), outcome));

    // AP-TIME-01: a planned date that has passed on a bill not yet overdue asks for a new one.
    if let Some(passed) = planned {
        let mut stale = item(
            ItemKind::PlannedDatePassed,
            subject(document),
            AP_TIME_01,
            Role::Owner,
        );
        stale.evidence = Some(Evidence::PlannedDatePassed(passed));
        stale.priority = candidate.priority;
        run.decision_items.push(stale);
    }
}
