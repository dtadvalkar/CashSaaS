//! One test per Scenario in `docs/scenarios/ar.md`. Each declares the Rule IDs it covers as string
//! literals, so `grep` answers coverage (ADR-0021, Q206), and checks its facts and outputs against
//! the approved document's own tables (Q259).

use cashsaas_core::settings::AccountClass;
use scenarios::Scenario;
use scenarios::render::check;

const DOC: &str = include_str!("../../docs/scenarios/ar.md");

#[test]
fn ar_s01_open_draft_voided_zero_total_and_paid_invoices() {
    let rules = ["AR-OPEN-01", "AR-OPEN-02", "AR-OPEN-04", "AR-TIME-02"];
    let mut s = Scenario::new("AR-S01");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    let customer = "Harbourview Strata Corp.";
    s.invoice("INV-1001", customer)
        .due("10-20")
        .total("5,250.00")
        .tax("250.00")
        .add();
    s.invoice("INV-1002", customer)
        .due("11-05")
        .total("2,100.00")
        .tax("100.00")
        .draft()
        .add();
    s.invoice("INV-1003", customer)
        .due("09-01")
        .total("0.00")
        .voided()
        .add();
    s.invoice("INV-1004", customer)
        .due("10-15")
        .total("1,050.00")
        .tax("50.00")
        .paid("09-28")
        .add();
    s.invoice("INV-1005", customer)
        .due("10-20")
        .total("0.00")
        .add();
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ar_s02_owner_entered_expected_dates_current_and_stale() {
    let rules = ["AR-TIME-01", "AR-TIME-04", "AR-COLLECT-01"];
    let mut s = Scenario::new("AR-S02");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    let customer = "Alder Creek Homes";
    s.invoice("INV-2001", customer)
        .due("09-15")
        .total("3,150.00")
        .expected("10-30")
        .add();
    s.invoice("INV-2002", customer)
        .due("09-01")
        .total("1,575.00")
        .expected("10-01")
        .add();
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ar_s03_collection_history_times_overdue_and_not_yet_due_invoices() {
    let rules = ["AR-TIME-02", "AR-COLLECT-01"];
    let mut s = Scenario::new("AR-S03");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    let cedar = "Cedar Point Property Mgmt.";
    let westgate = "Westgate Business Park";
    let riverbend = "Riverbend Elementary PAC";
    s.invoice("H-3101", cedar)
        .due("06-01")
        .total("2,000.00")
        .paid("06-06")
        .add();
    s.invoice("H-3102", cedar)
        .due("06-15")
        .total("2,000.00")
        .paid("06-25")
        .add();
    s.invoice("H-3103", westgate)
        .due("07-01")
        .total("2,000.00")
        .paid("07-21")
        .add();
    s.invoice("H-3104", westgate)
        .due("07-15")
        .total("2,000.00")
        .paid("08-14")
        .add();
    s.invoice("H-3105", riverbend)
        .due("08-01")
        .total("2,000.00")
        .paid("09-10")
        .add();
    s.invoice("INV-3001", cedar)
        .due("10-17")
        .total("4,200.00")
        .add();
    s.invoice("INV-3002", westgate)
        .due("09-22")
        .total("2,625.00")
        .add();
    s.invoice("INV-3003", riverbend)
        .due("09-02")
        .total("1,050.00")
        .add();
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ar_s04_history_says_invoices_this_old_are_not_collected() {
    let rules = ["AR-TIME-03", "AR-COLLECT-01", "AR-WRITEOFF-01"];
    let mut s = Scenario::new("AR-S04");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    let pinecrest = "Pinecrest Builders Inc.";
    let sandpiper = "Sandpiper Motel";
    s.invoice("H-4101", pinecrest)
        .due("01-15")
        .total("1,500.00")
        .credited("07-14")
        .add();
    s.invoice("H-4102", pinecrest)
        .due("02-15")
        .total("1,800.00")
        .credited("08-14")
        .add();
    s.invoice("H-4103", pinecrest)
        .due("03-15")
        .total("1,200.00")
        .credited("09-11")
        .add();
    s.invoice("H-4104", sandpiper)
        .due("04-01")
        .total("900.00")
        .paid("04-11")
        .add();
    s.invoice("H-4105", sandpiper)
        .due("05-01")
        .total("900.00")
        .paid("05-21")
        .add();
    s.invoice("INV-4001", pinecrest)
        .due("07-09")
        .total("2,310.00")
        .add();
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ar_s05_history_says_collection_falls_beyond_the_horizon() {
    let rules = ["AR-TIME-03", "AR-COLLECT-01"];
    let mut s = Scenario::new("AR-S05");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    let oakview = "Oakview Senior Living";
    s.invoice("H-5101", oakview)
        .due("01-10")
        .total("2,400.00")
        .paid("05-10")
        .add();
    s.invoice("H-5102", oakview)
        .due("02-10")
        .total("2,400.00")
        .paid("06-20")
        .add();
    s.invoice("H-5103", oakview)
        .due("03-10")
        .total("2,400.00")
        .paid("07-28")
        .add();
    s.invoice("INV-5001", oakview)
        .due("09-27")
        .total("2,400.00")
        .add();
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ar_s06_intercompany_foreign_currency_and_early_payment_discounts() {
    let rules = ["AR-OPEN-03", "AR-TIME-02", "AR-FX-01", "AR-DISC-01"];
    let mut s = Scenario::new("AR-S06");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    s.intercompany("Birch Hill Nursery Ltd.", "Birch Hill Nursery Ltd.");
    s.invoice("INV-6001", "Birch Hill Nursery Ltd.")
        .dated("09-16")
        .due("10-16")
        .total("2,000.00")
        .terms("net 30")
        .add();
    s.invoice("INV-6002", "Summit Outdoor Supply Inc.")
        .currency("USD", "1.3600")
        .dated("09-27")
        .due("10-27")
        .total("1,000.00")
        .terms("net 30")
        .add();
    s.invoice("INV-6003", "Lakeshore Condominium Assoc.")
        .dated("10-06")
        .due("11-05")
        .total("3,000.00")
        .terms("2% 10 days, net 30")
        .add();
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ar_s07_unapplied_credit_and_a_customer_credit_balance() {
    let rules = ["AR-TIME-02", "AR-UNAPPLIED-01", "AR-UNAPPLIED-02"];
    let mut s = Scenario::new("AR-S07");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    let cypress = "Cypress Bay Dental";
    s.invoice("INV-7001", cypress)
        .due("10-10")
        .total("1,000.00")
        .add();
    s.invoice("INV-7002", cypress)
        .due("10-24")
        .total("1,500.00")
        .add();
    s.overpayment("OP-7001", cypress, "09-29", "400.00");
    s.credit_note("CN-7101", "Fernwood Café", "09-18", "300.00");
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ar_s08_scheduled_invoices_already_generated_still_to_come_and_missed() {
    let rules = ["AR-OPEN-01", "AR-TIME-02", "AR-SCHED-01", "AR-SCHED-02"];
    let mut s = Scenario::new("AR-S08");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    let granite = "Granite Peak Fitness";
    s.template("T1", granite)
        .amount("840.00")
        .starting("10-15")
        .due_after(15)
        .add();
    s.template("T2", "Northshore Rowing Club")
        .reminder()
        .amount("525.00")
        .starting("08-01")
        .due_after(30)
        .add();
    s.invoice("INV-8001", granite)
        .dated("10-15")
        .due("10-30")
        .total("840.00")
        .template("T1")
        .add();
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ar_s09_accrued_revenue_and_ar_that_does_not_agree_with_the_ledger() {
    let rules = ["AR-TIME-02", "AR-UNBILLED-01", "AR-TIE-01"];
    let mut s = Scenario::new("AR-S09");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    s.map_account("Accounts Receivable", AccountClass::ArControl);
    s.map_account("Unbilled Revenue", AccountClass::UnbilledReceivable);
    let kingsway = "Kingsway Medical Clinic";
    s.invoice("INV-9001", kingsway)
        .dated("09-19")
        .due("10-19")
        .total("6,000.00")
        .add();
    s.invoice("INV-9002", kingsway)
        .dated("09-26")
        .due("10-26")
        .total("6,000.00")
        .add();
    s.balance("Accounts Receivable", "09-30", "12,350.00");
    s.balance("Unbilled Revenue", "09-30", "4,200.00");
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ar_s10_an_invoice_fully_offset_by_credit() {
    let rules = ["AR-TIME-02", "AR-UNAPPLIED-01"];
    let mut s = Scenario::new("AR-S10");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    let cypress = "Cypress Bay Dental";
    s.invoice("INV-10001", cypress)
        .due("10-09")
        .total("700.00")
        .add();
    s.invoice("INV-10002", cypress)
        .due("10-23")
        .total("1,200.00")
        .add();
    s.credit_note("CN-10101", cypress, "09-30", "1,000.00");
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ar_s11_a_scheduled_invoice_timed_by_collection_history() {
    let rules = ["AR-TIME-02", "AR-TIME-03", "AR-SCHED-01"];
    let mut s = Scenario::new("AR-S11");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    let granite = "Granite Peak Fitness";
    s.invoice("H-11101", "Cedar Point Property Mgmt.")
        .dated("05-31")
        .due("06-30")
        .total("1,000.00")
        .terms("net 30")
        .paid("07-10")
        .add();
    s.invoice("H-11102", "Westgate Business Park")
        .dated("07-01")
        .due("07-31")
        .total("1,000.00")
        .terms("net 30")
        .paid("08-20")
        .add();
    s.invoice("H-11103", granite)
        .dated("08-01")
        .due("08-31")
        .total("1,000.00")
        .terms("net 30")
        .paid("09-05")
        .add();
    s.invoice("H-11104", "Riverbend Elementary PAC")
        .dated("08-16")
        .due("09-15")
        .total("1,000.00")
        .terms("net 30")
        .paid("10-05")
        .add();
    s.template("T1", granite)
        .amount("1,000.00")
        .starting("10-20")
        .due_after(30)
        .add();
    let run = s.run();
    check(&s, &run, DOC, &rules);
}
