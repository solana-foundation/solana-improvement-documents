---
simd: '0049'
title: Syscall for remaining compute units
authors:
  - Christian Kamm
category: Standard
type: Core
status: Implemented
created: 2023-05-17
feature: [5TuppMutoyzhUSfuYdhgzD47F92GL1g89KpCZQKqedxP](https://github.com/solana-labs/solana/issues/33325)
development: 
 - Anza - [Implemented](https://github.com/solana-labs/solana/pull/31640) but held 
 - Firedancer - Prioritized
---

## Summary

Add a new syscall to access the number of compute units that are left to use
in a transaction.

## Motivation

Some instructions can use a variable amount of compute. Examples include:

- Event processing instructions that can process as many events as are available.
- Orderbook matching instructions, which want to match a large taker order as
  deeply into the orderbook as is necessary to fully fill it.
- Routing instructions, that may want to pick routes depending on remaining compute.

Currently these types of instructions often require the user to pass in
a limit (like the maximum number of events to process, for the first example).
That means the user needs to request the right compute budget for the transaction
and guess the best possible limits for their instructions.

This often leads to users over-provisioning compute budget for a transaction.
If the instruction could access the remaining budget, it could decide on its
own whether another round of processing would fit or not, leading to better
utilization of requested compute.

Example:

```
fn process_events() {
  while (true) {
    let event = peek_event();
    match event.type {
      CheapEvent => {
        if sol_remaining_compute_units() < 10_000 {
          break;
        }
        process_cheap(event);
      },
      ExpensiveEvent => {
        if sol_remaining_compute_units() < 40_000 {
          break;
        }
        process_expensive(event);
      }
    }
    pop_event();
  }
}
```

The example is inspired by an actual potential usecase in the Mango program. It
shows how it would be preferable to "process events for all the 150k CU that remain"
(so up to 15 cheap ones) instead of the current "process at most three events"
(because 4 expensive ones would not fit).

## Alternatives Considered

Instead of knowing the transaction-wide remaining budget, instructions could be
allowed to access the amount of compute they have already used.

Then instructions could be written in a way where users can pass in compute targets
and the instruction aims to stay below the target.

## New Terminology

None

## Detailed Design

Add a syscall `sol_remaining_compute_units() -> u64` that returns the remaining
compute budget for the current transaction.

The syscall must cost no more than the syscall base cost of 100 CU.

The PR is at https://github.com/solana-labs/solana/pull/31640.

## Impact

- New syscall, needs a feature flag
- Dapp developers get new capabilities

## Security Considerations

Programs having access to the remaining compute budget would allow them to:

- Check if they are the first instruction, but they can already do that via
  introspection.
- Guess the effect of a CPI call by observing how much compute it used. But since
  program and data bytes are public, that shouldn't create new capabilities.

## Backwards Compatibility

Not applicable.
