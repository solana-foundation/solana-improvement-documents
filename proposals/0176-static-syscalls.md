---
simd: '0176'
title: SBPF Static Syscalls
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

This SIMD introduces a new instruction syscall in the SBPF instruction set to 
represent syscalls. Such a change aims to remove relocations when resolving 
syscalls and simplify the instruction set, allowing for the straightforward 
differentiation between external and internal calls.

## Motivation

The resolution of syscalls during ELF loading requires relocating addresses, 
which is a performance burden for the validator. Relocations require an entire 
copy of the ELF file in memory to either relocate addresses we fetch from the 
symbol table or offset addresses to after the start of the virtual machine’s 
memory. Moreover, relocations pose security concerns, as they allow the 
arbitrary modification of program headers and programs sections. A new 
separate opcode for syscalls modifies the behavior of the ELF loader, allowing 
us to resolve syscalls without relocations.

## New Terminology

None.

## Detailed Design

The following must go into effect if and only if a program indicates the SBPF 
version XX or higher in its ELF header e_flags field, according to the 
specification of SIMD-0161.

### New syscall instruction

We introduce a new instruction in the SBPF instruction set, which we call 
`syscall`. It must be associated with all syscalls in the SBPF format. Its 
encoding consists of an opcode `0x95` and an immediate, which must refer to a 
previously registered syscall. For more reference on the SBF ISA format, see 
the 
[spec document](https://github.com/solana-labs/rbpf/blob/main/doc/bytecode.md).

For simplicity, syscalls must be represented as a natural number greater than 
zero, so that they can be organized in a lookup table. This choice allows for 
quick retrieval of syscall information from integer indexes. An instruction 
`syscall 2` must represent a call to the function registered at position two 
in the lookup table.

Consequently, system calls in the Solana SDK and in any related compiler tools 
must be registered as function pointers, whose address is a natural number 
greater than zero, representing their position in a syscall lookup table. The 
verifier must enforce that the immediate of a syscall instruction points to a 
valid syscall, and throw `VerifierError::InvalidFunction` otherwise.

This new instruction comes together with modifications in the verification 
phase. `call imm` (opcode `0x85`) instructions must only refer to internal 
calls and its immediate field must only be interpreted as a relative address 
to jump from the program counter. 

### Change of opcode for the exit instrcution

The opcode `0x9D` must represent the exit instruction, while the old opcode 
`0x95` must now be assigned to the new syscall instruction.

## Alternatives Considered

None.

## Impact

The changes proposed in this SIMD are transparent to dApp developers. The 
compiler toolchain will emit correct code for the specified SBF version. 
Static syscalls obviate relocations for call instructions and move the virtual 
machine closer to eliminating relocations altogether, which can bring 
considerable performance improvements.

## Security Considerations

None.
