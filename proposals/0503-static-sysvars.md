---
simd: '0503'
title: Static Sysvars
authors:
  - Dean Little (Blueshift)
  - Joe Caulfield (Anza)
category: Standard
type: Core
status: Idea
created: 2026-03-25
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Leverage existing memory translation infrastructure of JIT compilation to enable
static resolution of sysvars.

## Motivation

In order to access Sysvar values, developers currently have three options:

1. Invoke a specific getter syscall
2. Invoke the get_sysvar syscall, or
3. Include a Sysvar account in their program.

The downsides of all of these approaches are threefold:

1. Sysvar values are globals that are always available to the validator, but 
   aren't exposed as globals during execution. This is a clunky anti-pattern 
   resulting in degraded developer experience.
2. Invoking a syscall to access a global requires both a stack allocation and 
   halting execution – an immense amount of overhead just to read a value that 
   is already readily available.
3. Having to pass in an account to access a global results in a 10kb penalty 
   to data serialization and has negative implications for composability.

If these globals were simply exposed to the VM, as is common in kernel BPF, 
and resolved JIT, we could dramatically improve developer experience, whilst 
also reducing the runtime overhead of accessing them to just 2 CUs. This is 
generalizable to all Sysvars, however the majority of gains are realized by 
simply implementing `Rent` and `Clock`.

## New Terminology

Static Sysvars - A static global variable pointing to current sysvar values, 
accessible within the VM by a pointer to its murmur hash code resolved during 
JIT compilation.

## Detailed Design

As with static syscalls, we define the hash code for a sysvar as the murmur32 
hash of its respective name:

```rust
const MM_STATIC_SYSVARS: u64 = 0x05 << 32;
// murmur3(SOL_RENT_SYSVAR) = 0x494df715
const SOL_RENT_SYSVAR: *const u8 =
    (0x494df715u64 & 0xffffffff | MM_STATIC_SYSVARS)
    as *const u8;
// murmur3(SOL_CLOCK_SYSVAR) = 0xff395088
const SOL_CLOCK_SYSVAR: *const u8 =
    (0xff395088u64 & 0xffffffff | MM_STATIC_SYSVARS)
    as *const u8;
```

We cast this value to a pointer, which is then consumed in a program:

```rust
let lamports_per_byte: u64 = unsafe { *(SOL_RENT_SYSVAR as *const u64) };
```

This produces the following bytecode:

```asm
lddw r3, 0x5494df715 // SOL_RENT_SYSVAR
ldxdw r3, [r3+0]
```

### Memory Region

This proposal introduces a new read-only VM memory region for static sysvars:

| Region | Address Range | Permissions |
|--------|--------------|-------------|
| Static Sysvars | `0x500000000..0x600000000` | Read-only |

This is the fifth memory region in the SBPF virtual address space, following
the existing regions for readonly data (`0x1`), stack (`0x2`), heap (`0x3`),
and serialized input (`0x4`).

The virtual address of a static sysvar is computed as:

```
address = MM_STATIC_SYSVARS | (murmur32(name) & 0xFFFFFFFF)
```

Where `MM_STATIC_SYSVARS` is `0x05 << 32` (`0x500000000`).

### JIT Resolution

The JIT maintains an array of host pointers, one per supported sysvar, backed
by the `SysvarCache`. When the JIT encounters a memory access targeting the
`0x500000000` region, it resolves the murmur hash portion of the address to
the corresponding host pointer and emits a direct native load.

At the slot boundary, the `SysvarCache` is instantiated or copied with current
sysvar values. The pointer array is updated accordingly. This incurs a small
per-slot setup cost but introduces zero runtime overhead per-access — each
sysvar read is a single native memory load.

An access to an address in the static sysvar region whose murmur hash does
not correspond to a known sysvar must produce an access violation, identical
to dereferencing an unmapped address in any other region.

### Supported Sysvars

The following sysvars are supported through the static sysvar interface. All
sysvars present in the `SysvarCache` are eligible for exposure.

| Sysvar | Canonical Name | murmur32 | Size (bytes) |
|--------|---------------|----------|--------------|
| Rent | `SOL_RENT_SYSVAR` | `0x494df715` | 17 |
| Clock | `SOL_CLOCK_SYSVAR` | `0xff395088` | 40 |
| EpochSchedule | `SOL_EPOCH_SCHEDULE_SYSVAR` | TBD | 33 |
| LastRestartSlot | `SOL_LAST_RESTART_SLOT_SYSVAR` | TBD | 8 |
| EpochRewards | `SOL_EPOCH_REWARDS_SYSVAR` | TBD | 49 |

Unless specified otherwise, any new sysvars added to the `SysvarCache` in the
future also become accessible through this interface by registering their
murmur32 hash.

## Alternatives Considered

### Unified Syscall (SIMD-0127)

SIMD-0127 introduced `sol_get_sysvar`, a single syscall that retrieves
arbitrary byte ranges from any sysvar. While this reduced syscall bloat, it
still requires a syscall invocation per access — incurring stack allocation,
VM exit, and CU costs proportional to the data length. Static sysvars
eliminate this overhead entirely, reducing sysvar reads to native memory
loads.

### VM Memory Region (as rejected by SIMD-0127)

SIMD-0127 considered and rejected a memory-mapped region approach, citing
address translation complexity, dependency on direct mapping, and full data
copies per VM instantiation. Static sysvars avoid all three: the JIT
resolves addresses at compile time via a pointer array into the existing
`SysvarCache`, so no runtime address translation or data copying occurs.

## Impact

1. Improved program composability
2. Fewer stack allocations
3. Reduced complexity of calculating rent exemption
4. Reduced cost of resolving sysvar values to 2 CUs

## Security Considerations

1. **Intra-slot immutability.** Static sysvar pointers are updated from the
   `SysvarCache` at the slot boundary, before any program execution begins,
   and must remain immutable for the duration of the slot.

2. **Sign extension.** SDK constants must mask the murmur32 hash to 32 bits
   (`& 0xFFFFFFFF`) before OR'ing with `MM_STATIC_SYSVARS` to prevent sign
   extension from producing addresses outside the `0x500000000` region.

3. **Invalid address access.** Programs can construct arbitrary addresses in
   the `0x500000000` region. Accesses to addresses that do not correspond to
   a registered sysvar hash must produce an access violation.

## Backwards Compatibility

This feature is a breaking change that will require feature-gated activation. 
Realizing the performance benefits of static sysvars will require adoption by 
all relevant programs and SDKs. All existing Syscall/Sysvar APIs will continue 
to function as normal.