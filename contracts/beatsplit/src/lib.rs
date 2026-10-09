#![no_std]

mod errors;
mod events;
mod storage;
mod types;

pub mod amend;
pub mod distribute;

pub use errors::Error;
pub use types::{Recipient, Split, SplitStatus};

#[cfg(test)]
mod test;

use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Vec};

use crate::amend::AmendmentProposal;
use crate::distribute::{compute_shares, payout_or_hold};
use storage::{
    bump_instance, clear_accepted, clear_claimable, clear_lock_approvals, get_claimable as storage_get_claimable,
    get_earned as storage_get_earned, is_lock_approved, load_split, next_id, save_split,
    set_lock_approved, has_proposal, load_proposal, save_proposal, remove_proposal,
};

#[contract]
pub struct BeatSplitContract;

#[contractimpl]
impl BeatSplitContract {
    /// Create a new royalty split.
    ///
    /// Validates the recipient list, stores the split in `Pending` state, and
    /// emits `SplitCreated`. Returns the new split's id.
    ///
    /// # Authorization
    /// Requires `creator` to authorise this call.
    ///
    /// # Validation
    /// * `recipients` must have 2 to 20 entries.
    /// * Every recipient's `bps` must be > 0.
    /// * No duplicate addresses are allowed.
    /// * The sum of all `bps` must equal exactly 10,000.
    pub fn create_split(
        env: Env,
        creator: Address,
        token: Address,
        recipients: Vec<Recipient>,
        metadata_hash: BytesN<32>,
    ) -> Result<u64, Error> {
        // ── Auth ──────────────────────────────────────────────────────────────
        creator.require_auth();

        // ── Validate recipient count ──────────────────────────────────────────
        let count = recipients.len();
        if !(2..=20).contains(&count) {
            return Err(Error::InvalidRecipientCount);
        }

        // ── Validate each recipient and accumulate bps sum ────────────────────
        let mut bps_sum: u32 = 0;
        for i in 0..count {
            let r = recipients.get(i).unwrap();

            // Zero-share check
            if r.bps == 0 {
                return Err(Error::ZeroShare);
            }

            // Duplicate address check: compare against all previous entries
            for j in 0..i {
                if recipients.get(j).unwrap().addr == r.addr {
                    return Err(Error::DuplicateRecipient);
                }
            }

            // Accumulate — use checked add so we don't silently wrap
            bps_sum = bps_sum.checked_add(r.bps).ok_or(Error::Overflow)?;
        }

        // ── Validate bps total ────────────────────────────────────────────────
        if bps_sum != 10_000 {
            return Err(Error::BpsTotalInvalid);
        }

        // ── Assign id and persist ─────────────────────────────────────────────
        bump_instance(&env);
        let id = next_id(&env);

        let split = Split {
            id,
            creator: creator.clone(),
            token,
            recipients,
            status: SplitStatus::Pending,
            metadata_hash,
            total_received: 0,
            version: 1,
        };
        save_split(&env, &split);

        // ── Emit event ────────────────────────────────────────────────────────
        events::split_created(&env, id, &creator);

        Ok(id)
    }

    /// Accept a share in a split.
    ///
    /// Records the caller's consent. When every recipient has accepted, the
    /// split transitions from `Pending` to `Active` and emits `Activated`.
    ///
    /// # Authorization
    /// Requires `recipient` to authorise this call.
    ///
    /// # Errors
    /// * `SplitNotFound` — no split with this id.
    /// * `NotPending` — split is already Active or Locked.
    /// * `NotARecipient` — caller is not in the recipient list.
    /// * `AlreadyAccepted` — caller has already accepted.
    pub fn accept(env: Env, id: u64, recipient: Address) -> Result<(), Error> {
        // ── Auth ──────────────────────────────────────────────────────────────
        recipient.require_auth();

        bump_instance(&env);

        // ── Load split ────────────────────────────────────────────────────────
        let mut split = load_split(&env, id).ok_or(Error::SplitNotFound)?;

        // ── Status guard ──────────────────────────────────────────────────────
        if split.status != SplitStatus::Pending {
            return Err(Error::NotPending);
        }

        // ── Recipient membership check ─────────────────────────────────────────
        let is_recipient = split.recipients.iter().any(|r| r.addr == recipient);
        if !is_recipient {
            return Err(Error::NotARecipient);
        }

        // ── Double-accept guard ───────────────────────────────────────────────
        if storage::is_accepted(&env, id, &recipient) {
            return Err(Error::AlreadyAccepted);
        }

        // ── Record acceptance ─────────────────────────────────────────────────
        storage::set_accepted(&env, id, &recipient);
        events::accepted(&env, id, &recipient);

        // ── Check if all recipients have now accepted ──────────────────────────
        let all_accepted = split
            .recipients
            .iter()
            .all(|r| storage::is_accepted(&env, id, &r.addr));

        if all_accepted {
            split.status = SplitStatus::Active;
            save_split(&env, &split);
            events::activated(&env, id);
        }

        Ok(())
    }

    /// Deposit tokens into an active split and distribute shares immediately.
    ///
    /// Pulls `amount` of the split's token from `from` into the contract, computes
    /// shares per the distribution algorithm, updates accounting state (effects),
    /// then performs failure-isolated payouts (interactions).
    ///
    /// # Authorization
    /// Requires `from` to authorise this call.
    ///
    /// # Errors
    /// * `SplitNotFound` — no split with this id.
    /// * `NotActive` — split is Pending (deposits only allowed on Active or Locked).
    /// * `InvalidAmount` — amount must be > 0.
    /// * `Overflow` — arithmetic overflow in share computation or total_received update.
    pub fn deposit(env: Env, id: u64, from: Address, amount: i128) -> Result<(), Error> {
        // ── Auth ──────────────────────────────────────────────────────────────
        from.require_auth();

        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        bump_instance(&env);

        // ── Load split ────────────────────────────────────────────────────────
        let mut split = load_split(&env, id).ok_or(Error::SplitNotFound)?;

        // ── Status guard: only Active or Locked splits accept deposits ───────
        if !matches!(split.status, SplitStatus::Active | SplitStatus::Locked) {
            return Err(Error::NotActive);
        }

        // ── Token client ──────────────────────────────────────────────────────
        let token_client = TokenClient::new(&env, &split.token);

        // ── Pull tokens from payer to contract ────────────────────────────────
        token_client.transfer(&from, env.current_contract_address(), &amount);

        // ── Compute shares ────────────────────────────────────────────────────
        let shares = compute_shares(&env, &split.recipients, amount)?;

        // ── Effects: update accounting BEFORE payouts (checks-effects-interactions) ──
        split.total_received = split
            .total_received
            .checked_add(amount)
            .ok_or(Error::Overflow)?;
        save_split(&env, &split);

        // ── Emit Deposited event ──────────────────────────────────────────────
        events::deposited(&env, id, &from, amount);

        // ── Interactions: failure-isolated payouts ────────────────────────────
        for (recipient, share) in split.recipients.iter().zip(shares.iter()) {
            payout_or_hold(&env, &token_client, id, &recipient.addr, share);
        }

        Ok(())
    }

    /// Read a split by id. Returns `None` if not found.
    pub fn get_split(env: Env, id: u64) -> Option<Split> {
        bump_instance(&env);
        load_split(&env, id)
    }

    /// Read the claimable (held) balance for a recipient in a split.
    pub fn get_claimable(env: Env, id: u64, addr: Address) -> i128 {
        bump_instance(&env);
        storage::get_claimable(&env, id, &addr)
    }

    /// Read the lifetime earnings for a recipient in a split.
    pub fn get_earned(env: Env, id: u64, addr: Address) -> i128 {
        bump_instance(&env);
        storage_get_earned(&env, id, &addr)
    }

    /// Claim a held (failed-push) balance for a recipient.
    ///
    /// Requires auth from the recipient. The claimable balance is set to zero
    /// BEFORE transferring (checks-effects-interactions). If the transfer fails,
    /// the entire call reverts.
    ///
    /// # Authorization
    /// Requires `recipient` to authorise this call.
    ///
    /// # Errors
    /// * `SplitNotFound` — no split with this id.
    /// * `NothingToClaim` — recipient has no held balance.
    /// * `Overflow` — arithmetic overflow (should not occur).
    pub fn claim(env: Env, id: u64, recipient: Address) -> Result<(), Error> {
        // ── Auth ──────────────────────────────────────────────────────────────
        recipient.require_auth();

        bump_instance(&env);

        // ── Load split ────────────────────────────────────────────────────────
        let split = load_split(&env, id).ok_or(Error::SplitNotFound)?;

        // ── Get claimable balance ─────────────────────────────────────────────
        let claimable = storage_get_claimable(&env, id, &recipient);
        if claimable == 0 {
            return Err(Error::NothingToClaim);
        }

        // ── Token client ──────────────────────────────────────────────────────
        let token_client = TokenClient::new(&env, &split.token);

        // ── Effects: clear claimable balance BEFORE transfer ──────────────────
        clear_claimable(&env, id, &recipient);

        // ── Transfer to recipient ─────────────────────────────────────────────
        token_client.transfer(&env.current_contract_address(), &recipient, &claimable);

        // ── Emit Claimed event ────────────────────────────────────────────────
        events::claimed(&env, id, &recipient, claimable);

        Ok(())
    }

    /// Distribute any token balance sent directly to the contract for this split.
    ///
    /// This allows anyone to send tokens directly to the contract address (instead
    /// of calling `deposit`), and then call this function to distribute them per
    /// the split's recipient list. No auth required.
    ///
    /// IMPORTANT: This implementation tracks unaccounted balance PER SPLIT by
    /// comparing the contract's total token balance against the split's
    /// `total_received`. Since a single contract can manage multiple splits for
    /// the SAME token, we cannot safely isolate one split's direct transfers from
    /// another's. Therefore, this function is intentionally NOT implemented for
    /// multi-split scenarios with a shared token.
    ///
    /// To use this safely, each split must use a UNIQUE token, or the caller must
    /// ensure no other splits share the token.
    ///
    /// # Errors
    /// * `SplitNotFound` — no split with this id.
    /// * `InvalidAmount` — no unaccounted balance to distribute.
    pub fn distribute_balance(env: Env, id: u64) -> Result<(), Error> {
        bump_instance(&env);

        // ── Load split ────────────────────────────────────────────────────────
        let mut split = load_split(&env, id).ok_or(Error::SplitNotFound)?;

        // ── Token client ──────────────────────────────────────────────────────
        let token_client = TokenClient::new(&env, &split.token);

        // ── Get contract's total token balance ─────────────────────────────────
        let contract_balance = token_client.balance(&env.current_contract_address());

        // ── Unaccounted = contract_balance - total_received ────────────────────
        // Note: This is UNSAFE if multiple splits share the same token, because
        // one split's direct transfer would be attributed to all splits using
        // that token. The README acknowledges this limitation.
        let unaccounted = contract_balance
            .checked_sub(split.total_received)
            .ok_or(Error::Overflow)?;

        if unaccounted <= 0 {
            return Err(Error::InvalidAmount);
        }

        // ── Compute shares for unaccounted amount ──────────────────────────────
        let shares = compute_shares(&env, &split.recipients, unaccounted)?;

        // ── Effects: update total_received BEFORE payouts ──────────────────────
        split.total_received = split
            .total_received
            .checked_add(unaccounted)
            .ok_or(Error::Overflow)?;
        save_split(&env, &split);

        // ── Emit Deposited event (with from = contract address) ────────────────
        events::deposited(&env, id, &env.current_contract_address(), unaccounted);

        // ── Interactions: failure-isolated payouts ────────────────────────────
        for (recipient, share) in split.recipients.iter().zip(shares.iter()) {
            payout_or_hold(&env, &token_client, id, &recipient.addr, share);
        }

        Ok(())
    }

    /// Propose an amendment to change the recipient list of an active split.
    ///
    /// Any current recipient can propose a new recipient list. Only one proposal
    /// can be open at a time. The proposal must be approved by all current
    /// recipients before it takes effect.
    ///
    /// # Authorization
    /// Requires `proposer` to authorise this call. `proposer` must be a current
    /// recipient of the split.
    ///
    /// # Validation
    /// * Split must exist and be Active (not Pending or Locked).
    /// * Proposer must be a current recipient.
    /// * No other amendment proposal can be open.
    /// * New recipient list must pass the same validation as create_split (2-20
    ///   recipients, positive bps, no duplicates, sum = 10,000).
    ///
    /// # Errors
    /// * `SplitNotFound` — no split with this id.
    /// * `SplitLocked` — split is locked and cannot be amended.
    /// * `NotActive` — split is not Active (Pending or Locked).
    /// * `NotARecipient` — proposer is not a current recipient.
    /// * `AmendmentAlreadyOpen` — another proposal is already pending.
    /// * `InvalidRecipientCount` / `ZeroShare` / `DuplicateRecipient` /
    ///   `BpsTotalInvalid` — validation errors on the new recipient list.
    pub fn propose_amendment(
        env: Env,
        id: u64,
        proposer: Address,
        new_recipients: Vec<Recipient>,
    ) -> Result<(), Error> {
        // ── Auth ──────────────────────────────────────────────────────────────
        proposer.require_auth();

        bump_instance(&env);

        // ── Load split ────────────────────────────────────────────────────────
        let split = load_split(&env, id).ok_or(Error::SplitNotFound)?;

        // ── Status guards ──────────────────────────────────────────────────────
        if split.status == SplitStatus::Locked {
            return Err(Error::SplitLocked);
        }
        if split.status != SplitStatus::Active {
            return Err(Error::NotActive);
        }

        // ── Proposer must be a current recipient ───────────────────────────────
        let is_recipient = split.recipients.iter().any(|r| r.addr == proposer);
        if !is_recipient {
            return Err(Error::NotARecipient);
        }

        // ── No open proposal allowed ───────────────────────────────────────────
        if storage::has_proposal(&env, id) {
            return Err(Error::AmendmentAlreadyOpen);
        }

        // ── Validate new recipient list (same rules as create_split) ────────────
        let count = new_recipients.len();
        if !(2..=20).contains(&count) {
            return Err(Error::InvalidRecipientCount);
        }

        let mut bps_sum: u32 = 0;
        for i in 0..count {
            let r = new_recipients.get(i).unwrap();

            if r.bps == 0 {
                return Err(Error::ZeroShare);
            }

            for j in 0..i {
                if new_recipients.get(j).unwrap().addr == r.addr {
                    return Err(Error::DuplicateRecipient);
                }
            }

            bps_sum = bps_sum.checked_add(r.bps).ok_or(Error::Overflow)?;
        }

        if bps_sum != 10_000 {
            return Err(Error::BpsTotalInvalid);
        }

        // ── Create and store proposal ──────────────────────────────────────────
        let proposal = AmendmentProposal {
            split_id: id,
            proposer: proposer.clone(),
            new_recipients,
            approvals: Vec::new(&env),
            base_version: split.version,
        };

        save_proposal(&env, id, &proposal);

        // ── Emit event ────────────────────────────────────────────────────────
        events::amendment_proposed(&env, id, &proposer);

        Ok(())
    }

    /// Approve the current amendment proposal for a split.
    ///
    /// Each current recipient must approve the proposal. When all recipients have
    /// approved, the amendment is automatically applied: the recipient list is
    /// updated, the split version is incremented, and the proposal is removed.
    ///
    /// # Authorization
    /// Requires `approver` to authorise this call. `approver` must be a current
    /// recipient of the split.
    ///
    /// # Errors
    /// * `SplitNotFound` — no split with this id.
    /// * `SplitLocked` — split is locked.
    /// * `NoOpenAmendment` — no proposal is open for this split.
    /// * `NotARecipient` — approver is not a current recipient.
    /// * `AlreadyApprovedAmendment` — approver has already approved this proposal.
    pub fn approve_amendment(env: Env, id: u64, approver: Address) -> Result<(), Error> {
        // ── Auth ──────────────────────────────────────────────────────────────
        approver.require_auth();

        bump_instance(&env);

        // ── Load split ────────────────────────────────────────────────────────
        let mut split = load_split(&env, id).ok_or(Error::SplitNotFound)?;

        // ── Status guards ──────────────────────────────────────────────────────
        if split.status == SplitStatus::Locked {
            return Err(Error::SplitLocked);
        }

        // ── Must have an open proposal ────────────────────────────────────────
        let mut proposal = storage::load_proposal::<AmendmentProposal>(&env, id)
            .ok_or(Error::NoOpenAmendment)?;

        // ── Approver must be a current recipient ───────────────────────────────
        let is_recipient = split.recipients.iter().any(|r| r.addr == approver);
        if !is_recipient {
            return Err(Error::NotARecipient);
        }

        // ── Check if already approved ──────────────────────────────────────────
        if proposal.has_approved(&approver) {
            return Err(Error::AlreadyApprovedAmendment);
        }

        // ── Check if proposal is stale (split version changed) ─────────────────
        if proposal.base_version != split.version {
            // Proposal is stale, remove it and reject
            storage::remove_proposal(&env, id);
            return Err(Error::NoOpenAmendment);
        }

        // ── Add approval ───────────────────────────────────────────────────────
        proposal.add_approval(approver.clone());
        storage::save_proposal(&env, id, &proposal);

        // ── Emit event ────────────────────────────────────────────────────────
        events::amendment_approved(&env, id, &approver);

        // ── Check if all recipients have approved ──────────────────────────────
        if proposal.is_fully_approved(&split.recipients) {
            // Apply the amendment
            Self::apply_amendment(&env, &mut split, &mut proposal, id);
        }

        Ok(())
    }

    /// Internal: apply an approved amendment.
    ///
    /// Updates the split's recipient list, increments version, clears acceptance
    /// flags for new recipients, and removes the proposal.
    fn apply_amendment(
        env: &Env,
        split: &mut Split,
        proposal: &mut AmendmentProposal,
        id: u64,
    ) {
        // Update recipients
        split.recipients = proposal.new_recipients.clone();
        // Increment version
        split.version = split.version.checked_add(1).unwrap();
        // Save updated split
        save_split(env, split);

        // Clear acceptance flags for all recipients (new and old)
        // This requires all recipients to re-accept the new terms
        for recipient in split.recipients.iter() {
            clear_accepted(env, id, &recipient.addr);
        }

        // Remove proposal
        storage::remove_proposal(env, id);

        // Emit event
        events::amendment_applied(env, id, split.version);
    }

    /// Cancel an open amendment proposal.
    ///
    /// Any current recipient can cancel the proposal. This removes the proposal
    /// and allows a new one to be created.
    ///
    /// # Authorization
    /// Requires `canceller` to authorise this call. `canceller` must be a current
    /// recipient of the split.
    ///
    /// # Errors
    /// * `SplitNotFound` — no split with this id.
    /// * `SplitLocked` — split is locked.
    /// * `NoOpenAmendment` — no proposal is open for this split.
    /// * `NotARecipient` — canceller is not a current recipient.
    pub fn cancel_amendment(env: Env, id: u64, canceller: Address) -> Result<(), Error> {
        // ── Auth ──────────────────────────────────────────────────────────────
        canceller.require_auth();

        bump_instance(&env);

        // ── Load split ────────────────────────────────────────────────────────
        let split = load_split(&env, id).ok_or(Error::SplitNotFound)?;

        // ── Status guard ───────────────────────────────────────────────────────
        if split.status == SplitStatus::Locked {
            return Err(Error::SplitLocked);
        }

        // ── Must have an open proposal ────────────────────────────────────────
        if !storage::has_proposal(&env, id) {
            return Err(Error::NoOpenAmendment);
        }

        // ── Canceller must be a current recipient ──────────────────────────────
        let is_recipient = split.recipients.iter().any(|r| r.addr == canceller);
        if !is_recipient {
            return Err(Error::NotARecipient);
        }

        // ── Remove proposal ────────────────────────────────────────────────────
        storage::remove_proposal(&env, id);

        // ── Emit event ────────────────────────────────────────────────────────
        events::amendment_cancelled(&env, id, &canceller);

        Ok(())
    }

    /// Approve locking a split.
    ///
    /// Each current recipient must call this to approve locking the split.
    /// When all recipients have approved, the split is automatically locked.
    ///
    /// # Authorization
    /// Requires `approver` to authorise this call. `approver` must be a current
    /// recipient of the split.
    ///
    /// # Errors
    /// * `SplitNotFound` — no split with this id.
    /// * `SplitLocked` — split is already locked.
    /// * `NotActive` — split is not Active (must be Active to lock).
    /// * `NotARecipient` — approver is not a current recipient.
    pub fn approve_lock(env: Env, id: u64, approver: Address) -> Result<(), Error> {
        // ── Auth ──────────────────────────────────────────────────────────────
        approver.require_auth();

        bump_instance(&env);

        // ── Load split ────────────────────────────────────────────────────────
        let mut split = load_split(&env, id).ok_or(Error::SplitNotFound)?;

        // ── Status guards ──────────────────────────────────────────────────────
        if split.status == SplitStatus::Locked {
            return Err(Error::SplitLocked);
        }
        if split.status != SplitStatus::Active {
            return Err(Error::NotActive);
        }

        // ── Approver must be a current recipient ───────────────────────────────
        let is_recipient = split.recipients.iter().any(|r| r.addr == approver);
        if !is_recipient {
            return Err(Error::NotARecipient);
        }

        // ── Check if already approved ──────────────────────────────────────────
        if is_lock_approved(&env, id, &approver) {
            return Err(Error::AlreadyApprovedAmendment); // Reuse error for "already approved"
        }

        // ── Record approval ────────────────────────────────────────────────────
        set_lock_approved(&env, id, &approver);

        // ── Check if all recipients have approved ──────────────────────────────
        let all_approved = split
            .recipients
            .iter()
            .all(|r| is_lock_approved(&env, id, &r.addr));

        if all_approved {
            // Lock the split
            split.status = SplitStatus::Locked;
            save_split(&env, &split);

            // Clear lock approvals
            clear_lock_approvals(&env, id, &split.recipients);

            // Emit event
            events::locked(&env, id);
        }

        Ok(())
    }
}
