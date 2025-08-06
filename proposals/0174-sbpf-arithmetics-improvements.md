---
simd: '0174'
title: SBPF arithmetics improvements
authors:
  - Alexander Meißner
category: Standard
type: Core
status: Review
created: 2024-09-06
feature: F6UVKh1ujTEFK3en2SyAL3cdVnqko1FVEXWhmdLRu6WP
extends: SIMD-0161
---

## Summary

This proposal introduces wide multiplication, signed division and explicit sign
extension to SBPF.

## Motivation

All major hardware ISAs support 64 x 64 = 128 bit multiplication; BPF does not.
This hurts performance of big integer arithmetics. Similarly, signed division
and signed remainder / modulo instructions must be emulated in software as
well.

Another issue related to arithmetics is that some 32 bit instructions perform
sign extension of their results (output, **not** input, there is a difference)
implicitly. This is first of all useless because source languages do not read
the 32 MSBs of a 32 bit result stored in a 64 bit register. And second, it
requires interpreters and compilers to perform extra work to add sign extension
at the end of these instructions. Instead we should go the same route as the
underlying hardware ISAs and require sign extension to be made explicit in a
dedicated instruction.

Furthermore, the instruction `sub dst, imm` is redundant as it can also be
encoded as `add dst, -imm`. If we were to swap the operands meaning of minuend
and subtrahend, then the instruction would become useful and would even render
`neg dst` redundant. Negation would then be encoded as `reg = 0 - reg`. This
would also make it clear how sign extension rules work for negation.

## Alternatives Considered

None.

## New Terminology

None.

## Detailed Design

The following must go into effect if and only if a program indicates the
SBPF-version (v2) or higher in its program header (see SIMD-0161).

### Changes to the Bytecode Verifier

A program containing one of the following instructions must throw
`VerifierError::UnknownOpCode` during verification:

- the `MUL` instruction (opcodes `0x24`, `0x2C`, `0x27` and `0x2F`)
- the `DIV` instruction (opcodes `0x34`, `0x3C`, `0x37` and `0x3F`)
- the `MOD` instruction (opcodes `0x94`, `0x9C`, `0x97` and `0x9F`)
- the `NEG` instruction (opcodes `0x84` and `0x87`)

A program containing one of the following instructions must **not** throw
`VerifierError::UnknownOpCode` during verification anymore (opcodes are for
immediate and register variant each, in that order):

- the `UHMUL64` instruction (opcode `0x36` and `0x3E`)
- the `UDIV32` instruction (opcode `0x46` and `0x4E`)
- the `UDIV64` instruction (opcode `0x56` and `0x5E`)
- the `UREM32` instruction (opcode `0x66` and `0x6E`)
- the `UREM64` instruction (opcode `0x76` and `0x7E`)
- the `LMUL32` instruction (opcode `0x86` and `0x8E`)
- the `LMUL64` instruction (opcode `0x96` and `0x9E`)
- the `SHMUL64` instruction (opcode `0xB6` and `0xBE`)
- the `SDIV32` instruction (opcode `0xC6` and `0xCE`)
- the `SDIV64` instruction (opcode `0xD6` and `0xDE`)
- the `SREM32` instruction (opcode `0xE6` and `0xEE`)
- the `SREM64` instruction (opcode `0xF6` and `0xFE`)

The verification rule, that an immediate divisor must not be zero or else
`VerifierError::DivisionByZero` is thrown, is moved to the new division like
instructions: `UDIV32_IMM`, `UDIV64_IMM`, `UREM32_IMM`, `UREM64_IMM`,
`SDIV32_IMM`, `SDIV64_IMM`, `SREM32_IMM`, `SREM64_IMM`.

### Changes to Execution

#### PQR Instruction Class

A new instruction class product, quotient and remainder (PQR) is introduced:

- the `UHMUL64` instruction (opcode `0x36` and `0x3E`) produces the 64 MSBs of
the product of an unsigned 64 x 64 bit multiplication (`dst * imm` and
`dst * src`).
- the `UDIV32` instruction (opcode `0x46` and `0x4E`) produces the quotient of
an unsigned 32 bit division (`dst / imm` and `dst / src`).
- the `UDIV64` instruction (opcode `0x56` and `0x5E`) produces the quotient of
an unsigned 64 bit division (`dst / imm` and `dst / src`).
- the `UREM32` instruction (opcode `0x66` and `0x6E`) produces the remainder of
an unsigned 64 bit division (`dst % imm` and `dst % src`).
- the `UREM64` instruction (opcode `0x76` and `0x7E`) produces the remainder of
an unsigned 64 bit division (`dst % imm` and `dst % src`).
- the `LMUL32` instruction (opcode `0x86` and `0x8E`) produces the 32 LSBs of
the product of any 32 x 32 bit multiplication (`dst * imm` and `dst * src`).
- the `LMUL64` instruction (opcode `0x96` and `0x9E`) produces the 64 LSBs of
the product of any 64 x 64 bit multiplication (`dst * imm` and `dst * src`).
- the `SHMUL64` instruction (opcode `0xB6` and `0xBE`) produces the 64 MSBs of
the product of a signed 64 x 64 bit multiplication (`dst * imm` and
`dst * src`).
- the `SDIV32` instruction (opcode `0xC6` and `0xCE`) produces the quotient of
a signed 32 bit division (`dst / imm` and `dst / src`).
- the `SDIV64` instruction (opcode `0xD6` and `0xDE`) produces the quotient of
a signed 64 bit division (`dst / imm` and `dst / src`).
- the `SREM32` instruction (opcode `0xE6` and `0xEE`) produces the remainder of
a signed 32 bit division (`dst / imm` and `dst / src`).
- the `SREM64` instruction (opcode `0xF6` and `0xFE`) produces the remainder of
a signed 64 bit division (`dst / imm` and `dst / src`).

Runtime exceptions are:

- If the divisor (`imm` or `src`) of any division is zero,
`EbpfError::DivideByZero` must be thrown.
- If the dividend (`dst`) of a signed division has the minimal value for its
bit-width and the divisor (`imm` or `src`) is `-1`, then
`EbpfError::DivideOverflow` must be thrown.

#### Explicit Sign Extension

The following instructions must stop performing implicit sign extension of
their results, instead filling the 32 MSBs of `dst` with zeros:

- the `ADD32` instruction (opcode `0x04` and `0x0C`)
- the `SUB32` instruction (opcode `0x14` and `0x1C`)
- all of the new 32 bit PQR instructions: `UDIV32`, `UREM32`, `SDIV32`,
`SREM32` and `LMUL32`

Instead the `MOV32_REG` instruction (opcode `0xBC`) which until now did zero
out the 32 MSBs, must now perform sign extension in the 32 MSBs. Meaning this
instruction becomes the explicit sign extension operation.

#### Register Immediate Subtraction

The operands roles of `SUB32_IMM` (opcode `0x14`) and `SUB64_IMM` (opcode
`0x17`) must be swapped: Until now the resulting difference was `src - imm` and
it must be changed to `imm - src`.

## Impact

The toolchain will emit machine-code according to the selected SBPF version.
Big integer arithmetics and signed divisions will become cheaper CU wise.

## Security Considerations

None.
