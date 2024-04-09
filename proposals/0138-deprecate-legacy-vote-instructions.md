---
simd: '0138'
title: Deprecate legacy vote instructions
authors:
  - Ashwin Sekar
category: Standard
type: Core
status: Review
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

When the feature - `deprecate legacy vote instructions`, with key
`depVvnQ2UysGrhwdiwU42tCadZL8GcBb1i2GYhMopQv` is activated, processing of the
following instructions will result in an `InvalidInstructionData` error:

* `Vote`
* `VoteSwitch`
* `UpdateVoteState`
* `UpdateVoteStateSwitch`
* `CompactUpdateVoteState`
* `CompactUpdateVoteStateSwitch`

## Impact

Sending transactions that include the mentioned instructions will fail.

## Security Considerations

Implementations should ensure that the `TowerSync` instruction has been
activated before enabling the `deprecate legacy vote instructions` feature,
in order to ensure that at least one vote instruction available for the client.

## Backwards Compatibility

Incompatible
