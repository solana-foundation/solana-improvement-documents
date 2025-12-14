---
simd: '0385'
title: Transaction V1 Format
authors:
  - jacobcreech
  - apfitzge
category: Standard
type: Core
status: Review
created: 2025-10-24
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

The current live transaction formats, v0 and legacy, have a number of 
limitations that make them unsuitable for efficient ingestion, processing, and
unable to support developer features that are required for today's applications.

This proposal presents a new transaction format, `v1`, that is designed to 
get rid of the need for compute budget instructions, address lookup tables, 
and more to speed up transaction ingestion and processing.

## Motivation

The current live transaction formats, v0 and legacy, have a number of 
inefficiencies and limitations that adversely impact the performance of the 
cluster. Ingestion is slowed down when trying to prioritize transactions by 
their Compute Budget instructions. Address lookup tables currently supported by 
transaction format v0 are also a significant source of complexity for 
validators to support.

By introducing a new transaction format, `v1`, we can address these
issues and improve the performance of transactions ingestion, processing, and 
save developers space in their transaction by including compute budget 
information in the transaction header itself.

## New Terminology

- 'v1 transaction' - A new transaction format that is designed to enable larger
  transactions sizes while not having the address lookup table features 
  introduced in v0 transactions. The v1 transaction format also does not require
  compute budget instructions to be present within the transaction, but instead
  the TransactionConfigMask detailed below should be used.

## Detailed Design

A new transaction format, `v1`, is proposed to enable larger transaction
sizes.

### Transaction V1 Specification

```
VersionByte (u8)
LegacyHeader (u8, u8, u8) 
NumInstructions (u8)
TransactionConfigMask (u32) -- Bitmask of which config requests are present.
LifetimeSpecifier [u8; 32]
NumAddresses (u8)
Addresses [[u8; 32]] -- Length matches NumAddresses
ConfigValues [[u8; 4]] -- Length equal to the popcount (number of set bits)
  of TransactionConfigMask. See section TransactionConfigMask for details.
InstructionHeaders [(u8, u8, u16)] -- Length of NumInstructions. Values are
  (ProgramAccountIndex, NumInstructionAccounts, NumInstructionDataBytes)
InstructionPayloads [InstructionPayload] -- Length = NumInstructions.
  Each InstructionPayload is the concatenation of the following byte arrays:
    InstructionAccountIndexes [u8] -- Length = NumInstructionAccounts from the
    corresponding InstructionHeader
    InstructionData [u8] -- Length = NumInstructionDataBytes from the
    corresponding InstructionHeader
Signatures [[u8; 64]]
```

#### VersionByte

The VersionByte MUST be set to `129` to distinguish v1 transactions from
legacy/v0 formats.

#### LegacyHeader

The LegacyHeader is unchanged from prior transaction formats.
For reference, it consists of three `u8` values with no padding. The values
(in order) are:

- `num_required_signatures`: The number of requires signatures
- `num_readonly_signed_accounts`: The number of the accounts with
    signatures that are loaded as read-only
- `num_readonly_unsigned_accounts`: The number of accounts without
    signatures that are loaded as read-only

As in prior formats, the transaction fails sanitization if
`num_readonly_signed_accounts >= num_required_signatures`. Note that
`num_readonly_signed_accounts == num_required_signatures` is a sanitization
failure because it would imply the fee payer account is readonly. Note that
this also implies `num_required_signatures >= 1`.

#### NumInstructions

NumInstructions is set to the number of instructions in the transaction. As
explained in the Transaction Constraints section, the transaction fails
sanitization if `NumInstructions > 64`.

#### TransactionConfigMask

TransactionConfigMask is explained below in detail.

#### LifetimeSpecifier

LifetimeSpecifier is the same as the Recent Blockhash field in prior
transaction formats. It has been renamed to clarify its use without changing
its meaning.

#### NumAddresses

NumAddresses is set to the number of account addresses the transaction
references. The transaction fails sanitization if `num_addresses <
num_required_signatures + num_readonly_unsigned_accounts`. The transaction
also fails sanitization if `num_addresses > 64`, as explained in the
Transaction Constraints section.

#### Addresses

Addresses is an array of all the account addresses that the transaction
references. It is a sanitization failure for this array to contain any
duplicates. This list has `NumAddresses` elements. The ordering of the
addresses is unchanged from prior transaction formats.
For reference, they are:

- `num_required_signatures-num_readonly_signed_accounts` additional addresses
  for which the transaction contains signatures and are loaded as writable,
  of which the first is the fee payer
- `num_readonly_signed_accounts` addresses for which the transaction contains
  signatures and are loaded as readonly
- `num_addresses-num_required_signatures-num_readonly_unsigned_accounts`
  addresses for which the transaction does not contain signatures and are
  loaded as writable
- `num_readonly_unsigned_accounts` addresses for which the transaction does
  not contain signatures and are loaded as readonly

#### ConfigValues

ConfigValues is explained below with TransactionConfigMask in detail.

#### InstructionHeaders

InstructionHeaders is an array with NumInstructions elements. Each element
consists of three fields with no padding:

- `ProgramAccountIndex: u8`: the index in the array of addresses of the
  address of the program to invoke for this instruction. This field was known
  as `program_id_index` in previous versions of the transaction format; it
  has been renamed to clarify its use without changing its meaning.
- `NumInstructionAccounts: u8`: the number of addresses that will be passed
  to the program when it is invoked. This field was implicitly represented in
  a different encoding in a previous versions of the transaction format as
  the element count of the `accounts` vector.
- `NumInstructionDataBytes: u16`: the size in bytes of the data that will be
  passed as input to the program when it is invoked. This field was
  implicitly represented in a different encoding in a previous versions of
  the transaction format as the element count of the `data` vector.

There is also no padding between each 4-byte element.

#### InstructionPayloads

`InstructionPayloads` is a concatenation of byte arrays. For each of the
`NumInstructions` instructions, it consists of `NumInstructionAccounts` bytes
of account indices followed by `NumInstructionDataBytes` of instruction data.
For example, the InstructionPayloads for a two-instruction transaction looks
like:

- instruction 0's InstructionAccountIndices
  (InstructionHeaders[0].NumInstructionAccounts bytes)
- instruction 0's InstructionData
  (InstructionHeaders[0].NumInstructionDataBytes bytes)
- instruction 1's InstructionAccountIndices
  (InstructionHeaders[1].NumInstructionAccounts bytes)
- instruction 1's InstructionData
  (InstructionHeaders[1].NumInstructionDataBytes bytes)

The transaction fails sanitization if any element of
InstructionAccountIndices is greater than or equal to NumAddresses

#### Signatures

Signatures is an array with `num_required_signatures` elements, each
consisting of a 64-byte Ed25519 signature. Specifically, `Signatures[i]` is
the signature resulting from signing the portion of the transaction prior to
the Signatures field with the keypair associated with the public key at
`Addresses[i]`.

The transaction must not contain trailing data after the signatures field.
There are no padding fields.

This new `v1` transaction format notably does not include address lookup
tables.

### Transaction Constraints

Any transaction violating these constraints will be considered invalid and will 
not be included in the chain. Violations are considered sanitization failures:

| value | max specified by this SIMD | prior max | max implied by format |
| --- | --- | --- | --- |
| transaction size | 4096 | 1232 | 4096 |
| signatures per transaction | 12 | 12 | 42 |
| num accounts | 64 | 64 | 96 |
| num instructions | 64 | 64 | 255 |
| accounts/instruction | 255 | 255 | 255 |

### TransactionConfigMask

The transaction config mask is used to configure specific fee and resource 
requests in a transaction.
It is intended to be flexible enough to add new fields in the future, and is 
initially aimed at replacing the ComputeBudgetProgram instructions that are 
currently used to configure transactions.
Each bit in the mask represents 4-bytes in the `ConfigRequests` array.
If a configured value, such as priority-fee, needs more than 4-bytes a field can
use 2 bits in the mask.

Initially supported fields and assigned bits:

- [0, 1] - total lamports for transaction priority-fee. 8-byte LE u64.
- [2] - compute-unit-limit. 4 byte LE u32.
- [3] - requested loaded accounts data size limit. 4 byte LE u32.
- [4] - requested heap size. 4 byte LE u32.

For 2 bit fields, such as priority-fee, both bits MUST be set. If only one of 
the bits is set, the transaction is invalid and cannot be included in blocks.

For TxV1 transactions, any ComputeBudgetProgram instructions are ignored for 
configuration, even if they are invalid.
The instructions will still consume compute-units if included and be processed 
as a successful no-op instruction.

For the cost-model, all TxV1 transactions are treated as if requests are 
present.
For all fields, if the bit(s) are not set, the minimum allowed value is used:

- If bits [0, 1] are not set, the priority-fee is 0.
- If bit 2 is not set, the requested compute-unit-limit is 0.
- If bit 3 is not set the requested accounts data size limit is 0. This is 
  different from the current requested accounts data size limit defaulting today
  to 64MiB (MAX_LOADED_ACCOUNTS_DATA_SIZE_BYTES).
- If bit 4 is not set the requested heap size is 32768 (MIN_HEAP_FRAME_BYTES).

## Alternatives Considered

None at this time.

## Impact

As developers adopt the new transaction format, validator clients accepting 
their transactions will be much more efficient at ingestion. The transaction
format as currently defined may also support other features such as larger
transaction sizes.

Notably the proposed format does not support address lookup tables, which are
commonly used in the developer community today.

## Security Considerations

None at this time.
