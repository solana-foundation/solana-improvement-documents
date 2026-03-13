---
simd: '0488'
title: Rent-Adjusted Stake Delegations
authors:
  - Jon C (Anza)
category: Standard
type: Core
status: Review
created: 2026-03-06
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

A new calculation is proposed to adjust stake delegation amounts during the
epoch rewards payout system of
[SIMD-118](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0118-partitioned-epoch-reward-distribution.md),
based on the Rent sysvar parameters at the beginning of that epoch.

## Motivation

This proposal is a prerequisite for
[SIMD-0438 (Rent Increase)](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0438-rent-increase-safeguard.md).

If SIMD-0438 is enabled after any of the Rent decreases described in
[SIMD-0437](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0437-incremental-rent-reduction.md),
a stake account created with lower Rent may have an inflated delegation amount,
consisting of the delta between the previous (decreased) minimum balance and the
new (increased) minimum balance.

Although the potential divergence is on the order of 1/1000 of a SOL per stake
account, an incorrect delegation amount gives validators an artificially higher
stake weight, reflecting stake that is not backed by lamports in a stake
account.

## New Terminology

None.

## Detailed Design

During the partitioned epoch rewards calculation outlined in
[SIMD-118](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0118-partitioned-epoch-reward-distribution.md),
a stake's updated delegation MUST be calculated with the following formula:

```
post_delegation = min(
    pre_delegation + stake_rewards,
    lamports + stake_rewards - rent_exempt_reserve
)
```

Where:

* `post_delegation`: the account's post-reward delegated lamport amount
* `pre_delegation`: the account's pre-reward delegated lamport amount
* `stake_rewards`: the account's calculated stake reward lamport amount for the
  past epoch
* `lamports`: the account's pre-reward lamports
* `rent_exempt_reserve`: the minimum lamport balance required for the stake
  account

All arithmetic operations MUST be saturating and use unsigned 64-bit integers.

The `rent_exempt_reserve` calculation MUST use current `Rent` sysvar parameters.
Any updates to the `Rent` sysvar values MUST take place before epoch rewards
calculation takes place.

During distribution, the `delegation.stake` field (offset `[156,164)`) in the
stake account's data MUST be set to the new delegation amount, expressed as a
little-endian unsigned 64-bit integer.

If the new delegation amount is 0, then `delegation.deactivation_epoch` (offset
`[172,180)`) MUST be set to the rewarded epoch, expressed as a little-endian
unsigned 64-bit integer.

If the stake account does not have enough lamports to meet the minimum balance,
no other change is required. The account will continue to exist as any other
account that does not meet minimum balance requirements as described in
[SIMD-0392](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0392-relax-minimum-balance-check.md).

The stake weight for the delegated vote account MUST take into account the new
calculation.

New entries in the Stake History sysvar MUST take into account the adjusted
delegation amounts.

During the implementation of block revenue distribution in
[SIMD-0123](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0123-block-revenue-distribution.md),
block rewards MUST be used to cover the new required minimum balance, so
the formula becomes:

```
post_delegation = min(
    delegation + stake_rewards,
    lamports + stake_rewards + block_rewards - rent_exempt_reserve
)
```

Where `block_rewards` represents the block rewards earned by the stake account
in that epoch. All other variables are the same as before.

## Alternatives Considered

We could fix the minimum balance for stake accounts to the current minimum
balance for 200 bytes. This approach breaks any existing on-chain programs or
tooling that use the Rent sysvar to calculate the minimum balance of a stake
account.

We could explicitly tie this logic to
[SIMD-0438](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0438-rent-increase-safeguard.md)
or any future proposal to increase rent. That approach would complicate a rent
increase feature, and complicate any attempt for dynamic rent in the future.

This proposal even works if Rent changes dynamically during an epoch, since
delegation amounts will always be correct when new stake weights are calculated.

## Impact

The biggest impact is that a delegation MAY decrease between epochs. Any
consumer (on-chain program, dApp, etc) MUST relax assumptions that delegation
amounts only increase or stay the same.

Consumers MUST allow for stake accounts to become inactive as a result of reward
distribution, without an explicit call to `Deactivate` or `DeactivateDelinquent`.

## Security Considerations

Nothing to note in particular.

In general, reward calculation and distribution is a complicated part of the
protocol, and this change introduces more complexity to the calculation portion.
