---
simd: '0332'
title: Reduce ChaCha rounds for Turbine from 20 to 8
authors:
  - Brennan Watt
category: Standard
type: Core
status: Review
created: 2025-08-06
feature: TBD
---

## Summary

Reduce the number of ChaCha rounds for turbine weighted shuffle calculation
from 20 to 8 in order to reduce the amount of compute time spent on this.

## Motivation

Cheap, easy performance win for Turbine. This doesn't need to be ultra secure.
Just needs to be random enough to prevent malicious nodes from censoring blocks.

## New Terminology

None

## Detailed Design

- Add a feature for controlling switching to 8 ChaCha rounds.
- Pull feature state to determine if we should perform 20 rounds (legacy) or 8
  rounds (after feature activation) of ChaCha
  - Broadcast stage
  - Retransmit stage
  - Retransmit signature check
- Use the specified ChaCha rounds for computing turbine tree weighted shuffle of
  staked peers before determining parent/children.

## Alternatives Considered

What alternative designs were considered and what pros/cons does this feature
have relative to them?

## Impact

Should be unnoticeable by most. Small performance improvement in ability to send
block data out.

## Security Considerations

If someone could predictably grind and influence turbine tree to ensure some
malicious minority of stake could censor unrecoverable portions of a block, this
would be a problem. Today, the grinding can be done, which means further ChaCha
rounds provide very limited beenfit, but the ability to influence the generated
tree is clamped down because the input data includes:

- shred index
- shred slot
- shred type (coding or data)
- leader node
