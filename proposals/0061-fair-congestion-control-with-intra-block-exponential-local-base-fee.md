---
simd: '0061'
title: Fair congestion control with intra-block exponential local base fee
authors:
  - Ryo Onodera (Solana Labs)
category: Standard/Meta
type: Core
status: Draft
created: 2023-07-05
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This feature is a congestion-control mechanism in the form of an extension to
local fee markets while leaving their locality of transaction fee dynamics
intact.

To that end, it introduces a dynamic base fees to individual local fee markets.
It also attains very short feedback loop of intra block frequency to maintain
full efficacy of Solana's peculiar execution model compared to other
blockchains: multi-threaded and low latency.

This is realized with some incentive tweak to combat against the obvious base fee
manipulation with such short interval.

## Motivation

- Write lock cu limit is bad (bot can lock out at the very first of block for the entire duration of whole blocktime (400ms)
- Increased Defi activity around any volatile financial markets could starve payment transactions for extended time


## Alternatives Considered

Related proposals:

(TODO: add any relation of this to them)

https://github.com/solana-foundation/solana-improvement-documents/pull/4

https://github.com/solana-foundation/solana-improvement-documents/pull/16

https://github.com/solana-foundation/solana-improvement-documents/pull/45

## New Terminology

Is there any new terminology introduced with this proposal?

## Detailed Design

Explain the feature as if it was already implemented and you're explaining it
to another Solana core contributor. The generally means:

- Explain the proposed change and how it works
- Where the feature fits in to the runtime, core, or relevant sub-system
- How this feature was/could be implemented
- Interaction with other features
- Edge cases

## Impact

How will the implemented proposal impacts dapp developers, validators, and core contributors?

## Security Considerations

What security implications/considerations come with implementing this feature?
Are there any implementation-specific guidance or pitfalls?

## Drawbacks *(Optional)*

Why should we not do this?

## Backwards Compatibility *(Optional)*

Does the feature introduce any breaking changes? All incompatibilities and
consequences should be listed.
