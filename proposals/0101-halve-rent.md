---
simd: '0101'
title: Halve Rent
authors:
  - Brooks Prumo (Solana Labs)
category: Standard
type: Core
status: Draft
created: 2024-01-05
feature: https://github.com/solana-labs/solana/issues/34656
---

## Summary

Halve rent.

## Motivation

Projects that build on Solana face operational cost volatility if they
manage/create accounts for their users.  This is because creating accounts
incurs *rent*, and that cost is both (1) fixed, and (2) denominated in SOL.

The original cost of rent was based on calculations assuming 1 SOL == 1 USD.
As the SOL token accrues more value, one side effect is that rent increases
*as a function of fiat*.  Since not all projects have treasuries exclusively
in SOL, this can play out where project growth/user acquisition becomes more
and more expensive over time.

The cost of rent can be reduced to offset user acquisition expenses.

## Alternatives Considered

As noted in the Motivation, rent is both fixed and denominated in SOL.  This
SIMD changes neither of those two facts, but a longer-term project is in the
works that addressed the 'fixed' nature of rent.  Ideally the protocol should
price state based on current usage and demand—incentivizing users to close
unused accounts, and charging users proportionally, based on demand, for new
state.  That project will take much longer to design and implement; in the
meantime, steps can still be made to help projects grow.

## New Terminology

None.

## Detailed Design

A new feature-gate will be added that halves Rent's lamports-per-byte-year.

## Impact

The cost of rent per account will be halved.

Validators will *not* be economically impacted.  Since accounts must carry a
minimum balance to be rent-exempt, this means there are no rent-paying
accounts.  Thus, there are no rent fees that validators collect, and changing
the rent amount will not change that.

Developers *will* be impacted, if they create accounts.  Now, these accounts
will require half as much rent, in SOL, as before.

## Security Considerations

Unbounded account growth can exhaust physical resources of validators.
Reducing rent can make it economically easier to create accounts.  The amount
that rent is *reduced* should be significantly lower than the amount the SOL
token increases.

## Drawbacks

Rent could be seen as a way to lock value on the network.  Advocates of this
view may be in favor of *increasing* rent instead.  Similarly, validators with
lower physical specs may be against reducing rent, as they may want to minimize
account growth to not incur operational expenses upgrading hardware.

The author does not consider either view as particularly strong.  Most do not
consider rent as locked value.  And the network should not cater to lower-spec'd
validators.
