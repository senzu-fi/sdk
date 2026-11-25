//! The fees a live swap pays liquidity providers, as an app credits them before the chain is read again.

use alloy_primitives::U256;

use crate::error::Result;
use crate::num::Checked;

/// Pips (millionths) in 100%.
const PIPS: u64 = 1_000_000;

/// The pool's protocol fee on a swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub enum ProtocolFee {
    /// Uniswap v3: `1 / divisor` of the swap fee (0 for none).
    Divisor(u8),
    /// Uniswap v4 and Aero: pips of the amount in, taken before the swap fee.
    Pips(u32),
}

/// The liquidity providers' fee on `amount_in` at a swap fee of `fee_pips`, less the protocol's, rounded down.
///
/// # Errors
///
/// [`crate::Error::Overflow`] on an absurd amount.
pub fn lp_fee(amount_in: U256, fee_pips: u32, protocol: ProtocolFee) -> Result<U256> {
    let pips = U256::from(PIPS);
    match protocol {
        ProtocolFee::Pips(taken) => amount_in
            .try_mul(U256::from(fee_pips.saturating_sub(taken)))?
            .try_div(pips),
        ProtocolFee::Divisor(divisor) => {
            let fee = amount_in.try_mul(U256::from(fee_pips))?.try_div(pips)?;
            if divisor == 0 {
                Ok(fee)
            } else {
                fee.try_sub(fee.try_div(U256::from(divisor))?)
            }
        }
    }
}

/// What `liquidity` in range earns of `fee` paid to `active_liquidity`, rounded down; zero without active liquidity.
///
/// # Errors
///
/// [`crate::Error::Overflow`] on an absurd fee.
pub fn fee_share(fee: U256, liquidity: U256, active_liquidity: U256) -> Result<U256> {
    if active_liquidity.is_zero() {
        return Ok(U256::ZERO);
    }
    fee.try_mul(liquidity.min(active_liquidity))?.try_div(active_liquidity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_protocol_takes_its_cut_first() {
        let amount = U256::from(1_000_000_000_u64);
        assert_eq!(
            lp_fee(amount, 3_000, ProtocolFee::Divisor(0)).unwrap(),
            U256::from(3_000_000_u64)
        );
        assert_eq!(
            lp_fee(amount, 3_000, ProtocolFee::Divisor(4)).unwrap(),
            U256::from(2_250_000_u64)
        );
        assert_eq!(
            lp_fee(amount, 3_000, ProtocolFee::Pips(1_000)).unwrap(),
            U256::from(2_000_000_u64)
        );
        assert_eq!(lp_fee(amount, 500, ProtocolFee::Pips(1_000)).unwrap(), U256::ZERO);
        let fee = U256::from(1_000_u64);
        assert_eq!(
            fee_share(fee, U256::from(25_u8), U256::from(100_u8)).unwrap(),
            U256::from(250_u64)
        );
        assert_eq!(fee_share(fee, U256::from(500_u64), U256::from(100_u8)).unwrap(), fee);
    }
}
