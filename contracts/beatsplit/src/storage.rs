//! Typed storage helpers and TTL management for BeatSplit.
//!
//! Every persistent entry has its TTL extended on every read and write so that
//! active splits stay alive on the ledger. A keeper process should also call
//! the bump helpers periodically for splits that go a long time between
//! transactions.
//!
//! TTL constants are expressed in ledger-close units. On Stellar mainnet a
//! ledger closes roughly every 5 seconds, so:
//!   * 1 day  ≈ 17,280 ledgers
//!   * 1 year ≈ 6,307,200 ledgers
//!
//! We target a minimum lifetime of 1 year and always extend to 2 years so that
//! a single call to bump covers a long runway. These numbers are conservative
//! for the MVP; the TTL keeper can extend further.

use soroban_sdk::{Address, Env};

use crate::types::{Split, SplitStatus};

// ── TTL constants ────────────────────────────────────────────────────────────

/// If a persistent entry's remaining TTL falls below this threshold, extend it.
/// ≈ 1 year in ledgers (17,280 ledgers/day × 365).
pub const PERSISTENT_BUMP_THRESHOLD: u32 = 6_307_200;

/// Extend persistent entries to this TTL when bumping.
/// ≈ 2 years in ledgers.
pub const PERSISTENT_BUMP_TO: u32 = 12_614_400;

/// If the instance (contract-level) entry's remaining TTL falls below this
/// threshold, extend it.
pub const INSTANCE_BUMP_THRESHOLD: u32 = 6_307_200;

/// Extend the instance entry to this TTL when bumping.
pub const INSTANCE_BUMP_TO: u32 = 12_614_400;

// ── Storage keys ─────────────────────────────────────────────────────────────

/// Discriminated union of every persistent storage key used by the contract.
///
/// Each variant maps to a row in Soroban persistent storage. Keeping them in
/// one enum makes exhaustive matching easy and avoids key collisions.
#[soroban_sdk::contracttype]
enum DataKey {
    /// `Split(id)` → `Split`. The full split record.
    Split(u64),
    /// `Accepted(id, addr)` → `bool`. Whether `addr` has accepted split `id`.
    Accepted(u64, Address),
    /// `Claimable(id, addr)` → `i128`. Held balance for `addr` in split `id`.
    Claimable(u64, Address),
    /// `Earned(id, addr)` → `i128`. Lifetime earnings for `addr` in split `id`.
    Earned(u64, Address),
    /// `Proposal(id)` → `AmendmentProposal`. Pending amendment for split `id`.
    Proposal(u64),
}

/// `NextId` lives in instance storage (shared contract-level state) because it
/// is a single monotonic counter, not per-split data.
#[soroban_sdk::contracttype]
enum InstanceKey {
    NextId,
}

// ── Instance storage helpers ─────────────────────────────────────────────────

/// Bump the contract's instance entry TTL (call on every entry point).
pub fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_BUMP_THRESHOLD, INSTANCE_BUMP_TO);
}

/// Returns the next split id and increments the counter atomically.
///
/// Initialises to 1 on first call. Uses instance storage so the counter
/// persists across all splits.
pub fn next_id(env: &Env) -> u64 {
    let current: u64 = env
        .storage()
        .instance()
        .get(&InstanceKey::NextId)
        .unwrap_or(0u64);
    let next = current + 1;
    env.storage().instance().set(&InstanceKey::NextId, &next);
    next
}

// ── Split read / write ────────────────────────────────────────────────────────

/// Persist a `Split`, extending its TTL.
pub fn save_split(env: &Env, split: &Split) {
    let key = DataKey::Split(split.id);
    env.storage().persistent().set(&key, split);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_BUMP_THRESHOLD, PERSISTENT_BUMP_TO);
}

/// Load a `Split` by id, extending its TTL on hit.
///
/// Returns `None` if no split with that id exists.
pub fn load_split(env: &Env, id: u64) -> Option<Split> {
    let key = DataKey::Split(id);
    let split: Option<Split> = env.storage().persistent().get(&key);
    if split.is_some() {
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_BUMP_THRESHOLD, PERSISTENT_BUMP_TO);
    }
    split
}

/// Returns `true` if the split exists and is in `Active` or `Locked` state.
#[allow(dead_code)]
pub fn split_is_active(env: &Env, id: u64) -> bool {
    match load_split(env, id) {
        Some(s) => matches!(s.status, SplitStatus::Active | SplitStatus::Locked),
        None => false,
    }
}

// ── Acceptance tracking ───────────────────────────────────────────────────────

/// Returns `true` if `addr` has already accepted split `id`.
pub fn is_accepted(env: &Env, id: u64, addr: &Address) -> bool {
    let key = DataKey::Accepted(id, addr.clone());
    let accepted: bool = env.storage().persistent().get(&key).unwrap_or(false);
    if accepted {
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_BUMP_THRESHOLD, PERSISTENT_BUMP_TO);
    }
    accepted
}

/// Record that `addr` has accepted split `id`.
pub fn set_accepted(env: &Env, id: u64, addr: &Address) {
    let key = DataKey::Accepted(id, addr.clone());
    env.storage().persistent().set(&key, &true);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_BUMP_THRESHOLD, PERSISTENT_BUMP_TO);
}

// ── Claimable balance ─────────────────────────────────────────────────────────

/// Returns the current held (claimable) balance for `addr` in split `id`.
pub fn get_claimable(env: &Env, id: u64, addr: &Address) -> i128 {
    let key = DataKey::Claimable(id, addr.clone());
    let amount: i128 = env.storage().persistent().get(&key).unwrap_or(0i128);
    if amount != 0 {
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_BUMP_THRESHOLD, PERSISTENT_BUMP_TO);
    }
    amount
}

/// Add `delta` to the claimable balance for `addr` in split `id`.
///
/// Returns the new balance, or `None` if the addition would overflow.
pub fn add_claimable(env: &Env, id: u64, addr: &Address, delta: i128) -> Option<i128> {
    let current = get_claimable(env, id, addr);
    let new_balance = current.checked_add(delta)?;
    let key = DataKey::Claimable(id, addr.clone());
    env.storage().persistent().set(&key, &new_balance);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_BUMP_THRESHOLD, PERSISTENT_BUMP_TO);
    Some(new_balance)
}

/// Set the claimable balance for `addr` in split `id` to zero (after a claim).
pub fn clear_claimable(env: &Env, id: u64, addr: &Address) {
    let key = DataKey::Claimable(id, addr.clone());
    env.storage().persistent().set(&key, &0i128);
    // No TTL bump needed; entry is effectively dead.
}

// ── Lifetime earnings ─────────────────────────────────────────────────────────

/// Returns the lifetime earnings for `addr` in split `id`.
pub fn get_earned(env: &Env, id: u64, addr: &Address) -> i128 {
    let key = DataKey::Earned(id, addr.clone());
    let amount: i128 = env.storage().persistent().get(&key).unwrap_or(0i128);
    if amount != 0 {
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_BUMP_THRESHOLD, PERSISTENT_BUMP_TO);
    }
    amount
}

/// Add `delta` to the lifetime earnings for `addr` in split `id`.
///
/// Returns the new total, or `None` on overflow.
pub fn add_earned(env: &Env, id: u64, addr: &Address, delta: i128) -> Option<i128> {
    let current = get_earned(env, id, addr);
    let new_total = current.checked_add(delta)?;
    let key = DataKey::Earned(id, addr.clone());
    env.storage().persistent().set(&key, &new_total);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_BUMP_THRESHOLD, PERSISTENT_BUMP_TO);
    Some(new_total)
}

// ── Amendment proposal ────────────────────────────────────────────────────────
//
// The full `AmendmentProposal` type is defined in `amend.rs`. These helpers
// are generic over the stored value type to keep `storage.rs` free of
// amendment-specific logic.

/// Returns `true` if there is an open amendment proposal for split `id`.
pub fn has_proposal(env: &Env, id: u64) -> bool {
    env.storage().persistent().has(&DataKey::Proposal(id))
}

/// Store an amendment proposal. The value type `T` must implement `IntoVal<Env, Val>`.
pub fn save_proposal<T>(env: &Env, id: u64, proposal: &T)
where
    T: soroban_sdk::IntoVal<Env, soroban_sdk::Val>,
{
    let key = DataKey::Proposal(id);
    env.storage().persistent().set(&key, proposal);
    env.storage()
        .persistent()
        .extend_ttl(&key, PERSISTENT_BUMP_THRESHOLD, PERSISTENT_BUMP_TO);
}

/// Load an amendment proposal, extending its TTL.
pub fn load_proposal<T>(env: &Env, id: u64) -> Option<T>
where
    T: soroban_sdk::TryFromVal<Env, soroban_sdk::Val>,
{
    let key = DataKey::Proposal(id);
    let proposal: Option<T> = env.storage().persistent().get(&key);
    if proposal.is_some() {
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_BUMP_THRESHOLD, PERSISTENT_BUMP_TO);
    }
    proposal
}

/// Remove an amendment proposal (after it is applied or cancelled).
pub fn remove_proposal(env: &Env, id: u64) {
    env.storage().persistent().remove(&DataKey::Proposal(id));
}
