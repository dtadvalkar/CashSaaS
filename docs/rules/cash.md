# CASH — Forecast position, weeks and cash findings

Status: **approved by the owner** (2026-09-14). Being built: `core/src/cash.rs`, Scenario by
Scenario, with coverage gated against the table in `docs/scenarios/cash.md`; the run log is
`docs/plans/cash-build-log.md`. CASH-OPEN-01 and CASH-ROLL-01 were built a Family early because
AP-DISC-02 reads them (Q265), and this build rewrites them in place.

What this Family does: starts each Entity's forecast from its Opening Cash, rolls every other
Family's Placements through the weeks, measures Confidence, finds the Low Point, raises cash
findings, orders all Decision Items, and builds the Group view. It reads the other Families'
Placements; they never read its output.

Evidence: ADR-0020 and design-session-2026-09-13 Q84–Q95, plus
`docs/research/2026-09-13-ledger-document-model-qbo-xero.md` (cited `[ledger: Part n]`) and
`docs/research/2026-09-13-ar-general-practice.md` (cited `[ar: Part n]`). Entry format follows
ADR-0009. Every Rule here is Universal (ADR-0017).

## Decisions behind these Rules

Decided by the owner on 2026-09-13 and 2026-09-14.

| # | Decision | Rules |
|---|---|---|
| Q84 | Opening Cash is the book balance of bank-classified accounts as of the run date; restricted accounts are excluded; clearing balances are week-1 receipts | CASH-OPEN-01 to 04 |
| Q85 | Credit is not cash: cards are Gaps, undrawn lines are Headroom | CASH-OPEN-04, CASH-HEAD-01 |
| Q86 | Buckets are consecutive 7-day periods from the run date | CASH-WEEK-01 |
| Q87 | Entity-first; the Group view is a labelled sum, not pooled cash | CASH-GROUP-01, 02 |
| Q88 | Confidence is a level per Placement; a week's is the share of its amount at each level | CASH-CONF-01 |
| Q89 | Base forecast only in M1 | CASH-ROLL-01 |
| Q91, Q95 | One shortfall and one below-buffer item per Entity; the buffer is optional | CASH-SHORT-01, CASH-BUFFER-01 |
| Q92, Q93 | Severity classes, ordering by cash impact, due dates only where the finding has one | CASH-ORDER-01 |
| Q132 | Owner-entered scheduled receipts, the inflow counterpart of GAP-SCHED-01 | CASH-SCHED-01 |
| Q240 | A foreign-currency bank account counts at its book home value, with its account-currency amount shown (2026-09-16) | CASH-OPEN-01 |

---

## Opening position

### CASH-OPEN-01 — Opening Cash is the book balance of bank accounts on the run date

- **Scope:** Universal · **Status:** active
- **Statement:** An Entity's Opening Cash is the sum of the balances of its bank-classified
  accounts as of the run date, in Home Currency, including accounts that are overdrawn. A bank
  account in another currency counts at its book value, the ledger's home balance at the rates
  booked on its transactions, and its Placement shows the account-currency balance beside it
  (Q240).
  Payments already recorded but not yet cleared are inside these balances and are never
  forecast again.
- **Justification:** A direct-method forecast starts from cash on hand and schedules receipts
  and payments from there `[ar: Part 1]`. Balances come from the dated balance fact, never a
  "current balance" field, so a run can be reproduced. Both ledgers supply dated balances only
  through reports `[ledger: Part 4]`. Bank accounts are identifiable from metadata in both
  ledgers `[ledger: Part 3]`. An overdraft is negative cash, not a separate liability, for
  this purpose (IAS 7.8, **UNVERIFIED**: not re-read in this session's research). Book cash
  differs from the bank's statement balance until statements are available (Q70). A
  foreign-currency account's book value is at historical rates, not today's; revaluing it is part
  of the full close (ADR-0018), and a rate Setting would leave every such Entity Provisional
  (ADR-0019).
- **Reads:** Canonical Facts: account balance as of a date. Classifications: bank.
- **Produces:** each Entity's Opening Cash, citing each account balance used, Confidence firm.
- **Marks Provisional:** no.

### CASH-OPEN-02 — Restricted cash is shown, not counted

- **Scope:** Universal · **Status:** active
- **Statement:** A bank account the Entity maps as restricted (trust, escrow, a security
  deposit) is left out of Opening Cash and shown separately with its balance.
- **Justification:** Cash the Entity cannot use freely should not make the forecast look
  healthier. Significant cash not available for use is disclosed separately in general
  practice (IAS 7.48, **UNVERIFIED**: not re-read in this session's research). Neither ledger
  marks restriction in metadata `[ledger: Part 3]`, so it is a per-Group mapping (Q68).
- **Reads:** Canonical Facts: account balance as of a date. Classifications: restricted.
- **Produces:** exclusion Placement, reason "restricted", carrying the balance.
- **Marks Provisional:** no.

### CASH-OPEN-03 — Money in a clearing account arrives in week 1

- **Scope:** Universal · **Status:** active
- **Statement:** The balance of an account classified as clearing on the run date (for
  example undeposited customer receipts) is placed as a receipt in week 1, not added to
  Opening Cash.
- **Justification:** A payment into clearing has already left AR but is not yet in a bank
  account (Q71). QBO identifies Undeposited Funds from metadata; other clearing accounts need a
  mapping `[ledger: Parts 2–3]`. Keeping it out of Opening Cash stops week 1 from overstating
  what can be spent today. A clearing balance left too long is a CLOSE finding.
- **Reads:** Canonical Facts: account balance as of a date. Classifications: clearing.
- **Produces:** Placement in week 1, basis "clearing balance", Confidence firm.
- **Marks Provisional:** no.

### CASH-OPEN-04 — A credit card is never cash, even when the ledger lists it as a bank account

- **Scope:** Universal · **Status:** active
- **Statement:** A credit card account is left out of Opening Cash, whatever type the ledger
  gives it. Its balance is an obligation that the GAP Family schedules on the card's payment
  day.
- **Justification:** Card balances are credit used, not cash held (Q85). Xero types credit
  cards as bank accounts with `BankAccountType: CREDITCARD`; QBO uses its own Credit Card
  account type `[ledger: Part 3]`. Without this Rule, a Xero Entity's Opening Cash would net
  card debt against its bank balances.
- **Reads:** Canonical Facts: account balance as of a date. Classifications: credit card.
- **Produces:** exclusion Placement, reason "credit card", citing the GAP Family.
- **Marks Provisional:** no.

### CASH-HEAD-01 — Undrawn credit is Headroom beside the forecast

- **Scope:** Universal · **Status:** active
- **Statement:** For each credit line with a limit set, Headroom is the limit less the drawn
  balance on the run date. It is shown beside every week of the forecast and never added to
  cash. With no limit set, no Headroom is shown. A limit set for a line whose account is not
  mapped produces a Decision Item to map the account.
- **Justification:** Counting undrawn credit as cash hides a crunch (ADR-0020). Limits are not
  held in either ledger's account data `[ledger: Part 3]`, so the limit is an optional Setting
  and the drawn balance comes from the mapped account (ADR-0019). Interest and repayments on
  drawn amounts are loan Gaps.
- **Reads:** Canonical Facts: account balance as of a date. Classifications: credit line.
  Settings: credit line limit (optional, no default).
- **Produces:** Headroom per Entity as of the run date; Decision Item kind "map credit line
  account" when unmapped.
- **Acted on by:** owner.
- **Marks Provisional:** no.

### CASH-SCHED-01 — An owner-entered scheduled receipt arrives on its dates (Q132)

- **Scope:** Universal · **Status:** active
- **Statement:** A scheduled receipt the owner has entered (payer, amount, frequency, next date,
  optional end date) produces a receipt for each occurrence within the horizon. Examples are an
  owner injection, loan proceeds, a grant, or an expected tax refund. An occurrence is covered,
  and becomes an exclusion citing what covers it, when a bank receipt from that payer is booked,
  dated nearer this scheduled date than either neighbouring one (Q149). For this test
  the schedule's frequency extends before its first date and past its end date, and a
  document exactly halfway covers the earlier occurrence. Owner drawings, funds introduced and shareholder
  loans are forecast only this way.
- **Justification:** Neither ledger schedules inflows that aren't invoices; they are
  discretionary or set by agreements outside the ledger
  (`docs/research/2026-09-13-ic-general-practice.md`, Part 6). No other Family covers them. It
  mirrors GAP-SCHED-01 for outflows, and its coverage test mirrors GAP-SCHED-02.
- **Reads:** Settings: scheduled receipt (payer, amount, frequency, next date, end date).
  Canonical Facts: bank transactions.
- **Produces:** Placement per occurrence, basis "scheduled receipt", Confidence firm; exclusion
  Placement, reason "covered by", when covered.
- **Marks Provisional:** no.

---

## Weeks

### CASH-WEEK-01 — The forecast is a run of 7-day weeks from the run date

- **Scope:** Universal · **Status:** active
- **Statement:** A Forecast Run has as many buckets as the Group's horizon Setting says. Each
  bucket is a consecutive 7-day period starting on the run date and is labelled by its dates.
  With no horizon set, no weeks are produced, and a Decision Item asks for it.
- **Justification:** A partial first week was ruled out (design-session-2026-09-02 Q14), so
  weeks run from the run date (Q86). Two runs on the same weekday line up exactly. The horizon
  has no general default (design-session-2026-09-02 Q10; ADR-0019).
- **Reads:** Settings: horizon (required, per Group).
- **Produces:** the week buckets; Decision Item kind "set horizon" when missing.
- **Acted on by:** owner.
- **Marks Provisional:** no.

### CASH-ROLL-01 — Each week closes at its opening cash plus that week's Placements

- **Scope:** Universal · **Status:** active
- **Statement:** Week 1 opens at the Entity's Opening Cash. Each week's closing cash is its
  opening cash plus the receipts and less the payments placed in it by every Family. The next
  week opens at that closing cash. Exclusion Placements add nothing. M1 computes only the base
  forecast.
- **Justification:** This is the direct-method roll-forward `[ar: Part 1]`. Conservative and
  optimistic scenarios come after M1, from data Placements already record (Q89).
- **Reads:** CASH-OPEN-01 and every Family's Placements.
- **Produces:** weekly opening and closing cash per Entity.
- **Marks Provisional:** no.

### CASH-CONF-01 — A week's Confidence is the share of its amount at each level

- **Scope:** Universal · **Status:** active
- **Statement:** A week's Confidence is the share of the total size of its receipts and
  payments placed at each Confidence level: firm or estimated. A payment counts by its
  size, not netted against receipts. A week with nothing placed has no shares (Q153).
- **Justification:** A single blended score hides where the evidence runs out
  (design-session-2026-09-02 Q14; ADR-0020). Counting size rather than net amount keeps a
  large estimated receipt from being hidden by an equal firm payment.
- **Reads:** every Placement's amount and Confidence level.
- **Produces:** per week, the percentage of placed amount that is firm and estimated.
- **Marks Provisional:** no.

### CASH-PROV-01 — A run says why it is Provisional

- **Scope:** Universal · **Status:** active
- **Statement:** A Forecast Run is Provisional when any Rule whose entry marks the run
  Provisional has produced a finding, whether an obligation excluded for missing evidence or a
  failed check. Estimated Placements alone never make a run Provisional. The run lists every
  reason.
- **Justification:** A Provisional run is presented as provisional, never as settled
  (`CONTEXT.md`). Provisional means evidence is missing, not that the future is estimated
  (design-session-2026-09-13 Q123). Listing the reasons tells the owner what to fix.
- **Reads:** every Decision Item from a Rule that marks the run Provisional.
- **Produces:** the run's Provisional state and its list of reasons.
- **Marks Provisional:** yes, by definition.

---

## Cash findings

### CASH-LOW-01 — Each Entity's Low Point is its lowest closing cash

- **Scope:** Universal · **Status:** active
- **Statement:** An Entity's Low Point is its lowest weekly closing cash within the horizon,
  with the week it falls in. When weeks tie, it is the earliest (Q152).
- **Justification:** The Low Point is the forecast's headline answer, computed per Entity
  because Group cash is not pooled (Q87).
- **Reads:** CASH-ROLL-01.
- **Produces:** Low Point amount and week per Entity.
- **Marks Provisional:** no.

### CASH-SHORT-01 — Closing cash below zero raises one shortfall item

- **Scope:** Universal · **Status:** active
- **Statement:** When any week's closing cash falls below zero, the Entity has one open "cash
  shortfall" Decision Item. Its evidence lists each continuous stretch below zero (first week,
  last week, lowest closing cash) and the Headroom available. It is due at the start of the
  first stretch. If the run is Provisional, the item says so and lists the blocking items.
- **Justification:** Running out of cash matters for every business, so no Setting is needed
  (Q91). Identifying the item by kind alone keeps the owner's status across daily runs whose
  week dates shift (Q95; design-session-2026-09-11 Q31). Naming Headroom turns "you run out"
  into the actual decision (Q85). Each stretch also lists the trust obligations falling inside
  it, because those carry personal liability (Q100, AP-PRIORITY-01).
- **Reads:** CASH-ROLL-01, CASH-HEAD-01, CASH-PROV-01.
- **Produces:** Decision Item kind "cash shortfall", severity critical.
- **Acted on by:** owner.
- **Marks Provisional:** no.

### CASH-BUFFER-01 — Closing cash below the owner's buffer raises one item

- **Scope:** Universal · **Status:** active
- **Statement:** When the Entity has a Minimum Cash Buffer set and any week's closing cash is
  at or above zero but below it, the Entity has one open "below buffer" Decision Item. Its
  evidence lists each such stretch and the Headroom available, and it is due at the start of
  the first stretch. Weeks below zero belong to CASH-SHORT-01 instead. With no buffer set, this
  Rule does nothing.
- **Justification:** The right buffer depends on the business, so it has no general default
  (Q91; ADR-0019). Weeks below zero are covered by the shortfall item, so they aren't reported
  twice.
- **Reads:** CASH-ROLL-01, CASH-HEAD-01. Settings: Minimum Cash Buffer (optional, no default).
- **Produces:** Decision Item kind "below buffer", severity critical.
- **Acted on by:** owner.
- **Marks Provisional:** no.

### CASH-ORDER-01 — Every Decision Item gets a severity class and a place in the queue

- **Scope:** Universal · **Status:** active
- **Statement:** Each Decision Item from any Family is critical (cash shortfall, below buffer),
  blocking (produced by a Rule that marks the run Provisional) or action (everything else).
  Within a class, items concerning amounts held in trust for a government come first, then
  items concerning vendors the owner marks as critical or as secured lenders (AP-PRIORITY-01),
  then the rest. Within each of those groups, items are ordered by the cash they concern that
  lands on or before the Entity's Low Point week, then by the total amount they concern. An
  item carries a due date only when its finding has one. A cash shortfall item always comes
  before a below-buffer item (Q154). The queue is per Entity; an item about the Group, such as
  CASH-GROUP-02's, joins the queue of the Entity it concerns (Q155). An item about a pair of
  Entities is one item, shown in both Entities' queues with one status (Q161).
- **Justification:** Classes and ordering avoid numeric thresholds (Q92; ADR-0019). The class
  comes from facts already in each catalogue entry. Trust amounts rank first because
  directors can be personally liable for them (Q100; `[ap: Part 2]` in
  `docs/research/2026-09-13-ap-general-practice.md`). Due dates without a real date behind them
  would be arbitrary Settings (Q93).
- **Reads:** every Decision Item, its Rule's "marks Provisional" field, the Placements it
  concerns, CASH-LOW-01.
- **Produces:** severity class and queue order for every Decision Item.
- **Marks Provisional:** no.

---

## Group view

### CASH-GROUP-01 — The Group view is a labelled total, not pooled cash

- **Scope:** Universal · **Status:** active
- **Statement:** The Group view sums each Entity's weekly opening cash, receipts, payments and
  closing cash in the Group's Reporting Currency, with intercompany Placements eliminated in
  pairs (IC-ELIM-01). A cross-currency pair's residual is shown as an intercompany currency
  difference, not hidden. It
  is labelled as a total that does not mean cash can move between Entities. A Group total below
  zero or below any buffer raises no Decision Item.
- **Justification:** Common ownership implies nothing about shared cash (plan §2), and one
  company's surplus does not pay another's obligations without an intercompany transfer (Q87).
  Consolidation eliminates intragroup flows in full `[ar: Part 6, IFRS 10 B86(c)]`.
- **Reads:** each Entity's CASH-ROLL-01 output and Placements; IC Placements. Settings:
  conversion rate per currency pair (design-session-2026-09-11 Q30).
- **Produces:** the Group's weekly totals in the Reporting Currency.
- **Marks Provisional:** no.

### CASH-GROUP-02 — An Entity with no conversion rate is left out of Group totals

- **Scope:** Universal · **Status:** active
- **Statement:** When an Entity's Home Currency differs from the Reporting Currency and no rate
  is set for that pair, the Entity is left out of the Group totals as an exclusion. A Decision
  Item asks for the rate, and the run is Provisional. The Entity's own forecast is unaffected.
- **Justification:** A guessed rate gives a Group total that looks settled but isn't (ADR-0019,
  Q64).
- **Reads:** Settings: conversion rate per currency pair.
- **Produces:** exclusion Placement, reason "no conversion rate"; Decision Item kind "set
  conversion rate".
- **Acted on by:** owner.
- **Marks Provisional:** yes.
