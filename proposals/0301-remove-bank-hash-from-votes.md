---
simd: "0301"
title: Remove BankHash from Votes
authors:
  - Max Resnick (Anza)
category: Standard
type: Core
status: Draft
created: 2025-06-10
feature: (fill in with feature key and github tracking issues once accepted)
supersedes: (optional - fill this in if the SIMD supersedes a previous SIMD)
superseded-by: (optional - fill this in if the SIMD is superseded by a subsequent SIMD)
extends: (optional - fill this in if the SIMD extends the design of a previous SIMD)
---

## Summary

This proposal would remove BankHash from votes on Solana to allow voting before replay is complete. If all the prerequisites are satisfied before Alpenglow is ready to rollout then it can be activated with Alpenglow. Otherwise it must be a feature flag.

## Motivation

Currently, votes include the BankHash, which ties votes to a fully executed bank state. Removing BankHash from votes allows validators to vote before execution completes. This is also called Asynchronous Execution.

The synchronous confirmation flow after Alpenglow looks like this:

```text
| ----- leader broadcasts the block through rotor -------|
        |------------------ validator recieves block ------------|       |-- validator broadcasts votes--| <-  certificate is formed
              |--------------- validator executes block -----------------|
```

After this feature flag activates validators will be able to vote before they finish executing the block:

```text
| ----- leader broadcasts the block through rotor -------|
        |------------------ validator recieves block ------------||-- validator broadcasts votes--| <-  certificate is formed
              |--------------- validator executes block -----------------|
```

## Detailed Design

This proposal contains a single change:

1. Remove BankHash from votes. Validators will no longer include the BankHash in their vote messages. The vote structure and any related consensus logic will be updated accordingly.

## Dependencies

| SIMD Number                                                                             | Description                                 | Status    |
| --------------------------------------------------------------------------------------- | ------------------------------------------- | --------- |
| [SIMD-0159](https://github.com/solana-foundation/solana-improvement-documents/pull/159) | Pre-compile Instruction Verification        | Activated |
| [SIMD-0191](https://github.com/solana-foundation/solana-improvement-documents/pull/191) | Loaded Data Size and Program Account Checks | Activated |
| [SIMD-0192](https://github.com/solana-foundation/solana-improvement-documents/pull/192) | Address Lookup Table Relaxation             | Review    |
| [SIMD-0290](https://github.com/solana-foundation/solana-improvement-documents/pull/290) | Fee-payer Account Relaxation                | Review    |
| [SIMD-0295](https://github.com/solana-foundation/solana-improvement-documents/pull/295) | Relax CU limit overage                      | Review    |
| [SIMD-0297](https://github.com/solana-foundation/solana-improvement-documents/pull/297) | Durable Nonce Relaxation                    | Review    |
| [SIMD-0298](https://github.com/solana-foundation/solana-improvement-documents/pull/298) | Add BankHash to Block Header                | Review    |

## Impact

This change is not expected to have any direct impact on app developers. It is a consensus-breaking change and will require all validators to update their software to the new vote structure. Validators running older versions will be unable to participate in consensus once the feature flag is activated.

## Security Considerations

Removing BankHash from votes does not introduce new security risks by itself, but it is a foundational change for future protocol improvements. Care must be taken to ensure that consensus remains robust and that the transition is coordinated across the validator network.

## Drawbacks

This is a breaking change to the vote structure, requiring a coordinated upgrade across the network.

## Backwards Compatibility

This proposal requires a breaking change to the vote structure. All validators must implement the new vote format. The change will be gated behind a feature flag and activated in a coordinated manner. Validators running older versions will be unable to participate in consensus once the feature flag is activated.
