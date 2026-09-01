---
simd: '0607'
title: Remove Floating Point from Runtime
authors:
  - Tao Zhu (Anza)
  - Tom Pointon (Firedancer)
category: Standard
type: Core
status: Draft
created: 2026-08-21
feature: TBD
extends: '0194, 0550'
---

## Summary

Replace floating-point arithmetic in consensus-critical inflation and rent
calculations with deterministic integer arithmetic.

## Motivation

Inflation rewards and rent parameters affect capitalization, account state, and
bank hashes. These values must be computed identically by all validator clients,
without relying on IEEE-754 behavior or platform math-library functions such as
`powf`.

## Alternatives Considered

### General-purpose rational arithmetic

Use a general-purpose rational type for all fractional calculations. This
provides exact arithmetic but adds unnecessary complexity and overhead compared
with the bounded integer representation proposed here.

## New Terminology

No new terminologies, to clarify few terms used here:

- Scaled rate: A fractional rate represented as an integer with the fixed
denominator RATE_SCALE.
- Decay-per-slot: The scaled rate used to apply one slot of inflation decay.
- Anchor rate: The inflation rate from which subsequent decay is calculated.

## Detailed Design

Protocol percentages and ratios are represented as:

```text
Fraction { numerator: u64, denominator: nonzero u64 }
```

Multiplication by a fraction is defined as:

mul_fraction_floor(value, fraction) =
    floor(value * fraction.numerator / fraction.denominator)

The multiplication MUST use a widened intermediate at least 128 bits wide, or
an exactly equivalent implementation. Rounding MUST be toward zero.

Rust-like pseudocode:

```rust
struct Fraction {
    numerator: u64,
    denominator: NonZeroU64,
}

fn mul_fraction_floor(value: u64, fraction: Fraction) -> u64 {
    let product: u128 =
        u128::from(value) * u128::from(fraction.numerator);
    let quotient: u128 =
        product / u128::from(fraction.denominator.get());

    // Integer division discards any remainder; no round-to-nearest is applied.
    u64::try_from(quotient).expect("protocol constants keep the result in u64")
}
```

### Inflation Rewards

The scaled constants below are the integer protocol representation of the
existing inflation schedule. Current validator implementations may store these
values as floating point, but after activation clients MUST use the integer
representation for consensus calculations.

RATE_SCALE    = 1_000_000_000_000_000

INITIAL_RATE  = 80_000_000_000_000    // existing 0.08 initial inflation
TERMINAL_RATE = 15_000_000_000_000    // existing 0.015 terminal inflation
PICO_RATE     =    100_000_000_000    // existing 0.0001 pico inflation

TAPER_15      = Fraction { numerator: 15, denominator: 100 }
TAPER_30      = Fraction { numerator: 30, denominator: 100 }

Each protocol slot-time regime MUST define `slots_per_year` as an exact
rational value. For any slot-time regimes already active or specified by prior
features, this SIMD preserves their existing protocol values by converting the
current decimal constants into exact rational constants. For known slot-time
regimes, the value for `slots_per_year` are:

|slot-time|slots_per_year|
|---------|--------------|
|400ms | 78_892_314_984 / 1000|
|350ms | 90_162_645_696 / 1000|
|300ms | 105_189_753_312 / 1000|
|250ms | 126_227_703_974 / 1000|
|200ms | 157_784_629_968 / 1000|

Epoch inflation rewards are computed as:

annual_reward =
    floor(capitalization * validator_rate / RATE_SCALE)

epoch_reward =
    floor(annual_reward * slots_in_epoch * slots_per_year_denominator
          / slots_per_year_numerator)

This two-step flooring is normative.

#### Feature Activation and Reward Boundary

The integer reward calculation applies immediately in the boundary bank. If
the feature activates on epoch E -> E+1, rewards paid for epoch E during E+1
must use fixed-point math. Rewards calculated by banks before this feature
activation must use legacy calculation.

### Inflation Decay

Floating-point exponentiation is replaced with scaled integer decay.

For each supported taper and slot-time regime, the protocol defines a constant:

decay_per_slot =
    round_to_nearest_even((1 - taper)^(1 / slots_per_year) * RATE_SCALE)

These constants are part of the protocol specification. Validator clients MUST
NOT derive them at runtime with floating-point arithmetic. For each supported
slot-time regimes and known taper, the value of `decay_per_slot` are:

|slot-time|taper|decay_per_slot|
|---------|-----|--------------|
|400ms|15%| 999_999_997_939_990|
|350ms|15%| 999_999_998_197_492|
|300ms|15%| 999_999_998_454_993|
|250ms|15%| 999_999_998_712_494|
|200ms|15%| 999_999_998_969_995|
|400ms|30%| 999_999_995_478_965|
|350ms|30%| 999_999_996_044_094|
|300ms|30%| 999_999_996_609_224|
|250ms|30%| 999_999_997_174_353|
|200ms|30%| 999_999_997_739_482|


Scaled multiplication is:

mul_scaled_floor(a, b) = floor(a * b / RATE_SCALE)

Decay over n slots must use right-to-left binary exponentiation with
mul_scaled_floor at each multiplication:

pseudocode:

```
  result = RATE_SCALE
  while n > 0:
      if n is odd:
          result = mul_scaled_floor(result, base)
      n = floor(n / 2)
      if n > 0:
          base = mul_scaled_floor(base, base)
```

Because `mul_scaled_floor` floors after each multiplication, clients must use
this association order exactly.

The annual validator rate is:

rate = max(TERMINAL_RATE,
           floor(anchor_rate * decay_since_anchor / RATE_SCALE))

Before SIMD-0550 activation, anchor_rate = INITIAL_RATE and the taper is
15%. After SIMD-0550 activation, clients compute the activation anchor_rate
using the pre-activation integer schedule, then apply the 30% taper from that
activation slot forward. No floating-point re-anchoring of initial is used.

### Rent

SIMD-0194 has already deprecated the rent exemption threshold on mainnet-beta
since epoch 943.

Client implementations that need to replay the SIMD-0194 activation boundary
must use an integer-equivalent conversion for the historical migration:

if exemption_threshold == bytes(1.0):
    lamports_per_byte = lamports_per_byte

if exemption_threshold == bytes(2.0):
    lamports_per_byte = checked_mul(lamports_per_byte, 2)

## Backwards Compatibility

This change is consensus-affecting and requires feature activation at an epoch
boundary.

## Impact

No transaction or program interfaces are changed.

Validator implementations must use the specified arithmetic and rounding rules
after feature activation. Differences from the previous floating-point
calculation may occur at rounding boundaries.

### Expected epoch reward drift

Using the following method to calculate legacy f64 epoch reward and fixed-point
reward:

- f64 path:

```
year = epoch_start_slot / slots_per_year_f64
validator_rate = max(0.015, 0.08 * (1 - taper)^year)

epoch_reward = trunc(validator_rate * capitalization * slots_in_epoch /
                     slots_per_year_f64)
```

- fixed-point path:

```
slots_since_anchor = epoch_start_slot - anchor_slot
decay = pow_scaled_floor(decay_per_slot, slots_since_anchor)
validator_rate = max(TERMINAL_RATE, floor(INITIAL_RATE * decay / RATE_SCALE))
annual_reward = floor(capitalization * validator_rate / RATE_SCALE)
epoch_reward = floor(annual_reward * slots_in_epoch * slots_per_year_denominator
                     / slots_per_year_numerator)
```

For a normalized capitalization of 1,000,000,000 SOL, 432,000-slot epochs, and
the current 400ms, 350ms, 300ms, 250ms, and 200ms slot-time regimes, the largest
observed validator epoch reward difference over 6000 epochs is:

- 15% taper: 31,831,625 lamports, or 0.031831625 SOL per epoch
- 30% taper: 14,181,312 lamports, or 0.014181312 SOL per epoch

The observed relative difference:
`abs(fixed_point_epoch_reward - f64_epoch_reward) / f64_epoch_reward`, is below
2.1e-7.

## Security Considerations

These calculations affect consensus-critical state. Implementations must use the
specified arithmetic widths, constants, and rounding rules to avoid cross-client
divergence.

Intermediate arithmetic must not overflow, and protocol constants must not be
recomputed using floating-point arithmetic at runtime.

### Sequencing with SIMD-0550

This feature must be activated before or at the same epoch boundary as SIMD-0550.
When SIMD-0550 activates, clients compute the SIMD-0550 activation anchor rate
using pre-SIMD-0550 integer 15% taper schedule, then apply the integer 30% taper
from the SIMD-0550 activation slot forward.

## Conformance

Conformance tests MUST include:

- epoch reward rounding vectors
- terminal-rate clamping
- decay vectors for every protocol slot-time regime
- SIMD-0550 activation anchoring
- replay across activation with matching bank hashes
