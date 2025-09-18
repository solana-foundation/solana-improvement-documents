---
simd: '0XXX'
title: Minimal ELF Binary Encoding for sBPF Programs
authors:
  - Dean Little - Blueshift
  - Claire Fan - Blueshift
category: Standard
type: Core
status: Idea
created: 2025-09-18
feature: 
---

## Summary

Define a **minimal ELF binary encoding** for Solana sBPF programs that statically links binaries into exactly three contiguous sections; ELF header, `.rodata`, and `.text`, and removes the need for program/section header tables. This simplifies eliminates relocation logic in the JIT path, reduces binary size compared to sBPF v0 style artifacts and reduces linker complexity, helping us to achieve future upstream BPF compatibility.

## Motivation

Solana validators execute programs inside the rBPF/eBPF VM. Today’s artifacts often include superfluous ELF structures (program headers, section headers) that are primarily useful for debugging, but not needed by the runtime. Removing them enables:

* **Smaller binaries** (measurably reducing on-chain and network overhead).
* **Deterministic layout** that simplifies the VM’s memory scaffolding.
* **Simpler, faster JIT** (no dynamic reloc ations, fewer failure modes).
* **Safer memory access** (rigidly enforce `R^X` on `.text`, `R` on `.rodata`)

This proposal standardizes a minimal format so toolchains and the validator can interoperate consistently.

## New Terminology

N/A

## Detailed Design

There are 3 parts to the design; namely, a binary layout, runtime behavior, and loader behavior. 

### Binary Layout

A minimally encoded program binary consists of exactly three sections:

1. **ELF Header** (64 bytes, as usual for ELF64)
2. **Read-only data** (`.rodata`)
3. **Code** (`.text`)

*No other sections or tables are present.* In particular:

* **No Program Header Table** (`e_phnum = 0`)
* **No Section Header Table** (`e_shnum = 0`, `e_shstrndx = 0`)

The VM assumes contiguous placement in the above order.

### ELF Header Requirements

| Field        | Value / Rule                                                                                                           |
| ------------ | ---------------------------------------------------------------------------------------------------------------------- |
| `e_type`     | `ET_EXEC (0x0002)`                                                                                                     |
| `e_machine`  | `BPF (0xF7)`                                                                                                           |
| `e_entry`    | Virtual address (offset from file start) **pointing to `entrypoint` in `.text`**, which must come **after** `.rodata`. |
| `e_phnum`    | `0`                                                                                                                    |
| `e_shnum`    | `0`                                                                                                                    |
| `e_shstrndx` | `0`                                                                                                                    |

All remaining ELF header fields are ignored by the loader unless otherwise specified by existing Solana runtime rules, leaving room for us to reintroduce program headers in the future should the need .

### Loader Behavior

When deploying a minimal binary, the loader **must**:

1. **Validate** the ELF header fields match the rules above.
2. **Jump to `e_entry`** and perform static bytecode verification as usual for sBPF.
3. The loader must **hard-fail** on any binary that includes program/section header tables or violates the fixed order.

### Runtime Behavior

When excecuting a minimally encoded binary, the runtime must:
1. **Disable relocation/JIT relocation** logic (there are no relocations in this format).
2. **Resolve static call targets**
3. **Adhere to expected memory layout** (`.rodata` is mapped read-only, `.text` is mapped executable and read-only)

### Minimal Example (built with upstream BPF)

```rust
#![no_std]
#![no_main]

#[cfg(target_arch = "bpf")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}

pub fn sol_log(msg: &str) -> u64 {
    let sol_log_: unsafe extern "C" fn(message: *const u8, length: u64) -> u64 =
        unsafe { core::mem::transmute(4242usize) };
    unsafe { sol_log_(msg.as_ptr(), msg.len() as u64) }
}

#[no_mangle]
pub fn entrypoint(_input: *mut u8) -> u64 {
    sol_log("gm")
}
```

### Minimally Encoded Binary

```
# ELF header
7f454c4602010100 0000000000000000
0200f70001000000 4800000000000000
0000000000000000 0000000000000000
0000000040003800 0000400000000000

# .rodata
676d00000000000    # "gm\0"

# .text
1801000040000000 0000000000000000  # lddw r1, 0x0040
b702000002000000                     # mov64 r2, 0x02
85000000bd597520                     # call 0x207559bd (sol_log_)
9500000000000000                     # exit
```

> Note: In an sBPF v0 program, the equivalent artifact would be more than double this size due to additional ELF scaffolding.

## Backwards Compatibility

* Existing sBPF v0 binaries will remain valid moving forwards; this SIMD introduces an **additional** accepted format.
* In the runtime, minimally encoded binaries is a breaking change and must be enabled by **feature gate** activation during rollout.
* Tooling (SDKs/build scripts) should add a “minimal” output mode that emits this layout while continuing to support current formats.

## Alternatives Considered

* **Keep current ELF with headers present but ignored.** Reduces diff risk but fails to realize size and simplicity gains.
* **Custom non-ELF container.** Could be even smaller, but deviates from ELF ecosystem and further complicates tooling compatability.

## Impact

Minimally encoded reduces program binary size and complexity, resulting in cheaper program deployments, simpler tooling and more efficient execution. This simplified format also makes it easier to maintain custom linker tooling required to enable future upstream eBPF compatibility (in fact, in our above example, they are already compatible.)

## Security Considerations

We should be cautious to robustly defining and implement the bytecode checks required at deploy time.
