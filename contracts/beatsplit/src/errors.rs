use soroban_sdk::contracterror;

/// All error conditions the BeatSplit contract can return.
///
/// Numeric codes are stable ABI identifiers exposed to callers.
/// Do not reuse or reorder codes.
#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Error {
    // ── Recipient / share validation ────────────────────────────────────────

    /// The recipient list must contain between 2 and 20 entries (inclusive).
    /// A single-recipient split is not meaningful; more than 20 is unbounded
    /// resource usage.
    InvalidRecipientCount = 1,

    /// The sum of all `bps` values in the recipient list must equal exactly
    /// 10,000 (= 100%). Any other total is rejected.
    BpsTotalInvalid = 2,

    /// Every recipient must have `bps > 0`. A zero-share entry would receive
    /// nothing and serves no purpose.
    ZeroShare = 3,

    /// Each address in the recipient list must be unique. Duplicate addresses
    /// would silently double-count shares or create ambiguity in claimable
    /// balances.
    DuplicateRecipient = 4,

    // ── Deposit / amount validation ─────────────────────────────────────────

    /// The deposit or transfer amount must be strictly positive. Zero or
    /// negative amounts are rejected.
    InvalidAmount = 5,

    // ── Split lookup / lifecycle ─────────────────────────────────────────────

    /// No split with the given `id` exists in storage.
    SplitNotFound = 6,

    /// The operation requires the split to be in `Active` or `Locked` state,
    /// but it is currently `Pending`.
    NotActive = 7,

    /// The operation requires the split to be in `Pending` state (e.g.
    /// `accept`), but it is already `Active` or `Locked`.
    NotPending = 8,

    /// The caller is not in the split's recipient list and therefore has no
    /// authority to perform this operation.
    NotARecipient = 9,

    /// The recipient has already accepted this split. Double-accepting is a
    /// no-op guard rather than a silent pass-through.
    AlreadyAccepted = 10,

    // ── Arithmetic safety ───────────────────────────────────────────────────

    /// A checked arithmetic operation (add, multiply) overflowed. This should
    /// be unreachable under normal use with `i128` and bounded recipient counts,
    /// but is always checked defensively.
    Overflow = 11,

    // ── Amendment errors ─────────────────────────────────────────────────────

    /// An amendment proposal is already open for this split. Only one proposal
    /// can be pending at a time; the current one must be applied or cancelled
    /// first.
    AmendmentAlreadyOpen = 12,

    /// There is no open amendment proposal to approve or cancel.
    NoOpenAmendment = 13,

    /// The caller has already approved the current amendment proposal.
    AlreadyApprovedAmendment = 14,

    // ── Immutability ─────────────────────────────────────────────────────────

    /// The split has been permanently locked. No amendments, status changes,
    /// or other mutations are permitted.
    SplitLocked = 15,

    // ── Claim errors ─────────────────────────────────────────────────────────

    /// The recipient has no held (claimable) balance for this split.
    NothingToClaim = 16,
}
