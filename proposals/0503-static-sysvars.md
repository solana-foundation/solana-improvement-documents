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

Leverage existing static linking infrastructure of JIT compilation to enable 
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
// murmur3(SOL_RENT_SYSVAR) = 0x494df715
const SOL_RENT_SYSVAR: *const u8 = 0x494df715u64 as *const u8;
// murmur3(SOL_CLOCK_SYSVAR) = 0xff395088
const SOL_CLOCK_SYSVAR: *const u8 = 0xff395088u64 as *const u8;
```

We cast this value to a pointer, which is then consumed in a program:

```rust
let lamports_per_byte: u64 = unsafe { *(SOL_RENT_SYSVAR as *const u64) };
```

This produces the following bytecode:

```asm
lddw r3, 0x494df715 // SOL_RENT_SYSVAR
ldxdw r3, [r3+0]
```

In this case, the murmur hash value of `0x494df715` is then resolved JIT to a 
memory address containing the current `Rent` sysvar value. The same applies to 
other sysvar accounts, such as the murmur hash of `0xff395088` for `Clock`.

Ergo, we can safely and performantly expose any available global variable to 
the VM without the overhead of additonal account loads or syscall invocation.

## Alternatives Considered

- Leverage JIT intrinsics to provide similar syscall functionality.
- Don't improve the existing design of sysvars/syscalls.

## Impact

1. Improved program composability
2. Fewer stack allocations
3. Reduced complexity of calculating rent exemption
4. Reduced cost of resolving sysvar values to 2 CUs

## Security Considerations

1. We must ensure no intra-slot mutability of any exposed globals.
2. We must ensure static sysvar pointers remain synchronized with SysvarCache 
   at the slot boundary.
3. Despite being 32-bit hashes, it is important that we cast to u64 first as
   50% of 32-bit murmur hashes sign extend to negative 64-bit addresses. If 
   our toolchain reliably generated `mov32` without requiring inline assembly, 
   this would be more ideal, as it would save 8 bytes of binary size.

## Backwards Compatibility

This feature is a breaking change that will require feature-gated activation. 
Realizing the performance benefits of static sysvars will require adoption by 
all relevant programs and SDKs. All existing Syscall/Sysvar APIs will continue 
to function as normal.