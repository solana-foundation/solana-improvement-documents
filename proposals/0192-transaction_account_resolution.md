---
simd: '0192'
title: Relax Transaction Account Resolution
authors:
  - Andrew Fitzgerald (Anza)
category: Standard
type: Core
status: Review
created: 2024-11-06
feature:
supersedes:
superseded-by:
extends:
---

## Summary

This proposal aims to relax certain transaction errors related to the account
keys used in  a transaction, from protocol violations to runtime errors.

## Motivation

The current transaction constraints are overly restrictive and add complexity
to the protocol. Specifically the constraints that require account-state
in order to validate a block due to lookup tables. This account-state
dependence makes both block-production and block-validation more complex than
necessary.

## New Terminology

These terms are used elsewhere, but are defined here for clarity:

- Protocol Violating Transaction Error: A transaction error that violates the
  protocol. This class of errors must result in the entire block being rejected
  by the network.
- Runtime Transaction Error: A transaction error that results in a failed
  transaction, and may be included in the block. These transactions still
  incur transaction fees, and nonce advancements.

## Detailed Design

The current protocol requires that the the account keys used in a transaction:

1. Contain no duplicate account keys,
2. All address lookup tables must be resolvable:
    - The address lookup table account must exist.
    - The address lookup table account must be owned by the address lookup
      table program: `AddressLookupTab1e1111111111111111111111111`
    - The address lookup table account data must be deserializable into
      `AddressLookupTable` as defined in `solana-sdk`.
    - All account table indices specified in the transaction must be less than
      the number of active addresses in the address lookup table.

This proposal seeks to relax these constraints from protocol violation errors
to runtime errors.
This means that transactions that break any of these constraints may be
included in a block, if they are otherwise valid.
Such transactions do not need to attempt any further loading or execution; they
need only to pay fees and advance the nonce.
Such transactions must have transaction costs for block-limits applied only
to the fee-payer and nonce accounts, since execution will not even be
attempted.
Transaction-cost calculation is unchanged, and calculated as if the transaction
were a "fee-only" transaction.

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
  - Users must be more careful when constructing transactions to ensure the
    account keys requested are valid.

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
