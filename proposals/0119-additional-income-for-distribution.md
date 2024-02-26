---
simd: '0119'
title: Additional Amounts for Distribution
authors:
  - diman
category: Standard
type: Core
status: Draft
created: 2024-02-22
feature: none
---

## Summary

Expand the mechanism for distributing inflation rewards: add the distribution 
of additional amounts.

## Motivation

To enhance their competitive edge over stakers, validators employ various 
methods to allocate some of the additional amounts among their stakers. It is 
suggested to implement such a mechanism at the protocol level. This will, among 
other benefits, offer a simple and uniform way for independent 
analysts/websites to access this information.

## Alternatives Considered

Use of pools with a single validator: minting pool tokens, subsequent burning 
of tokens.

## New Terminology

None

## Detailed Design

Each validator has its own treasury, which is replenished during the epoch by a 
simple transfer. At the beginning of the next epoch, the amount distributed 
among the stakers of that validator is increased by the amount accumulated in 
the treasury.

Optionally (as this is not the final design, but a proposal for discussion), 
there could be several treasuries, at least two: one for distribution only 
among stakers and the other among both stakers and the validator (according to 
the validator’s commission rate on inflation).

The implementation as a separate program, or an extension of the vote program, 
is left for discussion of this proposal.

## Impact

Validators will be happy: they won’t have to deal with complex manipulations 
with pools or other distribution programs. The addition of the proposed feature 
itself does not require any additional actions on the part of the validator if 
they do not distribute additional amounts.

## Security Considerations

None

## Backwards Compatibility

This is a new functionality at the protocol level. Addition through a feature 
gate. There will be no backwards compatibility.
