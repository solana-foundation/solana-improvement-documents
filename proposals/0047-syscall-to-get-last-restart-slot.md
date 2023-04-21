---
simd: '0047'
title: Syscall to get the last restart slot
authors:
  - Godmode Galactus (Mango Markets)
category: Standard
type: Core
status: Draft
created: 2023-04-15
---

## Summary

Create a new syscall that can be used to get the last restart slot (currently
the last time hard fork was done on the cluster).

`fn sol_get_last_restart_slot() -> Slot`

## Motivation

In Solana, when the cluster cannot reach consensus, it is currently restarted
using `ledger-tool` with a hard fork on a slot. All participating nodes need to
restart their validator specifying the hard fork slot. Once all nodes
participating in the restart effort on the hard fork exceed 80% of the total
stakes, the restart is successful, and the cluster continues from the hard fork
slot. So hard fork is a tool to intentionally fork off the nodes not
participating in the restart effort. Currently, we can consider the slot on
which hard fork was done as a restart slot.

Program developers may find it useful to know that a hard fork has recently
occurred. This information can help them prevent arbitrage and liquidation
caused by outdated oracle price account states. However, the cluster's restart
process takes several hours; in that time, the world can change, causing asset
prices to fluctuate. As the cluster updates the state of all accounts, malicious
actors could take advantage of the delay by conducting trades using outdated
prices, anticipating that they will soon catch up to the actual prices of
real-world assets. Knowing that cluster restart has been done recently, programs
can manage these cases more appropriately.

## Alternatives Considered

We need to have the value of the last restart slot while executing the program.
We cannot use an account because then it should be updated just after the
restart is successful, which will add complexity. The best way is to create a
new syscall or sysvar to get this information during the execution of a
transaction, which will help us get the last restart slot. We prefer syscall
over sysvar because the newly created sysvar has to be passed in the instruction
this takes up transaction accounts space. This will break existing program
interfaces, and the transaction account space is limited which is already a pain
point for many DeFi projects.

## New Terminology

None

## Detailed Design

Currently, in Solana Labs validator client `hard fork` slots are good indicators
when the cluster was restarted. This may change in the future following criteria
that should be met to choose a restart slot.

* Should be indicative of when the cluster was restarted
* Should be monotonically increasing
* Should be less than equal to the current slot

The next part will consider hard fork slot is equal to the restart slot.

### Creation of a new syscall

The implementation of this syscall is pretty straightforward. In Solana Labs
validator client all the hard forks for a cluster are stored in the bank
structure. The last hard fork slot can be retrieved and then stored in invoke
context, so that the executing program can access it.

For other validator clients, we have to get the last hard fork slot information
and make it accessible to the runtime of the program. If there is no hard fork
done yet on the cluster we consider that the first hard fork is at Slot `0`.

### Overview of changes for solana client

The hardfork data is available in the `Bank`. The structure `HardForks` contains
a vector with all the previous hard forks. The vector is also sorted when we
register a slot. So the last element of the vector is usually the last slot at
which hard fork was done. We can get the last hard fork slot from the bank and
pass it to invoke context structure. We can register new syscall in the file
`definitions.rs` where other syscalls are defined.

```rust
define_syscall!(fn sol_get_last_restart_slot() -> Slot);
```

We can then return the data of the last hard fork slot passed to the invoke
context in this function's implementation.

## Impact

Programs will start using this new syscall to correctly address the security
concerns during network restart. This will increase the reliability of solana
cluster as a whole and make program developers more confident to handle edge
cases during such extreme events.

As the method is syscall the developers do not need to pass any new
accounts or sysvars to the instruction to use this feature.

## Security Considerations

None

## Backwards Compatibility

The programs using the new syscall could not be used on solana version which
does not implement this feature. Existing programs that do not use this feature
are not impacted at all.
