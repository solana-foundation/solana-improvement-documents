---
simd: '0017'
title: Priced Compute Units
authors:
  - Anoushk Kharangate (SuperteamDAO)
  - 7Layer (Overclock Validator)
  - Dubbelosix (Overclock Validator)
category: Fees
type: Fees
status: Draft
created: 2023-01-05
---

## Problem
According to the discussion on the following issue:
https://github.com/solana-labs/solana/issues/29492

Recently blocks have been reaching their compute unit limit of 48 million CU very frequently. The concerning part is most of these transactions
are requesting extra compute units without actually needing them; most of these being mev traders. The reason this happens is because
using extra compute units comes at no extra cost unlike something like rent but validators have to bear the load and aren't compensated for it
hence a new economic model is required for using compute that disincentivizes extra CU unless absolutely necessary.

## Solution
Introduce a new price and economic model for compute units. It's important to note that the new model might not want to make it more expensive to vote.

A good way to normalize and do this would be charge a fee per compute unit of 5 x 10<sup>-6</sup> SOL/(vote CU) per CU<br/>
Example: Txn A requires 10000 CU -> Cost of txn is 10000 x (5 x 10<sup>-6</sup> SOL)/2500 CU  (2500 is the vote CU)

**Note: This would cause problems for CLOBs since doing a large cancel order consumes 20k CU and would cost 0.1 SOL**

### Dynamic Base Fee based on Validator Governance
For a V2 change to this system, it would be ideal if base fees could be adjusted in some way to track fiat/sol prices better given that hardware and bandwidth costs are denominated in fiat, and spam resistance is also more meaningful at certain fiat pricing. One way this could be accomplished is through validator governance each epoch where the fee per CU base fee could be adjusted either up or down in some form of simply fixed increment to adjust for fiat changes. When SOL price is too low in fiat terms, the network becomes less spam resistant and validators need to accommodate larger blocks despite receiving less in fiat block rewards to pay for bandwidth and hardware. When Sol price increases significantly, the price increases may impinge on certain use cases and voting may become prohibitively expensive for validators.

This could also be useful for avoiding the CLOB issue, if SOL price goes up too much then for the next epoch fees can be reduced for CLOB activity to be economical.
