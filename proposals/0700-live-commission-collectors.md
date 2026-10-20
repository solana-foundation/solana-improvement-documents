---
simd: '0700'
title: Live Commission Collectors
authors:
  - joncinque
category: Standard
type: Core
created: 2026-10-20
status: Review
feature: (fill in with feature key and github tracking issues once accepted)
supersedes: '0232'
---

## Summary

When looking up the inflation and block collector addresses in a vote account,
use the live account instead of a snapshotted version.

## Motivation

The protocol depends on data that is not present in the accounts snapshot. This
state cannot be easily inspected by users and complicates client
implementations.

The custom commission collectors introduced in [SIMD-0232] are an example of
this kind of state, since they are read from a snapshot of vote accounts taken
at each epoch boundary.

The use of out-of-protocol snapshotted data is not required, and was likely
designed to correspond with the snapshot data used for commission values in
[SIMD-0249].

[SIMD-0249]: https://github.com/solana-foundation/solana-improvement-documents/pull/249

## Dependencies

This proposal depends on the following previously accepted proposals:

- **[SIMD-0232]: Custom Commission Collector Account**

    Custom commission collectors must be enabled for any choice to be made

- **[SIMD-0123]: Block Revenue Distribution**

    The custom block revenue commission collector is only available with block
    revenue distribution available

[SIMD-0232]: https://github.com/solana-foundation/solana-improvement-documents/pull/232
[SIMD-0123]: https://github.com/solana-foundation/solana-improvement-documents/pull/123

## New Terminology

N/A

## Detailed Design

During reward collection at the end of each block, and when minting inflation
rewards during the first block of a new epoch, read the current vote account
data for the leader.

The following checks MUST always be performed when reading live vote account
data. If any check fails, fees or rewards are burned.

- The account MUST be owned by the vote program
- The account MUST be deserializable as a vote state v3 or v4. If the account is
  v3, use the identity address for block rewards, and the vote address for
  inflation rewards. If the account is v4, read the appropriate collector from
  the account.

### Edge Cases

A vote account can be closed and reopened with new collector values. This is
acceptable since it functions just like a collector reassignment.

### Validator Components Affected

Which validator components are affected by this change?

| Validator Component             | Impact                              |
|---------------------------------|-------------------------------------|
| Transaction Execution (Runtime) | Change which vote account to read   |

## Alternatives Considered

N/A

## Impact

This proposal removes one piece of hidden cached data and simplifies the reward
distribution process.

## Security Considerations

N/A

## Conformance

The change will be accompanied by a ledger from a localnet cluster demonstrating
cached collector ids, followed by live collector ids, by updating throughout the
epoch, and seeing new collectors receive rewards.
