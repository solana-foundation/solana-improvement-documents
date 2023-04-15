---
simd: '0047'
title: Syscall to get the last hardfork
authors:
  - Godmode Galactus (Mango Markets)
category: Standard
type: Core
status: Draft
created: 2023-04-15
---

## Summary

Create a new sysvar `SysvarLastHardFork1111111111111111111111111`, which can
be used to get the last hard fork slot.

## Motivation

In Solana, when the cluster cannot reach consensus, it is restarted using
`ledger-tool` with a hard fork on a slot. A hard fork is usually done on an
optimistically confirmed slot which was voted on and accepted by 50% of the
cluster but still was unable to get a supermajority. The cluster is then
restarted from the hard fork slot, and all participating nodes need to restart
their validator specifying the hard fork slot. Once all nodes participating in
the restart effort on the hard fork exceed 80% of the total stakes, the restart
is successful, and the cluster continues from the hard fork slot. So hard fork
is a tool to intentionally fork off the nodes not participating in the restart
effort. After successfully restarting the network the hard fork slot is stored
in a variable called `hard_forks` in bank.

Dapp developers may find it useful to know that a hard fork has recently
occurred. This information can help them prevent arbitrage and liquidation
caused by outdated oracle price account states. However, the cluster's restart
process takes several hours; in that time, the world can change, causing asset
prices to fluctuate. As the cluster updates the state of all accounts,
malicious actors could take advantage of the delay by conducting trades using
outdated prices, anticipating that they will soon catch up to the actual
prices of real-world assets. Knowing that hard fork has been done recently,
dapps can manage these cases more appropriately.

## Alternatives Considered

No alternate considerations; we need to have the value of last hard fork slot
while executing the dapp to correctly treat this case. We cannot have an
account because then it should be updated just after the restart is successful,
which will add complexity. The best way is to create a new sysvar to get this
information during execution of transaction.

## Detailed Design

### Creation of a new sysvar

Addition of a new file `last_hard_fork.rs` at following location:
`monorepo/sdk/program/src/sysvar` to implement the new sysvar.

We should also implement a new structure to store the last hard fork data
similar to:

``` rust
#[repr(C)]
#[derive(Serialize, Deserialize, Debug, CloneZeroed, Default, PartialEq, Eq)]
pub struct LastHardFork {
  slot: Slot,
  count: u64,
}
```

`Sysvar` trait should be implemented for above structure.

``` rust

crate::declare_sysvar_id!("SysvarLastHardFork1111111111111111111111111",
LastHardFork);

impl Sysvar for LastHardFork {
    impl_sysvar_get!(sol_get_last_hard_fork_sysvar);
}

```

### Loading of sysvar during banking stage

The hardfork data is available in the `Bank` structure in the field
`hard_forks`. The structure `HardForks` contains a vector with all the previous
hard forks. The vector is also sorted when we register a slot. So the last
element of the vector is usually the last slot at which hard fork was done.

We can get the last hard fork slot from the bank and pass it to invoke context
structure. We also have to add a new field to the structure `SysvarCache` so
that this data could be efficiently cached. Now we can easily load the last
hard fork data when sysvar is called from invoke context.

Create a file `monorepo/programs/bpf_loader/src/syscalls/last_hard_fork.rs` to
declare syscall class `SyscallLastHardFork`. This helps to initialize the
strucure `LastHardFork` from invoke context and sysvar cache.

Registering `SyscallLastHardFork` in the file
`monorepo/programs/bpf_loader/src/syscalls/mod.rs` in method
`create_loader` like other sysvars.

### Updating documentation

We should add correct documentation for the new sysvar in `sysvar.md` file.

## Impact

Dapps will start using this new sysvar to correctly address the security
concerns during network restart. This will increase the reliability of solana
cluster as whole and make dapps more confident to handle edge cases during
such extreme events.

## Security Consideration

None

## Backwards Compatibility

The dapps using the new sysvar could not be used on solana version which does
not implement this feature. Existing dapps which do not use this feature are
not impacted at all.