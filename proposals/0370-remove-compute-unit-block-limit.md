---
simd: "0370"
title: Remove Compute Unit Block Limit
authors:
  - Firedancer Team
category: Standard
type: Core
status: Review
created: 2025-09-24
feature:
supersedes:
superseded-by:
extends:
---

## Summary

This is a post-Alpenglow proposal.

In Alpenglow, voter nodes broadcast a `SkipVote` if they do not manage to
execute a block in time. This makes the block compute limit redundant:
any blocks which take too long to execute are skipped.

This SIMD therefore removes the block compute unit limit enforcement
in the replay stage, effectively reverting (`apply_cost_tracker_during_replay`).

## Motivation

The current incentive structure for validator clients and program
developers is broken. The capacity of the network is determined not
by the capabilities of the hardware but by the arbitrary block compute
unit limit. Removing the limit means that:

- Block producers are incentivized to continually improve their
  performance, so that they can pack more transactions into a block
  and earn more revenue. If a single client is more performant than
  others, the other clients need to improve their performance to the
  same level or risk earning less rewards for validators running their
  software, which will in turn impact their market share.

- Validator clients with below-average performance are incentivized to
  improve their performance to match that of the block producer, so
  that they don't skip blocks produced by high-performing block producers.

Together, these two incentives produce a flywheel effect where
block producers continually improve their performance, which in turn
increases the average capacity of the validator client set, which in
turn makes it possible for block producers to safely push the limits,
and so on. The net result is that the capacity of the network is
governed by market forces - if demand is there, the capacity of the
network will increase to meet it.

## Alternatives Considered

Leave as is.

## New Terminology

N/A

## Detailed Design

Remove the replay compute unit check introduced with (`apply_cost_tracker_during_replay`).
In Agave, for example, this would mean removing the call to `check_block_cost_limits`
in `solana_ledger::blockstore_processor::execute_batch`:

```rust
        // Block verification (including unified scheduler) case;
        // collect and check transaction costs
        let tx_costs = get_transaction_costs(bank, &commit_results, batch.sanitized_transactions());
        check_block_cost_limits(bank, &tx_costs).map(|_| tx_costs)
```

Each validator client will also need to have a way of aborting the execution of
a block if the timeout has been exceeded. As this will be highly dependent on
each client's implementation, and does not affect the consensus algorithm,
the mechanism for this is out of scope for this SIMD.

## Impact

Over time, this will result in larger blocks being produced by the network,
increasing network capacity. This will likely happen at a faster rate than
if we were to continue manually increasing the block limit, which will result
in a better experience for users and on-chain app devs as block space will
become less scarce.

This also makes it easier to reduce slot times - as we don't have to reason
independently about the block compute unit limit and the slot time.

## Security Considerations

We will need to make sure that larger blocks can be properly disseminated
in sufficient time by the networking stack.

## Drawbacks

Removing the static block compute unit limit may impact the asynchronous execution
and certain variants of multiple concurrent proposers designs.

However, it makes sense to remove this limit now:

- There are several variants of the multiple concurrent proposers design being
  debated, some of which have asynchronous execution and some of which have
  synchronous execution. In some of these designs the block limit is needed
  and in some of them it is not. Therefore, removing the limit now does not
  preclude us from realizing multiple concurrent proposers.

- If a static block limit does turn out to be needed, this
  should be proposed as part of the multiple concurrent proposers design,
  not enshrined into the protocol forever because it happens to exist today.

- Removing the limit today has tangible benefits for the ecosystem and end
  users, as described above. These benefits can be realized today, without
  waiting for the future architecture of the network to be fleshed out.

