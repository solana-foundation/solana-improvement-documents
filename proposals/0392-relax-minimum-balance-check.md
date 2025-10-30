---
simd: '0392'
title: Relax Post-Execution Minimum Balance Check
authors:
  - Igor Durovic (anza)
category: Standard
type: Core
status: Idea
created: 2025-10-30
feature: (fill in with feature key and github tracking issues once accepted)
supersedes:
superseded-by:
extends:
---

## Summary

To allow for non-disruptive rent increases, relax post-execution account
minimum balance checks. The new lower-bound on post-exec balance is
`min(acc.pre_exec_balance, cur_rent_per_byte * acc.size())`.

This maintains the invariant that every account has a balance at or above the
minimum balance requirement calculated at any point since the most recent
allocation occurred. When enabled, only newly allocated accounts will be
subject to rent increases.

## Motivation

In order to safely reduce rent there must be a mechanism available for
non-disruptive rent increases. Without this change, a rent increase would
either place existing accounts in a gray area undefined by the protocol or
prevent write-locking all accounts with balances below the new rent value. 

## New Terminology

- `calculate_min_balance(acc_size) = acc_size * current_rent_per_byte`.
  Note, `acc_size` includes both the account data size and the 128 overhead
  bytes.

## Detailed Design

### Current behavior

For all write-locked accounts, post-execution account balance checks
currently verify:

```
min_balance = calculate_min_balance(acc.data_size_post)
assert(acc.post_exec_balance == 0 or acc.post_exec_balance >= post_min_balance)
```

This allows a transaction to reduce an account's balance as long as it still
satisfies the minimum balance for its post-execution data size.

If the rent price is increased then existing accounts may become sub-exempt,
which isn't currently allowed in the protocol as rent paying accounts have been
deprecated.

### Proposed behavior

For all write-locked accounts, post-execution account balance checks MUST
verify:

```
min_balance = calculate_min_balance(acc.post_exec_size)
if acc.pre_exec_balance > 0 and acc.post_exec_size == acc.pre_exec_size:
    min_balance = min(min_balance, acc.pre_exec_balance)

assert(acc.post_exec_balance == 0 or acc.post_exec_balance >= min_balance)
```

The check ensures that an account's balance is always at least the minimum
rent_per_byte price since the last allocation multiplied by the account
size:

- As the base case, it's clear this holds
  for new account creations or re-allocations: the post-execution rent price
  and account size is used to determine the minimum balance.
- The first subsequent transaction write-locking the same account but not
  performing any reallocations can either have (1) a balance higher than the
  original min_balance or (2) a lower balance that's bounded below by a reduce
  rent price.
- With this, we can inductively prove that the post-execution balance of
  every account is bounded below by the lowest rent since the last allocation.

### Implementation details

- The pre-execution balance MUST be captured before any instruction execution
  begins, based on the account's loaded state.
- The pre and post-execution sizes are compared to determine if reallocation
  occurred.
- The `calculate_min_balance()` function uses the current rent_per_byte value,
  which may vary based on active feature gates and rent policies.
- Newly created accounts have `pre_exec_balance = 0` and are not subject to the
  `min()` clause.
- As before, 0 post-balance is allowed and equivelent to closing an account.

### Edge cases

**Account creation:**

- Pre-execution: `pre_exec_balance = 0`
- Post-execution: only `calculate_min_balance(post_exec_size)` is enforced
- No change to existing behavior

**Account closure:**

- No change to existing behavior

**Size increase (reallocation upward):**

- No change to existing behavior

**Size decrease (reallocation downward):**

- No change to existing behavior

**No size change:**

- `post_exec_size == pre_exec_size`
- The `min()` clause applies: `min_balance = min(calculate_min_balance(size),
  pre_exec_balance)`
- **This is the key behavioral change**: allows accounts to retain their
  original rent price even if current rent increases

## Alternatives Considered

### Always enforce current rent price post-execution

While sub-exempt accounts are a gray area in the protocol, this would be the
closest to the current behavior.
All accounts would be forced to adopt new rent prices immediately, causing
widespread transaction failures when write-locks are attempted on accounts
with balances between the old and new rent price.

### Track "rent price at allocation" in metadata

More complex: requires account versioning and metadata changes. The proposed
approach achieves a similar goal with only runtime logic changes by using the
pre-execution balance as a proxy for the minimum rent price since last
allocation. The benefit of a dedicated metadata field is that the invariant
can be made stricter: every account's balance is bounded below by the rent
price at the most recent allocation rather than the minimum rent price *since*
the last allocation.

## Impact

- Dapp developers: Enables non-disruptive rent increases. Existing accounts are
  grandfathered at their creation-time rent price until they reallocate. New
  accounts and reallocated accounts adopt the current rent price. Minimal
  behavior change for typical applications; most applications do not reduce
  account balances below their minimum during normal operation.
- Validators: Minimal performance impact; requires storing pre-execution balance
  (one u64) and size per write-locked account during transaction execution.
- Core contributors: Changes to transaction processing logic to capture
  pre-execution state and implement the relaxed balance check.

## Security Considerations

- Enables safe rent increases without disrupting existing accounts: only newly
  allocated accounts are subject to new rent prices.
- Maintains the invariant that accounts always satisfy the minimum rent price
  since the time of their most recent allocation.
- No mechanism for forcing accounts to "upgrade" to current rent prices without
  reallocation; this is intentional to preserve non-disruptiveness.
- It's essentially free to adopt a new rent rate that's lower than the rent rate
  an account was subject to at allocation time. This isn't problematic in itself
  but it may cause users to be less sensitive to rent increases.

## Backwards Compatibility

This is a **relaxation** of existing constraints:

- The change makes the balance check less strict by allowing accounts to retain
  their original rent price when not reallocating.
- This is backwards compatible in the sense that transactions that currently
  succeed will continue to succeed.
- However, it changes consensus rules and must be activated behind a feature gate.
