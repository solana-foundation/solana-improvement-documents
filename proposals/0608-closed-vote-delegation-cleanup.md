---
simd: '0608'
title: 'DeactivateDelinquent for Closed Vote Accounts'
authors:
  - Gabe Rodriguez (Anza)
category: Standard
type: Core
status: Review
created: 2026-08-24
feature: (to be assigned upon acceptance)
---

## Summary

Allow the Stake Program's existing `DeactivateDelinquent` instruction to
deactivate stake delegated to a closed vote account. The delegated vote account
is treated as closed when its address is no longer owned by the Vote Program
or its Vote Program owned data meets the uninitialized conditions below. All
other validation, the instruction interface, and deactivation behavior remain
unchanged.

## Motivation

`DeactivateDelinquent` allows anyone to begin cooldown on stake delegated to a
validator that has stopped voting. It requires initialized vote state showing
either no credits ever or none in the last five epochs, plus an active reference
vote account showing that the cluster continued voting.

If a validator closes their vote account, `DeactivateDelinquent` can no longer
validate that account's vote state and fails before examining the delegated
stake account. The stake authority can still deactivate normally, but if they
never act, the permissionless path is lost. The closed vote account leaves behind
an orphaned delegation whose stake continues contributing to stake-history
accounting until deactivated. This proposal restores a permissionless way to
begin the stake's cooldown.

Note: This case does not pose a persistent liveness risk because the closed
vote account is excluded from future epoch-stakes snapshots and leader schedules.

## New Terminology

N/A

## Detailed Design

### Delinquency conditions

`DeactivateDelinquent` treats the delinquency condition as satisfied in two new
cases:

1. The delegated vote account address is not owned by the Vote Program. This is
   the normal result of closure. Funding the address later creates a System
   Program owned account, which also satisfies this condition.
2. The address is owned by the Vote Program but contains uninitialized data.
   This state can exist if the address holder recreates a zero filled account
   and assigns it to the Vote Program. The same state can also result when
   lamports are returned during the transaction that closes the vote account.
   The data is considered uninitialized when:
   1. It contains fewer than four bytes and every byte is zero.
   2. It contains at least four bytes and its first four bytes encode the
      little endian zero discriminator. Only the discriminator is inspected so
      a large zero filled shell cannot make cleanup exceed the instruction's
      compute budget.

All other Vote Program owned data follows the existing vote state decoding and
epoch credit delinquency checks. Decoding failures, including unknown nonzero
discriminators, remain errors.

The active reference vote account and delegation address checks remain
unchanged. Successful processing deactivates the stake at the current epoch as
it does today.

## Alternatives Considered

- Keep the existing behavior, leaving abandoned stake without a permissionless
  deactivation path.
- Treat only zero-lamport addresses as closed. Funding a closed address would
  create a nonzero System Program account and block cleanup even though no
  initialized vote state exists there.
- Treat only addresses not owned by the Vote Program as closed. A closed
  address can remain or be recreated as a Vote Program-owned zero-filled shell,
  which would still block cleanup.
- Treat every vote-state decoding failure as a closed account. However, an
  unknown nonzero discriminator could represent a future vote account state.
- Preserve the five-epoch delay, but that would require new state because the
  Stake Program cannot determine when the vote account was closed.

## Impact

- Callers: Anyone can use the new path for stake whose delegated vote account
  meets one of the closed or uninitialized conditions defined above. The stake
  enters normal cooldown without moving lamports or changing its authorities.
- Instruction interface: No change.
- Validators: Clients must support the feature-gated upgrade to the Stake
  Program ELF containing this change. No other validator changes are required.

## Security Considerations

Closing a vote account can make its delegated stake eligible for
`DeactivateDelinquent` sooner. A vote account that has earned credits may be
closed after two epochs without credits, while `DeactivateDelinquent` normally
requires five. That delay gives an offline validator time to recover before
its delegated stake can be deactivated. Closing a vote account, however,
requires an explicit transaction authorized by its withdraw authority, so
there is no outage to wait out.

The new path applies only to stake already delegated to the supplied vote
account address. Stake can only become delegated to an address that previously
contained initialized vote state. `DelegateStake` continues to reject
zero-filled or uninitialized vote accounts.

Only the recognized closed and uninitialized states bypass vote-state
decoding. Malformed initialized formats and unknown nonzero discriminators
retain their existing errors. This prevents an older Stake Program from
treating an unsupported future vote-state version as delinquent.
