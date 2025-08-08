---
simd: '0333'
title: Serialize Block ID in Bank into Snapshot
authors:
  - Emily Wang
category: Standard
type: Core
status: Review
created: 2025-08-06
feature:
---

## Summary

Currently the block ID (the merkle root of the last FEC set) of the
snapshot slot is not included in the snapshot Bank Fields. This would
enforce the snapshot producer to include the block ID in the serialized
bank.

## Motivation

It is convenient for multiple validator clients to have the block ID
when booting from a snapshot, and allows the inter-slot equivocation
block ID verification to not be special-cased on startup.  In addition,
this future-proofs for Alpenglow, as Alpenglow votes and related
tracking is done on (Slot, Block ID), and repair & duplicate block
resolution uses block ID. Having the block ID available in snapshots
enables this.

## New Terminology

N/A

## Detailed Design

The block_id already lives in the bank. Snapshot producers for all
clients must include the block ID of the snapshot slot in the serialized
bank of the snapshot. This can be implemented by adding a `block_id`
field to the snapshot serialization schema (eg. `BankFieldsToSerialize`
in Agave or `fd_snapshot_manifest_t` in Firedancer).

Snapshot consumers may optionally read the block ID in the deserialized
bank, and use it to validate as seen fit.

## Alternatives Considered

N/A

## Impact

Clients will now need to include an extra field when generating a
snapshot.

## Security Considerations

Security is improved because this now allows inter-slot verification
between the snapshot slot and its child without repairing the block ID
of the snapshot slot.

## Backwards Compatibility

This feature is not backwards compatible.
