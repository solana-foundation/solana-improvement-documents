---
simd: '0191'
title: Relax Transaction Loading Constraints
authors:
  - Andrew Fitzgerald (Anza)
category: Standard
type: Core
status: Activated
created: 2024-11-06
feature: PaymEPK2oqwT9TXAVfadjztH2H6KfLEB9Hhd5Q5frvP (https://github.com/anza-xyz/agave/issues/3244)
supersedes:
superseded-by:
extends:
---

## Summary

This proposal aims to relax certain transaction errors related to loading
transaction accounts, from protocol violations to runtime errors.
Specifically, if a transaction fails to load a valid program account or
exceeds the requested maximum loaded account data size, the transaction
may be included in a block, and the transaction fee will be charged.

## Motivation

The current transaction constraints are overly restrictive and adds complexity
in determining whether a block is valid or not.
This proposal aims to relax these loading constraints to simplify the protocol,
and give block-producers more flexibility in determining which transactions
may be included in a block.
The goal is to remove this reliance on account-state in order to validate a
block.

## New Terminology

These terms are used elsewhere, but are defined here for clarity:

- Protocol Violating Transaction Error: A transaction error that violates the
  protocol. This class of errors must result in the entire block being rejected
  by the network.
- Runtime Transaction Error: A transaction error that results in a failed
  transaction, and may be included in the block. These transactions still
  incur transaction fees, and nonce advancements.

## Detailed Design

Among others, a transaction that fails to load due to violating one of the
following constraints is considered a protocol violation error:

1. The total loaded data size of the transaction must not exceed
   `requested_loaded_accounts_data_size_limit`, or the default limit (64MiB).
2. Any account used as a program in a top-level instruction must:
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

This proposal moves these errors from protocol violations to runtime errors.
A transaction that fails to load due to violating either one of these
constraints may be included in a block, so long as it is otherwise valid.
The transaction must pay transaction fees, and if present, the nonce must be
advanced.

Constraints SHOULD be checked for each transaction before execution to avoid
unnecessary computation. If a constraint violating transaction is executed, the
constraints MUST be checked BEFORE committing transaction changes.

The `TransactionError` variants do not need to change from their current
values. This proposal only changes how the validator handles these errors.

`agave` currently performs the relevant checks in the following order.
This order is not necessary for consensus, and is only provided for clarity.

For each `Pubkey` included in the transaction message, or loaded from an
address lookup table, the following checks MUST be performed, and SHOULD be
performed in this order for error-consistency:

- Check if the account exists. If not, assume default account state (empty).
- Accumulate account's `data` field `len`. If the total exceeds the
  `requested_loaded_accounts_data_size_limit` (or default if unspecified),
  return `MaxLoadedAccountDataSizeExceeded`.

For each transction-level instruction in the transaction the following
checks MUST be performed, and SHOULD be performed in this order for
error-consistency:

- If the program account is `native_loader`, continue to next
  instruction.
- If the program account does not exist, return `ProgramAccountNotFound`
- If the program account is not executable, return ``InvalidProgramForExecution`
  - This only applies until
    [SIMD-0162](https://github.com/solana-foundation/solana-improvement-documents/pull/162)
    is activated
- If the program account's owner is the native_loader, continue to next
  instruction.
- If the program account's owner does not exist, return `ProgramAccountNotFound`
- If the program account's owner is not the native_loader, return ``InvalidProgramForExecution`
- If the program account's owner is not executable, return ``InvalidProgramForExecution`
  - This only applies until
    [SIMD-0162](https://github.com/solana-foundation/solana-improvement-documents/pull/162)
    is activated
- Accumulate the owner account's `data` field `len` and check if the total
  exceeds the `requested_loaded_accounts_data_size_limit` (or default if
  unspecified), return `MaxLoadedAccountDataSizeExceeded`.
  - The owner's data size MUST only be accumulated on the first instruction
    that uses the program account.

## Alternatives Considered

- Do nothing
  - This is the simplest option, as we could leave the protocol as is.
  However, this leaves the protocol more complex than it needs to be.
- Relax additional constraints:
  - SIMD-0082 sought to relax additional constraints, but has not been
    accepted. This proposal is a subset of SIMD-0082, intended to make the
    review process simpler and faster. Therefore, we have decided to keep
    this proposal focused specifically on certain loading failures.

## Impact

- Transactions that would previously have been dropped with a protocol
  violation error can now be included and will be charged fees.
  - Users must be more careful when constructing transactions to ensure they
    are executable if they do not want to waste fees.
- Block-production is simplified as it can be done without needing to load
  large program accounts for the initial decision to include a transaction.

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
blocks would still be valid. However, new blocks may not be valid under the old
protocol.
