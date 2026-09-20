//! One test per Scenario in `docs/scenarios/ap.md`. Each declares the Rule IDs it covers as string
//! literals, so `grep` answers coverage (ADR-0021, Q206), and checks its facts and outputs against
//! the approved document's own tables (Q259).

use cashsaas_core::settings::AccountClass;
use chrono::Weekday;
use scenarios::Scenario;
use scenarios::render::check;

const DOC: &str = include_str!("../../docs/scenarios/ap.md");

#[test]
fn ap_s01_open_draft_voided_zero_total_paid_and_card_paid_bills() {
    let rules = ["AP-OPEN-01", "AP-OPEN-02", "AP-OPEN-04", "AP-TIME-02"];
    let mut s = Scenario::new("AP-S01");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    s.bill("BILL-1001", "Brandt Tractor Ltd.")
        .due("10-20")
        .total("3,150.00")
        .tax("150.00")
        .add();
    s.bill("BILL-1002", "Western Turf Farms")
        .due("10-30")
        .total("1,575.00")
        .tax("75.00")
        .draft()
        .add();
    s.bill("BILL-1003", "Home Depot Pro")
        .due("09-15")
        .total("0.00")
        .voided()
        .add();
    s.bill("BILL-1004", "Telus Business")
        .due("10-15")
        .total("450.00")
        .tax("21.43")
        .paid("10-01")
        .add();
    s.bill("BILL-1005", "Nutrien Ag Solutions")
        .due("10-25")
        .total("840.00")
        .tax("40.00")
        .paid_by_card("10-02")
        .add();
    s.bill("BILL-1006", "Brandt Tractor Ltd.")
        .due("10-09")
        .total("0.00")
        .add();
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ap_s02_owner_planned_payment_dates_current_and_stale() {
    let rules = ["AP-TIME-01", "AP-TIME-02", "AP-TIME-03"];
    let mut s = Scenario::new("AP-S02");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    s.bill("BILL-2001", "Stihl Canada")
        .due("11-05")
        .total("2,400.00")
        .planned("10-28")
        .add();
    s.bill("BILL-2002", "Brandt Tractor Ltd.")
        .due("10-30")
        .total("1,800.00")
        .planned("10-02")
        .add();
    s.bill("BILL-2003", "Western Turf Farms")
        .due("09-25")
        .total("1,250.00")
        .planned("10-05")
        .add();
    s.bill("BILL-2004", "Home Depot Pro")
        .due("09-30")
        .total("680.00")
        .add();
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ap_s03_a_weekly_pay_run() {
    let rules = ["AP-TIME-01", "AP-TIME-02", "AP-TIME-03", "AP-RUN-01"];
    let mut s = Scenario::new("AP-S03");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    s.pay_run(Weekday::Thu);
    s.bill("BILL-3001", "Stihl Canada")
        .due("10-20")
        .total("2,260.00")
        .add();
    s.bill("BILL-3002", "Brandt Tractor Ltd.")
        .due("10-22")
        .total("3,900.00")
        .add();
    s.bill("BILL-3003", "Telus Business")
        .due("10-01")
        .total("450.00")
        .add();
    s.bill("BILL-3004", "Western Turf Farms")
        .due("11-10")
        .total("1,100.00")
        .planned("11-06")
        .add();
    s.bill("BILL-3005", "Home Depot Pro")
        .due("10-12")
        .total("915.00")
        .add();
    s.bill("BILL-3006", "Nutrien Ag Solutions")
        .due("10-07")
        .total("1,340.00")
        .add();
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ap_s05_a_committed_purchase_a_foreign_bill_and_an_intercompany_bill() {
    let rules = ["AP-OPEN-03", "AP-TIME-02", "AP-PO-01", "AP-FX-01"];
    let mut s = Scenario::new("AP-S05");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    s.vendor_terms("Brandt Tractor Ltd.", "net 30");
    s.purchase_order("PO-5001", "Brandt Tractor Ltd.")
        .total("18,500.00")
        .delivery("10-20")
        .add();
    s.purchase_order("PO-5002", "Stihl Canada")
        .total("4,200.00")
        .add();
    s.bill("BILL-5101", "Pacific Growers Supply LLC")
        .currency("USD", "1.3700")
        .due("10-29")
        .total("2,000.00")
        .add();
    s.bill("BILL-5102", "Birch Hill Nursery Ltd.")
        .due("10-16")
        .total("3,000.00")
        .add();
    s.intercompany("Birch Hill Nursery Ltd.", "Birch Hill Nursery Ltd.");
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ap_s07_unapplied_vendor_credit_and_a_supplier_prepayment() {
    let rules = ["AP-TIME-02", "AP-UNAPPLIED-01", "AP-UNAPPLIED-02"];
    let mut s = Scenario::new("AP-S07");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    let western = "Western Turf Farms";
    s.bill("BILL-7001", western)
        .due("10-12")
        .total("900.00")
        .add();
    s.bill("BILL-7002", western)
        .due("10-26")
        .total("1,400.00")
        .add();
    s.vendor_credit("VC-7101", western, "09-28", "1,200.00");
    s.prepayment("PP-7201", "Kubota Canada Ltd.", "10-01", "2,500.00");
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ap_s10_open_ap_that_does_not_agree_with_the_ledger() {
    let rules = ["AP-OPEN-01", "AP-TIME-02", "AP-TIE-01"];
    let mut s = Scenario::new("AP-S10");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    s.map_account("Accounts Payable", AccountClass::ApControl);
    s.bill("BILL-10001", "Brandt Tractor Ltd.")
        .dated("09-10")
        .due("10-15")
        .total("4,000.00")
        .add();
    s.bill("BILL-10002", "Stihl Canada")
        .dated("09-22")
        .due("10-22")
        .total("2,500.00")
        .add();
    s.bill("BILL-10003", "Home Depot Pro")
        .dated("09-15")
        .due("10-15")
        .total("1,000.00")
        .paid("10-03")
        .add();
    s.balance("Accounts Payable", "09-30", "7,850.00");
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ap_s04_scheduled_bills_generated_paid_by_card_still_to_come_and_reminder_only() {
    let rules = ["AP-OPEN-01", "AP-TIME-02", "AP-SCHED-01"];
    let mut s = Scenario::new("AP-S04");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    s.map_account("RBC Visa Business", AccountClass::CreditCard);
    let telus = "Telus Business";
    s.bill_template("T1", telus)
        .amount("450.00")
        .starting("06-10")
        .due_after(20)
        .add();
    s.bill("BILL-4001", telus)
        .dated("10-10")
        .due("10-30")
        .total("450.00")
        .template("T1")
        .add();
    s.bill_template("T2", "Waste Connections of Canada")
        .amount("610.00")
        .starting("01-08")
        .due_after(10)
        .add();
    s.spend(
        "CARD-4101",
        "RBC Visa Business",
        "Waste Connections of Canada",
        "10-06",
        "610.00",
    );
    s.bill_template("T3", "Kubota Canada Ltd.")
        .amount("1,280.00")
        .starting("01-01")
        .due_after(30)
        .reminder()
        .add();
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ap_s08_government_trust_critical_vendors_and_secured_lenders_are_marked() {
    let rules = ["AP-TIME-02", "AP-TIME-03", "AP-PRIORITY-01"];
    let mut s = Scenario::new("AP-S08");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    s.government_trust("Receiver General for Canada");
    s.critical_vendor("Nutrien Ag Solutions");
    s.secured_lender("Kubota Credit Corporation Canada");
    s.bill("RG-0915", "Receiver General for Canada")
        .due("10-15")
        .total("3,400.00")
        .add();
    s.bill("NAS-8101", "Nutrien Ag Solutions")
        .due("09-30")
        .total("1,200.00")
        .add();
    s.bill("KCC-8102", "Kubota Credit Corporation Canada")
        .due("10-25")
        .total("1,850.00")
        .add();
    s.bill("HD-8103", "Home Depot Pro")
        .due("10-21")
        .total("700.00")
        .add();
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ap_s09_possible_duplicates_and_a_bill_possibly_paid_another_way() {
    let rules = ["AP-TIME-02", "AP-DUP-01", "AP-PAID-01"];
    let mut s = Scenario::new("AP-S09");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    s.map_account("RBC Business Chequing", AccountClass::Bank);
    s.bill("BILL-9001", "Home Depot Pro")
        .reference("HD-55120")
        .dated("09-28")
        .due("10-28")
        .total("1,130.00")
        .add();
    s.bill("BILL-9002", "Home Depot Pro")
        .reference("HD-55120")
        .dated("09-29")
        .due("10-29")
        .total("1,130.00")
        .add();
    s.bill("BILL-9003", "Stihl Canada")
        .reference("ST-7781")
        .dated("10-01")
        .due("10-31")
        .total("2,260.00")
        .add();
    s.bill("BILL-9004", "Stihl Canada")
        .reference("ST-7790")
        .dated("10-01")
        .due("10-31")
        .total("2,260.00")
        .add();
    s.bill("BILL-9005", "Telus Business")
        .reference("TB-1025")
        .dated("09-25")
        .due("10-25")
        .total("450.00")
        .add();
    s.bill("BILL-9006", "Brandt Tractor Ltd.")
        .reference("BT-6612")
        .dated("10-02")
        .due("11-01")
        .total("900.00")
        .add();
    s.bill("BILL-9007", "Western Turf Farms")
        .reference("WT-3310")
        .dated("09-22")
        .due("10-22")
        .total("800.00")
        .add();
    s.spend(
        "BANK-9101",
        "RBC Business Chequing",
        "Telus Business",
        "10-02",
        "450.00",
    );
    s.spend(
        "BANK-9102",
        "RBC Business Chequing",
        "Brandt Tractor Ltd.",
        "10-01",
        "900.00",
    );
    s.spend(
        "BANK-9103",
        "RBC Business Chequing",
        "Western Turf Farms",
        "10-03",
        "850.00",
    );
    let run = s.run();
    check(&s, &run, DOC, &rules);
}

#[test]
fn ap_s06_early_payment_discounts_assumed_not_taken_suggested_when_affordable() {
    let rules = ["AP-TIME-02", "AP-DISC-01", "AP-DISC-02"];
    let mut s = Scenario::new("AP-S06");
    s.entity("Maple Ridge Landscaping Ltd.", "CAD");
    s.map_account("RBC Business Chequing", AccountClass::Bank);
    s.balance("RBC Business Chequing", "10-07", "25,000.00");
    s.cash_buffer("5,000.00");
    s.bill("BILL-6101", "Nutrien Ag Solutions")
        .dated("10-06")
        .terms("2% 10 days, net 30")
        .due("11-05")
        .total("10,000.00")
        .add();
    s.bill("BILL-6102", "Brandt Tractor Ltd.")
        .dated("10-02")
        .terms("1% 15 days, net 45")
        .due("11-16")
        .total("15,000.00")
        .add();
    s.bill("BILL-6103", "Stihl Canada")
        .dated("09-20")
        .terms("2% 10 days, net 30")
        .due("10-20")
        .total("1,000.00")
        .add();
    s.invoice("INV-6001", "Harbourview Strata Corp.")
        .due("10-23")
        .total("12,000.00")
        .add();
    let run = s.run();
    check(&s, &run, DOC, &rules);
}
