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
- Decay-per-slot: An integer factor with denominator `DECAY_SCALE = 2^64`, used
  to apply one slot of inflation decay.
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

Rust implementation:

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

Clients MUST perform both flooring operations in the order shown.

#### Feature Activation and Reward Boundary

The integer reward calculation applies immediately in the boundary bank. If
the feature activates on epoch E -> E+1, rewards paid for epoch E during E+1
must use fixed-point math. Rewards calculated by banks before this feature
activation must use legacy calculation.

### Inflation Decay

Floating-point exponentiation is replaced with deterministic integer decay.
Clients MUST derive the per-slot factor from the configured `ns_per_slot` and
taper fraction using the algorithm below. No per-duration decay table is
required.
This also accommodates short slot durations used by tests and local validators
without requiring additional decay constants.

The calculation uses three scales:

- `RATE_SCALE = 10^15`: Initial, terminal, anchor, and calculated validator
  rates.
- `DECAY_SCALE = 2^64`: Per-slot and accumulated decay factors.
- `WORK_SCALE = 2^88`: Intermediate logarithm and exponential approximations.

Each integer represents its value multiplied by the corresponding scale;
dividing by that scale recovers the represented value. For example, an annual
rate of 8% is stored as `80_000_000_000_000` at `RATE_SCALE`.

Annual rates retain their existing decimal representation at `RATE_SCALE`.
Decay factors remain in `[0, DECAY_SCALE]`, where `DECAY_SCALE` represents 1.0.
The finer `DECAY_SCALE` limits the error amplified by exponentiating a factor
close to 1 over many slots. `WORK_SCALE` provides additional precision during
factor derivation; the derived factor is rounded to `DECAY_SCALE` before use.

#### Derivation of the per-slot factor

The mathematical quantity approximated is:

```text
DECAY_SCALE * (1 - taper)^(ns_per_slot / YEAR_NS)
    = DECAY_SCALE * exp(-x)
x = -ln(1 - taper) * ns_per_slot / YEAR_NS
YEAR_NS = 31_556_925_993_600_000
```

`WORK_SCALE` is `2^24` times `DECAY_SCALE` to limit rounding
error during factor derivation while keeping intermediates within `u128` for
the protocol parameters. For the 15% and 30% tapers, the truncation error of the
32-term logarithm series is smaller than one working-scale unit. The exponential
uses a cubic approximation because the per-slot exponent is small; the quadratic
and cubic terms account for the leading corrections to linear decay.

Clients MUST reproduce the results of the following integer algorithm, including
its term counts, division positions, and final round-to-nearest-even operation.
Clients MAY use an equivalent implementation if it produces the same result or
failure for every input. The mathematical formula above describes the
approximation target; it does not require a correctly rounded transcendental
result for every possible input.

All values in the Rust implementation are unsigned. Division discards the
remainder.
Inputs from a `Fraction` MUST be widened before arithmetic. Clients MUST NOT use
floating-point operations to derive the factor, including as a fallback.

```rust
const WORK_SCALE: u128 = 1 << 88;
const DECAY_SCALE: u128 = 1 << 64;
const YEAR_NS: u128 = 31_556_925_993_600_000;

// Approximates WORK_SCALE * ln(numerator / denominator).
fn approximate_logarithm(numerator: u128, denominator: u128) -> Option<u128> {
    if denominator == 0 || numerator < denominator {
        return None;
    }
    // Represent z = (numerator - denominator) / (numerator + denominator)
    // as a fraction. Then (1 + z) / (1 - z) = numerator / denominator,
    // so ln(numerator / denominator) = 2 * (z + z^3/3 + z^5/5 + ...).
    let z_num = numerator.checked_sub(denominator)?;
    let z_den = numerator.checked_add(denominator)?;
    let z_num_squared = z_num.checked_mul(z_num)?;
    let z_den_squared = z_den.checked_mul(z_den)?;
    // Initially, term is z scaled by WORK_SCALE.
    let mut term = WORK_SCALE.checked_mul(z_num)? / z_den;
    let mut sum = 0u128;
    for i in 0..32 {
        sum = sum.checked_add(term / (2 * i + 1))?;
        if i < 31 {
            // Multiply by z^2 to advance to the next scaled odd power.
            term = term.checked_mul(z_num_squared)? / z_den_squared;
        }
    }
    sum.checked_mul(2)
}

// Approximates WORK_SCALE * exp(-x / WORK_SCALE), including the leading 1.
fn approximate_exponential(x: u128) -> Option<u128> {
    let second = x.checked_mul(x)? / (2 * WORK_SCALE);
    let third = second.checked_mul(x)? / (3 * WORK_SCALE);
    let decrement = x.checked_sub(second)?.checked_add(third)?;
    WORK_SCALE.checked_sub(decrement)
}

fn decay_per_slot(ns: u128, taper_num: u128, taper_den: u128) -> Option<u128> {
    if ns == 0 || taper_den == 0 || taper_num >= taper_den {
        return None;
    }
    // -ln(1 - taper) = ln(taper_den / (taper_den - taper_num)).
    let annual_exponent =
        approximate_logarithm(taper_den, taper_den.checked_sub(taper_num)?)?;
    let slot_exponent = annual_exponent.checked_mul(ns)? / YEAR_NS;
    let decay = approximate_exponential(slot_exponent)?;

    // Scale the small decrement; scaling the near-1 decay directly overflows.
    let numerator = WORK_SCALE.checked_sub(decay)?.checked_mul(DECAY_SCALE)?;
    let whole = numerator / WORK_SCALE;
    let rem = numerator % WORK_SCALE;
    let up = u128::from(
        rem > WORK_SCALE / 2 || (rem == WORK_SCALE / 2 && whole % 2 != 0),
    );
    // DECAY_SCALE is even, so complement rounding preserves ties-to-even.
    DECAY_SCALE.checked_sub(whole.checked_add(up)?)
}
```

`None` denotes invalid inputs or failure of checked arithmetic. Derivation is
expected to succeed for the protocol's chosen slot durations and tapers; checked
arithmetic exposes unsupported choices during testing. Clients MUST NOT wrap,
saturate, clamp inputs, or substitute a floating-point result.

#### Accumulating decay

For factors in `[0, DECAY_SCALE]`, scaled multiplication is
`floor(a * b / DECAY_SCALE)`. Multiplication by the identity MUST be handled
without evaluating `DECAY_SCALE * DECAY_SCALE`, which exceeds `u128`.
Products of two factors strictly below `DECAY_SCALE` fit in `u128`.

Decay over `n` slots MUST use the following right-to-left binary exponentiation.
Clients MUST preserve this association order and floor after each
multiplication.

```rust
fn mul_decay_floor(a: u128, b: u128) -> u128 {
    if a == DECAY_SCALE {
        return b;
    }
    if b == DECAY_SCALE {
        return a;
    }
    (a * b) >> 64
}

fn pow_decay_floor(mut base: u128, mut n: u64) -> u128 {
    let mut result = DECAY_SCALE;
    while n > 0 {
        if n & 1 != 0 {
            result = mul_decay_floor(result, base);
        }
        n >>= 1;
        if n > 0 {
            base = mul_decay_floor(base, base);
        }
    }
    result
}
```

For a constant slot duration, the annual validator rate, expressed in
`RATE_SCALE` units, is:

```text
factor = decay_per_slot(ns_per_slot, taper.numerator, taper.denominator)
decay_since_anchor = pow_decay_floor(factor, slots_since_anchor)
rate = max(TERMINAL_RATE,
           floor(anchor_rate * decay_since_anchor / DECAY_SCALE))
```

The rate calculation MUST use a `u128` intermediate or an exactly equivalent
implementation. `anchor_rate` is in `[0, RATE_SCALE]`, so this product fits in
`u128`; division by `DECAY_SCALE` is equivalent to shifting right by 64 bits.
Clients MUST NOT round the per-slot factor or accumulated decay to `RATE_SCALE`
before applying it to the anchor rate. Epoch reward calculations are unchanged.

Before SIMD-0550 activation, `anchor_rate = INITIAL_RATE` and the taper is 15%.
At SIMD-0550 activation, clients MUST calculate the activation anchor rate using
the preceding integer schedule, including its terminal clamp. This anchor
remains in `RATE_SCALE` units. Clients then apply the decay factor derived for
the 30% taper from the activation slot forward. At zero elapsed slots, decay is
exactly `DECAY_SCALE`, preserving the integer anchor rate without a
discontinuity.


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

The following comparison evaluates each integer schedule against the current
floating-point calculation. Floating point is a comparison baseline, not an
exact mathematical reference.

```text
year = epoch_start_slot / slots_per_year_f64
legacy_rate = max(0.015, 0.08 * (1 - taper)^year)
legacy_reward = trunc(legacy_rate * capitalization * slots_in_epoch
    / slots_per_year_f64)

decay = pow_decay_floor(decay_per_slot, slots_since_anchor)
validator_rate = max(TERMINAL_RATE,
    floor(anchor_rate * decay / DECAY_SCALE))
annual_reward = floor(capitalization * validator_rate / RATE_SCALE)
epoch_reward = floor(annual_reward * slots_in_epoch * slots_per_year_denominator
    / slots_per_year_numerator)
```

For a normalized capitalization of 1,000,000,000 SOL, 8% initial inflation,
432,000-slot epochs, and the 400ms, 350ms, 300ms, 250ms, and 200ms slot-time
regimes, the largest observed absolute epoch reward differences over 6000 epochs
(epoch indices 0 through 5999) are shown below. Capitalization, taper, and slot
duration are held constant within each schedule. Both integer approaches retain
`RATE_SCALE = 10^15` for annual rates and use the same two reward floors.

Differences are in lamports:

| Taper | Original `10^15` decay | Proposed `2^64` decay |
|-------|-----------------------|-----------------------|
| 15% | 31,831,625 | 2,062 |
| 30% | 14,181,312 | 1,061 |

The proposed calculation's maximum is 0.000002062 SOL per epoch. The observed
relative difference,
`abs(integer_epoch_reward - legacy_reward) / legacy_reward`, is below
`2.15e-11` for both tapers. These are measured results for the comparison above,
not universal error bounds.

Using `DECAY_SCALE = 2^64` reduces rounding error in the per-slot factor and
accumulated decay while retaining the existing annual-rate representation.
Deriving the factor from
`ns_per_slot` also permits other slot durations without adding decay-table
entries.

## Security Considerations

These calculations affect consensus-critical state. Implementations must use the
specified arithmetic widths, constants, and rounding rules to avoid cross-client
divergence.

Intermediate arithmetic must not overflow, and protocol constants must not be
recomputed using floating-point arithmetic at runtime.

### Sequencing with SIMD-0550

This feature must be activated before or at the same epoch boundary as
SIMD-0550.
When SIMD-0550 activates, clients compute the SIMD-0550 activation anchor rate
using pre-SIMD-0550 integer 15% taper schedule, then apply the integer 30% taper
from the SIMD-0550 activation slot forward.

## Conformance

Conformance tests MUST include:

- epoch reward rounding vectors
- terminal-rate clamping
- decay test cases with exact expected results for every protocol slot-time
  regime and taper
- SIMD-0550 activation anchoring
- replay across activation with matching bank hashes
