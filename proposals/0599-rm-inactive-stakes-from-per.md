---
simd: '0599'
title: Remove Inactive Stakes from Rewards
authors:
  - Ishan Bhatt (Firedancer)
  - Hana (Anza)
  - Jon C (Anza)
category: Standard
type: Core
status: Idea
created: 2026-08-10
feature: RMsTKfD6hZnBhhNvgGBeKNrqCNkeoP3DYYxNtcuWtRg
---

## Summary

The partitioned epoch rewards process considers all stake accounts, including
inactive stakes. Inactive stakes do not contribute to stake weight or rewards,
so must not be included.

Only activating, active, and deactivating stake accounts must be included.

## Motivation

The Solana runtime currently includes inactive stake accounts during partitioned
epoch reward calculations, which are useless, but consensus critical.

Their inclusion during the rewards period can impact:

1. The number of partitions
2. The lattice hash

As a result, all validator implementations must also include inactive stakes,
which is a maintenance burden and potential footgun.

## New Terminology

Not new, but "inactive stake" means "a stake with no effective or activating
stake".

As a reminder, a deactivating stake has effective stake.

## Detailed Design

During epoch rollover from an epoch E to E + 1, all stakes that were inactive in
E must not be included in the partitioned epoch rewards phases: calculation and
distribution.

Specifically, during epoch rollover, inactive stakes must be excluded from the
set of all potential reward-receiving stakes.

During account updates, if a stake account is inactive in *both* the current and
previous epochs, it must be removed from consideration for a validator's stake
weight.

As a reminder, during the calculation and distribution phases of partitioned
epoch rewards, stake state changes are not allowed through transactions. 

This table summarizes the cases and actions to take.

| Status in E | Status in E+1 | During rewards for E |
| -- | -- | -- |
| Activating / active | Activating / active | Include |
| Active | Inactive | Include |
| Inactive | Inactive | Exclude |

### Edge Cases

This case is covered in the design, but it is useful to explain. Stakes going
from non-zero effective stake to inactive at the start of epoch E + 1 must be
included in the reward set.

During partitioned epoch reward payout, the newly-inactive stake must receive
its rewards.

### Validator Components Affected

Which validator components are affected by this change?

| Validator Component             | Impact                            |
|---------------------------------|-----------------------------------|
| Transaction Execution (Runtime) | Partitioned epoch rewards changed |

## Alternatives Considered

None.

## Impact

Validator development will be simpler since developers no longer need to
consider inactive stakes in partitioned epoch rewards.

## Security Considerations

None.

## Conformance

We will provide a ledger with active, inactive and deactivating stakes at epoch
E, covering all cases in the table under "Detailed Design".
