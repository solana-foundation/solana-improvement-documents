---
simd: '0558'
title: Leader Info Sysvar
authors:
  - cavey
  - frank
category: Standard
type: Core
status: Draft
created: 2026-08-28
feature:
development:
---

## Summary

Create a new accountless sysvar that returns the leader for the current &
the next slot.

`SysvarLeader1nfo111111111111111111111111111`

## Motivation

Currently it is not possible to identify the leader for a given slot fully
onchain. This information is needed for market makers to update quotes based on
swaps coming in on specific leaders (e.g. a leader is malicious, very remote,
or has historically unreliable scheduling characteristics).

## New Terminology

No new terminology is introduced by this proposal.

## Detailed Design

This sysvar is intended to be accountless and therefore is only
accessible via `sol_get_sysvar`, as described in [SIMD-0127](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0127-get-sysvar-syscall.md).

There is no new syscall introduced by this proposal.

Control flow is specified in SIMD-0127.

### Returned Result

```rust
#[repr(C)]
pub struct LeaderInfo {
    pub leader_identity: Pubkey,
    pub leader_vote: Pubkey,
    pub next_leader_identity: Pubkey,
    pub next_leader_vote: Pubkey
}
```

The `leader_identity` and `leader_vote` fields
must be the block producer's identity pubkey and vote account pubkey for the
current slot. The `next_leader_identity` and `next_leader_vote` fields must be
the block producer's identity pubkey and vote account pubkey for slot
`current_slot + 1`.

### Program Access

Programs may access the `LeaderInfo` sysvar via the `sol_get_sysvar` syscall.

No new syscall is introduced.


### CU Cost

CU cost is specified in SIMD-0127.

### Leader & Vote Pubkeys

For the duration of an epoch, the `leader_identity`, `leader_vote`,
`next_leader_identity`, and `next_leader_vote` fields must correspond to the
canonical leader schedule generated for that epoch. This is to address
mid-epoch `UpdateValidatorIdentity` changes and the fact that multiple vote
accounts may correspond to the same identity.

### Epoch Boundaries

On the last slot of an epoch, when the next leader is in epoch
`current_epoch + 1`, the `next_leader` value represents the first leader of
the next epoch.  This value comes from the next epoch's leader schedule.

## Alternatives Considered

- Add fields to the current `Clock`. This was rejected as many programs assert
  invariants like `clock.data().len() == 40`, so adding fields would break these
  programs.
- Only include the current leader. We opted to include both current and next
  because knowledge of the next leader enables programs to compensate for, for
  example, the next leader having a history of outright censorship. A program
  can then take actions to ensure that its state remains valid even if it is
  censored for the entire next leader window.  Even though the current / next
  leader will usually be the same it is only 64 bytes / CU and other schemes
  for counting the next leader (i.e, the actual next non-current leader)
  seemed to have awkward semantics or would exhibit odd behavior on single-node
  clusters. The % of time that these fields are the same will also decrease
  as we shorten leader windows.
- A full `LeaderSchedule` sysvar containing the entire schedule for the current
  epoch. This was rejected to avoid variable-length sysvars and because the
  account would be well over 200,000 bytes. Additionally, getting the current
  slot from `Clock` and indexing by window offset makes the current-leader
  lookup cost `O(500)` CU instead of `O(228)` CU here.

## Impact

Programs such as market makers may now use this sysvar to better update quotes
based on specific and undesirable leader characteristics. This will further
improve the robustness of Solana's market-making environment.

Programs will need to be recompiled and redeployed to adopt this feature.

Similar to the `Clock`, programs relying on this sysvar may exhibit different
behavior if simulated and executed at different slots.

## Security Considerations

None

## Backwards Compatibility

Programs accessing this sysvar could not be used on Solana versions which do
not implement it. Existing programs that do not use this sysvar are not
impacted. Therefore, a feature gate should be used to enable this feature when
the majority of the cluster is using the required version.
