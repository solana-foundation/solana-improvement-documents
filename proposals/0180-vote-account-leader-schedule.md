---
simd: '0180'
title: Vote Account Address Keyed Leader Schedule
authors: Justin Starry (Anza)
category: Standard
type: Core
status: Review
created: 2024-10-03
feature: (fill in with feature tracking issues once accepted)
---

## Summary

The epoch leader schedule for block production will be migrated from being keyed
by validator identity addresses to being keyed by vote account addresses. The
expected block signer for a given slot will be determined by the vote account's 
designated validator identity.

## Motivation

Using validator identity addresses in the leader schedule means there is no
straightforward way to map a block producer to a particular vote account and its
delegated stake. This is because the same validator identity could be designated
by multiple vote accounts. By migrating the leader schedule to being keyed by
vote account addresses, we know exactly what delegated stake led to a
validator's leader schedule slot allocation. This will make certain protocol
improvements much easier to design like how to distribute block rewards and how
to slash validators that produce duplicate blocks.

## New Terminology

NA

## Detailed Design

### Leader Schedule Generation

The leader schedule MUST continue to be generated at epoch boundaries with the
existing stake weighted randomized leader schedule algorithm. However, the stake
weights used in this algorithm MUST be adjusted to be accumulated by delegated
vote account address rather than accumulating all stake by validator identity
address. As before, delegated vote accounts MUST be valid to be included in the
leader schedule generation algorithm.

### Valid Vote Accounts

For reference, valid vote accounts are defined as accounts with the following
requirements:

- non-zero lamport balance
- owned by the vote program (`Vote111111111111111111111111111111111111111`)
- either:
  - data size of 3731 bytes and `data[4..86] != [0; 82]`
  - data size of 3762 bytes and `data[4..118] != [0; 114]`

### Validator Identity Address Lookup

Block shreds MUST still be signed by the validator identity private key and
block rewards MUST still be collected into the validator identity account (also
known as fee collection account).

Once the leader schedule is keyed by vote account addresses, validator identity
pubkey's will need to be looked up by loading the vote account state for the
designated vote account address for a particular leader slot. Since the vote
program allows updating the validator identity address at any time after leader
schedule generation, the vote account state from the beginning of the previous
epoch MUST be used.

Since only valid vote accounts are used during leader schedule generation, a
valid vote account is guaranteed to exist in epoch stakes and its validator
identity address can be fetched from its account state.

### RPC Migration

Existing leader schedule and slot leader RPC endpoints SHOULD continue returning
the resolved validator identity address to avoid breaking downstream users of
these endpoints that expect the leader schedule to use validator identity.
However, new RPC endpoints for fetching the new leader schedule keyed by vote
account addresses SHOULD be added.

## Alternatives Considered

Alternatively, the protocol could create a strict one-to-one mapping between
validator identity addresses and vote account addresses. However this would
require quite a lot of onchain program and account state changes to be able to
enforce this mapping. And migrating existing one-to-many relationships is not
very straightforward and would likely require validators to manually migrate
which could take a long time.

## Impact

Negligible impact expected. There will be some extra overhead to looking up /
caching the validator identity address for each vote account address.

## Security Considerations

NA

## Drawbacks *(Optional)*

NA

## Backwards Compatibility *(Optional)*

Feature gate will be required to enable this migration since leader schedule
generation will be different.
