---
simd: '0529'
title: Big Integer ModExp Syscall
authors:
  - SK, ZZ
category: Standard
type: Core
status: Idea
created: 2026-05-01
feature: expH2ppKPW2ANEdEmAjfhSEcnBQJfmoX4FjuNpe9ttg
---

## Summary

Add a `sol_big_mod_exp` syscall that computes modular exponentiation over
unsigned big integers:

```text
result = (base ^ exponent) mod modulus
```

The syscall is analogous to Ethereum's ModExp precompile specified by
[EIP-198], but exposes a Solana-native syscall interface instead of the EVM
precompile ABI. Unlike EIP-198, this syscall rejects even moduli, so it is not
input-domain compatible with every EIP-198 ModExp call. Inputs and output are
byte slices encoded as little-endian unsigned integers, with the output written
into caller-provided VM memory.

## Motivation

Modular exponentiation is a foundation for RSA verification, accumulators,
some verifiable delay functions, and other number-theoretic cryptography.
These operations are prohibitively expensive when implemented directly in
sBPF, especially for common RSA modulus sizes such as 2048, 3072, or 4096 bits.
The same primitive can also provide a native modular reduction operation by
using exponent `1`, which is useful for programs that need large integer
reduction without general exponentiation.

Ethereum exposes the same arithmetic operation through its ModExp precompile.
Adding a Solana syscall provides similar cryptographic building blocks to
on-chain programs while preserving Solana's program-facing syscall model and
compute metering.

This syscall is also useful for interoperability. Programs that verify
Ethereum-oriented proofs, signatures, or attestations can reuse the same
high-level arithmetic assumptions while adapting only the call interface.

## Alternatives Considered

### Exact EIP-198 ABI

The syscall could accept one packed input buffer using the exact EIP-198
format:

```text
<length_of_BASE> <length_of_EXPONENT> <length_of_MODULUS>
<BASE> <EXPONENT> <MODULUS>
```

This maximizes byte-level compatibility with Ethereum tooling, but it is a poor
fit for Solana syscalls. EIP-198 uses 32-byte length prefixes, treats calldata
as infinitely right-padded with zero bytes, and ignores excess bytes. Solana
syscalls should instead use explicit VM memory ranges and fail deterministically
on invalid memory accesses.

### Precompile Or Native Program

A transaction precompile or native program would follow the pattern used for
some signature verification features. A syscall is preferred because programs
can invoke it directly without instruction introspection, and because the
operation is general arithmetic rather than transaction signature validation.

### On-chain sBPF Implementation

Programs can implement modular exponentiation in sBPF today, but the compute
cost is too high for practical cryptographic use cases. A native syscall allows
validators to use audited bigint libraries while charging compute based on the
actual operation size.

### RSA-specific Syscalls

A full RSA verification syscall would cover the most common immediate use case,
but would unnecessarily bake message hashing, padding schemes, and key sizes
into the runtime. A fixed-exponent RSA helper syscall is also unnecessary when
generic ModExp metering follows the EIP-198 exponent-aware cost model. Programs
can verify RSA signatures by invoking `sol_big_mod_exp` with exponent `65537`,
while remaining responsible for hashing, padding checks, key validation, and
domain separation.

## New Terminology

- **ModExp**: Modular exponentiation, computing
  `(base ^ exponent) mod modulus`.
- **Big integer**: A non-negative integer encoded as a variable-length byte
  string.

## Detailed Design

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in [RFC 2119] and [RFC 8174].

### Syscall Interface

Add the following program-facing syscall function:

```rust
pub fn sol_big_mod_exp(
    params: *const BigModExpParams,
    result: *mut u8,
);
```

Where `params` points to the following struct in VM memory:

```rust
#[repr(C)]
pub struct BigModExpParams {
    pub base: *const u8,
    pub base_len: u64,
    pub exponent: *const u8,
    pub exponent_len: u64,
    pub modulus: *const u8,
    pub modulus_len: u64,
}
```

`params` is a VM pointer to a readable 48-byte `BigModExpParams` record with
the following ABI layout:

| Offset | Size | Field          |
| ------ | ---- | -------------- |
| 0      | 8    | `base`         |
| 8      | 8    | `base_len`     |
| 16     | 8    | `exponent`     |
| 24     | 8    | `exponent_len` |
| 32     | 8    | `modulus`      |
| 40     | 8    | `modulus_len`  |

Each field is a 64-bit little-endian unsigned integer. The pointer fields
`base`, `exponent`, and `modulus` are VM pointers to readable byte slices
with the following lengths:

1. `base` - `base_len` bytes
2. `exponent` - `exponent_len` bytes
3. `modulus` - `modulus_len` bytes

`result` is a VM pointer to a writable output buffer of exactly `modulus_len`
bytes.

The syscall computes:

```text
result = (base ^ exponent) mod modulus
```

and writes exactly `modulus_len` bytes to `result`. The output is encoded using
the same little-endian representation as the inputs and is padded to exactly
`modulus_len` bytes with trailing zeroes.

`base_len` MAY be smaller than, equal to, or larger than `modulus_len`. If the
decoded exponent value is `1`, the syscall computes a modular reduction:

```text
result = base mod modulus
```

The `result` range MAY overlap the `params` range or input ranges.
Implementations MUST behave as if the `params` record and all input bytes were
read from VM memory before any result byte is written.

### Length Limits

The initial maximum supported size is:

```rust
pub const BIG_MOD_EXP_MAX_BYTES: u64 = 512;
```

Each of `base_len`, `exponent_len`, and `modulus_len` MUST be less than or
equal to `BIG_MOD_EXP_MAX_BYTES`. This bound is applied to the explicit encoded
base, exponent, and modulus lengths, and to the implicit result length.
`exponent_len` bounds the number of exponent bits that can drive repeated
multiplication, while `base_len` and `modulus_len` bound operand parsing,
reduction, multiplication size, and the output length. The 512-byte limit covers
4096-bit RSA moduli and keeps the first version within a predictable compute
envelope. Larger operands can be introduced by a later SIMD after benchmarking
and validator implementation experience.

Zero-length `base` and `exponent` inputs are valid and are interpreted as the
integer `0`. `modulus_len` MUST be greater than zero.

### Output And Abort Behavior

The function returns no value on success. There are no non-fatal error return
values.

The syscall MUST abort the virtual machine if any of the following are true:

- The `params` pointer does not refer to a readable VM memory range of
  48 bytes.
- `base_len`, `exponent_len`, or `modulus_len` is greater than
  `BIG_MOD_EXP_MAX_BYTES`.
- Any pointer plus length calculation overflows, including `base + base_len`,
  `exponent + exponent_len`, `modulus + modulus_len`, and `result + modulus_len`.
- Any required VM memory range is not readable or writable as required.
- `modulus_len == 0`.
- The decoded modulus value is less than or equal to `1`.
- The decoded modulus value is even.
- The transaction does not have enough remaining compute units.

### Validation And Charging Order

Implementations MUST perform validation, compute charging, and arithmetic in
the following order:

1. Validate and read the `params` record from VM memory.
2. Validate all length fields, including maximum length checks and nonzero
   `modulus_len`.
3. Validate pointer plus length calculations for overflow, including
   `base + base_len`, `exponent + exponent_len`, `modulus + modulus_len`,
   and `result + modulus_len`.
4. Validate required input VM memory ranges are readable and the output VM
   memory range is writable.
5. Read all input bytes from VM memory, decode the exponent and modulus, and
   validate that the modulus is odd and greater than `1`.
6. Determine the compute cost.
7. Abort if the transaction does not have enough remaining compute units.
8. Charge compute.
9. Perform the arithmetic and write the result.

Aborts from steps 1 through 7 MUST NOT charge the syscall compute cost. After
step 8 succeeds, the charged compute units are consumed even if an
implementation-level failure aborts the virtual machine. Implementations MUST
NOT perform arithmetic before completing step 8.

### Arithmetic Semantics

All inputs are little-endian unsigned integers. Trailing zeroes are allowed and
do not change the integer value.

The decoded modulus value MUST be odd and greater than `1`. This requirement
rejects zero, one, and all even moduli, allowing implementations to rely on
reduction algorithms that require an odd non-degenerate modulus.

If `exponent` is zero, the result is `1 mod modulus`, encoded in exactly
`modulus_len` bytes. This defines the zero-base, empty-exponent case as
`0^0 mod modulus = 1` for every valid modulus.

### Compute Metering

The syscall MUST charge compute before performing the arithmetic. Metering MUST
follow the EIP-198 operation complexity model for the general ModExp path,
adapted to Solana compute units. A separate modular-reduction path is used when
the decoded exponent value is `1`.

```text
if decoded_exponent == 1:
    reduction_complexity =
        mod_reduce_complexity(base_len, modulus_len)
    compute_units =
        BIG_MOD_EXP_BASE_CU + ceil(reduction_complexity / BIG_MOD_EXP_CU_DIVISOR)
else:
    max_operand_len = max(base_len, modulus_len)
    effective_exponent_length =
        max(adjusted_exponent_length, BIG_MOD_EXP_MIN_EXPONENT_LENGTH)
    operation_complexity =
        mult_complexity(max_operand_len) * effective_exponent_length
    compute_units =
        BIG_MOD_EXP_BASE_CU + ceil(operation_complexity / BIG_MOD_EXP_CU_DIVISOR)
```

The initial draft constants are:

```rust
pub const BIG_MOD_EXP_BASE_CU: u64 = 422;
pub const BIG_MOD_EXP_CU_DIVISOR: u64 = 189;
pub const BIG_MOD_EXP_MIN_EXPONENT_LENGTH: u64 = 75;
pub const BIG_MOD_EXP_MOD_REDUCTION_COMPLEXITY_FACTOR: u64 = 15;
```

These values are preliminary, based on early EIP-198 cost-sweep benchmark data,
and MUST be finalized from implementation benchmarks before activation.
`BIG_MOD_EXP_BASE_CU` accounts for syscall overhead that is not represented by
EIP-198's pure arithmetic complexity formula.
`BIG_MOD_EXP_MIN_EXPONENT_LENGTH` accounts for fixed-exponent cases, including
common RSA exponents, whose measured runtime is not well represented by very
small adjusted exponent lengths.

The `decoded_exponent == 1` branch prices the modular-reduction use case
directly. This branch MUST be selected by the decoded exponent value, not by
`adjusted_exponent_length` or `effective_exponent_length`, because the minimum
exponent length raises small nonzero exponents for the general ModExp path.

The multiplication complexity function is:

```text
mult_complexity(x):
    if x <= 64:
        return x ** 2
    if x <= 1024:
        return x ** 2 // 4 + 96 * x - 3072
    return x ** 2 // 16 + 480 * x - 199680
```

The initial modular-reduction complexity function is:

```text
mod_reduce_complexity(base_len, modulus_len):
    return (
        mult_complexity(max(base_len, modulus_len)) *
        BIG_MOD_EXP_MOD_REDUCTION_COMPLEXITY_FACTOR
    )
```

`adjusted_exponent_length` MUST be computed using the EIP-198 rules over the
decoded exponent. Implementations MUST compute the same value that would be
obtained by viewing the little-endian exponent in most-significant-byte-first
order across exactly `exponent_len` bytes.

- If `exponent_len <= 32` and all exponent bits are zero, then
  `adjusted_exponent_length = 0`.
- If `exponent_len <= 32` and the exponent is nonzero, then
  `adjusted_exponent_length` is the zero-based index of the exponent's highest
  set bit.
- If `exponent_len > 32`, then `adjusted_exponent_length` is
  `8 * (exponent_len - 32)` plus the zero-based index of the highest set bit in
  the most significant 32 bytes of the fixed-width exponent. If those most
  significant 32 bytes are all zero, the index term is zero.

The general ModExp formula is based on encoded lengths, not the minimal
numerical byte length of any decoded value, so trailing zeroes do not reduce
`base_len`, `modulus_len`, or `exponent_len`. The same formula applies to RSA
verification use cases. For example, an RSA-2048 verification with exponent
`65537` uses `base_len = 256`, `modulus_len = 256`,
`adjusted_exponent_length = 16`, and
`effective_exponent_length = BIG_MOD_EXP_MIN_EXPONENT_LENGTH`.

### Benchmark Methodology

Benchmark results used to set the compute constants MUST be reproducible before
activation. The benchmark report SHOULD include:

- the validator implementation commit and bigint backend,
- hardware, operating system, compiler, and optimization settings,
- the exact benchmark command or harness,
- input generation details for balanced, RSA-style, modulus-driven, and
  exponent-driven cases, including `decoded_exponent == 1` modular-reduction
  cases,
- the selected values of `BIG_MOD_EXP_BASE_CU`, `BIG_MOD_EXP_CU_DIVISOR`,
  `BIG_MOD_EXP_MIN_EXPONENT_LENGTH`, and
  `BIG_MOD_EXP_MOD_REDUCTION_COMPLEXITY_FACTOR`, and
- the rule used to convert benchmark time to compute units.

### Test Vectors

Implementations MUST include tests for:

- The EIP-198 example, encoded as little-endian inputs and output:
  - `base =
    0x0300000000000000000000000000000000000000000000000000000000000000`
  - `exponent =
    0x2efcfffffeffffffffffffffffffffffffffffffffffffffffffffffffffffff`
  - `modulus =
    0x2ffcfffffeffffffffffffffffffffffffffffffffffffffffffffffffffffff`
  - `result =
    0x0100000000000000000000000000000000000000000000000000000000000000`
- Zero base and empty exponent with modulus `0x03`, writing `0x01`.
- Zero base and empty exponent with modulus `0x01` aborting the virtual
  machine.
- A base longer than the modulus with exponent `1`, such as
  `100000000^1 mod 5555`, encoded as little-endian inputs:
  - `base = 0x00e1f505`
  - `exponent = 0x01`
  - `modulus = 0xb315`
  - `result = 0x5d11`
- Zero, one, and even moduli aborting the virtual machine.
- Little-endian input decoding and output padding.
- Each VM abort condition listed above.

### Feature Activation

The syscall MUST be feature-gated and unavailable before activation. Validator
implementations MUST agree on:

- the syscall name and ABI,
- `BIG_MOD_EXP_MAX_BYTES`,
- output and abort behavior,
- `BIG_MOD_EXP_BASE_CU`, `BIG_MOD_EXP_CU_DIVISOR`, and
  `BIG_MOD_EXP_MIN_EXPONENT_LENGTH`,
- `BIG_MOD_EXP_MOD_REDUCTION_COMPLEXITY_FACTOR`,
- the `mod_reduce_complexity(base_len, modulus_len)` function, and
- the arithmetic test vectors.

## Impact

Dapp developers gain a practical primitive for RSA verification and other
number-theoretic cryptography. Programs remain responsible for higher-level
protocol details such as hashing, padding, key validation, and domain
separation.

Validators add a new variable-cost syscall backed by bigint arithmetic. The
bounded input size, deterministic edge-case behavior, and benchmarked compute
cost are required to keep execution predictable.

Programs that need modular reduction can use the same syscall with exponent
`1`, including cases where `base_len` is larger than `modulus_len`.

## Security Considerations

Underpricing is the main risk. Modular exponentiation has input-dependent cost,
especially as base size, modulus size, exponent length, and exponent density
change. Since syscall metering uses an EIP-198-style complexity formula for the
general ModExp path and a separate modular-reduction formula for exponent `1`,
the compute cost constants MUST be benchmarked across validator implementations
and should leave margin for worst-case valid inputs, including dense exponents,
large bases, and odd moduli that are slow for the selected implementation.

The syscall MUST NOT expose library-specific error behavior. All valid byte
strings within the length limit are unsigned integers with an odd decoded
modulus. Rejecting zero and even moduli avoids implementation-dependent
division-by-zero handling and slower even-modulus reduction paths.

The syscall is not suitable for secret exponents. On-chain program data is
public, and validator implementations are not required to execute bigint
operations in constant time.

Programs using this syscall for RSA signatures MUST implement the relevant
padding scheme checks, such as RSASSA-PKCS1-v1_5 or RSA-PSS, outside the
syscall. Raw modular exponentiation alone is not signature verification.

[EIP-198]: https://eips.ethereum.org/EIPS/eip-198
[RFC 2119]: https://www.ietf.org/rfc/rfc2119.txt
[RFC 8174]: https://www.ietf.org/rfc/rfc8174.txt
