---
simd: "0100"
title: Maintain Dynamic Set of Reserved Account Keys
authors:
  - Justin Starry
category: Standard
type: Core
status: Draft
created: 2024-01-17
feature: (fill in with feature tracking issues once accepted)
---

## Summary

The transaction scheduler and the runtime both demote transaction write locks
for builtin programs and sysvars using a static list of reserved IDs. This
proposal replaces the currently used static lists of builtin program and sysvar
IDs with a dynamic set of reserved account keys that can be updated at epoch
boundaries with feature gates.

## Motivation

The current approach of using static lists of reserved IDs doesn't allow core
developers to modify which account write locks should be demoted without
breaking consensus.

Since the static lists were introduced years ago, a few sysvars and some popular
builtin programs have been developed that should not be able to be write locked
but their keys cannot be added to the static lists. This demonstrates a need for
a set of reserved keys that can be updated safely over time.

## Alternatives Considered

1. Remove write-lock demotion for reserved accounts

An alternative is to remove write-lock demotion for reserved accounts altogether
and instead fail transactions that write-lock a reserved account. Both sysvars
and builtin programs are easily identified by their owner program while loading
accounts so no list needs to be maintained for transaction processing.

However, even if write-lock demotion is removed, the transaction scheduler still
needs to be aware of reserved keys to prevent transactions that set reserved key
write-locks from degrading transaction scheduling.

This approach would also mean that users and developers which mistakenly write
lock a reserved account will no longer have the nice experience of having the
write lock automatically demoted to a read-lock and their transactions still
being executed. Instead, their transaction would fail and a lot of apps could be
affected by this change in behavior.

2. Demote write-locks lazily

Another alternative is to demote write-locks while loading transaction accounts
if a loaded account is owned by a reserved id like "native loader" or "sysvar".
This approach means that a simpler set of reserved program ids could be tracked
rather than tracking a full set of all reserved accounts.

Again, the problem here is that the scheduler also needs to be aware of how
write locks might be demoted in order to schedule transactions efficiently. So
transaction schedulers would likely need to track a dynamic list of reserved
accounts anyways to ensure that transaction locks are handled in exactly the
same way as the runtime to avoid scheduling issues. Otherwise if a scheduler
prematurely demotes transaction write locks before that sysvar exists, it could
send an invalid batch of conflicting transactions to the runtime.

## New Terminology

Reserved Accounts: Any builtin program or sysvar or other account key managed by
the Solana core protocol that should not be write locked by a transaction. This
excludes the incinerator account which is designed to be write locked.

## Detailed Design

To determine the post-demotion set of write lock requested keys for a
transaction, each transaction write locked account key is checked against a set
of reserved keys maintained by the validator for a given block.

The set of reserved keys will be initialized as the full set of sysvar and
builtin program keys currently tracked in static lists in the Solana Labs
validator implementation:

```rust
pub static ref SYSVAR_IDS: Vec<Pubkey> = vec![
    clock::id(),
    epoch_schedule::id(),
    #[allow(deprecated)]
    fees::id(),
    #[allow(deprecated)]
    recent_blockhashes::id(),
    rent::id(),
    rewards::id(),
    slot_hashes::id(),
    slot_history::id(),
    stake_history::id(),
    instructions::id(),
];

pub static ref BUILTIN_PROGRAMS_KEYS: [Pubkey; 10] = {
    let parse = |s| Pubkey::from_str(s).unwrap();
    [
        parse("Config1111111111111111111111111111111111111"),
        parse("Feature111111111111111111111111111111111111"),
        parse("NativeLoader1111111111111111111111111111111"),
        parse("Stake11111111111111111111111111111111111111"),
        parse("StakeConfig11111111111111111111111111111111"),
        parse("Vote111111111111111111111111111111111111111"),
        system_program::id(),
        bpf_loader::id(),
        bpf_loader_deprecated::id(),
        bpf_loader_upgradeable::id(),
    ]
};
```

The set of reserved keys may only be modified via feature gate activation. On
epoch boundaries, validator implementations should add or remove reserved keys
as dictated by feature gated code. The first feature gate activated modification
will likely include the following keys:

```rust
pub static ref NEW_RESERVED_KEYS: Vec<Pubkey> = vec![
    address_lookup_table::program::id(),
    compute_budget::id(),
    ed25519_program::id(),
    secp256k1_program::id(),
    loader_v4::id(),
    sysvar::id(),
    epoch_rewards::id(),
    last_restart_slot::id(),
    Pubkey::from_str("ZkTokenProof1111111111111111111111111111111").unwrap(),
];
```

For validator operations not subject to consensus like RPC services, ledger
history storage, and debugging tools which don't have a block context for
fetching the set of reserved accounts, a static list can be maintained and
updated without requiring feature gates.

## Impact

Impact should be negligible. Dapp developers don't need to change how they build
transactions due to the developer friendly nature of the write lock demotion
feature.

## Security Considerations

The main consideration is making sure that all `is_writable` checks are
consistent before and after implementing this proposal to avoid breaking
consensus.
