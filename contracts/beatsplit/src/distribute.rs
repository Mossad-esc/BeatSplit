//! Distribution logic: share computation and failure-isolated payouts.

use soroban_sdk::{Env, Vec};

use crate::types::Recipient;
use crate::Error;

/// Compute the share for each recipient.
///
/// Algorithm per README "Distribution algorithm":
/// - For every recipient except the first: `share_i = floor(amount * bps_i / 10_000)`
/// - The first recipient receives `amount - sum(other_shares)` (absorbs rounding dust).
///
/// Uses checked arithmetic throughout. Returns `Error::Overflow` on arithmetic failure.
/// Rejects `amount <= 0` with `Error::InvalidAmount`.
pub fn compute_shares(
    env: &Env,
    recipients: &Vec<Recipient>,
    amount: i128,
) -> Result<Vec<i128>, Error> {
    if amount <= 0 {
        return Err(Error::InvalidAmount);
    }

    let count = recipients.len();
    if count == 0 {
        return Err(Error::InvalidRecipientCount);
    }

    let mut shares = Vec::new(env);
    let mut sum_others: i128 = 0;

    // Compute shares for all recipients except the first (index 0)
    for i in 1..count {
        let r = recipients.get(i).unwrap();
        let bps = r.bps as i128;

        // share = floor(amount * bps / 10_000)
        let share = amount
            .checked_mul(bps)
            .ok_or(Error::Overflow)?
            .checked_div(10_000)
            .ok_or(Error::Overflow)?;

        sum_others = sum_others.checked_add(share).ok_or(Error::Overflow)?;
        shares.push_back(share);
    }

    // First recipient gets the remainder (absorbs dust)
    let first_share = amount.checked_sub(sum_others).ok_or(Error::Overflow)?;
    let mut result = Vec::new(env);
    result.push_back(first_share);

    // Append the rest in order
    for s in shares.iter() {
        result.push_back(s);
    }

    // Invariant check: sum(shares) == amount
    let mut total: i128 = 0;
    for s in result.iter() {
        total = total.checked_add(s).ok_or(Error::Overflow)?;
    }
    if total != amount {
        return Err(Error::Overflow);
    }

    Ok(result)
}

/// Perform a failure-isolated payout for a single share.
///
/// Attempts to transfer `share` from the contract to `recipient` using
/// `token_client.try_transfer`. On success, records the payout in `Earned`
/// and emits `Payout`. On failure, adds the share to `Claimable` and emits `Held`.
pub fn payout_or_hold(
    env: &Env,
    token_client: &soroban_sdk::token::Client,
    id: u64,
    recipient: &soroban_sdk::Address,
    share: i128,
) {
    // Note: try_transfer returns Result<(), Error> in newer SDK versions.
    // We match on the result to isolate failures.
    match token_client.try_transfer(&env.current_contract_address(), recipient, &share) {
        Ok(_) => {
            // Success: record earnings and emit Payout
            crate::storage::add_earned(env, id, recipient, share);
            crate::events::payout(env, id, recipient, share);
        }
        Err(_) => {
            // Failure: hold the balance for later claim and emit Held
            crate::storage::add_claimable(env, id, recipient, share);
            crate::events::held(env, id, recipient, share);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

    fn make_recipients(env: &Env, bps_list: &[u32]) -> Vec<Recipient> {
        let mut v = Vec::new(env);
        for bps in bps_list {
            v.push_back(Recipient {
                addr: Address::generate(env),
                bps: *bps,
            });
        }
        v
    }

    #[test]
    fn compute_shares_even_split() {
        let env = Env::default();
        let recipients = make_recipients(&env, &[5000, 5000]);
        let shares = compute_shares(&env, &recipients, 100_000_000).unwrap(); // 10 USDC (7 decimals)
        assert_eq!(shares.len(), 2);
        // First gets remainder: 50,000,000 each exactly
        assert_eq!(shares.get(0).unwrap(), 50_000_000);
        assert_eq!(shares.get(1).unwrap(), 50_000_000);
        assert_eq!(shares.get(0).unwrap() + shares.get(1).unwrap(), 100_000_000);
    }

    #[test]
    fn compute_shares_uneven_bps() {
        let env = Env::default();
        let recipients = make_recipients(&env, &[7000, 3000]);
        let shares = compute_shares(&env, &recipients, 100_000_000).unwrap();
        assert_eq!(shares.len(), 2);
        // floor(100_000_000 * 3000 / 10000) = 30_000_000
        // first gets 100_000_000 - 30_000_000 = 70_000_000
        assert_eq!(shares.get(1).unwrap(), 30_000_000);
        assert_eq!(shares.get(0).unwrap(), 70_000_000);
        assert_eq!(shares.get(0).unwrap() + shares.get(1).unwrap(), 100_000_000);
    }

    #[test]
    fn compute_shares_one_unit_deposit() {
        let env = Env::default();
        let recipients = make_recipients(&env, &[5000, 5000]);
        let shares = compute_shares(&env, &recipients, 1).unwrap();
        assert_eq!(shares.len(), 2);
        // floor(1 * 5000 / 10000) = 0 for second
        // first gets 1 - 0 = 1 (all dust to first)
        assert_eq!(shares.get(1).unwrap(), 0);
        assert_eq!(shares.get(0).unwrap(), 1);
        assert_eq!(shares.get(0).unwrap() + shares.get(1).unwrap(), 1);
    }

    #[test]
    fn compute_shares_three_way_split_of_100() {
        let env = Env::default();
        let recipients = make_recipients(&env, &[4000, 3000, 3000]);
        let shares = compute_shares(&env, &recipients, 100).unwrap();
        assert_eq!(shares.len(), 3);
        // floor(100 * 3000 / 10000) = 30 for index 1 and 2
        // first gets 100 - 30 - 30 = 40
        assert_eq!(shares.get(1).unwrap(), 30);
        assert_eq!(shares.get(2).unwrap(), 30);
        assert_eq!(shares.get(0).unwrap(), 40);
        assert_eq!(
            shares.get(0).unwrap() + shares.get(1).unwrap() + shares.get(2).unwrap(),
            100
        );
    }

    #[test]
    fn compute_shares_large_amount_near_overflow() {
        let env = Env::default();
        let recipients = make_recipients(&env, &[5000, 5000]);
        // Use a large amount that won't overflow when multiplied by max bps (10000)
        // i128::MAX / 10000 ≈ 1.7e34
        let large_amount = i128::MAX / 20000;
        let shares = compute_shares(&env, &recipients, large_amount).unwrap();
        assert_eq!(shares.len(), 2);
        let sum = shares.get(0).unwrap() + shares.get(1).unwrap();
        assert_eq!(sum, large_amount);
    }

    #[test]
    fn compute_shares_rejects_zero_amount() {
        let env = Env::default();
        let recipients = make_recipients(&env, &[5000, 5000]);
        let err = compute_shares(&env, &recipients, 0).unwrap_err();
        assert_eq!(err, Error::InvalidAmount);
    }

    #[test]
    fn compute_shares_rejects_negative_amount() {
        let env = Env::default();
        let recipients = make_recipients(&env, &[5000, 5000]);
        let err = compute_shares(&env, &recipients, -1).unwrap_err();
        assert_eq!(err, Error::InvalidAmount);
    }

    #[test]
    fn compute_shares_invariant_sum_equals_amount() {
        let env = Env::default();
        // Test multiple random-ish splits
        let test_cases_bps: &[&[u32]] = &[
            &[5000, 5000],
            &[7000, 3000],
            &[4000, 3000, 3000],
            &[1000, 2000, 3000, 4000],
            &[3333, 3333, 3334],
        ];
        let test_cases_amount: &[i128] = &[12345, 9999999, 100, 1000000, 10000];

        for i in 0..test_cases_bps.len() {
            let bps_list = test_cases_bps[i];
            let amount = test_cases_amount[i];
            let recipients = make_recipients(&env, bps_list);
            let shares = compute_shares(&env, &recipients, amount).unwrap();
            let mut sum: i128 = 0;
            for s in shares.iter() {
                sum = sum.checked_add(s).unwrap();
            }
            assert_eq!(sum, amount);
        }
    }
}
