/* tslint:disable */
/* eslint-disable */

/** uint256 as a string: "0x"-prefixed hex on output, hex or decimal on input. `BigInt(value)` reads it. */
export type U256 = string;
/** uint160 (sqrt prices) as a string, like U256. */
export type U160 = string;
/** int256 as a decimal string. `BigInt(value)` reads it. */
export type I256 = string;


/**
 * A borrowed asset as the math sees it: one of the pair's tokens, or an unrelated token with its own price.
 */
export type Asset = "token0" | "token1" | { other: TokenInput };

/**
 * A debt and its value, grown.
 */
export interface AccruedDebt {
    /**
     * Owed now.
     */
    assets: U256;
    /**
     * Its USD value now, at the read's price.
     */
    valueUsdWad: U256;
}

/**
 * A debt in a token outside the pair (a liquid token, as WETH on NVDA/USDG): an edit leaves it as it is, and the
 * health check stresses it up by its own σ.
 */
export interface OtherDebt {
    /**
     * Amount owed.
     */
    assets: U256;
    /**
     * The token's oracle price, decimals and σ.
     */
    token: TokenInput;
}

/**
 * A debt of `amount` of `asset` against a position, valued as the collateral values it.
 */
export interface DebtRequest {
    /**
     * The position.
     */
    collateral: CollateralInput;
    /**
     * The borrowed asset.
     */
    asset: Asset;
    /**
     * Amount owed, in the asset's units.
     */
    amount: U256;
}

/**
 * A debt outside the pair (a liquid token, as WETH on NVDA/USDG), valued at its own market's price.
 */
export interface OutsideDebt {
    /**
     * Amount owed.
     */
    assets: U256;
    /**
     * The borrowed token's USD price and decimals.
     */
    price: Priced;
}

/**
 * A debt read some seconds ago.
 */
export interface AccrueDebtRequest {
    /**
     * How its rate is known.
     */
    rate: DebtRate;
    /**
     * Seconds since the read.
     */
    elapsed: string;
    /**
     * Owed at the read.
     */
    assets: U256;
    /**
     * Its USD value at the read.
     */
    valueUsdWad: U256;
}

/**
 * A deposit after a swap, within a slippage.
 */
export interface MinimumsAfterSwapRequest {
    /**
     * One end of the range.
     */
    sqrtPaX96: U160;
    /**
     * The other end.
     */
    sqrtPbX96: U160;
    /**
     * The pool's sqrt price now.
     */
    sqrtPriceX96: U160;
    /**
     * The deposit before the swap.
     */
    amounts: PerToken<U256>;
    /**
     * The swap, paying only its least output.
     */
    swap: RangeSwap | undefined;
    /**
     * Accepted price move, in basis points.
     */
    slippageBps: string;
}

/**
 * A deposit of one token into a range.
 */
export interface PairedAmountRequest {
    /**
     * The range.
     */
    range: TickRange;
    /**
     * The pool's sqrt price now.
     */
    poolSqrtPriceX96: U160;
    /**
     * The token deposited.
     */
    from: Side;
    /**
     * How much of it.
     */
    amount: U256;
}

/**
 * A deposit of up to `amount0` and `amount1` in a range, at a pool price, within a slippage.
 */
export interface DepositBand {
    /**
     * One end of the range, a Q64.96 sqrt price.
     */
    sqrtPaX96: U160;
    /**
     * The other end.
     */
    sqrtPbX96: U160;
    /**
     * Most token0 the deposit may take.
     */
    amount0: U256;
    /**
     * Most token1 the deposit may take.
     */
    amount1: U256;
    /**
     * The pool's sqrt price now.
     */
    sqrtPriceX96: U160;
    /**
     * Accepted pool price move, in basis points (50 = 0.5%).
     */
    slippageBps: string;
}

/**
 * A deposit to bring into a range's ratio.
 */
export interface RangeSwapRequest {
    /**
     * One end of the range.
     */
    sqrtPaX96: U160;
    /**
     * The other end.
     */
    sqrtPbX96: U160;
    /**
     * The deposit.
     */
    amounts: PerToken<U256>;
    /**
     * The pool's sqrt price now.
     */
    poolSqrtPriceX96: U160;
    /**
     * The pool's fee, 1e18 = 100%.
     */
    swapFeeWad: U256;
    /**
     * Accepted price move, in basis points.
     */
    slippageBps: string;
}

/**
 * A deposit to fit into a range without a swap.
 */
export interface FitRequest {
    /**
     * One end of the range.
     */
    sqrtPaX96: U160;
    /**
     * The other end.
     */
    sqrtPbX96: U160;
    /**
     * Where it mints.
     */
    sqrtPriceX96: U160;
    /**
     * The deposit.
     */
    amounts: PerToken<U256>;
}

/**
 * A holder's fee checkpoint in one position id (`FeeCheckpoint` in `Types.sol`).
 */
export interface FeeCheckpoint {
    /**
     * Token0 fee growth inside the range at the last checkpoint (Q128).
     */
    feeGrowthInside0X128: U256;
    /**
     * Token1 fee growth inside the range at the last checkpoint (Q128).
     */
    feeGrowthInside1X128: U256;
    /**
     * Token0 fees settled and not yet collected.
     */
    owed0: string;
    /**
     * Token1 fees settled and not yet collected.
     */
    owed1: string;
}

/**
 * A holder's token ledger in a position over its life, per pair token (token units): liquidity added (opens,
 * increases, compounds) and taken out (redeems, fees paid out, the close), and debt borrowed less repaid.
 *
 * What the holder put in, net, is liquidity added less liquidity taken out less debt: holding exactly that instead is
 * what providing liquidity is measured against. Swaps the router makes on the way in or out are trades, not liquidity
 * provision, so they stay out of it (they are in the profit).
 */
export interface TokenFlows {
    /**
     * Token0 added as liquidity.
     */
    added0: U256;
    /**
     * Token1 added as liquidity.
     */
    added1: U256;
    /**
     * Token0 taken out of the liquidity, fees paid out included.
     */
    removed0: U256;
    /**
     * Token1 taken out.
     */
    removed1: U256;
    /**
     * Token0 borrowed less repaid (negative after repaying more than was borrowed: interest).
     */
    borrowedNet0: I256;
    /**
     * Token1 borrowed less repaid.
     */
    borrowedNet1: I256;
}

/**
 * A lender's booked interest and shares, and the live market.
 */
export interface LenderEarnedRequest {
    /**
     * Interest booked by the indexer (asset units; negative after bad debt).
     */
    earnedAssets: I256;
    /**
     * The lender's shares as indexed.
     */
    shares: U256;
    /**
     * The booked rate (assets per 1e36 shares).
     */
    lastRate: U256;
    /**
     * The lender's assets now (zero with `live_shares` zero: no live read).
     */
    liveAssets: U256;
    /**
     * The lender's shares now.
     */
    liveShares: U256;
}

/**
 * A lender's deposit and the shares asked for.
 */
export interface WithdrawalRequest {
    /**
     * What the deposit is worth.
     */
    depositAssets: U256;
    /**
     * Its shares.
     */
    depositShares: U256;
    /**
     * Shares asked for; absent for all.
     */
    askedShares: U256 | undefined;
}

/**
 * A leverage increase's borrow into a range.
 */
export interface LeverageUpSwapRequest {
    /**
     * One end of the range.
     */
    sqrtPaX96: U160;
    /**
     * The other end.
     */
    sqrtPbX96: U160;
    /**
     * What is borrowed.
     */
    borrow: PerToken<U256>;
    /**
     * The pool's sqrt price now.
     */
    poolSqrtPriceX96: U160;
    /**
     * The pool's fee, 1e18 = 100%.
     */
    swapFeeWad: U256;
    /**
     * Accepted price move, in basis points.
     */
    slippageBps: string;
}

/**
 * A liquidation's sale and its quote.
 */
export interface LiquidationMinOutRequest {
    /**
     * From `liquidationAmounts`.
     */
    amounts: LiquidationAmounts;
    /**
     * The quote for selling `amounts.sellAmount`; without one the pool price is used.
     */
    quotedOut: U256 | undefined;
    /**
     * The pool's sqrt price now.
     */
    poolSqrtPriceX96: U160;
    /**
     * The pool's fee, 1e18 = 100%.
     */
    swapFeeWad: U256;
    /**
     * Accepted price move, in basis points.
     */
    slippageBps: string;
}

/**
 * A market after accrual, with its rates.
 */
export interface AccrueResult {
    /**
     * The market accrued to `now`.
     */
    market: Market;
    /**
     * What the accrual booked.
     */
    accrual: Accrual;
    /**
     * Borrow rate per second after accrual.
     */
    borrowRatePerSecond: U256;
    /**
     * Supply rate per second after accrual.
     */
    supplyRatePerSecond: U256;
}

/**
 * A market and how its cash moves.
 */
export interface MarketAprsAfterRequest {
    /**
     * Its cash now.
     */
    cash: U256;
    /**
     * Its borrows.
     */
    borrows: U256;
    /**
     * The yearly rate at target.
     */
    rateAtTargetAprWad: U256;
    /**
     * Share of interest to the reserves.
     */
    reserveFactorWad: U256;
    /**
     * Cash added (a supply).
     */
    cashAdded: U256;
    /**
     * Cash taken (a withdrawal or a borrow).
     */
    cashTaken: U256;
}

/**
 * A market's borrows, rate and reserve factor.
 */
export interface MarketFlowsRequest {
    /**
     * What is borrowed.
     */
    borrowAssets: U256;
    /**
     * The borrow rate.
     */
    borrowAprWad: U256;
    /**
     * Share of interest to the reserves.
     */
    reserveFactorWad: U256;
}

/**
 * A market's curve inputs at a utilization.
 */
export interface MarketAprsRequest {
    /**
     * The yearly rate at target (1e18 = 100%).
     */
    rateAtTargetAprWad: U256;
    /**
     * Share of interest to the reserves.
     */
    reserveFactorWad: U256;
    /**
     * Utilization (1e18 = 100%).
     */
    utilizationWad: U256;
}

/**
 * A market's interest over a year at today's borrows and rate, in its asset.
 */
export interface MarketFlows {
    /**
     * What borrowers pay.
     */
    interest: U256;
    /**
     * The reserves' share of it.
     */
    reserves: U256;
    /**
     * The lenders' share.
     */
    lenders: U256;
}

/**
 * A market's yearly rates (1e18 = 100%).
 */
export interface MarketAprs {
    /**
     * The utilization they are at.
     */
    utilizationWad: U256;
    /**
     * What borrowers pay.
     */
    borrowAprWad: U256;
    /**
     * What lenders earn: the borrow rate on the borrowed part, less the reserve factor.
     */
    supplyAprWad: U256;
}

/**
 * A new position's earnings, projected.
 */
export interface OpenEarningsInput {
    /**
     * The range's fee APR ([`range_fee_apr`]).
     */
    feeApr: number;
    /**
     * The position's size (USD, 1e18).
     */
    sizeUsdWad: U256;
    /**
     * Its margin.
     */
    marginUsdWad: U256;
    /**
     * Its debt.
     */
    debtUsdWad: U256;
    /**
     * The debt's average yearly rate.
     */
    borrowAprWad: U256;
    /**
     * The opening fee (1e18 = 100%).
     */
    borrowFeeWad: U256;
    /**
     * Days held.
     */
    days: number;
}

/**
 * A new position's inputs for its net yearly rate.
 */
export interface OpenNetAprRequest {
    /**
     * The range's fee APR.
     */
    feeApr: number;
    /**
     * The position's size (USD, 1e18).
     */
    sizeUsdWad: U256;
    /**
     * Its debt.
     */
    debtUsdWad: U256;
    /**
     * The debt's average rate.
     */
    borrowAprWad: U256;
    /**
     * The margin to rate against.
     */
    marginUsdWad: U256;
}

/**
 * A payoff's two lines at one price (USD).
 */
export interface PayoffPoint {
    /**
     * The position's profit or loss there.
     */
    pnl: number;
    /**
     * Holding what it holds now instead.
     */
    hold: number;
}

/**
 * A plain liquidity position (no debt), as an indexer and the chain see it. Amounts in raw token units, values in USD.
 */
export interface LpInput {
    /**
     * Token0 its liquidity holds now.
     */
    held0: number;
    /**
     * Token1.
     */
    held1: number;
    /**
     * Token0 fees owed.
     */
    owed0: number;
    /**
     * Token1 fees owed.
     */
    owed1: number;
    /**
     * Token0 deposited less withdrawn: what holding would have kept.
     */
    kept0: number;
    /**
     * Token1.
     */
    kept1: number;
    /**
     * Token0's decimals.
     */
    decimals0: number;
    /**
     * Token1's decimals.
     */
    decimals1: number;
    /**
     * Token0's USD price now.
     */
    usd0: number;
    /**
     * Token1's.
     */
    usd1: number;
    /**
     * Rewards earned, claimed and to claim.
     */
    rewardsUsd: number;
    /**
     * Token0 fees already collected.
     */
    collected0: number;
    /**
     * Token1 fees already collected.
     */
    collected1: number;
    /**
     * Fees already collected, in USD when each was collected: what [`LpMetrics::pnl`] and the fee APR count.
     */
    feesCollectedUsd: number;
    /**
     * Everything withdrawn.
     */
    withdrawnUsd: number;
    /**
     * Everything deposited.
     */
    depositedUsd: number;
    /**
     * Seconds since the open.
     */
    ageSeconds: number;
}

/**
 * A plain liquidity position's value and returns (USD).
 */
export interface LpMetrics {
    /**
     * What its liquidity holds.
     */
    valueUsd: number;
    /**
     * Fees owed.
     */
    owedUsd: number;
    /**
     * Fees collected and owed, and rewards.
     */
    feesEarnedUsd: number;
    /**
     * Value, fees, rewards and withdrawals less deposits.
     */
    pnl: number;
    /**
     * Of what was deposited, in percent.
     */
    pnlPct: number;
    /**
     * Against holding what was deposited less withdrawn, every token at today's prices (collected fees included).
     */
    vsHold: number;
    /**
     * What was earned over what was deposited, a year at this pace, in percent.
     */
    feeApr: number;
}

/**
 * A position after an edit, to rate.
 */
export interface EditAprInput {
    /**
     * Its margin after the edit (USD, 1e18).
     */
    marginUsdWad: I256;
    /**
     * Its debt after.
     */
    debtUsdWad: U256;
    /**
     * Its liquidity after.
     */
    liquidity: U256;
    /**
     * Its liquidity now (in the pool's active liquidity).
     */
    liquidityNow: U256;
    /**
     * The pool's active liquidity now.
     */
    poolLiquidity: U256;
    /**
     * The pool price now.
     */
    sqrtPriceX96: U160;
    /**
     * One end of the range.
     */
    sqrtPaX96: U160;
    /**
     * The other end.
     */
    sqrtPbX96: U160;
    /**
     * Staked in a gauge.
     */
    staked: boolean;
    /**
     * The pool's swap fees a year, in USD.
     */
    feesPerYearUsd: number;
    /**
     * Staked: the rewards the liquidity after the edit earns a year in range; `None` while unpriced (counted as 0).
     */
    rewardsPerYearUsd: number | undefined;
    /**
     * The debt's average yearly rate.
     */
    borrowAprWad: U256;
}

/**
 * A position and a slippage.
 */
export interface CompoundRequest {
    /**
     * The position.
     */
    position: ClosingPosition;
    /**
     * Accepted price move, in basis points.
     */
    slippageBps: string;
}

/**
 * A position and how it would borrow more.
 */
export interface EditMaxLeverageRequest {
    /**
     * The position.
     */
    position: EditPosition;
    /**
     * How it borrows.
     */
    borrow: BorrowSplit;
    /**
     * The deployment's opening fee.
     */
    borrowFeeWad: U256;
}

/**
 * A position and how its pair is priced at its pool.
 */
export interface MarkRequest {
    /**
     * The pair's prices at the pool.
     */
    pricing: PoolPricing;
    /**
     * The position.
     */
    position: MarkedPosition;
}

/**
 * A position being closed, its debts and the pair's prices.
 */
export interface CloseRequest {
    /**
     * The position.
     */
    position: ClosingPosition;
    /**
     * Its debts, in order.
     */
    debts: ClosingDebt[];
    /**
     * The pair's prices (`poolValuedPrices` when one is not live); null without a market.
     */
    prices: PerToken<Priced | undefined>;
    /**
     * Accepted price move, in basis points.
     */
    slippageBps: string;
    /**
     * The pool's fee, 1e18 = 100%.
     */
    swapFeeWad: U256;
}

/**
 * A position being liquidated and the debt repaid.
 */
export interface Liquidated {
    /**
     * One end of the range, a Q64.96 sqrt price.
     */
    sqrtPaX96: U160;
    /**
     * The other end.
     */
    sqrtPbX96: U160;
    /**
     * The account's liquidity.
     */
    liquidity: string;
    /**
     * The pool's sqrt price now.
     */
    poolSqrtPriceX96: U160;
    /**
     * The collateral's value in USD (1e18), from the lens.
     */
    collateralUsdWad: U256;
    /**
     * All its debt in USD.
     */
    debtUsdWad: U256;
    /**
     * The repaid debt's value in USD.
     */
    repaidUsdWad: U256;
    /**
     * The repaid debt's penalty (`repayPenaltyWad`), or the position's.
     */
    penaltyWad: U256;
    /**
     * The pair token repaid; `None` for a token outside the pair.
     */
    repaid: Side | undefined;
}

/**
 * A position being liquidated, within a slippage.
 */
export interface LiquidationRequest {
    /**
     * The position and the debt repaid.
     */
    position: Liquidated;
    /**
     * Accepted price move, in basis points.
     */
    slippageBps: string;
}

/**
 * A position in a state.
 */
export interface PreviewStateRequest {
    /**
     * The position.
     */
    position: EditPosition;
    /**
     * The state to preview.
     */
    state: EditState;
    /**
     * Also search the liquidation prices (the slow part).
     */
    withLiquidationPrices: boolean;
}

/**
 * A position in its state now.
 */
export interface PositionStateRequest {
    /**
     * The position.
     */
    position: EditPosition;
    /**
     * Its state now.
     */
    state: EditState;
}

/**
 * A position in some state: its liquidity and each pair debt, in token units.
 */
export interface EditState {
    /**
     * The account's liquidity.
     */
    liquidity: string;
    /**
     * Debt in each pair token.
     */
    debt: PerToken<U256>;
}

/**
 * A position to chart.
 */
export interface PayoffInputs {
    /**
     * Its liquidity.
     */
    liquidity: U256;
    /**
     * The range's lower sqrt price.
     */
    sqrtLowerX96: U160;
    /**
     * Its upper sqrt price.
     */
    sqrtUpperX96: U160;
    /**
     * Token0's decimals.
     */
    decimals0: number;
    /**
     * Token1's decimals.
     */
    decimals1: number;
    /**
     * Token0 owed, the opening fee included.
     */
    debt0: U256;
    /**
     * Token1 owed.
     */
    debt1: U256;
    /**
     * The pool price now, token1 per token0 in whole tokens.
     */
    price: number;
    /**
     * Token1's USD price.
     */
    token1Usd: number;
}

/**
 * A position to close or value: its range, liquidity, the pool's price and the fees it is owed.
 */
export interface ClosingPosition {
    /**
     * One end of the range, a Q64.96 sqrt price.
     */
    sqrtPaX96: U160;
    /**
     * The other end.
     */
    sqrtPbX96: U160;
    /**
     * The account's liquidity.
     */
    liquidity: string;
    /**
     * The pool's sqrt price now, where the redeem executes.
     */
    poolSqrtPriceX96: U160;
    /**
     * Token0 fees owed to the account.
     */
    feesOwed0: U256;
    /**
     * Token1 fees owed to the account.
     */
    feesOwed1: U256;
}

/**
 * A position valued at its pool's price (USD in 1e18).
 */
export interface Mark {
    /**
     * Token0's USD price at the pool.
     */
    price0Wad: U256;
    /**
     * Token1's.
     */
    price1Wad: U256;
    /**
     * The liquidity's token0 at the pool price.
     */
    amount0: U256;
    /**
     * Its token1.
     */
    amount1: U256;
    /**
     * The liquidity's value.
     */
    sizeUsdWad: U256;
    /**
     * The debts' value.
     */
    debtUsdWad: U256;
    /**
     * Size less debt; negative once the debt is worth more.
     */
    equityUsdWad: I256;
    /**
     * The fees owed's value.
     */
    feesUsdWad: U256;
}

/**
 * A position with what its previews need: its range, the pair's oracle prices and σ, and the penalties.
 */
export interface EditPosition {
    /**
     * One end of the range, a Q64.96 sqrt price.
     */
    sqrtPaX96: U160;
    /**
     * The other end.
     */
    sqrtPbX96: U160;
    /**
     * The pair's oracle prices, decimals and σ.
     */
    tokens: PerToken<TokenInput>;
    /**
     * The oracle's sqrt price (the lens's `oracleSqrtPriceX96`).
     */
    oracleSqrtPriceX96: U160;
    /**
     * The position's liquidation penalty (its debts outside the pair included).
     */
    positionPenaltyWad: U256;
    /**
     * The pair's.
     */
    pairPenaltyWad: U256;
    /**
     * Debts outside the pair.
     */
    otherDebts?: OtherDebt[];
}

/**
 * A position's USD liquidation prices and its pair's oracle prices.
 */
export interface LiquidationPoolPricesRequest {
    /**
     * The token whose price moves.
     */
    moving: Side;
    /**
     * Its lower USD liquidation price (1e18).
     */
    lowerWad: U256 | undefined;
    /**
     * Its upper one.
     */
    upperWad: U256 | undefined;
    /**
     * Token0's USD price (1e18).
     */
    price0Wad: U256;
    /**
     * Token1's.
     */
    price1Wad: U256;
}

/**
 * A position's debts' flows.
 */
export interface InterestPaidRequest {
    /**
     * One per debt asset.
     */
    debts: DebtFlow[];
}

/**
 * A position's debts, for picking one to liquidate.
 */
export interface LargestDebtRequest {
    /**
     * In the position's order.
     */
    debts: RankedDebt[];
}

/**
 * A position's liquidation prices as pool prices (token1 per token0, whole tokens, unsorted), and the pool price
 * now at oracle prices. For display.
 */
export interface LiquidationPoolPrices {
    /**
     * At the moving token's lower USD liquidation price; `None` without one.
     */
    atLower: number | undefined;
    /**
     * At its upper one.
     */
    atUpper: number | undefined;
    /**
     * At the oracle prices now.
     */
    now: number;
}

/**
 * A position's numbers in some state, at oracle prices (USD in 1e18).
 */
export interface EditPreview {
    /**
     * The collateral's value.
     */
    collateralUsdWad: U256;
    /**
     * The debt's value.
     */
    debtUsdWad: U256;
    /**
     * Collateral less debt, never negative.
     */
    marginUsdWad: U256;
    /**
     * Size over margin (1e18 = 1x), rounded down; zero without margin.
     */
    leverageWad: U256;
    /**
     * Health at the liquidation check: above 1e18 is safe; `U256::MAX` without debt.
     */
    healthWad: U256;
    /**
     * Passes the contract's check, so the transaction would go through.
     */
    allowed: boolean;
    /**
     * Also keeps the 2% safety buffer an SDK's maximums aim for.
     */
    withinMargin: boolean;
    /**
     * Where it would be liquidated; `None` without debt, or when not asked for.
     */
    liquidation: LiquidationPrices | undefined;
}

/**
 * A position's profit or loss.
 */
export interface Pnl {
    /**
     * Equity, fees, rewards and what was taken out, less what was put in.
     */
    usdWad: I256;
    /**
     * Of what was put in, in basis points (toward zero).
     */
    pctBps: I256;
    /**
     * What was put in: the deposits, or without them the opening margin.
     */
    entryUsdWad: U256;
    /**
     * Deposited after the open: zero unless over a cent and 1% of what was put in.
     */
    addedUsdWad: U256;
}

/**
 * A position's range, units, liquidity and the pool's price.
 */
export interface Redeemable {
    /**
     * One end of the range, a Q64.96 sqrt price.
     */
    sqrtPaX96: U160;
    /**
     * The other end.
     */
    sqrtPbX96: U160;
    /**
     * The account's units of the position.
     */
    units: U256;
    /**
     * The account's liquidity.
     */
    liquidity: string;
    /**
     * The pool's sqrt price now.
     */
    poolSqrtPriceX96: U160;
}

/**
 * A position's yearly rates.
 */
export interface PositionApr {
    /**
     * Fees and rewards less interest, over the margin.
     */
    net: number;
    /**
     * Fees over the size.
     */
    fees: number;
    /**
     * Rewards over the size; `None` while they cannot be priced.
     */
    rewards: number | undefined;
    /**
     * The debt's yearly rate.
     */
    borrow: number;
}

/**
 * A position, its pool pricing and the pool's fee.
 */
export interface MarkReceiveRequest {
    /**
     * The pair's prices at the pool.
     */
    pricing: PoolPricing;
    /**
     * The position.
     */
    position: MarkedPosition;
    /**
     * The pool's fee (1e18 = 100%), paid on a swap for a short debt.
     */
    swapFeeWad: U256;
}

/**
 * A position, its state and a target leverage.
 */
export interface LeverageChangeRequest {
    /**
     * The position.
     */
    position: EditPosition;
    /**
     * Its state now.
     */
    state: EditState;
    /**
     * Size over margin to reach, 1e18 = 1x.
     */
    targetLeverageWad: U256;
    /**
     * How an increase borrows.
     */
    borrow: BorrowSplit;
    /**
     * Where a pair borrow takes the range's ratio, usually the pool's sqrt price.
     */
    splitSqrtPriceX96: U160;
}

/**
 * A position, its state and the margin added.
 */
export interface AddMarginStateRequest {
    /**
     * The position.
     */
    position: EditPosition;
    /**
     * Its state now.
     */
    state: EditState;
    /**
     * Margin added, in token units.
     */
    margin: PerToken<U256>;
}

/**
 * A range and the liquidity in it. A `CollateralInput` is one too: its other fields are ignored.
 */
export interface LiquidityRange {
    /**
     * One end, a Q64.96 sqrt price.
     */
    sqrtPaX96: U160;
    /**
     * The other end.
     */
    sqrtPbX96: U160;
    /**
     * The liquidity in the range.
     */
    liquidity: string;
}

/**
 * A range by its ends and the pair's oracle prices: what valuing it needs.
 */
export interface PricedRange {
    /**
     * One end, a Q64.96 sqrt price.
     */
    sqrtPaX96: U160;
    /**
     * The other end.
     */
    sqrtPbX96: U160;
    /**
     * The pair's oracle prices.
     */
    prices: PerToken<Priced>;
}

/**
 * A range by its ends, as Q64.96 sqrt prices (either order).
 */
export interface SqrtRange {
    /**
     * One end.
     */
    sqrtPaX96: U160;
    /**
     * The other end.
     */
    sqrtPbX96: U160;
}

/**
 * A range by its ticks.
 */
export interface TickRange {
    /**
     * The lower tick.
     */
    tickLower: number;
    /**
     * The upper tick.
     */
    tickUpper: number;
}

/**
 * A range with prices, at a price.
 */
export interface PairShareRequest {
    /**
     * The range and the pair's oracle prices.
     */
    range: PricedRange;
    /**
     * Where to value it, usually the pool's sqrt price.
     */
    sqrtPriceX96: U160;
}

/**
 * A range's two ends as pool prices.
 */
export interface PriceRange {
    /**
     * At its lower tick.
     */
    lower: number;
    /**
     * At its upper tick.
     */
    upper: number;
}

/**
 * A rate and the value it applies to.
 */
export interface ValuedRate {
    /**
     * The value, in USD (1e18).
     */
    valueUsdWad: U256;
    /**
     * Its yearly rate (1e18 = 100%).
     */
    aprWad: U256;
}

/**
 * A redeem of a share of a position: the units it takes and its minimums.
 */
export interface RedeemShare {
    /**
     * `units · fraction`, rounded down.
     */
    units: U256;
    /**
     * The fraction is all of it: redeem every unit (the router's `ALL`).
     */
    all: boolean;
    /**
     * Least token0 it pays anywhere in the slippage band.
     */
    min0: U256;
    /**
     * Least token1.
     */
    min1: U256;
}

/**
 * A share of a position to redeem.
 */
export interface RedeemShareRequest {
    /**
     * The position.
     */
    position: Redeemable;
    /**
     * Share redeemed, 1e18 = all.
     */
    fractionWad: U256;
    /**
     * Accepted price move, in basis points.
     */
    slippageBps: string;
}

/**
 * A state and the share of its liquidity redeemed.
 */
export interface RemoveMarginStateRequest {
    /**
     * The state now.
     */
    state: EditState;
    /**
     * Share redeemed, 1e18 = all.
     */
    fractionWad: U256;
}

/**
 * A state and what is repaid.
 */
export interface RepayStateRequest {
    /**
     * The state now.
     */
    state: EditState;
    /**
     * Repaid, in token units.
     */
    repay: PerToken<U256>;
}

/**
 * A swap of one pair token into the other, with its least output.
 */
export interface RangeSwap {
    /**
     * The token sold.
     */
    sell: Side;
    /**
     * Amount sold.
     */
    amountIn: U256;
    /**
     * Least it buys.
     */
    minOut: U256;
}

/**
 * A swap of one pair token into the other.
 */
export interface DebtSwap {
    /**
     * The token sold; the other one is bought.
     */
    sell: Side;
    /**
     * Amount sold.
     */
    amountIn: U256;
    /**
     * Least it must buy.
     */
    minOut: U256;
}

/**
 * A swap to tilt toward a one-token debt.
 */
export interface LeanRequest {
    /**
     * The swap.
     */
    swap: RangeSwap;
    /**
     * The borrowed token.
     */
    debt: Side;
    /**
     * Most there is to sell of the other token.
     */
    available: U256;
}

/**
 * A swap's fee to liquidity providers.
 */
export interface LpFeeRequest {
    /**
     * What the swapper paid in.
     */
    amountIn: U256;
    /**
     * The swap fee, in pips.
     */
    feePips: number;
    /**
     * The protocol's cut.
     */
    protocol: ProtocolFee;
}

/**
 * A swap, or none.
 */
export interface MaybeSwap {
    /**
     * Undefined when nothing needs swapping.
     */
    swap: RangeSwap | undefined;
}

/**
 * A token's USD price and decimals: what converting between two tokens at the oracle needs.
 */
export interface Priced {
    /**
     * USD per whole token, 1e18 = $1.
     */
    priceWad: U256;
    /**
     * ERC-20 decimals.
     */
    decimals: number;
}

/**
 * A token's oracle price, as a market reports it.
 */
export interface MarketPrice {
    /**
     * USD per whole token, 1e18 = $1; zero when the feed has none.
     */
    priceWad: U256;
    /**
     * ERC-20 decimals.
     */
    decimals: number;
    /**
     * The price is a weekend's last close: shown, never traded on.
     */
    lastClose: boolean;
}

/**
 * A walk out from the price now.
 */
export interface DepthWalkRequest {
    /**
     * The raw sqrt price now.
     */
    sqrtNow: number;
    /**
     * The tick now.
     */
    tickNow: number;
    /**
     * Upward or downward.
     */
    up: boolean;
    /**
     * How many ticks to walk.
     */
    maxTicks: number;
    /**
     * Ticks between points.
     */
    resolution: number;
    /**
     * Token0's decimals.
     */
    decimals0: number;
    /**
     * Token1's decimals.
     */
    decimals1: number;
}

/**
 * A withdrawal: the shares burned and the assets they pay.
 */
export interface Withdrawal {
    /**
     * Shares burned: those asked for, at most the whole deposit.
     */
    shares: U256;
    /**
     * What they pay: the deposit's assets in proportion, rounded down, or all of them for the whole deposit.
     */
    assets: U256;
}

/**
 * An ended position against holding the tokens put in.
 */
export interface ClosedVsHoldInput {
    /**
     * The holder's token ledger, the close included.
     */
    flows: TokenFlows;
    /**
     * Token0's decimals.
     */
    decimals0: number;
    /**
     * Token1's decimals.
     */
    decimals1: number;
    /**
     * The pair's quote token (a dollar stablecoin).
     */
    quote: Side;
    /**
     * Its USD price.
     */
    quoteUsd: number;
    /**
     * The pool price at the end, token1 per token0.
     */
    exitPrice: number;
    /**
     * Rewards claimed over its life (USD).
     */
    rewardsUsd: number;
}

/**
 * An exact open (`OPEN_EXACT`, no swap): the liquidity margin plus debt mint at the pool price, and the most it may
 * borrow of each token: what the mint takes anywhere in the slippage band, less the margin.
 */
export interface ExactOpen {
    /**
     * Liquidity minted.
     */
    liquidity: string;
    /**
     * Most borrowed of each token.
     */
    maxBorrow: PerToken<U256>;
}

/**
 * An exact open's sizing and debt.
 */
export interface ExactOpenRequest {
    /**
     * The open.
     */
    sizing: OpenSizing;
    /**
     * What it borrows.
     */
    debt: PerToken<U256>;
}

/**
 * An open position against holding the tokens put in.
 */
export interface VsHoldInput {
    /**
     * What the position is worth to its owner now: its equity, fees owed, and rewards claimed and to claim.
     */
    valueUsdWad: I256;
    /**
     * The holder's token ledger.
     */
    flows: TokenFlows;
    /**
     * Token0's decimals.
     */
    decimals0: number;
    /**
     * Token1's decimals.
     */
    decimals1: number;
    /**
     * Token0's USD price now (the mark's).
     */
    price0Wad: U256;
    /**
     * Token1's.
     */
    price1Wad: U256;
}

/**
 * An open, what it borrows and its quoted swap.
 */
export interface OpenMinimumsRequest {
    /**
     * The open.
     */
    sizing: OpenSizing;
    /**
     * What it borrows.
     */
    debt: PerToken<U256>;
    /**
     * The swap into the range, its least output from the quote.
     */
    swap: RangeSwap | undefined;
}

/**
 * Both tokens' USD prices (1e18).
 */
export interface PoolPrices {
    /**
     * Token0's.
     */
    price0Wad: U256;
    /**
     * Token1's.
     */
    price1Wad: U256;
}

/**
 * Both tokens' USD prices.
 */
export interface UsdPrices {
    /**
     * Token0's.
     */
    usd0: number;
    /**
     * Token1's.
     */
    usd1: number;
}

/**
 * Breakdown of a liquidation in USD (1e18), at the lens's values.
 */
export interface LiquidationUsd {
    /**
     * Collateral value seized (USD, 1e18).
     */
    seizedUsdWad: U256;
    /**
     * Debt value repaid (USD, 1e18).
     */
    repaidUsdWad: U256;
    /**
     * Protocol liquidation fee (USD, 1e18): zero when collateral is under 1.02x the debt or when debt is written off.
     */
    protocolFeeUsdWad: U256;
    /**
     * Liquidator's net incentive (USD, 1e18): seized less repaid less protocol fee.
     */
    liquidatorIncentiveUsdWad: U256;
    /**
     * Debt written off (USD, 1e18): zero unless taking every unit leaves unpaid debt.
     */
    writtenOffUsdWad: U256;
}

/**
 * How [`max_leverage`] borrows.
 */
export type LeverageBorrow = "pairRatio" | { single: Asset };

/**
 * How a close buys one debt asset it cannot repay from the redeem.
 */
export interface CloseSwap {
    /**
     * Index of the debt in the input.
     */
    debt: number;
    /**
     * The pair token sold.
     */
    sell: Side;
    /**
     * Most the close may sell for this debt (`SWAP_FOR_DEBT`'s cap).
     */
    maxIn: U256;
    /**
     * What a fixed-amount swap (an aggregator's) sells: the shortfall at the pool's price now (zero when the redeem
     * covers the debt there), at most what the redeem pays at every end of the band. Quote this one.
     */
    exactIn: U256;
    /**
     * `max_in - exact_in`: the cap of the top-up after a fixed-amount swap.
     */
    topUp: U256;
}

/**
 * How a debt's price moves in the stress corners (`DebtTie` in `Types.sol`; `Untied` is its `None`).
 */
export type DebtTie = "untied" | "token0" | "token1";

/**
 * How a debt's rate is known: its market's curve inputs, or failing those its yearly rate at the read.
 */
export interface DebtRate {
    /**
     * The debt's yearly rate at the read: used without a market.
     */
    borrowAprWad: U256;
    /**
     * The market's yearly rate at target (the lens's `rateAtTargetAprWad`); with `utilization_wad`, the rate adapts
     * over the period as the contract's does.
     */
    rateAtTargetAprWad: U256 | undefined;
    /**
     * The market's utilization.
     */
    utilizationWad: U256;
}

/**
 * How a pair is priced at its pool: the quote token at its market's USD price, the other as the pool implies.
 */
export interface PoolPricing {
    /**
     * The pool's sqrt price.
     */
    poolSqrtPriceX96: U160;
    /**
     * The token priced by its market (USDG, ETH: the pair's quote).
     */
    quote: Side;
    /**
     * Its USD price (1e18).
     */
    quotePriceWad: U256;
    /**
     * Token0's decimals.
     */
    decimals0: number;
    /**
     * Token1's decimals.
     */
    decimals1: number;
}

/**
 * How a position gets to a target leverage: `state` is where it ends, the rest what moves (token units).
 */
export interface LeverageChange {
    /**
     * The position after the change.
     */
    state: EditState;
    /**
     * Up: what is borrowed.
     */
    borrow: PerToken<U256>;
    /**
     * Down: the share of the liquidity redeemed (1e18 = all).
     */
    redeemWad: U256;
    /**
     * Down: what is repaid.
     */
    repay: PerToken<U256>;
}

/**
 * Inputs of [`adaptive_rate`].
 */
export interface AdaptiveRateRequest {
    /**
     * Rate at target at the start of the period, per second.
     */
    rateAtTarget: U256;
    /**
     * Utilization at the start of the period, 1e18 = 100%.
     */
    utilization: U256;
    /**
     * Length of the period in seconds.
     */
    elapsed: U256;
}

/**
 * Inputs of [`fees_pending`].
 */
export interface FeesRequest {
    /**
     * The holder's checkpoint.
     */
    checkpoint: FeeCheckpoint;
    /**
     * The holder's units.
     */
    balance: U256;
    /**
     * Current token0 fee growth inside the range.
     */
    feeGrowth0X128: U256;
    /**
     * Current token1 fee growth inside the range.
     */
    feeGrowth1X128: U256;
}

/**
 * Inputs of [`swap_for_range`].
 */
export interface SwapRequest {
    /**
     * One end of the range.
     */
    sqrtPaX96: U160;
    /**
     * The other end.
     */
    sqrtPbX96: U160;
    /**
     * Token0 available.
     */
    amount0: U256;
    /**
     * Token1 available.
     */
    amount1: U256;
    /**
     * Price the swap executes at.
     */
    sqrtPriceX96: U160;
    /**
     * Pool fee, 1e18 = 100%.
     */
    swapFeeWad: U256;
}

/**
 * Inputs of a health check.
 */
export interface HealthRequest {
    /**
     * The position.
     */
    collateral: CollateralInput;
    /**
     * Its debts, valued in USD.
     */
    debts: DebtInput[];
    /**
     * The position's liquidation penalty, e.g. 1.08e18: the highest of its pair's tokens and its debts outside the
     * pair (the lens's `penaltyWad`).
     */
    penaltyWad: U256;
}

/**
 * Inputs of a liquidation price search.
 */
export interface LiquidationPricesRequest {
    /**
     * The position.
     */
    collateral: CollateralInput;
    /**
     * Its debts.
     */
    debts: DebtInput[];
    /**
     * The position's liquidation penalty, e.g. 1.08e18: the highest of its pair's tokens and its debts outside the
     * pair (the lens's `penaltyWad`).
     */
    penaltyWad: U256;
}

/**
 * Inputs of a liquidation.
 */
export interface LiquidateRequest {
    /**
     * The borrower's position, with `liquidity` for the borrower's `units`.
     */
    collateral: CollateralInput;
    /**
     * All its debts.
     */
    debts: DebtInput[];
    /**
     * The debt being repaid.
     */
    repaid: RepaidDebt;
    /**
     * The borrower's balance of the position id.
     */
    units: U256;
    /**
     * The most the liquidator repays ("0xff…ff" for as much as allowed).
     */
    maxRepayAssets: U256;
    /**
     * The position's liquidation penalty, e.g. 1.08e18: the highest of its pair's tokens and its debts outside the
     * pair (the lens's `penaltyWad`), for the health check.
     */
    penaltyWad: U256;
    /**
     * The repaid debt's penalty, what the liquidator seizes at: the higher of its pair's tokens, or the asset's own
     * when it's outside the pair and higher.
     */
    repayPenaltyWad: U256;
    /**
     * The DEX's protocol liquidation fee share of the bonus (1e18 = 100%, max 0.2e18; default 0).
     */
    liquidationFeeWad?: U256;
    /**
     * Whether the position owes another market besides `repaid` (default false).
     */
    hasOtherDebt?: boolean;
}

/**
 * Inputs of a max borrow search.
 */
export interface MaxBorrowRequest {
    /**
     * The position.
     */
    collateral: CollateralInput;
    /**
     * Its debts in other assets (its debt in `asset` comes from `account_shares`).
     */
    otherDebts: DebtInput[];
    /**
     * The asset to borrow.
     */
    asset: Asset;
    /**
     * Its market, accrued to now.
     */
    market: Market;
    /**
     * The position's borrow shares in `market`.
     */
    accountShares: U256;
    /**
     * The position's liquidation penalty, e.g. 1.08e18: the highest of its pair's tokens and its debts outside the
     * pair (the lens's `penaltyWad`).
     */
    penaltyWad: U256;
    /**
     * The deployment's opening fee.
     */
    borrowFeeWad: U256;
}

/**
 * Inputs of a max leverage search.
 */
export interface MaxLeverageRequest {
    /**
     * The pair, range and prices (`liquidity` ignored).
     */
    collateral: CollateralInput;
    /**
     * How the position borrows.
     */
    borrow: LeverageBorrow;
    /**
     * The position's liquidation penalty, e.g. 1.08e18: the highest of its pair's tokens and its debts outside the
     * pair (the lens's `penaltyWad`).
     */
    penaltyWad: U256;
    /**
     * The deployment's opening fee.
     */
    borrowFeeWad: U256;
}

/**
 * Inputs of an accrual.
 */
export interface AccrueRequest {
    /**
     * The market as last written on chain.
     */
    market: Market;
    /**
     * ERC-6909 total supply of the market's lender id.
     */
    totalSupplyShares: U256;
    /**
     * Timestamp to accrue to.
     */
    now: string;
}

/**
 * Inputs of the Create Position panel (`ISenzuLens.OpenParams` with prices already resolved).
 */
export interface OpenParams {
    /**
     * The pair, range and prices; `liquidity` is ignored.
     */
    collateral: CollateralInput;
    /**
     * Token0 the user deposits.
     */
    deposit0: U256;
    /**
     * Token1 the user deposits.
     */
    deposit1: U256;
    /**
     * What is borrowed, one entry per asset (the router borrows each once).
     */
    borrows: OpenBorrow[];
    /**
     * Swap into the range's ratio; `false` adds only what fits.
     */
    swap: boolean;
    /**
     * The new position's liquidation penalty: [`health::position_penalty`] of its pair and every asset it borrows
     * outside the pair (a zero amount is no debt), as `SenzuPreviews.previewOpen` reads it.
     */
    penaltyWad: U256;
    /**
     * The deployment's opening fee, 1e18 = 100%.
     */
    borrowFeeWad: U256;
}

/**
 * Interest booked by one [`Market::accrue`].
 */
export interface Accrual {
    /**
     * Interest added to both borrows and supply.
     */
    interest: U256;
    /**
     * Supply shares minted to the DAO reserves.
     */
    reserveShares: U256;
}

/**
 * Liquidity in a range, at a pool price, within a slippage.
 */
export interface LiquidityBand {
    /**
     * One end of the range, a Q64.96 sqrt price.
     */
    sqrtPaX96: U160;
    /**
     * The other end.
     */
    sqrtPbX96: U160;
    /**
     * The liquidity minted or redeemed.
     */
    liquidity: string;
    /**
     * The pool's sqrt price now.
     */
    sqrtPriceX96: U160;
    /**
     * Accepted pool price move, in basis points (50 = 0.5%).
     */
    slippageBps: string;
}

/**
 * One borrow of a new position.
 */
export interface OpenBorrow {
    /**
     * What is borrowed.
     */
    asset: Asset;
    /**
     * How much, before the opening fee.
     */
    amount: U256;
    /**
     * Its market, accrued to now.
     */
    market: Market;
}

/**
 * One debt asset's flows: what is owed now and what was borrowed less repaid.
 */
export interface DebtFlow {
    /**
     * Owed now; zero for a debt since repaid.
     */
    owed: U256;
    /**
     * Borrowed less repaid, over the position's life.
     */
    netBorrowed: I256;
    /**
     * What is owed now in USD (1e18).
     */
    valueUsdWad: U256;
}

/**
 * One debt of a position being closed.
 */
export interface ClosingDebt {
    /**
     * The pair token borrowed; `None` for a token outside the pair.
     */
    side: Side | undefined;
    /**
     * Amount owed.
     */
    assets: U256;
    /**
     * The borrowed token's price; `None` when its market has none.
     */
    price: Priced | undefined;
}

/**
 * One debt of a position, as a liquidator ranks it.
 */
export interface RankedDebt {
    /**
     * Amount owed.
     */
    assets: U256;
    /**
     * Its value in USD (1e18).
     */
    valueUsdWad: U256;
}

/**
 * One debt of a position, valued in USD at the current price (`DebtInput` in `Types.sol`).
 */
export interface DebtInput {
    /**
     * Debt in USD, 1e18 = $1, rounded up.
     */
    valueUsdWad: U256;
    /**
     * σ of the asset (only read for untied debts).
     */
    sigmaWad: U256;
    /**
     * How the debt moves in the corners.
     */
    tie: DebtTie;
}

/**
 * One of the pair's two tokens.
 */
export type Side = "token0" | "token1";

/**
 * One token deposited at the open.
 */
export interface Deposit {
    /**
     * The pair token; `None` for a token outside the pair (no comparison with holding then).
     */
    side: Side | undefined;
    /**
     * Amount, in token units.
     */
    amount: U256;
}

/**
 * One token's USD price and a pool price.
 */
export interface PoolPriceRequest {
    /**
     * The pool's sqrt price.
     */
    poolSqrtPriceX96: U160;
    /**
     * The token whose USD price is known.
     */
    known: Side;
    /**
     * Its USD price (1e18).
     */
    knownPriceWad: U256;
    /**
     * Token0's decimals.
     */
    decimals0: number;
    /**
     * Token1's decimals.
     */
    decimals1: number;
}

/**
 * One value for each of the pair's tokens.
 */
export interface PerToken<T> {
    /**
     * Token0's.
     */
    token0: T;
    /**
     * Token1's.
     */
    token1: T;
}

/**
 * Oracle price, decimals and stress factor of one token.
 */
export interface TokenInput {
    /**
     * USD per whole token, 1e18 = $1.
     */
    priceWad: U256;
    /**
     * ERC-20 decimals.
     */
    decimals: number;
    /**
     * Stress factor σ from the risk config, at least 1e18.
     */
    sigmaWad: U256;
}

/**
 * Outcome of one liquidation (what `LiquidationLogic.liquidate` returns).
 */
export interface Liquidation {
    /**
     * Assets the liquidator repays toward the debt.
     */
    repayAssets: U256;
    /**
     * Units of the position moved to the liquidator.
     */
    seizedUnits: U256;
    /**
     * Protocol liquidation fee in the repaid asset (0 on write-off or when the fee setting is 0).
     */
    feeAssets: U256;
}

/**
 * Rates and the values they apply to.
 */
export interface AverageAprRequest {
    /**
     * Each rate and its value.
     */
    rates: ValuedRate[];
}

/**
 * Result of [`preview_open_summary`]: [`OpenPreview`] without the liquidation prices.
 */
export interface OpenSummary {
    /**
     * Liquidity of the new position.
     */
    liquidity: string;
    /**
     * Token0 in the position at the oracle price.
     */
    amount0: U256;
    /**
     * Token1 in the position at the oracle price.
     */
    amount1: U256;
    /**
     * Position value ("Size").
     */
    collateralUsdWad: U256;
    /**
     * Value of the deposits.
     */
    marginUsdWad: U256;
    /**
     * Value of the debts, opening fee included.
     */
    debtUsdWad: U256;
    /**
     * Size / margin, 0 without margin.
     */
    leverageWad: U256;
    /**
     * Health of the liquidation check.
     */
    healthWad: U256;
    /**
     * Whether the borrow check passes, with at most two assets borrowed.
     */
    allowed: boolean;
}

/**
 * Result of [`preview_open`] (`ISenzuLens.OpenPreview`).
 */
export interface OpenPreview {
    /**
     * Liquidity of the new position.
     */
    liquidity: string;
    /**
     * Token0 in the position at the oracle price.
     */
    amount0: U256;
    /**
     * Token1 in the position at the oracle price.
     */
    amount1: U256;
    /**
     * Position value ("Size").
     */
    collateralUsdWad: U256;
    /**
     * Value of the deposits.
     */
    marginUsdWad: U256;
    /**
     * Value of the debts, opening fee included.
     */
    debtUsdWad: U256;
    /**
     * Size / margin, 0 without margin.
     */
    leverageWad: U256;
    /**
     * Health of the liquidation check.
     */
    healthWad: U256;
    /**
     * Whether the borrow check passes, with at most two assets borrowed.
     */
    allowed: boolean;
    /**
     * Liquidation prices of the new position.
     */
    liquidation: LiquidationPrices;
}

/**
 * Result of a health check (`HealthResult` in `Types.sol`).
 */
export interface HealthResult {
    /**
     * Worst corner of `collateral − debt · penalty`. Negative: the check fails.
     */
    minSurplusUsdWad: I256;
    /**
     * Worst corner of `collateral / (debt · penalty)`, `U256::MAX` without debt. Below 1e18: liquidatable.
     */
    healthWad: U256;
    /**
     * Collateral value at current (unstressed) prices.
     */
    collateralUsdWad: U256;
    /**
     * Debt value at current (unstressed) prices.
     */
    debtUsdWad: U256;
}

/**
 * Rounding direction of a share conversion.
 */
export type Rounding = "down" | "up";

/**
 * Shares booked by one [`Market::borrow`].
 */
export interface Borrow {
    /**
     * Borrow shares added to the position (for `assets + fee`).
     */
    shares: U256;
    /**
     * Supply shares minted to the DAO for the fee.
     */
    feeShares: U256;
}

/**
 * State of one lending market (the accounting fields of `Market` in `Types.sol`).
 */
export interface Market {
    /**
     * Cash plus borrows.
     */
    totalSupplyAssets: string;
    /**
     * Debt owed to lenders, interest included.
     */
    totalBorrowAssets: string;
    /**
     * Borrow shares of all positions.
     */
    totalBorrowShares: string;
    /**
     * Timestamp of the last accrual.
     */
    lastAccrual: string;
    /**
     * Share of interest minted to the DAO reserves, 1e18 = 100%.
     */
    reserveFactorWad: string;
    /**
     * Interest accrued but under one unit of the asset, times 1e18, carried to the next accrual.
     */
    interestRemainder: string;
    /**
     * Borrow rate at the target utilization as of `last_accrual`, per second (1e18 = 100%).
     */
    rateAtTarget: string;
}

/**
 * The borrow rate integrated over one accrual period and where it leaves the rate at target.
 */
export interface AdaptiveRate {
    /**
     * `∫ borrow rate dt` over the period, WAD (`0.04e18` is a year at 4%), rounded down: the debt grows
     * `e^integral` times.
     */
    integral: U256;
    /**
     * The rate at target at the end of the period, per second.
     */
    endRateAtTarget: U256;
}

/**
 * The borrower's fees owed and the share of its units a liquidation seizes.
 */
export interface SeizedFeesRequest {
    /**
     * The borrower's fees owed (`feesOwed`).
     */
    feesOwed: PerToken<U256>;
    /**
     * The share seized (1e18 = all), as `seizedShareWad` gives it.
     */
    seizedShareWad: U256;
}

/**
 * The collateral side of a health check (`CollateralInput` in `Types.sol`).
 */
export interface CollateralInput {
    /**
     * One end of the range as a Q64.96 sqrt price (full-range positions use `MIN_SQRT_PRICE`).
     */
    sqrtPaX96: U160;
    /**
     * The other end (full range: `MAX_SQRT_PRICE`).
     */
    sqrtPbX96: U160;
    /**
     * The account's share of the position's liquidity: `units · liquidity / totalUnits`.
     */
    liquidity: string;
    /**
     * Token0 of the pair.
     */
    token0: TokenInput;
    /**
     * Token1 of the pair.
     */
    token1: TokenInput;
}

/**
 * The debt being repaid in a liquidation.
 */
export interface RepaidDebt {
    /**
     * The position's debt in the asset, interest accrued (`debtOf`).
     */
    debtAssets: U256;
    /**
     * Oracle price of the asset, USD per whole token (1e18).
     */
    priceWad: U256;
    /**
     * Decimals of the asset.
     */
    decimals: number;
}

/**
 * The highest leverage a new position opens with, keeping the safety buffer.
 */
export interface OpenMaxLeverageRequest {
    /**
     * The range and the pair's prices and σ (`liquidity` ignored).
     */
    collateral: CollateralInput;
    /**
     * How it borrows.
     */
    borrow: BorrowSplit;
    /**
     * The pair's liquidation penalty, before the buffer.
     */
    penaltyWad: U256;
    /**
     * The deployment's opening fee.
     */
    borrowFeeWad: U256;
    /**
     * Where a mix compares its share with the pair's, usually the pool's sqrt price.
     */
    splitSqrtPriceX96: U160;
}

/**
 * The interest a position's debts have cost.
 */
export interface InterestPaid {
    /**
     * Per debt, in its token: owed less net borrowed; zero where nothing was paid.
     */
    paid: U256[];
    /**
     * All of it in USD, at each debt's value now (a debt since repaid counts nothing).
     */
    usdWad: U256;
}

/**
 * The margin in each token and the pair's prices.
 */
export interface MarginRequest {
    /**
     * The pair's oracle prices.
     */
    prices: PerToken<Priced>;
    /**
     * Margin in token units.
     */
    margin: PerToken<U256>;
}

/**
 * The numbers of a close: the redeem's minimums and a swap for each debt the redeem cannot cover.
 */
export interface CloseAmounts {
    /**
     * Least token0 the whole redeem pays.
     */
    min0: U256;
    /**
     * Least token1.
     */
    min1: U256;
    /**
     * In debt order.
     */
    swaps: CloseSwap[];
}

/**
 * The numbers of a liquidation: the liquidity seized, the redeem's minimums and what is sold into the debt asset.
 */
export interface LiquidationAmounts {
    /**
     * Liquidity seized.
     */
    liquidity: string;
    /**
     * Least token0 the redeem pays.
     */
    min0: U256;
    /**
     * Least token1.
     */
    min1: U256;
    /**
     * The pair token sold into the debt asset: the one that is not the debt (token0 for a debt outside the pair).
     */
    sell: Side;
    /**
     * What the redeem pays of it at the pool price: quote this amount. Zero: no swap.
     */
    sellAmount: U256;
    /**
     * What the redeem surely pays of it: the router sells all it holds, guaranteed only down to this.
     */
    least: U256;
}

/**
 * The numbers of an open: what it borrows and the swap into the range before it is quoted.
 */
export interface OpenAmounts {
    /**
     * Borrowed, in token units.
     */
    debt: PerToken<U256>;
    /**
     * The swap into the range; quote it and replace its least output with [`slippage::min_out`] of the quote.
     */
    swap: RangeSwap | undefined;
}

/**
 * The opening deposits and the pool price then.
 */
export interface EntryPricesRequest {
    /**
     * The opening deposits.
     */
    deposits: Deposit[];
    /**
     * Token0's decimals.
     */
    decimals0: number;
    /**
     * Token1's decimals.
     */
    decimals1: number;
    /**
     * The pool's price right after the open.
     */
    entrySqrtPriceX96: U160;
    /**
     * The opening margin's value (USD, 1e18).
     */
    openingUsdWad: U256;
}

/**
 * The pair's market prices and the pool's price.
 */
export interface PoolValuedRequest {
    /**
     * Token0's market.
     */
    token0: MarketPrice;
    /**
     * Token1's market.
     */
    token1: MarketPrice;
    /**
     * The pool's sqrt price now.
     */
    poolSqrtPriceX96: U160;
}

/**
 * The pool's protocol fee on a swap.
 */
export type ProtocolFee = { divisor: number } | { pips: number };

/**
 * The swap that turns two token amounts into exactly a range's ratio.
 */
export interface SwapQuote {
    /**
     * Sell token0 for token1 (`false`: sell token1 for token0).
     */
    zeroForOne: boolean;
    /**
     * Amount sold.
     */
    amountIn: U256;
    /**
     * Amount bought at the given price, after the swap fee.
     */
    amountOut: U256;
}

/**
 * The swaps of a deleverage, in order.
 */
export interface DebtSwaps {
    /**
     * Token0's debt first.
     */
    swaps: DebtSwap[];
}

/**
 * Token amounts and how the pair is priced at its pool.
 */
export interface ValueAtPoolRequest {
    /**
     * The pair's prices at the pool.
     */
    pricing: PoolPricing;
    /**
     * Amounts of each token.
     */
    amounts: PerToken<U256>;
}

/**
 * Token amounts of a liquidity amount in a range.
 */
export interface TokenAmounts {
    /**
     * Token0.
     */
    amount0: U256;
    /**
     * Token1.
     */
    amount1: U256;
}

/**
 * What [`mark`] values: a position's range, liquidity, debts and fees owed.
 */
export interface MarkedPosition {
    /**
     * One end of the range, a Q64.96 sqrt price.
     */
    sqrtPaX96: U160;
    /**
     * The other end.
     */
    sqrtPbX96: U160;
    /**
     * The account's liquidity.
     */
    liquidity: string;
    /**
     * Debt in each pair token.
     */
    debt: PerToken<U256>;
    /**
     * Debts outside the pair, at their own prices.
     */
    outsideDebts?: OutsideDebt[];
    /**
     * Fees earned and not collected.
     */
    feesOwed: PerToken<U256>;
}

/**
 * What a deleverage holds and repays.
 */
export interface SwapIntoDebtsRequest {
    /**
     * What the redeem surely paid.
     */
    held: PerToken<U256>;
    /**
     * What is repaid of each debt.
     */
    repays: PerToken<U256>;
    /**
     * The pair's prices.
     */
    prices: PerToken<Priced>;
    /**
     * Accepted price move, in basis points.
     */
    slippageBps: string;
    /**
     * The pool's fee, 1e18 = 100%.
     */
    swapFeeWad: U256;
}

/**
 * What a liquidation of one debt seizes, as the contract sizes it.
 */
export interface SeizedShareRequest {
    /**
     * The collateral's value in USD (1e18).
     */
    collateralUsdWad: U256;
    /**
     * All the position's debt in USD.
     */
    debtUsdWad: U256;
    /**
     * The repaid debt's value in USD.
     */
    repaidUsdWad: U256;
    /**
     * The repaid debt's penalty, or the position's.
     */
    penaltyWad: U256;
    /**
     * The DEX's liquidation fee share of the bonus (1e18 = 100%, max 0.2e18; default 0).
     */
    liquidationFeeWad?: U256;
    /**
     * Whether the position owes another debt besides the repaid one (default: `debtUsdWad > repaidUsdWad`).
     */
    hasOtherDebt?: boolean | undefined;
}

/**
 * What a new position would earn and pay over some days (USD).
 */
export interface OpenEarnings {
    /**
     * Fees a day.
     */
    feesPerDay: number;
    /**
     * Interest a day.
     */
    interestPerDay: number;
    /**
     * Fees less interest a day.
     */
    netPerDay: number;
    /**
     * Fees over the days.
     */
    fees: number;
    /**
     * Interest over the days.
     */
    interest: number;
    /**
     * Paid once at the open, on what is borrowed (rounded up, as the contract charges it).
     */
    openingFee: number;
    /**
     * Fees less interest and the opening fee.
     */
    net: number;
    /**
     * `net` over the margin.
     */
    roi: number;
}

/**
 * What a position borrows.
 */
export type BorrowSplit = "pair" | "token0" | "token1" | { mix: { token0Share: U256 } };

/**
 * What a position earns and pays a year.
 */
export interface PositionAprInput {
    /**
     * Its size (USD, 1e18).
     */
    sizeUsdWad: U256;
    /**
     * Its equity.
     */
    equityUsdWad: I256;
    /**
     * Its debt.
     */
    debtUsdWad: U256;
    /**
     * The debt's average yearly rate ([`average_apr`]).
     */
    borrowAprWad: U256;
    /**
     * Its liquidity.
     */
    liquidity: U256;
    /**
     * The pool's active liquidity, its own included.
     */
    poolLiquidity: U256;
    /**
     * The pool price now.
     */
    sqrtPriceX96: U160;
    /**
     * One end of the range.
     */
    sqrtPaX96: U160;
    /**
     * The other end.
     */
    sqrtPbX96: U160;
    /**
     * Staked in a gauge: earns rewards, not swap fees.
     */
    staked: boolean;
    /**
     * The pool's swap fees a year, in USD ([`fees_per_year`]).
     */
    feesPerYearUsd: number;
    /**
     * Staked: the rewards its liquidity earns a year in range, in USD; `None` while they cannot be priced.
     */
    rewardsPerYearUsd: number | undefined;
}

/**
 * What a position's profit is made of (USD in 1e18).
 */
export interface PnlInput {
    /**
     * Its equity now (size less debt, at the pool price).
     */
    equityUsdWad: I256;
    /**
     * Fees owed.
     */
    feesUsdWad: U256;
    /**
     * Rewards earned (claimed and to claim).
     */
    rewardsUsdWad: U256;
    /**
     * The margin put in at the open, at its block's price.
     */
    openingUsdWad: U256;
    /**
     * Everything deposited, the open included, when the indexer has it.
     */
    depositedUsdWad: U256 | undefined;
    /**
     * Everything taken out.
     */
    withdrawnUsdWad: U256;
}

/**
 * What a repayment removes.
 */
export interface Repay {
    /**
     * Assets taken from the payer.
     */
    assets: U256;
    /**
     * Borrow shares burned.
     */
    shares: U256;
}

/**
 * What an open borrows, split how.
 */
export interface BorrowAmountsRequest {
    /**
     * The range and the pair's oracle prices.
     */
    range: PricedRange;
    /**
     * Margin in token units.
     */
    margin: PerToken<U256>;
    /**
     * Size over margin, 1e18 = 1x.
     */
    leverageWad: U256;
    /**
     * How the debt splits.
     */
    borrow: BorrowSplit;
    /**
     * Where a pair borrow takes the range's ratio, usually the pool's sqrt price.
     */
    splitSqrtPriceX96: U160;
}

/**
 * What an open is sized from: the range and prices, the margin, the leverage and borrow, and the pool.
 */
export interface OpenSizing {
    /**
     * The range and the pair's oracle prices.
     */
    range: PricedRange;
    /**
     * Margin in token units.
     */
    margin: PerToken<U256>;
    /**
     * Size over margin, 1e18 = 1x.
     */
    leverageWad: U256;
    /**
     * What it borrows.
     */
    borrow: BorrowSplit;
    /**
     * Swap into the range's ratio; false mints only what fits.
     */
    swap: boolean;
    /**
     * The pool's sqrt price now, where the DEX mints.
     */
    poolSqrtPriceX96: U160;
    /**
     * The pool's fee, 1e18 = 100%.
     */
    swapFeeWad: U256;
    /**
     * Accepted price move, in basis points.
     */
    slippageBps: string;
}

/**
 * What closing sends back at the pool price.
 */
export interface MarkReceive {
    /**
     * Token0 received.
     */
    amount0: U256;
    /**
     * Token1 received.
     */
    amount1: U256;
    /**
     * Both, in USD at the pool price.
     */
    usdWad: U256;
}

/**
 * What liquidity holds at each end of the slippage band.
 */
export interface BandEnds {
    /**
     * At the lowest price.
     */
    low: TokenAmounts;
    /**
     * At the highest price.
     */
    high: TokenAmounts;
}

/**
 * What withdrawing a whole deposit can take now.
 */
export interface WithdrawAll {
    /**
     * Shares to burn.
     */
    shares: U256;
    /**
     * What comes out, at most the market's cash.
     */
    out: U256;
    /**
     * What stays supplied because the market's cash is lent out.
     */
    left: U256;
}

/**
 * Where a position gets liquidated as one token's USD price moves (the other's held fixed).
 */
export interface LiquidationPrices {
    /**
     * The token whose price moves: the more volatile one (higher σ; token1 on a tie).
     */
    moving: Side;
    /**
     * Its USD price (1e18) at liquidation below the current price, `None` within 1/1000x.
     */
    lowerWad: U256 | undefined;
    /**
     * Its USD price at liquidation above the current price, `None` within 1000x.
     */
    upperWad: U256 | undefined;
}

/**
 * Which way [`align_tick`] moves a tick.
 */
export type TickDirection = "down" | "up";


/**
 * Which tokens a range takes at the pool's tick: only token0 above the price, only token1 below it, both in range.
 *
 * # Errors
 *
 * Never in practice; the result crosses into JS.
 */
export function acceptedTokens(range: TickRange, pool_tick: number): PerToken;

/**
 * Accrues a market to `now` (`MarketLogic.accrue`), as the borrowable's views do: the exact integral of the
 * rate, compounded, the rate at target adapted; the rates returned are the instantaneous ones as of `now`.
 *
 * # Errors
 *
 * Throws where the contract reverts.
 */
export function accrue(request: AccrueRequest): AccrueResult;

/**
 * A debt grown by its interest since the read, as the contract accrues it.
 *
 * # Errors
 *
 * Throws where the rate math overflows (never for a real market).
 */
export function accrueDebt(request: AccrueDebtRequest): AccruedDebt;

/**
 * The borrow rate integrated over a period and the rate at target it ends at (`MarketLogic.adaptiveRate`), e.g.
 * to chart where a market's rate heads if its utilization holds. The debt grows `e^integral` times.
 *
 * # Errors
 *
 * Throws on overflow.
 */
export function adaptiveRate(request: AdaptiveRateRequest): AdaptiveRate;

/**
 * The state after adding margin as liquidity.
 *
 * # Errors
 *
 * Throws for liquidity over 128 bits.
 */
export function addMarginState(request: AddMarginStateRequest): EditState;

/**
 * What a holder is paid of `amount` of fees or rewards owed, the protocol's cut taken (rounded down), hex.
 *
 * # Errors
 *
 * Throws on overflow or a cut over 100%.
 */
export function afterCut(amount: string, cut_wad: string): string;

/**
 * `tick` moved down or up to a multiple of the pool's tick spacing, kept inside the ticks a pool allows.
 *
 * # Errors
 *
 * Throws for a spacing below 1 or a NaN tick.
 */
export function alignTick(tick: number, spacing: number, direction: TickDirection): number;

/**
 * The amount of a token worth `usd_wad` at its USD price, rounded down (zero at a zero price), as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 */
export function amountForUsd(usd_wad: string, decimals: number, price_wad: string): string;

/**
 * Token amounts `range.liquidity` holds at `sqrtPriceX96`, rounded down (`LiquidityAmounts.getAmountsForLiquidity`).
 *
 * # Errors
 *
 * Throws for a zero lower price.
 */
export function amountsForLiquidity(range: LiquidityRange, sqrt_price_x96: string): TokenAmounts;

/**
 * What to approve for `amount`: 2% more and one wei, as hex.
 *
 * # Errors
 *
 * Throws for a malformed amount.
 */
export function approvalAmount(amount: string): string;

/**
 * The value-weighted average rate, as hex.
 *
 * # Errors
 *
 * Throws on absurd values.
 */
export function averageApr(request: AverageAprRequest): string;

/**
 * What the pool holds between consecutive raw sqrt prices from `sqrt_now`, from `segmentSqrts`: flattened as
 * `[amount0, amount1, …]`, one pair per edge.
 */
export function bookAmounts(sqrt_segments_flat: Float64Array, sqrt_now: number, edges: Float64Array, decimals0: number, decimals1: number): Float64Array;

/**
 * What an open borrows, in token units, before the opening fee.
 *
 * # Errors
 *
 * Throws for a zero price.
 */
export function borrowAmounts(request: BorrowAmountsRequest): PerToken;

/**
 * The opening fee on a borrow of `assets`, rounded up as the contract charges it, hex.
 *
 * # Errors
 *
 * Throws on overflow.
 */
export function borrowFee(assets: string, borrow_fee_wad: string): string;

/**
 * A liquidation penalty with the 2% safety buffer on top, as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 */
export function bufferedPenalty(penalty_wad: string): string;

/**
 * The numbers of a close: the redeem's minimums and a swap for each debt it cannot cover.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 */
export function closeAmounts(request: CloseRequest): CloseAmounts;

/**
 * What an ended position made against holding the tokens put in to its end (USD), at the exit price.
 *
 * # Errors
 *
 * Throws for a malformed input.
 */
export function closedVsHold(request: ClosedVsHoldInput): number | undefined;

/**
 * The minimums of compounding the position's owed fees.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 */
export function compoundMinimums(request: CompoundRequest): TokenAmounts;

/**
 * The [`DebtInput`] of a debt (`HealthLogic._debtInput`).
 *
 * # Errors
 *
 * Throws on overflow.
 */
export function debtInput(request: DebtRequest): DebtInput;

/**
 * What a borrow of `assets` owes, its opening fee rounded up as the contract charges it, hex.
 *
 * # Errors
 *
 * Throws on overflow.
 */
export function debtWithFee(assets: string, borrow_fee_wad: string): string;

/**
 * The most of each token minting the liquidity takes anywhere in the band, plus one wei: the router's maximums.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 */
export function depositMaximums(band: LiquidityBand): TokenAmounts;

/**
 * The least of each token the deposit takes anywhere in the band: the router's minimums for an add.
 *
 * # Errors
 *
 * Throws for a slippage above 100% or an empty range.
 */
export function depositMinimums(band: DepositBand): TokenAmounts;

/**
 * The tokens a swap moving the price trades, from flattened segments: flattened as `[tick, amount0, amount1, …]`.
 *
 * # Errors
 *
 * Throws for a malformed walk.
 */
export function depthWalk(segments_flat: Float64Array, walk: DepthWalkRequest): Float64Array;

/**
 * A position's net yearly rate after an edit; undefined without margin.
 *
 * # Errors
 *
 * Throws for a malformed input.
 */
export function editApr(request: EditAprInput): number | undefined;

/**
 * The highest leverage (1e18) the position's range takes keeping the safety buffer, as hex.
 *
 * # Errors
 *
 * Throws where the lens reverts.
 */
export function editMaxLeverage(request: EditMaxLeverageRequest): string;

/**
 * Both tokens' USD prices at the open; undefined when the deposits are worth nothing.
 *
 * # Errors
 *
 * Throws for a malformed input.
 */
export function entryUsdPrices(request: EntryPricesRequest): UsdPrices | undefined;

/**
 * What closing sends back, per token, before slippage and the plan's headroom (`slippageBps` is ignored).
 *
 * # Errors
 *
 * Throws for a zero price.
 */
export function estimateReceive(request: CloseRequest): TokenAmounts;

/**
 * Four-corner health check of a position (`HealthMath.evaluate`).
 *
 * # Errors
 *
 * Throws where the contract reverts.
 */
export function evaluateHealth(request: HealthRequest): HealthResult;

/**
 * The liquidity an exact open mints and the most it may borrow.
 *
 * # Errors
 *
 * Throws for an empty range.
 */
export function exactOpen(request: ExactOpenRequest): ExactOpen;

/**
 * The pool price as a fractional tick. For display.
 *
 * # Errors
 *
 * Throws for a malformed sqrt price.
 */
export function exactTickAtSqrtPrice(sqrt_price_x96: string): number;

/**
 * What `liquidity` in range earns of `fee` paid to `active_liquidity`, as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 */
export function feeShare(fee: string, liquidity: string, active_liquidity: string): string;

/**
 * LP fees owed to a holder (`FeeLogic.pending`).
 *
 * # Errors
 *
 * Throws on overflow.
 */
export function feesPending(request: FeesRequest): TokenAmounts;

/**
 * A pool's swap fees over a year, from those seen over `window_seconds`, times what LPs keep.
 */
export function feesPerYear(fees_24h_usd: number, window_seconds: number, keep: number): number;

/**
 * What of the deposit a mint takes without a swap.
 *
 * # Errors
 *
 * Throws for an empty range.
 */
export function fitToRange(request: FitRequest): PerToken;

/**
 * The same position seen from token1.
 *
 * # Errors
 *
 * Throws for malformed inputs.
 */
export function flipPayoff(inputs: PayoffInputs): PayoffInputs;

/**
 * The widest range a pool with this tick spacing allows.
 *
 * # Errors
 *
 * Throws for a spacing below 1.
 */
export function fullRange(spacing: number): TickRange;

/**
 * What a Senzu holder keeps of swap fees or rewards: the DEX's LP share (`keep`) less the protocol's cut.
 *
 * # Errors
 *
 * Throws for a malformed number.
 */
export function holderKeep(keep: number, cut_wad: string): number;

/**
 * What to pay for `wanted` at a quote's rate, rounded up, as hex.
 *
 * # Errors
 *
 * Throws on absurd amounts.
 */
export function inputForOutput(wanted: string, amount_in: string, amount_out: string): string;

/**
 * The interest each debt has cost.
 *
 * # Errors
 *
 * Throws on absurd values.
 */
export function interestPaid(request: InterestPaidRequest): InterestPaid;

/**
 * The index of the debt worth the most (a tie goes to the later one), or -1 when nothing is owed.
 *
 * # Errors
 *
 * Throws for a malformed request.
 */
export function largestDebt(request: LargestDebtRequest): number;

/**
 * The swap tilted so what does not fit is left in the borrowed token.
 *
 * # Errors
 *
 * Throws for an empty swap.
 */
export function leanTowardDebt(request: LeanRequest): RangeSwap;

/**
 * A lender's interest to date in asset units, as a decimal int256.
 *
 * # Errors
 *
 * Throws on absurd values.
 */
export function lenderEarned(request: LenderEarnedRequest): string;

/**
 * The leverage offered for a maximum (1e18), in tenths (25 = 2.5x).
 *
 * # Errors
 *
 * Throws for a malformed number.
 */
export function leverageCap(max_leverage_wad: string): number;

/**
 * What it takes to reach the target leverage.
 *
 * # Errors
 *
 * Throws where the health check reverts.
 */
export function leverageChange(request: LeverageChangeRequest): LeverageChange;

/**
 * The swap of a one-token borrow into the range, tilted toward the debt.
 *
 * # Errors
 *
 * Throws for an invalid range.
 */
export function leverageUpSwap(request: LeverageUpSwapRequest): MaybeSwap;

/**
 * Size over margin (1e18 = 1x), as hex; zero without margin. `equity_usd_wad` is a decimal or hex int256.
 *
 * # Errors
 *
 * Throws for a malformed number.
 */
export function leverageWad(size_usd_wad: string, equity_usd_wad: string): string;

/**
 * Repay, seize and fee amounts of a liquidation (`LiquidationLogic.liquidate`).
 *
 * # Errors
 *
 * Throws where the contract reverts (e.g. `position is not liquidatable`).
 */
export function liquidate(request: LiquidateRequest): Liquidation;

/**
 * The numbers of a liquidation: the liquidity seized, the redeem's minimums and the sale into the debt asset.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 */
export function liquidationAmounts(request: LiquidationRequest): LiquidationAmounts;

/**
 * The liquidation sale's least output, as hex.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 */
export function liquidationMinOut(request: LiquidationMinOutRequest): string;

/**
 * The liquidation prices as pool prices (token1 per token0, unsorted) and the pool price now. For display.
 *
 * # Errors
 *
 * Throws for a malformed request.
 */
export function liquidationPoolPrices(request: LiquidationPoolPricesRequest): LiquidationPoolPrices;

/**
 * Liquidation prices of a position (`SenzuLens._liquidationPrices`).
 *
 * # Errors
 *
 * Throws where the lens reverts.
 */
export function liquidationPrices(request: LiquidationPricesRequest): LiquidationPrices;

/**
 * Full USD breakdown of a liquidation: seized, repaid, protocol fee, liquidator's net incentive, and write-off.
 *
 * # Errors
 *
 * Throws on absurd values.
 */
export function liquidationUsd(request: SeizedShareRequest): LiquidationUsd;

/**
 * The pool's liquidity in buckets, from segments flattened as `[lower, upper, liquidity, …]`: flattened as
 * `[tickLower, tickUpper, liquidity, amount0, amount1, active (0 or 1), …]`.
 */
export function liquidityBars(segments_flat: Float64Array, tick: number, bucket: number, from_tick: number, to_tick: number, decimals0: number, decimals1: number): Float64Array;

/**
 * Most liquidity `amount0` and `amount1` can add in `range` at `sqrtPriceX96`, rounded down, as hex
 * (`LiquidityAmounts.getLiquidityForAmounts`).
 *
 * # Errors
 *
 * Throws for an empty range, or when the liquidity does not fit in 128 bits.
 */
export function liquidityForAmounts(range: SqrtRange, sqrt_price_x96: string, amount0: string, amount1: string): string;

/**
 * The liquidity providers' fee on a swap's amount in, less the protocol's, as hex.
 *
 * # Errors
 *
 * Throws on an absurd amount.
 */
export function lpFee(request: LpFeeRequest): string;

/**
 * A plain liquidity position's value, profit, comparison with holding, and fee APR.
 *
 * # Errors
 *
 * Throws for a malformed input.
 */
export function lpMetrics(request: LpInput): LpMetrics;

/**
 * The margin's value in USD (1e18), as hex.
 *
 * # Errors
 *
 * Throws on absurd amounts.
 */
export function marginUsd(request: MarginRequest): string;

/**
 * The position valued at its pool's price.
 *
 * # Errors
 *
 * Throws for an invalid range.
 */
export function markPosition(request: MarkRequest): Mark;

/**
 * What closing the position now sends back, at the pool's price.
 *
 * # Errors
 *
 * Throws for a zero price.
 */
export function markReceive(request: MarkReceiveRequest): MarkReceive;

/**
 * A market's borrow and supply APRs at a utilization.
 *
 * # Errors
 *
 * Throws for a reserve factor above 100%.
 */
export function marketAprs(request: MarketAprsRequest): MarketAprs;

/**
 * A market's APRs after its cash moves.
 *
 * # Errors
 *
 * Throws for a reserve factor above 100%.
 */
export function marketAprsAfter(request: MarketAprsAfterRequest): MarketAprs;

/**
 * A market's interest over a year, split between reserves and lenders.
 *
 * # Errors
 *
 * Throws on absurd values.
 */
export function marketFlows(request: MarketFlowsRequest): MarketFlows;

/**
 * Most of `asset` the position can borrow now (`SenzuLens.maxBorrow`), hex.
 *
 * # Errors
 *
 * Throws where the lens reverts.
 */
export function maxBorrow(request: MaxBorrowRequest): string;

/**
 * Highest leverage (WAD, hex) a new position in the range passes the borrow check with.
 *
 * # Errors
 *
 * Throws where the lens reverts.
 */
export function maxLeverage(request: MaxLeverageRequest): string;

/**
 * The largest share of the liquidity (1e18 = all) a redeem can take keeping the safety buffer, as hex.
 *
 * # Errors
 *
 * Throws where the health check reverts.
 */
export function maxRemovableWad(request: PositionStateRequest): string;

/**
 * The least a swap quoted at `amount` may pay within `slippageBps`, as hex.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 */
export function minOut(amount: string, slippage_bps: string): string;

/**
 * The mint's minimums after the swap pays only its least output.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 */
export function minimumsAfterSwap(request: MinimumsAfterSwapRequest): TokenAmounts;

/**
 * The multiple of the pool's tick spacing nearest `tick` (halfway rounds up), kept inside the ticks a pool allows.
 *
 * # Errors
 *
 * Throws for a spacing below 1 or a NaN tick.
 */
export function nearestUsableTick(tick: number, spacing: number): number;

/**
 * Whether `allowance` is too low to spend `amount` (under the amount plus 1%).
 *
 * # Errors
 *
 * Throws for a malformed number.
 */
export function needsApproval(amount: string, allowance: string): boolean;

/**
 * What an open borrows and its swap into the range, before the swap is quoted.
 *
 * # Errors
 *
 * Throws for a zero price or an invalid range.
 */
export function openAmounts(sizing: OpenSizing): OpenAmounts;

/**
 * A new position's projected earnings; undefined without margin.
 *
 * # Errors
 *
 * Throws on an absurd debt.
 */
export function openEarnings(request: OpenEarningsInput): OpenEarnings | undefined;

/**
 * The liquidation prices of [`previewOpen`](preview_open) alone.
 *
 * # Errors
 *
 * Throws where the lens reverts.
 */
export function openLiquidationPrices(params: OpenParams): LiquidationPrices;

/**
 * The highest leverage (1e18) a new position opens with, keeping the safety buffer, as hex.
 *
 * # Errors
 *
 * Throws where the lens reverts.
 */
export function openMaxLeverage(request: OpenMaxLeverageRequest): string;

/**
 * The mint's minimums of an open.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 */
export function openMinimums(request: OpenMinimumsRequest): TokenAmounts;

/**
 * A new position's net yearly rate on its margin; undefined without margin.
 *
 * # Errors
 *
 * Throws for a malformed input.
 */
export function openNetApr(request: OpenNetAprRequest): number | undefined;

/**
 * Token0's share of the range's value, out of 1e10 (basis points with six decimals), as hex; undefined for a range
 * that holds nothing.
 *
 * # Errors
 *
 * Throws for an invalid range.
 */
export function pairToken0Share(request: PairShareRequest): string | undefined;

/**
 * Without a swap, the amount of the other token that fits the deposit in the range at the pool price, as hex.
 *
 * # Errors
 *
 * Throws for a tick out of range.
 */
export function pairedAmount(request: PairedAmountRequest): string;

/**
 * The position's and holding's profit or loss at `price`.
 *
 * # Errors
 *
 * Throws for malformed inputs.
 */
export function payoffAt(inputs: PayoffInputs, price: number): PayoffPoint;

/**
 * The position's profit or loss at each of `prices` (fees not included).
 *
 * # Errors
 *
 * Throws for malformed inputs.
 */
export function payoffCurve(inputs: PayoffInputs, prices: Float64Array): Float64Array;

/**
 * Net token0 exposure now, in whole tokens (positive is long).
 *
 * # Errors
 *
 * Throws for malformed inputs.
 */
export function payoffExposure0(inputs: PayoffInputs): number;

/**
 * The range's ends as pool prices (token1 per token0, whole tokens).
 *
 * # Errors
 *
 * Throws for malformed inputs.
 */
export function payoffRange(inputs: PayoffInputs): PriceRange;

/**
 * A position's profit or loss against what was put in.
 *
 * # Errors
 *
 * Throws on absurd values.
 */
export function pnl(request: PnlInput): Pnl;

/**
 * Both tokens' USD prices at a pool price: the known one as given, the other as the pool implies.
 *
 * # Errors
 *
 * Throws for a price over 256 bits.
 */
export function poolPrices(request: PoolPriceRequest): PoolPrices;

/**
 * The pair's prices for sizing a close, a token without a live price valued at the pool.
 *
 * # Errors
 *
 * Throws for a price that does not fit in 256 bits.
 */
export function poolValuedPrices(request: PoolValuedRequest): PerToken;

/**
 * A position's yearly rates; undefined without size or margin.
 *
 * # Errors
 *
 * Throws for a malformed input.
 */
export function positionApr(request: PositionAprInput): PositionApr | undefined;

/**
 * What providing liquidity made against holding the tokens put in (USD), at today's prices; undefined before anything
 * was put in.
 *
 * # Errors
 *
 * Throws for a malformed input.
 */
export function positionVsHold(request: VsHoldInput): number | undefined;

/**
 * The Create Position preview (`SenzuLens.previewOpen`).
 *
 * # Errors
 *
 * Throws where the lens reverts.
 */
export function previewOpen(params: OpenParams): OpenPreview;

/**
 * [`previewOpen`](preview_open) without the liquidation prices: the cheap part, for live updates.
 *
 * # Errors
 *
 * Throws where the lens reverts.
 */
export function previewOpenSummary(params: OpenParams): OpenSummary;

/**
 * The position's numbers in the state, from the contracts' own health math.
 *
 * # Errors
 *
 * Throws where the health check reverts.
 */
export function previewState(request: PreviewStateRequest): EditPreview;

/**
 * The tick whose price is nearest `price` (token1 per token0, whole tokens); align it before use.
 */
export function priceToTick(price: number, decimals0: number, decimals1: number): number;

/**
 * The quote a minimum was written from within `slippageBps`, as hex.
 *
 * # Errors
 *
 * Throws for a slippage of 100% or more.
 */
export function quoteForMin(min_out: string, slippage_bps: string): string;

/**
 * A new range's fee APR over its size; undefined without size or liquidity.
 *
 * # Errors
 *
 * Throws for a malformed number.
 */
export function rangeFeeApr(fees_per_year_usd: number, liquidity: string, pool_liquidity: string, size_usd: number): number | undefined;

/**
 * The swap that brings a deposit to the range's ratio at the pool price.
 *
 * # Errors
 *
 * Throws for an invalid range.
 */
export function rangeSwap(request: RangeSwapRequest): MaybeSwap;

/**
 * A sqrt price as a raw number (token1 base units per token0 base unit, square-rooted). For display.
 *
 * # Errors
 *
 * Throws for a malformed sqrt price.
 */
export function rawSqrtPrice(sqrt_price_x96: string): number;

/**
 * The units a redeem takes and its minimums.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 */
export function redeemShare(request: RedeemShareRequest): RedeemShare;

/**
 * What a redeem of the share pays at the pool price (`slippageBps` is ignored).
 *
 * # Errors
 *
 * Throws for an invalid range.
 */
export function redeemedAmounts(request: RedeemShareRequest): PerToken;

/**
 * The state after redeeming a share of the liquidity.
 *
 * # Errors
 *
 * Throws for a share over 1e18.
 */
export function removeMarginState(request: RemoveMarginStateRequest): EditState;

/**
 * What to pull from the wallet to repay `repay` of `debt` with `balance`, as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 */
export function repayPull(repay: string, debt: string, balance: string): string;

/**
 * The state after a repayment.
 *
 * # Errors
 *
 * Throws for a malformed request.
 */
export function repayState(request: RepayStateRequest): EditState;

/**
 * The vault's share of the interest borrowers pay (1e18 = all), as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 */
export function reserveShareWad(inflow: string, interest: string, reserve_factor_wad: string): string;

/**
 * Flattened segments with their ticks as raw sqrt prices, for `bookAmounts`: work out once per curve.
 */
export function segmentSqrts(segments_flat: Float64Array): Float64Array;

/**
 * The fees that go with the seized units to the liquidator, rounded down.
 *
 * # Errors
 *
 * Throws on absurd values.
 */
export function seizedFees(request: SeizedFeesRequest): PerToken;

/**
 * The share of the liquidity (1e18 = all) a repayment seizes, taken 0.5% low, as hex.
 *
 * # Errors
 *
 * Throws on absurd values.
 */
export function seizedShareWad(request: SeizedShareRequest): string;

/**
 * `part` of `total` (1e18 = all), as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 */
export function shareWad(part: string, total: string): string;

/**
 * Sqrt price (Q64.96, hex) at a tick.
 *
 * # Errors
 *
 * Throws outside `[MIN_TICK, MAX_TICK]`.
 */
export function sqrtPriceAtTick(tick: number): string;

/**
 * Token1 per token0, in whole tokens, at an exact sqrt price. For display.
 *
 * # Errors
 *
 * Throws for a malformed sqrt price.
 */
export function sqrtPriceToPrice(sqrt_price_x96: string, decimals0: number, decimals1: number): number;

/**
 * The sqrt price at a pool price (token1 per token0, whole tokens), rounded down, as hex. For display and
 * approximate inputs.
 *
 * # Errors
 *
 * Throws for a negative or non-finite price.
 */
export function sqrtPriceX96AtPrice(price: number, decimals0: number, decimals1: number): string;

/**
 * Uniswap sqrt price (Q64.96, hex) of token0 in token1 from two USD prices. A zero `price1Wad` gives `MAX_SQRT_PRICE`.
 *
 * # Errors
 *
 * Throws on overflow.
 */
export function sqrtPriceX96FromUsd(price0_wad: string, price1_wad: string, decimals0: number, decimals1: number): string;

/**
 * What `stake` of `total_staked` earns of `inflow`, as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 */
export function stakeIncome(inflow: string, stake: string, total_staked: string): string;

/**
 * A stake's share of all staked (1e18 = all), as hex; undefined without a stake or stakers.
 *
 * # Errors
 *
 * Throws for a malformed number.
 */
export function stakeShareWad(staked: string, total_staked: string): string | undefined;

/**
 * What a reward stream still pays, as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 */
export function streamRemaining(rate_wad: string, period_finish: bigint, now: bigint): string;

/**
 * The swap that fits two amounts to the range's ratio (`SenzuLens.swapForRange`).
 *
 * # Errors
 *
 * Throws where the lens reverts.
 */
export function swapForRange(request: SwapRequest): SwapQuote;

/**
 * The swaps that turn what a deleverage's redeem paid into the debts it repays.
 *
 * # Errors
 *
 * Throws for a zero price.
 */
export function swapIntoDebts(request: SwapIntoDebtsRequest): DebtSwaps;

/**
 * Greatest tick whose sqrt price is at most `sqrtPriceX96`.
 *
 * # Errors
 *
 * Throws outside `[MIN_SQRT_PRICE, MAX_SQRT_PRICE)`.
 */
export function tickAtSqrtPrice(sqrt_price_x96: string): number;

/**
 * Token1 per token0, in whole tokens, at a tick. For display.
 */
export function tickToPrice(tick: number, decimals0: number, decimals1: number): number;

/**
 * `shares → assets` with the markets' virtual shares, hex.
 *
 * # Errors
 *
 * Throws on overflow.
 */
export function toAssets(shares: string, total_assets: string, total_shares: string, round_up: boolean): string;

/**
 * `assets → shares` with the markets' virtual shares, hex.
 *
 * # Errors
 *
 * Throws on overflow.
 */
export function toShares(assets: string, total_assets: string, total_shares: string, round_up: boolean): string;

/**
 * One standard deviation of a day's log return, from hourly closes; undefined with under a day of them.
 */
export function typicalDailyMove(hourly_closes: Float64Array): number | undefined;

/**
 * USD value (1e18) of `amount` of a token at its USD price, rounded down, as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 */
export function usdValue(amount: string, decimals: number, price_wad: string): string;

/**
 * Borrows over cash plus borrows (1e18 = 100%), as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 */
export function utilization(cash: string, borrows: string): string;

/**
 * The amounts in USD (1e18) at the pool's price, as hex.
 *
 * # Errors
 *
 * Throws on absurd amounts.
 */
export function valueAtPool(request: ValueAtPoolRequest): string;

/**
 * What withdrawing a whole deposit can take now.
 *
 * # Errors
 *
 * Throws on absurd values.
 */
export function withdrawAll(deposit_assets: string, deposit_shares: string, max_withdraw: string): WithdrawAll;

/**
 * What the liquidity holds at the band's low and high ends.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 */
export function withdrawEnds(band: LiquidityBand): BandEnds;

/**
 * The least of each token redeeming the liquidity pays anywhere in the band: the router's minimums for a redeem.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 */
export function withdrawMinimums(band: LiquidityBand): TokenAmounts;

/**
 * The shares a withdrawal burns and what they pay.
 *
 * # Errors
 *
 * Throws on absurd values.
 */
export function withdrawal(request: WithdrawalRequest): Withdrawal;

/**
 * The debt (USD, 1e18) a liquidation writes off, as hex: zero unless it seizes every unit.
 *
 * # Errors
 *
 * Throws on absurd values.
 */
export function writtenOffUsdWad(request: SeizedShareRequest): string;

/**
 * What `amount` earns in a year at `apr_wad`, as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 */
export function yearly(amount: string, apr_wad: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly acceptedTokens: (a: any, b: number) => [number, number, number];
    readonly accrue: (a: any) => [number, number, number];
    readonly accrueDebt: (a: any) => [number, number, number];
    readonly adaptiveRate: (a: any) => [number, number, number];
    readonly addMarginState: (a: any) => [number, number, number];
    readonly afterCut: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly alignTick: (a: number, b: number, c: any) => [number, number, number];
    readonly amountForUsd: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly amountsForLiquidity: (a: any, b: number, c: number) => [number, number, number];
    readonly approvalAmount: (a: number, b: number) => [number, number, number, number];
    readonly averageApr: (a: any) => [number, number, number, number];
    readonly bookAmounts: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly borrowAmounts: (a: any) => [number, number, number];
    readonly borrowFee: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly bufferedPenalty: (a: number, b: number) => [number, number, number, number];
    readonly closeAmounts: (a: any) => [number, number, number];
    readonly closedVsHold: (a: any) => [number, number, number, number];
    readonly compoundMinimums: (a: any) => [number, number, number];
    readonly debtInput: (a: any) => [number, number, number];
    readonly debtWithFee: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly depositMaximums: (a: any) => [number, number, number];
    readonly depositMinimums: (a: any) => [number, number, number];
    readonly depthWalk: (a: number, b: number, c: any) => [number, number, number, number];
    readonly editApr: (a: any) => [number, number, number, number];
    readonly editMaxLeverage: (a: any) => [number, number, number, number];
    readonly entryUsdPrices: (a: any) => [number, number, number];
    readonly estimateReceive: (a: any) => [number, number, number];
    readonly evaluateHealth: (a: any) => [number, number, number];
    readonly exactOpen: (a: any) => [number, number, number];
    readonly exactTickAtSqrtPrice: (a: number, b: number) => [number, number, number];
    readonly feeShare: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly feesPending: (a: any) => [number, number, number];
    readonly feesPerYear: (a: number, b: number, c: number) => number;
    readonly fitToRange: (a: any) => [number, number, number];
    readonly flipPayoff: (a: any) => [number, number, number];
    readonly fullRange: (a: number) => [number, number, number];
    readonly holderKeep: (a: number, b: number, c: number) => [number, number, number];
    readonly inputForOutput: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly interestPaid: (a: any) => [number, number, number];
    readonly largestDebt: (a: any) => [number, number, number];
    readonly leanTowardDebt: (a: any) => [number, number, number];
    readonly lenderEarned: (a: any) => [number, number, number, number];
    readonly leverageCap: (a: number, b: number) => [number, number, number];
    readonly leverageChange: (a: any) => [number, number, number];
    readonly leverageUpSwap: (a: any) => [number, number, number];
    readonly leverageWad: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly liquidate: (a: any) => [number, number, number];
    readonly liquidationAmounts: (a: any) => [number, number, number];
    readonly liquidationMinOut: (a: any) => [number, number, number, number];
    readonly liquidationPoolPrices: (a: any) => [number, number, number];
    readonly liquidationPrices: (a: any) => [number, number, number];
    readonly liquidationUsd: (a: any) => [number, number, number];
    readonly liquidityBars: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number];
    readonly liquidityForAmounts: (a: any, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly lpFee: (a: any) => [number, number, number, number];
    readonly lpMetrics: (a: any) => [number, number, number];
    readonly marginUsd: (a: any) => [number, number, number, number];
    readonly markPosition: (a: any) => [number, number, number];
    readonly markReceive: (a: any) => [number, number, number];
    readonly marketAprs: (a: any) => [number, number, number];
    readonly marketAprsAfter: (a: any) => [number, number, number];
    readonly marketFlows: (a: any) => [number, number, number];
    readonly maxBorrow: (a: any) => [number, number, number, number];
    readonly maxLeverage: (a: any) => [number, number, number, number];
    readonly maxRemovableWad: (a: any) => [number, number, number, number];
    readonly minOut: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly minimumsAfterSwap: (a: any) => [number, number, number];
    readonly nearestUsableTick: (a: number, b: number) => [number, number, number];
    readonly needsApproval: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly openAmounts: (a: any) => [number, number, number];
    readonly openEarnings: (a: any) => [number, number, number];
    readonly openLiquidationPrices: (a: any) => [number, number, number];
    readonly openMaxLeverage: (a: any) => [number, number, number, number];
    readonly openMinimums: (a: any) => [number, number, number];
    readonly openNetApr: (a: any) => [number, number, number, number];
    readonly pairToken0Share: (a: any) => [number, number, number, number];
    readonly pairedAmount: (a: any) => [number, number, number, number];
    readonly payoffAt: (a: any, b: number) => [number, number, number];
    readonly payoffCurve: (a: any, b: number, c: number) => [number, number, number, number];
    readonly payoffExposure0: (a: any) => [number, number, number];
    readonly payoffRange: (a: any) => [number, number, number];
    readonly pnl: (a: any) => [number, number, number];
    readonly poolPrices: (a: any) => [number, number, number];
    readonly poolValuedPrices: (a: any) => [number, number, number];
    readonly positionApr: (a: any) => [number, number, number];
    readonly positionVsHold: (a: any) => [number, number, number, number];
    readonly previewOpen: (a: any) => [number, number, number];
    readonly previewOpenSummary: (a: any) => [number, number, number];
    readonly previewState: (a: any) => [number, number, number];
    readonly priceToTick: (a: number, b: number, c: number) => number;
    readonly quoteForMin: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly rangeFeeApr: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly rangeSwap: (a: any) => [number, number, number];
    readonly rawSqrtPrice: (a: number, b: number) => [number, number, number];
    readonly redeemShare: (a: any) => [number, number, number];
    readonly redeemedAmounts: (a: any) => [number, number, number];
    readonly removeMarginState: (a: any) => [number, number, number];
    readonly repayPull: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly repayState: (a: any) => [number, number, number];
    readonly reserveShareWad: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly segmentSqrts: (a: number, b: number) => [number, number];
    readonly seizedFees: (a: any) => [number, number, number];
    readonly seizedShareWad: (a: any) => [number, number, number, number];
    readonly shareWad: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly sqrtPriceAtTick: (a: number) => [number, number, number, number];
    readonly sqrtPriceToPrice: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly sqrtPriceX96AtPrice: (a: number, b: number, c: number) => [number, number, number, number];
    readonly sqrtPriceX96FromUsd: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly stakeIncome: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number, number];
    readonly stakeShareWad: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly streamRemaining: (a: number, b: number, c: bigint, d: bigint) => [number, number, number, number];
    readonly swapForRange: (a: any) => [number, number, number];
    readonly swapIntoDebts: (a: any) => [number, number, number];
    readonly tickAtSqrtPrice: (a: number, b: number) => [number, number, number];
    readonly tickToPrice: (a: number, b: number, c: number) => number;
    readonly toAssets: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly toShares: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly typicalDailyMove: (a: number, b: number) => [number, number];
    readonly usdValue: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly utilization: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly valueAtPool: (a: any) => [number, number, number, number];
    readonly withdrawAll: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly withdrawEnds: (a: any) => [number, number, number];
    readonly withdrawMinimums: (a: any) => [number, number, number];
    readonly withdrawal: (a: any) => [number, number, number];
    readonly writtenOffUsdWad: (a: any) => [number, number, number, number];
    readonly yearly: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
