---
simd: '0016'
title: Application Fees
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

This SIMD will discuss additional fees called Application Fees.
These fees are decided and set by the dapp developer to interact with the dapp.
Dapp developers then can decide to rebate these fees if a user interacts with the dapp
as intended and disincentivize the users which do not.
These fees will be applied even if the transaction eventually fails and collected on
the same writable account.
Account authority (i.e owner of the account) can do lamport transfers to recover these fees.
So instead of fees going to the validator these fees go to the **dapp developers**.
It will be dapp developer's responsibility to advertise the required application fees to its
users.

Discussion for the issue : <https://github.com/solana-labs/solana/issues/21883>

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

Currently, there is no way for dapp developers to enforce appropriate behavior and the way their
contracts should be used. Bots spamming on dapps make them unusable and dapps lose their
users/clients because the UX is laggy and inefficient. Validators gain base fees and 
prioritization fees even if the transactions are executed unsuccessfully but unsuccessful 
transactions deny block space to potentially valid transactions which reduces activity on the dapp.
For dapps like mango or openbook increasing fees without any rebates or dynamically based other
proposed mechanisms will punish potentially thousands of users because of a handful of malicious
users. Giving dapp's authority more control to incentivize proper utilization of its contract is the
primary motivation for this proposal.

Adding dynamic fees per account penalizes the dapp and its user. Without proper rebates to
users Solana gas fees will increase and it will lose the edge for being low fees cluster. If the
dynamic gas fees are low then it won't solve the spamming issues. Either way, dynamic fees will
not be covered by this proposal because for users can't tell beforehand how much fees will be paid
for the transaction they have sent. This will make the cluster not interesting for required players
like market makers for whom profit margins are thin and a dynamic cluster will make it impossible
to predict the outcome.

We are motivated to improve the user experience, and user activity and keep Solana a low gas fee
blockchain. Keeping gas fees low and increasing user activity on the chain will help all the
communities based on Solana grow.

## Alternatives Considered

* Having a fixed write lock fee and a read lock fee.

Pros: Simpler to implement, Simple to calculate fees.

Cons: Will increase fees for everyone, dapp cannot decide to rebate fees, (or not all existing
apps have to develop a rebate system). Fees cannot be decided by the dapp developer.

* Extend existing account structure in ledger to store fee settings and collect lamports.

Pros: High efficiency, minimal performance impact, no need to add additional instruction in transaction,
helps to avoid denial of service attack on the smart contract.

Cons: Modification of account structure which impacts a lot of code. Needs to load all accounts to calculate
total fees to charge the payer, to avoid that have to implement multiple level of fees.

## New Terminology

Application Fees: Fees that are decided by the dapp developer that will be charged if a user wants
to use the dapp. They will be applied even if transaction fails contrary to lamport transfers.

## Detailed Design

As a high-performance cluster, we want to incentivize players which feed the network
with responsibly behaved transactions instead of spamming the network. The introduction
of application fees would be an interesting way to penalize the bad actors and dapps
can rebate these fees to the good actors. This means the dapp developer has to decide who to
rebate and who to penalize special instructions. In particular, it needs to be able to
penalize, even if the application is not CPI'd to. There were multiple possible approaches
discussed to provide the application with access to transfer lamports outside of the regular cpi
execution context. The following approach seems the best.

*Passing the application fees in the instruction for each account and validating in the dapp*. A `PayApplicationFee`
instruction is like infallible transfer instruction and will do the transfer even if the transaction fails. A new
instruction `CheckApplicationFee` will check if the application fee for an account has been paid.

    1. High degree of flexibility for programs (**++**)
    2. No need to save the application fees on the ledger. (**++**)
    3. Each transaction has to include additional instructions so it will break existing dapp interfaces (**-**)
    4. Account structure does not need to be modified (**++**)
    5. Does not prevent any denial of service attack on a smart contract (**-**)
    6. Each dapp has to implement a cpi to check if a transaction has paid app fees

If existing dapps like openbook want to implement application fees then all the dapps depending on openbook
also have to do additional development. The checks on the application fees will be responsibility of dapp developers.
The plus point is that we do not have to touch the core solana structure to store the application fees.
It is also more easier to calculate total fees.

### A new application fee program

We add a new native solana program called application fees program with program id.

```
App1icationFees1111111111111111111111111111
```

This native program will be used to check application fee for an account, to intialize rebates
initaited by the account authority, and a special instruction by which fee payer accepts
amount of lamports they are willing to pay as application fees per account.

#### PayApplicationFees Instruction

With this instruction, the fee payer accepts to pay application fees specifying the amount.
This instruction **MUST** be included in the transaction that interacts with dapps having application fees.
This instruction is like an infallible transfer if the payer has enough funds
i.e even if the transaction fails the payer will end up paying.
If this instruction is added then even if the dapp does not check the application fees the payer ends
up paying.
If the payer does not have enough balance the transaction fails with error `InsufficientFunds`.
This instruction is decoded in the `calculate_fee` method of `bank.rs` where other fees are calculated and the sum
of the fees on all the accounts are deducted from the payer. The resulting map for (Accounts -> Fees) are passed
into invoke context and execution results. If the transaction is successfully executed then rebates are returned to the
payer and the remaining fees are transferred to respective accounts if the transaction fails to execute then the
fees are transferred to respective accounts without any rebates, this will happen in `filter_program_errors_and_collect_fee`
stage in `bank.rs`.

It requires:

Accounts :

* List of accounts that need an application fees.

Argument : Corresponding list of application fees for each account in `Accounts`.

The arguments list must be of same length as number of accounts.
The index of fees and account should match.
The account fees for each account is 8 bytes represented in rust type `u64`

#### CheckApplicationFees Instruction

This instruction will check if an application fee for an account is paid.
It requires :

* Account where fees are paid.

Argument: required fees in lamport (u64).

If application fees are not paid or are paid insufficiently this instruction will return
an error. The idea is dapp developer uses this instruction to check if the required fees
are paid and fail the transaction if they are not paid or partially paid.
In case of partial payment, the user will lose the partially paid amount.
A payer may overpay for the fees. This instruction can be called multiple times across multiple instructions.
Internally it checks if curresponding fees are present in the map calculated in `calculate_fee` stage.

#### Rebate Instruction

This instruction should be called by the dapp using CPI or by the owner of the account.
It requires :

* Account on which a fee was paid
* Authority (owner) of the account (signer)

Argument: Number of lamports to rebate (u64) can be u64::MAX to rebate all the fees.

The authority or the owner could be easily deduced from the `AccountMeta`. In case of PDA's
usually account and owner are the same (if it was not changed), then `invoke_signed` can be used
to issue a rebate.
In case of multiple rebate instructions, only the maximum rebate will one will be issued.
Payer has to pay full application fees initially even if they are eligible for a rebate.
There will be no rebates if the transaction fails even if the authority had rebated the fees back.
If there is no application fee associated with the account we ignore the rebate instruction.

### Changes in the core solana code

These are following changes that we have identified as required to implement this feature.

#### Changes in `load_transaction_accounts`

When we load transaction accounts we have to calculate the application fees by decoding the `PayApplicationFees`
instruction. Then we verify that fee payer has miminum balance of:
`per-transaction base fees + prioritization fees + sum of application fees on all accounts`

If the payer has sufficient balance then we continue loading other accounts. If `PayApplicationFees` is missing
then application fees is 0. If payer has insufficient balance transaction fails with error `Insufficient Balance`.

#### Changes in invoke context

The structure `invoke context` is passed to all the native solana program while execution. We create
a new structure called `ApplicationFeeChanges` which contains one hashmap mapping application fees
(`Pubkey` -> `application fees(u64)`), another containing rebates (`Pubkey` -> `amount rebated (u64)`).
The `ApplicationFeeChanges` structure is already filled with application fees in the stage `load_transaction_accounts`.

```rust
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ApplicationFeeChanges {
    pub application_fees: HashMap<Pubkey, u64>, // To store application fees by account
    pub rebated: HashMap<Pubkey, u64>, // to store rebates by account
}
```

`PayApplicationFees` will add the accounts specified in the `application_fees` field of this structure.
Each time `CheckApplicationFees` is called we just check that the account is present in the map `application_fees`
and that the amount passed in the instruction is `<=` value in the `application_fees` map.
On each `Rebate` instruction we find the minimum between `application_fees` and the rebate amount for the account.
If there is already a rebate in the `rebated` map then we update it by
`max(rebate amount in map, rebate amount in instruction)`, if the
map does not have any value for the account, then we add the rebated amount in the map.

In verify stage we verify that `Application Fees` >= `Rebates` for each account and return `UnbalancedInstruction` on failure.

#### Changes in Bank

In method `filter_program_errors_and_collect_fee` we will add logic to deposit application fees
to the respective mutable accounts and to reimburse the rebates to the payer in case the transaction was
successful. If the transaction fails then we withdraw
`per-transaction base fees + prioritization fees + sum of application fees on all accounts`
from the payer.
We also transfer the application fees to the repective accounts in this stage.

## Impact

If the dapps want to rebate application fees they have to implement very carefully the logic of rebate.
They should be very meticoulous before calling rebate so that a malicious user could not use this
feature to bypass application fees. Dapp developer also have to implement additional instruction
to collect these fees using lamport transfers.

DApp developers have to consider the following way to bypass application fees is possible:
A Defi smart contract with two instructions IxA and IxB. Both IxA and IxB issue a rebate.
IxA is an instruction that places an order on the market which can be used to extract profit.
IxB is an instruction that just does some bookkeeping like settling funds overall harmless instruction.
Malicious users then can create a custom smart contract to bypass the application fees where it CPI's IxA
only if they can extract profit or else they use IxB to issue a rebate for the application fees.
So DApp developers have to be sure when to do rebates usually white listing and black listing instruction sequence
would be ideal.

A dapp can break the cpi interface with other dapps if it implements this feature. This is because
it will require additional account for application fees program to all the instructions which calls
`CheckApplicationFees` or `Rebate`, interface also has to add an additional instruction `PayApplicationFees`
to the correct account.

It is the DApp's responsibility to publish the application fee required for each account and instruction.
They should also add appropriate `PayApplicationFee` instruction in their client library while creating transactions
or provide visible API to get these application fees. As DApp has to be redeployed to change application fees,
we do not expect to change it often.

Overall this feature will incentivise creation of proper transaction and spammers would have to pay
much more fees reducing congestion in the cluster. This will add very low calculation overhead on the validators.

## Security Considerations

User could overpay (more than required by dapp) application fees, if the `CheckApplicationFees` instruction
has amount more than the application fee required by dapp.

Denial of service attack for a dapp is possible by flooding the cluster with a lot of transactions write-locking
an account used by the dapp. This attack is already possible on the network.
This implementation of application fees does not protect the dapp from denial of service
attacks. An attacker can always flood the network with the transactions without the `CheckApplicationFees` instruction.
None of these transactions will be executed successfully but the write lock on the account was taken without any
payment of application fees.

To solve this kind of attack the application fees should be stored in the ledger but this involves a lot of changes in
the solana core code. And this kind of attack can only affect one dapp at a time, attacker has to burn a lot of gas
fees to sustain for a very long time.

## Backwards Compatibility

This feature does not introduce any breaking changes. The transaction without using this feature should
work as it is.
To use this feature supermajority of the validators should move to a branch which implements this feature.
Validators which do not implement this feature cannot replay the blocks with transaction using application fees.

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
