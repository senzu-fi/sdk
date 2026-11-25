//! `MarketLogic` and the pure part of `BorrowLogic`: share math, interest accrual, borrowing and repaying in one
//! lending market.

use alloy_primitives::U256;

use crate::error::{Error, Result};
use crate::irm::{self, AdaptiveRate};
use crate::num::{Checked, WAD, to_u128};
use crate::uniswap::{mul_div, mul_div_rounding_up};

/// Virtual shares added to every share conversion.
pub const VIRTUAL_SHARES: U256 = U256::from_limbs([1_000_000, 0, 0, 0]);
/// Virtual assets added to every share conversion.
pub const VIRTUAL_ASSETS: U256 = U256::from_limbs([1, 0, 0, 0]);
/// Most borrow shares one position can hold in one market (`DebtLanes`: under 2^119 per debt).
pub const MAX_POSITION_SHARES: U256 = U256::from_limbs([u64::MAX, (1 << 55) - 1, 0, 0]);

/// A position's shares in one market after a borrow adds `added` to its `existing` ones (`DebtLanes.lane`).
///
/// # Errors
///
/// [`Error::DebtTooLarge`] past [`MAX_POSITION_SHARES`], and an overflow of the sum.
pub fn position_shares(existing: U256, added: U256) -> Result<U256> {
    let total = existing.try_add(added)?;
    if total > MAX_POSITION_SHARES {
        return Err(Error::DebtTooLarge);
    }
    Ok(total)
}

/// Rounding direction of a share conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub enum Rounding {
    /// Toward zero.
    Down,
    /// Away from zero.
    Up,
}

/// `assets → shares` with virtual shares and assets.
///
/// # Errors
///
/// [`Error::Overflow`] or [`Error::MulDiv`] when the totals or the result overflow.
pub fn to_shares(assets: U256, total_assets: U256, total_shares: U256, rounding: Rounding) -> Result<U256> {
    let shares = total_shares.try_add(VIRTUAL_SHARES)?;
    let total = total_assets.try_add(VIRTUAL_ASSETS)?;
    match rounding {
        Rounding::Up => mul_div_rounding_up(assets, shares, total),
        Rounding::Down => mul_div(assets, shares, total),
    }
}

/// `shares → assets` with virtual shares and assets.
///
/// # Errors
///
/// [`Error::Overflow`] or [`Error::MulDiv`] when the totals or the result overflow.
pub fn to_assets(shares: U256, total_assets: U256, total_shares: U256, rounding: Rounding) -> Result<U256> {
    let total = total_assets.try_add(VIRTUAL_ASSETS)?;
    let all_shares = total_shares.try_add(VIRTUAL_SHARES)?;
    match rounding {
        Rounding::Up => mul_div_rounding_up(shares, total, all_shares),
        Rounding::Down => mul_div(shares, total, all_shares),
    }
}

/// Opening fee of a borrow: `assets · borrowFee`, rounded up (`SenzuBorrowable.borrow`).
///
/// # Errors
///
/// [`Error::MulDiv`] on overflow.
pub fn borrow_fee(assets: U256, borrow_fee_wad: U256) -> Result<U256> {
    mul_div_rounding_up(assets, borrow_fee_wad, WAD)
}

/// What a borrow of `assets` owes: the assets plus their opening fee ([`borrow_fee`]).
///
/// # Errors
///
/// [`Error::MulDiv`] or [`Error::Overflow`] on overflow.
pub fn debt_with_fee(assets: U256, borrow_fee_wad: U256) -> Result<U256> {
    assets.try_add(borrow_fee(assets, borrow_fee_wad)?)
}

/// State of one lending market (the accounting fields of `Market` in `Types.sol`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct Market {
    /// Cash plus borrows.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub total_supply_assets: u128,
    /// Debt owed to lenders, interest included.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub total_borrow_assets: u128,
    /// Borrow shares of all positions.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub total_borrow_shares: u128,
    /// Timestamp of the last accrual.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub last_accrual: u64,
    /// Share of interest minted to the DAO reserves, 1e18 = 100%.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub reserve_factor_wad: u64,
    /// Interest accrued but under one unit of the asset, times 1e18, carried to the next accrual.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub interest_remainder: u64,
    /// Borrow rate at the target utilization as of `last_accrual`, per second (1e18 = 100%).
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub rate_at_target: u64,
}

/// Interest booked by one [`Market::accrue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct Accrual {
    /// Interest added to both borrows and supply.
    pub interest: U256,
    /// Supply shares minted to the DAO reserves.
    pub reserve_shares: U256,
}

/// Shares booked by one [`Market::borrow`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct Borrow {
    /// Borrow shares added to the position (for `assets + fee`).
    pub shares: U256,
    /// Supply shares minted to the DAO for the fee.
    pub fee_shares: U256,
}

/// What a repayment removes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct Repay {
    /// Assets taken from the payer.
    pub assets: U256,
    /// Borrow shares burned.
    pub shares: U256,
}

impl Market {
    /// Idle cash.
    ///
    /// # Errors
    ///
    /// [`Error::Underflow`] if borrows exceed supply (never for a real market).
    pub fn cash(&self) -> Result<U256> {
        Ok(U256::from(self.total_supply_assets.try_sub(self.total_borrow_assets)?))
    }

    /// Borrows / supply, 1e18 = 100%.
    ///
    /// # Errors
    ///
    /// [`Error::MulDiv`] on overflow (never for a real market).
    pub fn utilization(&self) -> Result<U256> {
        if self.total_supply_assets == 0 {
            return Ok(U256::ZERO);
        }
        mul_div(
            U256::from(self.total_borrow_assets),
            WAD,
            U256::from(self.total_supply_assets),
        )
    }

    /// Borrow rate per second now, for a market accrued to now (`MarketLogic.borrowRate`): the curve at the
    /// market's utilization around its rate at target.
    ///
    /// # Errors
    ///
    /// As [`Market::utilization`] and [`irm::curve`].
    pub fn borrow_rate(&self) -> Result<U256> {
        irm::curve(
            U256::from(self.rate_at_target),
            irm::utilization_error(self.utilization()?)?,
        )
    }

    /// Supply rate per second: `borrowRate · utilization · (1 − reserveFactor)`.
    ///
    /// # Errors
    ///
    /// [`Error::Underflow`] for a reserve factor above 100%, [`Error::MulDiv`] on overflow.
    pub fn supply_rate(&self, borrow_rate_per_second: U256) -> Result<U256> {
        let keep = WAD.try_sub(U256::from(self.reserve_factor_wad))?;
        mul_div(mul_div(borrow_rate_per_second, self.utilization()?, WAD)?, keep, WAD)
    }

    /// Most that can be borrowed now: the market's cash (`SenzuLens.maxBorrow`).
    ///
    /// # Errors
    ///
    /// As [`Market::cash`].
    pub fn borrow_limit(&self) -> Result<U256> {
        self.cash()
    }

    /// Accrues interest up to `now` (`MarketLogic.accrue`). `total_supply_shares` is the ERC-6909 total supply of
    /// the market's lender id. The period's integrated rate and the new rate at target come from the utilization
    /// before accrual ([`irm::adaptive_rate`]); the interest is `borrows · (e^integral − 1)`, the integral capped at
    /// [`irm::MAX_GROWTH_EXPONENT`], rounded down with the part under one unit carried. The rate at target adapts
    /// even without borrows, but not in an empty market.
    ///
    /// # Errors
    ///
    /// [`Error::Underflow`] when `now` is before the last accrual, [`Error::CastOverflow`] when a total passes
    /// 128 bits or the rate at target 64, and the rate math's errors.
    pub fn accrue(&mut self, total_supply_shares: U256, now: u64) -> Result<Accrual> {
        let mut next = *self;
        let elapsed = now.checked_sub(next.last_accrual).ok_or(Error::Underflow)?;
        if elapsed == 0 {
            return Ok(Accrual::default());
        }
        next.last_accrual = now;
        if next.total_supply_assets == 0 {
            *self = next;
            return Ok(Accrual::default());
        }
        let AdaptiveRate {
            integral,
            end_rate_at_target,
        } = irm::adaptive_rate(
            U256::from(next.rate_at_target),
            next.utilization()?,
            U256::from(elapsed),
        )?;
        next.rate_at_target = u64::try_from(end_rate_at_target).map_err(|_| Error::CastOverflow)?;
        if next.total_borrow_assets == 0 {
            *self = next;
            return Ok(Accrual::default());
        }

        let exponent = integral.min(irm::MAX_GROWTH_EXPONENT);
        let growth = irm::to_uint(irm::exp_minus_one(irm::to_int(exponent)?)?)?;
        let scaled =
            mul_div(U256::from(next.total_borrow_assets), growth, WAD)?.try_add(U256::from(next.interest_remainder))?;
        let (interest, rest) = scaled.div_rem(WAD);
        next.interest_remainder = rest.saturating_to::<u64>();
        if interest.is_zero() {
            *self = next;
            return Ok(Accrual::default());
        }

        next.total_borrow_assets = to_u128(U256::from(next.total_borrow_assets).try_add(interest)?)?;
        next.total_supply_assets = to_u128(U256::from(next.total_supply_assets).try_add(interest)?)?;

        let reserve_assets = mul_div(interest, U256::from(next.reserve_factor_wad), WAD)?;
        let reserve_shares = if reserve_assets.is_zero() {
            U256::ZERO
        } else {
            let supply_before = U256::from(next.total_supply_assets).try_sub(reserve_assets)?;
            to_shares(reserve_assets, supply_before, total_supply_shares, Rounding::Down)?
        };
        *self = next;
        Ok(Accrual {
            interest,
            reserve_shares,
        })
    }

    /// Books a borrow of `assets` with opening `fee` (`BorrowLogic.addDebt`, market part). The market must be
    /// accrued to now. The position's limits (max debts, [`position_shares`]) depend on other state and are not
    /// checked.
    ///
    /// # Errors
    ///
    /// [`Error::InsufficientCash`] and the arithmetic errors of the contract.
    pub fn borrow(&mut self, assets: U256, fee: U256, total_supply_shares: U256) -> Result<Borrow> {
        let mut next = *self;
        if assets > next.cash()? {
            return Err(Error::InsufficientCash);
        }
        let debt = assets.try_add(fee)?;

        let mut fee_shares = U256::ZERO;
        if !fee.is_zero() {
            fee_shares = to_shares(
                fee,
                U256::from(next.total_supply_assets),
                total_supply_shares,
                Rounding::Down,
            )?;
            next.total_supply_assets = next.total_supply_assets.try_add(to_u128(fee)?)?;
        }

        let shares = to_shares(
            debt,
            U256::from(next.total_borrow_assets),
            U256::from(next.total_borrow_shares),
            Rounding::Up,
        )?;
        next.total_borrow_assets = next.total_borrow_assets.try_add(to_u128(debt)?)?;
        next.total_borrow_shares = next.total_borrow_shares.try_add(to_u128(shares)?)?;
        *self = next;
        Ok(Borrow { shares, fee_shares })
    }

    /// Debt of a position holding `shares` borrow shares, rounded up (`SenzuBorrowable.debtOf` on an accrued
    /// market).
    ///
    /// # Errors
    ///
    /// As [`to_assets`].
    pub fn debt_of(&self, shares: U256) -> Result<U256> {
        to_assets(
            shares,
            U256::from(self.total_borrow_assets),
            U256::from(self.total_borrow_shares),
            Rounding::Up,
        )
    }

    /// What repaying up to `assets` removes from a position with `debt_shares` (`BorrowLogic.repayAmounts`): the
    /// whole debt if `assets` covers it, otherwise the shares worth the whole wei `assets` clears (none for a
    /// payment that clears no whole wei; the rest of `assets` goes to lenders).
    ///
    /// # Errors
    ///
    /// [`Error::NoDebt`] without debt and [`Error::ZeroAmount`] for a payment of 0.
    pub fn repay_amounts(&self, debt_shares: U256, assets: U256) -> Result<Repay> {
        if debt_shares.is_zero() {
            return Err(Error::NoDebt);
        }
        let debt = self.debt_of(debt_shares)?;
        if assets >= debt {
            return Ok(Repay {
                assets: debt,
                shares: debt_shares,
            });
        }
        if assets.is_zero() {
            return Err(Error::ZeroAmount);
        }
        let borrowed = U256::from(self.total_borrow_assets);
        let borrow_shares = U256::from(self.total_borrow_shares);
        let bought = to_shares(assets, borrowed, borrow_shares, Rounding::Down)?;
        let clears = to_assets(bought, borrowed, borrow_shares, Rounding::Down)?;
        let shares = to_shares(clears, borrowed, borrow_shares, Rounding::Up)?;
        Ok(Repay { assets, shares })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accrue_and_borrow_leave_market_unchanged_on_error() {
        let mut m = Market {
            total_supply_assets: u128::MAX.saturating_sub(1),
            total_borrow_assets: u128::MAX.saturating_sub(10),
            total_borrow_shares: u128::MAX.saturating_sub(10),
            last_accrual: 1_000,
            rate_at_target: 10_000_000_000,
            ..Market::default()
        };
        let before = m;
        assert_eq!(m.accrue(U256::from(u128::MAX), 31_537_000), Err(Error::CastOverflow));
        assert_eq!(m, before);

        let mut b = Market {
            total_supply_assets: u128::MAX,
            total_borrow_assets: 0,
            total_borrow_shares: u128::MAX.saturating_sub(1),
            ..Market::default()
        };
        let before_b = b;
        assert_eq!(
            b.borrow(U256::from(100_u64), U256::from(10_u64), U256::from(1_000_u64)),
            Err(Error::Overflow)
        );
        assert_eq!(b, before_b);
    }
}
