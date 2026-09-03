---
simd: '0558'
title: Current Leader Sysvar
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

Create a new sysvar that tracks the current leader for the current epoch.

Sysvar pubkey:
`SysvarCurrentLeader111111111111111111111111`

## Motivation

Currently it is not possible to identify the leader for a given slot fully
onchain. This information is needed for market makers to update quotes based on
swaps coming in on specific leaders (e.g. a leader is malicious, very remote,
or has historically unreliable scheduling characteristics).

## New Terminology

No new terminology is introduced by this proposal.

## Detailed Design

### Sysvar Account

```rust
#[repr(C)]
pub struct CurrentLeader {
    pub slot: Slot,
    pub epoch_start_timestamp: UnixTimestamp,
    pub epoch: Epoch,
    pub leader_schedule_epoch: Epoch,
    pub unix_timestamp: UnixTimestamp,
    pub leader_identity: Pubkey,
    pub leader_vote: Pubkey,
    pub next_leader_identity: Pubkey,
    pub next_leader_vote: Pubkey
}
```

The `slot`, `epoch_start_timestamp`, `epoch`, `leader_schedule_epoch`, and
`unix_timestamp` values stored here must be equivalent to the corresponding
values stored on the `Clock`. The `leader_identity` and `leader_vote` fields
must be the block producer's identity pubkey and vote account pubkey for the
current slot. The `next_leader_identity` and `next_leader_vote` fields must be
the block producer's identity pubkey and vote account pubkey for slot
`slot + 1`.

### Updating

The fields which the Current Leader sysvar shares with the `Clock` must be
updated in lockstep with the `Clock`. Notably, post-Alpenglow, the
`unix_timestamp` field must be updated via
`BlockComponentProcessor::update_bank_from_footer_fields` (in Agave) to prevent
conflicting `unix_timestamp` values.

The other fields in the Current Leader sysvar must be updated prior to
transaction execution for a given slot. A prudent location to update these
fields is immediately after the `Clock` is updated.

### Program Access

No new syscall is introduced. Programs may access the Current Leader sysvar via
the `sol_get_sysvar` syscall or by passing in the `AccountInfo` explicitly.

### Mismatches

Under TowerBFT, a validator which misimplements this sysvar and produces a block
that has incorrect execution results or an incorrect bankhash will throw a BHM
and crash. The remainder of the cluster will accept the correctly-executed
block and proceed as normal.

In Alpenglow, the produced block will still BHM, but Alpenglow marks BHM as a
cluster-wide dead slot.

### Leader & Vote Pubkeys

For the duration of an epoch, the `leader_identity`, `leader_vote`,
`next_leader_identity`, and `next_leader_vote` fields must correspond to the
canonical leader schedule generated for that epoch. This is to address
mid-epoch `UpdateValidatorIdentity` changes and the fact that multiple vote
accounts may correspond to the same identity.

## Alternatives Considered

- Add fields to the current `Clock`. This was rejected as many programs assert
  invariants like `clock.data().len() == 40`, so adding fields would break these
  programs.
- Only include leader identities. This was rejected as forcing the caller to
  also fetch the `Clock` to get the `(slot, leader)` pair is inefficient.
- Only include the current leader. We opted to include both current and next
  because knowledge of the next leader enables programs to compensate for, for
  example, the next leader having a history of outright censorship. A program
  can then take actions to ensure that its state remains valid even if it is
  censored for the entire next leader window.
- A full `LeaderSchedule` sysvar containing the entire schedule for the current
  epoch. This was rejected to avoid variable-length sysvars and because the
  account would be well over 200,000 bytes. Additionally, getting the current
  slot from `Clock` and indexing by window offset makes the current-leader
  lookup cost `O(500)` CU instead of `O(150)` CU here.

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