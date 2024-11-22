---
simd: '0173'
title: SBPF instruction encoding improvements
authors:
  - Alexander Meißner
category: Standard
type: Core
status: Review
created: 2024-09-05
feature: F6UVKh1ujTEFK3en2SyAL3cdVnqko1FVEXWhmdLRu6WP
extends: SIMD-0161
---

## Summary

There are some instructions with questionable encodings, that when slightly
adjusted, could significantly simplify verification and execution of programs.

## Motivation

The instruction `lddw dst, imm` is currently the only instruction which takes
two instruction slots. This proposal splits it into a two one-slot instruction
sequence: `mov32 dst, imm` and an introduced `hor64 dst, imm`. This way all
instructions will be exactly one slot long which will simplify:

- Calculating the number of instructons in a program will no longer require a
full linear scan. A division of the length of the text section by the
instruction slot size will suffice.
- The instruction meter will no longer have to skip one instruction slot when
counting a `LDDW` instruction.
- Jump and call instructions will no longer have to verify that the desination
is not the second half of a `LDDW` instruction.
- The verifier will no longer have to check that `LDDW` instructions are
complete and its first or second half does not occur without the other on its
own.

The `LE` instruction is essentially useless as only `BE` performs a byte-swap.
Its runtime behavior is close to no-op and can be replicated by other
instructions:

- `le dst, 16` behaves the same as `and32 dst, 0xFFFF`
- `le dst, 32` behaves the same as `and32 dst, 0xFFFFFFFF`
- `le dst, 64` behaves the same as `mov64 dst, src`

The `CALLX` instruction encodes its source register in the immediate field.
This is makes the instruction decoder more complex because it is the only case
in which a register is encoded in the immediate field, for no reason.

With all of the above changes and the ones defined in SIMD-0174, the memory
related instructions can be moved into the ALU instruction classes. Doing so
would free up 8 instruction classes completely, giving us back three bits of
instruction encoding.

## Alternatives Considered

None.

## New Terminology

None.

## Detailed Design

The following must go into effect if and only if a program indicates the
SBPF-version v2 or higher in its program header (see SIMD-0161). Some now
unreachable verification and execution checks around `LDDW` can be safely
removed (see motivation).

### Changes to the Bytecode Verifier

A program containing one of the following instructions must throw
`VerifierError::UnknownOpCode` during verification:

- the `LDDW` instruction (opcodes `0x18` and `0x00`)
- the `LE` instruction (opcode `0xD4`)
- the moved opcodes:
  - `0x72`, `0x71`, `0x73` (`STB`, `LDXB`, `STXB`)
  - `0x6A`, `0x69`, `0x6B` (`STH`, `LDXH`, `STXH`)
  - `0x62`, `0x61`, `0x63` (`STW`, `LDXW`, `STXW`)
  - `0x7A`, `0x79`, `0x7B` (`STDW`, `LDXDW`, `STXDW`)

A program containing one of the following instructions must **not** throw
`VerifierError::UnknownOpCode` during verification anymore:

- the `HOR64` instruction (opcode `0xF7`)
- the moved opcodes:
  - `0x27`, `0x2C`, `0x2F` (`STB`, `LDXB`, `STXB`)
  - `0x37`, `0x3C`, `0x3F` (`STH`, `LDXH`, `STXH`)
  - `0x87`, `0x8C`, `0x8F` (`STW`, `LDXW`, `STXW`)
  - `0x97`, `0x9C`, `0x9F` (`STDW`, `LDXDW`, `STXDW`)

When a `CALLX` instruction (opcode `0x8D`) is encountered during verification,
the `src` register field must be verified instead of the `imm` immediate field.
Otherwise, the verification rule stays the same: The src register must be in
the inclusive range from R0 to R9.

### Changes to Execution

The introduced `HOR64` instruction (opcode `0xF7`) must take its immediate
value, shift it 32 bit towards the MSBs (multiplication-like left shift) and
then bitwise OR it into the given `dst` register.

For the `CALLX` instruction (opcode `0x8D`) the jump destination must be read
from the `src` register field instead of the `imm` immediate field.

The execution behavior of the moved instructions is transferred to their new
opcodes:

- `0x72` => `0x27`, `0x71` => `0x2C`, `0x73` => `0x2F`
- `0x6A` => `0x37`, `0x69` => `0x3C`, `0x6B` => `0x3F`
- `0x62` => `0x87`, `0x61` => `0x8C`, `0x63` => `0x8F`
- `0x7A` => `0x97`, `0x79` => `0x9C`, `0x7B` => `0x9F`

## Impact

The toolchain will emit machinecode according to the selected SBPF version.
As most proposed changes affect the encoding only, and not the functionallity,
we expect to see no impact on dApp developers. The only exception is that
64-bit immediate loads will now cost 2 CU instead of 1 CU.

## Security Considerations

None.
