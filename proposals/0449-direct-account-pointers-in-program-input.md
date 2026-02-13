---
simd: "0449"
title: Direct Account Pointers in Program Input
authors:
  - febo (Anza)
category: Standard
type: Core
status: Review
created: 2026-01-24
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Serialize pointers for instruction accounts to allow direct account access
without requiring parsing of the account section.

## Motivation

Currently, sBPF programs in ABIv1 must parse the accounts section of the
serialized input region to extract account boundary information in order 
to provide the program with a slice of accounts. This parsing represents 
most of the entrypoint's cost.

Since account boundaries are already known to the VM when preparing the
program input, they can instead be serialized directly into the input. This
allows the entrypoint to derive the accounts slice without iterating over
or parsing the accounts section.

As a result, the entrypoint consumes a constant number of compute units,
independent of the number of accounts.

## New Terminology

- **accounts slice pointer**: A slice of 64-bit (8-byte) pointers, with
  one pointer per instruction account, stored at the end of the program
  input parameters.

## Detailed Design

When the feature is activated, the VM serializes the slice of account
pointers after the program ID in the program input parameters for any
ABIv1 program. All other parameters remain unchanged:

```
- 8 bytes: number of accounts (little-endian)
- <variable>: accounts section
- 8 bytes: length of instruction data (little-endian)
- <variable>: instruction data bytes
- 32 bytes: program ID
- 0-7 bytes: padding bytes to align offset to 8-bytes
- [u64; <number of accounts>]: slice of account pointers
```

Each account pointer is the address (`u64` little-endian) of the first byte
of the account information. They all must be valid addresses within the
input section. Duplicated accounts will have the same pointer value, i.e.,
they will point to the same account. These are the same addresses that
entrypoints previously had to compute when parsing the accounts section,
but it is now provided directly as an input parameter. 

The slice of account pointers is made available to all ABIv1 programs, under
all loaders, regardless of whether it is read or not. Since this data is
appended after the standard input parameters, existing programs are unaffected.

Given that, after [SIMD-0321](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0321-vm-r2-instruction-data-pointer.md)
is activated, programs will receive a
direct pointer to the instruction data, programs can derive the location
of the slice of account pointers without parsing the accounts section:

- read the number of accounts from `r1`.
- read the length of the instruction data from `r2 - 8`.
- pass the following three references to the program entrypoint:
  1. `program_id`: a `&[u8]` slice created from
    `r2 + <length of the instruction data>` with length equal
    to `32 bytes`.
  2. `accounts`: a `&[AccountView]` slice created from
    `r2 + <length of the instruction data> + 32 + padding` with length
    equal to `<number of accounts>`.
  3. `instruction_data`: a `&[u8]` slice created from `r2` with
    length equal to `<length of instruction data>`.

## Alternatives Considered

While ABIv2 proposes a new serialization layout that provides a similar
slice of accounts and likewise avoids parsing the accounts section,
the approach proposed in this SIMD is comparatively simpler and fully
backward compatible with current ABIv1.

## Impact

On-chain programs are positively impacted by this change. The program
entrypoint complexity is significantly reduced, along with the number of
compute units consumed. The implementation is relatively simple, as it
relies on information that is already available during program input
serialization.

- Current entrypoint (pinocchio):

| Name         | CUs | Delta |
|--------------|-----|-------|
| Account (1)  | 17  |  --   |
| Account (2)  | 17  |  --   |
| Account (3)  | 37  |  --   |
| Account (4)  | 45  |  --   |
| Account (8)  | 78  |  --   |
| Account (16) | 143 |  --   |
| Account (32) | 261 |  --   |
| Account (64) | 504 |  --   |

- Estimated entrypoint (after the changes proposed):

| Name         | CUs | Delta |
|--------------|-----|-------|
| Account (1)  | 11  |  -6   |
| Account (2)  | 11  |  -6   |
| Account (3)  | 11  |  -26  |
| Account (4)  | 11  |  -34  |
| Account (8)  | 11  |  -67  |
| Account (16) | 11  |  -132 |
| Account (32) | 11  |  -250 |
| Account (64) | 11  |  -493 |

The benchmark above represents the cost of parsing accounts in the
entrypoint of a program with an empty instruction processor. The
scaffold code for it can be found [here](https://github.com/febo/playground).

## Security Considerations

Since programs currently do not read data from the input parameters
beyond the program ID, this change does not introduce any security
concerns.

## Backwards Compatibility

This feature is fully backwards compatible with current ABIv1 since no program
entrypoint reads data from the input parameters after the program ID.
