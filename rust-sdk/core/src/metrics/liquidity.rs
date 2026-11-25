//! A pool's liquidity as charts and books show it: bars per tick bucket, the depth a swap walks through, and what
//! each price level holds. `f64` in raw units of liquidity and ticks, for display.

/// The most bars one call returns.
const MAX_BARS: usize = 4_000;
/// Each tick moves the price by this factor.
const TICK_BASE: f64 = 1.0001;

/// Liquidity active between two ticks.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Segment {
    /// The lower tick.
    pub lower: f64,
    /// The upper tick.
    pub upper: f64,
    /// Its liquidity.
    pub liquidity: f64,
}

/// A tick's raw sqrt price (token1 base units per token0 base unit).
#[must_use]
pub fn sqrt_at_tick(tick: f64) -> f64 {
    TICK_BASE.powf(tick / 2.0)
}

/// Token0 (raw units) liquidity `l` holds between raw sqrt prices `a` < `b`.
fn amount0_between(l: f64, a: f64, b: f64) -> f64 {
    l * (1.0 / a - 1.0 / b)
}

/// Token1 (raw units) liquidity `l` holds between raw sqrt prices `a` < `b`.
fn amount1_between(l: f64, a: f64, b: f64) -> f64 {
    l * (b - a)
}

fn scale(decimals: u8) -> f64 {
    10_f64.powi(i32::from(decimals))
}

/// One bucket of ticks.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LiquidityBar {
    /// Its lower tick.
    pub tick_lower: f64,
    /// Its upper tick.
    pub tick_upper: f64,
    /// Its average liquidity.
    pub liquidity: f64,
    /// Token0 it holds at the price now (whole tokens).
    pub amount0: f64,
    /// Token1 it holds.
    pub amount1: f64,
    /// The price is in it.
    pub active: bool,
}

/// The pool's liquidity in buckets of `bucket` ticks from `from_tick` to `to_tick` (at most 4,000), each with what it
/// holds at `tick`. `segments` ascend and do not overlap.
#[must_use]
pub fn liquidity_bars(
    segments: &[Segment],
    tick: f64,
    bucket: f64,
    from_tick: f64,
    to_tick: f64,
    decimals0: u8,
    decimals1: u8,
) -> Vec<LiquidityBar> {
    let mut bars = Vec::new();
    if !(bucket > 0.0 && bucket.is_finite() && from_tick.is_finite() && to_tick.is_finite()) {
        return bars;
    }
    let sqrt_now = sqrt_at_tick(tick);
    let scale0 = scale(decimals0);
    let scale1 = scale(decimals1);
    let mut first = 0;
    let mut lo = (from_tick / bucket).floor() * bucket;
    let mut sqrt_lo = sqrt_at_tick(lo);
    while lo < to_tick && bars.len() < MAX_BARS {
        let hi = lo + bucket;
        let sqrt_hi = sqrt_at_tick(hi);
        while segments.get(first).is_some_and(|s| s.upper <= lo) {
            first = first.saturating_add(1);
        }
        let (mut weighted, mut amount0, mut amount1) = (0.0, 0.0, 0.0);
        for s in segments.iter().skip(first).take_while(|s| s.lower < hi) {
            let from = s.lower.max(lo);
            let to = s.upper.min(hi);
            if to <= from {
                continue;
            }
            let l = s.liquidity;
            weighted += l * (to - from);
            let sa = if s.lower <= lo { sqrt_lo } else { sqrt_at_tick(from) };
            let sb = if s.upper >= hi { sqrt_hi } else { sqrt_at_tick(to) };
            if sb <= sqrt_now {
                amount1 += amount1_between(l, sa, sb);
            } else if sa >= sqrt_now {
                amount0 += amount0_between(l, sa, sb);
            } else {
                amount1 += amount1_between(l, sa, sqrt_now);
                amount0 += amount0_between(l, sqrt_now, sb);
            }
        }
        bars.push(LiquidityBar {
            tick_lower: lo,
            tick_upper: hi,
            liquidity: weighted / bucket,
            amount0: amount0 / scale0,
            amount1: amount1 / scale1,
            active: lo <= tick && tick < hi,
        });
        lo = hi;
        sqrt_lo = sqrt_hi;
    }
    bars
}

/// What a swap from the price now has moved through by a tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DepthPoint {
    /// The tick reached.
    pub tick: f64,
    /// Token0 that changed hands (whole tokens, positive).
    pub amount0: f64,
    /// Token1.
    pub amount1: f64,
}

/// A walk out from the price now.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DepthWalk {
    /// The raw sqrt price now.
    pub sqrt_now: f64,
    /// The tick now.
    pub tick_now: f64,
    /// Upward (token1 in) or downward.
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

/// The tokens a swap moving the price `walk.max_ticks` up or down trades, every `walk.resolution` ticks.
#[must_use]
#[expect(clippy::float_cmp, reason = "ticks are whole numbers, exact in f64")]
pub fn depth_walk(segments: &[Segment], walk: &DepthWalk) -> Vec<DepthPoint> {
    let w = walk;
    let mut points = vec![DepthPoint {
        tick: w.tick_now,
        amount0: 0.0,
        amount1: 0.0,
    }];
    if !(w.resolution > 0.0 && w.resolution.is_finite() && w.max_ticks >= 0.0 && w.max_ticks.is_finite()) {
        return points;
    }
    let sign = if w.up { 1.0 } else { -1.0 };
    let end = w.tick_now + sign * w.max_ticks;
    let scale0 = scale(w.decimals0);
    let scale1 = scale(w.decimals1);
    let (mut amount0, mut amount1) = (0.0, 0.0);
    let (mut sqrt, mut at) = (w.sqrt_now, w.tick_now);
    let mut walk_to = |l: f64, edge: f64, points: &mut Vec<DepthPoint>| {
        while sign * (edge - at) > 0.0 {
            let next = if w.up {
                edge.min(at + w.resolution)
            } else {
                edge.max(at - w.resolution)
            };
            let sqrt_next = sqrt_at_tick(next);
            let (a, b) = if w.up { (sqrt, sqrt_next) } else { (sqrt_next, sqrt) };
            if b > a && l > 0.0 {
                amount0 += amount0_between(l, a, b) / scale0;
                amount1 += amount1_between(l, a, b) / scale1;
            }
            sqrt = sqrt_next;
            at = next;
            points.push(DepthPoint {
                tick: at,
                amount0,
                amount1,
            });
        }
    };
    if w.up {
        for s in segments {
            if s.upper <= w.tick_now {
                continue;
            }
            if s.lower >= end {
                break;
            }
            walk_to(0.0, s.lower.min(end), &mut points);
            walk_to(s.liquidity, s.upper.min(end), &mut points);
        }
    } else {
        for s in segments.iter().rev() {
            if s.lower >= w.tick_now {
                continue;
            }
            if s.upper <= end {
                break;
            }
            walk_to(0.0, s.upper.max(end), &mut points);
            walk_to(s.liquidity, s.lower.max(end), &mut points);
        }
    }
    if at != end {
        points.push(DepthPoint {
            tick: end,
            amount0,
            amount1,
        });
    }
    points
}

/// `segments` with their ticks as raw sqrt prices, for [`book_amounts`]: work out once per curve.
#[must_use]
pub fn segment_sqrts(segments: &[Segment]) -> Vec<Segment> {
    segments
        .iter()
        .map(|s| Segment {
            lower: sqrt_at_tick(s.lower),
            upper: sqrt_at_tick(s.upper),
            liquidity: s.liquidity,
        })
        .collect()
}

/// What the pool holds between consecutive raw sqrt prices walking out from `sqrt_now` (whole tokens): `edges` ordered
/// away from it. An order book's levels: above the price only token0, below only token1. `sqrt_segments` from
/// [`segment_sqrts`].
#[must_use]
pub fn book_amounts(
    sqrt_segments: &[Segment],
    sqrt_now: f64,
    edges: &[f64],
    decimals0: u8,
    decimals1: u8,
) -> Vec<(f64, f64)> {
    let scale0 = scale(decimals0);
    let scale1 = scale(decimals1);
    let sum_slice = |slice: &[Segment], lo: f64, hi: f64| -> (f64, f64) {
        let (mut amount0, mut amount1) = (0.0, 0.0);
        for s in slice {
            let a = s.lower.max(lo);
            let b = s.upper.min(hi);
            if b <= a || s.liquidity <= 0.0 {
                continue;
            }
            amount0 += amount0_between(s.liquidity, a, b);
            amount1 += amount1_between(s.liquidity, a, b);
        }
        (amount0 / scale0, amount1 / scale1)
    };

    let ascending =
        edges.first().is_none_or(|&e| e >= sqrt_now) && edges.windows(2).all(|w| matches!(w, [a, b] if a <= b));
    if ascending {
        let mut out = Vec::with_capacity(edges.len());
        let mut first = sqrt_segments.partition_point(|s| s.upper <= sqrt_now);
        let mut from = sqrt_now;
        for &edge in edges {
            let (lo, hi) = (from, edge);
            from = edge;
            while sqrt_segments.get(first).is_some_and(|s| s.upper <= lo) {
                first = first.saturating_add(1);
            }
            let tail = sqrt_segments.get(first..).unwrap_or_default();
            let count = tail.iter().take_while(|s| s.lower < hi).count();
            out.push(sum_slice(tail.get(..count).unwrap_or_default(), lo, hi));
        }
        return out;
    }

    let descending =
        edges.first().is_none_or(|&e| e <= sqrt_now) && edges.windows(2).all(|w| matches!(w, [a, b] if a >= b));
    if descending {
        let mut out = Vec::with_capacity(edges.len());
        let mut end = sqrt_segments.partition_point(|s| s.lower < sqrt_now);
        let mut from = sqrt_now;
        for &edge in edges {
            let (lo, hi) = (edge, from);
            from = edge;
            while let Some(prev) = end.checked_sub(1) {
                if sqrt_segments.get(prev).is_some_and(|s| s.lower >= hi) {
                    end = prev;
                } else {
                    break;
                }
            }
            let head = sqrt_segments.get(..end).unwrap_or_default();
            let mut start = end;
            while let Some(prev) = start.checked_sub(1) {
                if head.get(prev).is_some_and(|s| s.upper > lo) {
                    start = prev;
                } else {
                    break;
                }
            }
            out.push(sum_slice(head.get(start..).unwrap_or_default(), lo, hi));
        }
        return out;
    }

    let mut from = sqrt_now;
    edges
        .iter()
        .map(|&edge| {
            let (lo, hi) = (from.min(edge), from.max(edge));
            from = edge;
            sum_slice(sqrt_segments, lo, hi)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEGMENTS: [Segment; 2] = [
        Segment {
            lower: -600.0,
            upper: 0.0,
            liquidity: 1e18,
        },
        Segment {
            lower: 0.0,
            upper: 600.0,
            liquidity: 2e18,
        },
    ];

    #[test]
    fn bars_split_at_the_price() {
        let bars = liquidity_bars(&SEGMENTS, 0.0, 300.0, -600.0, 600.0, 18, 18);
        assert_eq!(bars.len(), 4);
        assert!(bars[1].amount0 == 0.0 && bars[1].amount1 > 0.0 && !bars[1].active);
        assert!(bars[2].active && bars[2].amount0 > 0.0 && bars[2].amount1 == 0.0);
        assert!((bars[3].liquidity - 2e18).abs() < 1.0);
    }

    #[test]
    fn a_walk_up_trades_only_through_liquidity_above() {
        let walk = DepthWalk {
            sqrt_now: 1.0,
            tick_now: 0.0,
            up: true,
            max_ticks: 1_000.0,
            resolution: 100.0,
            decimals0: 18,
            decimals1: 18,
        };
        let points = depth_walk(&SEGMENTS, &walk);
        assert_eq!(points.last().map(|p| p.tick), Some(1_000.0));
        assert!((points[6].amount0 - points.last().unwrap().amount0).abs() < f64::EPSILON);
    }

    #[test]
    fn the_book_matches_the_bars() {
        let sqrts = segment_sqrts(&SEGMENTS);
        let book = book_amounts(&sqrts, 1.0, &[sqrt_at_tick(300.0), sqrt_at_tick(600.0)], 18, 6);
        let bars = liquidity_bars(&SEGMENTS, 0.0, 300.0, 0.0, 600.0, 18, 6);
        assert!((book[0].0 - bars[0].amount0).abs() < 1e-9 && (book[1].0 - bars[1].amount0).abs() < 1e-9);
    }

    #[test]
    fn non_positive_step_does_not_loop() {
        assert_eq!(
            liquidity_bars(&SEGMENTS, 0.0, 0.0, -600.0, 600.0, 18, 18),
            Vec::<LiquidityBar>::new()
        );
        assert_eq!(
            liquidity_bars(&SEGMENTS, 0.0, -10.0, -600.0, 600.0, 18, 18),
            Vec::<LiquidityBar>::new()
        );
        let walk = DepthWalk {
            sqrt_now: 1.0,
            tick_now: 0.0,
            up: true,
            max_ticks: 1_000.0,
            resolution: 0.0,
            decimals0: 18,
            decimals1: 18,
        };
        assert_eq!(depth_walk(&SEGMENTS, &walk).len(), 1);
    }
}
