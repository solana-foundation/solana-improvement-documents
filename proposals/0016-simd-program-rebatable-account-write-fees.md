---
simd: '0016'
title: Program Rebatable Account Write Fees
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

This SIMD will discuss additional fees called Program Rebatable Account Write
Fees or PRAW Fees. These fees are decided and set by the dapp developer to
interact with the dapp. Dapp developers then can decide to rebate these fees if
a user interacts with the dapp as intended and disincentivize the users which do
not. These fees will be applied even if the transaction eventually fails and are
collected on the account on which they were set. These fees would be only
applied if the account is write locked by the transaction. The owner of the
account can do lamport transfer to recover these fees. So instead of fees going
to the validator, these fees go to the **writable account** i.e owner of the
writable account. It will be dapp developer's responsibility to advertise the
required PRAW fees to its users. User will have to specify the maximum amount of
PRAW fees that they are willing to pay in the transaction. This way solana user
and wallet adapters always know how much maximum fees they will end up paying to
execute the transactions. If a user paid PRAW fees in excess the remaining
amount of fees will be returned back to the user. Like prioritization fees,
these fees will depend on the CU requested by the transaction.


## Motivation

### Broken Mechanics

When write-lock is taken, there are no checks on whether the program needs it or
guarantees it will be used eventually. Whenever a transaction takes the
write-lock, there is no punishment for taking the write-lock and not using it.
Dapps have no authority over who can write-access the accounts owned by them.
There is a 12M CUs per block per writable account limit, which makes the block
space very important, and dapps would like to use it efficiently. Write-lock
grievers use this advantage to get a competitive edge by write-locking the
accounts of their competitors or dapp. Dapps may also want to incentivize
certain transactions more than others during specific scenarios or conditions.
Currently, dapps can regulate transactions that are executed successfully but
cannot punish transactions that fail or that never CPI into the dapp. This makes
honest users the cost bearer. There should be a mechanism where Dapps can
monetize the accounts owned by them and then distribute the incentives as they
see fit.

### Spamming

There are multiple accounts of OpenBook, mango markets, etc, which are used in
high-frequency trading. During extreme congestion, specialized trading programs
write-lock these accounts but never CPI into the respective programs unless they
can extract profit, effectively starving the actual users for access. Current
low cluster fees, and jitter, incentivize spammers to spam the network with
transactions rather than improve infrastructure to read the cluster state and
create transactions.

With the correct fee mechanism, we can discourage spamming of the dapp and let
dapp control its traffic.

### Closing incentive loop

With prioritization fees and MEV bribes, leaders will only include the
transactions that maximize their profit. During extreme congestion or high
network usage, these fees will increase drastically. For proper functioning of
Dapps, they will need to incentivize some types of transactions more than
others. These incentives will be distributed to subsidize the cost of sending
transactions to the cluster. These incentives will eventually end up with the
validators and their stakes accounts. These costs of creating incentives will
then be transmitted to the users, making it lose its competitive advantages.

We should focus on circulating rewards evenly between the communities of Solana
ecosystems and make Solana attractive to all DeFi, Validators, MEV, and other
protocols. With correctly managed PRAW fees, we can solve the issue for local
fee markets and give actors the correct incentives to send proper instructions
at the right time. This will also help dapp incentivize good behavior from the
penalties collected for bad behavior. Better functioning of dapps will also
increase profits by MEV or validator communities.

## Alternatives Considered

* Having a seperate app chain for defi dapps

Pros: Dapps can implement their own incentive loop and have fine control over
accounts.

Cons: A solana fork has to be developed and maintain, bidges have to be built.
Not in the best interest for solana as a community.

* Having a fixed write lock fee and a read lock fee.

Pros: Simpler to implement, Simple to calculate fees.

Cons: Will increase fees for everyone, dapp cannot decide to rebate fees, (or
not all existing apps have to develop a rebate system). Fees cannot be decided
by the dapp developer. Penalizes everyone for a few bad actors.

* Passing fees for each account by instruction and validating them
  during the execution inside user application code

Pros: High efficiency, minimal performance impact, more generic, no need to
change the account structure.

Cons: Cannot prevent denial of service attacks, or account blocking
attacks that do not call into the respective program

## New Terminology

Program Rebatable Account Write Fees or PRAW Fees: Fees that are decided by the
dapp developer will be charged if a user wants to use the dapp. They will be
applied even if the transaction fails contrary to lamport transfers. The program
can decide to rebate these fees back to the user if certain conditions decided
by dapp developers are met.

### Other terminology

Account `Authority`: The true authority for the account. Let's take an example
of associated token account. A associated token account is a `PDA` derived from
associated token program, where the owner is token program. But the program
internally saves who has the authority over the account. Only token program can
change the data of a token account. For operations like withdrawing tokens, the
authority of the associated token account has to be the signer for the
transaction containing transfer instruction. To receive tokens from another
account, the authority's signature is not required.

`Other Fees`: Fees other than PRAW fees `base fees + prioritization fees`.

## Detailed Design

PRAW fees enable dapp developer to decide who to rebate and who to penalize
through runtime features. In particular, programs needs to be able to penalize,
even if the application is not CPI'd to.

The checks on the PRAW fees will be taken care of by Solana runtime. Total PRAW
fees committed will be included in the transaction through a special parameter
called `SetProgramRebatableAccountWriteFees` so that it is easier to calculate
the total amount of fees that will be paid by the transaction. This parameter
will be part of a new `TransactionHeaderProgram` where all the transaction
parameters will be set through a TVL. In case of nonpayment or partial payment,
the program is never executed. This instruction cannot be CPI'ed into.

Addition of new syscalls to get, set and rebate PRAW fees during program
runtime. Application developers will be able to rebate these fees through the
new syscall, but only if the transaction succeeds, this way transactions
write-locking the respective accounts will require full fee payment before
starting the execution of the transaction. Program will have to rebate fees back
to the payer in case of rebates

Once PRAW fees are paid on an account by the payer they are valid for the whole
transaction, even if the same account is write-locked in multiple instructions.

### Payment

`SetProgramRebatableAccountWriteFees` will be part of the updated compute budget
program. `SetProgramRebatableAccountWriteFees` will take a u64 parameter to set
lamports per requested CU as the PRAW fees. Instead of having separate
instructions for each compute budget parameter and PRAW fee we will have a
single instruction that will set all the compute budget and PRAW fee parameters.
Compute budget parameters could be considered as transaction header that sets
some parameters for the transaction. We create a new solana native program
`TransactionHeaderProgram` that decodes the instruction data using a type length
value structure (`TLV`) format and computes the required
`TransactionHeaderParameter`s for the transaction. The `TLV` structure is well
defined for accounts in solana program library repository we will extend the
definition for instruction data. As we have multiple transaction header
parameters we convert instruction data into a list of
`TransactionHeaderParameter`. In case we fail to deserialize the transaction
data, the transaction will fail with an error `InvalidInstructionData`. If same
`TransactionHeaderParameter` is used multiple times or has multiple instructions
using `TransactionHeaderProgram` we return an error `DuplicateInstruction`.
Similarly, if we use old Compute Budget instructions with the new instruction to
set the same parameters we return an error `DuplicateInstruction`.

Program Id for the program `TransactionHeaderProgram`:

```
TrasactionHeader111111111111111111111111111
```

Definition for transaction header parameters:

```
enum TransactionHeaderParameter {
  // existing compute budget parameters
  RequestHeapFrame(u32),
  SetComputeUnitLimit(u32),
  SetComputeUnitPrice(u64),
  SetLoadedAccountsDataSizeLimit(u32),
  // Limit on maximum of PRAW fees to be paid
  // Fees is set in microlamports per CU
  SetProgramRebatableAccountWriteFees(u64),
}
```

When the cluster receives a new transaction, with `TransactionHeaderParameter`
instruction is decoded to get the `SetProgramRebatableAccountWriteFees` and then
calculates the total fees required for the transaction message. Then we verify
that the fee-payer has a minimum balance of: `per-transaction base fees` +
`prioritization fees` + `PRAW fees`.

Where PRAW fees is :
`Microlamports per CU set by SetProgramRebatableAccountWriteFees` *
`requested CUs set by SetComputeUnitLimit`.

If the payer does not have enough balance, the transaction is not scheduled and
fails with an error `InsufficientFunds`. Consider this case the same as if the
payer does not have enough funds to pay `other fees`.

Before processing the message, we check if any loaded account has associated
PRAW fees. For all accounts with PRAW fees, the fees paid should be greater than
the required fees. In case of overpayment, the difference
will be paid back to the payer in any case. In case the PRAW fees
are insufficiently paid or not paid then the transaction fails with an error
`ProgramRebatableAccountWriteFeesNotPaid`. The payer will pay just `other fees`.
This failure is before the execution of the transaction.

The PRAW fees minus rebates and minus overpayment are transferred to the
respective accounts lamport balance, from where the owner can collect them and
then the rest would be transferred back to the payer.

### Syscall to change PRAW fees

The runtime will automatically set the PRAW fee to `0` on account creation. PRAW
fee will not be changed if the ownership of the account changes.

A new syscall will be added to update the PRAW fees:

```
fn set_program_rebatable_account_write_fee(
  address: Pubkey, 
  microlamports_per_requested_cu: u64
) -> Result<()>;
```

The syscall will take as arguments the account on which fee should be updated
and the microlamports per cu that should be charged and return a `Ok` if the
changes were successful else will return `Err`.

The accounts database will need to store the value of microlamports per
requested CU for the account. Changes to update the PRAW fees could be done only
by the account owner. All programs will need to implement special instructions
so that the program admin can use this syscall to update PRAW fees on the
required accounts.

Setting the microlamports per CU to `0` disables PRAW fees. The maximum amount
of microlamports per CU that could be set will be 2 ^ 42. Lets assume that the
maximum CU required to update these fees on an account is 1,000 CU. So the
maximum PRAW fees required to update an account (`M`) is

```
M = 2^42 * 1000 CU * 10^-6 lamports
M = 4.398046511104 * 10^12 * 1000 * 10^-6
M = 4.398046511104 SOLs
```

So this makes sure that admin does not lock itself out from changing the PRAW
fees.

### Syscall to read PRAW fees

A new syscall will be added to read PRAW fees on an account.

```
fn get_program_rebatable_account_write_fee(
  address: Pubkey
) -> u64;
```

The syscall will take address as input and return microlamports per
CU. It will return `0` if no PRAW is set on the address.

### Rebates

Rebates will be handled inside the program itself using system program transfer
instruction.

### Changes in JSON RPC

Currently JSON RPC service is used to fetch account data from RPC endpoint by
the clients. We extend the http method `getAccountInfo` to also return the PRAW
fee in microlamports per CU requested for the account. The http method
`getFeeForMessage` should be extended to decode the instruction data for the
program `TransactionHeaderProgram` into `TransactionHeaderParameter`s and then
calculate the total fees correctly including the PRAW fees. For purpose of
security we assume that program never rebates the PRAW fees on the accounts.
Wallets should be extended to use these two methods to display user the correct
information.

### Calculation of PRAW Fees

User creates a transaction by setting maximum PRAW fees in terms of
microlamports per requested CUs (`m`) and requested CUs (`C`). Solana runtime
will multiply `m` and `C` to calculate total maximum PRAW fees (`M`) payer is
willing to pay, then check if user has enough balance to pay all the fees
including PRAW fees then it will load the accounts required by the transaction.

Then we iterate on accounts to calculate total PRAW fees required (`T`).
Let (`Pi`) be PRAW fee set for ith account.
Let number of accounts loaded writable be (`L`)

```
T = sum(Pi * C) for i in 0 to L.
```

If M>=T we continue to execute else the transaction fails with an error
`ProgramRebatableAccountWriteFeesNotPaid`.

Let Overpaid amount (`O = M - T`) which will returned back to the user.

### Consumption of CUs

Currently `DEFAULT_COMPUTE_UNITS` assigned to `ComputeBudgetProgram` and
`SystemProgram` is 150 CUs. Considering these costs, `TransactionHeaderProgram`
will consume a fixed 300 CUs to decode one or more
`TransactionHeaderParameters`. An additional 150 CUs per writable account which
has a PRAW fee will be charged for the transaction which is equivalent to
`transfer` instruction in `SystemProgram`. These costs could change and
the future development related to the consumption of CUs while loading writable
accounts can additionally take into consideration if PRAW fees are enabled on
the loaded accounts.

### Looking at common cases

For simplicity, we have written examples in total PRAW fees instead of
microlamports per requested CUs.

#### No PRAW fees enabled

* A payer does not include `SetProgramRebatableAccountWriteFees` in the
  transaction. The transaction does not write lock any accounts with PRAW fees.
  Then the transaction is executed without the PRAW fee feature. The payer ends
  up paying other fees.

* A payer includes `SetProgramRebatableAccountWriteFees(PRAW fees)` with
  non-zero microlamports per CU as PRAW fees in the transaction but none of the
  accounts have any PRAW fees. This case is considered an overpay case. The
  payer balance is checked for `other fees + PRAW fees`.
  1. The payer does not have enough balance: Transaction fails with an error
    `Insufficient Balance` and the transaction is not even scheduled for
    execution.
  2. The payer has enough balance then the transaction is executed and PRAW
    fees paid are transferred back to the payer in any case.
  
  Note in this case
    even if there are no PRAW fees involved the payer balance is checked
    against PRAW fees.

#### PRAW fees are enabled

* Fees not paid case:

  A payer does not include `SetProgramRebatableAccountWriteFees` in the
  transaction. The transaction includes one or more accounts with PRAW fees.
  Then the transaction is failed with an error
  `ProgramRebatableAccountWriteFeesNotPaid`. The transaction is not executed.
  The payer ends up paying only other fees.

* Fees paid no rebates case:

  A payer includes instruction `SetProgramRebatableAccountWriteFees(100)` in the
  transaction. There is an account `accA` which is write-locked by the
  transaction and it has an PRAW fee of `100` lamports. Consider that the
  program does not have any rebate mechanism. Then in any case (execution fails
  or succeeds) `accA` will receive `100` lamports. The payer will end up paying
  `other fees` + `100` lamports.

* Fees paid rebates case:

  A payer includes instruction `SetProgramRebatableAccountWriteFees(100)` in the
  transaction. There is an accounts `accA` which is write-locked by the
  transaction and it has an PRAW fees fee of `100` lamports. Consider during
  execution the program will rebate the PRAW fees fee on the account. Then payer
  should have a minimum balance of `other fees` + `100` lamports to execute the
  transaction. After successful execution of the transaction, the `R` lamports
  will be rebated by the program to the payer. So the payer will finally end up
  paying `other fees` + `100` - `R` lamports only.

* Fees paid full rebates but the execution failed case:

  A payer includes instruction `SetProgramRebatableAccountWriteFees(100)` in the
  transaction. There is an account `accA` which is write-locked by the
  transaction and it has an PRAW fees fee of `100` lamports. Consider during
  execution the program will rebate all the PRAW fees on the account but later
  the execution failed. Then payer should have a minimum balance of 
  `other fees` + `100` lamports to execute the transaction. The program rebated
  PRAW fees but as executing the transaction failed, no rebate will be issued.
  The PRAW fees fees will be transferred to respective accounts, and the payer
  will finally end up paying `other fees` + `100` lamports as fees.

* Fees are over paid case:

  A payer includes instruction `SetProgramRebatableAccountWriteFees(1000)` in
  the transaction. There is an account `accA` that is write-locked by the
  transaction and it has an PRAW fees fee of `100` lamports. The minimum balance
  required by payer will be `other fees` + `1000` lamports as PRAW fees. So the
  payer pays 100 lamports for the account as PRAW fees and 900 lamports is an
  overpayment. The 900 lamports will be transferred back to the user even if the
  transaction succeeds or fails. The 100 lamports will be transferred to `accA`
  in all cases except if the transaction is successful and the program issued a
  rebate.

* Fees underpaid case:

  A payer includes instruction `SetProgramRebatableAccountWriteFees(150)` in the
  transaction. There is an accounts `accA` that is write-locked by the
  transaction and it has an PRAW fees of `300` lamports. The minimum balance
  required by payer will be `other fees` + `150` lamports as PRAW fees to load
  accounts and schedule transactions for execution. Here payer has
  insufficiently paid the PRAW fees paying 150 lamports instead of 300 lamports.
  So before program execution, we detect that the PRAW fees are not sufficiently
  paid and execution fails with the error
  `ProgramRebatableAccountWriteFeesNotPaid` and the partially paid amount is
  transferred back to the payer. So the payer pays only `other fees` in the end
  but the transaction is unsuccessful.

#### Edge cases

  * If PRAW fee is changed multiple times in the transaction, only last update
    will be taken into account.
  
  * In a transaction on an account with PRAW fees, a instruction that call
    `set_program_rebatable_account_write_fee` to set PRAW fee to 0. Then the
    fee PRAW fee requirement will be dropped from the next transactions and all
    the transactions with PRAW fees on the account will be considered as
    overpayment and reverted back to the payer.

  * In a transaction on an account with PRAW fees, with a instruction that call
    `set_program_rebatable_account_write_fee` to update the PRAW fee. When
    we do the syscall to get the PRAW fees
    `get_program_rebatable_account_write_fee`, it will always return the PRAW
    fee before any updates. The maximum rebate will be the initial PRAW fee for
    the account. This should be the case even if the PRAW fee is updated to 0.

## Impact

Overall this feature fixes bug where there are no negative effects of taking
write lock on an account. It will also incentivize the creation of proper
transactions and spammers would have to pay much more fees reducing congestion
in the cluster. This will add very low calculation overhead on the validators.
It will also enable users to protect their accounts against malicious read and
write locks. This feature will encourage everyone to write better-quality code
to help avoid congestion.

It is the dapp's responsibility to publish the PRAW fees fee required for each
account and instruction. They should also add appropriate
`SetProgramRebatableAccountWriteFees` instruction in their client library while
creating transactions or provide visible API to get these PRAW fees. Some
changes have to be done in web3.js client library to get PRAW fees when we
request the account. The dapp developer have to also take into account PRAW fees
on the programs they are dependent on. Dapps can improve infrastructure to
properly update and advertise these fees to their clients in specific scenarios
making them dynamic.

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
the PRAW fees feature on their accounts and rebate the fees if the user
initiates the transaction. The attacker will find this option unfeasible as
they will consume their SOL tokens more rapidly to maintain the attack.

## Security Considerations

If the PRAW fees fee for an account is set too high then we cannot ever mutate
that account anymore. Maximum fees should be set to around 4.398046511104 *
10^12 or 2^42 microlamports per CU, assuming that instruction that updates the
PRAW fees on the accounts will use around 1000 CUs, program owner will need
roughly 4.3980 SOLs to reupdate the PRAW fees. Optimizing the instruction to
update the PRAW fees is a must for dapp developers.

For an account that has collected PRAW fees, to transfer these fees collected to
another account we have to pay PRAW fees to write lock the account. In case of
any bugs, while transferring PRAW fees from the account to the authority, there
can be an endless loop where the authority creates a transaction to recover
collected PRAW fees, with an instruction to pay PRAW fees to modify the account
and an instruction to rebate. If the transaction fails because of the bug, the
user fails to recover collected fees, in turn increasing PRAW fees collected on
the account.

Program developers should take care of rebating. They can rebate more than what
PRAW fees were paid during the transaction. This may add into incentivising
users to send some transaction.

## Backwards Compatibility

This feature does not introduce any breaking changes. The transaction without
using this feature should work as it is. To use this feature supermajority of
the validators should move to a branch that implements this feature. Validators
that do not implement this feature cannot replay the blocks with transactions
using PRAW fees they also could not validate the block including transactions
with PRAW fees.


## Additional Notes

If the dapps want to rebate PRAW fees they have to implement very
carefully the logic of rebate. They should be very meticulous before calling
rebate so that a malicious user could not use this feature to bypass
PRAW fees. Dapp developers also have to implement additional
instruction to collect these fees using lamport transfers.

Dapp developers have to consider the following way to bypass PRAW fees
is possible: A Defi smart contract with two instructions IxA and IxB. Both IxA
and IxB issue a rebate. IxA is an instruction that places an order on the
market which can be used to extract profit. IxB is an instruction that just
does some bookkeeping like settling funds overall harmless instruction.
Malicious users then can create a custom smart contract to bypass the
PRAW fees where it CPI's IxA only if they can extract profit or else
they use IxB to issue a rebate for the PRAW fees. So dapp developers
have to be sure when to do rebates usually white listing and black listing
instruction sequence would be ideal.

Dapp should be careful before rolling out this feature. Because the
transaction would start failing if the rollout is sudden. It is preferable to
implement rebate, and add pay PRAW fees in their APIs, so that the user
pays full PRAW fees fee but is then rebated if the transaction succeeds.
Then once everyone starts using the new API they can add the check on
PRAW fees.

Another proposal will also introduce protection against unwanted read-locking
of the accounts. Many accounts like token account rarely need to be read-locked
this proposal will force these accounts to be write-locked instead and pay
PRAW fees if needed. This feature is out of the scope of this proposal.

### Calculating PRAW fees for a dapp

Let us consider setting PRAW fees for Openbook DEX. We can set fees
comparable to the rent of the accounts involved or something fixed. Setting
PRAW fees too high means dapp users need more balance to interact with
the dapps and if they are too low then it won't prevent spamming or malicious
use. In case of openbook the intent is to avoid spamming.

Most of the OpenBook accounts like asks, bids and event queues are used in write
mode only we can disable read-locks on these accounts. Now we can consider there
are 48 M CUs per block and 2.5 blocks per second. Considering each instruction
takes 200K CUs so around 600 transactions per second. Currently, base fees are
around 0.00005 SOLs, with so low gas fees spammers have the advantage to spam
the cluster. A reasonable PRAW fees fee could be around 0.1 SOLs per transaction
that could be distributed among different accounts. For a user interacting with
dapp with 0.1 SOLs in the account seems reasonable assuming that the
transactions are executed successfully and the fees are rebated. This will make
spammers spend their SOLs 2000 times more rapidly than before. The thumb rule
for dapps to set PRAW fees on their accounts is
`More important the account = Higher PRAW fees`.

### Oracle Usecase

Oracles's price feeds currently serve around 33M CU peak read load. An attacker
could write lock those for 12M CU and cause scheduling issues by crowding out
the majority of the block. A high PRAW fees fee of 2^42 microlamports per CU
could prevent anyone except price feed publishers from write-locking a price
feed account.

To effectively block DeFi protocols from using oracles a malicious attacker
needs to write lock oracles with a large amount of CUs. With current limits of
CUs per writable account of around 10 million CUs the attacker has to spend
around 4398 SOLs to block oracle for a block.

### Mango V4 Usecase

With this feature implemented Mango-V4 will be able to charge users who spam
risk-free arbitrage or spam liquidations by increasing PRAW fees on
perp-markets, token banks and mango-user accounts.

#### Perp markets

PRAW fees on perp liquidations, perp place order, perp cancel, perp
consume, perp settle fees. Rebates on: successful liquidations, consume
events, HFT market making refresh (cancel all, N* place POST, no IOC).

#### Token vaults

PRAW fees on open order book liquidations, deposit, withdrawals. Rebate
on successful liquidations, place IOC & fill in isolation, HFT marketmaking
refresh (cancel all, N* place POST, no IOC).

#### Mango accounts

PRAW fees on all liquidity transactions, consume events, settle pnl,
all user signed transactions. Rebate on transaction signed by owner or
delegate, successful liquidations, settlements, consume events.