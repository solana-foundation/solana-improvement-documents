---
simd: '0645'
title: SVM JIT Intrinsics
authors:
  - Dean Little
  - Claire Fan
category: Standard
type: Core
status: Draft
created: 2026-08-25
feature: TBD
---

## Summary

This SIMD introduces `sol_multi3`, a static BPF call for wrapping 128-bit
multiplication. An SVM implementation can take advantage of the JIT process by
lowering the call to native wide-multiplication operations on x86-64.

`sol_multi3` uses the static `CALL_IMM` syscall identification mechanism.
This enables efficient access to host-architecture instructions without
introducing new instructions that would break compatibility with the BPF ISA.

## Motivation

Some operations are expensive to express using BPF instructions despite
having efficient implementations on the host architecture. For example, BPF
does not provide native instructions for many wide arithmetic or vector
operations. These operations must instead be implemented as sequences of BPF
instructions, even when the host architecture can perform the equivalent
computation much more efficiently.

An SVM can take advantage of the JIT process by recognizing selected static
calls and lowering them directly to native host-architecture instructions. For
`sol_multi3`, this allows the implementation to leverage wide-multiplication
operations on x86-64 without introducing Solana-specific SBPF instructions or
breaking compatibility with the eBPF ISA.

SBPF v2 exposed wide multiplication through the Solana-specific PQR
instructions `LMUL64` and `UHMUL64`, but they are deprecated because they are
not part of the upstream eBPF ISA. `sol_multi3` recovers this functionality
through a standard static-call encoding without reintroducing Solana-specific
opcodes.

The same JIT technique can later be applied to operations that benefit from CPU
SIMD and other host-architecture-specific instructions.

### Prototype results

We prototyped `sol_multi3` in
[sbpf](https://github.com/anza-xyz/sbpf/commit/4ce37fad6c4773730c4a2445674c9e9b55621b09).
In a benchmark performing 10,000 `u128` multiplications, the JIT-lowered method
consumed ~75% less compute (110k versus 450k CU) and ran approximately twice as
fast in wall-clock time. These results are described in our [research article].

[research article]: https://blueshift.gg/research/accelerating-u128-math-with-libcalls-and-jit-intrinsics

## Dependencies

n/a

## New Terminology

`sol_multi3` (see below detailed design)

## Detailed Design

- opcode 0x85
- src/dst register fields set to zero
- immediate field containing `murmur32("sol_multi3")`

The immediate value identifies `sol_multi3` using the same mechanism used to
identify static syscalls. This allows the operation to use the existing BPF call
encoding without introducing new instructions or changing the bytecode format.

When loading a program, a static call whose immediate matches the registered
`sol_multi3` identifier is treated as the operation defined below. All other
static calls continue to use the existing syscall resolution and dispatch
behavior.

### `sol_multi3`

This proposal registers one operation, `sol_multi3`. Its identifier is:

```text
murmur32("sol_multi3") = 0xDB0F6D13 = -619746029 as i32
```

`sol_multi3` performs wrapping multiplication of two unsigned 128-bit values.
It matches the `__multi3` BPF ABI:

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
All other registers must remain unchanged. The operation does not read or write
VM memory.

The operation consumes one compute unit: the normal instruction-meter charge
for its `CALL_IMM` instruction. It must not incur any additional charge.

### Host-Architecture Execution

An implementation can take advantage of the JIT process by recognizing
`sol_multi3` and emitting equivalent native host-architecture instructions
directly instead of generating the normal dispatch sequence.

On x86-64, this lowering leverages native 64-bit unsigned `MUL` operations,
which produce a 128-bit result, along with `ADD` operations to compute the low
128 bits of the product. The precise native instruction sequence is
implementation-defined. Whether or not this optional lowering is used, the
operation must produce the register results and compute-unit consumption
specified above.

An SVM implementation that does not provide a JIT lowering must execute
`sol_multi3` through an interpreter or another backend with identical
observable behavior.

### Verification

`CALL_IMM` instructions with a source register field of `0` are static calls and
their immediate field is an identifier, not a PC-relative call offset.

The verifier must therefore only perform relative call-target validation when
the source register field indicates an internal function call. Static call
identifiers, including the `sol_multi3` identifier, must not be interpreted as
relative branch offsets.

### Edge Cases

- Multiplication by zero must produce zero.
- Multiplication overflow must wrap modulo `2^128`.
- The maximum operand value, `2^128 - 1`, must be handled without host-language
  overflow or undefined behavior.
- The result must be identical regardless of host architecture or whether the
  program is interpreted or JIT-compiled.

## Alternatives Considered

- New BPF instructions could represent accelerated operations directly and
  bind operands to arbitrary registers, avoiding register-shuffling
  instructions. However, they would extend the BPF ISA and require
  ecosystem-wide ISA support.
- Regular syscalls could provide the same operations, but syscall dispatch
  introduces unnecessary overhead for small computational operations that can
  be emitted directly by the JIT.

## Impact

The `sol_multi3` operation provides a portable BPF interface for wrapping
128-bit multiplication while allowing SVM implementations to take advantage of
host-architecture capabilities.

## Security Considerations

All execution backends must agree on the `r0` and `r2` results and compute-unit
consumption for every input. A host architecture's integer overflow behavior or
native ABI must not leak into the BPF-visible semantics. Implementations must
preserve the original high limb in `r2` until it has been used in the
multiplication.

The `sol_multi3` name and its Murmur3 identifier are protocol constants. Its
identifier must be checked for collisions with existing static call identifiers
before activation.

## Drawbacks *(Optional)*

n/a

## Backwards Compatibility *(Optional)*

Existing programs are unaffected. Programs may opt into `sol_multi3` by using
its static call identifier.

## Conformance

Conformance tests must verify the specified `r0` and `r2` results, that all
other registers remain unchanged, and identical compute-unit consumption
across execution backends. They must also verify that `sol_multi3` does not
modify VM memory.

The test vectors must include zero, one, `u64::MAX`, `2^127`, and `u128::MAX`
operands and products that do and do not overflow 128 bits.
