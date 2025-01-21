---
simd: '0197'
title: Chili Peppers
authors:
  - Josh Siegel
  - Jeff Bezaire
  - Tom Pointon
category: Standard
type: Core
status: Review
created: 2024-11-19
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This proposal adds a new consumable resource governing tiered memory
bandwidth usage similar to the way that Compute Units seek to govern
CPU usage. Tiered memory bandwidth will become a performance
bottleneck as transaction throughput and total state size
increase. This proposal serves to outline changes to the Solana
protocol that would enable:

- Deterministic, easily computable and cluster-wide separation of
  state into hot and cold tiers.
- Block level constraints on the total cold to hot state transition.

## Motivation

In commodity hardware (for fixed cost), there is a fundamental
tradeoff between the the size of accessible state and the bandwidth of
random access to that state. On-chip caches >> RAM >> SSD >> HDD >>
NAS increase by orders of magnitude in size, while falling by orders
of magnitude in bandwidth.

For Solana (or any blockchain), treating all state as equivalent
(regardless of its usage patterns) means that either total state size
will be limited by the size of RAM, or the throughput of the network
will be limited to the bandwidth of disks. Actual usage patterns (and
expectations for future usage patterns as the network grows) show that
a relatively small amount of the total state is accessed frequently,
and most of the state is accessed infrequently. This usage pattern
allows a hot/cold tiered state design to allow the total state size
available from disk, while achieving the throughput available from
RAM.

## New Terminology

Cold Storage Loads
  - This is tracked in terms of bytes
  - Accessing meta data is considered a different operation than
    accessing account data and counted as 128 bytes
  - Accessing an alut (address lookup table) will only be counted as
    loading the alut itself and the dereferenced requested account. The
    non-dereferenced accounts in the alut are not made hot.

Hot Cache Size
  - How big is the hot state in bytes
  - Concensus critical

Hot Account Table
  - mapping of accounts/metadata to their Hot Cache Clock Timestamp values

Block Cold State Load Limit
  - The maximum number of cold storage bytes that can be loaded
    in a single block
  - Concensus critical

Hot Cache Clock
  - Number of cold storage bytes loaded that block plus the sum
    of cold storage bytes loaded in previous blocks
  - Concensus critical

Hot Cache Clock Timestamp
  - The value of the Hot Cache Clock the last time a hot account was
    touched (read or written to). Used to determine when an account falls out
    of the hot cache.

## Detailed Design

We will introduce an LRU style of account data cache into solana with
the size being controlled by the "Hot Cache Size" parameter.

A transaction that refers to cold accounts or cold metadata will be
accounted the number of cold bytes that need to be loaded.

The job of producing valid blocks that do not exceed the cold storage
load limit is on the block producer.  Exceeding this value will cause
the entire block to be rejected and the producer wouldnt get paid.

Users would adjust their priority fees to encourage the block
producer to land their blocks if they depend on accessing cold
data.

At the end of a block, the sum of all cold storage bytes loaded in
that block get added to the previous Hot Cache Clock and persisted
into a sysvar.

The Hot Cache Clock Timestamp for all accounts/metadata accessed (both read or
written to) in a block get updated at the end of the block.

Eviction of accounts from the hot cache is done in block/slot sized
batches starting from the oldest hot accounts. Entries in the Hot Account Table
can also be eliminated at this point reducing the size of that table. This
eliminated any ordering dependencies of how accounts were added into the cache.
This also means it is possible to have a total cache size smaller then the Hot
Cache Size.

Note that the hot/cold cache designation in this proposal is strictly a logical
one to allow an in-consensus method of limiting per block bandwidth usage to
"cold" state to ensure that block producers produce blocks that the
majority of validators will be able to successfully replay quickly enough.
Different validator implementations may implement their own tiered state
storage, and individual validators with varying hardware may have different
amounts of the state actually in RAM, etc. Even two identicaly configured
validators might differ in what account data they actually have in RAM due
to different different forks. The designation of hot/cold and the
accounting of data loaded from "cold" within each block is explicitly
independent of the actual "in RAM vs not in RAM" status of account data on on
any individual validator.

## Hot and Cold Account Designation

An account is designated as cold when its Account Hot Cache Clock Timestamp
falls behind the current Hot Cache Clock by more than the Hot Cache
Size parameter.

An account which has never existed is considered cold. An account that is
deleted is still considered hot until its state unit clock lapses into
cold.

Creating an account against a deleted account which is still hot, will create
the hot account again.

This allows for "ephemeral accounts" that are created, used, deleted
repeatedly, reusing the same account address each time to still be considered
hot if they are being reused often. This is apparently a usage pattern
currently used by some applications.

## Snapshots

Hot Cache Account Table is persisted into the snapshot since this
is concensus critical on knowing what is considered cold and what
accounts to evict from the cache.

If this table is not in the snapshot, all accounts are considered cold.

## Bootstrapping

Initially, we will set the Hot Cache Size to 25gb and the Block Cold
Storage Load Limit to inf.   This means there will be no actual effect
on the user community but will cause the "Hot Account Table" to
be initialized correctly and consistantly.

Then, we will then tune the parameters (via features) to more
reasonable numbers.

## Interesting aspects of this design

This creates an economic incentive for the block producer to have addtional RAM
to actually store more metadata than the Hot Cache Size so that it has
additional information it needs to produce better paying blocks by
choosing more attractive cold transactions.

We would also expect the block producer to initially produce entries
with transactions that only reference hot accounts in the early
entries.  This means votes will continue to land and the block chain
will always progress.   Then, once the cold data successfully is
retrieved, it will begin to add in transactions that reference cold
data as well.

## Alternatives Considered

Many

## Impact

If there are sufficiently few requests for new cold storage access in
a block there should be zero impact on the user community or the
validators.

## Security Considerations

Even under a deliberate cold storage DOS attack, the block
chain will proceed by executing votes and transactions that access hot accounts.

## Backwards Compatibility

None required
