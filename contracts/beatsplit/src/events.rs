//! Event types for BeatSplit.
//!
//! Uses `#[contractevent]` (soroban-sdk ≥ 23), the preferred API over the
//! deprecated `env.events().publish(...)` call.
//!
//! The macro emits the struct name as the first topic automatically.
//! Additional `#[topic]` fields are appended as further topics.
//! Non-`#[topic]` fields go into the event data payload.
//!
//! Event names match the README specification exactly.

use soroban_sdk::{contractevent, Address, Env};

// ── SplitCreated ─────────────────────────────────────────────────────────────

/// Emitted when a new split is created (status = Pending).
#[contractevent]
pub struct SplitCreated {
    #[topic]
    pub id: u64,
    pub creator: Address,
}

// ── Accepted ─────────────────────────────────────────────────────────────────

/// Emitted when a single recipient accepts their share.
#[contractevent]
pub struct Accepted {
    #[topic]
    pub id: u64,
    pub recipient: Address,
}

// ── Activated ────────────────────────────────────────────────────────────────

/// Emitted when the last recipient accepts, making the split Active.
#[contractevent]
pub struct Activated {
    #[topic]
    pub id: u64,
}

// ── Deposited ────────────────────────────────────────────────────────────────

/// Emitted on every deposit into an active split.
#[contractevent]
pub struct Deposited {
    #[topic]
    pub id: u64,
    pub from: Address,
    pub amount: i128,
}

// ── Payout ───────────────────────────────────────────────────────────────────

/// Emitted when a share is successfully transferred to a recipient.
#[contractevent]
pub struct Payout {
    #[topic]
    pub id: u64,
    pub recipient: Address,
    pub share: i128,
}

// ── Held ─────────────────────────────────────────────────────────────────────

/// Emitted when a transfer fails and the share is held for later claim.
#[contractevent]
pub struct Held {
    #[topic]
    pub id: u64,
    pub recipient: Address,
    pub share: i128,
}

// ── Claimed ──────────────────────────────────────────────────────────────────

/// Emitted when a recipient claims their held balance.
#[contractevent]
pub struct Claimed {
    #[topic]
    pub id: u64,
    pub recipient: Address,
    pub amount: i128,
}

// ── AmendmentProposed ─────────────────────────────────────────────────────────

/// Emitted when an amendment is proposed.
#[contractevent]
pub struct AmendmentProposed {
    #[topic]
    pub id: u64,
    pub proposer: Address,
}

// ── AmendmentApproved ─────────────────────────────────────────────────────────

/// Emitted when a recipient approves the current amendment proposal.
#[contractevent]
pub struct AmendmentApproved {
    #[topic]
    pub id: u64,
    pub approver: Address,
}

// ── AmendmentApplied ──────────────────────────────────────────────────────────

/// Emitted when all recipients have approved and the amendment is applied.
#[contractevent]
pub struct AmendmentApplied {
    #[topic]
    pub id: u64,
    /// New version number after the amendment.
    pub version: u32,
}

// ── AmendmentCancelled ────────────────────────────────────────────────────────

/// Emitted when an amendment proposal is cancelled.
#[contractevent]
pub struct AmendmentCancelled {
    #[topic]
    pub id: u64,
    pub canceller: Address,
}

// ── Locked ───────────────────────────────────────────────────────────────────

/// Emitted when a split is permanently locked.
#[contractevent]
pub struct Locked {
    #[topic]
    pub id: u64,
}

// ── Emitter helpers ───────────────────────────────────────────────────────────
//
// Thin wrappers so business logic in lib.rs stays readable.

pub fn split_created(env: &Env, id: u64, creator: &Address) {
    SplitCreated {
        id,
        creator: creator.clone(),
    }
    .publish(env);
}

pub fn accepted(env: &Env, id: u64, recipient: &Address) {
    Accepted {
        id,
        recipient: recipient.clone(),
    }
    .publish(env);
}

pub fn activated(env: &Env, id: u64) {
    Activated { id }.publish(env);
}

pub fn deposited(env: &Env, id: u64, from: &Address, amount: i128) {
    Deposited {
        id,
        from: from.clone(),
        amount,
    }
    .publish(env);
}

pub fn payout(env: &Env, id: u64, recipient: &Address, share: i128) {
    Payout {
        id,
        recipient: recipient.clone(),
        share,
    }
    .publish(env);
}

pub fn held(env: &Env, id: u64, recipient: &Address, share: i128) {
    Held {
        id,
        recipient: recipient.clone(),
        share,
    }
    .publish(env);
}

pub fn claimed(env: &Env, id: u64, recipient: &Address, amount: i128) {
    Claimed {
        id,
        recipient: recipient.clone(),
        amount,
    }
    .publish(env);
}

pub fn amendment_proposed(env: &Env, id: u64, proposer: &Address) {
    AmendmentProposed {
        id,
        proposer: proposer.clone(),
    }
    .publish(env);
}

pub fn amendment_approved(env: &Env, id: u64, approver: &Address) {
    AmendmentApproved {
        id,
        approver: approver.clone(),
    }
    .publish(env);
}

pub fn amendment_applied(env: &Env, id: u64, version: u32) {
    AmendmentApplied { id, version }.publish(env);
}

pub fn amendment_cancelled(env: &Env, id: u64, canceller: &Address) {
    AmendmentCancelled {
        id,
        canceller: canceller.clone(),
    }
    .publish(env);
}

pub fn locked(env: &Env, id: u64) {
    Locked { id }.publish(env);
}
