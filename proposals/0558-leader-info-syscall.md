---
simd: '0558'
title: Leader Info Syscall
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

Create a new syscall that returns the leader for the current & the next slot.

`fn sol_get_leader(result: *mut u8) -> u64`

## Motivation

Currently it is not possible to identify the leader for a given slot fully
onchain. This information is needed for market makers to update quotes based on
swaps coming in on specific leaders (e.g. a leader is malicious, very remote,
or has historically unreliable scheduling characteristics).

## New Terminology

No new terminology is introduced by this proposal.

## Detailed Design

`result` is a virtual address. On success the syscall writes 128 bytes of
`LeaderInfo` to `[result, result + 128)`.

Compute units are consumed first. If that exceeds the budget, the virtual
machine aborts and no bytes are written.

Otherwise the syscall aborts the virtual machine, without returning to the
caller and without writing `LeaderInfo`, when any of the following are true.
The checks run in this order:

1. The program was loaded by a loader that does not enforce aligned accesses.
   The error is `SyscallError::UnalignedPointer`. This is the same guard the
   sysvar getter syscalls use. `LeaderInfo` is a `#[repr(C)]` struct of four
   pubkeys, so its alignment is 1. When the loader does enforce alignment, the
   address is not rejected for being unaligned.

2. `result >= 0x4_0000_0000` (`MM_INPUT_START`, the input region). The error
   is `SyscallError::InvalidPointer`. This syscall rejects an input-region
   destination directly, the same way a sysvar getter does. The check does
   not depend on SIMD-0459 being active.

3. Any byte in `[result, result + 128)` is not writable. That includes an
   unmapped address, a read-only region, and a range that crosses a region
   boundary. This is an access violation. Address 0 fails here; there is no
   separate null check.

If every check passes, the syscall writes `LeaderInfo` and returns 0.

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

The `LeaderInfo` struct is written to the `result` formal by the syscall.

The `leader_identity` and `leader_vote` fields
must be the block producer's identity pubkey and vote account pubkey for the
current slot. The `next_leader_identity` and `next_leader_vote` fields must be
the block producer's identity pubkey and vote account pubkey for slot
`current_slot + 1`.

### Returned Value

The syscall itself returns `0` on success. If any validation condition fails,
the syscall aborts VM execution without returning to the calling program.

### Program Access

Programs may access the `LeaderInfo` information via the `sol_get_leader` syscall.

No sysvar is introduced.

### CU Cost

The `sol_get_leader` syscall CU cost is defined as:

$$
  \mathtt{sysvar\\_base\\_cost} +
  \max \left(
    \mathtt{mem\\_op\\_base\\_cost},
    \left\lfloor
      \frac{\mathit{sizeof}(\mathtt{LeaderInfo)}}
      {\mathtt{cpi\\_bytes\\_per\\_unit}}
    \right\rfloor
  \right)
$$

As of mainnet epoch 1036, this is 110 CU, which matches `sol_get_sysvar`:

$$
  100 +
  \max \left(
    \left\lfloor \frac{128}{250} \right\rfloor,
    10
  \right)
= 110
$$

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
  leader will usually be the same it is only 110 CU total and other schemes
  for counting the next leader (i.e, the actual next non-current leader)
  seemed to have awkward semantics or would exhibit odd behavior on single-node
  clusters. The % of time that these fields are the same will also decrease
  as we shorten leader windows.
- A full `LeaderSchedule` sysvar containing the entire schedule for the current
  epoch. This was rejected to avoid variable-length sysvars and because the
  account would be well over 200,000 bytes. Additionally, getting the current
  slot from `Clock` and indexing by window offset makes the current-leader
  lookup cost `O(500)` CU instead of `O(110)` CU here.

## Impact

Programs such as market makers may now use this syscall to better update quotes
based on specific and undesirable leader characteristics. This will further
improve the robustness of Solana's market-making environment.

Programs will need to be recompiled and redeployed to adopt this feature.

Similar to the `Clock`, programs relying on this syscall may exhibit different
behavior if simulated and executed at different slots.

## Security Considerations

None

## Backwards Compatibility

Programs accessing this syscall could not be used on Solana versions which do
not implement it. Existing programs that do not use this syscall are not
impacted. Therefore, a feature gate should be used to enable this feature when
the majority of the cluster is using the required version.
