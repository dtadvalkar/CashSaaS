//! One test per CASH Scenario in `docs/scenarios/cash.md`, each declaring as string literals the
//! Rule IDs that document's `## Coverage` table maps to it (ADR-0021). CASH is the Family that
//! reads the run rather than the facts: every Scenario here depends on what AR, AP and the Rules
//! this build writes ahead of GAP and IC placed first.

use cashsaas_core::facts::FrequencyUnit;
use cashsaas_core::settings::AccountClass;
use scenarios::Scenario;
use scenarios::render::check;

const DOC: &str = include_str!("../../docs/scenarios/cash.md");

#[test]
fn cash_s01_what_opening_cash_includes() {
    let mut scenario = Scenario::new("CASH-S01");
    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.map_account("RBC Business Chequing", AccountClass::Bank);
    scenario.balance("RBC Business Chequing", "10-07", "18,400.00");
    scenario.map_account("RBC Business Savings", AccountClass::Bank);
    scenario.balance("RBC Business Savings", "10-07", "25,000.00");
    scenario.map_account("Scotiabank Operating", AccountClass::Bank);
    scenario.balance("Scotiabank Operating", "10-07", "-1,250.00");
    scenario.map_account("Holdback Trust Account", AccountClass::Restricted);
    scenario.balance("Holdback Trust Account", "10-07", "12,000.00");
    scenario.map_account("Customer Receipts Clearing", AccountClass::Clearing);
    scenario.balance("Customer Receipts Clearing", "10-07", "2,300.00");
    scenario.map_account("RBC Visa Business", AccountClass::CreditCard);
    scenario.balance("RBC Visa Business", "10-07", "-3,480.00");
    scenario.card_payment_day("RBC Visa Business", 22);
    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &[
            "CASH-OPEN-01",
            "CASH-OPEN-02",
            "CASH-OPEN-03",
            "CASH-OPEN-04",
            "CASH-ROLL-01",
            "CASH-LOW-01",
        ],
    );
}

#[test]
fn cash_s02_undrawn_credit_is_headroom_never_cash() {
    let mut scenario = Scenario::new("CASH-S02");
    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.map_account("RBC Business Chequing", AccountClass::Bank);
    scenario.balance("RBC Business Chequing", "10-07", "9,000.00");
    scenario.map_account("BDC Operating Line", AccountClass::CreditLine);
    scenario.balance("BDC Operating Line", "10-07", "-15,000.00");
    scenario.map_account("TD Equipment Line", AccountClass::CreditLine);
    scenario.balance("TD Equipment Line", "10-07", "-4,000.00");
    scenario.credit_line_limit("BDC Operating Line", "50,000.00");
    scenario.credit_line_limit("RBC Royal Line of Credit", "20,000.00");
    scenario.scheduled_obligation(
        "SO1",
        "BDC Operating Line",
        "110.00",
        FrequencyUnit::Month,
        "10-25",
        None,
    );
    scenario.scheduled_obligation(
        "SO2",
        "TD Equipment Line",
        "350.00",
        FrequencyUnit::Month,
        "10-20",
        None,
    );
    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &[
            "CASH-OPEN-01",
            "CASH-HEAD-01",
            "CASH-ROLL-01",
            "CASH-LOW-01",
        ],
    );
}

#[test]
fn cash_s07_confidence_by_size_and_what_makes_a_run_provisional() {
    let mut scenario = Scenario::new("CASH-S07");
    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.map_account("RBC Business Chequing", AccountClass::Bank);
    scenario.balance("RBC Business Chequing", "10-07", "20,000.00");
    scenario.map_account("RBC Visa Business", AccountClass::CreditCard);
    scenario.balance("RBC Visa Business", "10-07", "-2,500.00");
    scenario
        .invoice("INV-7101", "Harbourview Strata Corp.")
        .due("10-16")
        .total("6,000.00")
        .add();
    scenario
        .invoice("INV-7102", "Alder Creek Homes")
        .due("09-10")
        .total("1,800.00")
        .add();
    scenario
        .bill("WT-7401", "Western Turf Farms")
        .due("10-19")
        .total("4,000.00")
        .draft()
        .add();
    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &[
            "CASH-OPEN-01",
            "CASH-OPEN-04",
            "CASH-ROLL-01",
            "CASH-CONF-01",
            "CASH-PROV-01",
            "CASH-LOW-01",
            "CASH-ORDER-01",
        ],
    );
}

#[test]
fn cash_s06_no_horizon_set() {
    let mut scenario = Scenario::new("CASH-S06");
    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.no_horizon();
    scenario.map_account("RBC Business Chequing", AccountClass::Bank);
    scenario.balance("RBC Business Chequing", "10-07", "12,000.00");
    scenario
        .invoice("INV-6101", "Harbourview Strata Corp.")
        .due("10-20")
        .total("4,000.00")
        .add();
    let run = scenario.run();
    check(&scenario, &run, DOC, &["CASH-OPEN-01", "CASH-WEEK-01"]);
}

#[test]
fn cash_s05_owner_entered_scheduled_receipts_one_covered() {
    let mut scenario = Scenario::new("CASH-S05");
    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.map_account("RBC Business Chequing", AccountClass::Bank);
    scenario.balance("RBC Business Chequing", "10-07", "31,000.00");
    scenario.bank_receipt(
        "BANK-5101",
        "RBC Business Chequing",
        "Jordan Lee",
        "10-02",
        "3,000.00",
    );
    scenario.bank_receipt(
        "BANK-5102",
        "RBC Business Chequing",
        "Farm Credit Canada",
        "10-06",
        "25,000.00",
    );
    scenario.scheduled_receipt(
        "SR1",
        "Jordan Lee",
        "3,000.00",
        FrequencyUnit::Month,
        "10-01",
        None,
    );
    scenario.scheduled_receipt(
        "SR2",
        "Farm Credit Canada",
        "25,000.00",
        FrequencyUnit::Month,
        "10-15",
        Some("11-15"),
    );
    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &[
            "CASH-OPEN-01",
            "CASH-ROLL-01",
            "CASH-LOW-01",
            "CASH-SCHED-01",
        ],
    );
}

#[test]
fn cash_s04_below_the_buffer_and_below_zero() {
    let mut scenario = Scenario::new("CASH-S04");
    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.cash_buffer("10,000.00");
    scenario.map_account("RBC Business Chequing", AccountClass::Bank);
    scenario.balance("RBC Business Chequing", "10-07", "14,000.00");
    scenario
        .bill("Bill WT-4401", "Western Turf Farms")
        .due("10-15")
        .total("6,000.00")
        .add();
    scenario
        .bill("Bill BT-4402", "Brandt Tractor Ltd.")
        .due("11-12")
        .total("16,000.00")
        .add();
    scenario
        .invoice("Invoice INV-4101", "Harbourview Strata Corp.")
        .due("10-23")
        .total("5,000.00")
        .add();
    scenario
        .invoice("Invoice INV-4102", "Oakview Senior Living")
        .due("11-20")
        .total("8,000.00")
        .add();
    scenario
        .invoice("Invoice INV-4103", "Cedar Point Property Mgmt.")
        .due("11-27")
        .total("7,000.00")
        .add();
    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &[
            "CASH-OPEN-01",
            "CASH-ROLL-01",
            "CASH-LOW-01",
            "CASH-SHORT-01",
            "CASH-BUFFER-01",
            "CASH-ORDER-01",
        ],
    );
}

#[test]
fn cash_s08_the_order_of_the_queue() {
    let mut scenario = Scenario::new("CASH-S08");
    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.government_trust("Receiver General for Canada");
    scenario.critical_vendor("Nutrien Ag Solutions");
    scenario.map_account("RBC Business Chequing", AccountClass::Bank);
    scenario.balance("RBC Business Chequing", "10-07", "8,000.00");
    scenario
        .bill("RG-0815", "Receiver General for Canada")
        .due("09-15")
        .total("2,600.00")
        .add();
    scenario
        .bill("NAS-5512", "Nutrien Ag Solutions")
        .due("09-30")
        .total("1,200.00")
        .add();
    scenario
        .bill("HD-88410", "Home Depot Pro")
        .due("10-02")
        .total("3,100.00")
        .add();
    scenario
        .bill("TEL-1009", "Telus Business")
        .due("10-05")
        .total("450.00")
        .add();
    scenario
        .bill("BT-2210", "Brandt Tractor Ltd.")
        .due("10-23")
        .total("6,000.00")
        .add();
    scenario
        .bill("WT-0417", "Western Turf Farms")
        .due("12-01")
        .total("5,000.00")
        .draft()
        .add();
    scenario
        .invoice("INV-8101", "Harbourview Strata Corp.")
        .due("11-20")
        .total("12,000.00")
        .add();
    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &[
            "CASH-OPEN-01",
            "CASH-ROLL-01",
            "CASH-CONF-01",
            "CASH-LOW-01",
            "CASH-SHORT-01",
            "CASH-ORDER-01",
        ],
    );
}

#[test]
fn cash_s09_the_group_view_intercompany_eliminated() {
    let mut scenario = Scenario::new("CASH-S09");
    scenario.reporting_currency("CAD");
    scenario.conversion_rate("USD", "CAD", "1.3600");

    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.map_account("RBC Business Chequing", AccountClass::Bank);
    scenario.balance("RBC Business Chequing", "10-07", "30,000.00");
    scenario.intercompany("Birch Hill Nursery Ltd.", "Birch Hill Nursery Ltd.");
    scenario.intercompany("Cascade Garden Supply Inc.", "Cascade Garden Supply Inc.");
    scenario
        .bill("MR-9001", "Birch Hill Nursery Ltd.")
        .dated("10-01")
        .due("10-21")
        .total("4,000.00")
        .add();
    scenario
        .bill("MR-9002", "Cascade Garden Supply Inc.")
        .dated("10-01")
        .due("10-28")
        .currency("USD", "1.3500")
        .total("1,000.00")
        .add();
    scenario
        .invoice("INV-9101", "Kingsway Medical Clinic")
        .dated("10-07")
        .due("11-06")
        .total("8,000.00")
        .add();

    scenario.entity("Birch Hill Nursery Ltd.", "CAD");
    scenario.map_account("RBC Business Chequing", AccountClass::Bank);
    scenario.balance("RBC Business Chequing", "10-07", "5,000.00");
    scenario.intercompany(
        "Maple Ridge Landscaping Ltd.",
        "Maple Ridge Landscaping Ltd.",
    );
    scenario
        .invoice("BH-501", "Maple Ridge Landscaping Ltd.")
        .dated("10-01")
        .due("10-21")
        .total("4,000.00")
        .add();
    scenario
        .bill("WT-9401", "Western Turf Farms")
        .dated("10-07")
        .due("11-13")
        .total("12,000.00")
        .add();

    scenario.entity("Cascade Garden Supply Inc.", "USD");
    scenario.map_account("Chase Business Checking", AccountClass::Bank);
    scenario.balance("Chase Business Checking", "10-07", "7,000.00");
    scenario.intercompany(
        "Maple Ridge Landscaping Ltd.",
        "Maple Ridge Landscaping Ltd.",
    );
    scenario
        .invoice("CG-2207", "Maple Ridge Landscaping Ltd.")
        .dated("10-01")
        .due("10-28")
        .total("1,000.00")
        .add();
    scenario
        .bill("PG-9402", "Pacific Growers Supply LLC")
        .dated("10-07")
        .due("12-04")
        .total("2,500.00")
        .add();

    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &[
            "CASH-OPEN-01",
            "CASH-ROLL-01",
            "CASH-LOW-01",
            "CASH-SHORT-01",
            "CASH-ORDER-01",
            "CASH-GROUP-01",
        ],
    );
}

#[test]
fn cash_s10_an_entity_with_no_conversion_rate() {
    let mut scenario = Scenario::new("CASH-S10");
    scenario.reporting_currency("CAD");
    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.map_account("RBC Business Chequing", AccountClass::Bank);
    scenario.balance("RBC Business Chequing", "10-07", "15,000.00");
    scenario
        .invoice("INV-10101", "Harbourview Strata Corp.")
        .due("10-16")
        .total("2,000.00")
        .add();
    scenario.entity("Cascade Garden Supply Inc.", "USD");
    scenario.map_account("Chase Business Checking", AccountClass::Bank);
    scenario.balance("Chase Business Checking", "10-07", "4,000.00");
    scenario
        .bill("PG-10401", "Pacific Growers Supply LLC")
        .due("10-23")
        .total("1,500.00")
        .add();
    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &[
            "CASH-OPEN-01",
            "CASH-ROLL-01",
            "CASH-LOW-01",
            "CASH-PROV-01",
            "CASH-GROUP-01",
            "CASH-GROUP-02",
        ],
    );
}
