---
simd: '0054'
title: Sysvar for active stake
authors:
  - x19
category: Standard
type: Core
status: Draft
created: 2023-06-7
feature: (fill in with feature tracking issues once accepted)
---

## Summary

We propose to add a new sysvar that contains vote account pubkeys and 
their corresponding active stake.

## Motivation

Currently, if a validator wants to prove its active stake to a program, 
it needs to 
pass in all of the stake accounts which have delegated to it. This is 
infeasible due to the large number of stake accounts this would require.

Using the proposed sysvar, a program can look up the corresponding 
vote account and verify the amount of active stake it has delegated to it.

This sysvar would unlock new use cases which use stake amount in their logic 
including on-chain governance, attestations, and more.

## Alternatives Considered

None

## New Terminology

None

## Detailed Design

- sysvar structure: `Vec<(vote_account: Pubkey, active_stake_in_lamports: u64)>`
- sysvar address: `SysvarStakeWeight11111111111111111111111111`


### Ordering

The sysvar structure would be sorted by `vote_account` in ascending order
to improve look-up speeds. There will also never be any 
duplicate `vote_account` entries.

### Serialization

The serialization format of the sysvar will use a u64 
for the vector's length, followed by 40 bytes per entry 
(32 bytes for the Pubkey and 8 for the active stake). 

### Maximum Sysvar Size

We also need to consider a maximum data size for the sysvar. 
Currently, there are 3422 vote accounts on mainnet (1818 active and 1604 delinquint),
so we can use a maximum limit of 4096 entries and still include 
all the vote accounts for now.
Using 4096 as the max number of entries the size would be (8 + 40 * 4096) = 
163,848 bytes. Once the number of entries exceeds the max allowed, 
vote accounts with the least amount of stake will be removed from the sysvar. 

### Changes Required

Stake weight information should already be available on full node clients 
since it's required to construct the leader schedule. Since stake weights 
can only be
modified on a per-epoch basis, validators will only need to update this 
account on epoch boundaries.

We would also need a new feature gate to activate this sysvar.

## Impact

Implementing the proposed sysvar will enable new types of programs which are 
not possible now,
improving Solana's ecosystem.

## Security Considerations

None 

## Backwards Compatibility

Existing programs are not impacted at all.