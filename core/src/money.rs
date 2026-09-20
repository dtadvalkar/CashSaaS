//! The `Money` shape every monetary field uses (`docs/facts.md`, Money).

use rust_decimal::Decimal;

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
