---
simd: '0047'
title: Syscall and Sysvar for last restart slot
authors:
  - Godmode Galactus (Mango Markets)
category: Standard
type: Core
status: Implemented
created: 2023-04-15
feature: [HooKD5NC9QNxk25QuzCssB8ecrEzGt6eXEPBUxWp1LaR](https://github.com/solana-labs/solana/issues/32177)
development: 
 - Anza - [implemented in 1.17.0](https://github.com/solana-labs/solana/pull/31957) 
 - Firedancer - implemented
---

## Summary

Create a new syscall and sysvar that can be used to get the last restart slot.

A new Sysvar :
`SysvarLastRestartS1ot1111111111111111111111`

Method signature for syscall:
`fn sol_get_last_restart_slot() -> Slot`

## Motivation

In Solana, when the cluster cannot reach consensus, it is currently restarted
using `ledger-tool` with a hard fork on a slot. All participating nodes need to
restart their validator specifying the hard fork slot. Once all nodes
participating in the restart effort exceed 80% of the total stakes, the restart
is successful, and the cluster continues from the hard fork slot. So hard fork
is a tool to intentionally fork off the nodes not participating in the restart
effort.

Program developers may find it useful to know that a cluster restart has
recently occurred. This information can help them prevent arbitrage and
liquidation caused by outdated oracle price account states. However, the
cluster's restart process takes several hours; in that time, the world can
change, causing asset prices to fluctuate. As the cluster updates the state of
all accounts, malicious actors could take advantage of the delay by conducting
trades using outdated prices, anticipating that they will soon catch up to the
actual prices of real-world assets. Knowing that cluster restart has been done
recently, programs can manage these cases more appropriately.

## Alternatives Considered

We need to have the value of the last restart slot while executing the program.
We cannot use an account because then it should be updated just after the
restart is successful, which will add complexity. The best way is to create a
new syscall and sysvar to get this information during the execution of a
transaction, which will help us get the last restart slot. We prefer syscall
over sysvar because the newly created sysvar has to be passed in the instruction
this takes up transaction accounts space. In the long-term Solana evolution, we
plan to get rid of syscalls and make everything a sysvar, so we have decided to
implement both a syscall and a sysvar to address short and long-term evolution.

## New Terminology

None

## Detailed Design

The following criteria should be met to choose a restart slot.

* Should be of type unsigned int 64 bits.
* Should be indicative of when the cluster was restarted
* Should be monotonically increasing
* Should be less than equal to the current slot
* Should be synchronized in the whole cluster
* Can also be indicative of a slow block
* The first restart slot should be `0`

In Solana client `hard fork` variable satisfies nearly all the conditions except
it is not indicative of slow block, it is a good candidate.

The syscall and sysvar will be available in architectures eBPF and SBF.
For now, this functionality is left undefined in program runtime v2.

### Feature Gate

This feature should be protected by the feature gate this is because this
feature can create differences in consensus. Solana core developers should
create a new keypair and use it to enable this feature on the cluster.

Upon activation, the functionality described below becomes available in the next
epoch.

### Creation of a new sysvar

Similar to the clock or rent sysvar we can create a new sysvar for last restart
slot. We have to implement a new class to store the required values, which then
could be used by sysvar during execution.

Sysvar Id: `SysvarLastRestartS1ot1111111111111111111111`

### Creation of a new syscall

The implementation of this syscall is pretty straightforward. The syscall should
have following signature:

`fn sol_get_last_restart_slot() -> u64`

Murmur3 hash is: `0xb532af38`

### Overview of changes for Solana client

In Solana Labs validator client all the cluster restart-related data is already
stored in the bank structure. For other validator clients, we have to get the
last restart slot information and make it accessible to the runtime of the
program. We have to set this data in invoke context executing the program. Then
the syscall and sysvar can use invoke context to return the data to the
executing program.

### Compute budget for the syscall

As this feature involves moving a u64 from the bank to the invoke context and
returning it when syscall or sysvar is called.

So considering :

```
/// Cluster averaged compute unit to micro-sec conversion rate
COMPUTE_UNIT_TO_US_RATIO = 30;
```

We can set the maximum compute limit for the syscall to:

```
COMPUTE_UNITS_LIMIT = 2 * COMPUTE_UNIT_TO_US_RATIO
```

## Impact

Programs managing time-sensitive states may upgrade to use this syscall to
better manage exceptional events like restarts. We expect this change to improve
the robustness of Solana's financial markets as outlined above.

Programs will need to be recompiled and redeployed (upgraded) to adopt this
feature. Interface changes (e.g. account inputs) are not required.

## Security Considerations

None

## Backwards Compatibility

The programs using the new syscall could not be used on Solana version which
does not implement this feature. Existing programs that do not use this feature
are not impacted at all. Feature gate should be used to enable this feature when
the majority of the cluster is using the required version.
