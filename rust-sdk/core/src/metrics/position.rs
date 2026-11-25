//! A position valued at its pool's price: what the app shows as its size, equity and close value, where the risk
//! numbers stay at the oracle.

use alloy_primitives::{I256, U160, U256};

use super::prices::pool_prices;
use crate::error::Result;
use crate::flows::close::{ClosingDebt, ClosingPosition, estimate_receive};
use crate::flows::{PerToken, Priced};
use crate::num::{Checked, to_i256};
use crate::types::Side;
use crate::uniswap::amounts_for_liquidity;

/// How a pair is priced at its pool: the quote token at its market's USD price, the other as the pool implies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct PoolPricing {
    /// The pool's sqrt price.
    pub pool_sqrt_price_x96: U160,
    /// The token priced by its market (USDG, ETH: the pair's quote).
    pub quote: Side,
    /// Its USD price (1e18).
    pub quote_price_wad: U256,
    /// Token0's decimals.
    pub decimals0: u8,
    /// Token1's decimals.
    pub decimals1: u8,
}

impl PoolPricing {
    /// Both tokens' prices.
    ///
    /// # Errors
    ///
    /// As [`pool_prices`].
    pub fn prices(&self) -> Result<PerToken<Priced>> {
        let (p0, p1) = pool_prices(
            self.pool_sqrt_price_x96,
            self.quote,
            self.quote_price_wad,
            self.decimals0,
            self.decimals1,
        )?;
        Ok(PerToken {
            token0: Priced {
                price_wad: p0,
                decimals: self.decimals0,
            },
            token1: Priced {
                price_wad: p1,
                decimals: self.decimals1,
            },
        })
    }
}

/// `amounts` in USD (1e18) at `prices`, each token rounded down.
fn value_of(prices: PerToken<Priced>, amounts: PerToken<U256>) -> Result<U256> {
    prices
        .token0
        .usd(amounts.token0)?
        .try_add(prices.token1.usd(amounts.token1)?)
}

/// A debt outside the pair (a liquid token, as WETH on NVDA/USDG), valued at its own market's price.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct OutsideDebt {
    /// Amount owed.
    pub assets: U256,
    /// The borrowed token's USD price and decimals.
    pub price: Priced,
}

/// What [`mark`] values: a position's range, liquidity, debts and fees owed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct MarkedPosition {
    /// One end of the range, a Q64.96 sqrt price.
    pub sqrt_pa_x96: U160,
    /// The other end.
    pub sqrt_pb_x96: U160,
    /// The account's liquidity.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_num"))]
    #[cfg_attr(feature = "wasm", tsify(type = "string"))]
    pub liquidity: u128,
    /// Debt in each pair token.
    pub debt: PerToken<U256>,
    /// Debts outside the pair, at their own prices.
    #[cfg_attr(feature = "serde", serde(default))]
    pub outside_debts: Vec<OutsideDebt>,
    /// Fees earned and not collected.
    pub fees_owed: PerToken<U256>,
}

/// A position valued at its pool's price (USD in 1e18).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct Mark {
    /// Token0's USD price at the pool.
    pub price0_wad: U256,
    /// Token1's.
    pub price1_wad: U256,
    /// The liquidity's token0 at the pool price.
    pub amount0: U256,
    /// Its token1.
    pub amount1: U256,
    /// The liquidity's value.
    pub size_usd_wad: U256,
    /// The debts' value.
    pub debt_usd_wad: U256,
    /// Size less debt; negative once the debt is worth more.
    pub equity_usd_wad: I256,
    /// The fees owed's value.
    pub fees_usd_wad: U256,
}

/// The position valued at its pool's price.
///
/// # Errors
///
/// As [`pool_prices`] and [`amounts_for_liquidity`].
pub fn mark(pricing: &PoolPricing, position: &MarkedPosition) -> Result<Mark> {
    let prices = pricing.prices()?;
    let p = position;
    let (amount0, amount1) =
        amounts_for_liquidity(pricing.pool_sqrt_price_x96, p.sqrt_pa_x96, p.sqrt_pb_x96, p.liquidity)?;
    let size_usd_wad = value_of(
        prices,
        PerToken {
            token0: amount0,
            token1: amount1,
        },
    )?;
    let debt_usd_wad = p
        .outside_debts
        .iter()
        .try_fold(value_of(prices, p.debt)?, |sum, d| sum.try_add(d.price.usd(d.assets)?))?;
    Ok(Mark {
        price0_wad: prices.token0.price_wad,
        price1_wad: prices.token1.price_wad,
        amount0,
        amount1,
        size_usd_wad,
        debt_usd_wad,
        equity_usd_wad: to_i256(size_usd_wad)?.try_sub(to_i256(debt_usd_wad)?)?,
        fees_usd_wad: value_of(prices, p.fees_owed)?,
    })
}

/// `amounts` of the pair's tokens in USD (1e18) at the pool's price: an entry's margin at its block's price.
///
/// # Errors
///
/// As [`pool_prices`].
pub fn value_at_pool(pricing: &PoolPricing, amounts: PerToken<U256>) -> Result<U256> {
    value_of(pricing.prices()?, amounts)
}

/// What closing sends back at the pool price.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct MarkReceive {
    /// Token0 received.
    pub amount0: U256,
    /// Token1 received.
    pub amount1: U256,
    /// Both, in USD at the pool price.
    pub usd_wad: U256,
}

/// What closing the position now sends back, at the pool's price: the close's own estimate
/// ([`estimate_receive`]) with both tokens at their pool prices. Slippage is not counted.
///
/// # Errors
///
/// As [`estimate_receive`].
pub fn mark_receive(pricing: &PoolPricing, position: &MarkedPosition, swap_fee_wad: U256) -> Result<MarkReceive> {
    let prices = pricing.prices()?;
    let p = position;
    let closing = ClosingPosition {
        sqrt_pa_x96: p.sqrt_pa_x96,
        sqrt_pb_x96: p.sqrt_pb_x96,
        liquidity: p.liquidity,
        pool_sqrt_price_x96: pricing.pool_sqrt_price_x96,
        fees_owed0: p.fees_owed.token0,
        fees_owed1: p.fees_owed.token1,
    };
    let pair_debts = [Side::Token0, Side::Token1].map(|side| ClosingDebt {
        side: Some(side),
        assets: p.debt.get(side),
        price: Some(prices.get(side)),
    });
    let outside_debts = p.outside_debts.iter().map(|d| ClosingDebt {
        side: None,
        assets: d.assets,
        price: Some(d.price),
    });
    let debts: Vec<ClosingDebt> = pair_debts.into_iter().chain(outside_debts).collect();
    let (amount0, amount1) = estimate_receive(
        &closing,
        &debts,
        PerToken {
            token0: Some(prices.token0),
            token1: Some(prices.token1),
        },
        swap_fee_wad,
    )?;
    let usd_wad = value_of(
        prices,
        PerToken {
            token0: amount0,
            token1: amount1,
        },
    )?;
    Ok(MarkReceive {
        amount0,
        amount1,
        usd_wad,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::num::WAD;
    use crate::uniswap::sqrt_price_at_tick;

    fn pricing() -> PoolPricing {
        PoolPricing {
            pool_sqrt_price_x96: sqrt_price_at_tick(0).unwrap(),
            quote: Side::Token1,
            quote_price_wad: WAD,
            decimals0: 18,
            decimals1: 18,
        }
    }

    fn position(debt1: U256) -> MarkedPosition {
        MarkedPosition {
            sqrt_pa_x96: sqrt_price_at_tick(-600).unwrap(),
            sqrt_pb_x96: sqrt_price_at_tick(600).unwrap(),
            liquidity: 10_u128.pow(21),
            debt: PerToken {
                token0: U256::ZERO,
                token1: debt1,
            },
            fees_owed: PerToken {
                token0: WAD,
                token1: U256::ZERO,
            },
            outside_debts: Vec::new(),
        }
    }

    #[test]
    fn equity_is_size_less_debt_and_goes_negative() {
        let m = mark(&pricing(), &position(WAD)).unwrap();
        assert_eq!(
            m.equity_usd_wad,
            to_i256(m.size_usd_wad).unwrap() - to_i256(WAD).unwrap()
        );
        assert_eq!(m.fees_usd_wad, WAD);
        let under = mark(&pricing(), &position(m.size_usd_wad.try_mul(U256::from(2_u8)).unwrap())).unwrap();
        assert!(under.equity_usd_wad.is_negative());
    }

    #[test]
    fn a_close_without_debt_returns_the_liquidity_and_fees() {
        let p = position(U256::ZERO);
        let m = mark(&pricing(), &p).unwrap();
        let r = mark_receive(&pricing(), &p, U256::ZERO).unwrap();
        assert_eq!((r.amount0, r.amount1), (m.amount0 + WAD, m.amount1));
        assert_eq!(r.usd_wad, m.size_usd_wad + m.fees_usd_wad);
    }

    /// 0.001 WETH (18 decimals) at $3,000 owed on a pair of 6-decimal tokens: $3, never 1e15 of token0
    #[test]
    fn a_debt_outside_the_pair_is_valued_at_its_own_price() {
        let usdg = |amount: u64| U256::from(amount) * U256::from(10_u64.pow(6));
        let pricing = PoolPricing {
            decimals0: 6,
            decimals1: 6,
            ..pricing()
        };
        let mut p = position(U256::ZERO);
        p.fees_owed = PerToken::default();
        let free = mark_receive(&pricing, &p, U256::ZERO).unwrap();
        p.outside_debts = vec![OutsideDebt {
            assets: U256::from(10_u64.pow(15)),
            price: Priced {
                price_wad: U256::from(3_000_u64) * WAD,
                decimals: 18,
            },
        }];
        let m = mark(&pricing, &p).unwrap();
        assert_eq!(m.debt_usd_wad, U256::from(3_u8) * WAD);
        let owing = mark_receive(&pricing, &p, U256::ZERO).unwrap();
        assert_eq!(free.amount0 - owing.amount0, usdg(3));
        assert_eq!(owing.amount1, free.amount1);
    }
}
