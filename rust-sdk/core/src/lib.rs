//! Senzu's contract math in Rust.
//!
//! Pure functions, no I/O: the four-corner health check, Uniswap tick and liquidity math, lending share and
//! interest math (the adaptive rate curve), liquidation amounts, LP fee accounting and the lens searches
//! (liquidation prices, max borrow, max leverage, swap into a range, open preview). Callers bring the on-chain
//! state (prices, σ, market totals).
//!
//! Every function is a port of the Solidity code in `contracts/src` and is **bit-exact** with it: same rounding,
//! and an [`Error`] exactly where the contract reverts. `tests/vectors.rs` proves it on vectors that
//! `contracts/script/SdkVectors.s.sol` generates from the contracts themselves.
//!
//! It also holds the math every SDK client must compute identically but no contract does: the [`slippage`] bounds a
//! router plan carries. Clients read the chain and build transactions; any number they compute comes from here.
//!
//! ```
//! use alloy_primitives::{U256, uint};
//! use senzu_sdk_core::{CollateralInput, TokenInput, health, uniswap};
//!
//! # fn main() -> Result<(), senzu_sdk_core::Error> {
//! let eth = TokenInput { price_wad: uint!(2000_000000000000000000_U256), decimals: 18, sigma_wad: uint!(1150000000000000000_U256) };
//! let usdc = TokenInput { price_wad: uint!(1000000000000000000_U256), decimals: 6, sigma_wad: uint!(1020000000000000000_U256) };
//! let collateral = CollateralInput {
//!     sqrt_pa_x96: uniswap::MIN_SQRT_PRICE,
//!     sqrt_pb_x96: uniswap::MAX_SQRT_PRICE,
//!     liquidity: 100_000_000_000_000,
//!     token0: eth,
//!     token1: usdc,
//! };
//! let debt = collateral.debt(&senzu_sdk_core::Asset::Token1, U256::from(100_000_000_u64))?;
//! // ETH's 1.08 and USDC's 1.05: the position pays the higher (no debt outside the pair)
//! let penalty = health::position_penalty(uint!(1080000000000000000_U256), uint!(1050000000000000000_U256), &[]);
//! let result = health::evaluate(&collateral, &[debt], penalty)?;
//! assert!(result.passes());
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//!
//! - `serde`: `Serialize` / `Deserialize` for every public type. Every integer wider than `u8` is a string
//!   (`U256` / `U160` as `0x` hex, `I256`, `u64` and `u128` as decimal; all accept decimal or hex), which
//!   JavaScript's `BigInt()` reads directly.
//! - `wasm`: `wasm-bindgen` exports and TypeScript types for the `ts-sdk/core` package.

mod error;
mod num;
#[cfg(feature = "serde")]
mod serde_num;
mod types;

pub mod approval;
pub mod fees;
pub mod flows;
pub mod health;
pub mod irm;
pub mod lens;
pub mod liquidation;
pub mod market;
pub mod metrics;
pub mod slippage;
pub mod uniswap;

#[cfg(feature = "wasm")]
pub mod wasm;

pub use error::{Error, Result};
pub use fees::FeeCheckpoint;
pub use irm::AdaptiveRate;
pub use market::{Market, Rounding};
pub use num::WAD;
pub use types::{Asset, CollateralInput, DebtInput, DebtTie, HealthResult, Side, TokenInput};
