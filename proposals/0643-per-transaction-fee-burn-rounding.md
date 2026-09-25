---
simd: '0643'
title: Per-Transaction Fee Burn Rounding
authors:
  - Andrew Fitzgerald (Anza)
category: Standard
type: Core
status: Draft
created: 2026-09-17
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Round fee burn per transaction instead of per slot.

## Motivation

Rounding per transaction simplifies fee burning. Burning half of a transaction's
base fee naturally describes a per-transaction calculation. Summing base fees
for the slot before rounding is an unnecessary complication.

Per-slot rounding also requires accumulating `(base_fee, priority_fee)`.
Per-transaction rounding allows accumulating `(burned, collected)` instead.
This supports future changes that burn priority fees in certain circumstances
without complicating slot-level fee accounting.

## New Terminology

None.

## Detailed Design

For each transaction charged fees, calculate the following in lamports:

```text
burned = floor(base_fee / 2)
collected = base_fee - burned + priority_fee
```

Accumulate `(burned, collected)` for the slot. The slot's burn and reward
amounts MUST be the sums of these per-transaction amounts. Fractional lamports
MUST NOT carry between transactions.
This applies to all transaction formats, including transactions charged fees
that fail loading or execution. Transactions charged no fees contribute zero.

Fee amounts charged to payers, reward distribution timing, and handling of
rewards that cannot be deposited are unchanged.

## Alternatives Considered

- Keep per-slot rounding. This retains the complication in fee accounting.

## Impact

Fees charged to users are unchanged. For every pair of transactions with odd
base fees in a slot, one fewer lamport is burned and one more is collected for
rewards. Transactions with even base fees are unaffected.

## Security Considerations

Requires a feature gate to avoid forking the network.

## Backwards Compatibility

Transaction validity and fees charged are unchanged. Burn and reward totals may
differ after activation, so all validators must use the same rounding rule.

## Conformance

Verify zero, even, and odd base fees, including multiple odd fees in one slot,
nonzero priority fees, and failed transactions charged fees. Verify both sides
of feature activation.
