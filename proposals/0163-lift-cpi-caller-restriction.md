---
simd: '0163'
title: Lift the CPI caller restriction
authors:
  - Alexander Meißner
category: Standard
type: Core
status: Withdrawn
created: 2024-07-16
feature: HcW8ZjBezYYgvcbxNJwqv1t484Y2556qJsfNDWvJGZRH
---

## Summary

Remove the check which forces CPI callers to have the program account of the
callee available to them as an instruction account.

## Motivation

The restriction is purely historical and not necessary for the protocol or its
implementations.

Removing it would improve composability and reduce transaction building
complexity because program accounts would no longer be passed all the way from
transaction level instructions to the inner most CPI instructions.

Additionally, not passing these program accounts is a significant reduction in
data copied during serialization as program accounts of loader v1, v2 and v4
contain the entire 10 MB ELF. This massively reduces the CU cost for nested CPI
and disincentivizes programs from accessing program accounts, which could later
allow us to remove program loading from transaction loading.

### Example transaction

- Accounts:
  - Fee payer
  - Caller Program
  - Callee Program
  - Token
- Transaction-level instruction:
  - Program Account: Caller Program
  - Instruction Accounts:
    - **Callee Program** (this could be removed)
    - Token
  - CPI:
    - Program Account: Callee Program
    - Instruction Accounts:
      - Token

Currently, in order to be used as a program account for a nested instruction
the callee program must be passed as an instruction account to all outer
instructions recursively.

## New Terminology

None.

## Detailed Design

In CPI a feature gate must switch the search for the program id of the callee
from the instruction accounts of the caller to the account list of the
transaction.

Invoking a program in CPI which was un/re/deployed in the same transaction is
prevented by the "delay visibility" feature and thus unproblematic.

## Alternatives Considered

None.

## Impact

See motivation and security considerations.

Dapp developers who wish to benefit from the lifting of the restriction shall:

- Hard-code the callee address in the CPI call, in case they want a static
dispatch
- Use the instruction **data**, not instruction **accounts**, to receive the
callee address as a parameter, in case they want a dynamic dispatch

Transaction building should append the required program accounts, which are not
passed as instruction accounts, at the end of the transaction accounts list.
How the dapps describe which callee prorgams they require to be present in the
transaction is explicitly left unspecified.

## Security Considerations

For a program to be allowed to modify an account it must be the owner and the
instruction account must have been passed in with the `is_writable` flag set.
Some programs take custody of an accounts ownership and allow control via a
signer authority. In these cases the instruction account must have been passed
in with the `is_signer` flag set additionally. Both these flags can only be set
in the inner most nested CPI if all parent CPIs did also set them. However,
they can not be controlled at per top-level-instruction. All of these
instructions share the same flags throughout the transaction.

This means the CPI caller restriction has been used as a security feature to
prevent a top-level-instruction from calling into a program which has custody
of an account that another top-level-instruction required as a signer. Removing
this restriction would require a lot more care to be taken in transaction
builing to prevent the relaxation from being exploited.

## Backwards Compatibility

Programs remain unaffected the way they are, unless they want to profit from
this change. In that case they can do one of the following:

Existing programs, which have hard-coded the callee statically and only need it
as any instruction account to satisfy the constraint imposed by the runtime,
can be fed a placeholder like `NativeLoader1111111111111111111111111111111`, in
order not to shift the indices of the other instruction accounts.

All other existing programs, which dynamically call whatever is passed in a
specific instruction account will have to be updated and redeployed to benefit
from the lifting of the restriction.
