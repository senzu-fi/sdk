//! Differential tests against the vectors `contracts/script/SdkVectors.s.sol` generates from the contracts.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines
)]

use std::path::Path;

use alloy_primitives::{I256, U160, U256};
use senzu_sdk_core::lens::{self, LeverageBorrow, OpenBorrow, OpenParams};
use senzu_sdk_core::liquidation::{self, RepaidDebt};
use senzu_sdk_core::market::{self, Rounding};
use senzu_sdk_core::{
    Asset, CollateralInput, DebtInput, DebtTie, Error, FeeCheckpoint, Market, Result, TokenInput, fees, health, irm,
    uniswap,
};

struct Args<'a>(std::str::SplitWhitespace<'a>);

impl Args<'_> {
    fn next(&mut self) -> &str {
        self.0.next().expect("vector has fewer inputs than expected")
    }
    fn u256(&mut self) -> U256 {
        self.next().parse().expect("uint256")
    }
    fn u160(&mut self) -> U160 {
        self.next().parse().expect("uint160")
    }
    fn u128(&mut self) -> u128 {
        self.next().parse().expect("uint128")
    }
    fn u64(&mut self) -> u64 {
        self.next().parse().expect("uint64")
    }
    fn u8(&mut self) -> u8 {
        self.next().parse().expect("uint8")
    }
    fn i32(&mut self) -> i32 {
        self.next().parse().expect("int24")
    }
    fn bool(&mut self) -> bool {
        match self.next() {
            "0" => false,
            "1" => true,
            other => panic!("bool expected, got {other}"),
        }
    }
    fn usize(&mut self) -> usize {
        self.next().parse().expect("count")
    }

    fn token(&mut self) -> TokenInput {
        TokenInput {
            price_wad: self.u256(),
            decimals: self.u8(),
            sigma_wad: self.u256(),
        }
    }
    fn collateral(&mut self) -> CollateralInput {
        CollateralInput {
            sqrt_pa_x96: self.u160(),
            sqrt_pb_x96: self.u160(),
            liquidity: self.u128(),
            token0: self.token(),
            token1: self.token(),
        }
    }
    fn asset(&mut self, other: TokenInput) -> Asset {
        match self.u8() {
            0 => Asset::Token0,
            1 => Asset::Token1,
            2 => Asset::Other(other),
            index => panic!("unknown asset {index}"),
        }
    }
    fn debts(&mut self) -> Vec<DebtInput> {
        (0..self.usize())
            .map(|_| DebtInput {
                value_usd_wad: self.u256(),
                sigma_wad: self.u256(),
                tie: match self.u8() {
                    0 => DebtTie::Untied,
                    1 => DebtTie::Token0,
                    _ => DebtTie::Token1,
                },
            })
            .collect()
    }
    fn borrows(&mut self, other: TokenInput) -> Vec<(Asset, U256)> {
        (0..self.usize()).map(|_| (self.asset(other), self.u256())).collect()
    }
    /// Token0's, token1's and the other token's liquidation penalties.
    fn penalties(&mut self) -> [U256; 3] {
        [self.u256(), self.u256(), self.u256()]
    }
}

/// `HealthLogic.position`'s penalty: the pair's two and the other token's once per debt in it.
fn position_penalty<'a>(penalties: [U256; 3], assets: impl IntoIterator<Item = &'a Asset>) -> U256 {
    let outside: Vec<U256> = assets
        .into_iter()
        .filter(|asset| matches!(asset, Asset::Other(_)))
        .map(|_| penalties[2])
        .collect();
    health::position_penalty(penalties[0], penalties[1], &outside)
}

trait Token {
    fn token(&self) -> String;
}

macro_rules! display_token {
    ($($t:ty),*) => {$(impl Token for $t { fn token(&self) -> String { self.to_string() } })*};
}
display_token!(U256, I256, U160, u128, u64, i32);

impl Token for bool {
    fn token(&self) -> String {
        u8::from(*self).to_string()
    }
}

impl<T: Token> Token for Option<T> {
    fn token(&self) -> String {
        self.as_ref().map_or_else(|| "0".to_owned(), Token::token)
    }
}

fn join(values: &[&dyn Token]) -> String {
    values.iter().map(|v| v.token()).collect::<Vec<_>>().join(" ")
}

/// Maps each Rust error variant to the Solidity revert selector(s) or panic code it mirrors.
fn err_matches(err: &Error, expected: &str) -> bool {
    match err {
        Error::Overflow | Error::Underflow => expected == "err:panic:17",
        Error::DivisionByZero => expected == "err:panic:18",
        Error::MulDiv => expected == "err:empty" || expected == "err:0xae47f702",
        Error::CastOverflow => expected == "err:0x35278d12" || expected == "err:0x93dafdf1",
        Error::InvalidTick(_) => expected == "err:0x8b86327a",
        Error::InvalidSqrtPrice(_) => expected == "err:0x61487524",
        Error::ZeroSqrtPrice => expected == "err:0x00bfc921",
        Error::InvalidPriceOrLiquidity => expected == "err:0x4f2461b8",
        Error::NotEnoughLiquidity => expected == "err:0x4323a555",
        Error::PriceOverflow => expected == "err:0xf5c787f1",
        Error::InvalidSigma => expected == "err:0xf744a0d3",
        Error::InvalidPrice => expected == "err:0x2e3ac92c",
        Error::ExpOverflow => expected == "err:0xa37bfec9",
        Error::LnWadUndefined => expected == "err:0x1615e638",
        Error::InsufficientCash => expected == "err:0x8377f5d3",
        Error::DebtTooLarge => expected == "err:0x6fef3e9c",
        Error::ZeroAmount => expected == "err:0xf7a06424" || expected == "err:0xa181a597",
        Error::NotLiquidatable => expected == "err:0x8dab30e7",
        Error::NoDebt => expected == "err:0xeab73711" || expected == "err:0x86506913",
        _ => false,
    }
}

fn match_err(err: &Error, expected: &str) -> String {
    if err_matches(err, expected) {
        expected.to_owned()
    } else {
        format!("Err({err:?})")
    }
}

fn one<T: Token>(result: Result<T>, expected: &str) -> String {
    match result {
        Ok(value) => value.token(),
        Err(err) => match_err(&err, expected),
    }
}

fn fmt_res<T>(result: Result<T>, expected: &str, f: impl FnOnce(T) -> String) -> String {
    match result {
        Ok(value) => f(value),
        Err(err) => match_err(&err, expected),
    }
}

fn check(name: &str, f: impl Fn(&mut Args<'_>, &str) -> String) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/vectors")
        .join(format!("{name}.txt"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut failures = Vec::new();
    let mut count = 0;
    for (number, line) in text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.starts_with('#') && !l.is_empty())
    {
        let (inputs, expected) = line.split_once(" => ").expect("vector without =>");
        let expected = expected.trim();
        let got = f(&mut Args(inputs.split_whitespace()), expected);
        count += 1;
        if got != expected {
            failures.push(format!(
                "{name}.txt:{}\n  inputs:   {inputs}\n  expected: {expected}\n  got:      {got}",
                number + 1
            ));
        }
    }
    assert!(count > 0, "{name}: no vectors");
    assert!(
        failures.is_empty(),
        "{} of {count} vectors differ:\n{}",
        failures.len(),
        failures[..failures.len().min(8)].join("\n")
    );
}

#[test]
fn mul_div() {
    check("mul_div", |a, exp| {
        let (x, y, d) = (a.u256(), a.u256(), a.u256());
        let mut parts = exp.split_whitespace();
        let (e1, e2) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        format!(
            "{} {}",
            one(uniswap::mul_div(x, y, d), e1),
            one(uniswap::mul_div_rounding_up(x, y, d), e2)
        )
    });
}

#[test]
fn sqrt() {
    check("sqrt", |a, exp| one(uniswap::sqrt(a.u256()), exp));
}

#[test]
fn tick_math() {
    check("tick_math", |a, exp| {
        let (tick, sqrt_price) = (a.i32(), a.u160());
        let mut parts = exp.split_whitespace();
        let (e1, e2) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        format!(
            "{} {}",
            one(uniswap::sqrt_price_at_tick(tick), e1),
            one(uniswap::tick_at_sqrt_price(sqrt_price), e2)
        )
    });
}

#[test]
fn amount_delta() {
    check("amount_delta", |a, exp| {
        let (lower, upper, liquidity, round_up) = (a.u160(), a.u160(), a.u128(), a.bool());
        let mut parts = exp.split_whitespace();
        let (e1, e2) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        format!(
            "{} {}",
            one(uniswap::amount0_delta(lower, upper, liquidity, round_up), e1),
            one(uniswap::amount1_delta(lower, upper, liquidity, round_up), e2)
        )
    });
}

#[test]
fn next_sqrt_price() {
    check("next_sqrt_price", |a, exp| {
        let (sqrt_price, liquidity, amount, zero_for_one, input) = (a.u160(), a.u128(), a.u256(), a.bool(), a.bool());
        one(
            if input {
                uniswap::next_sqrt_price_from_input(sqrt_price, liquidity, amount, zero_for_one)
            } else {
                uniswap::next_sqrt_price_from_output(sqrt_price, liquidity, amount, zero_for_one)
            },
            exp,
        )
    });
}

#[test]
fn liquidity_amounts() {
    check("liquidity_amounts", |a, exp| {
        let (price, lower, upper, liquidity, amount0, amount1) =
            (a.u160(), a.u160(), a.u160(), a.u128(), a.u256(), a.u256());
        let mut parts = exp.split_whitespace();
        let (e0, _e1, e2) = (
            parts.next().unwrap_or(""),
            parts.next().unwrap_or(""),
            parts.next().unwrap_or(""),
        );
        let amounts = match uniswap::amounts_for_liquidity(price, lower, upper, liquidity) {
            Ok((x, y)) => format!("{x} {y}"),
            Err(err) => {
                let tag = match_err(&err, e0);
                format!("{tag} {tag}")
            }
        };
        format!(
            "{amounts} {}",
            one(
                uniswap::liquidity_for_amounts(price, lower, upper, amount0, amount1),
                e2
            )
        )
    });
}

#[test]
fn sqrt_price_from_usd() {
    check("sqrt_price_from_usd", |a, exp| {
        one(health::sqrt_price_x96_from_usd(a.u256(), a.u256(), a.u8(), a.u8()), exp)
    });
}

#[test]
fn evaluate() {
    check("evaluate", |a, exp| {
        let collateral = a.collateral();
        let penalty = a.u256();
        let debts = a.debts();
        fmt_res(health::evaluate(&collateral, &debts, penalty), exp, |h| {
            join(&[
                &h.min_surplus_usd_wad,
                &h.health_wad,
                &h.collateral_usd_wad,
                &h.debt_usd_wad,
            ])
        })
    });
}

#[test]
fn evaluate_holdings() {
    check("evaluate_holdings", |a, exp| {
        let (token0, token1) = (a.token(), a.token());
        let mut holdings = health::Holdings::default();
        for i in 0..health::PRICES {
            (holdings.amounts0[i], holdings.amounts1[i]) = (a.u256(), a.u256());
        }
        let debts = a.debts();
        let penalty = a.u256();
        let (exp_prices, exp_res) = exp.split_once(" | ").expect("evaluate_holdings separator");
        let prices = fmt_res(health::sqrt_prices(&token0, &token1), exp_prices, |prices| {
            prices.iter().map(ToString::to_string).collect::<Vec<_>>().join(" ")
        });
        let result = fmt_res(
            health::evaluate_holdings(&token0, &token1, &holdings, &debts, penalty),
            exp_res,
            |h| {
                join(&[
                    &h.min_surplus_usd_wad,
                    &h.health_wad,
                    &h.collateral_usd_wad,
                    &h.debt_usd_wad,
                ])
            },
        );
        format!("{prices} | {result}")
    });
}

#[test]
fn collateral_usd_at() {
    check("collateral_usd_at", |a, exp| {
        let (sqrt_pa_x96, sqrt_pb_x96, liquidity) = (a.u160(), a.u160(), a.u128());
        let token0 = TokenInput {
            price_wad: a.u256(),
            decimals: a.u8(),
            sigma_wad: U256::ZERO,
        };
        let token1 = TokenInput {
            price_wad: a.u256(),
            decimals: a.u8(),
            sigma_wad: U256::ZERO,
        };
        let collateral = CollateralInput {
            sqrt_pa_x96,
            sqrt_pb_x96,
            liquidity,
            token0,
            token1,
        };
        one(health::collateral_usd_at(&collateral, a.u256(), a.u256()), exp)
    });
}

#[test]
fn shares() {
    check("shares", |a, exp| {
        let (amount, total_assets, total_shares) = (a.u256(), a.u256(), a.u256());
        let rounding = if a.bool() { Rounding::Up } else { Rounding::Down };
        let mut parts = exp.split_whitespace();
        let (e1, e2) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        format!(
            "{} {}",
            one(market::to_shares(amount, total_assets, total_shares, rounding), e1),
            one(market::to_assets(amount, total_assets, total_shares, rounding), e2)
        )
    });
}

#[test]
fn exp_wad() {
    check("exp_wad", |a, exp| {
        let x: I256 = a.next().parse().expect("int256");
        one(irm::exp_wad(x), exp)
    });
}

#[test]
fn ln_wad() {
    check("ln_wad", |a, exp| {
        let x: I256 = a.next().parse().expect("int256");
        one(irm::ln_wad(x), exp)
    });
}

#[test]
fn adaptive_rate() {
    check("adaptive_rate", |a, exp| {
        let (rate_at_target, util, elapsed) = (a.u256(), a.u256(), a.u256());
        let res = irm::adaptive_rate(rate_at_target, util, elapsed).and_then(|r| {
            let exponent = r.integral.min(irm::MAX_GROWTH_EXPONENT);
            let growth = irm::exp_minus_one(I256::from_raw(exponent))?;
            Ok(join(&[&r.integral, &r.end_rate_at_target, &growth]))
        });
        fmt_res(res, exp, |s| s)
    });
}

#[test]
fn accrue() {
    check("accrue", |a, exp| {
        let mut m = Market {
            total_supply_assets: a.u128(),
            total_borrow_assets: a.u128(),
            total_borrow_shares: a.u128(),
            last_accrual: a.u64(),
            reserve_factor_wad: a.u64(),
            interest_remainder: a.u64(),
            rate_at_target: a.u64(),
        };
        let (supply_shares, now) = (a.u256(), a.u64());
        fmt_res(m.accrue(supply_shares, now), exp, |r| {
            join(&[
                &r.interest,
                &r.reserve_shares,
                &m.total_supply_assets,
                &m.total_borrow_assets,
                &m.last_accrual,
                &m.interest_remainder,
                &m.rate_at_target,
            ])
        })
    });
}

#[test]
fn market_rates() {
    check("market_rates", |a, exp| {
        let m = Market {
            total_supply_assets: a.u128(),
            total_borrow_assets: a.u128(),
            reserve_factor_wad: a.u64(),
            rate_at_target: a.u64(),
            ..Market::default()
        };
        let rates = m
            .utilization()
            .and_then(|u| m.borrow_rate().and_then(|b| m.supply_rate(b).map(|s| (u, b, s))));
        fmt_res(rates, exp, |(u, b, s)| format!("{u} {b} {s}"))
    });
}

#[test]
fn borrow() {
    check("borrow", |a, exp| {
        let mut m = Market {
            total_supply_assets: a.u128(),
            total_borrow_assets: a.u128(),
            total_borrow_shares: a.u128(),
            ..Market::default()
        };
        let (assets, fee, supply_shares) = (a.u256(), a.u256(), a.u256());
        let res = m
            .borrow(assets, fee, supply_shares)
            .and_then(|b| market::position_shares(U256::ZERO, b.shares).map(|_| b));
        fmt_res(res, exp, |b| {
            format!(
                "{} {} {} {} {}",
                b.shares, b.fee_shares, m.total_supply_assets, m.total_borrow_assets, m.total_borrow_shares
            )
        })
    });
}

#[test]
fn repay() {
    check("repay", |a, exp| {
        let m = Market {
            total_borrow_assets: a.u128(),
            total_borrow_shares: a.u128(),
            ..Market::default()
        };
        let (debt_shares, assets) = (a.u256(), a.u256());
        fmt_res(m.repay_amounts(debt_shares, assets), exp, |r| {
            format!("{} {}", r.assets, r.shares)
        })
    });
}

#[test]
fn repay_book() {
    check("repay_book", |a, exp| {
        let m = Market {
            total_supply_assets: a.u128(),
            total_borrow_assets: a.u128(),
            total_borrow_shares: a.u128(),
            ..Market::default()
        };
        let (debt_shares, assets) = (a.u256(), a.u256());
        let res = (|| -> Result<String> {
            let r = m.repay_amounts(debt_shares, assets)?;
            let borrowed = U256::from(m.total_borrow_assets);
            let borrow_shares = U256::from(m.total_borrow_shares);
            let booked = if r.shares == borrow_shares {
                borrowed
            } else {
                market::to_assets(r.shares, borrowed, borrow_shares, Rounding::Down)?
            };
            let next_supply = U256::from(m.total_supply_assets)
                .checked_add(r.assets)
                .ok_or(Error::Overflow)?
                .checked_sub(booked)
                .ok_or(Error::Underflow)?;
            let next_supply = u128::try_from(next_supply).map_err(|_| Error::CastOverflow)?;
            let next_shares = m
                .total_borrow_shares
                .checked_sub(u128::try_from(r.shares).map_err(|_| Error::CastOverflow)?)
                .ok_or(Error::Underflow)?;
            let next_borrow = m
                .total_borrow_assets
                .checked_sub(u128::try_from(booked).map_err(|_| Error::CastOverflow)?)
                .ok_or(Error::Underflow)?;
            let left = debt_shares.checked_sub(r.shares).ok_or(Error::Underflow)?;
            Ok(format!("{} {next_supply} {next_borrow} {next_shares} {left}", r.assets))
        })();
        fmt_res(res, exp, |s| s)
    });
}

#[test]
fn write_off() {
    check("write_off", |a, exp| {
        let (supply, borrow, borrow_shares, debt_shares) = (a.u128(), a.u128(), a.u128(), a.u256());
        let res = (|| -> Result<String> {
            let mut loss = market::to_assets(debt_shares, U256::from(borrow), U256::from(borrow_shares), Rounding::Up)?;
            if debt_shares == U256::from(borrow_shares) || loss > U256::from(borrow) {
                loss = U256::from(borrow);
            }
            let next_shares = borrow_shares
                .checked_sub(u128::try_from(debt_shares).map_err(|_| Error::CastOverflow)?)
                .ok_or(Error::Underflow)?;
            let loss_u128 = u128::try_from(loss).map_err(|_| Error::CastOverflow)?;
            let next_borrow = borrow.checked_sub(loss_u128).ok_or(Error::Underflow)?;
            let next_supply = supply.saturating_sub(loss_u128);
            Ok(format!("{next_supply} {next_borrow} {next_shares} 0"))
        })();
        fmt_res(res, exp, |s| s)
    });
}

#[test]
fn reward_ledger() {
    const Q128: U256 = U256::from_limbs([0, 0, 1, 0]);
    check("reward_ledger", |a, exp| {
        let n = a.usize();
        let mut growth = [U256::ZERO; 3];
        let mut pos_rewards = [U256::ZERO; 3];
        let mut held = U256::ZERO;
        let mut cp_growth = [[U256::ZERO; 3]; 4];
        let mut cp_owed = [[U256::ZERO; 3]; 4];
        let mut exp_tokens = exp.split_whitespace();
        let mut out = Vec::new();
        let mut failed = false;

        for _ in 0..n {
            let kind = a.usize();
            let (p1, p2, p3, p4, p5, p6) = (a.u256(), a.u256(), a.u256(), a.u256(), a.u256(), a.u256());
            if failed {
                continue;
            }
            let exp_tok = exp_tokens.next().unwrap_or("");
            let step: Result<Option<U256>> = (|| match kind {
                0 => {
                    let lp = usize::try_from(p1).expect("lp");
                    if !p3.is_zero() && !p2.is_zero() {
                        let add = uniswap::mul_div(p2, Q128, p3)?;
                        growth[lp] = growth[lp].checked_add(add).ok_or(Error::Overflow)?;
                        pos_rewards[lp] = pos_rewards[lp].checked_add(p2).ok_or(Error::Overflow)?;
                    }
                    Ok(None)
                }
                1 => {
                    if !p1.is_zero() {
                        held = held.checked_add(p1).ok_or(Error::Overflow)?;
                    }
                    Ok(None)
                }
                2 => {
                    let acc = usize::try_from(p1).expect("acc");
                    let lp = usize::try_from(p2).expect("lp");
                    let g = growth[lp];
                    let diff = g.checked_sub(cp_growth[acc][lp]).ok_or(Error::Underflow)?;
                    let earned = uniswap::mul_div(p3, diff, Q128)?;
                    cp_owed[acc][lp] = cp_owed[acc][lp].checked_add(earned).ok_or(Error::Overflow)?;
                    cp_growth[acc][lp] = g;
                    Ok(None)
                }
                3 => {
                    let acc = usize::try_from(p1).expect("acc");
                    let lp = usize::try_from(p2).expect("lp");
                    let available = pos_rewards[lp].min(held);
                    let amt = cp_owed[acc][lp].min(available);
                    if !amt.is_zero() {
                        cp_owed[acc][lp] -= amt;
                        pos_rewards[lp] -= amt;
                        held -= amt;
                    }
                    Ok(Some(amt))
                }
                4 => {
                    let from = usize::try_from(p1).expect("from");
                    let to = usize::try_from(p2).expect("to");
                    let lp = usize::try_from(p3).expect("lp");
                    let (from_bal, to_bal, units) = (p4, p5, p6);
                    if from != to && !units.is_zero() {
                        cp_growth[to][lp] =
                            fees::merge_mark(cp_growth[to][lp], cp_growth[from][lp], to_bal, units, growth[lp])?;
                        let owed = cp_owed[from][lp];
                        let moved = if from_bal.is_zero() || owed.is_zero() {
                            U256::ZERO
                        } else if units >= from_bal {
                            owed
                        } else {
                            uniswap::mul_div(owed, units, from_bal)?
                        };
                        cp_owed[from][lp] -= moved;
                        cp_owed[to][lp] = cp_owed[to][lp].checked_add(moved).ok_or(Error::Overflow)?;
                    }
                    Ok(None)
                }
                _ => panic!("unknown reward_ledger step {kind}"),
            })();

            match step {
                Ok(None) => out.push("-".to_owned()),
                Ok(Some(paid)) => out.push(paid.to_string()),
                Err(err) => {
                    out.push(match_err(&err, exp_tok));
                    failed = true;
                }
            }
        }
        if !failed {
            out.push("|".to_owned());
            for lp in 1..=2 {
                out.push(growth[lp].to_string());
                out.push(pos_rewards[lp].to_string());
            }
            out.push(held.to_string());
            for acc in 1..=3 {
                for lp in 1..=2 {
                    out.push(cp_growth[acc][lp].to_string());
                    out.push(cp_owed[acc][lp].to_string());
                }
            }
        }
        out.join(" ")
    });
}

#[test]
fn merge_mark() {
    check("merge_mark", |a, exp| {
        let (held_mark, moved_mark, held, units, growth) = (a.u256(), a.u256(), a.u256(), a.u256(), a.u256());
        one(fees::merge_mark(held_mark, moved_mark, held, units, growth), exp)
    });
}

#[test]
fn fees() {
    check("fees", |a, exp| {
        let cp = FeeCheckpoint {
            fee_growth_inside0_x128: a.u256(),
            fee_growth_inside1_x128: a.u256(),
            owed0: a.u128(),
            owed1: a.u128(),
        };
        let (balance, fg0, fg1) = (a.u256(), a.u256(), a.u256());
        fmt_res(cp.pending(balance, fg0, fg1), exp, |(x, y)| format!("{x} {y}"))
    });
}

#[test]
fn fee_checkpoint() {
    check("fee_checkpoint", |a, exp| {
        let mut cp = FeeCheckpoint {
            fee_growth_inside0_x128: a.u256(),
            fee_growth_inside1_x128: a.u256(),
            owed0: a.u128(),
            owed1: a.u128(),
        };
        let (balance, fg0, fg1) = (a.u256(), a.u256(), a.u256());
        fmt_res(cp.checkpoint(balance, fg0, fg1), exp, |()| {
            format!(
                "{} {} {} {}",
                cp.fee_growth_inside0_x128, cp.fee_growth_inside1_x128, cp.owed0, cp.owed1
            )
        })
    });
}

#[test]
fn fee_cut() {
    check("fee_cut", |a, exp| {
        let (amount, cut_wad) = (a.u256(), a.u256());
        let res = fees::cut(amount, cut_wad).and_then(|cut| fees::after_cut(amount, cut_wad).map(|paid| (cut, paid)));
        fmt_res(res, exp, |(cut, paid)| format!("{cut} {paid}"))
    });
}

fn debt_inputs(collateral: &CollateralInput, borrows: &[(Asset, U256)]) -> Result<Vec<DebtInput>> {
    borrows
        .iter()
        .map(|(asset, amount)| collateral.debt(asset, *amount))
        .collect()
}

#[test]
fn liquidate() {
    check("liquidate", |a, exp| {
        let collateral = a.collateral();
        let other = a.token();
        let penalties = a.penalties();
        let borrows = a.borrows(other);
        let penalty = position_penalty(penalties, borrows.iter().map(|(asset, _)| asset));
        let units = a.u256();
        let repay_asset = a.asset(other);
        let repay_penalty = position_penalty(penalties, [&repay_asset]);
        let max_repay = a.u256();
        let fee_wad = a.u256();

        let result = (|| {
            let debts = debt_inputs(&collateral, &borrows)?;
            let health = health::evaluate(&collateral, &debts, penalty)?;
            let token = match repay_asset {
                Asset::Token0 => collateral.token0,
                Asset::Token1 => collateral.token1,
                Asset::Other(token) => token,
            };
            let debt_assets = borrows
                .iter()
                .filter(|(asset, _)| *asset == repay_asset)
                .map(|(_, amount)| *amount)
                .sum();
            let has_other_debt = borrows
                .iter()
                .any(|(asset, amount)| *asset != repay_asset && !amount.is_zero());
            let debt = RepaidDebt {
                debt_assets,
                price_wad: token.price_wad,
                decimals: token.decimals,
            };
            liquidation::liquidate(&health, &debt, units, max_repay, repay_penalty, fee_wad, has_other_debt)
        })();
        fmt_res(result, exp, |r| {
            format!("{} {} {}", r.repay_assets, r.seized_units, r.fee_assets)
        })
    });
}

#[test]
fn seize() {
    check("seize", |a, exp| {
        let collateral_usd = a.u256();
        let debt_usd = a.u256();
        let debt_assets = a.u256();
        let price_wad = a.u256();
        let decimals = a.u8();
        let penalty_wad = a.u256();
        let max_repay = a.u256();
        let units = a.u256();

        let health = senzu_sdk_core::HealthResult {
            min_surplus_usd_wad: I256::ZERO,
            health_wad: U256::ZERO,
            collateral_usd_wad: collateral_usd,
            debt_usd_wad: debt_usd,
        };
        let debt = RepaidDebt {
            debt_assets,
            price_wad,
            decimals,
        };
        fmt_res(
            liquidation::seize_for(&health, &debt, max_repay, units, penalty_wad, U256::ZERO, false),
            exp,
            |r| format!("{} {}", r.seized_units, r.repay_assets),
        )
    });
}

#[test]
fn seize_fee() {
    check("seize_fee", |a, exp| {
        let collateral_usd = a.u256();
        let debt_usd = a.u256();
        let debt_assets = a.u256();
        let price_wad = a.u256();
        let decimals = a.u8();
        let penalty_wad = a.u256();
        let max_repay = a.u256();
        let units = a.u256();
        let fee_wad = a.u256();
        let has_other_debt = a.bool();

        let health = senzu_sdk_core::HealthResult {
            min_surplus_usd_wad: I256::ZERO,
            health_wad: U256::ZERO,
            collateral_usd_wad: collateral_usd,
            debt_usd_wad: debt_usd,
        };
        let debt = RepaidDebt {
            debt_assets,
            price_wad,
            decimals,
        };
        fmt_res(
            liquidation::seize_for(&health, &debt, max_repay, units, penalty_wad, fee_wad, has_other_debt),
            exp,
            |r| format!("{} {} {}", r.seized_units, r.repay_assets, r.fee_assets),
        )
    });
}

#[test]
fn swap_for_range() {
    check("swap_for_range", |a, exp| {
        let (lower, upper, amount0, amount1, price, fee) = (a.u160(), a.u160(), a.u256(), a.u256(), a.u160(), a.u256());
        let (p0, d0, p1, d1) = (a.u256(), a.u8(), a.u256(), a.u8());
        let result = (|| {
            let price = if price.is_zero() {
                health::sqrt_price_x96_from_usd(p0, p1, d0, d1)?
            } else {
                price
            };
            lens::swap_for_range(lower, upper, amount0, amount1, price, fee)
        })();
        fmt_res(result, exp, |q| {
            format!("{} {} {}", u8::from(q.zero_for_one), q.amount_in, q.amount_out)
        })
    });
}

#[test]
fn liquidation_prices() {
    check("liquidation_prices", |a, exp| {
        let collateral = a.collateral();
        let penalty = a.u256();
        let debts = a.debts();
        fmt_res(lens::liquidation_prices(&collateral, &debts, penalty), exp, |lp| {
            format!(
                "{} {} {}",
                u8::from(lp.moving == senzu_sdk_core::Side::Token1),
                lp.lower_wad.token(),
                lp.upper_wad.token()
            )
        })
    });
}

#[test]
fn max_leverage() {
    check("max_leverage", |a, exp| {
        let collateral = a.collateral();
        let other = a.token();
        let penalties = a.penalties();
        let fee = a.u256();
        let borrow = match a.u8() {
            0 => LeverageBorrow::PairRatio,
            1 => LeverageBorrow::Single(Asset::Token0),
            2 => LeverageBorrow::Single(Asset::Token1),
            _ => LeverageBorrow::Single(Asset::Other(other)),
        };
        let penalty = match &borrow {
            LeverageBorrow::Single(asset) => position_penalty(penalties, [asset]),
            LeverageBorrow::PairRatio => position_penalty(penalties, []),
        };
        one(lens::max_leverage(&collateral, &borrow, penalty, fee), exp)
    });
}

#[test]
fn max_borrow() {
    check("max_borrow", |a, exp| {
        let collateral = a.collateral();
        let other = a.token();
        let penalties = a.penalties();
        let fee = a.u256();
        let borrows = a.borrows(other);
        let asset = a.asset(other);
        let penalty = position_penalty(penalties, borrows.iter().map(|(asset, _)| asset).chain([&asset]));
        let market = Market {
            total_supply_assets: a.u128(),
            total_borrow_assets: a.u128(),
            total_borrow_shares: a.u128(),
            ..Market::default()
        };
        let account_shares = a.u256();
        let others: Vec<_> = borrows.into_iter().filter(|(borrowed, _)| *borrowed != asset).collect();
        one(
            (|| {
                let debts = debt_inputs(&collateral, &others)?;
                lens::max_borrow(&collateral, &debts, &asset, &market, account_shares, penalty, fee)
            })(),
            exp,
        )
    });
}

#[test]
fn preview_open() {
    check("preview_open", |a, exp| {
        let collateral = a.collateral();
        let other = a.token();
        let penalties = a.penalties();
        let borrow_fee_wad = a.u256();
        let (deposit0, deposit1, swap) = (a.u256(), a.u256(), a.bool());
        let amounts = a.borrows(other);
        let penalty = position_penalty(
            penalties,
            amounts
                .iter()
                .filter(|(_, amount)| !amount.is_zero())
                .map(|(asset, _)| asset),
        );
        let market = Market {
            total_supply_assets: u128::MAX,
            total_borrow_assets: a.u128(),
            total_borrow_shares: a.u128(),
            ..Market::default()
        };
        let borrows = amounts
            .into_iter()
            .map(|(asset, amount)| OpenBorrow { asset, amount, market })
            .collect();
        let open = OpenParams {
            collateral,
            deposit0,
            deposit1,
            borrows,
            swap,
            penalty_wad: penalty,
            borrow_fee_wad,
        };
        let whole_res = lens::preview_open(&open);
        if let Ok(whole) = &whole_res {
            let summary = lens::preview_open_summary(&open).expect("summary of a valid open");
            assert_eq!(
                (
                    summary.liquidity,
                    summary.amount0,
                    summary.amount1,
                    summary.health_wad,
                    summary.allowed
                ),
                (
                    whole.liquidity,
                    whole.amount0,
                    whole.amount1,
                    whole.health_wad,
                    whole.allowed
                )
            );
            assert_eq!(
                (
                    summary.collateral_usd_wad,
                    summary.margin_usd_wad,
                    summary.debt_usd_wad,
                    summary.leverage_wad
                ),
                (
                    whole.collateral_usd_wad,
                    whole.margin_usd_wad,
                    whole.debt_usd_wad,
                    whole.leverage_wad
                )
            );
            assert_eq!(
                lens::open_liquidation_prices(&open).expect("prices of a valid open"),
                whole.liquidation
            );
        } else {
            if let Err(err) = lens::preview_open_summary(&open) {
                assert!(
                    err_matches(&err, exp),
                    "preview_open_summary error {err:?} does not match {exp}"
                );
            }
            let liq_res = lens::open_liquidation_prices(&open);
            assert!(
                liq_res.as_ref().is_err_and(|err| err_matches(err, exp)),
                "open_liquidation_prices got {liq_res:?}, expected {exp}"
            );
        }
        fmt_res(whole_res, exp, |p| {
            [
                p.liquidity.token(),
                p.amount0.token(),
                p.amount1.token(),
                p.collateral_usd_wad.token(),
                p.margin_usd_wad.token(),
                p.debt_usd_wad.token(),
                p.leverage_wad.token(),
                p.health_wad.token(),
                p.allowed.token(),
                u8::from(p.liquidation.moving == senzu_sdk_core::Side::Token1).to_string(),
                p.liquidation.lower_wad.token(),
                p.liquidation.upper_wad.token(),
            ]
            .join(" ")
        })
    });
}
