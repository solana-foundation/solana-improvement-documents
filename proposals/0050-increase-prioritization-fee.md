---
simd: 0050
title: Increase Prioritization Fee
authors:
  - Tao Zhu
category: Standard
type: Core
status: Draft
created: 2023-05-22
feature: #31453
---

## Summary

Cost of prioritizing should be a meaningful portion of transaction base fee.

## Motivation

Prioritization fee is intended to be used sensitively during congestion,
provides advanced users a tool to access particular resources when contented
with a meaningful fee, while other users may opt to delay access to avoid
paying extra fee. When local fee market functions as designed, it provides
economic incentives to reduce congestion.

`Prioritization_fee = compute_unit_limit * compute_unit_price`,
where compute_unit_price has unit of microlamport. 

The small unit of compute_unit_price manifests into:

1. Prioritization_fee is proportionally insignificant to base fee;
2. Therefore, some payers set priority even there is no contention, potentially
   distorts local fee market, leaves other users at a disadvantage;
3. Little consideration was given to compute_unit_limit when setting priority;
These are being documented in https://github.com/solana-labs/solana/issues/31755.

To encourage prioritization fee to be used as intended as congestion control
tool, it propose to regulate `compute_unit_price` by rounding down to its
nearest 1_000 microlamports. The effect is user should set `compute_unit_price`
in increment of 1_000 microlamports. Transaction has less then 1_000
`compute_unit_price` will have no priority nor be charged a priority fee.

## Alternatives Considered

1. to change `compute_unit_price` unit from microlamports to milli-lamport.
This approach will require a new version of compute_budget instruction that
uses `milli-lamport` as unit, to coexist with existing instruction,
later to be deprecated when users migrate to new instruction. It requires
significantly more work on rolling out the change.

Current approach avoid changing API, nor introducing an additional terminology.

## Detailed Design

- No change to compute_budget instruction APIs;
- No change to how prioritization fee is calculated;
- No change to how banking_stage prioritization works;
- Add feature gated function to round user specified `compute_unit_price` down
  to its nearest 1_000 microlamports.
- The rounded `compute_unit_price` will be used by leader in prioritizing, and
  used by bank to calculate prioritization fee.

### Implementation

PoC https://github.com/solana-labs/solana/pull/31469

## Impact

- To vote transactions, no impact;
- To transactions don't set `compute_unit_price` (which is ~32% of non-vote
  transactions) , no impact;
- To transactions set `compute_unit_price`, user needs to reevaluate strategy
  of when to use priority, and how much. Specifically:
  - user might want to set `compute_unit_price` only when block or account
    contention increase;

    User can do that by either check RPC endpoint `getRecentPrioritizationFees`
    for minimal fee to land to block or write lock specific accounts when
    constructing transaction; 

    User can also continuously pull RPC endpoint to built up prioritization
    fee historical stats locally, then generate adquent prioritization fee
    algorithmically when constructing transactions.
  - when setting `compute_unit_price`, it is advised to set in increment of
    1_000 microlamports (eg 0.001 lamport);
  - When setting `compute_unit_price`, it is important to also evaluate
    value for `compute_unit_limit` to avoid paying too high prioritization fee;
    especially for ~50% non-vote transactions currently not setting
    `compute_unit_limit` at all.

### Examples

- 31.27% non-vote transactions in mainnet-beta set `compute_unit_price` when
  both block and account CUs are below 75% of limit (eg., not congested).
  They are
  currently paying insignificant amount of prioritization fee; With this
  proposal, the payers of these transaction would have to consider only
  paying prioritization fee when truly needed.

- about 6% non-vote transactions in mainnet-beta that set `compute_unit_price`
  but either not setting `compute_unit_limit` or setting it _above_ Max 1.4M.
  with this proposal, payers of these transactions would have to consider
  setting accurate/reasonable `compute_unit_limit`.


## Backwards Compatibility

No breaking changes.
