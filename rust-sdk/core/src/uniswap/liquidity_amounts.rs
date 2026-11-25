//! `LiquidityAmounts` (Senzu code on top of v4-core `SqrtPriceMath`): token amounts of a liquidity amount in a
//! range and the inverse.

use alloy_primitives::{U160, U256};

use crate::error::Result;
use crate::num::{to_u128, widen};
use crate::uniswap::full_math::mul_div;
use crate::uniswap::sqrt_price_math::{Q96, amount0_delta, amount1_delta};

/// Token amounts held by `liquidity` in `[√a, √b]` at `√P`, rounded down.
///
/// # Errors
///
/// [`Error::ZeroSqrtPrice`](crate::Error::ZeroSqrtPrice) when the range's lower price is zero.
pub fn amounts_for_liquidity(
    sqrt_price_x96: U160,
    sqrt_price_a_x96: U160,
    sqrt_price_b_x96: U160,
    liquidity: u128,
) -> Result<(U256, U256)> {
    let (lower, upper) = sorted(sqrt_price_a_x96, sqrt_price_b_x96);
    if sqrt_price_x96 <= lower {
        Ok((amount0_delta(lower, upper, liquidity, false)?, U256::ZERO))
    } else if sqrt_price_x96 < upper {
        Ok((
            amount0_delta(sqrt_price_x96, upper, liquidity, false)?,
            amount1_delta(lower, sqrt_price_x96, liquidity, false)?,
        ))
    } else {
        Ok((U256::ZERO, amount1_delta(lower, upper, liquidity, false)?))
    }
}

/// Most liquidity `amount0` / `amount1` can add in `[√a, √b]` at `√P`, rounded down.
///
/// # Errors
///
/// [`Error::MulDiv`](crate::Error::MulDiv) for an empty range (`√a == √b`) and
/// [`Error::CastOverflow`](crate::Error::CastOverflow) when the liquidity does not fit in 128 bits.
pub fn liquidity_for_amounts(
    sqrt_price_x96: U160,
    sqrt_price_a_x96: U160,
    sqrt_price_b_x96: U160,
    amount0: U256,
    amount1: U256,
) -> Result<u128> {
    let (lower, upper) = sorted(sqrt_price_a_x96, sqrt_price_b_x96);
    if sqrt_price_x96 <= lower {
        liquidity0(lower, upper, amount0)
    } else if sqrt_price_x96 < upper {
        Ok(liquidity0(sqrt_price_x96, upper, amount0)?.min(liquidity1(lower, sqrt_price_x96, amount1)?))
    } else {
        liquidity1(lower, upper, amount1)
    }
}

fn liquidity0(lower: U160, upper: U160, amount0: U256) -> Result<u128> {
    let (lower, upper) = (widen(lower), widen(upper));
    let intermediate = mul_div(lower, upper, Q96)?;
    to_u128(mul_div(amount0, intermediate, upper.wrapping_sub(lower))?)
}

fn liquidity1(lower: U160, upper: U160, amount1: U256) -> Result<u128> {
    to_u128(mul_div(amount1, Q96, widen(upper).wrapping_sub(widen(lower)))?)
}

fn sorted(a: U160, b: U160) -> (U160, U160) {
    if a > b { (b, a) } else { (a, b) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uniswap::tick_math::sqrt_price_at_tick;

    #[test]
    fn amounts_depend_on_the_side_of_the_range() {
        let (lower, upper) = (sqrt_price_at_tick(-600).unwrap(), sqrt_price_at_tick(600).unwrap());
        let below = amounts_for_liquidity(
            sqrt_price_at_tick(-1_000).unwrap(),
            lower,
            upper,
            1_000_000_000_000_000_000,
        )
        .unwrap();
        let inside =
            amounts_for_liquidity(sqrt_price_at_tick(0).unwrap(), lower, upper, 1_000_000_000_000_000_000).unwrap();
        let above = amounts_for_liquidity(
            sqrt_price_at_tick(1_000).unwrap(),
            lower,
            upper,
            1_000_000_000_000_000_000,
        )
        .unwrap();
        assert!(below.0 > U256::ZERO && below.1.is_zero());
        assert!(inside.0 > U256::ZERO && inside.1 > U256::ZERO);
        assert!(above.0.is_zero() && above.1 > U256::ZERO);
    }

    #[test]
    fn liquidity_for_amounts_inverts_amounts_for_liquidity() {
        let (price, lower, upper) = (
            sqrt_price_at_tick(10).unwrap(),
            sqrt_price_at_tick(-600).unwrap(),
            sqrt_price_at_tick(600).unwrap(),
        );
        let liquidity = 123_456_789_000_000_u128;
        let (amount0, amount1) = amounts_for_liquidity(price, lower, upper, liquidity).unwrap();
        let back = liquidity_for_amounts(price, lower, upper, amount0, amount1).unwrap();
        assert!(back <= liquidity && liquidity - back < 1_000);
    }
}
