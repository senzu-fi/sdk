/* @ts-self-types="./senzu_sdk_core.d.ts" */

/**
 * Which tokens a range takes at the pool's tick: only token0 above the price, only token1 below it, both in range.
 *
 * # Errors
 *
 * Never in practice; the result crosses into JS.
 * @param {TickRange} range
 * @param {number} pool_tick
 * @returns {PerToken}
 */
export function acceptedTokens(range, pool_tick) {
    const ret = wasm.acceptedTokens(range, pool_tick);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Accrues a market to `now` (`MarketLogic.accrue`), as the borrowable's views do: the exact integral of the
 * rate, compounded, the rate at target adapted; the rates returned are the instantaneous ones as of `now`.
 *
 * # Errors
 *
 * Throws where the contract reverts.
 * @param {AccrueRequest} request
 * @returns {AccrueResult}
 */
export function accrue(request) {
    const ret = wasm.accrue(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * A debt grown by its interest since the read, as the contract accrues it.
 *
 * # Errors
 *
 * Throws where the rate math overflows (never for a real market).
 * @param {AccrueDebtRequest} request
 * @returns {AccruedDebt}
 */
export function accrueDebt(request) {
    const ret = wasm.accrueDebt(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The borrow rate integrated over a period and the rate at target it ends at (`MarketLogic.adaptiveRate`), e.g.
 * to chart where a market's rate heads if its utilization holds. The debt grows `e^integral` times.
 *
 * # Errors
 *
 * Throws on overflow.
 * @param {AdaptiveRateRequest} request
 * @returns {AdaptiveRate}
 */
export function adaptiveRate(request) {
    const ret = wasm.adaptiveRate(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The state after adding margin as liquidity.
 *
 * # Errors
 *
 * Throws for liquidity over 128 bits.
 * @param {AddMarginStateRequest} request
 * @returns {EditState}
 */
export function addMarginState(request) {
    const ret = wasm.addMarginState(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * What a holder is paid of `amount` of fees or rewards owed, the protocol's cut taken (rounded down), hex.
 *
 * # Errors
 *
 * Throws on overflow or a cut over 100%.
 * @param {string} amount
 * @param {string} cut_wad
 * @returns {string}
 */
export function afterCut(amount, cut_wad) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passStringToWasm0(amount, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(cut_wad, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.afterCut(ptr0, len0, ptr1, len1);
        var ptr3 = ret[0];
        var len3 = ret[1];
        if (ret[3]) {
            ptr3 = 0; len3 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * `tick` moved down or up to a multiple of the pool's tick spacing, kept inside the ticks a pool allows.
 *
 * # Errors
 *
 * Throws for a spacing below 1 or a NaN tick.
 * @param {number} tick
 * @param {number} spacing
 * @param {TickDirection} direction
 * @returns {number}
 */
export function alignTick(tick, spacing, direction) {
    const ret = wasm.alignTick(tick, spacing, direction);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ret[0];
}

/**
 * The amount of a token worth `usd_wad` at its USD price, rounded down (zero at a zero price), as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 * @param {string} usd_wad
 * @param {number} decimals
 * @param {string} price_wad
 * @returns {string}
 */
export function amountForUsd(usd_wad, decimals, price_wad) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passStringToWasm0(usd_wad, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(price_wad, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.amountForUsd(ptr0, len0, decimals, ptr1, len1);
        var ptr3 = ret[0];
        var len3 = ret[1];
        if (ret[3]) {
            ptr3 = 0; len3 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * Token amounts `range.liquidity` holds at `sqrtPriceX96`, rounded down (`LiquidityAmounts.getAmountsForLiquidity`).
 *
 * # Errors
 *
 * Throws for a zero lower price.
 * @param {LiquidityRange} range
 * @param {string} sqrt_price_x96
 * @returns {TokenAmounts}
 */
export function amountsForLiquidity(range, sqrt_price_x96) {
    const ptr0 = passStringToWasm0(sqrt_price_x96, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.amountsForLiquidity(range, ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * What to approve for `amount`: 2% more and one wei, as hex.
 *
 * # Errors
 *
 * Throws for a malformed amount.
 * @param {string} amount
 * @returns {string}
 */
export function approvalAmount(amount) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(amount, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.approvalAmount(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * The value-weighted average rate, as hex.
 *
 * # Errors
 *
 * Throws on absurd values.
 * @param {AverageAprRequest} request
 * @returns {string}
 */
export function averageApr(request) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.averageApr(request);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * What the pool holds between consecutive raw sqrt prices from `sqrt_now`, from `segmentSqrts`: flattened as
 * `[amount0, amount1, …]`, one pair per edge.
 * @param {Float64Array} sqrt_segments_flat
 * @param {number} sqrt_now
 * @param {Float64Array} edges
 * @param {number} decimals0
 * @param {number} decimals1
 * @returns {Float64Array}
 */
export function bookAmounts(sqrt_segments_flat, sqrt_now, edges, decimals0, decimals1) {
    const ptr0 = passArrayF64ToWasm0(sqrt_segments_flat, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passArrayF64ToWasm0(edges, wasm.__wbindgen_malloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.bookAmounts(ptr0, len0, sqrt_now, ptr1, len1, decimals0, decimals1);
    var v3 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
    return v3;
}

/**
 * What an open borrows, in token units, before the opening fee.
 *
 * # Errors
 *
 * Throws for a zero price.
 * @param {BorrowAmountsRequest} request
 * @returns {PerToken}
 */
export function borrowAmounts(request) {
    const ret = wasm.borrowAmounts(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The opening fee on a borrow of `assets`, rounded up as the contract charges it, hex.
 *
 * # Errors
 *
 * Throws on overflow.
 * @param {string} assets
 * @param {string} borrow_fee_wad
 * @returns {string}
 */
export function borrowFee(assets, borrow_fee_wad) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passStringToWasm0(assets, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(borrow_fee_wad, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.borrowFee(ptr0, len0, ptr1, len1);
        var ptr3 = ret[0];
        var len3 = ret[1];
        if (ret[3]) {
            ptr3 = 0; len3 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * A liquidation penalty with the 2% safety buffer on top, as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 * @param {string} penalty_wad
 * @returns {string}
 */
export function bufferedPenalty(penalty_wad) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(penalty_wad, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.bufferedPenalty(ptr0, len0);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * The numbers of a close: the redeem's minimums and a swap for each debt it cannot cover.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 * @param {CloseRequest} request
 * @returns {CloseAmounts}
 */
export function closeAmounts(request) {
    const ret = wasm.closeAmounts(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * What an ended position made against holding the tokens put in to its end (USD), at the exit price.
 *
 * # Errors
 *
 * Throws for a malformed input.
 * @param {ClosedVsHoldInput} request
 * @returns {number | undefined}
 */
export function closedVsHold(request) {
    const ret = wasm.closedVsHold(request);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    return ret[0] === 0 ? undefined : ret[1];
}

/**
 * The minimums of compounding the position's owed fees.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 * @param {CompoundRequest} request
 * @returns {TokenAmounts}
 */
export function compoundMinimums(request) {
    const ret = wasm.compoundMinimums(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The [`DebtInput`] of a debt (`HealthLogic._debtInput`).
 *
 * # Errors
 *
 * Throws on overflow.
 * @param {DebtRequest} request
 * @returns {DebtInput}
 */
export function debtInput(request) {
    const ret = wasm.debtInput(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * What a borrow of `assets` owes, its opening fee rounded up as the contract charges it, hex.
 *
 * # Errors
 *
 * Throws on overflow.
 * @param {string} assets
 * @param {string} borrow_fee_wad
 * @returns {string}
 */
export function debtWithFee(assets, borrow_fee_wad) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passStringToWasm0(assets, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(borrow_fee_wad, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.debtWithFee(ptr0, len0, ptr1, len1);
        var ptr3 = ret[0];
        var len3 = ret[1];
        if (ret[3]) {
            ptr3 = 0; len3 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * The most of each token minting the liquidity takes anywhere in the band, plus one wei: the router's maximums.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 * @param {LiquidityBand} band
 * @returns {TokenAmounts}
 */
export function depositMaximums(band) {
    const ret = wasm.depositMaximums(band);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The least of each token the deposit takes anywhere in the band: the router's minimums for an add.
 *
 * # Errors
 *
 * Throws for a slippage above 100% or an empty range.
 * @param {DepositBand} band
 * @returns {TokenAmounts}
 */
export function depositMinimums(band) {
    const ret = wasm.depositMinimums(band);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The tokens a swap moving the price trades, from flattened segments: flattened as `[tick, amount0, amount1, …]`.
 *
 * # Errors
 *
 * Throws for a malformed walk.
 * @param {Float64Array} segments_flat
 * @param {DepthWalkRequest} walk
 * @returns {Float64Array}
 */
export function depthWalk(segments_flat, walk) {
    const ptr0 = passArrayF64ToWasm0(segments_flat, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.depthWalk(ptr0, len0, walk);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v2 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
    return v2;
}

/**
 * A position's net yearly rate after an edit; undefined without margin.
 *
 * # Errors
 *
 * Throws for a malformed input.
 * @param {EditAprInput} request
 * @returns {number | undefined}
 */
export function editApr(request) {
    const ret = wasm.editApr(request);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    return ret[0] === 0 ? undefined : ret[1];
}

/**
 * The highest leverage (1e18) the position's range takes keeping the safety buffer, as hex.
 *
 * # Errors
 *
 * Throws where the lens reverts.
 * @param {EditMaxLeverageRequest} request
 * @returns {string}
 */
export function editMaxLeverage(request) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.editMaxLeverage(request);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Both tokens' USD prices at the open; undefined when the deposits are worth nothing.
 *
 * # Errors
 *
 * Throws for a malformed input.
 * @param {EntryPricesRequest} request
 * @returns {UsdPrices | undefined}
 */
export function entryUsdPrices(request) {
    const ret = wasm.entryUsdPrices(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * What closing sends back, per token, before slippage and the plan's headroom (`slippageBps` is ignored).
 *
 * # Errors
 *
 * Throws for a zero price.
 * @param {CloseRequest} request
 * @returns {TokenAmounts}
 */
export function estimateReceive(request) {
    const ret = wasm.estimateReceive(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Four-corner health check of a position (`HealthMath.evaluate`).
 *
 * # Errors
 *
 * Throws where the contract reverts.
 * @param {HealthRequest} request
 * @returns {HealthResult}
 */
export function evaluateHealth(request) {
    const ret = wasm.evaluateHealth(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The liquidity an exact open mints and the most it may borrow.
 *
 * # Errors
 *
 * Throws for an empty range.
 * @param {ExactOpenRequest} request
 * @returns {ExactOpen}
 */
export function exactOpen(request) {
    const ret = wasm.exactOpen(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The pool price as a fractional tick. For display.
 *
 * # Errors
 *
 * Throws for a malformed sqrt price.
 * @param {string} sqrt_price_x96
 * @returns {number}
 */
export function exactTickAtSqrtPrice(sqrt_price_x96) {
    const ptr0 = passStringToWasm0(sqrt_price_x96, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.exactTickAtSqrtPrice(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ret[0];
}

/**
 * What `liquidity` in range earns of `fee` paid to `active_liquidity`, as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 * @param {string} fee
 * @param {string} liquidity
 * @param {string} active_liquidity
 * @returns {string}
 */
export function feeShare(fee, liquidity, active_liquidity) {
    let deferred5_0;
    let deferred5_1;
    try {
        const ptr0 = passStringToWasm0(fee, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(liquidity, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(active_liquidity, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.feeShare(ptr0, len0, ptr1, len1, ptr2, len2);
        var ptr4 = ret[0];
        var len4 = ret[1];
        if (ret[3]) {
            ptr4 = 0; len4 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred5_0 = ptr4;
        deferred5_1 = len4;
        return getStringFromWasm0(ptr4, len4);
    } finally {
        wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
    }
}

/**
 * LP fees owed to a holder (`FeeLogic.pending`).
 *
 * # Errors
 *
 * Throws on overflow.
 * @param {FeesRequest} request
 * @returns {TokenAmounts}
 */
export function feesPending(request) {
    const ret = wasm.feesPending(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * A pool's swap fees over a year, from those seen over `window_seconds`, times what LPs keep.
 * @param {number} fees_24h_usd
 * @param {number} window_seconds
 * @param {number} keep
 * @returns {number}
 */
export function feesPerYear(fees_24h_usd, window_seconds, keep) {
    const ret = wasm.feesPerYear(fees_24h_usd, window_seconds, keep);
    return ret;
}

/**
 * What of the deposit a mint takes without a swap.
 *
 * # Errors
 *
 * Throws for an empty range.
 * @param {FitRequest} request
 * @returns {PerToken}
 */
export function fitToRange(request) {
    const ret = wasm.fitToRange(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The same position seen from token1.
 *
 * # Errors
 *
 * Throws for malformed inputs.
 * @param {PayoffInputs} inputs
 * @returns {PayoffInputs}
 */
export function flipPayoff(inputs) {
    const ret = wasm.flipPayoff(inputs);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The widest range a pool with this tick spacing allows.
 *
 * # Errors
 *
 * Throws for a spacing below 1.
 * @param {number} spacing
 * @returns {TickRange}
 */
export function fullRange(spacing) {
    const ret = wasm.fullRange(spacing);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * What a Senzu holder keeps of swap fees or rewards: the DEX's LP share (`keep`) less the protocol's cut.
 *
 * # Errors
 *
 * Throws for a malformed number.
 * @param {number} keep
 * @param {string} cut_wad
 * @returns {number}
 */
export function holderKeep(keep, cut_wad) {
    const ptr0 = passStringToWasm0(cut_wad, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.holderKeep(keep, ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ret[0];
}

/**
 * What to pay for `wanted` at a quote's rate, rounded up, as hex.
 *
 * # Errors
 *
 * Throws on absurd amounts.
 * @param {string} wanted
 * @param {string} amount_in
 * @param {string} amount_out
 * @returns {string}
 */
export function inputForOutput(wanted, amount_in, amount_out) {
    let deferred5_0;
    let deferred5_1;
    try {
        const ptr0 = passStringToWasm0(wanted, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(amount_in, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(amount_out, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.inputForOutput(ptr0, len0, ptr1, len1, ptr2, len2);
        var ptr4 = ret[0];
        var len4 = ret[1];
        if (ret[3]) {
            ptr4 = 0; len4 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred5_0 = ptr4;
        deferred5_1 = len4;
        return getStringFromWasm0(ptr4, len4);
    } finally {
        wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
    }
}

/**
 * The interest each debt has cost.
 *
 * # Errors
 *
 * Throws on absurd values.
 * @param {InterestPaidRequest} request
 * @returns {InterestPaid}
 */
export function interestPaid(request) {
    const ret = wasm.interestPaid(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The index of the debt worth the most (a tie goes to the later one), or -1 when nothing is owed.
 *
 * # Errors
 *
 * Throws for a malformed request.
 * @param {LargestDebtRequest} request
 * @returns {number}
 */
export function largestDebt(request) {
    const ret = wasm.largestDebt(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ret[0];
}

/**
 * The swap tilted so what does not fit is left in the borrowed token.
 *
 * # Errors
 *
 * Throws for an empty swap.
 * @param {LeanRequest} request
 * @returns {RangeSwap}
 */
export function leanTowardDebt(request) {
    const ret = wasm.leanTowardDebt(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * A lender's interest to date in asset units, as a decimal int256.
 *
 * # Errors
 *
 * Throws on absurd values.
 * @param {LenderEarnedRequest} request
 * @returns {string}
 */
export function lenderEarned(request) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.lenderEarned(request);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * The leverage offered for a maximum (1e18), in tenths (25 = 2.5x).
 *
 * # Errors
 *
 * Throws for a malformed number.
 * @param {string} max_leverage_wad
 * @returns {number}
 */
export function leverageCap(max_leverage_wad) {
    const ptr0 = passStringToWasm0(max_leverage_wad, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.leverageCap(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ret[0] >>> 0;
}

/**
 * What it takes to reach the target leverage.
 *
 * # Errors
 *
 * Throws where the health check reverts.
 * @param {LeverageChangeRequest} request
 * @returns {LeverageChange}
 */
export function leverageChange(request) {
    const ret = wasm.leverageChange(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The swap of a one-token borrow into the range, tilted toward the debt.
 *
 * # Errors
 *
 * Throws for an invalid range.
 * @param {LeverageUpSwapRequest} request
 * @returns {MaybeSwap}
 */
export function leverageUpSwap(request) {
    const ret = wasm.leverageUpSwap(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Size over margin (1e18 = 1x), as hex; zero without margin. `equity_usd_wad` is a decimal or hex int256.
 *
 * # Errors
 *
 * Throws for a malformed number.
 * @param {string} size_usd_wad
 * @param {string} equity_usd_wad
 * @returns {string}
 */
export function leverageWad(size_usd_wad, equity_usd_wad) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passStringToWasm0(size_usd_wad, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(equity_usd_wad, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.leverageWad(ptr0, len0, ptr1, len1);
        var ptr3 = ret[0];
        var len3 = ret[1];
        if (ret[3]) {
            ptr3 = 0; len3 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * Repay, seize and fee amounts of a liquidation (`LiquidationLogic.liquidate`).
 *
 * # Errors
 *
 * Throws where the contract reverts (e.g. `position is not liquidatable`).
 * @param {LiquidateRequest} request
 * @returns {Liquidation}
 */
export function liquidate(request) {
    const ret = wasm.liquidate(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The numbers of a liquidation: the liquidity seized, the redeem's minimums and the sale into the debt asset.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 * @param {LiquidationRequest} request
 * @returns {LiquidationAmounts}
 */
export function liquidationAmounts(request) {
    const ret = wasm.liquidationAmounts(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The liquidation sale's least output, as hex.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 * @param {LiquidationMinOutRequest} request
 * @returns {string}
 */
export function liquidationMinOut(request) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.liquidationMinOut(request);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * The liquidation prices as pool prices (token1 per token0, unsorted) and the pool price now. For display.
 *
 * # Errors
 *
 * Throws for a malformed request.
 * @param {LiquidationPoolPricesRequest} request
 * @returns {LiquidationPoolPrices}
 */
export function liquidationPoolPrices(request) {
    const ret = wasm.liquidationPoolPrices(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Liquidation prices of a position (`SenzuLens._liquidationPrices`).
 *
 * # Errors
 *
 * Throws where the lens reverts.
 * @param {LiquidationPricesRequest} request
 * @returns {LiquidationPrices}
 */
export function liquidationPrices(request) {
    const ret = wasm.liquidationPrices(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Full USD breakdown of a liquidation: seized, repaid, protocol fee, liquidator's net incentive, and write-off.
 *
 * # Errors
 *
 * Throws on absurd values.
 * @param {SeizedShareRequest} request
 * @returns {LiquidationUsd}
 */
export function liquidationUsd(request) {
    const ret = wasm.liquidationUsd(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The pool's liquidity in buckets, from segments flattened as `[lower, upper, liquidity, …]`: flattened as
 * `[tickLower, tickUpper, liquidity, amount0, amount1, active (0 or 1), …]`.
 * @param {Float64Array} segments_flat
 * @param {number} tick
 * @param {number} bucket
 * @param {number} from_tick
 * @param {number} to_tick
 * @param {number} decimals0
 * @param {number} decimals1
 * @returns {Float64Array}
 */
export function liquidityBars(segments_flat, tick, bucket, from_tick, to_tick, decimals0, decimals1) {
    const ptr0 = passArrayF64ToWasm0(segments_flat, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.liquidityBars(ptr0, len0, tick, bucket, from_tick, to_tick, decimals0, decimals1);
    var v2 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
    return v2;
}

/**
 * Most liquidity `amount0` and `amount1` can add in `range` at `sqrtPriceX96`, rounded down, as hex
 * (`LiquidityAmounts.getLiquidityForAmounts`).
 *
 * # Errors
 *
 * Throws for an empty range, or when the liquidity does not fit in 128 bits.
 * @param {SqrtRange} range
 * @param {string} sqrt_price_x96
 * @param {string} amount0
 * @param {string} amount1
 * @returns {string}
 */
export function liquidityForAmounts(range, sqrt_price_x96, amount0, amount1) {
    let deferred5_0;
    let deferred5_1;
    try {
        const ptr0 = passStringToWasm0(sqrt_price_x96, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(amount0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(amount1, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.liquidityForAmounts(range, ptr0, len0, ptr1, len1, ptr2, len2);
        var ptr4 = ret[0];
        var len4 = ret[1];
        if (ret[3]) {
            ptr4 = 0; len4 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred5_0 = ptr4;
        deferred5_1 = len4;
        return getStringFromWasm0(ptr4, len4);
    } finally {
        wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
    }
}

/**
 * The liquidity providers' fee on a swap's amount in, less the protocol's, as hex.
 *
 * # Errors
 *
 * Throws on an absurd amount.
 * @param {LpFeeRequest} request
 * @returns {string}
 */
export function lpFee(request) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.lpFee(request);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * A plain liquidity position's value, profit, comparison with holding, and fee APR.
 *
 * # Errors
 *
 * Throws for a malformed input.
 * @param {LpInput} request
 * @returns {LpMetrics}
 */
export function lpMetrics(request) {
    const ret = wasm.lpMetrics(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The margin's value in USD (1e18), as hex.
 *
 * # Errors
 *
 * Throws on absurd amounts.
 * @param {MarginRequest} request
 * @returns {string}
 */
export function marginUsd(request) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.marginUsd(request);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * The position valued at its pool's price.
 *
 * # Errors
 *
 * Throws for an invalid range.
 * @param {MarkRequest} request
 * @returns {Mark}
 */
export function markPosition(request) {
    const ret = wasm.markPosition(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * What closing the position now sends back, at the pool's price.
 *
 * # Errors
 *
 * Throws for a zero price.
 * @param {MarkReceiveRequest} request
 * @returns {MarkReceive}
 */
export function markReceive(request) {
    const ret = wasm.markReceive(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * A market's borrow and supply APRs at a utilization.
 *
 * # Errors
 *
 * Throws for a reserve factor above 100%.
 * @param {MarketAprsRequest} request
 * @returns {MarketAprs}
 */
export function marketAprs(request) {
    const ret = wasm.marketAprs(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * A market's APRs after its cash moves.
 *
 * # Errors
 *
 * Throws for a reserve factor above 100%.
 * @param {MarketAprsAfterRequest} request
 * @returns {MarketAprs}
 */
export function marketAprsAfter(request) {
    const ret = wasm.marketAprsAfter(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * A market's interest over a year, split between reserves and lenders.
 *
 * # Errors
 *
 * Throws on absurd values.
 * @param {MarketFlowsRequest} request
 * @returns {MarketFlows}
 */
export function marketFlows(request) {
    const ret = wasm.marketFlows(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Most of `asset` the position can borrow now (`SenzuLens.maxBorrow`), hex.
 *
 * # Errors
 *
 * Throws where the lens reverts.
 * @param {MaxBorrowRequest} request
 * @returns {string}
 */
export function maxBorrow(request) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.maxBorrow(request);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Highest leverage (WAD, hex) a new position in the range passes the borrow check with.
 *
 * # Errors
 *
 * Throws where the lens reverts.
 * @param {MaxLeverageRequest} request
 * @returns {string}
 */
export function maxLeverage(request) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.maxLeverage(request);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * The largest share of the liquidity (1e18 = all) a redeem can take keeping the safety buffer, as hex.
 *
 * # Errors
 *
 * Throws where the health check reverts.
 * @param {PositionStateRequest} request
 * @returns {string}
 */
export function maxRemovableWad(request) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.maxRemovableWad(request);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * The least a swap quoted at `amount` may pay within `slippageBps`, as hex.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 * @param {string} amount
 * @param {string} slippage_bps
 * @returns {string}
 */
export function minOut(amount, slippage_bps) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passStringToWasm0(amount, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(slippage_bps, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.minOut(ptr0, len0, ptr1, len1);
        var ptr3 = ret[0];
        var len3 = ret[1];
        if (ret[3]) {
            ptr3 = 0; len3 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * The mint's minimums after the swap pays only its least output.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 * @param {MinimumsAfterSwapRequest} request
 * @returns {TokenAmounts}
 */
export function minimumsAfterSwap(request) {
    const ret = wasm.minimumsAfterSwap(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The multiple of the pool's tick spacing nearest `tick` (halfway rounds up), kept inside the ticks a pool allows.
 *
 * # Errors
 *
 * Throws for a spacing below 1 or a NaN tick.
 * @param {number} tick
 * @param {number} spacing
 * @returns {number}
 */
export function nearestUsableTick(tick, spacing) {
    const ret = wasm.nearestUsableTick(tick, spacing);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ret[0];
}

/**
 * Whether `allowance` is too low to spend `amount` (under the amount plus 1%).
 *
 * # Errors
 *
 * Throws for a malformed number.
 * @param {string} amount
 * @param {string} allowance
 * @returns {boolean}
 */
export function needsApproval(amount, allowance) {
    const ptr0 = passStringToWasm0(amount, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(allowance, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.needsApproval(ptr0, len0, ptr1, len1);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ret[0] !== 0;
}

/**
 * What an open borrows and its swap into the range, before the swap is quoted.
 *
 * # Errors
 *
 * Throws for a zero price or an invalid range.
 * @param {OpenSizing} sizing
 * @returns {OpenAmounts}
 */
export function openAmounts(sizing) {
    const ret = wasm.openAmounts(sizing);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * A new position's projected earnings; undefined without margin.
 *
 * # Errors
 *
 * Throws on an absurd debt.
 * @param {OpenEarningsInput} request
 * @returns {OpenEarnings | undefined}
 */
export function openEarnings(request) {
    const ret = wasm.openEarnings(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The liquidation prices of [`previewOpen`](preview_open) alone.
 *
 * # Errors
 *
 * Throws where the lens reverts.
 * @param {OpenParams} params
 * @returns {LiquidationPrices}
 */
export function openLiquidationPrices(params) {
    const ret = wasm.openLiquidationPrices(params);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The highest leverage (1e18) a new position opens with, keeping the safety buffer, as hex.
 *
 * # Errors
 *
 * Throws where the lens reverts.
 * @param {OpenMaxLeverageRequest} request
 * @returns {string}
 */
export function openMaxLeverage(request) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.openMaxLeverage(request);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * The mint's minimums of an open.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 * @param {OpenMinimumsRequest} request
 * @returns {TokenAmounts}
 */
export function openMinimums(request) {
    const ret = wasm.openMinimums(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * A new position's net yearly rate on its margin; undefined without margin.
 *
 * # Errors
 *
 * Throws for a malformed input.
 * @param {OpenNetAprRequest} request
 * @returns {number | undefined}
 */
export function openNetApr(request) {
    const ret = wasm.openNetApr(request);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    return ret[0] === 0 ? undefined : ret[1];
}

/**
 * Token0's share of the range's value, out of 1e10 (basis points with six decimals), as hex; undefined for a range
 * that holds nothing.
 *
 * # Errors
 *
 * Throws for an invalid range.
 * @param {PairShareRequest} request
 * @returns {string | undefined}
 */
export function pairToken0Share(request) {
    const ret = wasm.pairToken0Share(request);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    let v1;
    if (ret[0] !== 0) {
        v1 = getStringFromWasm0(ret[0], ret[1]);
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    }
    return v1;
}

/**
 * Without a swap, the amount of the other token that fits the deposit in the range at the pool price, as hex.
 *
 * # Errors
 *
 * Throws for a tick out of range.
 * @param {PairedAmountRequest} request
 * @returns {string}
 */
export function pairedAmount(request) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.pairedAmount(request);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * The position's and holding's profit or loss at `price`.
 *
 * # Errors
 *
 * Throws for malformed inputs.
 * @param {PayoffInputs} inputs
 * @param {number} price
 * @returns {PayoffPoint}
 */
export function payoffAt(inputs, price) {
    const ret = wasm.payoffAt(inputs, price);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The position's profit or loss at each of `prices` (fees not included).
 *
 * # Errors
 *
 * Throws for malformed inputs.
 * @param {PayoffInputs} inputs
 * @param {Float64Array} prices
 * @returns {Float64Array}
 */
export function payoffCurve(inputs, prices) {
    const ptr0 = passArrayF64ToWasm0(prices, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.payoffCurve(inputs, ptr0, len0);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v2 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
    return v2;
}

/**
 * Net token0 exposure now, in whole tokens (positive is long).
 *
 * # Errors
 *
 * Throws for malformed inputs.
 * @param {PayoffInputs} inputs
 * @returns {number}
 */
export function payoffExposure0(inputs) {
    const ret = wasm.payoffExposure0(inputs);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ret[0];
}

/**
 * The range's ends as pool prices (token1 per token0, whole tokens).
 *
 * # Errors
 *
 * Throws for malformed inputs.
 * @param {PayoffInputs} inputs
 * @returns {PriceRange}
 */
export function payoffRange(inputs) {
    const ret = wasm.payoffRange(inputs);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * A position's profit or loss against what was put in.
 *
 * # Errors
 *
 * Throws on absurd values.
 * @param {PnlInput} request
 * @returns {Pnl}
 */
export function pnl(request) {
    const ret = wasm.pnl(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Both tokens' USD prices at a pool price: the known one as given, the other as the pool implies.
 *
 * # Errors
 *
 * Throws for a price over 256 bits.
 * @param {PoolPriceRequest} request
 * @returns {PoolPrices}
 */
export function poolPrices(request) {
    const ret = wasm.poolPrices(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The pair's prices for sizing a close, a token without a live price valued at the pool.
 *
 * # Errors
 *
 * Throws for a price that does not fit in 256 bits.
 * @param {PoolValuedRequest} request
 * @returns {PerToken}
 */
export function poolValuedPrices(request) {
    const ret = wasm.poolValuedPrices(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * A position's yearly rates; undefined without size or margin.
 *
 * # Errors
 *
 * Throws for a malformed input.
 * @param {PositionAprInput} request
 * @returns {PositionApr | undefined}
 */
export function positionApr(request) {
    const ret = wasm.positionApr(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * What providing liquidity made against holding the tokens put in (USD), at today's prices; undefined before anything
 * was put in.
 *
 * # Errors
 *
 * Throws for a malformed input.
 * @param {VsHoldInput} request
 * @returns {number | undefined}
 */
export function positionVsHold(request) {
    const ret = wasm.positionVsHold(request);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    return ret[0] === 0 ? undefined : ret[1];
}

/**
 * The Create Position preview (`SenzuLens.previewOpen`).
 *
 * # Errors
 *
 * Throws where the lens reverts.
 * @param {OpenParams} params
 * @returns {OpenPreview}
 */
export function previewOpen(params) {
    const ret = wasm.previewOpen(params);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * [`previewOpen`](preview_open) without the liquidation prices: the cheap part, for live updates.
 *
 * # Errors
 *
 * Throws where the lens reverts.
 * @param {OpenParams} params
 * @returns {OpenSummary}
 */
export function previewOpenSummary(params) {
    const ret = wasm.previewOpenSummary(params);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The position's numbers in the state, from the contracts' own health math.
 *
 * # Errors
 *
 * Throws where the health check reverts.
 * @param {PreviewStateRequest} request
 * @returns {EditPreview}
 */
export function previewState(request) {
    const ret = wasm.previewState(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The tick whose price is nearest `price` (token1 per token0, whole tokens); align it before use.
 * @param {number} price
 * @param {number} decimals0
 * @param {number} decimals1
 * @returns {number}
 */
export function priceToTick(price, decimals0, decimals1) {
    const ret = wasm.priceToTick(price, decimals0, decimals1);
    return ret;
}

/**
 * The quote a minimum was written from within `slippageBps`, as hex.
 *
 * # Errors
 *
 * Throws for a slippage of 100% or more.
 * @param {string} min_out
 * @param {string} slippage_bps
 * @returns {string}
 */
export function quoteForMin(min_out, slippage_bps) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passStringToWasm0(min_out, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(slippage_bps, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.quoteForMin(ptr0, len0, ptr1, len1);
        var ptr3 = ret[0];
        var len3 = ret[1];
        if (ret[3]) {
            ptr3 = 0; len3 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * A new range's fee APR over its size; undefined without size or liquidity.
 *
 * # Errors
 *
 * Throws for a malformed number.
 * @param {number} fees_per_year_usd
 * @param {string} liquidity
 * @param {string} pool_liquidity
 * @param {number} size_usd
 * @returns {number | undefined}
 */
export function rangeFeeApr(fees_per_year_usd, liquidity, pool_liquidity, size_usd) {
    const ptr0 = passStringToWasm0(liquidity, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(pool_liquidity, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.rangeFeeApr(fees_per_year_usd, ptr0, len0, ptr1, len1, size_usd);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    return ret[0] === 0 ? undefined : ret[1];
}

/**
 * The swap that brings a deposit to the range's ratio at the pool price.
 *
 * # Errors
 *
 * Throws for an invalid range.
 * @param {RangeSwapRequest} request
 * @returns {MaybeSwap}
 */
export function rangeSwap(request) {
    const ret = wasm.rangeSwap(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * A sqrt price as a raw number (token1 base units per token0 base unit, square-rooted). For display.
 *
 * # Errors
 *
 * Throws for a malformed sqrt price.
 * @param {string} sqrt_price_x96
 * @returns {number}
 */
export function rawSqrtPrice(sqrt_price_x96) {
    const ptr0 = passStringToWasm0(sqrt_price_x96, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.rawSqrtPrice(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ret[0];
}

/**
 * The units a redeem takes and its minimums.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 * @param {RedeemShareRequest} request
 * @returns {RedeemShare}
 */
export function redeemShare(request) {
    const ret = wasm.redeemShare(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * What a redeem of the share pays at the pool price (`slippageBps` is ignored).
 *
 * # Errors
 *
 * Throws for an invalid range.
 * @param {RedeemShareRequest} request
 * @returns {PerToken}
 */
export function redeemedAmounts(request) {
    const ret = wasm.redeemedAmounts(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The state after redeeming a share of the liquidity.
 *
 * # Errors
 *
 * Throws for a share over 1e18.
 * @param {RemoveMarginStateRequest} request
 * @returns {EditState}
 */
export function removeMarginState(request) {
    const ret = wasm.removeMarginState(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * What to pull from the wallet to repay `repay` of `debt` with `balance`, as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 * @param {string} repay
 * @param {string} debt
 * @param {string} balance
 * @returns {string}
 */
export function repayPull(repay, debt, balance) {
    let deferred5_0;
    let deferred5_1;
    try {
        const ptr0 = passStringToWasm0(repay, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(debt, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(balance, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.repayPull(ptr0, len0, ptr1, len1, ptr2, len2);
        var ptr4 = ret[0];
        var len4 = ret[1];
        if (ret[3]) {
            ptr4 = 0; len4 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred5_0 = ptr4;
        deferred5_1 = len4;
        return getStringFromWasm0(ptr4, len4);
    } finally {
        wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
    }
}

/**
 * The state after a repayment.
 *
 * # Errors
 *
 * Throws for a malformed request.
 * @param {RepayStateRequest} request
 * @returns {EditState}
 */
export function repayState(request) {
    const ret = wasm.repayState(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The vault's share of the interest borrowers pay (1e18 = all), as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 * @param {string} inflow
 * @param {string} interest
 * @param {string} reserve_factor_wad
 * @returns {string}
 */
export function reserveShareWad(inflow, interest, reserve_factor_wad) {
    let deferred5_0;
    let deferred5_1;
    try {
        const ptr0 = passStringToWasm0(inflow, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(interest, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(reserve_factor_wad, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.reserveShareWad(ptr0, len0, ptr1, len1, ptr2, len2);
        var ptr4 = ret[0];
        var len4 = ret[1];
        if (ret[3]) {
            ptr4 = 0; len4 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred5_0 = ptr4;
        deferred5_1 = len4;
        return getStringFromWasm0(ptr4, len4);
    } finally {
        wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
    }
}

/**
 * Flattened segments with their ticks as raw sqrt prices, for `bookAmounts`: work out once per curve.
 * @param {Float64Array} segments_flat
 * @returns {Float64Array}
 */
export function segmentSqrts(segments_flat) {
    const ptr0 = passArrayF64ToWasm0(segments_flat, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.segmentSqrts(ptr0, len0);
    var v2 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
    return v2;
}

/**
 * The fees that go with the seized units to the liquidator, rounded down.
 *
 * # Errors
 *
 * Throws on absurd values.
 * @param {SeizedFeesRequest} request
 * @returns {PerToken}
 */
export function seizedFees(request) {
    const ret = wasm.seizedFees(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The share of the liquidity (1e18 = all) a repayment seizes, taken 0.5% low, as hex.
 *
 * # Errors
 *
 * Throws on absurd values.
 * @param {SeizedShareRequest} request
 * @returns {string}
 */
export function seizedShareWad(request) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.seizedShareWad(request);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * `part` of `total` (1e18 = all), as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 * @param {string} part
 * @param {string} total
 * @returns {string}
 */
export function shareWad(part, total) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passStringToWasm0(part, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(total, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.shareWad(ptr0, len0, ptr1, len1);
        var ptr3 = ret[0];
        var len3 = ret[1];
        if (ret[3]) {
            ptr3 = 0; len3 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * Sqrt price (Q64.96, hex) at a tick.
 *
 * # Errors
 *
 * Throws outside `[MIN_TICK, MAX_TICK]`.
 * @param {number} tick
 * @returns {string}
 */
export function sqrtPriceAtTick(tick) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.sqrtPriceAtTick(tick);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Token1 per token0, in whole tokens, at an exact sqrt price. For display.
 *
 * # Errors
 *
 * Throws for a malformed sqrt price.
 * @param {string} sqrt_price_x96
 * @param {number} decimals0
 * @param {number} decimals1
 * @returns {number}
 */
export function sqrtPriceToPrice(sqrt_price_x96, decimals0, decimals1) {
    const ptr0 = passStringToWasm0(sqrt_price_x96, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.sqrtPriceToPrice(ptr0, len0, decimals0, decimals1);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ret[0];
}

/**
 * The sqrt price at a pool price (token1 per token0, whole tokens), rounded down, as hex. For display and
 * approximate inputs.
 *
 * # Errors
 *
 * Throws for a negative or non-finite price.
 * @param {number} price
 * @param {number} decimals0
 * @param {number} decimals1
 * @returns {string}
 */
export function sqrtPriceX96AtPrice(price, decimals0, decimals1) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.sqrtPriceX96AtPrice(price, decimals0, decimals1);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Uniswap sqrt price (Q64.96, hex) of token0 in token1 from two USD prices. A zero `price1Wad` gives `MAX_SQRT_PRICE`.
 *
 * # Errors
 *
 * Throws on overflow.
 * @param {string} price0_wad
 * @param {string} price1_wad
 * @param {number} decimals0
 * @param {number} decimals1
 * @returns {string}
 */
export function sqrtPriceX96FromUsd(price0_wad, price1_wad, decimals0, decimals1) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passStringToWasm0(price0_wad, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(price1_wad, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.sqrtPriceX96FromUsd(ptr0, len0, ptr1, len1, decimals0, decimals1);
        var ptr3 = ret[0];
        var len3 = ret[1];
        if (ret[3]) {
            ptr3 = 0; len3 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * What `stake` of `total_staked` earns of `inflow`, as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 * @param {string} inflow
 * @param {string} stake
 * @param {string} total_staked
 * @returns {string}
 */
export function stakeIncome(inflow, stake, total_staked) {
    let deferred5_0;
    let deferred5_1;
    try {
        const ptr0 = passStringToWasm0(inflow, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(stake, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(total_staked, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.stakeIncome(ptr0, len0, ptr1, len1, ptr2, len2);
        var ptr4 = ret[0];
        var len4 = ret[1];
        if (ret[3]) {
            ptr4 = 0; len4 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred5_0 = ptr4;
        deferred5_1 = len4;
        return getStringFromWasm0(ptr4, len4);
    } finally {
        wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
    }
}

/**
 * A stake's share of all staked (1e18 = all), as hex; undefined without a stake or stakers.
 *
 * # Errors
 *
 * Throws for a malformed number.
 * @param {string} staked
 * @param {string} total_staked
 * @returns {string | undefined}
 */
export function stakeShareWad(staked, total_staked) {
    const ptr0 = passStringToWasm0(staked, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(total_staked, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.stakeShareWad(ptr0, len0, ptr1, len1);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    let v3;
    if (ret[0] !== 0) {
        v3 = getStringFromWasm0(ret[0], ret[1]);
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    }
    return v3;
}

/**
 * What a reward stream still pays, as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 * @param {string} rate_wad
 * @param {bigint} period_finish
 * @param {bigint} now
 * @returns {string}
 */
export function streamRemaining(rate_wad, period_finish, now) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(rate_wad, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.streamRemaining(ptr0, len0, period_finish, now);
        var ptr2 = ret[0];
        var len2 = ret[1];
        if (ret[3]) {
            ptr2 = 0; len2 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * The swap that fits two amounts to the range's ratio (`SenzuLens.swapForRange`).
 *
 * # Errors
 *
 * Throws where the lens reverts.
 * @param {SwapRequest} request
 * @returns {SwapQuote}
 */
export function swapForRange(request) {
    const ret = wasm.swapForRange(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The swaps that turn what a deleverage's redeem paid into the debts it repays.
 *
 * # Errors
 *
 * Throws for a zero price.
 * @param {SwapIntoDebtsRequest} request
 * @returns {DebtSwaps}
 */
export function swapIntoDebts(request) {
    const ret = wasm.swapIntoDebts(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Greatest tick whose sqrt price is at most `sqrtPriceX96`.
 *
 * # Errors
 *
 * Throws outside `[MIN_SQRT_PRICE, MAX_SQRT_PRICE)`.
 * @param {string} sqrt_price_x96
 * @returns {number}
 */
export function tickAtSqrtPrice(sqrt_price_x96) {
    const ptr0 = passStringToWasm0(sqrt_price_x96, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.tickAtSqrtPrice(ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ret[0];
}

/**
 * Token1 per token0, in whole tokens, at a tick. For display.
 * @param {number} tick
 * @param {number} decimals0
 * @param {number} decimals1
 * @returns {number}
 */
export function tickToPrice(tick, decimals0, decimals1) {
    const ret = wasm.tickToPrice(tick, decimals0, decimals1);
    return ret;
}

/**
 * `shares → assets` with the markets' virtual shares, hex.
 *
 * # Errors
 *
 * Throws on overflow.
 * @param {string} shares
 * @param {string} total_assets
 * @param {string} total_shares
 * @param {boolean} round_up
 * @returns {string}
 */
export function toAssets(shares, total_assets, total_shares, round_up) {
    let deferred5_0;
    let deferred5_1;
    try {
        const ptr0 = passStringToWasm0(shares, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(total_assets, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(total_shares, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.toAssets(ptr0, len0, ptr1, len1, ptr2, len2, round_up);
        var ptr4 = ret[0];
        var len4 = ret[1];
        if (ret[3]) {
            ptr4 = 0; len4 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred5_0 = ptr4;
        deferred5_1 = len4;
        return getStringFromWasm0(ptr4, len4);
    } finally {
        wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
    }
}

/**
 * `assets → shares` with the markets' virtual shares, hex.
 *
 * # Errors
 *
 * Throws on overflow.
 * @param {string} assets
 * @param {string} total_assets
 * @param {string} total_shares
 * @param {boolean} round_up
 * @returns {string}
 */
export function toShares(assets, total_assets, total_shares, round_up) {
    let deferred5_0;
    let deferred5_1;
    try {
        const ptr0 = passStringToWasm0(assets, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(total_assets, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(total_shares, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.toShares(ptr0, len0, ptr1, len1, ptr2, len2, round_up);
        var ptr4 = ret[0];
        var len4 = ret[1];
        if (ret[3]) {
            ptr4 = 0; len4 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred5_0 = ptr4;
        deferred5_1 = len4;
        return getStringFromWasm0(ptr4, len4);
    } finally {
        wasm.__wbindgen_free(deferred5_0, deferred5_1, 1);
    }
}

/**
 * One standard deviation of a day's log return, from hourly closes; undefined with under a day of them.
 * @param {Float64Array} hourly_closes
 * @returns {number | undefined}
 */
export function typicalDailyMove(hourly_closes) {
    const ptr0 = passArrayF64ToWasm0(hourly_closes, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.typicalDailyMove(ptr0, len0);
    return ret[0] === 0 ? undefined : ret[1];
}

/**
 * USD value (1e18) of `amount` of a token at its USD price, rounded down, as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 * @param {string} amount
 * @param {number} decimals
 * @param {string} price_wad
 * @returns {string}
 */
export function usdValue(amount, decimals, price_wad) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passStringToWasm0(amount, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(price_wad, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.usdValue(ptr0, len0, decimals, ptr1, len1);
        var ptr3 = ret[0];
        var len3 = ret[1];
        if (ret[3]) {
            ptr3 = 0; len3 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * Borrows over cash plus borrows (1e18 = 100%), as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 * @param {string} cash
 * @param {string} borrows
 * @returns {string}
 */
export function utilization(cash, borrows) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passStringToWasm0(cash, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(borrows, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.utilization(ptr0, len0, ptr1, len1);
        var ptr3 = ret[0];
        var len3 = ret[1];
        if (ret[3]) {
            ptr3 = 0; len3 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * The amounts in USD (1e18) at the pool's price, as hex.
 *
 * # Errors
 *
 * Throws on absurd amounts.
 * @param {ValueAtPoolRequest} request
 * @returns {string}
 */
export function valueAtPool(request) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.valueAtPool(request);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * What withdrawing a whole deposit can take now.
 *
 * # Errors
 *
 * Throws on absurd values.
 * @param {string} deposit_assets
 * @param {string} deposit_shares
 * @param {string} max_withdraw
 * @returns {WithdrawAll}
 */
export function withdrawAll(deposit_assets, deposit_shares, max_withdraw) {
    const ptr0 = passStringToWasm0(deposit_assets, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(deposit_shares, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ptr2 = passStringToWasm0(max_withdraw, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len2 = WASM_VECTOR_LEN;
    const ret = wasm.withdrawAll(ptr0, len0, ptr1, len1, ptr2, len2);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * What the liquidity holds at the band's low and high ends.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 * @param {LiquidityBand} band
 * @returns {BandEnds}
 */
export function withdrawEnds(band) {
    const ret = wasm.withdrawEnds(band);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The least of each token redeeming the liquidity pays anywhere in the band: the router's minimums for a redeem.
 *
 * # Errors
 *
 * Throws for a slippage above 100%.
 * @param {LiquidityBand} band
 * @returns {TokenAmounts}
 */
export function withdrawMinimums(band) {
    const ret = wasm.withdrawMinimums(band);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The shares a withdrawal burns and what they pay.
 *
 * # Errors
 *
 * Throws on absurd values.
 * @param {WithdrawalRequest} request
 * @returns {Withdrawal}
 */
export function withdrawal(request) {
    const ret = wasm.withdrawal(request);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * The debt (USD, 1e18) a liquidation writes off, as hex: zero unless it seizes every unit.
 *
 * # Errors
 *
 * Throws on absurd values.
 * @param {SeizedShareRequest} request
 * @returns {string}
 */
export function writtenOffUsdWad(request) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ret = wasm.writtenOffUsdWad(request);
        var ptr1 = ret[0];
        var len1 = ret[1];
        if (ret[3]) {
            ptr1 = 0; len1 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred2_0 = ptr1;
        deferred2_1 = len1;
        return getStringFromWasm0(ptr1, len1);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * What `amount` earns in a year at `apr_wad`, as hex.
 *
 * # Errors
 *
 * Throws for a malformed number.
 * @param {string} amount
 * @param {string} apr_wad
 * @returns {string}
 */
export function yearly(amount, apr_wad) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passStringToWasm0(amount, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(apr_wad, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.yearly(ptr0, len0, ptr1, len1);
        var ptr3 = ret[0];
        var len3 = ret[1];
        if (ret[3]) {
            ptr3 = 0; len3 = 0;
            throw takeFromExternrefTable0(ret[2]);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg_Error_30c8987f7c2ed4e2: function(arg0, arg1) {
            const ret = Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_Number_14af1003b8dd5ead: function(arg0) {
            const ret = Number(arg0);
            return ret;
        },
        __wbg_String_8564e559799eccda: function(arg0, arg1) {
            const ret = String(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_bigint_get_as_i64_a2383202b9353e4c: function(arg0, arg1) {
            const v = arg1;
            const ret = typeof(v) === 'bigint' ? v : undefined;
            getDataViewMemory0().setBigInt64(arg0 + 8 * 1, isLikeNone(ret) ? BigInt(0) : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_boolean_get_5b446f51afd21013: function(arg0) {
            const v = arg0;
            const ret = typeof(v) === 'boolean' ? v : undefined;
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        },
        __wbg___wbindgen_debug_string_4687d8d8c2017d52: function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_in_92f62ee1427d9e49: function(arg0, arg1) {
            const ret = arg0 in arg1;
            return ret;
        },
        __wbg___wbindgen_is_bigint_b123553bed3bb382: function(arg0) {
            const ret = typeof(arg0) === 'bigint';
            return ret;
        },
        __wbg___wbindgen_is_function_1f9d30630b8b1d3d: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_object_3c45d4f2dde4e749: function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            return ret;
        },
        __wbg___wbindgen_is_string_90b56bc79aad6f6c: function(arg0) {
            const ret = typeof(arg0) === 'string';
            return ret;
        },
        __wbg___wbindgen_is_undefined_8865fb403f8fe9d8: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_jsval_eq_02babf21faa37971: function(arg0, arg1) {
            const ret = arg0 === arg1;
            return ret;
        },
        __wbg___wbindgen_jsval_loose_eq_677f21e468d6b461: function(arg0, arg1) {
            const ret = arg0 == arg1;
            return ret;
        },
        __wbg___wbindgen_number_get_2e0e7dee9f701a71: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_string_get_0380ccaa2f57f0d9: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_41e9ee4f547fc59a: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_call_6137034ef55c9d0f: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.call(arg1);
            return ret;
        }, arguments); },
        __wbg_done_b41a1d26cdb37fb6: function(arg0) {
            const ret = arg0.done;
            return ret;
        },
        __wbg_entries_fb6397112b1de25f: function(arg0) {
            const ret = Object.entries(arg0);
            return ret;
        },
        __wbg_get_658f6698067d9515: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_6c896e0571ddae51: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_unchecked_288889d017702237: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_with_ref_key_6412cf3094599694: function(arg0, arg1) {
            const ret = arg0[arg1];
            return ret;
        },
        __wbg_instanceof_ArrayBuffer_a99f175873e5d9b8: function(arg0) {
            let result;
            try {
                result = arg0 instanceof ArrayBuffer;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Map_b2611749102d7ba3: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Map;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Uint8Array_828cef2aaacafc31: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Uint8Array;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_isArray_e15a2ff68ffdbef2: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_isSafeInteger_717808ad6a54bd9e: function(arg0) {
            const ret = Number.isSafeInteger(arg0);
            return ret;
        },
        __wbg_iterator_e3c31c892080e444: function() {
            const ret = Symbol.iterator;
            return ret;
        },
        __wbg_length_7f3c00c40364105e: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_d4bdea10311bd9cf: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_new_1dbf7428bba60a42: function(arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        },
        __wbg_new_617a8cdb8bb1130e: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_ee2291f50781bf1d: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_next_33784799010f1bbe: function(arg0) {
            const ret = arg0.next;
            return ret;
        },
        __wbg_next_f4aac29c42af995c: function() { return handleError(function (arg0) {
            const ret = arg0.next();
            return ret;
        }, arguments); },
        __wbg_prototypesetcall_bc27214492979395: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_set_6be42768c690e380: function(arg0, arg1, arg2) {
            arg0[arg1] = arg2;
        },
        __wbg_set_bea140a88be9b277: function(arg0, arg1, arg2) {
            arg0[arg1 >>> 0] = arg2;
        },
        __wbg_value_f3c585ee8f5ba40c: function(arg0) {
            const ret = arg0.value;
            return ret;
        },
        __wbindgen_generic_0000000000000001: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_generic_0000000000000002: function(arg0) {
            // Cast intrinsic for `I64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_generic_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_generic_0000000000000004: function(arg0) {
            // Cast intrinsic for `U64 -> Externref`.
            const ret = BigInt.asUintN(64, arg0);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./senzu_sdk_core_bg.js": import0,
    };
}

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function getArrayF64FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat64ArrayMemory0().subarray(ptr / 8, ptr / 8 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat64ArrayMemory0 = null;
function getFloat64ArrayMemory0() {
    if (cachedFloat64ArrayMemory0 === null || cachedFloat64ArrayMemory0.byteLength === 0) {
        cachedFloat64ArrayMemory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function passArrayF64ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 8, 8) >>> 0;
    getFloat64ArrayMemory0().set(arg, ptr / 8);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedFloat64ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (!module.ok) {
            throw new Error(`failed to fetch Wasm: ${module.status} ${module.statusText} fetching '${module.url}'`);
        }

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('senzu_sdk_core_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
