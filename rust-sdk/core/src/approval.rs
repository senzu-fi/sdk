//! Token approvals: how much an app asks a wallet to approve, and when an allowance is too low to send.
//!
//! Not a contract port. A position's amounts move a little between a preview and the block that includes it (interest
//! accrues, the pool moves), so an exact approval would make the transaction revert; these leave headroom.

use alloy_primitives::U256;

use crate::error::Result;
use crate::num::Checked;

/// An approval covers the amount plus 2%, and one wei.
const APPROVAL_HEADROOM_DIVISOR: u64 = 50;
/// An allowance is enough once it covers the amount plus 1%.
const ALLOWANCE_HEADROOM_DIVISOR: u64 = 100;

/// What to approve for `amount`: 2% more, and one wei, rounded down.
///
/// # Errors
///
/// [`crate::Error::Overflow`] for an amount within 2% of `2^256`.
pub fn approval_amount(amount: U256) -> Result<U256> {
    amount
        .try_add(amount.try_div(U256::from(APPROVAL_HEADROOM_DIVISOR))?)?
        .try_add(U256::from(1_u8))
}

/// Whether `allowance` is too low to send a transaction spending `amount`: under the amount plus 1%. Never for nothing.
#[must_use]
pub fn needs_approval(amount: U256, allowance: U256) -> bool {
    if amount.is_zero() {
        return false;
    }
    let headroom = amount
        .checked_div(U256::from(ALLOWANCE_HEADROOM_DIVISOR))
        .unwrap_or(U256::ZERO);
    amount.checked_add(headroom).is_none_or(|needed| allowance < needed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_approval_leaves_more_room_than_a_check_needs() {
        let amount = U256::from(10_000_u64);
        let approved = approval_amount(amount).unwrap();
        assert_eq!(approved, U256::from(10_201_u64));
        assert!(!needs_approval(amount, approved));
        assert!(needs_approval(amount, U256::from(10_099_u64)));
        assert!(!needs_approval(U256::ZERO, U256::ZERO));
    }
}
