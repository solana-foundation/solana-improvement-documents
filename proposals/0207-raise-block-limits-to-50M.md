---
simd: '0207'
title: Raise Block Limits to 50M CUs
authors:
  - Andrew Fitzgerad (Anza)
category: Standard
type: Core
status: Accepted
created: 2024-12-05
feature: 5oMCU3JPaFLr8Zr4ct7yFA7jdk6Mw1RmB8K4u9ZbS42z (https://github.com/anza-xyz/agave/issues/4042)
development:
  - Anza - Implemented in https://github.com/anza-xyz/agave/pull/4026
---

## Summary

Raise the block limit from 48M to 50M CUs.

## Motivation

Current block limits are set to 48M CUs.
Block limits's primary purpose is to make sure that the vast majority of
network participants are able to keep up with the network, by restricting the
amount of work a leader is allowed to pack into a block.
However, current main net beta traffic is largely not constrained by large
block execution times.
This proposal aims a modest increase in block limits to 50M CUs, in order to
give some additional capacity to the network, and client implementations ready
for future increases as the performance of the network improves.

## New Terminology

No new terms, but the following definitions are given for clarity:

- Max Block Units - The maximum number of CUs that can be included in a block.
- Max Writable Account Units - The maximum number of CUs that can be used to
  write any given account in a block.
- Max Vote Units - The maximum number of CUs that can be used by simple votes
  in a block.
- Max Block Accounts Data Size Delta - The maximum size of the accounts data
  size delta in a block.

## Detailed Design

The following table shows the current block limits and the proposed block
limits:

| Type | Current Block Limit | Proposed Block Limit |
|------|----------------------|----------------------|
| Max Block Units | 48M | 50M |
| Max Writable Account Units | 12M | 12M |
| Max Vote Units | 36M | 36M |
| Max Block Accounts Data Size Delta | 100MB | 100MB |

This proposal only changes the `Max Block Units` limit.
The purpose is to increase capacity for non-vote transactions.
The `Max Writable Account Units` is left unchanged since there is no specific
push to increase the capacity for individual accounts at this time.
Keeping `Max Writeable Account Units` unchanged while raising the
`Max Block Units` , allows for additional parallel capacity.

## Alternatives Considered

- Leave the block limits as they are
  - This leaves capacity on the table, we want to get ready for future
    increases.
- Double limit to 96M CUs
  - Viewed as too aggressive at this time, and may cause unforeseen issues
    particularly in infrastructure supporting the network users.
  - We instead plan to increase the limits incrementally as the network
    performance improves.

## Impact

- More transactions can be included per block.

## Security Considerations

- Blocks may take longer to execute, slowing down network progress.

## Drawbacks

- Larger blocks may cause unforseen issues in infrastructure beyond the
  validators.

## Backwards Compatibility

- All previously valid blocks are still valid, since limits are only
  increasing.
- Blocks produced after the change may be rejected by previous versions that do
  not support the new limits.
