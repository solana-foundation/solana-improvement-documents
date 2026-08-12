---
simd: '0599'
title: Remove Inactive Stakes from the Stakes Cache
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

The stakes cache contains all stake accounts, including inactive stakes. The
inactive stakes must be removed.

## Motivation

The stakes cache in the Solana runtime currently includes inactive stake
accounts, which are useless, but consensus critical.

They are consensus critical because they are included during the rewards period
and can impact:

1. The number of partitions
2. The lattice hash

This means all validator implementation must maintain an identical view of the
inactive stakes in the stake cache, which is a maintenance burden and potential
footgun.

## New Terminology

Not new, but "inactive stake" means "a stake with no effective or activating
stake".

As a reminder, a deactivating stake has effective stake.

## Detailed Design

During epoch rollover from an epoch E to E + 1, all stakes that were inactive in
E must be removed from the stakes cache.

Additionally, during epoch rollover, inactive stakes must be excluded from the
set of all potential reward-receiving stakes.

During account updates, if the slot is outside the rewards period (calculation
and distribution), inactive stakes must be removed from the stakes cache. The
stakes cache is updated on all account updates. This includes modifications due
to executed transactions, and also modifications that take place outside of
transactions, such as block and inflation reward payout.

As a reminder, during the calculation and distribution phases of partitioned
epoch rewards, stake state changes are not allowed through transactions. 

This table summarizes the cases and actions to take.

| Status in E | Status in E+1 | During rewards for E | After rewards for E |
| -- | -- | -- | -- |
| Active | Inactive | Retain | Evicted if updated |
| Inactive | Inactive | Exclude and remove at rollover | No additional action |
| Activating / active | Activating / active | Retain | Retain |

### Edge Cases

This case is covered in the design, but it is useful to explain. Stakes going
from non-zero effective stake to inactive at the start of epoch E + 1 must be
included in the reward set.

During partitioned epoch reward payout, the newly-inactive stake must remain in
the stakes cache to receive its rewards, but after the rewards period is over,
the inactive stake is treated as inactive by both removal mechanisms.

### Validator Components Affected

Which validator components are affected by this change?

| Validator Component             | Impact                             |
|---------------------------------|------------------------------------|
| Transaction Execution (Runtime) | Stakes cache operation changed     |

## Alternatives Considered

None.

## Impact

Validator development will be simpler since developers no longer need to
consider inactive stakes in the cache.

## Security Considerations

None.

## Conformance

We will provide a ledger with active, inactive and deactivating stakes at epoch
E, covering all cases in the table under "Detailed Design".
