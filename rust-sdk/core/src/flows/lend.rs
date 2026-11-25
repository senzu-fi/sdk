//! Lending: what a withdrawal of a lender's shares pays, and what withdrawing everything can take now.

use alloy_primitives::U256;

use crate::error::Result;
use crate::num::Checked;
use crate::slippage::min_out;

/// A withdrawal: the shares burned and the assets they pay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct Withdrawal {
    /// Shares burned: those asked for, at most the whole deposit.
    pub shares: U256,
    /// What they pay: the deposit's assets in proportion, rounded down, or all of them for the whole deposit.
    pub assets: U256,
}

/// Sizes a withdrawal of `asked_shares` (`None`: all) from a deposit of `deposit_shares` worth `deposit_assets`.
///
/// # Errors
///
/// [`crate::Error::Overflow`] on absurd values.
pub fn withdrawal(deposit_assets: U256, deposit_shares: U256, asked_shares: Option<U256>) -> Result<Withdrawal> {
    let shares = asked_shares.map_or(deposit_shares, |asked| asked.min(deposit_shares));
    let assets = if shares == deposit_shares {
        deposit_assets
    } else {
        deposit_assets.try_mul(shares)?.try_div(deposit_shares)?
    };
    Ok(Withdrawal { shares, assets })
}

/// Interest accrues while the wallet signs: a withdrawal limited by the market's cash stays this far under it.
const CASH_HEADROOM_BPS: u64 = 10;

/// What withdrawing a whole deposit can take now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct WithdrawAll {
    /// Shares to burn.
    pub shares: U256,
    /// What comes out, at most the market's cash.
    pub out: U256,
    /// What stays supplied because the market's cash is lent out.
    pub left: U256,
}

/// Withdrawing a deposit of `deposit_shares` worth `deposit_assets` when the market can pay out `max_withdraw`: all
/// of it when the cash covers it, else a little under the cash (interest accrues while the wallet signs).
///
/// # Errors
///
/// [`crate::Error::Overflow`] on absurd values.
pub fn withdraw_all(deposit_assets: U256, deposit_shares: U256, max_withdraw: U256) -> Result<WithdrawAll> {
    if deposit_shares.is_zero() || deposit_assets.is_zero() {
        return Ok(WithdrawAll::default());
    }
    if max_withdraw >= deposit_assets {
        return Ok(WithdrawAll {
            shares: deposit_shares,
            out: deposit_assets,
            left: U256::ZERO,
        });
    }
    let out = min_out(max_withdraw, CASH_HEADROOM_BPS)?;
    let shares = out.try_mul(deposit_shares)?.try_div(deposit_assets)?;
    let out = if shares.is_zero() { U256::ZERO } else { out };
    Ok(WithdrawAll {
        shares,
        out,
        left: deposit_assets.try_sub(out)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn withdrawing_all_stays_under_the_cash() {
        let (assets, shares) = (U256::from(10_000_u64), U256::from(9_000_u64));
        let full = withdraw_all(assets, shares, U256::from(20_000_u64)).unwrap();
        assert_eq!(
            full,
            WithdrawAll {
                shares,
                out: assets,
                left: U256::ZERO
            }
        );
        let capped = withdraw_all(assets, shares, U256::from(5_000_u64)).unwrap();
        assert_eq!(
            capped,
            WithdrawAll {
                shares: U256::from(4_495_u64),
                out: U256::from(4_995_u64),
                left: U256::from(5_005_u64)
            }
        );
        assert_eq!(
            withdraw_all(U256::ZERO, shares, U256::ZERO).unwrap(),
            WithdrawAll::default()
        );
    }

    #[test]
    fn a_withdrawal_pays_its_share_and_all_pays_everything() {
        let (assets, shares) = (U256::from(1_000_u64), U256::from(900_u64));
        let all = Withdrawal { shares, assets };
        assert_eq!(withdrawal(assets, shares, None).unwrap(), all);
        assert_eq!(withdrawal(assets, shares, Some(U256::from(5_000_u64))).unwrap(), all);
        let third = withdrawal(assets, shares, Some(U256::from(300_u64))).unwrap();
        assert_eq!(
            third,
            Withdrawal {
                shares: U256::from(300_u64),
                assets: U256::from(333_u64)
            }
        );
    }
}
