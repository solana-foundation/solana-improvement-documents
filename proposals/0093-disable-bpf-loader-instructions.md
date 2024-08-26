---
simd: '0093'
title: Disable Bpf loader V2 program deployment
authors:
  - Haoran Yi
category: Standard
type: Core
status: Activated
created: 2023-12-13
feature: [7WeS1vfPRgeeoXArLh7879YcB9mgE9ktjPDtajXeWfXn](https://github.com/solana-labs/solana/issues/34424)
development:
  - Anza - [Implemented](https://github.com/solana-labs/solana/pull/35164)
  - Firedancer - Implemented
---

## Summary

Disable BPF Loader V2 for program deployment.

## Motivation

An `account` on solana network is defined by the following struct. In the
struct, the bool metadata field `executable` was used to indicate whether the
account is executable by program runtime.

```
pub struct Account {
    pub lamports: u64,
    pub data: Vec<u8>,
    pub owner: Pubkey,
    pub executable: bool,
    pub rent_epoch: Epoch,
}
```

We want to deprecate the usage of *executable* metadata on accounts for program
runtime. The new variant of Bpf loader (i.e. V3/V4 etc.) no longer requires
*executable* metadata. However, the old Bpf loader (v2) still uses *executable*
metadata during its program deployment. And this is a blocker for deprecating
the usage of *executable* metadata for program runtime. Therefore, as we are
migrating from the old Bpf loader V2 to the new Bpf loader (V3/V4), we are going
to add a feature to disable old V2 Bpf program deployment so that we can
activate the feature and deprecate *executable* metadata in program runtime for
the new kinds of Bpf loaders.


## Alternatives Considered

None

## New Terminology

None

## Detailed Design

When the feature - "disable bpf loader instructions" is activated, no program of
bpf loader V2 can be deployed. Any such deployment attempt will result in a
"UnsupportedProgramId" error.

The PR for this work is at https://github.com/solana-labs/solana/pull/34194

## Impact

1. New programs will no longer be deployable with Bpf loader V2. People will have
   to migrate their program deployment to a new Bpf loader, i.e. V3 or V4.


## Security Considerations

Because when the feature is activated, people should have already migrated their
new programs to Bpf loader V3/V4. And existing already-deployed Bpf loader V2
programs will still run correctly. Hence, there should be no security concerns.

## Backwards Compatibility

Incompatible.
