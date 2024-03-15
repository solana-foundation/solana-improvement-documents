---
simd: '0127'
title: sol_get_sysvar syscall
authors:
  - Richard Patel
category: Standard
type: Core
status: Draft
created: 2024-03-15
---

## Summary

SIMD-0127 introduces the syscall `sol_get_sysvar` which allows BPF programs
to retrieve any sysvar.

New invariants are placed on the sysvar cache to enable safe retrieval.

Existing syscalls to retrieve sysvars are henceforth considered obsolete.

## Motivation

Sysvars are special accounts that are implicitly modified by the runtime.
Certain sysvars are essential for operation of the network.  For example, the
system program reads the 'recent block hashes' sysvar to provide durable nonce
functionality.

Currently, programs can read sysvars can using one of these three methods:

1. Via explicit account access by specifying the sysvar address in the list of
   transaction accounts
2. Via API calls to the sysvar cache (for native programs only)
3. Via syscalls that copy the sysvar content to virtual machine memory
   (for BPF programs only, does not work for all sysvars)

Improvements to these APIs are motivated by the following reasons.

### Allowing sysvar upgrades

The function signature of a `sol_get_{...}_sysvar` syscall is as follows:

```c
uint64_t sol_get_example_sysvar( void * dest_addr );
```

When called, the syscall handler will copy the sysvar to the given memory
location.  Notably, BPF programs do not inform the syscall handler of the
expected size of the sysvar.  BPF programs thus effectively assume that the
requested sysvars will forever have the same size.

This prevents future upgrades that shrink or extend sysvars.

### Syscall API fragmentation

The following sysvars are currently accessible using syscalls:

- `SysvarC1ock11111111111111111111111111111111` (via `sol_get_clock_sysvar`)
- `SysvarEpochRewards1111111111111111111111111` (via `sol_get_epoch_rewards_sysvar`)
- `SysvarEpochSchedu1e111111111111111111111111` (via `sol_get_epoch_schedule_sysvar`)
- `SysvarFees111111111111111111111111111111111` (via `sol_get_fees_sysvar`)
- `Sysvar1nstructions1111111111111111111111111` (indirectly via `sol_get_processed_sibling_instruction`)
- `SysvarLastRestartS1ot1111111111111111111111` (via `sol_get_last_restart_slot`)
- `SysvarRent111111111111111111111111111111111` (via `sol_get_rent_sysvar`)

BPF programs cannot access the following sysvars using syscalls.

- `SysvarRecentB1ockHashes11111111111111111111`
- `SysvarRewards111111111111111111111111111111`
- `SysvarS1otHashes111111111111111111111111111`
- `SysvarS1otHistory11111111111111111111111111`
- `SysvarStakeHistory1111111111111111111111111`

Introducing a new syscall for every new sysvar results in bloat of the syscall
interface.  The aforementioned function signature that most sysvar getters use
is also inefficient for large accounts.  Copying the entire sysvar to VM memory
might consume excessive compute units (e.g. 16392 bytes for 'stake history').

### Unblocking core BPF programs

As per [SIMD-0088](./0088-enable-core-bpf-programs.md), an effort is underway
to reduce code complexity of the Solana runtime by porting native programs to
core BPF programs.

These new programs target full compatibility with their predecessors to avoid
introducing breaking API changes.  In some cases, native programs read a sysvar
from the sysvar cache without requiring the user to include this sysvar in the
transaction account list.  This happens in the stake program 'redelegate'
instruction, for example.

As a result, it is currently impossible to port certain native programs to core
BPF programs.  Another factility to expose sysvars to the virtual machine is
required.

### Optimizing existing user programs

Reading sysvars with the proposed syscall is expected to be more efficient than
an explicit account access.  Removing explicit account accesses to sysvars will
generally result in smaller transactions.  Reducing the amount of account data
that is mapped into virtual machine memory may also yield cost savings.

## Alternatives Considered

Other mechanisms considered that provide sysvar data to BPF programs are as
follows:

### Sysvar-specific syscalls

TODO

### New memory regions

TODO

## Detailed Design

### Sysvar Cache

TODO

### Restrictions on sysvar data dependencies

TODO

### `sol_get_sysvar` syscalls

TODO this is outdated

```c
/* sol_get_sysvar_size queries the serialized byte size of a sysvar.
   It is equal to the length of the serialization of the sysvar value.
   The sysvar value is taken in the sysvar cache.

   sysvar_id points to the 32 byte address of the sysvar.

   If a non-empty sysvar was found in the sysvar cache, returns the
   byte size of that sysvar.

   If the requested sysvar was not found, is corrupt, is
   unsupported for the current feature set, or is zero size, returns 0.

   This syscall aborts the transaction if (and only if):
   - The compute budget was exhausted */

uint64_t
sol_get_sysvar_size( /* r1 */ void const * sysvar_id );

/* sol_get_sysvar copies sysvar data to VM memory.

   sysvar_id points to the 32 byte address of the sysvar.

   A sysvar is valid if sol_get_sysvar_size(sysvar_id) returns non-zero.
   If the sysvar is not valid, returns zero without writing to out.

   The byte range [offset,offset+size) selects the range of data to be
   copied. The sysvar data is the serialized value of the corresponding
   sysvar cache entry.
   The size of this sysvar data is returned by sol_get_sysvar_size.

   Copies the sysvar data to [out,out+size) and returns 1 if the sysvar is valid.

   This syscall aborts the transaction if:
   - Range [out,out+size) is not writable
   - The compute budget was exhausted */

int
sol_get_sysvar(
  /* r1 */ void const * sysvar_id,
  /* r2 */ uint8_t *    out,
  /* r3 */ uint64_t     offset,
  /* r4 */ uint64_t     size
);
```

## Impact

TODO

## Security Considerations

TODO
