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

### New return instruction

The opcode `0x9D` must represent the return instruction, which supersedes the 
`exit` instruction. The opcode (opcode `0x95`), previously assigned to the 
`exit` instruction, must now be interpreted as the new syscall instruction.

The verifier must detect an SBPF V1 program containing the `0x9D` opcode and 
throw a `VerifierError::UnknowOpCode`. Likewise, if, by any means, a V1 
program reaches the execution stage containing the `0x9D` opcode, an 
`EbpfError::UnsupportedInstruction` must be raised.

### Syscall numbering convention

Syscalls must be represented by a unique integer to maintain a dense lookup 
table data structure for indexing and dispatch. For a clear correlation 
between the existing syscalls and their respective identification number, 
syscalls must strictly follow the numbering below.

|           Syscall name                   |  Number  |
|------------------------------------------|----------|
|   abort                                  |    1     |
|   sol_panic_                             |    2     |
|   sol_memcpy_                            |    3     |
|   sol_memmove_                           |    4     |
|   sol_memset_                            |    5     |
|   sol_memcmp_                            |    6     |
|   sol_log                                |    7     |
|   sol_log_64                             |    8     |
|   sol_log_pubkey                         |    9     |
|   sol_log_compute_units_                 |    10    |
|   sol_alloc_free_                        |    11    |
|   sol_invoke_signed_c                    |    12    |
|   sol_invoke_signed_rust                 |    13    |
|   sol_set_return_data                    |    14    |
|   sol_get_return_data                    |    15    |
|   sol_log_data                           |    16    |
|   sol_sha256                             |    17    |
|   sol_keccak256                          |    18    |
|   sol_secp256k1_recover                  |    19    |
|   sol_blake3                             |    20    |
|   sol_poseidon                           |    21    |
|   sol_get_processed_sibling_instruction  |    22    |
|   sol_get_stack_height                   |    23    |
|   sol_curve_validate_point               |    24    |
|   sol_curve_group_op                     |    25    |
|   sol_curve_multiscalar_mul              |    26    |
|   sol_curve_pairing_map                  |    27    |
|   sol_alt_bn128_group_op                 |    28    |
|   sol_alt_bn128_compression              |    29    |
|   sol_big_mod_exp                        |    30    |
|   sol_remaining_compute_units            |    31    |
|   sol_create_program_address             |    32    |
|   sol_try_find_program_address           |    33    |
|   sol_get_sysvar                         |    34    |
|   sol_get_epoch_stake                    |    35    |
|   sol_get_clock_sysvar                   |    36    |
|   sol_get_epoch_schedule_sysvar          |    37    |
|   sol_get_last_restart_slot              |    38    |
|   sol_get_epoch_rewards_slot             |    39    |
|   sol_get_fees_sysvar                    |    40    |
|   sol_get_rent_sysvar                    |    41    |
|------------------------------------------|----------|

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
