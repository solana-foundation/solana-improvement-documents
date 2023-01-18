---
simd: '0019'
title: Data Availability
authors:
  - Eclipse Laboratories
category: Standard
type: Core
status: Draft
created: 2022-01-10
---

## Summary
We propose a new transaction type with a higher size limit that is not intended for execution.

## Motivation
Multiple rollups are looking to use Solana as a data availability layer to post rollup blocks to, including Eclipse Labs and Layer N. This is a poorly supported use-case—the only solution currently requires writing chunks of the data one-by-one to an on-chain account. In addition, since it requires minimal runtime execution/processing by the full node, the price of posting data would ideally be more in line with the cost of rent for account storage. This feature would allow rollups to seamlessly post blocks of ordered transactions to be finalized by Solana as an L1.

## Alternatives Considered
A new transaction type is not strictly necessary for Solana to support data availability. However, due to the hard limit on transaction size, a .1 MB block, for example, would require >80 transactions to upload on-chain. In addition, rollup data does not need to be written to state in the execution environment itself, but only a commitment to the data.

## New Terminology
Blob transaction: a new transaction type with a higher size limit, but whose data is not meant for execution.

## Proposed Solution
Drawing inspiration from EIP-4844, we propose introducing a new transaction type, called a blob transaction, with the express purpose of enabling DA posting on Solana. A blob transaction will contain an arbitrary payload of data, along with a commitment to that data according to a commitment scheme such as KZG, or more simply a hash of the data. Availability will be accomplished using a “sidecar” architecture, where only the commitment is written on-chain, but validators are required to provide the full data over the network upon request for a finite period of time (e.g. 1 month) after the initial blob transaction. This is sufficient for the use-case of rollups, since the requirement isn’t that block data be available on-chain, but that it can be queried from validators and verified against the on-chain commitment, so that third parties can reconstruct the rollup’s history.

The blob transaction should have a much higher transaction size limit, on the order of at least 130 KB (corresponding to 4096 32-byte field elements). In addition, we propose pricing the transaction according to the cost of rent for the size of the data posted over the required availability period.

Along with the new transaction type, this proposal would require a precompile implemented to verify KZG proofs against a commitment if such a commitment scheme were used, which would entail implementing group operations on a pairing-friendly curve, such as BLS12-381, as well as the curve pairing itself, in the precompile.

## Security Considerations
Allowing a transaction type with a much higher size could threaten to congest the throughput of the network. Since the blob data itself does not have to be known by validators at the time of block execution, it can be propagated lazily through a lower priority channel of the network. If handling these transactions still poses a significant strain on the throughput of the validator network, a hard cap on the number of blob transactions per-block can be implemented. 