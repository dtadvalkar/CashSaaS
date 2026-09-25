# CashSaaS

[![gates](https://github.com/dtadvalkar/CashSaaS/actions/workflows/gates.yml/badge.svg)](https://github.com/dtadvalkar/CashSaaS/actions/workflows/gates.yml)

A cash forecast for owner-operators who run several businesses under common ownership and cannot
justify a full-time CFO. It answers the questions an owner asks every week: how much cash will each
business have, week by week; when is the tightest week; and what should I do about it, in what
order?

This repository is the forecast engine, written in Rust. It takes an accounting ledger's
documents (open invoices, open bills, bank balances, scheduled items) and produces the forecast and
the list of things to act on. **It is an engine, not yet a product:** there is no ledger connector,
database, server or web interface yet (see [Status](#status)). What exists is the part that has to
be right before any of those matter, and it is specified and tested in detail.

## What the forecast gives you

For each business (an *Entity*) and for the whole *Group* of businesses:

- **Weekly cash.** A run of 7-day weeks from the run date, each closing at its opening cash plus that
  week's expected receipts and payments.
- **The Low Point.** The week with the lowest closing cash.
- **Firm or estimated.** Each week says how much of its amount is firm (a document, a booked balance,
  or a date the owner entered) and how much is estimated (derived from the business's own history).
- **A queue of Decision Items.** Findings a controller would raise before trusting the forecast, each
  with a severity, the role that should act on it, a due date, and the evidence. A cash shortfall
  ranks above a dip below your buffer; a government trust remittance or a critical vendor ranks
  above an ordinary bill.
- **What was left out, and why.** An invoice too old to be collected, a draft invoice, a business with
  no conversion rate left out of the Group total: each is listed with its reason, not silently
  dropped.

It follows a few principles that come from how a careful accountant works:

- **It does not guess.** Missing configuration (no horizon, no conversion rate, no buffer) is part of
  the output. A forecast that rests on something it had to assume says it is *provisional* and why.
- **Timing follows evidence, in order.** For receivables: the date the owner entered, then the
  business's own collection history, then the due date. An overdue invoice with no evidence is not
  given a made-up date.
- **Payables are the owner's choice, not history.** A customer decides when they pay you; you decide
  when you pay a vendor. A habit of paying late is not evidence of what you will do next, and
  following it would flatter the forecast.
- **A credit card is never cash**, even when the ledger lists it as a bank account. Restricted cash is
  shown, not counted. Intercompany balances are not external receipts.
- **A Group total is a labelled total, not pooled cash.** Businesses under common ownership are still
  separate legal entities.
- **Money is exact.** Decimal arithmetic throughout; floating point is banned in the engine by a
  compiler lint.

## A worked example

This is Scenario CASH-S04 from [`docs/scenarios/cash.md`](docs/scenarios/cash.md). The business has a
minimum cash buffer of 10,000.00 and a chequing balance of 14,000.00, with two bills and three
invoices open. (The names are invented.)

| Bill | Counterparty | Due | Open |
|---|---|---|---|
| WT-4401 | Western Turf Farms | 10-15 | 6,000.00 |
| BT-4402 | Brandt Tractor Ltd. | 11-12 | 16,000.00 |

| Invoice | Counterparty | Due | Open |
|---|---|---|---|
| INV-4101 | Harbourview Strata Corp. | 10-23 | 5,000.00 |
| INV-4102 | Oakview Senior Living | 11-20 | 8,000.00 |
| INV-4103 | Cedar Point Property Mgmt. | 11-27 | 7,000.00 |

The engine produces, for the weeks that have activity:

| Week | Opens | Receipts | Payments | Closes |
|---|---|---|---|---|
| W2 | 14,000.00 | — | 6,000.00 | 8,000.00 |
| W3 | 8,000.00 | 5,000.00 | — | 13,000.00 |
| W6 | 13,000.00 | — | 16,000.00 | **−3,000.00** |
| W7 | −3,000.00 | 8,000.00 | — | 5,000.00 |
| W8 | 5,000.00 | 7,000.00 | — | 12,000.00 |

**Low Point:** −3,000.00 in W6. And two items in the queue:

| # | Class | Item | Evidence |
|---|---|---|---|
| 1 | critical | cash shortfall | Week 6 closes at −3,000.00. No credit line to draw on. |
| 2 | critical | below buffer | Buffer is 10,000.00. Weeks 2 and 7 close at 8,000.00 and 5,000.00. |

Week 6 is below zero, so it is reported once, as the shortfall, and not again as a dip below the
buffer. The test for this Scenario renders the engine's output as these same tables and compares
them, figure for figure, with the document.

## Status

The specification covers six *Families* of Rules. Five are built and tested; CLOSE remains
specified for a later milestone:

| Family | What it does | Rules | Scenarios | State |
|---|---|---|---|---|
| **AR** | Turns open customer documents into expected receipts; raises collection, write-off and unbilled-revenue items | 18 | 11 | Built and tested |
| **AP** | Turns open bills and committed purchases into expected payments; raises duplicate, discount and ledger-tie items | 19 | 10 | Built and tested |
| **CASH** | Rolls every Family's expected amounts through the weeks; finds the Low Point; raises shortfall and buffer items; orders the queue; builds the Group view | 16 | 10 | Built and tested |
| **GAP** | Payroll, tax remittances, loans, leases, rent and subscriptions paid without a bill, card balances | 14 | 12 | Built and tested |
| **IC** | Cash between businesses in the same Group; pairing and eliminating both sides; who could fund whom | 10 | 7 | Built and tested |
| **CLOSE** | Whether the books are fit to forecast from: reconciliations, suspense accounts, period lock | 10 | 7 | Specified |

**Does not exist yet:** connectors to QuickBooks Online and Xero, a database, a server, a web
interface and a hosted service. The engine is built first because everything else depends on its
answers being right, and it needs no infrastructure to be tested.

## How it is built

I built CashSaaS. The specification is the source of truth: the Rules in
[`docs/rules/`](docs/rules/), the Canonical Facts in [`docs/facts.md`](docs/facts.md) and the
Scenarios in [`docs/scenarios/`](docs/scenarios/). The Rust matches it, and where the two disagree,
the code is wrong. The build is designed to catch that mechanically:

- **Every Scenario is a test.** It renders the engine's output as the Scenario document's own tables
  and compares them with the document.
- **Coverage is a gate.** Each Scenario's test must declare exactly the Rules the document's coverage
  table maps to it, and every Rule must appear in that table, so a Rule cannot be added without
  saying what exercises it.
- **A mutation sweep** ([`tools/mutate.py`](tools/mutate.py)) changes each figure in every Scenario,
  one at a time, and fails if a test still passes. A test that survives a wrong number is not
  testing.
- **Strict gates on every push** (CI): `cargo fmt`, `cargo clippy -D warnings` and `cargo test`.
  `unsafe` is forbidden and floating-point arithmetic is denied by lint.
- **The engine is pure.** `core` depends on no database, network, async runtime or clock. The run
  date is an input, and `run_forecast` returns a result and cannot fail: invalid facts are rejected
  when they are constructed.

## Repository layout

| Path | What is there |
|---|---|
| `core/` | The engine: Canonical Facts, Rules by Family, the forecast run |
| `scenarios/` | The Scenario builder and one test per Scenario |
| `docs/rules/` | The Rules, one file per Family, each with a stable Rule ID |
| `docs/scenarios/` | Each Scenario's inputs and expected outputs |
| `docs/facts.md` | The Canonical Fact types the engine reads |
| `CONTEXT.md` | The glossary: what each term means |
| `tools/` | The mutation sweep and the pre-commit hook |

## Run it

The toolchain is pinned in `rust-toolchain.toml`; `rustup` installs it on first use.

```
cargo test --workspace
```

To run the mutation sweep for one Family:

```
python3 tools/mutate.py ar
```

## License and working together

Copyright © 2026 Darshan Tadvalkar.

[AGPL-3.0-only](LICENSE). You may use and modify it for yourself. If you distribute it, or run a
modified version as a service for others, you must publish your source under the same license.

I hold the copyright, so commercial licensing, custom development for a business, and hosting are
available. Email [dtadvalkar@gmail.com](mailto:dtadvalkar@gmail.com) or find me at
[github.com/dtadvalkar](https://github.com/dtadvalkar). I am not accepting contributions to this
repository yet; forks are welcome.
