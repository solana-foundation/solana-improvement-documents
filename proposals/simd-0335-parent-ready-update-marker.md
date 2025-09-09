---
simd: '0337'
title: ParentReadyUpdate Block Marker
authors:
  - ksn6 (Anza)
category: Standard
type: Core
status: Review
created: 2025-08-13
feature: TBD
---

## Summary

This SIMD proposes upgrading `BlockMarkerV1`, introduced in SIMD-0307, to
`BlockMarkerV2` to support fast leader handover in the Alpenglow consensus
protocol. In particular, `BlockMarkerV2` includes a new `ParentReadyUpdate`
variant to support fast leader handover. This marker signals when a leader
has switched to a different parent block during fast leader handover
due to a `ParentReady` event, providing critical metadata for consensus
validation and block verification.

## Motivation

The Alpenglow consensus protocol, as specified in SIMD-0326, introduces
fast leader handover as a key performance optimization. This mechanism
allows leaders to begin constructing blocks on tentative parent blocks before
the actual parent is fully finalized through the notarization process. This
optimization significantly reduces block production latency by enabling
parallel processing of block construction and parent finalization.

However, during fast leader handover, a leader may need to switch from
their initially assumed parent to a different parent block when a
`ParentReady` event occurs. This event indicates that:

1. All necessary skip certificates have been collected for intervening slots
2. A `NotarizeFallback` certificate exists for a valid parent block
3. The leader must switch to this newly ready parent (as specified in
   Algorithm 3, Line 7 of the
   [Alpenglow whitepaper v1.1](https://www.anza.xyz/alpenglow-1-1))


Currently, there is no standardized mechanism to signal these parent switches
within the block structure. As such, without the capability that this SIMD
details, leader validators cannot indicate that a parent switch has occurred
during fast leader handover.

This SIMD addresses these issues by leveraging the extensible `BlockMarker`
framework introduced in SIMD-0307 to add a dedicated marker for parent switch
events.

## New Terminology

- **Block Marker**: A chunk of structured non-transaction data that can be
  placed before, after, or in-between entry batches in a block.
- **Fast Leader Handover**: A technique where leaders begin constructing
  blocks on a tentative parent before the actual parent is finalized
- **ParentReady Event**: A consensus event indicating that a block has met
  all requirements to serve as a parent for new block production
- **Parent Switch**: The action of changing from one parent block to another
  during fast leader handover
- **ParentReadyUpdate**: A block marker that records when and to which parent
  a switch occurred during fast leader handover

## Detailed Design

This proposal extends the block marker system introduced in SIMD-0307 by
adding a new variant specifically for Alpenglow's fast leader handover
requirements.

### Block Marker Version 2

We introduce `BlockMarkerV2` as an extension of the original `BlockMarkerV1`:

```
BlockMarkerV2 Layout:
+---------------------------------------+
| Variant ID                  (1 byte)  |
+---------------------------------------+
| Variant Data Byte Length   (2 bytes)  |
+---------------------------------------+
| Variant Data              (variable)  |
+---------------------------------------+

Variants:
- 0: BlockFooter (inherited from V1)
- 1: ParentReadyUpdate (new in V2)
```

### ParentReadyUpdate Specification

The `ParentReadyUpdate` marker contains information about the new parent block
that the leader switched to during fast leader handover. Each variant includes
its own version field to support independent evolution:

```
VersionedParentReadyUpdate Layout:
+---------------------------------------+
| Version                     (1 byte)  |
+---------------------------------------+
| Payload (version-specific) (variable) |
+---------------------------------------+
```

For Version 1:

```
ParentReadyUpdateV1 Payload:
+---------------------------------------+
| Parent Slot                (8 bytes)  |
+---------------------------------------+
| Parent Block ID           (32 bytes)  |
+---------------------------------------+

Total payload size: 40 bytes
Total size with version: 41 bytes
```

Fields:

- **Version**: Versioning byte for forward compatibility (u8)
- **Parent Slot**: The slot number of the new parent block (u64,
  little-endian)
- **Parent Block ID**: The block ID identifying the new parent block (32-byte
  hash)

### Versioned Structure

Following the pattern established in SIMD-0307, we use a versioned approach
for future extensibility. We illustrate code samples below in Rust, although
implements may employ other languages:

```rust
/// Versioned parent ready update for fast leader handover in Alpenglow.
///
/// Used to signal when the parent changes due to ParentReady being triggered
/// on an earlier parent in fast leader handover. Always deserializes to the
/// `Current` variant for forward compatibility.
///
/// # Serialization Format
///
/// ┌─────────────────────────────────────────┐
/// │ Version                      (1 byte)   │
/// ├─────────────────────────────────────────┤
/// │ Update Data               (variable)    │
/// └─────────────────────────────────────────┘
pub enum VersionedParentReadyUpdate {
    V1(ParentReadyUpdateV1),
    Current(ParentReadyUpdateV1),
}

/// Version 1 parent ready update data.
///
/// Contains slot and block ID information for the new parent. Uses bincode
/// serialization for all fields to maintain consistency with other network data.
///
/// # Serialization Format
/// ┌─────────────────────────────────────────┐
/// │ Parent Slot                  (8 bytes)  │
/// ├─────────────────────────────────────────┤
/// │ Parent Block ID             (32 bytes)  │
/// └─────────────────────────────────────────┘
pub struct ParentReadyUpdateV1 {
    pub new_parent_slot: Slot,
    pub new_parent_block_id: Hash,
}
```

### Integration with BlockComponent

The `ParentReadyUpdate` marker integrates with the existing `BlockComponent` system:

```rust
/// Version 2 block marker with extended functionality.
///
/// Supports all V1 features plus ParentReadyUpdate for optimistic block
/// packing in Alpenglow.
///
/// # Serialization Format
/// ┌─────────────────────────────────────────┐
/// │ Variant ID                   (1 byte)   │
/// ├─────────────────────────────────────────┤
/// │ Byte Length                  (2 bytes)  │
/// ├─────────────────────────────────────────┤
/// │ Variant Data              (variable)    │
/// └─────────────────────────────────────────┘
///
/// The byte length field indicates the size of the variant data that follows,
/// allowing for proper parsing even if unknown variants are encountered.
pub enum BlockMarkerV2 {
    BlockFooter(VersionedBlockFooter),
    ParentReadyUpdate(VersionedParentReadyUpdate),
}
```

### Serialization Format

When serialized within a `BlockComponent`, the complete structure is:

```
+---------------------------------------+
| Entry Count = 0             (8 bytes) |
+---------------------------------------+
| Marker Version = 2          (2 bytes) |
+---------------------------------------+
| Variant ID = 1              (1 byte)  |
+---------------------------------------+
| Length = 41                 (2 bytes) |
+---------------------------------------+
| Version = 1                 (1 byte)  |
+---------------------------------------+
| Parent Slot                 (8 bytes) |
+---------------------------------------+
| Parent Block ID            (32 bytes) |
+---------------------------------------+

Total overhead: 54 bytes per marker
```

## Alternatives Considered

- **No Explicit Marking / Do Nothing**.
  Relying on implicit detection of parent switches was considered but rejected
  because of:
  - Ambiguity in determining whether a parent switch occurred vs. a duplicate
    block was received
  - Inability to validate leader behavior
  - Lack of auditability for fast leader handover decisions

- **Special Transaction Type**.
  Using a special transaction to signal parent switches was considered but
  rejected because:
  - Higher overhead due to transaction processing requirements
  - Having a transaction "undo" the state transitions induced by other
    transactions is ill-advised

- **Extended Block Footer**.
  Extending the block footer to include parent switch information was
  considered but is inadequate because:
  - Parent switches occur during block production, not at the end
  - The footer appears only at block completion
  - We cannot accurately represent the switch point within the block

- **Shred Modification**.
  Modifying the shred structure to include parent switch metadata was
  considered but rejected due to:
  - Significant protocol changes required
  - Backward compatibility challenges
  - Complexity of implementation
  - Impact on existing shred processing infrastructure

## Impact

- **Positive**:
  - Enables full implementation of Alpenglow's fast leader handover
  - Fast leader handover, upon implementation, reduces block production
    latency
  - Minimal space overhead (54 bytes per switch event)
  - Independent versioning allows evolution without affecting other variants

- **Negative**:
  - Slight increase in block size when parent switches occur

## Security Considerations

The introduction of this marker does not introduce any new security risks
until Alpenglow itself is fully implemented and launched. Upon launching
Alpenglow, we make the following considerations:

### Parent Switch Validation

When a `ParentReadyUpdate` marker is present, validators MUST perform rigorous
validation:

- Validators MUST verify that the referenced parent block exists and is valid
- The certificate pool must have issued the event `ParentReady(s, hash(b))`,
  where `s` denotes the first slot of the leader window and `hash(b)` denotes
  the hash (Block ID) of the new parent
- The new parent MUST have either a `NotarizeFallback` or `Notarize` certificate,
  followed by skip certificates until slot `s`
- The switch MUST occur only when a `ParentReady` event is triggered according
  to Algorithm 3, Line 7-8 of the [Alpenglow whitepaper v1.1](https://www.anza.xyz/alpenglow-1-1)

**State Transition Rules**:

- State changes from transactions before the `ParentReadyUpdate` marker are discarded
- The `ParentReadyUpdate` marker itself does not modify state
- Post-`ParentReadyUpdate` marker transactions are built on the new parent's state
- Notably, despite pre-marker transaction state changes being discarded, the
transactions themselves WILL be included in the constructed block.

### Attack Vectors and Mitigations

**Double-Parent Attack**: A malicious leader attempting to create blocks on
multiple parents simultaneously:

- **Mitigation**: Only a single `ParentReadyUpdate` marker per block is allowed.
  Upon witnessing two or more markers with different signatures, a receiving
  validator MUST report the block as invalid and vote skip
- **Future Enhancement**: Slashing conditions will be specified for leaders who
  disseminate multiple conflicting `ParentReadyUpdate` markers

**Invalid Parent Reference Attack**: Leader references a non-existent or
invalid parent:

- **Mitigation**: Invalid parent and/or slot fields in the `ParentReadyUpdate`
  will cause receiving validators to report the block invalid and vote skip
- **Slashing**: Under certain circumstances, such activity may be slashable
  (details will be specified in a future proposal)

**Arbitrary `ParentReadyUpdate`s**: Leader emits a spurious `ParentReadyUpdate`

- **Mitigation**: Leaders cannot arbitrarily trigger parent switches, since
the appropriate `ParentReady` event would not properly fire for receiving
validators, causing receiving validators to report the block.
- **Future Enhancement**: a spurious `ParentReadyUpdate` message may be
grounds for slashing in the future.

**Feature Flag Protection**:

- `ParentReadyUpdate` markers will be processed if and only if the Alpenglow
  feature is activated
- Validators must run with a client containing Alpenglow code to process blocks
  with `ParentReadyUpdate` markers
- Missing or malformed version fields will fail deserialization gracefully

## Backwards Compatibility

This SIMD requires SIMD-0307 to be implemented first, as it depends on the
`BlockMarker` framework.

Blocks containing `ParentReadyUpdate` markers are **not** backward compatible
with validators that do not support Alpenglow. Blocks will not disseminate
`ParentReadyUpdate` markers until the Alpenglow feature flag has been
activated.

**Important**: This SIMD should be activated independently from the main
Alpenglow SIMD. Validators must be prepared to handle `ParentReadyUpdate`
markers even if full Alpenglow consensus is not yet active. The marker
processing logic must be defensive and validate all parent references
regardless of Alpenglow activation status.

## Reference Implementation

A reference implementation is available here: https://github.com/anza-xyz/alpenglow/pull/364

This implementation includes:

- Full `BlockComponent` enumeration with marker support
- Serialization/deserialization logic
