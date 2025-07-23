---
simd: '0306'
title: Increase writeable account limit to 20M CUs
authors:
  - Brennan Watt (Anza)
category: Standard
type: Core
status: Review
created: 2025-06-17
feature: TBD
---

## Summary

Increase the per account CU limit from a static 12M to 40% of the block CU limit.

## Motivation

This will allow more activity to target a single hot account in each block and
take advantage of the performance improvements that have been made. We are
consistently hitting the current 12M CU cap, so this will unlock more economic
activity.

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

The following table shows the current block limits and future proposed block
limits:

| Type | Current Block Limit | SIMD-0256 Limit | SIMD-0286 Limit |
|------|-----|---------------|---------------------|
| Max Block Units | 50M | 60M | 100M |
| Max Writable Account Units | 12M | 12M  | 12M |
| Max Vote Units | 36M | 36M  | 36M  |
| Max Block Accounts Data Size Delta | 100MB | 100MB | 100MB |

This proposal advocates to set `Max Writable Account Units` to 40% of `Max Block
Units`. This will result in the following `Max Writable Account Units` for
current and future proposed `Max Block Units`:

| Block Limit | Max Block Units | Max Writable Account Units |
| Current | 50M | 20M |
| SIMD-0256 | 60M | 24M |
| SIMD-0286 | 100M | 40M |

This proposal only changes the `Max Writable Account Units` limit. The purpose
is to increase amount of activity that can target a single account. The `Max
Block Unit` is left unchanged. Further increases to global limit will be
addressed with SIMD-0286. Increasing `Max Writeable Account Units` while leaving
`Max Block Units` unchanged takes advantage of underutilized serial execution
capacity.

The intention is for this to follow SIMD-0256 but activate before SIMD-0286
(which has not merged yet). Rationale is that we are ready to handle this
increased serialized execution today but the 100M CU increase has some
development dependencies. That said, we will be capable of handling a scenario
where SIMD-0286 merges and gets activated first.

## Alternatives Considered

Killing per account limits altogether. But this may be too easy to attack by
aggressively targeting a single account up to the global CU limit.

## Impact

This will allow for more serialized account access, making it easier for more
updates to some hot state to occur. It may increase block execution times, which
could impact slot times for some unforeseen cases.

## Security Considerations

Blocks may take longer to execute, slowing down network progress and catchup times.

## Drawbacks *(Optional)*

Increasing the CU limit for single account will increase worst case serialized
execution, which could increase block verification and/or slot time.

## Backwards Compatibility *(Optional)*

- All previously valid blocks are still valid, since limits are only
  increasing.
- Blocks produced after the change may be rejected by previous versions that do
  not support the new limits.