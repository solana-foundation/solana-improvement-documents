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
---

## Summary

To allow for non-disruptive rent increases, relax post-execution account
minimum balance checks. The new lower-bound on post-exec balance is
`min(acc.pre_exec_balance, calculate_min_balance(acc.size()))`.

This maintains the invariant that every account has a balance at or above the
minimum balance requirement calculated at any point since the most recent
allocation occurred. When enabled, only newly allocated accounts will be
subject to rent increases.

Sidenote: this proposal doesn't include any mechanism for increasing rent,
but when such a mechanism is added in the future it should cap the
effective rent-per-byte at the legacy rent-exempt per-byte rate. This
limitation isn't strictly necessary but it avoids needing to significantly
modify any core system programs (e.g. stake program). If increasing the
per-byte rate beyond this cap is desirable then the relevant core programs
need to be updated to be compatible.

## Motivation

In order to safely reduce rent there must be a mechanism available for
non-disruptive rent increases. Without this change, a rent increase would
either place existing accounts in a gray area undefined by the protocol or
prevent write-locking all accounts with balances below the new rent value. 

## New Terminology

- `calculate_min_balance(acc_size) = acc_size * current_rent_per_byte`.
  Note, `acc_size` includes both the account data size and the 128 overhead
  bytes.
- `legacy_rent_per_byte`: the fixed rent-exempt per-byte rate used prior to this
  proposal.

  ```
  acc_size = ACCOUNT_STORAGE_OVERHEAD + acc.data.len;
  // 6,960 lamports on mainnet-beta
  legacy_rent_per_byte = LAMPORTS_PER_BYTE_YEAR * EXEMPTION_THRESHOLD
  legacy_min_balance = legacy_rent_per_byte * acc_size
  ```

## Detailed Design

### Current behavior

For all write-locked accounts, post-execution account balance checks
currently verify:

```
min_allowed_balance = calculate_min_balance(acc.post_exec_size)
assert(acc.post_exec_balance == 0 or acc.post_exec_balance >= min_allowed_balance)
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
rent_exempt_min_balance = calculate_min_balance(acc.post_exec_size)
if acc.pre_exec_balance > 0
   and acc.post_exec_size <= acc.pre_exec_size
   and acc.post_exec_owner == acc.pre_exec_owner:
    min_allowed_balance = min(rent_exempt_min_balance, acc.pre_exec_balance)
else:
    min_allowed_balance = rent_exempt_min_balance

assert(acc.post_exec_balance == 0 or acc.post_exec_balance >= min_allowed_balance)
```

The same pre/post-balance semantics MUST also be applied to fee and commission
distribution:
transaction fee debits and subsequent fee/commission distribution are treated as
an additional balance-changing phase, and the invariant above MUST be enforced
using the pre-fee and post-fee balances. This applies to the current fee
distribution as well as the next version of commission distribution where fees
are distributed via block revenue commission (SIMD-0123)

The check ensures that an account's balance is always rent-exempt as of
the last (re)allocation of its data:

- As the base case, it's clear this holds
  for new account creations or upwards reallocations: the post-execution rent
  price and account size is used to determine the minimum balance.
- The first subsequent transaction write-locking the same account but not
  performing any upwards reallocations can either have (1) a balance higher than
  the original min_balance or (2) a lower balance that's bounded below by a
  reduced rent price.
- With this, we can inductively prove that the post-execution balance of
  every account is bounded below by the lowest rent since the last allocation.

The owner check is intended to make reselling low-rent account state more
difficult so a secondary market doesn't develop. See the security considerations
section for more details.

### Implementation details

- The pre-execution balance MUST be captured before any state is modified
  (e.g. before fee collection, instruction execution, etc). This same
  `pre_exec_balance` snapshot MUST be reused when enforcing the minimum balance
  invariant across both instruction execution and fee/commission
  debiting/distribution.
- The pre and post-execution sizes are compared to determine if upwards
  reallocation occurred.
- The pre-execution owner MUST be captured. If the account owner changes during
  execution, the `min(pre_exec_balance, …)` clause MUST NOT apply; enforce the
  current rent-exempt minimum for the post-exec size.
- The `calculate_min_balance()` function uses the current rent_per_byte value,
  which may vary based on active feature gates and rent policies.
- Newly created accounts have `pre_exec_balance = 0` and are not subject to the
  `min()` clause.
- As before, 0 post-balance is allowed and equivalent to closing an account.

### Edge cases

**Account creation:**

- Pre-execution: `pre_exec_balance = 0`
- Post-execution: only `calculate_min_balance(post_exec_size)` is enforced
- No change to existing behavior

**Account closure:**

- No change to existing behavior

**Size increase (reallocation upward):**

- No change to existing behavior

**No net size increase:**

- `post_exec_size <= pre_exec_size`
- Owner MUST remain unchanged for the `min()` clause to apply:
  `min_balance = min(calculate_min_balance(size), pre_exec_balance)`
- **This is the key behavioral change**: allows accounts to retain their
  original rent price even if current rent increases
- Note that if the balance increases but is still below the current rent price,
  the new balance becomes the effective minimum balance for the given account.
  This means that the balance can no longer be reduced back to the original
  rent price.

**Owner change:**

- If the account owner changes, always enforce the current rent-exempt minimum
  for the post-exec size; the `min(pre_exec_balance, …)` clause does not apply.

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
  grandfathered at their creation-time rent price until they increase in size. New
  accounts and accounts that have increased in size adopt the current rent price.
  Minimal behavior change for typical applications; most applications do not
  reduce account balances below their minimum during normal operation.
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
  upwards reallocation; this is intentional to preserve non-disruptiveness.
- It's essentially free to adopt a new rent rate that's lower than the rent rate
  an account was subject to at allocation time. This isn't problematic in itself
  but it may cause users to be less sensitive to rent increases.
- For similar reasons, there's potential for a secondary state market to develop,
  leading to an additional sudden increase in state allocation in anticipation of
  a future rent hike. This can further exacerbate an excessive state growth event.
  The owner change check during minimum balance calculation is intended to limit
  this by making reselling accounts more difficult.


## Backwards Compatibility

This is a **relaxation** of existing constraints:

- The change makes the balance check less strict by allowing accounts to retain
  their original rent price when not upwards reallocating.
- This is backwards compatible in the sense that transactions that currently
  succeed will continue to succeed.
- However, it changes consensus rules and must be activated behind a feature gate.
