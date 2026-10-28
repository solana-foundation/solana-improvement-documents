---
simd: '0708'
title: Vector Intrinsics
authors:
  - frank
  - cavey
  - ben
category: Standard
type: Core
status: Draft
created: 2026-10-28
feature:
development:
---

## Summary

Introduce native vector intrinsics & 30 512-bit vector registers to the SVM.

Vector intrinsics are static BPF calls corresponding to hardware AVX-512
instructions. Each vector intrinsic is represented as a single `CALL_IMM`
instruction with `src=2`. The intrinsic's opcode & operands are packed into the
32-bit immediate.

Similar to how [SIMD-0645] lowers the `sol_multi3` call to a single native host
instruction, vector intrinsics can typically be lowered to a single host
AVX-512 instruction.

## Motivation

Many desirable workloads are currently very expensive in the SVM. This is
because sBPF has no concept of vectorization or data-level parallelism.
Workloads such as post-quantum signature verification, hashing, field
arithmetic, and ML inference are extremely well suited for vectorized integer
arithmetic & logical operations.

Program developers today have two options:

1. Write their program in scalar sBPF. This is exorbitantly expensive.
2. Propose a dedicated syscall per algorithm (`sol_keccak256`, `sol_poseidon`).
   This is a protocol change and therefore requires a SIMD for each new
   operation.

Vector intrinsics allow developers to build highly sophisticated algorithms
on chain without compromising on efficiency.

### Prototype Results

We prototyped vector intrinsics in an [sBPF fork].

In three benchmark kernels (int8 MLP, Keccak-f[1600], and Falcon-512 signature
verification) we see wallclock improvements ranging from 25x-225x compared to
scalar sBPF. Our kernels also consume 8.8x-232x less CU at <2.8ns-per-CU
including loads & stores, indicating that 1 CU per op is not underpricing these
intrinsics.

| Workload | Scalar sBPF | Vector intrinsics | Ratio |
|---|---:|---:|---:|
| int8 MLP 256→128→16, CU | 490,048 | 2,108 | 232× |
| int8 MLP, JIT | 361 µs | 1.6 µs | 225× |
| Keccak-f[1600], CU per permutation | 11,824 | 988 | 12× |
| Keccak-f[1600], JIT per permutation | 8.9 µs | 0.32 µs | 28× |
| Falcon-512 verify, CU | 182,764 | 20,770 | 8.8× |
| Falcon-512 verify, JIT | 294 µs | 11.7 µs | 25× |

[sBPF fork]: https://github.com/temporalxyz/sbpf/tree/vector-ops

## Dependencies

This proposal depends on the precedents set in [SIMD-0645] pursuant to JIT
intrinsic support & metering. It does not depend on `sol_multi3` itself.

[SIMD-0645]: https://github.com/solana-foundation/solana-improvement-documents/pull/645

## New Terminology

- **Vector register**: one of `v0`-`v29`. Each is 512 bits of VM state and is
  not memory mapped.
- **Vector intrinsic**: a `CALL_IMM` instruction with `src=2`.
- **Lane width**: the size of integer a given vector intrinsic operates on. `b`
  is 64 × u8, `w` is 32 × u16, `d` is 16 × u32 and `q` is 8 × u64. Lane `i` of
  width `x` is denoted `vN.x[i]`. Lanes are little-endian.
- **Kind**: the operand shape of a vector intrinsic. The kind says which
  immediate fields are read, which are written, and whether the op uses `r0`,
  `r1` or `imm8`.

## Detailed Design

### Vector Registers

30 vector registers, `v0`-`v29`, each 512 bits wide, are added to the VM.

Programs can only access vector registers through vector intrinsics. Vector
registers have no address in any memory region.

Each program invocation (top-level or CPI) maintains its own vector register
file.

All vector registers are zero when a program's entrypoint is invoked. A CPI
gives the callee its own set of zeroed vector registers. The caller's vector
registers are unchanged on CPI return.

Vector registers are not tied to call frames. Internal calls do not save or
restore them.

Syscalls must not modify the caller's vector registers.

### Encoding

A vector intrinsic is an instruction with:

- `opc = 0x85` (`CALL_IMM`)
- `src = 2`
- `dst = 0`
- `off = 0`, except for memory ops
- `imm` laid out as follows:

```text
 31      24 23      16 15 14   10 9      5 4      0
+----------+----------+--+-------+--------+--------+
|    op    |   imm8   |0 |  out  |  in0   |  in1   |
+----------+----------+--+-------+--------+--------+
```

As a LE u64, a vector intrinsic is `0x85 | (0x20 << 8) | (imm << 32)`.

`out`, `in0`, and `in1` are vector register numbers. `imm8` is an 8-bit
immediate. Bit 15 is reserved and must be zero.

Unlike SIMD-0645 and SIMD-0178, the immediate is not a murmur32 hash. Vector
intrinsics do not share the static syscall namespace, so they cannot collide
with syscall identifiers.

### Scalar Conventions

- Intrinsics that accept a scalar input do so via `r1`.
- Intrinsics that produce a scalar output do so via `r0`.
- Intrinsics with no scalar output do not change `r0`.
- No intrinsic modifies any scalar register other than `r0`.

`r1` is not clobbered. The same pointer or value stored in `r1` can be used
across operations.

### Memory Operations

`ld`, `st`, `ldbc.d` and `ldbc.q` access memory at the address
`r1 + sext(off)`, exactly like scalar `ldx*` and `stx*`.

- Unaligned accesses are permitted.
- The whole access (64 bytes for `ld` and `st`, 4 or 8 bytes for broadcasts)
  must lie within one mapped region with the required permission. Otherwise,
  the intrinsic raises the same error as the equivalent scalar access.
- Vector registers are unchanged on memory operation fault.

### Kinds

| Kind | Reads | Writes | `imm8` |
|---|---|---|---|
| `r3` | in0, in1 | out | 0 |
| `r3i` | in0, in1 | out | per op |
| `a3` | out, in0, in1 | out | 0 |
| `a3i` | out, in0, in1 | out | per op |
| `r2` | in0 | out | 0 |
| `r2i` | in0 | out | per op |
| `r0` | — | out | 0 |
| `ls` | r1 | out | 0 |
| `st` | in0, r1 | memory | 0 |
| `sr` | in0 | r0 | 0 |
| `sri` | in0 | r0 | per op |
| `ai` | out, r1 | out | per op |

Register fields that the kind does not use must be zero. For the accumulating
kinds (`a3`, `a3i`, `ai`), `out` is both an input and the output.

### Supported Operations

Arithmetic wraps modulo lane width unless saturation is explicitly stated.
`sat8`, `sat16`, and other saturating operations saturate at the signed range
of that integer width. `usat8` saturates at the unsigned range.

The x86-64 column is not authoritative and shows the AVX-512 lowering used by
the prototype JIT implementation.

TODO: FINALIZE OPERATION LIST

#### Memory, splat and move

| op | Name | Kind | Semantics | x86-64 |
|---|---|---|---|---|
| 0x00 | `ld` | `ls` | out ← mem[a..a+64] | `vmovdqu64` |
| 0x01 | `st` | `st` | mem[a..a+64] ← in0 | `vmovdqu64` |
| 0x02 | `ldbc.d` | `ls` | out.d[i] ← mem32[a] | `vpbroadcastd` |
| 0x03 | `ldbc.q` | `ls` | out.q[i] ← mem64[a] | `vpbroadcastq` |
| 0x04 | `splat.b` | `ls` | out.b[i] ← r1[7:0] | `vpbroadcastb` |
| 0x05 | `splat.w` | `ls` | out.w[i] ← r1[15:0] | `vpbroadcastw` |
| 0x06 | `splat.d` | `ls` | out.d[i] ← r1[31:0] | `vpbroadcastd` |
| 0x07 | `splat.q` | `ls` | out.q[i] ← r1 | `vpbroadcastq` |
| 0x08 | `zero` | `r0` | out ← 0 | `vpxord` |
| 0x09 | `mov` | `r2` | out ← in0 | `vmovdqa64` |

`a` is `r1 + sext(off)`.

#### Integer arithmetic

| op | Name | Kind | Semantics | x86-64 |
|---|---|---|---|---|
| 0x10–0x13 | `add.{b,w,d,q}` | `r3` | in0 + in1 | `vpadd{b,w,d,q}` |
| 0x14–0x17 | `sub.{b,w,d,q}` | `r3` | in0 − in1 | `vpsub{b,w,d,q}` |
| 0x18 | `mullo.w` | `r3` | low 16 bits of product | `vpmullw` |
| 0x19 | `mullo.d` | `r3` | low 32 bits of product | `vpmulld` |
| 0x1a | `mullo.q` | `r3` | low 64 bits of product | `vpmullq` |
| 0x1b | `mulwu` | `r3` | even u32 lanes, u64 product | `vpmuludq` |
| 0x1c | `mulws` | `r3` | even i32 lanes, i64 product | `vpmuldq` |
| 0x1d | `mulhi.w` | `r3` | high 16 bits, signed | `vpmulhw` |
| 0x1e | `mulhiu.w` | `r3` | high 16 bits, unsigned | `vpmulhuw` |
| 0x20 | `max.d` | `r3` | signed | `vpmaxsd` |
| 0x21 | `min.d` | `r3` | signed | `vpminsd` |
| 0x22 | `maxu.d` | `r3` | unsigned | `vpmaxud` |
| 0x23 | `minu.d` | `r3` | unsigned | `vpminud` |
| 0x24 | `max.w` | `r3` | signed | `vpmaxsw` |
| 0x25 | `min.w` | `r3` | signed | `vpminsw` |
| 0x26 | `maxu.q` | `r3` | unsigned | `vpmaxuq` |
| 0x27 | `minu.q` | `r3` | unsigned | `vpminuq` |
| 0x2e | `maxu.w` | `r3` | unsigned | `vpmaxuw` |
| 0x2f | `minu.w` | `r3` | unsigned | `vpminuw` |

`mulwu` and `mulws` multiply the even 32-bit lanes into full 64-bit products:
`out.q[i] ← in0.d[2i] · in1.d[2i]`, unsigned for `mulwu` and signed for
`mulws`.

#### Bitwise, shifts and rotates

| op | Name | Kind | Semantics | x86-64 |
|---|---|---|---|---|
| 0x28 | `and` | `r3` | in0 & in1 | `vpandq` |
| 0x29 | `or` | `r3` | in0 \| in1 | `vporq` |
| 0x2a | `xor` | `r3` | in0 ^ in1 | `vpxorq` |
| 0x2b | `andn` | `r3` | ~in0 & in1 | `vpandnq` |
| 0x2c | `ternlog` | `a3i` | see below | `vpternlogq` |
| 0x30–0x32 | `shli.{w,d,q}` | `r2i` | in0 << imm8 | `vpsll{w,d,q}` |
| 0x33–0x35 | `shri.{w,d,q}` | `r2i` | logical in0 >> imm8 | `vpsrl{w,d,q}` |
| 0x36–0x38 | `sari.{w,d,q}` | `r2i` | arithmetic in0 >> imm8 | `vpsra{w,d,q}` |
| 0x39 | `roli.d` | `r2i` | rotate left by imm8 | `vprold` |
| 0x3a | `roli.q` | `r2i` | rotate left by imm8 | `vprolq` |
| 0x3b | `shlv.d` | `r3` | in0 << in1, per lane | `vpsllvd` |
| 0x3c | `shlv.q` | `r3` | in0 << in1, per lane | `vpsllvq` |
| 0x3d | `shrv.d` | `r3` | logical in0 >> in1, per lane | `vpsrlvd` |
| 0x3e | `shrv.q` | `r3` | logical in0 >> in1, per lane | `vpsrlvq` |
| 0x3f | `rolv.d` | `r3` | rotate left by in1 mod 32 | `vprolvd` |
| 0x40 | `rolv.q` | `r3` | rotate left by in1 mod 64 | `vprolvq` |

`imm8` must be less than the lane width in bits for immediate shifts & rotates.
A count greater than or equal to the lane width yields zero for variable
shifts.

#### Comparisons

Comparisons write all ones to lanes where the comparison is true and zero
elsewhere. There are no mask registers.

| op | Name | Kind | Semantics | x86-64 |
|---|---|---|---|---|
| 0x48 | `cmpeq.b` | `r3` | in0 == in1 | `vpcmpeqb` + `vpmovm2b` |
| 0x49 | `cmpeq.d` | `r3` | in0 == in1 | `vpcmpeqd` + `vpmovm2d` |
| 0x4a | `cmpeq.q` | `r3` | in0 == in1 | `vpcmpeqq` + `vpmovm2q` |
| 0x4b | `cmpgt.d` | `r3` | signed in0 > in1 | `vpcmpgtd` + `vpmovm2d` |
| 0x4c | `cmpgt.q` | `r3` | signed in0 > in1 | `vpcmpgtq` + `vpmovm2q` |
| 0x4d | `cmpgtu.q` | `r3` | unsigned in0 > in1 | `vpcmpuq` + `vpmovm2q` |

#### Dot products and multiply-add

| op | Name | Kind | Semantics | x86-64 |
|---|---|---|---|---|
| 0x50 | `dpbusd` | `a3` | u8 · s8 in groups of 4, accumulate | `vpdpbusd` |
| 0x51 | `dpwssd` | `a3` | s16 · s16 in pairs, accumulate | `vpdpwssd` |
| 0x52 | `maddubs` | `r3` | u8 · s8 in pairs, saturate | `vpmaddubsw` |
| 0x53 | `maddwd` | `r3` | s16 · s16 in pairs | `vpmaddwd` |
| 0x54 | `madd52lo` | `a3` | low 52 bits of 52-bit product | `vpmadd52luq` |
| 0x55 | `madd52hi` | `a3` | high 52 bits of 52-bit product | `vpmadd52huq` |

```text
dpbusd    out.d[i] += Σ(j<4) u8(in0.b[4i+j]) · s8(in1.b[4i+j])
dpwssd    out.d[i] += Σ(j<2) s16(in0.w[2i+j]) · s16(in1.w[2i+j])
maddubs   out.w[i] ← sat16(Σ(j<2) u8(in0.b[2i+j]) · s8(in1.b[2i+j]))
maddwd    out.d[i] ← Σ(j<2) s16(in0.w[2i+j]) · s16(in1.w[2i+j])
madd52lo  out.q[i] += lo52(m52(in0.q[i]) · m52(in1.q[i]))
madd52hi  out.q[i] += hi52(m52(in0.q[i]) · m52(in1.q[i]))
```

`m52(x)` is the low 52 bits of `x`. `lo52` and `hi52` are bits 0–51 and 52–103
of the 104-bit product. The accumulations in `dpbusd`, `dpwssd`, `maddwd`,
`madd52lo` and `madd52hi` wrap modulo the lane width.

#### Pack, permute and shuffle

| op | Name | Kind | Semantics | x86-64 |
|---|---|---|---|---|
| 0x58 | `packs.d` | `r3` | i32 → i16, `sat16` | `vpackssdw` |
| 0x59 | `packs.w` | `r3` | i16 → i8, `sat8` | `vpacksswb` |
| 0x5a | `packus.w` | `r3` | i16 → u8, `usat8` | `vpackuswb` |
| 0x5b | `permd` | `r3` | out.d[i] ← in1.d[in0.d[i] & 15] | `vpermd` |
| 0x5c | `permq` | `r3` | out.q[i] ← in1.q[in0.q[i] & 7] | `vpermq` |
| 0x5d | `shufb` | `r3` | see below | `vpshufb` |
| 0x5e | `perm2.w` | `a3` | see below | `vpermt2w` |

The pack ops work per 128-bit lane L: lane L of `out` holds the narrowed lanes
of `in0`'s lane L, followed by the narrowed lanes of `in1`'s lane L.

`shufb`: per 128-bit lane L,
`out.b[i] ← (in1.b[i] & 0x80) ? 0 : in0.b[16L + (in1.b[i] & 15)]`.

`perm2.w`: with `k = in0.w[i] & 63`,
`out.w[i] ← k < 32 ? out.w[k] : in1.w[k − 32]`. All lanes read the value `out`
had before the op.

#### Widen, narrow and popcount

| op | Name | Kind | Semantics | x86-64 |
|---|---|---|---|---|
| 0x60 | `narrowus.d.b` | `r2` | out.b[i] ← usat8(u32(in0.d[i])) | `vpmovusdb` |
| 0x61 | `narrows.d.b` | `r2` | out.b[i] ← sat8(i32(in0.d[i])) | `vpmovsdb` |
| 0x62 | `narrows.d.w` | `r2` | out.w[i] ← sat16(i32(in0.d[i])) | `vpmovsdw` |
| 0x63 | `narrow.d.b` | `r2` | out.b[i] ← in0.d[i] truncated | `vpmovdb` |
| 0x64 | `zx.b.d` | `r2` | out.d[i] ← u8(in0.b[i]) | `vpmovzxbd` |
| 0x65 | `sx.b.d` | `r2` | out.d[i] ← i8(in0.b[i]) | `vpmovsxbd` |
| 0x66 | `zx.w.d` | `r2` | out.d[i] ← u16(in0.w[i]) | `vpmovzxwd` |
| 0x67 | `sx.w.d` | `r2` | out.d[i] ← i16(in0.w[i]) | `vpmovsxwd` |
| 0x68 | `zx.d.q` | `r2` | out.q[i] ← u32(in0.d[i]) | `vpmovzxdq` |
| 0x69 | `sx.d.q` | `r2` | out.q[i] ← i32(in0.d[i]) | `vpmovsxdq` |
| 0x6a | `popcnt.b` | `r2` | per u8 | `vpopcntb` |
| 0x6b | `popcnt.q` | `r2` | per u64 | `vpopcntq` |

Narrowing ops fill the low 16 output lanes, with i < 16, and set the rest of
`out` to zero. Widening ops read the low input lanes, so 16 for `.d` results
and 8 for `.q` results.

#### Vector to scalar

| op | Name | Kind | Semantics | x86-64 |
|---|---|---|---|---|
| 0x70 | `hsum.d` | `sr` | r0 ← zext(Σ in0.d[i] mod 2^32) | extract/add tree |
| 0x71 | `hsum.q` | `sr` | r0 ← Σ in0.q[i] mod 2^64 | extract/add tree |
| 0x72 | `extract.q` | `sri` | r0 ← in0.q[imm8] | `vextracti32x4`, `vpextrq` |
| 0x73 | `extract.d` | `sri` | r0 ← in0.d[imm8] | `vextracti32x4`, `vpextrd` |
| 0x74 | `insert.q` | `ai` | out.q[imm8] ← r1 | implementation-defined |
| 0x75 | `movemask.b` | `sr` | r0[i] ← msb(in0.b[i]) | `vpmovb2m`, `kmovq` |

`imm8` must be less than 8 for `extract.q` and `insert.q`, and less than 16
for `extract.d`. `extract.d` zero-extends the lane into `r0`. `msb(x)` is the
most significant bit of `x`.

#### Carry-less multiply

| op | Name | Kind | Semantics | x86-64 |
|---|---|---|---|---|
| 0x78 | `clmul` | `r3i` | see below | `vpclmulqdq` |

Per 128-bit lane L, `out.lane[L] ← clmul(in0.q[2L + (imm8 & 1)],
in1.q[2L + ((imm8 >> 4) & 1)])`. `imm8` must be one of `0x00`, `0x01`, `0x10`
or `0x11`.

### Metering

All vector intrinsics consume 1 compute unit, the standard charge for
`CALL_IMM`.

### Execution Backends

The native lowering for vector intrinsics is implementation-defined. The
prototype JIT pins `vN` to `zmmN` and keeps `zmm30`, `zmm31` and `k1`–`k7` as
scratch. An implementation may manage vector registers and execute vector
intrinsics in any way, such as in an interpreter, AVX2 pairs, NEON quads, or
SVE, provided the backend produces identical register, memory, `r0`, and CU
results.

Alternative vector register management strategies must preserve behavior as
specified in this document.

### Edge Cases

- Operations that read & write the same register (`add.d v3, v3, v3`) are
  valid.
- Memory operations that cross regions fault even if both regions are mapped
  and writable.
- A memory op at `r1 + sext(off)` where the addition overflows 64 bits faults
  identically to the equivalent scalar access.
- `hsum.d` and `extract.d` zero-extend into `r0`.
- `perm2.w` uses the old value of `out` as both table and destination.

### Validator Components Affected

| Validator Component | Impact |
|---|---|
| Transaction Execution (Runtime) | 1,920 bytes of vector state per invocation |
| Virtual Machine | New VM state, interpreter and JIT |
| Block Packing | None |
| Consensus | None |
| Gossip | None |
| Turbine | None |
| Snapshots | None |
| On-Chain Core BPF Programs | None |
| Other (please describe) | None |

## Alternatives Considered

- The `sol_math` syscall. This was rejected as it required implementing what
  was essentially a new ISA & interpreter within the VM.
- A new memory region mapped to hardware vector registers. This was rejected
  to avoid overloading memory translation.

## Impact

Programs may now use vector intrinsics to access optimized integer operations.
This improves the robustness of Solana's computational environment by an order
of magnitude.

Programs will need to be recompiled and redeployed to adopt vector intrinsics.

## Security Considerations

All execution backends must agree on the resulting vector registers, `r0`,
memory and compute unit consumption for every vector op and every input. A host
architecture's instruction semantics must not leak into the program-visible
semantics. Examples are shift-count masking, saturation behavior, and the
results a host produces for out-of-range indices. This holds whether an op is
lowered to a single AVX-512 instruction, to a multi-instruction sequence on a
host without AVX-512, or executed by an interpreter.

Vector registers must not carry state between program invocations. An
implementation that keeps vector registers in host registers must zero them at
entry and save them before any host code runs, including syscalls and CPI. It
must restore them afterwards, and must leave the host's vector state as the
host ABI expects when control returns to it. A callee must not be able to read
or modify its caller's vector registers.

Vector memory ops must apply the same address translation and permission checks
as scalar loads and stores over the entire width of the access. A faulting
access must not partially write memory or modify any vector register.

A malformed vector operation that reaches execution must fail with the same
error on every backend.

## Backwards Compatibility

Programs using vector intrinsics could not be used on Solana or sBPF versions
which do not support it. Existing programs that do not use vector intrinsics
are not impacted. Therefore, a feature gate should be used to enable this
feature when the majority of the cluster is using the required version.

## Drawbacks

- VM state increases by 1,920 bytes per invocation
- Every client must implement all vector intrinsics in its interpreter & JIT
- Fallbacks from AVX-512 negate the speedup from vector intrinsics without
  changing CU cost

## Conformance

Conformance tests must verify, for every op, the resulting vector registers,
`r0`, memory and CU consumption on every backend. They must also verify that:

- vector registers are zero at entry, are preserved across internal calls and
  syscalls, and are independent across CPI
- memory ops fault on the first byte outside a region, on read-only regions for
  `st`, and on cross-region accesses, leaving vector registers unchanged
- inputs at the boundary values are handled for each lane width: 0, 1, the
  signed minimum and maximum, and all ones
- programs without vector intrinsics produce identical results and CU before
  and after activation
