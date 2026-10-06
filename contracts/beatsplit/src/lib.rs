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

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Vec};

use storage::{bump_instance, load_split, next_id, save_split};

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
        if count < 2 || count > 20 {
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
        storage::get_earned(&env, id, &addr)
    }
}
