//! The `Money` shape every monetary field uses (`docs/facts.md`, Money).

use rust_decimal::{Decimal, RoundingStrategy};

/// An ISO 4217 code: three uppercase ASCII letters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Currency([u8; 3]);

impl Currency {
    pub fn new(code: &str) -> Option<Self> {
        let bytes: [u8; 3] = code.as_bytes().try_into().ok()?;
        bytes
            .iter()
            .all(u8::is_ascii_uppercase)
            .then_some(Self(bytes))
    }

    pub fn code(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_default()
    }
}

/// An amount in the Entity's Home Currency. Distinct from reporting and transaction amounts (Q202).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HomeAmount(pub Decimal);

/// An amount in a record's own currency when that isn't the Home Currency (Q209).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TransactionAmount(pub Decimal);

/// Which way a booked rate is quoted (Q242). The rate is never inverted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quote {
    /// Home currency per foreign unit, as QBO quotes.
    HomePerUnit,
    /// Foreign units per home unit, as Xero documents `CurrencyRate`.
    UnitsPerHome,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransactionPart {
    pub amount: TransactionAmount,
    pub currency: Currency,
    /// Exactly as the ledger gave it.
    pub rate: Decimal,
    pub quote: Quote,
    /// True when the ledger gave no home figure and the Adapter computed it (Q219).
    pub home_derived: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Money {
    pub home: HomeAmount,
    /// Present only when the record's currency is not the Home Currency.
    pub transaction: Option<TransactionPart>,
}

impl Money {
    pub fn home(amount: Decimal) -> Self {
        Self {
            home: HomeAmount(amount),
            transaction: None,
        }
    }

    /// The amount in the record's own currency; `None` means the Home Currency (Q243).
    pub fn in_own_currency(&self) -> (Option<Currency>, Decimal) {
        match &self.transaction {
            Some(t) => (Some(t.currency), t.amount.0),
            None => (None, self.home.0),
        }
    }

    pub fn is_negative(&self) -> bool {
        self.home.0 < Decimal::ZERO
            || self
                .transaction
                .as_ref()
                .is_some_and(|t| t.amount.0 < Decimal::ZERO)
    }
}

/// To the currency's minor unit, half away from zero (Q182). One statement of the rounding rule
/// for every conversion: a document's own currency to Home (`document::to_home`), a discount
/// fraction (AP-DISC-02), and a Home amount to the Group's Reporting Currency (Q273).
// ponytail: minor unit fixed at 2; Reference Data when a Scenario needs another (Q182).
pub fn round_money(value: Decimal) -> Decimal {
    value.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

/// An amount in the Group's Reporting Currency (CASH-GROUP-01). The field is private and the only
/// constructor takes a `HomeAmount` and the owner's rate (Q273), so a Group total cannot be
/// produced without a rate lookup and a missing rate is CASH-GROUP-02's exclusion by construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReportingAmount(Decimal);

impl ReportingAmount {
    /// Zero, the one amount that is itself in every currency and so needs no rate. It is what a
    /// sum of Reporting amounts starts from, not a way around `convert` (Q273).
    pub const ZERO: Self = Self(Decimal::ZERO);

    /// Converts once and rounds once, at the rate the owner set for the pair — their current
    /// belief, not a market fact (`CONTEXT.md`, Q30). An Entity whose Home Currency is the
    /// Reporting Currency converts at one.
    pub fn convert(home: HomeAmount, rate: Decimal) -> Self {
        // As `document::to_home` does, an unrepresentable product falls back to the unconverted
        // figure rather than to zero: `core` never panics on data and never invents one.
        Self(round_money(home.0.checked_mul(rate).unwrap_or(home.0)))
    }

    /// Two amounts already in the Reporting Currency, which is the only sum this type allows.
    pub fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    pub fn amount(self) -> Decimal {
        self.0
    }
}
