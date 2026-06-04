---
simd: '0553'
title: Base Inclusion and Resource-based Fee
authors:
  - cavey
category: Standard
type: Core
status: Draft
created: 2026-06-03
feature:
  - BurnUeW8qFgJm3Y4uroMvqEQDrYWbhv7p231hHbrq8LV (`resource_fee_burn_1_10`)
  - Burn2YJ9k9wTcyQCEMztMsbnBReV3FzXpW1aBB7MhwuY (`resource_fee_burn_1_4`)
  - BurnSoLN7d4ASk4zVVxTg4ob3C8z71dN6cr8YsyEnaZ6 (`resource_fee_burn_1_2`)
---

## Summary

We split the present static **5000-lamport per-signature fee** (2500 burned,
2500 to the leader) into two components:

- **Base inclusion fee** — static **2500 lamports** per transaction, paid
  entirely to the leader (see **Base Inclusion Fee**).
- **Resource fee** — dynamic, **100% burned**. Computed from
  `requested_cost_units` and the resource fee rate (see **Resource Fee
  Calculation**). The rate ramps in three feature-gated steps — **1/10**,
  **1/4**, then **1/2** lamports per cost unit — replacing the burned half of
  today's fee and scaling with scheduler cost.

**Priority fees** are unchanged (100% to the leader, as per SIMD-0096).

The mechanism is suitable for the network before and after Alpenglow activation.

Originating discussion: [Discussion #547].

## Motivation

1. **Negligible SOL burn today.** At the current ~3000 TPS the network burns on
   the order of ~648 SOL/day from signature fees alone — several orders of
   magnitude below daily inflation (~60,000 SOL/day).

2. **The current base fee does not price resources.** Users pay a single static
   per-signature charge regardless of scheduler cost (compute, write locks,
   loaded data, etc.). This leaves some users underpaying for compute-heavy
   transactions.

## Alternatives Considered

**Uniform increase to the 5000-lamport per-signature fee.** Rejected.
High-volume senders (market makers, validators pre-Alpenglow) pay mostly the
flat charge; a uniform increase disproportionately harms them while leaving
resource usage mispriced.

**50/50 burn / leader split on the resource fee.** Rejected. The resource cost
is borne by every node that replays and votes on the transaction, not only the
leader. Paying the leader a share would create a much harder economic problem
to model with second order effects on validator incentives and market structure
(e.g. favoring inefficient programs that pay more resource fee to the leader,
favoring compute-heavy swaps over cheap maker updates). Burning 100% keeps
validator incentives and economics untouched for the most part.

**Dynamic resource fee rate.** A controller that adjusts the resource fee rate
each epoch from block utilization is a reasonable follow-up but adds
predictability complexity. This SIMD specifies three static rates activated via
separate feature gates (**1/10**, **1/4**, **1/2** lamports per cost unit); a utilization
controller or further adjustments can come later.

**SIMD-0110-style hot-account base fee.** Operates locally on short timescales
from account hotness. This proposal is global, tied to per-transaction requested
cost, and intended to move slowly (staged static rates via feature gates;
dynamic tuning may follow).

## New Terminology

- **Base inclusion fee** — static **2500 lamports** per transaction to the
  block leader; renamed from "base fee". Pays for inclusion, not resource
  consumption. **Required** so the leader is paid even when the priority fee is
  zero; otherwise game-theoretic incentives would lead leaders to drop zero-prio
  txns.

- **Resource fee** — lamports burned from `requested_cost_units` and the
  resource fee rate (see **Resource Fee Calculation**).

- **Requested cost units** — the pre-execution scheduler cost used for block
  packing and cost tracking today. It is the sum of five cost-model components
  (see **Resource Fee Calculation**). For program execution and loaded-account
  data, the **requested** compute-unit limit and **requested** loaded-accounts
  data size MUST be used (not consumed values).

- **Resource fee rate** — lamports per requested cost unit, expressed as a
  rational `resource_fee_rate_numerator / resource_fee_rate_denominator`.
  Three staged values: **1/10**, **1/4**, and **1/2**. See **Feature Gates**.

## Detailed Design

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD",
"SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in [RFC
2119](https://www.ietf.org/rfc/rfc2119.txt) and [RFC
8174](https://www.ietf.org/rfc/rfc8174.txt).

### Feature Gates

Three feature gates SHALL be introduced:

<!-- markdownlint-disable MD013 -->
| Gate | Feature ID | Rate | Numerator | Denominator |
| ---- | ---------- | ---- | --------- | ----------- |
| `resource_fee_burn_1_10` | `BurnUeW8qFgJm3Y4uroMvqEQDrYWbhv7p231hHbrq8LV` | 1/10 | 1 | 10 |
| `resource_fee_burn_1_4` | `Burn2YJ9k9wTcyQCEMztMsbnBReV3FzXpW1aBB7MhwuY` | 1/4 | 1 | 4 |
| `resource_fee_burn_1_2` | `BurnSoLN7d4ASk4zVVxTg4ob3C8z71dN6cr8YsyEnaZ6` | 1/2 | 1 | 2 |
<!-- markdownlint-enable MD013 -->

When **any** of these gates is active for a bank, transaction fees MUST follow
**Total Fee Calculation** below (base inclusion fee split plus resource fee).
When **multiple** gates are active, implementations MUST use the **highest**
effective resource fee rate from the table above.

The gates SHOULD be activated in order (1/10 → 1/4 → 1/2). Activating them out
of order is permitted; the effective rate is always the maximum among active
gates. The staged rollout lets the network observe wallet, RPC, and application
behavior at modest burn before reaching the terminal **1/2** rate.

#### Activation timing

Each gate follows the standard feature activation mechanism. If a gate is
activated between epoch `E - 1` and epoch `E`, the corresponding rate change
MUST apply starting at the **first slot of epoch `E`**. All earlier slots MUST
use the previous effective fee model (pre-activation flat fee, or a lower
resource fee rate if an earlier gate is already active).

Transaction simulation and fee estimation at epoch boundaries therefore depend
on which feature set the bank uses; clients SHOULD use the same feature
activation rules as the runtime when simulating near a boundary.

### Total Fee Calculation

```text
total_fee = base_inclusion_fee + priority_fee + resource_fee
```

| Component          | Formula                          | Burn | Leader |
| ------------------ | -------------------------------- | ---- | ------ |
| Base inclusion fee | 2500                             | 0%   | 100%   |
| Priority fee       | CU price × limit (existing)      | 0%   | 100%   |
| Resource fee (new) | see **Resource Fee Calculation** | 100% | 0%     |

**Fee-only transactions.** In Agave, a transaction that fails during account
loading may still be processed as **fee-only**
(`ProcessedTransaction::FeesOnly`): the fee payer is debited the full
**`total_fee`** even though the transaction does not execute. This behavior MUST
be preserved after the first gate activates, now also including the resource
fee.

### Base Inclusion Fee (static component)

The "base fee" / "transaction fee" SHOULD be called **base inclusion fee** in
documentation, RPC fields, and user-facing surfaces.

```text
base_inclusion_fee = 2500
```

Leaders MUST receive a **nonzero reward for including a transaction even when
the priority fee is zero**. Without it, game-theoretic fee maximization would
leave zero-priority transactions unincluded. Present designs for multiple
concurrent proposers introduce an inclusion fee; this split does not introduce a
new fee category or additional complexity beyond what's to come.

**Legacy per-signature base fee.** Today's 5000-lamport charge scales with
signature count (`5000 × num_signatures`, split 50/50 burn / leader) because
the original fee model priced each signature verification at the transaction
level. The scheduler cost model also counts signatures in `signature_cost`, so
signature work is priced twice today: once in the per-signature base fee and
again (relatively cheaply) in `requested_cost_units`.

**This SIMD.** The base inclusion fee is a flat **2500 lamports** per
transaction — not `2500 × num_signatures`. Signature verification is priced
only through `signature_cost` inside `requested_cost_units` and therefore the
resource fee.

**Follow-up path.**

1. **This SIMD** — flat base inclusion fee; resource fee uses today's
   (relatively cheap) `signature_cost` component in the cost model.
2. **Follow-up SIMD** — if needed, after some investigation, raise signature
   pricing in the cost model so multi-signer and precompile-heavy transactions
   pay an appropriate resource fee after step 1. Specific constants are out of
   scope here and belong in that proposal.

Multi-signer transactions pay less fees than under today's per-signature model
until that follow-up is activated.

### Resource Fee Calculation (dynamic component)

```text
resource_fee = ceil_div(
  requested_cost_units × resource_fee_rate_numerator,
  resource_fee_rate_denominator)
```

Implementations MUST compute `resource_fee` using **integer arithmetic** only.

The effective `resource_fee_rate_numerator` and
`resource_fee_rate_denominator` MUST be taken from **Feature Gates** for the
bank being executed or verified.

`requested_cost_units` is the transaction's pre-execution scheduler cost — the
same total computed for block packing and cost tracking today. Implementations
MUST compute it as the saturating sum of:

1. **Signature cost** (`signature_cost`) — cost of signature verification
   (including precompile signatures accounted by the cost model).
2. **Write-lock cost** (`write_lock_cost`).
3. **Instruction-data cost** (`data_bytes_cost`).
4. **Program-execution cost** (`programs_execution_cost`) — cost from the
   transaction's **requested** compute-unit limit (Compute Budget).
5. **Loaded-accounts-data-size cost** (`loaded_accounts_data_size_cost`) —
   cost from the transaction's **requested** loaded-accounts data size.

```text
requested_cost_units = signature_cost
                     + write_lock_cost
                     + data_bytes_cost
                     + programs_execution_cost      // requested CUs
                     + loaded_accounts_data_size_cost  // requested bytes
```

(saturating u64 addition at each step, as in validator cost-model code today.)

The resource fee MUST use this **requested** total, not post-execution
consumed costs, for the same reasons as the priority fee:

1. **Async execution.** Future async execution paths require a fee known before
   execution completes.
2. **Pre-execution validation.** The runtime can verify the fee payer can afford
   the fee without executing the transaction.
3. **Better UX.** Users know exactly how much they're paying before sending the
   transaction.
4. **Scheduling incentives.** Users should request accurate compute limits.

Note that the sum of **requested** cost units across transactions in a block MAY
exceed per-block packing limits.

### Fee Collection and Burn

Implementations MUST:

1. Compute `base_inclusion_fee`, `resource_fee`, and `priority_fee`; burn the
   full resource fee; route inclusion and priority fees to the collector.
2. Deduct `total_fee` from the fee payer as today.

### Validator Components Affected

| Validator Component             | Impact                                |
|---------------------------------|---------------------------------------|
| Transaction Execution (Runtime) | Fee calculation, burn accounting      |
| Virtual Machine                 | None                                  |
| Block Packing                   | Fee payer check uses new total fee    |
| Consensus                       | Burn totals in bank hash              |
| Gossip                          | None                                  |
| Turbine                         | None                                  |
| TPU                             | Vote / Simple Vote fast-path updates  |
| Snapshots                       | None                                  |
| On-Chain Core BPF Programs      | None                                  |
| Other (please describe)         | RPC fee responses, wallets, explorers |

### RPC and client APIs

Agave today exposes fees almost entirely as a **single lamport total** computed
by `Bank::get_fee_for_message` / transaction simulation (`fee` crate →
`FeeDetails::total_fee()`). Under this SIMD that total becomes
`base_inclusion_fee + resource_fee + priority_fee`, but the JSON shape can
stay scalar unless an implementation chooses to extend it.

| RPC method                      | Field            | Change         |
| ------------------------------- | ---------------- | -------------- |
| `getFeeForMessage`              | `result`         | New total fee  |
| `simulateTransaction`           | `fee`            | New total fee  |
| `sendTransaction` (preflight)   | `fee` in error   | As simulation  |
| `getTransaction`                | `meta.fee`       | Total charged  |
| `getRecentPrioritizationFees`   | (priority only)  | Unchanged      |

The **Banks** service
(`get_fee_for_message_with_commitment_and_context`) follows the same bank path
as `getFeeForMessage`.

RPC implementations **SHOULD** ensure the above totals match **Total Fee
Calculation**. They **MAY** add optional breakdown fields
(`baseInclusionFee`, `resourceFee`, `priorityFee`) in a follow-up; this SIMD
does not require a schema change. User-facing docs **SHOULD** stop treating
`meta.fee` / simulation `fee` as “base fee only” and label them **total
fee**.

`getTransaction` already exposes `meta.costUnits` (requested scheduler cost);
clients **MAY** use it with the resource fee rate to estimate resource fee, but
**SHOULD** prefer `getFeeForMessage` / `simulateTransaction` for authoritative
totals.

## Impact

**SOL holders.** Empirical network data (May 2026) suggests ~1,500–1,800,
~3,750–4,500, and ~7,500–9,000 SOL/day from resource fees at **1/10**, **1/4**,
and **1/2** lamports per requested cost unit respectively, each replacing the
flat 2500-lamport-per-signature burn (~648 SOL/day at current throughput). The
terminal **1/2** rate implies a meaningful net increase in total burn and roughly
0.5% deflation against ~3.8% inflation; earlier stages burn less while the
network validates the new fee model.

**Retail / low-priority users.** Example transactions at the terminal
`resource_fee_rate` of **1/2** (vote: 3765 requested cost units). At **1/10**
or **1/4**, multiply the table's resource-fee column by **1/5** or **1/2**
respectively (base inclusion and priority fees are unchanged):

<!-- markdownlint-disable MD013 MD060 -->
| Activity | Today | After | Δ |
| -------- | ----- | ----- | -- |
| Vote (w/ CB ixns) | 5000 | 2500 + 1883 | -12.3% |
| [High Prio Swap (DFlow)][dflow-tx] | 5000 + 2002500 | 2500 + 197720 + 2002500 | +9.72% |
| [Mid Prio Swap (OKX)][okx-tx] | 5000 + 130980 | 2500 + 412160 + 130980 | +301% |
| [Zero prio Pumpfun swap][pump-tx] | 5000 + 0 | 2500 + 159910 + 0 | +3150% |
| [Zerofi oracle update][zerofi-tx] | 5000 + 2568 | 2500 + 1220 + 2568 | -16.9% |
<!-- markdownlint-enable MD013 MD060 -->

At `resource_fee_rate` **1/2**, **low-resource-usage transactions win**: they pay
less total fee than under today's flat 5000-lamport charge (see Vote and Zerofi
rows). Burn tracks requested cost, rewarding efficient programs that request
only the resources they need.

**Compute-unit estimation.** Resource fee is charged on **requested** cost
units, including the Compute Budget unit **limit**, not consumed compute.
Applications that set a loose `SetComputeUnitLimit` (or never tighten it after
simulation) pay resource fee on the full requested amount—the same basis as
priority fees today, but now significant for **zero-priority** transactions
that previously paid only the flat base fee. Over-estimating requested compute
units increases `resource_fee` directly at `resource_fee_rate`.

Note that legacy vote transactions as sent today request on the order of **54k**
cost units — far more than the work a vote actually performs. At
`resource_fee_rate` **1/2** that would raise pre-Alpenglow vote cost to nearly
**6×** today's flat 5000-lamport fee. All client implementations **SHOULD**
send votes with Compute Budget instructions that set requested compute units and
loaded-accounts data size to reflect **actual vote work** (see the Vote row
above, **3765** requested units). Omitting those instructions remains valid but
is **NOT RECOMMENDED** under this fee model. [SIMD-0458] is live on
mainnet-beta; vote transactions that include Compute Budget instructions no
longer classify as Simple Vote, so implementations **SHOULD** update any TPU
vote filter or fast-path logic that assumes the legacy Simple Vote shape.

**Alpenglow.** Pre Alpenglow, validators submit **on-chain vote transactions**
at high volume (on the order of ~1 SOL/day in vote fees today). The vote
guidance above (requested cost, Compute Budget, TPU filters) matters directly
for validator operating cost in that phase.

Post Alpenglow, consensus **votes are no longer on-chain transactions**; they
are exchanged directly between validators, and vote economics move to separate
mechanisms (e.g. the validator admission ticket). **Total Fee Calculation**
still applies to every on-chain transaction before and after Alpenglow, but the
vote-specific impact above is primarily a (pre Alpenglow, migration) concern.

**Wallets and SDKs.** MUST estimate fees using the same formula as
**Resource Fee Calculation**, with the effective numerator and denominator for
the bank epoch being simulated.

## Security Considerations

**Minimum cost decreases** from 5000 lamports once the first gate is active.
At the terminal **1/2** rate the floor is 3010 lamports for minimal transactions;
at **1/10** and **1/4** the floor is lower still. This lowers the lamport floor
compared with today's flat 5000-lamport base fee and **reduces the cost of DoS
spam** at that floor. Note that such spam would consist of transactions that are
**extremely cheap to execute**.

## Drawbacks

**Modest burn vs inflation.** Even at the terminal **1/2** lamports per cost unit,
aggregate burn increases meaningfully but remains well below inflation.
Higher resource fee rates increase burn further and hit **high-compute,
low-priority** traffic hardest (see Pump.fun row at **1/2**). The staged gates
mitigate a single-step jump but extend the period during which burn stays below
the terminal target. Perhaps the terminal rate plus 100M/200M/uncapped CU
blocks will be enough burn to counteract inflation.

## Backwards Compatibility

This is a consensus-breaking change gated behind feature activation. Nodes that
do not implement the resource fee will diverge when any gate activates. All
validator client implementations (Agave, Firedancer, Sig, etc.) MUST support
all three feature gates before mainnet-beta activation of the first gate.

**Application transactions without Compute Budget instructions.** Under the
present fee model, many transactions pay only the flat 5000-lamport charge and
never set requested compute units or loaded-accounts data size. After
activation, those transactions still perform the same on-chain work, but
`requested_cost_units` — and therefore `resource_fee` — can be much larger
because the implicit requested limits are high relative to actual consumption
(see **Resource Fee Calculation**).

If the fee payer was funded for ~5000 lamports (+ priority) but the new
`total_fee` is higher, block packing and pre-execution fee checks MAY reject
the transaction for insufficient funds even though it would have landed under
the old model. This affects **zero-priority** and **low-priority** traffic
hardest when requested cost is high. Note that
this affects applications like Helium, as well
as compute-heavy applications like zk protocols.

Wallets, SDKs, and transaction senders **SHOULD** set Compute Budget
instructions with requested limits that reflect actual transaction work before
activation, and **SHOULD** re-estimate `total_fee` under **Total Fee
Calculation** rather than assuming the legacy flat fee. Programs **SHOULD**
optimize on-chain compute so callers can request lower compute-unit limits and
pay less resource fee. (Vote transactions are covered in **Impact** above.)

## Conformance

Requested cost units are already in protocol; all verifying nodes check whether
transactions exceed each of their requested resources (whether explicitly
specified or default amount). As such, there is no need for additional conformance
testing with requested cost units. Furthermore, the map from requested cost units
to resource fee in lamports is linear in the input. As such, implementations only
need a simple test suite (to check for edge cases, overflow, ceiling division) for
the linear map from requested cost units to resource fee lamports.

[Discussion #547]: ../discussions/547
[SIMD-0458]: 0458-stop-using-static-simplevote-transaction-cost.md

<!-- markdownlint-disable MD013 -->
[okx-tx]: https://solscan.io/tx/3cE8Af9vZ7W4iyFPp6CqeMA3YqAisXsXhKyww3oZnpUAikz1j9zsJN2b6ewdPUCs9Gcn5bzCHWUvYUQQeyWL8Udh
[pump-tx]: https://solscan.io/tx/5oGd26j5bxWar5PzJRxqYroWCBsqRtLrPbGePG76FRfqTFkQU81Z56LAARWevqKam1g9M2o3HomeMfxky5qnDfY2
[dflow-tx]: https://solscan.io/tx/V7X2RhiTH4oDW7kMJErQx6rZuETzp3tYrR1scTNkikdaW4aNpLgZeQWSkAkczwNLLPpjwN5q5cb61fT2JRpSGs8
[zerofi-tx]: https://solscan.io/tx/66vmDnYS9cujQkNgGXiC6e5GqGGkhRdyzX13j23GPUTA61xG7Kj3FNFqGnrLBtMsxhyxLQe4mZyBa4j9ziE7mMAD
<!-- markdownlint-enable MD013 -->
