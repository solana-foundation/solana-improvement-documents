---
simd: '0344'
title: Dynamic State Rent
authors:
  - Max Resnick (Anza)
category: Standard
type: Core
status: Idea
created: 2025-08-23
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

This proposal introduces a new dynamic rent mechanism that prices space in
the active Solana state and governs when inactive accounts are compressible.

## Motivation

Solana state rent is too damn high; however, lowering state rent would result
in faster state growth resulting in larger snapshots and an inability to fit
all of the active accounts in memory, both of which would impact performance.
Lowering state rent therefore requires a plan to deal with the resulting state
growth. That plan, as outlined across SIMD 0329, SIMD 0341, and this SIMD
consists of 3 components:

1. An economic mechanism to limit state growth (This SIMD)
2. A predicate to determine which accounts can be compressed (This SIMD and
   SIMD 0329)
3. A compression scheme that removes accounts from the global state while
   allowing for safe and simple recovery (SIMD 0341)

## New Terminology

- **Dynamic Rent Rate**: The current rent rate per byte per epoch
- **Dynamic Rent Controller**: A PID controller that adjusts the Dynamic Rent
  Rate based on how close the actual state size is to the target.
- **State Size Target**: The target active account state size
- **PID Parameters**: Proportional (Kp), Integral (Ki), and Derivative (Kd)
  gains for the rent controller
- **Rent Paid**: A new data field containing the rent that has been paid in
  SOL
- **Compression Eligibility**: A boolean that signals whether an account is
  eligible for compression
- **Last Write Epoch**: Epoch when account was last written to (tracked per
  account)
- **Dynamic Rent History Sysvar**: A sysvar exposing the time series of
  dynamic rent rates per epoch
- **Epoch State Size History Sysvar**: A sysvar exposing the time series of
  total active account data size per epoch

## Informal Design Description

The protocol maintains a dynamic rent rate based on how close the historical
accounts state size has been to the state size target. This controller
increases rent when account size is higher than the target and decreases it
when it is lower. When an account is created, or decompressed, it pays at
least 15 epochs worth of rent at the current rate. When an account is written
for the first time in an epoch it pays at least 1 epoch worth of rent. If an
account has not payed rent in a while, and the rent due is more than the rent
paid, the account becomes eligible for compression at which point the leader
can submit a transaction with compression instructions and earn a reward for
decompressing the accounts. This reward is set such that it is never more than
the sol burned in rent so no net new sol is created through this rent
mechanism.

## Detailed Design

The proposal introduces two new system variables (Sysvars):

- Dynamic Rent History (`sysvar: sol_get_dynamic_rent_history`, id TBD)
- Epoch State Size History (`sysvar: sol_get_epoch_state_size_history`, id TBD)

Both sysvars share common design principles:

- Append-only per-epoch vectors that grow over time
- Single-writer: populated by the bank at epoch boundary; never writable by
  programs
- Accessible to programs via the generic sysvar syscall (SIMD-0127)

### Layouts

**Dynamic Rent History**

- Array of `u64` values (8 bytes each)
- `[rent_rate_0, rent_rate_1, rent_rate_2, ...]`
- Index N = rent rate for epoch N (starting from activation epoch)

**Epoch State Size History**  

- Array of `u64` values (8 bytes each)
- `[data_size_0, data_size_1, data_size_2, ...]`
- Index N = total active account data size for epoch N

### Population semantics

At each epoch boundary, the bank appends one `u64` value to each sysvar:

- **Epoch State Size History**: Appends the total active account data size
  (bytes) from the completed epoch
- **Dynamic Rent History**: Appends the rent rate (lamports per byte) for the
  upcoming epoch by calling `sol_update_rent_controller()`

### PID Controller Syscall

A new syscall `sol_update_rent_controller()` updates the dynamic rent rate
using a PID controller:

**Signature:**

```c
uint64_t sol_update_rent_controller(
    uint64_t current_state_size,  // Current total active account data size
                                  // (bytes)
    uint64_t target_state_size,      // Target state size (bytes) 
    uint64_t current_rent_rate,   // Current rent rate (lamports per byte
                                  // per epoch)
    uint64_t kp,                     // Proportional gain (scaled by 1e9)
    uint64_t ki,                     // Integral gain (scaled by 1e9)  
    uint64_t kd,                     // Derivative gain (scaled by 1e9)
    uint64_t min_rent_rate,          // Minimum allowed rent rate
);
```

**Returns:** New rent rate (lamports per byte per epoch)

**Behavior:**

- Calculates error: `error = current_state_size - target_state_size`
- Applies PID formula: `output = Kp*error + Ki*integral + Kd*derivative`
- Updates rent rate: `new_rate = current_rent_rate + output`
- Clamps result between `min_rent_rate` and `max_rent_rate`
- Maintains internal state (integral, previous error) across calls

**Access Control:** Only callable by the bank during epoch boundary processing

### Account Rent Paid Field

Each account gets new fields:

- **rent_paid**: `u64` field storing total rent paid in lamports
- **last_write_epoch**: `u64` field tracking when account was last written
- Added to account metadata alongside existing fields (lamports, data, owner,
  etc.)
- **rent_paid** is set to zero when account is compressed (rent is "consumed"
  by the compression operation)


### Rent Collection Behavior

The dynamic rent system integrates with account compression (SIMD-0341) to
enforce rent payments at key lifecycle events:

#### 1. Account Creation

When creating a new account:

```c
// MUST pay at least 15 epochs worth of rent upfront
required_rent = account_data_size * current_rent_rate * 15;
if (transaction_rent_payment < required_rent) {
    return ERROR_INSUFFICIENT_RENT;
}
account.rent_paid = transaction_rent_payment;
account.last_write_epoch = current_epoch;
```

#### 2. Account Rehydration (Decompression)

When decompressing an account via `sol_decompress_account()`:

```c
// MUST pay at least 15 epochs worth of rent to reactivate
required_rent = account_data_size * current_rent_rate * 15;
if (transaction_rent_payment < required_rent) {
    return ERROR_INSUFFICIENT_RENT;
}
account.rent_paid = restored_rent_paid + transaction_rent_payment;
account.last_write_epoch = current_epoch;
```

#### 3. First Write Per Epoch

On the FIRST write to an account each epoch:

```c
// Calculate rent owed using historical rent rates since last write
rent_owed = account_data_size * sum(rent_history[epoch] for epoch in
                                    last_write_epoch..current_epoch);

// Payment processing
account.rent_paid += transaction_rent_payment;
account.rent_paid -= rent_owed;

// MUST pay at least 1 epoch's worth of rent ahead
minimum_rent = account_data_size * current_rent_rate * 1;
if (account.rent_paid < minimum_rent) {
    return ERROR_INSUFFICIENT_FUTURE_RENT;
}

account.last_write_epoch = current_epoch;
```

#### Compression Eligibility

**Function:** `sol_check_compression_eligibility(account_pubkey: &[u8; 32])`

**Purpose**: Determines if an account is eligible for compression based on
rent payment history.

**Parameters**:

- `account_pubkey`: 32-byte public key of the account to check

**Behavior**:

```c
bool sol_check_compression_eligibility(const uint8_t *account_pubkey) {
    AccountData account = get_account(account_pubkey);
    
    // Check account type eligibility
    if (!is_compressible_account_type(account)) {
        return false; // system accounts, vote accounts, stake accounts,
                      // reserved accounts etc. cannot be compressed
    }
    
    uint64_t *rent_history = sol_get_sysvar(dynamic_rent_history_id);
    uint64_t required_rent = 0;
    
    required_rent = account.data_size * sum(rent_history[epoch] for epoch in
                                            account.last_write_epoch..current_epoch);
    
    return account.rent_paid >= required_rent; /// Returns true if account
                                                /// is eligible for compression,
                                                /// false otherwise
}

bool is_compressible_account_type(AccountData account) {
    // Program accounts (executable = true)
    if (account.executable) {
        return true;
    }
    
    // Custom program-owned accounts (owner != system program)
    if (account.owner != SYSTEM_PROGRAM_ID && 
        account.owner != VOTE_PROGRAM_ID &&
        account.owner != STAKE_PROGRAM_ID) {
        return true;
    }
    
    // Data accounts with no special restrictions
    if (account.data_size > 0) {
        return true;
    }
    
    return false; // System accounts, empty accounts not eligible
}
```

### Compression Reward

When an account is successfully compressed, the entity performing the
compression operation receives a reward:

```c
// Reward is capped at the minimum of one epoch's worth of current rent and
// the account's rent_paid.
// This second condition ensures that no new SOL is minted by this scheme.
reward = min(account.rent_paid, account.data_size * current_rent_rate);
compressor_account.lamports += reward;
account.rent_paid = 0;
```

### Integration with Existing Systems

#### RPC Changes

**New RPC Methods:**

```javascript
getAccountCompressionStatus(pubkey) // Returns compression status and
                                    // eligibility
estimateDecompressionCost(pubkey)   // Returns required rent payment for
                                    // decompression
```

**Modified RPC Methods:**

```javascript
getAccountInfo()      // Returns compression status in account metadata
simulateTransaction() // Includes rent collection costs in simulation and fee
                      // payer validation
sendTransaction()     // Validates fee payer has sufficient funds for rent
                      // collection on first epoch access
```

#### Wallet Integration

**Account Access Flow:**

```c
if (account_is_compressed(pubkey)) {
    uint64_t required_rent = 15 * account_data_size * current_rent_rate;
    if (user_confirms_decompression_cost(required_rent)) {
        Transaction transaction = create_decompression_transaction(pubkey,
                                     original_data, required_rent);
        send_transaction(transaction);
    } else {
        show_error("Account compressed, decompression required");
    }
} else {
    // Check if first access this epoch requires rent collection
    if (is_first_write_this_epoch(pubkey)) {
        uint64_t rent_owed = calculate_rent_owed_since_last_write(pubkey);
        uint64_t total_cost = transaction_fee + rent_owed;
        if (fee_payer_balance < total_cost) {
            show_error("Insufficient funds for transaction fee + rent"
                       " collection");
            return;
        }
    }
    proceed_with_transaction();
}
```

**Fee Payer Validation:**

```c
// RPCs MUST validate fee payer has sufficient funds for transaction fees +
// rent collection
total_cost = transaction_fee + rent_collection_cost;
if (fee_payer.lamports < total_cost) {
    return ERROR_INSUFFICIENT_FUNDS_FOR_RENT;
}

// Wallets MUST estimate and display total transaction cost including rent
display_cost_breakdown(transaction_fee, rent_collection_cost, total_cost);
```

## Alternatives Considered

- Chili Peppers is a proposal that dealt only with the problem of
  differentiating disk and memory reads. This proposal hopes to eliminate the
  need for account data to be stored on disk through account compression.
- Lowering rent without a plan for dealing with the ensuing state growth is
  not a good idea.

## Impact

### Users

**Positive:**

- **Lower long-term rent costs**: Dynamic rates will decrease when state size
  is below target, reducing costs for active accounts
- **Dynamic pricing**: Historical rent calculation ensures fair costs even
  during rate transitions
- **Improved network performance**: Smaller active state leads to faster sync
  times and better validator performance

**Negative:**

- **Complexity**: Users must understand compression status and decompression
  costs
- **Potential access delays**: Compressed accounts require decompression
  transaction before use

### Validators

**Positive:**

- **Reduced storage requirements**: Compressed accounts significantly reduce
  disk and memory usage
- **Faster sync times**: Smaller snapshots improve bootstrap and catchup
  performance
- **Economic incentives**: Validators can earn compression rewards by cleaning
  up eligible accounts

**Negative:**

- **Implementation complexity**: New rent collection logic, compression
  tracking, and sysvar management

### Developers

**Positive:**

- **Scalability improvements**: Network can handle larger applications without
  prohibitive rent costs

**Negative:**

- **Breaking changes**: Programs must handle compressed account access
  patterns
- **Increased complexity**: Need to integrate compression checks and error
  handling

### Network

**Long-term benefits:**

- **Sustainable scaling**: Economic incentives naturally control state growth
  without hard limits
- **Market efficiency**: Dynamic Rent will find the market rate for time spent
  in the active account state

## Security Considerations

### Economic Attack Vectors

**Rent Rate Manipulation:**

- **Attack**: Coordinated creation/deletion of large accounts to manipulate
  PID controller
- **Mitigation**: PID controller parameters tuned for stability; 15-epoch
  upfront cost makes manipulation expensive

**Compression Reward Exploitation:**

- **Attack**: Creating accounts solely to compress them for rewards
- **Mitigation**: Reward capped at `min(rent_paid, 1_epoch_rent)` ensures no
  net SOL creation; 15-epoch upfront cost exceeds maximum reward

**State Size Attacks:**

- **Attack**: Rapidly expanding state size to force high rent rates
- **Mitigation**: High upfront costs (15 epochs) and ongoing rent collection
  make sustained attacks economically prohibitive

### Technical Attack Vectors

**Sysvar Data Integrity:**

- **Attack**: Corrupting historical rent or state size data
- **Mitigation**: Sysvars are write-protected from user programs; only bank
  can update during epoch boundaries; deterministic across all validators

**Compression Eligibility Bypass:**

- **Attack**: Accessing compressed accounts without proper decompression
- **Mitigation**: Runtime enforces compression status checks; compressed
  account access returns specific error codes; syscalls validate account state

**Fee Payer Bypass:**

- **Attack**: Submitting transactions without sufficient funds for rent
  collection
- **Mitigation**: RPCs and runtime validate total cost (fees + rent) before
  transaction execution; transactions fail atomically if insufficient funds

### Consensus and Fork Safety

**Historical Data Consistency:**

- **Risk**: Forks could have different historical rent/state data
- **Mitigation**: Sysvars follow bank state; fork resolution ensures
  consistent history; bank hash includes compressed account state

**Epoch Boundary Race Conditions:**

- **Risk**: Rent collection and PID updates could be inconsistent across
  validators
- **Mitigation**: All epoch boundary operations are deterministic; rent rates
  finalized before application; strict ordering of operations

### DoS and Resource Exhaustion

**Sysvar Size Growth:**

- **Risk**: Unbounded growth of historical data sysvars
- **Mitigation**: predictable growth rate (16 bytes per epoch per sysvar)

**Compression/Decompression Spam:**

- **Risk**: Excessive compression operations consuming compute resources
- **Mitigation**: blocks have max account state delta capped at 100mb

### Privacy and MEV Considerations

**Compression Timing:**

- **Risk**: MEV extraction from compression reward opportunities
- **Mitigation**: Only the leader can compress transactions

## Backwards Compatibility

### Breaking Changes

**Account Structure Changes:**

- New `rent_paid` and `last_write_epoch` fields added to all accounts
- Existing accounts will have these fields initialized to 0 during activation
- Account serialization format changes require updated client libraries

**Transaction Validation Changes:**

- Account creation now requires 15 epochs of rent payment
- First write per epoch triggers automatic rent collection
- Transactions may fail due to insufficient rent funds where they previously
  succeeded

**RPC API Changes:**

- `getAccountInfo()` returns additional compression and rent status fields
- `simulateTransaction()` includes rent collection costs in estimates
- New RPC methods for compression status and cost estimation

**Program Behavior Changes:**

- Account access patterns must handle compressed account errors
- Programs need to integrate rent payment syscalls for optimal UX
- Cross-program invocation may fail if target accounts are compressed

### Migration Plan

**Phase 1: Feature Activation (Epoch N)**

```c
// Initialize new account fields for all existing accounts
for (account in all_accounts) {
    account.rent_paid = 0;
    account.last_write_epoch = current_epoch;
}

// Activate sysvars with empty history
dynamic_rent_history = [];
epoch_state_size_history = [];

// PID controller starts at current rent level, disabled until old state
// evicted
current_rent_rate = existing_rent_rate; // Maintain current pricing
pid_controller_enabled = false;
```

**Phase 2: Rent Collection (Epoch N+1)**

```c
// Begin rent collection on first write
// Existing accounts get grace period - no historical rent owed initially
if (account.last_write_epoch == activation_epoch) {
    rent_owed = 0; // Grace period for existing accounts
} else {
    rent_owed = calculate_historical_rent(account);
}
```

**Phase 3: Compression Eligibility (Epoch N+10)**

```c
// Enable compression after sufficient history accumulates
if (current_epoch >= activation_epoch + 10) {
    enable_account_compression();
}
```

**Phase 4: PID Controller Activation (Epoch N+25)**

```c
// Enable dynamic rent adjustment only after old account state has been
// evicted
// This ensures PID controller operates on accounts that have paid proper rent
if (current_epoch >= activation_epoch + 25) {
    pid_controller_enabled = true;
    // Begin adjusting rent rates based on actual vs target state size
}
```

### Client Migration Requirements

**Wallet Updates:**

- Update transaction cost estimation to include rent collection
- Add UI for compression status and decompression costs
- Implement automatic rent payment for account access

**RPC Provider Updates:**

- Deploy updated RPC methods for compression and rent APIs
- Update transaction simulation to include rent costs
- Implement fee payer validation with rent collection

**Program Updates:**

- Add error handling for compressed account access
- Update account access patterns to check compression status

### Compatibility Guarantees

**Existing Account Data:**

- All existing account data remains accessible during migration
- No data loss or corruption during field addition
- Existing programs continue to function with rent collection

**API Compatibility:**

- Existing RPC methods maintain backward compatibility with additional fields
- Old client libraries receive default values for new fields
- Gradual deprecation of old APIs over multiple epochs

**Network Stability:**

- Phased rollout prevents sudden economic shocks
- Grace period for existing accounts prevents immediate rent collection
- PID controller starts at current rent level and remains disabled until old
  account state is evicted
- Dynamic rent adjustment only begins after the system reaches steady state
