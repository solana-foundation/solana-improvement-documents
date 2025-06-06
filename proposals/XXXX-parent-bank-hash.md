---
simd: '0298'
title: Add Parent Bank Hash to Block Header
authors:
  - Max Resnick
category: Standard
type: Core
status: Idea
created: 2024-03-26
---

## Summary

This proposal adds the parent bank hash to the block header and introduces validity constraints to ensure the parent bank hash matches the expected value. This change is part of the broader set of SIMDs neccesary for async execution.

## Motivation

This proposal is neccesary for the initial rollout of asynchronous execution. Currently validators wait until they have finished executing the block before voting, causing delays on the critical path and getting in the way of longer term roadmap items like multiple concurrent proposers and shorter block times. Async execution requires voting before execution has finished which means that validators cannot include the `BankHash` in their votes and must therefore vote only on the `blockID`. If we remove `BankHash` from votes without any other changes then execution would not be a part of consensus at all, meaning if there is a bug in the runtime which causes a `BankHash` mismatch it could be difficult to identify quickly and resolve. This proposal moves consensus on the `BankHash` back one slot by adding the `BankHash` of the parent block to the first FEC set of each block so that we still have consensus on execution state eventually.

## New Terminology

Block Header: This is an umbrella term for information contained in the first FEC set of each block including `blockID`, parent `blockID`, parent Bank Hash etc...

## Detailed Design

### Block Header Changes

The block header structure will be extended to include a new field:

```rust
pub struct BlockHeader {
    // ... existing fields ...
    pub parent_bank_hash: Hash,  // Hash of the parent block's bank state
}
```

### Validity Constraints

In addition to the current validity constraints a block will be considered invalid if:

1. The parent_bank_hash does not match the hash of the parent block's bank state
2. The parent_bank_hash is missing (for blocks after the feature is activated)

### Implementation Details

1. The bank hash will be computed at the end of processing each block
2. The hash will be stored in the block metadata
3. Validators will verify the parent_bank_hash matches their computed hash of the parent block's state
4. The feature will be gated behind a feature flag

### Feature Activation

The feature can be activated at any time before or after the Alpenglow rollout.

This change will be implemented behind a feature flag and activated through the feature activation program. The activation will require:

1. Implementation of the new block header structure in shreds and the BlockStore
2. Updates to block production logic to include the additional information
3. Updates to block replay logic to verify the BankHash correctly corresponds to the parent's bank state

## Alternatives Considered

1. **Remove Execution from Consensus Entirely**: We could simply remove the BankHash from votes and not include it anywhere in the block structure. This would mean that execution bugs could go undetected for longer periods, making it harder to identify and resolve runtime issues. Eventually we want to remove this completely but for now it seems useful to keep training wheels on particularly as we rollout Firedancer's VM implementation. 

2. **XOR BankHash with BlockID**: Instead of adding a separate parent_bank_hash field, we could XOR the parent's BankHash with the BlockID to create a combined hash. This would save space but make verification more complex and less transparent. This approach was rejected because it adds unnecessary complexity to the validation process and makes debugging more difficult.

## Impact

Blocks now contains slightly more metadata bytes which cannot be used for transactions.

## Security Considerations

This change makes block validity more restrictive. No blocks would be valid under this proposal that would otherwise be invalid today.

## Backwards Compatibility

This change is not backwards compatible and requires a feature flag for activation. All validators must upgrade to support the new block header format and validation rules before the feature is activated.

## Implementation Plan

1. Create feature flag for parent bank hash
2. Implement block header changes
3. Update block validation logic
4. Update block production logic
5. Add tests for new validation rules
6. Deploy to testnet
7. Activate feature on mainnet-beta