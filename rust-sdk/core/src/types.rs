use alloy_primitives::{I256, U160, U256};

use crate::error::Result;
use crate::num::pow10;
use crate::uniswap::mul_div_rounding_up;

/// Oracle price, decimals and stress factor of one token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct TokenInput {
    /// USD per whole token, 1e18 = $1.
    pub price_wad: U256,
    /// ERC-20 decimals.
    pub decimals: u8,
    /// Stress factor σ from the risk config, at least 1e18.
    pub sigma_wad: U256,
}

/// The collateral side of a health check (`CollateralInput` in `Types.sol`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct CollateralInput {
    /// One end of the range as a Q64.96 sqrt price (full-range positions use `MIN_SQRT_PRICE`).
    pub sqrt_pa_x96: U160,
    /// The other end (full range: `MAX_SQRT_PRICE`).
    pub sqrt_pb_x96: U160,
    /// The account's share of the position's liquidity: `units · liquidity / totalUnits`.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub liquidity: u128,
    /// Token0 of the pair.
    pub token0: TokenInput,
    /// Token1 of the pair.
    pub token1: TokenInput,
}

impl CollateralInput {
    /// The token on one side of the pair.
    #[must_use]
    pub const fn token(&self, side: Side) -> &TokenInput {
        match side {
            Side::Token0 => &self.token0,
            Side::Token1 => &self.token1,
        }
    }

    /// A debt of `amount` of `asset` against this collateral, valued at the current price and rounded up
    /// (`HealthLogic._debtInput`). A debt in a pair token reuses that token's price, decimals and σ.
    ///
    /// # Errors
    ///
    /// [`Error::Overflow`](crate::Error::Overflow) for 78 or more decimals and [`Error::MulDiv`](crate::Error::MulDiv)
    /// when the value overflows.
    pub fn debt(&self, asset: &Asset, amount: U256) -> Result<DebtInput> {
        let (token, tie) = match asset {
            Asset::Token0 => (&self.token0, DebtTie::Token0),
            Asset::Token1 => (&self.token1, DebtTie::Token1),
            Asset::Other(token) => (token, DebtTie::Untied),
        };
        Ok(DebtInput {
            value_usd_wad: mul_div_rounding_up(amount, token.price_wad, pow10(token.decimals)?)?,
            sigma_wad: token.sigma_wad,
            tie,
        })
    }
}

/// One of the pair's two tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub enum Side {
    /// Token0.
    Token0,
    /// Token1.
    Token1,
}

impl Side {
    /// The pair's other token.
    #[must_use]
    pub const fn other(self) -> Self {
        match self {
            Self::Token0 => Self::Token1,
            Self::Token1 => Self::Token0,
        }
    }
}

/// A borrowed asset as the math sees it: one of the pair's tokens, or an unrelated token with its own price.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub enum Asset {
    /// The pair's token0: its debt moves with token0 in the stress corners.
    Token0,
    /// The pair's token1: its debt moves with token1 in the stress corners.
    Token1,
    /// Any other token: its debt is always stressed up by its own σ.
    Other(TokenInput),
}

/// How a debt's price moves in the stress corners (`DebtTie` in `Types.sol`; `Untied` is its `None`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub enum DebtTie {
    /// An unrelated asset, always stressed up by its own σ.
    #[default]
    Untied,
    /// The collateral's token0: moves with that corner's token0 price.
    Token0,
    /// The collateral's token1: moves with that corner's token1 price.
    Token1,
}

/// One debt of a position, valued in USD at the current price (`DebtInput` in `Types.sol`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct DebtInput {
    /// Debt in USD, 1e18 = $1, rounded up.
    pub value_usd_wad: U256,
    /// σ of the asset (only read for untied debts).
    pub sigma_wad: U256,
    /// How the debt moves in the corners.
    pub tie: DebtTie,
}

/// Result of a health check (`HealthResult` in `Types.sol`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct HealthResult {
    /// Worst corner of `collateral − debt · penalty`. Negative: the check fails.
    pub min_surplus_usd_wad: I256,
    /// Worst corner of `collateral / (debt · penalty)`, `U256::MAX` without debt. Below 1e18: liquidatable.
    pub health_wad: U256,
    /// Collateral value at current (unstressed) prices.
    pub collateral_usd_wad: U256,
    /// Debt value at current (unstressed) prices.
    pub debt_usd_wad: U256,
}

impl HealthResult {
    /// Whether every corner is covered (`minSurplusUsdWad >= 0`).
    #[must_use]
    pub fn passes(&self) -> bool {
        !self.min_surplus_usd_wad.is_negative()
    }
}
