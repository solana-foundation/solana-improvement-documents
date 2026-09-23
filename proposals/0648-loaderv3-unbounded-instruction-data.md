---
simd: '0648'
title: Unbound LoaderV3 Instruction Data
authors:
  - Hanako Mumei
category: Standard
type: Core
status: Review
created: 2026-09-21
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

LoaderV3 currently bounds instruction deserialization at 1232 bytes, the
traditional maximum transaction size. This prevents using most of the 4096 bytes
made available by
[Transaction V1](https://github.com/solana-foundation/solana-improvement-documents/pull/385)
when writing to LoaderV3 buffers. Removing this bound would allow program
upgrades to use a fraction of the transactions now required.

We remove this bound.

## Motivation

Transaction V1 allows up to 4KiB transactions, and is active on all clusters.
However, the new larger transaction size cannot be fully utilized for writing
LoaderV3 buffers for program deployment and upgrade because LoaderV3 aborts if
instruction data exceeds 1232 bytes.

Due to LoaderV3 `Write` instruction metadata and necessary pubkeys, a legacy
transaction delivers at best 1012 payload bytes, assuming the fee-payer is the
buffer authority. This drops to 960 bytes per transaction when compute price and
limit instructions are included. A V1 transaction can deliver 1216 payload
bytes, a modest improvement over the prior status quo, but still severely
limited compared to what it ought to be.

By removing the artificial bound in LoaderV3, a V1 transaction can deliver 3872
payload bytes per transaction, or 3860 bytes with compute price and limit. This
means a program deployment or upgrade requires ~75% fewer transactions.

This also admits LoaderV3 buffer writes up to almost 10KiB via CPI. It is
possible this may be useful to some consumers such as multisigs, but not a
central goal of this proposal.

## New Terminology

No new terminology, but relevant Agave constants (mirrored in Firedancer):

* `PACKET_DATA_SIZE`: 1232, the current LoaderV3 instruction bound and maximum
size of a legacy or V0 transaction.
* `MAX_TRANSACTION_SIZE`: 4096, the maximum size of a V1 transaction.
* `MAX_INSTRUCTION_DATA_LEN`: 10240, the maximum size of CPI instruction data.

## Detailed Design

When deserializing LoaderV3 instructions, instead of aborting on instruction
data larger than 1232 bytes, deserialize all the data.

As an aside, System and Vote both enforce a 1232 byte limit on instruction
deserialization as well. There is however no reason to change this, since
neither program has any valid instruction longer than low hundreds of bytes.

### Edge Cases

Care should be taken that an incorrect length on the write `Vec` does not over-
allocate before failing validation.

### Validator Components Affected

Which validator components are affected by this change?

| Validator Component             | Impact                              |
|---------------------------------|-------------------------------------|
| Transaction Execution (Runtime) | LoaderV3 ixn deserialization changes. |
| Virtual Machine                 |                                     |
| Block Packing                   |                                     |
| Consensus                       |                                     |
| Gossip                          |                                     |
| Turbine                         |                                     |
| Snapshots                       |                                     |
| On-Chain Core BPF Programs      |                                     |
| Other (please describe)         |                                     |

## Alternatives Considered

We can always do nothing. However, we would fail to take advantage of larger
transactions to improve program deployment and upgrade transaction efficiency.

We could bound the deserialize by `MAX_TRANSACTION_SIZE` to allow full use of V1
transactions. This would retain a consensus-relevant bound on CPI. Ultimately
the difference between 4KiB and 10KiB is not enough to motivate keeping the
restriction.

We could bound by `MAX_INSTRUCTION_DATA_LEN` to effect the same result as having
no bound. In fact, this may be done as an implementation detail. But it is
better to explicitly unbound it in case new transaction formats or other changes
allow larger instruction data and reintroduce a LoaderV3 bound into consensus.

## Impact

Program deployment and upgrade tooling, including the Solana CLI, can use V1
transactions to deploy and upgrade programs much more efficiently. This should
be an opt-in flag for now, since not all external wallet tooling may support
Transaction V1.

## Security Considerations

As a built-in, LoaderV3 pays a fixed CU cost, so if a massive amount of
instruction data was allowed through, it may cause performance concerns. We rely
on the 10KiB CPI limit and the 4KiB transaction size limit to protect against
this.

## Conformance

Verify that LoaderV3 can successfully deserialize `Write` instructions of longer
than 1232 bytes.
