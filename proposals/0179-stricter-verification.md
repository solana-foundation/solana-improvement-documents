---
simd: '0179'
title: SBPF Stricter verification constraints
authors:
  - Alexander Meißner
  - Lucas Steuernagel
category: Standard
type: Core
status: Draft
created: 2024-10-03
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

## New Terminology

None.

## Detailed Design

The following must go into effect if and only if a program indicates the SBF 
version XX or higher in its ELF header e_flags field, according to the 
specification of SIMD-0161.

### Restrict functions’ last instruction

Functions must only end with the `ja` (opcode `0x05`) or the exit (opcode 
`0x9D` since SIMD-0178) instruction. Allowing calls to be the last instruction 
of functions was inconvenient, because when the call returns, and there is no 
other instruction to redirect the control flow, we will execute the very next 
program counter, resulting in a fallthrough into another function’s code. 
Offending this new validation condition must throw an 
`VerifierError::InvalidFunction` error.

### Restrict jump instruction destination

All jump instructions, except for `call` (opcode `0x85`) and `callx` (opcode 
`0x8D`), must now jump to a code location inside their own function. Jumping 
to arbitrary locations hinders a precise program verification. 
`VerifierError::JumpOutOfCode` must be thrown for offending this rule.

`call imm` (opcode `0x85`) must only be allowed to jump to a program counter 
previously registered as the start of a function. Otherwise 
`VerifierError::InvalidFunction` must be thrown. Functions must be registered 
if they are present in the symbol table. The entrypoint to the program must 
also define a valid function.

### Runtime check for callx

The jump destination of `callx` (opcode `0x8D`) must be checked during 
execution time to match the initial address of a registered function. If this 
is not the case, a `EbpfError::UnsupportedInstruction` must be thrown. This 
measure is supposed to improve security of programs, disallowing the malicious 
use of callx.

A function is registered according to the rules mentioned in the previous 
section: be present in the symbol table or be the entrypoint.

### Limit where a function can start

Presently, functions may start in any part of the ELF text section, however 
this is an encumbrance when it comes to `lddw` (opcode `0x18`) instructions. 
As they occupy two instruction slots in an ELF, it is possible for functions 
to start between these lddw slots, very likely resulting in undesired 
behavior. The verifier must throw an `VerifierError::JumpToMiddleOfLddw` error 
when that is the case.

### Removal of ExecutionOverrun error

As the jump instructions destinations are now limited to the range of the 
function they are located in, and the call instructions can only move the 
program counter to a known location, the `EbpfError::ExecutionOverrun` must not 
be reachable, so it must be removed.

## Alternatives Considered

None.

## Impact

The changes proposed in this SIMD are transparent to dApp developers. The 
compiler toolchain will emit correct code for the specified SBF version. The 
enhanced verification restrictions will improve the security of programs 
deployed to the blockchain, filtering out unwarranted behavior that could be 
used with malicious intents.

## Security Considerations

None.