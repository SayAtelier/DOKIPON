//! Money is integer minor units + ISO-4217 currency. Never floats.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Currency {
    Jpy,
    Usd,
    Eur,
    Cny,
    Twd,
}

impl Currency {
    pub fn as_str(self) -> &'static str {
        match self {
            Currency::Jpy => "JPY",
            Currency::Usd => "USD",
            Currency::Eur => "EUR",
            Currency::Cny => "CNY",
            Currency::Twd => "TWD",
        }
    }

    /// JPY has no minor unit; everything else uses 2 decimals.
    pub fn exponent(self) -> u32 {
        match self {
            Currency::Jpy => 0,
            _ => 2,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "JPY" => Some(Currency::Jpy),
            "USD" => Some(Currency::Usd),
            "EUR" => Some(Currency::Eur),
            "CNY" => Some(Currency::Cny),
            "TWD" => Some(Currency::Twd),
            _ => None,
        }
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// `amount_minor` is in the currency's smallest unit (e.g. cents).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Money {
    pub amount_minor: i64,
    pub currency: Currency,
}

impl Money {
    pub fn new(amount_minor: i64, currency: Currency) -> Result<Self, MoneyError> {
        if amount_minor < 0 {
            return Err(MoneyError::Negative);
        }
        Ok(Self { amount_minor, currency })
    }

    pub fn zero(currency: Currency) -> Self {
        Self { amount_minor: 0, currency }
    }

    pub fn checked_add(self, rhs: Money) -> Result<Money, MoneyError> {
        if self.currency != rhs.currency {
            return Err(MoneyError::CurrencyMismatch);
        }
        self.amount_minor
            .checked_add(rhs.amount_minor)
            .map(|amount_minor| Money { amount_minor, currency: self.currency })
            .ok_or(MoneyError::Overflow)
    }

    /// Quantity multiplication, overflow-checked.
    pub fn checked_mul(self, qty: u32) -> Result<Money, MoneyError> {
        self.amount_minor
            .checked_mul(qty as i64)
            .map(|amount_minor| Money { amount_minor, currency: self.currency })
            .ok_or(MoneyError::Overflow)
    }

    /// Rounding half-up on the minor unit; used for line-item totals.
    pub fn checked_mul_rounded(self, qty: u32) -> Result<Money, MoneyError> {
        let product = (self.amount_minor as i128) * (qty as i128);
        let exp = 10i128.pow(self.currency.exponent());
        let rounded = (product + exp / 2) / exp * exp;
        if rounded > i64::MAX as i128 {
            return Err(MoneyError::Overflow);
        }
        Money::new(rounded as i64, self.currency)
    }

    pub fn is_zero(self) -> bool {
        self.amount_minor == 0
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.currency.exponent() == 0 {
            write!(f, "{} {}", self.amount_minor, self.currency)
        } else {
            write!(f, "{:.2} {}", self.amount_minor as f64 / 100.0, self.currency)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MoneyError {
    #[error("money amount must not be negative")]
    Negative,
    #[error("currency mismatch in money arithmetic")]
    CurrencyMismatch,
    #[error("money arithmetic overflow")]
    Overflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_negative() {
        assert!(Money::new(-1, Currency::Usd).is_err());
    }

    #[test]
    fn rejects_cross_currency_addition() {
        let a = Money::new(100, Currency::Usd).unwrap();
        let b = Money::new(100, Currency::Jpy).unwrap();
        assert_eq!(a.checked_add(b), Err(MoneyError::CurrencyMismatch));
    }

    #[test]
    fn overflow_is_an_error_not_a_wrap() {
        let m = Money::new(i64::MAX, Currency::Usd).unwrap();
        assert_eq!(m.checked_add(Money::new(1, Currency::Usd).unwrap()), Err(MoneyError::Overflow));
        assert_eq!(m.checked_mul(2), Err(MoneyError::Overflow));
    }

    #[test]
    fn quantity_multiply() {
        let unit = Money::new(1250, Currency::Usd).unwrap();
        assert_eq!(unit.checked_mul(3).unwrap().amount_minor, 3750);
    }

    #[test]
    fn jpy_has_zero_exponent() {
        assert_eq!(Currency::Jpy.exponent(), 0);
        assert_eq!(Currency::Usd.exponent(), 2);
    }
}
