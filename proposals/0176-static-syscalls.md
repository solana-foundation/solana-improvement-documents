---
simd: '0176'
title: Static Syscalls for SBPF
authors:
  - Alessandro Decina
  - Alexander Meißner
  - Lucas Steuernagel
category: Standard
type: Core
status: Draft
created: 2024-09-27
---

## Summary

This SIMD proposes the removal of relocations from syscalls in the SBPF 
instruction set and the simplification of the encoding of the `call imm` 
(opcode `0x85`) instruction.

## Motivation

Address relocations are a performance burden for loading programs in the 
validator. Relocations require an entire copy of the ELF file in memory to 
either relocate addresses we fetch from the symbol table or offset addresses 
to after the start of the virtual machine’s memory. Moreover, relocations pose 
security concerns, as they allow the arbitrary modification of program headers 
and programs sections. We aim to remove relocations altogether at a later 
point, so removing them from syscall resolution is a step towards such a goal.

## New Terminology

None.

## Detailed Design

The following must go into effect if and only if a program indicates the SBPF 
version XX or higher in its ELF header e_flags field, according to the 
specification of SIMD-0161.

### New semantics for the call instruction

The source register field in the `call imm` (opcode `0x85`) instruction 
encoding does not currently carry any meaning. We want to repurpose such a 
field to encode whether the call is internal or external. When the source 
field is set to zero, the instruction must represent a system call, and when 
it is set to one, the instruction must represent an internal call. For more 
reference on the SBPF ISA format, see the 
[spec document](https://github.com/solana-labs/rbpf/blob/main/doc/bytecode.md).

This change comes together with modifications in the instruction execution. We 
must keep the existing behavior of `call imm` for internal calls: the 
immediate field represents the offset to add to the current program counter. 
For external calls, the immediate must contain the murmur32 hash of the system 
call name.

Consequently, system calls in the Solana SDK and in any related compiler tools 
must be registered as function pointers, whose address is the murmur32 hash of 
their name. The verifier must enforce that the immediate of a call instruction 
whose source field is zero represents a valid syscall, and throw 
`VerifierError::InvalidFunction` otherwise.

## Alternatives Considered

None.

## Impact

The changes proposed in this SIMD are transparent to dApp developers. The 
compiler toolchain will emit correct code for the specified SBF version. The 
static syscalls will obviate relocations for the call instructions and moves 
the virtual machine closer to eliminating relocations altogether, which can 
bring considerable performance improvements.

## Security Considerations

None.
