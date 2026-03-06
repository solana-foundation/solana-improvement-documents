---
simd: '0452'
title: Charge CUs for ABIv0/v1 Instructions
authors:
  - Alexander Meißner (Anza)
category: Standard
type: Core
status: Idea
created: 2026-01-23
feature: TBD
---

## Summary

Charge CUs for account data and instruction data better reflecting the cost
incurred on a validator in ABI v0 and v1.

## Motivation

- ABIv0/v1 only charge CUs for account data length in the call edge of CPI,
not at all in the top-level
- ABIv0/v1 do not charge CUs for instruction data length, neither in CPI nor
the top-level

## Dependencies

This proposal soft depends either one of the following proposals:

- **[SIMD-0177]: Program Runtime ABI v2**

    ABIv2 will offer an alternative path which does not charge these CUs.

- **[SIMD-0370]: Remove Compute Unit Block Limit**

    If the alternative (ABIv2) is not offered in time the increased CUs per IX
    would reach the block packing limit quicker. It thus would have to be
    increased or removed.

[SIMD-0177]: https://github.com/solana-foundation/solana-improvement-documents/pull/177
[SIMD-0370]: https://github.com/solana-foundation/solana-improvement-documents/pull/370

## New Terminology

None

## Detailed Design

Independent of any SIMD a validator should already charge CUs according to
`cpi_bytes_per_unit` for:

- instruction data length at the ABIv0 and ABIv1 CPI call edge
- original account data length of the callee at the ABIv0 and ABIv1 CPI call
edge

Starting with activation of the feature gate associated with SIMD-0339 a
validator should already charge CUs for:

- number of account metas at the ABIv0 and ABIv1 CPI call edge
- number of account infos at the ABIv0 and ABIv1 CPI call edge

Starting with activation of the feature gate associated with this SIMD a
validator must charge CUs according to `cpi_bytes_per_unit` for:

- instruction data length in ABIv0 and ABIv1 serialization
- account data length before the instruction in ABIv0 serialization
- account data length before the instruction + 10 KiB in ABIv1
serialization
- account data length after the instruction in ABIv0 and ABIv1 deserialization
- original account data length of the caller at the ABIv0 and ABIv1 CPI return
edge

## Alternatives Considered

This is inevitable especially when the network uses larger account sizes.

## Impact

An instruction will be charged for **each byte of instruction data**:

- at top-level: 1 time instead of 0 times (xUndefined or +Undefined%)
- at any CPI level: 2 times instead of 1 time (x2 or +100%)

In the normal case of accounts not being resized an instruction will be charged
for **each byte of every instruction account**:

- at top-level: 2 times instead of 0 times (xUndefined or +Undefined%)
- at top-level + one CPI level: 6 times instead of 1 time (x6 or +500%)
- at top-level + two CPI levels: 10 times instead of 2 times (x5 or +400%)
- at top-level + three CPI levels: 14 times instead of 3 times (x4.66 or +377%)
- at top-level + four CPI levels: 18 times instead of 4 times (x4.5 or +350%)

## Security Considerations

None

## Drawbacks

It will break existing TX building as the CUs required will be significantly
increased. To counter this we either have to offer an alternative (ABIv2) or
adjust the CU price / fees and block packing limits.
