//! A leveraged position's payoff: what it holds and is worth as the pool price moves. `f64`, for charts.

use alloy_primitives::{U160, U256};

use super::{to_f64, units};
use crate::flows::PerToken;
use crate::flows::range::{decimal_shift, sqrt_ratio};

/// Q192 (2^192): a sqrt price's inverse is `Q192 / sqrt`.
const Q192: U256 = U256::from_limbs([0, 0, 0, 1]);
/// Hours in a day, as the daily move scales an hourly variance.
const HOURS_PER_DAY: f64 = 24.0;
/// Hourly closes needed for a daily move.
const MIN_HISTORY_HOURS: usize = 24;

/// A position to chart.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct PayoffInputs {
    /// Its liquidity.
    pub liquidity: U256,
    /// The range's lower sqrt price.
    pub sqrt_lower_x96: U160,
    /// Its upper sqrt price.
    pub sqrt_upper_x96: U160,
    /// Token0's decimals.
    pub decimals0: u8,
    /// Token1's decimals.
    pub decimals1: u8,
    /// Token0 owed, the opening fee included.
    pub debt0: U256,
    /// Token1 owed.
    pub debt1: U256,
    /// The pool price now, token1 per token0 in whole tokens.
    pub price: f64,
    /// Token1's USD price.
    pub token1_usd: f64,
}

/// What the position holds at a pool `price` (whole tokens).
#[must_use]
pub fn amounts_at(i: &PayoffInputs, price: f64) -> PerToken<f64> {
    let l = to_f64(i.liquidity);
    let sa = sqrt_ratio(i.sqrt_lower_x96);
    let sb = sqrt_ratio(i.sqrt_upper_x96);
    let s = (price / decimal_shift(i.decimals0, i.decimals1)).sqrt().max(sa).min(sb);
    PerToken {
        token0: (l * (sb - s)) / (s * sb) / 10_f64.powi(i32::from(i.decimals0)),
        token1: (l * (s - sa)) / 10_f64.powi(i32::from(i.decimals1)),
    }
}

/// Holdings less debts at a pool `price` in USD, priced at `price`.
fn net_value(i: &PayoffInputs, held: PerToken<f64>, price: f64) -> f64 {
    let debt0 = units(i.debt0, i.decimals0);
    let debt1 = units(i.debt1, i.decimals1);
    ((held.token0 - debt0) * price + held.token1 - debt1) * i.token1_usd
}

/// The position less its debts in USD at a pool `price`.
#[must_use]
pub fn equity_at(i: &PayoffInputs, price: f64) -> f64 {
    net_value(i, amounts_at(i, price), price)
}

/// Profit or loss in USD if the pool price goes from now to `price` (fees earned meanwhile not included).
#[must_use]
pub fn pnl_at(i: &PayoffInputs, price: f64) -> f64 {
    equity_at(i, price) - equity_at(i, i.price)
}

/// [`pnl_at`] at each price.
#[must_use]
pub fn pnl_curve(i: &PayoffInputs, prices: &[f64]) -> Vec<f64> {
    let l = to_f64(i.liquidity);
    let sa = sqrt_ratio(i.sqrt_lower_x96);
    let sb = sqrt_ratio(i.sqrt_upper_x96);
    let shift = decimal_shift(i.decimals0, i.decimals1);
    let scale0 = 10_f64.powi(i32::from(i.decimals0));
    let scale1 = 10_f64.powi(i32::from(i.decimals1));
    let debt0 = to_f64(i.debt0) / scale0;
    let debt1 = to_f64(i.debt1) / scale1;
    let token1_usd = i.token1_usd;
    let equity = |price: f64| {
        let s = (price / shift).sqrt().max(sa).min(sb);
        let token0 = (l * (sb - s)) / (s * sb) / scale0;
        let token1 = (l * (s - sa)) / scale1;
        ((token0 - debt0) * price + token1 - debt1) * token1_usd
    };
    let now = equity(i.price);
    prices.iter().map(|&price| equity(price) - now).collect()
}

/// Profit or loss in USD had the tokens and debts the position holds now simply been held to `price`.
#[must_use]
pub fn hold_pnl_at(i: &PayoffInputs, price: f64) -> f64 {
    net_value(i, amounts_at(i, i.price), price) - equity_at(i, i.price)
}

/// Net token0 exposure now: what the position holds of it less what it owes (positive is long).
#[must_use]
pub fn exposure0(i: &PayoffInputs) -> f64 {
    amounts_at(i, i.price).token0 - units(i.debt0, i.decimals0)
}

/// The range's ends as pool prices (token1 per token0, whole tokens).
#[must_use]
pub fn range_prices(i: &PayoffInputs) -> (f64, f64) {
    let shift = decimal_shift(i.decimals0, i.decimals1);
    (
        sqrt_ratio(i.sqrt_lower_x96).powi(2) * shift,
        sqrt_ratio(i.sqrt_upper_x96).powi(2) * shift,
    )
}

/// The inverse of a sqrt price (`Q192 / sqrt`, rounded down); zero for zero.
fn inverse_sqrt(sqrt: U160) -> U160 {
    let sqrt = U256::from(sqrt);
    if sqrt.is_zero() {
        return U160::ZERO;
    }
    U160::saturating_from(Q192.checked_div(sqrt).unwrap_or(U256::ZERO))
}

/// `1 / x` for a positive finite `x`; zero otherwise.
fn invert_price(x: f64) -> f64 {
    if x > 0.0 && x.is_finite() { 1.0 / x } else { 0.0 }
}

/// The same position seen from token1: the tokens swap places, the range and price invert, and the new token1 (the
/// old token0) is priced at the old price in USD.
#[must_use]
pub fn flip(i: &PayoffInputs) -> PayoffInputs {
    PayoffInputs {
        liquidity: i.liquidity,
        sqrt_lower_x96: inverse_sqrt(i.sqrt_upper_x96),
        sqrt_upper_x96: inverse_sqrt(i.sqrt_lower_x96),
        decimals0: i.decimals1,
        decimals1: i.decimals0,
        debt0: i.debt1,
        debt1: i.debt0,
        price: invert_price(i.price),
        token1_usd: i.price * i.token1_usd,
    }
}

/// One standard deviation of a day's log return (a fraction), from hourly closes; `None` with under a day of them.
#[must_use]
pub fn typical_daily_move(hourly_closes: &[f64]) -> Option<f64> {
    if hourly_closes.len() < MIN_HISTORY_HOURS {
        return None;
    }
    let returns: Vec<f64> = hourly_closes
        .iter()
        .zip(hourly_closes.iter().skip(1))
        .map(|(a, b)| (b / a).ln())
        .collect();
    let count = u32::try_from(returns.len()).map_or(f64::MAX, f64::from);
    let mean = returns.iter().sum::<f64>() / count;
    let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (count - 1.0);
    Some((variance * HOURS_PER_DAY).sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uniswap::sqrt_price_at_tick;

    fn inputs() -> PayoffInputs {
        PayoffInputs {
            liquidity: U256::from(10_u128.pow(18)),
            sqrt_lower_x96: sqrt_price_at_tick(-1_000).unwrap(),
            sqrt_upper_x96: sqrt_price_at_tick(1_000).unwrap(),
            decimals0: 18,
            decimals1: 18,
            debt0: U256::from(10_u128.pow(16)),
            debt1: U256::ZERO,
            price: 1.0,
            token1_usd: 1.0,
        }
    }

    #[test]
    fn nothing_moves_at_the_price_now() {
        let i = inputs();
        assert!(pnl_at(&i, 1.0).abs() < 1e-12);
        assert!(hold_pnl_at(&i, 1.0).abs() < 1e-12);
        assert_eq!(pnl_curve(&i, &[1.0, 1.05]), vec![pnl_at(&i, 1.0), pnl_at(&i, 1.05)]);
    }

    #[test]
    fn a_flip_twice_is_the_position_again() {
        let i = inputs();
        let back = flip(&flip(&i));
        assert_eq!((back.decimals0, back.debt0), (i.decimals0, i.debt0));
        assert!((exposure0(&flip(&i)) - amounts_at(&i, i.price).token1).abs() < 1e-9);
    }

    #[test]
    fn a_flat_series_does_not_move() {
        assert_eq!(typical_daily_move(&[2.0; 30]), Some(0.0));
        assert_eq!(typical_daily_move(&[2.0; 23]), None);
    }
}
