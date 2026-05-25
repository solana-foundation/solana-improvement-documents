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
unsigned big integers, plus a fixed-exponent RSA helper syscall for the common
public exponent `65537`:

```text
result = (base ^ exponent) mod modulus
```

The generic syscall is analogous to Ethereum's ModExp precompile specified by
[EIP-198], but exposes a Solana-native syscall interface instead of the EVM
precompile ABI. Inputs are fixed byte slices, and the output is returned as a
fixed-width byte vector encoded either as a big-endian or little-endian unsigned
integer.

## Motivation

Modular exponentiation is a foundation for RSA verification, accumulators,
some verifiable delay functions, and other number-theoretic cryptography.
These operations are prohibitively expensive when implemented directly in
sBPF, especially for common RSA modulus sizes such as 2048 or 4096 bits.

Ethereum exposes the same arithmetic operation through its ModExp precompile.
Adding a Solana syscall provides similar cryptographic building blocks to
on-chain programs while preserving Solana's program-facing syscall model and
compute metering.

This syscall family is also useful for interoperability. Programs that verify
Ethereum-oriented proofs, signatures, or attestations can reuse the same
high-level arithmetic assumptions while adapting only the call interface.

## Alternatives Considered

### Exact EIP-198 ABI

The generic syscall could accept one packed input buffer using the exact EIP-198
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

### Full RSA Verification Syscall

A full RSA verification syscall would cover the most common immediate use case,
but would unnecessarily bake message hashing, padding schemes, and key sizes
into the runtime. This proposal keeps those protocol choices in programs. It
only adds a fixed-exponent arithmetic helper for common RSA-2048 and RSA-4096
public keys.

## New Terminology

- **ModExp**: Modular exponentiation, computing
  `(base ^ exponent) mod modulus`.
- **Big integer**: A non-negative integer encoded as a variable-length byte
  string.

## Detailed Design

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in [RFC 2119] and [RFC 8174].

### Syscall Interfaces

Add the following program-facing generic syscall function:

```rust
pub fn sol_big_mod_exp(
    endianness: u64,
    params: &BigModExpParams,
) -> Vec<u8>;
```

Add the following program-facing fixed-exponent RSA helper syscall function:

```rust
pub fn sol_big_mod_exp_rsa(
    endianness: u64,
    params: &BigModExpRsaParams,
) -> Vec<u8>;
```

`endianness` MUST be one of:

```rust
pub const BIG_MOD_EXP_ENDIANNESS_BE: u64 = 0;
pub const BIG_MOD_EXP_ENDIANNESS_LE: u64 = 1;
```

The generic syscall reads a parameter block from VM memory:

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

The fixed-exponent RSA helper syscall reads a parameter block from VM memory:

```rust
#[repr(C)]
pub struct BigModExpRsaParams {
    pub base: *const u8,
    pub modulus: *const u8,
    pub modulus_len: u64,
}
```

Pointer fields are VM pointers to byte slices. Length fields are unsigned
64-bit values. Implementations MUST interpret these structures according to the
stable sBPF ABI. No padding bytes are included beyond the fields shown above.

The generic syscall computes:

```text
result = (base ^ exponent) mod modulus
```

and returns the result as `Vec<u8>`. The returned vector length MUST equal
`modulus_len`. The output is encoded using the same endianness as the inputs
and is padded to exactly `modulus_len` bytes. For big-endian output, padding
bytes are leading zeroes. For little-endian output, padding bytes are trailing
zeroes. No separate result length is provided because the output length is
fully determined by `modulus_len`.

The fixed-exponent RSA helper syscall computes:

```text
result = (base ^ 65537) mod modulus
```

It MUST NOT accept or use any caller-provided exponent. `sol_big_mod_exp_rsa`
MUST read exactly `modulus_len` bytes for `base` and exactly `modulus_len`
bytes for `modulus`, and MUST return exactly `modulus_len` bytes. The RSA
helper inputs and outputs use the supplied `endianness` value in the same way as
`sol_big_mod_exp`.

### Length Limits

The initial maximum supported size for the generic syscall is:

```rust
pub const BIG_MOD_EXP_MAX_BYTES: u64 = 512;
```

Each of `base_len`, `exponent_len`, and `modulus_len` MUST be less than or
equal to `BIG_MOD_EXP_MAX_BYTES`. This single bound is intentionally applied to
all three operands. `exponent_len` bounds the number of exponent bits that can
drive repeated multiplication, while `base_len` and `modulus_len` bound operand
parsing, reduction and multiplication size, and the returned vector length. The
512-byte limit covers 4096-bit RSA moduli and keeps the first version within a
predictable compute envelope. Larger operands can be introduced by a later SIMD
after benchmarking and validator implementation experience.

Zero-length `base` and `exponent` inputs are valid and are interpreted as the
integer `0`. `modulus_len` MUST be greater than zero.

The RSA helper syscall supports the following operand sizes:

```rust
pub const BIG_MOD_EXP_RSA2048_BYTES: u64 = 256;
pub const BIG_MOD_EXP_RSA4096_BYTES: u64 = 512;
pub const BIG_MOD_EXP_RSA_PUBLIC_EXPONENT: u64 = 65_537;
```

### Return Value

The functions return the result bytes directly on success. There are no
non-fatal error return values.

Both syscalls MUST abort the virtual machine if any of the following are true:

- `endianness` is not a supported value.
- Any pointer plus dynamic or fixed length calculation overflows.
- Any required VM memory range is not readable as required.
- The decoded modulus value is even.
- The transaction does not have enough remaining compute units.

The generic syscall MUST also abort the virtual machine if any of the following
are true:

- Any input length is greater than `BIG_MOD_EXP_MAX_BYTES`.
- `modulus_len == 0`.

The RSA helper syscall MUST also abort the virtual machine if `modulus_len` is
not `BIG_MOD_EXP_RSA2048_BYTES` or `BIG_MOD_EXP_RSA4096_BYTES`.

### Arithmetic Semantics

All inputs are unsigned integers. Leading zeroes in big-endian inputs and
trailing zeroes in little-endian inputs are allowed and do not change the
integer value.

The decoded modulus value MUST be odd. This requirement rejects zero and all
even moduli, allowing implementations to rely on reduction algorithms that
require an odd modulus.

For the generic syscall, if `exponent` is zero, the result is `1 mod modulus`,
encoded in exactly `modulus_len` bytes.

### Compute Metering

The generic syscall MUST charge compute before performing the exponentiation.
Metering MUST be determined by the modulus size bucket, where:

```text
modulus_bits = modulus_len * 8
```

The bucket is based on `modulus_len`, not the numerical bit length of the
modulus value, so leading or trailing zeroes do not reduce the cost.

The following benchmark-derived constants define the initial bucket schedule for
`sol_big_mod_exp`. Values are rounded up from the upper end of the supplied
benchmark interval using `ceil(nanoseconds / 10)`.

| Modulus size | Compute units | Constant |
| --- | --- | --- |
| `1..=32` bits | `248` | `BIG_MOD_EXP_CU_32_BITS` |
| `33..=64` bits | `282` | `BIG_MOD_EXP_CU_64_BITS` |
| `65..=128` bits | `644` | `BIG_MOD_EXP_CU_128_BITS` |
| `129..=256` bits | `1_721` | `BIG_MOD_EXP_CU_256_BITS` |
| `257..=384` bits | `3_546` | `BIG_MOD_EXP_CU_384_BITS` |
| `385..=512` bits | `6_667` | `BIG_MOD_EXP_CU_512_BITS` |
| `513..=1024` bits | `38_183` | `BIG_MOD_EXP_CU_1024_BITS` |
| `1025..=2048` bits | `271_540` | `BIG_MOD_EXP_CU_2048_BITS` |
| `2049..=4096` bits | `2_080_200` | `BIG_MOD_EXP_CU_4096_BITS` |

The selected bucket constant is the syscall cost. The bucket constants MUST be
priced for the worst-case valid inputs in that modulus-size range, including
the maximum permitted `base_len` and `exponent_len`, returned output allocation,
and slower exponentiation cases. Runtime can vary for inputs with the same
modulus size, and dense exponents can require more multiplications than sparse
exponents in common variable-time exponentiation algorithms. Benchmarks used to
set the constants MUST include these slower valid cases, such as all-ones
exponents.

The fixed-exponent RSA helper syscall MUST charge compute before performing the
exponentiation. Metering MUST be determined by `modulus_len`:

| Modulus size | `modulus_len` | Exponent | Compute units | Constant |
| --- | --- | --- | --- | --- |
| 2048 bits | `256` | `65537` | `10_020` | `BIG_MOD_EXP_RSA_CU_2048_BITS` |
| 4096 bits | `512` | `65537` | `38_197` | `BIG_MOD_EXP_RSA_CU_4096_BITS` |

These fixed-exponent costs are only valid for `sol_big_mod_exp_rsa`. The
generic `sol_big_mod_exp` syscall MUST continue to use the existing
`1025..=2048` and `2049..=4096` modulus-size buckets for 2048-bit and 4096-bit
operations with caller-provided exponents.

### Test Vectors

Implementations MUST include tests for:

- The EIP-198 example:
  - `base = 0x03`
  - `exponent =
    0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2e`
  - `modulus =
    0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f`
  - result is 32 bytes ending in `0x01`
- Empty base and empty exponent with modulus `0x03`, returning `0x01`.
- Zero and even moduli aborting the virtual machine.
- Big-endian and little-endian encodings of the same values producing
  equivalent integer results.
- Returned vector length and padding for both endiannesses.
- `sol_big_mod_exp_rsa` producing the same result as `sol_big_mod_exp` with
  exponent `65537` for both supported RSA modulus sizes.
- Each VM abort condition listed above.

### Feature Activation

The syscalls MUST be feature-gated and unavailable before activation. Validator
implementations MUST agree on:

- the syscall names and ABI,
- `BIG_MOD_EXP_MAX_BYTES` and the supported RSA modulus sizes,
- return and abort behavior,
- the concrete compute cost constants, and
- the arithmetic test vectors.

## Impact

Dapp developers gain practical primitives for RSA verification and other
number-theoretic cryptography. Programs remain responsible for higher-level
protocol details such as hashing, padding, key validation, and domain
separation.

Validators add new variable-cost syscalls backed by bigint arithmetic. The
bounded input sizes, deterministic edge-case behavior, and benchmarked compute
costs are required to keep execution predictable.

## Security Considerations

Underpricing is the main risk. Modular exponentiation has input-dependent cost,
especially as modulus size, exponent length, and exponent density change. Since
generic syscall metering uses only the modulus-size bucket, the compute cost
constants MUST be benchmarked across validator implementations and should leave
margin for worst-case valid inputs in each bucket, including maximum-length
exponents, dense exponents, and odd moduli that are slow for the selected
implementation. The RSA helper costs rely on the supported `modulus_len`
values and fixed public exponent `65537`.

The syscalls MUST NOT expose library-specific error behavior. All valid byte
strings within their length limits are unsigned integers with an odd decoded
modulus. Rejecting zero and even moduli avoids implementation-dependent
division-by-zero handling and slower even-modulus reduction paths.

The syscalls are not suitable for secret exponents. On-chain program data is
public, and validator implementations are not required to execute bigint
operations in constant time.

Programs using these syscalls for RSA signatures MUST implement the relevant
padding scheme checks, such as RSASSA-PKCS1-v1_5 or RSA-PSS, outside the
syscall. Raw modular exponentiation alone is not signature verification.

[EIP-198]: https://eips.ethereum.org/EIPS/eip-198
[RFC 2119]: https://www.ietf.org/rfc/rfc2119.txt
[RFC 8174]: https://www.ietf.org/rfc/rfc8174.txt
