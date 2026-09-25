//! One test per IC Scenario in `docs/scenarios/ic.md`, each declaring as string literals the
//! Rule IDs that document's `## Coverage` table maps to it (ADR-0021). Four Rules arrived with
//! CASH (Q275); this file is the Family's own gate for the rest and re-checks those four.

use cashsaas_core::facts::FrequencyUnit;
use cashsaas_core::settings::AccountClass;
use scenarios::Scenario;
use scenarios::render::check;

const DOC: &str = include_str!("../../docs/scenarios/ic.md");

#[test]
fn ic_s01_recharges_between_two_entities() {
    let mut scenario = Scenario::new("IC-S01");

    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.intercompany("Birch Hill Nursery Ltd.", "Birch Hill Nursery Ltd.");
    scenario
        .bill("BH-501", "Birch Hill Nursery Ltd.")
        .dated("10-01")
        .due("10-31")
        .total("4,000.00")
        .reference("BH-501")
        .planned("10-21")
        .add();
    scenario
        .invoice("MR-M09", "Birch Hill Nursery Ltd.")
        .dated("09-15")
        .due("09-30")
        .total("1,500.00")
        .add();

    scenario.entity("Birch Hill Nursery Ltd.", "CAD");
    scenario.intercompany(
        "Maple Ridge Landscaping Ltd.",
        "Maple Ridge Landscaping Ltd.",
    );
    scenario
        .invoice("BH-501", "Maple Ridge Landscaping Ltd.")
        .dated("10-01")
        .due("10-31")
        .total("4,000.00")
        .add();
    scenario
        .invoice("BH-502", "Maple Ridge Landscaping Ltd.")
        .dated("10-05")
        .due("11-04")
        .total("2,200.00")
        .add();
    scenario
        .bill("MGMT-SEP", "Maple Ridge Landscaping Ltd.")
        .dated("09-15")
        .due("09-30")
        .total("1,500.00")
        .reference("MGMT-SEP")
        .add();

    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &["IC-MAP-01", "IC-DOC-01", "IC-DOC-02", "IC-ELIM-01"],
    );
}

#[test]
fn ic_s02_intercompany_loans_scheduled_unscheduled_and_not_settling() {
    let mut scenario = Scenario::new("IC-S02");

    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.intercompany("Birch Hill Nursery Ltd.", "Birch Hill Nursery Ltd.");
    scenario.intercompany_account("Loan to Birch Hill", "Birch Hill Nursery Ltd.");
    scenario.intercompany_account("Due to Birch Hill", "Birch Hill Nursery Ltd.");
    scenario.intercompany_account("Long-term Advance to Birch Hill", "Birch Hill Nursery Ltd.");
    scenario.balance("Loan to Birch Hill", "09-30", "30,000.00");
    scenario.balance("Loan to Birch Hill", "10-07", "27,500.00");
    scenario.balance("Due to Birch Hill", "09-30", "-8,000.00");
    scenario.balance("Due to Birch Hill", "10-07", "-8,000.00");
    scenario.balance("Long-term Advance to Birch Hill", "09-30", "50,000.00");
    scenario.balance("Long-term Advance to Birch Hill", "10-07", "50,000.00");
    scenario.intercompany_not_settling("Long-term Advance to Birch Hill");
    scenario.bank_receipt(
        "TX-L1-IN",
        "RBC Business Chequing",
        "Birch Hill Nursery Ltd.",
        "10-05",
        "2,500.00",
    );

    scenario.entity("Birch Hill Nursery Ltd.", "CAD");
    scenario.intercompany(
        "Maple Ridge Landscaping Ltd.",
        "Maple Ridge Landscaping Ltd.",
    );
    scenario.intercompany_account("Loan from Maple Ridge", "Maple Ridge Landscaping Ltd.");
    scenario.intercompany_account("Due from Maple Ridge", "Maple Ridge Landscaping Ltd.");
    scenario.intercompany_account(
        "Long-term Advance from Maple Ridge",
        "Maple Ridge Landscaping Ltd.",
    );
    scenario.balance("Loan from Maple Ridge", "09-30", "-30,000.00");
    scenario.balance("Loan from Maple Ridge", "10-07", "-27,500.00");
    scenario.balance("Due from Maple Ridge", "09-30", "8,000.00");
    scenario.balance("Due from Maple Ridge", "10-07", "8,000.00");
    scenario.balance("Long-term Advance from Maple Ridge", "09-30", "-50,000.00");
    scenario.balance("Long-term Advance from Maple Ridge", "10-07", "-50,000.00");
    scenario.intercompany_not_settling("Long-term Advance from Maple Ridge");
    scenario.spend(
        "TX-L1",
        "RBC Business Chequing",
        "Maple Ridge Landscaping Ltd.",
        "10-05",
        "2,500.00",
    );
    // Schedule after both Entities' loan accounts exist so both sides are listed on it.
    scenario.intercompany_settlement(
        "L1",
        ("Birch Hill Nursery Ltd.", "Maple Ridge Landscaping Ltd."),
        "2,500.00",
        FrequencyUnit::Month,
        "10-15",
        None,
    );
    scenario.settlement_covers("L1", &["Loan to Birch Hill", "Loan from Maple Ridge"]);

    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &["IC-LOAN-01", "IC-LOAN-02", "IC-AGREE-01", "IC-ELIM-01"],
    );
}

#[test]
fn ic_s03_unnamed_counterparty_and_unconnected() {
    let mut scenario = Scenario::new("IC-S03");

    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.intercompany_unnamed("Lee Family Holdings Inc.");
    scenario
        .bill("LFH-0930", "Lee Family Holdings Inc.")
        .dated("10-01")
        .due("10-20")
        .total("3,000.00")
        .add();
    // Cascade is named but has no books in the run (IC-ONESIDED-01).
    scenario.intercompany("Cascade Garden Supply Inc.", "Cascade");
    scenario
        .invoice("MR-C12", "Cascade Garden Supply Inc.")
        .dated("10-01")
        .due("10-28")
        .total("2,400.00")
        .add();

    // Connected sister present in the Group; no IC facts on its books in this Scenario.
    scenario.entity("Birch Hill Nursery Ltd.", "CAD");

    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &["IC-MAP-01", "IC-ONESIDED-01", "IC-DOC-01"],
    );
}

#[test]
fn ic_s04_same_currency_balances_disagree() {
    let mut scenario = Scenario::new("IC-S04");

    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.intercompany("Birch Hill Nursery Ltd.", "Birch Hill Nursery Ltd.");
    scenario.intercompany_account("Advance to Birch Hill", "Birch Hill Nursery Ltd.");
    scenario.balance("Advance to Birch Hill", "09-30", "12,400.00");
    scenario.balance("Advance to Birch Hill", "10-07", "12,400.00");
    scenario.intercompany_not_settling("Advance to Birch Hill");

    scenario.entity("Birch Hill Nursery Ltd.", "CAD");
    scenario.intercompany(
        "Maple Ridge Landscaping Ltd.",
        "Maple Ridge Landscaping Ltd.",
    );
    scenario.intercompany_account("Advance from Maple Ridge", "Maple Ridge Landscaping Ltd.");
    scenario.balance("Advance from Maple Ridge", "09-30", "-12,000.00");
    scenario.balance("Advance from Maple Ridge", "10-07", "-12,000.00");
    scenario.intercompany_not_settling("Advance from Maple Ridge");

    let run = scenario.run();
    check(&scenario, &run, DOC, &["IC-LOAN-02", "IC-AGREE-01"]);
}

#[test]
fn ic_s05_tolerance_and_cross_currency_balances() {
    let mut scenario = Scenario::new("IC-S05");
    scenario.reporting_currency("CAD");
    scenario.conversion_rate("USD", "CAD", "1.3600");
    scenario.intercompany_tolerance("50.00");

    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.intercompany("Birch Hill Nursery Ltd.", "Birch Hill Nursery Ltd.");
    scenario.intercompany("Cascade Garden Supply Inc.", "Cascade Garden Supply Inc.");
    scenario.intercompany_account("Advance to Birch Hill", "Birch Hill Nursery Ltd.");
    scenario.intercompany_account("Due to Cascade Garden Supply", "Cascade Garden Supply Inc.");
    scenario.balance("Advance to Birch Hill", "09-30", "12,025.00");
    scenario.balance("Advance to Birch Hill", "10-07", "12,025.00");
    scenario.balance("Due to Cascade Garden Supply", "09-30", "-6,700.00");
    scenario.balance("Due to Cascade Garden Supply", "10-07", "-6,700.00");
    scenario.intercompany_not_settling("Advance to Birch Hill");
    scenario.intercompany_not_settling("Due to Cascade Garden Supply");

    scenario.entity("Birch Hill Nursery Ltd.", "CAD");
    scenario.intercompany(
        "Maple Ridge Landscaping Ltd.",
        "Maple Ridge Landscaping Ltd.",
    );
    scenario.intercompany_account("Advance from Maple Ridge", "Maple Ridge Landscaping Ltd.");
    scenario.balance("Advance from Maple Ridge", "09-30", "-12,000.00");
    scenario.balance("Advance from Maple Ridge", "10-07", "-12,000.00");
    scenario.intercompany_not_settling("Advance from Maple Ridge");

    scenario.entity("Cascade Garden Supply Inc.", "USD");
    scenario.intercompany(
        "Maple Ridge Landscaping Ltd.",
        "Maple Ridge Landscaping Ltd.",
    );
    scenario.intercompany_account("Due from Maple Ridge", "Maple Ridge Landscaping Ltd.");
    scenario.balance("Due from Maple Ridge", "09-30", "5,000.00");
    scenario.balance("Due from Maple Ridge", "10-07", "5,000.00");
    scenario.intercompany_not_settling("Due from Maple Ridge");

    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &["IC-LOAN-02", "IC-AGREE-01", "IC-AGREE-02"],
    );
}

#[test]
fn ic_s06_recharges_across_currencies() {
    let mut scenario = Scenario::new("IC-S06");
    scenario.reporting_currency("CAD");
    scenario.conversion_rate("USD", "CAD", "1.3600");

    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.intercompany("Cascade Garden Supply Inc.", "Cascade Garden Supply Inc.");
    scenario
        .bill("CG-2207", "Cascade Garden Supply Inc.")
        .dated("10-01")
        .due("10-28")
        .currency("USD", "1.3500")
        .total("1,000.00")
        .add();
    scenario
        .invoice("MR-C20", "Cascade Garden Supply Inc.")
        .dated("10-06")
        .due("11-05")
        .total("2,000.00")
        .add();

    scenario.entity("Cascade Garden Supply Inc.", "USD");
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
        .bill("MR-C20", "Maple Ridge Landscaping Ltd.")
        .dated("10-06")
        .due("11-05")
        .currency("CAD", "0.7400")
        .total("2,000.00")
        .add();

    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &["IC-MAP-01", "IC-DOC-01", "IC-ELIM-01"],
    );
}

#[test]
fn ic_s07_sister_entity_could_cover_a_shortfall() {
    let mut scenario = Scenario::new("IC-S07");
    scenario.reporting_currency("CAD");
    scenario.conversion_rate("USD", "CAD", "1.3600");

    scenario.entity("Birch Hill Nursery Ltd.", "CAD");
    scenario.map_account("RBC Business Chequing", AccountClass::Bank);
    scenario.balance("RBC Business Chequing", "10-07", "5,000.00");
    scenario
        .bill("BH-W6", "Western Turf Farms")
        .due("11-12")
        .total("8,000.00")
        .add();
    scenario
        .bill("BH-W9", "Brandt Tractor Ltd.")
        .due("12-03")
        .total("1,500.00")
        .add();
    scenario
        .invoice("BH-W12", "Harbourview Strata Corp.")
        .due("12-24")
        .total("6,000.00")
        .add();

    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.cash_buffer("15,000.00");
    scenario.map_account("RBC Business Chequing", AccountClass::Bank);
    scenario.balance("RBC Business Chequing", "10-07", "30,000.00");
    scenario
        .bill("MR-W3", "Nutrien Ag Solutions")
        .due("10-22")
        .total("6,000.00")
        .add();
    scenario
        .bill("MR-W8", "Telus Business")
        .due("11-26")
        .total("3,000.00")
        .add();
    scenario
        .invoice("MR-W10", "Oakview Senior Living")
        .due("12-10")
        .total("5,000.00")
        .add();

    scenario.entity("Cascade Garden Supply Inc.", "USD");
    scenario.cash_buffer("5,000.00");
    scenario.map_account("Chase Business Checking", AccountClass::Bank);
    scenario.balance("Chase Business Checking", "10-07", "9,000.00");
    scenario
        .bill("CG-W7", "Pacific Growers Supply LLC")
        .due("11-19")
        .total("2,000.00")
        .add();

    let run = scenario.run();
    check(&scenario, &run, DOC, &["IC-FUND-01"]);
}
