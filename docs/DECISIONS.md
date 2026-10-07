# Architecture Decision Records

## ADR-001: distribute_balance Per-Split Tracking

**Date**: 2026-10-07
**Status**: Accepted

### Context

The `distribute_balance` function allows distributing tokens that were sent directly to the contract address (instead of via `deposit`). The challenge is tracking which split the direct transfer belongs to when multiple splits share the same token contract.

### Problem

A single BeatSplit contract can manage multiple splits. If two splits use the same token (e.g., both use USDC), and someone sends USDC directly to the contract address, there's no way to know which split the sender intended to fund. The naive approach of `contract_balance - total_received` would attribute the direct transfer to ALL splits using that token, leading to double-counting and potential double-spending.

### Decision

The `distribute_balance` function is implemented with the following constraints:

1. **It is UNSAFE for multi-split scenarios with shared tokens**. The implementation uses `contract_balance - total_received` for the specific split, but this calculation is only correct if:
   - The split uses a unique token not shared with any other split, OR
   - The caller guarantees no other splits share the token

2. **Documentation**: The function's docstring explicitly warns about this limitation.

3. **Alternative**: For safe multi-split deployments, each split should use a unique token, or users should always call `deposit` (which explicitly specifies the split ID) instead of sending tokens directly.

### Consequences

- **Positive**: Simple implementation, works correctly for single-split or unique-token deployments
- **Negative**: Caller must understand the limitation; not safe for arbitrary multi-split + shared-token deployments
- **Future**: If multi-asset support is added (planned feature), a proper per-split escrow mechanism should be designed

### Code Reference

See `contracts/beatsplit/src/lib.rs` lines 285-340 (`distribute_balance` function).