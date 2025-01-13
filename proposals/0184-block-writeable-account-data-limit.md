---
simd: '0184'
title: Block Writeable Account Data Limit
authors:
  - Brennan Watt (Anza)
category: Standard
type: Core
status: Idea
created: 2024-10-15
feature: (fill in with feature tracking issues once accepted)
supersedes:
superseded-by:
extends:
---

## Summary

A protocol limit on the amount of account data in bytes that may be written
during a single block.

## Motivation

When direct mapping is enabled, clients will be able to handle much more account
data because there will be fewer copies in memory (e.g. for each VM invocation /
CPI call). However, writeable account data cannot be direct mapped because it
needs to be written back after a successful tx execution. This necessitates a
decoupling between account read and write data in order to increase readable
account data limits.

Note: There is an existing per block limit on newly allocated account data
(100MB), but this does not encompass account data being updated/written back.

## New Terminology

None

## Detailed Design

This feature would check the amount of account data marked as writeable for each
transaction being handled by the block producer (for inclusion in a block) or
validator (replaying transactions in a block), add it to a running total of
total writeable account data for the block, and compare it against some
threshold (recommend 2GB to start) to decide if the transaction is allowed to be
included in the block (if sum is below the limit for the case of block producer)
or if the slot needs to be marked dead (if sum is above the limit for the case
of validator).

This feature to limit writeable account data per block would slot in nicely to
the existing cost model and cost tracker that maintain per block limits for
various resources transactions can consume. This will allow block producers to
determine when a block is "full" and validators to mark slots dead that do not
adhere to these limits.

This feature could be implemented like so (using Agave client as reference):

1. Update the cost model to compute the aggregate size of all accounts marked
  writeable for each transaction. This can be done similarly to how
  `data_bytes_len_total` is computed inside `get_transaction_cost` but limited
  to accounts marked writeable.
2. Update the cost tracker to check the current writeable account data in the
  block being produced + the amount of account data that would be written with
  the current transaction against the total block limit in the would_fit
  function
3. Store the write limit in a constant such as
  `MAX_BLOCK_ACCOUNTS_DATA_SIZE_WRITEABLE` and set this to `2_000_000_000`
  (2GB).
4. Add a new `CostTrackerError` type `WouldExceedAccountDataWriteableBlockLimit`
  and return this error if block limit is exceeded.
5. Treat this new error type (`WouldExceedAccountDataWriteableBlockLimit`) as
  retryable in execute_and_commit_transactions_locked

There is an edge case where if a single transaction tries to consume more than
the entire per block writeable account data budget, it should not be marked as
retryable. This might not be possible today given tx size limits and maximum
account size limits, but this could be exploitable in the future if either of
this constraints change.

## Alternatives Considered

Keep read/write data limits coupled even after direct mapping. Downside of this
is artificially constraining the amount of readable account data per block.

Unthrottle write data without any limit. This could negatively impact slot
replay times for validators with slow disks.

## Impact

Read and write account data limits per block can be increased independently of
each other.

## Security Considerations

Agave and FD (and any other) clients should implement this proposal to avoid
breaking consensus.

## Drawbacks

If a large amount of account data is attempted to be updated in a given block,
transactions will start being excluded and marked as retryable where they can
attempt to be included in the next block.

## Backwards Compatibility

With this new threshold, it is possible there are blocks that would have been
valid that are now marked dead due to exceeding the write threshold. This change
will need to be feature gated to ensure all validators change behavior in lock
step.
