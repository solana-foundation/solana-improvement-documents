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