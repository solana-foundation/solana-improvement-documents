---
simd: '0189'
title: SBPF stricter ELF headers
authors:
  - Alexander Meißner
category: Standard
type: Core
status: Idea
created: 2024-10-21
feature: TBD
extends: SIMD-0178, SIMD-0179
---

## Summary

Imposes more restrictions on what is expected of ELF headers.

## Motivation

After the removal of relocations in SIMD-0178 the ELF layout could be massively
be simplified by constraining it to a strict subset of what ELF otherwise
allows. Doing so not only reduces the complexity of validator implementations
but also reduces the attack surface.

## Alternatives Considered

Moving away from ELF as a container format altogether. However this would only
gain a very small file size advantage but otherwise loose all tooling
compatibility.

## New Terminology

None.

## Detailed Design

### File header

The file size must not be less than `size_of::<Elf64Ehdr>()` (64 bytes),
otherwise `ElfParserError::OutOfBounds` must be thrown.

- `e_ident.ei_mag` must be `[0x7F, 0x45, 0x4C, 0x46]`
- `e_ident.ei_class` must be `ELFCLASS64` (`0x02`)
- `e_ident.ei_data` must be `ELFDATA2LSB` (`0x01`)
- `e_ident.ei_version` must be `EV_CURRENT` (`0x01`)
- `e_ident.ei_osabi` must be `ELFOSABI_NONE` (`0x00`)
- `e_ident.ei_abiversion` must be `0x00`
- `e_ident.ei_pad` must be `[0x00; 7]`
- `e_type` must be `ET_DYN` (`0x0003`)
- `e_machine` must be `EM_SBPF` (`0x0263`)
- `e_version` must be `EV_CURRENT` (`0x00000001`)
- `e_entry` must point to a function start marker in the text section,
see SIMD-0179 for details on how a function start is marked.
- `e_phoff` must be `size_of::<Elf64Ehdr>()` (64 bytes)
- `e_shoff` is not checked
- `e_flags` see SIMD-0161
- `e_ehsize` must be `size_of::<Elf64Ehdr>()` (64 bytes)
- `e_phnum` must not be less than `0x0005`
- `e_phoff + e_phnum * size_of::<Elf64Phdr>()` must be less than the file size
- `e_phentsize` must be `size_of::<Elf64Phdr>()` (56 bytes)
- `e_shnum` is not checked
- `e_shentsize` must be `size_of::<Elf64Shdr>()` (64 bytes)
- `e_shstrndx` must be less than `e_shnum`

If any check fails `ElfParserError::InvalidFileHeader` must be thrown.

### Program headers

|  purpose  |    p_type    |   p_flags  | p_vaddr |
| --------- | ------------ | ---------- | ------- |
| bytecode  | PT_LOAD      | PF_X       | 0 << 32 |
| ro data   | PT_LOAD      | PF_R       | 1 << 32 |
| stack     | PT_GNU_STACK | PF_R, PF_W | 2 << 32 |
| heap      | PT_LOAD      | PF_R, PF_W | 3 << 32 |

For each of these predefined program headers:

- `p_type` must match the `p_type` of the entry in the table above
- `p_flags` must match the `p_flags` of the entry in the table above
- `p_offset` must not be less than `e_phoff + e_phnum * size_of::<Elf64Phdr>()`
- `p_offset` must be less than `file.len() as u64`
- `p_offset` must be  evenly divisible by 8 bytes,
- `p_vaddr` must match the `p_vaddr` of the entry in the table above
- `p_paddr` must match the `p_vaddr` of the entry in the table above
- `p_filesz` must be:
  - `0` if the section is writable (the `PF_W` flag is set)
  - `p_memsz` otherwise (the `PF_W` flag is clear)
- `p_filesz` must not be greater than `file.len() as u64 - p_offset`
- `p_memsz` must fit in 32 bits / be less than `1 << 32`
- `p_align` is ignored

If any check fails `ElfParserError::InvalidProgramHeader` must be thrown.

## Impact

The toolchain linker will use a new linker script to adhere to these
restrictions defined here and thus the change will be transparent to the dApp
developers.

The section headers are ignored so arbitrary metadata can continue to be
encoded there.

## Security Considerations

None.
