//! The numbers behind every router plan: how much to swap, borrow, repay and redeem, and the minimums each step
//! carries.
//!
//! Not a contract port: this is what an SDK client works out before it builds a transaction, so every client builds
//! the same one. A client reads the chain, calls these, and turns the results into router actions on its own routes.
//! A step that needs a swap quote is split in two: one function sizes the swap, the client quotes it, a second turns
//! the quote into a minimum.
//!
//! One module per action, as the TypeScript client's `flows/` folder.
//! [`range`] also holds tick alignment and the `f64` prices shown for ticks: display only, never for sizing.

pub mod close;
pub mod edit;
pub mod lend;
pub mod liquidate;
pub mod open;
pub mod range;

use alloy_primitives::U256;

use crate::error::Result;
use crate::num::{Checked, pow10};
use crate::types::Side;

/// A token's USD price and decimals: what converting between two tokens at the oracle needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct Priced {
    /// USD per whole token, 1e18 = $1.
    pub price_wad: U256,
    /// ERC-20 decimals.
    pub decimals: u8,
}

impl Priced {
    /// `amount` in USD (1e18), rounded down.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`](crate::Error::Overflow) on an absurd amount.
    pub fn usd(&self, amount: U256) -> Result<U256> {
        amount.try_mul(self.price_wad)?.try_div(pow10(self.decimals)?)
    }

    /// How much of the token `usd_wad` buys, rounded down.
    ///
    /// # Errors
    ///
    /// [`Error::DivisionByZero`](crate::Error::DivisionByZero) for a zero price.
    pub fn amount(&self, usd_wad: U256) -> Result<U256> {
        usd_wad.try_mul(pow10(self.decimals)?)?.try_div(self.price_wad)
    }
}

/// One value for each of the pair's tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct PerToken<T> {
    /// Token0's.
    pub token0: T,
    /// Token1's.
    pub token1: T,
}

impl<T: Copy> PerToken<T> {
    /// The value of one token.
    #[must_use]
    pub const fn get(&self, side: Side) -> T {
        match side {
            Side::Token0 => self.token0,
            Side::Token1 => self.token1,
        }
    }

    /// Replaces the value of one token.
    pub const fn set(&mut self, side: Side, value: T) {
        match side {
            Side::Token0 => self.token0 = value,
            Side::Token1 => self.token1 = value,
        }
    }
}

/// `x · (10⁴ + bps) / 10⁴`, rounded down.
fn plus_bps(x: U256, bps: u64) -> Result<U256> {
    let scaled = 10_000_u64.checked_add(bps).ok_or(crate::Error::Overflow)?;
    x.try_mul(U256::from(scaled))?.try_div(U256::from(10_000_u64))
}

/// `x · (10⁴ - bps) / 10⁴`, rounded down.
fn minus_bps(x: U256, bps: u64) -> Result<U256> {
    let keep = 10_000_u64.checked_sub(bps).ok_or(crate::Error::Underflow)?;
    x.try_mul(U256::from(keep))?.try_div(U256::from(10_000_u64))
}

/// `x` grossed up for a swap fee: `x · 10¹⁸ / (10¹⁸ - fee)`, rounded down.
fn with_fee(x: U256, swap_fee_wad: U256) -> Result<U256> {
    x.try_mul(crate::WAD)?.try_div(crate::WAD.try_sub(swap_fee_wad)?)
}
