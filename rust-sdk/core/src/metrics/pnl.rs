//! Profit and loss: a position's against what was put in, against holding the tokens put in, and the interest its
//! debts have cost.
//!
//! The profit is exact (USD in 1e18). The comparisons with holding are `f64`: the indexer's dollar flows are floats.

use alloy_primitives::{I256, U160, U256};

use crate::error::Result;
use crate::flows::PerToken;
use crate::flows::range::sqrt_price_to_price;
use crate::num::{Checked, to_i256};
use crate::types::Side;

/// Basis points in 100%.
const BPS: I256 = I256::from_raw(U256::from_limbs([10_000, 0, 0, 0]));
/// A deposit after the open is shown once it passes a cent and 1% of what went in.
const ADDED_MIN_USD_WAD: U256 = U256::from_limbs([10_000_000_000_000_000, 0, 0, 0]);

/// What a position's profit is made of (USD in 1e18).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct PnlInput {
    /// Its equity now (size less debt, at the pool price).
    pub equity_usd_wad: I256,
    /// Fees owed.
    pub fees_usd_wad: U256,
    /// Rewards earned (claimed and to claim).
    pub rewards_usd_wad: U256,
    /// The margin put in at the open, at its block's price.
    pub opening_usd_wad: U256,
    /// Everything deposited, the open included, when the indexer has it.
    pub deposited_usd_wad: Option<U256>,
    /// Everything taken out.
    pub withdrawn_usd_wad: U256,
}

/// A position's profit or loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct Pnl {
    /// Equity, fees, rewards and what was taken out, less what was put in.
    pub usd_wad: I256,
    /// Of what was put in, in basis points (toward zero).
    pub pct_bps: I256,
    /// What was put in: the deposits, or without them the opening margin.
    pub entry_usd_wad: U256,
    /// Deposited after the open: zero unless over a cent and 1% of what was put in.
    pub added_usd_wad: U256,
}

/// The position's profit or loss.
///
/// # Errors
///
/// [`crate::Error::Overflow`] on absurd values.
pub fn pnl(input: &PnlInput) -> Result<Pnl> {
    let i = input;
    let value = i
        .equity_usd_wad
        .try_add(to_i256(i.fees_usd_wad)?)?
        .try_add(to_i256(i.rewards_usd_wad)?)?;
    let entry = i.deposited_usd_wad.unwrap_or(i.opening_usd_wad);
    let usd_wad = value.try_add(to_i256(i.withdrawn_usd_wad)?)?.try_sub(to_i256(entry)?)?;
    let pct_bps = if entry.is_zero() {
        I256::ZERO
    } else {
        usd_wad.try_mul(BPS)?.try_div(to_i256(entry)?)?
    };
    let added = entry.saturating_sub(i.opening_usd_wad);
    let shown = added > ADDED_MIN_USD_WAD && added.try_mul(U256::from(100_u8))? > entry;
    Ok(Pnl {
        usd_wad,
        pct_bps,
        entry_usd_wad: entry,
        added_usd_wad: if shown { added } else { U256::ZERO },
    })
}

/// One token deposited at the open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct Deposit {
    /// The pair token; `None` for a token outside the pair (no comparison with holding then).
    pub side: Option<Side>,
    /// Amount, in token units.
    pub amount: U256,
}

use super::{to_f64, units};

fn signed_to_f64(value: I256) -> f64 {
    let magnitude = to_f64(value.unsigned_abs());
    if value.is_negative() { -magnitude } else { magnitude }
}

fn decimals_of(side: Side, decimals0: u8, decimals1: u8) -> u8 {
    match side {
        Side::Token0 => decimals0,
        Side::Token1 => decimals1,
    }
}

/// Both tokens' USD prices at the open: the pool's price then, scaled so the deposits are worth the opening margin.
/// `None` when the deposits are worth nothing at that price.
#[must_use]
pub fn entry_usd_prices(
    deposits: &[Deposit],
    decimals0: u8,
    decimals1: u8,
    entry_sqrt_price_x96: U160,
    opening_usd_wad: U256,
) -> Option<PerToken<f64>> {
    let put = |side: Side| {
        deposits
            .iter()
            .find(|d| d.side == Some(side))
            .map_or(0.0, |d| units(d.amount, decimals_of(side, decimals0, decimals1)))
    };
    let price0_in_1 = sqrt_price_to_price(entry_sqrt_price_x96, decimals0, decimals1);
    let in1 = put(Side::Token0) * price0_in_1 + put(Side::Token1);
    if in1.is_nan() || in1 <= 0.0 {
        return None;
    }
    let usd1 = to_f64(opening_usd_wad) / 1e18 / in1;
    Some(PerToken {
        token0: price0_in_1 * usd1,
        token1: usd1,
    })
}

/// A holder's token ledger in a position over its life, per pair token (token units): liquidity added (opens,
/// increases, compounds) and taken out (redeems, fees paid out, the close), and debt borrowed less repaid.
///
/// What the holder put in, net, is liquidity added less liquidity taken out less debt: holding exactly that instead is
/// what providing liquidity is measured against. Swaps the router makes on the way in or out are trades, not liquidity
/// provision, so they stay out of it (they are in the profit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct TokenFlows {
    /// Token0 added as liquidity.
    pub added0: U256,
    /// Token1 added as liquidity.
    pub added1: U256,
    /// Token0 taken out of the liquidity, fees paid out included.
    pub removed0: U256,
    /// Token1 taken out.
    pub removed1: U256,
    /// Token0 borrowed less repaid (negative after repaying more than was borrowed: interest).
    pub borrowed_net0: I256,
    /// Token1 borrowed less repaid.
    pub borrowed_net1: I256,
}

impl TokenFlows {
    /// Whether liquidity was ever added: without it there is nothing to hold instead.
    #[must_use]
    pub fn any_added(&self) -> bool {
        !self.added0.is_zero() || !self.added1.is_zero()
    }

    /// What holding the tokens put in, net, is worth at `usd` prices.
    fn held_usd(&self, decimals0: u8, decimals1: u8, usd: PerToken<f64>) -> f64 {
        let net = |added: U256, removed: U256, borrowed: I256, decimals: u8| {
            let scale = 10_f64.powi(i32::from(decimals));
            (to_f64(added) - to_f64(removed) - signed_to_f64(borrowed)) / scale
        };
        net(self.added0, self.removed0, self.borrowed_net0, decimals0) * usd.token0
            + net(self.added1, self.removed1, self.borrowed_net1, decimals1) * usd.token1
    }
}

/// An open position against holding the tokens put in.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct VsHoldInput {
    /// What the position is worth to its owner now: its equity, fees owed, and rewards claimed and to claim.
    pub value_usd_wad: I256,
    /// The holder's token ledger.
    pub flows: TokenFlows,
    /// Token0's decimals.
    pub decimals0: u8,
    /// Token1's decimals.
    pub decimals1: u8,
    /// Token0's USD price now (the mark's).
    pub price0_wad: U256,
    /// Token1's.
    pub price1_wad: U256,
}

/// What providing liquidity made against holding the tokens put in (USD): the position's value now less what holding
/// its net tokens ([`TokenFlows`]) is worth, all at today's prices. That is its fees and rewards, less impermanent
/// loss and interest. `None` before any liquidity was added.
#[must_use]
pub fn position_vs_hold(input: &VsHoldInput) -> Option<f64> {
    let i = input;
    if !i.flows.any_added() {
        return None;
    }
    let now = PerToken {
        token0: to_f64(i.price0_wad) / 1e18,
        token1: to_f64(i.price1_wad) / 1e18,
    };
    Some(signed_to_f64(i.value_usd_wad) / 1e18 - i.flows.held_usd(i.decimals0, i.decimals1, now))
}

/// An ended position against holding the tokens put in.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct ClosedVsHoldInput {
    /// The holder's token ledger, the close included.
    pub flows: TokenFlows,
    /// Token0's decimals.
    pub decimals0: u8,
    /// Token1's decimals.
    pub decimals1: u8,
    /// The pair's quote token (a dollar stablecoin).
    pub quote: Side,
    /// Its USD price.
    pub quote_usd: f64,
    /// The pool price at the end, token1 per token0.
    pub exit_price: f64,
    /// Rewards claimed over its life (USD).
    pub rewards_usd: f64,
}

/// What an ended position made against holding the tokens put in to its end (USD): its rewards less what holding its
/// net tokens ([`TokenFlows`]) is worth at the exit price. `None` without a quote price or an exit price, or before
/// any liquidity was added.
#[must_use]
pub fn closed_vs_hold(input: &ClosedVsHoldInput) -> Option<f64> {
    let i = input;
    let missing = |x: f64| x.is_nan() || x <= 0.0;
    if missing(i.quote_usd) || missing(i.exit_price) || !i.flows.any_added() {
        return None;
    }
    let q = i.quote_usd;
    let end = match i.quote {
        Side::Token0 => PerToken {
            token0: q,
            token1: q / i.exit_price,
        },
        Side::Token1 => PerToken {
            token0: i.exit_price * q,
            token1: q,
        },
    };
    Some(i.rewards_usd - i.flows.held_usd(i.decimals0, i.decimals1, end))
}

/// One debt asset's flows: what is owed now and what was borrowed less repaid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct DebtFlow {
    /// Owed now; zero for a debt since repaid.
    pub owed: U256,
    /// Borrowed less repaid, over the position's life.
    pub net_borrowed: I256,
    /// What is owed now in USD (1e18).
    pub value_usd_wad: U256,
}

/// The interest a position's debts have cost.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct InterestPaid {
    /// Per debt, in its token: owed less net borrowed; zero where nothing was paid.
    pub paid: Vec<U256>,
    /// All of it in USD, at each debt's value now (a debt since repaid counts nothing).
    pub usd_wad: U256,
}

/// The interest each debt has cost: what is owed beyond what was borrowed net.
///
/// # Errors
///
/// [`crate::Error::Overflow`] on absurd values.
pub fn interest_paid(debts: &[DebtFlow]) -> Result<InterestPaid> {
    let mut paid = Vec::with_capacity(debts.len());
    let mut usd_wad = U256::ZERO;
    for debt in debts {
        let interest = to_i256(debt.owed)?.try_sub(debt.net_borrowed)?;
        if !interest.is_positive() {
            paid.push(U256::ZERO);
            continue;
        }
        let interest = interest.into_raw();
        paid.push(interest);
        if !debt.owed.is_zero() {
            usd_wad = usd_wad.try_add(interest.try_mul(debt.value_usd_wad)?.try_div(debt.owed)?)?;
        }
    }
    Ok(InterestPaid { paid, usd_wad })
}

/// Seconds in a year, as a fee APR scales what has been earned.
const SECONDS_PER_YEAR: f64 = 365.0 * 86_400.0;

/// A plain liquidity position (no debt), as an indexer and the chain see it. Amounts in raw token units, values in USD.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct LpInput {
    /// Token0 its liquidity holds now.
    pub held0: f64,
    /// Token1.
    pub held1: f64,
    /// Token0 fees owed.
    pub owed0: f64,
    /// Token1 fees owed.
    pub owed1: f64,
    /// Token0 deposited less withdrawn: what holding would have kept.
    pub kept0: f64,
    /// Token1.
    pub kept1: f64,
    /// Token0's decimals.
    pub decimals0: u8,
    /// Token1's decimals.
    pub decimals1: u8,
    /// Token0's USD price now.
    pub usd0: f64,
    /// Token1's.
    pub usd1: f64,
    /// Rewards earned, claimed and to claim.
    pub rewards_usd: f64,
    /// Token0 fees already collected.
    pub collected0: f64,
    /// Token1 fees already collected.
    pub collected1: f64,
    /// Fees already collected, in USD when each was collected: what [`LpMetrics::pnl`] and the fee APR count.
    pub fees_collected_usd: f64,
    /// Everything withdrawn.
    pub withdrawn_usd: f64,
    /// Everything deposited.
    pub deposited_usd: f64,
    /// Seconds since the open.
    pub age_seconds: f64,
}

/// A plain liquidity position's value and returns (USD).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct LpMetrics {
    /// What its liquidity holds.
    pub value_usd: f64,
    /// Fees owed.
    pub owed_usd: f64,
    /// Fees collected and owed, and rewards.
    pub fees_earned_usd: f64,
    /// Value, fees, rewards and withdrawals less deposits.
    pub pnl: f64,
    /// Of what was deposited, in percent.
    pub pnl_pct: f64,
    /// Against holding what was deposited less withdrawn, every token at today's prices (collected fees included).
    pub vs_hold: f64,
    /// What was earned over what was deposited, a year at this pace, in percent.
    pub fee_apr: f64,
}

/// A plain liquidity position's value, profit, comparison with holding, and fee APR.
#[must_use]
pub fn lp_metrics(input: &LpInput) -> LpMetrics {
    let i = input;
    let worth = |raw0: f64, raw1: f64| {
        (raw0 / 10_f64.powi(i32::from(i.decimals0))) * i.usd0 + (raw1 / 10_f64.powi(i32::from(i.decimals1))) * i.usd1
    };
    let value_usd = worth(i.held0, i.held1);
    let owed_usd = worth(i.owed0, i.owed1);
    let fees_earned_usd = i.fees_collected_usd + owed_usd + i.rewards_usd;
    let pnl = value_usd + owed_usd + i.rewards_usd + i.withdrawn_usd + i.fees_collected_usd - i.deposited_usd;
    let deposited = i.deposited_usd > 0.0;
    LpMetrics {
        value_usd,
        owed_usd,
        fees_earned_usd,
        pnl,
        pnl_pct: if deposited {
            (pnl / i.deposited_usd) * 100.0
        } else {
            0.0
        },
        vs_hold: value_usd + owed_usd + i.rewards_usd + worth(i.collected0, i.collected1) - worth(i.kept0, i.kept1),
        fee_apr: if deposited && i.age_seconds > 0.0 {
            (fees_earned_usd / i.deposited_usd) * (SECONDS_PER_YEAR / i.age_seconds) * 100.0
        } else {
            0.0
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::num::WAD;

    fn usd(x: i64) -> I256 {
        I256::try_from(x).unwrap().try_mul(to_i256(WAD).unwrap()).unwrap()
    }

    fn wad(x: u64) -> U256 {
        U256::from(x).try_mul(WAD).unwrap()
    }

    #[test]
    fn pnl_counts_what_came_out_against_what_went_in() {
        let input = PnlInput {
            equity_usd_wad: usd(90),
            fees_usd_wad: wad(5),
            rewards_usd_wad: wad(1),
            opening_usd_wad: wad(100),
            deposited_usd_wad: Some(wad(120)),
            withdrawn_usd_wad: wad(30),
        };
        let p = pnl(&input).unwrap();
        assert_eq!(p.usd_wad, usd(6));
        assert_eq!(p.pct_bps, I256::try_from(500).unwrap());
        assert_eq!(p.added_usd_wad, wad(20));
        let losing = pnl(&PnlInput {
            equity_usd_wad: usd(-10),
            deposited_usd_wad: None,
            withdrawn_usd_wad: U256::ZERO,
            ..input
        })
        .unwrap();
        assert_eq!(losing.usd_wad, usd(-104));
        assert_eq!(losing.pct_bps, I256::try_from(-10_400).unwrap());
    }

    #[test]
    fn holding_compares_the_net_tokens_put_in() {
        // 1 token0 and 150 token1 added, 30 token1 taken out, 50 token1 borrowed: 1 token0 and 70 token1 put in, net
        let flows = TokenFlows {
            added0: wad(1),
            added1: wad(150),
            removed1: wad(30),
            borrowed_net1: usd(50),
            ..TokenFlows::default()
        };
        let input = VsHoldInput {
            value_usd_wad: usd(185),
            flows,
            decimals0: 18,
            decimals1: 18,
            price0_wad: wad(110),
            price1_wad: WAD,
        };
        // Holding is worth 110 + 70 = 180: 5 ahead
        assert!((position_vs_hold(&input).unwrap() - 5.0).abs() < 1e-9);
        assert_eq!(
            position_vs_hold(&VsHoldInput {
                flows: TokenFlows::default(),
                ..input
            }),
            None
        );
        // Closed: the rest came out (1 token0 and 125 token1) and the debt was repaid with 52 (2 of interest)
        let closed = ClosedVsHoldInput {
            flows: TokenFlows {
                removed0: wad(1),
                removed1: wad(155),
                borrowed_net1: usd(-2),
                ..flows
            },
            decimals0: 18,
            decimals1: 18,
            quote: Side::Token1,
            quote_usd: 1.0,
            exit_price: 110.0,
            rewards_usd: 3.0,
        };
        // Net in: 0 token0 and 150 - 155 + 2 = -3 token1 (3 more came out than went in), and 3 of rewards
        assert!((closed_vs_hold(&closed).unwrap() - 6.0).abs() < 1e-9);
        assert_eq!(
            closed_vs_hold(&ClosedVsHoldInput {
                exit_price: 0.0,
                ..closed
            }),
            None
        );
    }

    #[test]
    fn an_lp_that_earned_fees_beats_holding() {
        let m = lp_metrics(&LpInput {
            held0: 1e18,
            held1: 1e6,
            owed0: 0.0,
            owed1: 5e5,
            kept0: 1e18,
            kept1: 1e6,
            decimals0: 18,
            decimals1: 6,
            usd0: 2.0,
            usd1: 1.0,
            rewards_usd: 0.0,
            collected0: 0.0,
            collected1: 5e5,
            fees_collected_usd: 0.5,
            withdrawn_usd: 0.0,
            deposited_usd: 3.0,
            age_seconds: SECONDS_PER_YEAR,
        });
        assert!(
            (m.vs_hold - 1.0).abs() < 1e-12 && (m.pnl - 1.0).abs() < 1e-12 && (m.fee_apr - 100.0 / 3.0).abs() < 1e-9
        );
    }

    #[test]
    fn collected_fees_count_at_todays_price_against_holding() {
        // 0.1 token0 collected when it was worth 0.3; it is worth 0.2 now
        let m = lp_metrics(&LpInput {
            held0: 1e18,
            held1: 0.0,
            owed0: 0.0,
            owed1: 0.0,
            kept0: 1e18,
            kept1: 0.0,
            decimals0: 18,
            decimals1: 6,
            usd0: 2.0,
            usd1: 1.0,
            rewards_usd: 0.0,
            collected0: 1e17,
            collected1: 0.0,
            fees_collected_usd: 0.3,
            withdrawn_usd: 0.0,
            deposited_usd: 2.0,
            age_seconds: SECONDS_PER_YEAR,
        });
        assert!((m.vs_hold - 0.2).abs() < 1e-12 && (m.pnl - 0.3).abs() < 1e-12);
    }
    #[test]
    fn interest_is_what_is_owed_beyond_the_net_borrow() {
        let flows = [
            DebtFlow {
                owed: wad(110),
                net_borrowed: usd(100),
                value_usd_wad: wad(220),
            },
            DebtFlow {
                owed: U256::ZERO,
                net_borrowed: usd(-3),
                value_usd_wad: U256::ZERO,
            },
            DebtFlow {
                owed: wad(50),
                net_borrowed: usd(60),
                value_usd_wad: wad(50),
            },
        ];
        let paid = interest_paid(&flows).unwrap();
        assert_eq!(paid.paid, vec![wad(10), wad(3), U256::ZERO]);
        assert_eq!(paid.usd_wad, wad(20));
    }
}
