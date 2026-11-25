//! Slippage bounds for adding and removing liquidity.
//!
//! Not a contract port: these are the minimums and maximums an SDK writes into a router plan, so the plan reverts
//! when the pool moves further than the caller accepts. Every SDK must compute them the same way, so they live here.
//!
//! A slippage of `s` basis points accepts any pool price within `[(1 - s) · P, (1 + s) · P]`; on sqrt prices that is
//! `√P · √(1 ∓ s)`. Each bound is the worst a deposit or a redeem can do anywhere in that band.

use alloy_primitives::{U160, U256, uint};

use crate::error::{Error, Result};
use crate::num::Checked;
use crate::uniswap::{amounts_for_liquidity, liquidity_for_amounts, sqrt};

/// Basis points in 100%.
pub const BPS: u64 = 10_000;

/// `√(1 ∓ s)` is computed to 12 decimals: `⌊√((1 ∓ s) · 10²⁴)⌋ / 10¹²`.
const SCALE: U256 = U256::from_limbs([1_000_000_000_000, 0, 0, 0]);

/// `10^20`, scaling basis points (`10^4`) into `10^24` before taking the square root.
const TEN_20: U256 = uint!(100_000_000_000_000_000_000_U256);

/// The band's lowest and highest sqrt price, `√P · √(1 - s)` and `√P · √(1 + s)`, both rounded down.
///
/// # Errors
///
/// [`Error::Underflow`] for a slippage above 100%, [`Error::CastOverflow`] when the upper end does not fit in
/// 160 bits.
pub fn price_band(sqrt_price_x96: U160, slippage_bps: u64) -> Result<(U160, U160)> {
    let bound = |percent_bps: u64| -> Result<U160> {
        // (1 ∓ s) · 10²⁴ = (10⁴ ∓ bps) · 10²⁰
        let factor = sqrt(U256::from(percent_bps).try_mul(TEN_20)?)?;
        let bounded = U256::from(sqrt_price_x96).try_mul(factor)?.try_div(SCALE)?;
        U160::checked_from_limbs_slice(bounded.as_limbs()).ok_or(Error::CastOverflow)
    };
    let below = BPS.checked_sub(slippage_bps).ok_or(Error::Underflow)?;
    let above = BPS.checked_add(slippage_bps).ok_or(Error::Overflow)?;
    Ok((bound(below)?, bound(above)?))
}

/// The least a swap quoted at `amount` may pay: `amount · (1 - s)`, rounded down.
///
/// # Errors
///
/// [`Error::Underflow`] for a slippage above 100%.
pub fn min_out(amount: U256, slippage_bps: u64) -> Result<U256> {
    let keep = BPS.checked_sub(slippage_bps).ok_or(Error::Underflow)?;
    amount.try_mul(U256::from(keep))?.try_div(U256::from(BPS))
}

/// The quote a minimum was written from: `min_out / (1 - s)`, rounded down. A firm quote can come in under an
/// indicative one; this reads what it was from the minimum a quote API returns.
///
/// # Errors
///
/// [`Error::Underflow`] for a slippage of 100% or more.
pub fn quote_for_min(min_out: U256, slippage_bps: u64) -> Result<U256> {
    let keep = BPS
        .checked_sub(slippage_bps)
        .filter(|k| *k > 0)
        .ok_or(Error::Underflow)?;
    min_out.try_mul(U256::from(BPS))?.try_div(U256::from(keep))
}

/// What to pay for `wanted` at a quote's rate (`amount_in` for `amount_out`), rounded up so it buys at least
/// `wanted`; zero for a quote paying nothing.
///
/// # Errors
///
/// [`Error::Overflow`] on absurd amounts.
pub fn input_for_output(wanted: U256, amount_in: U256, amount_out: U256) -> Result<U256> {
    if amount_out.is_zero() {
        return Ok(U256::ZERO);
    }
    wanted
        .try_mul(amount_in)?
        .try_add(amount_out.try_sub(U256::from(1_u8))?)?
        .try_div(amount_out)
}

/// The least of each token a deposit of up to `amount0` and `amount1` takes anywhere in the band: what the router's
/// minimums for an add must be. One wei of each amount is held back, as the contracts round against the depositor.
///
/// # Errors
///
/// As [`price_band`], [`liquidity_for_amounts`] and [`amounts_for_liquidity`].
pub fn deposit_minimums(
    sqrt_pa_x96: U160,
    sqrt_pb_x96: U160,
    sqrt_price_x96: U160,
    amount0: U256,
    amount1: U256,
    slippage_bps: u64,
) -> Result<(U256, U256)> {
    let less = |amount: U256| amount.saturating_sub(U256::from(1_u8));
    let taken = |price: U160| -> Result<(U256, U256)> {
        let liquidity = liquidity_for_amounts(price, sqrt_pa_x96, sqrt_pb_x96, less(amount0), less(amount1))?;
        amounts_for_liquidity(price, sqrt_pa_x96, sqrt_pb_x96, liquidity)
    };
    let (low, high) = price_band(sqrt_price_x96, slippage_bps)?;
    let (at_low, at_high) = (taken(low)?, taken(high)?);
    Ok((at_low.0.min(at_high.0), at_low.1.min(at_high.1)))
}

/// The most of each token minting `liquidity` takes anywhere in the band, plus one wei: what the router's maximums
/// for a mint must be.
///
/// # Errors
///
/// As [`price_band`] and [`amounts_for_liquidity`]; [`Error::Overflow`] when a maximum is `U256::MAX`.
pub fn deposit_maximums(
    sqrt_pa_x96: U160,
    sqrt_pb_x96: U160,
    sqrt_price_x96: U160,
    liquidity: u128,
    slippage_bps: u64,
) -> Result<(U256, U256)> {
    let [at_low, at_high] = withdraw_ends(sqrt_pa_x96, sqrt_pb_x96, sqrt_price_x96, liquidity, slippage_bps)?;
    let one = U256::from(1_u8);
    Ok((
        at_low.0.max(at_high.0).try_add(one)?,
        at_low.1.max(at_high.1).try_add(one)?,
    ))
}

/// What `liquidity` holds at the band's low end and at its high end.
///
/// # Errors
///
/// As [`price_band`] and [`amounts_for_liquidity`].
pub fn withdraw_ends(
    sqrt_pa_x96: U160,
    sqrt_pb_x96: U160,
    sqrt_price_x96: U160,
    liquidity: u128,
    slippage_bps: u64,
) -> Result<[(U256, U256); 2]> {
    let (low, high) = price_band(sqrt_price_x96, slippage_bps)?;
    Ok([
        amounts_for_liquidity(low, sqrt_pa_x96, sqrt_pb_x96, liquidity)?,
        amounts_for_liquidity(high, sqrt_pa_x96, sqrt_pb_x96, liquidity)?,
    ])
}

/// The least of each token redeeming `liquidity` pays anywhere in the band: what the router's minimums for a redeem
/// must be. Token0 is fewest at the high end, token1 at the low end.
///
/// # Errors
///
/// As [`price_band`] and [`amounts_for_liquidity`].
pub fn withdraw_minimums(
    sqrt_pa_x96: U160,
    sqrt_pb_x96: U160,
    sqrt_price_x96: U160,
    liquidity: u128,
    slippage_bps: u64,
) -> Result<(U256, U256)> {
    let [at_low, at_high] = withdraw_ends(sqrt_pa_x96, sqrt_pb_x96, sqrt_price_x96, liquidity, slippage_bps)?;
    Ok((at_high.0, at_low.1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uniswap::{Q96, sqrt_price_at_tick};

    fn q96() -> U160 {
        U160::from(Q96)
    }

    #[test]
    fn a_minimum_reads_back_its_quote() {
        let quote = U256::from(1_000_000_u64);
        let min = min_out(quote, 100).unwrap();
        assert_eq!(quote_for_min(min, 100).unwrap(), quote);
        assert!(quote_for_min(min, BPS).is_err());
        assert_eq!(
            input_for_output(U256::from(3_u8), U256::from(10_u8), U256::from(4_u8)).unwrap(),
            U256::from(8_u8)
        );
        assert_eq!(
            input_for_output(U256::from(3_u8), U256::from(10_u8), U256::ZERO).unwrap(),
            U256::ZERO
        );
    }

    #[test]
    fn zero_slippage_is_the_price_itself() {
        assert_eq!(price_band(q96(), 0).unwrap(), (q96(), q96()));
    }

    #[test]
    fn band_is_the_exact_root_rounded_down() {
        // √0.81 = 0.9 and √1.21 = 1.1 are exact: 19% below and 21% above
        assert_eq!(
            price_band(q96(), 1_900).unwrap().0,
            U160::from(Q96 * U256::from(9) / U256::from(10))
        );
        assert_eq!(
            price_band(q96(), 2_100).unwrap().1,
            U160::from(Q96 * U256::from(11) / U256::from(10))
        );
        // √1.1664 = 1.08, where a float square root lands one unit low
        let (_, high) = price_band(U160::from(1_000_000_000_000_u64), 1_664).unwrap();
        assert_eq!(high, U160::from(1_080_000_000_000_u64));
    }

    #[test]
    fn more_than_all_is_an_error() {
        assert_eq!(price_band(q96(), BPS).unwrap().0, U160::ZERO);
        assert_eq!(price_band(q96(), BPS + 1), Err(Error::Underflow));
    }

    #[test]
    fn bounds_hold_what_the_band_can_do() {
        let (a, b) = (sqrt_price_at_tick(-600).unwrap(), sqrt_price_at_tick(600).unwrap());
        let price = sqrt_price_at_tick(17).unwrap();
        let liquidity = 10_u128.pow(21);
        let now = amounts_for_liquidity(price, a, b, liquidity).unwrap();
        let (min0, min1) = withdraw_minimums(a, b, price, liquidity, 100).unwrap();
        let (max0, max1) = deposit_maximums(a, b, price, liquidity, 100).unwrap();
        assert!(min0 < now.0 && now.0 < max0);
        assert!(min1 < now.1 && now.1 < max1);
        let (dmin0, dmin1) = deposit_minimums(a, b, price, now.0, now.1, 100).unwrap();
        assert!(dmin0 <= now.0 && dmin1 <= now.1);
    }
}
