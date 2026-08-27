---
simd: '0613'
title: CU-Scaled Base Fee
authors:
  - David Rhodus (@davidrhodus)
category: Standard
type: Core
status: Idea
created: 2026-08-27
feature: cu_scaled_base_fee
---

## Summary

Replace the per-signature base fee with a base fee that is a linear function of
the transaction's requested compute-unit limit.

When `cu_scaled_base_fee` is active:

```text
base_fee = ceil_div(requested_compute_units, 10)   # 1/10 lamport per CU
```

`base_fee` occupies the existing `transaction_fee` slot in `FeeDetails`.
Distribution is unchanged: `DEFAULT_BURN_PERCENT` (50) of `transaction_fee` is
burned; the remainder plus 100% of `prioritization_fee` is deposited to the
collector ([SIMD-0096]).

When the gate is inactive, `calculate_signature_fee` is unchanged.

## Motivation

`solana_fee::calculate_signature_fee` charges

```text
(num_transaction_signatures
   + num_ed25519_signatures
   + num_secp256k1_signatures
   + num_secp256r1_signatures) × lamports_per_signature
```

with `lamports_per_signature = 5_000`. The result does not depend on
`ComputeBudgetLimits.compute_unit_limit`.

Let `S = 5_000` and `L` be the requested compute-unit limit. Per-CU base fee is
`S / L` for a one-signature transaction with no precompiles:

| `L` (CU)  | base fee (lamports) | lamports / CU     |
| --------- | ------------------- | ----------------- |
| 5_000     | 5_000               | 1.0               |
| 50_000    | 5_000               | 0.1               |
| 200_000   | 5_000               | 0.025             |
| 1_400_000 | 5_000               | ~0.00357          |

`1_400_000 / 5_000 = 280`, so a transaction at `MAX_COMPUTE_UNIT_LIMIT`
pays 280× less base fee per requested CU than a 5_000-CU transaction.

The scheduler already packs on requested CU. Pricing the same quantity
removes that per-CU inversion. At rate `1/10` lamport/CU, `base_fee = L / 10`
(ceiling), so the 50_000-CU row is the break-even with today's 5_000-lamport
fee.

## New Terminology

**requested_compute_units** — the `u32` `compute_unit_limit` already produced
by Compute Budget instruction processing for the message
(`ComputeBudgetLimits.compute_unit_limit`). Not consumed CU.

## Detailed Design

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD",
"SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in [RFC 2119] and [RFC 8174].

### Feature gate

One feature gate: `cu_scaled_base_fee`.

Activation follows the standard feature-gate protocol. If the gate activates
between epoch `E-1` and epoch `E`, the new formula MUST apply at the first slot
of epoch `E` and all later slots of that bank's lineage. Earlier slots MUST use
`calculate_signature_fee`.

### Inactive gate (current behavior)

Implementations MUST keep the existing formula:

```text
signature_count = num_transaction_signatures
                + num_ed25519_signatures
                + num_secp256k1_signatures
                + num_secp256r1_signatures     # saturating u64 adds

transaction_fee = signature_count × lamports_per_signature
```

`lamports_per_signature` remains `5_000`. `num_*_signatures` are the counts
already used by `solana_fee::SignatureCounts`.

### Active gate

Implementations MUST compute:

```text
transaction_fee = ceil_div(requested_compute_units as u64, 10)
prioritization_fee = existing Compute Budget prioritization fee
total_fee = transaction_fee + prioritization_fee
```

`ceil_div(n, d)` is unsigned integer ceiling division, `(n + d - 1) / d` with
`d = 10`. Implementations MUST NOT use floating-point.

`signature_count` and `lamports_per_signature` MUST NOT enter
`transaction_fee` while the gate is active. Precompile signature counts
(`ed25519`, `secp256k1`, `secp256r1`) likewise MUST NOT enter
`transaction_fee`. Signature verification remains in the scheduler cost model
for packing; it is not a separate lamport charge.

`prioritization_fee` is unchanged: `compute_unit_price × requested_compute_units`
as today.

### `requested_compute_units`

Implementations MUST use the same `compute_unit_limit` already computed for
the message by Compute Budget processing. They MUST NOT re-derive a second
limit for fees.

Concretely, that value is:

1. The `u32` payload of `ComputeBudgetInstruction::SetComputeUnitLimit`, if
   present, then `min(value, MAX_COMPUTE_UNIT_LIMIT)`.
2. Otherwise
   `min(num_non_compute_budget_instructions × DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT,
        MAX_COMPUTE_UNIT_LIMIT)`.

Protocol constants (Agave `program-runtime`):

```text
DEFAULT_INSTRUCTION_COMPUTE_UNIT_LIMIT = 200_000   # u32
MAX_COMPUTE_UNIT_LIMIT                 = 1_400_000 # u32
```

Consumed compute units MUST NOT be used. The fee is a function of the
pre-execution reservation, matching prioritization fee and block packing.

`requested_compute_units = 0` is valid and MUST yield `transaction_fee = 0`.

`requested_compute_units = MAX_COMPUTE_UNIT_LIMIT` MUST yield
`transaction_fee = 140_000`. Maximum `transaction_fee` under this SIMD is
therefore 140_000 lamports; it fits in `u32` and no saturating multiply is
required for the rate itself.

### Distribution

`transaction_fee` and `prioritization_fee` MUST continue to populate
`FeeDetails` as they do today. Collector distribution MUST remain:

```text
burn    = transaction_fee * DEFAULT_BURN_PERCENT / 100    # 50
deposit = prioritization_fee + (transaction_fee - burn)
```

(`runtime/src/bank/fee_distribution.rs`, `calculate_reward_and_burn_fee_details`.)

Integer division on `burn` truncates. For odd `transaction_fee`, the extra
lamport is deposited, not burned. Implementations MUST NOT introduce a
separate 50/50 on `ceil_div` that disagrees with `DEFAULT_BURN_PERCENT`.

This SIMD does not change `DEFAULT_BURN_PERCENT`, SIMD-0096, or SIMD-0232
(custom collector).

### Fee-only transactions

A transaction processed as fee-only (`ProcessedTransaction::FeesOnly` /
account-loading failure) MUST debit `total_fee` including the CU-scaled
`transaction_fee`.

### Votes

On-chain vote transactions use the same formula. A vote that omits
`SetComputeUnitLimit` is charged on the default limit from Compute Budget
processing. Post-Alpenglow consensus votes are not on-chain transactions and
are out of scope.

### Runtime integration

The substitution is local to fee calculation:

| Site | Inactive | Active |
| ---- | -------- | ------ |
| `solana_fee::calculate_fee_details` | `calculate_signature_fee(...)` as `transaction_fee` | `ceil_div(compute_unit_limit, 10)` as `transaction_fee` |
| `Bank::get_fee_for_message` / SVM fee payer debit | unchanged call shape | same, new `transaction_fee` |
| Cost model / packing | unchanged | unchanged |
| VM | unchanged | unchanged |

`FeeFeatures` SHOULD gain a boolean for `cu_scaled_base_fee` so
`calculate_fee_details` can branch without reading the full `FeatureSet`.

### RPC

No new fields. When the bank has the gate active:

| Method | Field | Value |
| ------ | ----- | ----- |
| `getFeeForMessage` | `result` | `total_fee` |
| `simulateTransaction` | `fee` | `total_fee` |
| `getTransaction` | `meta.fee` | `total_fee` |
| `getRecentPrioritizationFees` | | unchanged |

Clients MUST estimate `transaction_fee` as `ceil_div(requested_compute_units, 10)`
rather than `signature_count × 5_000` when simulating against an active bank.

### Edge cases

| Case | Requirement |
| ---- | ----------- |
| `requested_compute_units = 0` | `transaction_fee = 0` |
| `requested_compute_units = 1` | `transaction_fee = 1` |
| `requested_compute_units = 9` | `transaction_fee = 1` |
| `requested_compute_units = 10` | `transaction_fee = 1` |
| `requested_compute_units = 11` | `transaction_fee = 2` |
| `requested_compute_units = 1_400_000` | `transaction_fee = 140_000` |
| `transaction_fee` odd | `burn = floor(transaction_fee / 2)`, remainder to collector |
| 0 transaction signatures (invalid on mainnet for a payable tx) | formula still uses CU, not signature count |
| `N` precompile signatures | no effect on `transaction_fee` |
| Duplicate / missing Compute Budget ix | same sanitization as today; fee uses the limits object that execution uses |
| Feature inactive | bit-identical to current `calculate_signature_fee` |

### Validator Components Affected

| Validator Component             | Impact |
| ------------------------------- | ------ |
| Transaction Execution (Runtime) | `calculate_fee_details` / fee-payer debit |
| Virtual Machine                 | None |
| Block Packing                   | Fee-payer lamport check uses new `total_fee` |
| Consensus                       | Burn totals that enter capitalization / bank hash |
| Gossip                          | None |
| Turbine                         | None |
| Snapshots                       | None |
| On-Chain Core BPF Programs      | None |
| Other                           | RPC fee totals; wallet/SDK fee estimation |

## Alternatives Considered

**Leave `calculate_signature_fee` unchanged.** Rejected. Base fee would remain
independent of the quantity the scheduler reserves.

**Scale by `requested_cost_units` (cost model: signature cost, write-lock cost,
instruction-data cost, programs-execution cost, loaded-accounts-data-size cost).**
That meter is a superset of requested CU. It prices additional packing
dimensions. This SIMD prices the `u32` already on the prioritization-fee path
only. Extending the meter is a separate change.

**[SIMD-0553] inclusion fee plus 100% burned resource fee.** 0553 replaces
`transaction_fee` with a constant inclusion fee (100% collector) and a
`requested_cost_units` resource fee (100% burn), with three rate gates
(`1/10`, `1/4`, `1/2`). This SIMD keeps `FeeDetails` layout and
`DEFAULT_BURN_PERCENT`, changes only how `transaction_fee` is computed, and
uses a single rate. It does not supersede 0553.

**Charge consumed CU.** Rejected. `transaction_fee` MUST be known before
execution (fee-payer pre-check, fee-only transactions, simulation).
`prioritization_fee` already uses requested CU.

**Dynamic rate (utilization controller, epoch-average priority fee,
stake-weighted parameter).** Out of scope. Rate is the integer constant `1/10`.

**Minimum `transaction_fee` floor.** Out of scope. `ceil_div(1, 10) = 1`, so
the floor is 1 lamport at `L ≥ 1`.

## Impact

**`SetComputeUnitLimit` present.** `transaction_fee = ceil_div(L, 10)`.
`L = 5_000` → 500 lamports. `L = 200_000` → 20_000. `L = 1_400_000` → 140_000.

**`SetComputeUnitLimit` absent.** Default `L = min(n * 200_000, 1_400_000)`
for `n` non-Compute-Budget instructions. A one-instruction message pays
20_000 lamports (`200_000 / 10`) instead of `5_000 × signature_count`.

**Multi-signature and precompile-heavy messages.** `transaction_fee` no longer
scales with those counts. A 2-sig, `L = 5_000` message pays 500 lamports
instead of 10_000.

**Collector income from `transaction_fee`.** Moves from
`floor(signature_count × 5_000 × 50 / 100)` to
`floor(ceil_div(L, 10) × 50 / 100)` per transaction. `prioritization_fee`
is unchanged.

**Fee-payer funding.** A fee payer loaded for a 5_000-lamport base fee with
`L > 50_000` MAY fail the pre-execution lamport check after activation.

## Security Considerations

The 1-signature `transaction_fee` floor drops from 5_000 lamports to
`ceil_div(L, 10)`. At `L = 1` the fee is 1 lamport. That reduces the lamport
cost of packing a transaction that requests almost no CU. Block CU limits and
account locks still bound how many such transactions fit. This SIMD does not
add a minimum fee.

## Drawbacks

Default Compute Budget limits (`n × 200_000`) become the dominant base-fee
input for messages that omit `SetComputeUnitLimit`. Those messages see a step
up from 5_000 lamports per signature to 20_000 lamports per non-CB
instruction (capped at 140_000).

CU is not a cycle-accurate model of replay time. This SIMD equalizes base fee
per reserved CU; it does not claim CU is a precise physical-resource unit.

## Backwards Compatibility

Consensus-breaking. Nodes that do not implement the gate will diverge on
activation. All validator clients MUST implement it before mainnet-beta
activation.

`getFeeForMessage` / `simulateTransaction` results change for active banks.
No RPC schema change.

## Conformance

Required test vectors (gate active, `prioritization_fee = 0` unless noted):

```text
L=0         => transaction_fee=0,     burn=0,      deposit=0
L=1         => transaction_fee=1,     burn=0,      deposit=1
L=9         => transaction_fee=1,     burn=0,      deposit=1
L=10        => transaction_fee=1,     burn=0,      deposit=1
L=11        => transaction_fee=2,     burn=1,      deposit=1
L=50_000    => transaction_fee=5_000, burn=2_500,  deposit=2_500
L=200_000   => transaction_fee=20_000,burn=10_000, deposit=10_000
L=1_400_000 => transaction_fee=140_000,burn=70_000, deposit=70_000
L=200_000, prioritization_fee=1_000
            => total_fee=21_000, burn=10_000, deposit=11_000
```

Gate inactive: `transaction_fee` MUST equal current
`calculate_signature_fee` for the same `SignatureCounts`, including
precompile signatures.

A localnet ledger with pre-activation fees, feature activation, and
post-activation fees is sufficient for cross-client checks.

## Explicitly out of scope

- Any rate other than `1/10` lamport per requested CU
- A process for changing the rate after activation
- Changing `DEFAULT_BURN_PERCENT`
- Inclusion-fee / resource-fee decomposition
- 100% burn of any fee component
- Pricing write locks, loaded-accounts data size, or other non-CU cost-model
  terms
- Inflation / emission
- A minimum `transaction_fee`

[SIMD-0096]: ./0096-reward-collected-priority-fee-in-entirety.md
[SIMD-0553]: ./0553-resource-fee-burn.md
[RFC 2119]: https://www.ietf.org/rfc/rfc2119.txt
[RFC 8174]: https://www.ietf.org/rfc/rfc8174.txt
