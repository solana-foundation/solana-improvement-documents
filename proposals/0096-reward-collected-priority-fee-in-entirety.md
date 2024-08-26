---
simd: '0096'
title: Reward full priority fee to validator
authors:
  - Tao Zhu
category: Standard
type: Core
status: Implemented
created: 2023-12-18
feature: [3opE3EzAKnUftUDURkzMgwpNgimBAypW1mNDYH4x4Zg7](https://github.com/solana-labs/solana/issues/34731)
developement:
  - Anza - [Implemented](https://github.com/anza-xyz/agave/pull/583)
  - Firedancer - Implemented
---

## Summary

Reward 100% of priority fee to validator.

## Motivation

To better align validator incentives, priority fee should change from current
50% burn 50% reward to 100% reward to validator.

## Alternatives Considered

Current 50-50% model is an alternative, which does not fully align with
validator's incentive, and encourages side deals.

## New Terminology

None

## Detailed Design

- When calculate fee for `SanitizedMessage`, it should separate prioritization_fee
from transaction fee.
  - change `FeeStructure.calculate_fee()`
- During fee collection, priority fees are accumulated separately from transaction
fees;
  - change `bank.collector_fees` and `bank.filter_program_errors_and_collect_fee()`
- When distributing collected fees, the collector_id receives the sum of priority
fees. Logic for distributing base transaction fees is unchanged.
  - Change `bank.distribute_transaction_fee()`

- No change to fee payer account validation;
- No change to how much transaction would be paying in total;

## Impact

The implemented proposal aims to enhance incentives for validators to
prioritize transactions with higher priority fees, thereby providing more
substantial compensation for the validators' efforts in processing higher-paying
transactions.

## Security Considerations

None

## Drawbacks

By paying the complete priority fee to their own accounts, leaders can now
inflate the reported priority fees in their blocks artificially, incurring
minimal costs. Previously, burning a portion of the priority fees deincentivized
such behavior. The artificial inflation of fees could lead wallets to
overestimate the required priority fee.

## Backwards Compatibility

The implementation of this proposal necessitates the use of a feature gate.
Although there will be no alteration to the transaction submitter's payment
structure, the software incorporating the proposal will allocate a greater
portion of fees to the leader compared to other versions. Consequently, a
feature gate is essential to ensure that all validators transition to the
new functionality at the epoch boundary, thereby preserving consensus.
