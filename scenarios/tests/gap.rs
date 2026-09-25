//! One test per GAP Scenario in `docs/scenarios/gap.md`, each declaring as string literals the
//! Rule IDs that document's `## Coverage` table maps to it (ADR-0021). GAP-CARD-01 and
//! GAP-SCHED-01 are also gated by CASH; this file is the Family's own gate for the rest.

use cashsaas_core::facts::FrequencyUnit;
use cashsaas_core::reference::RemitterType;
use cashsaas_core::settings::AccountClass;
use scenarios::Scenario;
use scenarios::render::check;

const DOC: &str = include_str!("../../docs/scenarios/gap.md");

#[test]
fn gap_s01_scheduled_obligations_and_what_covers_one() {
    let mut scenario = Scenario::new("GAP-S01");
    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.map_account("RBC Business Chequing", AccountClass::Bank);
    scenario.map_account("RBC Visa Business", AccountClass::CreditCard);
    scenario.scheduled_obligation(
        "O1",
        "Fraser Valley Properties Ltd.",
        "4,200.00",
        FrequencyUnit::Month,
        "11-01",
        None,
    );
    scenario.scheduled_obligation(
        "O2",
        "Intact Insurance",
        "385.00",
        FrequencyUnit::Month,
        "10-20",
        Some("11-20"),
    );
    scenario.scheduled_obligation(
        "O3",
        "Microsoft Canada",
        "96.00",
        FrequencyUnit::Month,
        "10-10",
        None,
    );
    scenario.scheduled_obligation(
        "O4",
        "Kubota Credit Corporation Canada",
        "1,850.00",
        FrequencyUnit::Month,
        "10-09",
        None,
    );
    scenario
        .bill("FVP-1101", "Fraser Valley Properties Ltd.")
        .dated("10-28")
        .due("11-01")
        .total("4,200.00")
        .add();
    scenario.spend(
        "BANK-Intact",
        "RBC Business Chequing",
        "Intact Insurance",
        "10-05",
        "412.00",
    );
    scenario.spend(
        "CARD-MS",
        "RBC Visa Business",
        "Microsoft Canada",
        "10-06",
        "104.50",
    );
    scenario.spend_no_payee("BANK-Kubota", "RBC Business Chequing", "10-06", "1,850.00");
    let run = scenario.run();
    check(&scenario, &run, DOC, &["GAP-SCHED-01", "GAP-SCHED-02"]);
}

#[test]
fn gap_s02_loans_leases_and_credit_cards() {
    let mut scenario = Scenario::new("GAP-S02");
    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.map_account("BDC Term Loan", AccountClass::Loan);
    scenario.balance("BDC Term Loan", "10-07", "48,000.00");
    scenario.map_account("Kubota Equipment Lease", AccountClass::LeaseLiability);
    scenario.balance("Kubota Equipment Lease", "10-07", "22,500.00");
    scenario.scheduled_obligation(
        "KubotaLease",
        "Kubota Canada Ltd.",
        "1,280.00",
        FrequencyUnit::Month,
        "10-16",
        None,
    );
    scenario.obligation_covers("KubotaLease", "Kubota Equipment Lease");
    scenario.map_account("RBC Visa Business", AccountClass::CreditCard);
    scenario.balance("RBC Visa Business", "10-07", "-3,480.00");
    scenario.card_payment_day("RBC Visa Business", 22);
    scenario.map_account("Amex Business Gold", AccountClass::CreditCard);
    scenario.balance("Amex Business Gold", "10-07", "-1,960.00");
    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &["GAP-SCHED-01", "GAP-LOAN-01", "GAP-CARD-01"],
    );
}

#[test]
fn gap_s03_payroll_net_pay_and_source_deductions() {
    let mut scenario = Scenario::new("GAP-S03");
    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.map_account("Business Chequing", AccountClass::Bank);
    scenario.map_account("Payroll Liabilities", AccountClass::PayrollLiability);
    scenario.balance("Payroll Liabilities", "09-30", "15,200.00");
    scenario.payroll_schedule(
        FrequencyUnit::Week,
        2,
        "10-08",
        "18,400.00",
        Some("7,600.00"),
        Some("Wagepoint"),
    );
    scenario.remitter_type(cashsaas_core::reference::RemitterType::Regular);
    scenario.government_trust("Receiver General for Canada");
    scenario.spend(
        "BANK-RG-1002",
        "Business Chequing",
        "Receiver General for Canada",
        "10-02",
        "5,000.00",
    );
    scenario.spend(
        "BANK-WP-1007",
        "Business Chequing",
        "Wagepoint",
        "10-07",
        "18,212.55",
    );
    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &["GAP-PAYROLL-01", "GAP-PAYROLL-03", "GAP-SCHED-02"],
    );
}

#[test]
fn gap_s04_payroll_with_a_missing_setting() {
    let mut scenario = Scenario::new("GAP-S04");

    scenario.entity("Birch Hill Nursery Ltd.", "CAD");
    scenario.map_account("Wages and Salaries", AccountClass::Wages);

    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.map_account("Payroll Liabilities", AccountClass::PayrollLiability);
    scenario.balance("Payroll Liabilities", "09-30", "0.00");
    scenario.payroll_schedule(
        FrequencyUnit::Month,
        1,
        "10-30",
        "9,800.00",
        Some("3,900.00"),
        None,
    );

    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &["GAP-PAYROLL-01", "GAP-PAYROLL-02", "GAP-PAYROLL-04"],
    );
}

#[test]
fn gap_s05_remittance_amounts_from_last_actual_or_not() {
    let mut scenario = Scenario::new("GAP-S05");

    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.map_account("Business Chequing", AccountClass::Bank);
    scenario.map_account("Payroll Liabilities", AccountClass::PayrollLiability);
    scenario.balance("Payroll Liabilities", "09-30", "4,100.00");
    scenario.payroll_schedule(FrequencyUnit::Month, 1, "10-30", "9,800.00", None, None);
    scenario.remitter_type(cashsaas_core::reference::RemitterType::Regular);
    scenario.government_trust("Receiver General for Canada");
    scenario.spend(
        "BANK-RG-0915",
        "Business Chequing",
        "Receiver General for Canada",
        "09-15",
        "3,950.00",
    );

    scenario.entity("Birch Hill Nursery Ltd.", "CAD");
    scenario.map_account("Payroll Liabilities", AccountClass::PayrollLiability);
    scenario.balance("Payroll Liabilities", "09-30", "0.00");
    scenario.payroll_schedule(FrequencyUnit::Month, 1, "10-30", "6,200.00", None, None);
    scenario.remitter_type(cashsaas_core::reference::RemitterType::Regular);

    let run = scenario.run();
    check(&scenario, &run, DOC, &["GAP-PAYROLL-01", "GAP-PAYROLL-03"]);
}

#[test]
fn gap_s06_gst_hst_booked_running_and_refund() {
    use cashsaas_core::settings::SalesTaxReportingPeriod;

    let mut scenario = Scenario::new("GAP-S06");

    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.map_account("Business Chequing", AccountClass::Bank);
    scenario.map_account("GST/HST Payable", AccountClass::SalesTaxLiability);
    scenario.balance("GST/HST Payable", "09-30", "2,150.00");
    scenario.sales_tax_period(SalesTaxReportingPeriod::Monthly);
    scenario.tax_remittance("TAX-AUG", "Business Chequing", "09-29", "1,980.00");

    scenario.entity("Birch Hill Nursery Ltd.", "CAD");
    scenario.map_account("GST/HST Payable", AccountClass::SalesTaxLiability);
    scenario.balance("GST/HST Payable", "09-30", "-1,240.00");
    scenario.sales_tax_period(SalesTaxReportingPeriod::Quarterly);

    let run = scenario.run();
    check(&scenario, &run, DOC, &["GAP-TAX-01", "GAP-TAX-02"]);
}

#[test]
fn gap_s07_period_from_ledger_none_and_no_calendar() {
    use cashsaas_core::facts::SalesTaxBasis;
    use cashsaas_core::reference::Jurisdiction;

    let mut scenario = Scenario::new("GAP-S07");

    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.ledger_sales_tax_period("3MONTHLY");
    scenario.ledger_sales_tax_basis(SalesTaxBasis::Standard);
    scenario.map_account("GST/HST Payable", AccountClass::SalesTaxLiability);
    scenario.balance("GST/HST Payable", "09-30", "5,400.00");
    scenario.map_account("PST Payable", AccountClass::SalesTaxLiability);
    scenario.balance("PST Payable", "09-30", "1,800.00");
    scenario.tax_jurisdiction("PST Payable", Jurisdiction::BritishColumbia);

    scenario.entity("Birch Hill Nursery Ltd.", "CAD");
    scenario.map_account("GST/HST Payable", AccountClass::SalesTaxLiability);
    scenario.balance("GST/HST Payable", "09-30", "3,100.00");

    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &["GAP-TAX-01", "GAP-TAX-03", "GAP-TAX-04"],
    );
}

#[test]
fn gap_s08_corporate_income_tax_instalments_and_balance() {
    let mut scenario = Scenario::new("GAP-S08");

    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.corporate_tax("01-01", Some("1,500.00"), None);

    scenario.entity("Birch Hill Nursery Ltd.", "CAD");
    scenario.corporate_tax("09-01", None, None);
    scenario.map_account("Income Tax Payable", AccountClass::IncomeTaxPayable);
    scenario.balance("Income Tax Payable", "08-31", "7,200.00");

    let run = scenario.run();
    check(&scenario, &run, DOC, &["GAP-INCOME-01"]);
}

#[test]
fn gap_s09_accruals() {
    use cashsaas_core::settings::AccrualSettlement;

    let mut scenario = Scenario::new("GAP-S09");
    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.map_account("Accrued Liabilities", AccountClass::AccruedLiabilities);
    scenario.balance("Accrued Liabilities", "10-07", "3,600.00");
    scenario.accrual_settlement(
        "Accrued Liabilities",
        AccrualSettlement::DaysAfterMonthEnd(15),
    );
    scenario.map_account("Accrued Bonuses", AccountClass::AccruedLiabilities);
    scenario.balance("Accrued Bonuses", "10-07", "12,000.00");
    scenario.accrual_settlement(
        "Accrued Bonuses",
        AccrualSettlement::OnDate(scenario.date("12-18")),
    );
    scenario.map_account(
        "Accrued Professional Fees",
        AccountClass::AccruedLiabilities,
    );
    scenario.balance("Accrued Professional Fees", "10-07", "5,500.00");

    let run = scenario.run();
    check(&scenario, &run, DOC, &["GAP-ACCRUAL-01"]);
}

fn maple_ridge_s03_payroll(scenario: &mut Scenario, remitter: RemitterType) {
    scenario.entity("Maple Ridge Landscaping Ltd.", "CAD");
    scenario.map_account("Business Chequing", AccountClass::Bank);
    scenario.map_account("Payroll Liabilities", AccountClass::PayrollLiability);
    scenario.balance("Payroll Liabilities", "09-30", "15,200.00");
    scenario.payroll_schedule(
        FrequencyUnit::Week,
        2,
        "10-08",
        "18,400.00",
        Some("7,600.00"),
        Some("Wagepoint"),
    );
    scenario.remitter_type(remitter);
    scenario.government_trust("Receiver General for Canada");
    scenario.spend(
        "BANK-RG-1002",
        "Business Chequing",
        "Receiver General for Canada",
        "10-02",
        "5,000.00",
    );
    scenario.spend(
        "BANK-WP-1007",
        "Business Chequing",
        "Wagepoint",
        "10-07",
        "18,212.55",
    );
}

#[test]
fn gap_s10_accelerated_threshold_1_remittances() {
    let mut scenario = Scenario::new("GAP-S10");
    maple_ridge_s03_payroll(&mut scenario, RemitterType::AcceleratedThreshold1);
    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &["GAP-PAYROLL-01", "GAP-PAYROLL-03", "GAP-SCHED-02"],
    );
}

#[test]
fn gap_s11_accelerated_threshold_2_remittances() {
    let mut scenario = Scenario::new("GAP-S11");
    maple_ridge_s03_payroll(&mut scenario, RemitterType::AcceleratedThreshold2);
    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &["GAP-PAYROLL-01", "GAP-PAYROLL-03", "GAP-SCHED-02"],
    );
}

#[test]
fn gap_s12_quarterly_remitter() {
    let mut scenario = Scenario::new("GAP-S12");
    maple_ridge_s03_payroll(&mut scenario, RemitterType::Quarterly);
    let run = scenario.run();
    check(
        &scenario,
        &run,
        DOC,
        &["GAP-PAYROLL-01", "GAP-PAYROLL-03", "GAP-SCHED-02"],
    );
}
