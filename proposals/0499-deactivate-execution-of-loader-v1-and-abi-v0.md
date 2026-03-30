---
simd: '0499'
title: Deactivate execution of loader-v1 and ABI-v0
authors:
  - Alexander Meißner (Anza)
  - Joe Caulfield (Anza)
category: Standard
type: Core
status: Review
created: 2026-03-16
feature: TBD
---

## Summary

Deactivate execution of loader-v1 and ABI-v0.

## Motivation

The trouble with ABIv0 is that it had no alignment padding in its serialization
format and simply ignored any alignment requirements for syscall parameters.
This has since become undefined behavior in Rust. Disabling execution of
loader-v1 would allow us to remove ABIv0 entirely (as it is the only loader
which supports that ABI version) and would reduce the maintenance and auditing
burden in most syscalls significantly.

## New Terminology

None.

## Detailed Design

After the activation of the associated feature key a validator must fail to
execute programs owned by loader-v1 throwing the error message:

- `TransactionError::InvalidProgramForExecution` for top level instructions at
transaction loading time (before executing the first instruction)
- `InstructionError::UnsupportedProgramId` for CPI calls

## Alternatives Considered

Continuing to support this barely used functionality.

## Impact

All programs owned by loader-v1 would stop working forever with their locked
funds effectively burned. This might be relevant for sleeper programs which
have not seen any activity in years.

| Program | Newest Slot | Oldest Slot | Slot Span | Age | Distribution |
| --- | --- | --- | --- | --- | --- |
| Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo | 404990100 | 404989073 | 1027 | 0s | ▃▃▂▄▂▂▃▁▆█ |
| 4ckmDgGdxQoPDLUkDT3vHgSAkzA3QRdNq5ywwY4sUSJn | 380543824 | 71298312 | 309245512 | 113.2d | █▅▁      ▁ |
| EUqojwWA2rd19FZrzeBncJsm38Jm1hEhE3zsmX3bRc2o | 403597839 | 380173287 | 23424552 | 6.4d | █▁▁   ▆▁ ▁ |
| BJ3jrUzddfuSrZHXSCxMUUQsjKEyLmuuyZebkcaFp2fg | 380321574 | 56516079 | 323805495 | 114.2d | █▂   ▁   ▁ |
| TokenSVp5gheXUvJ6jGWGeCsgPKgnE3YgdGKRVCMY9o | 318471951 | 31181988 | 287289963 | 400.5d | █   ▁ ▁▁   |
| J4zdjSD86JsUqnF1qPfQLiD9hhkCU9fRbUAJp9cFoUJJ | 333280982 | 70723587 | 262557395 | 332.0d | █      ▁   |
| BrEAK7zGZ6dM71zUDACDqJnekihmwF15noTddWTsknjC | 298961846 | 298250292 | 711554 | 490.9d | █          |

## Security Considerations

None.
