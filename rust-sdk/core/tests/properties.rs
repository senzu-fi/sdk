//! Properties that hold for any input, on top of the exact vectors: rounding always favors the protocol, searches
//! return what they claim, and liquidations never take more than the borrower holds.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use alloy_primitives::{U160, U256};
use proptest::prelude::*;
use senzu_sdk_core::lens::{self, LeverageBorrow};
use senzu_sdk_core::liquidation::{self, RepaidDebt};
use senzu_sdk_core::market::{self, Rounding};
use senzu_sdk_core::uniswap::{self, MAX_TICK, MIN_TICK};
use senzu_sdk_core::{Asset, CollateralInput, DebtTie, Market, Side, TokenInput, WAD, health, irm};

const PENALTY: U256 = U256::from_limbs([1_050_000_000_000_000_000, 0, 0, 0]);

fn wad(x: u64) -> U256 {
    U256::from(x) * U256::from(10_u64.pow(9))
}

prop_compose! {
    fn token()(price_gwei in 1_000_000_u64..100_000_000_000_000, eighteen in any::<bool>(), sigma in 0_u64..1_000_000_000) -> TokenInput {
        TokenInput { price_wad: wad(price_gwei), decimals: if eighteen { 18 } else { 6 }, sigma_wad: WAD + wad(sigma) }
    }
}

prop_compose! {
    fn collateral()(token0 in token(), token1 in token(), a in MIN_TICK..MAX_TICK, b in MIN_TICK..MAX_TICK, liquidity in 1_000_000_u128..1_000_000_000_000_000_000_000) -> CollateralInput {
        CollateralInput {
            sqrt_pa_x96: uniswap::sqrt_price_at_tick(a.min(b)).expect("valid tick"),
            sqrt_pb_x96: uniswap::sqrt_price_at_tick(a.max(b).max(a.min(b) + 1)).expect("valid tick"),
            liquidity,
            token0,
            token1,
        }
    }
}

fn value(c: &CollateralInput) -> U256 {
    health::collateral_usd_at(c, c.token0.price_wad, c.token1.price_wad).expect("value")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn ticks_round_trip(tick in MIN_TICK..MAX_TICK) {
        let sqrt_price = uniswap::sqrt_price_at_tick(tick).expect("valid tick");
        prop_assert_eq!(uniswap::tick_at_sqrt_price(sqrt_price), Ok(tick));
        prop_assert!(uniswap::sqrt_price_at_tick(tick + 1).expect("valid tick") > sqrt_price);
    }

    #[test]
    fn rounding_up_adds_at_most_one(a in any::<u128>(), b in any::<u128>(), d in 1_u128..) {
        let (a, b, d) = (U256::from(a), U256::from(b), U256::from(d));
        let down = uniswap::mul_div(a, b, d).expect("fits");
        let up = uniswap::mul_div_rounding_up(a, b, d).expect("fits");
        prop_assert!(up == down || up == down + U256::from(1));
    }

    #[test]
    fn shares_round_trip_never_favors_the_user(assets in any::<u128>(), total_assets in any::<u128>(), total_shares in any::<u128>()) {
        let (assets, total_assets, total_shares) = (U256::from(assets), U256::from(total_assets), U256::from(total_shares));
        let shares = market::to_shares(assets, total_assets, total_shares, Rounding::Down).expect("fits");
        prop_assert!(market::to_assets(shares, total_assets, total_shares, Rounding::Down).expect("fits") <= assets);
        let owed = market::to_shares(assets, total_assets, total_shares, Rounding::Up).expect("fits");
        prop_assert!(market::to_assets(owed, total_assets, total_shares, Rounding::Up).expect("fits") >= assets);
    }

    #[test]
    fn more_debt_never_improves_health(c in collateral(), pct in 0_u64..150, extra in 0_u64..50, side in 0_u8..3) {
        let tie = [DebtTie::Untied, DebtTie::Token0, DebtTie::Token1][usize::from(side)];
        let base = value(&c) * U256::from(pct) / U256::from(100);
        let debt = |v: U256| senzu_sdk_core::DebtInput { value_usd_wad: v, sigma_wad: wad(1_100_000_000), tie };
        let before = health::evaluate(&c, &[debt(base)], PENALTY).expect("health");
        let after = health::evaluate(&c, &[debt(base + base * U256::from(extra) / U256::from(100))], PENALTY).expect("health");
        prop_assert!(after.min_surplus_usd_wad <= before.min_surplus_usd_wad);
        prop_assert!(after.health_wad <= before.health_wad);
    }

    #[test]
    fn max_borrow_passes_the_borrow_check(c in collateral(), pct in 0_u64..60, asset in 0_u8..2) {
        let asset = if asset == 0 { Asset::Token0 } else { Asset::Token1 };
        let token = if matches!(asset, Asset::Token0) { c.token0 } else { c.token1 };
        let existing = value(&c) * U256::from(pct) / U256::from(100) * U256::from(10_u8).pow(U256::from(token.decimals)) / token.price_wad;
        let total = u128::try_from(existing).expect("u128");
        let market = Market {
            total_supply_assets: u128::MAX,
            total_borrow_assets: total,
            total_borrow_shares: total,
            ..Market::default()
        };
        let fee = wad(1_000_000);
        let amount = lens::max_borrow(&c, &[], &asset, &market, existing, PENALTY, fee).expect("search");
        if !amount.is_zero() {
            let booked = lens::booked_debt(&market, existing, amount, fee).expect("booked");
            prop_assert!(health::evaluate(&c, &[c.debt(&asset, booked).expect("debt")], PENALTY).expect("health").passes());
        }
    }

    #[test]
    fn max_leverage_is_at_least_one(c in collateral()) {
        let leverage = lens::max_leverage(&c, &LeverageBorrow::PairRatio, PENALTY, U256::ZERO).expect("search");
        prop_assert!(leverage >= WAD && leverage <= lens::MAX_LEVERAGE);
    }

    #[test]
    fn liquidation_prices_bracket_the_current_price(c in collateral(), pct in 0_u64..130) {
        let debts = vec![c.debt(&Asset::Token1, value(&c) * U256::from(pct) / U256::from(100) * U256::from(10_u8).pow(U256::from(c.token1.decimals)) / c.token1.price_wad).expect("debt")];
        let prices = lens::liquidation_prices(&c, &debts, PENALTY).expect("search");
        let current = c.token(prices.moving).price_wad;
        prop_assert_eq!(prices.moving, if c.token1.sigma_wad >= c.token0.sigma_wad { Side::Token1 } else { Side::Token0 });
        if let Some(lower) = prices.lower_wad { prop_assert!(lower <= current); }
        if let Some(upper) = prices.upper_wad { prop_assert!(upper >= current); }
    }

    #[test]
    fn liquidations_take_at_most_the_balance(
        c in collateral(),
        pct in 80_u64..300,
        units in 1_u64..,
        max_repay in any::<u128>(),
        fee_bps in 0_u64..=2000,
    ) {
        let amount = value(&c) * U256::from(pct) / U256::from(100) * U256::from(10_u8).pow(U256::from(c.token1.decimals)) / c.token1.price_wad;
        let debts = vec![c.debt(&Asset::Token1, amount).expect("debt")];
        let health = health::evaluate(&c, &debts, PENALTY).expect("health");
        let debt = RepaidDebt { debt_assets: amount, price_wad: c.token1.price_wad, decimals: c.token1.decimals };
        let fee_wad = U256::from(fee_bps) * U256::from(10_u64.pow(14));
        if let Ok(l) = liquidation::liquidate(&health, &debt, U256::from(units), U256::from(max_repay), PENALTY, fee_wad, false) {
            prop_assert!(l.seized_units <= U256::from(units));
            prop_assert!(l.repay_assets <= U256::from(max_repay) && l.repay_assets <= amount);
            if l.seized_units == U256::from(units) && l.repay_assets < amount {
                prop_assert!(l.fee_assets.is_zero());
            }
            if uniswap::mul_div(health.collateral_usd_wad, WAD, liquidation::MIN_INCENTIVE_WAD).expect("floor") < health.debt_usd_wad {
                prop_assert!(l.fee_assets.is_zero());
            }
        }
    }

    #[test]
    fn accrual_keeps_cash_and_moves_time(supply in 1_u128..1_000_000_000_000_000_000_000_000, borrowed_pct in 0_u128..100, elapsed in 0_u64..100_000_000) {
        let mut m = Market {
            total_supply_assets: supply,
            total_borrow_assets: supply / 100 * borrowed_pct,
            total_borrow_shares: supply / 100 * borrowed_pct * 1_000_000,
            last_accrual: 1_700_000_000,
            reserve_factor_wad: 100_000_000_000_000_000,
            interest_remainder: 0,
            rate_at_target: u64::try_from(irm::INITIAL_RATE_AT_TARGET).expect("u64"),
        };
        let cash = m.cash().expect("cash");
        let accrual = m.accrue(U256::from(supply) * U256::from(1_000_000), 1_700_000_000 + elapsed).expect("accrue");
        prop_assert_eq!(m.cash().expect("cash"), cash);
        prop_assert_eq!(m.last_accrual, 1_700_000_000 + elapsed);
        prop_assert_eq!(U256::from(m.total_borrow_assets), U256::from(supply / 100 * borrowed_pct) + accrual.interest);
    }

    #[test]
    fn swap_never_sells_more_than_it_has(a in MIN_TICK..0, b in 1..MAX_TICK, p in MIN_TICK..MAX_TICK, x in any::<u64>(), y in any::<u64>()) {
        let (lower, upper, price) = (
            uniswap::sqrt_price_at_tick(a).expect("tick"),
            uniswap::sqrt_price_at_tick(b).expect("tick"),
            uniswap::sqrt_price_at_tick(p).expect("tick"),
        );
        let quote = lens::swap_for_range(lower, upper, U256::from(x), U256::from(y), price, wad(3_000_000)).expect("quote");
        let available = if quote.zero_for_one { U256::from(x) } else { U256::from(y) };
        prop_assert!(quote.amount_in <= available);
    }
}

#[test]
fn full_range_eth_usdc_example_passes() {
    let eth = TokenInput {
        price_wad: wad(2_000_000_000_000),
        decimals: 18,
        sigma_wad: wad(1_150_000_000),
    };
    let usdc = TokenInput {
        price_wad: WAD,
        decimals: 6,
        sigma_wad: wad(1_020_000_000),
    };
    let c = CollateralInput {
        sqrt_pa_x96: uniswap::MIN_SQRT_PRICE,
        sqrt_pb_x96: uniswap::MAX_SQRT_PRICE,
        liquidity: 100_000_000_000_000,
        token0: eth,
        token1: usdc,
    };
    let _: U160 = c.sqrt_pa_x96;
    let debt = c.debt(&Asset::Token1, U256::from(100_000_000_u64)).expect("debt");
    let result = health::evaluate(&c, &[debt], PENALTY).expect("health");
    assert!(result.passes());
    assert!(result.health_wad > WAD);
}
