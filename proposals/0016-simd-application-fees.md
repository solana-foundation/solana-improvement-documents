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
decided and set by the dapp developer to interact with the dapp. Dapp
developers then can decide to rebate these fees if a user interacts with the
dapp as intended and disincentivize the users which do not. These fees will be
applied even if the transaction eventually fails and collected on the same
writable account. Owner of the account can do lamport transfers to recover
these fees. So instead of fees going to the validator these fees go to the
**dapp developers**. It will be dapp developer's responsibility to advertise
the required application fees to its users.

Discussion for the issue : <https://github.com/solana-labs/solana/issues/21883>

## Motivation

During network congestion, many transactions in the network cannot be
processed because they all want to write-lock same accounts. When a write lock
on an account is taken by a transaction batch no other batch can use this
account in parallel, so only transactions belonging to a single batch are
processed correctly and all others are retried again or forwarded to the
leader of the next slot. With all the forwarding and retrying the validator is
choked by the transactions write-locking the same accounts and effectively
processing valid transactions sequentially.

There are multiple accounts of OpenBook (formerly serum), mango markets, etc
which are used in high-frequency trading. During the event of extreme
congestion, we observe that specialized trading programs write lock these
accounts but never CPI into the respective programs unless they can extract
profit, effectively starving the actual users for access. Penalizing this
behavior with runtime fees collected by validators can create a perverse
incentive to artificially delay HFT transactions, cause them to fail, and
charge penalty fees.

Currently, there is no way for dapp developers to enforce appropriate behavior
and the way their contracts should be used. Bots spamming on dapps make them
unusable and dapps lose their users/clients because the UX is laggy and
inefficient. Validators gain base fees and prioritization fees even if the
transactions are executed unsuccessfully but unsuccessful transactions deny
block space to potentially valid transactions which reduces activity on the
dapp. For dapps like mango or openbook increasing fees without any rebates or
dynamically based other proposed mechanisms will punish potentially thousands
of users because of a handful of malicious users. Giving dapp's authority more
control to incentivize proper utilization of its contract is the primary
motivation for this proposal.

Adding dynamic fees per account penalizes the dapp and its user. Without
proper rebates to users Solana gas fees will increase and it will lose the
edge for being low fees cluster. If the dynamic gas fees are low then it won't
solve the spamming issues. Either way, dynamic fees will not be covered by
this proposal because for users can't tell beforehand how much fees will be
paid for the transaction they have sent. This will make the cluster not
interesting for required players like market makers for whom profit margins
are thin and a dynamic cluster will make it impossible to predict the outcome.

The cluster is currently vulnerable to a different kind of attack where an
adversary with malicious intent can block its competitors by writing or
read-locking their accounts through a transaction. This attack involves
carrying out intensive calculations that consume a large number of
computational units, thereby blocking competitors from performing MEV during
that particular block, and giving the attacker an unfair advantage. We have
identified specific transactions that perpetrate this attack and waste
valuable block space. The malicious transaction write locks multiple token
accounts and consumes 11.7 million CU i.e around 1/4 the block space. As a
result, such attacks can prevent users from using their own token accounts,
vote accounts, or stake accounts, and dapps from utilizing the required
accounts. With the proposed solution, every program, such as the token
program, stake program, and vote program, can include instructions to employ
the application fees feature on their accounts and rebate the fees if the user
initiates the transaction. The attacker will find this option unfeasible as
they will consume their SOL tokens more rapidly to maintain the attack.

We are motivated to improve the user experience, and user activity and keep
Solana a low gas fee blockchain. Keeping gas fees low and increasing user
activity on the chain will help all the communities based on Solana grow. This
will also help users to gain more control over accounts owned by them.

## Alternatives Considered

* Having a fixed write lock fee and a read lock fee.

Pros: Simpler to implement, Simple to calculate fees.

Cons: Will increase fees for everyone, dapp cannot decide to rebate fees, (or
not all existing apps have to develop a rebate system). Fees cannot be decided
by the dapp developer.

* Passing application fees for each account by instruction and validating them
  during the execution

Pros: High efficiency, minimal performance impact, more generic, no need to
change the account structure.

Cons: Cannot prevent denial of service attacks, or account blocking attacks.

## New Terminology

Application Fees: Fees that are decided by the dapp developer will be charged
if a user wants to use the dapp. They will be applied even if the transaction
fails contrary to lamport transfers.

## Other terminology

`Owner` of an account: It is the account owner as specified in the `owner`
field in the account structure. In the case of an externally owned account
(i.e, keypair) owner is a `system program` and in case of a Program derived
address owner is usually the program.

Account `Authority`: The true authority for the account. Let's take an example
of associated token account. A associated token account is a `PDA` derived
from associated token program, where the owner is token program. But the
program internally saves who has the authority over the account. Only token
program can change the data of a token account. For operations like
withdrawing tokens, the authority of the associated token account has to be
the signer for the transaction containing transfer instruction. To receive
tokens from another account, the authority's signature is not required.

## Detailed Design

As a high-performance cluster, we want to incentivize players which feed the
network with responsibly behaved transactions instead of spamming the network.
The introduction of application fees would be an interesting way to penalize
the bad actors and dapps can rebate these fees to the good actors. This means
the dapp developer has to decide who to rebate and who to penalize special
instructions. In particular, it needs to be able to penalize, even if the
application is not CPI'd to. There were multiple possible approaches discussed
to provide the application with access to transfer lamports outside of the
regular cpi execution context. The following approach seems the best.

*Updating core account structure to store application fees in ledger*. A
`PayApplicationFee` instruction is will be used by solana runtime to calculate
how much application fees are being paid by the transaction. A
`UpdateApplicationFees` instruction will update the application fees for an
account. A `Rebate` instruction will be used to rebate the application fees
back to the payer.

    1. High degree of flexibility for programs (**++**)
    2. Each transaction has to include additional instructions so it will
       break existing dapp interfaces (**-**)
    3. This implementation will prevent DOS on dapps and account blocking
       attacks (**+++**).
    4. Account structure needs to be modified, lot of changes in core Solana
       code (**---**)
    5. Application fees are checked before executing the dapps (**++**)
    6. Easier to calculate total fees to be paid by the payer (**+**)
    7. Application fees cannot be dynamic (**-**)
    8. Will be used to disable read locks on any accounts (**+**)

An additional option will be added to disable read-locking of accounts so that
an account having application fees could not be read-locked by any
transaction. This will disable attacks where transaction read locks an account
and prevents other transactions from write-locking the account.

If existing dapps like openbook want to implement application fees then all
the dapps depending on openbook also have to do additional development. The
checks on the application fees will be taken care by solana runtime. Total
application fees paid will be included in the transaction so that it is easier
to calculate the total amount of fees that will be paid and less scope for
fraud. The maximum amount of application fees that can be set for an account
will be limited to a predecided number of SOLs recommended (100 SOLs) so that
account does not become inaccessible.

All other programs have to implement required instructions so that the
authority of the accounts can cyclically sign to update application fees on
the accounts they own.

### A new application fee program

We add a new native solana program called application fees program with
program id.

```
App1icationFees1111111111111111111111111111
```

This native program will be used to update application fee for an account, to
initialize rebates initaited by the account authority and special instruction
by which fee payer accepts the maximum amount of lamports they are willing to
pay as application fees for the transaction.

#### PayApplicationFees Instruction

With this instruction, the fee payer accepts to pay application fees
specifying the maximum amount. This instruction **MUST** be included in the
transaction that interacts with dapps having application fees. This
instruction is like an infallible transfer if the payer has enough funds i.e,
even if the transaction fails, the payer will end up paying the required
amount of application fees. If the payer does not have enough balance, the
transaction fails with the error `InsufficientFunds`. This instruction is
decoded in the `calculate_fee` method of `bank.rs` where other fees are
calculated, and the sum of all the fees is deducted from the payer. The
(Accounts -> Fees) map will be created from loaded accounts and passed into
invoke context and execution results. Before executing the transaction, we
will check if enough application fees is paid to coverall loaded accounts. In
case of missing instruction or insufficient fees paid, the transaction will
fail, citing `ApplicationFeesNotPaid` error. If the transaction read locks an
account on which read lock has been disabled then the transaction fails with
an error `ReadLockDisabled`. If the transaction fails at this stage the payer
will end up paying `base fees + prioritization fees + application fees`. If
the transaction is successfully executed, then rebates are returned to the
payer and the remaining fees are transferred to respective accounts. If the
transaction fails to execute, the fees are transferred to respective accounts
without any rebates.

If payer has overpaid the application fees then after paying all application
fees the remaining amount will be returned to the payer even if the
transaction fails. In case of partial payment, the user will lose the
partially paid amount. To avoid burning application fees or creating new
accounts, we add a constraint that accounts on which application fees are paid
must exist and write-locked. Suppose we pay application fees on an account
that does not exist. In that case, the payer will end up paying
`base fees + prioritization fees + application fees on existing accounts`, and
the transaction will fail. This instruction cannot be CPI'ed into.

It requires:

Accounts :

* None
Argument: Maximum application fees to be paid in lamports as `u64`

#### UpdateApplicationFees Instruction

This instruction will update application fees for an account.
It requires :

* Writable account as (writable)
* Owner of the writable account as (signer)

Argument: fees in lamport (u64), disable read lock (boolean) by default false.

This instruction will set the application fees for an account in the `Account`
structure. It will also update if the read locks on the account should be
disabled in the `Account` structure. Before executing transactions Solana
runtime will check the data set by this instruction for an account against
application fees paid by the instruction. The account must already exist and
should be rent-free to change its application fees. Application fees cannot be
updated on externally owned accounts, i.e accounts where system program is the
owner of the account.

#### Rebate Instruction

This instruction should be called by the dapp using CPI or by the owner of the
account. It requires :

* Account on which a fee was paid
* Owner of the account (signer)

Argument: Number of lamports to rebate (u64) can be u64::MAX to rebate all the
fees.

The owner could be easily deduced from the `AccountMeta`. In case of PDA's
usually account and owner are the same (if it was not changed), then
`invoke_signed` can be used to issue a rebate. In case of multiple rebate
instructions, only the maximum rebate will one will be issued. Rebates on the
same accounts can be done in multiple instructions only the maximum one will
be issued. Payer has to pay full application fees initially even if they are
eligible for a rebate. There will be no rebates if the transaction fails even
if the authority had rebated the fees back. If there is no application fee
associated with the account rebate instruction does not do anything.

The existing programs could integrate cyclic signing to implement this
feature. For instance, token program can include a rebate instruction that
necessitates the token account authority's signature. Therefore, while
write-locking their token account to perform fund transfers, the authority can
add rebate instruction to initiate a rebate to itself.

### Changes in the core Solana code

These are following changes that we have identified as required to implement
this feature.

Currently account structure is defined as follows:

```Rust
#[repr(C)]
pub struct Account {
    /// lamports in the account
    pub lamports: u64,              // size = 8, align = 8, offset = 0
    /// data held in this account
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,              // size = 24, align = 8, offset = 8
    /// the program that owns this account. If executable,
    /// the program that loads this account.
    pub owner: Pubkey,              // size = 32, align = 1, offset = 32
    /// this account's data contains a loaded program (and is now read-only)
    pub executable: bool,           // size = 1, align = 1, offset = 64
    /// the epoch at which this account will next owe rent
    pub rent_epoch: Epoch,          // size = 8, align = 8, offset = 72
}
```

Here we can see that we have 7 bytes of space between executable and
rent_epoch. The rent_epoch is being deprecated and will be eventually removed.
So we can reuse the rent epoch to store application fees as both of them store
value as u64. We also add a new field called `has_application_fees` and rename
`rent_epoch` to `rent_epoch_or_application_fees`. If `has_application_fees` is
true then rent_epoch for the account is rent exempt i.e u64::MAX and the
application fee is decided by value of `rent_epoch_or_application_fees`. And
if `has_application_fees` is false then `rent_epoch` is
`rent_epoch_or_application_fees` and the application fee is 0.

So we cannot have both the rent epoch and application fees in the same space.
We cannot set application fees for accounts that are not rent-free. As in two
years, we won't have any account which is not rent-free I guess that won't be
an issue.

We will also add a boolean `disable_read_locks` after `has_application_fees`
boolean. This boolean can also be set by the `UpdateApplicationFees`
instruction. If true then no transaction can take a read lock on the account.
If any transaction takes the read lock then it will fail with
`ReadLockDisabled` error before the execution.

In append_vec.rs AccountMeta is the way an account is stored physically on the
disk. We use a similar concept as above but we do not have extra space to add
the `has_application_fees` boolean. Here we have decided to reuse the space in
the `executable` byte to store the value of `has_application_fees` and
`disable_read_locks`. So `executable` will be changed to `account_flags` where
1 LSB is `executable` and 2nd LSB is `has_application_fees` and 3rd LSB is
`disable_read_locks`. This change does not impact a lot of code and is very
localized to file append_vec.

The new account structure will look like this:

```Rust
#[repr(C)]
pub struct Account {
    /// lamports in the account
    pub lamports: u64,              // size = 8, align = 8, offset = 0
    /// data held in this account
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,              // size = 24, align = 8, offset = 8
    /// the program that owns this account. If executable, 
    /// the program that loads this account.
    pub owner: Pubkey,              // size = 32, align = 1, offset = 32
    /// this account's data contains a loaded program (and is now read-only)
    pub executable: bool,           // size = 1, align = 1, offset = 64
    /// this account's has application fees,
    /// if true value of the application fees is rent_epoch_or_application_fees
    /// if true the account is rent free
    pub has_application_fees: bool  // size = 1, align = 1, offset = 65
    /// transactions cannot take read lock on this account.
    /// They have to take write locks.
    pub disable_read_locks: bool    // size = 1, align = 1, offset = 66
    /// the epoch at which this account will next owe rent
    pub rent_epoch_or_application_fees: Epoch,// size = 8,align = 8,offset = 72
}
```

#### Changes in `MessageProcessor::process_message`

Before processing the message, we check if a loaded account has associated
application fees. For all accounts with application fees, the fees paid should
be greater than the required fees. In case of overpayment, the difference is
stored in a variable. In case the application fees are insufficiently paid or
not paid, then we set the transaction status as errored. If there was a read
lock taken on an account where the read lock has been disabled, then we set
the transaction status as errored.

#### Changes in `load_transaction_accounts`

When we load transaction accounts we have to calculate the application fees by
decoding the `PayApplicationFees` instruction. Then we verify that fee-payer
has minimum balance of:
`per-transaction base fees` + 
`prioritization fees` + 
`maximum application fees to be paid`

If the payer has a sufficient balance then we continue loading other accounts.
If `PayApplicationFees` is missing then application fees is 0. If payer has
insufficient balance transaction fails with error `Insufficient Balance`.

#### Changes in invoke context

The structure `invoke context` is passed to all the native solana program
while execution. We create a new structure called `ApplicationFeeChanges`
which contains one hashmap mapping application fees (`Pubkey` ->
`application fees(u64)`), and another containing rebates (`Pubkey` ->
`amount rebated (u64)`). The `ApplicationFeeChanges` structure will be filled
by iterating over all accounts and finding which accounts require application
fees.

```rust
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ApplicationFeeChanges {
    /// To store application fees by account
    pub application_fees: HashMap<Pubkey, u64>, 
    /// to store rebates by account
    pub rebated: HashMap<Pubkey, u64>,
}
```

Accounts having application fees will be added in the `application_fees` field
of this structure with corresponding application fees. On each `Rebate`
instruction we find the minimum between `application_fees` and the rebate
amount for the account. If there is already a rebate in the `rebated` map then
we update it by `max(rebate amount in map, rebate amount in instruction)`, if
the map does not have any value for the account, then we add the rebated
amount in the map.

In verify stage we verify that `Application Fees` >= `Rebates` +
`Overpaid amount` for each account and return `UnbalancedInstruction` on
failure.

#### Changes in Bank

In method `filter_program_errors_and_collect_fee` we will add logic to deposit
application fees to the respective mutable accounts and to reimburse the
rebates to the payer in case the transaction was successful. If the
transaction fails then we withdraw
`per-transaction base fees` +
`prioritization fees` + 
`sum of application fees on all accounts`
from the payer. We return the overpaid amount in case the transaction was
successful or unsuccessful. We also transfer the application fees to the
respective accounts at this stage.

## Impact

If the dapps want to rebate application fees they have to implement very
carefully the logic of rebate. They should be very meticulous before calling
rebate so that a malicious user could not use this feature to bypass
application fees. Dapp developers also have to implement additional
instruction to collect these fees using lamport transfers.

DApp developers have to consider the following way to bypass application fees
is possible: A Defi smart contract with two instructions IxA and IxB. Both IxA
and IxB issue a rebate. IxA is an instruction that places an order on the
market which can be used to extract profit. IxB is an instruction that just
does some bookkeeping like settling funds overall harmless instruction.
Malicious users then can create a custom smart contract to bypass the
application fees where it CPI's IxA only if they can extract profit or else
they use IxB to issue a rebate for the application fees. So DApp developers
have to be sure when to do rebates usually white listing and black listing
instruction sequence would be ideal.

A dapp can break the cpi interface with other dapps if it implements this
feature. This is because it will require an additional account for the
application fees program to all the instructions which calls `Rebate`. The
client library also has to add the correct amount of fees in instruction
`PayApplicationFees` while creating the transaction.

It is the DApp's responsibility to publish the application fee required for
each account and instruction. They should also add appropriate
`PayApplicationFee` instruction in their client library while creating
transactions or provide visible API to get these application fees. As DApp has
to be redeployed to change application fees, we do not expect to change it
often. We can also add additional methods in JsonRpc and solana web3 library
to get application fees for an account.

Dapp should be careful before rolling out this feature. Because the
transaction would start failing if the rollout is sudden. It is preferable to
implement rebate, and add pay application fees in their APIs, so that the user
pays full application fee but is then rebated if the transaction succeeds.
Then once everyone starts using the new API they can add the check on
application fees.

Additional instructions should be added to the known programs like Token
Program, Vote Program, Stake Program, etc. to enable this feature on the
TokenAccount, VoteAccount, and other accounts. Take an example of the token
program; we can let the user change application fees to its token account. For
that, we have to add an instruction `UpdateApplicationFees` instruction to the
token program, which will cpi `UpdateApplicationFees` for the token account
PDA. Same goes for `Rebate.` In this way, any malicious access to the user's
token account will fail without paying application fees. These token program
also have to find a way for a user to get back the application fees collected
on their accounts.

Overall this feature will incentivize the creation of proper transactions and
spammers would have to pay much more fees reducing congestion in the cluster.
This will add very low calculation overhead on the validators. It will also
enable users to protect their accounts against malicious read and write locks.
This feature will encourage everyone to write better-quality code to help
avoid congestion.

## Calculating Application Fees for a dapp

Lets consider setting application fees for Openbook DEX. We can set fees
comparable to the rent of the accounts involved or something fixed. Setting
application fees too high means dapp users need more balance to interact with
the dapps and if they are too low then it won't prevent spamming or malicious
use. In case of openbook the intent is to avoid spamming.

Most of the OpenBook accounts like asks, bids and event queues are used in
write mode only we can disable read-locks on these accounts. Now we can
consider there are 48 M CU's per block and 2.5 blocks per second. Considering
each instruction takes 200 CUs so around 600 transactions per second.
Currently, base fees are around 0.00005 SOLs, with so low gas fees spammers
have the advantage to spam the cluster. A reasonable application fee could be
around 0.1 SOLs per transaction that could be distributed among different
accounts. For a user interacting with dapp with 0.1 SOLs in the account seems
reasonable assuming that the transactions are executed successfully and the
fees are rebated. This will make spammers spend their SOLs 2000 times more
rapidly than before. The thumb rule for dapps to set application fees on their
accounts is `More important the account = Higher application fees`.

Suppose a user or entity desires to safeguard their token account from
potentially malicious read/write locking that obstructs their ability to
perform MEV. In that case, they can set the highest possible application fees
on their token account, rendering it impossible to extract profit by blocking
their account. Even if the transaction fails, and they have to pay the
application fees, they can recover the fees as they own the account. A general
guideline for users is that they should possess at least N times (where N=10
is recommended) the SOLs required to pay application fees on their accounts so
that they are not locked out. The user has to pay application fees to transfer
all collected application fees.

## Security Considerations

If the application fee for an account is set too high then we cannot ever
mutate that account anymore. Even updating the application fees for the
account will need a very high amount of balance. This issue can be easily
solved by setting a maximum limit to the application fees. We suggest setting
this limit to 100 SOLs.

For a dapp it is better to set very low application fees at the beginning so
that it could be rolled out easier.

For an account that has collected application fees, to transfer these fees
collected to another account we have to pay application fees to write lock the
account, we can include a rebate instruction in the transaction. In case of
any bugs, while transferring application fees from the account to the
authority, there can be an endless loop where the authority creates a
transaction to recover collected application fees, with an instruction to pay
application fees to modify the account and an instruction to rebate. If the
transaction fails because of the bug, the user fails to recover collected
fees, inturn increasing application fees collected on the account.

## Backwards Compatibility

This feature does not introduce any breaking changes. The transaction without
using this feature should work as it is. To use this feature supermajority of
the validators should move to a branch that implements this feature.
Validators that do not implement this feature cannot replay the blocks with
transactions using application fees they also could not validate the block
including transactions with application fees.

## Mango V4 Usecase

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
