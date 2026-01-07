---
simd: '0189'
title: SBPF stricter ELF headers
authors:
  - Alexander Meißner
category: Standard
type: Core
status: Idea
created: 2024-10-21
feature: GJav1vwg2etvSWraPT96QvYuQJswJTJwtcyARrvkhuV9
extends: SIMD-0178
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

The following must go into effect if and only if a program indicates the
SBPF-version v3 or higher in its program header (see SIMD-0161).

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
- `e_type` is not checked
- `e_machine` must be `EM_BPF` (`0x00F7`)
- `e_version` must be `EV_CURRENT` (`0x00000001`)
- `e_entry` must be within the bounds of the second program header
- `e_phoff` must be `size_of::<Elf64Ehdr>()` (64 bytes)
- `e_shoff` is not checked
- `e_flags` see SIMD-0161
- `e_ehsize` must be `size_of::<Elf64Ehdr>()` (64 bytes)
- `e_phnum` must be greater than or equal `0x0001`
- `e_phoff + e_phnum * size_of::<Elf64Phdr>()` must be
  less than or equal the file size
- `e_phentsize` must be `size_of::<Elf64Phdr>()` (56 bytes)
- `e_shnum` is not checked
- `e_shentsize` must be `size_of::<Elf64Shdr>()` (64 bytes)
- `e_shstrndx` must be less than `e_shnum`

If any check fails `ElfParserError::InvalidFileHeader` must be thrown.

### Program headers

| index |  purpose  |   p_flags  | p_vaddr |
| ----- | --------- | ---------- | ------- |
| 0     | ro data   | PF_R       | 0 << 32 |
| 1     | bytecode  | PF_X       | 1 << 32 |

If `p_flags` of the first program header is not `PF_R`, then only the second
program header is expected (effectively skipping the first). For each of these
predefined program headers:

- `p_type` must be `PT_LOAD`
- `p_flags` must match the `p_flags` of the entry in the table above
- `p_offset` be `e_phoff + e_phnum * size_of::<Elf64Phdr>()` for the first
  entry and be `p_offset + p_filesz` of the previous entry for all
  subsequent entries
- `p_offset` must be less than or equal `file.len() as u64`
- `p_offset` must be evenly divisible by 8 bytes,
- `p_vaddr` must match the `p_vaddr` of the entry in the table above
- `p_paddr` must match the `p_vaddr` of the entry in the table above
- `p_filesz` must be `p_memsz`
- `p_filesz` must not be greater than `file.len() as u64 - p_offset`
- `p_filesz` must be evenly divisible by 8 bytes,
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
