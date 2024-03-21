---
simd: '0127'
title: Get-Sysvar Syscall
authors:
  - Richard Patel (Jump)
  - Joe Caulfield (Anza)
category: Standard
type: Core
status: Draft
created: 2024-03-15
---

## Summary

This proposal outlines a new syscall interface, specifically pertaining to
sysvars, to retrieve dynamic ranges of sysvar data without fully copying the
entire data structure. As a result, the existing sysvar interface will remain
backwards compatible and could also support new methods for querying sysvar
data.

With a unified syscall interface to handle retrieval of sysvar data, many
existing syscalls will become obsolete, and the act of changing sysvar data
layouts or adding new sysvars will involve much less overhead.

## Motivation

Sysvars are special accounts that are implicitly modified by the runtime.
Certain sysvars are essential for operation of the network, and many others are
used by on-chain programs for a wide range of use cases.

The sysvar API should be completely consistent, available to all BPF programs,
and capable of more easily handling changes in the future. The syscall API for
sysvars should be designed to serve a wide range of requests on sysvar data
without bloating the overall syscall interface.

### Consistent API

Although it's true an on-chain program can access sysvar data by reading its
account directly, many sysvars - such as `Clock` - can be accessed directly via
syscalls, thereby not requiring an increase in transaction size to include the
sysvar account key.

Furthermore, reducing the amount of account data mapped into virtual machine
memory may also yield cost savings.

A consistent API should be present for accessing this data from on-chain
programs.

### Availability to BPF Programs

The proposed design herein would allow for access to previously unavailable
sysvar data - such as `SlotHashes` and `StakeHistory`. Although retrieving the
entire data structure may still be too heavy to copy into a BPF context,
fragments of the data can be made accessible via calls like
`SlotHashes::get_slot(slot)` and `StakeHistory::get_entry(epoch)`.

Additionally, as per [SIMD-0088](./0088-enable-core-bpf-programs.md), an effort
is underway to reduce code complexity of the Solana runtime by porting native
programs to Core BPF programs. These programs must be 100% backwards compatible,
therefore requiring access to this sysvar data via syscalls and *not* via
providing the account directly. Requiring the account would break the programs'
ABIs.

### Reducing Syscall Bloat

Currently, sysvar data layouts are very difficult to change. When called, the
syscall handler will copy the entire sysvar to the given memory location.

Notably, BPF programs do not inform the syscall handler of the expected size
of the sysvar. BPF programs thus effectively assume that the requested sysvars
will forever have the same size.

This prevents future upgrades that shrink or extend sysvars. In the same
spirit, if a new sysvar is created, a new syscall must be created.

Furthermore, if certain queries are requred by BPF programs - such as
`SlotHashes::get_slot()` and `StakeHistory::get_entry()` - those single-element
retrievals will *also* require new syscalls.

It's worth noting that at the time of this writing, the work required for
[SIMD-0088](./0088-enable-core-bpf-programs.md) will require at least five new
syscalls if the existing design is not ratified.

This tightly-coupled relationship between sysvars and syscalls will perpetually
swell the syscall interface as changes are made, making maintainence and
reimplementation difficult.

## Alternatives Considered

Other mechanisms considered that provide sysvar data to BPF programs are as
follows:

### Sysvar-Specific Syscalls

This approach would involve changing nothing. We'd continue on the same course,
directly coupling new sysvar fetch functions to new syscalls.

### New VM Memory Regions

A new memory region in the VM could be introduced to store sysvar data, which
could then be made available to BPF programs through an interface around raw
pointers, which loads the data similarly to the interface proposed herein.

This approach would depend on direct mapping of host memory to VM memory,
which has been in development for some time and requires further research
and development to introduce safely to the VM ABI. This extensive R&D period
would mean Core BPF program initiatives would remain blocked.

Without direct mapping, sysvar data would be fully copied to this memory
region each time a VM is created (currently one per transaction processed),
causing a degradation in runtime performance and likely reducing compute
by far less compared to the compute savings posed by this proposal.

## Detailed Design

Defining one single `get` API to retrieve data from the sysvar cache can be used
by all forward-facing sysvar interfaces. This `get` API should require the
`sysvar_id` to retrieve data from, the `offset` at which to start copying data,
and the `length` of the bytes to copy and return to the caller.

The `sysvar_id` can be the address of the sysvar account or simply a single-byte
enum aligned with the available sysvars in the sysvar cache. This is an
implementation detail.

A psuedo-code representation of the `get` API is as follows, where the returned
value is the slice of bytes from the requested sysvar.

```rust
fn get(sysvar_id: SysvarID, offset: usize, length: usize) -> &[u8];
```

In reality, the syscall will write this slice of data to a memory address,
exactly like the existing syscalls do for sysvar data.

Generally, the following cases would be handled with errors:

- The sysvar data is unavailable.
- The sysvar data is corrupt.
- The provided offset or length would be out of bounds on the sysvar data. 

Sysvars themselves should have internal logic for understanding their own size
and the size of their entries if they are a list-based data structure. With this
information, each sysvar can customize how it wishes to drive the `get` syscall.

A list-based sysvar - say `SlotHashes` - could leverage this API like so:

```rust
fn get(index: usize) -> SlotHash {
  let length = size_of::<SlotHash>(); 
  let offset = index * length;
  syscall::get(offset, length)
}
```

Any sysvar that wishes to offer a more robust API would then be responsible for
defining such an interface, rather than pushing that burden onto the syscall
interface. For example, `SlotHashes` could offer something like the following:

```rust
fn get_hash(index: usize) -> Hash {
  let offset = index * size_of::<SlotHash>() + size_of::<Slot>();
  let length = size_of::<Hash>();
  syscall::get(offset, length)
}
```

Any additionally complex methods - such as binary searches - should be left to
the on-chain program to implement locally. These should not be made available to
BPF programs via the sysvar interface.

The syscall interface for sysvars should only support `get` as defined above.

## Impact

As mentioned as partial motiviation for this proposal, the new syscall would
make changing and adding new sysvars in the future much easier, which would be
positively impactful to contributors.

It also greatly reduces the complexity and required upkeep for the syscall
interface.

Furthermore, it unblocks the efforts to give BPF programs more access to
sysvar data, so on-chain programs gain the ability to read more sysvar data
without increasing transaction size.

## Security Considerations

This new syscall interface does increase the chances of improper management of
bytes and offsets, which could throw critical errors.

However, this can be mitigated in testing, and the upside for reducing the numbe
of syscalls - thereby reducing the surface area for other critical bugs or
explots - is a decent tradeoff.

