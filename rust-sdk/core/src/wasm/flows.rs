//! The plan math of [`crate::flows`], one export per function.

use alloy_primitives::{U160, U256};
use serde::{Deserialize, Serialize};
use tsify::{Ts, Tsify};
use wasm_bindgen::prelude::*;

use super::{TokenAmounts, amounts, hex, input, js_error, output, parse, sqrt_price};
use crate::flows::close::{self, CloseAmounts, ClosingDebt, ClosingPosition, DebtSwap, MarketPrice};
use crate::flows::edit::{self, EditPosition, EditPreview, EditState, LeverageChange, RedeemShare, Redeemable};
use crate::flows::lend::{self, Withdrawal};
use crate::flows::liquidate::{self, Liquidated, LiquidationAmounts, LiquidationUsd, RankedDebt};
use crate::flows::open::{self, BorrowSplit, ExactOpen, OpenAmounts, OpenSizing, PricedRange, RangeSwap};
use crate::flows::range::{self, LiquidationPoolPrices, TickDirection};
use crate::flows::{PerToken, Priced};
use crate::slippage;
use crate::types::{CollateralInput, Side};

/// A slippage in basis points, from a decimal string.
fn bps(value: &str) -> Result<u64, JsError> {
    value
        .parse()
        .map_err(|_| JsError::new("slippageBps must be a decimal integer"))
}

/// The quote a minimum was written from within `slippageBps`, as hex.
///
/// # Errors
///
/// Throws for a slippage of 100% or more.
#[wasm_bindgen(js_name = quoteForMin)]
pub fn quote_for_min(min_out: &str, slippage_bps: &str) -> Result<String, JsError> {
    Ok(hex(
        slippage::quote_for_min(parse(min_out)?, bps(slippage_bps)?).map_err(js_error)?
    ))
}

/// What to pay for `wanted` at a quote's rate, rounded up, as hex.
///
/// # Errors
///
/// Throws on absurd amounts.
#[wasm_bindgen(js_name = inputForOutput)]
pub fn input_for_output(wanted: &str, amount_in: &str, amount_out: &str) -> Result<String, JsError> {
    Ok(hex(slippage::input_for_output(
        parse(wanted)?,
        parse(amount_in)?,
        parse(amount_out)?,
    )
    .map_err(js_error)?))
}

/// What withdrawing a whole deposit can take now.
///
/// # Errors
///
/// Throws on absurd values.
#[wasm_bindgen(js_name = withdrawAll)]
pub fn withdraw_all(
    deposit_assets: &str,
    deposit_shares: &str,
    max_withdraw: &str,
) -> Result<Ts<lend::WithdrawAll>, JsError> {
    output(&lend::withdraw_all(parse(deposit_assets)?, parse(deposit_shares)?, parse(max_withdraw)?).map_err(js_error)?)
}

/// The least a swap quoted at `amount` may pay within `slippageBps`, as hex.
///
/// # Errors
///
/// Throws for a slippage above 100%.
#[wasm_bindgen(js_name = minOut)]
pub fn min_out(amount: &str, slippage_bps: &str) -> Result<String, JsError> {
    Ok(hex(
        slippage::min_out(parse(amount)?, bps(slippage_bps)?).map_err(js_error)?
    ))
}

/// The pair's market prices and the pool's price.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct PoolValuedRequest {
    /// Token0's market.
    pub token0: MarketPrice,
    /// Token1's market.
    pub token1: MarketPrice,
    /// The pool's sqrt price now.
    pub pool_sqrt_price_x96: U160,
}

/// The pair's prices for sizing a close, a token without a live price valued at the pool.
///
/// # Errors
///
/// Throws for a price that does not fit in 256 bits.
#[wasm_bindgen(js_name = poolValuedPrices)]
pub fn pool_valued_prices(request: &Ts<PoolValuedRequest>) -> Result<Ts<PerToken<U256>>, JsError> {
    let r = input(request)?;
    let (token0, token1) = close::pool_valued_prices(r.token0, r.token1, r.pool_sqrt_price_x96).map_err(js_error)?;
    output(&PerToken { token0, token1 })
}

/// What a deleverage holds and repays.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct SwapIntoDebtsRequest {
    /// What the redeem surely paid.
    pub held: PerToken<U256>,
    /// What is repaid of each debt.
    pub repays: PerToken<U256>,
    /// The pair's prices.
    pub prices: PerToken<Priced>,
    /// Accepted price move, in basis points.
    #[serde(with = "crate::serde_num")]
    #[tsify(type = "string")]
    pub slippage_bps: u64,
    /// The pool's fee, 1e18 = 100%.
    pub swap_fee_wad: U256,
}

/// The swaps of a deleverage, in order.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct DebtSwaps {
    /// Token0's debt first.
    pub swaps: Vec<DebtSwap>,
}

/// The swaps that turn what a deleverage's redeem paid into the debts it repays.
///
/// # Errors
///
/// Throws for a zero price.
#[wasm_bindgen(js_name = swapIntoDebts)]
pub fn swap_into_debts(request: &Ts<SwapIntoDebtsRequest>) -> Result<Ts<DebtSwaps>, JsError> {
    let r = input(request)?;
    let swaps = close::swap_into_debts(r.held, r.repays, r.prices, r.slippage_bps, r.swap_fee_wad).map_err(js_error)?;
    output(&DebtSwaps { swaps })
}

/// A position being closed, its debts and the pair's prices.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct CloseRequest {
    /// The position.
    pub position: ClosingPosition,
    /// Its debts, in order.
    pub debts: Vec<ClosingDebt>,
    /// The pair's prices (`poolValuedPrices` when one is not live); null without a market.
    pub prices: PerToken<Option<Priced>>,
    /// Accepted price move, in basis points.
    #[serde(with = "crate::serde_num")]
    #[tsify(type = "string")]
    pub slippage_bps: u64,
    /// The pool's fee, 1e18 = 100%.
    pub swap_fee_wad: U256,
}

/// The numbers of a close: the redeem's minimums and a swap for each debt it cannot cover.
///
/// # Errors
///
/// Throws for a slippage above 100%.
#[wasm_bindgen(js_name = closeAmounts)]
pub fn close_amounts(request: &Ts<CloseRequest>) -> Result<Ts<CloseAmounts>, JsError> {
    let r = input(request)?;
    output(&close::close_amounts(&r.position, &r.debts, r.prices, r.slippage_bps, r.swap_fee_wad).map_err(js_error)?)
}

/// What closing sends back, per token, before slippage and the plan's headroom (`slippageBps` is ignored).
///
/// # Errors
///
/// Throws for a zero price.
#[wasm_bindgen(js_name = estimateReceive)]
pub fn estimate_receive(request: &Ts<CloseRequest>) -> Result<Ts<TokenAmounts>, JsError> {
    let r = input(request)?;
    output(&amounts(
        close::estimate_receive(&r.position, &r.debts, r.prices, r.swap_fee_wad).map_err(js_error)?,
    ))
}

/// A position and a slippage.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct CompoundRequest {
    /// The position.
    pub position: ClosingPosition,
    /// Accepted price move, in basis points.
    #[serde(with = "crate::serde_num")]
    #[tsify(type = "string")]
    pub slippage_bps: u64,
}

/// The minimums of compounding the position's owed fees.
///
/// # Errors
///
/// Throws for a slippage above 100%.
#[wasm_bindgen(js_name = compoundMinimums)]
pub fn compound_minimums(request: &Ts<CompoundRequest>) -> Result<Ts<TokenAmounts>, JsError> {
    let r = input(request)?;
    output(&amounts(
        close::compound_minimums(&r.position, r.slippage_bps).map_err(js_error)?,
    ))
}

/// A position's debts, for picking one to liquidate.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct LargestDebtRequest {
    /// In the position's order.
    pub debts: Vec<RankedDebt>,
}

/// The index of the debt worth the most (a tie goes to the later one), or -1 when nothing is owed.
///
/// # Errors
///
/// Throws for a malformed request.
#[wasm_bindgen(js_name = largestDebt)]
pub fn largest_debt(request: &Ts<LargestDebtRequest>) -> Result<i32, JsError> {
    let r = input(request)?;
    Ok(liquidate::largest_debt(&r.debts)
        .and_then(|i| i32::try_from(i).ok())
        .unwrap_or(-1))
}

/// What a liquidation of one debt seizes, as the contract sizes it.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct SeizedShareRequest {
    /// The collateral's value in USD (1e18).
    pub collateral_usd_wad: U256,
    /// All the position's debt in USD.
    pub debt_usd_wad: U256,
    /// The repaid debt's value in USD.
    pub repaid_usd_wad: U256,
    /// The repaid debt's penalty, or the position's.
    pub penalty_wad: U256,
    /// The DEX's liquidation fee share of the bonus (1e18 = 100%, max 0.2e18; default 0).
    #[serde(default)]
    pub liquidation_fee_wad: U256,
    /// Whether the position owes another debt besides the repaid one (default: `debtUsdWad > repaidUsdWad`).
    #[serde(default)]
    pub has_other_debt: Option<bool>,
}

/// The share of the liquidity (1e18 = all) a repayment seizes, taken 0.5% low, as hex.
///
/// # Errors
///
/// Throws on absurd values.
#[wasm_bindgen(js_name = seizedShareWad)]
pub fn seized_share_wad(request: &Ts<SeizedShareRequest>) -> Result<String, JsError> {
    let r = input(request)?;
    let share = liquidate::seized_share_wad(r.collateral_usd_wad, r.debt_usd_wad, r.repaid_usd_wad, r.penalty_wad);
    Ok(hex(share.map_err(js_error)?))
}

/// The debt (USD, 1e18) a liquidation writes off, as hex: zero unless it seizes every unit.
///
/// # Errors
///
/// Throws on absurd values.
#[wasm_bindgen(js_name = writtenOffUsdWad)]
pub fn written_off_usd_wad(request: &Ts<SeizedShareRequest>) -> Result<String, JsError> {
    let r = input(request)?;
    let off = liquidate::written_off_usd_wad(r.collateral_usd_wad, r.debt_usd_wad, r.repaid_usd_wad, r.penalty_wad);
    Ok(hex(off.map_err(js_error)?))
}

/// Full USD breakdown of a liquidation: seized, repaid, protocol fee, liquidator's net incentive, and write-off.
///
/// # Errors
///
/// Throws on absurd values.
#[wasm_bindgen(js_name = liquidationUsd)]
pub fn liquidation_usd(request: &Ts<SeizedShareRequest>) -> Result<Ts<LiquidationUsd>, JsError> {
    let r = input(request)?;
    let has_other = r.has_other_debt.unwrap_or_else(|| r.debt_usd_wad > r.repaid_usd_wad);
    output(
        &liquidate::liquidation_usd(
            r.collateral_usd_wad,
            r.debt_usd_wad,
            r.repaid_usd_wad,
            r.penalty_wad,
            r.liquidation_fee_wad,
            has_other,
        )
        .map_err(js_error)?,
    )
}

/// The borrower's fees owed and the share of its units a liquidation seizes.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct SeizedFeesRequest {
    /// The borrower's fees owed (`feesOwed`).
    pub fees_owed: PerToken<U256>,
    /// The share seized (1e18 = all), as `seizedShareWad` gives it.
    pub seized_share_wad: U256,
}

/// The fees that go with the seized units to the liquidator, rounded down.
///
/// # Errors
///
/// Throws on absurd values.
#[wasm_bindgen(js_name = seizedFees)]
pub fn seized_fees(request: &Ts<SeizedFeesRequest>) -> Result<Ts<PerToken<U256>>, JsError> {
    let r = input(request)?;
    output(&liquidate::seized_fees(r.fees_owed, r.seized_share_wad).map_err(js_error)?)
}

/// A position being liquidated, within a slippage.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct LiquidationRequest {
    /// The position and the debt repaid.
    pub position: Liquidated,
    /// Accepted price move, in basis points.
    #[serde(with = "crate::serde_num")]
    #[tsify(type = "string")]
    pub slippage_bps: u64,
}

/// The numbers of a liquidation: the liquidity seized, the redeem's minimums and the sale into the debt asset.
///
/// # Errors
///
/// Throws for a slippage above 100%.
#[wasm_bindgen(js_name = liquidationAmounts)]
pub fn liquidation_amounts(request: &Ts<LiquidationRequest>) -> Result<Ts<LiquidationAmounts>, JsError> {
    let r = input(request)?;
    output(&liquidate::liquidation_amounts(&r.position, r.slippage_bps).map_err(js_error)?)
}

/// A liquidation's sale and its quote.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct LiquidationMinOutRequest {
    /// From `liquidationAmounts`.
    pub amounts: LiquidationAmounts,
    /// The quote for selling `amounts.sellAmount`; without one the pool price is used.
    pub quoted_out: Option<U256>,
    /// The pool's sqrt price now.
    pub pool_sqrt_price_x96: U160,
    /// The pool's fee, 1e18 = 100%.
    pub swap_fee_wad: U256,
    /// Accepted price move, in basis points.
    #[serde(with = "crate::serde_num")]
    #[tsify(type = "string")]
    pub slippage_bps: u64,
}

/// The liquidation sale's least output, as hex.
///
/// # Errors
///
/// Throws for a slippage above 100%.
#[wasm_bindgen(js_name = liquidationMinOut)]
pub fn liquidation_min_out(request: &Ts<LiquidationMinOutRequest>) -> Result<String, JsError> {
    let r = input(request)?;
    let out = liquidate::liquidation_min_out(
        &r.amounts,
        r.quoted_out,
        r.pool_sqrt_price_x96,
        r.swap_fee_wad,
        r.slippage_bps,
    );
    Ok(hex(out.map_err(js_error)?))
}

/// A range with prices, at a price.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct PairShareRequest {
    /// The range and the pair's oracle prices.
    pub range: PricedRange,
    /// Where to value it, usually the pool's sqrt price.
    pub sqrt_price_x96: U160,
}

/// Token0's share of the range's value, out of 1e10 (basis points with six decimals), as hex; undefined for a range
/// that holds nothing.
///
/// # Errors
///
/// Throws for an invalid range.
#[wasm_bindgen(js_name = pairToken0Share)]
pub fn pair_token0_share(request: &Ts<PairShareRequest>) -> Result<Option<String>, JsError> {
    let r = input(request)?;
    Ok(open::pair_token0_share(&r.range, r.sqrt_price_x96)
        .map_err(js_error)?
        .map(hex))
}

/// The margin in each token and the pair's prices.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct MarginRequest {
    /// The pair's oracle prices.
    pub prices: PerToken<Priced>,
    /// Margin in token units.
    pub margin: PerToken<U256>,
}

/// The margin's value in USD (1e18), as hex.
///
/// # Errors
///
/// Throws on absurd amounts.
#[wasm_bindgen(js_name = marginUsd)]
pub fn margin_usd(request: &Ts<MarginRequest>) -> Result<String, JsError> {
    let r = input(request)?;
    Ok(hex(open::margin_usd(&r.prices, &r.margin).map_err(js_error)?))
}

/// What an open borrows, split how.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct BorrowAmountsRequest {
    /// The range and the pair's oracle prices.
    pub range: PricedRange,
    /// Margin in token units.
    pub margin: PerToken<U256>,
    /// Size over margin, 1e18 = 1x.
    pub leverage_wad: U256,
    /// How the debt splits.
    pub borrow: BorrowSplit,
    /// Where a pair borrow takes the range's ratio, usually the pool's sqrt price.
    pub split_sqrt_price_x96: U160,
}

/// What an open borrows, in token units, before the opening fee.
///
/// # Errors
///
/// Throws for a zero price.
#[wasm_bindgen(js_name = borrowAmounts)]
pub fn borrow_amounts(request: &Ts<BorrowAmountsRequest>) -> Result<Ts<PerToken<U256>>, JsError> {
    let r = input(request)?;
    output(
        &open::borrow_amounts(&r.range, &r.margin, r.leverage_wad, r.borrow, r.split_sqrt_price_x96)
            .map_err(js_error)?,
    )
}

/// A deposit to bring into a range's ratio.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct RangeSwapRequest {
    /// One end of the range.
    pub sqrt_pa_x96: U160,
    /// The other end.
    pub sqrt_pb_x96: U160,
    /// The deposit.
    pub amounts: PerToken<U256>,
    /// The pool's sqrt price now.
    pub pool_sqrt_price_x96: U160,
    /// The pool's fee, 1e18 = 100%.
    pub swap_fee_wad: U256,
    /// Accepted price move, in basis points.
    #[serde(with = "crate::serde_num")]
    #[tsify(type = "string")]
    pub slippage_bps: u64,
}

/// A swap, or none.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct MaybeSwap {
    /// Undefined when nothing needs swapping.
    pub swap: Option<RangeSwap>,
}

/// The swap that brings a deposit to the range's ratio at the pool price.
///
/// # Errors
///
/// Throws for an invalid range.
#[wasm_bindgen(js_name = rangeSwap)]
pub fn range_swap(request: &Ts<RangeSwapRequest>) -> Result<Ts<MaybeSwap>, JsError> {
    let r = input(request)?;
    let swap = open::range_swap(
        r.sqrt_pa_x96,
        r.sqrt_pb_x96,
        r.amounts,
        r.pool_sqrt_price_x96,
        r.swap_fee_wad,
        r.slippage_bps,
    );
    output(&MaybeSwap {
        swap: swap.map_err(js_error)?,
    })
}

/// A swap to tilt toward a one-token debt.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct LeanRequest {
    /// The swap.
    pub swap: RangeSwap,
    /// The borrowed token.
    pub debt: Side,
    /// Most there is to sell of the other token.
    pub available: U256,
}

/// The swap tilted so what does not fit is left in the borrowed token.
///
/// # Errors
///
/// Throws for an empty swap.
#[wasm_bindgen(js_name = leanTowardDebt)]
pub fn lean_toward_debt(request: &Ts<LeanRequest>) -> Result<Ts<RangeSwap>, JsError> {
    let r = input(request)?;
    output(&open::lean_toward_debt(r.swap, r.debt, r.available).map_err(js_error)?)
}

/// A deposit after a swap, within a slippage.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct MinimumsAfterSwapRequest {
    /// One end of the range.
    pub sqrt_pa_x96: U160,
    /// The other end.
    pub sqrt_pb_x96: U160,
    /// The pool's sqrt price now.
    pub sqrt_price_x96: U160,
    /// The deposit before the swap.
    pub amounts: PerToken<U256>,
    /// The swap, paying only its least output.
    pub swap: Option<RangeSwap>,
    /// Accepted price move, in basis points.
    #[serde(with = "crate::serde_num")]
    #[tsify(type = "string")]
    pub slippage_bps: u64,
}

/// The mint's minimums after the swap pays only its least output.
///
/// # Errors
///
/// Throws for a slippage above 100%.
#[wasm_bindgen(js_name = minimumsAfterSwap)]
pub fn minimums_after_swap(request: &Ts<MinimumsAfterSwapRequest>) -> Result<Ts<TokenAmounts>, JsError> {
    let r = input(request)?;
    let min = open::minimums_after_swap(
        r.sqrt_pa_x96,
        r.sqrt_pb_x96,
        r.sqrt_price_x96,
        r.amounts,
        r.swap.as_ref(),
        r.slippage_bps,
    );
    output(&amounts(min.map_err(js_error)?))
}

/// What an open borrows and its swap into the range, before the swap is quoted.
///
/// # Errors
///
/// Throws for a zero price or an invalid range.
#[wasm_bindgen(js_name = openAmounts)]
pub fn open_amounts(sizing: &Ts<OpenSizing>) -> Result<Ts<OpenAmounts>, JsError> {
    output(&open::open_amounts(&input(sizing)?).map_err(js_error)?)
}

/// An open, what it borrows and its quoted swap.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct OpenMinimumsRequest {
    /// The open.
    pub sizing: OpenSizing,
    /// What it borrows.
    pub debt: PerToken<U256>,
    /// The swap into the range, its least output from the quote.
    pub swap: Option<RangeSwap>,
}

/// The mint's minimums of an open.
///
/// # Errors
///
/// Throws for a slippage above 100%.
#[wasm_bindgen(js_name = openMinimums)]
pub fn open_minimums(request: &Ts<OpenMinimumsRequest>) -> Result<Ts<TokenAmounts>, JsError> {
    let r = input(request)?;
    output(&amounts(
        open::open_minimums(&r.sizing, r.debt, r.swap.as_ref()).map_err(js_error)?,
    ))
}

/// An exact open's sizing and debt.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct ExactOpenRequest {
    /// The open.
    pub sizing: OpenSizing,
    /// What it borrows.
    pub debt: PerToken<U256>,
}

/// The liquidity an exact open mints and the most it may borrow.
///
/// # Errors
///
/// Throws for an empty range.
#[wasm_bindgen(js_name = exactOpen)]
pub fn exact_open(request: &Ts<ExactOpenRequest>) -> Result<Ts<ExactOpen>, JsError> {
    let r = input(request)?;
    output(&open::exact_open(&r.sizing, r.debt).map_err(js_error)?)
}

/// A deposit to fit into a range without a swap.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct FitRequest {
    /// One end of the range.
    pub sqrt_pa_x96: U160,
    /// The other end.
    pub sqrt_pb_x96: U160,
    /// Where it mints.
    pub sqrt_price_x96: U160,
    /// The deposit.
    pub amounts: PerToken<U256>,
}

/// What of the deposit a mint takes without a swap.
///
/// # Errors
///
/// Throws for an empty range.
#[wasm_bindgen(js_name = fitToRange)]
pub fn fit_to_range(request: &Ts<FitRequest>) -> Result<Ts<PerToken<U256>>, JsError> {
    let r = input(request)?;
    output(&open::fit_to_range(r.sqrt_pa_x96, r.sqrt_pb_x96, r.sqrt_price_x96, r.amounts).map_err(js_error)?)
}

/// A liquidation penalty with the 2% safety buffer on top, as hex.
///
/// # Errors
///
/// Throws for a malformed number.
#[wasm_bindgen(js_name = bufferedPenalty)]
pub fn buffered_penalty(penalty_wad: &str) -> Result<String, JsError> {
    Ok(hex(open::buffered_penalty(parse(penalty_wad)?).map_err(js_error)?))
}

/// The leverage offered for a maximum (1e18), in tenths (25 = 2.5x).
///
/// # Errors
///
/// Throws for a malformed number.
#[wasm_bindgen(js_name = leverageCap)]
pub fn leverage_cap(max_leverage_wad: &str) -> Result<u32, JsError> {
    let tenths = open::leverage_cap(parse(max_leverage_wad)?).map_err(js_error)?;
    Ok(u32::try_from(tenths).unwrap_or(u32::MAX))
}

/// The highest leverage a new position opens with, keeping the safety buffer.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct OpenMaxLeverageRequest {
    /// The range and the pair's prices and σ (`liquidity` ignored).
    pub collateral: CollateralInput,
    /// How it borrows.
    pub borrow: BorrowSplit,
    /// The pair's liquidation penalty, before the buffer.
    pub penalty_wad: U256,
    /// The deployment's opening fee.
    pub borrow_fee_wad: U256,
    /// Where a mix compares its share with the pair's, usually the pool's sqrt price.
    pub split_sqrt_price_x96: U160,
}

/// The highest leverage (1e18) a new position opens with, keeping the safety buffer, as hex.
///
/// # Errors
///
/// Throws where the lens reverts.
#[wasm_bindgen(js_name = openMaxLeverage)]
pub fn open_max_leverage(request: &Ts<OpenMaxLeverageRequest>) -> Result<String, JsError> {
    let r = input(request)?;
    let max = open::open_max_leverage(
        &r.collateral,
        r.borrow,
        r.penalty_wad,
        r.borrow_fee_wad,
        r.split_sqrt_price_x96,
    );
    Ok(hex(max.map_err(js_error)?))
}

/// A position in a state.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct PreviewStateRequest {
    /// The position.
    pub position: EditPosition,
    /// The state to preview.
    pub state: EditState,
    /// Also search the liquidation prices (the slow part).
    pub with_liquidation_prices: bool,
}

/// The position's numbers in the state, from the contracts' own health math.
///
/// # Errors
///
/// Throws where the health check reverts.
#[wasm_bindgen(js_name = previewState)]
pub fn preview_state(request: &Ts<PreviewStateRequest>) -> Result<Ts<EditPreview>, JsError> {
    let r = input(request)?;
    output(&edit::preview_state(&r.position, &r.state, r.with_liquidation_prices).map_err(js_error)?)
}

/// A position, its state and the margin added.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct AddMarginStateRequest {
    /// The position.
    pub position: EditPosition,
    /// Its state now.
    pub state: EditState,
    /// Margin added, in token units.
    pub margin: PerToken<U256>,
}

/// The state after adding margin as liquidity.
///
/// # Errors
///
/// Throws for liquidity over 128 bits.
#[wasm_bindgen(js_name = addMarginState)]
pub fn add_margin_state(request: &Ts<AddMarginStateRequest>) -> Result<Ts<EditState>, JsError> {
    let r = input(request)?;
    output(&edit::add_margin_state(&r.position, &r.state, r.margin).map_err(js_error)?)
}

/// A state and what is repaid.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct RepayStateRequest {
    /// The state now.
    pub state: EditState,
    /// Repaid, in token units.
    pub repay: PerToken<U256>,
}

/// The state after a repayment.
///
/// # Errors
///
/// Throws for a malformed request.
#[wasm_bindgen(js_name = repayState)]
pub fn repay_state(request: &Ts<RepayStateRequest>) -> Result<Ts<EditState>, JsError> {
    let r = input(request)?;
    output(&edit::repay_state(&r.state, r.repay))
}

/// A state and the share of its liquidity redeemed.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct RemoveMarginStateRequest {
    /// The state now.
    pub state: EditState,
    /// Share redeemed, 1e18 = all.
    pub fraction_wad: U256,
}

/// The state after redeeming a share of the liquidity.
///
/// # Errors
///
/// Throws for a share over 1e18.
#[wasm_bindgen(js_name = removeMarginState)]
pub fn remove_margin_state(request: &Ts<RemoveMarginStateRequest>) -> Result<Ts<EditState>, JsError> {
    let r = input(request)?;
    output(&edit::remove_margin_state(&r.state, r.fraction_wad).map_err(js_error)?)
}

/// A position in its state now.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct PositionStateRequest {
    /// The position.
    pub position: EditPosition,
    /// Its state now.
    pub state: EditState,
}

/// The largest share of the liquidity (1e18 = all) a redeem can take keeping the safety buffer, as hex.
///
/// # Errors
///
/// Throws where the health check reverts.
#[wasm_bindgen(js_name = maxRemovableWad)]
pub fn max_removable_wad(request: &Ts<PositionStateRequest>) -> Result<String, JsError> {
    let r = input(request)?;
    Ok(hex(edit::max_removable_wad(&r.position, &r.state).map_err(js_error)?))
}

/// A position, its state and a target leverage.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct LeverageChangeRequest {
    /// The position.
    pub position: EditPosition,
    /// Its state now.
    pub state: EditState,
    /// Size over margin to reach, 1e18 = 1x.
    pub target_leverage_wad: U256,
    /// How an increase borrows.
    pub borrow: BorrowSplit,
    /// Where a pair borrow takes the range's ratio, usually the pool's sqrt price.
    pub split_sqrt_price_x96: U160,
}

/// What it takes to reach the target leverage.
///
/// # Errors
///
/// Throws where the health check reverts.
#[wasm_bindgen(js_name = leverageChange)]
pub fn leverage_change(request: &Ts<LeverageChangeRequest>) -> Result<Ts<LeverageChange>, JsError> {
    let r = input(request)?;
    let change = edit::leverage_change(
        &r.position,
        &r.state,
        r.target_leverage_wad,
        r.borrow,
        r.split_sqrt_price_x96,
    );
    output(&change.map_err(js_error)?)
}

/// A leverage increase's borrow into a range.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct LeverageUpSwapRequest {
    /// One end of the range.
    pub sqrt_pa_x96: U160,
    /// The other end.
    pub sqrt_pb_x96: U160,
    /// What is borrowed.
    pub borrow: PerToken<U256>,
    /// The pool's sqrt price now.
    pub pool_sqrt_price_x96: U160,
    /// The pool's fee, 1e18 = 100%.
    pub swap_fee_wad: U256,
    /// Accepted price move, in basis points.
    #[serde(with = "crate::serde_num")]
    #[tsify(type = "string")]
    pub slippage_bps: u64,
}

/// The swap of a one-token borrow into the range, tilted toward the debt.
///
/// # Errors
///
/// Throws for an invalid range.
#[wasm_bindgen(js_name = leverageUpSwap)]
pub fn leverage_up_swap(request: &Ts<LeverageUpSwapRequest>) -> Result<Ts<MaybeSwap>, JsError> {
    let r = input(request)?;
    let swap = edit::leverage_up_swap(
        r.sqrt_pa_x96,
        r.sqrt_pb_x96,
        r.borrow,
        r.pool_sqrt_price_x96,
        r.swap_fee_wad,
        r.slippage_bps,
    );
    output(&MaybeSwap {
        swap: swap.map_err(js_error)?,
    })
}

/// A share of a position to redeem.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct RedeemShareRequest {
    /// The position.
    pub position: Redeemable,
    /// Share redeemed, 1e18 = all.
    pub fraction_wad: U256,
    /// Accepted price move, in basis points.
    #[serde(with = "crate::serde_num")]
    #[tsify(type = "string")]
    pub slippage_bps: u64,
}

/// The units a redeem takes and its minimums.
///
/// # Errors
///
/// Throws for a slippage above 100%.
#[wasm_bindgen(js_name = redeemShare)]
pub fn redeem_share(request: &Ts<RedeemShareRequest>) -> Result<Ts<RedeemShare>, JsError> {
    let r = input(request)?;
    output(&edit::redeem_share(&r.position, r.fraction_wad, r.slippage_bps).map_err(js_error)?)
}

/// What a redeem of the share pays at the pool price (`slippageBps` is ignored).
///
/// # Errors
///
/// Throws for an invalid range.
#[wasm_bindgen(js_name = redeemedAmounts)]
pub fn redeemed_amounts(request: &Ts<RedeemShareRequest>) -> Result<Ts<PerToken<U256>>, JsError> {
    let r = input(request)?;
    output(&edit::redeemed_amounts(&r.position, r.fraction_wad).map_err(js_error)?)
}

/// What to pull from the wallet to repay `repay` of `debt` with `balance`, as hex.
///
/// # Errors
///
/// Throws for a malformed number.
#[wasm_bindgen(js_name = repayPull)]
pub fn repay_pull(repay: &str, debt: &str, balance: &str) -> Result<String, JsError> {
    Ok(hex(
        edit::repay_pull(parse(repay)?, parse(debt)?, parse(balance)?).map_err(js_error)?
    ))
}

/// A position and how it would borrow more.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct EditMaxLeverageRequest {
    /// The position.
    pub position: EditPosition,
    /// How it borrows.
    pub borrow: BorrowSplit,
    /// The deployment's opening fee.
    pub borrow_fee_wad: U256,
}

/// The highest leverage (1e18) the position's range takes keeping the safety buffer, as hex.
///
/// # Errors
///
/// Throws where the lens reverts.
#[wasm_bindgen(js_name = editMaxLeverage)]
pub fn edit_max_leverage(request: &Ts<EditMaxLeverageRequest>) -> Result<String, JsError> {
    let r = input(request)?;
    Ok(hex(
        edit::edit_max_leverage(&r.position, r.borrow, r.borrow_fee_wad).map_err(js_error)?
    ))
}

/// `tick` moved down or up to a multiple of the pool's tick spacing, kept inside the ticks a pool allows.
///
/// # Errors
///
/// Throws for a spacing below 1 or a NaN tick.
#[wasm_bindgen(js_name = alignTick)]
pub fn align_tick(tick: f64, spacing: i32, direction: &Ts<TickDirection>) -> Result<i32, JsError> {
    range::align_tick(tick, spacing, input(direction)?).map_err(js_error)
}

/// The multiple of the pool's tick spacing nearest `tick` (halfway rounds up), kept inside the ticks a pool allows.
///
/// # Errors
///
/// Throws for a spacing below 1 or a NaN tick.
#[wasm_bindgen(js_name = nearestUsableTick)]
pub fn nearest_usable_tick(tick: f64, spacing: i32) -> Result<i32, JsError> {
    range::nearest_usable_tick(tick, spacing).map_err(js_error)
}

/// A range by its ticks.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct TickRange {
    /// The lower tick.
    pub tick_lower: i32,
    /// The upper tick.
    pub tick_upper: i32,
}

/// The widest range a pool with this tick spacing allows.
///
/// # Errors
///
/// Throws for a spacing below 1.
#[wasm_bindgen(js_name = fullRange)]
pub fn full_range(spacing: i32) -> Result<Ts<TickRange>, JsError> {
    let (tick_lower, tick_upper) = range::full_range(spacing).map_err(js_error)?;
    output(&TickRange { tick_lower, tick_upper })
}

/// Which tokens a range takes at the pool's tick: only token0 above the price, only token1 below it, both in range.
///
/// # Errors
///
/// Never in practice; the result crosses into JS.
#[wasm_bindgen(js_name = acceptedTokens)]
pub fn accepted_tokens(range: &Ts<TickRange>, pool_tick: i32) -> Result<Ts<PerToken<bool>>, JsError> {
    let r = input(range)?;
    output(&range::accepted_tokens(r.tick_lower, r.tick_upper, pool_tick))
}

/// A deposit of one token into a range.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct PairedAmountRequest {
    /// The range.
    pub range: TickRange,
    /// The pool's sqrt price now.
    pub pool_sqrt_price_x96: U160,
    /// The token deposited.
    pub from: Side,
    /// How much of it.
    pub amount: U256,
}

/// Without a swap, the amount of the other token that fits the deposit in the range at the pool price, as hex.
///
/// # Errors
///
/// Throws for a tick out of range.
#[wasm_bindgen(js_name = pairedAmount)]
pub fn paired_amount(request: &Ts<PairedAmountRequest>) -> Result<String, JsError> {
    let r = input(request)?;
    let paired = range::paired_amount(
        r.range.tick_lower,
        r.range.tick_upper,
        r.pool_sqrt_price_x96,
        r.from,
        r.amount,
    );
    Ok(hex(paired.map_err(js_error)?))
}

/// Token1 per token0, in whole tokens, at a tick. For display.
#[wasm_bindgen(js_name = tickToPrice)]
#[must_use]
pub fn tick_to_price(tick: i32, decimals0: u8, decimals1: u8) -> f64 {
    range::tick_to_price(tick, decimals0, decimals1)
}

/// A sqrt price as a raw number (token1 base units per token0 base unit, square-rooted). For display.
///
/// # Errors
///
/// Throws for a malformed sqrt price.
#[wasm_bindgen(js_name = rawSqrtPrice)]
pub fn raw_sqrt_price(sqrt_price_x96: &str) -> Result<f64, JsError> {
    Ok(range::raw_sqrt_price(sqrt_price(sqrt_price_x96)?))
}

/// The sqrt price at a pool price (token1 per token0, whole tokens), rounded down, as hex. For display and
/// approximate inputs.
///
/// # Errors
///
/// Throws for a negative or non-finite price.
#[wasm_bindgen(js_name = sqrtPriceX96AtPrice)]
pub fn sqrt_price_x96_at_price(price: f64, decimals0: u8, decimals1: u8) -> Result<String, JsError> {
    let sqrt = range::sqrt_price_x96_at_price(price, decimals0, decimals1).map_err(js_error)?;
    Ok(format!("{sqrt:#x}"))
}

/// Token1 per token0, in whole tokens, at an exact sqrt price. For display.
///
/// # Errors
///
/// Throws for a malformed sqrt price.
#[wasm_bindgen(js_name = sqrtPriceToPrice)]
pub fn sqrt_price_to_price(sqrt_price_x96: &str, decimals0: u8, decimals1: u8) -> Result<f64, JsError> {
    Ok(range::sqrt_price_to_price(
        sqrt_price(sqrt_price_x96)?,
        decimals0,
        decimals1,
    ))
}

/// The tick whose price is nearest `price` (token1 per token0, whole tokens); align it before use.
#[wasm_bindgen(js_name = priceToTick)]
#[must_use]
pub fn price_to_tick(price: f64, decimals0: u8, decimals1: u8) -> f64 {
    range::price_to_tick(price, decimals0, decimals1)
}

/// The pool price as a fractional tick. For display.
///
/// # Errors
///
/// Throws for a malformed sqrt price.
#[wasm_bindgen(js_name = exactTickAtSqrtPrice)]
pub fn exact_tick_at_sqrt_price(sqrt_price_x96: &str) -> Result<f64, JsError> {
    Ok(range::exact_tick_at_sqrt_price(sqrt_price(sqrt_price_x96)?))
}

/// A position's USD liquidation prices and its pair's oracle prices.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct LiquidationPoolPricesRequest {
    /// The token whose price moves.
    pub moving: Side,
    /// Its lower USD liquidation price (1e18).
    pub lower_wad: Option<U256>,
    /// Its upper one.
    pub upper_wad: Option<U256>,
    /// Token0's USD price (1e18).
    pub price0_wad: U256,
    /// Token1's.
    pub price1_wad: U256,
}

/// The liquidation prices as pool prices (token1 per token0, unsorted) and the pool price now. For display.
///
/// # Errors
///
/// Throws for a malformed request.
#[wasm_bindgen(js_name = liquidationPoolPrices)]
pub fn liquidation_pool_prices(
    request: &Ts<LiquidationPoolPricesRequest>,
) -> Result<Ts<LiquidationPoolPrices>, JsError> {
    let r = input(request)?;
    output(&range::liquidation_pool_prices(
        r.moving,
        r.lower_wad,
        r.upper_wad,
        r.price0_wad,
        r.price1_wad,
    ))
}

/// A lender's deposit and the shares asked for.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawalRequest {
    /// What the deposit is worth.
    pub deposit_assets: U256,
    /// Its shares.
    pub deposit_shares: U256,
    /// Shares asked for; absent for all.
    pub asked_shares: Option<U256>,
}

/// The shares a withdrawal burns and what they pay.
///
/// # Errors
///
/// Throws on absurd values.
#[wasm_bindgen]
pub fn withdrawal(request: &Ts<WithdrawalRequest>) -> Result<Ts<Withdrawal>, JsError> {
    let r = input(request)?;
    output(&lend::withdrawal(r.deposit_assets, r.deposit_shares, r.asked_shares).map_err(js_error)?)
}
