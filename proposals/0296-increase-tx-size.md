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
possible with the use of QUIC. QUIC's RFC 9000 specification does not have an
explicit maximum stream size, allowing for larger transactions to be sent.

A new transaction format, `v1`, is proposed to enable larger transaction sizes.
The `v1` transaction format would be:

```
VersionByte (u8) - =129 to distinguish from legacy/v0 formats
LegacyHeader (u8, u8, u8) -- Required signatures from the current
`MessageHeader` type
NumInstructions (u8)
TransactionConfigMask (u32) -- Bitmask of which config requests are present.
LifetimeSpecifier [u8; 32]
NumAddresses (u8)
Addresses [[u8; 32]] -- Length matches NumAddresses
ConfigValues [[u8; 4]] -- Length equal to the popcount (number of set bits) of
  TransactionConfigMask. See section TransactionConfigMask for details.
InstructionHeaders [(u8, u8, u16)] -- Length of NumInstructions. Values are 
  (ProgramAccountIndex, NumInstructionAccounts, NumInstructionDataBytes)
InstructionPayloads [InstructionPayload] -- Length = NumInstructions.
  Each InstructionPayload is the concatenation of the following byte arrays:
    InstructionAccountIndexes [u8] -- Length = NumInstructionAccounts from the 
    corresponding InstructionHeader
    InstructionData [u8] -- Length = NumInstructionDataBytes from the 
    corresponding InstructionHeader
Signatures [[u8; 64]] -- Length of `num_required_signatures` from
 `LegacyHeader`
```

This new `v1` transaction format notably does not include address lookup
tables.

### Transaction Constraints

Any transaction violating these constraints will be considered invalid and will 
not be included in the chain. Violations are considered sanitization failures:

| max | value | new/existing |
| --- | --- | --- |
| max transaction size | 4096 | new |
| max num signatures per transaction | 42 | new |
| max num accounts | 96 | new |
| max num instructions | 64 | old |
| max accounts/instruction | 255 | new |

### Runtime Contraints

While the above lists sanitization failures, we will have the following runtime
contraints either stay unchanged by making sure they are explicitly defined:

| max | value | new/existing |
| --- | --- | --- |
| max num signatures per transaction | 12 | new |
| max num accounts | 64 | old |
| max accounts/instruction | 255 | old |

The runtime check for max number of signatures should be run immediately after
fee payer validations and before other runtime checks.

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

- [0, 1] - priority-fee. 8-byte LE u64.
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
