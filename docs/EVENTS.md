# BeatSplit Contract Events Reference

This document describes every event emitted by the BeatSplit Soroban contract. All events follow the `#[contractevent]` macro format (soroban-sdk ≥ 23), which emits the struct name as the first topic automatically. Additional `#[topic]` fields are appended as further topics. Non-`#[topic]` fields go into the event data payload.

All events include the split `id` as an indexed topic (first topic after the event name) to enable efficient filtering by split.

## Event List

### 1. SplitCreated

Emitted when a new split is created (status = `Pending`).

| Topic | Name | Type | Description |
|-------|------|------|-------------|
| 0 (auto) | — | string | Event name: `"SplitCreated"` |
| 1 | `id` | u64 | Unique split identifier |
| — | `creator` | Address | Address that created the split |

**Data payload:**
```json
{
  "creator": "G...creator address..."
}
```

---

### 2. Accepted

Emitted when a single recipient accepts their share.

| Topic | Name | Type | Description |
|-------|------|------|-------------|
| 0 (auto) | — | string | Event name: `"Accepted"` |
| 1 | `id` | u64 | Unique split identifier |
| 2 | `recipient` | Address | Address of the accepting recipient |

**Data payload:** (empty)

---

### 3. Activated

Emitted when the last recipient accepts, making the split `Active`.

| Topic | Name | Type | Description |
|-------|------|------|-------------|
| 0 (auto) | — | string | Event name: `"Activated"` |
| 1 | `id` | u64 | Unique split identifier |

**Data payload:** (empty)

---

### 4. Deposited

Emitted on every deposit into an active or locked split.

| Topic | Name | Type | Description |
|-------|------|------|-------------|
| 0 (auto) | — | string | Event name: `"Deposited"` |
| 1 | `id` | u64 | Unique split identifier |
| 2 | `from` | Address | Address of the payer |
| — | `amount` | i128 | Deposit amount in token's smallest unit |

**Data payload:**
```json
{
  "from": "G...payer address...",
  "amount": 100000000
}
```

---

### 5. Payout

Emitted when a share is successfully transferred to a recipient.

| Topic | Name | Type | Description |
|-------|------|------|-------------|
| 0 (auto) | — | string | Event name: `"Payout"` |
| 1 | `id` | u64 | Unique split identifier |
| 2 | `recipient` | Address | Address of the recipient |
| — | `share` | i128 | Amount transferred in token's smallest unit |

**Data payload:**
```json
{
  "recipient": "G...recipient address...",
  "share": 50000000
}
```

---

### 6. Held

Emitted when a transfer fails and the share is held for later claim.

| Topic | Name | Type | Description |
|-------|------|------|-------------|
| 0 (auto) | — | string | Event name: `"Held"` |
| 1 | `id` | u64 | Unique split identifier |
| 2 | `recipient` | Address | Address of the recipient whose transfer failed |
| — | `share` | i128 | Amount held in token's smallest unit |

**Data payload:**
```json
{
  "recipient": "G...recipient address...",
  "share": 30000000
}
```

---

### 7. Claimed

Emitted when a recipient claims their held balance.

| Topic | Name | Type | Description |
|-------|------|------|-------------|
| 0 (auto) | — | string | Event name: `"Claimed"` |
| 1 | `id` | u64 | Unique split identifier |
| 2 | `recipient` | Address | Address of the claiming recipient |
| — | `amount` | i128 | Amount claimed in token's smallest unit |

**Data payload:**
```json
{
  "recipient": "G...recipient address...",
  "amount": 30000000
}
```

---

### 8. AmendmentProposed

Emitted when an amendment is proposed.

| Topic | Name | Type | Description |
|-------|------|------|-------------|
| 0 (auto) | — | string | Event name: `"AmendmentProposed"` |
| 1 | `id` | u64 | Unique split identifier |
| 2 | `proposer` | Address | Address of the proposing recipient |

**Data payload:**
```json
{
  "proposer": "G...proposer address..."
}
```

---

### 9. AmendmentApproved

Emitted when a recipient approves the current amendment proposal.

| Topic | Name | Type | Description |
|-------|------|------|-------------|
| 0 (auto) | — | string | Event name: `"AmendmentApproved"` |
| 1 | `id` | u64 | Unique split identifier |
| 2 | `approver` | Address | Address of the approving recipient |

**Data payload:**
```json
{
  "approver": "G...approver address..."
}
```

---

### 10. AmendmentApplied

Emitted when all current recipients have approved and the amendment is applied atomically.

| Topic | Name | Type | Description |
|-------|------|------|-------------|
| 0 (auto) | — | string | Event name: `"AmendmentApplied"` |
| 1 | `id` | u64 | Unique split identifier |
| — | `version` | u32 | New version number after the amendment |

**Data payload:**
```json
{
  "version": 2
}
```

---

### 11. AmendmentCancelled

Emitted when an amendment proposal is cancelled.

| Topic | Name | Type | Description |
|-------|------|------|-------------|
| 0 (auto) | — | string | Event name: `"AmendmentCancelled"` |
| 1 | `id` | u64 | Unique split identifier |
| 2 | `canceller` | Address | Address of the recipient who cancelled |

**Data payload:**
```json
{
  "canceller": "G...canceller address..."
}
```

---

### 12. Locked

Emitted when a split is permanently locked (all recipients have approved locking).

| Topic | Name | Type | Description |
|-------|------|------|-------------|
| 0 (auto) | — | string | Event name: `"Locked"` |
| 1 | `id` | u64 | Unique split identifier |

**Data payload:** (empty)

---

## Indexer Implementation Notes

### Topic Structure

All events have at minimum 2 topics:
- Topic 0: Event name (automatically added by `#[contractevent]`)
- Topic 1: `id` (the split identifier, always indexed)

Events with an address field (`creator`, `recipient`, `from`, `proposer`, `approver`, `canceller`) have that address as Topic 2.

This allows efficient filtering:
- **By split**: Filter on Topic 1 = `id`
- **By split + recipient**: Filter on Topic 1 = `id` AND Topic 2 = `recipient`
- **All events for an address**: Filter on Topic 2 = `address` (requires scanning all splits)

### Reconstructing State

To rebuild the full state of a split from events:

1. **SplitCreated** → Initialize split record with `id`, `creator`, `status = Pending`
2. **Accepted** → Track acceptances per recipient
3. **Activated** → Set `status = Active`
4. **Deposited** → Increment `total_received`, record deposit
5. **Payout** / **Held** → Update per-recipient `earned` and `claimable` balances
6. **Claimed** → Move from `claimable` to `earned` (or external wallet)
7. **AmendmentProposed** → Record pending proposal
8. **AmendmentApproved** → Track approvals
9. **AmendmentApplied** → Apply new recipient list, increment `version`, reset acceptances
10. **AmendmentCancelled** → Clear pending proposal
11. **Locked** → Set `status = Locked`

### Version Tracking

The `AmendmentApplied` event includes the new `version` number. The split's `version` starts at 1 at creation and increments by 1 on each applied amendment. Indexers should track this to detect stale proposals.

### Dust Handling

The `Payout` event for the first recipient (index 0) will include any rounding remainder ("dust"). Indexers should be aware that the first recipient's share may be slightly larger than `amount * bps / 10000` due to integer division flooring.

### Error Handling

Events are only emitted on successful state changes. Failed transactions (reverts) produce no events. Indexers should only process events from successful transactions.

### Example Event Stream

For a split with 2 recipients (A: 5000 bps, B: 5000 bps):

```
SplitCreated  { id: 1, creator: G...A }
Accepted      { id: 1, recipient: G...A }
Accepted      { id: 1, recipient: G...B }
Activated     { id: 1 }
Deposited     { id: 1, from: G...payer, amount: 100000000 }
Payout        { id: 1, recipient: G...A, share: 50000000 }
Payout        { id: 1, recipient: G...B, share: 50000000 }
Deposited     { id: 1, from: G...payer2, amount: 50000000 }
Payout        { id: 1, recipient: G...A, share: 25000000 }
Payout        { id: 1, recipient: G...B, share: 25000000 }
AmendmentProposed { id: 1, proposer: G...A }
AmendmentApproved { id: 1, approver: G...A }
AmendmentApproved { id: 1, approver: G...B }
AmendmentApplied  { id: 1, version: 2 }
Accepted      { id: 1, recipient: G...A }  (re-accept after amendment)
Accepted      { id: 1, recipient: G...B }
Accepted      { id: 1, recipient: G...C }  (new recipient)
Activated     { id: 1 }
Locked        { id: 1 }
```

---

## Contract Version

This event schema corresponds to BeatSplit contract v0.1 (MVP). Event names, topic structure, and data layouts are considered stable within a major version. Breaking changes will only occur in major version increments with advance notice.