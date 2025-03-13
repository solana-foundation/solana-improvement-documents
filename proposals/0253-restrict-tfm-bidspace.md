---
simd: '253'
title: Restrict Transaction Fee Market Bid Space
authors:
 - Ajayi-Peters Oluwafikunmi (Eclipse)
category: Standard
type: Interface
status: Draft
created: 2024-12-20
feature: (fill in with feature tracking issues once accepted)
supersedes:
extends:
---

## Summary

This proposal makes breaking changes to the RPC API, introduces a fee
controller, and defines target (per block and per account) Compute Unit (CU)
utilization values that are distinct from the maximum CU utilization values to
improve the end user fee-paying and inclusion UX.

Notably, the design of the mechanism is such that there is no additional
overhead on voting nodes, and it will always maximize block space utilization.

## Motivation

The vast majority of Solana transactions currently overpay for inclusion because
there is little guidance on how much they need to pay. Another set of
transactions never made it on-chain because even though they were willing to pay
the price for inclusion, they were unaware of it. This is bad UX. The UX
degrades further during periods of high activity, which sees some users
effectively locked out of the chain. The primary reason for this phenomenon is
that Solana's current fee market implementation is a pure first-price auction
(FPA). Extensive research and empirical evidence suggest that this mechanism is
suboptimal for users; the experience on Solana only further confirms that.

Unfortunately, research also suggests that FPAs are the only credible static TFM
with potentially "byzantine" auctioneers, which means they are inevitable in
permissionless blockchains. Fortunately, extensive research has been conducted
on how to mitigate the mechanism's externalities, and restricting the auction
bid space is the only method known to work.

EIP-1559 is an implementation of a restricted bid space, but it comes with its
own problems, including extreme underutilization of network resources (the max
block size is twice as large as the target) and block space (when the base fee
is too high, transactions are dropped even if there is space for them).
Additionally, a naive replication of the 1559 mechanism on Solana is
non-beneficial as Solana is a multi-resource (contextually: local fee-market)
environment. EIP-1559 is also an invasive change as it requires modification of
the core protocol and adds additional overhead to voting nodes who must keep
track of the base fee.

Therefore, a mechanism that can achieve the desired results of restricting the
bid space without any of the externalities discussed above is highly desirable.

## New Terminology

- target Compute Unit utilization: This is a soft cap on how many Compute
  Units can fit in a valid block (per block and per account). It is a
  "protocol-aware" value.

- maximum Compute Unit utilization: This is a protocol-defined value that hard
  caps the number of Compute Units that can be packed into a valid block.

- slack is the ratio of target Compute Unit utilization to maximum Compute
  Unit utilization (per account and per block).

- fee: fee is used loosely throughout this document to refer to Compute Unit
  cost (lamports/CU).

- recommended priority fee: This is the fee that the mechanism "recommends"
  that a transaction pay. It is similar to but distinct from a per-account
  EIP-1559 base fee.

- cache: An in-memory data structure maintained by RPC nodes that tracks:
  - the most congested accounts,
  - the Exponential Moving Average (EMA) of the Compute Unit utilization for
    the accounts and,
  - the recommended priority fees for the corresponding accounts.

- `getPriorityFee`: A JSON RPC method to replace the
  `getRecentPrioritizationFees` and `getFeeForMessage` methods. The
  `getPriorityFee` request takes a mandatory list of transactions and returns
  the recommended fee to land a transaction, locking all the accounts in the
  list.

## Overview

Solana's fee market is designed such that only users bidding to access
**congested** (accounts for which demand is greater than supply) accounts need
to pay more. Everyone else (that is strictly interested in quick inclusion) only
need to pay the minimum fee for inclusion, which is determined by the global
demand for block space.

Unfortunately, as is the case with FPAs, the correct fee for the desired outcome
is undefined and unknown to users. The most useful tools that currently exist
are the `getRecentPrioritizationFees` RPC method and other proprietary
modifications to it that all do roughly the same thing:
- take a list of accounts locked by a transaction

- return the priority fee paid by a transaction that locked all accounts in
  the list in the last 150 blocks.

These tools, while helpful, are subpar because:
1.  they do not (correctly) price blockspace; they make recommendations based on
    (previous) uninformed bids.

2.  they waste RPC resources (150 blocks of data is far too much and can even be
    detrimental).

Furthermore, because different providers have different implementations, there
is some loss in efficiency.

These are the gaps that this proposal intends to address.

In the proposed system, when a user makes a `getPriorityFee` request to an RPC
node, the node checks the attached list of transactions against an in-memory
cache that tracks congested accounts and returns the recommended fee for the
most expensive (should also be the most congested) account in the list. If none
of the accounts in the list are in the cache, then the method returns the
recommended global priority fee.

The fee is described as a recommendation because there is no enforcement that
transactions pay (at least) this fee to be included a la EIP-1559. The
validators will be unaware of this system and continue processing transactions
as they do today.

The defining benefits of this approach compared to a protocol-enforced base fee
are: 
1.  blocks can be maximally packed at all times and

2.  there is no additional overhead from tracking a base fee on the protocol;
    only RPC nodes (already responsible for responding to these requests) must
    maintain the "cache."

## Detailed Design

### RPC Processing Requests

When an RPC node receives a `getPriorityFee` request, it:
- checks the cache for all the accounts in the accounts list attached.
- if none of the accounts in the list are in the cache, it returns the
  recommended global priority fee.
- if one or more of the accounts in the list are in the cache, it returns the
  recommended fee for the most expensive account (should also be the most
  congested).

### The cache

At the core of this proposal is a cache that tracks congested accounts and maps
the accounts to the recommended priority fee (calculation discussed below).
Global CU utilization is also tracked.

The "cache" is a finite capacity in-memory data structure maintained by RPC
nodes that tracks congested accounts and maps accounts to an `AccountData`
struct.

``` rust
Cache<Pubkey, AccountData>
```

The `AccountData` struct contains:
- the exponential moving average (EMA) of CU utilization of the associated
  account in the five most recent blocks,
- the recommended fee for block n (the most recent block seen by the node),
- the recommended fee that transactions that want to access the account in the
  next block (n + 1) should pay.

``` rust
struct AccountData {
    ema_of_cu_utilization_over_the_last_five_blocks // n-4 through n
    median_fee_in_block_n
    recommended_priority_fee_to_access_account_in_block_n_plus_one
}
```

The median (as opposed to a previous recommendation) is tracked because it
allows the mechanism to detect changes in demand at a particular price point
without setting a base fee (which would do so by dropping transactions below the
price point).

The global median fee and global CU utilization are also tracked in a different
data structure.

``` rust
struct GlobalData {
    ema_of_global_cu_utilization_over_the_last_five_blocks
    median_fee_global_in_block_n
    recommended_priority_fee_global_in_block_n_plus_one
}
```

### Target and Maximum Compute Unit Utilization

This proposal introduces a target CU utilization value at both the block and
account levels. Setting this value is required to allow the TFM to reliably
determine when demand outstrips supply. Without a target utilization amount, it
is difficult to detect whether demand matches or outstrips supply. Additionally,
distinguishing between the two values improves the UX by allowing additional
transactions than the target when there is a sudden increase in demand.

The greater the *slack* (the difference between target and maximum CU
utilization), the more pronounced the effects. However, setting the target CU
utilization to half of the maximum CU utilization a la EIP-1559 results in
severe underutilization of block space. Because of this, it is proposed that the
target CU utilization be 85% of the maximum CU utilization (i.e.,
$target \ cu \ utilization = 0.85 * max \ cu \ utilization$) to balance the
benefits and externalities of setting a target CU utilization.

Note that this does not require any changes to the core protocol.

### How the recommended priority fee is calculated

The recommended priority fees are determined based on a simple principle: if the
EMA of CU utilization is lower than the target, recommend a lower fee than the
median of the previous block, and if it is higher, recommend a higher fee. The
degree of how much higher (or lower) depends on the difference between the
observed value and the target. Mathematically, this can be expressed as:

let $n$ be the most recent block seen by the node (such that the next block is
$o$)\
let $f^g_n$ be the global median fee in block $n$\
let $f^g_o$ be the recommended global fee for block $o \ni o = n + 1$\
let $\mu^g_{\lambda}$ be the EMA of global CU utilization over the last five
blocks\
let $\mu^g_{\tau}$ be the target per block CU utilization\
let $\theta$ be a sensitivity parameter

The recommended global fee for block $o$ is determined by:\
if $\mu^g_{\tau} > \mu^g_{\lambda}$,\
$\ \ \ \ f^g_o = f^g_n * exp(\theta, \ (\frac{\mu^g_{\lambda}}{\mu^g_{\tau}} -1))$

if $\mu^g_{\tau} < \mu^g_{\lambda}$,\
$\ \ \ \ f^g_o = f^g_n * (1 - exp(\theta, \ (\frac{\mu^g_{\lambda}}{\mu^g_{\tau}} -1)))$  


The recommended global fee for block $o$ is a bit more involved because it must
also consider global fees but it is determined by the following equations:

let $n$ be the most recent block seen by the node (such that the next block is
$o$)  
let $f^{\alpha}\_n$ be the median fee for account $\alpha$ in block $n$  
let $f^{\alpha}\_o$ be the recommended global fee for block $o \ni o = n + 1$  
let $\mu^{\alpha}\_{\lambda}$ be the EMA of CU utilization for account ${\alpha}$
over the last five blocks  
let $\mu^{\alpha}\_{\tau}$ be the target per block CU utilization  
let $\theta$ be a sensitivity parameter  

if $\mu^{\alpha}\_{\tau} > \mu^{\alpha}\_{\lambda}$,  
$\ \ \ \ f^{\alpha}\_o = max \ \{f^{\alpha}\_n * exp(\theta, \ (\frac{\mu^{\alpha}\_{\lambda}}{\mu^{\alpha}\_{\tau}} -1)), \ f^g_o \}$

if $\mu^{\alpha}\_{\tau} < \mu^{\alpha}\_{\lambda}$,  
$\ \ \ \ f^{\alpha}\_o = max \ \{f^{\alpha}\_n * (1 - exp(\theta, \ (\frac{\mu^{\alpha}\_{\lambda}}{\mu^{\alpha}\_{\tau}} -1))), \ f^g_o \}$

As is observable from the above, the controller is exponential. An exponential
controller was chosen for two reasons:
1.  There is no desire to impose additional costs on users unless they bid for
    congested accounts. Because of this, the slack (difference between target
    and maximum utilization) must be as small as possible. Given the small
    slack, aggressive responses are crucial even if they cause some loss in
    efficiency. However, because the TFM does not enforce the base fee, as long
    as there is sufficient block space, users with a lower willingness to pay
    can still be included.

2.  Research shows that transaction arrival can be modeled by a Poisson's
    process, and exponential controllers are effective for this class of
    problems, given the additional desiderata.

### Cache updates and eviction

Every RPC node independently refreshes its cache when it receives a new block.
The global and per-account EMAs are updated during this process, and the
recommended fees for the upcoming block are calculated according to the
relations above. Also, the entries with the least congestion are evicted, and
the most congested accounts not already in the cache are inserted.

To allow effective tracking of congested accounts, we propose tracking up to
$5 * max \ lockable \ accounts \ per \ transaction * ceil(\frac{max \ block \ CU \ utilization}{target \ per \ account \ CU \ utilization})$
accounts with utilization greater than 60% of the target utilization.

This is based on the fact that:
1.  the EMA is tracked over 5 blocks

2.  the maximum number of accounts that can satisfy the criteria of being
    congested is given by $ceil(\frac{max \ block \ CU \ utilization}{target \ per \ account \ CU \ utilization}) * max \ lockable \ accounts \ per \ transaction$.

This is a simple but effective way to set an upper bound on the cache size. It
is also small enough that there is no meaningful overhead from keeping a cache
of that size--under the current conditions ($48$ M CUs per block, 64 lockable
accounts per transaction and $0.85 * 12$ M per CUs per account), the cache 
will have a capacity of 1600.

Finally, we propose a (maximum) churn rate of 5 entries per slot for the same
reason as above.

That is a complete overview of all the components of the design.

## Alternatives Considered

1.  EIP-1559-like system with a protocol-enforced base fee.

    The primary alternative consideration is a per-account EIP-1559
    implementation with parameters and a controller similar to the ones
    discussed here. But for reasons already discussed in the body of this text,
    we believe exploring the proposed implementation (and other similar variants)
    are better first steps than a (modified) per-account EIP-1559.
    
2.  Using a different controller.
    
    While the core mechanism and parameters are set, the controller is still
    open to tuning. The current design was chosen because it is the simplest
    that would satisfy the desiderata.
    A PID controller remains a viable alternative.

3.  Doing Nothing.
    
    Users will continue to overbid (best case), and the UX will continue being
    subpar.

## Impact

-   validators: unaffected
-   core contributors: unaffected.
-   RPC Nodes improve users' transaction landing probability and potentially
    reduce spam since transaction fees are more efficacious.
-   users: Users can make better-informed bids and enjoy a significantly better
    experience. They should also pay less for uncongested accounts.
-   dapp developers: will have to rewrite applications and switch to the new
    method.

## Security Considerations

None

## Drawbacks

Extreme care must be taken to ensure that TFM modifications do not create attack
vectors, especially for myopic block producers. However, seeing as the proposal
does not modify any parts of the existing mechanism, it is as sound as the
existing mechanism by reduction.

However, as mentioned earlier, the existing TFM is not incentive-compatible for
myopic block producers, and this proposal does not make it any better or worse.

But this proposal is the first of a set of proposals aimed at improving Solana's
TFM. The second will address myopic block producer incentive compatibility.

## Backwards Compatibility

Yes, core code remains unchanged.
