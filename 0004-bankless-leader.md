---
simd: '0004'
title: Bankless Leader
authors:
  - Tao Zhu (Solana Labs)
category: Standard
type: Core
status: Draft
created: 2022-12-08
feature: (fill in with feature tracking issues once accepted)
---

## Summary

A bankless leader does the minimum amount of work to produce a valid block.
The leader is tasked with ingress transactions, sorting and filtering valid
transactions, arranging them into entries, shredding the entries and
broadcasting the shreds. While a validator only needs to reassemble the block
and replay execution of well formed entries. 

## Motivation

Without having to execute transactions, leader threads will have more cycles
to pull packets from receiving buffer, and processing more packets per slot.

Feeding more packets to filtering and sorting logic allows higher prioritized
transactions to have higher chance be added to the top of queue, therefore have
priority to be considered to block inclusion.

It also ingests more non-conflicting transactions to be packed into block,
therefore improve block quality.

Combine with scheduler logic, it'd produce entries that are parallelizable,
potential simplify replay and reduce time.  

## Detailed Design

Normal bank operation for a spend needs to do 2 loads and 2 stores. With this
design leader just does 1 load. So 4x less account_db work before generating the
block. The store operations are likely to be more expensive than reads.

When replay stage starts processing the same transactions, it can assume that
PoH is valid, and that all the entries are safe for parallel execution. The fee
accounts that have been loaded to produce the block are likely to still be in
memory, so the additional load should be warm and the cost is likely to be
amortized.

### Fee Account

The [fee account](../terminology.md#fee_account) pays for the transaction to be
included in the block. The leader only needs to validate that the fee account
has the balance to pay for the fee.

### Balance Cache

For the duration of the leaders consecutive blocks, the leader maintains a
temporary balance cache for all the processed fee accounts. The cache is a map
of pubkeys to lamports.

At the start of the first block the balance cache is empty. At the end of the
last block the cache is destroyed.

The balance cache lookups must reference the same base fork for the entire
duration of the cache. At the block boundary, the cache can be reset along with
the base fork after replay stage finishes verifying the previous block.

### Balance Check

Prior to the balance check, the leader validates all the signatures in the
transaction.

1. Verify the accounts are not in use and BlockHash is valid.
2. Check if the fee account is present in the cache, or load the account from
   accounts_db and store the lamport balance in the cache.
3. If the balance is less than the fee, drop the transaction.
4. Subtract the fee from the balance.
5. If account is called by system program, or it is passed to a instruction
   with system program, then reduce its balance to 0 in the cache.

### Leader Replay

Leaders will need to replay their blocks as part of the standard replay stage
operation.

### Leader Replay With Consecutive Blocks

A leader can be scheduled to produce multiple blocks in a row. In that scenario
the leader is likely to be producing the next block while the replay stage for
the first block is playing.

When the leader finishes the replay stage it can reset the balance cache by
clearing it, and set a new fork as the base for the cache which can become
active on the next block.

### Resetting the Balance Cache

1. At the start of the block, if the balance cache is uninitialized, set the
   base fork for the balance cache to be the parent of the block and create an
empty cache.
2. if the cache is initialized, check if block's parents has a new frozen bank
   that is newer than the current base fork for the balance cache.
3. if a parent newer than the cache's base fork exist, reset the cache to the
   parent.

### Impact on Clients

The same fee account can be reused many times in the same block until it is
called by system program, or it is passed to a instruction alongside system
program.

Clients that transmit a large number of transactions per second should use a
dedicated fee account.

Once an account fee is used by system program, it will fail the balance check
until the balance cache is reset. 


## Security Considerations

What security implications/considerations come with implementing this feature?
Are there any implementation-specific guidance or pitfalls?

## Drawbacks

There will be failed transactions in block;

