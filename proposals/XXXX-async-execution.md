---
# TODO: After opening the PR, update 'simd' field and filename to match the PR number (e.g., simd: '0123', proposals/0123-async-execution.md)
# Status: Draft PR – for community and core contributor feedback

simd: 'XXXX'
title: Asynchronous Execution
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

<!--
NOTE: After opening the PR, update the 'simd' field above and the filename to match the PR number assigned by GitHub.
-->

## Summary

This proposal represents the final feature flag for asynchronous execution on Solana. It will remove BankHash from votes, leaving behind only the BlockID and therefore allow validators to vote on blocks before they have finished executing them. the feature flag is intended to be activated after Alpenglow activates.

## Motivation

Asynchronous execution removes replay from the critical path of consensus, allowing validators to vote on blocks before they are finished replaying them. This will reduce end to end confirmation latency somewhat after Alpenglow. But aside from the small reduction in end to end latency, Asynchronous execution is a key blocker of both Multiple Concurrent Leaders (MCL) and pipelined consensus (faster than global round trip slot times).

## New Terminology

Throughout this SIMD when we refer to Asynchronous Execution we are reffering to validators voting on blocks before they have finished replaying them. There is also the matter of "Bankless Leader" which would allow the leader to propose a block without executing transactions or knowing the parent bank state. This proposal is about moving us to Asynchronous Execution. At some later point we will get to Bankless Leader. 

## Detailed Design

This proposal (and feature flag) contain exactly 3 changes

1. Remove BankHash from votes.
2. Create a new static block validation check
3. Enable Voting after the static validation check is complete but before execution finishes.

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


To enable this, we need a gaurentee: When we receive the entire block, we can run a static check on that block such that, if that passes, the block will certainly be valid after replay. To get there, we need to deal with all of the ways that a block can currently be considered invalid which can only be detected by executing the block.



### Current Protocol-Violation Errors

Currently the following runtime errors result in a failed block (taken from SIMD 82 which is now closed)

1. Transaction must be deserializable via `bincode` (or equivalent) into a structure:

    ```rust
    pub struct VersionedTransaction {
        #[serde(with = "short_vec")]
        pub signatures: Vec<Signature>,
        pub message: VersionedMessage,
    }
    ```

    where `Signature` and `VersionedMessage` are defined in `solana-sdk`.
2. Transaction serialized size must be less than or equal to 1232 bytes.
3. Transaction signatures must be valid and in the same order as the static
   account keys in the `VersionedMessage`.
4. Transaction must have exactly the number of required signatures from the
   `VersionedMessage` header.
5. Transaction must not have more signatures than static account keys.
6. The `VersionedMessage` must pass sanitization checks:
    - The sum of the number of required signatures and the number of read-only
      unsigned accounts must not be greater than the number of static account
      keys.
    - The number of readonly signed accounts must be less than the number of
      required signatures.
    - Each lookup table, if present, must be used to load at least one account.
    - The total number of accounts, static or dynamic, must be less than 256.
    - Each instruction's `program_id_index` must be less than the number of
      static account keys.
    - Each instruction's `program_id_index` must not be the payer index (0).
    - All account indices in instructions must be less than the number of total
      accounts.
7. Transactions that use address lookup tables must be resolvable:
    - The address lookup table account must exist.
    - The address lookup table account must be owned by the address lookup
      table program: `AddressLookupTab1e1111111111111111111111111`
    - The address lookup table account data must be deserializable into
      `AddressLookupTable` as defined in `solana-sdk`.
    - All account table indices specified in the transaction must be less than
      the number of active addresses in the address lookup table.
8. Transactions containing pre-compile instructions must pass pre-compile
   verification checks.
9. The transaction must not load the same account more than once.
10. The transaction must have fewer than 64 accounts.
    - The limit is subject to change to 128 with the activation of
      `9LZdXeKGeBV6hRLdxS1rHbHoEUsKqesCC2ZAPTPKJAbK`.
11. The `recent_blockhash` of the transaction message must be valid:
    - It must exist and not have an age greater than 150.
    - OR the transaction must be a nonced transaction, and the nonce
      account must exist and be valid for the given `recent_blockhash`.
12. The transaction must not have already been processed.
13. The transaction fee-payer account must:
    - exist
    - be owned by the system program: `11111111111111111111111111111111`
    - have more lamports than the fee
    - have more lamports than the fee plus the minimum balance
14. The total loaded data size of the transaction must not exceed
    `requested_loaded_accounts_data_size_limit`, or the default limit (64MB).
15. Any account used as a program in a top-level instruction must:
    - be the native loader: `NativeLoader1111111111111111111111111111111`
    - OR
      - exist
      - be executable
      - be owned by the native loader: `NativeLoader1111111111111111111111111111111`
    - OR
      - exist
      - be executable
      - the owner account be owned by the native loader: `NativeLoader1111111111111111111111111111111`
      - the owner account must be executable
16. Durable nonce transactions must:
    - use a recent blockhash value different from the durable nonce for the
      current bank
    - have at least one transaction instruction, the first of which is
      designated the nonce advance instruction which must:
        - invoke the system program `11111111111111111111111111111111`
        - have instruction data that deserializes to the
          `SystemInstruction::AdvanceNonceAccount` variant
        - have at least one account input, the first of which is designated the
          nonce address/account which must:
           - be loaded with a write-lock
           - be owned by the system program: `11111111111111111111111111111111`
           - be deserializable to non-legacy initialized nonce state
           - have a durable nonce hash equal to the transaction's recent
             blockhash field
    - be signed by the nonce authority deserialized from the nonce account


We can sort these into two categories. The first category are static checks we will handle by checking them before voting:

| #  | Check Description                                                                 | Static Check? | Handling Approach                                                                                 | SIMD Status                |
|----|-----------------------------------------------------------------------------------|:-------------:|--------------------------------------------------------------------------------------------------|----------------------------|
| 1  | Transaction is deserializable                                                     |      ✔        | Enforced as part of static block validity check                                                   | Not Needed                 |
| 2  | Transaction ≤ 1232 bytes                                                          |      ✔        | Enforced as part of static block validity check                                                   | Not Needed                 |
| 3  | Signatures valid and ordered                                                      |      ✔        | Enforced as part of static block validity check                                                   | Not Needed                 |
| 4  | Required signatures present                                                       |      ✔        | Enforced as part of static block validity check                                                   | Not Needed                 |
| 5  | No extra signatures                                                               |      ✔        | Enforced as part of static block validity check                                                   | Not Needed                 |
| 6  | Message sanitization (indices, counts, structure)                                 |      ✔        | Enforced as part of static block validity check                                                   | Not Needed                 |
| 7  | Lookup table checks (existence, ownership, deserialization, indices)              |      ✗        | Requires relaxation; to be addressed in a dedicated SIMD                                          | [SIMD-0192](https://github.com/solana-foundation/solana-improvement-documents/pull/192) (Review)         |
| 8  | Pre-compile instruction verification                                              |      ✗        | Requires relaxation; to be addressed in a dedicated SIMD                                          | [SIMD-159](https://github.com/solana-foundation/solana-improvement-documents/pull/159) (Activated)       |
| 9  | No duplicate account loads                                                        |      ✔        | Enforced as part of static block validity check                                                   | Not Needed                 |
| 10 | Fewer than 64/128 accounts                                                        |      ✔        | Enforced as part of static block validity check                                                   | Not Needed                 |
| 11 | Contains Recent Block Hash or Nonced transaction                                  |      ✔        | Enforced as part of static block validity check                                                   | Not Needed                 |
| 12 | Not already processed (replay protection)                                         |      ✔        | Enforced as part of static block validity check                                                   | Not Needed                 |
| 13 | Fee-payer: existence, ownership, lamports for fee/minimum balance                 |      ✗        | Requires relaxation; to be addressed in a dedicated SIMD                                          | [SIMD-0290](https://github.com/solana-foundation/solana-improvement-documents/pull/290) (Review)         |
| 14 | Loaded data size ≤ limit                                                          |      ✗        | Requires relaxation; to be addressed in a dedicated SIMD                                          | [SIMD-0191](https://github.com/solana-foundation/solana-improvement-documents/pull/191) (Activated)      |
| 15 | Program account checks (existence, executable, ownership)                         |      ✗        | Requires relaxation; to be addressed in a dedicated SIMD                                          | [SIMD-0191](https://github.com/solana-foundation/solana-improvement-documents/pull/191) (Activated)      |
| 16 | Durable nonce checks (structure, ownership, signatures, state)                    |      ✗        | Requires relaxation; to be addressed in a dedicated SIMD                                          | [SIMD-0297](https://github.com/solana-foundation/solana-improvement-documents/pull/297) (Review)         |

## Dependencies

| SIMD Number | Description | Status |
|-------------|-------------|---------|
| [SIMD-159](https://github.com/solana-foundation/solana-improvement-documents/pull/159) | Pre-compile Instruction Verification | Activated |
| [SIMD-0191](https://github.com/solana-foundation/solana-improvement-documents/pull/191) | Loaded Data Size and Program Account Checks | Activated |
| [SIMD-0192](https://github.com/solana-foundation/solana-improvement-documents/pull/192) | Address Lookup Table Relaxation | Review |
| [SIMD-0290](https://github.com/solana-foundation/solana-improvement-documents/pull/290) | Fee-payer Account Relaxation | Review |
| [SIMD-0297](https://github.com/solana-foundation/solana-improvement-documents/pull/297) | Durable Nonce Relaxation | Review |
| [SIMD-0298](https://github.com/solana-foundation/solana-improvement-documents/pull/298) | Add BankHash to Block Header | Review |

These dependencies must be implemented and activated before this proposal can be fully realized.

## Alternatives Considered

We could always abandon Async execution and resign ourselves to single leader slow slot times forever.

## Impact

This will have minimal impact on app developers other than making confirmation times slightly faster.

## Security Considerations

Asynchronous execution allows for blocks to be added to the consensus ledger before they are executed. If an adversary can add a block that takes a long time to replay this way then it could cause skipped slots while everyone replay's the slow block.

## Drawbacks

Because Asynchronous execution adds blocks to the consensus ledger before they are done replaying, it means that we must always have static block size constraints. That is, we will not be able to remove CU limits.

## Backwards Compatibility

This proposal requires several breaking changes to enable asynchronous execution:

1. Removal of BankHash from votes - This is a breaking change to the vote structure that all validators must implement
2. New static block validation checks - All validators must implement these new validation rules
3. Changes to voting timing - Validators must be updated to vote after static validation but before execution

These changes will be gated behind a feature flag that can only be activated after all dependencies (SIMD-159, SIMD-0191, SIMD-0290, SIMD-0292, SIMD-0297, and SIMD-0298) are implemented and activated.

The feature flag will be activated in a coordinated manner across the validator network to ensure a smooth transition. Validators running older versions will be unable to participate in consensus once the feature flag is activated.
