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

This SIMD introduces static syscalls, using the eBPF call instruction 
encoding, to remove runtime relocations while keeping compatibility with the 
eBPF encoding.

## Motivation

The resolution of syscalls during ELF loading requires relocating addresses, 
which is a performance burden for the validator. Relocations require an entire 
copy of the ELF file in memory to either relocate addresses we fetch from the 
symbol table or offset addresses to after the start of the virtual machine's 
memory. Moreover, relocations pose security concerns, as they allow the 
arbitrary modification of program headers and programs sections. Introducing 
static syscalls allows us to resolve all program relocations during link time.

## New Terminology

None.

## Detailed Design

The following must go into effect if and only if a program indicates the SBPF 
version `0x03` or higher in its ELF header e_flags field, according to the 
specification of SIMD-0161.

### Statuc syscall instruction


We follow the encoding referenced in the 
[eBPF specification](https://www.ietf.org/archive/id/draft-thaler-bpf-isa-00.html#name-jump-instructions)
for encoding static syscalls. That means we must use the call instruction 
(opcode `0x85`) with the source register field set to zero. The immediate field 
must be filled with a registered syscall hash code. For more reference on the 
SBF ISA format, see the 
[spec document](https://github.com/solana-labs/rbpf/blob/main/doc/bytecode.md).

We define the hash code for a syscall as the murmur32 hash of its respective 
name. The 32-bit immediate value of the `call` instruction must be the 
integer representation of such a hash. For instance, the code for `abort` is 
given by `murmur32("abort")`, so the instruction assembly should look like 
`call 3069975057`.

Consequently, system calls in the Solana SDK and in any related compiler tools 
must be registered as function pointers, whose address is the murmur32 hash of 
their name. The bytecode verifier must enforce that the immediate value of a 
syscall instruction points to a valid syscall, and throw 
`VerifierError::InvalidSyscall` otherwise.

This new instruction comes together with modifications in the semantics of 
`call imm` (opcode `0x85` with source register set to one) instructions, which 
must only refer to internal calls and their immediate field must only be 
interpreted as a relative address to jump from the program counter.

Syscall names must NOT be present in the symbol table anymore, since the new 
scheme does not require symbol relocations and obviates the need for symbols 
to be referenced in the table.

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
