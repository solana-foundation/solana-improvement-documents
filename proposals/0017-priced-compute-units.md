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
transactions or atleast make sure the fees collected from CU are worth the extra voting fees and things like Token 22 
which require a lot more CU by default give the proof sizes.

A few ways this could be implemented:
- Flat fee per compute unit of 5 x 10<sup>-7</sup> SOL per CU
- Slab based system that becomes more expensive with every level, for example:
  - First 400k CU 5 x 10<sup>-7</sup> SOL per CU, next 400k could be 2x and so on.
- Keep CU fee exemptions for specific programs like vote program and token 22.
- Charge penalty on unused compute units similar to write lock fees that could be rebated if used
