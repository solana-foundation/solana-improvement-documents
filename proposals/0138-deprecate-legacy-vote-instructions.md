---
simd: '0138'
title: Deprecate legacy vote instructions
authors:
  - Ashwin Sekar
category: Standard
type: Core
status: Accepted
created: 2024-04-09
feature: (fill in with feature tracking issues once accepted)
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

## Impact

Sending transactions that include the mentioned instructions will fail.

## Security Considerations

None

## Backwards Compatibility

Incompatible
