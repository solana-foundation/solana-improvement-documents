---
simd: '0357'
title: Alpenglow Validator Admission Ticket
authors:
  - Wen Xu (Anza)
  - Roger Wattenhofer (Anza)
category: Standard
type: Core
status: Review
created: 2025-09-11
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

This SIMD describes how the validator admission ticket (VAT) collection
described in [SIMD 326](https://github.com/solana-foundation/solana-improvement-documents/pull/326)
will be implemented. Specifically, how it affects validator operation
procedures.

The validator admission ticket is a mechanism translating the current cost of
voting into a similar economic equilibrium for Alpenglow. By charging every
voting validator 1.6 SOL per epoch, it replaces the current voting fee at ~2
SOL per epoch, and reduces the likelihood there are too many voting validators
immediately after Alpenglow launches.

The general VAT concept has already been accepted with the governance vote on
SIMD 326.

Everything specified below are protocol-level changes, they need to be
implemented by all Solana clients. The client specific data structure
changes are omitted.

## Dependencies

The Validator Admission Ticket is specified in SIMD 326 (Alpenglow).

The compressed BLS public key in Vote Account is specified in [SIMD 185 (Vote
Account v4)](https://github.com/solana-foundation/solana-improvement-documents/pull/185)

## Motivation

Adding more staked validators to a blockchain does come with costs. At the
very least, the votes and corresponding rewards need to be processed by
every voting validator. Therefore, every additional staked validator increases
the number of messages for every validator.

Right now every voting validator pays voting fee on any vote transaction
included in a block. The voting fee adds up to ~2 SOL per epoch if the
validator votes most of the time. It is a burden, yet at the same time it's an
economic barrier to having too many voting validators.

VAT is only a temporary solution to maintain the current economic equilibrium.
This proposal intentionally strives to keep voting validator protocol costs
similar to pre-Alpenglow consensus.

## New Terminology

- **Validator Admission Ticket(VAT)**: The 1.6 SOL charged once per epoch to
every validator eligible to participate in the next epoch

## Different Vote Related Accounts

Before Alpenglow, we have the following accounts related to voting:

- Vote account for saving all the vote states and receiving commission

- Identity account for receiving block rewards and priority fees, the vote
transaction fees are currently paid out of this account

After Alpenglow, we will still have roughly the same accounts serving the same
purpose:

- Vote account: This account must contain the correct BLS public key
corresponding to vote authority keypair. It continues to keep all the vote
credits and all validator identity/authority/commission information updates
happen here, but the `votes` list in vote state will be empty. This will be
the account where the VAT is deducted from.

- Identity account: It continues receiving block rewards and priority fees.

## Detailed Design

### How to implement the checks

1. When a new bank crosses an epoch boundary (bank.epoch() >
parent_bank.epoch()), calculate the participating staked validators for the
next epoch (bank.epoch() + 1). This is the same as now.

2. Perform stake activation and deactivation so that the intended stake values
are used for the new epoch. This operation occurs before any transactions are
processed in the new epoch’s bank.

3. The calculation iterates all vote accounts and filters those that meet
the following criteria:

  - The account has a balance of at least VAT plus the necessary storage
  rent amount for a new epoch

  - The account has a BLS public key

4. If the number of filtered accounts exceeds 2,000, sort them according to the
following rules and select the top 2,000. Otherwise, return the entire list:

  - Sort in descending order of stake (largest to smallest)

  - Do not tiebreak. If total valid vote accounts exceed the limit of 2000,
  omit all accounts whose stake is equal to that of the 2001st account

5. Subtract VAT from the vote account for each validator in the accepted
list from the previous step

6. Record the VAT fee subtraction in the bank, move the lamports directly into
the incinerator account.

## Alternatives Considered

- Have all validators send a transaction to deduct VAT every epoch. A few
problems with this approach:

  - This transaction must land before the staked validators are selected

  - If this validator ends up not being selected, this VAT fee needs to be
returned, which means we have to implement some type of accounting and ensure
the VAT fee returning properly lands on the chain.

  - It does deduct an additional transaction fee from the vote account

  - If somehow validators make a mistake by sending multiple transactions,
we also need to return the fee collected.

## Impact

The voting set of validators will be strictly capped at 2,000.

The validator operators must also take care to keep their vote account topped
up, instead of relying on funds inside identity account.

## Security Considerations

Keeping funds in vote accounts for VAT is safer than keeping funds in identity
accounts. Because the keypair for identity account needs to be in a hot wallet
to support real-time validator operations, while the vote authorized withdrawer
keypair does not need to be in a hot wallet.

In furtherance of improving operational security, a future simd may change
the deposit location of validator rewards to the vote account. This would
eliminate the necessity for any funds to be controlled by a hot keypair.

## Backwards Compatibility

This feature is not backwards compatible.