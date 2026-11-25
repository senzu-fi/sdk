//! JavaScript bindings, built by `wasm-pack` into the `ts-sdk/core` package.
//!
//! Each export takes one request object and returns one result object, typed by `tsify`. Integers wider than
//! `u8` are strings (see the crate docs); scalar results are `0x` hex strings. Errors throw a JS `Error` whose
//! message is the [`Error`](crate::Error) text.

use alloy_primitives::{U160, U256};
use serde::{Deserialize, Serialize};
use tsify::{Ts, Tsify};
use wasm_bindgen::prelude::*;

use crate::fees::{self, FeeCheckpoint};
use crate::irm::{self, AdaptiveRate};
use crate::lens::{self, LeverageBorrow, LiquidationPrices, OpenParams, OpenPreview, OpenSummary, SwapQuote};
use crate::liquidation::{self, Liquidation, RepaidDebt};
use crate::market::{self, Accrual, Market, Rounding};
use crate::types::{Asset, CollateralInput, DebtInput, HealthResult};
use crate::{health, slippage, uniswap};

mod charts;
mod flows;
mod metrics;
pub use charts::*;
pub use flows::*;
pub use metrics::*;

#[wasm_bindgen(typescript_custom_section)]
const INTEGER_TYPES: &str = r#"
/** uint256 as a string: "0x"-prefixed hex on output, hex or decimal on input. `BigInt(value)` reads it. */
export type U256 = string;
/** uint160 (sqrt prices) as a string, like U256. */
export type U160 = string;
/** int256 as a decimal string. `BigInt(value)` reads it. */
export type I256 = string;
"#;

fn js_error(error: crate::Error) -> JsError {
    JsError::new(&error.to_string())
}

fn input<T>(value: &Ts<T>) -> Result<T, JsError>
where
    T: Tsify + serde::de::DeserializeOwned,
    T::JsType: Clone,
{
    value.to_rust().map_err(|e| JsError::new(&e.to_string()))
}

fn output<T: Tsify + Serialize>(value: &T) -> Result<Ts<T>, JsError> {
    Ts::from_rust(value).map_err(|e| JsError::new(&e.to_string()))
}

fn hex(value: U256) -> String {
    format!("{value:#x}")
}

fn parse(value: &str) -> Result<U256, JsError> {
    value
        .parse()
        .map_err(|_| JsError::new("expected a decimal or 0x-hex integer"))
}

/// Inputs of a health check.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct HealthRequest {
    /// The position.
    pub collateral: CollateralInput,
    /// Its debts, valued in USD.
    pub debts: Vec<DebtInput>,
    /// The position's liquidation penalty, e.g. 1.08e18: the highest of its pair's tokens and its debts outside the
    /// pair (the lens's `penaltyWad`).
    pub penalty_wad: U256,
}

/// Four-corner health check of a position (`HealthMath.evaluate`).
///
/// # Errors
///
/// Throws where the contract reverts.
#[wasm_bindgen(js_name = evaluateHealth)]
pub fn evaluate_health(request: &Ts<HealthRequest>) -> Result<Ts<HealthResult>, JsError> {
    let request = input(request)?;
    output(&health::evaluate(&request.collateral, &request.debts, request.penalty_wad).map_err(js_error)?)
}

/// A debt of `amount` of `asset` against a position, valued as the collateral values it.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct DebtRequest {
    /// The position.
    pub collateral: CollateralInput,
    /// The borrowed asset.
    pub asset: Asset,
    /// Amount owed, in the asset's units.
    pub amount: U256,
}

/// The [`DebtInput`] of a debt (`HealthLogic._debtInput`).
///
/// # Errors
///
/// Throws on overflow.
#[wasm_bindgen(js_name = debtInput)]
pub fn debt_input(request: &Ts<DebtRequest>) -> Result<Ts<DebtInput>, JsError> {
    let request = input(request)?;
    output(
        &request
            .collateral
            .debt(&request.asset, request.amount)
            .map_err(js_error)?,
    )
}

/// Uniswap sqrt price (Q64.96, hex) of token0 in token1 from two USD prices. A zero `price1Wad` gives `MAX_SQRT_PRICE`.
///
/// # Errors
///
/// Throws on overflow.
#[wasm_bindgen(js_name = sqrtPriceX96FromUsd)]
pub fn sqrt_price_x96_from_usd(
    price0_wad: &str,
    price1_wad: &str,
    decimals0: u8,
    decimals1: u8,
) -> Result<String, JsError> {
    let sqrt_price = health::sqrt_price_x96_from_usd(parse(price0_wad)?, parse(price1_wad)?, decimals0, decimals1)
        .map_err(js_error)?;
    Ok(hex(U256::from(sqrt_price)))
}

/// Sqrt price (Q64.96, hex) at a tick.
///
/// # Errors
///
/// Throws outside `[MIN_TICK, MAX_TICK]`.
#[wasm_bindgen(js_name = sqrtPriceAtTick)]
pub fn sqrt_price_at_tick(tick: i32) -> Result<String, JsError> {
    Ok(hex(U256::from(uniswap::sqrt_price_at_tick(tick).map_err(js_error)?)))
}

/// Greatest tick whose sqrt price is at most `sqrtPriceX96`.
///
/// # Errors
///
/// Throws outside `[MIN_SQRT_PRICE, MAX_SQRT_PRICE)`.
#[wasm_bindgen(js_name = tickAtSqrtPrice)]
pub fn tick_at_sqrt_price(sqrt_price_x96: &str) -> Result<i32, JsError> {
    uniswap::tick_at_sqrt_price(sqrt_price(sqrt_price_x96)?).map_err(js_error)
}

/// A range by its ends, as Q64.96 sqrt prices (either order).
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct SqrtRange {
    /// One end.
    pub sqrt_pa_x96: U160,
    /// The other end.
    pub sqrt_pb_x96: U160,
}

/// A range and the liquidity in it. A `CollateralInput` is one too: its other fields are ignored.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityRange {
    /// One end, a Q64.96 sqrt price.
    pub sqrt_pa_x96: U160,
    /// The other end.
    pub sqrt_pb_x96: U160,
    /// The liquidity in the range.
    #[serde(with = "crate::serde_num")]
    #[tsify(type = "string")]
    pub liquidity: u128,
}

/// Token amounts of a liquidity amount in a range.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct TokenAmounts {
    /// Token0.
    pub amount0: U256,
    /// Token1.
    pub amount1: U256,
}

fn amounts((amount0, amount1): (U256, U256)) -> TokenAmounts {
    TokenAmounts { amount0, amount1 }
}

fn sqrt_price(value: &str) -> Result<U160, JsError> {
    U160::checked_from_limbs_slice(parse(value)?.as_limbs())
        .ok_or_else(|| JsError::new("sqrt price does not fit in 160 bits"))
}

/// Token amounts `range.liquidity` holds at `sqrtPriceX96`, rounded down (`LiquidityAmounts.getAmountsForLiquidity`).
///
/// # Errors
///
/// Throws for a zero lower price.
#[wasm_bindgen(js_name = amountsForLiquidity)]
pub fn amounts_for_liquidity(range: &Ts<LiquidityRange>, sqrt_price_x96: &str) -> Result<Ts<TokenAmounts>, JsError> {
    let range = input(range)?;
    let (amount0, amount1) = uniswap::amounts_for_liquidity(
        sqrt_price(sqrt_price_x96)?,
        range.sqrt_pa_x96,
        range.sqrt_pb_x96,
        range.liquidity,
    )
    .map_err(js_error)?;
    output(&TokenAmounts { amount0, amount1 })
}

/// Most liquidity `amount0` and `amount1` can add in `range` at `sqrtPriceX96`, rounded down, as hex
/// (`LiquidityAmounts.getLiquidityForAmounts`).
///
/// # Errors
///
/// Throws for an empty range, or when the liquidity does not fit in 128 bits.
#[wasm_bindgen(js_name = liquidityForAmounts)]
pub fn liquidity_for_amounts(
    range: &Ts<SqrtRange>,
    sqrt_price_x96: &str,
    amount0: &str,
    amount1: &str,
) -> Result<String, JsError> {
    let range = input(range)?;
    let liquidity = uniswap::liquidity_for_amounts(
        sqrt_price(sqrt_price_x96)?,
        range.sqrt_pa_x96,
        range.sqrt_pb_x96,
        parse(amount0)?,
        parse(amount1)?,
    )
    .map_err(js_error)?;
    Ok(hex(U256::from(liquidity)))
}

/// A deposit of up to `amount0` and `amount1` in a range, at a pool price, within a slippage.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct DepositBand {
    /// One end of the range, a Q64.96 sqrt price.
    pub sqrt_pa_x96: U160,
    /// The other end.
    pub sqrt_pb_x96: U160,
    /// Most token0 the deposit may take.
    pub amount0: U256,
    /// Most token1 the deposit may take.
    pub amount1: U256,
    /// The pool's sqrt price now.
    pub sqrt_price_x96: U160,
    /// Accepted pool price move, in basis points (50 = 0.5%).
    #[serde(with = "crate::serde_num")]
    #[tsify(type = "string")]
    pub slippage_bps: u64,
}

/// Liquidity in a range, at a pool price, within a slippage.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct LiquidityBand {
    /// One end of the range, a Q64.96 sqrt price.
    pub sqrt_pa_x96: U160,
    /// The other end.
    pub sqrt_pb_x96: U160,
    /// The liquidity minted or redeemed.
    #[serde(with = "crate::serde_num")]
    #[tsify(type = "string")]
    pub liquidity: u128,
    /// The pool's sqrt price now.
    pub sqrt_price_x96: U160,
    /// Accepted pool price move, in basis points (50 = 0.5%).
    #[serde(with = "crate::serde_num")]
    #[tsify(type = "string")]
    pub slippage_bps: u64,
}

/// What liquidity holds at each end of the slippage band.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct BandEnds {
    /// At the lowest price.
    pub low: TokenAmounts,
    /// At the highest price.
    pub high: TokenAmounts,
}

/// The least of each token the deposit takes anywhere in the band: the router's minimums for an add.
///
/// # Errors
///
/// Throws for a slippage above 100% or an empty range.
#[wasm_bindgen(js_name = depositMinimums)]
pub fn deposit_minimums(band: &Ts<DepositBand>) -> Result<Ts<TokenAmounts>, JsError> {
    let b = input(band)?;
    let result = slippage::deposit_minimums(
        b.sqrt_pa_x96,
        b.sqrt_pb_x96,
        b.sqrt_price_x96,
        b.amount0,
        b.amount1,
        b.slippage_bps,
    );
    output(&amounts(result.map_err(js_error)?))
}

/// The most of each token minting the liquidity takes anywhere in the band, plus one wei: the router's maximums.
///
/// # Errors
///
/// Throws for a slippage above 100%.
#[wasm_bindgen(js_name = depositMaximums)]
pub fn deposit_maximums(band: &Ts<LiquidityBand>) -> Result<Ts<TokenAmounts>, JsError> {
    let b = input(band)?;
    let result = slippage::deposit_maximums(
        b.sqrt_pa_x96,
        b.sqrt_pb_x96,
        b.sqrt_price_x96,
        b.liquidity,
        b.slippage_bps,
    );
    output(&amounts(result.map_err(js_error)?))
}

/// What the liquidity holds at the band's low and high ends.
///
/// # Errors
///
/// Throws for a slippage above 100%.
#[wasm_bindgen(js_name = withdrawEnds)]
pub fn withdraw_ends(band: &Ts<LiquidityBand>) -> Result<Ts<BandEnds>, JsError> {
    let b = input(band)?;
    let [low, high] = slippage::withdraw_ends(
        b.sqrt_pa_x96,
        b.sqrt_pb_x96,
        b.sqrt_price_x96,
        b.liquidity,
        b.slippage_bps,
    )
    .map_err(js_error)?;
    output(&BandEnds {
        low: amounts(low),
        high: amounts(high),
    })
}

/// The least of each token redeeming the liquidity pays anywhere in the band: the router's minimums for a redeem.
///
/// # Errors
///
/// Throws for a slippage above 100%.
#[wasm_bindgen(js_name = withdrawMinimums)]
pub fn withdraw_minimums(band: &Ts<LiquidityBand>) -> Result<Ts<TokenAmounts>, JsError> {
    let b = input(band)?;
    let result = slippage::withdraw_minimums(
        b.sqrt_pa_x96,
        b.sqrt_pb_x96,
        b.sqrt_price_x96,
        b.liquidity,
        b.slippage_bps,
    );
    output(&amounts(result.map_err(js_error)?))
}

/// Inputs of a liquidation price search.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct LiquidationPricesRequest {
    /// The position.
    pub collateral: CollateralInput,
    /// Its debts.
    pub debts: Vec<DebtInput>,
    /// The position's liquidation penalty, e.g. 1.08e18: the highest of its pair's tokens and its debts outside the
    /// pair (the lens's `penaltyWad`).
    pub penalty_wad: U256,
}

/// Liquidation prices of a position (`SenzuLens._liquidationPrices`).
///
/// # Errors
///
/// Throws where the lens reverts.
#[wasm_bindgen(js_name = liquidationPrices)]
pub fn liquidation_prices(request: &Ts<LiquidationPricesRequest>) -> Result<Ts<LiquidationPrices>, JsError> {
    let request = input(request)?;
    output(&lens::liquidation_prices(&request.collateral, &request.debts, request.penalty_wad).map_err(js_error)?)
}

/// Inputs of a max borrow search.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct MaxBorrowRequest {
    /// The position.
    pub collateral: CollateralInput,
    /// Its debts in other assets (its debt in `asset` comes from `account_shares`).
    pub other_debts: Vec<DebtInput>,
    /// The asset to borrow.
    pub asset: Asset,
    /// Its market, accrued to now.
    pub market: Market,
    /// The position's borrow shares in `market`.
    pub account_shares: U256,
    /// The position's liquidation penalty, e.g. 1.08e18: the highest of its pair's tokens and its debts outside the
    /// pair (the lens's `penaltyWad`).
    pub penalty_wad: U256,
    /// The deployment's opening fee.
    pub borrow_fee_wad: U256,
}

/// Most of `asset` the position can borrow now (`SenzuLens.maxBorrow`), hex.
///
/// # Errors
///
/// Throws where the lens reverts.
#[wasm_bindgen(js_name = maxBorrow)]
pub fn max_borrow(request: &Ts<MaxBorrowRequest>) -> Result<String, JsError> {
    let request = input(request)?;
    let amount = lens::max_borrow(
        &request.collateral,
        &request.other_debts,
        &request.asset,
        &request.market,
        request.account_shares,
        request.penalty_wad,
        request.borrow_fee_wad,
    )
    .map_err(js_error)?;
    Ok(hex(amount))
}

/// Inputs of a max leverage search.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct MaxLeverageRequest {
    /// The pair, range and prices (`liquidity` ignored).
    pub collateral: CollateralInput,
    /// How the position borrows.
    pub borrow: LeverageBorrow,
    /// The position's liquidation penalty, e.g. 1.08e18: the highest of its pair's tokens and its debts outside the
    /// pair (the lens's `penaltyWad`).
    pub penalty_wad: U256,
    /// The deployment's opening fee.
    pub borrow_fee_wad: U256,
}

/// Highest leverage (WAD, hex) a new position in the range passes the borrow check with.
///
/// # Errors
///
/// Throws where the lens reverts.
#[wasm_bindgen(js_name = maxLeverage)]
pub fn max_leverage(request: &Ts<MaxLeverageRequest>) -> Result<String, JsError> {
    let request = input(request)?;
    let leverage = lens::max_leverage(
        &request.collateral,
        &request.borrow,
        request.penalty_wad,
        request.borrow_fee_wad,
    )
    .map_err(js_error)?;
    Ok(hex(leverage))
}

/// Inputs of [`swap_for_range`].
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct SwapRequest {
    /// One end of the range.
    pub sqrt_pa_x96: U160,
    /// The other end.
    pub sqrt_pb_x96: U160,
    /// Token0 available.
    pub amount0: U256,
    /// Token1 available.
    pub amount1: U256,
    /// Price the swap executes at.
    pub sqrt_price_x96: U160,
    /// Pool fee, 1e18 = 100%.
    pub swap_fee_wad: U256,
}

/// The swap that fits two amounts to the range's ratio (`SenzuLens.swapForRange`).
///
/// # Errors
///
/// Throws where the lens reverts.
#[wasm_bindgen(js_name = swapForRange)]
pub fn swap_for_range(request: &Ts<SwapRequest>) -> Result<Ts<SwapQuote>, JsError> {
    let request = input(request)?;
    let quote = lens::swap_for_range(
        request.sqrt_pa_x96,
        request.sqrt_pb_x96,
        request.amount0,
        request.amount1,
        request.sqrt_price_x96,
        request.swap_fee_wad,
    )
    .map_err(js_error)?;
    output(&quote)
}

/// The Create Position preview (`SenzuLens.previewOpen`).
///
/// # Errors
///
/// Throws where the lens reverts.
#[wasm_bindgen(js_name = previewOpen)]
pub fn preview_open(params: &Ts<OpenParams>) -> Result<Ts<OpenPreview>, JsError> {
    output(&lens::preview_open(&input(params)?).map_err(js_error)?)
}

/// [`previewOpen`](preview_open) without the liquidation prices: the cheap part, for live updates.
///
/// # Errors
///
/// Throws where the lens reverts.
#[wasm_bindgen(js_name = previewOpenSummary)]
pub fn preview_open_summary(params: &Ts<OpenParams>) -> Result<Ts<OpenSummary>, JsError> {
    output(&lens::preview_open_summary(&input(params)?).map_err(js_error)?)
}

/// The liquidation prices of [`previewOpen`](preview_open) alone.
///
/// # Errors
///
/// Throws where the lens reverts.
#[wasm_bindgen(js_name = openLiquidationPrices)]
pub fn open_liquidation_prices(params: &Ts<OpenParams>) -> Result<Ts<LiquidationPrices>, JsError> {
    output(&lens::open_liquidation_prices(&input(params)?).map_err(js_error)?)
}

/// Inputs of a liquidation.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct LiquidateRequest {
    /// The borrower's position, with `liquidity` for the borrower's `units`.
    pub collateral: CollateralInput,
    /// All its debts.
    pub debts: Vec<DebtInput>,
    /// The debt being repaid.
    pub repaid: RepaidDebt,
    /// The borrower's balance of the position id.
    pub units: U256,
    /// The most the liquidator repays ("0xff…ff" for as much as allowed).
    pub max_repay_assets: U256,
    /// The position's liquidation penalty, e.g. 1.08e18: the highest of its pair's tokens and its debts outside the
    /// pair (the lens's `penaltyWad`), for the health check.
    pub penalty_wad: U256,
    /// The repaid debt's penalty, what the liquidator seizes at: the higher of its pair's tokens, or the asset's own
    /// when it's outside the pair and higher.
    pub repay_penalty_wad: U256,
    /// The DEX's protocol liquidation fee share of the bonus (1e18 = 100%, max 0.2e18; default 0).
    #[serde(default)]
    pub liquidation_fee_wad: U256,
    /// Whether the position owes another market besides `repaid` (default false).
    #[serde(default)]
    pub has_other_debt: bool,
}

/// Repay, seize and fee amounts of a liquidation (`LiquidationLogic.liquidate`).
///
/// # Errors
///
/// Throws where the contract reverts (e.g. `position is not liquidatable`).
#[wasm_bindgen]
pub fn liquidate(request: &Ts<LiquidateRequest>) -> Result<Ts<Liquidation>, JsError> {
    let request = input(request)?;
    let health = health::evaluate(&request.collateral, &request.debts, request.penalty_wad).map_err(js_error)?;
    let liquidation = liquidation::liquidate(
        &health,
        &request.repaid,
        request.units,
        request.max_repay_assets,
        request.repay_penalty_wad,
        request.liquidation_fee_wad,
        request.has_other_debt,
    )
    .map_err(js_error)?;
    output(&liquidation)
}

/// Inputs of an accrual.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct AccrueRequest {
    /// The market as last written on chain.
    pub market: Market,
    /// ERC-6909 total supply of the market's lender id.
    pub total_supply_shares: U256,
    /// Timestamp to accrue to.
    #[tsify(type = "string")]
    #[serde(with = "crate::serde_num")]
    pub now: u64,
}

/// A market after accrual, with its rates.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct AccrueResult {
    /// The market accrued to `now`.
    pub market: Market,
    /// What the accrual booked.
    pub accrual: Accrual,
    /// Borrow rate per second after accrual.
    pub borrow_rate_per_second: U256,
    /// Supply rate per second after accrual.
    pub supply_rate_per_second: U256,
}

/// Accrues a market to `now` (`MarketLogic.accrue`), as the borrowable's views do: the exact integral of the
/// rate, compounded, the rate at target adapted; the rates returned are the instantaneous ones as of `now`.
///
/// # Errors
///
/// Throws where the contract reverts.
#[wasm_bindgen]
pub fn accrue(request: &Ts<AccrueRequest>) -> Result<Ts<AccrueResult>, JsError> {
    let request = input(request)?;
    let mut market = request.market;
    let accrual = market
        .accrue(request.total_supply_shares, request.now)
        .map_err(js_error)?;
    let borrow_rate_per_second = market.borrow_rate().map_err(js_error)?;
    let supply_rate_per_second = market.supply_rate(borrow_rate_per_second).map_err(js_error)?;
    output(&AccrueResult {
        market,
        accrual,
        borrow_rate_per_second,
        supply_rate_per_second,
    })
}

/// Inputs of [`adaptive_rate`].
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct AdaptiveRateRequest {
    /// Rate at target at the start of the period, per second.
    pub rate_at_target: U256,
    /// Utilization at the start of the period, 1e18 = 100%.
    pub utilization: U256,
    /// Length of the period in seconds.
    pub elapsed: U256,
}

/// The borrow rate integrated over a period and the rate at target it ends at (`MarketLogic.adaptiveRate`), e.g.
/// to chart where a market's rate heads if its utilization holds. The debt grows `e^integral` times.
///
/// # Errors
///
/// Throws on overflow.
#[wasm_bindgen(js_name = adaptiveRate)]
pub fn adaptive_rate(request: &Ts<AdaptiveRateRequest>) -> Result<Ts<AdaptiveRate>, JsError> {
    let request = input(request)?;
    let rate = irm::adaptive_rate(request.rate_at_target, request.utilization, request.elapsed).map_err(js_error)?;
    output(&rate)
}

/// The opening fee on a borrow of `assets`, rounded up as the contract charges it, hex.
///
/// # Errors
///
/// Throws on overflow.
#[wasm_bindgen(js_name = borrowFee)]
pub fn borrow_fee(assets: &str, borrow_fee_wad: &str) -> Result<String, JsError> {
    Ok(hex(
        market::borrow_fee(parse(assets)?, parse(borrow_fee_wad)?).map_err(js_error)?
    ))
}

/// What a holder is paid of `amount` of fees or rewards owed, the protocol's cut taken (rounded down), hex.
///
/// # Errors
///
/// Throws on overflow or a cut over 100%.
#[wasm_bindgen(js_name = afterCut)]
pub fn after_cut(amount: &str, cut_wad: &str) -> Result<String, JsError> {
    Ok(hex(fees::after_cut(parse(amount)?, parse(cut_wad)?).map_err(js_error)?))
}

/// What a borrow of `assets` owes, its opening fee rounded up as the contract charges it, hex.
///
/// # Errors
///
/// Throws on overflow.
#[wasm_bindgen(js_name = debtWithFee)]
pub fn debt_with_fee(assets: &str, borrow_fee_wad: &str) -> Result<String, JsError> {
    Ok(hex(
        market::debt_with_fee(parse(assets)?, parse(borrow_fee_wad)?).map_err(js_error)?
    ))
}

/// `assets → shares` with the markets' virtual shares, hex.
///
/// # Errors
///
/// Throws on overflow.
#[wasm_bindgen(js_name = toShares)]
pub fn to_shares(assets: &str, total_assets: &str, total_shares: &str, round_up: bool) -> Result<String, JsError> {
    let rounding = if round_up { Rounding::Up } else { Rounding::Down };
    Ok(hex(market::to_shares(
        parse(assets)?,
        parse(total_assets)?,
        parse(total_shares)?,
        rounding,
    )
    .map_err(js_error)?))
}

/// `shares → assets` with the markets' virtual shares, hex.
///
/// # Errors
///
/// Throws on overflow.
#[wasm_bindgen(js_name = toAssets)]
pub fn to_assets(shares: &str, total_assets: &str, total_shares: &str, round_up: bool) -> Result<String, JsError> {
    let rounding = if round_up { Rounding::Up } else { Rounding::Down };
    Ok(hex(market::to_assets(
        parse(shares)?,
        parse(total_assets)?,
        parse(total_shares)?,
        rounding,
    )
    .map_err(js_error)?))
}

/// Inputs of [`fees_pending`].
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct FeesRequest {
    /// The holder's checkpoint.
    pub checkpoint: FeeCheckpoint,
    /// The holder's units.
    pub balance: U256,
    /// Current token0 fee growth inside the range.
    pub fee_growth0_x128: U256,
    /// Current token1 fee growth inside the range.
    pub fee_growth1_x128: U256,
}

/// LP fees owed to a holder (`FeeLogic.pending`).
///
/// # Errors
///
/// Throws on overflow.
#[wasm_bindgen(js_name = feesPending)]
pub fn fees_pending(request: &Ts<FeesRequest>) -> Result<Ts<TokenAmounts>, JsError> {
    let request = input(request)?;
    let (amount0, amount1) = request
        .checkpoint
        .pending(request.balance, request.fee_growth0_x128, request.fee_growth1_x128)
        .map_err(js_error)?;
    output(&TokenAmounts { amount0, amount1 })
}
