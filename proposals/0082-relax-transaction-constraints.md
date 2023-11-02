---
simd: '0082'
title: Relax Transaction Constraints
authors:
  - Andrew Fitzgerald (Solana Labs)
category: Standard
type: Core
status: Draft
created: 2023-10-30
feature:
---

## Summary

This proposal aims to relax some of the constraints on which individual
transactions can be included in a valid block.
The proposal does not relax the constraints required for a transaction to be
executed.

## Motivation

The current protocol places many constraints on the structure and contents
of blocks; if any constraints are broken, block-validators will mark the block
as invalid.
Many of these constraints are necessary, but some of them are not, and lead to
additional complexity in the protocol.
This proposal aims to relax some of the constraints at the individual
transaction level, in order to simplify the protocol, and give more flexibility
to block-producer and block-validator implementations.

More specifically, this proposal aims to relax the constraints which require
account state in order to determine the validity of a transaction's inclusion.
The reason these constraints are targetted specifically, is that reliance on
account state necessarily requires synchronous execution within the protocol.
This proposal on its' own, will not enable asynchronous execution, but it will
remove one of the barriers to asynchronous execution.

## Alternatives Considered

1. Do nothing
    - This is the simplest option, as we could leave the protocol as is.
    However, this leaves the protocol more complex than it needs to be.
2. Relax all but fee-paying constraint
    - This was actually the initial proposal, but it was decided that it would
    be better to relax as many constraints as possible, rather than just some.
    Any reliance on account state, means the protocol requires synchronous
    execution.
3. Additionally, relax the address lookup table resolution constraint
    - This was considered, since it is a transaction-level constraint that is
    depdendent on account-state. However, due to entry-level and block-level
    constraints that rely on the address lookup table resolution, this
    constraint cannot easily be relaxed without also relaxing those
    constraints.

## New Terminology

None

## Detailed Design

Specifically, this proposal relaxes constraints that a transaction included in
a block must:

1. Have a fee-payer with enough funds to pay fees
2. Have a valid nonce account, if specified
3. Have program accounts that:
   1. exist
   2. are executable
4. Have writable accounts that are:
   1. not executable, unless owned by
   `BPFLoaderUpgradeab1e11111111111111111111111`
   2. if owned by `BPFLoaderUpgradeab1e11111111111111111111111`,
   then `BPFLoaderUpgradeab1e11111111111111111111111` must be included
   3. if owned by `Stake11111111111111111111111111111111111111`, then the
   current slot must not be within the epoch stakes reward distribution period
5. Have executable `BPFLoaderUpgradeab1e11111111111111111111111` owned accounts
with `UpgradeableLoaderState::Program` state, and derived program data account
that exist
6. Have a call chain depth of 5 or less
7. Have no builtin loader ownership chains
(pending `4UDcAfQ6EcA6bdcadkeHpkarkhZGJ7Bpq7wTAiRMjkoi`)
8. Have total loaded data size which does not exceed
requested_loaded_accounts_data_size_limit
(pending `DdLwVYuvDz26JohmgSbA7mjpJFgX5zP2dkp8qsF2C33V`)

The intent with relaxing these constraints is to minimize the amount of
account-state which is required in order to validate a block. This gives more
flexibility to when and how account-state is updated during both block
production and validation.

With these constraints removed, there is still one account-state constraint
at the transaction level: address lookup table resolution.
With this proposal, this constraint is intentionally not relaxed since it is
necessary for validation of entry-level and block-level constraints.
However, if/when those constraints are relaxed, this constraint should be
relaxed as well.

The relaxation of these constraints is only relaxed for transactions being
included in a block. During block validation, if any of these constraints
are broken, the entire block is marked invalid; with this proposal, the
block is not marked invalid, but the transaction will not be executed.
These constraints must still be satisfied in order for the transaction to be
executed, and have an effect on state. However, there are some new considerations
which must be taken into account, specifically as it relates to fees and block
limits.

### Fee-Paying

Currently, iff a transaction's fee-payer does not have enough funds to pay the
fee, the transaction cannot be included in a block. With this proposal, it is
possible for such a transaction to be included in the block, and there are
three different edge-cases to consider:

1. The fee-payer account does not exist (0 lamports)
2. The fee-payer account does not have enough funds for the entire fee
3. The fee-payer account has enough funds for the fee, but would no longer be
rent-exempt

In case 1, the transaction should simply be ignored for execution, and have no
effect on state.
In case 2, the fee-paying account should be drained of all funds, but have no
other effect on state.
In case 3, the fee-paying account should be drained of all funds, with fees
being paid to the block-producer, and the remainder of lamports being dropped.

### Block-Limits

Pending the activation of `2ry7ygxiYURULZCrypHhveanvP5tzZ4toRwVp89oCNSj`,
validators must validate a block is within block-limits.
With this proposal, some transactions may not be executable, but will still
count towards block-limits.
If these transactions did not count towards block-limits, the validation of
block-limits would require the validator to check whether or not a transaction
is executable, which negates the benefits of this proposal.

Additionally, f these transactions did not count towards block-limits, a
malicious leader could produce a block with non-executable transactions and
overload the network.

## Impact

- Transactions that would previously be dropped with an error, can now be
  included, and even charged fees.
  - Users must be more careful when constructing transactions to ensure they
    are executable if they don't want to waste fees
- The validity of a block is no longer dependent on intra-block account-state
updates. This is because the only account-state required for transaction
validation is the ALT resolution, which is resolved at the beginning of a slot.
  - Block-production could be done asynchronously
  - Block-validation could be done without execution, but still relies on the
  execution of previous blocks.

## Security Considerations

- Removing fee-paying requirement could allow validator clients to produce
  blocks that do not pay fees. If not checked by block-producer, this could
  allow malicious users to abuse such producers/leaders.

## Drawbacks

- Any dynamic fees based on block utilization cannot/should not reward the
block-producer directly. Otherwise, this incentivizes block-producers to fill
blocks with non-fee paying transactions in order to raise fees.
- Non-fee paying transactions included in a block will be recorded in long-term
transaction history. Without any fees being paid, there is no way to reward
long-term storage of these transactions. It is important to note, there is also
currently no way to reward this storage.

## Backwards Compatibility

This proposal is backwards compatible with the current protocol, since it only
relaxes constraints, and does not add any new constraints. All previously valid
blocks would still be valid.
