//! The adaptive interest rate curve of `MarketLogic` (Morpho's `AdaptiveCurveIrm` design) and Solady's `expWad`
//! and `lnWad`.
//!
//! A market's one rate variable is its rate at the 90% target utilization. It moves exponentially with the time
//! spent away from the target (doubling in ln 2 / 100 years at 100%, halving as fast at 0%), within
//! [`MIN_RATE_AT_TARGET`] and [`MAX_RATE_AT_TARGET`]; the curve sets the rate around it, from a quarter of it at 0%
//! utilization to four times it at 100%. A period charges the exact integral of that rate, compounded
//! continuously ([`adaptive_rate`], [`exp_minus_one`]), so the interest doesn't depend on how often anyone accrues.

use alloy_primitives::{I256, U256, uint};

use crate::error::{Error, Result};
use crate::num::{Checked, WAD};
use crate::uniswap::mul_div;

/// 365 days, the contracts' year for per-year ⇄ per-second rates.
pub const SECONDS_PER_YEAR: U256 = uint!(31_536_000_U256);
/// Utilization the curve is centered on, 1e18 = 100%.
pub const TARGET_UTILIZATION: U256 = uint!(900_000_000_000_000_000_U256);
/// The rate at 100% utilization over the rate at target, and the rate at target over the rate at 0%, WAD.
pub const CURVE_STEEPNESS: I256 = I256::from_raw(uint!(4_000_000_000_000_000_000_U256));
/// 100 a year, per second, WAD: at 100% utilization the rate at target grows `e^(100 · years)`.
pub const ADJUSTMENT_SPEED: I256 = I256::from_raw(uint!(3_170_979_198_376_U256));
/// A new market's rate at target, 8% a year, per second.
pub const INITIAL_RATE_AT_TARGET: U256 = uint!(2_536_783_358_U256);
/// Floor of the rate at target, 1% a year, per second.
pub const MIN_RATE_AT_TARGET: U256 = uint!(317_097_919_U256);
/// Ceiling of the rate at target, 37.5% a year, per second: no borrow rate passes 150% a year.
pub const MAX_RATE_AT_TARGET: U256 = uint!(11_891_171_993_U256);
/// Largest exponent the adaptation passes to [`exp_wad`], WAD (`e^4` is past `MAX / MIN`).
pub const MAX_EXPONENT: I256 = I256::from_raw(uint!(4_000_000_000_000_000_000_U256));
/// Most one accrual compounds, WAD: `e^15` ≈ 3.3 million times the debt, ten years at 150%.
pub const MAX_GROWTH_EXPONENT: U256 = uint!(15_000_000_000_000_000_000_U256);
/// Within ±0.05 (WAD) [`exp_minus_one`] sums its Taylor series; past it, it goes through [`exp_wad`].
pub const SERIES_LIMIT: I256 = I256::from_raw(uint!(50_000_000_000_000_000_U256));

const I_WAD: I256 = I256::from_raw(WAD);

const fn int(value: U256) -> I256 {
    I256::from_raw(value)
}

/// `SafeCastLib.toInt256`.
///
/// # Errors
///
/// [`Error::CastOverflow`] above `int256`'s maximum.
pub fn to_int(value: U256) -> Result<I256> {
    crate::num::to_i256(value)
}

/// `SafeCastLib.toUint256`.
///
/// # Errors
///
/// [`Error::CastOverflow`] below zero.
pub fn to_uint(value: I256) -> Result<U256> {
    if value.is_negative() {
        return Err(Error::CastOverflow);
    }
    Ok(value.into_raw())
}

/// `e^x` in WAD (Solady's `FixedPointMathLib.expWad`): 0 at or below `-41.45e18`.
///
/// # Errors
///
/// [`Error::ExpOverflow`] at or above `135.3e18`, where the result passes `int256`.
#[expect(
    clippy::many_single_char_names,
    reason = "Solady's names, to read side by side with it"
)]
pub fn exp_wad(x: I256) -> Result<I256> {
    if x <= int(uint!(41_446_531_673_892_822_313_U256)).wrapping_neg() {
        return Ok(I256::ZERO);
    }
    if x >= int(uint!(135_305_999_368_893_231_589_U256)) {
        return Err(Error::ExpOverflow);
    }

    // Solady's unchecked block: nothing below overflows for x in (-42, 136) · 1e18
    let mut x = x.wrapping_shl(78).wrapping_div(int(uint!(3_814_697_265_625_U256)));
    let ln2 = int(uint!(54_916_777_467_707_473_351_141_471_128_U256));
    let k = x
        .wrapping_shl(96)
        .wrapping_div(ln2)
        .wrapping_add(int(U256::from(1_u128 << 95)))
        .asr(96);
    x = x.wrapping_sub(k.wrapping_mul(ln2));

    let mut y = x.wrapping_add(int(uint!(1_346_386_616_545_796_478_920_950_773_328_U256)));
    y = y
        .wrapping_mul(x)
        .asr(96)
        .wrapping_add(int(uint!(57_155_421_227_552_351_082_224_309_758_442_U256)));
    let mut p = y
        .wrapping_add(x)
        .wrapping_sub(int(uint!(94_201_549_194_550_492_254_356_042_504_812_U256)));
    p = p
        .wrapping_mul(y)
        .asr(96)
        .wrapping_add(int(uint!(28_719_021_644_029_726_153_956_944_680_412_240_U256)));
    p = p
        .wrapping_mul(x)
        .wrapping_add(int(uint!(4_385_272_521_454_847_904_659_076_985_693_276_U256)).wrapping_shl(96));

    let mut q = x.wrapping_sub(int(uint!(2_855_989_394_907_223_263_936_484_059_900_U256)));
    q = q
        .wrapping_mul(x)
        .asr(96)
        .wrapping_add(int(uint!(50_020_603_652_535_783_019_961_831_881_945_U256)));
    q = q
        .wrapping_mul(x)
        .asr(96)
        .wrapping_sub(int(uint!(533_845_033_583_426_703_283_633_433_725_380_U256)));
    q = q
        .wrapping_mul(x)
        .asr(96)
        .wrapping_add(int(uint!(3_604_857_256_930_695_427_073_651_918_091_429_U256)));
    q = q
        .wrapping_mul(x)
        .asr(96)
        .wrapping_sub(int(uint!(14_423_608_567_350_463_180_887_372_962_807_573_U256)));
    q = q
        .wrapping_mul(x)
        .asr(96)
        .wrapping_add(int(uint!(26_449_188_498_355_588_339_934_803_723_976_023_U256)));

    // Solady's `sdiv`: q has no zeros in the domain (its roots are complex)
    let r = p.checked_div(q).ok_or(Error::DivisionByZero)?;
    let product = r.into_raw().wrapping_mul(uint!(
        3_822_833_074_963_236_453_042_738_258_902_158_003_155_416_615_667_U256
    ));
    let shift = int(uint!(195_U256)).wrapping_sub(k).into_raw();
    // EVM `shr` by 256 or more is 0, as ruint's `wrapping_shr`
    let shifted = usize::try_from(shift).map_or(U256::ZERO, |s| product.wrapping_shr(s));
    Ok(int(shifted))
}

/// `ln(x)` in WAD (Solady's `FixedPointMathLib.lnWad`).
///
/// # Errors
///
/// [`Error::LnWadUndefined`] for `x <= 0`.
#[expect(
    clippy::many_single_char_names,
    reason = "Solady's names, to read side by side with it"
)]
pub fn ln_wad(x: I256) -> Result<I256> {
    if x <= I256::ZERO {
        return Err(Error::LnWadUndefined);
    }
    let ux = x.into_raw();
    // `255 ^ floor(log2(ux)) == 255 - (255 - leading_zeros) == leading_zeros`
    let r = ux.leading_zeros();

    let x = int(ux.wrapping_shl(r).wrapping_shr(159));
    let sar = |v: I256| v.asr(96);
    let mut p = sar(x
        .wrapping_add(int(uint!(3_273_285_459_638_523_848_632_254_066_296_U256)))
        .wrapping_mul(x));
    p = sar(p
        .wrapping_add(int(uint!(24_828_157_081_833_163_892_658_089_445_524_U256)))
        .wrapping_mul(x));
    p = sar(p
        .wrapping_add(int(uint!(43_456_485_725_739_037_958_740_375_743_393_U256)))
        .wrapping_mul(x))
    .wrapping_sub(int(uint!(11_111_509_109_440_967_052_023_855_526_967_U256)));
    p = sar(p.wrapping_mul(x)).wrapping_sub(int(uint!(45_023_709_667_254_063_763_336_534_515_857_U256)));
    p = sar(p.wrapping_mul(x)).wrapping_sub(int(uint!(14_706_773_417_378_608_786_704_636_184_526_U256)));
    p = p
        .wrapping_mul(x)
        .wrapping_sub(int(uint!(795_164_235_651_350_426_258_249_787_498_U256).wrapping_shl(96)));

    let mut q = int(uint!(5_573_035_233_440_673_466_300_451_813_936_U256)).wrapping_add(x);
    for c in [
        uint!(71_694_874_799_317_883_764_090_561_454_958_U256),
        uint!(283_447_036_172_924_575_727_196_451_306_956_U256),
        uint!(401_686_690_394_027_663_651_624_208_769_553_U256),
        uint!(204_048_457_590_392_012_362_485_061_816_622_U256),
        uint!(31_853_899_698_501_571_402_653_359_427_138_U256),
        uint!(909_429_971_244_387_300_277_376_558_375_U256),
    ] {
        q = int(c).wrapping_add(sar(x.wrapping_mul(q)));
    }

    // Solady's `sdiv`: q has no zeros in the domain
    p = p.checked_div(q).ok_or(Error::DivisionByZero)?;
    p = int(uint!(1_677_202_110_996_718_588_342_820_967_067_443_963_516_166_U256)).wrapping_mul(p);
    let k = int(uint!(159_U256)).wrapping_sub(int(U256::from(r)));
    p = int(uint!(
        16_597_577_552_685_614_221_487_285_958_193_947_469_193_820_559_219_878_177_908_093_499_208_371_U256
    ))
    .wrapping_mul(k)
    .wrapping_add(p);
    p = int(uint!(
        600_920_179_829_731_861_736_702_779_321_621_459_595_472_258_049_074_101_567_377_883_020_018_308_U256
    ))
    .wrapping_add(p);
    Ok(p.asr(174))
}

/// Distance of `util` from the target, signed WAD: `-1e18` at 0%, `0` at 90%, `1e18` at 100%
/// (`MarketLogic.utilizationError`).
///
/// # Errors
///
/// [`Error::Overflow`] or [`Error::Underflow`] when the arithmetic leaves `int256` (never for a real market).
pub fn utilization_error(util: U256) -> Result<I256> {
    let norm = if util > TARGET_UTILIZATION {
        WAD.try_sub(TARGET_UTILIZATION)?
    } else {
        TARGET_UTILIZATION
    };
    to_int(util)?
        .try_sub(int(TARGET_UTILIZATION))?
        .try_mul(I_WAD)?
        .try_div(int(norm))
}

/// The curve's rate at `err` around `rate_at_target`, per second (`MarketLogic.curve`).
///
/// # Errors
///
/// [`Error::Overflow`] or [`Error::Underflow`] when the arithmetic leaves `int256`.
pub fn curve(rate_at_target: U256, err: I256) -> Result<U256> {
    let coefficient = if err.is_negative() {
        I_WAD.try_sub(I_WAD.try_mul(I_WAD)?.try_div(CURVE_STEEPNESS)?)?
    } else {
        CURVE_STEEPNESS.try_sub(I_WAD)?
    };
    let factor = coefficient.try_mul(err)?.try_div(I_WAD)?.try_add(I_WAD)?;
    to_uint(factor.try_mul(to_int(rate_at_target)?)?.try_div(I_WAD)?)
}

/// `rate_at_target · e^linear_adaptation`, the exponent clamped to [`MAX_EXPONENT`] and the result to the floor and
/// ceiling (`MarketLogic.adaptedRateAtTarget`).
///
/// # Errors
///
/// [`Error::Overflow`] when the product overflows.
pub fn adapted_rate_at_target(rate_at_target: U256, linear_adaptation: I256) -> Result<U256> {
    let x = linear_adaptation.clamp(MAX_EXPONENT.wrapping_neg(), MAX_EXPONENT);
    let adapted = rate_at_target.try_mul(to_uint(exp_wad(x)?)?)?.try_div(WAD)?;
    Ok(adapted.clamp(MIN_RATE_AT_TARGET, MAX_RATE_AT_TARGET))
}

/// `e^x − 1` in 1e36 (`MarketLogic.expMinusOne`): the Taylor series within ±[`SERIES_LIMIT`], each term rounded
/// toward zero, [`exp_wad`] past it.
///
/// # Errors
///
/// [`Error::ExpOverflow`] at or above `135.3e18`, and [`Error::Overflow`] when the result passes `int256`.
pub fn exp_minus_one(x: I256) -> Result<I256> {
    if x > SERIES_LIMIT || x < SERIES_LIMIT.wrapping_neg() {
        return exp_wad(x)?.try_sub(I_WAD)?.try_mul(I_WAD);
    }
    let mut term = x.try_mul(I_WAD)?;
    let mut sum = I256::ZERO;
    let mut n = int(uint!(2_U256));
    while !term.is_zero() {
        sum = sum.try_add(term)?;
        term = term.try_mul(x)?.try_div(n.try_mul(I_WAD)?)?;
        n = n.try_add(I256::ONE)?;
    }
    Ok(sum)
}

/// `∫ r dt` over the period, `r(t) = clamp(r₀ · e^(speed · t))`, in rate · seconds, WAD
/// (`MarketLogic.rateAtTargetArea`): `r₀ (e^(speed · T) − 1) / speed` unclamped, `B (T − τ) + (B − r₀) / speed` when
/// it reaches the bound `B` at `τ = ln(B / r₀) / speed`.
///
/// # Errors
///
/// The arithmetic errors of the contract (never for a real market).
pub fn rate_at_target_area(
    rate_at_target: U256,
    speed: I256,
    linear_adaptation: I256,
    end_rate_at_target: U256,
    elapsed: U256,
) -> Result<U256> {
    let start = rate_at_target.clamp(MIN_RATE_AT_TARGET, MAX_RATE_AT_TARGET);
    let bound = if speed.is_positive() {
        MAX_RATE_AT_TARGET
    } else {
        MIN_RATE_AT_TARGET
    };
    if start == bound {
        return bound.try_mul(elapsed);
    }
    let abs = |v: I256| -> Result<U256> {
        Ok(if v.is_positive() {
            v
        } else {
            v.checked_neg().ok_or(Error::Overflow)?
        }
        .into_raw())
    };
    if end_rate_at_target != bound
        && linear_adaptation <= MAX_EXPONENT
        && linear_adaptation >= MAX_EXPONENT.wrapping_neg()
    {
        let growth = exp_minus_one(linear_adaptation)?;
        return mul_div(start, abs(growth)?, abs(speed)?.try_mul(WAD)?);
    }
    let ramp = to_int(bound)?
        .try_mul(ln_wad(to_int(bound.try_mul(WAD)?.try_div(start)?)?)?)?
        .try_sub(to_int(bound)?.try_sub(to_int(start)?)?.try_mul(I_WAD)?)?;
    to_uint(to_int(bound.try_mul(elapsed)?)?.try_sub(ramp.try_div(speed)?)?)
}

/// The borrow rate integrated over one accrual period and where it leaves the rate at target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct AdaptiveRate {
    /// `∫ borrow rate dt` over the period, WAD (`0.04e18` is a year at 4%), rounded down: the debt grows
    /// `e^integral` times.
    pub integral: U256,
    /// The rate at target at the end of the period, per second.
    pub end_rate_at_target: U256,
}

/// The borrow rate over `elapsed` seconds from a market at `util` with `rate_at_target`
/// (`MarketLogic.adaptiveRate`): the curve applied to the exact integral of the rate at target's path, which the
/// curve is linear in ([`rate_at_target_area`]).
///
/// # Errors
///
/// [`Error::Overflow`] or [`Error::Underflow`] when the arithmetic overflows (never for a real market).
pub fn adaptive_rate(rate_at_target: U256, util: U256, elapsed: U256) -> Result<AdaptiveRate> {
    let err = utilization_error(util)?;
    let speed = ADJUSTMENT_SPEED.try_mul(err)?.try_div(I_WAD)?;
    let linear_adaptation = speed.try_mul(to_int(elapsed)?)?;
    if linear_adaptation.is_zero() {
        return Ok(AdaptiveRate {
            integral: curve(rate_at_target.try_mul(elapsed)?, err)?,
            end_rate_at_target: rate_at_target,
        });
    }

    let end_rate_at_target = adapted_rate_at_target(rate_at_target, linear_adaptation)?;
    let area = rate_at_target_area(rate_at_target, speed, linear_adaptation, end_rate_at_target, elapsed)?;
    Ok(AdaptiveRate {
        integral: curve(area, err)?,
        end_rate_at_target,
    })
}

/// A per-second rate as a simple yearly rate (`rate · 365 days`), as the lens reports APRs.
///
/// # Errors
///
/// [`Error::Overflow`] when the product overflows.
pub fn annualize(rate_per_second: U256) -> Result<U256> {
    rate_per_second.try_mul(SECONDS_PER_YEAR)
}
