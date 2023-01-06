---
simd: '0017'
title: Priced Compute Units
authors:
  - Anoushk Kharangate (SuperteamDAO)
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
Introduce a new price and economic model for compute units. It's important to note that the new model must not make it more expensive to vote i.e affect vote
transactions.

A good way to normalize and do this would be charge a fee per compute unit of 5 x 10<sup>-6</sup>/vote CU price SOL per CU<br/>
Example: Txn A requires 10000 CU -> Cost of txn is 10000 x (5 x 10<sup>-6</sup>)/2500 SOL (2500 is the vote CU)


Suggestion:
- Change the burn mechanism to burn 50% base fee and remaining 50% goes to validator and give 100% of the priority fee to the validator.
