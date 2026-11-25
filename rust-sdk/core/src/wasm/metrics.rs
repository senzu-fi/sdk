//! Bindings for [`crate::metrics`].

use alloy_primitives::{I256, U160, U256};
use serde::{Deserialize, Serialize};
use tsify::{Ts, Tsify};
use wasm_bindgen::prelude::*;

use super::{hex, input, js_error, output, parse};
use crate::flows::PerToken;
use crate::metrics::pnl::{
    self, ClosedVsHoldInput, DebtFlow, Deposit, InterestPaid, LpInput, LpMetrics, Pnl, PnlInput, VsHoldInput,
};
use crate::metrics::position::{self, Mark, MarkReceive, MarkedPosition, PoolPricing};
use crate::metrics::prices;
use crate::metrics::rates::{self, DebtRate, MarketAprs};
use crate::metrics::swap_fees::{self, ProtocolFee};
use crate::metrics::vault;
use crate::metrics::yields::{
    self, EditAprInput, MarketFlows, OpenEarnings, OpenEarningsInput, PositionApr, PositionAprInput, ValuedRate,
};
use crate::types::Side;

/// One token's USD price and a pool price.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct PoolPriceRequest {
    /// The pool's sqrt price.
    pub pool_sqrt_price_x96: U160,
    /// The token whose USD price is known.
    pub known: Side,
    /// Its USD price (1e18).
    pub known_price_wad: U256,
    /// Token0's decimals.
    pub decimals0: u8,
    /// Token1's decimals.
    pub decimals1: u8,
}

/// Both tokens' USD prices (1e18).
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct PoolPrices {
    /// Token0's.
    pub price0_wad: U256,
    /// Token1's.
    pub price1_wad: U256,
}

/// Both tokens' USD prices at a pool price: the known one as given, the other as the pool implies.
///
/// # Errors
///
/// Throws for a price over 256 bits.
#[wasm_bindgen(js_name = poolPrices)]
pub fn pool_prices(request: &Ts<PoolPriceRequest>) -> Result<Ts<PoolPrices>, JsError> {
    let r = input(request)?;
    let (price0_wad, price1_wad) = prices::pool_prices(
        r.pool_sqrt_price_x96,
        r.known,
        r.known_price_wad,
        r.decimals0,
        r.decimals1,
    )
    .map_err(js_error)?;
    output(&PoolPrices { price0_wad, price1_wad })
}

/// Borrows over cash plus borrows (1e18 = 100%), as hex.
///
/// # Errors
///
/// Throws for a malformed number.
#[wasm_bindgen]
pub fn utilization(cash: &str, borrows: &str) -> Result<String, JsError> {
    Ok(hex(rates::utilization(parse(cash)?, parse(borrows)?).map_err(js_error)?))
}

/// A market's curve inputs at a utilization.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct MarketAprsRequest {
    /// The yearly rate at target (1e18 = 100%).
    pub rate_at_target_apr_wad: U256,
    /// Share of interest to the reserves.
    pub reserve_factor_wad: U256,
    /// Utilization (1e18 = 100%).
    pub utilization_wad: U256,
}

/// A market's borrow and supply APRs at a utilization.
///
/// # Errors
///
/// Throws for a reserve factor above 100%.
#[wasm_bindgen(js_name = marketAprs)]
pub fn market_aprs(request: &Ts<MarketAprsRequest>) -> Result<Ts<MarketAprs>, JsError> {
    let r = input(request)?;
    output(&rates::market_aprs(r.rate_at_target_apr_wad, r.reserve_factor_wad, r.utilization_wad).map_err(js_error)?)
}

/// A market and how its cash moves.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct MarketAprsAfterRequest {
    /// Its cash now.
    pub cash: U256,
    /// Its borrows.
    pub borrows: U256,
    /// The yearly rate at target.
    pub rate_at_target_apr_wad: U256,
    /// Share of interest to the reserves.
    pub reserve_factor_wad: U256,
    /// Cash added (a supply).
    pub cash_added: U256,
    /// Cash taken (a withdrawal or a borrow).
    pub cash_taken: U256,
}

/// A market's APRs after its cash moves.
///
/// # Errors
///
/// Throws for a reserve factor above 100%.
#[wasm_bindgen(js_name = marketAprsAfter)]
pub fn market_aprs_after(request: &Ts<MarketAprsAfterRequest>) -> Result<Ts<MarketAprs>, JsError> {
    let r = input(request)?;
    let aprs = rates::market_aprs_after(
        r.cash,
        r.borrows,
        r.rate_at_target_apr_wad,
        r.reserve_factor_wad,
        r.cash_added,
        r.cash_taken,
    );
    output(&aprs.map_err(js_error)?)
}

/// A debt read some seconds ago.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct AccrueDebtRequest {
    /// How its rate is known.
    pub rate: DebtRate,
    /// Seconds since the read.
    #[serde(with = "crate::serde_num")]
    #[tsify(type = "string")]
    pub elapsed: u64,
    /// Owed at the read.
    pub assets: U256,
    /// Its USD value at the read.
    pub value_usd_wad: U256,
}

/// A debt and its value, grown.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct AccruedDebt {
    /// Owed now.
    pub assets: U256,
    /// Its USD value now, at the read's price.
    pub value_usd_wad: U256,
}

/// A debt grown by its interest since the read, as the contract accrues it.
///
/// # Errors
///
/// Throws where the rate math overflows (never for a real market).
#[wasm_bindgen(js_name = accrueDebt)]
pub fn accrue_debt(request: &Ts<AccrueDebtRequest>) -> Result<Ts<AccruedDebt>, JsError> {
    let r = input(request)?;
    let growth = rates::debt_growth(&r.rate, r.elapsed).map_err(js_error)?;
    output(&AccruedDebt {
        assets: rates::grown(r.assets, growth).map_err(js_error)?,
        value_usd_wad: rates::grown(r.value_usd_wad, growth).map_err(js_error)?,
    })
}

/// A position and how its pair is priced at its pool.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct MarkRequest {
    /// The pair's prices at the pool.
    pub pricing: PoolPricing,
    /// The position.
    pub position: MarkedPosition,
}

/// The position valued at its pool's price.
///
/// # Errors
///
/// Throws for an invalid range.
#[wasm_bindgen(js_name = markPosition)]
pub fn mark_position(request: &Ts<MarkRequest>) -> Result<Ts<Mark>, JsError> {
    let r = input(request)?;
    output(&position::mark(&r.pricing, &r.position).map_err(js_error)?)
}

/// Token amounts and how the pair is priced at its pool.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct ValueAtPoolRequest {
    /// The pair's prices at the pool.
    pub pricing: PoolPricing,
    /// Amounts of each token.
    pub amounts: PerToken<U256>,
}

/// The amounts in USD (1e18) at the pool's price, as hex.
///
/// # Errors
///
/// Throws on absurd amounts.
#[wasm_bindgen(js_name = valueAtPool)]
pub fn value_at_pool(request: &Ts<ValueAtPoolRequest>) -> Result<String, JsError> {
    let r = input(request)?;
    Ok(hex(position::value_at_pool(&r.pricing, r.amounts).map_err(js_error)?))
}

/// A position, its pool pricing and the pool's fee.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct MarkReceiveRequest {
    /// The pair's prices at the pool.
    pub pricing: PoolPricing,
    /// The position.
    pub position: MarkedPosition,
    /// The pool's fee (1e18 = 100%), paid on a swap for a short debt.
    pub swap_fee_wad: U256,
}

/// What closing the position now sends back, at the pool's price.
///
/// # Errors
///
/// Throws for a zero price.
#[wasm_bindgen(js_name = markReceive)]
pub fn mark_receive(request: &Ts<MarkReceiveRequest>) -> Result<Ts<MarkReceive>, JsError> {
    let r = input(request)?;
    output(&position::mark_receive(&r.pricing, &r.position, r.swap_fee_wad).map_err(js_error)?)
}

/// A position's profit or loss against what was put in.
///
/// # Errors
///
/// Throws on absurd values.
#[wasm_bindgen]
pub fn pnl(request: &Ts<PnlInput>) -> Result<Ts<Pnl>, JsError> {
    output(&pnl::pnl(&input(request)?).map_err(js_error)?)
}

/// The opening deposits and the pool price then.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct EntryPricesRequest {
    /// The opening deposits.
    pub deposits: Vec<Deposit>,
    /// Token0's decimals.
    pub decimals0: u8,
    /// Token1's decimals.
    pub decimals1: u8,
    /// The pool's price right after the open.
    pub entry_sqrt_price_x96: U160,
    /// The opening margin's value (USD, 1e18).
    pub opening_usd_wad: U256,
}

/// Both tokens' USD prices.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct UsdPrices {
    /// Token0's.
    pub usd0: f64,
    /// Token1's.
    pub usd1: f64,
}

/// Both tokens' USD prices at the open; undefined when the deposits are worth nothing.
///
/// # Errors
///
/// Throws for a malformed input.
#[wasm_bindgen(js_name = entryUsdPrices)]
pub fn entry_usd_prices(request: &Ts<EntryPricesRequest>) -> Result<Option<Ts<UsdPrices>>, JsError> {
    let r = input(request)?;
    let prices = pnl::entry_usd_prices(
        &r.deposits,
        r.decimals0,
        r.decimals1,
        r.entry_sqrt_price_x96,
        r.opening_usd_wad,
    );
    prices
        .map(|p| {
            output(&UsdPrices {
                usd0: p.token0,
                usd1: p.token1,
            })
        })
        .transpose()
}

/// What providing liquidity made against holding the tokens put in (USD), at today's prices; undefined before anything
/// was put in.
///
/// # Errors
///
/// Throws for a malformed input.
#[wasm_bindgen(js_name = positionVsHold)]
pub fn position_vs_hold(request: &Ts<VsHoldInput>) -> Result<Option<f64>, JsError> {
    Ok(pnl::position_vs_hold(&input(request)?))
}

/// What an ended position made against holding the tokens put in to its end (USD), at the exit price.
///
/// # Errors
///
/// Throws for a malformed input.
#[wasm_bindgen(js_name = closedVsHold)]
pub fn closed_vs_hold(request: &Ts<ClosedVsHoldInput>) -> Result<Option<f64>, JsError> {
    Ok(pnl::closed_vs_hold(&input(request)?))
}

/// A position's debts' flows.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct InterestPaidRequest {
    /// One per debt asset.
    pub debts: Vec<DebtFlow>,
}

/// The interest each debt has cost.
///
/// # Errors
///
/// Throws on absurd values.
#[wasm_bindgen(js_name = interestPaid)]
pub fn interest_paid(request: &Ts<InterestPaidRequest>) -> Result<Ts<InterestPaid>, JsError> {
    output(&pnl::interest_paid(&input(request)?.debts).map_err(js_error)?)
}

/// Size over margin (1e18 = 1x), as hex; zero without margin. `equity_usd_wad` is a decimal or hex int256.
///
/// # Errors
///
/// Throws for a malformed number.
#[wasm_bindgen(js_name = leverageWad)]
pub fn leverage_wad(size_usd_wad: &str, equity_usd_wad: &str) -> Result<String, JsError> {
    let equity: I256 = equity_usd_wad
        .parse()
        .map_err(|_| JsError::new("expected a decimal or 0x-hex int256"))?;
    Ok(hex(
        yields::leverage_wad(parse(size_usd_wad)?, equity).map_err(js_error)?
    ))
}

/// Rates and the values they apply to.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct AverageAprRequest {
    /// Each rate and its value.
    pub rates: Vec<ValuedRate>,
}

/// The value-weighted average rate, as hex.
///
/// # Errors
///
/// Throws on absurd values.
#[wasm_bindgen(js_name = averageApr)]
pub fn average_apr(request: &Ts<AverageAprRequest>) -> Result<String, JsError> {
    Ok(hex(yields::average_apr(&input(request)?.rates).map_err(js_error)?))
}

/// What `amount` earns in a year at `apr_wad`, as hex.
///
/// # Errors
///
/// Throws for a malformed number.
#[wasm_bindgen]
pub fn yearly(amount: &str, apr_wad: &str) -> Result<String, JsError> {
    Ok(hex(yields::yearly(parse(amount)?, parse(apr_wad)?).map_err(js_error)?))
}

/// `part` of `total` (1e18 = all), as hex.
///
/// # Errors
///
/// Throws for a malformed number.
#[wasm_bindgen(js_name = shareWad)]
pub fn share_wad(part: &str, total: &str) -> Result<String, JsError> {
    Ok(hex(yields::share_wad(parse(part)?, parse(total)?).map_err(js_error)?))
}

/// A market's borrows, rate and reserve factor.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct MarketFlowsRequest {
    /// What is borrowed.
    pub borrow_assets: U256,
    /// The borrow rate.
    pub borrow_apr_wad: U256,
    /// Share of interest to the reserves.
    pub reserve_factor_wad: U256,
}

/// A market's interest over a year, split between reserves and lenders.
///
/// # Errors
///
/// Throws on absurd values.
#[wasm_bindgen(js_name = marketFlows)]
pub fn market_flows(request: &Ts<MarketFlowsRequest>) -> Result<Ts<MarketFlows>, JsError> {
    let r = input(request)?;
    output(&yields::market_flows(r.borrow_assets, r.borrow_apr_wad, r.reserve_factor_wad).map_err(js_error)?)
}

/// A pool's swap fees over a year, from those seen over `window_seconds`, times what LPs keep.
#[wasm_bindgen(js_name = feesPerYear)]
#[must_use]
pub fn fees_per_year(fees_24h_usd: f64, window_seconds: f64, keep: f64) -> f64 {
    yields::fees_per_year(fees_24h_usd, window_seconds, keep)
}

/// What a Senzu holder keeps of swap fees or rewards: the DEX's LP share (`keep`) less the protocol's cut.
///
/// # Errors
///
/// Throws for a malformed number.
#[wasm_bindgen(js_name = holderKeep)]
pub fn holder_keep(keep: f64, cut_wad: &str) -> Result<f64, JsError> {
    Ok(yields::holder_keep(keep, parse(cut_wad)?))
}

/// A new range's fee APR over its size; undefined without size or liquidity.
///
/// # Errors
///
/// Throws for a malformed number.
#[wasm_bindgen(js_name = rangeFeeApr)]
pub fn range_fee_apr(
    fees_per_year_usd: f64,
    liquidity: &str,
    pool_liquidity: &str,
    size_usd: f64,
) -> Result<Option<f64>, JsError> {
    Ok(yields::range_fee_apr(
        fees_per_year_usd,
        parse(liquidity)?,
        parse(pool_liquidity)?,
        size_usd,
    ))
}

/// A position's yearly rates; undefined without size or margin.
///
/// # Errors
///
/// Throws for a malformed input.
#[wasm_bindgen(js_name = positionApr)]
pub fn position_apr(request: &Ts<PositionAprInput>) -> Result<Option<Ts<PositionApr>>, JsError> {
    yields::position_apr(&input(request)?)
        .map(|apr| output(&apr))
        .transpose()
}

/// A position's net yearly rate after an edit; undefined without margin.
///
/// # Errors
///
/// Throws for a malformed input.
#[wasm_bindgen(js_name = editApr)]
pub fn edit_apr(request: &Ts<EditAprInput>) -> Result<Option<f64>, JsError> {
    Ok(yields::edit_apr(&input(request)?))
}

/// A new position's projected earnings; undefined without margin.
///
/// # Errors
///
/// Throws on an absurd debt.
#[wasm_bindgen(js_name = openEarnings)]
pub fn open_earnings(request: &Ts<OpenEarningsInput>) -> Result<Option<Ts<OpenEarnings>>, JsError> {
    yields::open_earnings(&input(request)?)
        .map_err(js_error)?
        .map(|e| output(&e))
        .transpose()
}

/// A new position's inputs for its net yearly rate.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct OpenNetAprRequest {
    /// The range's fee APR.
    pub fee_apr: f64,
    /// The position's size (USD, 1e18).
    pub size_usd_wad: U256,
    /// Its debt.
    pub debt_usd_wad: U256,
    /// The debt's average rate.
    pub borrow_apr_wad: U256,
    /// The margin to rate against.
    pub margin_usd_wad: U256,
}

/// A new position's net yearly rate on its margin; undefined without margin.
///
/// # Errors
///
/// Throws for a malformed input.
#[wasm_bindgen(js_name = openNetApr)]
pub fn open_net_apr(request: &Ts<OpenNetAprRequest>) -> Result<Option<f64>, JsError> {
    let r = input(request)?;
    Ok(yields::open_net_apr(
        r.fee_apr,
        r.size_usd_wad,
        r.debt_usd_wad,
        r.borrow_apr_wad,
        r.margin_usd_wad,
    ))
}

/// A lender's booked interest and shares, and the live market.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct LenderEarnedRequest {
    /// Interest booked by the indexer (asset units; negative after bad debt).
    pub earned_assets: I256,
    /// The lender's shares as indexed.
    pub shares: U256,
    /// The booked rate (assets per 1e36 shares).
    pub last_rate: U256,
    /// The lender's assets now (zero with `live_shares` zero: no live read).
    pub live_assets: U256,
    /// The lender's shares now.
    pub live_shares: U256,
}

/// A lender's interest to date in asset units, as a decimal int256.
///
/// # Errors
///
/// Throws on absurd values.
#[wasm_bindgen(js_name = lenderEarned)]
pub fn lender_earned(request: &Ts<LenderEarnedRequest>) -> Result<String, JsError> {
    let r = input(request)?;
    let earned = yields::lender_earned(r.earned_assets, r.shares, r.last_rate, r.live_assets, r.live_shares);
    Ok(earned.map_err(js_error)?.to_string())
}

/// A stake's share of all staked (1e18 = all), as hex; undefined without a stake or stakers.
///
/// # Errors
///
/// Throws for a malformed number.
#[wasm_bindgen(js_name = stakeShareWad)]
pub fn stake_share_wad(staked: &str, total_staked: &str) -> Result<Option<String>, JsError> {
    Ok(vault::stake_share_wad(parse(staked)?, parse(total_staked)?)
        .map_err(js_error)?
        .map(hex))
}

/// What `stake` of `total_staked` earns of `inflow`, as hex.
///
/// # Errors
///
/// Throws for a malformed number.
#[wasm_bindgen(js_name = stakeIncome)]
pub fn stake_income(inflow: &str, stake: &str, total_staked: &str) -> Result<String, JsError> {
    Ok(hex(vault::stake_income(
        parse(inflow)?,
        parse(stake)?,
        parse(total_staked)?,
    )
    .map_err(js_error)?))
}

/// The vault's share of the interest borrowers pay (1e18 = all), as hex.
///
/// # Errors
///
/// Throws for a malformed number.
#[wasm_bindgen(js_name = reserveShareWad)]
pub fn reserve_share_wad(inflow: &str, interest: &str, reserve_factor_wad: &str) -> Result<String, JsError> {
    Ok(hex(vault::reserve_share_wad(
        parse(inflow)?,
        parse(interest)?,
        parse(reserve_factor_wad)?,
    )
    .map_err(js_error)?))
}

/// What a reward stream still pays, as hex.
///
/// # Errors
///
/// Throws for a malformed number.
#[wasm_bindgen(js_name = streamRemaining)]
pub fn stream_remaining(rate_wad: &str, period_finish: u64, now: u64) -> Result<String, JsError> {
    Ok(hex(
        vault::stream_remaining(parse(rate_wad)?, period_finish, now).map_err(js_error)?
    ))
}

/// A plain liquidity position's value, profit, comparison with holding, and fee APR.
///
/// # Errors
///
/// Throws for a malformed input.
#[wasm_bindgen(js_name = lpMetrics)]
pub fn lp_metrics(request: &Ts<LpInput>) -> Result<Ts<LpMetrics>, JsError> {
    output(&pnl::lp_metrics(&input(request)?))
}

/// What to approve for `amount`: 2% more and one wei, as hex.
///
/// # Errors
///
/// Throws for a malformed amount.
#[wasm_bindgen(js_name = approvalAmount)]
pub fn approval_amount(amount: &str) -> Result<String, JsError> {
    Ok(hex(crate::approval::approval_amount(parse(amount)?).map_err(js_error)?))
}

/// Whether `allowance` is too low to spend `amount` (under the amount plus 1%).
///
/// # Errors
///
/// Throws for a malformed number.
#[wasm_bindgen(js_name = needsApproval)]
pub fn needs_approval(amount: &str, allowance: &str) -> Result<bool, JsError> {
    Ok(crate::approval::needs_approval(parse(amount)?, parse(allowance)?))
}

/// USD value (1e18) of `amount` of a token at its USD price, rounded down, as hex.
///
/// # Errors
///
/// Throws for a malformed number.
#[wasm_bindgen(js_name = usdValue)]
pub fn usd_value(amount: &str, decimals: u8, price_wad: &str) -> Result<String, JsError> {
    Ok(hex(
        crate::lens::usd(parse(amount)?, parse(price_wad)?, decimals).map_err(js_error)?
    ))
}

/// A swap's fee to liquidity providers.
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct LpFeeRequest {
    /// What the swapper paid in.
    pub amount_in: U256,
    /// The swap fee, in pips.
    pub fee_pips: u32,
    /// The protocol's cut.
    pub protocol: ProtocolFee,
}

/// The liquidity providers' fee on a swap's amount in, less the protocol's, as hex.
///
/// # Errors
///
/// Throws on an absurd amount.
#[wasm_bindgen(js_name = lpFee)]
pub fn lp_fee(request: &Ts<LpFeeRequest>) -> Result<String, JsError> {
    let r = input(request)?;
    Ok(hex(
        swap_fees::lp_fee(r.amount_in, r.fee_pips, r.protocol).map_err(js_error)?
    ))
}

/// What `liquidity` in range earns of `fee` paid to `active_liquidity`, as hex.
///
/// # Errors
///
/// Throws for a malformed number.
#[wasm_bindgen(js_name = feeShare)]
pub fn fee_share(fee: &str, liquidity: &str, active_liquidity: &str) -> Result<String, JsError> {
    Ok(hex(swap_fees::fee_share(
        parse(fee)?,
        parse(liquidity)?,
        parse(active_liquidity)?,
    )
    .map_err(js_error)?))
}

/// The amount of a token worth `usd_wad` at its USD price, rounded down (zero at a zero price), as hex.
///
/// # Errors
///
/// Throws for a malformed number.
#[wasm_bindgen(js_name = amountForUsd)]
pub fn amount_for_usd(usd_wad: &str, decimals: u8, price_wad: &str) -> Result<String, JsError> {
    Ok(hex(crate::lens::amount_for_usd(
        parse(usd_wad)?,
        parse(price_wad)?,
        decimals,
    )
    .map_err(js_error)?))
}
