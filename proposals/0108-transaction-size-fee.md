---
simd: '0108'
title: Transaction Size Fee
authors:
  - Jon Cinque (Solana Labs)
category: Standard
type: Core
status: Draft
created: 2024-01-18
feature: (fill in with feature tracking issues once accepted)
---

## Summary

Add a new transaction fee based on the serialized size of the transaction.

## Motivation

Currently, fees are based entirely on the time it takes to process the
transaction in a node:

* the fee per signature covers the sigverify time, before execution
* priority fees are priced on compute units, which cover the virtual machine
execution time

A transaction takes up more than time resources, however. The network must also
broadcast the transaction through Turbine so that all nodes can replay it, which
consumes bandwidth.

To see this in practice, let's imagine we have two transactions. They take the
same amount of time to execute, meaning that they have the same number of
signatures to verify, and they consume the same number of compute units.

If one of those transactions has a larger payload, however, it will use up more
bandwidth, and potentially reduce the overall transaction throughput of the network.

To quantify the impact of transaction payload size, we ran tests and reported
the results in [this issue](https://github.com/solana-labs/solana/issues/28063).
We setup a small private test network, and used `solana-bench-tps` to send large
numbers of transactions with useless random padding bytes, ranging from 0 to 2206.

The test network removed all QUIC congestion control to better capture total
throughput.

The inner work in each transaction was exactly the same, performing a CPI into 
the system program to transfer SOL. This way, we fixed the time dimension of
transaction process and isolated the impact of transaction size.

The base transaction size is 257 bytes, and the tool sent batches of 50k
transactions. Here is the table of results, taken from
[this comment](https://github.com/solana-labs/solana/issues/28063#issuecomment-1727151064):

Without congestion control, there is slight decrease in transactions per second
with more padding bytes. However, the overall variance is larger than any effect
from padding bytes. So there may be some negligible effect based on transaction
size.

This test, however, was only on a small network, so we can't come to any
conclusions about how larger transactions will impact a real network.

## New Terminology

Transaction size fee: the additional lamports paid by a user, based on the number
of bytes in the serialized transaction.

## Detailed Design

### Fee Calculation

We propose a solution with a few important features:

* the new fee does not impact current transactions
* the fee contains a non-linear component to dissuade huge transactions,
especially while we see the impact of large transactions on a real cluster
* the fee also includes a linear component for the future, if the effect on a
real cluster is small

Given `total_transaction_bytes` and the fee parameters
`square_lamports_per_byte` and `linear_lamports_per_byte`, the fee is given as:

```
transction_bytes_over_mtu = total_transaction_bytes - 1232
linear_fee = ceil(linear_lamports_per_byte * transaction_bytes_over_mtu)
square_fee = ceil((square_lamports_per_byte * transaction_bytes_over_mtu) ^ 2)
transaction_size_fee = linear_fee + square_fee
```

The `linear_lamports_per_byte` and `square_lamports_per_byte` can be modified by
the cluster, similar to `lamports_per_signature`.

In this proposal, we err on the side of caution in order to make larger
transactions quadratically more expensive, and omit the linear component.

* `linear_lamports_per_byte`: 0
* `square_lamports_per_byte`: 0.05

This gives the following fees for different transaction sizes, in lamports, and
a comparison to the current base transaction fee of 5,000 lamports:

Tx Multiple | Tx Byte Size | Size Fee (Lamports) | ~Fee / 5,000
--- | --- | --- | ---
2x | 2,464 | 3,795 | 0.75x
3x | 3,696 | 15,179 | 3x
5x | 6,160 | 60,713 | 12x
10x | 12,320 | 307,360 | 62x
20x | 24,640 | 1,369,837 | 274x

Again, the approach of a non-linearly increasing fee is very conservative, to
properly compensate leaders if large transactions reduce cluster TPS or
significantly increase bandwidth usage.

Over time, as the impact of large transactions on a cluster is measured
concretely, the fee could be greatly reduced, or switched to the linear
coefficient only.

The fee coefficients can be updated through a stake-weighted vote from the
validator set, but the update process is out of scope for this document.

### Validator Changes

When calculating the fee, the transaction size fee must be calculated and deducted
from the fee payer account as outlined earlier.

As with other fees, half of the fee is burned, and half goes to the leader.

### On-chain changes

When the feature for the transaction-size fee is enabled, the runtime creates a
new sysvar account with the size fee parameters at address
`SysvarSizeFees11111111111111111111111111111`, with the following data:

```
struct TransactionSizeFees {
    linear_micro_lamports_per_byte: u64,
    square_micro_lamports_per_byte: u64,
}
```

`u64`s are little-endian 64-bit unsigned integers.

The `square_micro_lamports_per_byte` is given as millionths of lamports. So a
`square_lamports_per_byte` of 0.05, the `square_micro_lamports_per_byte` is the
number 50,000 stored as a little-endian `u64`.

### RPC Changes

The `getFeeForMessage` endpoint includes the transaction size fee in its
calculation.

## Impact

Dapp developers need to consciously craft smaller transactions to avoid
forcing their users to pay too much. Address-lookup-tables are a great help
here.

Wallets must always query the fee using `getFeeForMessage` or update their
internal fee calculation to take the new fee parameters into account.

Developers using the deprecated `getFees` RPC endpoint will start receiving
incorrect information. This is acceptable considering the deprecation started
in Solana v1.9.

Validators will likely collect more block rewards due to the eventual presence
of transactions larger than 1 MTU.

No impact on core contributors, except to be sure that vote transactions do not
get larger than 1,232 bytes and end up costing validators higher vote fees.

## Alternatives Considered

We can simply impose a flat cost per byte, but that doesn't capture a non-linear
effect on the transaction size.

We can also impose the fee for the first 1,232 bytes, but that would cause issues
for current users, and especially for validators who pay voting fees.

## Security Considerations

No particular concerns, the fee calculation simply needs to be correct across
all validator implementations.

## Backwards Compatibility *(Optional)*

The deprecated `FeeCalculator` will start giving incorrect calculations.

If we break ABI compatibility and update `FeeCalculator` to use the transaction
size fee parameters, users will be forced to upgrade to the new `FeeCalculator`.

If users are forced to upgrade, we should point them towards `getFeeForMessage`
instead.
