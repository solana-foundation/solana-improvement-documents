---
simd: "0085"
title: Additional Fee-Collector Constraints
authors:
  - Justin Starry
category: Standard
type: Core
status: Activated
created: 2023-11-05
feature: [prpFrMtgNmzaNzkPJg9o753fVvbHKqNrNTm76foJ2wm](https://github.com/solana-labs/solana/issues/33888)
development:
   - Anza - [Implemented](https://github.com/solana-labs/solana/pull/33887)
   - Firedancer - Implemented
---

## Summary

Every validator defines a node id which is also used as the validator's
fee-collector account for collecting earned protocol fees. After implementing
this proposal, validator implementations will burn fees if they would otherwise
be distributed into a fee-collector account that violates one of the following
constraints:

1. Must be a system owned account
2. Must be rent-exempt

These constraints apply to both transaction fees and rent fees. Note that the
rent-exempt constraint was already added for rent fee collection in
[Feature 30151](https://github.com/solana-labs/solana/issues/30151).

## Motivation

1. Fee distribution occurs outside of the transaction runtime because the Solana
   protocol mandates that fees are distributed to "fee-collector" accounts at the
   end of each block. By restricting fee-collector accounts to be system owned, the
   number of account modification edge cases as well as protocol complexity are
   both reduced.
2. Prevent new rent-paying accounts from being created since rent collection is
   planned to be disabled in SIMD-0084.

## Alternatives Considered

### Elide the system-owned constraint

Restricting fee-collector accounts to be system-owned is perhaps overly
restrictive and limits the amount of flexibility that validator operators have
when managing sensitive accounts with funds. However, the risk of having more
runtime edge cases is too high to allow any program-owned account to collect
fees. The Solana protocol should aim to limit the types of account modifications
that can occur outside of the transaction processor to avoid introducing
loopholes.

### Introduce an enshrined "validator-node" account

Rather than restricting fee-collector accounts to be system-owned, a new type of
"validator-node" account could be introduced. Currently, in normal validator
operations, the fee-collector account is also used as the node id as well as
the vote fee payer. Introducing a validator-node account that is owned by a
validator-node program which allows configuring a withdraw authority and
vote fee payer could help increase validator operation flexibility and
increase clarity in how validator keys are used in the protocol.

This approach requires a migration of all fee-collector accounts as well as
the development of a new on-chain program to manage the new validator-node
accounts. It will be a big effort compared to the proposed constraints in this
SIMD and should be discussed in a new SIMD if this approach is desired.
Furthermore, durable nonce accounts already have a configurable authority field
which can be used to manage fee-collector account funds in a more flexible way.

## New Terminology

Fee-Collector Account: The account that receives block and rent fees distributed
by validators.

## Detailed Design

At the end of a block, validators MUST ONLY distribute fees to accounts that are
both system owned and rent-exempt. If a fee-collector account does not satisfy
these constraints, the fees MUST be burned by not distributing them to anyone.

## Impact

New and existing validators must ensure that their fee-collector account is
rent-exempt and owned by the system program in order to receive fees. Since the
Solana Labs validator implementation currently requires the fee-collector
account to be same account as the fee payer for vote transactions, this is
unlikely to impact any validators unless they run a custom implementation.

Validators will still be able to collect fees into durable nonce accounts if
they wish. If a validator does not wish to use a hot wallet to have custody
over collected fees, they may use durable nonce accounts which have a
configurable authority address.

## Security Considerations

Note that durable nonce accounts are system owned and rent exempt and can
therefore continue to be used for fee collection.
