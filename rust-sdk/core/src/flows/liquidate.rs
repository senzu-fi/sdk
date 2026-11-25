//! Liquidating a position: which debt to repay, the share of the liquidity it seizes, the redeem's minimums and the
//! sale of the other token into the debt asset.

use alloy_primitives::{U160, U256, U512};

use super::PerToken;
use super::minus_bps;
use crate::error::{Error, Result};
use crate::liquidation;
use crate::num::{Checked, WAD, to_u128};
use crate::slippage;
use crate::types::Side;
use crate::uniswap::{amounts_for_liquidity, mul_div, mul_div_rounding_up};

/// The contract's floor on the liquidation penalty (`MIN_INCENTIVE`): 1.02.
pub const MIN_INCENTIVE_WAD: U256 = liquidation::MIN_INCENTIVE_WAD;
/// Room for rounding and an oracle update between the read and the transaction: the seized share is taken this much
/// lower, so the minimums it sets hold. 0.5%.
pub const SEIZE_MARGIN_BPS: u64 = 50;

/// One debt of a position, as a liquidator ranks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct RankedDebt {
    /// Amount owed.
    pub assets: U256,
    /// Its value in USD (1e18).
    pub value_usd_wad: U256,
}

/// The index of the debt worth the most, among those with anything owed; a tie goes to the later one.
#[must_use]
pub fn largest_debt(debts: &[RankedDebt]) -> Option<usize> {
    debts
        .iter()
        .enumerate()
        .filter(|(_, d)| !d.assets.is_zero())
        .fold(None, |best: Option<(usize, U256)>, (i, d)| match best {
            Some((_, value)) if value > d.value_usd_wad => best,
            _ => Some((i, d.value_usd_wad)),
        })
        .map(|(i, _)| i)
}

/// The share of the position's liquidity (1e18 = all) that repaying `repaid_usd_wad` seizes, as the contract sizes
/// it: the debt's value times the penalty over the collateral's value, the penalty shrunk to what the collateral can
/// bear (not under [`MIN_INCENTIVE_WAD`]) and the share capped at all of it. Taken [`SEIZE_MARGIN_BPS`] low.
/// `penalty_wad` is the repaid debt's own (the lens's `repayPenaltyWad`).
///
/// # Errors
///
/// [`Error::Overflow`] on absurd values.
pub fn seized_share_wad(
    collateral_usd_wad: U256,
    debt_usd_wad: U256,
    repaid_usd_wad: U256,
    penalty_wad: U256,
) -> Result<U256> {
    if collateral_usd_wad.is_zero() {
        return Ok(U256::ZERO);
    }
    let penalty = liquidation::bearable_penalty(collateral_usd_wad, debt_usd_wad, penalty_wad)?;
    let share = mul_div(mul_div(repaid_usd_wad, penalty, WAD)?, WAD, collateral_usd_wad)?;
    minus_bps(share.min(WAD), SEIZE_MARGIN_BPS)
}

/// Breakdown of a liquidation in USD (1e18), at the lens's values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct LiquidationUsd {
    /// Collateral value seized (USD, 1e18).
    pub seized_usd_wad: U256,
    /// Debt value repaid (USD, 1e18).
    pub repaid_usd_wad: U256,
    /// Protocol liquidation fee (USD, 1e18): zero when collateral is under 1.02x the debt or when debt is written off.
    pub protocol_fee_usd_wad: U256,
    /// Liquidator's net incentive (USD, 1e18): seized less repaid less protocol fee.
    pub liquidator_incentive_usd_wad: U256,
    /// Debt written off (USD, 1e18): zero unless taking every unit leaves unpaid debt.
    pub written_off_usd_wad: U256,
}

/// Breaks down a liquidation in USD (1e18): seized value, repaid debt, protocol fee, liquidator's net incentive
/// and written-off debt, matching `LiquidationLogic.seizeFor` (zero protocol fee when collateral is under 1.02x
/// the debt or when the liquidation writes debt off).
///
/// # Errors
///
/// [`Error::Overflow`] on absurd values.
pub fn liquidation_usd(
    collateral_usd_wad: U256,
    debt_usd_wad: U256,
    repaid_usd_wad: U256,
    penalty_wad: U256,
    liquidation_fee_wad: U256,
    has_other_debt: bool,
) -> Result<LiquidationUsd> {
    if collateral_usd_wad.is_zero() || repaid_usd_wad.is_zero() {
        return Ok(LiquidationUsd::default());
    }
    let penalty = liquidation::bearable_penalty(collateral_usd_wad, debt_usd_wad, penalty_wad)?;
    let raw_seized = mul_div(repaid_usd_wad, penalty, WAD)?;

    let (seized, repaid, full_seize) = if raw_seized < collateral_usd_wad {
        (raw_seized, repaid_usd_wad, false)
    } else {
        let coverable = mul_div_rounding_up(collateral_usd_wad, WAD, MIN_INCENTIVE_WAD)?;
        (collateral_usd_wad, repaid_usd_wad.min(coverable), true)
    };
    let written_off = if full_seize {
        debt_usd_wad.saturating_sub(repaid)
    } else {
        U256::ZERO
    };

    let bad_debt = mul_div(collateral_usd_wad, WAD, MIN_INCENTIVE_WAD)? < debt_usd_wad
        || (full_seize && (!written_off.is_zero() || has_other_debt));
    let fee = if !liquidation_fee_wad.is_zero() && !bad_debt && seized > repaid {
        mul_div(seized.try_sub(repaid)?, liquidation_fee_wad, WAD)?
    } else {
        U256::ZERO
    };

    let liquidator_incentive = seized.saturating_sub(repaid.saturating_add(fee));
    Ok(LiquidationUsd {
        seized_usd_wad: seized,
        repaid_usd_wad: repaid,
        protocol_fee_usd_wad: fee,
        liquidator_incentive_usd_wad: liquidator_incentive,
        written_off_usd_wad: written_off,
    })
}

/// The debt (USD, 1e18) a liquidation repaying `repaid_usd_wad` writes off: zero unless it seizes every unit, then
/// all the position's debt it doesn't repay. Taking everything caps the repayment at the collateral over
/// [`MIN_INCENTIVE_WAD`], as `LiquidationLogic.seizeFor` does. An estimate at the lens's USD values.
///
/// # Errors
///
/// [`Error::Overflow`] on absurd values.
pub fn written_off_usd_wad(
    collateral_usd_wad: U256,
    debt_usd_wad: U256,
    repaid_usd_wad: U256,
    penalty_wad: U256,
) -> Result<U256> {
    let penalty = liquidation::bearable_penalty(collateral_usd_wad, debt_usd_wad, penalty_wad)?;
    if mul_div(repaid_usd_wad, penalty, WAD)? < collateral_usd_wad {
        return Ok(U256::ZERO);
    }
    let coverable = mul_div_rounding_up(collateral_usd_wad, WAD, MIN_INCENTIVE_WAD)?;
    Ok(debt_usd_wad.saturating_sub(repaid_usd_wad.min(coverable)))
}

/// The borrower's fees owed that go with `seized_share_wad` of its units (1e18 = all), rounded down: seized units
/// carry their uncollected fees, which the liquidator's full redeem pays to its wallet.
///
/// # Errors
///
/// [`Error::Overflow`] on absurd values.
pub fn seized_fees(fees_owed: PerToken<U256>, seized_share_wad: U256) -> Result<PerToken<U256>> {
    let share = seized_share_wad.min(WAD);
    Ok(PerToken {
        token0: fees_owed.token0.try_mul(share)?.try_div(WAD)?,
        token1: fees_owed.token1.try_mul(share)?.try_div(WAD)?,
    })
}

/// A position being liquidated and the debt repaid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct Liquidated {
    /// One end of the range, a Q64.96 sqrt price.
    pub sqrt_pa_x96: U160,
    /// The other end.
    pub sqrt_pb_x96: U160,
    /// The account's liquidity.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub liquidity: u128,
    /// The pool's sqrt price now.
    pub pool_sqrt_price_x96: U160,
    /// The collateral's value in USD (1e18), from the lens.
    pub collateral_usd_wad: U256,
    /// All its debt in USD.
    pub debt_usd_wad: U256,
    /// The repaid debt's value in USD.
    pub repaid_usd_wad: U256,
    /// The repaid debt's penalty (`repayPenaltyWad`), or the position's.
    pub penalty_wad: U256,
    /// The pair token repaid; `None` for a token outside the pair.
    pub repaid: Option<Side>,
}

/// The numbers of a liquidation: the liquidity seized, the redeem's minimums and what is sold into the debt asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct LiquidationAmounts {
    /// Liquidity seized.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub liquidity: u128,
    /// Least token0 the redeem pays.
    pub min0: U256,
    /// Least token1.
    pub min1: U256,
    /// The pair token sold into the debt asset: the one that is not the debt (token0 for a debt outside the pair).
    pub sell: Side,
    /// What the redeem pays of it at the pool price: quote this amount. Zero: no swap.
    pub sell_amount: U256,
    /// What the redeem surely pays of it: the router sells all it holds, guaranteed only down to this.
    pub least: U256,
}

/// Sizes a liquidation of one whole debt: the share it seizes ([`seized_share_wad`]), the redeem's minimums within
/// the slippage, and the sale of the other pair token.
///
/// # Errors
///
/// As [`slippage::withdraw_minimums`].
pub fn liquidation_amounts(position: &Liquidated, slippage_bps: u64) -> Result<LiquidationAmounts> {
    let p = position;
    let share = seized_share_wad(p.collateral_usd_wad, p.debt_usd_wad, p.repaid_usd_wad, p.penalty_wad)?;
    let liquidity = to_u128(U256::from(p.liquidity).try_mul(share)?.try_div(WAD)?)?;
    let (min0, min1) = slippage::withdraw_minimums(
        p.sqrt_pa_x96,
        p.sqrt_pb_x96,
        p.pool_sqrt_price_x96,
        liquidity,
        slippage_bps,
    )?;
    let (expected0, expected1) = amounts_for_liquidity(p.pool_sqrt_price_x96, p.sqrt_pa_x96, p.sqrt_pb_x96, liquidity)?;
    let sell = if p.repaid == Some(Side::Token0) {
        Side::Token1
    } else {
        Side::Token0
    };
    let (sell_amount, least) = match sell {
        Side::Token0 => (expected0, min0),
        Side::Token1 => (expected1, min1),
    };
    Ok(LiquidationAmounts {
        liquidity,
        min0,
        min1,
        sell,
        sell_amount,
        least,
    })
}

/// What `amount_in` of one pair token is worth in the other at the pool price, less the pool's fee.
///
/// # Errors
///
/// [`Error::CastOverflow`] when the value does not fit in 256 bits.
pub fn at_pool(sqrt_price_x96: U160, amount_in: U256, sell: Side, swap_fee_wad: U256) -> Result<U256> {
    let sqrt = U512::from(U256::from(sqrt_price_x96));
    let square = sqrt.wrapping_mul(sqrt);
    let out = match sell {
        Side::Token0 => U512::from(amount_in).try_mul(square)?.wrapping_shr(192),
        Side::Token1 if square.is_zero() => U512::ZERO,
        Side::Token1 => U512::from(amount_in).wrapping_shl(192).wrapping_div(square),
    };
    let out = U256::checked_from_limbs_slice(out.as_limbs()).ok_or(Error::CastOverflow)?;
    out.try_mul(WAD.try_sub(swap_fee_wad)?)?.try_div(WAD)
}

/// The sale's least output: its quote scaled down to what the redeem surely pays (`least` of `sell_amount`), or
/// without a quote the pool price less its fee; then less the slippage.
///
/// # Errors
///
/// As [`at_pool`]; [`Error::Underflow`] for a slippage above 100%.
pub fn liquidation_min_out(
    amounts: &LiquidationAmounts,
    quoted_out: Option<U256>,
    pool_sqrt_price_x96: U160,
    swap_fee_wad: U256,
    slippage_bps: u64,
) -> Result<U256> {
    let out = match quoted_out {
        Some(quoted) => quoted.try_mul(amounts.least)?.try_div(amounts.sell_amount)?,
        None => at_pool(pool_sqrt_price_x96, amounts.least, amounts.sell, swap_fee_wad)?,
    };
    slippage::min_out(out, slippage_bps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_liquidation_that_takes_everything_writes_off() {
        let penalty = wad(108) / U256::from(100);
        // $1,000 of collateral, $900 of debt in one token: seizes $972, nothing written off
        assert_eq!(
            written_off_usd_wad(wad(1_000), wad(900), wad(900), penalty).unwrap(),
            U256::ZERO
        );
        // $1,000 of collateral, $1,100 of debt: everything goes, the repayment capped at 1,000 / 1.02
        let coverable = mul_div_rounding_up(wad(1_000), WAD, MIN_INCENTIVE_WAD).unwrap();
        assert_eq!(
            written_off_usd_wad(wad(1_000), wad(1_100), wad(1_100), penalty).unwrap(),
            wad(1_100) - coverable
        );
        // Repaying the smaller of two debts that doesn't take everything writes nothing off
        assert_eq!(
            written_off_usd_wad(wad(1_000), wad(1_100), wad(300), penalty).unwrap(),
            U256::ZERO
        );
    }

    #[test]
    fn worked_examples_at_10_and_20_percent() {
        let penalty = wad(108) / U256::from(100);
        let ten_pct = wad(10) / U256::from(100);
        let twenty_pct = wad(20) / U256::from(100);

        // Case 1: $1,100 collateral, $1,000 debt -> $80 bonus -> $8 protocol, $72 liquidator
        let c1 = liquidation_usd(wad(1_100), wad(1_000), wad(1_000), penalty, ten_pct, false).unwrap();
        assert_eq!(c1.seized_usd_wad, wad(1_080));
        assert_eq!(c1.protocol_fee_usd_wad, wad(8));
        assert_eq!(c1.liquidator_incentive_usd_wad, wad(72));
        assert_eq!(c1.written_off_usd_wad, U256::ZERO);

        // Case 2: $1,050 collateral, $1,000 debt -> penalty shrinks to 1.05 -> $50 bonus -> $10 protocol at 20%
        let c2 = liquidation_usd(wad(1_050), wad(1_000), wad(1_000), penalty, twenty_pct, false).unwrap();
        assert_eq!(c2.seized_usd_wad, wad(1_050));
        assert_eq!(c2.protocol_fee_usd_wad, wad(10));
        assert_eq!(c2.liquidator_incentive_usd_wad, wad(40));
        assert_eq!(c2.written_off_usd_wad, U256::ZERO);

        // Case 3: $1,010 collateral, $1,000 debt -> floor 1.02, write-off unchanged, protocol gets 0
        let coverable = mul_div_rounding_up(wad(1_010), WAD, MIN_INCENTIVE_WAD).unwrap();
        let c3 = liquidation_usd(wad(1_010), wad(1_000), wad(1_000), penalty, ten_pct, false).unwrap();
        assert_eq!(c3.seized_usd_wad, wad(1_010));
        assert_eq!(c3.repaid_usd_wad, coverable);
        assert_eq!(c3.protocol_fee_usd_wad, U256::ZERO);
        assert_eq!(c3.written_off_usd_wad, wad(1_000) - coverable);
        assert_eq!(c3.liquidator_incentive_usd_wad, wad(1_010) - coverable);

        // Two-debt full seize: second debt writes off -> protocol gets 0 even when Debt 0 is paid in full
        let c4 = liquidation_usd(wad(1_020), wad(1_200), wad(1_000), penalty, ten_pct, true).unwrap();
        assert_eq!(c4.protocol_fee_usd_wad, U256::ZERO);
        assert_eq!(c4.written_off_usd_wad, wad(200));

        // Partial liquidation under 1.02x total debt -> no protocol fee even though call 1 doesn't take the last unit
        let c5 = liquidation_usd(wad(1_010), wad(1_000), wad(500), penalty, ten_pct, true).unwrap();
        assert_eq!(c5.protocol_fee_usd_wad, U256::ZERO);
        assert_eq!(c5.written_off_usd_wad, U256::ZERO);

        // Two-debt between 1.02x and 1.08x ($1,050 collateral, two $500 debts) -> fee on both calls, 0 write-off
        let c6a = liquidation_usd(wad(1_050), wad(1_000), wad(500), penalty, ten_pct, true).unwrap();
        assert_eq!(c6a.seized_usd_wad, wad(525));
        assert_eq!(c6a.protocol_fee_usd_wad, wad(25) / U256::from(10));
        assert_eq!(c6a.written_off_usd_wad, U256::ZERO);
        let c6b = liquidation_usd(wad(525), wad(500), wad(500), penalty, ten_pct, false).unwrap();
        assert_eq!(c6b.seized_usd_wad, wad(525));
        assert_eq!(c6b.protocol_fee_usd_wad, wad(25) / U256::from(10));
        assert_eq!(c6b.written_off_usd_wad, U256::ZERO);
    }

    #[test]
    fn seized_units_take_their_share_of_the_fees() {
        let fees = PerToken {
            token0: U256::from(1_000_u64),
            token1: U256::from(7_u64),
        };
        let half = seized_fees(fees, WAD / U256::from(2_u8)).unwrap();
        assert_eq!((half.token0, half.token1), (U256::from(500_u64), U256::from(3_u64)));
        assert_eq!(seized_fees(fees, WAD * U256::from(2_u8)).unwrap(), fees);
    }

    fn wad(x: u64) -> U256 {
        U256::from(x).try_mul(WAD).unwrap()
    }

    #[test]
    fn the_largest_debt_wins_and_a_tie_goes_to_the_later_one() {
        let debt = |assets: u64, value: u64| RankedDebt {
            assets: U256::from(assets),
            value_usd_wad: wad(value),
        };
        assert_eq!(largest_debt(&[debt(1, 5), debt(1, 9)]), Some(1));
        assert_eq!(largest_debt(&[debt(1, 9), debt(1, 5)]), Some(0));
        assert_eq!(largest_debt(&[debt(1, 5), debt(1, 5)]), Some(1));
        assert_eq!(largest_debt(&[debt(1, 5), debt(0, 9)]), Some(0));
        assert_eq!(largest_debt(&[]), None);
    }

    #[test]
    fn the_penalty_shrinks_to_what_the_collateral_bears() {
        // $105 of collateral for $100 of debt: a 1.08 penalty becomes 1.05; repaying it all seizes all, less the margin
        let share = seized_share_wad(wad(105), wad(100), wad(100), wad(108) / U256::from(100)).unwrap();
        assert_eq!(share, minus_bps(WAD, SEIZE_MARGIN_BPS).unwrap());
        // Never under 1.02: $101 for $100 seizes everything
        let share = seized_share_wad(wad(101), wad(100), wad(50), wad(108) / U256::from(100)).unwrap();
        assert_eq!(share, minus_bps(wad(51) / U256::from(101), SEIZE_MARGIN_BPS).unwrap());
    }

    #[test]
    fn a_quote_is_scaled_to_what_the_redeem_surely_pays() {
        let amounts = LiquidationAmounts {
            liquidity: 0,
            min0: U256::ZERO,
            min1: U256::ZERO,
            sell: Side::Token0,
            sell_amount: U256::from(1_000),
            least: U256::from(990),
        };
        // 2,000 quoted for 1,000: 1,980 for the 990 surely paid, less 1%
        let min = liquidation_min_out(&amounts, Some(U256::from(2_000)), U160::ZERO, U256::ZERO, 100).unwrap();
        assert_eq!(min, U256::from(1_960));
    }

    #[test]
    fn at_pool_converts_both_ways() {
        let one = U160::from(crate::uniswap::Q96);
        assert_eq!(
            at_pool(one, U256::from(1_000), Side::Token0, U256::ZERO).unwrap(),
            U256::from(1_000)
        );
        assert_eq!(
            at_pool(one, U256::from(1_000), Side::Token1, U256::ZERO).unwrap(),
            U256::from(1_000)
        );
        assert_eq!(
            at_pool(U160::ZERO, U256::from(1_000), Side::Token1, U256::ZERO).unwrap(),
            U256::ZERO
        );
    }

    #[test]
    fn coverable_rounds_up_to_match_contract() {
        let penalty = wad(108) / U256::from(100);
        let expected_coverable = mul_div_rounding_up(wad(1_010), WAD, MIN_INCENTIVE_WAD).unwrap();
        let usd = liquidation_usd(wad(1_010), wad(1_000), wad(1_000), penalty, U256::ZERO, false).unwrap();
        assert_eq!(usd.repaid_usd_wad, expected_coverable);
        assert_eq!(usd.written_off_usd_wad, wad(1_000) - expected_coverable);
        assert_eq!(
            written_off_usd_wad(wad(1_010), wad(1_000), wad(1_000), penalty).unwrap(),
            wad(1_000) - expected_coverable
        );
    }
}
