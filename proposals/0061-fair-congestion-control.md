---
simd: '0061'
title: Fair congestion control with intra-block exponential local base fee
authors:
  - Ryo Onodera (Solana Labs)
category: Standard/Meta
type: Core
status: Draft
created: 2023-07-05
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This feature is a fair congestion-control mechanism in the form of an extension
to local fee markets while leaving their locality of transaction fee dynamics
intact.

To that end, it introduces a exponentially-scaled dynamic base fees to
individual local fee markets. It also attains very short feedback loop of
per-tx frequency to maintain full efficacy of Solana's peculiar execution
model compared to other blockchains: multi-threaded and low latency.

This is realized by means of some incentive tweaks to combat against the
obvious base fee manipulation with such short intervals.

## Motivation

- Write lock cu limit is bad (bot can lock out at the very first of block for
  the entire duration of whole blocktime (400ms)
- Increased Defi activities around any volatile financial markets could starve
  payment transactions for extended time
- Inter-block and linear Voluntary fee escalation with vanilla fee market
  auction can't guarantee the scheduling deadline of casual payment txes (which
  needs 99.99% sub-second confirmation at very minimum).

## Alternatives Considered

Related proposals:

(TODO: add any relation of this to them)

dynamic base fees
https://github.com/solana-foundation/solana-improvement-documents/pull/4

program rebatable account write fees:
https://github.com/solana-foundation/solana-improvement-documents/pull/16

asynchronous program execution:
https://github.com/solana-foundation/solana-improvement-documents/pull/45

increase prioritization fee:
https://github.com/solana-foundation/solana-improvement-documents/pull/50

bankless
https://github.com/solana-foundation/solana-improvement-documents/pull/5

## New Terminology

Is there any new terminology introduced with this proposal?

casual tx:
fairness:
block fullness in terms of number of actively-execution threads
dark/filler tx: 
tx base fee:
address base fee:
reserve <=> reward
requested <=> required

## High-Level Design with Example

This proposal tries to localize congestions by means of increasing minimum
required `cu_price`s for each write-locked addresses. This increase will be
done exponentially measured by the consumed CU by each addresses at the moment.
This means a transaction at least must cost the sum of `requested_cu *
base_cu_price` for all of its write-locked addresses.

This results in filtering out existing crowded subset of transaction waiting
for block inclusion with market-rate priority fee quickly while other
transactions are processed for block inclusion.

This rate-limiting gets enforced, only when the cluster deemed to
be congested. Also, those increased `cu_price` will be decreased 



## Detailed Design

### Incentive alignment







(i jotted this down in 10min before going to bed! pardon for being so random
writings...)

to *determiniscally* define active thread count (`TC_a`), additionally record
transaction termination events into poh stream.

also, derive stake-weighted average transaction execution thread count
(`TC_stake_weighted`).

so, full is defined as the duration when  `TC_a == TC_stake_weighted` (this is
updated at ~10ms intervals).

when not full, maximize throughput of each of any single threaded transaction
executions. note that, this mode exponentially cools down any hot addresses if
any.

when full, effectively pause any txes touching the hot state by exponentially
increasing the local base fees. so casual txes can be executed.

so, leaders are incentivised to manipulate in this naive form.

so, split priority fee into two parts: (1) collected, (2) accrued for the next
tx's base fee payment. The portion of (1) is calculated as if tx's cu *
`TC_stake_weighted`. (i.e. as if validator stuffed spam txes to capture the
exessive part (2) of prirotiy fee)

in this way, there's no meaning to spam blocks by leaders. at the same time,
it's still incentivied to pack txes according to p.f. desceding order, because
leaders and clients alike are want to increse of their single threaded tps)

also requested fee is basis for fee cals, block fullness calc, not the actual
cu.
- to prevent bad behavior, rebate 50% of (requested CU - actual CU)?
  - so that leaders want txes success (want to increase actual CU) => usually
    execute in the order
  - specified by user
  - so that users want txes fail fast (want to decrease actual CU)
  - 25% is burntd and 25% is collected to leaders
    - so, avoid too much requested cu.

finally, when substantial blocks are full for extended duration, the global base-fee
will naturally starts ceiling up. That's unavoidable no matter what.

priority fee isn't collected at all.
banking can implement this congenstion mechanism without forced consensus rules if disired for experiment

### Example

`TC_stake_weighted == 10` 

100 kcu

tx1a -- tx2a -- tx3a

tx1a:
  cu 100kcu
  base fee: 

tx2a:
  cu 200kcu

tx3a:
  cu 300kcu

why leaders are incentized for picking more prioritized txes even if they only receive fixed base fees?
  predictable auction mechanism and ceiling base fee as much as possible
  compound reserve from firstly-executed txes?


## Impact

How will the implemented proposal impacts dapp developers, validators, and core contributors?

## Security Considerations

What security implications/considerations come with implementing this feature?
Are there any implementation-specific guidance or pitfalls?

## Drawbacks *(Optional)*

Why should we not do this?

## Backwards Compatibility *(Optional)*

Does the feature introduce any breaking changes? All incompatibilities and
consequences should be listed.
