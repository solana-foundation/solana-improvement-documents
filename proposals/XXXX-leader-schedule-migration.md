---
simd: 'XXXX'
title: Leader Schedule Migration
authors: Justin Starry (Anza)
category: Standard
type: Core
status: Draft
created: 2024-10-03
feature: (fill in with feature tracking issues once accepted)
---

## Summary

The epoch leader schedule for block production will be migrated from using
validator identity addresses to using vote account addresses. The expected
block signer for a given slot will be determined by the vote account's 
designated validator identity.

## Motivation

Using validator identity addresses in the leader schedule means there is no
straightforward way to map a block producer to a particular vote account and its
delegated stake. This is becuase the same validator identity could be designated
by multiple vote accounts. By migrating to the vote account address, we know
exactly what delegated stake led to a validator's leader schedule slot
allocation. This will make certain protocol improvements much easier to design
like how to distribute block rewards and how to slash validators that produce
duplicate blocks.

## New Terminology

NA

## Detailed Design

### Leader Schedule Generation

When generating the leader schedule at epoch boundaries, rather than
accumulating all stake by the node id, stake should be accumulated according to
vote pubkey. Then use the existing stake weighted randomized leader schedule
generation using vote pubkeys and their delegated stake rather than node id
pubkeys and the accumulated delegated stake across (potentially more than one)
vote accounts. As before, only valid and initialized vote accounts should be
used during leader schedule generation.

### Node Id Lookup

Block shreds should still be signed by a node pubkey and block rewards should
still be collected into the node id account (also known as fee collection
account). However, after the migration this node pubkey will need to be looked
up by first finding the vote account for the designated vote pubkey for a
particular leader slot in bank epoch stakes. Bank epoch stakes are keyed by
leader schedule epoch and therefore the vote account state should be retrieved
by looking up the epoch stakes for the current epoch. Since only valid vote
accounts are used during leader schedule generation, a valid vote account is
guaranteed to exist in epoch stakes and its node pubkey can be fetched from its
account state.

### RPC Migration

Existing leader schedule and slot leader RPC endpoints should continue returning
the resolved node id to avoid breaking downstream users of these endpoints that
expect the leader schedule to have node pubkeys. However, new RPC endpoints
for fetching the new leader schedule using vote pubkeys should be added.

## Alternatives Considered

Alternatively, the protocol could create a strict one-to-one mapping between
node pubkeys and vote accounts. However this would require quite a lot of
onchain program and account state changes to be able to enforce this mapping.
And migrating existing one-to-many relationships is not very straightforward and
would likely require validators to manually migrate which could take a long
time.

## Impact

Negligible impact expected. There will be some extra overhead to looking up /
caching the node pubkey for each vote pubkey.

## Security Considerations

NA

## Drawbacks *(Optional)*

NA

## Backwards Compatibility *(Optional)*

Feature gate will be required to enable this migration since leader schedule
generation will be different.
