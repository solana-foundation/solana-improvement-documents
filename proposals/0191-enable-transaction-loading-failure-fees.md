---
simd: '0191'
title: Relax Transaction Loading Constraints
authors:
  - Andrew Fitzgerald (Anza)
category: Standard
type: Core
status: Review
created: 2024-11-06
feature: PaymEPK2oqwT9TXAVfadjztH2H6KfLEB9Hhd5Q5frvP
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
