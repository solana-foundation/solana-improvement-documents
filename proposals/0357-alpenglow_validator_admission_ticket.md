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
happen here, but it doesn’t contain vote information any more. This will be
the account where the 1.6 SOL VAT is deducted from.

- Identity account: It continues receiving block rewards and priority fees.
But we may change that in a separate SIMD.

## Detailed Design

### What is considered a valid vote account

A valid vote account in an Alpenglow epoch must contain:

- a BLS public key

- at least 1.6 SOL VAT fee plus the necessary storage rent amount for a new
epoch in the vote account

When the staked validators for a new epoch are calculated, all validators must
perform the following operations:

- When there are more than 2,000 valid validators, sort all valid vote accounts
by descending order of stake. If some validator with stake S is in position
2001, then we remove all validators with stake S and less. If there are fewer
than 2,000 valid validators, pick all of them.

- Deduct 1.6 SOL VAT fee from each picked vote account once

- Mark the fee burned and write the result into the bank

### How to implement the checks

1. When a new bank crosses an epoch boundary (bank.epoch() >
parent_bank.epoch()), calculate the participating staked validators for the
next epoch (bank.epoch() + 1). This is the same as now.

2. Perform stake activation and deactivation so that the intended stake values
are used for the new epoch. This operation occurs before any transactions are
processed in the new epoch’s bank.

3. The calculation iterates all vote accounts and filters those that meet
the following criteria:

  - The account has a balance of at least 1.6 SOL

  - The account has a valid BLS compressed public key (i.e., it can be
    correctly decompressed)

4. If the number of filtered accounts exceeds 2,000, sort them according to the
following rules and select the top 2,000. Otherwise, return the entire list:

  - Sort in descending order of stake (largest to smallest)

  - If multiple validators have exactly the same amount of stake and including
  all of them would exceed the 2,000 limit, then exclude all of them

5. Subtract 1.6 SOL from the vote account for each validator in the accepted
list from the previous step

6. Record the VAT fee subtraction in the bank, which reduces the bank’s
capitalization. The recording of fee subtraction occurs when the bank is
frozen even though fee reduction happens before any transaction in the
bank is executed.

## Operation Considerations

- To be included in epoch e+1, validator operators must ensure the vote
account has at least 1.6 SOL before epoch e-1 ends

- Validator operators must ensure they have valid BLS public key specified in
their vote account

- We will not allow removing the BLS public key

## Alternatives Considered

- Have all validators send a transaction to deduct 1.6 SOL VAT fee every epoch.
A few problems with this approach:

  - This transaction must land before the staked validators are selected

  - If this validator ends up not being selected, this VAT fee needs to be
returned, which means we have to implement some type of accounting and ensure
the VAT fee returning properly lands on the chain.

  - It does deduct an additional transaction fee from the vote account

  - If somehow validators make a mistake by sending multiple transactions,
we also need to return the fee collected.

## Impact

Validators not providing BLS public key or desired fee will not be able to
participate in an Alpenglow epoch regardless of their stake. Also, only the
nodes selected by this process will receive votes or certificates from other
selected validators in real time.

Validator operators need to ensure they have enough funds and correct BLS
public key before end of epoch e-1 to participate in epoch e+1. This poses some
new operation challenges.

We are requiring correct setup of vote accounts at end of epoch e-1 for
inclusion in epoch e+1 because of the following reason: We want to be
absolutely sure that we have a finalized state before we enter epoch e+1.
Specifically, if the last slots of epoch e-1 are not finalized at the end
of the epoch, we have enough slots in epoch e to finalize epoch e-1's state.

## Security Considerations

Keeping funds in vote accounts for VAT is safer than keeping funds in identity
accounts. Because the keypair for identity account needs to be in a hot wallet
to support real-time validator operations, while the vote authorized withdrawer
keypair does not need to be in a hot wallet.

## Backwards Compatibility

This feature is not backwards compatible.