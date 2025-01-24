---
simd: '0232'
title: Custom Commission Collector Account
authors: Justin Starry (Anza)
category: Standard
type: Core
status: Review
created: 2025-01-24
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Allow validators to specify custom commission collector accounts where block
revenue and inflation rewards commissions will be deposited.

## Motivation

Validator commission collector accounts must currently be the same as their
identity hot wallet account. This means that program derived addresses are
unable to be used for block revenue collection adding friction to validators
wishing to distribute their revenue in custom ways. By allowing validators to
specify a separate custom commission collector account, they can use onchain
programs to customize how their block revenue is distributed.

## Alternatives Considered

NA

## New Terminology

- Block Revenue Commission Collector: The account used to collect commissioned
block revenue for validators. Previously collected by default into the validator
identity account.

- Inflation Rewards Commission Collector: The account used to collect
commissioned inflation rewards for validators. Previously collected by default
into the vote account.

## Detailed Design

This proposal requires the adoption of SIMD-0180 and SIMD-0185. SIMD-0180
adjusts the leader schedule algorithm to make it possible to designate a
specific vote account for a given leader slot. SIMD-0185 adds block revenue and
inflation rewards commission collector address fields to the vote account state.

### Runtime

#### Block Revenue Commission Collection

When collecting block fee revenue after processing all transactions in a block,
the runtime must look up the validator's specified block revenue collector
address. After adoption of SIMD-0180 and SIMD-0185, a given block's fee
commission collector address can be looked up via the designated vote account
for the leader schedule slot that the block was produced in. Note that by
default, the commission rate for block fee revenue is 100% and support for other
rates will be added in SIMD-0249.

In order to eliminate the overhead of tracking the latest commission collector
address of each vote account, the commission collector address should be fetched
from the state of the vote account at the beginning of the previous epoch. This
is the same vote account state used to build the leader schedule for the current
epoch.

Note that the fee collector constraints defined in SIMD-0085 still hold. The
designated commission collector must be a system program owned account that is
rent-exempt after receiving collected block fee rewards. Additionally, the
designated commission collector must not be a reserved account (note that
currently the only system program owned reserved accounts are the native loader
and the sysvar owner id). If any of these constraints are violated, the fees
collected for that block will be burned. 

#### Inflation Rewards Commission Collection

The inflation rewards collector address should be fetched from the state
of the vote account at the beginning of the previous epoch. This is the same
vote account state used to build the leader schedule for the current epoch.

The designated commission collector must either be a system or vote program
owned account that is rent-exempt after receiving collected block fee rewards.
Additionally, the designated commission collector must not be a reserved account
(note that currently the only system program owned reserved accounts are the
native loader and the sysvar owner id). If any of these constraints are
violated, the inflation rewards commission collected for that epoch will be
burned.

### Vote Program

```rust
pub enum VoteInstruction {
    // ..
    UpdateCommissionCollector { // 16u32
        pubkey: Pubkey,
        kind: CommissionKind,
    },
}

#[repr(u8)]
pub enum CommissionKind {
    InflationRewards = 0,
    BlockRevenue = 1,
}
```

#### `UpdateCommissionCollector`

A new instruction for setting collector accounts will be added to the vote
program with the enum discriminant value of `16u32` little endian encoded in the
first 4 bytes.

## Impact

Validator identity and fee collector accounts no longer need to be the same
account. This opens up the ability to use PDA accounts for block revenue
collection.

Inflation reward commissions no longer need to be collected into a validator's
vote account. This validators more flexibility over managing inflation reward
income.

## Security Considerations

NA

## Drawbacks *(Optional)*

NA

## Backwards Compatibility *(Optional)*

This change will require the use of a new feature gate which will enable
collecting block fees and inflation rewards into custom commission collector
addresses if specified by a validator.
