---
simd: '0123'
title: Block Fee Distribution
authors: Justin Starry (Anza)
category: Standard
type: Core
status: Draft
created: 2024-03-10
feature: (fill in with feature tracking issues once accepted)
---

## Summary

A new mechanism for distributing block fees to delegated stake accounts is
proposed to allow validators to share block fee revenue with their delegators.
Validators will be able to specify a commission rate for block fees and the
protocol will automatically distribute the remaining fees to delegated stake
accounts at the end of each epoch.

## Motivation

Delegated stake directly increases the number of blocks that a validator is allocated
in an epoch leader schedule but the core protocol doesn't support diverting any of
that extra revenue to stake delegators.

## Alternatives Considered

Due to the lack of core protocol support for this feature, validators have
developed independent ways to distribute block revenue which is not enforced by
the core protocol. For example, the Cogent validator diverts part of its fee
revenue to NFT holders. But it's up the NFT holders to audit and hold Cogent
accountable to promised commission.

Another alternative is Jito's mechanism for block "tips" (not fees, but the idea
is similar). Jito's validator implementation includes a tip distribution program
which it instructs validator operators to divert all of their tips to but cannot
enforce perfect compliance. It's up to stakers and the Jito team to audit
compliance by validator operators. This mechanism requires trusting a
third-party (in this case Jito) to calculate reward distribution in an accurate
and fair mannger. It also relies on using a merkle tree to distribute fees to
all stake accounts and the distributed fees are not automatically staked in
recipient stake accounts.

## New Terminology

- validator fee program: New core program which supports diverting a portion of
block fees to stake delegators

- validator fee account: New account type which supports setting a commission
for collected block fees

## Detailed Design

Currently, all block fees, including both transaction base fees and priority
fees, are collected into a validator's node id account. As of SIMD 85, the
validator id account must be both system-owned and rent-exempt to receive
collected fees. 

In order to allow validators to set a block fee commission rate, a new validator
fee program and validator fee distribution account is proposed below. Note that
vote accounts cannot be used because there isn't a guaranteed 1:1 mapping from
validator node id addresses to vote accounts.

Since validator node id's are not easily changed and are predominantly used for
paying vote transaction fees, this proposal introduces new fee distribution
accounts whose addresses are derived from validator node ids at a derivation
path proposed below.

### Block Fee Collection

After all transactions are processed in a block for a given leader, rather than
collecting fees into the validator node id account, the protocol REQUIRES
checking the existence of an initialized validator fee account derived from the
leader's node id address. If no account is found at that path or the account has
not been initialized, fees MUST continue being collected into the validator node
id account balance in adherence to SIMD 85 fee collector constraints. If an
initialized account is found, the account MUST be loaded and deserialized to
determine the leader's fee commission. Then the protocol REQUIRES that collected
fees are first split at the determined commission rate and then the commission
fee MUST be transferred to the validator node id account and the leftover fees
MUST be transferred to the validator fee account.

### Block Fee Reward Calculation

At the end of an epoch, validator fee accounts MUST be derived for all validator
node id's in the epoch leader schedule. The protocol REQUIRES checking if an
initialized validator fee account exists and has a lamport surplus above its
rent-exempt balance. For every validator fee account with a lamport surplus,
the lamport surplus is divided evenly by delegated stake weight across all vote
accounts.

TODO: Prevent vote accounts from changing node id?
TODO: Prevent multiple vote accounts for one node?

### Block Fee Reward Distribution

Each fee reward calculated for a delegated stake account will be included into a
list of reward entries which MUST be partitioned and distributed according to
SIMD 118.


### Validator Fee Program

Created as an enshrined SVM program..

TODO

- `InitFeeAccount`
- `UpdateFeeCommission`

### Validator Fee Account

TODO

- `account_type`
- `version`
- `commission`

### Validator Fee Derived Address

TODO

1. `"VALIDATOR_FEE"`
2. Validator node id address

## Impact

Validators will need to initialize and set commissions for their new validator
fee accounts.

Stake delegators will receive additional stake reward income when delegating to
validators who adopt this new feature.

## Security Considerations

TODO

## Drawbacks *(Optional)*

Why should we not do this?

## Backwards Compatibility *(Optional)*

Does the feature introduce any breaking changes? All incompatibilities and
consequences should be listed.