---
simd: '0272'
title: SBPF Encoding Efficiency
authors:
  - Alexander Meißner
  - Lucas Steuernagel
category: Standard
type: Core
status: Idea
created: 2025-04-01
feature: TBD
extends: SIMD-0161
---

## Summary

Improve encoding efficiency in SBPF-v5.

## Motivation

SBPF inherited the 64 bit intruction layout from BPF including its space
inefficiency. For example the 16 bit displacement is only used for memory
access and conditional branch instructions and the 32 bit immediate value is
only used in instructions with an immediate operand. Yet, all other
instructions still need to encode them.

Additionally, there are only 10 general purpose registers which leads to a lot
of stack spilling and thus additional instructions and larger executable files.
Unlike typical CISC instruction sets like x86, which do have memory indirect
operands to compensate for their low number of general purpose registers, BPF
and thus in turn SBPF has no such thing.

Reducing the instruction frame size from 64 bit to 32 bit and increasing the
addressable registers from 11 to 32 should dramatically improve the encoding
efficiency and allow for bigger (meaning more complex) on-chain programs and
lower rent excemption funds being required for programs at the same complexity.

## New Terminology

None.

## Detailed Design

### Register Layouts

The current register layout from SBPF-v1 is:

|         name | kind            | Solana ABI
|-------------:|:----------------|:----------
| `r0`         | GPR             | Return value
| `r1` to `r5` | GPR             | Argument registers
| `r6` to `r9` | GPR             | Call-preserved
| `r10`        | Frame pointer   | System register
| `pc`         | Program counter | Hidden register

The register layout from SBPF-v5 on is:

|           name | kind            | Solana ABI
|---------------:|:----------------|:----------
| `r0`           | GPR             | Return value
| `r1` to `r5`   | GPR             | Argument registers
| `r6` to `r17`  | GPR             | Callee-saved
| `r18` to `r30` | GPR             | Caller-saved
| `r31`          | Frame pointer   | System register
| `pc`           | Program counter | Hidden register

### Instruction Layouts

The current instruction layout from SBPF-v1 is:

| bit index | meaning
| --------- | -------
| 0..=2     | instruction class
| 3..=7     | operation code
| 8..=11    | destination register and first source register
| 12..=15   | second source register
| 16..=31   | offset
| 32..=63   | immediate

The instruction layout from SBPF-v5 on depends on the instruction class,
see below:

#### Two Source Register Operands

For the 32 and 64 bit immediate-less variants of the following instructions:
add, sub, xor, or, and, lsh, rsh, arsh, udiv, urem, sdiv, srem, lmul, uhmul, shmul

| bit index | meaning
| --------- | -------
| 0..=6     | instruction class
| 7..=11    | destination register
| 12..=14   | lower 7 bits of operation code
| 15..=19   | first source register
| 20..=24   | second source register
| 25..=31   | upper 7 bits of operation code

#### One Source Register and a 12 bit Immidiate Operand

For the 32 and 64 bit immediate-valued variants of the following instructions:
add, sub, xor, or, and, lsh, rsh, arsh, udiv, urem, sdiv, srem, lmul, uhmul, shmul

And for the following instructions:
ldxb, ldxh, ldxw, ldxdw, callx, mov

| bit index | meaning
| --------- | -------
| 0..=6     | instruction class
| 7..=11    | destination register
| 12..=14   | operation code
| 15..=19   | first source register
| 20..=31   | 12 bit immediate

#### Two Source Register Operands and 12 bit Immidiate Operand

For the immediate-less variants of the following instructions:
jeq, jgt, jge, jset, jne, jsgt, jsge, jlt, jle, jslt, jsle

And for the following instructions:
stxb, stxh, stxw, stxdw

| bit index | meaning
| --------- | -------
| 0..=6     | instruction class
| 7..=11    | lower 5 bits of 12 bit immediate
| 12..=14   | operation code
| 15..=19   | first source register
| 20..=24   | second source register
| 25..=31   | upper 7 bits of 12 bit immediate

#### 20 bit Immidiate Operand

For the immediate-valued variants of the following instructions:
jeq, jgt, jge, jset, jne, jsgt, jsge, jlt, jle, jslt, jsle

And for the following instructions:
stb, sth, stw, stdw, hor, call, syscall, exit, ja

| bit index | meaning
| --------- | -------
| 0..=6     | instruction class
| 7..=11    | destination register
| 12..=31   | 20 bit immediate

## Alternatives Considered

None.

## Impact

Like the other SBPF versions these changes will be hidden inside the compiler
toolchain and be transparent to the dApp developers.

## Security Considerations

None.

## Drawbacks

This increases the complexity of the instruction decoder and thus slows down
interpreter based execution. Whether the increased encoding efficiency, reduced
memory bandwidth and cache pressure can make up for it depends on the
implementation.
