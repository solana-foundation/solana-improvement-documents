---
simd: '0496'
title: Integer Inflation Rewards
authors:
  - ksn6 (Anza)
category: Standard
type: Core
status: Idea
created: 2026-03-18
feature: (to be assigned upon acceptance)
---

## Summary

Replace the `f64` arithmetic in the inflation reward pipeline with fixed-point
`u128` integer math gated behind a feature flag. The integer path is
deterministic across CPU architectures and agrees with the existing `f64` path
to within 1 lamport.

## Motivation

Inflation rewards are computed once per epoch in
`calculate_epoch_inflation_rewards`. The current path uses `f64` for the
inflation rate, taper decay, epoch duration, and the final lamport payout.

IEEE-754 does not guarantee identical results across architectures.
Differences in FMA availability, rounding-mode defaults, and
extended-precision intermediates can cause the same source to produce different
lamport totals on different hardware. This is a consensus-splitting bug that
is currently masked by a homogeneous validator fleet. As the ecosystem moves
toward multiple client implementations and diverse hardware, the risk becomes
concrete.

SIMD-0391 addresses the same class of problem for the stake program's
warmup/cooldown logic. This proposal is independent but shares the same goal:
removing `f64` from consensus-critical paths.

## New Terminology

- **Scaled integer**: A `u128` representing a real number multiplied by a
  fixed scale factor `S`. With `S = 2^60`, the value `0.08` is stored as
  `round(0.08 * 2^60) = 92233720368547758`.

## Detailed Design

### Overview

The reward pipeline has four stages:

1. **Total inflation rate** — compute the tapered annual rate for the current
   slot.
2. **Decay** — evaluate `(1 - taper) ^ year` for a fractional year.
3. **Validator/foundation split** — subtract the foundation share from the
   total rate.
4. **Final lamport reward** — multiply the validator rate by capitalization
   and epoch duration.

This proposal replaces every `f64` operation in that pipeline with exact `u128`
fixed-point arithmetic. For each stage below, we show the current `f64` logic
and the new integer logic side by side.

### Fixed-point scale

A power-of-two scale reduces divisions to bit shifts. `S = 2^60` gives 60
bits of fractional precision — well beyond `f64`'s 53-bit mantissa — while
leaving 68 bits of headroom for intermediate products.

Implementations MAY use a different scale provided the final lamport result is
bit-for-bit identical to the arithmetic specified below.

### Constants

```text
S              = 1 << 60
NS_PER_SLOT    = DEFAULT_MS_PER_SLOT * 1_000_000
NANOS_PER_YEAR = 31_556_925_993_600_000          (365.242199 days)
```

### Converting f64 parameters to scaled integers

The `Inflation` struct stores its fields as `f64`. These are set at genesis
and never change. The naive conversion `(v * S as f64) as u128` silently
loses precision because `S` exceeds `2^53`. Implementations MUST extract the
mantissa and exponent directly from the IEEE-754 bit representation.

Given a non-negative, finite `f64` value `v`:

1. Extract the 53-bit mantissa (with implicit leading 1) and biased exponent
   from the IEEE-754 bits.
2. Compute `shift = biased_exp - 1023 - 52 + 60`.
3. If `shift >= 0`, left-shift the mantissa. Saturate to `u128::MAX` if the
   shift exceeds 75.
4. If `shift < 0`, right-shift with round-to-nearest. Return 0 if the shift
   exceeds 127.

A sample implementation is provided in the [Sample Implementations](#sample-implementations) section.

For mainnet's `Inflation::default()`:

| Field | f64 | Scaled u128 |
|---|---|---|
| `initial` | 0.08 | 92233720368547758 |
| `terminal` | 0.015 | 17293822569102705 |
| `1 - taper` | 0.85 | 980881958878066688 |
| `foundation` | 0.05 | 57646075230342349 |

### Reward calculation

#### Total inflation rate

**Current (f64):**

```text
year  = num_slots / slots_per_year
total = max(terminal, initial * (1 - taper).powf(year))
```

**Proposed (integer):**

```text
year_nanos = num_slots * NS_PER_SLOT
tapered    = initial_scaled * compute_decay(decay_base_scaled, year_nanos) / S
total      = max(tapered, terminal_scaled)
```

#### Decay: `(1 - taper) ^ year`

**Current (f64):**

```text
(1.0 - taper).powf(year)    // single f64 call
```

**Proposed (integer):**

Decompose into integer and fractional year parts:

```text
full_years = year_nanos / NANOS_PER_YEAR
remainder  = year_nanos % NANOS_PER_YEAR

int_part  = fixed_pow(decay_base_scaled, full_years)
frac_part = fixed_exp(remainder * fixed_ln(decay_base_scaled) / NANOS_PER_YEAR)

decay = int_part * frac_part / S
```

If `remainder == 0`, skip the fractional part.

`fixed_pow` computes integer exponentiation by repeated squaring in
fixed-point.

`fixed_ln` computes the natural log of a scaled value in `[S/2, S]` (real
values in [0.5, 1.0]) via the `atanh` series:

```text
ln(x) = 2 * atanh(z),  where z = (x - 1) / (x + 1)
      = 2 * (z + z^3/3 + z^5/5 + ...)
```

**Convergence.** With `x` in `[0.5, 1.0]`, `z` falls in `[-1/3, 0]`. The
k-th term satisfies `|term_k| <= (1/3)^(2k+1) / (2k+1)`. At `k = 19`:
`(1/3)^39 < 2^{-60}`, so the term underflows `S` and the series has
converged. Implementations MUST iterate until the term reaches zero or for at
least 25 iterations.

`fixed_exp` computes the exponential via Taylor series:

```text
exp(x) = 1 + x + x^2/2! + x^3/3! + ...
       = sum of term_k, where term_k = term_{k-1} * x / (k * S)
```

**Convergence.** `|term_k| <= 0.163^k / k!`. At `k = 12`:
`0.163^12 / 12! < 2^{-60}`, underflowing `S`. Implementations MUST iterate
until the term reaches zero or for at least 35 iterations.

Sample implementations of `fixed_pow`, `fixed_ln`, and `fixed_exp` are
provided in the [Sample Implementations](#sample-implementations) section.

#### Validator and foundation rates

**Current (f64):**

```text
foundation_rate = if year < foundation_term { total * foundation } else { 0.0 }
validator_rate  = total - foundation_rate
```

**Proposed (integer):**

```text
foundation_share = if year_nanos < foundation_term_nanos {
    total * foundation_scaled / S
} else { 0 }

validator_rate = total - foundation_share
```

This preserves the existing semantics of the foundation term.

#### Final reward in lamports

**Current (f64):**

```text
epoch_duration_in_years = slots_in_epoch / slots_per_year
validator_rewards = (validator_rate * capitalization * epoch_duration_in_years) as u64
```

**Proposed (integer):**

```text
epoch_nanos = slots_in_epoch * NS_PER_SLOT
rate_cap    = muldiv(validator_scaled, capitalization, S)
result      = muldiv(rate_cap, epoch_nanos, NANOS_PER_YEAR)
```

`muldiv(a, b, d)` computes `floor(a * b / d)` exactly, even when `a * b`
overflows `u128`. One approach uses the identity:

```text
a * b / d = (a / d) * b + (a % d) * b / d
```

applied recursively when `a.checked_mul(b)` overflows. Any algorithm that
yields `floor(a * b / d)` exactly is acceptable. A sample implementation is
provided in the [Sample Implementations](#sample-implementations) section.

### Feature activation

Gated behind the `integer_inflation_rewards` feature flag, activated through
the standard feature activation program as a standalone activation. When
inactive, the existing `f64` path MUST be used. When active, the integer path
MUST be used.

All implementations MUST produce bit-for-bit identical
`validator_rewards_lamports` for the same inputs. The arithmetic above fully
determines the result — there is no tolerance band.

### Sample Implementations

The following Rust implementations are provided as reference. Any
implementation that produces bit-for-bit identical results for the arithmetic
specified above is acceptable.

#### `f64_to_scaled`

```rust
fn f64_to_scaled(v: f64) -> u128 {
    assert!(v >= 0.0 && v.is_finite());
    if v == 0.0 {
        return 0;
    }
    let bits = v.to_bits();
    let mantissa = (bits & 0x000F_FFFF_FFFF_FFFF)
                 | 0x0010_0000_0000_0000; // 53 bits with implicit 1
    let biased_exp = ((bits >> 52) & 0x7FF) as i32;
    // v = mantissa * 2^(biased_exp - 1023 - 52)
    // v * S = mantissa * 2^(biased_exp - 1023 - 52 + 60)
    let shift = biased_exp - 1023 - 52 + 60;

    match shift >= 0 {
        true => {
            let shift = shift as u32;
            if shift > 128 - 53 {
                return u128::MAX;
            }
            (mantissa as u128) << shift
        }
        false => {
            let right = (-shift) as u32;
            if right >= 128 {
                return 0;
            }
            // Round to nearest.
            ((mantissa as u128) + (1u128 << (right - 1))) >> right
        }
    }
}
```

#### `fixed_pow`

Repeated squaring in fixed-point:

```rust
fn fixed_pow(base_scaled: u128, exp: u128) -> u128 {
    if exp == 0 { return S; }
    let mut result = S;
    let mut base = base_scaled;
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            result = result * base / S;
        }
        e >>= 1;
        if e > 0 {
            base = base * base / S;
        }
    }
    result
}
```

Since `base_scaled <= S`, each product fits in `u128`.

#### `fixed_ln`

Natural log via `atanh` series for `x` in `[S/2, S]`:

```rust
fn fixed_ln(x_scaled: u128) -> i128 {
    let x = x_scaled as i128;
    let s = S as i128;
    let z = (x - s) * s / (x + s);
    let z_sq = z * z / s;

    let mut sum = z;
    let mut power = z;
    for k in 1..=25u64 {
        power = power * z_sq / s;
        let term = power / (2 * k + 1) as i128;
        if term == 0 { break; }
        sum += term;
    }
    2 * sum
}
```

#### `fixed_exp`

Taylor series for a signed scaled input:

```rust
fn fixed_exp(x_scaled: i128) -> u128 {
    let s = S as i128;
    let mut sum = s;
    let mut term = s;
    for k in 1..=35u64 {
        term = term * x_scaled / (k as i128 * s);
        if term == 0 { break; }
        sum += term;
    }
    sum.max(0) as u128
}
```

#### `muldiv`

Computes `floor(a * b / d)` exactly even when `a * b` overflows `u128`:

```rust
fn muldiv(a: u128, b: u128, d: u128) -> u128 {
    match a.checked_mul(b) {
        Some(product) => product / d,
        None => {
            // a * b / d = (a / d) * b + (a % d) * b / d
            let quotient = a / d;
            let remainder = a % d;
            quotient * b + muldiv(remainder, b, d)
        }
    }
}
```

## Alternatives Considered

- **Step-wise taper by epoch.** Drop fractional exponents entirely and apply
  the taper only at integer year boundaries. Removes the need for `ln`/`exp`
  but introduces a discontinuous rate drop at each year boundary, changing
  existing economics.

- **External fixed-point crates** (`fixed`, `rust_decimal`, etc.). General-
  purpose libraries that would need auditing for our input ranges, tracking
  across version updates, and add a third-party dependency to consensus. The
  math here is ~200 LOC including tests.

## Impact

Validators must upgrade before activation. No configuration changes needed.

The integer path agrees with the `f64` path to within 1 lamport across 1M
fuzzed time points and 100k epochs at mainnet parameters. Cumulative drift
over the full inflation schedule is negligible.

No changes to public interfaces.

## Security Considerations

The core property is determinism: `u128` integer operations are identical on
all platforms by construction. Implementations MUST include differential fuzz
tests comparing the integer path against an `f64` oracle across a wide range
of time points and inflation parameters.

## Backwards Compatibility

Not backwards compatible. The integer path produces lamport values that differ
by at most 1 lamport from the `f64` path for most epochs. All validators must
upgrade before the feature activates.
