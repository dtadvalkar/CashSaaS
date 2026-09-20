//! Renders a Scenario's facts and its Forecast Run as the Scenario document's own tables, and
//! checks them against the approved section (ADR-0010 as amended, ADR-0021, Q259).

use std::collections::BTreeSet;

use cashsaas_core::facts::{
    Direction, Document, DocumentKind, DueRule, FactId, FrequencyUnit, PaymentTerms,
    PurchaseOrderStatus, Side, TemplateMode,
};
use cashsaas_core::forecast::{
    Basis, Confidence, DecisionItem, DraftCorrection, DuplicateMatch, Evidence, Exclusion,
    ForecastRun, ItemKind, Outcome, Placement, Priority, Role, Severity, Stretch, Subject, Week,
};
use cashsaas_core::money::HomeAmount;
use cashsaas_core::settings::{AccountClass, classify};

use cashsaas_core::{ar, document};
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;

use crate::Scenario;
use crate::tables::{Match, Table, compare, is_rule_id, section, tables, tokens};

const NONE: &str = "—";

/// Whether any row of this column produced a value, which makes it a column the renderer derives.
/// A row that then produced none is a fact the run does not hold, not commentary.
fn derived_column(rendered: &[Vec<Option<String>>], column: usize) -> bool {
    rendered
        .iter()
        .any(|row| row.get(column).is_some_and(Option::is_some))
}

/// Whether the document writes a digit anywhere in this column. A column no renderer derives is
/// commentary, shown and not compared, only while it carries no number: if there is a figure in
/// the cell, it is checked, so an amount or a date cannot reach the owner unverified (Q192).
fn column_has_digit(table: &Table, column: usize) -> bool {
    table
        .rows
        .iter()
        .filter_map(|row| row.get(column))
        .any(|cell| cell.chars().any(|c| c.is_ascii_digit()))
}

/// `5,250.00`, with the document's minus sign.
pub fn amount(value: Decimal) -> String {
    let text = value.abs().round_dp(2).to_string();
    let (whole, fraction) = text.split_once('.').unwrap_or((&text, "00"));
    let mut grouped = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    let sign = if value.is_sign_negative() && !value.is_zero() {
        "−"
    } else {
        ""
    };
    format!("{sign}{grouped}.{fraction:0<2}")
}

/// `10-20` in the Scenario's year, else `2027-01-05`.
pub fn date(scenario: &Scenario, value: NaiveDate) -> String {
    if value.year() == scenario.year() {
        value.format("%m-%d").to_string()
    } else {
        value.format("%Y-%m-%d").to_string()
    }
}

/// `+5`, `−17` or `0`.
pub fn age(days: i64) -> String {
    match days {
        d if d > 0 => format!("+{d}"),
        d if d < 0 => format!("−{}", d.abs()),
        _ => "0".to_owned(),
    }
}

fn number(scenario: &Scenario, id: &FactId) -> String {
    scenario
        .facts()
        .documents()
        .get(id)
        .and_then(|d| d.number.clone())
        .or_else(|| {
            scenario
                .facts()
                .payments()
                .get(id)
                .map(|_| id.provider_id.clone())
        })
        .or_else(|| {
            scenario
                .facts()
                .scheduled_templates()
                .get(id)
                .map(|_| id.provider_id.clone())
        })
        .or_else(|| scenario.facts().accounts().get(id).map(|a| a.name.clone()))
        .unwrap_or_else(|| id.provider_id.clone())
}

fn subject(scenario: &Scenario, subject: &Subject) -> String {
    match subject {
        Subject::Fact(id) => number(scenario, id),
        Subject::Occurrence { template, date: on } => {
            format!(
                "{} occurrence {}",
                number(scenario, template),
                date(scenario, *on)
            )
        }
        Subject::AccountMonth { account, month } => {
            format!("{}, {}", number(scenario, account), month.format("%B %Y"))
        }
        Subject::Pair(a, b) => format!("{}, {}", covering(scenario, a), covering(scenario, b)),
        // CASH-OPEN-01 cites every bank account it summed, which is the document's `Item` cell.
        Subject::Position { accounts, .. } => accounts
            .iter()
            .map(|id| number(scenario, id))
            .collect::<Vec<_>>()
            .join(", "),
        Subject::Group => "Group".to_owned(),
        Subject::Entity(id) => id.0.clone(),
    }
}

/// `bills at 09-30`: the last day of the month a Decision Item is about.
fn month_end(scenario: &Scenario, month: NaiveDate) -> String {
    month
        .checked_add_months(chrono::Months::new(1))
        .and_then(|next| next.pred_opt())
        .map_or("bills".to_owned(), |end| {
            format!("bills at {}", date(scenario, end))
        })
}

fn priority(mark: Option<Priority>) -> String {
    match mark {
        Some(Priority::GovernmentTrust) => "government trust",
        Some(Priority::CriticalVendor) => "critical vendor",
        Some(Priority::SecuredLender) => "secured lender",
        None => NONE,
    }
    .to_owned()
}

/// What covered an occurrence, as the Scenario documents name it: a bill by its number, and a
/// spend by what it is and the day it was recorded — `card charge 10-06`, `bank spend 10-02` —
/// since an account line has no number a reader would recognise.
fn covering(scenario: &Scenario, id: &FactId) -> String {
    let Some(line) = scenario.facts().account_lines().get(id) else {
        return number(scenario, id);
    };
    let kind = match classify(scenario.facts(), scenario.settings())
        .account(&line.account)
        .class
    {
        AccountClass::CreditCard => "card charge",
        // Money in to a bank account is a receipt, not a spend: it covers a CASH-SCHED-01
        // occurrence where a spend covers an AP-SCHED-01 one.
        _ if line.amount.home.0.is_sign_positive() && !line.amount.home.0.is_zero() => {
            "bank receipt"
        }
        _ => "bank spend",
    };
    format!("{kind} {}", date(scenario, line.date))
}

fn basis(scenario: &Scenario, placement: &Placement, as_of: NaiveDate) -> String {
    match &placement.outcome {
        Outcome::Stated { .. } => format!("book balance at {}", date(scenario, as_of)),
        Outcome::Placed { basis, .. } => match basis {
            Basis::DueDate => "due date",
            Basis::ExpectedDate => "expected date",
            Basis::EntityHistory => "Entity history",
            Basis::CustomerHistory => "customer history",
            Basis::ScheduledInvoice => "scheduled invoice",
            Basis::PlannedDate => "planned date",
            Basis::Overdue => "overdue",
            Basis::PayRun => "pay run",
            Basis::ScheduledBill => "scheduled bill",
            Basis::PurchaseOrder => "purchase order",
            Basis::ScheduledReceipt => "scheduled receipt",
        }
        .to_owned(),
        Outcome::Excluded(reason) => match reason {
            Exclusion::NotIssued => "not issued".to_owned(),
            Exclusion::Intercompany => "intercompany, see IC".to_owned(),
            Exclusion::BeyondHorizon(on) => format!("beyond horizon ({})", date(scenario, *on)),
            Exclusion::HistoryUncollected => "history says uncollected".to_owned(),
            Exclusion::NoTimingEvidence => "no timing evidence".to_owned(),
            Exclusion::CoveredBy(id) => format!("covered by {}", covering(scenario, id)),
            Exclusion::TemplateNotAutomatic => "template not automatic".to_owned(),
            Exclusion::OffsetByCredit(id) => format!("offset by credit ({})", number(scenario, id)),
            Exclusion::CreditBalance => "credit balance".to_owned(),
            Exclusion::CommittedPurchaseTimingUnknown => {
                "committed purchase, timing unknown".to_owned()
            }
            Exclusion::VendorCreditBalance => "vendor credit balance".to_owned(),
        },
    }
}

fn week(scenario: &Scenario, placement: &Placement) -> String {
    match &placement.outcome {
        Outcome::Stated { .. } => NONE.to_owned(),
        Outcome::Placed {
            week,
            date: on,
            basis,
            ..
        } => {
            let due = match (basis, placement.history) {
                (Basis::DueDate, _) | (Basis::ScheduledInvoice | Basis::ScheduledBill, None) => {
                    "due "
                }
                _ => "",
            };
            format!("W{} ({due}{})", week.0, date(scenario, *on))
        }
        Outcome::Excluded(_) => NONE.to_owned(),
    }
}

fn placement_amount(scenario: &Scenario, placement: &Placement) -> String {
    let Some(value) = placement.amount else {
        return NONE.to_owned();
    };
    if placement.reductions.is_empty() {
        return amount(value.0);
    }
    let gross = placement
        .reductions
        .iter()
        .fold(value.0, |g, r| g.saturating_add(r.amount));
    let less: Vec<String> = placement
        .reductions
        .iter()
        .map(|r| {
            let credit = number(scenario, &r.credit);
            if r.remainder {
                format!("{credit}'s remaining {}", amount(r.amount))
            } else {
                format!("{credit} {}", amount(r.amount))
            }
        })
        .collect();
    format!(
        "{} ({} less {})",
        amount(value.0),
        amount(gross),
        less.join(", ")
    )
}

fn rules(placement_rules: &[cashsaas_core::forecast::RuleId]) -> String {
    placement_rules
        .iter()
        .map(|r| r.0)
        .collect::<Vec<_>>()
        .join(", ")
}

fn placement_cell(
    scenario: &Scenario,
    placement: &Placement,
    header: &str,
    as_of: NaiveDate,
) -> String {
    match header {
        // A stated position is named by the Rule that states it, not by the word "Placement":
        // the CASH documents head its row `Opening Cash`.
        "Output" => match placement.outcome {
            Outcome::Stated { .. } if placement.rule.0 == "CASH-OPEN-01" => "Opening Cash",
            Outcome::Stated { .. } => "Position",
            Outcome::Placed { .. } => "Placement",
            Outcome::Excluded(_) => "Exclusion",
        }
        .to_owned(),
        "Invoice" | "Bill" | "Item" | "Placement" => subject(scenario, &placement.subject),
        "Week" => week(scenario, placement),
        "Priority mark" => priority(placement.priority),
        "Amount" | "Amount (CAD)" => placement_amount(scenario, placement),
        "Basis / reason" | "Basis" | "Reason" => basis(scenario, placement, as_of),
        "Confidence" => match placement.outcome {
            Outcome::Placed {
                confidence: Confidence::Firm,
                ..
            }
            | Outcome::Stated {
                confidence: Confidence::Firm,
            } => "firm",
            Outcome::Placed {
                confidence: Confidence::Estimated,
                ..
            }
            | Outcome::Stated {
                confidence: Confidence::Estimated,
            } => "estimated",
            Outcome::Excluded(_) => NONE,
        }
        .to_owned(),
        "Median" => placement
            .history
            .and_then(|h| h.median_days)
            .map_or(NONE.to_owned(), |m| format!("{m} days")),
        "Comparable" => placement.history.map_or(NONE.to_owned(), |h| {
            if h.median_days.is_some() {
                h.comparable.to_string()
            } else if h.never_collected == h.comparable {
                format!("{} (all never collected)", h.comparable)
            } else {
                format!("{} ({} never collected)", h.comparable, h.never_collected)
            }
        }),
        "Recorded on Placement" => placement.foreign.map_or(NONE.to_owned(), |f| {
            format!(
                "{} {} at {:.4}",
                f.currency.code(),
                amount(f.amount),
                f.rate
            )
        }),
        "Rules" => rules(&placement.rules),
        _ => format!("? {header}"),
    }
}

fn draft(scenario: &Scenario, draft: &DraftCorrection) -> String {
    match draft {
        DraftCorrection::CreditNote {
            invoice,
            amount: value,
            account,
        } => {
            let mut text = format!(
                "Credit note for {} against {}",
                amount(value.0),
                number(scenario, invoice)
            );
            if account.is_none() {
                text.push_str("; incomplete: no bad-debt account mapped");
            }
            text
        }
        DraftCorrection::PaymentApplication { from, to } => {
            let parts: Vec<String> = to
                .iter()
                .enumerate()
                .map(|(i, (document, value))| {
                    if i == 0 {
                        format!(
                            "{} of {} to {}",
                            amount(*value),
                            number(scenario, from),
                            number(scenario, document)
                        )
                    } else {
                        format!("{} to {}", amount(*value), number(scenario, document))
                    }
                })
                .collect();
            format!("Payment application: {}", parts.join(", "))
        }
    }
}

fn item_cell(scenario: &Scenario, item: &DecisionItem, header: &str) -> String {
    match header {
        "Decision Item" => {
            let kind = match item.kind {
                ItemKind::Collect => "collect",
                ItemKind::SetExpectedDate => "set expected date",
                ItemKind::ExpectedDatePassed => "expected date passed",
                ItemKind::InvoiceMissed => "invoice missed",
                ItemKind::ApplyCredit => "apply credit",
                ItemKind::RefundOrApplyCredit => "refund or apply credit",
                ItemKind::ReviewForWriteOff => "review for write-off",
                ItemKind::InvoiceAccruedRevenue => "invoice accrued revenue",
                ItemKind::ReconcileAr => "reconcile AR",
                ItemKind::ApproveOrDeleteBill => "approve or delete bill",
                ItemKind::PlannedDatePassed => "planned date passed",
                ItemKind::OverdueBill => "overdue bill",
                ItemKind::TakeDiscount => "take discount",
                ItemKind::ApplyVendorCredit => "apply vendor credit",
                ItemKind::ClaimRefundOrHoldCredit => "claim refund or hold credit",
                ItemKind::PossibleDuplicateBill => "possible duplicate bill",
                ItemKind::BillMayAlreadyBePaid => "bill may already be paid",
                ItemKind::ReconcileAp => "reconcile AP",
                ItemKind::SetHorizon => "set horizon",
                ItemKind::CashShortfall => "cash shortfall",
                ItemKind::BelowBuffer => "below buffer",
            };
            match item.evidence {
                Some(Evidence::ExpectedDatePassed(on)) => {
                    format!(
                        "{kind} (cites expected date {}, passed)",
                        date(scenario, on)
                    )
                }
                _ => kind.to_owned(),
            }
        }
        // The document writes an item's occurrence `T2, occurrence 10-01`, an output's `T1
        // occurrence 10-15`.
        "Subject" => match &item.subject {
            Subject::Occurrence { template, date: on } => format!(
                "{}, occurrence {}",
                number(scenario, template),
                date(scenario, *on)
            ),
            other => subject(scenario, other),
        },
        "Class" => match item.severity {
            Severity::Critical => "critical",
            Severity::Blocking => "blocking",
            Severity::Action => "action",
        }
        .to_owned(),
        "Due" => item.due.map_or(NONE.to_owned(), |on| date(scenario, on)),
        "Acted on by" => match item.acted_on_by {
            Role::Owner => "owner",
            Role::Bookkeeper => "bookkeeper",
            Role::Accountant => "accountant",
        }
        .to_owned(),
        "Draft Correction" => item
            .draft
            .as_ref()
            .map_or(NONE.to_owned(), |d| draft(scenario, d)),
        "Priority mark" => priority(item.priority),
        "Evidence" => match &item.evidence {
            Some(Evidence::Balance(value)) => format!("Balance {}", amount(value.0)),
            Some(Evidence::ControlDifference {
                open,
                control,
                difference,
            }) => {
                // AR writes "Open invoices 12,000.00 …"; AP writes "Open bills at 09-30 7,500.00
                // …", so the payable side states the day the subledger was totalled on. It is the
                // end of the item's own month, which its subject already names.
                let at = match (item.rule.family(), &item.subject) {
                    ("AP", Subject::AccountMonth { month, .. }) => month_end(scenario, *month),
                    ("AP", _) => "bills".to_owned(),
                    _ => "invoices".to_owned(),
                };
                format!(
                    "Open {at} {} against control {}; difference {}",
                    amount(open.0),
                    amount(control.0),
                    amount(difference.0)
                )
            }
            Some(Evidence::ExpectedDatePassed(on)) => {
                format!("expected date {}, passed", date(scenario, *on))
            }
            Some(Evidence::PlannedDatePassed(on)) => format!("planned {}", date(scenario, *on)),
            Some(Evidence::Overdue { due, planned }) => match planned {
                Some(on) => format!(
                    "due {}; planned {}, passed",
                    date(scenario, *due),
                    date(scenario, *on)
                ),
                None => format!("due {}", date(scenario, *due)),
            },
            Some(Evidence::Discount {
                pay,
                by,
                saving,
                implied_annual_rate,
            }) => format!(
                "Pay {} by {}; saving {}; implied annual rate of not taking it {:.2}%",
                amount(pay.0),
                date(scenario, *by),
                amount(saving.0),
                implied_annual_rate
            ),
            Some(Evidence::Duplicate(DuplicateMatch::SameReference)) => {
                "same vendor reference".to_owned()
            }
            Some(Evidence::Duplicate(DuplicateMatch::SameAmountAndDate)) => {
                "same amount and bill date".to_owned()
            }
            Some(Evidence::MaybePaid) => {
                "same vendor and amount, on or after the bill date".to_owned()
            }
            Some(Evidence::CashStretches {
                buffer,
                stretches,
                trust_relevant,
                headroom,
            }) => cash_stretches(scenario, *buffer, stretches, *trust_relevant, *headroom),
            None => NONE.to_owned(),
        },
        // A mark on an item is what AP-PRIORITY-01 produced, so the item cites it beside the Rule
        // that raised it, as the marked Placements do.
        "Rules" => match item.priority {
            Some(_) => format!("{}, {}", item.rule.0, cashsaas_core::ap::AP_PRIORITY_01.0),
            None => item.rule.0.to_owned(),
        },
        _ => format!("? {header}"),
    }
}

/// `15 days after invoice date`, as the document writes a template's due rule. A payable
/// template's dates are bill dates.
fn due_rule(rule: DueRule, side: Side) -> String {
    let document = match side {
        Side::Receivable => "invoice",
        Side::Payable => "bill",
    };
    match rule {
        DueRule::DaysAfterDate(n) => format!("{n} days after {document} date"),
        DueRule::DaysAfterMonthEnd(n) => format!("{n} days after month end"),
        DueRule::DayOfCurrentMonth(n) => format!("day {n} of the {document} month"),
        DueRule::DayOfFollowingMonth(n) => format!("day {n} of the following month"),
    }
}

/// `net 30` or `2% 10 days, net 30`, as the document writes an invoice's terms.
fn terms(terms: &PaymentTerms) -> String {
    let net = match terms.due_rule {
        DueRule::DaysAfterDate(n) => format!("net {n}"),
        other => due_rule(other, Side::Receivable),
    };
    match &terms.discount {
        Some(d) => format!("{}% {} days, {net}", d.percent, d.days),
        None => net,
    }
}

/// A row of a facts table, rendered from the facts for the columns the builder can derive.
/// `None` means no renderer derives this column, or the key names no document: either way the
/// caller marks the cell unrendered rather than copying the document's own, so a fact the run
/// does not hold cannot pass by being compared with itself.
fn fact_cell(scenario: &Scenario, run: &ForecastRun, key: &str, header: &str) -> Option<String> {
    let facts = scenario.facts();
    let document: Option<&Document> = scenario.document_numbered(key);
    let payment = scenario.payment(key);
    let template = scenario.scheduled_template(key);
    let order = scenario.purchase_order_numbered(key);
    let line = scenario.account_line(key);
    let scheduled = scenario.scheduled_receipt_named(key);
    let account = scenario.account_named(key);
    let balance = account.and_then(|a| {
        facts
            .account_balances()
            .values()
            .filter(|b| b.account == a.id)
            .max_by_key(|b| b.as_of)
    });
    let id = document.map(|d| &d.id).or(payment.map(|p| &p.id));
    let counterparty = document
        .map(|d| &d.counterparty)
        .or_else(|| payment.and_then(|p| p.purpose.counterparty()))
        .or(template.map(|t| &t.counterparty))
        .or(order.map(|o| &o.vendor))
        .or_else(|| line.and_then(|l| l.counterparty.as_ref()))
        .or(scheduled.map(|r| &r.payer));
    let dated = document
        .map(|d| d.date)
        .or(payment.map(|p| p.date))
        .or(line.map(|l| l.date));
    let home = |document: &Document| {
        facts
            .ledger_settings()
            .get(&document.id.entity)
            .map(|s| s.home_currency)
    };
    let cell = match header {
        "Invoice" | "Bill" | "Item" | "Purchase order" | "Transaction" | "Document" => {
            key.to_owned()
        }
        // The account a transaction is posted to; in a balance table the account is the key.
        "Account" => match line {
            Some(l) => facts.accounts().get(&l.account).map(|a| a.name.clone())?,
            None => key.to_owned(),
        },
        // `Payer` is a receipt's counterparty, `Payee` a payment's; both name the contact.
        "Payee" | "Payer" => scenario.counterparty_name(counterparty?),
        "Receipt" | "Obligation" => key.to_owned(),
        "End date" => scheduled?
            .end
            .map_or(NONE.to_owned(), |on| date(scenario, on)),
        // A CASH facts table holds both sides at once, so the column is neither.
        "Customer" | "Vendor" | "Counterparty" => scenario.counterparty_name(counterparty?),
        "Discount date" => {
            let bill = document?;
            let discount = bill
                .terms
                .as_ref()
                .and_then(|t| facts.payment_terms().get(t))
                .and_then(|t| t.discount.as_ref())?;
            date(
                scenario,
                bill.date
                    .checked_add_days(chrono::Days::new(u64::from(discount.days)))?,
            )
        }
        "Vendor reference" => document?
            .vendor_reference
            .clone()
            .unwrap_or_else(|| NONE.to_owned()),
        "Planned payment date" => document?
            .expected_date
            .map_or(NONE.to_owned(), |on| date(scenario, on)),
        "Delivery date" => order?
            .delivery_date
            .map_or(NONE.to_owned(), |on| date(scenario, on)),
        "Vendor terms" => facts
            .counterparties()
            .get(counterparty?)
            .and_then(|c| c.payable_terms.as_ref())
            .and_then(|t| facts.payment_terms().get(t))
            .map_or(NONE.to_owned(), terms),
        "Classification" => match classify(facts, scenario.settings())
            .account(&account?.id)
            .class
        {
            AccountClass::ArControl => "AR control",
            AccountClass::UnbilledReceivable => "unbilled receivable",
            AccountClass::BadDebtWriteOff => "bad-debt write-off",
            AccountClass::ApControl => "AP control",
            AccountClass::Bank => "bank",
            AccountClass::CreditCard => "credit card",
            AccountClass::Unclassified => "unclassified",
        }
        .to_owned(),
        "As of" => date(scenario, balance?.as_of),
        "Balance" => amount(balance?.amount.home.0),
        "Kind" => match (payment, document) {
            (Some(p), _) if p.direction == Direction::Out => "prepayment paid",
            (Some(_), _) => "overpayment received",
            (None, Some(d)) if d.kind == DocumentKind::CreditNote => match d.side {
                Side::Payable => "vendor credit issued",
                Side::Receivable => "credit note issued",
            },
            _ => return None,
        }
        .to_owned(),
        "Unapplied" => amount(document::unapplied(facts, id?, run.as_of)?.0),
        "Template" => match document {
            Some(d) => d
                .template
                .as_ref()
                .map_or(NONE.to_owned(), |t| t.provider_id.clone()),
            None => template?.id.provider_id.clone(),
        },
        "Mode" => match template?.mode {
            TemplateMode::Automatic => "automatic",
            TemplateMode::AutomaticDraft => "automatic draft",
            TemplateMode::Reminder => "reminder",
            TemplateMode::Manual => "manual",
        }
        .to_owned(),
        "Amount" => amount(
            template
                .map(|t| t.amount.home.0)
                .or_else(|| scheduled.map(|r| r.amount.0))
                .or_else(|| line.map(|l| l.amount.home.0.abs()))?,
        ),
        "Every" => {
            let f = template
                .map(|t| t.frequency)
                .or(scheduled.map(|r| r.frequency))?;
            let unit = match f.unit {
                FrequencyUnit::Day => "day",
                FrequencyUnit::Week => "week",
                FrequencyUnit::Month => "month",
                FrequencyUnit::Year => "year",
            };
            if f.interval == 1 {
                unit.to_owned()
            } else {
                format!("{} {unit}s", f.interval)
            }
        }
        "Starting" => date(
            scenario,
            template.map(|t| t.start).or(scheduled.map(|r| r.start))?,
        ),
        "Due rule" => due_rule(template?.due_rule, template?.side),
        "Currency" => document?
            .total
            .transaction
            .as_ref()
            .map(|t| t.currency)
            .or_else(|| home(document?))?
            .code()
            .to_owned(),
        "Open (document)" => amount(document::open_own(facts, document?, run.as_of)),
        "Booked rate" => document?
            .total
            .transaction
            .as_ref()
            .map_or(NONE.to_owned(), |t| format!("{:.4}", t.rate)),
        "Terms" => document?
            .terms
            .as_ref()
            .and_then(|t| facts.payment_terms().get(t))
            .map_or(NONE.to_owned(), terms),
        "Status" => match (document.map(|d| d.status), order.map(|o| o.status)) {
            (Some(status), _) => match status {
                cashsaas_core::facts::DocumentStatus::Draft => "draft",
                cashsaas_core::facts::DocumentStatus::AwaitingApproval => "awaiting approval",
                cashsaas_core::facts::DocumentStatus::Posted => "posted",
                cashsaas_core::facts::DocumentStatus::Voided => "voided",
                cashsaas_core::facts::DocumentStatus::Deleted => "deleted",
            },
            (None, Some(status)) => match status {
                PurchaseOrderStatus::Draft => "draft",
                PurchaseOrderStatus::AwaitingApproval => "awaiting approval",
                PurchaseOrderStatus::Authorised => "authorised",
                PurchaseOrderStatus::Billed => "billed",
                PurchaseOrderStatus::Closed => "closed",
                PurchaseOrderStatus::Deleted => "deleted",
            },
            (None, None) => return None,
        }
        .to_owned(),
        "Dated" => date(scenario, dated?),
        "Due" => date(scenario, document::due_date(document?)?),
        "Total" => amount(
            document
                .map(|d| d.total.home.0)
                .or_else(|| order.map(|o| o.total.home.0))?,
        ),
        "Tax in total" => amount(document?.tax.home.0),
        "Open" | "Open (CAD)" => amount(document::open_home(facts, document?, run.as_of).0),
        "Expected payment date" => document?
            .expected_date
            .map_or(NONE.to_owned(), |d| date(scenario, d)),
        "Paid" => ar::collected_on(facts, document?, run.as_of)
            .map_or(NONE.to_owned(), |d| date(scenario, d)),
        "Age at payment" => {
            let document = document?;
            let due = document::due_date(document)?;
            ar::collected_on(facts, document, run.as_of)
                .map_or(NONE.to_owned(), |paid| age(document::age(paid, due)))
        }
        "Age at run" => age(document::age(run.as_of, document::due_date(document?)?)),
        "Days after invoice date" => {
            let document = document?;
            ar::collected_on(facts, document, run.as_of).map_or(NONE.to_owned(), |paid| {
                (paid - document.date).num_days().to_string()
            })
        }
        _ => return None,
    };
    Some(cell)
}

fn how_placements(header: &str) -> Match {
    match header {
        "Week" | "Amount" | "Amount (CAD)" | "Comparable" => Match::Loose,
        "Rules" => Match::Rules,
        _ => Match::Exact,
    }
}

fn how_items(header: &str) -> Match {
    match header {
        "Rules" => Match::Rules,
        _ => Match::Exact,
    }
}

fn render_placements(scenario: &Scenario, run: &ForecastRun, headers: &[String]) -> Table {
    let family = scenario.name.split('-').next().unwrap_or_default();
    Table {
        headers: headers.to_vec(),
        rows: run
            .placements
            .iter()
            .filter(|p| p.rule.family() == family)
            .map(|p| {
                headers
                    .iter()
                    .map(|h| placement_cell(scenario, p, h, run.as_of))
                    .collect()
            })
            .collect(),
    }
}

/// The **Inputs from other Families** table: every Placement whose Rule belongs to a different
/// Family from this Scenario's own, so the roll-forward can be checked (Conventions). Its headers
/// differ per Scenario (`Placement` first in S01–S03 and S08, `Output` in S07, `Entity` in S09),
/// so `check` dispatches it by the presence of a `Rules` column, which no facts table carries,
/// rather than by its first header.
fn render_other_families(scenario: &Scenario, run: &ForecastRun, headers: &[String]) -> Table {
    let family = scenario.name.split('-').next().unwrap_or_default();
    Table {
        headers: headers.to_vec(),
        rows: run
            .placements
            .iter()
            .filter(|p| p.rule.family() != family)
            .map(|p| {
                headers
                    .iter()
                    .map(|h| match h.as_str() {
                        // A Placement's own amount is a magnitude; CASH-ROLL-01's direction is
                        // what this table previews, so the sign here is the direction's, not
                        // whatever sign the source amount happened to carry.
                        "Amount" | "Amount (CAD)" => signed_amount(p),
                        other => placement_cell(scenario, p, other, run.as_of),
                    })
                    .collect()
            })
            .collect(),
    }
}

/// `+7,000.00` for money in, `−9,500.00` for money out (CASH-ROLL-01's direction, Conventions).
fn signed_amount(placement: &Placement) -> String {
    let Some(value) = placement.amount else {
        return NONE.to_owned();
    };
    match placement.outcome {
        Outcome::Placed {
            direction: Direction::In,
            ..
        } => format!("+{}", amount(value.0.abs())),
        Outcome::Placed {
            direction: Direction::Out,
            ..
        } => amount(-value.0.abs()),
        _ => amount(value.0),
    }
}

fn render_items(scenario: &Scenario, run: &ForecastRun, headers: &[String]) -> Table {
    let family = scenario.name.split('-').next().unwrap_or_default();
    Table {
        headers: headers.to_vec(),
        rows: run
            .decision_items
            .iter()
            .filter(|i| i.rule.family() == family)
            .map(|i| headers.iter().map(|h| item_cell(scenario, i, h)).collect())
            .collect(),
    }
}

/// CASH-ROLL-01's weeks, with CASH-CONF-01's shares. The documents list only weeks with
/// Placements: an unlisted week opens and closes at the previous week's closing cash and has no
/// shares (Q153), so the renderer emits the same subset rather than all thirteen.
fn render_weeks(run: &ForecastRun, headers: &[String]) -> Table {
    let percent = |value: Decimal| format!("{}%", value.round_dp(0).normalize());
    Table {
        headers: headers.to_vec(),
        rows: run
            .entities
            .iter()
            .flat_map(|entity| entity.weeks.iter())
            .filter(|week| week.confidence.is_some())
            .map(|week| {
                headers
                    .iter()
                    .map(|h| match h.as_str() {
                        "Week" => format!("W{}", week.week.0),
                        "Opens" => amount(week.opening.0),
                        "Receipts" if week.receipts.0.is_zero() => NONE.to_owned(),
                        "Receipts" => amount(week.receipts.0),
                        "Payments" if week.payments.0.is_zero() => NONE.to_owned(),
                        "Payments" => amount(week.payments.0),
                        "Closes" => amount(week.closing.0),
                        "Firm" => week.confidence.map_or(NONE.to_owned(), |c| percent(c.firm)),
                        "Estimated" => week
                            .confidence
                            .map_or(NONE.to_owned(), |c| percent(c.estimated)),
                        other => format!("? {other}"),
                    })
                    .collect()
            })
            .collect(),
    }
}

/// The Low Point line of a section: the figure and the week it names. It is prose because no table
/// holds it, and it is compared for the reason `provisional()` is — a figure that reaches the
/// owner unverified is the failure Q192 exists to stop.
fn low_point_line(section: &str) -> Option<(String, String)> {
    for line in section.lines() {
        let line = line.replace("**", "");
        let Some(rest) = line
            .trim_start()
            .trim_start_matches("- ")
            .strip_prefix("Low Point:")
        else {
            continue;
        };
        let words = tokens(rest);
        let value = words.iter().find(|w| w.contains('.'))?;
        let week = words
            .iter()
            .find(|w| w.starts_with('W') && w[1..].chars().all(|c| c.is_ascii_digit()))?;
        return Some((value.clone(), week.clone()));
    }
    None
}

/// `Stretch W6` for a single week, `Stretch W3–W6` for a range.
fn stretch_clause(stretch: &Stretch) -> String {
    let range = if stretch.first == stretch.last {
        format!("W{}", stretch.first.0)
    } else {
        format!("W{}–W{}", stretch.first.0, stretch.last.0)
    };
    format!("Stretch {range}, lowest {}", amount(stretch.lowest.0))
}

/// CASH-SHORT-01's and CASH-BUFFER-01's Evidence: every stretch, the buffer for a below-buffer
/// item, the trust obligations inside each stretch when the Entity has any elsewhere in the run
/// (Q276), and the Headroom available. The reading, confirmed by CASH-S04 and CASH-S08: when no
/// trust obligation is relevant to this Entity at all, the clauses read as short sentences; when
/// one is, the whole evidence is one sentence naming, for each stretch, whether it falls inside.
fn cash_stretches(
    scenario: &Scenario,
    buffer: Option<HomeAmount>,
    stretches: &[Stretch],
    trust_relevant: bool,
    headroom: Option<HomeAmount>,
) -> String {
    if trust_relevant {
        let mut parts: Vec<String> = Vec::new();
        if let Some(buffer) = buffer {
            parts.push(format!("Buffer {}", amount(buffer.0)));
        }
        for stretch in stretches {
            parts.push(stretch_clause(stretch));
            parts.push(if stretch.trust.is_empty() {
                "no trust obligations inside".to_owned()
            } else {
                format!(
                    "trust obligations inside: {}",
                    stretch
                        .trust
                        .iter()
                        .map(|id| number(scenario, id))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            });
        }
        parts.push(match headroom {
            Some(headroom) => format!("Headroom {}", amount(headroom.0)),
            None => "no Headroom".to_owned(),
        });
        parts.join("; ")
    } else {
        let mut parts: Vec<String> = Vec::new();
        if let Some(buffer) = buffer {
            parts.push(format!("Buffer {}.", amount(buffer.0)));
        }
        for stretch in stretches {
            parts.push(format!("{}.", stretch_clause(stretch)));
        }
        parts.push(match headroom {
            Some(headroom) => format!("Headroom {}.", amount(headroom.0)),
            None => "No Headroom.".to_owned(),
        });
        parts.join(" ")
    }
}

/// The total and the on-or-before-Low-Point-week size of the Placements a Decision Item
/// concerns, by matching Subject — CASH-ORDER-01's own ranking criteria, read again here to
/// explain a row rather than to sort it.
fn item_amounts(run: &ForecastRun, subject: &Subject, low: Option<Week>) -> (Decimal, Decimal) {
    let mut total = Decimal::ZERO;
    let mut landing = Decimal::ZERO;
    for p in run.placements.iter().filter(|p| &p.subject == subject) {
        let Outcome::Placed { week, .. } = p.outcome else {
            continue;
        };
        let Some(value) = p.amount else { continue };
        let size = value.0.abs();
        total = total.saturating_add(size);
        if low.is_some_and(|low| week <= low) {
            landing = landing.saturating_add(size);
        }
    }
    (total, landing)
}

/// CASH-ORDER-01's rationale for one row of the queue, S08's `Why here` column: CASH's own
/// findings show their own Evidence; every other Family's item shows the criterion that placed
/// it — a trust or marked-vendor priority, or the cash it concerns against the Low Point week.
fn why_here(scenario: &Scenario, run: &ForecastRun, item: &DecisionItem) -> String {
    if matches!(item.evidence, Some(Evidence::CashStretches { .. })) {
        return item_cell(scenario, item, "Evidence");
    }
    match item.priority {
        Some(Priority::GovernmentTrust) => "held in trust for a government".to_owned(),
        Some(Priority::CriticalVendor) => "critical vendor".to_owned(),
        Some(Priority::SecuredLender) => "secured lender".to_owned(),
        None => {
            let low = item
                .subject
                .entity()
                .and_then(|e| run.entities.iter().find(|ef| &ef.entity == e))
                .and_then(|ef| ef.low_point);
            let (total, landing) = item_amounts(run, &item.subject, low.map(|lp| lp.week));
            match low {
                Some(lp) if !landing.is_zero() => {
                    format!("{} lands on or before W{}", amount(landing), lp.week.0)
                }
                Some(lp) => format!("nothing lands by W{}; total {}", lp.week.0, amount(total)),
                None => format!("total {}", amount(total)),
            }
        }
    }
}

/// CASH-ORDER-01's queue: every Family's Decision Items, not one Family's, numbered from 1 in the
/// order the run holds them. `#` is a compared cell, so a wrong order fails. Every row's class
/// and position are CASH-ORDER-01's doing, whichever Family's own Rule raised the item, so its
/// `Rules` cell always cites CASH-ORDER-01 beside the item's own Rule.
fn render_queue(scenario: &Scenario, run: &ForecastRun, headers: &[String]) -> Table {
    Table {
        headers: headers.to_vec(),
        rows: run
            .decision_items
            .iter()
            .enumerate()
            .map(|(at, item)| {
                headers
                    .iter()
                    .map(|h| match h.as_str() {
                        "#" => (at + 1).to_string(),
                        "Why here" => why_here(scenario, run, item),
                        "Rules" => format!("{}, CASH-ORDER-01", item_cell(scenario, item, h)),
                        other => item_cell(scenario, item, other),
                    })
                    .collect()
            })
            .collect(),
    }
}

/// The Provisional line of a section: whether the run is Provisional and the Rules cited.
fn provisional(section: &str) -> Option<(bool, Vec<String>)> {
    for line in section.lines() {
        let line = line.replace("**", "");
        if let Some(rest) = line.trim_start_matches("- ").strip_prefix("Provisional:") {
            let yes = rest.trim_start().starts_with("yes");
            let cited = tokens(rest).into_iter().filter(|t| is_rule_id(t)).collect();
            return Some((yes, cited));
        }
        if line.contains("not Provisional") {
            return Some((false, Vec::new()));
        }
    }
    None
}

/// Checks a Scenario's facts and run against its section of the Family's Scenario document, and
/// that every declared Rule ID is cited by an output. A failure shows two tables, never code.
pub fn check(scenario: &Scenario, run: &ForecastRun, doc: &str, declared: &[&str]) {
    let name = &scenario.name;
    let section =
        section(doc, name).unwrap_or_else(|| panic!("{name}: no section in the Scenario document"));
    let (facts_part, expected_part) = section
        .split_once("**Expected**")
        .unwrap_or_else(|| panic!("{name}: no **Expected** in its section"));

    let facts_tables = tables(facts_part);
    assert!(
        !facts_tables.is_empty(),
        "{name}: its facts are prose, with no table to compare. Every fact a Rule reads belongs \
         in a facts table."
    );
    for (i, table) in facts_tables.iter().enumerate() {
        let what = format!("{name}: facts table {}", i + 1);
        // The Inputs from other Families table (Conventions): the only facts-part table with a
        // Rules column, since a raw Canonical Fact never cites one. It is Placements, not facts,
        // so it is rendered and compared as the Output table is.
        if table.headers.iter().any(|h| h == "Rules") {
            let actual = render_other_families(scenario, run, &table.headers);
            if let Err(message) = compare(&what, table, &actual, &how_placements) {
                panic!("{message}");
            }
            continue;
        }
        let rendered: Vec<Vec<Option<String>>> = table
            .rows
            .iter()
            .map(|row| {
                let key = row.first().cloned().unwrap_or_default();
                table
                    .headers
                    .iter()
                    .map(|h| fact_cell(scenario, run, &key, h))
                    .collect()
            })
            .collect();
        let actual = Table {
            headers: table.headers.clone(),
            rows: table
                .rows
                .iter()
                .enumerate()
                .map(|(r, row)| {
                    table
                        .headers
                        .iter()
                        .enumerate()
                        .zip(row)
                        .map(|((c, h), cell)| match &rendered[r][c] {
                            Some(value) => value.clone(),
                            None if derived_column(&rendered, c) || column_has_digit(table, c) => {
                                format!("? {h} not rendered")
                            }
                            None => cell.clone(),
                        })
                        .collect()
                })
                .collect(),
        };
        if let Err(message) = compare(&what, table, &actual, &|_| Match::Exact) {
            panic!("{message}");
        }
    }

    let expected = tables(expected_part);
    let mut saw_outputs = false;
    let mut saw_items = false;
    for table in &expected {
        let (what, actual, how): (&str, Table, fn(&str) -> Match) =
            match table.headers.first().map(String::as_str) {
                Some("Output") => {
                    saw_outputs = true;
                    (
                        "outputs",
                        render_placements(scenario, run, &table.headers),
                        how_placements,
                    )
                }
                Some("Decision Item") => {
                    saw_items = true;
                    (
                        "Decision Items",
                        render_items(scenario, run, &table.headers),
                        how_items,
                    )
                }
                Some("#") => {
                    saw_items = true;
                    (
                        "the queue",
                        render_queue(scenario, run, &table.headers),
                        how_items,
                    )
                }
                Some("Week") => ("the weeks", render_weeks(run, &table.headers), how_items),
                _ => continue,
            };
        if let Err(message) = compare(&format!("{name}: {what}"), table, &actual, &how) {
            panic!("{message}");
        }
    }
    assert!(saw_outputs, "{name}: the section has no Output table");
    if !saw_items {
        let headers = ["Decision Item", "Subject", "Acted on by", "Rules"].map(str::to_owned);
        let none = Table {
            headers: headers.to_vec(),
            rows: Vec::new(),
        };
        let actual = render_items(scenario, run, &headers);
        if let Err(message) = compare(
            &format!("{name}: Decision Items"),
            &none,
            &actual,
            &how_items,
        ) {
            panic!("{message}");
        }
    }

    // CASH-LOW-01's figure lives in a prose line, so it is compared like the Provisional line.
    if let Some((value, week)) = low_point_line(section) {
        let found = run.entities.iter().find_map(|entity| {
            entity
                .low_point
                .map(|low| (amount(low.amount.0), format!("W{}", low.week.0)))
        });
        assert!(
            found.as_ref() == Some(&(value.clone(), week.clone())),
            "{name}: the document says the Low Point is {value} in {week}; the run says {found:?}"
        );
    }

    let (yes, cited) = provisional(section)
        .unwrap_or_else(|| panic!("{name}: the section says nothing about Provisional"));
    // Both ways: a reason the document does not cite is as wrong as one the run does not raise.
    // On a "no" line there are no reasons, so a Rule named there is prose saying why the run is
    // not Provisional — CASH-S06's "no, as CASH-WEEK-01 marks" — and not a list to compare.
    let reasons: BTreeSet<&str> = run.provisional.iter().map(|r| r.rule.0).collect();
    let cited: BTreeSet<&str> = if yes {
        cited.iter().map(String::as_str).collect()
    } else {
        BTreeSet::new()
    };
    assert!(
        yes == run.is_provisional() && cited == reasons,
        "{name}: the document says Provisional {} citing {:?}; the run says {} citing {:?}",
        if yes { "yes" } else { "no" },
        cited,
        if run.is_provisional() { "yes" } else { "no" },
        reasons
    );

    // A Rule is answered by an output: a Placement, a Decision Item, or the per-Entity forecast,
    // which is what CASH-ROLL-01, CASH-CONF-01 and CASH-LOW-01 produce instead of a Placement.
    let cited_by_run = |rule: &str| {
        run.placements
            .iter()
            .any(|p| p.rules.iter().any(|r| r.0 == rule))
            || run.decision_items.iter().any(|i| i.rule.0 == rule)
            || run
                .entities
                .iter()
                .any(|e| e.rules.iter().any(|r| r.0 == rule))
    };
    let uncited: Vec<&&str> = declared.iter().filter(|r| !cited_by_run(r)).collect();
    assert!(
        uncited.is_empty(),
        "{name}: declared Rules not cited by any output: {uncited:?}"
    );
}
