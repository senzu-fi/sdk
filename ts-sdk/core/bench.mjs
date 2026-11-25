// Node benchmark for @senzu/sdk-core WebAssembly, driven by the same vector inputs as rust-sdk/core/benches/core.rs.
// Run pinned to E-cores: taskset -c 4-11 node ts-sdk/core/bench.mjs

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { performance } from "node:perf_hooks";
import { fileURLToPath } from "node:url";

import * as wasm from "./pkg/senzu_sdk_core.js";

const here = dirname(fileURLToPath(import.meta.url));
wasm.initSync({ module: readFileSync(resolve(here, "pkg/senzu_sdk_core_bg.wasm")) });

const VECTORS_DIR = resolve(here, "../../rust-sdk/core/tests/vectors");
const hex = (s) => `0x${BigInt(s).toString(16)}`;
const dec = (s) => BigInt(s).toString(10);

function vectorLines(name) {
  return readFileSync(resolve(VECTORS_DIR, `${name}.txt`), "utf8")
    .split("\n")
    .filter((l) => l && !l.startsWith("#"))
    .map((l) => {
      const [inp, out] = l.split(" => ");
      return [inp.trim().split(/\s+/), out.trim()];
    });
}

class Args {
  constructor(tokens) {
    this.t = tokens;
    this.i = 0;
  }
  next() {
    return this.t[this.i++];
  }
  hex() {
    return hex(this.next());
  }
  dec() {
    return dec(this.next());
  }
  num() {
    return Number(this.next());
  }
  token() {
    return { priceWad: this.hex(), decimals: this.num(), sigmaWad: this.hex() };
  }
  collateral() {
    return {
      sqrtPaX96: this.hex(),
      sqrtPbX96: this.hex(),
      liquidity: this.dec(),
      token0: this.token(),
      token1: this.token(),
    };
  }
  asset(other) {
    const k = this.num();
    if (k === 0) return "token0";
    if (k === 1) return "token1";
    return { other };
  }
  debts() {
    const n = this.num();
    const out = [];
    for (let i = 0; i < n; i++) {
      const valueUsdWad = this.hex();
      const sigmaWad = this.hex();
      const tieNum = this.num();
      const tie = tieNum === 0 ? "untied" : tieNum === 1 ? "token0" : "token1";
      out.push({ valueUsdWad, sigmaWad, tie });
    }
    return out;
  }
  borrows(other) {
    const n = this.num();
    const out = [];
    for (let i = 0; i < n; i++) {
      out.push([this.asset(other), this.hex()]);
    }
    return out;
  }
  penalties() {
    return [BigInt(this.next()), BigInt(this.next()), BigInt(this.next())];
  }
}

function posPenalty(penalties, assets) {
  let max = penalties[0] > penalties[1] ? penalties[0] : penalties[1];
  for (const a of assets) {
    if (typeof a === "object" && penalties[2] > max) max = penalties[2];
  }
  return `0x${max.toString(16)}`;
}

// 1. evaluate & liquidation_prices cases
const evalCases = [];
const liqCases = [];
for (const [tokens, out] of vectorLines("evaluate")) {
  if (out.startsWith("err")) continue;
  const a = new Args(tokens);
  const collateral = a.collateral();
  const penaltyWad = a.hex();
  const debts = a.debts();
  if (collateral.liquidity !== "0" && debts.length > 0) {
    const req = { collateral, debts, penaltyWad };
    if (evalCases.length < 50) evalCases.push(req);
    if (!out.startsWith("-") && liqCases.length < 20) liqCases.push(req);
  }
  if (evalCases.length >= 50 && liqCases.length >= 20) break;
}

// 2. max_borrow cases
const maxBorrowCases = [];
for (const [tokens, out] of vectorLines("max_borrow")) {
  if (out.startsWith("err") || out === "0") continue;
  const a = new Args(tokens);
  const collateral = a.collateral();
  const other = a.token();
  const penalties = a.penalties();
  const borrowFeeWad = a.hex();
  const borrows = a.borrows(other);
  const asset = a.asset(other);
  const penaltyWad = posPenalty(penalties, [...borrows.map(([x]) => x), asset]);
  const market = {
    totalSupplyAssets: a.dec(),
    totalBorrowAssets: a.dec(),
    totalBorrowShares: a.dec(),
    lastAccrual: "0",
    reserveFactorWad: "0",
    interestRemainder: "0",
    rateAtTarget: "0",
  };
  const accountShares = a.hex();
  const others = borrows.filter(([b]) => JSON.stringify(b) !== JSON.stringify(asset));
  const otherDebts = others.map(([x, amount]) => wasm.debtInput({ collateral, asset: x, amount }));
  maxBorrowCases.push({
    collateral,
    otherDebts,
    asset,
    market,
    accountShares,
    penaltyWad,
    borrowFeeWad,
  });
  if (maxBorrowCases.length >= 20) break;
}

// 3. max_leverage cases
const maxLeverageCases = [];
for (const [tokens, out] of vectorLines("max_leverage")) {
  if (out.startsWith("err")) continue;
  const a = new Args(tokens);
  const collateral = a.collateral();
  const other = a.token();
  const penalties = a.penalties();
  const borrowFeeWad = a.hex();
  const mode = a.num();
  const borrow =
    mode === 0
      ? "pairRatio"
      : mode === 1
        ? { single: "token0" }
        : mode === 2
          ? { single: "token1" }
          : { single: { other } };
  const penaltyWad = mode === 0 ? posPenalty(penalties, []) : posPenalty(penalties, [borrow.single]);
  maxLeverageCases.push({ collateral, borrow, penaltyWad, borrowFeeWad });
  if (maxLeverageCases.length >= 20) break;
}

// 4. max_removable_wad & close_amounts cases
const editCases = [];
const closeCases = [];
for (const req of liqCases.slice(0, 10)) {
  const c = req.collateral;
  const oracle = wasm.sqrtPriceX96FromUsd(c.token0.priceWad, c.token1.priceWad, c.token0.decimals, c.token1.decimals);
  let totalDebtUsd = 0n;
  for (const d of req.debts) totalDebtUsd += BigInt(d.valueUsdWad);
  const debt1 = wasm.amountForUsd(`0x${(totalDebtUsd / 2n).toString(16)}`, c.token1.decimals, c.token1.priceWad);
  const position = {
    sqrtPaX96: c.sqrtPaX96,
    sqrtPbX96: c.sqrtPbX96,
    tokens: { token0: c.token0, token1: c.token1 },
    oracleSqrtPriceX96: oracle,
    positionPenaltyWad: req.penaltyWad,
    pairPenaltyWad: req.penaltyWad,
    otherDebts: [],
  };
  const state = {
    liquidity: c.liquidity,
    debt: { token0: "0x0", token1: debt1 },
  };
  editCases.push({ position, state });
  closeCases.push({
    position: {
      sqrtPaX96: c.sqrtPaX96,
      sqrtPbX96: c.sqrtPbX96,
      liquidity: c.liquidity,
      poolSqrtPriceX96: oracle,
      feesOwed0: "0x0",
      feesOwed1: "0x0",
    },
    debts: [
      {
        side: "token1",
        assets: debt1,
        price: { priceWad: c.token1.priceWad, decimals: c.token1.decimals },
      },
    ],
    prices: {
      token0: { priceWad: c.token0.priceWad, decimals: c.token0.decimals },
      token1: { priceWad: c.token1.priceWad, decimals: c.token1.decimals },
    },
    slippageBps: "50",
    swapFeeWad: `0x${(3_000_000_000_000_000n).toString(16)}`,
  });
}

// 5. amounts_for_liquidity cases (exercises amount1_delta + amount0_delta in WASM)
const amountsCases = [];
for (const [tokens, out] of vectorLines("liquidity_amounts")) {
  if (out.startsWith("err")) continue;
  const a = new Args(tokens);
  const sqrtPriceX96 = a.hex();
  const sqrtPaX96 = a.hex();
  const sqrtPbX96 = a.hex();
  const liquidity = a.dec();
  amountsCases.push([{ sqrtPaX96, sqrtPbX96, liquidity }, sqrtPriceX96]);
  if (amountsCases.length >= 200) break;
}

// 6. book_amounts cases
const rawSegs = new Float64Array(200 * 3);
for (let i = 0; i < 200; i++) {
  rawSegs[i * 3] = -6000 + i * 60;
  rawSegs[i * 3 + 1] = -6000 + (i + 1) * 60;
  rawSegs[i * 3 + 2] = 1e18 * (1 + (i % 7));
}
const sqrtSegs = wasm.segmentSqrts(rawSegs);
const edgesUp = Float64Array.from({ length: 40 }, (_, i) => 1.0001 ** (((i + 1) * 120) / 2));
const edgesDn = Float64Array.from({ length: 40 }, (_, i) => 1.0001 ** (-((i + 1) * 120) / 2));

// 7. pnl_curve case
const payoffInputs = {
  liquidity: `0x${(10n ** 20n).toString(16)}`,
  sqrtLowerX96: wasm.sqrtPriceAtTick(-1000),
  sqrtUpperX96: wasm.sqrtPriceAtTick(1000),
  decimals0: 18,
  decimals1: 6,
  debt0: `0x${(5n * 10n ** 18n).toString(16)}`,
  debt1: `0x${(5000n * 10n ** 6n).toString(16)}`,
  price: 1700.0,
  token1Usd: 1.0,
};
const payoffPrices = Float64Array.from({ length: 161 }, (_, i) => 1200 + i * 6);

// 8. swap_for_range cases
const swapCases = [];
for (const [tokens, out] of vectorLines("swap_for_range")) {
  if (out.startsWith("err")) continue;
  const a = new Args(tokens);
  const sqrtPaX96 = a.hex();
  const sqrtPbX96 = a.hex();
  const amount0 = a.hex();
  const amount1 = a.hex();
  let sqrtPriceX96 = a.hex();
  const swapFeeWad = a.hex();
  const p0 = a.hex();
  const d0 = a.num();
  const p1 = a.hex();
  const d1 = a.num();
  if (sqrtPriceX96 === "0x0") {
    sqrtPriceX96 = wasm.sqrtPriceX96FromUsd(p0, p1, d0, d1);
  }
  swapCases.push({ sqrtPaX96, sqrtPbX96, amount0, amount1, sqrtPriceX96, swapFeeWad });
  if (swapCases.length >= 50) break;
}

function bench(name, fn, callsPerBatch, batches = 15, warmup = 3) {
  for (let i = 0; i < warmup; i++) fn();
  const samples = [];
  for (let i = 0; i < batches; i++) {
    const t0 = performance.now();
    fn();
    samples.push(performance.now() - t0);
  }
  samples.sort((a, b) => a - b);
  const medianBatchMs = samples[Math.floor(samples.length / 2)];
  const perCallUs = (medianBatchMs * 1000) / callsPerBatch;
  console.log(
    `${name.padEnd(22)} batch=${medianBatchMs.toFixed(3).padStart(8)} ms  per_call=${perCallUs.toFixed(2).padStart(8)} µs`,
  );
}

const wasmBytes = readFileSync(resolve(here, "pkg/senzu_sdk_core_bg.wasm")).length;
console.log(`WASM size: ${wasmBytes} bytes\n`);

bench(
  "liquidationPrices",
  () => {
    for (const r of liqCases) wasm.liquidationPrices(r);
  },
  liqCases.length,
);
bench(
  "maxBorrow",
  () => {
    for (const r of maxBorrowCases) wasm.maxBorrow(r);
  },
  maxBorrowCases.length,
);
bench(
  "maxLeverage",
  () => {
    for (const r of maxLeverageCases) wasm.maxLeverage(r);
  },
  maxLeverageCases.length,
);
bench(
  "maxRemovableWad",
  () => {
    for (const r of editCases) wasm.maxRemovableWad(r);
  },
  editCases.length,
);
bench(
  "evaluateHealth",
  () => {
    for (const r of evalCases) wasm.evaluateHealth(r);
  },
  evalCases.length,
);
bench(
  "amountsForLiquidity",
  () => {
    for (const [range, sp] of amountsCases) wasm.amountsForLiquidity(range, sp);
  },
  amountsCases.length,
);
bench(
  "bookAmounts",
  () => {
    for (let i = 0; i < 100; i++) {
      wasm.bookAmounts(sqrtSegs, 1.0, edgesUp, 18, 6);
      wasm.bookAmounts(sqrtSegs, 1.0, edgesDn, 18, 6);
    }
  },
  200,
);
bench(
  "payoffCurve",
  () => {
    for (let i = 0; i < 100; i++) wasm.payoffCurve(payoffInputs, payoffPrices);
  },
  100,
);
bench(
  "closeAmounts",
  () => {
    for (let i = 0; i < 5; i++) for (const r of closeCases) wasm.closeAmounts(r);
  },
  closeCases.length * 5,
);
bench(
  "swapForRange",
  () => {
    for (let i = 0; i < 5; i++) for (const r of swapCases) wasm.swapForRange(r);
  },
  swapCases.length * 5,
);
