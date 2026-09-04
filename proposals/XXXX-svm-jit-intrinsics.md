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

This SIMD introduces `sol_multi3` as the first JIT intrinsic in SVM. It allows
the static BPF call for wrapping 128-bit multiplication to be recognized by
the virtual machine and lowered directly to optimized host-architecture
operations.

JIT intrinsics overload the static CALL_IMM calling convention defined in
SIMD-0178 to enable efficient access to host-architecture instructions without
having to introduce new instructions that would break compatibility with the
BPF ISA.

## Motivation

BPF static call → SVM JIT intrinsic → host-architecture implementation

Some operations are expensive to express using BPF instructions despite
having efficient implementations on the host architecture. For example, BPF
does not provide native instructions for many wide arithmetic or vector
operations. These operations must instead be implemented as sequences of BPF
instructions, even when the host architecture can perform the equivalent
computation much more efficiently.

JIT intrinsics allow the SVM to recognize selected static calls during JIT
compilation and lower them directly to native host-architecture instructions.
This provides access to host-architecture capabilities without introducing
Solana-specific SBPF instructions or breaking compatibility with the eBPF ISA.

SBPF v2 exposed wide multiplication through the Solana-specific PQR
instructions `LMUL64` and `UHMUL64`, but they are deprecated because they are
not part of the upstream eBPF ISA. `sol_multi3` recovers this functionality
through a standard static-call encoding without reintroducing Solana-specific
opcodes.

The mechanism can later be applied to operations that benefit from CPU SIMD
and other host-architecture-specific instructions.

### Compiler integration through libcalls

Compiler libcalls provide a way to leverage JIT intrinsics without requiring
application developers to change their source code.

Compilers already lower operations unsupported by the target ISA into libcalls.
LLVM lowers 128-bit integer multiplication to the `__multi3` libcall. Compiler
tooling can implement `__multi3` using the `sol_multi3` JIT intrinsic instead
of a BPF software implementation. This allows existing programs to benefit
from the intrinsic simply by recompiling with a compiler-builtins library
optimized for SVM, without source-level rewrites.

### Prototype results

We prototyped `sol_multi3` in
[sbpf](https://github.com/anza-xyz/sbpf/commit/4ce37fad6c4773730c4a2445674c9e9b55621b09).
In a benchmark performing 10,000 `u128` multiplications, the intrinsic method
consumed approximately four times fewer compute units (110k versus 450k CU) and
ran approximately twice as fast in wall-clock time. These results are described
in our [research article].

[research article]: https://blueshift.gg/research/accelerating-u128-math-with-libcalls-and-jit-intrinsics

## Dependencies

This proposal depends on:

- **[SIMD-0178]: SBPF Static Syscalls**

  JIT intrinsics reuse the static syscall encoding and hash-based call
  resolution introduced by SIMD-0178.

[SIMD-0178]: https://github.com/solana-foundation/solana-improvement-documents/pull/178

## New Terminology

**JIT intrinsic:** A protocol-defined operation represented as a static BPF
call that enables zero-abstraction access to native hardware capabilities
directly within the runtime without breaking the BPF ISA

- Are recognized as regular instructions during JIT compilation
- Are lowered directly to host-architecture instruction sequences
- Execute inline at runtime without exiting the JIT

**Host architecture:** The physical instruction set on which an SVM
implementation executes JIT-compiled code. The initial host architecture
targeted by this proposal is x86-64.

## Detailed Design

JIT intrinsics use the static call encoding defined by SIMD-0178:

- opcode 0x85
- source register field set to zero
- immediate field containing murmur32(intrinsic_name)

The immediate value identifies the intrinsic using the same mechanism used to
identify static syscalls. This allows JIT intrinsics to use the existing BPF
call encoding without introducing new instructions or changing the bytecode
format.

When loading a program, a static call whose immediate matches a registered JIT
intrinsic is treated as an intrinsic call. Static calls that do not match a
registered JIT intrinsic continue to use the existing syscall resolution and
dispatch behavior.

### Initial Intrinsic: `sol_multi3`

This proposal registers one initial intrinsic, `sol_multi3`. Its identifier is:

```text
murmur32("sol_multi3") = 0xDB0F6D13 = -619746029 as i32
```

`sol_multi3` performs wrapping multiplication of two unsigned 128-bit values.
It matches the `__multi3` SBPF ABI:

| Phase | Register | Meaning |
|-------|----------|---------|
| Entry | `r1` | Low 64 bits of the first operand |
| Entry | `r2` | High 64 bits of the first operand |
| Entry | `r3` | Low 64 bits of the second operand |
| Entry | `r4` | High 64 bits of the second operand |
| Return | `r0` | Low 64 bits of the result |
| Return | `r2` | High 64 bits of the result |

The operands and result are defined as:

```text
a = r1 + (r2 << 64)
b = r3 + (r4 << 64)
result = (a * b) mod 2^128
```

On return, `r0` must contain the low 64 bits of `result`, and `r2` must contain
the high 64 bits. The implementation must capture the high 64 bits of the first
operand from `r2` before overwriting `r2` with the high 64 bits of the result.
The intrinsic does not read or write VM memory.

The intrinsic consumes the normal instruction-meter charge for its `CALL_IMM`
instruction and must not incur the additional dispatch cost of a syscall.

### Host-Architecture Execution

When the JIT encounters a registered JIT intrinsic, it emits equivalent native
host-architecture instructions directly instead of generating the normal
syscall dispatch sequence.

The initial JIT implementation targets x86-64. It lowers `sol_multi3` to native
64-bit unsigned `MUL` and `ADD` instructions that compute the low 128 bits of
the product. The precise native instruction sequence is implementation-defined,
but it must produce the register results and compute-unit consumption specified
above.

An SVM implementation that does not provide an x86-64 JIT lowering must execute
`sol_multi3` through an interpreter or another host-architecture backend with
identical observable behavior.

### Verification

`CALL_IMM` instructions with a source register field of `0` are static calls and
their immediate field is an identifier, not a PC-relative call offset.

The verifier must therefore only perform relative call-target validation when
the source register field indicates an internal function call. Static syscall
and JIT intrinsic identifiers must not be interpreted as relative branch
offsets.

### Edge Cases

- Multiplication by zero must produce zero.
- Multiplication overflow must wrap modulo `2^128`.
- The maximum operand value, `2^128 - 1`, must be handled without host-language
  overflow or undefined behavior.
- The result must be identical regardless of host architecture or whether the
  program is interpreted or JIT-compiled.

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

- New BPF instructions could represent accelerated operations directly, but
  would extend the BPF ISA and require changes across the compiler and tooling
  ecosystem
- Regular syscalls could provide the same operations, but syscall dispatch
  introduces unnecessary overhead for small computational operations that can
  be emitted directly by the JIT.

## Impact

The initial `sol_multi3` intrinsic allows existing applications to benefit from
faster 128-bit multiplication without source-code changes.

This also provides a stable compiler-to-SVM optimization interface and allows
Solana to take advantage of host-architecture capabilities while keeping BPF
programs portable.

## Security Considerations

JIT and interpreter implementations must agree on the `r0` and `r2` results and
compute-unit consumption for every input. A host architecture's integer
overflow behavior or native ABI must not leak into the BPF-visible semantics.
Implementations must preserve the original high limb in `r2` until it has been
used in the multiplication.

Intrinsic names and their Murmur3 identifiers are protocol constants. New
intrinsics must be checked for collisions with existing syscalls and intrinsics
before activation.

## Drawbacks *(Optional)*

n/a

## Backwards Compatibility *(Optional)*

Existing programs are unaffected. Existing source code can adopt `sol_multi3`
by recompiling with a Solana compiler-builtins implementation that lowers
`__multi3` to the intrinsic.

## Conformance

Conformance tests must verify identical `r0` and `r2` results and compute-unit
consumption between the interpreter and every JIT backend. They must also
verify that `sol_multi3` does not modify VM memory.

The test vectors must include zero, one, `u64::MAX`, `2^127`, and `u128::MAX`
operands and products that do and do not overflow 128 bits.
