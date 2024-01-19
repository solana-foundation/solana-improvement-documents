---
simd: '0110'
title: Exponential fee for write lock accounts
authors:
  - Anatoly Yakovenko, Tao Zhu
category: Standard/Meta
type: Core/Networking/Interface/Meta
status: Draft
created: 2024-01-18
feature: (fill in with feature tracking issues once accepted)
supersedes: (optional - fill this in if the SIMD supersedes a previous SIMD)
extends: (optional - fill this in if the SIMD extends the design of a previous
 SIMD)
---

## Summary:

In a permissionless environment with low fees, users often submit transactions
for opportunistic trades without considering the success probability. Almost
half of the Compute Units (CUs) in a block are allocated to failed transactions,
leading to undesirable scenario where large portion of compute powers primarily
serving failed DeFi arbitrage transactions. To address this, the proposed
feature introduces economic back pressure, discouraging spam activities and
ensuring efficient network resource utilization. It suggests tracking the
Exponential Moving Average (EMA) of contentious accounts' CU utilization
per block and exponentially increasing the cost to write-lock these accounts
if their utilization remains high.

## Motivation:

The motivation behind this feature is to introduce economic back pressure,
dissuading DeFi spammers from overwhelming the network. DeFi spam, defined as
opportunistic trades with a positive return on investment, currently occupies
almost half of the CUs in a block as failed transactions. Economic back
pressure aims to create a deterrent for such spam activities, ensuring network
resources are efficiently utilized and preventing continuous block congestion
caused by failed DeFi spam transactions.

## Alternatives Considered

While the priority fee serves to mitigate low-cost spams by decreasing the
likelihood of less prioritized transactions being included, it cannot entirely
eliminate the inclusion of spam transactions in a block. As long as there
remains a chance, no matter how small, to inexpensively include transactions,
the incentive for spamming persists. The proposed feature recognizes that the
current mechanisms have limitations in fully addressing the spam issue, and
thus, seeks to introduce a more robust solution to discourage opportunistic
trades and ensure a more secure and efficient network environment.

## New Terminology

- *compute-unit utilization*: denominated in `cu`, it represents total
 compute-units applied to a given resource.
- *cost rate*: denominated in `lamport/cu`, it represents the cost per
compute-unit at a given condition.
- *compute unit pricer*: a componenet tracks Exponential Moving Average of
*compute-unit utilization*, applies a pricing algorithm to provide current
*cost rate*.
- *write lock fee*: denominated in `lamport`, it is fee dedicated for write
lock an account, calculated as `compute-unit-pricer.cost-rate() * transaction.requested_cu()`.

## Design Highlights:

- Account Association with Compute Unit Pricer:
  - Accounts are associated with a *compute unit pricer*, and the *runtime*
  maintains an LRU cache of actively contentious accounts' public keys and
  their *compute unit pricers*.
  - Alternatively, each account can have its *compute unit pricer* stored
  onchain, which would require modifying accounts.
- Compute Unit Pricer:
  - Tracks an account's EMA *compute-unit utilization*, updated after the
  current bank is frozen.
  - Provides the current *cost rate* when queried.
- EMA of Compute-Unit Utilization:
  - Uses 150 slots for EMA calculation.
  - EMA Alpha, representing the degree of weighting decrease, is calculated as
  `alpha = 2 / (N+1)`.
- Pricing Algorithm:
  - Adjusts write-lock *cost rate* based on an account's EMA *compute-unit
  utilization*. Initial write-lock cost rate is `1000 lamport/CU`.
  - For each block, if an account's EMA *compute-unit utilization* is more than
  half of its max limit, its write-lock *cost rate* increases by 1%. If it's
  below half, the *cost rate* decreases by 1%.
- Calculate *Write Lock Fee*:
  - Fee required to write-lock an account is calculated by multiplying the
  write-lock *cost rate* by the transaction's requested CU.

## Detailed Design:

- Initialization and Inheritance:
  - Bank initializes an empty account_write_lock_fee_cache, an LRU Cache of
  {account_pubkey, compute-unit-pricer}.
  - Child banks inherit the parent's cache.
- Transaction Fee Calculation:
  - Calculate write-lock fee for each account a transaction needs to write,
  summing up to be its *write lock fee*. This, along with signature fee and
  priority fee, constitutes the total fee for the transaction.
  - Leader checks fee payer's balance before scheduling the transaction.
- Cost Tracking:
  - Cost_tracker tracks CUs for the current block and each write-locked accounti
  as-is;
  - Ensuring cost tracking is enabled at the replay stage.
- End of Block Processing:
  - Identify write-locked accounts with *compute-unit utilization* > half of
  account max CU limit. Add/update bank's account_write_lock_fee_cache. 
  - Adding new account into LRU cache could push out eldest account;
  - LRU cache has capacity of 1024, which should be large enough for hot accounts
  in 150 slots.
- Fee Handling:
  - Collected write-lock fees are 100% burnt.
  - Collected priority fees are 100% rewarded.
  - Collected signature fees are 50% burnt, 50% rewarded.

## Impact:

- Rate of successful CU inclusion in a block is expected to increase, reducing
failed transactions.
- Transactions writing to contentious accounts will experience increased fees,
particularly during congestion.
- DeFi arbitrage traders will need to adjust strategies to account for the
heightened fees.


## Other Considerations:

- Users may need new instruction to set a maximum write-lock fee for transaction
- Consider tooling for wallets/simulators to query "min/max write lock fee."
- Acknowledge read lock contention, deferring EMA fee implementation for read locks.
- In the future, a percentage of collected write-lock-fee could be deposited
to an account, allowing dApps to refund cranks and other service providers.
This decision should be done via a governance vote.


## Security Considerations

n/a


## Backwards Compatibility

Needs feature gate.
