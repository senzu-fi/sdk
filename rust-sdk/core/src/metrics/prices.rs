//! USD prices a pool implies.

use alloy_primitives::{U160, U256, U512};

use crate::error::Result;
use crate::num::{Checked, pow10};
use crate::types::Side;

/// The USD price (1e18) the pool implies for the token that is not `known`, from `known`'s USD price: token0's is
/// token1's times the pool price (token1 per token0), token1's is token0's over it. Rounded down; zero at a zero pool
/// price; `U256::MAX` past 256 bits (only outside Uniswap's price range), so a display never breaks.
///
/// # Errors
///
/// [`crate::Error::Overflow`] for 78 or more decimals.
pub fn pool_implied_price(
    pool_sqrt_price_x96: U160,
    known: Side,
    known_price_wad: U256,
    decimals0: u8,
    decimals1: u8,
) -> Result<U256> {
    let sqrt = U512::from(U256::from(pool_sqrt_price_x96));
    let square = sqrt.try_mul(sqrt)?;
    if square.is_zero() {
        return Ok(U256::ZERO);
    }
    let q192 = U512::from(1_u8).wrapping_shl(192);
    let (d0, d1) = (U512::from(pow10(decimals0)?), U512::from(pow10(decimals1)?));
    let known_price = U512::from(known_price_wad);
    let price = match known {
        Side::Token1 => known_price.try_mul(square)?.try_mul(d0)?.try_div(q192.try_mul(d1)?)?,
        Side::Token0 => known_price.try_mul(q192)?.try_mul(d1)?.try_div(square.try_mul(d0)?)?,
    };
    Ok(U256::checked_from_limbs_slice(price.as_limbs()).unwrap_or(U256::MAX))
}

/// Both tokens' USD prices (1e18) at a pool price: `known` at `known_price_wad`, the other as the pool implies.
///
/// # Errors
///
/// As [`pool_implied_price`].
pub fn pool_prices(
    pool_sqrt_price_x96: U160,
    known: Side,
    known_price_wad: U256,
    decimals0: u8,
    decimals1: u8,
) -> Result<(U256, U256)> {
    let other = pool_implied_price(pool_sqrt_price_x96, known, known_price_wad, decimals0, decimals1)?;
    Ok(match known {
        Side::Token0 => (known_price_wad, other),
        Side::Token1 => (other, known_price_wad),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::num::WAD;
    use crate::uniswap::Q96;

    #[test]
    fn a_pool_at_one_prices_both_tokens_alike() {
        let one = U160::from(Q96);
        assert_eq!(pool_implied_price(one, Side::Token1, WAD, 18, 18).unwrap(), WAD);
        assert_eq!(pool_implied_price(one, Side::Token0, WAD, 18, 18).unwrap(), WAD);
        assert_eq!(
            pool_implied_price(U160::ZERO, Side::Token0, WAD, 18, 18).unwrap(),
            U256::ZERO
        );
    }

    #[test]
    fn decimals_scale_the_pool_price() {
        // 1 token0 (18 decimals) = 2,000 token1 (6 decimals): raw price 2000e6 / 1e18 = 2e-9, √ = 4.47e-5
        let sqrt = U160::from(3_543_191_142_285_914_205_922_034_u128);
        let usd0 = pool_implied_price(sqrt, Side::Token1, WAD, 18, 6).unwrap();
        assert!(
            usd0 / WAD == U256::from(1_999_u64) || usd0 / WAD == U256::from(2_000_u64),
            "{usd0}"
        );
        let (p0, p1) = pool_prices(sqrt, Side::Token0, usd0, 18, 6).unwrap();
        assert_eq!(p0, usd0);
        assert!(p1.abs_diff(WAD) < U256::from(1_000_000_u64), "{p1}");
    }
}
