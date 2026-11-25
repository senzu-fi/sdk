//! `BitMath` from Uniswap v4-core, and the Newton square root of `HealthMath`.

use alloy_primitives::U256;

use crate::error::Result;
use crate::num::Checked;

/// Index of the most significant set bit, `None` for zero (where the contract reverts).
#[inline]
#[must_use]
pub fn most_significant_bit(x: U256) -> Option<u8> {
    x.bit_len().checked_sub(1).and_then(|msb| u8::try_from(msb).ok())
}

/// `HealthMath.sqrt`: `floor(√x)`.
///
/// # Errors
///
/// None in practice; the checked additions mirror the contract's and cannot overflow.
pub fn sqrt(x: U256) -> Result<U256> {
    let Some(msb) = most_significant_bit(x) else {
        return Ok(U256::ZERO);
    };
    let mut z = U256::from(1_u8).wrapping_shl(usize::from(msb >> 1).wrapping_add(1));
    let mut y = z.try_add(x.try_div(z)?)?.wrapping_shr(1);
    while y < z {
        z = y;
        y = z.try_add(x.try_div(z)?)?.wrapping_shr(1);
    }
    Ok(z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msb_matches_bit_positions() {
        assert_eq!(most_significant_bit(U256::ZERO), None);
        assert_eq!(most_significant_bit(U256::from(1)), Some(0));
        assert_eq!(most_significant_bit(U256::MAX), Some(255));
    }

    #[test]
    fn sqrt_is_the_floor_root() {
        for x in 0_u64..2_000 {
            let root = sqrt(U256::from(x)).unwrap();
            assert!(root * root <= U256::from(x) && (root + U256::from(1)).pow(U256::from(2)) > U256::from(x));
        }
        assert_eq!(sqrt(U256::MAX).unwrap(), U256::from(u128::MAX));
    }
}
