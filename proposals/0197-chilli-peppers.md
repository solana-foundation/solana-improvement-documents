---
simd: '0197'
title: Chilli Peppers
authors:
  - Firedancer Team
category: Standard
type: Core
status: Review
created: 2024-11-19
feature: (fill in with feature tracking issues once accepted)
---


## Summary
 
This proposal adds a new consumable resource governing tiered memory bandwidth usage similar to the way that Compute Units seek to govern CPU usage.
Tiered memory bandwidth will become a performance bottleneck as transaction throughput and total state size increase. This proposal serves to outline changes to the Solana protocol that would enable:
Deterministic, easily computable and cluster-wide separation of state into hot and cold tiers
A new transaction level resource requesting the transfer of state from cold to hot
Block level constraints on the total cold to hot state transition
These added features will allow for a pricing market for the bandwidth from cold state to hot state (via priority fees), and allow block producers to more optimally pack blocks to get the highest possible throughput on already hot state and constrain access to cold state to be within bounds that the validator network will be able to keep up with.
 
 
 
## Motivation
 
In commodity hardware (for fixed cost), there is a fundamental tradeoff between the the size of accessible state and the bandwidth of random access to that state. On-chip caches >> RAM >> SSD >> HDD >> NAS increase by orders of magnitude in size, while falling by orders of magnitude in bandwidth.
For Solana (or any blockchain), treating all state as equivalent (regardless of its usage patterns) means that either total state size will
be limited by the size of RAM, or the throughput of the network will be limited to the bandwidth of disks. Actual usage patterns
(and expectations for future usage patterns as the network grows) show that a relatively small amount of the total state is accessed
frequently, and most of the state is accessed infrequently.
This usage pattern allows a hot/cold tiered state design to allow the total state size available from disk, while achieving the
throughput available from RAM.
 
## New Terminology
 
Chili Peppers (State Units?) - a consumable resource representing the number of bytes loaded into the "hot store" (active, frequently
accessed memory) for state operations on the Solana blockchain. Each Chili Pepper corresponds directly to one byte of data loaded, providing
a precise mechanism to quantify and limit the resources consumed by transactions in terms of state memory usage.
Note that Chili Peppers required for a transaction are not the size of hot state touched by the transaction, but rather the amount of cold
state made hot or new state allocated. Transactions that are only accessing already hot state would require 0 Chili Peppers.
 
Block Chili Pepper Limit - The maximum number of Chili Peppers that can be requested in a single block.
 
Block Chili Pepper Clock - a cumulative measure of the total Chili Peppers requested since the genesis of the blockchain. At the beginning of each block, the Block State Unit Clock is updated to reflect the cumulative total of Chili Peppers consumed up to that point, ensuring a monotonically increasing record of state consumption.
 
Account Chili Pepper Clock Timestamp - each account in the "hot" state needs to keep track of the value of the Block Chili Pepper Clock the last time it was accessed (read or written to). This allows determination of which accounts are hot and which are cold.
 
Hot Cache Size - A new predefined constant, termed "Hot Cache Size" corresponds approximately to the size of the hot state supported
by the validator network. An account is designated as hot if it was last
accessed within this threshold of the current Block Chili Pepper Clock
(Account Chili Pepper Clock Timestamp > Block Chili Pepper Clock - Hot Cache Size), otherwise it is designated
as cold.
 
 
## Detailed Design
 
### Integration with ComputeBudget Program
 
To facilitate the utilization of Chili Peppers, every transaction on the Solana network will be required to use the ComputeBudget program to request the maximum number of Chili Peppers they require. Alongside existing functionalities, a new instruction will be introduced to specify the requested State Units for the transaction. This ensures that developers have the flexibility to request resources based on the anticipated needs of their transactions, within the constraints of the block's Chili Peppers capacity.
The new ComputeBudgetInstruction will be as follows:
 
```rust
discriminant: 5
ComputeBudgetInstruction::SetChiliPepperLimit(u32)
```
 
The 32-bit unsigned integer in the instruction indicates the number of Chili Peppers requested for the transaction.
 
### Block Chili Pepper Clock
 
The Block Chili Pepper Clock serves as a cumulative measure of the total Chili Peppers requested since the genesis of the blockchain. At the beginning of each block, the Block State Unit Clock is updated to reflect the cumulative total of Chili Peppers requested up to that point, ensuring a monotonically increasing record of state consumption.
 
Computing the block Chili Pepper clock:
 
```python
block_chili_pepper_clock = prev_block_chili_pepper_clock + sum(txn.requested_chili_peppers for txn in block.txns)
```
 
Implemented as a 64-bit unsigned integer (uint64), this clock is updated at the beginning of every block to reflect the total Chili Peppers requested since the chain's genesis. This monotonically increasing value is stored in a dedicated system variable (sysvar), ensuring that it remains accessible and immutable throughout the blockchain's operation.
The new sysvar will have identifier: `SysvarB1ockChiliPepperC1ock111111111111111111`
 
```rust
struct SysvarBlockChiliPepperClock {
  uint64_t state_unit_clock;
}
```
 
 
 
### Account Chili Pepper Clock Timestamp
 
The Account Chili Pepper Clock Timestamp is an integral component within each "hot" account, representing the account's interaction with the blockchain's
state management resources. This clock is dynamically set to match the current Block Chili Pepper Clock at any instance an account is accessed (read or written to) within a given block.
These clocks are also implemented as 64-bit unsigned integers. Whenever a hot account is read from or written to within a block, its Account State Unit Clock is updated to match the current Block State Unit Clock.
All accounts will have a Chili Pepper Clock Timestamp, which will only exist for accounts which are hot, and is discarded for accounts which are cold. All implementations should keep track of the current hot accounts.
 
#### Hot and Cold Account Designation
 
An account is designated as cold when its Account Chili Pepper Clock Timestamp falls behind the current Block Chili Pepper Clock by more than the Hot Cache Size parameter.
An account which has never existed is considered cold. An account that is deleted is still considered hot until its state unit clock lapses into cold. Creating an account against a
deleted account which is still hot, will create the hot account again.
 
#### Storage and Management
 
To manage the Account State Unit Clocks efficiently, Solana employs a table, associating each hot account with its respective Chili Pepper Clock Timestamp. This table enables the dynamic tracking and updating of account states, facilitating the transition of accounts between hot and cold statuses based on their activity.
 
### Error Cases for State Units Implementation
 
Here are common error scenarios related to State Units and their respective handling mechanisms:
 
#### Exceeding Block Chili Pepper Limit
 
- **Error Description**: This error occurs when a transaction's requested Chili Peppers exceed the remaining capacity of the Block Chili Pepper Clock for the current block.
- **Handling**: The block is marked as invalid and cannot be processed.
 
#### Invalid Chili Pepper Request
 
- **Error Description**: A transaction specifies an invalid number of Chili Peppers, either by requesting more than a predefined maximum limit per transaction or by formatting the request improperly.
- **Handling**: The transaction is invalidated, and an "Invalid Chili Pepper Request" error message is issued. Developers must ensure that Chili Pepper requests conform to protocol specifications, including proper formatting and adherence to maximum limits.
 
#### Account State Unit Clock Synchronization Failure
 
#### Accessing Cold Account
 
- **Error Description**: A transaction attempts to interact with an account that has been designated as "cold" due to its Chili Pepper Clock Timestamp falling below the "Hot Cache Size" threshold relative to the current Block Chili Pepper Clock, without requesting sufficient Chili Peppers.
- **Handling**: The transaction is rejected with a "Cold Account Access Attempted" error.
 
 
## Alternatives Considered
 
What alternative designs were considered and what pros/cons does this feature
have relative to them?
 
## Impact
 
How will the implemented proposal impacts dapp developers, validators, and core contributors?
 
## Security Considerations
 
What security implications/considerations come with implementing this feature?
Are there any implementation-specific guidance or pitfalls?

 
## Backwards Compatibility 
 
This proposal requires changes to the Solana Runtime Protocol and the ComputeBudget program. It is not backwards compatible and will require updates to existing programs and transactions to specify State Unit requirements. A transition period and comprehensive developer support will be essential to implement these changes smoothly across the ecosystem.