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

Use a general-purpose rational type for all fractional calculations. This provides
exact arithmetic but adds unnecessary complexity and overhead compared with the
bounded integer representation proposed here.

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
current decimal constants into exact rational constants. For example, the
current 400ms and 350ms regimes are represented as:

78_892_314.984 == 78_892_314_984 / 1000
90_162_645.696 == 90_162_645_696 / 1000

Epoch inflation rewards are computed as:

annual_reward =
    floor(capitalization * validator_rate / RATE_SCALE)

epoch_reward =
    floor(annual_reward * slots_in_epoch * slots_per_year_denominator
          / slots_per_year_numerator)

This two-step flooring is normative.

### Inflation Decay

Floating-point exponentiation is replaced with scaled integer decay.

For each supported taper and slot-time regime, the protocol defines a constant:

decay_per_slot =
    round_to_nearest_even((1 - taper)^(1 / slots_per_year) * RATE_SCALE)

These constants are part of the protocol specification. Validator clients MUST
NOT derive them at runtime with floating-point arithmetic.

Scaled multiplication is:

mul_scaled_floor(a, b) = floor(a * b / RATE_SCALE)

Decay over n slots is computed with deterministic exponentiation by squaring
using mul_scaled_floor.

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

CLient implementations that need to replay the SIMD-0194 activation boundary
must use an integer-equilalent conversion for the historical migration:

if exemption_threshold == bytes(1.0):
    lamports_per_byte = lamports_per_byte

if exemption_threshold == bytes(2.0):
    lamports_per_byte = checked_mul(lamports_per_byte, 2)

## Backwards Compatibility

This change is consensus-affecting and requires feature activation at an epoch
boundary.

## Impact

Validator implementations must use the specified arithmetic and rounding rules
after feature activation. Differences from the previous floating-point calculation
may occur at rounding boundaries.

No transaction or program interfaces are changed.

## Security Considerations

These calculations affect consensus-critical state. Implementations must use the
specified arithmetic widths, constants, and rounding rules to avoid cross-client
divergence.

Intermediate arithmetic must not overflow, and protocol constants must not be recomputed
using floating-point arithmetic at runtime.

## Conformance

Conformance tests MUST include:

- epoch reward rounding vectors
- terminal-rate clamping
- decay vectors for every protocol slot-time regime
- SIMD-0550 activation anchoring
- replay across activation with matching bank hashes
