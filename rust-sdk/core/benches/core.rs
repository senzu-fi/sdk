//! Criterion benchmarks for `senzu-sdk-core`, driven by realistic inputs from `tests/vectors/`.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::cast_precision_loss,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    missing_docs
)]

use std::hint::black_box;
use std::path::Path;

use alloy_primitives::{U160, U256};
use criterion::{Criterion, criterion_group, criterion_main};
use senzu_sdk_core::flows::close::{self, ClosingDebt, ClosingPosition};
use senzu_sdk_core::flows::edit::{self, EditPosition, EditState};
use senzu_sdk_core::flows::{PerToken, Priced};
use senzu_sdk_core::lens::{self, LeverageBorrow};
use senzu_sdk_core::metrics::liquidity::{Segment, book_amounts, segment_sqrts, sqrt_at_tick};
use senzu_sdk_core::metrics::payoff::{PayoffInputs, pnl_curve};
use senzu_sdk_core::{Asset, CollateralInput, DebtInput, DebtTie, Market, Side, TokenInput, health, uniswap};

struct Args<'a>(std::str::SplitWhitespace<'a>);

impl Args<'_> {
    fn next(&mut self) -> &str {
        self.0.next().expect("vector field")
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
    fn u8(&mut self) -> u8 {
        self.next().parse().expect("uint8")
    }
    fn bool(&mut self) -> bool {
        self.next() == "1"
    }
    fn usize(&mut self) -> usize {
        self.next().parse().expect("usize")
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
            _ => Asset::Other(other),
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
    fn penalties(&mut self) -> [U256; 3] {
        [self.u256(), self.u256(), self.u256()]
    }
}

fn position_penalty<'a>(penalties: [U256; 3], assets: impl IntoIterator<Item = &'a Asset>) -> U256 {
    let outside: Vec<U256> = assets
        .into_iter()
        .filter(|asset| matches!(asset, Asset::Other(_)))
        .map(|_| penalties[2])
        .collect();
    health::position_penalty(penalties[0], penalties[1], &outside)
}

fn vector_lines(name: &str) -> Vec<(String, String)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/vectors")
        .join(format!("{name}.txt"));
    let text = std::fs::read_to_string(&path).expect("vector file");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| {
            let (i, o) = l.split_once(" => ").expect("=>");
            (i.to_owned(), o.trim().to_owned())
        })
        .collect()
}

fn load_evaluate_cases(limit: usize) -> Vec<(CollateralInput, Vec<DebtInput>, U256)> {
    vector_lines("evaluate")
        .into_iter()
        .filter(|(_, out)| !out.starts_with("err"))
        .filter_map(|(inp, _)| {
            let mut a = Args(inp.split_whitespace());
            let collateral = a.collateral();
            let penalty = a.u256();
            let debts = a.debts();
            if collateral.liquidity > 0 && !debts.is_empty() {
                Some((collateral, debts, penalty))
            } else {
                None
            }
        })
        .take(limit)
        .collect()
}

fn load_liquidation_cases(limit: usize) -> Vec<(CollateralInput, Vec<DebtInput>, U256)> {
    vector_lines("evaluate")
        .into_iter()
        .filter(|(_, out)| !out.starts_with("err") && !out.starts_with('-'))
        .filter_map(|(inp, _)| {
            let mut a = Args(inp.split_whitespace());
            let collateral = a.collateral();
            let penalty = a.u256();
            let debts = a.debts();
            if collateral.liquidity > 0 && !debts.is_empty() {
                Some((collateral, debts, penalty))
            } else {
                None
            }
        })
        .take(limit)
        .collect()
}

type MaxBorrowCase = (CollateralInput, Vec<DebtInput>, Asset, Market, U256, U256, U256);

fn load_max_borrow_cases(limit: usize) -> Vec<MaxBorrowCase> {
    vector_lines("max_borrow")
        .into_iter()
        .filter(|(_, out)| !out.starts_with("err") && out != "0")
        .filter_map(|(inp, _)| {
            let mut a = Args(inp.split_whitespace());
            let collateral = a.collateral();
            let other = a.token();
            let penalties = a.penalties();
            let fee = a.u256();
            let borrows = a.borrows(other);
            let asset = a.asset(other);
            let penalty = position_penalty(penalties, borrows.iter().map(|(x, _)| x).chain([&asset]));
            let market = Market {
                total_supply_assets: a.u128(),
                total_borrow_assets: a.u128(),
                total_borrow_shares: a.u128(),
                ..Market::default()
            };
            let account_shares = a.u256();
            let others: Vec<_> = borrows.into_iter().filter(|(b, _)| *b != asset).collect();
            let debts: Vec<DebtInput> = others
                .iter()
                .map(|(x, amt)| collateral.debt(x, *amt))
                .collect::<senzu_sdk_core::Result<_>>()
                .ok()?;
            Some((collateral, debts, asset, market, account_shares, penalty, fee))
        })
        .take(limit)
        .collect()
}

fn load_max_leverage_cases(limit: usize) -> Vec<(CollateralInput, LeverageBorrow, U256, U256)> {
    vector_lines("max_leverage")
        .into_iter()
        .filter(|(_, out)| !out.starts_with("err"))
        .map(|(inp, _)| {
            let mut a = Args(inp.split_whitespace());
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
            (collateral, borrow, penalty, fee)
        })
        .take(limit)
        .collect()
}

fn load_mul_div_cases(limit: usize) -> Vec<(U256, U256, U256)> {
    vector_lines("mul_div")
        .into_iter()
        .filter(|(_, out)| !out.contains("err"))
        .map(|(inp, _)| {
            let mut a = Args(inp.split_whitespace());
            (a.u256(), a.u256(), a.u256())
        })
        .take(limit)
        .collect()
}

fn load_amount1_cases(limit: usize) -> Vec<(U160, U160, u128, bool)> {
    vector_lines("amount_delta")
        .into_iter()
        .filter(|(_, out)| !out.contains("err"))
        .map(|(inp, _)| {
            let mut a = Args(inp.split_whitespace());
            (a.u160(), a.u160(), a.u128(), a.bool())
        })
        .take(limit)
        .collect()
}

fn load_swap_cases(limit: usize) -> Vec<(U160, U160, U256, U256, U160, U256)> {
    vector_lines("swap_for_range")
        .into_iter()
        .filter(|(_, out)| !out.starts_with("err"))
        .filter_map(|(inp, _)| {
            let mut a = Args(inp.split_whitespace());
            let (sa, sb, x, y, sp, fee) = (a.u160(), a.u160(), a.u256(), a.u256(), a.u160(), a.u256());
            let (p0, d0, p1, d1) = (a.u256(), a.u8(), a.u256(), a.u8());
            let price = if sp.is_zero() {
                health::sqrt_price_x96_from_usd(p0, p1, d0, d1).ok()?
            } else {
                sp
            };
            Some((sa, sb, x, y, price, fee))
        })
        .take(limit)
        .collect()
}

fn make_edit_cases(liq_cases: &[(CollateralInput, Vec<DebtInput>, U256)]) -> Vec<(EditPosition, EditState)> {
    liq_cases
        .iter()
        .take(10)
        .map(|(coll, debts, pen)| {
            let oracle = health::sqrt_price_x96_from_usd(
                coll.token0.price_wad,
                coll.token1.price_wad,
                coll.token0.decimals,
                coll.token1.decimals,
            )
            .expect("oracle");
            let debt_usd: U256 = debts
                .iter()
                .map(|d| d.value_usd_wad)
                .fold(U256::ZERO, U256::saturating_add);
            let debt1 = lens::amount_for_usd(debt_usd / U256::from(2_u8), coll.token1.price_wad, coll.token1.decimals)
                .expect("debt1");
            (
                EditPosition {
                    sqrt_pa_x96: coll.sqrt_pa_x96,
                    sqrt_pb_x96: coll.sqrt_pb_x96,
                    tokens: PerToken {
                        token0: coll.token0,
                        token1: coll.token1,
                    },
                    oracle_sqrt_price_x96: oracle,
                    position_penalty_wad: *pen,
                    pair_penalty_wad: *pen,
                    other_debts: Vec::new(),
                },
                EditState {
                    liquidity: coll.liquidity,
                    debt: PerToken {
                        token0: U256::ZERO,
                        token1: debt1,
                    },
                },
            )
        })
        .collect()
}

fn bench_lens(c: &mut Criterion) {
    let liq_cases = load_liquidation_cases(20);
    c.bench_function("liquidation_prices", |b| {
        b.iter(|| {
            for (coll, debts, pen) in &liq_cases {
                let _ = black_box(lens::liquidation_prices(black_box(coll), black_box(debts), *pen));
            }
        });
    });

    let mb_cases = load_max_borrow_cases(20);
    c.bench_function("max_borrow", |b| {
        b.iter(|| {
            for (coll, debts, asset, market, shares, pen, fee) in &mb_cases {
                let _ = black_box(lens::max_borrow(
                    black_box(coll),
                    black_box(debts),
                    asset,
                    market,
                    *shares,
                    *pen,
                    *fee,
                ));
            }
        });
    });

    let ml_cases = load_max_leverage_cases(20);
    c.bench_function("max_leverage", |b| {
        b.iter(|| {
            for (coll, borrow, pen, fee) in &ml_cases {
                let _ = black_box(lens::max_leverage(black_box(coll), borrow, *pen, *fee));
            }
        });
    });

    let edit_cases = make_edit_cases(&liq_cases);
    c.bench_function("max_removable_wad", |b| {
        b.iter(|| {
            for (pos, state) in &edit_cases {
                let _ = black_box(edit::max_removable_wad(black_box(pos), black_box(state)));
            }
        });
    });

    let close_cases: Vec<(ClosingPosition, Vec<ClosingDebt>, PerToken<Option<Priced>>)> = edit_cases
        .iter()
        .map(|(pos, state)| {
            let closing = ClosingPosition {
                sqrt_pa_x96: pos.sqrt_pa_x96,
                sqrt_pb_x96: pos.sqrt_pb_x96,
                liquidity: state.liquidity,
                pool_sqrt_price_x96: pos.oracle_sqrt_price_x96,
                fees_owed0: U256::ZERO,
                fees_owed1: U256::ZERO,
            };
            let debts = vec![ClosingDebt {
                side: Some(Side::Token1),
                assets: state.debt.token1,
                price: Some(Priced {
                    price_wad: pos.tokens.token1.price_wad,
                    decimals: pos.tokens.token1.decimals,
                }),
            }];
            let prices = PerToken {
                token0: Some(Priced {
                    price_wad: pos.tokens.token0.price_wad,
                    decimals: pos.tokens.token0.decimals,
                }),
                token1: Some(Priced {
                    price_wad: pos.tokens.token1.price_wad,
                    decimals: pos.tokens.token1.decimals,
                }),
            };
            (closing, debts, prices)
        })
        .collect();
    let swap_fee = U256::from(3_000_000_000_000_000_u64);
    c.bench_function("close_amounts", |b| {
        b.iter(|| {
            for (pos, debts, prices) in &close_cases {
                let _ = black_box(close::close_amounts(
                    black_box(pos),
                    black_box(debts),
                    *prices,
                    50,
                    swap_fee,
                ));
            }
        });
    });
}

fn bench_math_and_charts(c: &mut Criterion) {
    let eval_cases = load_evaluate_cases(50);
    c.bench_function("health_evaluate", |b| {
        b.iter(|| {
            for (coll, debts, pen) in &eval_cases {
                let _ = black_box(health::evaluate(black_box(coll), black_box(debts), *pen));
            }
        });
    });

    let md_cases = load_mul_div_cases(200);
    c.bench_function("mul_div", |b| {
        b.iter(|| {
            for &(x, y, d) in &md_cases {
                let _ = black_box(uniswap::mul_div(black_box(x), black_box(y), black_box(d)));
                let _ = black_box(uniswap::mul_div_rounding_up(black_box(x), black_box(y), black_box(d)));
            }
        });
    });

    let a1_cases = load_amount1_cases(200);
    c.bench_function("amount1_delta", |b| {
        b.iter(|| {
            for &(sa, sb, l, up) in &a1_cases {
                let _ = black_box(uniswap::amount1_delta(black_box(sa), black_box(sb), l, up));
            }
        });
    });

    let segs: Vec<Segment> = (0..200_i32)
        .map(|i| Segment {
            lower: -6000.0 + f64::from(i) * 60.0,
            upper: -6000.0 + f64::from(i + 1) * 60.0,
            liquidity: 1e18 * (1.0 + f64::from(i % 7)),
        })
        .collect();
    let sqrt_segs = segment_sqrts(&segs);
    let edges_up: Vec<f64> = (1..=40_i32).map(|i| sqrt_at_tick(f64::from(i) * 120.0)).collect();
    let edges_dn: Vec<f64> = (1..=40_i32).map(|i| sqrt_at_tick(-f64::from(i) * 120.0)).collect();
    c.bench_function("book_amounts", |b| {
        b.iter(|| {
            let _ = black_box(book_amounts(black_box(&sqrt_segs), 1.0, black_box(&edges_up), 18, 6));
            let _ = black_box(book_amounts(black_box(&sqrt_segs), 1.0, black_box(&edges_dn), 18, 6));
        });
    });

    let payoff = PayoffInputs {
        liquidity: U256::from(10_u128.pow(20)),
        sqrt_lower_x96: uniswap::sqrt_price_at_tick(-1_000).expect("tick"),
        sqrt_upper_x96: uniswap::sqrt_price_at_tick(1_000).expect("tick"),
        decimals0: 18,
        decimals1: 6,
        debt0: U256::from(5_u128 * 10_u128.pow(18)),
        debt1: U256::from(5_000_u128 * 10_u128.pow(6)),
        price: 1700.0,
        token1_usd: 1.0,
    };
    let prices: Vec<f64> = (0..161_i32).map(|i| 1200.0 + f64::from(i) * 6.0).collect();
    c.bench_function("pnl_curve", |b| {
        b.iter(|| {
            let _ = black_box(pnl_curve(black_box(&payoff), black_box(&prices)));
        });
    });

    let swap_cases = load_swap_cases(50);
    c.bench_function("swap_for_range", |b| {
        b.iter(|| {
            for &(sa, sb, x, y, sp, fee) in &swap_cases {
                let _ = black_box(lens::swap_for_range(sa, sb, x, y, sp, fee));
            }
        });
    });
}

criterion_group!(benches, bench_lens, bench_math_and_charts);
criterion_main!(benches);
