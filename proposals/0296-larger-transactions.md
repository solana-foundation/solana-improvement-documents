---
simd: '0296'
title: Larger Transaction Size
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
bytes. This proposal presents a new transaction size of 4096 bytes.

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

Not at this time.

## Detailed Design

Increasing the transaction size limit above the current MTU max size is
possible with the use of QUIC. QUIC's RFC 9000 specification does not have
an explicit maximum stream size, allowing for larger transactions to be
sent. The larger transaction size would only be supported by the v1 transaction
format as described in SIMD-0385. Clients will need to support accepting the
larger transaction sizes on their network ports and not reject the larger size
during replay.

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
format designed in SIMD-0385 to take advantage of the larger transaction size. 
Those developers that had previously been using address lookup tables would be 
required to update the new transactions with the full address list instead of 
the address lookup table and its indices.

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

As all of these changes are implemented with the v1 transaction format, the
increase on the transaction size limit does not affect or break `v0` or `legacy`
transactions.
