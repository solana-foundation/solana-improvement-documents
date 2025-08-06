---
simd: '0123'
title: Block Revenue Sharing
authors: Justin Starry (Anza)
category: Standard
type: Core
status: Review
created: 2024-03-10
feature: (fill in with feature tracking issues once accepted)
---

## Summary

A new mechanism is proposed to allow validators to share part of their block
revenue with their delegators. Commission rates from validator vote accounts
will be used by the protocol to calculate post-commission rewards that will be
automatically distributed to delegated stake accounts after an epoch is
completed.

## Motivation

Delegated stake directly increases the number of blocks that a validator is
allocated in an epoch leader schedule but the core protocol doesn't support
diverting any of that extra revenue to stake delegators.

## Dependencies

This proposal depends on the following previously accepted proposals:

- **[SIMD-0180]: Use Vote Account Address To Key Leader Schedule**

    Necessary for looking up a block producer's vote account

- **[SIMD-0185]: Vote Account v4**

    Introduces version 4 of the vote account state, which adds new fields
    for block revenue commission and pending delegation rewards

- **[SIMD-0232]: Custom Commission Collector Account**

    Necessary for looking up a block producer's commission collector account

- **[SIMD-0291]: Commssion Rate in Basis Points**

    Introduces a new instruction type for setting commission rates in basis
    points

[SIMD-0180]: https://github.com/solana-foundation/solana-improvement-documents/pull/180
[SIMD-0185]: https://github.com/solana-foundation/solana-improvement-documents/pull/185
[SIMD-0232]: https://github.com/solana-foundation/solana-improvement-documents/pull/232
[SIMD-0291]: https://github.com/solana-foundation/solana-improvement-documents/pull/291

## Alternatives Considered

### Distribute Rewards as Activated Stake

The runtime could ensure that any distributed stake rewards get activated as
well but it would require extra complexity in the protocol to support that
feature. Instead, stakers will receive inactive SOL in their stake accounts that
they will have to manage themselves. [SIMD-0022] aims to make this experience
better for stakers by allowing stake accounts to separately delegate any
unstaked balance in their accounts.

[SIMD-0022]: https://github.com/solana-foundation/solana-improvement-documents/pull/22

### Out of protocol reward distribution 

Due to the lack of core protocol support for distributing block revenue to
stakers, validators have developed their own solutions which are not enforced by
the core protocol. For example, the Cogent validator diverts part of its fee
revenue to NFT holders. But it's up the NFT holders to audit and hold Cogent
accountable to a specific commission rate.

Another alternative is Jito's mechanism for block "tips" (not fees, but the idea
is similar). Jito's validator implementation includes a tip distribution program
which it instructs validator operators to divert all of their tips to but cannot
enforce perfect compliance. It's up to stakers and the Jito team to audit
compliance by validator operators. This mechanism requires trusting a
third-party (in this case Jito) to calculate reward distribution in an accurate
and fair manner. It also relies on using a merkle tree to distribute fees to all
stake accounts and the distributed fees are not automatically staked in
recipient stake accounts.

## New Terminology

NA

## Detailed Design

### Runtime: Block Revenue Collection

After all transactions are processed in a block for a given leader, rather than
collecting all block revenue into the validator identity account, the protocol
will look up the block producer's vote account as described in [SIMD-0180]. Then
it MUST check if the validator's vote account has specified a block revenue
commission rate and collector addresses in the new vote account version
described in [SIMD-0185]. As described in [SIMD-0232], the latest block revenue
commission rate and collector address MUST be loaded from the vote account state
at the beginning of the previous epoch. This is the same vote account state used
to build the leader schedule for the current epoch.

If the block revenue commission rate and collector account aren't set (e.g., the
vote account state version has not been updated to v4 yet), all revenue will be
collected into the validator's identity account as before. If the block revenue
commission rate and collector account *are* specified, the rewards MUST be
distributed according to the commission and delegator rewards collection
sections below.

#### Commission Collection

The commission amount MUST be calculated by first multiplying the amount of
revenue by the lesser of the vote account's block revenue commission rate or the
maximum of `10,000` basis points. Then use integer division to divide by
`10,000` and discard the remainder. If the commission amount is non-zero, the
block revenue commission collector account MUST be loaded and checked for the
following conditions:

1. account is system program owned AND
2. account is rent-exempt after depositing the commission.

If the conditions are met, the commission amount MUST be deposited into the
block revenue commission collector account. If either of these conditions is
violated, the commission amount MUST be burned.

#### Delegator Rewards Collection

The delegator rewards amount MUST be calculated by subtracting the calculated
commission from the block fee revenue. If the delegator rewards amount is
non-zero, the vote account must be loaded and checked for the following
conditions:

1. account is vote program owned AND
2. account is initialized with vote state v4 or later

If the conditions are met, the delegator rewards amount MUST be added to the
vote state field `pending_delegator_rewards` and added to the balance of vote
account. If either of these conditions is violated, the delegator rewards amount
MUST be burned.

### Runtime: Delegator Rewards Distribution

When calculating stake delegation rewards for a particular completed reward
epoch, construct a list of all vote accounts that were initialized at the
beginning of the reward epoch and had a non-zero active stake delegation. For
each vote account, retrieve its state at the end of the reward epoch and check
the `pending_delegator_rewards` field in its vote state. Let this value be `P`.
If `P` is non-zero, use it to calculate rewards for each of the stake accounts
delegated to the vote account as follows: 

1. Sum all active stake delegated to the vote account during the reward epoch
epoch. Let this total be `A`.

2. For each individual stake account, multiply its active stake from the
reward epoch by `P`, and divide the result by `A` using integer division.
Discard any fractional lamports.

After calculating all individual stake rewards, sum them to obtain `D`, the
total distribution amount. Because of integer division, the full amount `P` may
not be distributed so compute the amount to be burned, `B`, as the difference
between `P` and `D`.

If no blocks in the epoch following the completed reward epoch have been
processed yet, subtract `B` from both the vote account’s lamport balance and its
`pending_delegator_rewards` field and store the updated vote account. Finally,
the burn amount `B` should also be deducted from the cluster capitalization.

#### Individual Delegator Reward

The stake reward distribution amounts for each stake account calculated above
can then be used to construct a list of stake reward entries which MUST be
partitioned and distributed according to [SIMD-0118].

When reward entries are used to distribute rewards pool funds during partitioned
rewards distribution, the delegated vote account for each rewarded stake account
must have its `pending_delegator_rewards` field and its balance deducted with
the amount of rewards distributed to keep capitalization consistent.

[SIMD-0118]: https://github.com/solana-foundation/solana-improvement-documents/pull/118

### Vote Program

#### Withdraw

Since pending delegator rewards will be stored in the validator's vote account
until distribution at the next epoch boundary, those funds will be unable to be
withdrawn.

The `Withdraw` instruction must be modified so that if the balance indicated by
the `pending_delegator_rewards` field is non-zero, the vote account will no
longer be closeable by fully withdrawing funds. The withdrawable balance when
`pending_delegator_rewards` is non-zero will be equal to the vote account's
balance minus `pending_delegator_rewards` and the minimum rent exempt balance.

#### UpdateCommissionBps

The `UpdateCommissionBps` instruction added in [SIMD-0291] must be updated to
add support for updating the block revenue commission rate.

When the specified commission kind is `CommissionKind::BlockRevenue`, update the
`block_revenue_commission_bps` field instead of the previous behavior of
returning an `InstructionError::InvalidInstructionData`.

Note that the commission rate is allowed to be set and stored as any `u16` value
but as detailed above, it will capped at 10,000 during the actual commission
calculation.

#### DepositDelegatorRewards

A new instruction for distributing lamports to stake delegators will be added to
the vote program with the enum discriminant value of `18u32` little endian
encoded in the first 4 bytes.

```rust
pub enum VoteInstruction {
    /// # Account references
    ///   0. `[WRITE]` Vote account to be updated with the deposit
    ///   1. `[SIGNER, WRITE]` Source account for deposit funds
    DepositDelegatorRewards { // 18u32
        deposit: u64,
    },
}
```

Perform the following checks:

- If the number of account inputs is less than 2, return
`InstructionError::NotEnoughAccountKeys`
- If the vote account (index `0`) fails to deserialize, return
`InstructionError::InvalidAccountData`
- If the vote account is not initialized with state version 4, return
`InstructionError::InvalidAccountData`

Then the processor should perform a system transfer CPI of `deposit` lamports
from the source account (index `1`) to the vote account. Lastly, increment the
`pending_delegator_rewards` value by `deposit`.

## Impact

Stake delegators will receive additional income when delegating to validators
who adopt this new feature by setting a block revenue commission rate less than
the default of `100%`.

## Security Considerations

NA

## Backwards Compatibility

A feature gate will be used to enable block reward distribution at an epoch
boundary.
