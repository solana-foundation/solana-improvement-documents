---
simd: '0138'
title: Deprecate legacy vote instructions
authors:
  - Ashwin Sekar
category: Standard
type: Core
status: Implemented
created: 2024-04-09
feature: depVvnQ2UysGrhwdiwU42tCadZL8GcBb1i2GYhMopQv
---

## Summary

Disables the legacy `Vote`, `UpdateVoteState` and `CompactUpdateVoteState`
instruction variants.

## Motivation

These instructions are no longer sent by validator clients.
It is cumbersome to support these old vote instructions and ensure parity.

## Alternatives Considered

None

## New Terminology

None

## Detailed Design

When the features:

* `deprecate legacy vote instructions`, with key
    `depVvnQ2UysGrhwdiwU42tCadZL8GcBb1i2GYhMopQv`
* `enable tower sync vote instruction`, with key
    `tSynMCspg4xFiCj1v3TDb4c7crMR5tSBhLz4sF7rrNA`

are activated, the following instructions will result in an
`InvalidInstructionData` error:

* `Vote`
* `VoteSwitch`
* `UpdateVoteState`
* `UpdateVoteStateSwitch`
* `CompactUpdateVoteState`
* `CompactUpdateVoteStateSwitch`

Enabling `deprecate legacy vote instructions` without 
`enable tower sync vote instruction` should have no effect.

The `deprecate legacy vote instructions` feature is not planned to be
activated. Instead, the vote program rejects the same instructions with
`InvalidInstructionData` once the cluster has migrated to Alpenglow
([SIMD-0384]), so they are retired as part of that migration.

## Impact

Sending transactions that include the mentioned instructions will fail.

## Security Considerations

None

## Backwards Compatibility

Incompatible

[SIMD-0384]: https://github.com/solana-foundation/solana-improvement-documents/pull/384
