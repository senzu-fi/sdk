//! Opening a position: what it borrows, the swap into its range, the mint's minimums, and the leverage it can take.
//! The edits reuse the swap helpers ([`range_swap`], [`lean_toward_debt`], [`minimums_after_swap`]).

use alloy_primitives::{U160, U256};

use super::{PerToken, Priced, minus_bps};
use crate::error::Result;
use crate::lens::{self, LeverageBorrow, REF_LIQUIDITY};
use crate::num::{Checked, WAD, to_i256};
use crate::slippage;
use crate::types::{Asset, CollateralInput, Side};
use crate::uniswap::{amounts_for_liquidity, liquidity_for_amounts};

/// The scale of a pair token's share of a position's value: basis points with six decimals, `10¹⁰` = all of it.
pub const SHARE_SCALE: U256 = U256::from_limbs([10_000_000_000, 0, 0, 0]);
/// The safety buffer of every "max" an SDK offers: limits are computed as if the liquidation penalty were 2% higher.
pub const APP_BUFFER_BPS: u64 = 10_200;
/// How far [`lean_toward_debt`] tilts a swap: 3%.
pub const DEBT_BIAS_BPS: u64 = 300;
/// The share of the distance from 1x to the maximum [`leverage_cap`] offers, in percent, so a price tick between the
/// preview and the transaction cannot push an open over it.
pub const LEVERAGE_HEADROOM_PERCENT: u64 = 97;
/// Rounds [`open_amounts`] spends sizing a one-token borrow so the swap leaves nothing over.
const LEFTOVER_ROUNDS: usize = 3;

/// What a position borrows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub enum BorrowSplit {
    /// Both tokens, in the ratio the range holds.
    Pair,
    /// Only token0.
    Token0,
    /// Only token1.
    Token1,
    /// Both, `token0_share` of the value in token0 (out of [`SHARE_SCALE`]) and the rest in token1.
    #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
    Mix {
        /// Token0's share of the debt's value, out of [`SHARE_SCALE`].
        token0_share: U256,
    },
}

impl BorrowSplit {
    /// The one token borrowed, for [`BorrowSplit::Token0`] and [`BorrowSplit::Token1`].
    #[must_use]
    pub const fn single(self) -> Option<Side> {
        match self {
            Self::Token0 => Some(Side::Token0),
            Self::Token1 => Some(Side::Token1),
            Self::Pair | Self::Mix { .. } => None,
        }
    }
}

/// A range by its ends and the pair's oracle prices: what valuing it needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct PricedRange {
    /// One end, a Q64.96 sqrt price.
    pub sqrt_pa_x96: U160,
    /// The other end.
    pub sqrt_pb_x96: U160,
    /// The pair's oracle prices.
    pub prices: PerToken<Priced>,
}

/// Token0's share of the value the range holds at `sqrt_price_x96`, out of [`SHARE_SCALE`]: what a pair borrow
/// splits by. `None` for a range that holds nothing.
///
/// # Errors
///
/// As [`amounts_for_liquidity`].
pub fn pair_token0_share(range: &PricedRange, sqrt_price_x96: U160) -> Result<Option<U256>> {
    let (r0, r1) = amounts_for_liquidity(sqrt_price_x96, range.sqrt_pa_x96, range.sqrt_pb_x96, REF_LIQUIDITY)?;
    let (usd0, usd1) = (range.prices.token0.usd(r0)?, range.prices.token1.usd(r1)?);
    let total = usd0.try_add(usd1)?;
    if total.is_zero() {
        return Ok(None);
    }
    Ok(Some(usd0.try_mul(SHARE_SCALE)?.try_div(total)?))
}

/// The margin's value in USD (1e18).
///
/// # Errors
///
/// [`crate::Error::Overflow`] on absurd amounts.
pub fn margin_usd(prices: &PerToken<Priced>, margin: &PerToken<U256>) -> Result<U256> {
    prices
        .token0
        .usd(margin.token0)?
        .try_add(prices.token1.usd(margin.token1)?)
}

/// What an open borrows, in token units, before the opening fee: the margin's value times (leverage - 1), split by
/// `borrow` (a pair borrow at `split_sqrt_price_x96`, usually the pool's price).
///
/// # Errors
///
/// [`crate::Error::DivisionByZero`] for a zero price on a borrowed token, and [`crate::Error::Overflow`] on absurd
/// amounts.
pub fn borrow_amounts(
    range: &PricedRange,
    margin: &PerToken<U256>,
    leverage_wad: U256,
    borrow: BorrowSplit,
    split_sqrt_price_x96: U160,
) -> Result<PerToken<U256>> {
    let none = PerToken::default();
    if leverage_wad <= WAD {
        return Ok(none);
    }
    let prices = &range.prices;
    let debt_usd = margin_usd(prices, margin)?
        .try_mul(leverage_wad.try_sub(WAD)?)?
        .try_div(WAD)?;
    let share = match borrow {
        BorrowSplit::Token0 => {
            return Ok(PerToken {
                token0: prices.token0.amount(debt_usd)?,
                token1: U256::ZERO,
            });
        }
        BorrowSplit::Token1 => {
            return Ok(PerToken {
                token0: U256::ZERO,
                token1: prices.token1.amount(debt_usd)?,
            });
        }
        BorrowSplit::Pair => pair_token0_share(range, split_sqrt_price_x96)?,
        BorrowSplit::Mix { token0_share } => Some(token0_share),
    };
    let Some(share) = share else {
        return Ok(none);
    };
    let debt0_usd = debt_usd.try_mul(share)?.try_div(SHARE_SCALE)?;
    Ok(PerToken {
        token0: prices.token0.amount(debt0_usd)?,
        token1: prices.token1.amount(debt_usd.try_sub(debt0_usd)?)?,
    })
}

/// A swap of one pair token into the other, with its least output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct RangeSwap {
    /// The token sold.
    pub sell: Side,
    /// Amount sold.
    pub amount_in: U256,
    /// Least it buys.
    pub min_out: U256,
}

/// The swap that turns `amounts` into the ratio the range holds at the pool price, sized with the pool's fee
/// ([`lens::swap_for_range`]); its least output allows the slippage. `None` when the amounts already fit.
///
/// # Errors
///
/// As [`lens::swap_for_range`].
pub fn range_swap(
    sqrt_pa_x96: U160,
    sqrt_pb_x96: U160,
    amounts: PerToken<U256>,
    pool_sqrt_price_x96: U160,
    swap_fee_wad: U256,
    slippage_bps: u64,
) -> Result<Option<RangeSwap>> {
    let quote = lens::swap_for_range(
        sqrt_pa_x96,
        sqrt_pb_x96,
        amounts.token0,
        amounts.token1,
        pool_sqrt_price_x96,
        swap_fee_wad,
    )?;
    if quote.amount_in.is_zero() {
        return Ok(None);
    }
    let sell = if quote.zero_for_one { Side::Token0 } else { Side::Token1 };
    Ok(Some(RangeSwap {
        sell,
        amount_in: quote.amount_in,
        min_out: slippage::min_out(quote.amount_out, slippage_bps)?,
    }))
}

/// Tilts a swap so that what does not fit the range is left in the borrowed token, where the plan's closing repay
/// returns it to the debt: sells [`DEBT_BIAS_BPS`] less of the debt token, or that much more of the other (at most
/// `available`). Without it a price move leaves dust in the token the position did not borrow.
///
/// # Errors
///
/// [`crate::Error::DivisionByZero`] for an empty swap.
pub fn lean_toward_debt(swap: RangeSwap, debt: Side, available: U256) -> Result<RangeSwap> {
    if swap.sell == debt {
        return Ok(RangeSwap {
            amount_in: minus_bps(swap.amount_in, DEBT_BIAS_BPS)?,
            min_out: minus_bps(swap.min_out, DEBT_BIAS_BPS)?,
            ..swap
        });
    }
    let amount_in = super::plus_bps(swap.amount_in, DEBT_BIAS_BPS)?.min(available);
    Ok(RangeSwap {
        amount_in,
        min_out: swap.min_out.try_mul(amount_in)?.try_div(swap.amount_in)?,
        ..swap
    })
}

/// What `amounts` become after `swap` pays only its least output.
fn after_swap(amounts: PerToken<U256>, swap: Option<&RangeSwap>) -> Result<PerToken<U256>> {
    let Some(s) = swap else {
        return Ok(amounts);
    };
    let mut after = amounts;
    after.set(s.sell, amounts.get(s.sell).saturating_sub(s.amount_in));
    after.set(s.sell.other(), amounts.get(s.sell.other()).try_add(s.min_out)?);
    Ok(after)
}

/// The mint's minimums for a deposit of `amounts` after `swap` pays only its least output, anywhere in the slippage
/// band.
///
/// # Errors
///
/// As [`slippage::deposit_minimums`].
pub fn minimums_after_swap(
    sqrt_pa_x96: U160,
    sqrt_pb_x96: U160,
    sqrt_price_x96: U160,
    amounts: PerToken<U256>,
    swap: Option<&RangeSwap>,
    slippage_bps: u64,
) -> Result<(U256, U256)> {
    let after = after_swap(amounts, swap)?;
    slippage::deposit_minimums(
        sqrt_pa_x96,
        sqrt_pb_x96,
        sqrt_price_x96,
        after.token0,
        after.token1,
        slippage_bps,
    )
}

/// What an open is sized from: the range and prices, the margin, the leverage and borrow, and the pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct OpenSizing {
    /// The range and the pair's oracle prices.
    pub range: PricedRange,
    /// Margin in token units.
    pub margin: PerToken<U256>,
    /// Size over margin, 1e18 = 1x.
    pub leverage_wad: U256,
    /// What it borrows.
    pub borrow: BorrowSplit,
    /// Swap into the range's ratio; false mints only what fits.
    pub swap: bool,
    /// The pool's sqrt price now, where the DEX mints.
    pub pool_sqrt_price_x96: U160,
    /// The pool's fee, 1e18 = 100%.
    pub swap_fee_wad: U256,
    /// Accepted price move, in basis points.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub slippage_bps: u64,
}

impl OpenSizing {
    /// The swap of the margin plus `debt` into the range, tilted toward a one-token borrow.
    fn swap_for(&self, debt: PerToken<U256>) -> Result<Option<RangeSwap>> {
        if !self.swap {
            return Ok(None);
        }
        let holding = PerToken {
            token0: self.margin.token0.try_add(debt.token0)?,
            token1: self.margin.token1.try_add(debt.token1)?,
        };
        let r = &self.range;
        let swap = range_swap(
            r.sqrt_pa_x96,
            r.sqrt_pb_x96,
            holding,
            self.pool_sqrt_price_x96,
            self.swap_fee_wad,
            self.slippage_bps,
        )?;
        match (swap, self.borrow.single()) {
            (Some(s), Some(debt_side)) => Ok(Some(lean_toward_debt(s, debt_side, holding.get(s.sell))?)),
            (swap, _) => Ok(swap),
        }
    }

    /// What is left in the borrowed token after minting all that fits, `swap` paying its full quote.
    fn leftover(&self, debt: PerToken<U256>, swap: Option<&RangeSwap>, debt_side: Side) -> Result<U256> {
        let mut held = [
            to_i256(self.margin.token0.try_add(debt.token0)?)?,
            to_i256(self.margin.token1.try_add(debt.token1)?)?,
        ];
        if let Some(s) = swap {
            let kept = U256::from(
                slippage::BPS
                    .checked_sub(self.slippage_bps)
                    .ok_or(crate::Error::Underflow)?,
            );
            let out = to_i256(s.min_out.try_mul(U256::from(slippage::BPS))?.try_div(kept)?)?;
            let amount_in = to_i256(s.amount_in)?;
            let [h0, h1] = held;
            held = match s.sell {
                Side::Token0 => [h0.try_sub(amount_in)?, h1.try_add(out)?],
                Side::Token1 => [h0.try_add(out)?, h1.try_sub(amount_in)?],
            };
        }
        let r = &self.range;
        let (r0, r1) = amounts_for_liquidity(self.pool_sqrt_price_x96, r.sqrt_pa_x96, r.sqrt_pb_x96, REF_LIQUIDITY)?;
        if r0.is_zero() || r1.is_zero() {
            return Ok(U256::ZERO);
        }
        let (r0, r1, wad) = (to_i256(r0)?, to_i256(r1)?, to_i256(WAD)?);
        let [h0, h1] = held;
        let liquidity = h0.try_mul(wad)?.try_div(r0)?.min(h1.try_mul(wad)?.try_div(r1)?);
        let (h, reference) = match debt_side {
            Side::Token0 => (h0, r0),
            Side::Token1 => (h1, r1),
        };
        let left = h.try_sub(liquidity.try_mul(reference)?.try_div(wad)?)?;
        Ok(if left.is_negative() {
            U256::ZERO
        } else {
            left.into_raw()
        })
    }
}

/// The numbers of an open: what it borrows and the swap into the range before it is quoted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct OpenAmounts {
    /// Borrowed, in token units.
    pub debt: PerToken<U256>,
    /// The swap into the range; quote it and replace its least output with [`slippage::min_out`] of the quote.
    pub swap: Option<RangeSwap>,
}

/// Sizes an open: the borrow ([`borrow_amounts`] at the pool price), and the swap of margin plus debt into the range.
/// A one-token borrow with a swap borrows a little more, so the swap's tilt toward the debt leaves nothing over.
///
/// # Errors
///
/// As [`borrow_amounts`] and [`range_swap`].
pub fn open_amounts(sizing: &OpenSizing) -> Result<OpenAmounts> {
    let s = sizing;
    let mut debt = borrow_amounts(&s.range, &s.margin, s.leverage_wad, s.borrow, s.pool_sqrt_price_x96)?;
    let borrows = !debt.token0.is_zero() || !debt.token1.is_zero();
    if let (true, true, Some(side)) = (s.swap, borrows, s.borrow.single()) {
        let target = debt.get(side);
        let mut borrowed = target;
        for _ in 0..LEFTOVER_ROUNDS {
            let mut trial = PerToken::default();
            trial.set(side, borrowed);
            borrowed = target.try_add(s.leftover(trial, s.swap_for(trial)?.as_ref(), side)?)?;
        }
        debt.set(side, borrowed);
    }
    Ok(OpenAmounts {
        debt,
        swap: s.swap_for(debt)?,
    })
}

/// The mint's minimums of an open: margin plus `debt`, after `swap` (quoted) pays only its least output.
///
/// # Errors
///
/// As [`minimums_after_swap`].
pub fn open_minimums(sizing: &OpenSizing, debt: PerToken<U256>, swap: Option<&RangeSwap>) -> Result<(U256, U256)> {
    let (s, r) = (sizing, &sizing.range);
    let amounts = PerToken {
        token0: s.margin.token0.try_add(debt.token0)?,
        token1: s.margin.token1.try_add(debt.token1)?,
    };
    minimums_after_swap(
        r.sqrt_pa_x96,
        r.sqrt_pb_x96,
        s.pool_sqrt_price_x96,
        amounts,
        swap,
        s.slippage_bps,
    )
}

/// An exact open (`OPEN_EXACT`, no swap): the liquidity margin plus debt mint at the pool price, and the most it may
/// borrow of each token: what the mint takes anywhere in the slippage band, less the margin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct ExactOpen {
    /// Liquidity minted.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub liquidity: u128,
    /// Most borrowed of each token.
    pub max_borrow: PerToken<U256>,
}

/// Sizes an exact open of `margin` plus `debt`.
///
/// # Errors
///
/// As [`liquidity_for_amounts`] and [`slippage::deposit_maximums`].
pub fn exact_open(sizing: &OpenSizing, debt: PerToken<U256>) -> Result<ExactOpen> {
    let (s, r) = (sizing, &sizing.range);
    let liquidity = liquidity_for_amounts(
        s.pool_sqrt_price_x96,
        r.sqrt_pa_x96,
        r.sqrt_pb_x96,
        s.margin.token0.try_add(debt.token0)?,
        s.margin.token1.try_add(debt.token1)?,
    )?;
    let (max0, max1) = slippage::deposit_maximums(
        r.sqrt_pa_x96,
        r.sqrt_pb_x96,
        s.pool_sqrt_price_x96,
        liquidity,
        s.slippage_bps,
    )?;
    let max_borrow = PerToken {
        token0: max0.saturating_sub(s.margin.token0),
        token1: max1.saturating_sub(s.margin.token1),
    };
    Ok(ExactOpen { liquidity, max_borrow })
}

/// What of `amounts` a mint at `sqrt_price_x96` takes without a swap: the rest stays in the wallet.
///
/// # Errors
///
/// As [`liquidity_for_amounts`].
pub fn fit_to_range(
    sqrt_pa_x96: U160,
    sqrt_pb_x96: U160,
    sqrt_price_x96: U160,
    amounts: PerToken<U256>,
) -> Result<PerToken<U256>> {
    let liquidity = liquidity_for_amounts(sqrt_price_x96, sqrt_pa_x96, sqrt_pb_x96, amounts.token0, amounts.token1)?;
    let (token0, token1) = amounts_for_liquidity(sqrt_price_x96, sqrt_pa_x96, sqrt_pb_x96, liquidity)?;
    Ok(PerToken { token0, token1 })
}

/// A liquidation penalty with the [`APP_BUFFER_BPS`] safety buffer on top.
///
/// # Errors
///
/// [`crate::Error::Overflow`] on an absurd penalty.
pub fn buffered_penalty(penalty_wad: U256) -> Result<U256> {
    penalty_wad
        .try_mul(U256::from(APP_BUFFER_BPS))?
        .try_div(U256::from(10_000_u64))
}

/// The leverage offered for a maximum, in tenths (25 = 2.5x): [`LEVERAGE_HEADROOM_PERCENT`] of the way from 1x to
/// `max_leverage_wad`, rounded down to a tenth, at least 1x.
///
/// # Errors
///
/// [`crate::Error::Overflow`] on an absurd maximum.
pub fn leverage_cap(max_leverage_wad: U256) -> Result<u64> {
    let above_one = max_leverage_wad.saturating_sub(WAD);
    let offered = WAD
        .try_mul(U256::from(100_u8))?
        .try_add(above_one.try_mul(U256::from(LEVERAGE_HEADROOM_PERCENT))?)?;
    let tenths = offered.try_div(WAD.try_mul(U256::from(10_u8))?)?;
    Ok(u64::try_from(tenths).unwrap_or(u64::MAX).max(10))
}

/// The highest leverage (1e18) a new position in `collateral`'s range opens with keeping the [`APP_BUFFER_BPS`]
/// margin ([`lens::max_leverage`] at the buffered penalty). A mix takes the lower of the pair borrow and the one
/// token it leans to (token0 when its share is above the pair's at `split_sqrt_price_x96`).
///
/// # Errors
///
/// As [`lens::max_leverage`].
pub fn open_max_leverage(
    collateral: &CollateralInput,
    borrow: BorrowSplit,
    penalty_wad: U256,
    borrow_fee_wad: U256,
    split_sqrt_price_x96: U160,
) -> Result<U256> {
    let penalty = buffered_penalty(penalty_wad)?;
    let max = |b: LeverageBorrow| lens::max_leverage(collateral, &b, penalty, borrow_fee_wad);
    match borrow {
        BorrowSplit::Pair => max(LeverageBorrow::PairRatio),
        BorrowSplit::Token0 => max(LeverageBorrow::Single(Asset::Token0)),
        BorrowSplit::Token1 => max(LeverageBorrow::Single(Asset::Token1)),
        BorrowSplit::Mix { token0_share } => {
            let range = PricedRange {
                sqrt_pa_x96: collateral.sqrt_pa_x96,
                sqrt_pb_x96: collateral.sqrt_pb_x96,
                prices: PerToken {
                    token0: Priced {
                        price_wad: collateral.token0.price_wad,
                        decimals: collateral.token0.decimals,
                    },
                    token1: Priced {
                        price_wad: collateral.token1.price_wad,
                        decimals: collateral.token1.decimals,
                    },
                },
            };
            let pair_share = pair_token0_share(&range, split_sqrt_price_x96)?.unwrap_or_default();
            let lean = if token0_share > pair_share {
                Asset::Token0
            } else {
                Asset::Token1
            };
            let Some(ctx) = lens::MaxLeverageContext::new(collateral)? else {
                return Ok(WAD);
            };
            Ok(ctx
                .search(&LeverageBorrow::PairRatio, penalty, borrow_fee_wad)?
                .min(ctx.search(&LeverageBorrow::Single(lean), penalty, borrow_fee_wad)?))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uniswap::sqrt_price_at_tick;

    fn dollar(decimals: u8) -> Priced {
        Priced {
            price_wad: WAD,
            decimals,
        }
    }

    fn range(lower: i32, upper: i32) -> PricedRange {
        PricedRange {
            sqrt_pa_x96: sqrt_price_at_tick(lower).unwrap(),
            sqrt_pb_x96: sqrt_price_at_tick(upper).unwrap(),
            prices: PerToken {
                token0: dollar(18),
                token1: dollar(18),
            },
        }
    }

    #[test]
    fn a_symmetric_range_at_its_middle_is_half_each() {
        let share = pair_token0_share(&range(-600, 600), sqrt_price_at_tick(0).unwrap())
            .unwrap()
            .unwrap();
        let half = SHARE_SCALE / U256::from(2);
        assert!(share.abs_diff(half) < U256::from(1_000), "{share}");
    }

    #[test]
    fn one_x_borrows_nothing_and_three_x_borrows_twice_the_margin() {
        let margin = PerToken {
            token0: WAD,
            token1: U256::ZERO,
        };
        let r = range(-600, 600);
        let mid = sqrt_price_at_tick(0).unwrap();
        assert_eq!(
            borrow_amounts(&r, &margin, WAD, BorrowSplit::Token1, mid).unwrap(),
            PerToken::default()
        );
        let three = WAD * U256::from(3);
        let debt = borrow_amounts(&r, &margin, three, BorrowSplit::Token1, mid).unwrap();
        assert_eq!(
            debt,
            PerToken {
                token0: U256::ZERO,
                token1: WAD * U256::from(2)
            }
        );
    }

    #[test]
    fn leaning_sells_less_of_the_debt_or_more_of_the_other() {
        let swap = RangeSwap {
            sell: Side::Token0,
            amount_in: U256::from(10_000),
            min_out: U256::from(9_000),
        };
        let less = lean_toward_debt(swap, Side::Token0, U256::MAX).unwrap();
        assert_eq!((less.amount_in, less.min_out), (U256::from(9_700), U256::from(8_730)));
        let more = lean_toward_debt(swap, Side::Token1, U256::from(10_100)).unwrap();
        assert_eq!((more.amount_in, more.min_out), (U256::from(10_100), U256::from(9_090)));
    }

    #[test]
    fn the_cap_keeps_headroom_and_never_offers_under_one_x() {
        let wad = |tenths: u64| U256::from(tenths) * WAD / U256::from(10);
        assert_eq!(leverage_cap(wad(5)).unwrap(), 10);
        assert_eq!(leverage_cap(wad(10)).unwrap(), 10);
        // 1 + 9 · 0.97 = 9.73
        assert_eq!(leverage_cap(wad(100)).unwrap(), 97);
        assert_eq!(leverage_cap(wad(20)).unwrap(), 19);
    }
}
