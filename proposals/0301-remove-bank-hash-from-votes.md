---
simd: "0301"
title: Replace bank_hash with parent_bank_hash
authors:
  - Max Resnick (Anza)
category: Standard
type: Core
status: Review
created: 2025-06-10
feature: 
supersedes: SIMD 0298
superseded-by:
extends:
---

## Summary

This proposal would replace the `bank_hash` in the block footer introduced by
SIMD 0298 with the `parent_bank_hash`.

## Motivation

Asynchronous execution would not require execution to verify the validity of
the block. Enforcing the validity check of the current block's `bank_hash`
against the post execution state is not compatible with asynchronous execution.
This proposal moves consensus on bank state back one slot.

## New Terminology

- **parent_bank_hash**: The bank hash from the previous slot, replacing the
  current slot's bank_hash in the block footer

## Detailed Design

This proposal a simple change:

1. Replace `bank_hash` in block footer with `parent_bank_hash`
2. Replace the validity logic requires a valid `bank_hash` against the post
   execution state of the block with a check that the pre execution state
   matches the `parent_bank_hash`

## Dependencies

| SIMD Number | Description | Status |  
| ----------- | ----------- | ------ |
| [SIMD-0159](../pull/159) | Pre-compile Instruction Verification | Activated |
| [SIMD-0191](../pull/191) | Loaded Data Size and Program Account Checks | Activated |
| [SIMD-0192](../pull/192) | Address Lookup Table Relaxation | Review |
| [SIMD-0290](../pull/290) | Fee-payer Account Relaxation | Approved |
| [SIMD-0295](../pull/295) | Relax CU limit overage | Review |
| [SIMD-0297](../pull/297) | Durable Nonce Relaxation | Review |
| [SIMD-0298](../pull/298) | Add bank_hash to block footer | Review |

## Alternatives Considered

1. **Keep current bank_hash**: Continue with the current design from SIMD 0298,
   but this would not be compatible with asynchronous execution.
2. **Remove bank_hash entirely**: Remove all execution state from consensus,
   but this would make it harder to detect execution bugs.

## Impact

This change is not expected to have any direct impact on app developers. It is a
consensus-breaking change and will require all validators to update their
software to the new vote structure. Validators running older versions will be
unable to participate in consensus once the feature flag is activated.

## Security Considerations

This would delay the detection of `bank_hash` mismatches due to bugs in
execution client implementations by 1 block.

## Drawbacks

## Backwards Compatibility

This proposal requires a breaking change to block validity. All validators
the change. The change will be gated behind a feature
flag and activated in a coordinated manner. May differ in their view of valid
blocks until they upgrade.