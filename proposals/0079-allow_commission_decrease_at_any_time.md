---
simd: '0079'
title: Allow Commission Decrease at Any Time
authors:
  - Bryan Ischo (bryan@ischo.com)
category: Standard
type: Core
status: Implemented
created: 2023-10-26
feature: [decoMktMcnmiq6t3u7g5BfgcQu91nKZr6RvMYf9z1Jb](https://github.com/solana-labs/solana/issues/33994)
development:
  - Anza - [Implemented](https://github.com/solana-labs/solana/pull/33847)
  - Firedancer - Implemented
---

## Summary

The commission_updates_only_allowed_in_first_half_of_epoch feature disallows
commission decrease in the second half of epoch. Given that the purpose of this
feature was to prevent 'rug pulls' which are accomplished by increasing
commission at the end of epoch and then decreasing commission at the beginning
of next epoch, disallowing decreases during the second half of epoch is
unnecessary.

A feature gate must be added to support this SIMD as all validators' vote
programs must treat commission change instructions the same or else consensus
will diverge.

## Motivation

Some validator operators may need to decrease commission in order to satisfy
their own operational criteria.

As an example, a validator operator may have a policy whereby any error that
results in reduced stake account rewards for the epoch, will result in the
operator choosing to reduce commission to 0% for that epoch to ensure that
stake accounts are not disadvantaged by that error.  Not being allowed to do
this in the second half of an epoch is a problem because it would prevent that
commission change until the next epoch, which will not allow this policy to
take effect for stake accounts which were de-activating during the epoch.

## Alternatives Considered

No alternatives were considered.

## New Terminology

None

## Detailed Design

A feature will be added which, when enabled, must cause all node
implementations' vote program's set-commission instruction handling to first
check whether the proposed commission change is a decrease or no change, and if
so, do not invoke the "only allow commission change in first half of epoch"
rule.

## Impact

Validators will now be able to decrease commission at any time in the epoch,
but only increase commission in the first half of epochs (because of the
commission_updates_only_allowed_in_first_half_of_epoch feature already
implemented).

## Security Considerations

None

## Drawbacks

It may cause additional confusion to validators who might not understand why
some types of commission changes succeed only in the first half of epochs while
others succeed always.

## Backwards Compatibility

This feature requires a feature gate because software which includes the
implementation will allow certain set-commission transactions to succeed where
software without the implementation would fail those transactions.  Thus all
validators must be updated to the new functionality at an epoch boundary so
that all validators agree on the result of executing those transactions.

When activated, breaks the ability of older Solana node software to verify
ledgers with this feature.
