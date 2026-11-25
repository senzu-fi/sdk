//! `FeeLogic`: per-holder LP fee accounting.
//!
//! `owed += balance · (feeGrowthInsideNow − feeGrowthInsideLast) / 2^128`, with fee growth allowed to wrap as in
//! Uniswap. Every payout of fees or rewards sends the protocol its cut ([`cut`]) and the holder the rest
//! ([`after_cut`]).

use alloy_primitives::U256;

use crate::error::Result;
use crate::num::{Checked, WAD, to_u128};
use crate::uniswap::{mul_div, mul_div_rounding_up};

const Q128: U256 = U256::from_limbs([0, 0, 1, 0]);

/// A holder's fee checkpoint in one position id (`FeeCheckpoint` in `Types.sol`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct FeeCheckpoint {
    /// Token0 fee growth inside the range at the last checkpoint (Q128).
    pub fee_growth_inside0_x128: U256,
    /// Token1 fee growth inside the range at the last checkpoint (Q128).
    pub fee_growth_inside1_x128: U256,
    /// Token0 fees settled and not yet collected.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub owed0: u128,
    /// Token1 fees settled and not yet collected.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub owed1: u128,
}

impl FeeCheckpoint {
    /// Fees owed to a holder of `balance` units at the given fee growth, without settling (`FeeLogic.pending`).
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`](crate::Error::Overflow) or [`Error::MulDiv`](crate::Error::MulDiv) on overflow.
    pub fn pending(&self, balance: U256, fee_growth0_x128: U256, fee_growth1_x128: U256) -> Result<(U256, U256)> {
        let delta0 = fee_growth0_x128.wrapping_sub(self.fee_growth_inside0_x128);
        let delta1 = fee_growth1_x128.wrapping_sub(self.fee_growth_inside1_x128);
        Ok((
            U256::from(self.owed0).try_add(mul_div(balance, delta0, Q128)?)?,
            U256::from(self.owed1).try_add(mul_div(balance, delta1, Q128)?)?,
        ))
    }

    /// Settles pending fees into `owed0` / `owed1` and moves the snapshot (`FeeLogic.checkpoint`).
    ///
    /// # Errors
    ///
    /// As [`FeeCheckpoint::pending`], and [`Error::CastOverflow`](crate::Error::CastOverflow) when owed fees pass
    /// 128 bits.
    pub fn checkpoint(&mut self, balance: U256, fee_growth0_x128: U256, fee_growth1_x128: U256) -> Result<()> {
        let (owed0, owed1) = self.pending(balance, fee_growth0_x128, fee_growth1_x128)?;
        *self = Self {
            fee_growth_inside0_x128: fee_growth0_x128,
            fee_growth_inside1_x128: fee_growth1_x128,
            owed0: to_u128(owed0)?,
            owed1: to_u128(owed1)?,
        };
        Ok(())
    }
}

/// The bookmark a liquidator holds after a seize (`FeeLogic.mergeMark`): `held` units of its own at `held_mark`,
/// and `units` seized at the borrower's `moved_mark`, both at or under `growth_x128`. The seized units take what
/// they earned and the borrower hadn't collected with them; the merged mark rounds so they are never credited more
/// than their two bookmarks would credit them. Use it with [`FeeCheckpoint::pending`] to preview what a liquidator
/// is owed after a seize. Works for fees (either token) and rewards alike.
///
/// # Errors
///
/// [`Error::Overflow`](crate::Error::Overflow) when `held + units` passes 256 bits, and
/// [`Error::MulDiv`](crate::Error::MulDiv) when both are 0, as the Solidity reverts.
pub fn merge_mark(held_mark: U256, moved_mark: U256, held: U256, units: U256, growth_x128: U256) -> Result<U256> {
    // Fee growth wraps, as in Uniswap: the marks are compared by their distance to the growth
    let held_gap = growth_x128.wrapping_sub(held_mark);
    let moved_gap = growth_x128.wrapping_sub(moved_mark);
    let total = held.try_add(units)?;
    let gap = if moved_gap >= held_gap {
        held_gap.wrapping_add(mul_div(units, moved_gap.wrapping_sub(held_gap), total)?)
    } else {
        held_gap.wrapping_sub(mul_div_rounding_up(units, held_gap.wrapping_sub(moved_gap), total)?)
    };
    Ok(growth_x128.wrapping_sub(gap))
}

/// The protocol's cut of `amount` of fees or rewards at `cut_wad` (1e18 = 100%, `ISenzuRiskConfig.feeCut`), rounded
/// down so a holder is never charged more than the rate (`FeeLogic.cut`).
///
/// # Errors
///
/// [`Error::MulDiv`](crate::Error::MulDiv) when the cut itself passes 256 bits (a cut over 100%).
pub fn cut(amount: U256, cut_wad: U256) -> Result<U256> {
    mul_div(amount, cut_wad, WAD)
}

/// What a holder is paid of `amount` owed: the amount less the protocol's [`cut`], as the adapter's `feesOwed`,
/// `rewardsOwed` and every payout show it.
///
/// # Errors
///
/// As [`cut`], and [`Error::Underflow`](crate::Error::Underflow) for a cut over 100%.
pub fn after_cut(amount: U256, cut_wad: U256) -> Result<U256> {
    amount.try_sub(cut(amount, cut_wad)?)
}
