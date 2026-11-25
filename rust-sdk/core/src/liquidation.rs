//! `LiquidationLogic`: how much a liquidator repays, how many units it takes, and the protocol fee.
//!
//! The penalty shrinks to `collateral / debt` when the position cannot bear it, never below
//! [`MIN_INCENTIVE_WAD`]. Seized units round down; a repayment worth, with the penalty, at least the collateral
//! takes everything and the repay shrinks to `collateral / MIN_INCENTIVE_WAD`. The protocol takes `fee_wad`
//! (0–20%, rounded down) of what was actually seized minus what was repaid, in the repaid asset; zero when
//! collateral is under 1.02x the debt or when the liquidation writes debt off.

use alloy_primitives::U256;

use crate::error::{Error, Result};
use crate::num::{Checked, WAD, pow10};
use crate::types::HealthResult;
use crate::uniswap::{mul_div, mul_div_rounding_up};

/// The smallest liquidation incentive (`MIN_INCENTIVE`, 1.02e18): the floor a full seize shrinks the bonus to.
pub const MIN_INCENTIVE_WAD: U256 = U256::from_limbs([1_020_000_000_000_000_000, 0, 0, 0]);

/// Outcome of one liquidation (what `LiquidationLogic.liquidate` returns).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct Liquidation {
    /// Assets the liquidator repays toward the debt.
    pub repay_assets: U256,
    /// Units of the position moved to the liquidator.
    pub seized_units: U256,
    /// Protocol liquidation fee in the repaid asset (0 on write-off or when the fee setting is 0).
    pub fee_assets: U256,
}

/// The debt being repaid in a liquidation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct RepaidDebt {
    /// The position's debt in the asset, interest accrued (`debtOf`).
    pub debt_assets: U256,
    /// Oracle price of the asset, USD per whole token (1e18).
    pub price_wad: U256,
    /// Decimals of the asset.
    pub decimals: u8,
}

/// Liquidation of one debt (`LiquidationLogic.liquidate`).
///
/// `health` is the health check of the borrower's `units` at the position's penalty
/// ([`health::position_penalty`](crate::health::position_penalty) of its pair and every debt outside it);
/// `penalty_wad` is the repaid debt's: the pair's, or the asset's own when higher (`position_penalty` of the pair
/// and that asset alone, if outside the pair). `max_repay_assets` is the most the liquidator wants to repay
/// (`U256::MAX` = as much as allowed). `fee_wad` is the DEX's `liquidationFeeWad` (1e18 = 100%, max 0.2e18);
/// `has_other_debt` is true when the position also owes another market.
///
/// # Errors
///
/// [`Error::NotLiquidatable`] when every corner is covered, [`Error::ZeroAmount`] with nothing to seize or
/// nothing to repay, [`Error::NoDebt`] without debt in the asset, and the arithmetic errors of the contract.
pub fn liquidate(
    health: &HealthResult,
    debt: &RepaidDebt,
    units: U256,
    max_repay_assets: U256,
    penalty_wad: U256,
    fee_wad: U256,
    has_other_debt: bool,
) -> Result<Liquidation> {
    if health.passes() {
        return Err(Error::NotLiquidatable);
    }
    seize_for(
        health,
        debt,
        max_repay_assets,
        units,
        penalty_wad,
        fee_wad,
        has_other_debt,
    )
}

/// Pure seize math (`LiquidationLogic.seizeFor`), after the health check.
///
/// # Errors
///
/// [`Error::ZeroAmount`] with zero collateral or zero repay, [`Error::NoDebt`] with zero debt, and the arithmetic
/// errors of the contract.
pub fn seize_for(
    health: &HealthResult,
    debt: &RepaidDebt,
    max_repay_assets: U256,
    units: U256,
    penalty_wad: U256,
    fee_wad: U256,
    has_other_debt: bool,
) -> Result<Liquidation> {
    if health.collateral_usd_wad.is_zero() {
        return Err(Error::ZeroAmount);
    }
    if debt.debt_assets.is_zero() {
        return Err(Error::NoDebt);
    }

    let repay_assets = max_repay_assets.min(debt.debt_assets);
    if repay_assets.is_zero() {
        return Err(Error::ZeroAmount);
    }

    let unit = pow10(debt.decimals)?;
    let penalty = bearable_penalty(health.collateral_usd_wad, health.debt_usd_wad, penalty_wad)?;
    let repay_usd = mul_div(repay_assets, debt.price_wad, unit)?;
    let seize_usd = mul_div(repay_usd, penalty, WAD)?;
    let mut repay = repay_assets;
    let mut fee = U256::ZERO;

    let seized = if seize_usd >= health.collateral_usd_wad {
        let coverable = mul_div_rounding_up(
            mul_div_rounding_up(health.collateral_usd_wad, WAD, MIN_INCENTIVE_WAD)?,
            unit,
            debt.price_wad,
        )?;
        if coverable < repay {
            repay = coverable;
        }
        units
    } else {
        mul_div(units, seize_usd, health.collateral_usd_wad)?
    };

    if !fee_wad.is_zero() && !seized.is_zero() {
        let bad_debt = mul_div(health.collateral_usd_wad, WAD, MIN_INCENTIVE_WAD)? < health.debt_usd_wad
            || (seized == units && (repay < debt.debt_assets || has_other_debt));
        if !bad_debt {
            let seized_assets = mul_div(mul_div(health.collateral_usd_wad, seized, units)?, unit, debt.price_wad)?;
            if seized_assets > repay {
                fee = mul_div(seized_assets.try_sub(repay)?, fee_wad, WAD)?;
            }
        }
    }

    Ok(Liquidation {
        repay_assets: repay,
        seized_units: seized,
        fee_assets: fee,
    })
}

/// `LiquidationLogic._bearable`: the penalty the collateral can bear, capped by `collateral / debt` and floored at
/// [`MIN_INCENTIVE_WAD`].
pub(crate) fn bearable_penalty(collateral_usd_wad: U256, debt_usd_wad: U256, penalty_wad: U256) -> Result<U256> {
    if debt_usd_wad.is_zero() {
        return Ok(penalty_wad);
    }
    let ratio = mul_div(collateral_usd_wad, WAD, debt_usd_wad)?;
    if ratio >= penalty_wad {
        return Ok(penalty_wad);
    }
    Ok(ratio.max(MIN_INCENTIVE_WAD))
}
