---
simd: '0256'
title: Increase Block Limits to 60M CUs
authors:
  - Brennan Watt (Anza)
category: Standard
type: Core
status: Review
created: 2025-03-06
feature: 6oMCUgfY6BzZ6jwB681J6ju5Bh6CjVXbd7NeWYqiXBSu
development:
  - Anza - TBD
---

## Summary

Increase the block limit to 60M CUs.

## Motivation

Current block limits are set to 48M CUs. SIMD-0207 will increase them to 50M.
Block limits' primary purpose is to ensure the vast majority of network
participants are able to keep up with the network, by restricting the amount of
work a leader is allowed to pack into a block. However, current mainnet traffic
is largely not constrained by large block execution times. This proposal aims a
substantial increase in block limits to 60M CUs, in order to provide additional
capacity to the network.

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

| Type | Current Block Limit | SIMD-0207 Limit | Proposed Block Limit |
|------|---------------------|-----------------|----------------------|
| Max Block Units | 48M | 50M | 60M |
| Max Writable Account Units | 12M | 12M | 12M |
| Max Vote Units | 36M | 36M | 36M |
| Max Block Accounts Data Size Delta | 100MB | 100MB | 100MB |

This proposal only changes the `Max Block Units` limit.
The purpose is to increase capacity for non-vote transactions.
The `Max Writable Account Units` is left unchanged since there is no specific
push to increase the capacity for individual accounts at this time.
Keeping `Max Writeable Account Units` unchanged while raising the
`Max Block Units` , allows for additional parallel capacity.

The intention is for this to follow SIMD-0207 (as opposed to replacing).
SIMD-0207 is a much more modest increase in block space, but will be valuable in
discovering any unforeseen problems with changing the longstanding 48M CU cap
and verifying assumptions. This proposal aims to increase block space more
aggressively.

## Alternatives Considered

- Leave the block limits as they are
  - This leaves capacity on the table.
- Larger increase of limit to 96M CUs
  - Viewed as too aggressive at this time, and may cause unforeseen issues
    particularly in turbine and infrastructure supporting the network users.
  - We instead plan to increase the limits incrementally as the network
    performance improves.

## Impact

- More transactions can be included per block.

## Security Considerations

- Blocks may take longer to execute, slowing down network progress and catchup times.

## Drawbacks

- Larger blocks may cause unforseen issues in infrastructure beyond the
  validators.

## Backwards Compatibility

- All previously valid blocks are still valid, since limits are only
  increasing.
- Blocks produced after the change may be rejected by previous versions that do
  not support the new limits.
