---
simd: '0005'
title: account-level-base-fees-with-frequency-incentive
authors:
  - betaconger

category: Economics
type: Standards Track
status: Draft
created: 2022-12-08
---

## Summary

This proposal sets the base fee for transactions much higher (somewhere in 1 
penny to 10ths of a penny range), but offers fee discounts for fee-payers that
execute multiple transactions over a trailing window of time or slots.  A 
fee-payer that executes a sufficiently large number of transactions will be 
required to pay the the Lowest Base Fee proposed in 
https://github.com/solana-foundation/solana-improvement-documents/pull/4 
for consideration in block inclusion.

## Specification
### Terminology

- Lookback Window : rolling period of time or number of slots over which 
transactions are counted
- Entry Fee : The fee required by the fee-payer for their first transaction 
during the Lookback Window
- Transaction Count : number of transactions executed by the fee payer during 
the Lookback Window
- Required Minimum Fee : The fee required by the fee-payer as a result of 
executing multiple transactions in the Lookback Window
- Reward Factor : the amount the Required Minimum Fee is reduced per additional
transaction during the Lookback Window
- Lowest Base Fee : The fee determined by the system as the lowest allowable
 fee
- Transaction Count Threshold : the number of transactions a fee payer must do 
before they are eligible for paying the Lowest Base Fee

The fee that a fee payer will be required to pay to be considered for block 
inclusion is:

Required Minimum Fee = 
Maximum ( Entry Fee - Transaction Count * Reward Factor  ,  Lowest Base Fee)

Additionally:

Transaction Count Threshold = (Entry Fee - Lowest Base Fee) / Reward Factor

### Priority Fees and Block Inclusion

Priority fees may continue to be added at the user's discretion.  Blocks will 
continue to be packed by the validators economically.  This means that those 
required to pay the Entry Fee should always be almost always be included in a 
block while the Lowest Base Fee is below the Entry Fee.  However, if a fee 
payer attempts to pay a fee that is below their required minimum fee, that 
transaction will simply not be eligible for block inclusion and will be 
dropped.

## Motivation

Solana is designed as a high-throughput system, with the goal of being able 
to operate at the limits of hardware and network throughput.  It is able to 
process tens of thousands of transactiosn per seconds, and with the impending 
release of firedancer, this amount may increas by another order of magnitude.  
This means that in times of low or even moderate network utilization, the 
supply of block space far exceeds the supply created by the validators.  

Economically, it makes sense that in times of low utilization, the cost to use 
blockspace would also be low.  However, with base bees currently set at a small
fraction of a penny, the cost to utilize blockspace is below what most 
blockchain users are willing to pay.  This is evidenced by many users of other 
chains paying fees several orders of magnitude higher than this even in times 
of similarly low utilization.

The goal of this proposal is to try to increase fee revenue for the validators 
and stakers.  This should help increase ROI for validators and stakers and 
encourage more validators to particpate, increasing decentralization.

## Supporting observation

This type of volume incentive program exists at many centralized exchanges as 
a mechanism to maximize fee revenue.

## Technical Considerations

I admitedly do not know enough about the structure of core components to know 
if this would even be possible, or how complex the changes would be if it is 
possible.  However, the success of the proposal would certainly increase the 
ROI for validators and stakers significantly.
