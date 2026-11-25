//! `HealthMath`: the four-corner stress test of one position. Collateral rounds down and debt rounds up.
//!
//! The contract knows no exchange: it asks a position's adapter what its units hold at [`sqrt_prices`] (the
//! adapter's `amountsAt`) and stresses those [`Holdings`] ([`evaluate_holdings`]). [`evaluate`] does both for a
//! range position, as every adapter today reports.

use alloy_primitives::{I256, U160, U256, U512, uint};

use crate::error::{Error, Result};
use crate::num::{Checked, WAD, pow10, to_i256, to_u160, widen};
use crate::types::{CollateralInput, DebtInput, DebtTie, HealthResult, TokenInput};
use crate::uniswap::{MAX_SQRT_PRICE, MIN_SQRT_PRICE, amounts_for_liquidity, mul_div, mul_div_rounding_up, sqrt};

/// Number of stress corners. Bit 0 of a corner stresses token0 up, bit 1 stresses token1 up.
pub const CORNERS: u8 = 4;

/// Number of prices a health check values a position at: the oracle's, then each corner's.
pub const PRICES: usize = 1 + CORNERS as usize;

/// What a position holds at each of [`sqrt_prices`] (index 0 the oracle's price, `1 + corner` each corner's), as
/// its adapter's `amountsAt` reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Holdings {
    /// Token0 held at each price.
    pub amounts0: [U256; PRICES],
    /// Token1 held at each price.
    pub amounts1: [U256; PRICES],
}

/// `floor(U256::MAX / WAD)`: above this collateral, `collateral * WAD / required` can overflow when `required < WAD`.
const MAX_SAFE_HEALTH_COLLATERAL: U256 =
    uint!(115_792_089_237_316_195_423_570_985_008_687_907_853_269_984_665_640_564_039_457_U256);

/// A position's liquidation penalty (`HealthLogic.position`): the highest of its pair's two tokens' and of each of
/// its debts outside the pair (`TokenRisk.penaltyWad` of each).
#[must_use]
pub fn position_penalty(penalty0_wad: U256, penalty1_wad: U256, outside_pair_wad: &[U256]) -> U256 {
    outside_pair_wad
        .iter()
        .fold(penalty0_wad.max(penalty1_wad), |max, penalty| max.max(*penalty))
}

/// Runs the stress test of a range position with `penalty_wad`, the position's liquidation penalty
/// ([`position_penalty`]).
///
/// # Errors
///
/// [`Error::InvalidSigma`] for a σ below 1e18, [`Error::InvalidPrice`] for a zero token price, and the arithmetic
/// errors the contract reverts with.
pub fn evaluate(collateral: &CollateralInput, debts: &[DebtInput], penalty_wad: U256) -> Result<HealthResult> {
    let corners = StressCorners::new(&collateral.token0, &collateral.token1)?;
    let prices = corners.sqrt_prices();
    let holdings = range_holdings(collateral, &prices)?;
    evaluate_holdings_stressed(&corners.pair, &holdings, debts, penalty_wad)
}

/// The sqrt prices (token0 in token1, Q64.96) a health check asks the adapter about: the oracle's, then each
/// corner's, from that corner's stressed USD prices ([`stressed_prices`]).
///
/// # Errors
///
/// [`Error::InvalidSigma`] for a σ below 1e18, [`Error::InvalidPrice`] for a zero token price, and the arithmetic
/// errors the contract reverts with.
pub fn sqrt_prices(token0: &TokenInput, token1: &TokenInput) -> Result<[U160; PRICES]> {
    Ok(StressCorners::new(token0, token1)?.sqrt_prices())
}

/// Runs the stress test on what the position's adapter reports it holds at [`sqrt_prices`], with `penalty_wad`,
/// the position's liquidation penalty ([`position_penalty`]).
///
/// # Errors
///
/// [`Error::InvalidSigma`] for a σ below 1e18, [`Error::InvalidPrice`] for a zero token price, and the arithmetic
/// errors the contract reverts with.
pub fn evaluate_holdings(
    token0: &TokenInput,
    token1: &TokenInput,
    holdings: &Holdings,
    debts: &[DebtInput],
    penalty_wad: U256,
) -> Result<HealthResult> {
    let stressed = StressedPair::new(token0, token1)?;
    evaluate_holdings_stressed(&stressed, holdings, debts, penalty_wad)
}

/// USD value of `collateral.liquidity` at the given token prices, rounded down.
///
/// # Errors
///
/// The arithmetic errors of the contract (e.g. [`Error::Overflow`] for a price too large).
pub fn collateral_usd_at(collateral: &CollateralInput, price0_wad: U256, price1_wad: U256) -> Result<U256> {
    if collateral.liquidity == 0 {
        return Ok(U256::ZERO);
    }
    let unit0 = pow10(collateral.token0.decimals)?;
    let unit1 = pow10(collateral.token1.decimals)?;
    let sqrt_price_x96 = sqrt_price_x96_from_usd_units(price0_wad, price1_wad, unit0, unit1)?;
    value_at_units(collateral, sqrt_price_x96, price0_wad, price1_wad, unit0, unit1)
}

/// Token prices in one corner: `p · σ` (up) or `p / σ` (down), rounded down. Bit 0 of `corner` stresses token0
/// up, bit 1 stresses token1 up.
///
/// # Errors
///
/// [`Error::MulDiv`] on overflow or a zero σ.
pub fn stressed_prices(
    price0_wad: U256,
    price1_wad: U256,
    sigma0_wad: U256,
    sigma1_wad: U256,
    corner: u8,
) -> Result<(U256, U256)> {
    Ok((
        stress(price0_wad, sigma0_wad, corner & 1 != 0)?,
        stress(price1_wad, sigma1_wad, corner & 2 != 0)?,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StressedPair {
    token0: TokenInput,
    token1: TokenInput,
    unit0: U256,
    unit1: U256,
    p0_down: U256,
    p0_up: U256,
    p1_down: U256,
    p1_up: U256,
}

impl StressedPair {
    fn new(token0: &TokenInput, token1: &TokenInput) -> Result<Self> {
        check(token0, token1)?;
        Ok(Self {
            token0: *token0,
            token1: *token1,
            unit0: pow10(token0.decimals)?,
            unit1: pow10(token1.decimals)?,
            p0_down: stress(token0.price_wad, token0.sigma_wad, false)?,
            p0_up: stress(token0.price_wad, token0.sigma_wad, true)?,
            p1_down: stress(token1.price_wad, token1.sigma_wad, false)?,
            p1_up: stress(token1.price_wad, token1.sigma_wad, true)?,
        })
    }

    fn corner_prices(&self) -> [(U256, U256); 4] {
        [
            (self.p0_down, self.p1_down),
            (self.p0_up, self.p1_down),
            (self.p0_down, self.p1_up),
            (self.p0_up, self.p1_up),
        ]
    }
}

struct StressedDebts {
    debt_usd_wad: U256,
    corners: [U256; 4],
}

impl StressedDebts {
    /// Mirrors `HealthMath.evaluate`'s debt loop so overflow and `InvalidSigma` errors fire in the contract's order.
    fn new(token0: &TokenInput, token1: &TokenInput, debts: &[DebtInput]) -> Result<Self> {
        let mut debt_usd_wad = U256::ZERO;
        let (mut d0_down, mut d0_up) = (U256::ZERO, U256::ZERO);
        let (mut d1_down, mut d1_up) = (U256::ZERO, U256::ZERO);
        let mut d_fixed = U256::ZERO;

        for debt in debts {
            debt_usd_wad = debt_usd_wad.try_add(debt.value_usd_wad)?;
            match debt.tie {
                DebtTie::Token0 => {
                    d0_down = d0_down.try_add(stress_rounding_up(debt.value_usd_wad, token0.sigma_wad, false)?)?;
                    d0_up = d0_up.try_add(stress_rounding_up(debt.value_usd_wad, token0.sigma_wad, true)?)?;
                }
                DebtTie::Token1 => {
                    d1_down = d1_down.try_add(stress_rounding_up(debt.value_usd_wad, token1.sigma_wad, false)?)?;
                    d1_up = d1_up.try_add(stress_rounding_up(debt.value_usd_wad, token1.sigma_wad, true)?)?;
                }
                DebtTie::Untied => {
                    if debt.sigma_wad < WAD {
                        return Err(Error::InvalidSigma);
                    }
                    d_fixed = d_fixed.try_add(mul_div_rounding_up(debt.value_usd_wad, debt.sigma_wad, WAD)?)?;
                }
            }
        }

        let corners = [
            d0_down.try_add(d1_down)?.try_add(d_fixed)?,
            d0_up.try_add(d1_down)?.try_add(d_fixed)?,
            d0_down.try_add(d1_up)?.try_add(d_fixed)?,
            d0_up.try_add(d1_up)?.try_add(d_fixed)?,
        ];
        Ok(Self { debt_usd_wad, corners })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Corner {
    stressed0: U256,
    stressed1: U256,
    sqrt_price_x96: U160,
}

/// Precomputed oracle and four-corner prices for repeated health checks against a fixed pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StressCorners {
    pair: StressedPair,
    oracle_sqrt_price_x96: U160,
    corners: [Corner; 4],
    fixed_collateral: Option<[U256; 4]>,
}

impl StressCorners {
    pub(crate) fn new(token0: &TokenInput, token1: &TokenInput) -> Result<Self> {
        let pair = StressedPair::new(token0, token1)?;
        let oracle_sqrt_price_x96 =
            sqrt_price_x96_from_usd_units(token0.price_wad, token1.price_wad, pair.unit0, pair.unit1)?;
        Self::from_pair(&pair, oracle_sqrt_price_x96)
    }

    pub(crate) fn new_with_oracle(
        token0: &TokenInput,
        token1: &TokenInput,
        oracle_sqrt_price_x96: U160,
    ) -> Result<Self> {
        let pair = StressedPair::new(token0, token1)?;
        Self::from_pair(&pair, oracle_sqrt_price_x96)
    }

    fn from_pair(pair: &StressedPair, oracle_sqrt_price_x96: U160) -> Result<Self> {
        let make = |(stressed0, stressed1)| -> Result<Corner> {
            Ok(Corner {
                stressed0,
                stressed1,
                sqrt_price_x96: sqrt_price_x96_from_usd_units(stressed0, stressed1, pair.unit0, pair.unit1)?,
            })
        };
        let [p0, p1, p2, p3] = pair.corner_prices();
        Ok(Self {
            pair: *pair,
            oracle_sqrt_price_x96,
            corners: [make(p0)?, make(p1)?, make(p2)?, make(p3)?],
            fixed_collateral: None,
        })
    }

    pub(crate) fn with_collateral(mut self, collateral: &CollateralInput) -> Result<Self> {
        let _ = value_at_units(
            collateral,
            self.oracle_sqrt_price_x96,
            self.pair.token0.price_wad,
            self.pair.token1.price_wad,
            self.pair.unit0,
            self.pair.unit1,
        )?;
        self.fixed_collateral = Some(self.corner_collaterals(collateral)?);
        Ok(self)
    }

    pub(crate) fn sqrt_prices(&self) -> [U160; PRICES] {
        let [c0, c1, c2, c3] = self.corners;
        [
            self.oracle_sqrt_price_x96,
            c0.sqrt_price_x96,
            c1.sqrt_price_x96,
            c2.sqrt_price_x96,
            c3.sqrt_price_x96,
        ]
    }

    /// Runs the full four-corner check in fixed order `0..4`, matching `HealthMath.evaluate`'s exact error behavior.
    pub(crate) fn passes(&self, collateral: &CollateralInput, debts: &[DebtInput], penalty_wad: U256) -> Result<bool> {
        let mut all_pass = true;
        if let Some(corner_coll) = self.fixed_collateral {
            let stressed = StressedDebts::new(&self.pair.token0, &self.pair.token1, debts)?;
            for (collateral_usd, stressed_debt) in corner_coll.into_iter().zip(stressed.corners) {
                all_pass &= corner_passes(collateral_usd, stressed_debt, penalty_wad)?;
            }
            return Ok(all_pass);
        }

        let _ = value_at_units(
            collateral,
            self.oracle_sqrt_price_x96,
            self.pair.token0.price_wad,
            self.pair.token1.price_wad,
            self.pair.unit0,
            self.pair.unit1,
        )?;
        let stressed = StressedDebts::new(&self.pair.token0, &self.pair.token1, debts)?;
        for (c, stressed_debt) in self.corners.into_iter().zip(stressed.corners) {
            let collateral_usd = value_at_units(
                collateral,
                c.sqrt_price_x96,
                c.stressed0,
                c.stressed1,
                self.pair.unit0,
                self.pair.unit1,
            )?;
            all_pass &= corner_passes(collateral_usd, stressed_debt, penalty_wad)?;
        }
        Ok(all_pass)
    }

    fn corner_collaterals(&self, collateral: &CollateralInput) -> Result<[U256; 4]> {
        if collateral.liquidity == 0 {
            return Ok([U256::ZERO; 4]);
        }
        let val = |c: Corner| {
            value_at_units(
                collateral,
                c.sqrt_price_x96,
                c.stressed0,
                c.stressed1,
                self.pair.unit0,
                self.pair.unit1,
            )
        };
        let [c0, c1, c2, c3] = self.corners;
        Ok([val(c0)?, val(c1)?, val(c2)?, val(c3)?])
    }
}

/// Evaluates health at `penalty_wad` and also checks whether the same corners pass at `buffered_penalty_wad`,
/// computing prices, holdings, corner collaterals and stressed debts only once.
pub(crate) fn evaluate_with_buffered(
    collateral: &CollateralInput,
    debts: &[DebtInput],
    penalty_wad: U256,
    buffered_penalty_wad: U256,
) -> Result<(HealthResult, bool)> {
    let corners = StressCorners::new(&collateral.token0, &collateral.token1)?;
    let prices = corners.sqrt_prices();
    let holdings = range_holdings(collateral, &prices)?;
    let (result, corner_coll, stressed_corners) =
        evaluate_holdings_with_corners(&corners.pair, &holdings, debts, penalty_wad)?;
    let mut all_pass = true;
    for (collateral_usd, stressed_debt) in corner_coll.into_iter().zip(stressed_corners) {
        all_pass &= corner_passes(collateral_usd, stressed_debt, buffered_penalty_wad)?;
    }
    Ok((result, all_pass))
}

#[inline]
fn corner_passes(collateral_usd: U256, stressed_debt: U256, penalty_wad: U256) -> Result<bool> {
    let required = mul_div_rounding_up(stressed_debt, penalty_wad, WAD)?;
    // `to_i256` checks both values fit in `int256`, matching `HealthMath._applyCorner`'s subtraction.
    let ok = to_i256(collateral_usd)? >= to_i256(required)?;
    // `collateral * WAD / required` can only overflow `U256` when `required < WAD` and `collateral > MAX / WAD`.
    if !required.is_zero() && required < WAD && collateral_usd > MAX_SAFE_HEALTH_COLLATERAL {
        let _ = mul_div(collateral_usd, WAD, required)?;
    }
    Ok(ok)
}

fn apply_corner(
    collateral_usd: U256,
    stressed_debt: U256,
    penalty_wad: U256,
    min_surplus_usd_wad: &mut I256,
    health_wad: &mut U256,
) -> Result<()> {
    let required = mul_div_rounding_up(stressed_debt, penalty_wad, WAD)?;
    let surplus = to_i256(collateral_usd)?
        .checked_sub(to_i256(required)?)
        .ok_or(Error::Overflow)?;
    *min_surplus_usd_wad = (*min_surplus_usd_wad).min(surplus);
    if !required.is_zero() {
        *health_wad = (*health_wad).min(mul_div(collateral_usd, WAD, required)?);
    }
    Ok(())
}

/// Uniswap sqrt price of token0 in token1 from two USD prices:
/// `sqrt(p0 · 10^dec1 / (p1 · 10^dec0)) · 2^96`, clamped to `[MIN_SQRT_PRICE, MAX_SQRT_PRICE]`.
///
/// # Errors
///
/// [`Error::Overflow`] when a price times `10^decimals` overflows. A zero `price1_wad` gives `MAX_SQRT_PRICE`.
pub fn sqrt_price_x96_from_usd(price0_wad: U256, price1_wad: U256, decimals0: u8, decimals1: u8) -> Result<U160> {
    sqrt_price_x96_from_usd_units(price0_wad, price1_wad, pow10(decimals0)?, pow10(decimals1)?)
}

fn sqrt_price_x96_from_usd_units(price0_wad: U256, price1_wad: U256, unit0: U256, unit1: U256) -> Result<U160> {
    let numerator = price0_wad.try_mul(unit1)?;
    let denominator = price1_wad.try_mul(unit0)?;

    // A stressed price of a wei or two rounds to 0: the top price, as for a ratio past every pool's
    if denominator.is_zero() {
        return Ok(MAX_SQRT_PRICE);
    }
    let denom_lz = denominator.leading_zeros();
    // `numerator / denominator >= 2^128` without a division
    if denom_lz >= 128 && numerator >= denominator.wrapping_shl(128) {
        return Ok(MAX_SQRT_PRICE);
    }

    // Below 2^64 the root keeps all 96 fractional bits; above, 64 do (2^192 · ratio would overflow)
    let [n0, n1, n2, n3] = *numerator.as_limbs();
    let sqrt_price = if denom_lz < 64 || numerator < denominator.wrapping_shl(64) {
        let q = U512::from_limbs([0, 0, 0, n0, n1, n2, n3, 0]).wrapping_div(U512::from(denominator));
        let [q0, q1, q2, q3, ..] = *q.as_limbs();
        sqrt(U256::from_limbs([q0, q1, q2, q3]))?
    } else {
        let q = U512::from_limbs([0, 0, n0, n1, n2, n3, 0, 0]).wrapping_div(U512::from(denominator));
        let [q0, q1, q2, q3, ..] = *q.as_limbs();
        sqrt(U256::from_limbs([q0, q1, q2, q3]))?.wrapping_shl(32)
    };
    clamp_sqrt_price(sqrt_price)
}

fn check(token0: &TokenInput, token1: &TokenInput) -> Result<()> {
    if token0.sigma_wad < WAD || token1.sigma_wad < WAD {
        return Err(Error::InvalidSigma);
    }
    if token0.price_wad.is_zero() || token1.price_wad.is_zero() {
        return Err(Error::InvalidPrice);
    }
    Ok(())
}

fn evaluate_holdings_stressed(
    pair: &StressedPair,
    holdings: &Holdings,
    debts: &[DebtInput],
    penalty_wad: U256,
) -> Result<HealthResult> {
    evaluate_holdings_with_corners(pair, holdings, debts, penalty_wad).map(|(result, _, _)| result)
}

fn evaluate_holdings_with_corners(
    pair: &StressedPair,
    holdings: &Holdings,
    debts: &[DebtInput],
    penalty_wad: U256,
) -> Result<(HealthResult, [U256; 4], [U256; 4])> {
    let [oracle0, corners0 @ ..] = holdings.amounts0;
    let [oracle1, corners1 @ ..] = holdings.amounts1;
    let collateral_usd_wad = value_usd_units(
        oracle0,
        oracle1,
        pair.token0.price_wad,
        pair.token1.price_wad,
        pair.unit0,
        pair.unit1,
    )?;
    let stressed = StressedDebts::new(&pair.token0, &pair.token1, debts)?;
    let mut min_surplus_usd_wad = I256::MAX;
    let mut health_wad = U256::MAX;
    let mut corner_coll = [U256::ZERO; 4];
    for (slot, (((amount0, amount1), (stressed0, stressed1)), stressed_debt)) in corner_coll.iter_mut().zip(
        corners0
            .into_iter()
            .zip(corners1)
            .zip(pair.corner_prices())
            .zip(stressed.corners),
    ) {
        let collateral_usd = value_usd_units(amount0, amount1, stressed0, stressed1, pair.unit0, pair.unit1)?;
        *slot = collateral_usd;
        apply_corner(
            collateral_usd,
            stressed_debt,
            penalty_wad,
            &mut min_surplus_usd_wad,
            &mut health_wad,
        )?;
    }
    Ok((
        HealthResult {
            min_surplus_usd_wad,
            health_wad,
            collateral_usd_wad,
            debt_usd_wad: stressed.debt_usd_wad,
        },
        corner_coll,
        stressed.corners,
    ))
}

fn range_holdings(collateral: &CollateralInput, prices: &[U160; PRICES]) -> Result<Holdings> {
    let mut holdings = Holdings::default();
    if collateral.liquidity == 0 {
        return Ok(holdings);
    }
    for ((price, amount0), amount1) in prices.iter().zip(&mut holdings.amounts0).zip(&mut holdings.amounts1) {
        (*amount0, *amount1) = amounts_for_liquidity(
            *price,
            collateral.sqrt_pa_x96,
            collateral.sqrt_pb_x96,
            collateral.liquidity,
        )?;
    }
    Ok(holdings)
}

fn value_at_units(
    collateral: &CollateralInput,
    sqrt_price_x96: U160,
    price0_wad: U256,
    price1_wad: U256,
    unit0: U256,
    unit1: U256,
) -> Result<U256> {
    if collateral.liquidity == 0 {
        return Ok(U256::ZERO);
    }
    let (amount0, amount1) = amounts_for_liquidity(
        sqrt_price_x96,
        collateral.sqrt_pa_x96,
        collateral.sqrt_pb_x96,
        collateral.liquidity,
    )?;
    value_usd_units(amount0, amount1, price0_wad, price1_wad, unit0, unit1)
}

fn value_usd_units(
    amount0: U256,
    amount1: U256,
    price0_wad: U256,
    price1_wad: U256,
    unit0: U256,
    unit1: U256,
) -> Result<U256> {
    mul_div(amount0, price0_wad, unit0)?.try_add(mul_div(amount1, price1_wad, unit1)?)
}

fn clamp_sqrt_price(sqrt_price: U256) -> Result<U160> {
    if sqrt_price < widen(MIN_SQRT_PRICE) {
        return Ok(MIN_SQRT_PRICE);
    }
    if sqrt_price > widen(MAX_SQRT_PRICE) {
        return Ok(MAX_SQRT_PRICE);
    }
    to_u160(sqrt_price)
}

fn stress(value: U256, sigma_wad: U256, up: bool) -> Result<U256> {
    if up {
        mul_div(value, sigma_wad, WAD)
    } else {
        mul_div(value, WAD, sigma_wad)
    }
}

fn stress_rounding_up(value: U256, sigma_wad: U256, up: bool) -> Result<U256> {
    if up {
        mul_div_rounding_up(value, sigma_wad, WAD)
    } else {
        mul_div_rounding_up(value, WAD, sigma_wad)
    }
}
