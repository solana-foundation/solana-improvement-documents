---
simd: '0286'
title: Increase Block Limits to 100M CUs
authors:
  - Lucas Bruder (Jito Labs)
category: Standard
type: Core
status: Review
created: 2025-05-20
feature: TBD
development:
  - Anza - TBD
  - Firedancer - TBD
  - Jito Labs - TBD
---

## Summary

Increase the block limit to 100M CUs.

## Motivation

Current block limits are set to 50M CUs. SIMD-0256 will increase them to 60M.
Block limits' primary purpose is to ensure the vast majority of network
participants are able to keep up with the network, by restricting the amount of
work a leader is allowed to pack into a block. However, current mainnet traffic
is largely not constrained by large block execution times. This proposal aims a
substantial increase in block limits to 100M CUs, in order to provide additional
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

| Type | Current Block Limit | SIMD-0256 Limit | Proposed Block Limit |
|------|-----|-----------------|---------------------|
| Max Block Units | 50M | 60M             | 100M |
| Max Writable Account Units | 12M | 12M             | 12M |
| Max Vote Units | 36M | 36M             | 36M  |
| Max Block Accounts Data Size Delta | 100MB | 100MB           | 100MB |

This proposal only changes the `Max Block Units` limit.
The purpose is to increase capacity for non-vote transactions.
The `Max Writable Account Units` is left unchanged since there is no specific
push to increase the capacity for individual accounts at this time.
Keeping `Max Writeable Account Units` unchanged while raising the
`Max Block Units` , allows for additional parallel capacity.

The intention is for this to follow SIMD-0256 (as opposed to replacing).
SIMD-0256 is a much more modest increase in block space. This proposal 
aims to increase block space more aggressively.

## Alternatives Considered

- Smaller increase of limit to 80M CUs
    - Not IBRL'ing fast enough

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
