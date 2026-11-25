//! Borrow and supply rates as APRs, and debt grown since a read.
//!
//! The adaptive curve is linear in the rate at target, so applying [`irm::curve`] to a yearly rate at target gives the
//! yearly borrow rate exactly as the contract's per-second one would, scaled.

use alloy_primitives::{U256, uint};

use crate::error::Result;
use crate::irm::{self, SECONDS_PER_YEAR};
use crate::num::{Checked, WAD};
use crate::uniswap::mul_div;

/// `1e36`: [`irm::exp_minus_one`]'s scale.
pub(crate) const WAD_SQUARED: U256 = uint!(1_000_000_000_000_000_000_000_000_000_000_000_000_U256);

/// Borrows over supply (cash plus borrows), 1e18 = 100%, rounded down; zero for an empty market
/// (`MarketLogic.utilization`).
///
/// # Errors
///
/// [`crate::Error::Overflow`] on absurd totals.
pub fn utilization(cash: U256, borrows: U256) -> Result<U256> {
    let supply = cash.try_add(borrows)?;
    if supply.is_zero() {
        return Ok(U256::ZERO);
    }
    mul_div(borrows, WAD, supply)
}

/// A market's yearly rates (1e18 = 100%).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct MarketAprs {
    /// The utilization they are at.
    pub utilization_wad: U256,
    /// What borrowers pay.
    pub borrow_apr_wad: U256,
    /// What lenders earn: the borrow rate on the borrowed part, less the reserve factor.
    pub supply_apr_wad: U256,
}

/// A market's rates at `utilization_wad`: the adaptive curve around its yearly rate at target, and the supply rate
/// that leaves (`MarketLogic.borrowRate`, `supplyRate`).
///
/// # Errors
///
/// [`crate::Error::Underflow`] for a reserve factor above 100%, and the curve's errors (never for a real market).
pub fn market_aprs(
    rate_at_target_apr_wad: U256,
    reserve_factor_wad: U256,
    utilization_wad: U256,
) -> Result<MarketAprs> {
    let borrow_apr_wad = irm::curve(rate_at_target_apr_wad, irm::utilization_error(utilization_wad)?)?;
    let lent = mul_div(borrow_apr_wad, utilization_wad, WAD)?;
    let supply_apr_wad = mul_div(lent, WAD.try_sub(reserve_factor_wad)?, WAD)?;
    Ok(MarketAprs {
        utilization_wad,
        borrow_apr_wad,
        supply_apr_wad,
    })
}

/// A market's rates after its cash moves by `cash_delta` (a supply adds, a withdrawal or a borrow takes; the cash
/// never goes under zero).
///
/// # Errors
///
/// As [`market_aprs`].
pub fn market_aprs_after(
    cash: U256,
    borrows: U256,
    rate_at_target_apr_wad: U256,
    reserve_factor_wad: U256,
    cash_added: U256,
    cash_taken: U256,
) -> Result<MarketAprs> {
    let after = cash.try_add(cash_added)?.saturating_sub(cash_taken);
    market_aprs(rate_at_target_apr_wad, reserve_factor_wad, utilization(after, borrows)?)
}

/// How a debt's rate is known: its market's curve inputs, or failing those its yearly rate at the read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct DebtRate {
    /// The debt's yearly rate at the read: used without a market.
    pub borrow_apr_wad: U256,
    /// The market's yearly rate at target (the lens's `rateAtTargetAprWad`); with `utilization_wad`, the rate adapts
    /// over the period as the contract's does.
    pub rate_at_target_apr_wad: Option<U256>,
    /// The market's utilization.
    pub utilization_wad: U256,
}

/// How much a debt grows over `elapsed` seconds, `e^∫rate − 1` in 1e36: the market's adaptive rate integrated as
/// `MarketLogic.accrue` does (the exponent capped at [`irm::MAX_GROWTH_EXPONENT`]), or the simple yearly rate without
/// a market.
///
/// # Errors
///
/// The rate math's errors (never for a real market).
pub fn debt_growth(rate: &DebtRate, elapsed: u64) -> Result<U256> {
    let elapsed = U256::from(elapsed);
    let integral = match rate.rate_at_target_apr_wad {
        Some(apr) => {
            let per_second = apr.try_div(SECONDS_PER_YEAR)?;
            irm::adaptive_rate(per_second, rate.utilization_wad, elapsed)?.integral
        }
        None => rate.borrow_apr_wad.try_mul(elapsed)?.try_div(SECONDS_PER_YEAR)?,
    };
    irm::to_uint(irm::exp_minus_one(irm::to_int(
        integral.min(irm::MAX_GROWTH_EXPONENT),
    )?)?)
}

/// `amount` grown by [`debt_growth`]'s `growth`, rounded down.
///
/// # Errors
///
/// [`crate::Error::MulDiv`] on overflow.
pub fn grown(amount: U256, growth: U256) -> Result<U256> {
    amount.try_add(mul_div(amount, growth, WAD_SQUARED)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pct(x: u64) -> U256 {
        U256::from(x).try_mul(WAD).unwrap().try_div(U256::from(100_u8)).unwrap()
    }

    #[test]
    fn wad_squared_is_1e36() {
        assert_eq!(WAD_SQUARED, WAD.try_mul(WAD).unwrap());
    }

    #[test]
    fn the_curve_is_the_rate_at_target_at_target() {
        let at = market_aprs(pct(4), pct(10), irm::TARGET_UTILIZATION).unwrap();
        assert_eq!(at.borrow_apr_wad, pct(4));
        // 4% on 90% lent, less 10%: 3.24%
        assert_eq!(at.supply_apr_wad, U256::from(32_400_000_000_000_000_u64));
        assert_eq!(market_aprs(pct(4), pct(10), U256::ZERO).unwrap().borrow_apr_wad, pct(1));
        assert_eq!(market_aprs(pct(4), pct(10), WAD).unwrap().borrow_apr_wad, pct(16));
    }

    #[test]
    fn utilization_of_an_empty_market_is_zero() {
        assert_eq!(utilization(U256::ZERO, U256::ZERO).unwrap(), U256::ZERO);
        assert_eq!(utilization(U256::from(1_u8), U256::from(3_u8)).unwrap(), pct(75));
        let after = market_aprs_after(
            U256::from(1_u8),
            U256::from(3_u8),
            pct(4),
            U256::ZERO,
            U256::ZERO,
            U256::from(9_u8),
        );
        assert_eq!(after.unwrap().utilization_wad, WAD);
    }

    #[test]
    fn a_year_at_ten_percent_grows_by_e_to_the_tenth() {
        let rate = DebtRate {
            borrow_apr_wad: pct(10),
            rate_at_target_apr_wad: None,
            utilization_wad: U256::ZERO,
        };
        let growth = debt_growth(&rate, 31_536_000).unwrap();
        // e^0.1 - 1 = 0.10517091807564762...
        assert_eq!(
            grown(WAD, growth).unwrap() / U256::from(1_000_000_000_u64),
            U256::from(1_105_170_918_u64)
        );
    }
}
