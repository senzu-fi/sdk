//! `TickMath` from Uniswap v4-core: sqrt prices at ticks (1.0001^(tick/2) · 2^96) and back.

use alloy_primitives::{I256, U160, U256, uint};

use crate::error::{Error, Result};
use crate::uniswap::bit_math::most_significant_bit;

/// Lowest tick a sqrt price can be computed for.
pub const MIN_TICK: i32 = -887_272;
/// Highest tick a sqrt price can be computed for.
pub const MAX_TICK: i32 = 887_272;

/// `getSqrtPriceAtTick(MIN_TICK)`.
pub const MIN_SQRT_PRICE: U160 = uint!(4295128739_U160);
/// `getSqrtPriceAtTick(MAX_TICK)`.
pub const MAX_SQRT_PRICE: U160 = uint!(1461446703485210103287273052203988822378723970342_U160);

/// Highest tick that is a multiple of `tick_spacing`.
///
/// # Errors
///
/// [`Error::DivisionByZero`] for a zero spacing.
pub fn max_usable_tick(tick_spacing: i32) -> Result<i32> {
    let steps = MAX_TICK.checked_div(tick_spacing).ok_or(Error::DivisionByZero)?;
    steps.checked_mul(tick_spacing).ok_or(Error::Overflow)
}

/// Lowest tick that is a multiple of `tick_spacing`.
///
/// # Errors
///
/// [`Error::DivisionByZero`] for a zero spacing.
pub fn min_usable_tick(tick_spacing: i32) -> Result<i32> {
    let steps = MIN_TICK.checked_div(tick_spacing).ok_or(Error::DivisionByZero)?;
    steps.checked_mul(tick_spacing).ok_or(Error::Overflow)
}

/// The Q128 factors √(1.0001)^(−2^i) for i = 1..19 (bit 0 seeds the product).
const FACTORS: [U256; 19] = [
    uint!(0xfff97272373d413259a46990580e213a_U256),
    uint!(0xfff2e50f5f656932ef12357cf3c7fdcc_U256),
    uint!(0xffe5caca7e10e4e61c3624eaa0941cd0_U256),
    uint!(0xffcb9843d60f6159c9db58835c926644_U256),
    uint!(0xff973b41fa98c081472e6896dfb254c0_U256),
    uint!(0xff2ea16466c96a3843ec78b326b52861_U256),
    uint!(0xfe5dee046a99a2a811c461f1969c3053_U256),
    uint!(0xfcbe86c7900a88aedcffc83b479aa3a4_U256),
    uint!(0xf987a7253ac413176f2b074cf7815e54_U256),
    uint!(0xf3392b0822b70005940c7a398e4b70f3_U256),
    uint!(0xe7159475a2c29b7443b29c7fa6e889d9_U256),
    uint!(0xd097f3bdfd2022b8845ad8f792aa5825_U256),
    uint!(0xa9f746462d870fdf8a65dc1f90e061e5_U256),
    uint!(0x70d869a156d2a1b890bb3df62baf32f7_U256),
    uint!(0x31be135f97d08fd981231505542fcfa6_U256),
    uint!(0x9aa508b5b7a84e1c677de54f3e99bc9_U256),
    uint!(0x5d6af8dedb81196699c329225ee604_U256),
    uint!(0x2216e584f5fa1ea926041bedfe98_U256),
    uint!(0x48a170391f7dc42444e8fa2_U256),
];

/// `getSqrtPriceAtTick`: 1.0001^(tick/2) as a Q64.96, rounded up.
///
/// # Errors
///
/// [`Error::InvalidTick`] when `|tick| > MAX_TICK`.
pub fn sqrt_price_at_tick(tick: i32) -> Result<U160> {
    let abs_tick = tick.unsigned_abs();
    if abs_tick > MAX_TICK.unsigned_abs() {
        return Err(Error::InvalidTick(tick));
    }

    let mut price = if abs_tick & 1 == 0 {
        U256::from(1_u8).wrapping_shl(128)
    } else {
        uint!(0xfffcb933bd6fad37aa2d162d1a594001_U256)
    };
    for (bit, factor) in FACTORS.iter().enumerate() {
        if abs_tick & (2_u32 << bit) != 0 {
            // Both factors are below 2^128 here, so the product fits (unchecked in the contract)
            price = price.wrapping_mul(*factor).wrapping_shr(128);
        }
    }
    if tick > 0 {
        price = U256::MAX.wrapping_div(price);
    }

    // Q128.128 to Q128.96, rounding up; the result is below 2^160 for every valid tick
    let sqrt_price = price.wrapping_add(U256::from(u32::MAX)).wrapping_shr(32);
    Ok(sqrt_price.wrapping_to())
}

/// `getTickAtSqrtPrice`: the greatest tick whose sqrt price is at most `sqrt_price_x96`.
///
/// # Errors
///
/// [`Error::InvalidSqrtPrice`] outside `[MIN_SQRT_PRICE, MAX_SQRT_PRICE)`.
pub fn tick_at_sqrt_price(sqrt_price_x96: U160) -> Result<i32> {
    if sqrt_price_x96 < MIN_SQRT_PRICE || sqrt_price_x96 >= MAX_SQRT_PRICE {
        return Err(Error::InvalidSqrtPrice(sqrt_price_x96));
    }
    let invalid = || Error::InvalidSqrtPrice(sqrt_price_x96);

    let price = U256::from(sqrt_price_x96).wrapping_shl(32);
    let msb = most_significant_bit(price).ok_or_else(invalid)?;
    let mut r = if msb >= 128 {
        price.wrapping_shr(usize::from(msb.wrapping_sub(127)))
    } else {
        price.wrapping_shl(usize::from(127_u8.wrapping_sub(msb)))
    };

    // log2 as a signed Q64.64: integer part from the msb, then 14 fractional bits by repeated squaring
    let integer = i128::from(msb).wrapping_sub(128).wrapping_shl(64);
    let mut log_2 = I256::try_from(integer).map_err(|_| invalid())?.into_raw();
    for bit in (50_usize..=63).rev() {
        r = r.wrapping_mul(r).wrapping_shr(127);
        let f = r.wrapping_shr(128);
        log_2 |= f.wrapping_shl(bit);
        if bit > 50 {
            r = r.wrapping_shr(usize::from(!f.is_zero()));
        }
    }

    // log_√1.0001(price) as a Q22.128, and the tick bounds around it
    let log_sqrt10001 = I256::from_raw(log_2).wrapping_mul(I256::from_raw(uint!(255738958999603826347141_U256)));
    let tick_low =
        to_tick(log_sqrt10001.wrapping_sub(I256::from_raw(uint!(3402992956809132418596140100660247210_U256))))
            .ok_or_else(invalid)?;
    let tick_high =
        to_tick(log_sqrt10001.wrapping_add(I256::from_raw(uint!(291339464771989622907027621153398088495_U256))))
            .ok_or_else(invalid)?;

    if tick_low == tick_high {
        return Ok(tick_low);
    }
    Ok(if sqrt_price_at_tick(tick_high)? <= sqrt_price_x96 {
        tick_high
    } else {
        tick_low
    })
}

/// `int24(x >> 128)` of a Q128 log value, which always fits for valid prices.
fn to_tick(value: I256) -> Option<i32> {
    i32::try_from(value.asr(128)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_match_the_constants() {
        assert_eq!(sqrt_price_at_tick(MIN_TICK), Ok(MIN_SQRT_PRICE));
        assert_eq!(sqrt_price_at_tick(MAX_TICK), Ok(MAX_SQRT_PRICE));
        assert_eq!(sqrt_price_at_tick(0), Ok(U160::from(1_u8) << 96));
        assert_eq!(sqrt_price_at_tick(MAX_TICK + 1), Err(Error::InvalidTick(MAX_TICK + 1)));
        assert_eq!(sqrt_price_at_tick(MIN_TICK - 1), Err(Error::InvalidTick(MIN_TICK - 1)));
    }

    #[test]
    fn tick_round_trips_at_tick_boundaries() {
        for tick in [MIN_TICK, -500_000, -1, 0, 1, 42, 500_000, MAX_TICK - 1] {
            let sqrt_price = sqrt_price_at_tick(tick).unwrap();
            assert_eq!(tick_at_sqrt_price(sqrt_price), Ok(tick));
        }
        assert_eq!(
            tick_at_sqrt_price(MAX_SQRT_PRICE),
            Err(Error::InvalidSqrtPrice(MAX_SQRT_PRICE))
        );
        let below = MIN_SQRT_PRICE - U160::from(1_u8);
        assert_eq!(tick_at_sqrt_price(below), Err(Error::InvalidSqrtPrice(below)));
    }

    #[test]
    fn usable_ticks_are_multiples_of_the_spacing() {
        assert_eq!(max_usable_tick(60), Ok(887_220));
        assert_eq!(min_usable_tick(60), Ok(-887_220));
        assert_eq!(max_usable_tick(0), Err(Error::DivisionByZero));
    }
}
