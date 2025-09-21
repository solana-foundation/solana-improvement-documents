---
simd: '0366'
title: Enforce DATA_COMPLETE_SHRED Placement
authors:
  - ksn6 (Anza)
category: Standard
type: Core
status: Review
created: 2025-09-21
feature: https://github.com/anza-xyz/agave/pull/8099
---

## Summary

This SIMD enforces that the `DATA_COMPLETE_SHRED` flag can only be set on the
final data shred within an FEC (Forward Error Correction) set. One key
use-case for this SIMD: this restriction enables efficient detection of
soon-to-be-introduced `BlockComponent`s at shred ingestion time, providing
performance improvements for critical consensus operations. The alternative to
this SIMD would be to detect `BlockComponent`s during replay, which would be
substantially slower. One key example is `UpdateParent` detection in
Alpenglow's fast leader handover; detection during replay would not be ideal,
as malicious leaders could intentionally misplace `DATA_COMPLETE_SHRED` flags
to force validators into expensive search operations, creating DoS attack
vectors and delaying block repairs.

## Motivation

Currently, the `DATA_COMPLETE_SHRED` flag can theoretically be set on any data
shred within an FEC set, though in practice it marks FEC set boundaries. This
ambiguity creates significant performance challenges:

1. **Expensive BlockComponent Detection**: Without guaranteed placement,
   detecting `BlockComponent`s requires expensive searches across multiple
   shreds and FEC sets.

2. **Performance Impact**: alternative `BlockComponent` methods require either
   complex inference algorithms or expensive replay operations.

3. **Security Vulnerabilities**: Malicious leaders could intentionally
   misplace `DATA_COMPLETE_SHRED` flags to force validators into expensive
   search operations, creating DoS attack vectors; for certain proposed
   `BlockComponent`s (specifically, `UpdateParent`), malicious leaders could
   delay block repairs for following leaders.

By enforcing `DATA_COMPLETE_SHRED` placement only on the last data shred in
each FEC set, validators can efficiently detect `BlockComponent` boundaries
during online shred ingestion.

## Forward Dependencies

This proposal is required for:

- **[SIMD-0337]: UpdateParent Marker for Alpenglow Fast Leader Handover**

    Alpenglow's fast leader handover feature depends on efficient
    UpdateParent detection, which this SIMD enables through guaranteed FEC set
    boundary markers.

[SIMD-0337]: https://github.com/solana-foundation/solana-improvement-documents/pull/337

## New Terminology

- **FEC Set**: A fixed-size group of exactly 32 data shreds plus associated
  coding shreds used for erasure coding and error correction.

- **FEC Set Boundary**: The transition point between consecutive FEC sets,
  marked by the `DATA_COMPLETE_SHRED` flag on the final data shred of each
  set.

- **BlockComponent**: A control element within a block that provides metadata
  or instructions for block processing. `BlockComponent`s are aligned with FEC
  set boundaries when the previous shred has `DATA_COMPLETE_SHRED` set. E.g.,
  see https://github.com/anza-xyz/agave/pull/8100 for an implementation of
  `BlockComponent` within Agave.

## Detailed Design

### FEC Set Structure

FEC sets in Solana have a fixed structure:

- Each FEC set contains exactly 32 data shreds
- Each FEC set contains exactly 32 coding shreds for error correction
- FEC sets are never partial - if insufficient data exists to fill a complete
  set, the remaining positions are zero-padded

### `DATA_COMPLETE_SHRED` Placement Rules

The following rules MUST be enforced by all validator implementations:

1. **Placement Restriction**: The `DATA_COMPLETE_SHRED` flag MUST only be set
   to `true` on the final data shred within an FEC set (index position 31
   within the set, or more generally at positions where
   `(shred_index + 1) % 32 == 0` for the data shred's position within the
   slot).

2. **Validation Requirement**: Validators MUST check `DATA_COMPLETE_SHRED`
   placement during shred ingestion (either at turbine receipt or blockstore
   insertion).

3. **Invalid Placement Handling**: If a validator detects
   `DATA_COMPLETE_SHRED` set to `true` on any data shred other than the last
   one in an FEC set:
   - The validator MUST mark the entire slot as dead

4. **Zero-Padding Behavior**: For the final FEC set in a block that contains
   fewer than 32 data shreds:
   - The `DATA_COMPLETE_SHRED` flag MUST be set on the last actual data shred
     before zero-padding
   - The remaining positions in the FEC set MUST be zero-padded to maintain
     the fixed 32-shred structure

### `BlockComponent` Alignment

With enforced `DATA_COMPLETE_SHRED` placement:

1. **Boundary Detection**: `BlockComponent`s start at the beginning of an FEC
   set when the previous FEC set's last data shred has `DATA_COMPLETE_SHRED`
   set to `true`.

2. **Efficient Lookup**: `BlockComponent`s can be detected by checking only
   specific, predictable shred positions rather than searching across multiple
   shreds.

3. **Online Processing**: `BlockComponent` detection can occur during shred
   ingestion without requiring batch processing or replay.

## Use Case: Alpenglow Fast Leader Handover

Alpenglow's fast leader handover feature relies on an `UpdateParent`
`BlockComponent` to signal parent slot changes. With this SIMD:

1. **Current Challenge**: Without guaranteed `DATA_COMPLETE_SHRED` placement,
   detecting `UpdateParent` requires:
   - Searching across multiple shreds in potentially multiple FEC sets
   - Complex inference algorithms to guess `BlockComponent` locations
   - Or expensive replay operations to determine `UpdateParent` positions

2. **With This SIMD**: `UpdateParent` detection becomes deterministic:
   - Check the 0th shred of the next FEC set when `DATA_COMPLETE_SHRED` is
     true
   - Check the current shred when transitioning from a `DATA_COMPLETE_SHRED`
     boundary
   - No searching or inference required
   - See https://github.com/anza-xyz/alpenglow/pull/459 for an example implementation.

3. **Performance Impact**: This optimization will provide performance
   improvements to alternative implementations in `UpdateParent` detection
   latency, and more broadly, `BlockMarker` detection latency, enabling
   Alpenglow's fast leader handover to operate efficiently at scale.

## Alternatives Considered

### Complex Inference Algorithms

Implement sophisticated algorithms to infer `BlockComponent` locations through
partial data analysis.

**Rejected because**: Substantially more complex to maintain and test, slower
than the proposed approach, and prone to edge cases.

### Replay-Based Detection

Determine `UpdateParent` locations through transaction replay after full block
receipt.

**Rejected because**: Substantially slower than online detection, increases
block confirmation latency, requires significant computational resources, and
prone to DoS attacks from malicious actors.

### Status Quo

Continue allowing `DATA_COMPLETE_SHRED` placement anywhere within FEC sets.

**Rejected because**: Maintains current performance limitations and security
vulnerabilities.

## Impact

### Validator Implementations

All validator clients (Agave, Firedancer, Jito, etc.) MUST implement the
validation logic to check `DATA_COMPLETE_SHRED` placement and mark violating
slots as dead.

### Block Production

Block producers MUST ensure they only set `DATA_COMPLETE_SHRED` on the final
data shred in each FEC set. Most implementations already follow this pattern.

### Existing Infrastructure

- No impact on RPC methods or APIs
- No changes required for block explorers
- No impact on existing ledger data (validation only applies to new blocks)

## Security Considerations

### DoS Attack Prevention

By enforcing predictable `DATA_COMPLETE_SHRED` placement, this SIMD prevents
malicious leaders from:

- Forcing validators into expensive `BlockComponent` searches
- Specifically, with `UpdateParent` in Alpenglow fast leader handover, causing
  delayed block repairs

### Deterministic Validation

All validators will deterministically agree on whether a block has valid
`DATA_COMPLETE_SHRED` placement, preventing consensus splits due to
implementation differences.

### Consequences of Invalid `DATA_COMPLETE_SHRED` Placement

Blocks with invalid `DATA_COMPLETE_SHRED` placement are marked as dead if they
contain valid transactions.

## Backwards Compatibility

This change maintains full backwards compatibility:

1. **Existing Ledgers**: Validation only applies to newly produced blocks.
   Historical ledger data is not re-validated.

2. **Rollout Strategy**: The enforcement will be controlled by a feature gate
   (`discard_unexpected_data_complete_shreds`).

3. **Version Compatibility**: Validators running older versions will continue
   to accept blocks with misplaced `DATA_COMPLETE_SHRED` flags until they
   upgrade.

4. **No Replay Required**: Existing snapshots and ledger data remain valid
   without any migration or replay necessary.
