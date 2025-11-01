---
simd: '0298'
title: Add `bank_hash` to block footer.
authors:
  - Max Resnick
category: Standard
type: Core
status: Idea
created: 2024-03-26
---

## Summary

This proposal adds the bank hash to the block footer and introduces
validity constraints to ensure the bank hash matches the expected value.

## Motivation

This proposal is part of the changes required for Alpenglow. In Alpenglow,
votes do not contain the `bank_hash`. Instead they are purely on `block_id`
If we remove `bank_hash` from votes without any other
changes, then execution would not be a part of consensus at all, meaning if
there is a bug in the runtime which causes a `bank_hash` mismatch it could be
difficult to identify quickly and resolve. This proposal moves consensus on the
`bank_hash` to the block footer so that we still have consensus
on execution state.

## New Terminology

No new terminology but we recall the Block Footer structure and `bank_hash`

### Existing Block Footer Structure (SIMD 0307)

The block footer is implemented as a block marker variant that must occur
once after the last entry batch in a block. The footer contains general block
and producer metadata, along with any metrics that must be computed after the
block has been produced.

#### Block Marker Header

The block footer uses a block marker system to differentiate it from regular
entry batches. Entry batch data starts with an 8-byte value representing the
number of entries in the batch, which cannot be zero. The block marker header
begins with 8 zero bytes (the `block_marker_flag`), allowing replay parsers to
distinguish block markers from regular entry batches.

The block marker header structure is:

```
        Block Marker Header Layout
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| block_marker_flag      (64 bits of 0) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| version=1                   (16 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| variant                      (8 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| length                      (16 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|              --payload--              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

The block footer follows this structure:

```
     Block Marker Variant -- Footer
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| block_marker_flag      (64 bits of 0) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| version=1                   (16 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| variant=0                    (8 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| length                      (16 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| footer_version=1            (16 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| block_producer_time_nanos   (64 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| block_user_agent_len         (8 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| block_user_agent        (0-255 bytes) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**Existing Footer Fields:**

- `footer_version: u16` - Version number for the footer structure (currently 1)
- `block_producer_time_nanos: u64` - Nanosecond UNIX timestamp when the block
  producer started constructing the block
- `block_user_agent_len: u8` - Length of the user agent string in bytes
- `block_user_agent: String` - Variable length UTF-8 encoded string (up to 255
  bytes) identifying the block producer


#### Bank Hash Structure

The `bank_hash` represents the cryptographic hash of the bank state after
executing all transactions in a block. It serves as a commitment to the
complete execution state and is calculated from several components:

```
           Bank Hash Components
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| parent_bank_hash           (32 bytes) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| signature_count            (64 bits)  |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| last_blockhash             (32 bytes) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| accounts_lt_hash_checksum  (32 bytes) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

```

**Bank Hash Components:**

- `parent_bank_hash` - Hash of the parent bank's state (32 bytes)
- `signature_count` - Total number of signatures processed in the bank (64 bits)
- `last_blockhash` - The blockhash used for transaction processing (32 bytes)
- `accounts_lt_hash_checksum` - Checksum of the accounts ledger tree (32 bytes)
- `accounts` - Detailed account state information (variable size)

**Optional Component:**

- **Hard Fork Data** (when applicable) - If the current slot is a hard fork
  slot, additional hard fork data is hashed into the final result

The bank hash provides a cryptographic commitment to the complete execution
state, ensuring that any change to the bank's state (accounts, balances,
program state, etc.) will result in a different hash. This makes it a reliable
mechanism for consensus on execution results.

## Detailed Design

### Block Footer Changes

The block footer structure outlined in SIMD 0307
will be extended to include a new field. The updated footer structure will be:

```
     Block Footer (Extended)
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| block_marker_flag      (64 bits of 0) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| version=1                   (16 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| variant=0                    (8 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| length                      (16 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| footer_version=1            (16 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| block_producer_time_nanos   (64 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| block_user_agent_len         (8 bits) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| block_user_agent        (0-255 bytes) |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| bank_hash                  (32 bytes) |  ←NEW
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

**Note on Versioning**: While adding a field to the footer would typically warrant
a version increment, we maintain `footer_version=1` for simplicity. As of November
2024, clients do not yet disseminate block footers or block markers, making this
an appropriate time to extend the version 1 format before widespread adoption.

### Validity Constraints

In addition to the current validity constraints, a block will be considered
invalid if:

1. The `bank_hash` does not match the post execution state of the block
2. The `bank_hash` is missing (for blocks after the feature is activated)

### Implementation Details

1. The `bank_hash` will be computed at the end of packing each block.
The validator proposing the block will include it in the block footer.
2. Replaying validators will verify the `bank_hash` in the block footer
exists and matches their computed hash of the state after executing the block.
This is similar to what they do with the `bank_hash`
contained in votes today.

If the `bank_hash` does not match the post execution `bank_hash` then the block
is marked invalid. Validators will treat this the same way they treat a
protocol violatiing error. In particular, the block will be marked dead and
the validator will not vote on it or any of its children. Post alpenglow
the validator will vote skip on this block and any children.

### Feature Activation

The feature MUST be actiavted before or the Alpenglow rollout.

This change will be implemented behind a feature flag and activated through the
feature activation program. The activation will require:

1. Implementation of the new block footer structure
2. Updates to block production logic to include the `bank_hash`
3. Updates to block replay logic to verify the `bank_hash` correctly
   corresponds to the post execution bank state

## Alternatives Considered

1. **Remove Execution from Consensus Entirely**: We could simply remove the
   `bank_hash` from votes and not include it anywhere in the block structure. This
   would mean that execution bugs could go undetected for longer periods,
   making it harder to identify and resolve runtime issues. Eventually we want
   to remove this completely but for now it seems useful to keep training
   wheels on, particularly as we rollout Firedancer's VM implementation.

2. **XOR bank_hash with block_id**: Instead of adding a separate
   bank_hash field to the footer, we could XOR the `bank_hash` with the `block_id`
   to create a combined hash in votes. This would save space but would
   make verification more complex and less transparent.
   This approach was rejected because it adds
   unnecessary complexity to the validation process and makes debugging more difficult.

## Impact

Blocks now contain slightly more metadata bytes which cannot be used for
transactions.

## Security Considerations

This change makes block validity more restrictive. No blocks would be valid
under this proposal that would otherwise be invalid today.

## Backwards Compatibility

This change is not backwards compatible and requires a feature flag for
activation. All validators must upgrade to support the new block footer format
and validation rules before the feature is activated.
