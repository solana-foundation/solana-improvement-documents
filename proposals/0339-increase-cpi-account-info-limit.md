---
simd: '0339'
title: Increase CPI Account Info Limit
authors:
  - Justin Starry (Anza)
category: Standard
type: Core
status: Review
created: 2025-08-15
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Increase the maximum account info length for cross-program invoked (CPI)
instructions from 64 to 255 and consume compute units for serialized account
info's and instruction account meta's.

## Motivation

CPI's are restricted to a limit of 64 account info's passed to the syscall.
This limit is burdensome for onchain programs which themselves were invoked with
more than 64 accounts because it means they cannot simply pass through the same
list of account info's that they were invoked with to another CPI syscall. They
are faced with the burden of first deduplicating the account info's and
constructing a new list before making the syscall.

This problem arises when onchain programs wrap another program (such as Jupiter)
that composes many other programs and are invoked with over 64 accounts
(including duplicates).

Along with the increasing the account info limit, start charging for both
serialized account info's and instruction account meta's to avoid abuse of
resources in CPI calls.

## New Terminology

- **Account Info:** The serialized information for each account referenced in a
CPI instruction used to read account info from the caller program.

## Detailed Design

### Maximum Account Info Length

Increase the maximum account info length imposed on CPI syscalls from **64** to
**255**. The maximum is inclusive, meaning that a list of account info's with a
length of 255 is valid.

### Account Info Cost

Consume **1 compute unit (CU)** for every `cpi_bytes_per_unit` (currently 250)
bytes of account info.

Fixed size of **80 bytes** for each `account_info` broken down as

  - 32 bytes for account address
  - 32 bytes for owner address
  - 8 bytes for lamport balance
  - 8 bytes for data length

The total cost of account info's can be computed with
`(account_info_size * account_infos_len) / cpi_bytes_per_unit` rounded down to
the nearest CU.

### Instruction Account Metadata Cost

Consume **1 compute unit (CU)** for every `cpi_bytes_per_unit` (currently 250)
bytes of instruction account metadata.

Fixed size of **34 bytes** for each `ix_account_metadata` broken down as

  - 32 bytes for account address
  - 1 byte for signer flag
  - 1 byte for writable flag

The total cost of instruction account metadata can be computed with
`(ix_account_metadata_size * ix.accounts.len()) / cpi_bytes_per_unit` rounded
down to the nearest CU.

## Alternatives Considered

What alternative designs were considered and what pros/cons does this feature
have relative to them?

## Impact

Since the list of account info's passed to a CPI can now be ~4 times longer,
there will be more overhead in the SVM to map each instruction account address
to one of the account info's and translate account info's to callee instruction
accounts.

CPI's will consume additional compute units proportional to the number of
account info's and instruction accounts.

## Security Considerations

NA

## Drawbacks *(Optional)*

Why should we not do this?

## Backwards Compatibility *(Optional)*

The max account info length increase for CPI's and new costs will be feature
gated. Since the limit is being increased, existing behavior will not be
restricted. However, new imposed costs for CPI instruction accounts and account
info's may cause onchain programs to consume additional CU's.
