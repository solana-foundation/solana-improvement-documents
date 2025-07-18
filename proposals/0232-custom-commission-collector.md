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

## Dependencies

This proposal depends on the following previously accepted proposals:

- **[SIMD-0180]: Use Vote Account Address To Key Leader Schedule**

    Necessary for designating a specific vote account for a given leader slot

- **[SIMD-0185]: Vote Account v4**

    Adds necessary block revenue and inflation rewards commission collector
    address fields to the vote account state

[SIMD-0180]: https://github.com/solana-foundation/solana-improvement-documents/pull/180
[SIMD-0185]: https://github.com/solana-foundation/solana-improvement-documents/pull/185

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

### Runtime

#### Block Revenue Commission Collection

When collecting block fee revenue after processing all transactions in a block,
the runtime must look up the validator's specified block revenue collector
address. After adoption of [SIMD-0180] and [SIMD-0185], a given block's fee
commission collector address can be looked up via the designated vote account
for the leader schedule slot that the block was produced in. Note that by
default, the commission rate for block fee revenue is 100% and support for other
rates will be added in [SIMD-0123].

In order to eliminate the overhead of tracking the latest commission collector
address of each vote account, the commission collector address should be fetched
from the state of the vote account at the beginning of the previous epoch. This
is the same vote account state used to build the leader schedule for the current
epoch.

The designated commission collector must either be equal to the vote account's
address OR satisfy ALL of the following constraints:

1. Must be a system program owned account
2. Must be rent-exempt after depositing block revenue commission
3. Must not be a reserved account (note that currently the only system program
owned reserved accounts are the native loader and the sysvar owner id).

If any of these constraints are violated, the fees collected for that block will
be burned.

Note that it's technically allowed to set the collector account to the
incinerator address. Incinerator funds are burned after block revenue collection
at the end of the block.

[SIMD-0123]: https://github.com/solana-foundation/solana-improvement-documents/pull/123

#### Inflation Rewards Commission Collection

For a given epoch `E`, the earned inflation rewards for each vote account are
calculated at the beginning of the next epoch `E + 1`. This proposal doesn't
change the commission calculation but it does define new rules for how the
calculated commission rewards are collected into an account. Rather than
collecting the inflation rewards commission into the vote accounts by default,
the protocol must fetch the inflation rewards commission collector address from
the vote account state at the beginning of epoch `E + 1`. This is the same
vote account state used to get the commission rate and latest vote credits
for inflation rewards calculation.

The designated commission collector must either be equal to the vote account's
address OR satisfy ALL of the following constraints:

1. Must be a system program owned account
2. Must be rent-exempt after depositing inflation rewards commission
3. Must not be a reserved account (note that currently the only system program
owned reserved accounts are the native loader and the sysvar owner id).

If any of these constraints are violated, the inflation rewards commission
collected for that epoch will be burned.

Note that it's technically allowed to set the collector account to the
incinerator address. Incinerator funds are burned at the end of the rewards
distribution block.

### Vote Program

```rust
pub enum VoteInstruction {
    /// # Account references
    ///   0. `[WRITE]` Vote account to be updated with the new collector public key
    ///   1. `[WRITE]` New collector account
    ///   2. `[SIGNER]` Withdraw authority
    UpdateCommissionCollector { // 16u32
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

Perform the following checks:

- If the number of account inputs is less than 2, return
`InstructionError::NotEnoughAccountKeys`
- If the vote account (index `0`) fails to deserialize, return
`InstructionError::InvalidAccountData`
- If the vote account's authorized withdrawer is not an account input for the
instruction or is not a signer, return
`InstructionError::MissingRequiredSignature`
- If the new collector account (index `1`) is not the same as the vote account
and not system program owned, return `InstructionError::InvalidAccountOwner` 
- If the new collector account is not rent-exempt, return
`InstructionError::InsufficientFunds`
- If the new collector account is not writable, return
`InstructionError::InvalidArgument`. Note that this check is performed to ensure
that the new collector account is not a reserved account.

Update the corresponding field for the specified commission kind:

- `CommissionKind::InflationRewards`: update the `inflation_rewards_collector` field
- `CommissionKind::BlockRevenue`: update the `block_revenue_collector` field

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
