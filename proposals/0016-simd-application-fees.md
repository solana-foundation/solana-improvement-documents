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

This SIMD will discuss additional fees called Application Fees. These fees are
decided and set by the dapp developer to interact with the dapp. Dapp developers
then can decide to rebate these fees if a user interacts with the dapp as
intended and disincentivize the users which do not. These fees will be applied
even if the transaction eventually fails and are collected on the account on
which they were set. These fees would be only applied if the account is write
locked by the transaction. The owner of the account can do lamport transfer to
recover these fees. So instead of fees going to the validator, these fees go to
the **dapp developers**. It will be dapp developer's responsibility to advertise
the required application fees to its users.

## Motivation

Currently, there is no way for dapp developers to enforce appropriate behavior
and the way their contracts should be used. Bots spamming on dapps make them
unusable and dapps lose their users/clients because the UX is laggy and
inefficient. Unsuccessful transactions deny block space to potentially valid
transactions which reduces activity on the dapp. For dapps like mango markets or
Openbook increasing fees without any rebates or dynamically based other proposed
mechanisms will punish potentially thousands of users because of a handful of
malicious users. Giving dapps authority more control to incentivize proper
utilization of its contract is the primary motivation for this proposal.

During network congestion, many transactions in the network cannot be processed
because they all want to write-lock the same accounts. When a write lock on an
account is taken by a transaction batch no other batch can use this account in
parallel, so only transactions belonging to a single batch are processed
correctly and all others are retried again or forwarded to the leader of the
next slot. With all the forwarding and retrying the validator is choked by the
transactions write-locking the same accounts and effectively processing valid
transactions sequentially.

There are multiple accounts of OpenBook, mango markets, etc which are used in
high-frequency trading. During the event of extreme congestion, we observe that
specialized trading programs write lock these accounts but never CPI into the
respective programs unless they can extract profit, effectively starving the
actual users for access. With current low cluster fees, it incentivizes spammers
to spam the network with transactions.

Without any proper rebate mechanism for users, Solana fees will increase and it
will lose the edge for being low fees cluster. For entities like market makers,
Solana must remain a low-fee cluster as they have a thin profit margin and have
to quote quite often. The goal of this proposal is to reduce spamming without
really increasing fees for everyone. Encourage users to create proper
transactions. Read the cluster state before creating transactions instead of
spamming. We are motivated to improve the user experience, and user activity and
keep Solana a low gas fee blockchain. Keeping gas fees low and increasing user
activity on the chain will help all the communities based on Solana grow. This
will also help users to gain more control over accounts owned by them.

Note that this proposal does not aim to increase gas fees for all transactions
or add any kind of congestion control mechanism.

## Alternatives Considered

* Having a fixed write lock fee and a read lock fee.

Pros: Simpler to implement, Simple to calculate fees.

Cons: Will increase fees for everyone, dapp cannot decide to rebate fees, (or
not all existing apps have to develop a rebate system). Fees cannot be decided
by the dapp developer. Penalizes everyone for a few bad actors.

* Passing application fees for each account by instruction and validating them
  during the execution inside user application code

Pros: High efficiency, minimal performance impact, more generic, no need to
change the account structure.

Cons: Cannot prevent denial of service attacks, or account blocking
attacks that do not call into the respective program

## New Terminology

Application Fees: Fees that are decided by the dapp developer will be charged if
a user wants to use the dapp. They will be applied even if the transaction fails
contrary to lamport transfers. The program can decide to rebate these fees back
to the user if certain conditions decided by dapp developers are met.

### Other terminology

Account `Authority`: The true authority for the account. Let's take an example
of associated token account. A associated token account is a `PDA` derived
from associated token program, where the owner is token program. But the
program internally saves who has the authority over the account. Only token
program can change the data of a token account. For operations like
withdrawing tokens, the authority of the associated token account has to be
the signer for the transaction containing transfer instruction. To receive
tokens from another account, the authority's signature is not required.

`Other Fees`: Fees other than application fees
`base fees + prioritization fees`.

## Detailed Design

Application fees enable dapp developer to decide who to rebate and who to
penalize through runtime features. In particular, programs needs to be able
to penalize, even if the application is not CPI'd to.

The checks on the application fees will be taken care of by Solana runtime.
Total application fees paid will be included in the transaction through a
special `PayApplicationFees` similar to the `ComputeBudget` program so that it
is easier to calculate the total amount of fees that will be paid. In case of
nonpayment or partial payment, the program is never executed. This instruction
cannot be CPI'ed into.

Application developers will be able to rebate these fees through a new runtime
mechanism, but only if the transaction succeeds, this way transactions
write-locking the respective accounts will require full fee payment before
program is executed.

### Payment

When the cluster receives a new transaction, `PayApplicationFees` instruction is
decoded to calculate the total fees required for the transaction message. Then
we verify that the fee-payer has a minimum balance of:
`per-transaction base fees` + `prioritization fees` +
`maximum application fees to be paid`

If the payer does not have enough balance, the transaction is not scheduled
and fails with an error `InsufficientFunds`. Consider this case the same as if
the payer does not have enough funds to pay `other fees`.

Before processing the message, we check if any loaded account has associated
application fees. For all accounts with application fees, the fees paid should
be greater than the required fees. In case of overpayment, the difference is
stored in a variable and will be paid back to the payer in any case. In case the
application fees are insufficiently paid or not paid then the transaction fails
with an error `ApplicationFeesNotPaid`. The payer will pay just `other fees`.
This failure is before the execution of the transaction.

The application fees minus rebates and minus overpayment are transferred to
the respecitve accounts lamport balance, from where the owner can collect them.

### Configuration

Application fees should be considered as set and forget kind of fees by dapp
developers. They are not aimed to control congestion over the network instead
they aim to reduce spamming.

The accounts database will need to store the amount of lamports required for
every account. Hence changes should be constrained like any account write to the
owner of the account. All programs will need to implement special instructions
so that the authority of the accounts can sign to update application fees on the
accounts.

A new syscall and sysvar should be added to set application fees; syscall to
support current runtime and sysvar for future runtimes when syscalls would be
deprecated. Setting the amount to `0` disables application fees. The maximum
amount of application fees that can be set for an account will be limited to a
constant amount of SOL so that accounts cannot become inaccessible through
simple programming errors. The maximum limit should be U32::MAX which comes
about to be 4.294967296 SOLs which should be affordable for most dapps incase of
human errors setting application fees, while at the same time punishing
malicious write locks on highly read-locked accounts. This limit will have added
advantage of requiring only 4 bytes to store application fees.

### Rebate

The owner of the account can issue a rebate of the application fees paid to
write lock a specific account. Similar to the configuration this will need to be
exposed through a syscall and sysvar so that programs can implement custom
authorization and delegate the decision to composing programs.

Simple rebate schemes will verify merely signers, e.g an oracle preventing 3rd
parties from write-locking their price feed. Dexes will need more complex rebate
schemes based on instruction sysvar introspection.

Rebate takes the amount of lamports to be rebated, and account on which rebate
is issued as input. In case of multiple rebates from the same account only the
highest amount of rebate will be taken into account. The rebated amount is
always the minimum of rebate issued by the program and the application fees on
the account. If program rebates `U64::MAX` it means all the application fees on
the account are rebated. The rebate amount cannot be negative.

### Looking at common cases

#### No application fees enabled

* A payer does not include `PayApplicationFees` in the transaction. The
  transaction does not write lock any accounts with application fees. Then the
  transaction is executed without the application fee feature. The payer ends up
  paying other fees.

* A payer includes `PayApplicationFees(app fees)` in the transaction but none of
  the accounts have any application fees. This case is considered an overpay
  case. The payer balance is checked for `other fees + app fees`.
  1. The payer does not have enough balance: Transaction fails with an error
    `Insufficient Balance` and the transaction is not even scheduled for
    execution.
  2. The payer has enough balance then the transaction is executed and application
    fees paid are transferred back to the payer in any case.
  
  Note in this case
    even if there are no application fees involved the payer balance is checked
    against application fees.

#### Application fees are enabled

* Fees not paid case:

  A payer does not include `PayApplicationFees` in the transaction. The
  transaction includes one or more accounts with application fees. Then the
  transaction is failed with an error `ApplicationFeesNotPaid`. The program is
  not executed at all. The payer ends up paying only other fees.

* Fees paid no rebates case:

  A payer includes instruction `PayApplicationFees(100)` in the transaction.
  There is an account `accA` which is write-locked by the transaction and it has
  an application fee of `100` lamports. Consider that the program does not have
  any rebate mechanism. Then in any case (execution fails or succeeds) `accA`
  will receive `100` lamports. The payer will end up paying `other fees` + `100`
  lamports.

* Fees paid full rebates case:

  A payer includes instruction `PayApplicationFees(100)` in the transaction.
  There is an accounts `accA` which is write-locked by the transaction and it
  has an application fee of `100` lamports. Consider during execution the
  program will rebate the application fee on the account. Then payer should have
  a minimum balance of `other fees` + `100` lamports to execute the transaction.
  After successful execution of the transaction, the `100` lamports will be
  rebated by the program and then Solana runtime will transfer them back to the
  payer. So the payer will finally end up paying `other fees` only.

* Fees paid multiple partial rebates case:

  A payer includes instruction `PayApplicationFees(200)` in the transaction. The
  transaction has three instructions (`Ix1`, `Ix2`, `Ix3`). There are accounts
  (`accA`, `accB`) that are write-locked by the transaction and each of them has
  an application fee of `100` lamports. Lets consider `Ix1` rebates 25 lamports
  on both accounts, `Ix2` rebates 75 lamports on `accA` and `Ix3` rebates 10
  lamports on `accB`. In the case of multiple rebates only the maximum of all
  the rebates is applied. Consider the transaction is executed successfully. The
  maximum of all the rebates for `accA` is 75 lamports and `accB` is 25
  lamports. So a total of 100 lamports are rebated back to the payer, `accA`
  gets 25 lamports and `accB` gets 75 lamports. The payer will end up paying
  `other fees` + `100` lamports.

* Fees paid full rebates but the execution failed case:

  A payer includes instruction `PayApplicationFees(100)` in the transaction.
  There is an account `accA` which is write-locked by the transaction and it has
  an application fee of `100` lamports. Consider during execution the program
  will rebate all the application fees on the account but later the execution
  failed. Then payer should have a minimum balance of `other fees` + `100`
  lamports to execute the transaction. The program rebated application fees but
  as executing the transaction failed, no rebate will be issued. The application
  fees will be transferred to respective accounts, and the payer will finally
  end up paying `other fees` + `100` lamports as application fees.

* Fees are over paid case:

  A payer includes instruction `PayApplicationFees(1000)` in the transaction.
  There is an account `accA` that is write-locked by the transaction and it has
  an application fee of `100` lamports. The minimum balance required by payer
  will be `other fees` + `1000` lamports as application fees. So the payer pays
  100 lamports for the account as application fees and 900 lamports is an
  overpayment. The 900 lamports will be transferred back to the user even if the
  transaction succeeds or fails. The 100 lamports will be transferred to `accA`
  in all cases except if the transaction is successful and the program issued
  a rebate.

* Fees underpaid case:

  A payer includes instruction `PayApplicationFees(150)` in the transaction.
  There is an accounts `accA` that is write-locked by the transaction and it has
  an application fees of `300` lamports. The minimum balance required by payer
  will be `other fees` + `150` lamports as application fees to load accounts and
  schedule transactions for execution. Here payer has insufficiently paid the
  application fees paying 150 lamports instead of 300 lamports. So before
  program execution, we detect that the application fees are not sufficiently
  paid and execution fails with the error `ApplicationFeesNotPaid` and the
  partially paid amount is transferred back to the payer. So the payer pays only
  `base fees` in the end but the transaction is unsuccessful.

## Impact

Overall this feature will incentivize the creation of proper transactions and
spammers would have to pay much more fees reducing congestion in the cluster.
This will add very low calculation overhead on the validators. It will also
enable users to protect their accounts against malicious read and write locks.
This feature will encourage everyone to write better-quality code to help
avoid congestion.

It is the dapp's responsibility to publish the application fee required for each
account and instruction. They should also add appropriate `PayApplicationFee`
instruction in their client library while creating transactions or provide
visible API to get these application fees. We expect these fees to be set and
forget kind of fees and do not expect them to be changed frequently. Some
changes have to be done in web3.js client library to get application fees when
we request the account. Additional instructions should be added to the known
programs like Token Program, to enable this feature on the TokenAccounts. The
dapp developer have to also take into account application fees on the programs
they are dependent on.

The cluster is currently vulnerable to a different kind of attack where an
adversary with malicious intent can block its competitors by writing or
read-locking their accounts through a transaction. This attack involves
carrying out intensive calculations that consume a large number of
computational units, thereby blocking competitors from performing MEV during
that particular block, and giving the attacker an unfair advantage. We have
identified specific transactions that perpetrate this attack and waste
valuable block space. The malicious transaction write locks multiple token
accounts and consumes 11.7 million CU i.e around 1/4 the block space. As a
result, such attacks can prevent users from using their token accounts,
vote accounts, or stake accounts, and dapps from utilizing the required
accounts. With the proposed solution, every program, such as the token
program, stake program, and vote program, can include instructions to employ
the application fees feature on their accounts and rebate the fees if the user
initiates the transaction. The attacker will find this option unfeasible as
they will consume their SOL tokens more rapidly to maintain the attack.

## Security Considerations

If the application fee for an account is set too high then we cannot ever
mutate that account anymore. Even updating the application fees for the
account will need a very high amount of balance. This issue can be easily
solved by setting a maximum limit to the application fees.

For an account that has collected application fees, to transfer these fees
collected to another account we have to pay application fees to write lock the
account, we can include a rebate in the transaction. In case of
any bugs, while transferring application fees from the account to the
authority, there can be an endless loop where the authority creates a
transaction to recover collected application fees, with an instruction to pay
application fees to modify the account and an instruction to rebate. If the
transaction fails because of the bug, the user fails to recover collected
fees, in turn increasing application fees collected on the account.

## Backwards Compatibility

This feature does not introduce any breaking changes. The transaction without
using this feature should work as it is. To use this feature supermajority of
the validators should move to a branch that implements this feature.
Validators that do not implement this feature cannot replay the blocks with
transactions using application fees they also could not validate the block
including transactions with application fees.


## Additional Notes

If the dapps want to rebate application fees they have to implement very
carefully the logic of rebate. They should be very meticulous before calling
rebate so that a malicious user could not use this feature to bypass
application fees. Dapp developers also have to implement additional
instruction to collect these fees using lamport transfers.

Dapp developers have to consider the following way to bypass application fees
is possible: A Defi smart contract with two instructions IxA and IxB. Both IxA
and IxB issue a rebate. IxA is an instruction that places an order on the
market which can be used to extract profit. IxB is an instruction that just
does some bookkeeping like settling funds overall harmless instruction.
Malicious users then can create a custom smart contract to bypass the
application fees where it CPI's IxA only if they can extract profit or else
they use IxB to issue a rebate for the application fees. So dapp developers
have to be sure when to do rebates usually white listing and black listing
instruction sequence would be ideal.

Dapp should be careful before rolling out this feature. Because the
transaction would start failing if the rollout is sudden. It is preferable to
implement rebate, and add pay application fees in their APIs, so that the user
pays full application fee but is then rebated if the transaction succeeds.
Then once everyone starts using the new API they can add the check on
application fees.

Another proposal will also introduce protection against unwanted read-locking
of the accounts. Many accounts like token account rarely need to be read-locked
this proposal will force these accounts to be write-locked instead and pay
application fees if needed. This feature is out of the scope of this proposal.

### Calculating Application Fees for a dapp

Let us consider setting application fees for Openbook DEX. We can set fees
comparable to the rent of the accounts involved or something fixed. Setting
application fees too high means dapp users need more balance to interact with
the dapps and if they are too low then it won't prevent spamming or malicious
use. In case of openbook the intent is to avoid spamming.

Most of the OpenBook accounts like asks, bids and event queues are used in
write mode only we can disable read-locks on these accounts. Now we can
consider there are 48 M CU's per block and 2.5 blocks per second. Considering
each instruction takes 200K CUs so around 600 transactions per second.
Currently, base fees are around 0.00005 SOLs, with so low gas fees spammers
have the advantage to spam the cluster. A reasonable application fee could be
around 0.1 SOLs per transaction that could be distributed among different
accounts. For a user interacting with dapp with 0.1 SOLs in the account seems
reasonable assuming that the transactions are executed successfully and the
fees are rebated. This will make spammers spend their SOLs 2000 times more
rapidly than before. The thumb rule for dapps to set application fees on their
accounts is `More important the account = Higher application fees`.

### Pyth Usecase

Pyth's price feeds currently serve around 33M CU peak read load. An attacker
could write lock those for 12M CU and cause scheduling issues by crowding out
the majority of the block. A high application fee of 10 SOL could prevent anyone
except price feed publishers from write locking a price feed account.

### Mango V4 Usecase

With this feature implemented Mango-V4 will be able to charge users who spam
risk-free arbitrage or spam liquidations by increasing application fees on
perp-markets, token banks and mango-user accounts.

#### Perp markets

Application fees on perp liquidations, perp place order, perp cancel, perp
consume, perp settle fees. Rebates on: successful liquidations, consume
events, HFT market making refresh (cancel all, N* place POST, no IOC).

#### Token vaults

Application fees on open order book liquidations, deposit, withdrawals. Rebate
on successful liquidations, place IOC & fill in isolation, HFT marketmaking
refresh (cancel all, N* place POST, no IOC).

#### Mango accounts

Application fees on all liquidity transactions, consume events, settle pnl,
all user signed transactions. Rebate on transaction signed by owner or
delegate, successful liquidations, settlements, consume events.
