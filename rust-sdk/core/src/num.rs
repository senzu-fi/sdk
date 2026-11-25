use alloy_primitives::{I256, U160, U256, U512};

use crate::error::{Error, Result};

/// 1e18, the fixed-point unit of every `*_wad` value.
pub const WAD: U256 = U256::from_limbs([1_000_000_000_000_000_000, 0, 0, 0]);

pub(crate) trait Checked: Sized {
    fn try_add(self, rhs: Self) -> Result<Self>;
    fn try_sub(self, rhs: Self) -> Result<Self>;
    fn try_mul(self, rhs: Self) -> Result<Self>;
    fn try_div(self, rhs: Self) -> Result<Self>;
}

macro_rules! impl_checked {
    ($($t:ty),*) => {$(
        impl Checked for $t {
            #[inline]
            fn try_add(self, rhs: Self) -> Result<Self> {
                self.checked_add(rhs).ok_or(Error::Overflow)
            }
            #[inline]
            fn try_sub(self, rhs: Self) -> Result<Self> {
                self.checked_sub(rhs).ok_or(Error::Underflow)
            }
            #[inline]
            fn try_mul(self, rhs: Self) -> Result<Self> {
                self.checked_mul(rhs).ok_or(Error::Overflow)
            }
            #[inline]
            fn try_div(self, rhs: Self) -> Result<Self> {
                self.checked_div(rhs).ok_or(Error::DivisionByZero)
            }
        }
    )*};
}

impl_checked!(U256, U512, u128, I256);

const fn mul10([l0, l1, l2, l3]: [u64; 4]) -> [u64; 4] {
    let p0 = (l0 as u128).wrapping_mul(10);
    let p1 = (l1 as u128).wrapping_mul(10).wrapping_add(p0 >> 64);
    let p2 = (l2 as u128).wrapping_mul(10).wrapping_add(p1 >> 64);
    let p3 = (l3 as u128).wrapping_mul(10).wrapping_add(p2 >> 64);
    #[expect(clippy::cast_possible_truncation, reason = "low 64 bits of each limb product")]
    [p0 as u64, p1 as u64, p2 as u64, p3 as u64]
}

#[expect(clippy::indexing_slicing, reason = "i < 78 matches POW10 length")]
const POW10: [U256; 78] = {
    let mut table = [U256::ZERO; 78];
    let mut limbs = [1_u64, 0, 0, 0];
    let mut i = 0;
    while i < 78 {
        table[i] = U256::from_limbs(limbs);
        limbs = mul10(limbs);
        i = i.wrapping_add(1);
    }
    table
};

#[inline]
pub(crate) fn pow10(decimals: u8) -> Result<U256> {
    POW10.get(usize::from(decimals)).copied().ok_or(Error::Overflow)
}

/// `SafeCastLib.toUint128`
#[inline]
pub(crate) fn to_u128(value: U256) -> Result<u128> {
    u128::try_from(value).map_err(|_| Error::CastOverflow)
}

/// `SafeCastLib.toUint160`
#[inline]
pub(crate) fn to_u160(value: U256) -> Result<U160> {
    U160::checked_from_limbs_slice(value.as_limbs()).ok_or(Error::CastOverflow)
}

/// `SafeCastLib.toInt256`
#[inline]
pub(crate) fn to_i256(value: U256) -> Result<I256> {
    I256::try_from(value).map_err(|_| Error::CastOverflow)
}

#[inline]
pub(crate) fn widen(value: U160) -> U256 {
    U256::from(value)
}
