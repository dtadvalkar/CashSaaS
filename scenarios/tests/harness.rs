//! The comparison checks itself. A Scenario test is only worth its green if a wrong fact fails it,
//! so the ways a facts table could pass without checking anything are tests of their own: a column
//! no renderer derives, a fact the document lists that the run does not hold, and a figure that
//! reaches the owner unverified. The AR build log proved these by hand once per checkpoint (Q192);
//! this proves them on every run (Q262).
//!
//! AR-S01 is the fixture because it has one of every shape: a Placement, an exclusion, a
//! commentary column, and rows that appear in the facts table but in no Expected row.

use scenarios::Scenario;
use scenarios::render::check;
use scenarios::tables::{is_rule_id, tokens};

const DOC: &str = include_str!("../../docs/scenarios/ar.md");

/// AR-S01's facts, optionally missing INV-1005 — the zero-total invoice the document lists in its
/// facts table and in no Expected row, so only the facts comparison can catch its absence.
fn ar_s01(with_inv_1005: bool) -> Scenario {
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
    if with_inv_1005 {
        s.invoice("INV-1005", customer)
            .due("10-20")
            .total("0.00")
            .add();
    }
    s
}

fn check_s01(doc: &str, with_inv_1005: bool) {
    let scenario = ar_s01(with_inv_1005);
    let run = scenario.run();
    check(&scenario, &run, doc, &["AR-OPEN-01"]);
}

#[test]
fn a_digit_free_column_no_renderer_derives_is_shown_and_not_compared() {
    let doc = DOC.replace("No-charge warranty visit", "any prose the owner likes");
    check_s01(&doc, true);
}

#[test]
#[should_panic(expected = "not rendered")]
fn a_figure_in_a_column_no_renderer_derives_is_compared() {
    // The same commentary column, with a date put into one cell: it is checked from then on.
    let doc = DOC.replace("| Voided in September |", "| Voided 09-25 |");
    check_s01(&doc, true);
}

#[test]
#[should_panic(expected = "not rendered")]
fn a_renamed_column_fails_instead_of_being_compared_with_itself() {
    // Rename a derived column so no renderer knows it, and put a wrong amount under it. Were the
    // document's own cell copied into the run's row, this would compare with itself and pass.
    let doc = DOC
        .replace(
            "| Total | Tax in total | Open | Notes |",
            "| Total | Tax owing | Open | Notes |",
        )
        .replace(
            "| 10-20 | 5,250.00 | 250.00 |",
            "| 10-20 | 5,250.00 | 999,999.00 |",
        );
    check_s01(&doc, true);
}

#[test]
#[should_panic(expected = "not rendered")]
fn a_fact_the_document_lists_but_the_run_lacks_fails() {
    check_s01(DOC, false);
}

#[test]
#[should_panic(expected = "does not match the approved Scenario document")]
fn a_wrong_amount_in_a_derived_column_fails() {
    let doc = DOC.replace(
        "| INV-1001 | Harbourview Strata Corp. | posted | 10-20 | 5,250.00 |",
        "| INV-1001 | Harbourview Strata Corp. | posted | 10-20 | 5,251.00 |",
    );
    check_s01(&doc, true);
}

#[test]
fn a_rule_id_ending_a_sentence_is_still_cited() {
    // The AR-S09 reviewer found `Reason: AR-TIE-01.` cited nothing: the full stop stayed on the
    // token, so the Provisional reason was compared with an empty list.
    let cited: Vec<String> = tokens("yes. Reason: AR-TIE-01.")
        .into_iter()
        .filter(|t| is_rule_id(t))
        .collect();
    assert_eq!(cited, ["AR-TIE-01"]);
}
