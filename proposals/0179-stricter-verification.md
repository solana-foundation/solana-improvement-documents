---
simd: '0179'
title: SBPF Stricter verification constraints
authors:
  - Alexander Meißner
  - Lucas Steuernagel
category: Standard
type: Core
status: Review
created: 2024-10-03
extends: SIMD-0178, SIMD-0377
---

## Summary

This SIMD proposes improvements in the verification rules of SBPF programs. 
The introduction of the syscall instruction in SIMD-0178 allows for the 
differentiation of internal and external calls, opening up for stricter 
verification constraints and increased security of deployed programs.

## Motivation

The `call` instruction (opcode `0x85`) in SBF, especially when it requires a 
relocation, is ambiguous. It does not carry information about whether it is 
a system call or an internal call. This ambiguity prevents more rigid 
verification constraints that would reject programs jumping to inconvenient 
locations, like invalid destinations or the middle of an `lddw` instruction. 
The new syscall instruction introduced in SIMD-0178 permits the 
differentiation of internal and external calls, so we can introduce new 
verification rules to prevent any unwarranted behavior.

Another motivitation is to have the ability to treat functions as
self-contained compilation units, allowing a hybrid virtual machine that works 
with both JIT compilation and an interpreter, depending on the trade off 
between performance and compilation time on a per function basis.

## New Terminology

None.

## Detailed Design

The following must go into effect if and only if a program indicates the SBF 
version `0x3` or higher in its ELF header e_flags field, according to the 
specification of SIMD-0161.

### Restrict functions' first instruction

Since SIMD-0166, SBPF functions can modify the stack pointer using the 
`add64 r10, imm` (opcode `0x07`) with the restriction that `r10` is the 
destination register and the source is an immediate value.

The instruction `add64 r10, imm` must be used as a marker for a function 
start. Therefore, the verifier must enfroce that functions start with 
an `add64 r10, imm` instruction that modifies the stack pointer `r10`, without 
any restriction for the immediate value.

As a corollary of this definition, a function must only have a single 
`add64 r10, imm` instruction. A second one would consequently indicate the 
start of another adjacent function.

### Restrict functions’ last instruction

Functions must only end with the `ja` (opcode `0x05`), the exit (opcode 
`0x95`) instruction or the `jx` instruction (opcode `0x0D`). Allowing calls to 
be the last instruction of functions was inconvenient, because when the call 
returns, and there is no other instruction to redirect the control flow, we 
will execute the very next program counter, resulting in a fallthrough into 
another function's code. Offending this new validation condition must throw an 
`VerifierError::InvalidFunction` error.

### Jump restrictions

This SIMD introduces in the two following subsections restrictions for jump 
destinations to be verified both during runtime and during verification time. 
They depend on knowing beforehand which program counter addresses represent 
valid function starts.

As previously mentioned, except for the first function in the ELF, 
`add64 r10, imm` must represent the start of a function, so validation must 
rely on this instruction as a source of truth.

#### Restrict jump instruction destination

All jump instructions, except for `call` (opcode `0x85`) and `callx` (opcode 
`0x8D`), must now jump to a code location inside their own function. Jumping 
to arbitrary locations hinders a precise program verification. 
`VerifierError::JumpOutOfCode` must be thrown for offending this rule.

`call imm` (opcode `0x85`) must only be allowed to jump to a program counter 
that points to an `add64 r10, imm` instruction. Otherwise 
`VerifierError::InvalidFunction` must be thrown.

#### Runtime check for callx and jx

The jump destination of `callx` (opcode `0x8D`) must be checked during 
execution time to be an `add64 reg, imm` instruction. If this is not the case, 
a `EbpfError::UnsupportedInstruction` must be thrown. This measure is supposed 
to improve security of programs, disallowing the malicious use of callx.

Likewise, the destination of a `jx` instruction (opcode `0x0D`) must be 
checked during runtime to be within the function it is located. If this is not 
the case, a `EbpfError::UnsupportedInstruction` must be thrown. Not only does 
this measure prevents malicious usage of the indirect branch, but also does it 
allow for tiered JITting.

### Prevent calls to middle of LLDW instruction

As `lddw` (opcode `0x18`) occupies two instruction slots in an ELF, it is 
possible for call instrcutions to jump in between these lddw slots, very 
likely resulting in undesired behavior. The verifier must throw an 
`VerifierError::JumpToMiddleOfLddw` error when that is the case.

### Removal of ExecutionOverrun error

As the jump instructions destinations are now limited to the range of the 
function they are located in, and the call instructions can only move the 
program counter to a known location, the `EbpfError::ExecutionOverrun` must not 
be reachable, so it must be removed.

## Alternatives Considered

None.

## Impact

The changes proposed in this SIMD are transparent to dApp developers. The 
compiler toolchain will emit correct code for the specified SBPF version. The 
enhanced verification restrictions will improve the security of programs 
deployed to the blockchain, filtering out unwarranted behavior that could be 
used with malicious intentions.

## Security Considerations

None.