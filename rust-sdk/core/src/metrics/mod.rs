//! The numbers an app shows about markets and positions: prices a pool implies, a position valued at them, its profit,
//! borrow and supply rates, debt accrued since a read, yields and the vault's shares.
//!
//! Not a contract port, except where a function says so: what every client must show the same way, so it lives here.

pub mod liquidity;
pub mod payoff;
pub mod pnl;
pub mod position;
pub mod prices;
pub mod rates;
pub mod swap_fees;
pub mod vault;
pub mod yields;

use alloy_primitives::U256;

pub(super) fn to_f64(value: U256) -> f64 {
    f64::from(value)
}

/// `amount` in whole tokens.
pub(super) fn units(amount: U256, decimals: u8) -> f64 {
    to_f64(amount) / 10_f64.powi(i32::from(decimals))
}
