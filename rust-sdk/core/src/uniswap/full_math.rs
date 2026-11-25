//! `FullMath` from Uniswap v4-core: `a · b / denominator` with a 512-bit intermediate product.

use alloy_primitives::{U256, U512};

use crate::error::{Error, Result};

/// `floor(a · b / denominator)` without overflowing in the product.
///
/// # Errors
///
/// [`Error::MulDiv`] when `denominator` is zero or the result does not fit in 256 bits (the contract's
/// `require(denominator > prod1)`).
#[inline]
pub fn mul_div(a: U256, b: U256, denominator: U256) -> Result<U256> {
    if denominator.is_zero() {
        return Err(Error::MulDiv);
    }
    if a.is_zero() || b.is_zero() {
        return Ok(U256::ZERO);
    }
    let product: U512 = a.widening_mul(b);
    let [l0, l1, l2, l3, h0, h1, h2, h3] = *product.as_limbs();
    let prod1 = U256::from_limbs([h0, h1, h2, h3]);
    if prod1.is_zero() {
        return Ok(U256::from_limbs([l0, l1, l2, l3]).wrapping_div(denominator));
    }
    if denominator <= prod1 {
        return Err(Error::MulDiv);
    }
    let quotient = product.wrapping_div(U512::from(denominator));
    let [q0, q1, q2, q3, ..] = *quotient.as_limbs();
    Ok(U256::from_limbs([q0, q1, q2, q3]))
}

/// `ceil(a · b / denominator)`.
///
/// # Errors
///
/// As [`mul_div`], and [`Error::MulDiv`] when rounding up passes `U256::MAX`.
#[inline]
pub fn mul_div_rounding_up(a: U256, b: U256, denominator: U256) -> Result<U256> {
    if denominator.is_zero() {
        return Err(Error::MulDiv);
    }
    if a.is_zero() || b.is_zero() {
        return Ok(U256::ZERO);
    }
    let product: U512 = a.widening_mul(b);
    let [l0, l1, l2, l3, h0, h1, h2, h3] = *product.as_limbs();
    let prod1 = U256::from_limbs([h0, h1, h2, h3]);
    let (result, has_remainder) = if prod1.is_zero() {
        let (q, r) = U256::from_limbs([l0, l1, l2, l3]).div_rem(denominator);
        (q, !r.is_zero())
    } else {
        if denominator <= prod1 {
            return Err(Error::MulDiv);
        }
        let (q, r) = product.div_rem(U512::from(denominator));
        let [q0, q1, q2, q3, ..] = *q.as_limbs();
        (U256::from_limbs([q0, q1, q2, q3]), !r.is_zero())
    };
    if has_remainder {
        result.checked_add(U256::from(1_u8)).ok_or(Error::MulDiv)
    } else {
        Ok(result)
    }
}

/// `UnsafeMath.divRoundingUp`: `ceil(x / y)`, and 0 for `y == 0` (EVM division semantics).
#[inline]
#[must_use]
pub fn div_rounding_up(x: U256, y: U256) -> U256 {
    if y.is_zero() {
        return U256::ZERO;
    }
    let (quotient, remainder) = x.div_rem(y);
    if remainder.is_zero() {
        quotient
    } else {
        quotient.wrapping_add(U256::from(1_u8))
    }
}

/// `floor(a · b / 2^96)` without overflowing the intermediate product, bit-exact with `mul_div(a, b, Q96)`.
#[inline]
pub(crate) fn mul_shr96(a: U256, b: U256) -> Result<U256> {
    if a.is_zero() || b.is_zero() {
        return Ok(U256::ZERO);
    }
    let product: U512 = a.widening_mul(b);
    let [_, p1, p2, p3, p4, p5, p6, p7] = *product.as_limbs();
    if (p5 >> 32) != 0 || p6 != 0 || p7 != 0 {
        return Err(Error::MulDiv);
    }
    Ok(U256::from_limbs([
        (p1 >> 32) | (p2 << 32),
        (p2 >> 32) | (p3 << 32),
        (p3 >> 32) | (p4 << 32),
        (p4 >> 32) | (p5 << 32),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mul_div_handles_a_512_bit_product() {
        let max = U256::MAX;
        assert_eq!(mul_div(max, max, max), Ok(max));
        assert_eq!(mul_div(max, U256::from(2), U256::from(4)), Ok(max >> 1));
    }

    #[test]
    fn mul_div_rejects_zero_denominator_and_overflow() {
        assert_eq!(mul_div(U256::from(1), U256::from(1), U256::ZERO), Err(Error::MulDiv));
        assert_eq!(mul_div(U256::MAX, U256::from(2), U256::from(1)), Err(Error::MulDiv));
    }

    #[test]
    fn rounding_up_adds_one_only_with_a_remainder() {
        assert_eq!(
            mul_div_rounding_up(U256::from(10), U256::from(3), U256::from(5)),
            Ok(U256::from(6))
        );
        assert_eq!(
            mul_div_rounding_up(U256::from(10), U256::from(3), U256::from(4)),
            Ok(U256::from(8))
        );
        assert_eq!(
            mul_div_rounding_up(U256::MAX, U256::MAX, U256::MAX - U256::from(1)),
            Err(Error::MulDiv)
        );
    }

    #[test]
    fn div_rounding_up_follows_evm_division_by_zero() {
        assert_eq!(div_rounding_up(U256::from(7), U256::ZERO), U256::ZERO);
        assert_eq!(div_rounding_up(U256::from(7), U256::from(2)), U256::from(4));
    }
}
