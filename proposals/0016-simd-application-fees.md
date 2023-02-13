---
simd: '0016'
title: Application Fees (Write-lock fees)
authors:
  - Godmode Galactus (Mango Markets)
  - Maximilian Schneider (Mango Markets)
category: Standard
type: Core, Fees
status: Draft
created: 2022-12-23
feature:
---


## Summary

This SIMD will discuss additional fees called Application Fees or write lock fees.
These fees are decided and set by the dapp developer to interact with the dapp usually by
write-locking one of the accounts. Dapp developers then can decide to rebate these fees
if a user interacts with the dapp as intended and disincentivize the users which do not
interact with the app as intentended. These fees will be applied even if the transaction
eventually fails. These fees will be collected on the same writable account and the
authority can do lamport transfers to recover these fees.

Discussion for the issue : https://github.com/solana-labs/solana/issues/21883

## Motivation

During network congestion, many transactions in the network cannot be processed
because they all want to write-lock same accounts. When a write lock on an account is
taken by a transaction batch no other batch can use this account in parallel, so only
transactions belonging to a single batch are processed correctly and all others are retried
again or forwarded to the leader of the next slot. With all the forwarding and retrying the
validator is choked by the transactions write-locking the same accounts and effectively
processing valid transactions sequentially.
There are multiple accounts of OpenBook (formerly serum), mango markets, etc which
are used in high-frequency trading. During the event of extreme congestion, we observe
that specialized trading programs write lock these accounts but never CPI into the respective
programs unless they can extract profit, effectively starving the actual users for access.
Penalizing this behavior with runtime fees collected by validators can create a perverse
incentive to artificially delay HFT transactions, cause them to fail, and charge penalty fees.

## Alternatives Considered

Having a fixed write lock fee and a read lock fee.

Pros: Simpler to implement, Simple to calculate fees.

Cons: Will increase fees for everyone, dapp cannot decide to rebate fees, (or not all existing
apps have to develop a rebate system). Fees cannot be decided by the dapp developer.

## New Terminology

Application Fees: Fees that are decided by the dapp developer that will be charged if a user
successfully write locks an account.

Base Fee Surcharge: An extra fee is applied depending on the number of accounts that were loaded for
a transaction. This fee is only charged after loading multiple accounts and then failing because
the payer did not have enough lamports to pay application fees or the limit set by `LimitApplicationFees`
instruction is not enough described below.  

## Detailed Design

As a high-performance cluster, we want to incentivize players which feed the network
with correctly formed transactions instead of spamming the network. The introduction
of application fees would be an interesting way to penalize the bad actors and dapps
can rebate these fees to the good actors. This means the dapp developer has to decide who to
rebate and who to penalize special instructions. In particular, it needs to be able to
penalize, even if the application is not CPI'd to. There are multiple possible approaches
to provide the application with access to transfer lamports outside of the regular cpi
execution context:

1. A new specialized fee collection mechanism that uses per-account meta-data to encode
additional fees. Lamports should be collected on the actual accounts until claimed by
their owner. Account owners can trigger a per account rebate on the fee collected
during a transaction. Two strategies have been proposed:

    1. Create PDAs of the application program to store fee settings

        1. Concern is well encapsulated and implementation can be easily modified (**+**)
        2. Overhead for loading a lot of PDAs might be high (**---**)
        3. Hard to calculate fee for a message (**-**)

    2. **Extend existing account structure in ledger to store fee settings and collect lamports**

        1. High efficiency, minimal performance impact (**++**)
        2. Accounts structure is difficult to modify (**---**)
        3. Easier to calculate fees for a message (**+**)
        4. If application fees are high can prevent denial of service attack on smart contract. (**+**)
    POC is implemented using this method.

2. A generic execution phase that gets invoked for every program account passed to a
transaction. Programs would optionally contain a fee entry-point in their binary
code that gets invoked with a list of all accessed account keys and their lock
status. Programs would need access to a new sysvar identifying the transaction fee
payer to rebate lamports to, to prevent breaking API changes for clients.

    1. High degree of flexibility for programs (**++**)
    2. Least data structures modified in runtime (**++**)
    3. Does not allow guarding non-pda accounts (e.g. an end-users signer key) (**-**)
    4. Allowing program execution that continues on failure might invite users to
implement unforseen side-effects inside the fee entry-point (**---**)
    5. Very hard to calculate fees for a message (**--**)

3. *Passing the application fees in the instruction for each account and validating in the dapp*. A `PayApplicationFee`
instruction is like irrevocable transfer instruction and will do the transfer even if the transaction fails. A new
instruction `CheckApplicationFee` will check if the application fee for an account has been paid.

    1. High degree of flexibility for programs (**++**)
    2. No need to save the application fees on the ledger. (**++**)
    3. Each transaction has to include additional instructions so it will break existing dapp interfaces (**--**)
    4. Account structure does not need to be modified (**++**)
    5. Does not prevent any denial of service attack on a smart contract (**-**)
    6. Each dapp has to implement a cpi to check if a transaction has paid app fees. (**-**)

After discussion with the internal team and Solana teams, we have decided to go ahead with
implementation **1.2** i.e extending the account structure in the ledger to store fee setting.
This decision was made because it will make the application fee a core feature of Solana.
The downside is that we have to change one of the core structures which makes a lot of code changes
but the rent epoch is being deprecated which we can reuse to store application fees.

Recently we have also been discussing **3** which could be easier to implement than **1.2** the only
catch is that each dapp that wants to use this feature has to do additional development. If existing dapps
like openbook want to implement application fees then all the dapps depending on openbook also have to do additional
development. The checks on the application fees will move to dapp side. The plus point is that we
do not have to touch the account structure, and do not have to implement the base fee surcharge stage.
It is also more easier to calculate total fees. I will write more about **3** at the end of the document.

### Base fee surcharge

The main issue with the approach is that the validator will have to load all the accounts for
a transaction before it can decide how much the payer has to pay as fees. And if the fee payer
has insufficient lamports, then the work for loading of accounts is a waste of resources.
Previously with fixed base fees and priority fees the total fees were already detemined, we just
load the payer account and check if it has a sufficient balance. But now we can imagine a
transaction with many accounts and a payer having sufficient amount to pay base fees but not the
application fees, if the account with application fees was set at the end then the validator loads
all the accounts till the end and then realize that payer is unable to pay application fees.
This can be used by an attacker to slow the cluster.


To address this issue we will add additional surcharge for base fees if the transaction uses
any account with application fees but was not available to pay them. It will work as follows.

1. Before loading accounts we check that payer has
`minimum balance = base fees + other fees + base fees * number of accounts` in the transaction.
2. If payer does not have this balance minimum transaction fails.
3. If payer has this balance then we start loading accounts and checking if there are any application fees.
4. If payer does not have enough balance to pay application fees then we charge payer
`total fees = base fees + other fees + base fees * accounts loaded`.
5. If payer has enough balance to pay application fees but `LimitApplicationFees` instruction set amount too low.
`total fees = base fees + other fees + base fees * accounts loaded`.
6. If payer has enough balance then to pay application fee and has included the instruction `LimitApplicationFees`
with sufficient limit in the instruction.
`total fees = base fees + other fees + application fees`.
7. If there is no application fees involved then the payer pays.
`total fees = base fees + other fees`

So this method adds the requirement that payer **MUST** have additional balance of number of accounts * base fees.
With base fees so low we hope that wont be an issue for the user and this additional fees will goes to
the validators and not the dapp account.

The overall fees for transactions without any accounts using this feature **WONT** change.

We start describing the design with a new solana native program.

### A new application fee program

We add a new native solana program called application fees program with program id.

```
App1icationFees1111111111111111111111111111
```

This program will be used to change application fee for an account, to intialize rebates
initaited by the account authority, and a special instruction by which fee payer accepts
amount of lamports they are willing to pay as application fees.

#### LimitApplicationFees Instruction

With this instruction, the fee payer limits to pay application fees specifying the maximum amount.
If the application fee required for the account is more than specified, then the payer has to pay
a base fee surcharge. This instruction **OPTIONAL** be included in the transaction that writes locks
accounts that have implemented this feature.

It requires:
Argument : Maximum application fees intented to pay in lamports (u64).

#### UpdateFees Instruction

This instruction will update application fees for an account.
It requires :
* authority of the writable account as (signer)
* Writable account as (writable)

Argument: updated fees in lamport (u64).

#### Rebate Instruction

This instruction should be called by the dapp using CPI or by the owner of the account.
It requires :
* Authority of the writable account (signer)
* Writable account (writable)

Argument: Number of lamports to rebate (u64) can be u64::MAX to rebate all the fees.

### Changes in the core solana code

These are following changes that we have identified as required to implement this feature.

#### Account structure

Currently account structure is defined as follows:

```Rust
#[repr(C)]
pub struct Account {
    /// lamports in the account
    pub lamports: u64,              // size = 8, align = 8, offset = 0
    /// data held in this account
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,              // size = 24, align = 8, offset = 8
    /// the program that owns this account. If executable, the program that loads this account.
    pub owner: Pubkey,              // size = 32, align = 1, offset = 32
    /// this account's data contains a loaded program (and is now read-only)
    pub executable: bool,           // size = 1, align = 1, offset = 64
    /// the epoch at which this account will next owe rent
    pub rent_epoch: Epoch,          // size = 8, align = 8, offset = 72
}
```

Here we can see that we have 7 bytes of space between executable and rent_epoch.
The rent_epoch is being deprecated and will be eventually removed. So we can reuse the rent epoch
to store application fees as both of them store value as u64. We also add a new field called
`has_application_fees` and rename `rent_epoch` to `rent_epoch_or_application_fees`. If `has_application_fees`
is true then rent_epoch for the account is rent exempt i.e u64::MAX and the application fee is
decided by value of `rent_epoch_or_application_fees`. And if `has_application_fees` is false then
`rent_epoch` is `rent_epoch_or_application_fees` and the application fee is 0.

So we cannot have both the rent epoch and application fees in the same space. We cannot set
application fees for accounts that are not rent-free. As in two years, we won't have any
account which is not rent-free I guess that won't be an issue.

In append_vec.rs AccountMeta is the way an account is stored physically on the disk. We use a similar
concept as above but we do not have extra space to add the `has_application_fees` boolean. Here we have
decided to reuse the space in the `executable` byte to store the value of `has_application_fees`.
So `executable` will be changed to `account_flags` where 1 LSB is `executable` and 2nd LSB is `has_application_fees`.
This change does not impact a lot of code and is very localized to file append_vec.

#### Changes in `load_transaction_accounts`

When we load transaction accounts we have to calculate the application fees, decode the `LimitApplicationFees`
instruction and implement the logic described in the base fee surcharge part above.

#### Changes in invoke context

The structure `invoke context` is passed to all the native solana program while execution. We create
a new structure called `application fee changes` which contains one hashmap mapping application fees
(`Pubkey` -> `application fees(u64)`), another containing rebates (`Pubkey` -> `amount rebated (u64)`)
and a third to store all the updates in application fees (`Pubkey` -> `New application fees (u64)`).
The `application fee changes` structure is already filled with application fees that were decided
while we were loading all the accounts. This new structure we add as a field in invoke structure so
that it can be used by native program `App1icationFees1111111111111111111111111111`.

```Rust
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ApplicationFeeChanges {
    pub application_fees: HashMap<Pubkey, u64>, // To store application fees by account
    pub rebated: HashMap<Pubkey, u64>, // to store rebates by account
    pub updated: Vec<(Pubkey, u64)>, // to store updates by account
}
```

In the application fees program we will just add the new values of application fees in case of
`UpdateFees` instruction. In case of `Rebate` instruction add the rebated value in the relevant
hashmap and remove the same amount from the application fees hash map.

In verify stage we verify that `Old Application Fees` = `New Application Fees` + `Rebates` for each
account.

#### Changes in Bank

In method `filter_program_errors_and_collect_fee` we will add logic to deposit application fees
to the respective mutable accounts and to reimburse the rebates to the payer in case the transaction was
successful. If the transaction fails then we withdraw base fees with application fees.

The updates in the application fees will be stored in a special hashmap and the changes will be
applied at the end of the slot when the bank is freezing. This will effectively make any updates to
the application fees valid from the next slot and not in the current slot.

## Impact

This feature is very intersting for dapps as they can earn application fees from users.
If the dapps want to rebate application fees they have to implement very carefully the logic of rebate.
They should be very meticoulous before calling rebate so that a malicious user could not use this
feature to bypass application fees. Dapp developer also have to implement additional instruction
to collect these fees using lamport transfers.

This could also add additional fees collection for the validator if the transactions are not correctly
formed, like missing `PayApplicationFee` instruction or insuffucient payer balance.

## Security Considerations

If the application fee for an account is set too high then we cannot ever mutate that account anymore.
Even updating the application fees for the account will need a very high amount of balance. This issue
can be easily solved by setting a maximum limit to the application fees.

## Mango V4 Usecase
With this feature implemented Mango-V4 will be able to charge users who spam risk-free aribitrage
or spam liquidations by increasing application fees on perp-markets, token banks
and mango-user accounts.
#### Perp markets
Application fees on perp liquidations, perp place order, perp cancel, perp consume, perp settle fees.
Rebates on : successful liquidations, consume events, HFT marketmaking refresh
(cancel all, N* place POST, no IOC).

#### Token vaults
Application fees on openorderbook liquidations, deposit, withdrawals.
Rebate on successful liquidations, place IOC & fill in isolation, HFT marketmaking 
refresh (cancel all, N* place POST, no IOC).

#### Mango accounts 
Application fees on all liquidity transactions, consume events, settle pnl, all user 
signed transactions.
Rebate on transaction signed by owner or delegate, successful liquidations, 
settlements, consume events.