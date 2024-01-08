---
simd: '0096'
title: Reward full priority fee to validator
authors:
  - Tao Zhu
category: Standard
type: Core
status: Draft
created: 2023-12-18
feature: (fill in with feature tracking issues once accepted)
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
- When accumulate collector fees, prioritization_fee should also be separated
from transaction fee;
  - change `bank.collector_fees` and `bank.filter_program_errors_and_collect_fee()`
- When distribute transaction fee, should deposit unburnt transaction fee and 100%
prioritization_fee to collector_id.
  - Change `bank.distrbute_transaction_fee()`

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

100% rewarding priority fee allows leaders to artificially bump up priority fee.
They send a bunch of txs to themselves with a massive priority fee and recover
all of it, pumping the price artificially.

## Backwards Compatibility

The implementation of this proposal necessitates the use of a feature gate.
Although there will be no alteration to the transaction submitter's payment
structure, the software incorporating the proposal will allocate a greater
portion of fees to the leader compared to other versions. Consequently, a
feature gate is essential to ensure that all validators transition to the
new functionality at the epoch boundary, thereby preserving consensus.
