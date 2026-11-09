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

Leverage memory translation to enable static sysvar resolution.

## Motivation

In order to access Sysvar values, developers currently have three options:

1. Invoke a specific getter syscall such as `sol_get_rent_sysvar`
2. Invoke the `sol_get_sysvar` syscall, or
3. Include a `Sysvar` account in their program inputs.

The downsides of these approaches are threefold:

1. Sysvar values are globals that are always available to the validator, but 
   aren't exposed as globals during execution. This is a clunky anti-pattern 
   resulting in degraded developer experience.
2. Invoking a syscall to access Sysvar data requires allocating the full 
   amount of the sysvar data in memory.
3. Passing in accounts to access Sysvars is [far slower][benchmarks] than
   syscalls or static sysvars, yet they are currently priced the cheapest in
   CUs, actively incentivizing worse execution with less composable program
   APIs.

If these globals were simply exposed to the VM and resolved JIT, we could 
dramatically improve developer experience, whilst also reducing the runtime 
overhead of accessing them to just 2 CUs. While this is generalizable to all 
sysvars, the majority of gains are realized by implementing `Rent` and `Clock`.

## New Terminology

Static Sysvars - Sysvar values exposed directly in VM memory within a single
contiguous region at fixed, statically-known offsets, readable without syscall
invocation or account deserialization.

## Detailed Design

We define a single contiguous memory region that exposes every supported
sysvar at a fixed, statically-known offset. The base address of a sysvar is the
region base OR'd with its assigned offset:

```rust
const MM_STATIC_SYSVARS: u64 = 0x05 << 32;
// Rent occupies offset 0x00 within the region
const SOL_RENT_SYSVAR: *const u8 =
    (MM_STATIC_SYSVARS | 0x00) as *const u8;
// Clock occupies offset 0x18 within the region
const SOL_CLOCK_SYSVAR: *const u8 =
    (MM_STATIC_SYSVARS | 0x18) as *const u8;
```

We cast this value to a pointer, which is then consumed in a program:

```rust
let lamports_per_byte: u64 = unsafe { *(SOL_RENT_SYSVAR as *const u64) };
```

This produces the following bytecode:

```asm
lddw r3, 0x500000000 // SOL_RENT_SYSVAR
ldxdw r3, [r3+0]
```

### Memory Region

This proposal introduces a new read-only VM memory region for static sysvars:

| Region | Address Range | Permissions |
|--------|--------------|-------------|
| Static Sysvars | `0x500000000..0x600000000` | Read-only |

This is the fifth memory region in the SBPF virtual address space, following
the existing regions for readonly data (`0x100000000`), stack
(`0x200000000`), heap (`0x300000000`), and serialized input
(`0x400000000`).

The region has a fixed layout: supported sysvars are tightly packed at stable
offsets within the region (see [Supported Sysvars](#supported-sysvars)), each
8-byte aligned so every sysvar begins on a natural boundary. The bytes at a
sysvar's offset are byte-identical to that sysvar's canonical account data
(bincode), so existing deserialization logic reads them unchanged. Any padding
bytes inserted between sysvars to satisfy alignment are zero-filled.

The virtual address of a static sysvar is computed as:

```
address = MM_STATIC_SYSVARS | offset(sysvar)
```

Where `MM_STATIC_SYSVARS` is `0x05 << 32` (`0x500000000`) and `offset(sysvar)`
is the sysvar's assigned offset within the region. Offsets are part of the
protocol and, once assigned, are permanent.

### Meory Mapping

The region is backed by a single contiguous, read-only host buffer laid out
exactly as described above. A single `MemoryRegion` maps the entire
`0x500000000` range onto this buffer, so translating a static sysvar address is
a base-plus-offset resolution — the same lowered form as any other
direct-mapped memory access, with no syscall, no stack allocation, and no copy.
This feature is only active under direct mapping.

The runtime owns and maintains this buffer. At the slot boundary, when the
current sysvar values are computed, their canonical bytes are written into the
buffer in place. Only sysvars that actually changed need to be re-written — in
steady state this is `Clock` and `SlotHashes`; `Rent`,
`EpochSchedule`, and others change rarely. The buffer is immutable for the
duration of the slot, so it is shared by reference into every transaction's
memory mapping with no per-transaction setup and no per-access overhead.

The region does not fault on gaps. Reads of the alignment padding between
sysvars return zero. Only accesses that fall outside the region's mapped extent
produce an access violation, identical to dereferencing an unmapped address in
any other region.

### Supported Sysvars

The following sysvars are supported through the static sysvar interface. All
sysvars present in the `SysvarCache` are eligible for exposure.

| Sysvar | Canonical Name | Offset | Size (bytes) |
|--------|---------------|--------|--------------|
| Rent | `SOL_RENT_SYSVAR` | `0x00` | 17 |
| Clock | `SOL_CLOCK_SYSVAR` | `0x18` | 40 |
| EpochSchedule | `SOL_EPOCH_SCHEDULE_SYSVAR` | `0x40` | 33 |
| LastRestartSlot | `SOL_LAST_RESTART_SLOT_SYSVAR` | `0x68` | 8 |
| EpochRewards | `SOL_EPOCH_REWARDS_SYSVAR` | `0x70` | 49 |
| SlotHashes | `SOL_SLOT_HASHES_SYSVAR` | `0xA8` | 20488 |

New sysvars are added by appending them at the next available aligned offset;
existing offsets never move. Larger sysvars are appended in the same way.

## Alternatives Considered

### Unified Syscall (SIMD-0127)

SIMD-0127 introduced `sol_get_sysvar`, a single syscall that retrieves
arbitrary byte ranges from any sysvar. While this reduces syscall bloat, it
still requires a syscall invocation per access — incurring stack allocation,
VM exit, and CU costs proportional to the data length. Static sysvars
eliminate this overhead entirely, reducing sysvar reads to native memory
loads.

In evaluating that syscall, SIMD-0127 also considered and rejected the
memory-mapped region approach that this proposal adopts, citing address
translation complexity, full data copies per VM instantiation, and a dependency
on direct mapping. A single read-only buffer maintained once per slot and 
shared by reference into every transaction's memory mapping removes the need for
a per-VM copy, making translation an ordinary base-plus-offset resolution 
without the need for any bespoke logic.

### Reserved offsets for extensibility

Rather than packing sysvars tightly, each could be assigned an aligned slot
large enough to accommodate future growth, with reserved space left between
entries. This would allow a sysvar's serialized value to be extended in place
without shifting the offsets of the sysvars that follow it. We chose tight
packing for simplicity — it avoids reserving and tracking large gaps in the
region — accepting that extending an existing sysvar's layout is a rare,
protocol-level event that can be handled if and when it arises.

### Faulting on inter-sysvar padding

The alignment padding between sysvars could be treated as unmapped and made to
fault on access. We instead zero-fill it and allow reads to succeed. Both are
sound options.

## Impact

1. Improved program composability
2. Smaller stack allocations
3. Reduced complexity of calculating rent exemption
4. Reduced cost of resolving sysvar values to ~2 CUs per 8 bytes

## Security Considerations

1. **Intra-slot immutability.** Current proposal assumes memory region is
   populated once per slot at the slot boundary, before program execution 
   begins. This is perfectly safe and very performant. If we wish to enable 
   intra-slot updates, this would require aditional consideration.

2. **Invalid address access.** Programs can construct arbitrary addresses in
   the region. Reads of inter-sysvar padding, or past a sysvar into a neighbor,
   succeed rather than fault. Only accesses outside the region's mapped extent 
   produce an access violation.

## Backwards Compatibility

This feature is a breaking change that will require feature-gated activation. 
Realizing the performance benefits of static sysvars will require adoption by 
all relevant programs and SDKs. All existing Syscall/Sysvar APIs will continue 
to function as normal.

[benchmarks]: https://github.com/blueshift-gg/static-sysvars/blob/a6ab300c5aaeaebc510fc3091a2950275e568b89/BENCHMARKS.md