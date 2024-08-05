---
simd: '0082'
title: Relax Transaction Constraints
authors:
  - Andrew Fitzgerald (Solana Labs)
category: Standard
type: Core
status: Draft
created: 2023-10-30
feature:
---

## Summary

Transaction errors fall into two categories: protocol violating errors and
runtime errors.
Protocol violation errors are those that break some constraint of the protocol,
and any blocks containing such transactions must be rejected by the network.
Runtime errors are those that occur during the processing of a transaction, and
generally these errors mean that state changes are limited to a few accounts,
but the transaction itself may be included in a block.
Any block containing a transaction that results in a runtime error is still
valid.
This proposal aims to change several protocol violation errors into runtime
errors, in order to simplify the protocol, and give more flexibility to
block-producer and block-validator implementations.

## Motivation

The current protocol places many constraints on the structure and contents
of blocks; if any constraints are broken, block-validators will mark the
entire block as invalid.
Many of these constraints are necessary, but some of them are not, and lead to
additional complexity in the protocol.
This proposal aims to relax some of the constraints at the individual
transaction level, in order to simplify the protocol, and give more flexibility
to block-producer and block-validator implementations.

More specifically, this proposal changes several protocol violation errors into
runtime errors.
This means that transactions that would previously be dropped with an error,
can now be included in a block, but will either not be executed at all, or
will have limited state changes.
The goal is to remove much of the reliance on account-state in order to
validate a block.
This proposal on its' own, will not enable asynchronous execution, but it will
remove one of the barriers to asynchronous execution.

## Alternatives Considered

1. Do nothing
    - This is the simplest option, as we could leave the protocol as is.
    However, this leaves the protocol more complex than it needs to be.
2. Also relax fee-paying constraint
    - This was considered, and included in the intially reviewed proposal.
    However, this was decided to be moved to a separate follow-up proposal,
    should this one be accepted.
    There was significant disagreements on how exactly transactions that cannot
    pay fees should be handled.
    By keeping fee-paying as a constraint for the current proposal, it will
    allow this proposal to be accepted more quickly, which will give
    significant benefits to the network.
3. Additionally, relax the address lookup table resolution constraint
    - This was considered, since it is a transaction-level constraint that is
    dependent on account-state. However, due to entry-level and block-level
    constraints that rely on the address lookup table resolution, this
    constraint cannot easily be relaxed without also relaxing those
    constraints.

## New Terminology

None

## Detailed Design

### Current Protocol-Violation Errors

Prior to this change, the list of protocol violation errors at the individual
transaction level are:

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

### Proposed Protocol-Violation Errors

The proposal is to move some items of the above list from protocol violations
to runtime errors.
Specifically, the following constraints will remain as protocol violations:

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
8. The `recent_blockhash` of the transaction message must be valid:
    - It must exist and not have an age greater than 150.
    - OR the transaction must be a nonced transaction, and the nonce
      account must exist and be valid for the given `recent_blockhash`.
9. The transaction must not have already been processed.
10. The transaction fee-payer account must:
    - exist
    - be owned by the system program: `11111111111111111111111111111111`
    - have more lamports than the fee
    - have more lamports than the fee plus the minimum balance
11. Durable nonce transactions must:
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

### Proposed New Runtime Errors

The following constraints will be moved to become runtime errors.
Transactions that break these constraints may be included in a block.
The fee-payer account must be charged fees, nonce advanced, but the transaction
will not be executed.

1. Transactions containing pre-compile instructions must pass pre-compile
   verification checks.
2. The transaction must not load the same account more than once.
3. The transaction must have fewer than 64 accounts.
    - The limit is subject to change to 128 with the activation of
      `9LZdXeKGeBV6hRLdxS1rHbHoEUsKqesCC2ZAPTPKJAbK`.
4. The total loaded data size of the transaction must not exceed
   `requested_loaded_accounts_data_size_limit`, or the default limit (64MB).
5. Any account used as a program in a top-level instruction must:
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

### Block-Limits

This proposal allows for transactions which do not execute to be included in a
block.
This was not previously possible, so how transaction costs are applied to
limits must be clarified.
This proposal proposes that an included unexecuted transaction should have all
non-execution transaction costs applied towards block-limits, as well as all
the writable account limits.

### Rationale

The intent with relaxing these constraints is to reduce the amount of
account-state which is required in order to validate a block.
This gives more flexibility to when and how account-state is updated during
both block production and validation.
Particularly with the relaxation of the program account constraints,
block-packing can be done without needing to load large program accounts
for the initial decision to include or not.
This is a major step towards asynchronous block production and validation.

Any transaction included in the block, regardless of whether or not the
transaction is executed, must be charged fees, inserted into the status cache,
and the nonce account, if present, must be advanced.

## Impact

- Transactions that would previously be dropped with an error, can now be
  included and will be charged fees.
  - Users must be more careful when constructing transactions to ensure they
    are executable if they don't want to waste fees
- Block-production is simplified as it can be done without needing to load
  large program accounts for the initial decision to include or not.

## Security Considerations

None

## Drawbacks

- Users must be more careful about what they sign, as they will be charged fees
  for transactions that are included in a block, even if they are not executed.
- This will likely break a lot of tooling, such as explorers, which may expect
  all transactions to attempt execution.

## Backwards Compatibility

This proposal is backwards compatible with the current protocol, since it only
relaxes constraints, and does not add any new constraints. All previously valid
blocks would still be valid.
