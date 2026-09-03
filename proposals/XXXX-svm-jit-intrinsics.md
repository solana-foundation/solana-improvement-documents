---
simd: 'XXXX'
title: SVM JIT Intrinsics
authors:
  - Dean Little
  - Claire Fan
category: Standard
type: Core
status: Draft
created: 2026-08-25
feature: TBD
extends: '0178'
---

## Summary

This SIMD introduces JIT intrinsics in SVM, allowing selected static SBPF calls
to be recognized by the virtual machine and lowered directly to optimized host
operations.

JIT intrinsics reuse the static CALL_IMM encoding introduced by SIMD-0178 and
do not introduce new SBPF instructions.

## Motivation

SBPF static call → SVM JIT intrinsic → native CPU implementation

Some operations are expensive to express using SBPF instructions despite
having efficient implementations on the host CPU. For example, SBPF does not
provide native instructions for many wide arithmetic or vector operations.
These operations must instead be implemented as sequences of SBPF instructions,
even when the host CPU can perform the equivalent computation much more
efficiently.

JIT intrinsics allow the SVM to recognize selected static calls during JIT
compilation and lower them directly to native host instructions. This provides
access to host CPU capabilities without introducing Solana-specific SBPF
instructions or breaking compatibility with the eBPF ISA.

The mechanism can be used for wide integer arithmetic and can later be applied
to operations that benefit from CPU SIMD and other host-specific instructions.

### Compiler integration through libcalls

Compiler libcalls provide a way to leverage JIT intrinsics without requiring
application developers to change their source code.

Compilers already lower operations unsupported by the target ISA into libcalls.
For example, LLVM can lower wide integer multiplication to the `__multi3`
libcall. Compiler toolings can implement such a libcall using a JIT intrinsic
instead of an SBPF software implementation. This allows existing programs to
benefit from newly introduced JIT intrinsics simply by recompiling with a
compiler-builtins library optimized for SVM, without source-level rewrites.

## Dependencies

This proposal depends on:

- **[SIMD-0178]: SBPF Static Syscalls**

  JIT intrinsics reuse the static syscall encoding and hash-based call
  resolution introduced by SIMD-0178.

[SIMD-0178]: https://github.com/solana-foundation/solana-improvement-documents/pull/178

## New Terminology

**JIT intrinsic:** A protocol-defined operation represented as a static SBPF
call that enables zero-abstraction access to native hardware capabilities
directly within the runtime without breaking the BPF ISA

- Are recognized as regular instructions during JIT compilation
- Are lowered directly to optimal x86 instruction sequences
- Execute inline at runtime without exiting the JIT

## Detailed Design

JIT intrinsics use the static call encoding defined by SIMD-0178:

- opcode 0x85
- source register field set to zero
- immediate field containing murmur32(intrinsic_name)

The immediate value identifies the intrinsic using the same mechanism used to
identify static syscalls. This allows JIT intrinsics to use the existing sBPF
call encoding without introducing new instructions or changing the bytecode
format.

When loading a program, a static call whose immediate matches a registered JIT
intrinsic is treated as an intrinsic call. Static calls that do not match a
registered JIT intrinsic continue to use the existing syscall resolution and
dispatch behavior.

### JIT execution

When the JIT encounters a registered JIT intrinsic, it emits equivalent native
host instructions directly instead of generating the normal syscall dispatch
sequence.

For example, an intrinsic implementing a 128-bit multiplication may be lowered
directly into the host's native multiply and add instructions. Arguments and
return values follow the ABI defined for that intrinsic.

### Verification

`CALL_IMM` instructions with a source register field of `0` are static calls and
their immediate field is an identifier, not a PC-relative call offset.

The verifier must therefore only perform relative call-target validation when
the source register field indicates an internal function call. Static syscall
and JIT intrinsic identifiers must not be interpreted as relative branch
offsets.

We've prototyped in
[sbpf](https://github.com/anza-xyz/sbpf/commit/4ce37fad6c4773730c4a2445674c9e9b55621b09)
and for a program doing 10000 times u128 multiply, it shows the intrinsic method
costs ~4x fewer compute units (110k vs 450k CU) and runs ~2x faster in wall-clock
time. ([Research article] published by us)

[Research article]: https://blueshift.gg/research/accelerating-u128-math-with-libcalls-and-jit-intrinsics

### Edge Cases

n/a

### Validator Components Affected

| Validator Component             | Impact                                     |
|---------------------------------|--------------------------------------------|
| Transaction Execution (Runtime) | Intrinsic registration and compute metering |
| Virtual Machine                 | Interpreter and JIT implementation         |
| Block Packing                   | None                                       |
| Consensus                       | None                                       |
| Gossip                          | None                                       |
| Turbine                         | None                                       |
| Snapshots                       | None                                       |
| On-Chain Core BPF Programs      | None                                       |
| Other                           | None                                       |

## Alternatives Considered

- New SBPF instructions could represent accelerated operations directly, but
  would extend the SBPF ISA and require changes across the compiler and tooling
  ecosystem
- Regular syscalls could provide the same operations, but syscall dispatch
  introduces unnecessary overhead for small computational operations that can
  be emitted directly by the JIT.

## Impact

JIT intrinsics allow existing applications to benefit from new VM optimizations
without source-code changes.

This also provides a stable compiler-to-SVM optimization interface and allows
Solana to take advantage of host CPU capabilities while keeping SBPF portable.

## Security Considerations

TBD

## Drawbacks *(Optional)*

n/a

## Backwards Compatibility *(Optional)*

Existing programs are unaffected. Existing source code can adopt JIT intrinsics
by recompiling with an solana-compiler-builtins

## Conformance

Conformance tests must verify identical results between JIT and fallback
implementations.
