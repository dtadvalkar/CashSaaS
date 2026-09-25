//! Markdown tables as `docs/scenarios/*.md` writes them: parsing a Scenario's section, comparing a
//! rendered table with the document's, and showing both when they differ (Q259).

/// One Markdown table: a header row and its body rows, cells trimmed and bold markers removed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Table {
    pub fn column(&self, header: &str) -> Option<usize> {
        self.headers.iter().position(|h| h == header)
    }

    pub(crate) fn markdown(&self) -> String {
        let mut out = row_markdown(&self.headers);
        out.push_str(&row_markdown(
            &self
                .headers
                .iter()
                .map(|_| "---".to_owned())
                .collect::<Vec<_>>(),
        ));
        for row in &self.rows {
            out.push_str(&row_markdown(row));
        }
        out
    }
}

fn row_markdown(cells: &[String]) -> String {
    format!("| {} |\n", cells.join(" | "))
}

/// The text of one section: from its `## <name>` heading to the next heading. The name must be the
/// whole heading or be followed by a space, so `## Coverage` is found and `## AR-S1` does not match
/// `## AR-S10 — …`.
pub fn section<'a>(doc: &'a str, name: &str) -> Option<&'a str> {
    let heading = format!("## {name}");
    let mut from = 0;
    while let Some(i) = doc[from..].find(&heading) {
        let start = from + i + heading.len();
        from = start;
        if !matches!(doc.as_bytes().get(start), None | Some(b' ' | b'\n' | b'\r')) {
            continue;
        }
        let body = &doc[start..];
        let end = body.find("\n## ").map_or(body.len(), |i| i + 1);
        return Some(&body[..end]);
    }
    None
}

fn cells(line: &str) -> Vec<String> {
    let inner = line.trim().trim_start_matches('|').trim_end_matches('|');
    inner
        .split('|')
        .map(|c| c.trim().replace("**", ""))
        .collect()
}

fn is_divider(cells: &[String]) -> bool {
    cells
        .iter()
        .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':'))
}

/// Every table in a piece of Markdown, in order.
pub fn tables(text: &str) -> Vec<Table> {
    let mut tables = Vec::new();
    let mut current: Option<Table> = None;
    for line in text.lines() {
        if line.trim_start().starts_with('|') {
            let cells = cells(line);
            match current.as_mut() {
                None => {
                    current = Some(Table {
                        headers: cells,
                        rows: Vec::new(),
                    });
                }
                Some(table) if is_divider(&cells) => {
                    let _ = table;
                }
                Some(table) => table.rows.push(cells),
            }
        } else if let Some(table) = current.take() {
            tables.push(table);
        }
    }
    tables.extend(current);
    tables
}

/// Whether a token looks like a Rule ID: `FAMILY-SUBJECT-NN`.
pub fn is_rule_id(token: &str) -> bool {
    let parts: Vec<&str> = token.split('-').collect();
    parts.len() == 3
        && parts[..2]
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_uppercase()))
        && parts[2].len() == 2
        && parts[2].chars().all(|c| c.is_ascii_digit())
}

/// The words of a cell, without the punctuation prose adds around them.
pub fn tokens(cell: &str) -> Vec<String> {
    cell.split(|c: char| c.is_whitespace() || c == '(' || c == ')' || c == ';')
        .map(|t| t.trim_end_matches([',', ':', '.']).trim_end_matches("'s"))
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

fn rule_id_set(cell: &str) -> Vec<String> {
    let mut ids: Vec<_> = tokens(cell).into_iter().filter(|t| is_rule_id(t)).collect();
    ids.sort();
    ids.dedup();
    ids
}

/// How a column is compared. The document may say less than the run knows, never something
/// different: what it states must match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Match {
    /// Cell for cell.
    Exact,
    /// The first word matches exactly and every other word the document uses appears.
    Loose,
    /// Exact set equality of Rule IDs: every ID the document cites appears on the run, and every
    /// ID the run cites appears in the document. Order and non-ID tokens (Q-notes) are ignored.
    Rules,
    /// Shown, never compared: prose.
    Ignored,
}

pub fn cell_matches(how: Match, expected: &str, actual: &str) -> bool {
    match how {
        Match::Exact => expected == actual,
        Match::Ignored => true,
        Match::Loose => {
            let expected = tokens(expected);
            let actual = tokens(actual);
            expected.first() == actual.first() && expected.iter().all(|t| actual.contains(t))
        }
        Match::Rules => rule_id_set(expected) == rule_id_set(actual),
    }
}

/// Compares two tables with the same headers, row order ignored. `Err` carries what the owner
/// reads: the document's table and the run's, with the rows that found no match.
pub fn compare(
    what: &str,
    expected: &Table,
    actual: &Table,
    how: &dyn Fn(&str) -> Match,
) -> Result<(), String> {
    let matches = |e: &[String], a: &[String]| {
        e.len() == a.len()
            && expected
                .headers
                .iter()
                .zip(e.iter().zip(a))
                .all(|(h, (e, a))| cell_matches(how(h), e, a))
    };
    let mut unmatched_actual: Vec<&Vec<String>> = actual.rows.iter().collect();
    let mut unmatched_expected = Vec::new();
    for row in &expected.rows {
        match unmatched_actual.iter().position(|a| matches(row, a)) {
            Some(i) => {
                unmatched_actual.remove(i);
            }
            None => unmatched_expected.push(row.clone()),
        }
    }
    if expected.headers == actual.headers
        && unmatched_expected.is_empty()
        && unmatched_actual.is_empty()
    {
        return Ok(());
    }
    let mut message = format!("{what} does not match the approved Scenario document.\n\n");
    message.push_str(&format!(
        "Expected (the document):\n\n{}\n",
        expected.markdown()
    ));
    message.push_str(&format!("Actual (the run):\n\n{}\n", actual.markdown()));
    if !unmatched_expected.is_empty() {
        let missing = Table {
            headers: expected.headers.clone(),
            rows: unmatched_expected,
        };
        message.push_str(&format!(
            "Expected but not produced:\n\n{}\n",
            missing.markdown()
        ));
    }
    if !unmatched_actual.is_empty() {
        let extra = Table {
            headers: actual.headers.clone(),
            rows: unmatched_actual.into_iter().cloned().collect(),
        };
        message.push_str(&format!(
            "Produced but not expected:\n\n{}\n",
            extra.markdown()
        ));
    }
    Err(message)
}
