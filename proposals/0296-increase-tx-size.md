---
simd: '0296'
title: Increase Transaction Size
authors:
  - jacobcreech
  - apfitzge
category: Standard
type: Core
status: Review
created: 2025-05-28
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Historically, messages transmitted over the network must not exceed the IPv6 MTU
size to ensure a fast and reliable network delivery. Solana has used a 
conservative MTU size of 1280 bytes, which after accounting for some of the
overhead, leaves a maximum transaction size of 1232 bytes for data in the
payload.

With QUIC, we can now use a larger transaction size than the original 1280
bytes. This proposal presents a new transaction size starting at 4096 bytes,
and the new v1 transaction format required to support the larger size. v1 
transactions are a direct upgrade from the legacy format, while proposing 
a removal of Address Lookup Tables compared to v0 transactions.

## Motivation

The current transaction size limit of 1232 bytes is too small for many
developer use cases. While there have been some attempts to artificially
increase the size capacity with address lookup tables, the current cap still is
artificially limiting the number of signatures and size of other data.

Use cases that are currently limited by the transaction size include:

- ZK proofs such as what is used within [Confidential
 Balances](https://www.solana-program.com/docs/confidential-balances/zkps)
- Untruncated Winternitz one-time signatures
- Nested Multisig used by corporations under Squads
- Onchain cryptography signature schemes without precompiles such as BLS

and many more. A larger transaction size would also help remove the need for
address lookup tables, which have been a source of complexity on the validator
client as well as a frictional developer experience for many developers.

A number of developers have also resorted to using Jito bundles to artificially
increase the size of their transactions. This is a current workaround that
developers are using, but it does not have the same guarantees as a transaction
does on the protocol level, namely that bundles are not guaranteed to be atomic
like singular transactions.

For these reasons, increasing the current transaction size limit would enable
developers to create new applications and use cases that were previously
unfeasible.

## New Terminology

- 'v1 transaction' - A new transaction format that is designed to enable larger
  transactions sizes while not having the address lookup table features 
  introduced in v0 transactions.

## Detailed Design

Increasing the transaction size limit above the current MTU max size is
possible with the use of QUIC. QUIC's RFC 9000 specification does not have
an explicit maximum stream size, allowing for larger transactions to be
sent.

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

-   `num_required_signatures`: The number of requires signatures
-   `num_readonly_signed_accounts`: The number of the accounts with
    signatures that are loaded as read-only
-   `num_readonly_unsigned_accounts`: The number of accounts without
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

Any section with 0 addresses is skipped.

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
- If bit 3 is not set the requested accounts data size limit is 0.
- If bit 4 is not set the requested heap size is 32768 (MIN_HEAP_FRAME_BYTES).

## Alternatives Considered

Alternative designs considered:

- Having a transaction loading feature that would allow developers to load
the transaction parts in a buffer and then be able to execute them at the
end.
This method is no longer considered as it requires a much higher level of
latency on the application level and a decent amount of complexity within
the validator
- Bundles at the protocol level. This would not solve all problems that are
 solved by larger transaction sizes. Bundles would still limit the ability for
 developers to use cryptographic signature schemes that have large proof sizes.

## Impact

Developers would have to update with applications to use the new transaction
format to take advantage of the larger transaction size. Those developers that
had previously been using address lookup tables would be required to update the
new transactions with the full address list instead of the address lookup table
and its indices.

In consideration of what size the new transaction size limit should increase
to, [Jito's bundle

sizes](https://jitolabs.grafana.net/dashboard/snapshot/ISnwjbxw02UBLrj1xy4n4dFkAPyZ46ll?orgId=0&from=2025-05-30T18:45:00.000Z&to=2025-05-30T21:45:00.000Z&timezone=utc&var-cluster=mainnet&var-region=$__all&viewPanel=panel-40&inspect=panel-40&inspectTab=data)
that were used to increase the overall transaction size should be considered.
The following is the rough distribution of the transaction sizes:

- 2048 bytes or lower - 50% of bundles
- 6144 bytes or lower = 65% of all bundles
- 9216 bytes or lower = 100% of all bundles

In consideration of the above, the use cases mentioned within the motivation
section, and the current page size for managing memory within a validator, a
new transaction size limit of 4096 bytes is proposed. This new limit should
cover a majority of use cases mentioned within the motivation section, as well
as enable most of the developers using Jito bundles for larger transactions to
migrate to the new transaction format. Similarly, since the new limit proposed
can accommodate the max accounts used in ALTs directly in the transaction,
developers are able to migrate from `v0` transactions to `v1` transactions.

A number of changes are required to be made to the validator to support the new
transaction size limit. Consensus would need to be updated to support the
larger transaction size, as larger transactions included in a block would be
marked as invalid by the cluster today. The scheduler would also need to be
modified to  support the larger transaction sizes. While this proposal does not
introduce a new fee structure around bytes in a transaction, the scheduler
should prioritize larger transactions differently, requiring developers to pay
a higher priority fee to land their transaction.

Testing for this new transaction size limit should be done extensively to
ensure that performance on the cluster is not adversely affected. This testing
should include the different use cases mentioned within the motivation section,
as well as the different sizes of transactions that are currently being used by
developers on the network.

## Security Considerations

Larger transaction sizes could results in a number of bandwidth issues on the 
cluster than need to be tested thoroughly to ensure that performance is not
adversely affected.

## Drawbacks 

There is an argument to be made that an increase in the transaction size limit
above the max MTU packet size would incur some network overhead due to
fragmentation retransmits. For a validator, this could mean maintaining
in-memory buffers for larger transactions compared to just receiving and
processing a single UDP packet. These issues will need to be addressed in the
scheduler and how priority fees for larger transactions are handled.

## Backwards Compatibility

As all of these changes are implemented with a new transaction format, the
increase on the transaction size limit does not affect or break `v0` or `legacy`
transactions.
