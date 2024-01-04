---
simd: '0099'
title: Feature Activation Status Syscall
authors:
  - Hana Mumei
category: Standard
type: Core
status: Draft
created: 2024-01-03
feature: (fill in with feature tracking issues once accepted)
---

## Summary

We propose a new syscall, `is_feature_active`, which can be invoked in a
BPF program with a feature id to check whether that feature has been activated
on the cluster. A proof of concept is provided
[here](https://github.com/solana-labs/solana/pull/34611).

## Motivation

The immediate motivation for this is to enable porting native programs to
BPF, as part of the work on Firedancer and Runtime v2. The Stake Program is
regularly updated using feature gates, so some mechanism for it to be aware of
their status is necessary.

Currently there are four in use by the program, most notably
`stake_raise_minimum_delegation_to_1_sol`, which will likely not be activated
until some unknown time in the future.

## Alternatives Considered

The main alternative we considered was to eliminate the use of feature gates in
what are now native programs, and instead upgrade them via a mechanism to
swap the bytecode implementation of the program at the appropriate time,
whether automatically or manually. However, this would be more complex and
likely more brittle than the proposed solution, and may also make it
easier to railroad changes without the same level over oversight that
feature gates provide.

We may also consider a null option, i.e. no longer making consensus-breaking
changes to the native programs at all. But this is likely not a realistic
option as the network continues to evolve.

## New Terminology

N/A.

## Detailed Design

Implementation is fairly straightforward. A new syscall is added to
`programs/bpf_loader` called `SyscallIsFeatureActive`. It accepts two positional
arguments:

* `var_addr`: pointer to `bool`, a memory location to write feature status
* `feature_pubkey_addr`: pointer to `Pubkey`, the id of the feature to check

The syscall begins by consuming compute. As an example, we have used
`sysvar_base_cost` plus `size_of::<bool>()`, but this can be discussed.
Then the syscall checks `invoke_context.feature_set.is_active()` for the
feature id, writes the result into `var_addr`, and returns `Ok(SUCCESS)`.

As-written, the syscall successfully returns `false` if the feature id is not
found. This is identical to the behavior of the `is_active()` function and seems
more appropriate than signalling failure.

A new function is provided in `sdk/program` called `is_feature_active()`, which
accepts the feature id as `&Pubkey` and returns `Result<bool, ProgramError>`.

The stub version of the syscall completes normally with an invariant result of
`false`, though there is no reason it couldn't be `true`.

We also move all feature ids from `sdk` to `sdk/program` so that they can be
used in a program context.

## Impact

A benefit of this proposal is that dapp developers will be able to query feature
activation status in any BPF program, which may allow them to make programs that
are more robust to new features, to preemptively code against new features and
let the chain state handle "activation," and allow upstream more flexibility in
designing features, knowing they can signal information downstream this way.

The first obvious impact is that the Firedancer team will need to approve and
implement this.

Another impact is the way in which this SIMD interacts with the Multi-Client
Feature Gates SIMDs detailed in
<https://github.com/solana-foundation/solana-improvement-documents/issues/76>.
These SIMDs will need to be coordinated with each other, but it does not appear
at first read that they will interfere. It does mean that feature ids and
feature state will need to remain available via `InvokeContext` and cannot
be made resident under the `Feature111111111111111111111111111111111111`
program, but this would likely be the case anyway since features are used
pervasively throughout the runtime. Further investigation is needed.

## Security Considerations

Other than possible mistakes in implementation, it does not seem that this has
a potential security impact. The proof-of-concept implementation is modeled
after `Clock::get()` and does not differ substantially: it simply surfaces
data to a BPF program with no ability for the caller to mutate state.

## Backwards Compatibility

Feature ids moved to `sdk/program` are reexported in their original `sdk` module
to ensure no interface breakage downstream.
