//! `SqrtPriceMath` from Uniswap v4-core: token deltas between two prices and the price after a swap step.

use alloy_primitives::{U160, U256, U512};

use crate::error::{Error, Result};
use crate::num::{Checked, to_u160, widen};
use crate::uniswap::full_math::{div_rounding_up, mul_div, mul_div_rounding_up};

const RESOLUTION: usize = 96;

/// 2^96, the Q64.96 unit.
pub const Q96: U256 = U256::from_limbs([0, 1 << 32, 0, 0]);

/// Price after adding (`add`) or removing `amount` of token0, rounded up.
///
/// # Errors
///
/// [`Error::PriceOverflow`] when removing more token0 than the liquidity holds, and the arithmetic errors of the
/// contract.
pub fn next_sqrt_price_from_amount0_rounding_up(
    sqrt_price_x96: U160,
    liquidity: u128,
    amount: U256,
    add: bool,
) -> Result<U160> {
    if amount.is_zero() {
        return Ok(sqrt_price_x96);
    }
    let sqrt_price = widen(sqrt_price_x96);
    let numerator1 = U256::from(liquidity).wrapping_shl(RESOLUTION);
    let product = amount.wrapping_mul(sqrt_price);
    let product_fits = product.wrapping_div(amount) == sqrt_price;

    if add {
        if product_fits {
            let denominator = numerator1.wrapping_add(product);
            if denominator >= numerator1 {
                // At most sqrt_price, so the uint160 truncation keeps every bit
                return Ok(mul_div_rounding_up(numerator1, sqrt_price, denominator)?.wrapping_to());
            }
        }
        let denominator = numerator1.try_div(sqrt_price)?.try_add(amount)?;
        return Ok(div_rounding_up(numerator1, denominator).wrapping_to());
    }

    if !(product_fits && numerator1 > product) {
        return Err(Error::PriceOverflow);
    }
    to_u160(mul_div_rounding_up(
        numerator1,
        sqrt_price,
        numerator1.wrapping_sub(product),
    )?)
}

/// Price after adding (`add`) or removing `amount` of token1, rounded down.
///
/// # Errors
///
/// [`Error::NotEnoughLiquidity`] when removing more token1 than the liquidity holds, and the arithmetic errors of
/// the contract.
pub fn next_sqrt_price_from_amount1_rounding_down(
    sqrt_price_x96: U160,
    liquidity: u128,
    amount: U256,
    add: bool,
) -> Result<U160> {
    let sqrt_price = widen(sqrt_price_x96);
    let liquidity = U256::from(liquidity);
    let fits_shift = amount <= widen(U160::MAX);

    if add {
        let quotient = if fits_shift {
            amount.wrapping_shl(RESOLUTION).try_div(liquidity)?
        } else {
            mul_div(amount, Q96, liquidity)?
        };
        return to_u160(sqrt_price.try_add(quotient)?);
    }

    let quotient = if fits_shift {
        div_rounding_up(amount.wrapping_shl(RESOLUTION), liquidity)
    } else {
        mul_div_rounding_up(amount, Q96, liquidity)?
    };
    if sqrt_price <= quotient {
        return Err(Error::NotEnoughLiquidity);
    }
    Ok(sqrt_price.wrapping_sub(quotient).wrapping_to())
}

/// Price after swapping `amount_in` into the pool (`zero_for_one`: token0 in).
///
/// # Errors
///
/// [`Error::InvalidPriceOrLiquidity`] for a zero price or liquidity, and the errors of the step functions.
pub fn next_sqrt_price_from_input(
    sqrt_price_x96: U160,
    liquidity: u128,
    amount_in: U256,
    zero_for_one: bool,
) -> Result<U160> {
    if sqrt_price_x96.is_zero() || liquidity == 0 {
        return Err(Error::InvalidPriceOrLiquidity);
    }
    if zero_for_one {
        next_sqrt_price_from_amount0_rounding_up(sqrt_price_x96, liquidity, amount_in, true)
    } else {
        next_sqrt_price_from_amount1_rounding_down(sqrt_price_x96, liquidity, amount_in, true)
    }
}

/// Price after taking `amount_out` out of the pool (`zero_for_one`: token1 out).
///
/// # Errors
///
/// [`Error::InvalidPriceOrLiquidity`] for a zero price or liquidity, and the errors of the step functions.
pub fn next_sqrt_price_from_output(
    sqrt_price_x96: U160,
    liquidity: u128,
    amount_out: U256,
    zero_for_one: bool,
) -> Result<U160> {
    if sqrt_price_x96.is_zero() || liquidity == 0 {
        return Err(Error::InvalidPriceOrLiquidity);
    }
    if zero_for_one {
        next_sqrt_price_from_amount1_rounding_down(sqrt_price_x96, liquidity, amount_out, false)
    } else {
        next_sqrt_price_from_amount0_rounding_up(sqrt_price_x96, liquidity, amount_out, false)
    }
}

/// Token0 between two prices: `L · 2^96 · (√b − √a) / (√a · √b)`.
///
/// # Errors
///
/// [`Error::ZeroSqrtPrice`] when the lower price is zero.
pub fn amount0_delta(sqrt_price_a_x96: U160, sqrt_price_b_x96: U160, liquidity: u128, round_up: bool) -> Result<U256> {
    let (lower, upper) = ordered(sqrt_price_a_x96, sqrt_price_b_x96);
    if lower.is_zero() {
        return Err(Error::ZeroSqrtPrice);
    }
    let numerator1 = U256::from(liquidity).wrapping_shl(RESOLUTION);
    let numerator2 = upper.wrapping_sub(lower);
    if round_up {
        Ok(div_rounding_up(
            mul_div_rounding_up(numerator1, numerator2, upper)?,
            lower,
        ))
    } else {
        Ok(mul_div(numerator1, numerator2, upper)?.wrapping_div(lower))
    }
}

/// Token1 between two prices: `L · |√b − √a| / 2^96`.
///
/// # Errors
///
/// None in practice (the product always fits); the signature matches the other deltas.
pub fn amount1_delta(sqrt_price_a_x96: U160, sqrt_price_b_x96: U160, liquidity: u128, round_up: bool) -> Result<U256> {
    let (lower, upper) = ordered(sqrt_price_a_x96, sqrt_price_b_x96);
    let difference = upper.wrapping_sub(lower);
    let product: U512 = U256::from(liquidity).widening_mul(difference);
    // Product is under 2^288, so shifting right by 96 fits in 192 bits (< U256::MAX).
    let [p0, p1, p2, p3, p4, _, _, _] = *product.as_limbs();
    let amount = U256::from_limbs([
        (p1 >> 32) | (p2 << 32),
        (p2 >> 32) | (p3 << 32),
        (p3 >> 32) | (p4 << 32),
        p4 >> 32,
    ]);
    let has_remainder = p0 != 0 || (p1 & 0xffff_ffff) != 0;
    Ok(if round_up && has_remainder {
        amount.wrapping_add(U256::from(1_u8))
    } else {
        amount
    })
}

fn ordered(a: U160, b: U160) -> (U256, U256) {
    let (a, b) = (widen(a), widen(b));
    if a > b { (b, a) } else { (a, b) }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: U160 = U160::from_limbs([0, 1 << 32, 0]);

    #[test]
    fn deltas_of_equal_prices_are_zero() {
        assert_eq!(amount0_delta(ONE, ONE, 1_000, true), Ok(U256::ZERO));
        assert_eq!(amount1_delta(ONE, ONE, 1_000, true), Ok(U256::ZERO));
    }

    #[test]
    fn amount0_delta_rejects_a_zero_price() {
        assert_eq!(amount0_delta(U160::ZERO, ONE, 1, false), Err(Error::ZeroSqrtPrice));
    }

    #[test]
    fn swap_steps_reject_zero_liquidity() {
        assert_eq!(
            next_sqrt_price_from_input(ONE, 0, U256::from(1), true),
            Err(Error::InvalidPriceOrLiquidity)
        );
        assert_eq!(
            next_sqrt_price_from_output(U160::ZERO, 1, U256::from(1), true),
            Err(Error::InvalidPriceOrLiquidity)
        );
    }

    #[test]
    fn output_larger_than_reserves_fails() {
        let liquidity = 1_000_000_u128;
        assert_eq!(
            next_sqrt_price_from_output(ONE, liquidity, U256::from(liquidity), true),
            Err(Error::NotEnoughLiquidity)
        );
        assert_eq!(
            next_sqrt_price_from_output(ONE, liquidity, U256::from(liquidity), false),
            Err(Error::PriceOverflow)
        );
    }
}
