//! Coverage is a gate, not a grep (ADR-0009, ADR-0021, Q185, Q206).
//!
//! The `## Coverage` table of `docs/scenarios/<family>.md` is the owner's statement of which
//! Scenario covers which Rule, so it is the source of truth here too: a Scenario that has a test
//! must declare exactly the Rules that table maps to it, no more and no fewer. A Scenario with no
//! test yet is listed in `PENDING`, which can only shrink — an entry that turns out to be covered
//! fails, as does one that names no Scenario. Every Rule in the catalogue must appear in the
//! table, so a Rule cannot be added without the owner saying what exercises it. A Family with no
//! test file is not being built yet and is not checked.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use scenarios::tables::{is_rule_id, section, tables};

/// Scenarios with no test yet. Delete an entry as its test goes green. AR's and AP's entries went
/// at their done-lines, and the convention was that this list is empty at every Family's done-line
/// (ADR-0021, Q213). CASH is the first Family to break it: **CASH-S03 stays here past the CASH
/// done-line**, because it needs GAP-TAX-01 and Q275 leaves that Rule to the GAP build, which
/// builds CASH-S03 alongside GAP-S06 and GAP-S07. The rest go as the CASH build reaches them.
const PENDING: &[&str] = &["CASH-S03"];

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the scenarios crate has a workspace root")
        .to_owned()
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The Families being built: a `tests/<family>.rs` whose Family has a Rule catalogue, with its
/// source. A test file that names no Family — this one, `harness.rs` — is not one.
fn families() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut families = Vec::new();
    for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("a readable directory entry").path();
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if path.extension().and_then(|e| e.to_str()) != Some("rs")
            || !workspace()
                .join("docs/rules")
                .join(format!("{stem}.md"))
                .is_file()
        {
            continue;
        }
        families.push((stem.to_uppercase(), read(&path)));
    }
    families.sort();
    assert!(
        !families.is_empty(),
        "no Family test file in scenarios/tests"
    );
    families
}

/// The Rule IDs a catalogue defines, from its `### <RULE-ID> — …` headings.
fn catalogue_rules(rules_doc: &str, family: &str) -> BTreeSet<String> {
    rules_doc
        .lines()
        .filter_map(|line| line.strip_prefix("### "))
        .filter_map(|heading| heading.split_whitespace().next())
        .filter(|id| is_rule_id(id) && id.starts_with(&format!("{family}-")))
        .map(str::to_owned)
        .collect()
}

/// The Scenarios a Family's document defines, from its `## <FAMILY>-S<NN>` headings. It is the
/// document, not the Coverage table, that says which Scenarios exist, so `all` can be read.
fn scenario_ids(scenarios_doc: &str, family: &str) -> Vec<String> {
    scenarios_doc
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .filter_map(|heading| heading.split_whitespace().next())
        .filter_map(|id| id.strip_prefix(&format!("{family}-")))
        .filter(|s| {
            s.strip_prefix('S')
                .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        })
        .map(str::to_owned)
        .collect()
}

/// One cell of the `Scenarios` column as the Scenarios it names. The owner writes a list
/// (`S01, S07`), a range (`S01–S10`), `all`, `all but S06`, and a note after a semicolon
/// (`S06; every other Scenario has 13 weeks`), which names the Scenario before it.
fn expand(short: &str, all: &[String]) -> Vec<String> {
    let short = short.split(';').next().unwrap_or_default().trim();
    if let Some(rest) = short.strip_prefix("all") {
        let except: BTreeSet<&str> = rest
            .trim()
            .strip_prefix("but")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        return all
            .iter()
            .filter(|s| !except.contains(s.as_str()))
            .cloned()
            .collect();
    }
    let range = short
        .split_once('–')
        .or_else(|| short.split_once('-'))
        .and_then(|(from, to)| {
            let n = |s: &str| s.strip_prefix('S')?.parse::<u32>().ok();
            Some((n(from)?, n(to)?))
        });
    match range {
        Some((from, to)) => (from..=to).map(|n| format!("S{n:02}")).collect(),
        None => vec![short.to_owned()],
    }
}

/// The owner's `## Coverage` table, read the other way round: for each Scenario, the Rules it is
/// said to cover. `| AR-COLLECT-01 | S02, S03, S04, S05 |` becomes an entry under each Scenario,
/// and `S01–S10` under each of the ten.
fn coverage(scenarios_doc: &str, family: &str) -> BTreeMap<String, BTreeSet<String>> {
    let defined = scenario_ids(scenarios_doc, family);
    let section = section(scenarios_doc, "Coverage")
        .unwrap_or_else(|| panic!("{family}: its Scenario document has no ## Coverage section"));
    let table = tables(section)
        .into_iter()
        .find(|t| t.headers.first().is_some_and(|h| h == "Rule"))
        .unwrap_or_else(|| panic!("{family}: the Coverage section has no table headed Rule"));
    let scenarios = table
        .column("Scenarios")
        .unwrap_or_else(|| panic!("{family}: the Coverage table has no Scenarios column"));
    let mut by_scenario: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for row in &table.rows {
        let rule = row.first().map(String::as_str).unwrap_or_default();
        assert!(
            is_rule_id(rule),
            "{family}: Coverage row {rule:?} is no Rule"
        );
        let cell = row.get(scenarios).map(String::as_str).unwrap_or_default();
        for short in cell
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .flat_map(|short| expand(short, &defined))
        {
            by_scenario
                .entry(format!("{family}-{short}"))
                .or_default()
                .insert(rule.to_owned());
        }
    }
    by_scenario
}

/// Each test, as source: the chunks between `#[test]` markers, so a Scenario's declared Rules are
/// read from its own test and not from the file as a whole.
fn test_bodies(source: &str) -> Vec<&str> {
    source.split("#[test]").skip(1).collect()
}

/// The Rule IDs a test declares as string literals (ADR-0009, as amended by ADR-0021).
fn declared_rules(body: &str, family: &str) -> BTreeSet<String> {
    body.split('"')
        .filter(|token| is_rule_id(token) && token.starts_with(&format!("{family}-")))
        .map(str::to_owned)
        .collect()
}

#[test]
fn every_scenario_declares_the_rules_the_coverage_table_maps_to_it() {
    let pending: BTreeSet<&str> = PENDING.iter().copied().collect();
    assert_eq!(
        pending.len(),
        PENDING.len(),
        "PENDING lists the same Scenario twice"
    );

    let mut all_scenarios: BTreeSet<String> = BTreeSet::new();
    let mut tested: BTreeSet<String> = BTreeSet::new();
    let mut wrong: Vec<String> = Vec::new();

    for (family, source) in families() {
        let docs = workspace().join("docs");
        let scenarios_doc = read(&docs.join("scenarios").join(family.to_lowercase() + ".md"));
        let rules_doc = read(&docs.join("rules").join(family.to_lowercase() + ".md"));

        // A Family with a test file is built, so its documents may not still say it is not. The
        // owner edits that line; this only refuses to let it go stale unnoticed, so
        // the next Family cannot ship the way AR did with "Nothing here is built" still at the top.
        for (folder, doc) in [("rules", &rules_doc), ("scenarios", &scenarios_doc)] {
            assert!(
                !doc.contains("Nothing here is built"),
                "{family} is built, and docs/{folder}/{lower}.md still says it is not — \
                 the owner edits that line",
                lower = family.to_lowercase(),
            );
        }

        let catalogue = catalogue_rules(&rules_doc, &family);
        let coverage = coverage(&scenarios_doc, &family);
        assert!(
            !catalogue.is_empty(),
            "{family}: no Rule headings in its Rule catalogue"
        );

        // Every Rule the catalogue defines is mapped to a Scenario by the owner's table.
        let unmapped: Vec<&str> = catalogue
            .iter()
            .map(String::as_str)
            .filter(|rule| !coverage.values().any(|rules| rules.contains(*rule)))
            .collect();
        assert!(
            unmapped.is_empty(),
            "{family}: the catalogue defines these Rules and the Coverage table maps no Scenario \
             to them: {unmapped:?}"
        );

        let bodies = test_bodies(&source);
        for (scenario, should_cover) in coverage {
            all_scenarios.insert(scenario.clone());
            let Some(body) = bodies
                .iter()
                .find(|b| b.contains(&format!("Scenario::new(\"{scenario}\")")))
            else {
                continue;
            };
            tested.insert(scenario.clone());
            let declares = declared_rules(body, &family);
            if declares != should_cover {
                wrong.push(format!(
                    "{scenario}: the Coverage table says {:?}; the test declares {:?}",
                    should_cover.iter().collect::<Vec<_>>(),
                    declares.iter().collect::<Vec<_>>()
                ));
            }
        }
    }

    assert!(
        wrong.is_empty(),
        "a test declares Rules the Coverage table does not map to it, or omits ones it does:\n  {}",
        wrong.join("\n  ")
    );

    let untested: Vec<&str> = all_scenarios
        .iter()
        .map(String::as_str)
        .filter(|s| !tested.contains(*s) && !pending.contains(s))
        .collect();
    assert!(
        untested.is_empty(),
        "these Scenarios have no test and PENDING does not list them: {untested:?}"
    );

    let stale: Vec<&str> = pending
        .iter()
        .copied()
        .filter(|s| tested.contains(*s))
        .collect();
    assert!(
        stale.is_empty(),
        "PENDING still lists these, but they now have a test — delete them from PENDING: {stale:?}"
    );

    let unknown: Vec<&str> = pending
        .iter()
        .copied()
        .filter(|s| !all_scenarios.contains(*s))
        .collect();
    assert!(
        unknown.is_empty(),
        "PENDING lists Scenarios the Coverage table does not name: {unknown:?}"
    );
}
