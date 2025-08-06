---
simd: '0183'
title: Skip Rent Rewrites
authors:
  - brooks@anza.xyz
category: Standard
type: Core
status: Activated
created: 2024-10-04
feature: CGB2jM8pwZkeeiXQ66kBMyBR6Np61mggL7XUsmLjVcrw (https://github.com/solana-labs/solana/issues/26599)
---

## Summary

Do not rewrite accounts *that are unchanged* by rent collection.

## Motivation

Rent collection checks every account at least once per epoch.  This process
loads *and stores* accounts, regardless if the account has actually changed or
not.  All accounts now must be rent-exempt, and there are zero rent-paying
accounts on mainnet-beta.  Thus, almost every account stored due to rent
collection is unchanged, and storing unchanged accounts is useless work.

Worse, storing accounts incurs all the downstream costs on the accounts
database: computing the various accounts hashes on extra accounts unnecessarily,
tracking/cleaning/purging the new and old versions of accounts, plus it bloats
the incremental snapshots with accounts that haven't meaningfully changed.

## Alternatives Considered

We could remove rent collection entirely, and that is already planned
(see SIMD-0084).  Skipping rewrites is a smaller, less complex change, which
can allow rollout and activation sooner.

## New Terminology

"Skipping rewrites" means to not store accounts that are unchanged by rent
collection.

## Detailed Design

If rent collection has *not* changed an account, the account must not be
stored/written back in that slot.

An important note, rent collection updates an account's `rent epoch` in addition
to its balance.  If rent collection does not collect rent (i.e. the account's
balance is unchanged), but *does* change the account's `rent epoch`, the
account must still be stored.

To state another consequence explicitly, since these accounts will not be
rewritten, they will no longer be part of the accounts delta hash nor the
incremental accounts hash.

## Impact

Validators will see performance improvements:

* Fewer accounts will need to be stored per slot.
* Fewer accounts will need to be hashed for the various accounts hashes.
* Fewer accounts will need to be included in incremental snapshots.

## Security Considerations

Having all accounts rewritten due to rent collection results in all accounts
being included in at least one bank hash per epoch.  Since the bank hash is
part of what is voted on for consensus, this means every account is verified by
the network at least once per epoch.

By skipping rewrites, we lose this security property.  This is OK because the
Epoch Accounts Hash (EAH) was added to directly address this issue.  See the
[EAH proposal](https://docs.solanalabs.com/implemented-proposals/epoch_accounts_hash)
for more information.
