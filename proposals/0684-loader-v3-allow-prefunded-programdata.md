---
simd: '0684'
title: 'Loader V3: Allow Prefunded ProgramData'
authors:
    - Joe Caulfield (Anza)
category: Standard
type: Core
status: Review
created: 2026-10-05
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

This SIMD proposes changing the `DeployWithMaxDataLen` instruction in Loader
V3 to create the programdata account with the System program's
`CreateAccountAllowPrefund` instruction ([SIMD-0312]) instead of
`CreateAccount`, so that pre-funded programdata accounts are accepted.

## Motivation

`DeployWithMaxDataLen` currently creates the programdata account by invoking
the System program's `CreateAccount` instruction via CPI. `CreateAccount` fails
with `AccountAlreadyInUse` if the target account holds any lamports.

This can create an annoying scenario if funds are sent to the programdata
account ahead of time, requiring a new program keypair to be generated. Using
`CreateAccountAllowPrefund` avoids this scenario without incurring any further
CPI cost to the Loader program.

## Dependencies

This proposal depends on the following previously accepted proposals:

- **[SIMD-0312]: CreateAccountAllowPrefund**

    Its feature, `6sPDzwyARRExKH52LECxcGoqziH8G7SZofwuxi8Ja331`, is active on
    all clusters (mainnet-beta, testnet, and devnet).

## New Terminology

N/A

## Detailed Design

`DeployWithMaxDataLen` MUST invoke `CreateAccountAllowPrefund` instead of
`CreateAccount` to create the programdata account.

`CreateAccountAllowPrefund` takes the same instruction data as `CreateAccount`
(`lamports`, `space`, `owner`), so the implementation is otherwise the same,
except for `lamports`. Instead of the full rent-exempt minimum, the loader
transfers only the remainder not already covered by the programdata account's
balance (or zero, if it is already covered). The payer is omitted when no
lamports are transferred.

Note that `CreateAccountAllowPrefund` takes its accounts in reverse order:

```text
0. [ws] ProgramData account
1. [ws] Payer, if lamports > 0
```

## Alternatives Considered

The loader could invoke `Allocate` and `Assign` without transferring any
lamports. This would require every caller to pre-fund the programdata account
before deployment, which breaks existing deployment workflows and is not
worth the developer experience cost.

## Impact

Deployments no longer fail when the programdata account holds lamports.

## Security Considerations

The target is always the derived programdata address, so the wallet-locking
concern noted in [SIMD-0312] does not apply.

## Backwards Compatibility

This proposal changes the behavior of `DeployWithMaxDataLen` and requires a
feature gate for consensus safety. The instruction's interface is unchanged.

[SIMD-0312]: https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0312-create-account-allow-prefund.md
