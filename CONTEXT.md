# CashSaaS

A multi-tenant cash operating platform for owner-operators who run several businesses
under common ownership and cannot justify a full-time CFO. This file is the project's
glossary: what each term means, and which near-synonyms not to use.

## Tenancy

**Workspace**:
The billing, security, and collaboration container. Owns users, roles, invitations, and
subscription state.
_Avoid_: Account, tenant, org

**Group**:
A set of businesses under common ownership, operated or advised as one book of work. A
Group may contain entirely unrelated businesses; common ownership implies nothing about
shared operations, customers, or cash.
_Avoid_: Client, business group, portfolio

**Entity**:
A single legal or accounting unit whose books can be connected, synced, forecast, and
reported on. One QBO company, one Xero organisation, or one company inside an Odoo database
is one Entity.
_Avoid_: Company, business, org, subsidiary

**Operator**:
A person with access behind an owner's Workspace to configure and correct it. Distinct
from the owner-operator who is the Workspace's intended daily user.
_Avoid_: Admin, consultant, advisor

## Data layers

**Connection**:
An authorized link to one external system account — a QBO company, an Odoo database —
serving one or more Entities. Carries credentials, scopes, sync cursors, and rate-limit
state.
_Avoid_: Integration, link, source

**Transport**:
The route by which a Connection reaches a provider — aggregator-backed or direct. The same
provider reached by two Transports returns different payload shapes, so an adapter is
identified by provider *and* Transport, never provider alone.
_Avoid_: Method, channel, route

**Adapter**:
The code that fetches from one provider over one Transport and normalizes the result into
Canonical Facts. `qbo-composio` and `qbo-direct` are two Adapters with one canonical target.
_Avoid_: Client, driver, connector (reserve Connection for the authorized link)

**Source Snapshot**:
An immutable raw payload captured from an external system. Evidence, never the working
model.
_Avoid_: Raw data, dump, cache

**Canonical Fact**:
A provider-neutral normalized record derived from Source Snapshots, always retaining its
provenance. Documents (invoices, bills and credit notes), payments and their applications (an
overpayment or prepayment is a payment not yet applied; a payment to a tax authority is marked as
a tax remittance, payee or not), account lines (one transaction's posting to one account, which
is where bank transactions and reconciliation status live), an account's balance as of a date,
scheduled templates, purchase orders, payment terms, and the ledger settings Rules read. AR and AP are
documents, never aging-report lines; a balance is only ever the ledger side of a reconciliation.
Its id is deterministic, so the same record keeps its id across syncs.
_Avoid_: Record, row, normalized data, entity (in the ORM sense)

**Classification**:
The canonical category an account or counterparty belongs to — payroll, tax, intercompany,
related-party, or explicitly unclassified — together with where that came from: an
owner-confirmed mapping, provider metadata, or default keyword matching. Rules read
Classifications, never provider names. An intercompany Classification also names which Entity in
the Group the account or counterparty represents.
_Avoid_: Category, tag, account type, mapping (a mapping is the Setting that produces one)

**Setting**:
A tenant-owned, versioned value that changes what the engine computes — account and
counterparty mappings, intercompany names, routing keywords, materiality thresholds,
payroll cadence, AR timing assumptions.
Versioned so that past Forecast Runs stay explainable.
_Avoid_: Config, option, preference

**Reference Data**:
Versioned facts about the world that are not tenant-owned — statutory due-date calendars
(including CRA payroll remitter period boundaries and dues, ADR-0007 / Q285), holiday lists,
and other public-law schedules. Which calendar applies to an Entity is a Setting.
FX rates, provider metadata. Changes because the world changed, not because a tenant
chose differently.
_Avoid_: Constants, lookup table

**Home Currency**:
The currency an Entity keeps its books in. Every amount on a Canonical Fact has a home amount;
when a document is in another currency, the fact also keeps the amount in that currency and the
rate booked on the document.
_Avoid_: Base currency, local currency, source currency

**Reporting Currency**:
The currency a Group consolidates in. Conversion from a Home Currency uses a rate that is
a Setting — the owner's current belief — not a market fact.
_Avoid_: Base currency, consolidation currency

**Scenario**:
A named synthetic Group — Entities, Canonical Facts, and Settings — built to exercise one
situation, such as a missing payroll bill or a reciprocal intercompany entity that is not
connected. Until there are live books, Scenarios are the only model of reality; they also
seed the demo Workspace.
_Avoid_: Fixture, test data, sample data, mock

## Engine

**Family**:
The subject area a Rule belongs to — the `FAMILY` segment of a Rule ID
(`FAMILY-SUBJECT-NN`) and the unit the catalogue is organized by. Every Rule belongs to
exactly one Family.
_Avoid_: Category, module, group

**Rule**:
One numbered, catalogued statement of engine behaviour — `AR-OVERDUE-01`,
`GAP-PAYROLL-02`. The catalogue is the authority: code that disagrees with it is wrong. A
Rule that is built but excluded from output stays in the catalogue, marked excluded, with the
reason and date. Every Rule is either Universal or Specialized.
_Avoid_: Heuristic, check, logic, feature

**Universal Rule**:
A Rule a competent controller would arrive at from general accounting and cash-management
practice without knowing anything about a particular Group, or a Specialized Rule promoted
after recurring independently across unrelated Groups. Applies to every Group. A
Universal Rule that produces nothing for a Group because no matching facts exist is inactive,
not Specialized.
_Avoid_: Default rule, core rule, standard rule

**Specialized Rule**:
A Rule encoding a judgment, workaround, or preference that belongs to one kind of business
rather than to general practice. Applies only to Groups that have opted in.
_Avoid_: Custom rule, override, client rule

**Forecast Run**:
One reproducible execution of the model over a fixed set of Canonical Facts, Settings, and
a horizon. Runs are persisted, never overwritten, so any two can be compared.
_Avoid_: Forecast, model run, projection

**Horizon**:
The span a Forecast Run covers, expressed as a count of weekly buckets, each a consecutive
7-day period starting on the run date. Configurable per Group; there is no fixed default
length.
_Avoid_: Window, period, term

**Opening Cash**:
The book balance of an Entity's bank-classified accounts as of the run date — the cash a
Forecast Run starts from. Excludes restricted accounts, money still in clearing accounts and
credit card accounts (even where a ledger types them as bank accounts), and is never the bank's
own statement balance.
_Avoid_: Current cash, bank balance, available cash

**Low Point**:
An Entity's lowest closing cash within the Horizon of a Forecast Run. Computed per Entity;
a Group total has no Low Point that raises anything.
_Avoid_: Trough, minimum, danger point

**Minimum Cash Buffer**:
The closing cash an owner chooses never to fall below, set per Entity. Optional, with no
default; a week below it raises an item only when it is set.
_Avoid_: Headroom, reserve, floor, cushion

**Headroom**:
Undrawn credit an Entity could use, such as the unused part of a credit line. Shown beside a
forecast, never counted in it.
_Avoid_: Liquidity, available credit, buffer

**Confidence**:
How firm the evidence behind a Placement is: firm (a document, a booked balance, or a date or
amount a person entered) or estimated (derived from the Entity's own history, or a ledger value
nobody has confirmed). A week's Confidence is the share of its amount at each level, never a
single score or a probability. Missing evidence is not a level: the obligation is excluded and
the run is Provisional.
_Avoid_: Probability, certainty, score

**Gap**:
A cash obligation the forecast must pay that is not an open bill — recurring or one-off,
booked as a liability with no bill (a card balance, a tax or payroll liability, an accrual, a
loan) or not booked at all yet (the next payroll, the unbooked part of a tax period, rent paid
by debit). Gaps are why a forecast can look healthy while being wrong.
_Avoid_: Missing bill, accrual, unbooked liability

**Placement**:
One Canonical Fact or Gap assigned by one Rule to one weekly bucket — or explicitly to no
bucket — within one Forecast Run, carrying the Rule's parameters at the moment of
placement, such as the expected payment date or the exchange rate used. An exclusion is a Placement; "why is
week 6 short" is answered by listing Placements, including what was left out.
_Avoid_: Scheduled item, line item, entry, allocation

**Provisional**:
The state of a Forecast Run that rests on missing evidence: an obligation left out because a
Setting or other evidence it needs is missing, or books a Rule found unfit to forecast from — a
bank account not reconciled for the last month, same-currency intercompany balances that do not
agree. Estimates derived from the
Entity's own records do not make a run Provisional. A Provisional run is presented as
provisional, never as settled, and lists why.
_Avoid_: Draft, estimated, preliminary

**Decision Item**:
An evidence-backed unit of action or review, with a status, an owner, a severity, and a
link to the run that produced it. Identified by Entity, kind, and subject (the Canonical
Fact or Gap it is about), so the same item survives across Forecast Runs rather than being
re-created. States: Open, Snoozed, Waiting (on bookkeeper, accountant, or customer), Done,
Dismissed, Superseded — the last set automatically when a run stops producing it. Severity
is a class: critical (a cash shortfall or a week below the Minimum Cash Buffer), blocking
(the finding makes the run Provisional), or action (everything else). Each kind names the role
that usually acts on it — owner, bookkeeper or accountant — and carries a due date only when
its finding has one.
_Avoid_: Task, alert, notification, todo

**Draft Correction**:
A proposed ledger transaction that would resolve a Decision Item, in the form the Entity's
ledger uses for that fix — a journal entry (an accrual, a reclassification, an intercompany
true-up), a credit note, or a payment application. Produced by whichever Rule found the
problem; entered, if at all, by a person working in the Entity's ledger. A draft naming an
account the Entity has not mapped is incomplete, never filled in by guess.
_Avoid_: Draft journal entry (only one kind of Draft Correction), JE, adjustment, writeback, fix

**External Write**:
Any change sent back to a provider system. Always requires human approval; out of scope
for v1.
_Avoid_: Writeback, sync-back, push
