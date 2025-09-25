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

Block Footer: This is an umbrella term for information contained in the last
FEC set.
The exact structure is described in SIMD 0307.

## Detailed Design

### Block Footer Changes

The block footer structure outlined in SIMD 0307
will be extended to include a new field:

```rust
pub struct BlockFooter {
    // ... existing fields ...
    pub bank_hash: Hash,  // Hash of the block's bank state
}
```

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

The feature can be activated at any time before or after the Alpenglow rollout.

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
