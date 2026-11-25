//! Bindings for [`crate::metrics::payoff`] and [`crate::metrics::liquidity`]. Long series cross as flat
//! `Float64Array`s: one copy each way, no object per point.

use serde::{Deserialize, Serialize};
use tsify::{Ts, Tsify};
use wasm_bindgen::prelude::*;

use super::{input, output};
use crate::metrics::liquidity::{self, DepthWalk, Segment};
use crate::metrics::payoff::{self, PayoffInputs};

/// A payoff's two lines at one price (USD).
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct PayoffPoint {
    /// The position's profit or loss there.
    pub pnl: f64,
    /// Holding what it holds now instead.
    pub hold: f64,
}

/// The position's profit or loss at each of `prices` (fees not included).
///
/// # Errors
///
/// Throws for malformed inputs.
#[wasm_bindgen(js_name = payoffCurve)]
pub fn payoff_curve(inputs: &Ts<PayoffInputs>, prices: &[f64]) -> Result<Vec<f64>, JsError> {
    Ok(payoff::pnl_curve(&input(inputs)?, prices))
}

/// The position's and holding's profit or loss at `price`.
///
/// # Errors
///
/// Throws for malformed inputs.
#[wasm_bindgen(js_name = payoffAt)]
pub fn payoff_at(inputs: &Ts<PayoffInputs>, price: f64) -> Result<Ts<PayoffPoint>, JsError> {
    let i = input(inputs)?;
    output(&PayoffPoint {
        pnl: payoff::pnl_at(&i, price),
        hold: payoff::hold_pnl_at(&i, price),
    })
}

/// Net token0 exposure now, in whole tokens (positive is long).
///
/// # Errors
///
/// Throws for malformed inputs.
#[wasm_bindgen(js_name = payoffExposure0)]
pub fn payoff_exposure0(inputs: &Ts<PayoffInputs>) -> Result<f64, JsError> {
    Ok(payoff::exposure0(&input(inputs)?))
}

/// A range's two ends as pool prices.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct PriceRange {
    /// At its lower tick.
    pub lower: f64,
    /// At its upper tick.
    pub upper: f64,
}

/// The range's ends as pool prices (token1 per token0, whole tokens).
///
/// # Errors
///
/// Throws for malformed inputs.
#[wasm_bindgen(js_name = payoffRange)]
pub fn payoff_range(inputs: &Ts<PayoffInputs>) -> Result<Ts<PriceRange>, JsError> {
    let (lower, upper) = payoff::range_prices(&input(inputs)?);
    output(&PriceRange { lower, upper })
}

/// The same position seen from token1.
///
/// # Errors
///
/// Throws for malformed inputs.
#[wasm_bindgen(js_name = flipPayoff)]
pub fn flip_payoff(inputs: &Ts<PayoffInputs>) -> Result<Ts<PayoffInputs>, JsError> {
    output(&payoff::flip(&input(inputs)?))
}

/// One standard deviation of a day's log return, from hourly closes; undefined with under a day of them.
#[wasm_bindgen(js_name = typicalDailyMove)]
#[must_use]
pub fn typical_daily_move(hourly_closes: &[f64]) -> Option<f64> {
    payoff::typical_daily_move(hourly_closes)
}

/// Segments from `[lower, upper, liquidity, …]`.
fn segments(flat: &[f64]) -> Vec<Segment> {
    let (triples, _) = flat.as_chunks::<3>();
    triples
        .iter()
        .map(|&[lower, upper, liquidity]| Segment {
            lower,
            upper,
            liquidity,
        })
        .collect()
}

/// The pool's liquidity in buckets, from segments flattened as `[lower, upper, liquidity, …]`: flattened as
/// `[tickLower, tickUpper, liquidity, amount0, amount1, active (0 or 1), …]`.
#[wasm_bindgen(js_name = liquidityBars)]
#[must_use]
pub fn liquidity_bars(
    segments_flat: &[f64],
    tick: f64,
    bucket: f64,
    from_tick: f64,
    to_tick: f64,
    decimals0: u8,
    decimals1: u8,
) -> Vec<f64> {
    let bars = liquidity::liquidity_bars(
        &segments(segments_flat),
        tick,
        bucket,
        from_tick,
        to_tick,
        decimals0,
        decimals1,
    );
    bars.iter()
        .flat_map(|b| {
            let active = if b.active { 1.0 } else { 0.0 };
            [b.tick_lower, b.tick_upper, b.liquidity, b.amount0, b.amount1, active]
        })
        .collect()
}

/// A walk out from the price now.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct DepthWalkRequest {
    /// The raw sqrt price now.
    pub sqrt_now: f64,
    /// The tick now.
    pub tick_now: f64,
    /// Upward or downward.
    pub up: bool,
    /// How many ticks to walk.
    pub max_ticks: f64,
    /// Ticks between points.
    pub resolution: f64,
    /// Token0's decimals.
    pub decimals0: u8,
    /// Token1's decimals.
    pub decimals1: u8,
}

/// The tokens a swap moving the price trades, from flattened segments: flattened as `[tick, amount0, amount1, …]`.
///
/// # Errors
///
/// Throws for a malformed walk.
#[wasm_bindgen(js_name = depthWalk)]
pub fn depth_walk(segments_flat: &[f64], walk: &Ts<DepthWalkRequest>) -> Result<Vec<f64>, JsError> {
    let w = input(walk)?;
    let walk = DepthWalk {
        sqrt_now: w.sqrt_now,
        tick_now: w.tick_now,
        up: w.up,
        max_ticks: w.max_ticks,
        resolution: w.resolution,
        decimals0: w.decimals0,
        decimals1: w.decimals1,
    };
    let points = liquidity::depth_walk(&segments(segments_flat), &walk);
    Ok(points.iter().flat_map(|p| [p.tick, p.amount0, p.amount1]).collect())
}

/// Flattened segments with their ticks as raw sqrt prices, for `bookAmounts`: work out once per curve.
#[wasm_bindgen(js_name = segmentSqrts)]
#[must_use]
pub fn segment_sqrts(segments_flat: &[f64]) -> Vec<f64> {
    liquidity::segment_sqrts(&segments(segments_flat))
        .iter()
        .flat_map(|s| [s.lower, s.upper, s.liquidity])
        .collect()
}

/// What the pool holds between consecutive raw sqrt prices from `sqrt_now`, from `segmentSqrts`: flattened as
/// `[amount0, amount1, …]`, one pair per edge.
#[wasm_bindgen(js_name = bookAmounts)]
#[must_use]
pub fn book_amounts(
    sqrt_segments_flat: &[f64],
    sqrt_now: f64,
    edges: &[f64],
    decimals0: u8,
    decimals1: u8,
) -> Vec<f64> {
    liquidity::book_amounts(&segments(sqrt_segments_flat), sqrt_now, edges, decimals0, decimals1)
        .iter()
        .flat_map(|&(amount0, amount1)| [amount0, amount1])
        .collect()
}
