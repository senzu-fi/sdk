//! Ranges: aligning ticks to a pool's spacing, which tokens a range takes, the paired amount of a deposit, and the
//! prices shown for ticks and sqrt prices.
//!
//! The price conversions are `f64`, for display only: never size a transaction with them. Everything else is exact.

use alloy_primitives::{U160, U256};

use super::PerToken;
use crate::error::{Error, Result};
use crate::lens::REF_LIQUIDITY;
use crate::num::Checked;
use crate::types::Side;
use crate::uniswap::{MAX_TICK, amounts_for_liquidity, sqrt_price_at_tick};

/// Which way [`align_tick`] moves a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub enum TickDirection {
    /// To the usable tick at or below.
    Down,
    /// To the usable tick at or above.
    Up,
}

/// The highest tick a pool with this spacing allows: the largest multiple of the spacing within [`MAX_TICK`].
///
/// # Errors
///
/// [`Error::InvalidTickSpacing`] for a spacing below 1.
pub fn max_usable_tick(spacing: i32) -> Result<i32> {
    if spacing < 1 {
        return Err(Error::InvalidTickSpacing(spacing));
    }
    Ok(MAX_TICK.div_euclid(spacing).wrapping_mul(spacing))
}

/// Past every usable tick, so saturating there changes no aligned result.
const TICK_SATURATION: f64 = 4_000_000.0;

/// `⌊x⌋` as an integer, saturated at ±[`TICK_SATURATION`].
fn floor_tick(x: f64) -> Result<i64> {
    if x.is_nan() {
        return Err(Error::TickNotANumber);
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "whole and within +/-4e6 after the clamp, so the cast is exact"
    )]
    Ok(x.floor().clamp(-TICK_SATURATION, TICK_SATURATION) as i64)
}

fn clamp(steps: i64, spacing: i32) -> Result<i32> {
    let limit = i64::from(max_usable_tick(spacing)?);
    let tick = steps.wrapping_mul(i64::from(spacing));
    i32::try_from(tick.clamp(limit.wrapping_neg(), limit)).map_err(|_| Error::CastOverflow)
}

/// `tick` (fractional for an exact price) moved down or up to a multiple of the spacing, kept inside the ticks a
/// pool allows.
///
/// # Errors
///
/// [`Error::InvalidTickSpacing`] for a spacing below 1, [`Error::TickNotANumber`] for NaN.
pub fn align_tick(tick: f64, spacing: i32, direction: TickDirection) -> Result<i32> {
    let s = i64::from(spacing);
    max_usable_tick(spacing)?;
    // ⌊t/s⌋ = ⌊⌊t⌋/s⌋ and ⌈t/s⌉ = -⌊⌊-t⌋/s⌋ for a whole s > 0
    let steps = match direction {
        TickDirection::Down => floor_tick(tick)?.div_euclid(s),
        TickDirection::Up => floor_tick(-tick)?.div_euclid(s).wrapping_neg(),
    };
    clamp(steps, spacing)
}

/// The multiple of the spacing nearest `tick` (fractional for an exact price; halfway rounds up), kept inside the
/// ticks a pool allows.
///
/// # Errors
///
/// [`Error::InvalidTickSpacing`] for a spacing below 1, [`Error::TickNotANumber`] for NaN.
pub fn nearest_usable_tick(tick: f64, spacing: i32) -> Result<i32> {
    let s = i64::from(spacing);
    max_usable_tick(spacing)?;
    // ⌊t/s + ½⌋ = ⌊(⌊2t⌋ + s) / 2s⌋
    let steps = floor_tick(2.0 * tick)?.wrapping_add(s).div_euclid(s.wrapping_mul(2));
    clamp(steps, spacing)
}

/// The widest range a pool with this spacing allows, as `(tick_lower, tick_upper)`.
///
/// # Errors
///
/// [`Error::InvalidTickSpacing`] for a spacing below 1.
pub fn full_range(spacing: i32) -> Result<(i32, i32)> {
    let limit = max_usable_tick(spacing)?;
    Ok((limit.wrapping_neg(), limit))
}

/// Which tokens a range takes at the pool's tick: only token0 above the price, only token1 below it, both in range.
#[must_use]
pub const fn accepted_tokens(tick_lower: i32, tick_upper: i32, pool_tick: i32) -> PerToken<bool> {
    if pool_tick < tick_lower {
        PerToken {
            token0: true,
            token1: false,
        }
    } else if pool_tick >= tick_upper {
        PerToken {
            token0: false,
            token1: true,
        }
    } else {
        PerToken {
            token0: true,
            token1: true,
        }
    }
}

/// Without a swap, the amount of the other token that fits `amount` of `from` in the range at the pool price, in
/// the ratio the range holds there; zero when the range takes none of `from`.
///
/// # Errors
///
/// As [`sqrt_price_at_tick`] and [`amounts_for_liquidity`].
pub fn paired_amount(
    tick_lower: i32,
    tick_upper: i32,
    pool_sqrt_price_x96: U160,
    from: Side,
    amount: U256,
) -> Result<U256> {
    let (a, b) = (sqrt_price_at_tick(tick_lower)?, sqrt_price_at_tick(tick_upper)?);
    let (r0, r1) = amounts_for_liquidity(pool_sqrt_price_x96, a, b, REF_LIQUIDITY)?;
    let (have, want) = match from {
        Side::Token0 => (r0, r1),
        Side::Token1 => (r1, r0),
    };
    if have.is_zero() {
        return Ok(U256::ZERO);
    }
    amount.try_mul(want)?.try_div(have)
}

/// `10^(decimals0 - decimals1)`: what turns a raw price into token1 per token0 in whole tokens.
pub(crate) fn decimal_shift(decimals0: u8, decimals1: u8) -> f64 {
    10_f64.powf(f64::from(decimals0) - f64::from(decimals1))
}

/// A Q64.96 sqrt price as a plain number (rounded to the nearest `f64`).
pub(crate) fn sqrt_ratio(sqrt_price_x96: U160) -> f64 {
    f64::from(U256::from(sqrt_price_x96)) / 2_f64.powi(96)
}

/// Token1 per token0, in whole tokens (e.g. USDG per ETH), at a tick. For display.
#[must_use]
pub fn tick_to_price(tick: i32, decimals0: u8, decimals1: u8) -> f64 {
    1.0001_f64.powf(f64::from(tick)) * decimal_shift(decimals0, decimals1)
}

/// Token1 per token0, in whole tokens, at an exact sqrt price: show prices from this, not from ticks (a tick floors
/// the price). For display.
#[must_use]
pub fn sqrt_price_to_price(sqrt_price_x96: U160, decimals0: u8, decimals1: u8) -> f64 {
    sqrt_ratio(sqrt_price_x96).powi(2) * decimal_shift(decimals0, decimals1)
}

/// A Q64.96 sqrt price as a raw number (token1 base units per token0 base unit, square-rooted). For display.
#[must_use]
pub fn raw_sqrt_price(sqrt_price_x96: U160) -> f64 {
    sqrt_ratio(sqrt_price_x96)
}

/// `x` rounded down to an integer; `None` when negative, not a number or over 256 bits.
fn floor_to_u256(x: f64) -> Option<U256> {
    if x.is_nan() || x < 0.0 || !x.is_finite() {
        return None;
    }
    let bits = x.to_bits();
    let exponent = i64::try_from((bits >> 52) & 0x7ff).ok()?;
    if exponent == 0 {
        return Some(U256::ZERO);
    }
    let mantissa = U256::from((bits & ((1 << 52) - 1)) | (1 << 52));
    let shift = exponent.checked_sub(1075)?;
    let magnitude = usize::try_from(shift.unsigned_abs()).ok()?;
    if shift >= 0 {
        mantissa.checked_shl(magnitude)
    } else {
        // Drops the fraction (zero past 256 bits): the floor
        Some(mantissa.wrapping_shr(magnitude))
    }
}

/// The Q64.96 sqrt price at a pool `price` (token1 per token0, whole tokens), rounded down. For display and for
/// approximate inputs, not for exact amounts.
///
/// # Errors
///
/// [`Error::PriceNotUsable`] for a negative or non-finite price, or one past 160 bits.
pub fn sqrt_price_x96_at_price(price: f64, decimals0: u8, decimals1: u8) -> Result<U160> {
    let sqrt = (price / decimal_shift(decimals0, decimals1)).sqrt() * 2_f64.powi(96);
    let value = floor_to_u256(sqrt).ok_or(Error::PriceNotUsable)?;
    U160::checked_from_limbs_slice(value.as_limbs()).ok_or(Error::PriceNotUsable)
}

/// The tick whose price is nearest `price` (token1 per token0, whole tokens; halfway rounds up). Align it to the
/// pool's spacing before use.
#[must_use]
pub fn price_to_tick(price: f64, decimals0: u8, decimals1: u8) -> f64 {
    let exact = (price / decimal_shift(decimals0, decimals1)).ln() / 1.0001_f64.ln();
    let floor = exact.floor();
    if exact - floor >= 0.5 { floor + 1.0 } else { floor }
}

/// The pool price as a fractional tick: where inside its tick the price sits, which a tick rounds down. For display.
#[must_use]
pub fn exact_tick_at_sqrt_price(sqrt_price_x96: U160) -> f64 {
    2.0 * sqrt_ratio(sqrt_price_x96).ln() / 1.0001_f64.ln()
}

/// A position's liquidation prices as pool prices (token1 per token0, whole tokens, unsorted), and the pool price
/// now at oracle prices. For display.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct LiquidationPoolPrices {
    /// At the moving token's lower USD liquidation price; `None` without one.
    pub at_lower: Option<f64>,
    /// At its upper one.
    pub at_upper: Option<f64>,
    /// At the oracle prices now.
    pub now: f64,
}

/// USD (1e18) as a plain number.
fn usd(wad: U256) -> f64 {
    f64::from(wad) / 1e18
}

/// The pool prices where the position is liquidated: token0's USD price over token1's, the `moving` token's taken at
/// its liquidation price (the lens's `lowerWad` / `upperWad`, zero or `None` for none). For display.
#[must_use]
pub fn liquidation_pool_prices(
    moving: Side,
    lower_wad: Option<U256>,
    upper_wad: Option<U256>,
    price0_wad: U256,
    price1_wad: U256,
) -> LiquidationPoolPrices {
    let p1 = usd(price1_wad);
    let at = |wad: Option<U256>| {
        wad.filter(|w| !w.is_zero()).and_then(|w| match moving {
            Side::Token0 => (p1 > 0.0).then(|| usd(w) / p1),
            Side::Token1 => Some(usd(price0_wad) / usd(w)),
        })
    };
    LiquidationPoolPrices {
        at_lower: at(lower_wad),
        at_upper: at(upper_wad),
        now: if p1 > 0.0 { usd(price0_wad) / p1 } else { 0.0 },
    }
}

#[cfg(test)]
mod display_tests {
    use super::*;

    #[test]
    fn a_price_round_trips_through_its_sqrt() {
        for price in [1e-9, 0.5, 1.0, 3_000.0, 1e12] {
            let sqrt = sqrt_price_x96_at_price(price, 18, 6).unwrap();
            assert!((sqrt_price_to_price(sqrt, 18, 6) / price - 1.0).abs() < 1e-12);
        }
        assert_eq!(sqrt_price_x96_at_price(1.0, 18, 18).unwrap(), U160::from(1_u8) << 96);
        assert!(sqrt_price_x96_at_price(-1.0, 18, 18).is_err() && sqrt_price_x96_at_price(f64::NAN, 18, 18).is_err());
        assert_eq!(floor_to_u256(2.9), Some(U256::from(2_u8)));
        assert_eq!(floor_to_u256(0.4), Some(U256::ZERO));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_align_like_the_pools_do() {
        assert_eq!(align_tick(-61.0, 60, TickDirection::Down).unwrap(), -120);
        assert_eq!(align_tick(-61.0, 60, TickDirection::Up).unwrap(), -60);
        assert_eq!(align_tick(61.0, 60, TickDirection::Down).unwrap(), 60);
        assert_eq!(align_tick(61.0, 60, TickDirection::Up).unwrap(), 120);
        assert_eq!(align_tick(f64::from(MAX_TICK), 60, TickDirection::Up).unwrap(), 887_220);
        assert_eq!(full_range(60).unwrap(), (-887_220, 887_220));
        assert_eq!(align_tick(-60.5, 60, TickDirection::Up).unwrap(), -60);
        assert_eq!(align_tick(-0.5, 60, TickDirection::Down).unwrap(), -60);
        assert_eq!(align_tick(5.0, 0, TickDirection::Up), Err(Error::InvalidTickSpacing(0)));
        assert_eq!(align_tick(f64::NAN, 60, TickDirection::Up), Err(Error::TickNotANumber));
        assert_eq!(align_tick(f64::NEG_INFINITY, 60, TickDirection::Up).unwrap(), -887_220);
    }

    #[test]
    fn nearest_rounds_halfway_up() {
        assert_eq!(nearest_usable_tick(30.0, 60).unwrap(), 60);
        assert_eq!(nearest_usable_tick(-30.0, 60).unwrap(), 0);
        assert_eq!(nearest_usable_tick(-31.0, 60).unwrap(), -60);
        assert_eq!(nearest_usable_tick(29.9, 60).unwrap(), 0);
        assert_eq!(nearest_usable_tick(-30.5, 60).unwrap(), -60);
    }

    #[test]
    fn a_range_takes_one_token_outside_and_both_inside() {
        assert_eq!(
            accepted_tokens(-60, 60, -61),
            PerToken {
                token0: true,
                token1: false
            }
        );
        assert_eq!(
            accepted_tokens(-60, 60, 60),
            PerToken {
                token0: false,
                token1: true
            }
        );
        assert_eq!(
            accepted_tokens(-60, 60, 0),
            PerToken {
                token0: true,
                token1: true
            }
        );
    }

    #[test]
    fn prices_and_ticks_round_trip() {
        let price = tick_to_price(200_000, 18, 6);
        assert!((price_to_tick(price, 18, 6) - 200_000.0).abs() < f64::EPSILON);
        let sqrt = sqrt_price_at_tick(200_000).unwrap();
        assert!((sqrt_price_to_price(sqrt, 18, 6) / price - 1.0).abs() < 1e-9);
        assert!((exact_tick_at_sqrt_price(sqrt) - 200_000.0).abs() < 1e-6);
    }
}
