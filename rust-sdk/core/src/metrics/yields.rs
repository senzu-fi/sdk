//! Yields: leverage, average and yearly rates, a market's interest flows, what a range earns in fees, and a
//! position's net APR.
//!
//! The rates from on-chain numbers are exact (1e18 = 100%). The fee APRs are `f64`: the indexer's fee volumes are
//! floats.

use alloy_primitives::{I256, U160, U256};

use crate::error::Result;
use crate::num::{Checked, WAD, to_i256};

const DAYS_PER_YEAR: f64 = 365.0;
const SECONDS_PER_DAY: f64 = 86_400.0;

/// Size over margin (1e18 = 1x), rounded down; zero without margin (equity at or under zero).
///
/// # Errors
///
/// [`crate::Error::Overflow`] on an absurd size.
pub fn leverage_wad(size_usd_wad: U256, equity_usd_wad: I256) -> Result<U256> {
    if !equity_usd_wad.is_positive() {
        return Ok(U256::ZERO);
    }
    size_usd_wad.try_mul(WAD)?.try_div(equity_usd_wad.into_raw())
}

/// A rate and the value it applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct ValuedRate {
    /// The value, in USD (1e18).
    pub value_usd_wad: U256,
    /// Its yearly rate (1e18 = 100%).
    pub apr_wad: U256,
}

/// The value-weighted average of `rates`, rounded down; zero without value.
///
/// # Errors
///
/// [`crate::Error::Overflow`] on absurd values.
pub fn average_apr(rates: &[ValuedRate]) -> Result<U256> {
    let mut total = U256::ZERO;
    let mut weighted = U256::ZERO;
    for r in rates {
        total = total.try_add(r.value_usd_wad)?;
        weighted = weighted.try_add(r.apr_wad.try_mul(r.value_usd_wad)?)?;
    }
    if total.is_zero() {
        return Ok(U256::ZERO);
    }
    weighted.try_div(total)
}

/// What `amount` earns in a year at `apr_wad`, rounded down.
///
/// # Errors
///
/// [`crate::Error::Overflow`] on absurd values.
pub fn yearly(amount: U256, apr_wad: U256) -> Result<U256> {
    amount.try_mul(apr_wad)?.try_div(WAD)
}

/// `part` of `total` (1e18 = all), rounded down; zero of nothing.
///
/// # Errors
///
/// [`crate::Error::Overflow`] on an absurd part.
pub fn share_wad(part: U256, total: U256) -> Result<U256> {
    if total.is_zero() {
        return Ok(U256::ZERO);
    }
    part.try_mul(WAD)?.try_div(total)
}

/// Assets per this many lender shares, as the indexer books a lender's rate.
pub const LENDER_RATE_SCALE: U256 = crate::metrics::rates::WAD_SQUARED;

/// A lender's interest to date, in asset units: what the indexer booked, plus the indexed shares' gain from the
/// booked rate to the live one (`live_assets` over `live_shares`; the booked rate without live shares). Negative after
/// bad debt; rounded toward zero.
///
/// # Errors
///
/// [`crate::Error::Overflow`] on absurd values.
pub fn lender_earned(
    earned_assets: I256,
    shares: U256,
    last_rate: U256,
    live_assets: U256,
    live_shares: U256,
) -> Result<I256> {
    let rate = if live_shares.is_zero() {
        last_rate
    } else {
        live_assets.try_mul(LENDER_RATE_SCALE)?.try_div(live_shares)?
    };
    let gain = to_i256(rate)?.try_sub(to_i256(last_rate)?)?;
    earned_assets.try_add(to_i256(shares)?.try_mul(gain)?.try_div(to_i256(LENDER_RATE_SCALE)?)?)
}

/// A market's interest over a year at today's borrows and rate, in its asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct MarketFlows {
    /// What borrowers pay.
    pub interest: U256,
    /// The reserves' share of it.
    pub reserves: U256,
    /// The lenders' share.
    pub lenders: U256,
}

/// A market's interest over a year, split between the reserves and the lenders.
///
/// # Errors
///
/// [`crate::Error::Overflow`] on absurd values.
pub fn market_flows(borrow_assets: U256, borrow_apr_wad: U256, reserve_factor_wad: U256) -> Result<MarketFlows> {
    let interest = yearly(borrow_assets, borrow_apr_wad)?;
    let reserves = interest.try_mul(reserve_factor_wad)?.try_div(WAD)?;
    Ok(MarketFlows {
        interest,
        reserves,
        lenders: interest.try_sub(reserves)?,
    })
}

use super::to_f64;

fn usd(wad: U256) -> f64 {
    to_f64(wad) / 1e18
}

/// A pool's swap fees over a year: the fees seen over `window_seconds` (a day's worth, `fees_24h_usd`) scaled to a
/// year, times what liquidity providers keep (1 on Uniswap; Aero's unstaked share).
#[must_use]
pub fn fees_per_year(fees_24h_usd: f64, window_seconds: f64, keep: f64) -> f64 {
    (fees_24h_usd * DAYS_PER_YEAR * SECONDS_PER_DAY * keep) / window_seconds
}

/// What a Senzu holder keeps of a pool's swap fees or a gauge's rewards: the DEX's share for liquidity providers
/// (`keep`, 1 on Uniswap) less the protocol's cut (`cut_wad`, 1e18 = 100%; [`crate::fees::cut`]).
#[must_use]
pub fn holder_keep(keep: f64, cut_wad: U256) -> f64 {
    keep * (1.0 - usd(cut_wad))
}

/// What a new position's range earns in fees a year over its size: the pool's yearly fees times its share of the
/// liquidity once added. `None` without size or liquidity.
#[must_use]
pub fn range_fee_apr(fees_per_year_usd: f64, liquidity: U256, pool_liquidity: U256, size_usd: f64) -> Option<f64> {
    if size_usd <= 0.0 || liquidity.is_zero() {
        return None;
    }
    let share = to_f64(liquidity) / (to_f64(pool_liquidity) + to_f64(liquidity));
    Some((fees_per_year_usd * share) / size_usd)
}

/// Whether the pool price is in the range `[a, b)`.
fn in_range(sqrt_price_x96: U160, sqrt_pa_x96: U160, sqrt_pb_x96: U160) -> bool {
    sqrt_price_x96 >= sqrt_pa_x96 && sqrt_price_x96 < sqrt_pb_x96
}

/// What a position earns and pays a year.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct PositionAprInput {
    /// Its size (USD, 1e18).
    pub size_usd_wad: U256,
    /// Its equity.
    pub equity_usd_wad: I256,
    /// Its debt.
    pub debt_usd_wad: U256,
    /// The debt's average yearly rate ([`average_apr`]).
    pub borrow_apr_wad: U256,
    /// Its liquidity.
    pub liquidity: U256,
    /// The pool's active liquidity, its own included.
    pub pool_liquidity: U256,
    /// The pool price now.
    pub sqrt_price_x96: U160,
    /// One end of the range.
    pub sqrt_pa_x96: U160,
    /// The other end.
    pub sqrt_pb_x96: U160,
    /// Staked in a gauge: earns rewards, not swap fees.
    pub staked: bool,
    /// The pool's swap fees a year, in USD ([`fees_per_year`]).
    pub fees_per_year_usd: f64,
    /// Staked: the rewards its liquidity earns a year in range, in USD; `None` while they cannot be priced.
    pub rewards_per_year_usd: Option<f64>,
}

/// A position's yearly rates.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct PositionApr {
    /// Fees and rewards less interest, over the margin.
    pub net: f64,
    /// Fees over the size.
    pub fees: f64,
    /// Rewards over the size; `None` while they cannot be priced.
    pub rewards: Option<f64>,
    /// The debt's yearly rate.
    pub borrow: f64,
}

/// Rewards a year: none unstaked or out of range.
fn rewards_in_range(staked: bool, in_range: bool, rewards_per_year_usd: Option<f64>) -> Option<f64> {
    if staked && in_range {
        rewards_per_year_usd
    } else {
        Some(0.0)
    }
}

/// A position's yearly rates: its share of the pool's fees while in range (none staked), its rewards, its interest.
/// `None` without size or margin.
#[must_use]
pub fn position_apr(input: &PositionAprInput) -> Option<PositionApr> {
    let i = input;
    let size = usd(i.size_usd_wad);
    let margin = if i.equity_usd_wad.is_positive() {
        usd(i.equity_usd_wad.into_raw())
    } else {
        0.0
    };
    if size <= 0.0 || margin <= 0.0 {
        return None;
    }
    let inside = in_range(i.sqrt_price_x96, i.sqrt_pa_x96, i.sqrt_pb_x96);
    let share = if inside && !i.pool_liquidity.is_zero() {
        to_f64(i.liquidity) / to_f64(i.pool_liquidity)
    } else {
        0.0
    };
    let fees_usd = if i.staked {
        0.0
    } else {
        i.fees_per_year_usd * share.min(1.0)
    };
    let rewards_usd = rewards_in_range(i.staked, inside, i.rewards_per_year_usd);
    let debt_usd = usd(i.debt_usd_wad);
    let borrow = usd(i.borrow_apr_wad);
    Some(PositionApr {
        net: (fees_usd + rewards_usd.unwrap_or(0.0) - borrow * debt_usd) / margin,
        fees: fees_usd / size,
        rewards: rewards_usd.map(|r| r / size),
        borrow,
    })
}

/// A position after an edit, to rate.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct EditAprInput {
    /// Its margin after the edit (USD, 1e18).
    pub margin_usd_wad: I256,
    /// Its debt after.
    pub debt_usd_wad: U256,
    /// Its liquidity after.
    pub liquidity: U256,
    /// Its liquidity now (in the pool's active liquidity).
    pub liquidity_now: U256,
    /// The pool's active liquidity now.
    pub pool_liquidity: U256,
    /// The pool price now.
    pub sqrt_price_x96: U160,
    /// One end of the range.
    pub sqrt_pa_x96: U160,
    /// The other end.
    pub sqrt_pb_x96: U160,
    /// Staked in a gauge.
    pub staked: bool,
    /// The pool's swap fees a year, in USD.
    pub fees_per_year_usd: f64,
    /// Staked: the rewards the liquidity after the edit earns a year in range; `None` while unpriced (counted as 0).
    pub rewards_per_year_usd: Option<f64>,
    /// The debt's average yearly rate.
    pub borrow_apr_wad: U256,
}

/// The position's net yearly rate after an edit: fees on its new share of the pool, rewards, less interest, over the
/// new margin. `None` without margin.
#[must_use]
pub fn edit_apr(input: &EditAprInput) -> Option<f64> {
    let i = input;
    let margin = if i.margin_usd_wad.is_negative() {
        -usd(i.margin_usd_wad.unsigned_abs())
    } else {
        usd(i.margin_usd_wad.into_raw())
    };
    if margin <= 0.0 {
        return None;
    }
    let inside = in_range(i.sqrt_price_x96, i.sqrt_pa_x96, i.sqrt_pb_x96);
    let pool = i
        .pool_liquidity
        .saturating_add(i.liquidity)
        .saturating_sub(i.liquidity_now);
    let share = if inside && !pool.is_zero() && !i.staked {
        to_f64(i.liquidity) / to_f64(pool)
    } else {
        0.0
    };
    let rewards = rewards_in_range(i.staked, inside, i.rewards_per_year_usd).unwrap_or(0.0);
    let borrow = usd(i.borrow_apr_wad);
    Some((i.fees_per_year_usd * share.min(1.0) + rewards - borrow * usd(i.debt_usd_wad)) / margin)
}

/// A new position's earnings, projected.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct OpenEarningsInput {
    /// The range's fee APR ([`range_fee_apr`]).
    pub fee_apr: f64,
    /// The position's size (USD, 1e18).
    pub size_usd_wad: U256,
    /// Its margin.
    pub margin_usd_wad: U256,
    /// Its debt.
    pub debt_usd_wad: U256,
    /// The debt's average yearly rate.
    pub borrow_apr_wad: U256,
    /// The opening fee (1e18 = 100%).
    pub borrow_fee_wad: U256,
    /// Days held.
    pub days: f64,
}

/// What a new position would earn and pay over some days (USD).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct OpenEarnings {
    /// Fees a day.
    pub fees_per_day: f64,
    /// Interest a day.
    pub interest_per_day: f64,
    /// Fees less interest a day.
    pub net_per_day: f64,
    /// Fees over the days.
    pub fees: f64,
    /// Interest over the days.
    pub interest: f64,
    /// Paid once at the open, on what is borrowed (rounded up, as the contract charges it).
    pub opening_fee: f64,
    /// Fees less interest and the opening fee.
    pub net: f64,
    /// `net` over the margin.
    pub roi: f64,
}

/// A new position's projected earnings; `None` without margin.
///
/// # Errors
///
/// [`crate::Error::Overflow`] on an absurd debt.
pub fn open_earnings(input: &OpenEarningsInput) -> Result<Option<OpenEarnings>> {
    let i = input;
    let margin = usd(i.margin_usd_wad);
    if margin <= 0.0 {
        return Ok(None);
    }
    let size = usd(i.size_usd_wad);
    let debt = usd(i.debt_usd_wad);
    let borrow = usd(i.borrow_apr_wad);
    let fees_per_day = (i.fee_apr * size) / DAYS_PER_YEAR;
    let interest_per_day = (borrow * debt) / DAYS_PER_YEAR;
    let fees = fees_per_day * i.days;
    let interest = interest_per_day * i.days;
    let opening_fee = usd(crate::market::borrow_fee(i.debt_usd_wad, i.borrow_fee_wad)?);
    let net = fees - interest - opening_fee;
    Ok(Some(OpenEarnings {
        fees_per_day,
        interest_per_day,
        net_per_day: fees_per_day - interest_per_day,
        fees,
        interest,
        opening_fee,
        net,
        roi: net / margin,
    }))
}

/// A new position's yearly net rate on `margin_usd_wad`: fees at `fee_apr` on the size less interest on the debt.
/// `None` without margin.
#[must_use]
pub fn open_net_apr(
    fee_apr: f64,
    size_usd_wad: U256,
    debt_usd_wad: U256,
    borrow_apr_wad: U256,
    margin_usd_wad: U256,
) -> Option<f64> {
    let margin = usd(margin_usd_wad);
    if margin <= 0.0 {
        return None;
    }
    Some((fee_apr * usd(size_usd_wad) - usd(borrow_apr_wad) * usd(debt_usd_wad)) / margin)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wad(x: u64) -> U256 {
        U256::from(x).try_mul(WAD).unwrap()
    }

    #[test]
    fn leverage_is_size_over_margin() {
        let equity = I256::from_raw(wad(250));
        assert_eq!(leverage_wad(wad(1_000), equity).unwrap(), wad(4));
        assert_eq!(leverage_wad(wad(1_000), I256::ZERO).unwrap(), U256::ZERO);
    }

    #[test]
    fn the_average_weighs_by_value() {
        let rates = [
            ValuedRate {
                value_usd_wad: wad(300),
                apr_wad: wad(1) / U256::from(10_u8),
            },
            ValuedRate {
                value_usd_wad: wad(100),
                apr_wad: wad(1) / U256::from(2_u8),
            },
        ];
        assert_eq!(average_apr(&rates).unwrap(), wad(1) / U256::from(5_u8));
        assert_eq!(average_apr(&[]).unwrap(), U256::ZERO);
        assert_eq!(share_wad(wad(1), wad(4)).unwrap(), wad(1) / U256::from(4_u8));
    }

    #[test]
    fn a_lender_earns_the_rate_gain_on_its_shares() {
        let scale = LENDER_RATE_SCALE;
        let earned = lender_earned(I256::from_raw(wad(5)), wad(100), scale, wad(110), wad(100)).unwrap();
        assert_eq!(earned, I256::from_raw(wad(15)));
        let bad_debt = lender_earned(I256::ZERO, wad(100), scale, wad(90), wad(100)).unwrap();
        assert_eq!(bad_debt, -I256::from_raw(wad(10)));
        assert_eq!(
            lender_earned(I256::ZERO, wad(100), scale, U256::ZERO, U256::ZERO).unwrap(),
            I256::ZERO
        );
    }

    #[test]
    fn interest_splits_between_reserves_and_lenders() {
        let flows = market_flows(wad(1_000), wad(1) / U256::from(10_u8), wad(1) / U256::from(4_u8)).unwrap();
        assert_eq!(
            (flows.interest, flows.reserves, flows.lenders),
            (wad(100), wad(25), wad(75))
        );
    }

    #[test]
    fn staked_liquidity_earns_rewards_only_in_range() {
        let mut input = PositionAprInput {
            size_usd_wad: wad(1_000),
            equity_usd_wad: I256::from_raw(wad(500)),
            debt_usd_wad: wad(500),
            borrow_apr_wad: wad(1) / U256::from(10_u8),
            liquidity: U256::from(10_u8),
            pool_liquidity: U256::from(100_u8),
            sqrt_price_x96: U160::from(5_u8),
            sqrt_pa_x96: U160::from(1_u8),
            sqrt_pb_x96: U160::from(9_u8),
            staked: true,
            fees_per_year_usd: 10_000.0,
            rewards_per_year_usd: Some(200.0),
        };
        let apr = position_apr(&input).unwrap();
        assert!((apr.net - (200.0 - 50.0) / 500.0).abs() < 1e-12 && apr.fees == 0.0);
        input.sqrt_price_x96 = U160::from(9_u8);
        assert_eq!(position_apr(&input).unwrap().rewards, Some(0.0));
    }
}
