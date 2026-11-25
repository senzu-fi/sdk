//! Editing a position: its numbers in any state, the states after adding or removing margin and repaying, how to
//! reach a target leverage, and the amounts a redeem or a repay moves.

use alloy_primitives::{I256, U160, U256};

use super::open::{BorrowSplit, RangeSwap, buffered_penalty, lean_toward_debt, range_swap};
use super::{PerToken, Priced};
use crate::error::Result;
use crate::health;
use crate::lens::{self, LiquidationPrices, REF_LIQUIDITY};
use crate::num::{Checked, WAD, to_i256, to_u128};
use crate::slippage;
use crate::types::{Asset, CollateralInput, DebtInput, Side, TokenInput};
use crate::uniswap::{amounts_for_liquidity, mul_div};

/// Extra a payoff pulls over the debt, for interest accrued before it lands: 0.1%.
pub const PAYOFF_BUFFER_BPS: u64 = 10;
/// Halvings [`max_removable_wad`] searches: precise to 1e18 / 2²⁰, about one part in a million.
const REMOVABLE_STEPS: usize = 20;

/// A debt in a token outside the pair (a liquid token, as WETH on NVDA/USDG): an edit leaves it as it is, and the
/// health check stresses it up by its own σ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct OtherDebt {
    /// Amount owed.
    pub assets: U256,
    /// The token's oracle price, decimals and σ.
    pub token: TokenInput,
}

/// A position with what its previews need: its range, the pair's oracle prices and σ, and the penalties.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct EditPosition {
    /// One end of the range, a Q64.96 sqrt price.
    pub sqrt_pa_x96: U160,
    /// The other end.
    pub sqrt_pb_x96: U160,
    /// The pair's oracle prices, decimals and σ.
    pub tokens: PerToken<TokenInput>,
    /// The oracle's sqrt price (the lens's `oracleSqrtPriceX96`).
    pub oracle_sqrt_price_x96: U160,
    /// The position's liquidation penalty (its debts outside the pair included).
    pub position_penalty_wad: U256,
    /// The pair's.
    pub pair_penalty_wad: U256,
    /// Debts outside the pair.
    #[cfg_attr(feature = "serde", serde(default))]
    pub other_debts: Vec<OtherDebt>,
}

/// A position in some state: its liquidity and each pair debt, in token units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct EditState {
    /// The account's liquidity.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub liquidity: u128,
    /// Debt in each pair token.
    pub debt: PerToken<U256>,
}

impl EditPosition {
    /// The penalty its previews use: the position's, never under its pair's.
    fn penalty(&self) -> U256 {
        self.position_penalty_wad.max(self.pair_penalty_wad)
    }

    fn priced(&self, side: Side) -> Priced {
        let t = self.tokens.get(side);
        Priced {
            price_wad: t.price_wad,
            decimals: t.decimals,
        }
    }

    fn collateral(&self, liquidity: u128) -> CollateralInput {
        CollateralInput {
            sqrt_pa_x96: self.sqrt_pa_x96,
            sqrt_pb_x96: self.sqrt_pb_x96,
            liquidity,
            token0: self.tokens.token0,
            token1: self.tokens.token1,
        }
    }

    /// The debts outside the pair, as the health check reads them.
    fn other_debt_inputs(&self) -> Result<Vec<DebtInput>> {
        let collateral = self.collateral(0);
        self.other_debts
            .iter()
            .filter(|d| !d.assets.is_zero())
            .map(|d| collateral.debt(&Asset::Other(d.token), d.assets))
            .collect()
    }

    /// The range's value per [`REF_LIQUIDITY`] at `sqrt_price_x96`, in each token's USD.
    fn reference_usd(&self, sqrt_price_x96: U160) -> Result<(U256, U256)> {
        let (r0, r1) = amounts_for_liquidity(sqrt_price_x96, self.sqrt_pa_x96, self.sqrt_pb_x96, REF_LIQUIDITY)?;
        Ok((self.priced(Side::Token0).usd(r0)?, self.priced(Side::Token1).usd(r1)?))
    }
}

/// The liquidity `usd_wad` of value adds to the range at the oracle price.
///
/// # Errors
///
/// [`crate::Error::CastOverflow`] for liquidity over 128 bits.
pub fn liquidity_for_usd(position: &EditPosition, usd_wad: U256) -> Result<u128> {
    let (usd0, usd1) = position.reference_usd(position.oracle_sqrt_price_x96)?;
    let reference = usd0.try_add(usd1)?;
    if reference.is_zero() {
        return Ok(0);
    }
    to_u128(mul_div(usd_wad, U256::from(REF_LIQUIDITY), reference)?)
}

/// A position's numbers in some state, at oracle prices (USD in 1e18).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct EditPreview {
    /// The collateral's value.
    pub collateral_usd_wad: U256,
    /// The debt's value.
    pub debt_usd_wad: U256,
    /// Collateral less debt, never negative.
    pub margin_usd_wad: U256,
    /// Size over margin (1e18 = 1x), rounded down; zero without margin.
    pub leverage_wad: U256,
    /// Health at the liquidation check: above 1e18 is safe; `U256::MAX` without debt.
    pub health_wad: U256,
    /// Passes the contract's check, so the transaction would go through.
    pub allowed: bool,
    /// Also keeps the 2% safety buffer an SDK's maximums aim for.
    pub within_margin: bool,
    /// Where it would be liquidated; `None` without debt, or when not asked for.
    pub liquidation: Option<LiquidationPrices>,
}

/// The numbers of the position in `state`, from the contracts' own health math. `with_liquidation_prices: false`
/// skips the liquidation price search, the slow part.
///
/// # Errors
///
/// As [`health::evaluate`] and [`lens::liquidation_prices`].
pub fn preview_state(position: &EditPosition, state: &EditState, with_liquidation_prices: bool) -> Result<EditPreview> {
    let collateral = position.collateral(state.liquidity);
    let mut debts = position.other_debt_inputs()?;
    for (asset, amount) in [(Asset::Token0, state.debt.token0), (Asset::Token1, state.debt.token1)] {
        if !amount.is_zero() {
            debts.push(collateral.debt(&asset, amount)?);
        }
    }
    let penalty = position.penalty();
    let (health, within_margin) = if debts.is_empty() {
        (health::evaluate(&collateral, &debts, penalty)?, true)
    } else {
        health::evaluate_with_buffered(&collateral, &debts, penalty, buffered_penalty(penalty)?)?
    };
    let margin = health.collateral_usd_wad.saturating_sub(health.debt_usd_wad);
    let leverage_wad = if margin.is_zero() {
        U256::ZERO
    } else {
        mul_div(health.collateral_usd_wad, WAD, margin)?
    };
    let liquidation = if with_liquidation_prices && !debts.is_empty() {
        Some(lens::liquidation_prices_with_initial(
            &collateral,
            &debts,
            penalty,
            health.passes(),
        )?)
    } else {
        None
    };
    Ok(EditPreview {
        collateral_usd_wad: health.collateral_usd_wad,
        debt_usd_wad: health.debt_usd_wad,
        margin_usd_wad: margin,
        leverage_wad,
        health_wad: health.health_wad,
        allowed: debts.is_empty() || health.passes(),
        within_margin,
        liquidation,
    })
}

/// The state after adding `margin` (token units) as liquidity, valued at the oracle; the debts stay.
///
/// # Errors
///
/// As [`liquidity_for_usd`].
pub fn add_margin_state(position: &EditPosition, state: &EditState, margin: PerToken<U256>) -> Result<EditState> {
    let usd = position
        .priced(Side::Token0)
        .usd(margin.token0)?
        .try_add(position.priced(Side::Token1).usd(margin.token1)?)?;
    Ok(EditState {
        liquidity: state.liquidity.try_add(liquidity_for_usd(position, usd)?)?,
        ..*state
    })
}

/// The state after repaying `repay` (token units); a repayment over a debt clears it.
#[must_use]
pub fn repay_state(state: &EditState, repay: PerToken<U256>) -> EditState {
    let debt = PerToken {
        token0: state.debt.token0.saturating_sub(repay.token0),
        token1: state.debt.token1.saturating_sub(repay.token1),
    };
    EditState { debt, ..*state }
}

/// The state after redeeming `fraction_wad` of the liquidity (1e18 = all); the debts stay.
///
/// # Errors
///
/// [`crate::Error::Underflow`] for a fraction over 1e18.
pub fn remove_margin_state(state: &EditState, fraction_wad: U256) -> Result<EditState> {
    let removed = to_u128(mul_div(U256::from(state.liquidity), fraction_wad, WAD)?)?;
    Ok(EditState {
        liquidity: state.liquidity.try_sub(removed)?,
        ..*state
    })
}

/// The largest share of the liquidity (1e18 = all) a redeem can take while keeping the safety buffer; all of it
/// without debt. Searched by halving, so it is a hair under the exact limit.
///
/// # Errors
///
/// As [`preview_state`].
pub fn max_removable_wad(position: &EditPosition, state: &EditState) -> Result<U256> {
    let base = position.collateral(state.liquidity);
    let mut debts = position.other_debt_inputs()?;
    for (asset, amount) in [(Asset::Token0, state.debt.token0), (Asset::Token1, state.debt.token1)] {
        if !amount.is_zero() {
            debts.push(base.debt(&asset, amount)?);
        }
    }
    if debts.is_empty() {
        return Ok(WAD);
    }
    let penalty = buffered_penalty(position.penalty())?;
    let corners = health::StressCorners::new(&base.token0, &base.token1)?;
    let (mut low, mut high) = (U256::ZERO, WAD);
    for _ in 0..REMOVABLE_STEPS {
        let mid = low.try_add(high)?.try_div(U256::from(2_u8))?;
        let next = remove_margin_state(state, mid)?;
        let collateral = position.collateral(next.liquidity);
        if corners.passes(&collateral, &debts, penalty)? {
            low = mid;
        } else {
            high = mid;
        }
    }
    Ok(low)
}

/// How a position gets to a target leverage: `state` is where it ends, the rest what moves (token units).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct LeverageChange {
    /// The position after the change.
    pub state: EditState,
    /// Up: what is borrowed.
    pub borrow: PerToken<U256>,
    /// Down: the share of the liquidity redeemed (1e18 = all).
    pub redeem_wad: U256,
    /// Down: what is repaid.
    pub repay: PerToken<U256>,
}

/// Splits a borrow of `usd_wad` by `borrow`, a pair borrow in the range's ratio at `split_sqrt_price_x96`.
fn split_borrow(
    position: &EditPosition,
    usd_wad: U256,
    borrow: BorrowSplit,
    split_sqrt_price_x96: U160,
) -> Result<PerToken<U256>> {
    let (p0, p1) = (position.priced(Side::Token0), position.priced(Side::Token1));
    let usd0 = match borrow {
        BorrowSplit::Token0 => usd_wad,
        BorrowSplit::Token1 => U256::ZERO,
        BorrowSplit::Mix { token0_share } => usd_wad.try_mul(token0_share)?.try_div(super::open::SHARE_SCALE)?,
        BorrowSplit::Pair => {
            let (r0, r1) = position.reference_usd(split_sqrt_price_x96)?;
            let total = r0.try_add(r1)?;
            if total.is_zero() {
                return Ok(PerToken::default());
            }
            usd_wad.try_mul(r0)?.try_div(total)?
        }
    };
    Ok(PerToken {
        token0: p0.amount(usd0)?,
        token1: p1.amount(usd_wad.try_sub(usd0)?)?,
    })
}

/// What it takes to bring the position to `target_leverage_wad` (size over margin). Up: a borrow of the difference,
/// split by `borrow` at `split_sqrt_price_x96` (usually the pool's price). Down: a redeem of part of the liquidity and
/// a repayment of each debt in proportion.
///
/// # Errors
///
/// As [`preview_state`].
pub fn leverage_change(
    position: &EditPosition,
    state: &EditState,
    target_leverage_wad: U256,
    borrow: BorrowSplit,
    split_sqrt_price_x96: U160,
) -> Result<LeverageChange> {
    let unchanged = LeverageChange {
        state: *state,
        ..LeverageChange::default()
    };
    let now = preview_state(position, state, false)?;
    let target = mul_div(now.margin_usd_wad, target_leverage_wad, WAD)?;
    let delta = to_i256(target)?.try_sub(to_i256(now.collateral_usd_wad)?)?;
    if delta == I256::ZERO {
        return Ok(unchanged);
    }
    if delta.is_positive() {
        let usd = delta.into_raw();
        let added = split_borrow(position, usd, borrow, split_sqrt_price_x96)?;
        let next = EditState {
            liquidity: state.liquidity.try_add(liquidity_for_usd(position, usd)?)?,
            debt: PerToken {
                token0: state.debt.token0.try_add(added.token0)?,
                token1: state.debt.token1.try_add(added.token1)?,
            },
        };
        return Ok(LeverageChange {
            state: next,
            borrow: added,
            ..unchanged
        });
    }
    // Down repays the pair's debts only: one outside the pair stays
    let other_usd = position
        .other_debt_inputs()?
        .iter()
        .try_fold(U256::ZERO, |sum, d| sum.try_add(d.value_usd_wad))?;
    let pair_debt_usd = now.debt_usd_wad.saturating_sub(other_usd);
    if now.collateral_usd_wad.is_zero() || pair_debt_usd.is_zero() {
        return Ok(unchanged);
    }
    let cut = delta.unsigned_abs().min(pair_debt_usd);
    let redeem_wad = mul_div(cut, WAD, now.collateral_usd_wad)?.min(WAD);
    let repay = PerToken {
        token0: mul_div(state.debt.token0, cut, pair_debt_usd)?,
        token1: mul_div(state.debt.token1, cut, pair_debt_usd)?,
    };
    let after = remove_margin_state(state, redeem_wad)?;
    let next = EditState {
        liquidity: after.liquidity,
        debt: PerToken {
            token0: state.debt.token0.try_sub(repay.token0)?,
            token1: state.debt.token1.try_sub(repay.token1)?,
        },
    };
    Ok(LeverageChange {
        state: next,
        redeem_wad,
        repay,
        ..unchanged
    })
}

/// The swap a leverage increase makes of a one-token borrow into the range, tilted toward the debt; `None` for a
/// borrow of both tokens or one that already fits.
///
/// # Errors
///
/// As [`range_swap`].
pub fn leverage_up_swap(
    sqrt_pa_x96: U160,
    sqrt_pb_x96: U160,
    borrow: PerToken<U256>,
    pool_sqrt_price_x96: U160,
    swap_fee_wad: U256,
    slippage_bps: u64,
) -> Result<Option<RangeSwap>> {
    let debt = match (borrow.token0.is_zero(), borrow.token1.is_zero()) {
        (false, true) => Side::Token0,
        (true, false) => Side::Token1,
        _ => return Ok(None),
    };
    let swap = range_swap(
        sqrt_pa_x96,
        sqrt_pb_x96,
        borrow,
        pool_sqrt_price_x96,
        swap_fee_wad,
        slippage_bps,
    )?;
    swap.map(|s| lean_toward_debt(s, debt, borrow.token0.try_add(borrow.token1)?))
        .transpose()
}

/// A redeem of a share of a position: the units it takes and its minimums.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct RedeemShare {
    /// `units · fraction`, rounded down.
    pub units: U256,
    /// The fraction is all of it: redeem every unit (the router's `ALL`).
    pub all: bool,
    /// Least token0 it pays anywhere in the slippage band.
    pub min0: U256,
    /// Least token1.
    pub min1: U256,
}

/// A position's range, units, liquidity and the pool's price.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct Redeemable {
    /// One end of the range, a Q64.96 sqrt price.
    pub sqrt_pa_x96: U160,
    /// The other end.
    pub sqrt_pb_x96: U160,
    /// The account's units of the position.
    pub units: U256,
    /// The account's liquidity.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub liquidity: u128,
    /// The pool's sqrt price now.
    pub pool_sqrt_price_x96: U160,
}

/// Sizes a redeem of `fraction_wad` of a position (1e18 = all) within the slippage.
///
/// # Errors
///
/// As [`slippage::withdraw_minimums`].
pub fn redeem_share(position: &Redeemable, fraction_wad: U256, slippage_bps: u64) -> Result<RedeemShare> {
    let p = position;
    let all = fraction_wad >= WAD;
    let liquidity = if all {
        p.liquidity
    } else {
        to_u128(mul_div(U256::from(p.liquidity), fraction_wad, WAD)?)?
    };
    let (min0, min1) = slippage::withdraw_minimums(
        p.sqrt_pa_x96,
        p.sqrt_pb_x96,
        p.pool_sqrt_price_x96,
        liquidity,
        slippage_bps,
    )?;
    Ok(RedeemShare {
        units: mul_div(p.units, fraction_wad, WAD)?,
        all,
        min0,
        min1,
    })
}

/// What a redeem of `fraction_wad` of the liquidity pays at the pool price.
///
/// # Errors
///
/// As [`amounts_for_liquidity`].
pub fn redeemed_amounts(position: &Redeemable, fraction_wad: U256) -> Result<PerToken<U256>> {
    let p = position;
    let (all0, all1) = amounts_for_liquidity(p.pool_sqrt_price_x96, p.sqrt_pa_x96, p.sqrt_pb_x96, p.liquidity)?;
    Ok(PerToken {
        token0: mul_div(all0, fraction_wad, WAD)?,
        token1: mul_div(all1, fraction_wad, WAD)?,
    })
}

/// The highest leverage (1e18) the position's range takes keeping the safety buffer, at its penalty.
///
/// # Errors
///
/// As [`super::open::open_max_leverage`].
pub fn edit_max_leverage(position: &EditPosition, borrow: BorrowSplit, borrow_fee_wad: U256) -> Result<U256> {
    let collateral = position.collateral(0);
    super::open::open_max_leverage(
        &collateral,
        borrow,
        position.penalty(),
        borrow_fee_wad,
        position.oracle_sqrt_price_x96,
    )
}

/// What to pull from the wallet to repay `repay` of `debt`: paying it off pulls [`PAYOFF_BUFFER_BPS`] more (and a
/// wei), at most `balance` but never under `repay`.
///
/// # Errors
///
/// [`crate::Error::Overflow`] on an absurd debt.
pub fn repay_pull(repay: U256, debt: U256, balance: U256) -> Result<U256> {
    if repay < debt {
        return Ok(repay);
    }
    let buffer = debt
        .try_mul(U256::from(PAYOFF_BUFFER_BPS))?
        .try_div(U256::from(10_000_u64))?;
    let payoff = debt.try_add(buffer)?.try_add(U256::from(1_u8))?;
    Ok(if payoff < balance { payoff } else { balance.max(repay) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uniswap::sqrt_price_at_tick;

    fn position() -> EditPosition {
        let token = TokenInput {
            price_wad: WAD,
            decimals: 18,
            sigma_wad: U256::from(1_050_000_000_000_000_000_u64),
        };
        EditPosition {
            sqrt_pa_x96: sqrt_price_at_tick(-6_000).unwrap(),
            sqrt_pb_x96: sqrt_price_at_tick(6_000).unwrap(),
            tokens: PerToken {
                token0: token,
                token1: token,
            },
            oracle_sqrt_price_x96: sqrt_price_at_tick(0).unwrap(),
            position_penalty_wad: U256::from(1_080_000_000_000_000_000_u64),
            pair_penalty_wad: U256::from(1_050_000_000_000_000_000_u64),
            other_debts: Vec::new(),
        }
    }

    /// 0.1 WETH at $3,000 (σ 1.1) owed outside the pair: $300 of debt
    fn with_weth_debt() -> EditPosition {
        EditPosition {
            other_debts: vec![OtherDebt {
                assets: WAD.try_div(U256::from(10)).unwrap(),
                token: TokenInput {
                    price_wad: U256::from(3_000).try_mul(WAD).unwrap(),
                    decimals: 18,
                    sigma_wad: U256::from(1_100_000_000_000_000_000_u64),
                },
            }],
            ..position()
        }
    }

    fn state(debt1: u64) -> EditState {
        EditState {
            liquidity: 10_u128.pow(22),
            debt: PerToken {
                token0: U256::ZERO,
                token1: U256::from(debt1).try_mul(WAD).unwrap(),
            },
        }
    }

    #[test]
    fn without_debt_everything_is_removable_and_allowed() {
        let p = position();
        assert_eq!(max_removable_wad(&p, &state(0)).unwrap(), WAD);
        let preview = preview_state(&p, &state(0), true).unwrap();
        assert!(preview.allowed && preview.within_margin && preview.liquidation.is_none());
        assert_eq!(preview.health_wad, U256::MAX);
    }

    #[test]
    fn removing_the_max_keeps_the_buffer() {
        let p = position();
        let s = state(500);
        let max = max_removable_wad(&p, &s).unwrap();
        assert!(max > U256::ZERO && max < WAD);
        assert!(
            preview_state(&p, &remove_margin_state(&s, max).unwrap(), false)
                .unwrap()
                .within_margin
        );
    }

    #[test]
    fn a_leverage_change_lands_on_its_target() {
        let p = position();
        let s = state(300);
        let now = preview_state(&p, &s, false).unwrap().leverage_wad;
        for target in [now * U256::from(2), now / U256::from(2) + WAD / U256::from(2)] {
            let change = leverage_change(&p, &s, target, BorrowSplit::Token1, p.oracle_sqrt_price_x96).unwrap();
            let after = preview_state(&p, &change.state, false).unwrap().leverage_wad;
            assert!(after.abs_diff(target) < target / U256::from(50), "{after} for {target}");
        }
    }

    #[test]
    fn a_debt_outside_the_pair_counts_in_every_preview() {
        let (plain, owing) = (position(), with_weth_debt());
        let s = state(200);
        let (a, b) = (
            preview_state(&plain, &s, false).unwrap(),
            preview_state(&owing, &s, false).unwrap(),
        );
        assert_eq!(b.debt_usd_wad, a.debt_usd_wad + U256::from(300) * WAD);
        assert!(b.health_wad < a.health_wad);
        assert!(max_removable_wad(&owing, &s).unwrap() < max_removable_wad(&plain, &s).unwrap());
        // With no pair debt, the WETH debt still keeps some liquidity in
        assert!(max_removable_wad(&owing, &state(0)).unwrap() < WAD);
    }

    #[test]
    fn leverage_down_repays_the_pair_debts_and_leaves_the_other() {
        let p = with_weth_debt();
        let s = state(300);
        let now = preview_state(&p, &s, false).unwrap();
        let change = leverage_change(
            &p,
            &s,
            now.leverage_wad / U256::from(2),
            BorrowSplit::Token1,
            p.oracle_sqrt_price_x96,
        )
        .unwrap();
        let after = preview_state(&p, &change.state, false).unwrap();
        // The cut comes off the pair debt alone, so the debt drops by about the repayment's value
        let repaid_usd = change.repay.token1;
        assert!((now.debt_usd_wad - after.debt_usd_wad).abs_diff(repaid_usd) <= U256::from(1));
        assert!(after.debt_usd_wad >= U256::from(300) * WAD);
    }

    #[test]
    fn a_payoff_pulls_a_little_more_within_the_balance() {
        let (debt, much) = (U256::from(10_000), U256::from(1_000_000));
        assert_eq!(repay_pull(U256::from(5_000), debt, much).unwrap(), U256::from(5_000));
        assert_eq!(repay_pull(debt, debt, much).unwrap(), U256::from(10_011));
        assert_eq!(repay_pull(debt, debt, U256::from(10_005)).unwrap(), U256::from(10_005));
        assert_eq!(
            repay_pull(U256::from(10_020), debt, U256::from(10_005)).unwrap(),
            U256::from(10_020)
        );
    }
}
