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

Create a new syscall which can be used to get the last restart slot (currently
the last time hard fork was done on the cluster).

`fn sol_get_last_restart_slot() -> Slot`

## Motivation

In Solana, when the cluster cannot reach consensus, it is currently restarted
using `ledger-tool` with a hard fork on a slot. All participating nodes need to
restart their validator specifying the hard fork slot. Once all nodes
participating in the restart effort on the hard fork exceed 80% of the total
stakes, the restart is successful, and the cluster continues from the hard fork
slot. So hard fork is a tool to intentionally fork off the nodes not
participating in the restart effort. Currently can consider the slot on which
hardfork was done as a restart slot.

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

No alternate considerations; we need to have the value of last restart slot
while executing the program to correctly treat this case. We cannot have an
account because then it should be updated just after the restart is successful,
which will add complexity. The best way is to create a new syscall to get this
information during execution of transaction, which will help us get last
restart slot without any interface changes for the instruction.

## Detailed Design

Currently in solana client validator `hard fork` slots are a good indicator when
cluster was restarted. This may change in the future following criteas should be
met to choose restart slot.

* Should be indicative of when cluster was restarted
* Should be monotonically increasing
* Sould be less than equal to current slot

The next part will consider hardfork slot is equal to restart slot.

### Creation of a new syscall

The implementation of this syscall is pretty straitforward. In solana client all
the hardforks for a cluster are stored in the bank structure. The last hard fork
slot can be retrieved and then stored in invoke context, so that the executing
program can access it.

For other clients, we have to get the last hard fork slot information and make
it accessible to the runtime of the program. If there is no hard fork done yet
on the cluster we consider that the first hard for is at Slot `0`.

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

We can then return the data of last hardfork slot passed to the invoke context
in this functions implementation.

## Impact

Programs will start using this new syscall to correctly address the security
concerns during network restart. This will increase the reliability of solana
cluster as whole and make programs developers more confident to handle edge
cases during such extreme events.

As the method is syscall the developers do not need to pass any new
accounts or sysvars to the instruction to use this feature.

## Security Consideration

None

## Backwards Compatibility

The programs using the new syscall could not be used on solana version which
does not implement this feature. Existing programs which do not use this feature
are not impacted at all.