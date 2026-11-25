use alloy_primitives::U160;

/// Result alias used throughout the crate.
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Why a computation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Checked arithmetic overflowed (Solidity panic 0x11 on `+`, `*` or `**`).
    #[error("arithmetic overflow")]
    Overflow,
    /// Checked subtraction went below zero (Solidity panic 0x11 on `-`).
    #[error("arithmetic underflow")]
    Underflow,
    /// Division or modulo by zero (Solidity panic 0x12).
    #[error("division by zero")]
    DivisionByZero,
    /// `FullMath.mulDiv`: a zero denominator or a result that does not fit in 256 bits.
    #[error("mulDiv overflow or zero denominator")]
    MulDiv,
    /// A value did not fit the narrower integer type it is stored in (`SafeCast` / `SafeCastLib`).
    #[error("value does not fit in the target integer type")]
    CastOverflow,

    /// `TickMath.InvalidTick`: the tick is outside `[MIN_TICK, MAX_TICK]`.
    #[error("tick {0} is out of range")]
    InvalidTick(i32),
    /// A pool's tick spacing must be positive.
    #[error("tick spacing {0} is not positive")]
    InvalidTickSpacing(i32),
    /// A tick computed from a price was not a number.
    #[error("tick is not a number")]
    TickNotANumber,
    /// A price to turn into a sqrt price was negative, not a number or too large.
    #[error("price is not a usable number")]
    PriceNotUsable,
    /// `TickMath.InvalidSqrtPrice`: the sqrt price is outside `[MIN_SQRT_PRICE, MAX_SQRT_PRICE)`.
    #[error("sqrt price {0} is out of range")]
    InvalidSqrtPrice(U160),
    /// `SqrtPriceMath.InvalidPrice`: a zero sqrt price in an amount delta.
    #[error("zero sqrt price")]
    ZeroSqrtPrice,
    /// `SqrtPriceMath.InvalidPriceOrLiquidity`: a zero sqrt price or zero liquidity in a swap step.
    #[error("zero sqrt price or zero liquidity")]
    InvalidPriceOrLiquidity,
    /// `SqrtPriceMath.NotEnoughLiquidity`: the output takes more token1 than the liquidity holds.
    #[error("not enough liquidity")]
    NotEnoughLiquidity,
    /// `SqrtPriceMath.PriceOverflow`: the output takes more token0 than the liquidity holds.
    #[error("sqrt price overflow")]
    PriceOverflow,

    /// `HealthMath__InvalidSigma`: a stress factor below 1 (1e18).
    #[error("sigma is below 1e18")]
    InvalidSigma,
    /// `HealthMath__InvalidPrice`: a zero token price.
    #[error("zero token price")]
    InvalidPrice,
    /// `FixedPointMathLib.ExpOverflow`: `e^x` past `int256`, for `x` at or above `135.3e18`.
    #[error("exp overflow")]
    ExpOverflow,
    /// `FixedPointMathLib.LnWadUndefined`: `ln(x)` for `x <= 0`.
    #[error("ln of a non-positive number")]
    LnWadUndefined,

    /// `SenzuBorrowable__InsufficientCash`: the market does not hold the cash to lend.
    #[error("insufficient cash in the market")]
    InsufficientCash,
    /// `SenzuBorrowable__DebtTooLarge`: a position's borrow shares in one market at 2^119 or more.
    #[error("position debt too large")]
    DebtTooLarge,
    /// `SenzuBorrowable__ZeroAmount` / `SenzuCollateral__ZeroAmount`: nothing to repay, borrow or seize.
    #[error("zero amount")]
    ZeroAmount,
    /// `SenzuCollateral__NotLiquidatable`: every stress corner is covered.
    #[error("position is not liquidatable")]
    NotLiquidatable,
    /// `SenzuCollateral__NoDebt`: the position owes nothing in the asset being repaid.
    #[error("position has no debt in this asset")]
    NoDebt,
}
