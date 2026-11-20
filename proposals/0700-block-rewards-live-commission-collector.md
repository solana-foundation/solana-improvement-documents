---
simd: '0700'
title: Block Reward Live Commission Collector
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

When looking up the block collector address in a vote account, use the live
account instead of a snapshotted version.

## Motivation

The protocol depends on data that is not present in the accounts snapshot. This
state cannot be easily inspected by users and complicates client
implementations.

The custom block commission collector introduced in [SIMD-0232] is an example of
this kind of state, since it is read from a snapshot of vote accounts taken at
each epoch boundary.

The use of out-of-protocol snapshotted data is not required, and was likely
designed to correspond with the snapshot data used for commission values in
[SIMD-0249].

[SIMD-0249]: https://github.com/solana-foundation/solana-improvement-documents/pull/249

## Dependencies

This proposal depends on the following previously accepted proposals:

- **[SIMD-0232]: Custom Commission Collector Account**

    Custom commission collectors must be enabled for any choice to be made

[SIMD-0232]: https://github.com/solana-foundation/solana-improvement-documents/pull/232

## New Terminology

N/A

## Detailed Design

During reward collection at the end of each block, read the current vote account
data for the leader.

The following checks MUST always be performed when reading live vote account
data. If any check fails, fees or rewards are burned.

- The account MUST be owned by the vote program
- The account MUST be deserializable as a valid vote state. If the account is
  v4 or later, read the block reward collector from the account. Otherwise, use
  the identity address for block rewards.

### Edge Cases

A vote account can be closed and reopened with a new collector value. This is
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
a cached collector id, followed by a live collector id, by updating throughout
the epoch, and seeing new collectors receive rewards.
