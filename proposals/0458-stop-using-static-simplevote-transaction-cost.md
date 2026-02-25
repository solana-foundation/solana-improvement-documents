---
simd: '0458'
title: Stop special-casing of Vote CU cost
authors:
  - Tao Zhu
category: Standard
type: Core
status: Review
created: 2026-01-30
feature: (fill in with feature key and github tracking issues once accepted)
supersedes: SIMD-0387
---

## Summary

This proposal removes the use of statically defined compute unit (CU) costs for
Simple Vote transactions and instead accounts for them in the same way as normal
transactions when enforcing block CU limits. This also renders the Vote CU limit
obsolete. Because this change affects consensus behavior, it must be gated
behind a feature flag.

## Motivation

Simple Vote transactions currently reserve a fixed, statically defined number of
compute units for cost tracking, based on the assumption that the Vote program,
as a builtin program, has a static execution CU cost. However, SIMD-0387
specifies that the Vote program will be removed from builtin cost modeling. As a
result, the cost of Simple Vote transactions should no longer rely on static CU
assumptions and should instead be calculated in the same manner as other
transactions.

## New Terminology

N/A

## Detailed Design

- Stop using statically defined CU values for Simple Vote transactions.
- Calculate the cost of Simple Vote transactions using the same cost model and
  accounting path as normal transactions.
- Remove vote CU limit.

## Alternatives Considered

N/A

## Impact

The impact on cost consuming/tracking, and CU reserving for simple vote can be
summarized in table:

| |Total consumed CUs	| Total reserved CUs|
|:--|:--|:--|
|Current	|3428	|3428|
|Proposed	|3428* |19812**	|

- Cost tracking:
  The impact is expected to be minimal, as the actual executed CU consumption of
Simple Vote transactions is identical to the previously statically defined value
for the vast majority of votes.  CU consumption will only change for unusual
vote transactions that still classify as simple votes (e.g. two signatures, or
additional writable accounts), or unusually large vote accounts.

  [ * ]statically define CUs for simple vote includes 1 signature (720 CU), 2
write locks (600 CU), 1 vote instruction which has 2,100 CU, and 8 CU to load
small accounts, total 3428 CU. All components stay same except the CU for loaded
accounts data size may change, which is a small part overall CUs.  

- Block production:
  [ ** ] Under the current cost model, Simple Vote transactions may have higher
estimated CUs ( about 16K additional CUs) due to inclusion of default account
data loading costs. For block-packing strategies that reserve CUs upfront and
refund unused CUs after execution, this may result in larger initial CU 
reservations for Simple Vote transactions, followed by refunds. This effect is
specific to such “reserve-then-refund” packing strategies.

## Security Considerations

N/A

