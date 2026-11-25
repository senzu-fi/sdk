//! The pure searches and previews of `SenzuLens`: liquidation prices, max borrow, max leverage, the swap that
//! fits a range, and the Create Position preview.

use alloy_primitives::{U160, U256, uint};

use crate::error::Result;
use crate::health::{self, collateral_usd_at, sqrt_price_x96_from_usd};
use crate::market::{self as market_math, Market, Rounding};
use crate::num::{Checked, WAD, pow10, to_u128};
use crate::types::{Asset, CollateralInput, DebtInput, DebtTie, Side};
use crate::uniswap::full_math::mul_shr96;
use crate::uniswap::{Q96, amounts_for_liquidity, mul_div, mul_div_rounding_up, sqrt};

/// Liquidation prices are searched between 1/1000x and 1000x of the current price.
pub const SEARCH_RANGE: U256 = U256::from_limbs([1_000, 0, 0, 0]);
/// Bisection stops when the bracket is within 1e-6 (relative), in WAD.
pub const TOLERANCE: U256 = U256::from_limbs([1_000_000_000_000, 0, 0, 0]);
/// Highest leverage [`max_leverage`] searches, 100x.
pub const MAX_LEVERAGE: U256 = uint!(100_000_000_000_000_000_000_U256);
/// Upper end of [`max_borrow`]'s search, in multiples of the position's value (`SenzuPreviews.MAX_DEBT_PER_VALUE`).
pub const MAX_DEBT_PER_VALUE: U256 = U256::from_limbs([10, 0, 0, 0]);
/// The most assets a position may owe (`SenzuPreviews.MAX_DEBTS`).
pub const MAX_DEBTS: usize = 2;
/// Reference liquidity for per-unit valuations.
pub const REF_LIQUIDITY: u128 = 1_000_000_000_000_000_000;

/// Where a position gets liquidated as one token's USD price moves (the other's held fixed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct LiquidationPrices {
    /// The token whose price moves: the more volatile one (higher σ; token1 on a tie).
    pub moving: Side,
    /// Its USD price (1e18) at liquidation below the current price, `None` within 1/1000x.
    pub lower_wad: Option<U256>,
    /// Its USD price at liquidation above the current price, `None` within 1000x.
    pub upper_wad: Option<U256>,
}

/// The swap that turns two token amounts into exactly a range's ratio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct SwapQuote {
    /// Sell token0 for token1 (`false`: sell token1 for token0).
    pub zero_for_one: bool,
    /// Amount sold.
    pub amount_in: U256,
    /// Amount bought at the given price, after the swap fee.
    pub amount_out: U256,
}

/// How [`max_leverage`] borrows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub enum LeverageBorrow {
    /// Both pair tokens at the range's ratio, no swap (`borrowAsset = address(0)` in the lens).
    PairRatio,
    /// All debt in one asset, swapped into the range.
    Single(Asset),
}

/// One borrow of a new position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct OpenBorrow {
    /// What is borrowed.
    pub asset: Asset,
    /// How much, before the opening fee.
    pub amount: U256,
    /// Its market, accrued to now.
    pub market: Market,
}

/// Inputs of the Create Position panel (`ISenzuLens.OpenParams` with prices already resolved).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct OpenParams {
    /// The pair, range and prices; `liquidity` is ignored.
    pub collateral: CollateralInput,
    /// Token0 the user deposits.
    pub deposit0: U256,
    /// Token1 the user deposits.
    pub deposit1: U256,
    /// What is borrowed, one entry per asset (the router borrows each once).
    pub borrows: Vec<OpenBorrow>,
    /// Swap into the range's ratio; `false` adds only what fits.
    pub swap: bool,
    /// The new position's liquidation penalty: [`health::position_penalty`] of its pair and every asset it borrows
    /// outside the pair (a zero amount is no debt), as `SenzuPreviews.previewOpen` reads it.
    pub penalty_wad: U256,
    /// The deployment's opening fee, 1e18 = 100%.
    pub borrow_fee_wad: U256,
}

/// Result of [`preview_open`] (`ISenzuLens.OpenPreview`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct OpenPreview {
    /// Liquidity of the new position.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub liquidity: u128,
    /// Token0 in the position at the oracle price.
    pub amount0: U256,
    /// Token1 in the position at the oracle price.
    pub amount1: U256,
    /// Position value ("Size").
    pub collateral_usd_wad: U256,
    /// Value of the deposits.
    pub margin_usd_wad: U256,
    /// Value of the debts, opening fee included.
    pub debt_usd_wad: U256,
    /// Size / margin, 0 without margin.
    pub leverage_wad: U256,
    /// Health of the liquidation check.
    pub health_wad: U256,
    /// Whether the borrow check passes, with at most two assets borrowed.
    pub allowed: bool,
    /// Liquidation prices of the new position.
    pub liquidation: LiquidationPrices,
}

/// Result of [`preview_open_summary`]: [`OpenPreview`] without the liquidation prices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct OpenSummary {
    /// Liquidity of the new position.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub liquidity: u128,
    /// Token0 in the position at the oracle price.
    pub amount0: U256,
    /// Token1 in the position at the oracle price.
    pub amount1: U256,
    /// Position value ("Size").
    pub collateral_usd_wad: U256,
    /// Value of the deposits.
    pub margin_usd_wad: U256,
    /// Value of the debts, opening fee included.
    pub debt_usd_wad: U256,
    /// Size / margin, 0 without margin.
    pub leverage_wad: U256,
    /// Health of the liquidation check.
    pub health_wad: U256,
    /// Whether the borrow check passes, with at most two assets borrowed.
    pub allowed: bool,
}

/// Liquidation prices of a position (`SenzuLens._liquidationPrices`), found by geometric bisection on the more
/// volatile token's USD price (debts in that token move with it). An already liquidatable position gets the
/// current price on both sides.
///
/// # Errors
///
/// The errors of [`health::evaluate`].
pub fn liquidation_prices(
    collateral: &CollateralInput,
    debts: &[DebtInput],
    penalty_wad: U256,
) -> Result<LiquidationPrices> {
    liquidation_prices_inner(collateral, debts, penalty_wad, None)
}

pub(crate) fn liquidation_prices_with_initial(
    collateral: &CollateralInput,
    debts: &[DebtInput],
    penalty_wad: U256,
    initial_passes: bool,
) -> Result<LiquidationPrices> {
    liquidation_prices_inner(collateral, debts, penalty_wad, Some(initial_passes))
}

fn liquidation_prices_inner(
    collateral: &CollateralInput,
    debts: &[DebtInput],
    penalty_wad: U256,
    initial_passes: Option<bool>,
) -> Result<LiquidationPrices> {
    let moving = if collateral.token1.sigma_wad >= collateral.token0.sigma_wad {
        Side::Token1
    } else {
        Side::Token0
    };
    let price = collateral.token(moving).price_wad;
    let mut prices = LiquidationPrices {
        moving,
        lower_wad: None,
        upper_wad: None,
    };
    if debts.is_empty() || collateral.liquidity == 0 {
        return Ok(prices);
    }

    let mut search = Search {
        collateral,
        debts,
        penalty_wad,
        moving,
        scaled: Vec::with_capacity(debts.len()),
    };
    let already_unsafe = if let Some(passes) = initial_passes {
        !passes
    } else {
        search.surplus_negative(WAD)?
    };
    if already_unsafe {
        return Ok(LiquidationPrices {
            moving,
            lower_wad: Some(price),
            upper_wad: Some(price),
        });
    }

    let k_min = WAD.try_div(SEARCH_RANGE)?;
    let k_max = WAD.try_mul(SEARCH_RANGE)?;
    if search.surplus_negative(k_min)? {
        prices.lower_wad = Some(mul_div(price, search.bisect(k_min, WAD)?, WAD)?);
    }
    if search.surplus_negative(k_max)? {
        prices.upper_wad = Some(mul_div(price, search.bisect(k_max, WAD)?, WAD)?);
    }
    Ok(prices)
}

struct Search<'a> {
    collateral: &'a CollateralInput,
    debts: &'a [DebtInput],
    penalty_wad: U256,
    moving: Side,
    scaled: Vec<DebtInput>,
}

impl Search<'_> {
    fn bisect(&mut self, mut unsafe_k: U256, mut safe_k: U256) -> Result<U256> {
        loop {
            let (lo, hi) = if unsafe_k < safe_k {
                (unsafe_k, safe_k)
            } else {
                (safe_k, unsafe_k)
            };
            if hi.try_sub(lo)?.try_mul(WAD)? <= lo.try_mul(TOLERANCE)? {
                return Ok(unsafe_k);
            }
            let mid = sqrt(lo.try_mul(hi)?)?;
            if self.surplus_negative(mid)? {
                unsafe_k = mid;
            } else {
                safe_k = mid;
            }
        }
    }

    fn surplus_negative(&mut self, k: U256) -> Result<bool> {
        let mut moved = *self.collateral;
        let (moving_token, moving_tie) = match self.moving {
            Side::Token0 => (&mut moved.token0, DebtTie::Token0),
            Side::Token1 => (&mut moved.token1, DebtTie::Token1),
        };
        moving_token.price_wad = mul_div(moving_token.price_wad, k, WAD)?.max(U256::from(1_u8));

        self.scaled.clear();
        for debt in self.debts {
            self.scaled.push(DebtInput {
                value_usd_wad: if debt.tie == moving_tie {
                    mul_div_rounding_up(debt.value_usd_wad, k, WAD)?
                } else {
                    debt.value_usd_wad
                },
                ..*debt
            });
        }

        let corners = health::StressCorners::new(&moved.token0, &moved.token1)?;
        Ok(!corners.passes(&moved, &self.scaled, self.penalty_wad)?)
    }
}

/// What a position owes in `market`'s asset after borrowing `amount` more, exactly as the borrowable books it
/// (`BorrowLogic.addDebt`, `SenzuLens._booked`): the amount plus the opening fee as shares rounded up, added to the
/// position's `account_shares` and the market's totals, then back to assets rounded up. `market` is accrued to now.
///
/// # Errors
///
/// The arithmetic errors of the share math.
pub fn booked_debt(market: &Market, account_shares: U256, amount: U256, borrow_fee_wad: U256) -> Result<U256> {
    let debt = with_fee(amount, borrow_fee_wad)?;
    let (assets, shares) = (
        U256::from(market.total_borrow_assets),
        U256::from(market.total_borrow_shares),
    );
    let added = market_math::to_shares(debt, assets, shares, Rounding::Up)?;
    market_math::to_assets(
        account_shares.try_add(added)?,
        assets.try_add(debt)?,
        shares.try_add(added)?,
        Rounding::Up,
    )
}

/// Most of `asset` a position can borrow now and still pass the borrow check (`SenzuLens.maxBorrow`), searched
/// to 1e-6 and capped by the market's cash ([`Market::borrow_limit`]) and by 10x the position's value. Zero when
/// `other_debts` holds [`MAX_DEBTS`] already: a position owes at most two assets.
/// `other_debts` are the position's debts in other assets; its debt in `asset` is its `account_shares` in `market`
/// (accrued to now), booked again with each amount tried ([`booked_debt`]). `penalty_wad` is the
/// position's liquidation penalty with `asset` borrowed ([`health::position_penalty`]).
///
/// # Errors
///
/// The errors of [`health::evaluate`], and the arithmetic errors of the lens.
pub fn max_borrow(
    collateral: &CollateralInput,
    other_debts: &[DebtInput],
    asset: &Asset,
    market: &Market,
    account_shares: U256,
    penalty_wad: U256,
    borrow_fee_wad: U256,
) -> Result<U256> {
    let token = match asset {
        Asset::Token0 => &collateral.token0,
        Asset::Token1 => &collateral.token1,
        Asset::Other(token) => token,
    };
    // The pair is checked first, as `SenzuLens.maxBorrow` reads the position before counting its debts
    let corners = health::StressCorners::new(&collateral.token0, &collateral.token1)?;
    if other_debts.len() >= MAX_DEBTS {
        return Ok(U256::ZERO);
    }
    let corners = corners.with_collateral(collateral)?;
    let added = collateral.debt(asset, U256::ZERO)?;
    let unit = pow10(token.decimals)?;
    let mut all_debts = Vec::with_capacity(other_debts.len().saturating_add(1));
    all_debts.extend_from_slice(other_debts);
    all_debts.push(added);
    let mut surplus_ok = |amount: U256| -> Result<bool> {
        let added_usd = mul_div_rounding_up(
            booked_debt(market, account_shares, amount, borrow_fee_wad)?,
            token.price_wad,
            unit,
        )?;
        if let Some(last) = all_debts.last_mut() {
            last.value_usd_wad = added_usd;
        }
        corners.passes(collateral, &all_debts, penalty_wad)
    };

    let value_usd = collateral_usd_at(collateral, collateral.token0.price_wad, collateral.token1.price_wad)?;
    let mut hi = market
        .borrow_limit()?
        .min(mul_div(value_usd.try_mul(MAX_DEBT_PER_VALUE)?, unit, token.price_wad)?);
    if surplus_ok(hi)? {
        return Ok(hi);
    }
    if !surplus_ok(U256::ZERO)? {
        return Ok(U256::ZERO);
    }

    let mut lo = U256::ZERO;
    let one = U256::from(1_u8);
    let million = U256::from(1_000_000_u32);
    while hi.try_sub(lo)? > one && hi.try_sub(lo)? > lo.try_div(million)? {
        let mid = lo.try_add(hi)?.try_div(U256::from(2_u8))?;
        if surplus_ok(mid)? {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(lo)
}

/// Highest leverage (size / margin, WAD) a new position in `collateral`'s range passes the borrow check with
/// (`SenzuLens.maxLeverage`), searched to 1e-6 up to [`MAX_LEVERAGE`]. `collateral.liquidity` is ignored.
///
/// # Errors
///
/// The errors of [`health::evaluate`], and the arithmetic errors of the lens.
pub fn max_leverage(
    collateral: &CollateralInput,
    borrow: &LeverageBorrow,
    penalty_wad: U256,
    borrow_fee_wad: U256,
) -> Result<U256> {
    let Some(ctx) = MaxLeverageContext::new(collateral)? else {
        return Ok(WAD);
    };
    ctx.search(borrow, penalty_wad, borrow_fee_wad)
}

/// Precomputed reference range value, token0 share and stress corners for [`max_leverage`] searches on one pair.
pub(crate) struct MaxLeverageContext<'a> {
    collateral: &'a CollateralInput,
    value: U256,
    share0: U256,
    corners: health::StressCorners,
}

impl<'a> MaxLeverageContext<'a> {
    /// Returns `Ok(None)` when the reference range has zero USD value (in which case max leverage is 1x).
    pub(crate) fn new(collateral: &'a CollateralInput) -> Result<Option<Self>> {
        let oracle_sqrt = oracle_sqrt_price(collateral)?;
        let (amount0, amount1) = amounts_for_liquidity(
            oracle_sqrt,
            collateral.sqrt_pa_x96,
            collateral.sqrt_pb_x96,
            REF_LIQUIDITY,
        )?;
        let usd0 = usd(amount0, collateral.token0.price_wad, collateral.token0.decimals)?;
        let usd1 = usd(amount1, collateral.token1.price_wad, collateral.token1.decimals)?;
        let value = usd0.try_add(usd1)?;
        if value.is_zero() {
            return Ok(None);
        }
        let share0 = mul_div(usd0, WAD, value)?;
        let corners = health::StressCorners::new_with_oracle(&collateral.token0, &collateral.token1, oracle_sqrt)?;
        Ok(Some(Self {
            collateral,
            value,
            share0,
            corners,
        }))
    }

    pub(crate) fn search(&self, borrow: &LeverageBorrow, penalty_wad: U256, borrow_fee_wad: U256) -> Result<U256> {
        let collateral = self.collateral;
        let single_debt = match borrow {
            LeverageBorrow::PairRatio => None,
            LeverageBorrow::Single(asset) => Some(collateral.debt(asset, U256::ZERO)?),
        };
        let surplus_ok = |leverage: U256| -> Result<bool> {
            let position = CollateralInput {
                liquidity: to_u128(mul_div(U256::from(REF_LIQUIDITY), leverage, WAD)?)?,
                ..*collateral
            };
            let debt_usd = with_fee(
                mul_div_rounding_up(self.value, leverage.try_sub(WAD)?, WAD)?,
                borrow_fee_wad,
            )?;
            let pair_debts: [DebtInput; 2];
            let one_debt: [DebtInput; 1];
            let debts: &[DebtInput] = match single_debt {
                None => {
                    let debt0 = mul_div_rounding_up(debt_usd, self.share0, WAD)?;
                    pair_debts = [
                        DebtInput {
                            value_usd_wad: debt0,
                            sigma_wad: collateral.token0.sigma_wad,
                            tie: DebtTie::Token0,
                        },
                        DebtInput {
                            value_usd_wad: debt_usd.try_sub(debt0.min(debt_usd))?,
                            sigma_wad: collateral.token1.sigma_wad,
                            tie: DebtTie::Token1,
                        },
                    ];
                    &pair_debts
                }
                Some(template) => {
                    one_debt = [DebtInput {
                        value_usd_wad: debt_usd,
                        ..template
                    }];
                    &one_debt
                }
            };
            self.corners.passes(&position, debts, penalty_wad)
        };

        let mut lo = WAD;
        let mut hi = MAX_LEVERAGE;
        if surplus_ok(hi)? {
            return Ok(hi);
        }
        let million = U256::from(1_000_000_u32);
        while hi.try_sub(lo)? > lo.try_div(million)? {
            let mid = lo.try_add(hi)?.try_div(U256::from(2_u8))?;
            if surplus_ok(mid)? {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        Ok(lo)
    }
}

/// How much to swap so that `amount0` / `amount1` fit the range's token ratio exactly at `sqrt_price_x96`
/// (`SenzuLens.swapForRange`). Out of range the position holds one token, so all of the other is swapped.
/// `swap_fee_wad` is taken from the input (0.0005e18 for a 0.05% pool). Use [`sqrt_price_x96_from_usd`] for the
/// oracle price.
///
/// # Errors
///
/// [`Error::Underflow`](crate::Error::Underflow) for a fee above 100%, and the arithmetic errors of the lens.
pub fn swap_for_range(
    sqrt_pa_x96: U160,
    sqrt_pb_x96: U160,
    amount0: U256,
    amount1: U256,
    sqrt_price_x96: U160,
    swap_fee_wad: U256,
) -> Result<SwapQuote> {
    let keep = WAD.try_sub(swap_fee_wad)?;

    let (a, b) = if sqrt_pa_x96 < sqrt_pb_x96 {
        (sqrt_pa_x96, sqrt_pb_x96)
    } else {
        (sqrt_pb_x96, sqrt_pa_x96)
    };
    if a == b {
        return Ok(SwapQuote::default());
    }
    if sqrt_price_x96 <= a {
        return quote(false, amount1, sqrt_price_x96, keep);
    }
    if sqrt_price_x96 >= b {
        return quote(true, amount0, sqrt_price_x96, keep);
    }

    // Target value ratio token0 / token1 = s(b - s) / (b(s - a)), scaled by WAD.
    // Grouping the factor >= 1/2 with WAD keeps full 18-digit precision across the full u160 range.
    let (a, b, s) = (U256::from(a), U256::from(b), U256::from(sqrt_price_x96));
    let r = if s <= b.wrapping_shr(1) {
        mul_div(mul_div(b.wrapping_sub(s), WAD, b)?, s, s.wrapping_sub(a))?
    } else {
        mul_div(mul_div(s, WAD, b)?, b.wrapping_sub(s), s.wrapping_sub(a))?
    };
    if r.is_zero() {
        return quote(true, amount0, sqrt_price_x96, keep);
    }

    let sell0_denom = WAD.try_add(mul_div(r, keep, WAD)?)?;
    let sell1_denom = r.try_add(keep)?;
    if s < Q96 {
        let x1 = to_token1(amount0, sqrt_price_x96)?;
        if r <= WAD {
            let ry = mul_div(amount1, r, WAD)?;
            if x1 > ry {
                let excess0 = amount0.try_sub(to_token0(ry, sqrt_price_x96)?)?;
                return quote(true, mul_div(excess0, WAD, sell0_denom)?, sqrt_price_x96, keep);
            }
            return quote(false, mul_div(ry.try_sub(x1)?, WAD, sell1_denom)?, sqrt_price_x96, keep);
        }
        let x1_over_r = mul_div(x1, WAD, r)?;
        if x1_over_r < amount1 {
            return quote(
                false,
                mul_div(amount1.try_sub(x1_over_r)?, r, sell1_denom)?,
                sqrt_price_x96,
                keep,
            );
        }
        let ry = mul_div(amount1, r, WAD)?;
        let excess0 = amount0.try_sub(to_token0(ry, sqrt_price_x96)?)?;
        return quote(true, mul_div(excess0, WAD, sell0_denom)?, sqrt_price_x96, keep);
    }

    let y0 = to_token0(amount1, sqrt_price_x96)?;
    if r <= WAD {
        let ry0 = mul_div(y0, r, WAD)?;
        if amount0 > ry0 {
            return quote(
                true,
                mul_div(amount0.try_sub(ry0)?, WAD, sell0_denom)?,
                sqrt_price_x96,
                keep,
            );
        }
        let x_over_r = mul_div(amount0, WAD, r)?;
        let excess1 = amount1.try_sub(to_token1(x_over_r, sqrt_price_x96)?)?;
        return quote(false, mul_div(excess1, r, sell1_denom)?, sqrt_price_x96, keep);
    }
    let x_over_r = mul_div(amount0, WAD, r)?;
    if x_over_r < y0 {
        let excess1 = amount1.try_sub(to_token1(x_over_r, sqrt_price_x96)?)?;
        return quote(false, mul_div(excess1, r, sell1_denom)?, sqrt_price_x96, keep);
    }
    let ry0 = mul_div(y0, r, WAD)?;
    quote(
        true,
        mul_div(amount0.try_sub(ry0)?, WAD, sell0_denom)?,
        sqrt_price_x96,
        keep,
    )
}

/// The Create Position preview (`SenzuLens.previewOpen`), assuming any swap executes at the oracle price.
///
/// # Errors
///
/// The errors of [`health::evaluate`], and the arithmetic errors of the lens.
pub fn preview_open(open: &OpenParams) -> Result<OpenPreview> {
    let opened = open_position(open)?;
    let s = opened.summary;
    Ok(OpenPreview {
        liquidity: s.liquidity,
        amount0: s.amount0,
        amount1: s.amount1,
        collateral_usd_wad: s.collateral_usd_wad,
        margin_usd_wad: s.margin_usd_wad,
        debt_usd_wad: s.debt_usd_wad,
        leverage_wad: s.leverage_wad,
        health_wad: s.health_wad,
        allowed: s.allowed,
        liquidation: opened.liquidation_prices()?,
    })
}

/// [`preview_open`] without the liquidation prices: the cheap part, for live updates.
///
/// # Errors
///
/// The errors of [`preview_open`].
pub fn preview_open_summary(open: &OpenParams) -> Result<OpenSummary> {
    Ok(open_position(open)?.summary)
}

/// The liquidation prices of [`preview_open`] alone.
///
/// # Errors
///
/// The errors of [`preview_open`].
pub fn open_liquidation_prices(open: &OpenParams) -> Result<LiquidationPrices> {
    open_position(open)?.liquidation_prices()
}

struct OpenSetup {
    position: CollateralInput,
    debts: Vec<DebtInput>,
    liquidation_penalty: U256,
    margin_usd_wad: U256,
    debt_usd_wad: U256,
    borrowed: usize,
}

struct Opened {
    position: CollateralInput,
    debts: Vec<DebtInput>,
    liquidation_penalty: U256,
    passes: bool,
    summary: OpenSummary,
}

impl Opened {
    fn liquidation_prices(&self) -> Result<LiquidationPrices> {
        liquidation_prices_inner(&self.position, &self.debts, self.liquidation_penalty, Some(self.passes))
    }
}

fn open_setup(open: &OpenParams) -> Result<OpenSetup> {
    let input = &open.collateral;
    let (token0, token1) = (&input.token0, &input.token1);
    let margin_usd_wad = usd(open.deposit0, token0.price_wad, token0.decimals)?.try_add(usd(
        open.deposit1,
        token1.price_wad,
        token1.decimals,
    )?)?;

    let (mut total0, mut total1) = (open.deposit0, open.deposit1);
    let mut outside_pair_usd = U256::ZERO;
    let mut debt_usd_wad = U256::ZERO;
    let mut debts = Vec::with_capacity(open.borrows.len());
    let mut borrowed = 0_usize;
    for OpenBorrow { asset, amount, market } in &open.borrows {
        match asset {
            Asset::Token0 => total0 = total0.try_add(*amount)?,
            Asset::Token1 => total1 = total1.try_add(*amount)?,
            Asset::Other(token) => {
                outside_pair_usd = outside_pair_usd.try_add(usd(*amount, token.price_wad, token.decimals)?)?;
            }
        }
        if !amount.is_zero() {
            borrowed = borrowed.saturating_add(1);
        }
        let debt = input.debt(asset, booked_debt(market, U256::ZERO, *amount, open.borrow_fee_wad)?)?;
        debt_usd_wad = debt_usd_wad.try_add(debt.value_usd_wad)?;
        debts.push(debt);
    }

    let liquidity = if open.swap {
        let total_usd = usd(total0, token0.price_wad, token0.decimals)?
            .try_add(usd(total1, token1.price_wad, token1.decimals)?)?
            .try_add(outside_pair_usd)?;
        liquidity_for_usd(input, total_usd)?
    } else {
        liquidity_for_amounts(input, total0, total1)?
    };
    let position = CollateralInput { liquidity, ..*input };

    Ok(OpenSetup {
        position,
        debts,
        liquidation_penalty: open.penalty_wad,
        margin_usd_wad,
        debt_usd_wad,
        borrowed,
    })
}

fn open_position(open: &OpenParams) -> Result<Opened> {
    let setup = open_setup(open)?;
    let (amount0, amount1) = amounts_for_liquidity(
        oracle_sqrt_price(&setup.position)?,
        setup.position.sqrt_pa_x96,
        setup.position.sqrt_pb_x96,
        setup.position.liquidity,
    )?;

    let health = health::evaluate(&setup.position, &setup.debts, setup.liquidation_penalty)?;
    let leverage_wad = if setup.margin_usd_wad.is_zero() {
        U256::ZERO
    } else {
        mul_div(health.collateral_usd_wad, WAD, setup.margin_usd_wad)?
    };
    let passes = health.passes();
    let allowed = passes && setup.borrowed <= MAX_DEBTS;

    Ok(Opened {
        summary: OpenSummary {
            liquidity: setup.position.liquidity,
            amount0,
            amount1,
            collateral_usd_wad: health.collateral_usd_wad,
            margin_usd_wad: setup.margin_usd_wad,
            debt_usd_wad: setup.debt_usd_wad,
            leverage_wad,
            health_wad: health.health_wad,
            allowed,
        },
        position: setup.position,
        debts: setup.debts,
        liquidation_penalty: setup.liquidation_penalty,
        passes,
    })
}

/// Liquidity whose value at the oracle price is `usd_wad` (an ideal swap into the range).
///
/// # Errors
///
/// The arithmetic errors of the lens, [`Error::CastOverflow`](crate::Error::CastOverflow) past 128 bits.
pub fn liquidity_for_usd(collateral: &CollateralInput, usd_wad: U256) -> Result<u128> {
    let reference = CollateralInput {
        liquidity: REF_LIQUIDITY,
        ..*collateral
    };
    let value_per_ref = collateral_usd_at(&reference, collateral.token0.price_wad, collateral.token1.price_wad)?;
    if value_per_ref.is_zero() {
        return Ok(0);
    }
    to_u128(mul_div(usd_wad, U256::from(REF_LIQUIDITY), value_per_ref)?)
}

/// Most liquidity `amount0` / `amount1` add at the oracle price without a swap.
///
/// # Errors
///
/// The arithmetic errors of the lens.
pub fn liquidity_for_amounts(collateral: &CollateralInput, amount0: U256, amount1: U256) -> Result<u128> {
    let (ref0, ref1) = amounts_for_liquidity(
        oracle_sqrt_price(collateral)?,
        collateral.sqrt_pa_x96,
        collateral.sqrt_pb_x96,
        REF_LIQUIDITY,
    )?;
    let mut liquidity = U256::from(u128::MAX);
    if !ref0.is_zero() {
        liquidity = liquidity.min(mul_div(amount0, U256::from(REF_LIQUIDITY), ref0)?);
    }
    if !ref1.is_zero() {
        liquidity = liquidity.min(mul_div(amount1, U256::from(REF_LIQUIDITY), ref1)?);
    }
    if ref0.is_zero() && ref1.is_zero() {
        Ok(0)
    } else {
        to_u128(liquidity)
    }
}

/// Debt recorded for borrowing `amount`: amount plus the opening fee, rounded up as the borrowable does.
///
/// # Errors
///
/// [`Error::Overflow`](crate::Error::Overflow) or [`Error::MulDiv`](crate::Error::MulDiv) on overflow.
pub fn with_fee(amount: U256, borrow_fee_wad: U256) -> Result<U256> {
    market_math::debt_with_fee(amount, borrow_fee_wad)
}

/// USD value of `amount` of a token, rounded down.
///
/// # Errors
///
/// [`Error::Overflow`](crate::Error::Overflow) for 78 or more decimals and [`Error::MulDiv`](crate::Error::MulDiv)
/// on overflow.
pub fn usd(amount: U256, price_wad: U256, decimals: u8) -> Result<U256> {
    mul_div(amount, price_wad, pow10(decimals)?)
}

/// The amount of a token worth `usd_wad` at its USD price, rounded down; zero at a zero price.
///
/// # Errors
///
/// [`Error::Overflow`](crate::Error::Overflow) for 78 or more decimals and [`Error::MulDiv`](crate::Error::MulDiv)
/// on overflow.
pub fn amount_for_usd(usd_wad: U256, price_wad: U256, decimals: u8) -> Result<U256> {
    if price_wad.is_zero() {
        return Ok(U256::ZERO);
    }
    mul_div(usd_wad, pow10(decimals)?, price_wad)
}

fn oracle_sqrt_price(collateral: &CollateralInput) -> Result<U160> {
    sqrt_price_x96_from_usd(
        collateral.token0.price_wad,
        collateral.token1.price_wad,
        collateral.token0.decimals,
        collateral.token1.decimals,
    )
}

fn quote(zero_for_one: bool, amount_in: U256, sqrt_price_x96: U160, keep: U256) -> Result<SwapQuote> {
    let net = mul_div(amount_in, keep, WAD)?;
    let amount_out = if zero_for_one {
        to_token1(net, sqrt_price_x96)?
    } else {
        to_token0(net, sqrt_price_x96)?
    };
    Ok(SwapQuote {
        zero_for_one,
        amount_in,
        amount_out,
    })
}

fn to_token1(amount: U256, sqrt_price_x96: U160) -> Result<U256> {
    let sqrt_price = U256::from(sqrt_price_x96);
    mul_shr96(mul_shr96(amount, sqrt_price)?, sqrt_price)
}

fn to_token0(amount: U256, sqrt_price_x96: U160) -> Result<U256> {
    let sqrt_price = U256::from(sqrt_price_x96);
    mul_div(mul_div(amount, Q96, sqrt_price)?, Q96, sqrt_price)
}
