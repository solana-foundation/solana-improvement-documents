---
simd: '0178'
title: SBPF Static Syscalls
authors:
  - Alessandro Decina
  - Alexander Meißner
  - Lucas Steuernagel
category: Standard
type: Core
status: Review
created: 2024-09-27
feature: BUwGLeF3Lxyfv1J1wY8biFHBB2hrk2QhbNftQf3VV3cC
---

## Summary

This SIMD introduces a new instruction syscall in the SBPF instruction set to 
represent syscalls. Such a change aims to remove relocations when resolving 
syscalls and simplify the instruction set, allowing for the straightforward 
differentiation between external and internal calls. In addition, it proposes 
a new `return` instruction to supersede the `exit` instruction.

## Motivation

The resolution of syscalls during ELF loading requires relocating addresses, 
which is a performance burden for the validator. Relocations require an entire 
copy of the ELF file in memory to either relocate addresses we fetch from the 
symbol table or offset addresses to after the start of the virtual machine's 
memory. Moreover, relocations pose security concerns, as they allow the 
arbitrary modification of program headers and programs sections. A new 
separate opcode for syscalls modifies the behavior of the ELF loader, allowing 
us to resolve syscalls without relocations.

## New Terminology

None.

## Detailed Design

The following must go into effect if and only if a program indicates the SBPF 
version `0x03` or higher in its ELF header e_flags field, according to the 
specification of SIMD-0161.

### New syscall instruction

We introduce a new instruction in the SBPF instruction set, which we call 
`syscall`. It must be associated with all syscalls in the SBPF format. Its 
encoding consists of an opcode `0x95` and an immediate, which must refer to a 
previously registered syscall hash code. For more reference on the SBF ISA 
format, see the 
[spec document](https://github.com/solana-labs/rbpf/blob/main/doc/bytecode.md).

We define the hash code for a syscall as the murmur32 hash of its respective 
name. The 32-bit immediate value of the new `syscall` instruction must be the 
integer representation of such a hash. For instance, the code for `abort` is 
given by `murmur32("abort")`, so the instruction assembly should look like 
`syscall 3069975057`.

Consequently, system calls in the Solana SDK and in any related compiler tools 
must be registered as function pointers, whose address is the murmur32 hash of 
their name. The bytecode verifier must enforce that the immediate value of a 
syscall instruction points to a valid syscall, and throw 
`VerifierError::InvalidSyscall` otherwise.

This new instruction comes together with modifications in the semantics of 
`call imm` (opcode `0x85`) instructions, which must only refer to internal 
calls and their immediate field must only be interpreted as a relative address 
to jump from the program counter.

Syscall names must NOT be present in the symbol table anymore, since the new 
scheme does not require symbol relocations and obviates the need for symbols 
to be referenced in the table.

### New return instruction

The opcode `0x9D` must represent the return instruction, which supersedes the 
`exit` instruction. The opcode (opcode `0x95`), previously assigned to the 
`exit` instruction, must now be interpreted as the new syscall instruction.

The verifier must detect programs whose version is less than V3 containing
the `0x9D` opcode and throw a `VerifierError::UnknowOpCode`. Likewise, if, by 
any means, a V2 or earlier version program reaches the execution stage 
containing the `0x9D` opcode, an `EbpfError::UnsupportedInstruction` must be 
raised.

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
