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
- **Dynamic Rent Controller**: An integral controller that adjusts the Dynamic Rent
  Rate based on accumulated excess state size over time.
- **State Size Target**: The target active account state size
- **Integral Gain**: The gain parameter (Ki) that controls how aggressively rent
  increases with accumulated excess state
- **Rent Paid**: A new data field containing the rent that has been paid in
  SOL
- **Compression Eligibility**: A boolean that signals whether an account is
  eligible for compression
- **Last Write Slot**: Slot when account was last written to (tracked per
  account)
- **Dynamic Rent History Sysvar**: A sysvar exposing the time series of
  dynamic rent rates per epoch
- **Epoch State Size History Sysvar**: A sysvar exposing the time series of
  total active account data size per epoch

## Informal Design Description

The protocol maintains a dynamic rent rate based on accumulated excess state
size over time. The controller tracks the running sum of (actual_state -
target_state) and increases rent as an increasing function of this accumulated
excess. The accumulator never goes below zero, so rent only increases when
state is above target. When an account is created, or decompressed, it pays at
least 15 epochs worth of rent at the current rate. When an account is written
to, it pays rent based on slots elapsed since its last write slot, with proper
handling of partial epochs. When an account's size
changes, it must also pay 15 epochs worth of rent upfront at the new account size.
If an account has not paid sufficient rent, and the rent due is more than the rent
paid, the account becomes eligible for compression at which point compression
instructions can be submitted to compress the account.

## Detailed Design

The proposal introduces two new system variables (Sysvars):

- Dynamic Rent History (`sysvar: dynamic_rent_history`, id TBD)
- Epoch State Size History (`sysvar: epoch_state_size_history`, id TBD)

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

### Integral Controller Syscall

A new syscall `sol_update_rent_controller()` updates the dynamic rent rate
using an integral controller:

```c
// Pseudocode for integral controller behavior
uint64_t sol_update_rent_controller(
    uint64_t current_state_size,
    uint64_t target_state_size,
    uint64_t current_rent_rate,
    uint64_t integral_gain,
    uint64_t min_rent_rate,
    uint64_t max_rent_rate
) {
    // Calculate excess state (can be positive or negative)
    int64_t excess = (int64_t)current_state_size - (int64_t)target_state_size;
    
    // Update running accumulator (clamped to never go below 0)
    static int64_t accumulator = 0;  // Persistent across calls
    accumulator = max(0, accumulator + excess);
    
    // Calculate new rent rate (direct function of accumulator)
    uint64_t new_rate = integral_gain * accumulator / 1e9;
    
    // Clamp result between min and max bounds
    if (new_rate < min_rent_rate) {
        new_rate = min_rent_rate;
    }
    if (new_rate > max_rent_rate) {
        new_rate = max_rent_rate;
    }
    
    return new_rate;
}
```

**Access Control:** Only callable by the bank during epoch boundary processing

### Account Rent Paid Field

Each account gets new fields by repurposing existing unused bytes:

- **rent_paid**: `u64` field storing total rent paid in lamports (monotonically increasing)
- **last_write_slot**: `u64` field tracking when account was last written to
- **Repurposed from existing fields**:
  - `rent_epoch` field (8 bytes) - replaced by slot-based tracking with `last_write_slot`
- **No account size increase**: Fields fit within existing account metadata structure
- **rent_paid** is set to zero when account is compressed (rent is "consumed"
  by the compression operation)
- **last_write_slot** is updated on every write to the account

### Rent Collection Behavior

The dynamic rent system integrates with account compression (SIMD-0341) to
enforce rent payments at key lifecycle events. Rent calculation is slot-based
(using `last_write_slot`) while the dynamic rent rate remains epoch-based:

- **Dynamic Rent Rate**: Updated per epoch based on state size vs target
- **Rent Calculation**: Based on slots elapsed since `last_write_slot`
- **Rent Collection**: 
  - **Any write**: Pay rent accumulated since last_write_slot
  - **Account creation/decompression**: 15 epochs worth of rent upfront
  - **Account size changes**: Pay rent since last write + 15 epochs upfront
    at new size

**Slot-Based Rent Calculation Helper:**

```c
uint64_t calculate_rent_owed_since_last_write_slot(
    uint64_t last_write_slot, 
    uint64_t current_slot,
    uint64_t account_data_size,
    uint64_t *rent_history,
    uint64_t slots_per_epoch) {
    uint64_t last_write_epoch = last_write_slot / slots_per_epoch;
    uint64_t current_epoch = current_slot / slots_per_epoch;
    uint64_t rent_owed = 0;
    
    // Handle partial epoch at the start (from last_write_slot to end of
    // last_write epoch)
    if (last_write_epoch == current_epoch) {
        // Same epoch: only charge for slots between last_write_slot and
        // current_slot
        uint64_t slots_elapsed = current_slot - last_write_slot;
        rent_owed = account_data_size * rent_history[last_write_epoch] * 
                   slots_elapsed / slots_per_epoch;
    } else {
        // Different epochs: handle partial epoch at start
        uint64_t slots_remaining_in_last_write_epoch = 
            ((last_write_epoch + 1) * slots_per_epoch) - last_write_slot;
        uint64_t partial_start_rent = account_data_size * 
            rent_history[last_write_epoch] * 
            slots_remaining_in_last_write_epoch / slots_per_epoch;
        rent_owed += partial_start_rent;
        
        // Calculate rent for complete epochs
        for (uint64_t epoch = last_write_epoch + 1; 
             epoch < current_epoch; epoch++) {
            rent_owed += account_data_size * rent_history[epoch];
        }
        
        // Add rent for partial epoch at the end
        uint64_t slots_in_partial_epoch = current_slot % slots_per_epoch;
        if (slots_in_partial_epoch > 0) {
            uint64_t partial_end_rent = account_data_size * 
                rent_history[current_epoch] * 
                slots_in_partial_epoch / slots_per_epoch;
            rent_owed += partial_end_rent;
        }
    }
    
    return rent_owed;
}
```

#### 1. Account Creation

When creating a new account:

```c
// MUST pay at least 15 epochs worth of rent upfront
SLOTS_PER_EPOCH = 432000
required_rent = account_data_size * current_rent_rate * 15 * SLOTS_PER_EPOCH;
if (transaction_rent_payment < required_rent) {
    return ERROR_INSUFFICIENT_RENT;
}
account.rent_paid = transaction_rent_payment;
account.last_write_slot = current_slot;
```

#### 2. Account Rehydration (Decompression)

When decompressing an account via `sol_decompress_account()`:

```c
// MUST pay at least 15 epochs worth of rent to reactivate
required_rent = account_data_size * current_rent_rate * 15 * SLOTS_PER_EPOCH;
if (transaction_rent_payment < required_rent) {
    return ERROR_INSUFFICIENT_RENT;
}
account.rent_paid = transaction_rent_payment;
account.last_write_slot = current_slot;
```

#### 3. Any Write to Account

On any write to an account:

```c
// Calculate rent owed using slot-based calculation since last_write_slot
rent_owed = calculate_rent_owed_since_last_write_slot(
    account.last_write_slot, 
    current_slot, 
    account_data_size, 
    rent_history, 
    slots_per_epoch
);

// Payment processing
account.rent_paid += transaction_rent_payment;
account.rent_paid -= rent_owed;

// Update last_write_slot
account.last_write_slot = current_slot;
```

#### 4. Account Size Changes

When an account's data size changes:

```c
// Calculate rent owed using slot-based calculation since last_write_slot
rent_owed = calculate_rent_owed_since_last_write_slot(
    account.last_write_slot, 
    current_slot, 
    account_data_size, 
    rent_history, 
    slots_per_epoch
);

// Payment processing
account.rent_paid += transaction_rent_payment;
account.rent_paid -= rent_owed;

// MUST pay at least 15 epochs worth of rent upfront at the NEW account size
required_rent = new_account_data_size * current_rent_rate * 15;
if (account.rent_paid < required_rent) {
    return ERROR_INSUFFICIENT_RENT;
}

// Update last_write_slot and account size
account.last_write_slot = current_slot;
account.data_size = new_account_data_size;
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
    
    // Calculate rent owed since last_write_slot using slot-based calculation
    required_rent = calculate_rent_owed_since_last_write_slot(
        account.last_write_slot, 
        current_slot, 
        account.data_size, 
        rent_history, 
        slots_per_epoch
    );
    
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
    // Calculate rent requirements based on operation type
    uint64_t rent_required = 0;
    
    if (account_size_changed(pubkey)) {
        // Account size change: rent since last write + 15 epochs upfront at
    // new size
        uint64_t rent_owed = calculate_rent_owed_since_last_write_slot(
            account.last_write_slot, 
            current_slot, 
            account_data_size, 
            rent_history, 
            slots_per_epoch
        );
        uint64_t upfront_rent = new_account_data_size * current_rent_rate * 15;
        rent_required = rent_owed + upfront_rent;
    } else {
        // Regular write: just rent since last write
        rent_required = calculate_rent_owed_since_last_write_slot(
            account.last_write_slot, 
            current_slot, 
            account_data_size, 
            rent_history, 
            slots_per_epoch
        );
    }
    
    uint64_t total_cost = transaction_fee + rent_required;
    if (fee_payer_balance < total_cost) {
        show_error("Insufficient funds for transaction fee + rent collection");
        return;
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
- Voluntary compression does not solve the problem of old/abandoned accounts

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
  in the active account state through integral control of excess state accumulation

## Security Considerations

### Economic Attack Vectors

**Rent Rate Manipulation:**

- **Attack**: Coordinated creation/deletion of large accounts to manipulate
  integral controller accumulator
- **Mitigation**: Integral controller parameters tuned for stability; 15-epoch
  upfront cost makes manipulation expensive; accumulator provides natural smoothing

**State Size Attacks:**

- **Attack**: Rapidly expanding state size to force high rent rates
- **Mitigation**: High upfront costs (15 epochs) and ongoing rent collection
  make sustained attacks economically prohibitive


**Sysvar Size Growth:**

- **Risk**: Unbounded growth of historical data sysvars
- **Mitigation**: predictable growth rate (16 bytes per epoch per sysvar)

**Compression/Decompression Spam:**

- **Risk**: Excessive compression operations consuming compute resources
- **Mitigation**: blocks have max account state delta capped at 100mb

## Backwards Compatibility

### Breaking Changes

**Account Structure Changes:**

- New `rent_paid` and `last_write_slot` fields added by repurposing existing
  unused bytes
- `rent_epoch` field is removed/repurposed (it is no longer used now that all
  accounts are rent exempt)
- **No account size increase**: Fields fit within existing account metadata structure
- Existing accounts will have these fields initialized to 0 during activation
- Account serialization format changes require updated client libraries

**Transaction Validation Changes:**

- Account creation now requires 15 epochs of rent payment
- All account writes require rent payment as part of the fees paid based on
  slots elapsed since last write
- Account size changes require additional 15 epochs worth of rent upfront at
  new size
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
// Initialize new account fields for all existing accounts by repurposing
// existing bytes
for (account in all_accounts) {
    account.rent_paid = 0;  // Uses repurposed rent_epoch bytes
    account.last_write_slot = current_slot;
    // rent_epoch field is no longer accessible
}

// Activate sysvars with empty history
dynamic_rent_history = [];
epoch_state_size_history = [];

// Integral controller starts at current rent level, disabled until old state
// evicted
current_rent_rate = existing_rent_rate; // Maintain current pricing
integral_controller_enabled = false;
accumulator = 0; // Initialize excess state accumulator
```

**Phase 2: Rent Collection (Epoch N+1)**

```c
// Begin rent collection on any write
// Existing accounts get grace period - no historical rent owed initially
if (account.last_write_slot == activation_slot) {
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

**Phase 4: Integral Controller Activation (Epoch N+25)**

```c
// Enable dynamic rent adjustment only after old account state has been
// evicted
// This ensures integral controller operates on accounts that have paid proper rent
if (current_epoch >= activation_epoch + 25) {
    integral_controller_enabled = true;
    // Begin adjusting rent rates based on accumulated excess state size
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
