---
simd: '0683'
title: Pass ABIv1 input metadata via registers r3–r5
authors:
  - febo (Anza)
category: Standard
type: Core
status: Review
created: 2026-10-04
feature: (fill in with feature key and GitHub tracking issues once accepted)
---

## Summary

Provide additional input metadata in VM registers 3 (`r3`), 4 (`r4`) and 5
(`r5`), allowing programs to reconstruct the instruction data and accounts
slices without calculating offsets or loading their lengths.

## Motivation

Once [SIMD-0449] is activated, a slice of account pointers will be serialized
into the program input. This eliminates the need to parse the accounts section,
but programs must still calculate the offset from the instruction data pointer
to the start of the accounts slice.

Providing additional metadata representing the instruction data length, accounts
slice length, and accounts slice offset in VM registers 3, 4, and 5 will allow
programs to reconstruct the instruction data and accounts slices without
calculating offsets or loading their lengths.

## Dependencies

This proposal depends on the following previously accepted proposals:

- **[SIMD-0449]: Direct Account Pointers in Program Input**

    Defines the account pointers slice in the program input region that this proposal
    references in the new register assignments.

[SIMD-0449]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0449-direct-account-pointers-in-program-input.md

## New Terminology

N/A

## Detailed Design

The ABIv1 program entrypoint has 5 registers available. The first two registers
(`r1` and `r2`) are already used for the input region pointer and the pointer to
the instruction data section, respectively. After this proposal, all VM registers
will provide the following information:

- `r1`: Input region pointer (existing behavior)
- `r2`: Pointer to instruction data section (existing behavior)
- `r3`: instruction data length **(new)**
- `r4`: pointer to the accounts slice **(new)**
- `r5`: number of accounts **(new)**

The new register assignments are available to all programs, under all loaders,
regardless of whether or not the value is read. These registers currently contain
uninitialized data at the program entrypoint.

**New register details:**

- The value in `r3` is a little-endian 64-bit unsigned integer (8 bytes)
  representing the length of the instruction data section in bytes.
- The pointer value in `r4` is the address of the start of the accounts slice in
  the input region. The address is stored as a little-endian 64-bit pointer (8
  bytes).
- The value in `r5` is a little-endian 64-bit unsigned integer (8 bytes)
  representing the number of accounts in the accounts slice, including duplicate
  accounts. This value is equivalent to the number of accounts in the serialized
  input region at the start of the input region (`r1`).

### Validator Components Affected

Which validator components are affected by this change?

| Validator Component             | Impact                              |
|---------------------------------|-------------------------------------|
| Transaction Execution (Runtime) |                                     |
| Virtual Machine                 | New register assignments            |
| Block Packing                   |                                     |
| Consensus                       |                                     |
| Gossip                          |                                     |
| Turbine                         |                                     |
| Snapshots                       |                                     |
| On-Chain Core BPF Programs      |                                     |
| Other (please describe)         |                                     |

## Alternatives Considered

1. **Do nothing**: Leave the current implementation as is, which means that
   programs will need to manually parse the input region to extract the
   instruction
   data length, accounts slice pointer, and number of accounts. This approach is
   less efficient and, considering the effort required to implement this change,
   it does not seem reasonable to leave the current implementation as is.

2. **Modify serialization layout**: Change the serialization layout to place the
   instruction data length, accounts slice pointer, and number of accounts at
   the start of the input region. This would allow programs to read these values
   directly from the input region. However, this approach would require a breaking
   change to the serialization format, which is impractical.

Additionally, ABIv2 will change the serialization layout and simplify the input
region, so the optimization proposed provides an immediate benefit and does not
conflict with the future ABIv2 changes.

## Impact

On-chain programs will be able to access the instruction data length, accounts
slice pointer, and number of accounts directly from registers `r3`, `r4`, and
`r5`,
respectively. This will simplify program logic and improve performance by
eliminating
the need to parse the input region for this information.

## Security Considerations

Programs should not currently rely on the values in `r3–r5`, which are
uninitialized
at the program entrypoint. An analysis using
[program-sync](https://github.com/blueshift-gg/program-sync)
(with PR [#11](https://github.com/blueshift-gg/program-sync/pull/11)) flagged
three programs:
`4rsQkxtHoEHTzvwoUNt6FTCZjygNemEPZqz9Xeucg3fh` uses uninitialized values in
`r3`, `r4` and `r5`, but
always fails at the fourth instruction of the program, so it is not an
executable program in practice;
`GdY4puYdZiEfk8HCX7L9kJfzzjc3xSxXrVjKv2vphfDn` and
`fastC7gqs2WUXgcyNna2BZAe9mte4zcTGprv3mv18N3` are
false positives, since their entrypoints contain a `callx` and the analyzer
assumes that the callee
consumes all registers, but the callee never touches uninitialized registers.

## Backwards Compatibility

This feature is only backwards compatible for programs that currently do not
read from `r3`, `r4`, or `r5` at the program entrypoint.
