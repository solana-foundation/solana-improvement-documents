---
simd: '0006'
title: account-level-base-fees-with-frequency-discount
authors:
  - betaconger

category: Standard
type: Core
status: Draft
created: 2022-12-08
---

## Summary

This proposal sets the Required Minimum Fee for transactions much higher 
(somewhere in 1 penny to 10ths of a penny range), but offers fee discounts for 
fee-payers that execute multiple transactions over a trailing window of time or
slots.  A fee-payer that executes a sufficiently large number of transactions 
will be required to pay the the Lowest Base Fee proposed in 
https://github.com/solana-foundation/solana-improvement-documents/pull/4 
for consideration in block inclusion.

## Motivation

Solana is designed as a high-throughput system, with the goal of being able 
to operate at the limits of hardware and network throughput.  It is able to 
process tens of thousands of transactions per seconds, and with the impending 
release of firedancer, this amount may increase by another order of magnitude.  
This means that in times of low or even moderate network utilization, the 
supply of block space far exceeds the supply created by the validators.  

Economically, it makes sense that in times of low utilization, the cost to use 
blockspace would also be low.  However, with base fees currently set at a small
fraction of a penny, the cost to utilize blockspace is below what most 
blockchain users are willing to pay.  This is evidenced by many users of other 
chains paying fees several orders of magnitude higher than this even in times 
of similarly low utilization.

The goal of this proposal is to try to increase fee revenue for the validators 
and stakers.  This should help increase ROI for validators and stakers and 
encourage more validators to participate, increasing decentralization.


## New Terminology

- Lookback Window : rolling period of time or number of slots over which 
transactions are counted
- Entry Fee : The fee required by the fee-payer for their first transaction 
during the Lookback Window
- Lookback Transaction Count : number of transactions executed by the fee payer 
during the Lookback Window
- Required Minimum Fee : The fee required by the fee-payer as a result of 
executing multiple transactions in the Lookback Window
- Reward Factor : the amount the Required Minimum Fee is reduced per additional
transaction during the Lookback Window
- Lowest Base Fee : The fee determined by the system as the lowest allowable
 fee
- Transaction Count Threshold : the number of transactions a fee payer must do 
before they are eligible for paying the Lowest Base Fee



## Detailed Design

### Feature Implementation

This feature must be implemented during Banking Stage when transactions are
selected to be executed.  The validator can still select the tx's with the 
highest fees (behave economically), however, there must be a hard rule that 
excludes tx's from consideration if they do not meet their Required Minimum
Fee.  For avoidance of doubt, this means that if a transaction is submitted
that does not meet it's Required Minimum Fee, it is not eligible for block 
inclusion, and is dropped.  This introduces a problem, because if a block is 
not full, it would still be economical for the transaction to be included by 
the leader even if it does not meet the Required Minimum Fee.  It seems the 
only way to guarantee honesty is to have this feature be deterministic and 
easily tracked/validated from on-chain data.  Post-execution, if it was 
determined that a transaction was included that did not meet the Required 
Minimum Fee, it would be deemed invalid and cause a fork.  Eventually, 
including such transactions incorrectly could lead to slashing.

The fee that a fee payer will be required to pay to be considered for block 
inclusion is the Required Minimum Fee.  There are various ways to implement
how aggressive the discount is.  However for this proposal, I have included
a simple linear decline in Required Minimum Fee fee as Lookback Transaction 
Count increases.

Required Minimum Fee = 
Maximum ( Entry Fee - Lookback Transaction Count * Reward Factor  ,  
Lowest Base Fee)

Additionally:

Transaction Count Threshold = (Entry Fee - Lowest Base Fee) / Reward Factor

This feature interacts with and/or is built in conjunction with: 
https://github.com/solana-foundation/solana-improvement-documents/pull/4 

### Priority Fees and Block Inclusion

Priority fees may continue to be added at the user's discretion.  Blocks will 
continue to be packed by the validators economically.  This means that those 
required to pay the Entry Fee should always be almost always be included in a 
block while the Lowest Base Fee is below the Entry Fee.

## Impact

This will impact end users and dapps heavily.  Those users and dapps will need
to have advance knowledge of their Required Minimum Fee, otherwise they risk
having all of their transactions dropped.  It is strongly recommended that
all wallets display a user's Required Minimum Fee clearly.

## Drawbacks

The most obvious drawback to doing this is that it is not clear how intensive
this would be on the banking stage.  It seems computationally intensive to 
do a lookup over the Lookback Window to determine the Required Minimum Fee for
every single fee payer.


## Alternatives Considered

- Instead of a linear discount, an exponential discount could be used to more 
or less aggressively discount the fees as transactions increase.  Increasing
the discount aggressively (reducing the fee quickly) will probably lead to the 
best user experience/validator ROI tradeoff.

- Instead of Tx count, we could also track total compute units during the 
Lookback Window

- To address the drawback of potentially increased computational power to
determine and validate the Required Minimum Fee, we could also implement a 
system whereby the Required Minimum Fee is calculated, updated, and stored in
a separate account after each transaction is performed.  This reduces the 
computational effort dramatically at the cost of needing to rent a new account
to store this information.  Perhaps we could make this feature opt-in.  If
a user opts-in, they pay the fee to rent the account and are eligible for fee
discounts through this feature.  If they do not opt-in, they are required to 
pay at least the Entry Fee for all transactions.

## Backwards Compatibility

This change is breaking, since it will cause a fork if a transaction is
included below the Required Minimum Fee.

It might be possible to somehow implement this such that it does not cause a
fork, in which case this would be fully backwards compatible. That route is not
obvious to me.

## Supporting observation

This type of volume incentive program exists at many centralized exchanges as 
a mechanism to maximize fee revenue.

## Technical Considerations

I admittedly do not know enough about the structure of core components to know 
if this would even be possible, or how complex the changes would be if it is 
possible.  However, the success of the proposal would certainly increase the 
ROI for validators and stakers significantly.

