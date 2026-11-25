//! The SENZU vault: a staker's share, the reserves it streams and what a stake earns of them.

use alloy_primitives::U256;

use crate::error::Result;
use crate::num::{Checked, WAD};

/// A stake's share of all staked (1e18 = all), rounded down; `None` without a stake or stakers.
///
/// # Errors
///
/// [`crate::Error::Overflow`] on an absurd stake.
pub fn stake_share_wad(staked: U256, total_staked: U256) -> Result<Option<U256>> {
    if staked.is_zero() || total_staked.is_zero() {
        return Ok(None);
    }
    Ok(Some(staked.try_mul(WAD)?.try_div(total_staked)?))
}

/// What `stake` of `total_staked` earns of `inflow`, rounded down; zero without stakers.
///
/// # Errors
///
/// [`crate::Error::Overflow`] on absurd values.
pub fn stake_income(inflow: U256, stake: U256, total_staked: U256) -> Result<U256> {
    if total_staked.is_zero() {
        return Ok(U256::ZERO);
    }
    inflow.try_mul(stake)?.try_div(total_staked)
}

/// The vault's share of the interest borrowers pay (1e18 = all): its inflow over the interest, or the reserve factor
/// when there is no interest yet.
///
/// # Errors
///
/// [`crate::Error::Overflow`] on an absurd inflow.
pub fn reserve_share_wad(inflow: U256, interest: U256, reserve_factor_wad: U256) -> Result<U256> {
    if interest.is_zero() {
        return Ok(reserve_factor_wad);
    }
    inflow.try_mul(WAD)?.try_div(interest)
}

/// What a reward stream still pays: its rate (tokens a second, 1e18) over the seconds left until `period_finish`.
///
/// # Errors
///
/// [`crate::Error::Overflow`] on an absurd rate.
pub fn stream_remaining(rate_wad: U256, period_finish: u64, now: u64) -> Result<U256> {
    let left = U256::from(period_finish.saturating_sub(now));
    rate_wad.try_mul(left)?.try_div(WAD)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wad(x: u64) -> U256 {
        U256::from(x).try_mul(WAD).unwrap()
    }

    #[test]
    fn a_quarter_of_the_stake_earns_a_quarter() {
        assert_eq!(
            stake_share_wad(wad(25), wad(100)).unwrap(),
            Some(wad(1) / U256::from(4_u8))
        );
        assert_eq!(stake_share_wad(U256::ZERO, wad(100)).unwrap(), None);
        assert_eq!(stake_income(wad(40), wad(25), wad(100)).unwrap(), wad(10));
        assert_eq!(stake_income(wad(40), wad(25), U256::ZERO).unwrap(), U256::ZERO);
    }

    #[test]
    fn a_finished_stream_pays_nothing_more() {
        assert_eq!(stream_remaining(wad(2), 1_000, 400).unwrap(), U256::from(1_200_u64));
        assert_eq!(stream_remaining(wad(2), 1_000, 2_000).unwrap(), U256::ZERO);
        assert_eq!(
            reserve_share_wad(U256::ZERO, U256::ZERO, wad(1) / U256::from(2_u8)).unwrap(),
            wad(1) / U256::from(2_u8)
        );
    }
}
