use soroban_sdk::{contracttype, Address, BytesN, Vec};

/// A single recipient of a royalty split.
///
/// `bps` is the recipient's share expressed in basis points (1 bp = 0.01%).
/// All recipients in a split must sum to exactly 10,000 bps (= 100%).
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Recipient {
    /// Stellar address of this recipient.
    pub addr: Address,
    /// Share in basis points. Must be > 0.
    pub bps: u32,
}

/// Lifecycle state of a split.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum SplitStatus {
    /// Created; awaiting acceptance from all recipients before any deposits
    /// are distributed.
    Pending,
    /// All recipients have accepted. Deposits trigger immediate distribution.
    Active,
    /// Active and permanently immutable. No amendments are possible.
    Locked,
}

/// The on-chain record of a royalty split agreement.
#[contracttype]
#[derive(Clone, Debug)]
pub struct Split {
    /// Globally unique, auto-incrementing identifier.
    pub id: u64,
    /// Address that created the split. Required to authorize creation.
    pub creator: Address,
    /// Address of the Stellar Asset Contract token used for distributions
    /// (e.g. USDC SAC address).
    pub token: Address,
    /// Ordered list of recipients. The first entry receives any rounding
    /// remainder ("dust") from integer division.
    pub recipients: Vec<Recipient>,
    /// Current lifecycle state.
    pub status: SplitStatus,
    /// SHA-256 hash of the off-chain metadata JSON (title, ISRC, notes, etc.).
    /// Anchors the off-chain agreement to the on-chain record.
    pub metadata_hash: BytesN<32>,
    /// Cumulative total of all amounts deposited into this split, in the
    /// token's smallest unit (USDC has 7 decimals).
    pub total_received: i128,
    /// Increments on every successful amendment. Starts at 1 at creation.
    pub version: u32,
}
