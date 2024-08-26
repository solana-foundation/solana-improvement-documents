---
simd: '0127'
title: Get-Sysvar Syscall
authors:
  - Joe Caulfield (Anza)
category: Standard
type: Core
status: Accepted
created: 2024-03-15
feature: (fill in with feature tracking issues once accepted)
development:
  - Anza - Not Started
  - Firedancer - Not started
---

## Summary

This proposal outlines a new syscall interface for sysvars; one which can
retrieve dynamic ranges of sysvar data without fully copying the entire data
structure.

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
therefore requiring access to this sysvar data via syscalls and _not_ via
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
retrievals will _also_ require new syscalls.

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

Even with direct mapping, this change makes address translation more complex
and therefore slows down _all_ memory accesses.

## New Terminology

N/A.

## Detailed Design

A single `sol_get_sysvar` syscall can be defined, which can be used by all
forward-facing sysvar interfaces to retrieve data from any sysvar.

```c
/**
 * Retrieves a slice of data from a sysvar and copies it to VM memory.
 *
 * @param sysvar_id The identifier of the sysvar to retrieve data from.
 * @param var_addr  VM memory address to copy the retrieved data to.
 * @param offset    The offset to start copying data from.
 * @param length    The length of data to copy, in bytes.
 * @return          A 64-bit unsigned integer error code:
 *                    - 0 if the operation is successful.
 *                    - Non-zero error code.
 *
 * If the operation is not successful, data will not be written to the
 * provided VM memory address.
 */
uint64_t sol_get_sysvar(
  /* r1 */ void const * sysvar_id,
  /* r2 */ uint8_t *    var_addr,
  /* r3 */ uint64_t     offset,
  /* r4 */ uint64_t     length,
);

```

### Control Flow

The syscall aborts the virtual machine if any of these conditions are true:

- Not all bytes in VM memory range `[sysvar_id, sysvar_id + 32)` are readable.
- Not all bytes in VM memory range `[var_addr, var_addr + length)` are writable.
- `offset + length` is not in `[0, 2^64)`.
- `var_addr + length` is not in `[0, 2^64)`.
- Compute budget is exceeded.

All VM violations (above) are checked first, before to returning one of the
following error codes. The following checks are completed in the order they
appear below.

The syscall returns the following graceful codes:

- `2` if the sysvar data is not present in the Sysvar Cache.
- `1` if `offset + length` is greater than the length of the sysvar data.
- `0` if the process completed successfully and the requested sysvar data was
  written to the virtual memory at `var_addr`.

### Compute Unit Usage

The syscall will always consume the same amount of CUs regardless of
control flow.

CU usage is proportional to the `length` parameter.

```
sysvar_base + (32 / cpi_per_u) + max(mem_op_base, (length / cpi_per_u))
```

- `sysvar_base`: Base cost of accessing a sysvar.
- `cpi_per_u`: Number of account data bytes per CU charged during CPI.

### Supported Sysvars

The proposed unified syscall interface will support all non-deprecated sysvars
in the Sysvar Cache.

Unless specified otherwise, any new sysvars added to the Sysvar Cache in the
future also become accessible through this syscall.

The supported list of sysvars for the proposed syscall will be as follows:

- `SysvarC1ock11111111111111111111111111111111`
- `SysvarEpochRewards1111111111111111111111111`
- `SysvarEpochSchedu1e111111111111111111111111`
- `SysvarLastRestartS1ot1111111111111111111111`
- `SysvarRent111111111111111111111111111111111`
- `SysvarS1otHashes111111111111111111111111111`
- `SysvarStakeHistory1111111111111111111111111`

Sysvar APIs at the SDK level are responsible for defining exactly how the data
can be accessed using the new syscall. For example, multiple SDKs may choose to
allow users to access one element from a list-based sysvar at a time, while only
a few may offer lookups or binary searches on the data.

## Impact

As mentioned as partial motivation for this proposal, the new syscall would
make changing and adding new sysvars in the future much easier, which would be
positively impactful to contributors.

It also greatly reduces the complexity and required upkeep for the syscall
interface.

Furthermore, it unblocks the efforts to give BPF programs more access to
sysvar data, so on-chain programs gain the ability to read more sysvar data
without increasing transaction size.

One minor change is the behavior of verification of syscall availability for a
given program. Currently, the availability of specific sysvar data can be
verified by simply determining whether its corresponding syscall is available to
the program during the verification step.

With this change, even though the new unified syscall can be verified available
during program verification, the actual availability of specific sysvar data is
determined at runtime.

## Security Considerations

This new syscall interface does increase the chances of improper management of
bytes and offsets, which could throw critical errors.

However, this can be mitigated in testing, and the upside for reducing the
number of syscalls - thereby reducing the surface area for other critical bugs
or exploits - is a worthy tradeoff.
