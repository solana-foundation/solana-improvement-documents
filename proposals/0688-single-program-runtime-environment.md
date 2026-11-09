---
simd: '0688'
title: Single Program Runtime Environment
authors:
  - Joe Caulfield (Anza)
category: Standard
type: Core
status: Idea
created: 2026-09-17
feature: TBD
---

## Summary

Today, a program is verified on deployment against the program runtime
environment of the *next* slot. This proposal changes deployment to verify
against the environment of the *current* slot - the same one used for
execution.

## Motivation

Verifying deployments against the next slot requires every client to maintain
a second program runtime environment alongside the one used for execution, and
to keep the two consistent across epoch boundaries and feature activations.
Dropping it significantly simplifies runtime environment management, which
reduces the surface for consensus bugs and makes changes to the program runtime
easier to ship.

The two environments can only differ at the very last slot of an epoch, where
the next slot belongs to the next epoch, which may activate a feature that
affects the SBPF program runtime environment. Continuing to straddle such an
epoch boundary with two program runtime environments is cumbersome and can
actually be eliminated completely without significant impact to developers.

## New Terminology

None.

## Detailed Design

The program runtime environment - the registered syscalls and the verifier
configuration used to validate an ELF - is derived from the feature set of an
epoch.

After the activation of the associated feature key, Loader V3 instructions
`DeployWithMaxDataLen`, `Upgrade`, and `ExtendProgram` MUST use the program
runtime environment of the current slot, rather than that of the next slot.

This is a no-op at every slot except for the very last slot `S` of an epoch `N`
where the next slot `S + 1` belongs to `N + 1` and activates a feature that
affects SBPF program runtime environments. The deployment is verified against
the program runtime environment for slot `S` and *not* `S + 1`.

## Alternatives Considered

Keep both environments. This preserves the guarantee that a deployment which
succeeds at the very last slot of an epoch also loads in the next one, at the
cost of every client maintaining two environments indefinitely for the sake of
one slot per epoch.

## Impact

dApp developers deploying at the very last slot of an epoch at which an
environment-changing feature activates may see a deployment succeed and the
program then fail to load in the next epoch. Conversely, a program using such
a feature must wait until activation to deploy. Deployments at any other slot
are unaffected.

Validator implementations may drop the second environment.

## Security Considerations

None. A program is still verified against the environment in effect whenever it
is loaded for execution, so no unverified code can execute.

## Backwards Compatibility

Deploy transactions at the very last slot of an affected epoch may produce a
different result than before activation, which is why the change is feature
gated.
