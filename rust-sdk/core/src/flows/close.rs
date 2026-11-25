//! Closing a position: the redeem's minimums, the swaps into debts it cannot pay, compounding, and what the owner
//! receives.

use alloy_primitives::{I256, U160, U256};

use super::{PerToken, Priced, minus_bps, plus_bps, with_fee};
use crate::error::{Error, Result};
use crate::metrics::prices::pool_prices;
use crate::num::{Checked, pow10, to_i256};
use crate::slippage;
use crate::types::Side;
use crate::uniswap::amounts_for_liquidity;

/// Debt a close repays beyond what is owed, for interest accrued before it lands: 0.5%.
pub const INTEREST_BUFFER_BPS: u64 = 50;
/// Debt a deleverage's swap aims beyond what it repays: 3%.
pub const DEBT_HEADROOM_BPS: u64 = 300;
/// Uniswap v3 rounds a range's fees down at every poke, so a position can pay a few wei under what it owes.
pub const FEE_ROUNDING_WEI: u64 = 3;

/// A token's oracle price, as a market reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct MarketPrice {
    /// USD per whole token, 1e18 = $1; zero when the feed has none.
    pub price_wad: U256,
    /// ERC-20 decimals.
    pub decimals: u8,
    /// The price is a weekend's last close: shown, never traded on.
    pub last_close: bool,
}

/// The pair's prices for sizing a close's swaps: a token without a live price is valued at the pool against the other
/// one. Never use these for health. Returns each token's USD price (1e18); both unchanged when neither or both are
/// live.
///
/// # Errors
///
/// As [`pool_prices`].
pub fn pool_valued_prices(token0: MarketPrice, token1: MarketPrice, pool_sqrt_price_x96: U160) -> Result<(U256, U256)> {
    let live = |m: &MarketPrice| !m.price_wad.is_zero() && !m.last_close;
    let unchanged = (token0.price_wad, token1.price_wad);
    if pool_sqrt_price_x96.is_zero() || live(&token0) == live(&token1) {
        return Ok(unchanged);
    }
    let (known, price) = if live(&token1) {
        (Side::Token1, token1.price_wad)
    } else {
        (Side::Token0, token0.price_wad)
    };
    pool_prices(pool_sqrt_price_x96, known, price, token0.decimals, token1.decimals)
}

/// A swap of one pair token into the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct DebtSwap {
    /// The token sold; the other one is bought.
    pub sell: Side,
    /// Amount sold.
    pub amount_in: U256,
    /// Least it must buy.
    pub min_out: U256,
}

/// The swaps that turn what a deleverage's redeem surely paid (`held`) into each debt it repays (`repays`, in token0
/// and token1), the other token sold for the shortfall, in that order. Each aims [`DEBT_HEADROOM_BPS`] over the
/// repayment; a swap capped by what is held asks only what it can reach.
///
/// # Errors
///
/// [`Error::DivisionByZero`] for a zero price.
pub fn swap_into_debts(
    mut held: PerToken<U256>,
    repays: PerToken<U256>,
    prices: PerToken<Priced>,
    slippage_bps: u64,
    swap_fee_wad: U256,
) -> Result<Vec<DebtSwap>> {
    let mut swaps = Vec::new();
    for buy in [Side::Token0, Side::Token1] {
        let assets = repays.get(buy);
        if assets.is_zero() {
            continue;
        }
        let sell = buy.other();
        let need = plus_bps(assets, DEBT_HEADROOM_BPS)?;
        let have = held.get(buy);
        if have >= need {
            held.set(buy, have.try_sub(need)?);
            continue;
        }

        let (out, inp) = (prices.get(buy), prices.get(sell));
        let shortfall_usd = out.usd(need.try_sub(have)?)?;
        let mut amount_in = with_fee(plus_bps(inp.amount(shortfall_usd)?, slippage_bps)?, swap_fee_wad)?;
        let available = held.get(sell);
        let capped = amount_in > available;
        if capped {
            amount_in = available;
        }
        if amount_in.is_zero() {
            continue;
        }
        let reachable = out.amount(inp.usd(amount_in)?)?;
        let after_fee = reachable
            .try_mul(crate::WAD.try_sub(swap_fee_wad)?)?
            .try_div(crate::WAD)?;
        let after_costs = minus_bps(after_fee, slippage_bps)?;
        let shortfall_out = assets.saturating_sub(have);
        let min_out = if capped && after_costs < shortfall_out {
            after_costs
        } else {
            shortfall_out
        };

        swaps.push(DebtSwap {
            sell,
            amount_in,
            min_out,
        });
        held.set(sell, available.try_sub(amount_in)?);
        held.set(buy, U256::ZERO);
    }
    Ok(swaps)
}

/// A position to close or value: its range, liquidity, the pool's price and the fees it is owed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct ClosingPosition {
    /// One end of the range, a Q64.96 sqrt price.
    pub sqrt_pa_x96: U160,
    /// The other end.
    pub sqrt_pb_x96: U160,
    /// The account's liquidity.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub liquidity: u128,
    /// The pool's sqrt price now, where the redeem executes.
    pub pool_sqrt_price_x96: U160,
    /// Token0 fees owed to the account.
    pub fees_owed0: U256,
    /// Token1 fees owed to the account.
    pub fees_owed1: U256,
}

/// One debt of a position being closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct ClosingDebt {
    /// The pair token borrowed; `None` for a token outside the pair.
    pub side: Option<Side>,
    /// Amount owed.
    pub assets: U256,
    /// The borrowed token's price; `None` when its market has none.
    pub price: Option<Priced>,
}

/// How a close buys one debt asset it cannot repay from the redeem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct CloseSwap {
    /// Index of the debt in the input.
    pub debt: u8,
    /// The pair token sold.
    pub sell: Side,
    /// Most the close may sell for this debt (`SWAP_FOR_DEBT`'s cap).
    pub max_in: U256,
    /// What a fixed-amount swap (an aggregator's) sells: the shortfall at the pool's price now (zero when the redeem
    /// covers the debt there), at most what the redeem pays at every end of the band. Quote this one.
    pub exact_in: U256,
    /// `max_in - exact_in`: the cap of the top-up after a fixed-amount swap.
    pub top_up: U256,
}

/// The numbers of a close: the redeem's minimums and a swap for each debt the redeem cannot cover.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct CloseAmounts {
    /// Least token0 the whole redeem pays.
    pub min0: U256,
    /// Least token1.
    pub min1: U256,
    /// In debt order.
    pub swaps: Vec<CloseSwap>,
}

/// Sizes a close: collect the fees, redeem everything, and for each debt the redeem may not cover anywhere in the
/// slippage band, a swap of the other pair token sized at the oracle with the slippage and the pool's fee on top.
/// A narrow range's payout moves faster than the price, so each swap is sized at the band's end that pays the least
/// of the debt asset. `prices` are the pair's, [`pool_valued_prices`] when one is not live.
///
/// # Errors
///
/// As [`slippage::withdraw_ends`]; [`Error::DivisionByZero`] for a swap fee of 100%.
pub fn close_amounts(
    position: &ClosingPosition,
    debts: &[ClosingDebt],
    prices: PerToken<Option<Priced>>,
    slippage_bps: u64,
    swap_fee_wad: U256,
) -> Result<CloseAmounts> {
    let p = position;
    let [low, high] = slippage::withdraw_ends(
        p.sqrt_pa_x96,
        p.sqrt_pb_x96,
        p.pool_sqrt_price_x96,
        p.liquidity,
        slippage_bps,
    )?;
    let (min0, min1) = (high.0, low.1);
    let with_fees = |(amount0, amount1): (U256, U256)| -> Result<PerToken<U256>> {
        Ok(PerToken {
            token0: amount0.try_add(p.fees_owed0)?,
            token1: amount1.try_add(p.fees_owed1)?,
        })
    };
    let ends = [with_fees(low)?, with_fees(high)?];
    let now = with_fees(amounts_for_liquidity(
        p.pool_sqrt_price_x96,
        p.sqrt_pa_x96,
        p.sqrt_pb_x96,
        p.liquidity,
    )?)?;
    // What earlier debts already took of each pair token, at the worst end and at the price now
    let mut used = PerToken::<U256>::default();
    let mut used_now = PerToken::<U256>::default();
    let left = |end: &PerToken<U256>, used: &PerToken<U256>, token: Option<Side>| {
        token.map_or(U256::ZERO, |t| end.get(t).saturating_sub(used.get(t)))
    };
    let priced = |price: Option<Priced>| price.filter(|m| !m.price_wad.is_zero());

    let mut swaps = Vec::new();
    for (index, debt) in debts.iter().enumerate() {
        if debt.assets.is_zero() {
            continue;
        }
        let bought = debt.side;
        let sold = if bought == Some(Side::Token0) {
            Side::Token1
        } else {
            Side::Token0
        };
        let need = plus_bps(debt.assets, INTEREST_BUFFER_BPS)?;
        let [at_low, at_high] = &ends;
        let worst = if left(at_high, &used, bought) < left(at_low, &used, bought) {
            at_high
        } else {
            at_low
        };
        let have = left(worst, &used, bought);
        let have_now = left(&now, &used_now, bought);
        let available = left(worst, &used, Some(sold));
        if let (true, Some(m), Some(n), false) = (
            have < need,
            priced(debt.price),
            priced(prices.get(sold)),
            available.is_zero(),
        ) {
            let shortfall_in = n.amount(m.usd(need.try_sub(have)?)?)?;
            let at_oracle = with_fee(shortfall_in, swap_fee_wad)?;
            let max_in = with_fee(plus_bps(shortfall_in, slippage_bps)?, swap_fee_wad)?.min(available);
            let least = ends.iter().fold(available, |a, e| a.min(left(e, &used, Some(sold))));
            // A fixed-amount swap sells whatever the redeem paid, so it is sized by what is short now, not at the
            // worst end: a debt the redeem covers now is left to the top-up, which buys only what is short
            let short_now = if have_now < need {
                with_fee(n.amount(m.usd(need.try_sub(have_now)?)?)?, swap_fee_wad)?
            } else {
                U256::ZERO
            };
            let exact_in = short_now.min(at_oracle).min(least);
            used_now.set(sold, used_now.get(sold).try_add(exact_in)?);
            swaps.push(CloseSwap {
                debt: u8::try_from(index).map_err(|_| Error::CastOverflow)?,
                sell: sold,
                max_in,
                exact_in,
                top_up: max_in.saturating_sub(exact_in),
            });
            used.set(sold, used.get(sold).try_add(max_in)?);
        }
        if let Some(b) = bought {
            used_now.set(b, used_now.get(b).try_add(have_now.min(need))?);
            used.set(b, used.get(b).try_add(have.min(need))?);
        }
    }
    Ok(CloseAmounts { min0, min1, swaps })
}

/// The minimums of compounding a position's owed fees into its liquidity, anywhere in the slippage band; each fee is
/// taken [`FEE_ROUNDING_WEI`] low.
///
/// # Errors
///
/// As [`slippage::deposit_minimums`].
pub fn compound_minimums(position: &ClosingPosition, slippage_bps: u64) -> Result<(U256, U256)> {
    let paid = |fees: U256| fees.saturating_sub(U256::from(FEE_ROUNDING_WEI));
    let p = position;
    slippage::deposit_minimums(
        p.sqrt_pa_x96,
        p.sqrt_pb_x96,
        p.pool_sqrt_price_x96,
        paid(p.fees_owed0),
        paid(p.fees_owed1),
        slippage_bps,
    )
}

/// What closing sends back, per token: the redeem at the pool price and the fees, less the debts, a shortfall in one
/// token paid with the other at `prices` (the pool's fee on top). Slippage and the plan's headroom are not counted.
/// A debt outside the pair is bought with token0 at its own price (the fee on top), matching [`close_amounts`]; an
/// unpriced outside debt is skipped, as [`close_amounts`] plans no swap for it either.
///
/// # Errors
///
/// As [`amounts_for_liquidity`]; [`Error::DivisionByZero`] for a zero pair price on a cross-token shortfall or a 100%
/// swap fee.
pub fn estimate_receive(
    position: &ClosingPosition,
    debts: &[ClosingDebt],
    prices: PerToken<Option<Priced>>,
    swap_fee_wad: U256,
) -> Result<(U256, U256)> {
    let p = position;
    let (r0, r1) = amounts_for_liquidity(p.pool_sqrt_price_x96, p.sqrt_pa_x96, p.sqrt_pb_x96, p.liquidity)?;
    let mut b0 = to_i256(r0.try_add(p.fees_owed0)?)?;
    let mut b1 = to_i256(r1.try_add(p.fees_owed1)?)?;
    let convert = |amount: U256, from: Priced, to: Priced| -> Result<I256> {
        let usd = from.usd(amount)?;
        let amount = usd
            .try_mul(pow10(to.decimals)?)?
            .try_mul(crate::WAD)?
            .try_div(to.price_wad)?;
        to_i256(amount.try_div(crate::WAD.try_sub(swap_fee_wad)?)?)
    };
    let priced = |price: Option<Priced>| price.filter(|m| !m.price_wad.is_zero());
    for debt in debts {
        match (debt.side, priced(debt.price), priced(prices.token0)) {
            (Some(Side::Token0), _, _) => b0 = b0.try_sub(to_i256(debt.assets)?)?,
            (Some(Side::Token1), _, _) => b1 = b1.try_sub(to_i256(debt.assets)?)?,
            (None, Some(from), Some(to)) => b0 = b0.try_sub(convert(debt.assets, from, to)?)?,
            (None, _, _) => {}
        }
    }
    let clamp = |b: I256| if b.is_negative() { U256::ZERO } else { b.into_raw() };
    let (Some(m0), Some(m1)) = (prices.token0, prices.token1) else {
        return Ok((clamp(b0), clamp(b1)));
    };
    if b0.is_negative() {
        b1 = b1.try_sub(convert(b0.unsigned_abs(), m0, m1)?)?;
        b0 = I256::ZERO;
    }
    if b1.is_negative() {
        b0 = b0.try_sub(convert(b1.unsigned_abs(), m1, m0)?)?;
        b1 = I256::ZERO;
    }
    Ok((clamp(b0), clamp(b1)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uniswap::{Q96, sqrt_price_at_tick};

    const fn pair<T>(token0: T, token1: T) -> PerToken<T> {
        PerToken { token0, token1 }
    }

    const fn usd(price: u64, decimals: u8) -> Priced {
        Priced {
            price_wad: U256::from_limbs([price, 0, 0, 0]),
            decimals,
        }
    }

    #[test]
    fn a_stale_price_is_valued_at_the_pool() {
        // 1 token1 per token0, token1 at $2: token0 is $2 too
        let live = MarketPrice {
            price_wad: U256::from(2_000_000_000_000_000_000_u64),
            decimals: 18,
            last_close: false,
        };
        let stale = MarketPrice {
            last_close: true,
            ..live
        };
        let q96 = U160::from(Q96);
        assert_eq!(
            pool_valued_prices(stale, live, q96).unwrap(),
            (live.price_wad, live.price_wad)
        );
        assert_eq!(
            pool_valued_prices(live, stale, q96).unwrap(),
            (live.price_wad, live.price_wad)
        );
        assert_eq!(
            pool_valued_prices(stale, stale, q96).unwrap(),
            (live.price_wad, live.price_wad)
        );
    }

    #[test]
    fn a_closed_market_is_priced_at_the_pool() {
        // ETH (18 decimals) at 2,000 USDG (6 decimals) in the pool, USDG live at $1, ETH's feed closed
        let tick = crate::uniswap::tick_at_sqrt_price(U160::from(3_543_191_142_285_914_205_922_034_u128)).unwrap();
        let sqrt = sqrt_price_at_tick(tick).unwrap();
        let usdg = MarketPrice {
            price_wad: crate::WAD,
            decimals: 6,
            last_close: false,
        };
        let eth = MarketPrice {
            price_wad: U256::ZERO,
            decimals: 18,
            last_close: false,
        };
        let (price0, price1) = pool_valued_prices(eth, usdg, sqrt).unwrap();
        assert_eq!(price1, crate::WAD);
        let dollars = price0 / crate::WAD;
        assert!((1_999..=2_000).contains(&dollars.to::<u64>()), "{price0}");
    }

    #[test]
    fn a_held_debt_needs_no_swap() {
        let swaps = swap_into_debts(
            pair(U256::from(2_000), U256::ZERO),
            pair(U256::from(1_000), U256::ZERO),
            pair(usd(1, 0), usd(1, 0)),
            50,
            U256::ZERO,
        )
        .unwrap();
        assert_eq!(swaps, Vec::<DebtSwap>::new());
    }

    #[test]
    fn a_shortfall_sells_the_other_token() {
        let swaps = swap_into_debts(
            pair(U256::ZERO, U256::from(5_000)),
            pair(U256::from(1_000), U256::ZERO),
            pair(usd(1, 0), usd(1, 0)),
            0,
            U256::ZERO,
        )
        .unwrap();
        assert_eq!(
            swaps,
            vec![DebtSwap {
                sell: Side::Token1,
                amount_in: U256::from(1_030),
                min_out: U256::from(1_000)
            }]
        );
    }

    #[test]
    fn a_close_swaps_only_for_what_the_redeem_misses() {
        let position = ClosingPosition {
            sqrt_pa_x96: sqrt_price_at_tick(-600).unwrap(),
            sqrt_pb_x96: sqrt_price_at_tick(600).unwrap(),
            liquidity: 10_u128.pow(21),
            pool_sqrt_price_x96: sqrt_price_at_tick(0).unwrap(),
            ..ClosingPosition::default()
        };
        let price = Some(usd(1_000_000_000_000_000_000, 18));
        let debt = |side, assets| ClosingDebt {
            side: Some(side),
            assets: U256::from(assets),
            price,
        };
        let small = close_amounts(
            &position,
            &[debt(Side::Token0, U256::from(1_000))],
            pair(price, price),
            50,
            U256::ZERO,
        )
        .unwrap();
        assert_eq!(small.swaps, Vec::<CloseSwap>::new());
        let big = U256::from(10).pow(U256::from(20));
        let large = close_amounts(
            &position,
            &[debt(Side::Token0, big)],
            pair(price, price),
            50,
            U256::ZERO,
        )
        .unwrap();
        assert_eq!(large.swaps.len(), 1);
        assert_eq!(large.swaps[0].sell, Side::Token1);
        assert_eq!(large.swaps[0].top_up, large.swaps[0].max_in - large.swaps[0].exact_in);
        assert!(!large.swaps[0].exact_in.is_zero());
    }

    #[test]
    fn a_debt_the_redeem_covers_now_has_no_fixed_swap() {
        // A narrow range pays about half of each token now, and none of one at either end of the slippage band
        let position = ClosingPosition {
            sqrt_pa_x96: sqrt_price_at_tick(-60).unwrap(),
            sqrt_pb_x96: sqrt_price_at_tick(60).unwrap(),
            liquidity: 10_u128.pow(21),
            pool_sqrt_price_x96: sqrt_price_at_tick(0).unwrap(),
            ..ClosingPosition::default()
        };
        let price = Some(usd(1_000_000_000_000_000_000, 18));
        let (now0, now1) = amounts_for_liquidity(
            position.pool_sqrt_price_x96,
            position.sqrt_pa_x96,
            position.sqrt_pb_x96,
            position.liquidity,
        )
        .unwrap();
        let debt = |side, assets: U256| ClosingDebt {
            side: Some(side),
            assets,
            price,
        };
        let debts = [
            debt(Side::Token0, now0 * U256::from(8) / U256::from(10)),
            debt(Side::Token1, now1 * U256::from(8) / U256::from(10)),
        ];
        let amounts = close_amounts(&position, &debts, pair(price, price), 50, U256::ZERO).unwrap();
        assert_eq!(amounts.swaps.len(), 2);
        for swap in &amounts.swaps {
            assert!(swap.exact_in.is_zero());
            assert_eq!(swap.top_up, swap.max_in);
        }
    }

    #[test]
    fn outside_pair_debt_is_deducted_from_the_token_close_amounts_sells() {
        let position = ClosingPosition {
            sqrt_pa_x96: sqrt_price_at_tick(-600).unwrap(),
            sqrt_pb_x96: sqrt_price_at_tick(600).unwrap(),
            liquidity: 10_u128.pow(21),
            pool_sqrt_price_x96: sqrt_price_at_tick(0).unwrap(),
            ..ClosingPosition::default()
        };
        let p0 = Some(usd(2_000_000_000_000_000_000, 18)); // $2
        let p1 = Some(usd(1_000_000_000_000_000_000, 18)); // $1
        let prices = pair(p0, p1);
        // 10 units of an outside 6-decimal token at $1 ($10 total) -> 5 units of token0 at $2
        let outside = ClosingDebt {
            side: None,
            assets: U256::from(10_000_000_u64),
            price: Some(usd(1_000_000_000_000_000_000, 6)),
        };

        let plan = close_amounts(&position, &[outside], prices, 50, U256::ZERO).unwrap();
        assert_eq!(plan.swaps.len(), 1);
        assert_eq!(plan.swaps[0].sell, Side::Token0);

        let (free0, free1) = estimate_receive(&position, &[], prices, U256::ZERO).unwrap();
        let (recv0, recv1) = estimate_receive(&position, &[outside], prices, U256::ZERO).unwrap();
        assert_eq!(free0 - recv0, U256::from(5_000_000_000_000_000_000_u64));
        assert_eq!(recv1, free1);

        // Unpriced outside debt: close_amounts plans no swap and estimate_receive skips it too
        let unpriced = ClosingDebt { price: None, ..outside };
        let planned = close_amounts(&position, &[unpriced], prices, 50, U256::ZERO).unwrap();
        assert_eq!(planned.swaps, Vec::<CloseSwap>::new());
        assert_eq!(
            estimate_receive(&position, &[unpriced], prices, U256::ZERO).unwrap(),
            (free0, free1)
        );
    }
}
