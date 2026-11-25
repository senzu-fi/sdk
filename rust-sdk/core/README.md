# senzu-sdk-core

Senzu's contract math in Rust, bit-exact with the Solidity code in [`contracts/src`](../../contracts/src).
Pure functions, no I/O, one dependency that matters (`alloy-primitives`). The TypeScript core package is its
`wasm-pack` build (`wasm` feature). SDK clients compute no numbers of their own: anything they need beyond reading
the chain lives here, so the TypeScript and Rust clients can never disagree.

| Module | Port of | What it gives the frontend and the keeper |
|---|---|---|
| `health` | `HealthMath`, `HealthLogic.position` | Four-corner health check, collateral value, oracle sqrt price, a position's liquidation penalty |
| `lens` | `SenzuLens` + `SenzuPreviews` searches | Liquidation prices, max borrow, max leverage, swap into a range, open preview |
| `liquidation` | `LiquidationLogic` | Repay cap, seized units |
| `market` | `MarketLogic`, `BorrowLogic` (market part) | Shares, accrual, rates, borrow and repay amounts |
| `irm` | `MarketLogic` adaptive curve (`adaptiveRate`, `rateAtTargetArea`, `expMinusOne`, `curve`), Solady `expWad`, `lnWad` | Borrow rate per second, its exact integral over a period, the rate at target over time |
| `fees` | `FeeLogic` | Pending LP fees per holder |
| `uniswap` | vendored v4-core `TickMath`, `SqrtPriceMath`, `FullMath`, `BitMath`; `LiquidityAmounts` | Tick and liquidity math |
| `slippage` | none: SDK policy every client shares | The minimums and maximums a router plan carries for a slippage (exact integer square roots); a quote read back from its minimum; the input a wanted output takes |
| `approval` | none: SDK policy | How much to approve and when an allowance is too low (headroom for amounts that move before the block) |
| `flows` | none: what every client sizes before a transaction | Open, edit, close, liquidate, withdraw and withdraw-all sizing (swaps, borrows, repays, redeems and their minimums); tick alignment; display prices (`f64`, `range` only) |
| `metrics` | none: what every app shows | Position value and profit, against holding, APRs and projected earnings, lender and vault income, pool-implied USD prices, payoff curves, liquidity bars, depth and book (`f64` where only charted), a live swap's LP fees |

Callers supply on-chain state (oracle prices, σ, market totals, debts). Positions are described by
`CollateralInput` (range, the account's liquidity, and each token's price, decimals and σ); debts by `DebtInput`,
usually built with `CollateralInput::debt`.

## Bit-exactness

Every function returns exactly what its contract counterpart returns, and an `Error` exactly where the contract
reverts. That is proven, not assumed:

- `contracts/script/SdkVectors.s.sol` runs ~20,000 inputs (random, log-uniform magnitudes, edge values) through
  the real Solidity code, including `SenzuPreviews` (the lens and its previews) and `LiquidationLogic` against a stub deployment, and writes
  `formal/vectors/*.txt` there, copied to `tests/vectors/`.
- `tests/vectors.rs` replays every vector and requires identical output or an error. `write_off.txt` (the real
  `SenzuBorrowable._writeOff`, through `script/sdk/WriteOffHarness.sol`) is replayed by the core repository's `formal/` instead.
- `just vectors-check` fails if these vectors differ from the contracts' (the `contracts/` submodule), so a
  contract change can't silently drift from the port.

`tests/properties.rs` adds proptest properties (rounding favors the protocol, searches return what they claim,
liquidations never take more than the balance).

Every Solidity operation that can revert is a checked operation here; clippy's `arithmetic_side_effects`, cast
and panic lints are `deny`, and the crate forbids `unsafe`.

## Not included

- Ekubo math: the Ekubo code is under the Ekubo DAO Shared Revenue License, so it stays out of this crate until
  that is cleared with Ekubo.
- Position-level borrow limits that need other state (max debts per position, pair debt caps, pair collateral
  caps) and the slot0 deviation guard: the client crate checks them with the chain's values.

## Wire format (`serde` / `wasm`)

Every integer wider than `u8` is a string: `U256` and `U160` as `0x` hex, `I256`, `u64` and `u128` as decimal.
Inputs accept decimal or hex. JavaScript's `BigInt(value)` reads all of them.

## Commands

From `sdk/`:

```
just check     # fmt, clippy (host and wasm32), tests, docs, MSRV, wasm build, vectors in sync
just vectors   # regenerate tests/vectors from the contracts
```
