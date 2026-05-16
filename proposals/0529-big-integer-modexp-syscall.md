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

### RSA-specific Syscall

An RSA verification syscall would cover the most common immediate use case, but
would unnecessarily bake message hashing, padding schemes, and key sizes into
the runtime. A generic ModExp syscall keeps those protocol choices in programs.

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
    endianness: u64,
    params: &BigModExpParams,
) -> Vec<u8>;
```

`endianness` MUST be one of:

```rust
pub const BIG_MOD_EXP_ENDIANNESS_BE: u64 = 0;
pub const BIG_MOD_EXP_ENDIANNESS_LE: u64 = 1;
```

The syscall reads a parameter block from VM memory:

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

Pointer fields are VM pointers to byte slices. Length fields are unsigned
64-bit values. Implementations MUST interpret this structure according to the
stable sBPF ABI. No padding bytes are included beyond the fields shown above.

The syscall computes:

```text
result = (base ^ exponent) mod modulus
```

and returns the result as `Vec<u8>`. The returned vector length MUST equal
`modulus_len`. The output is encoded using the same endianness as the inputs
and is padded to exactly `modulus_len` bytes. For big-endian output, padding
bytes are leading zeroes. For little-endian output, padding bytes are trailing
zeroes. No separate result length is provided because the output length is
fully determined by `modulus_len`.

### Length Limits

The initial maximum supported size is:

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

### Return Value

The function returns the result bytes directly on success. There are no
non-fatal error return values.

The syscall MUST abort the virtual machine if any of the following are true:

- `endianness` is not a supported value.
- Any input length is greater than `BIG_MOD_EXP_MAX_BYTES`.
- Any pointer plus length calculation overflows.
- Any required VM memory range is not readable as required.
- `modulus_len == 0`.
- The decoded modulus value is even.
- The transaction does not have enough remaining compute units.

### Arithmetic Semantics

All inputs are unsigned integers. Leading zeroes in big-endian inputs and
trailing zeroes in little-endian inputs are allowed and do not change the
integer value.

The decoded modulus value MUST be odd. This requirement rejects zero and all
even moduli, allowing implementations to rely on reduction algorithms that
require an odd modulus.

If `exponent` is zero, the result is `1 mod modulus`, encoded in exactly
`modulus_len` bytes.

### Compute Metering

The syscall MUST charge compute before performing the exponentiation. Metering
MUST be determined by the modulus size bucket, where:

```text
modulus_bits = modulus_len * 8
```

The bucket is based on `modulus_len`, not the numerical bit length of the
modulus value, so leading or trailing zeroes do not reduce the cost.

The following placeholder constants define the initial bucket schedule. The
concrete compute unit values MUST be set from implementation benchmarks before
activation.

| Modulus size | Compute cost placeholder |
| --- | --- |
| `1..=32` bits | `big_mod_exp_cu_le_32_bits` |
| `33..=64` bits | `big_mod_exp_cu_33_to_64_bits` |
| `65..=128` bits | `big_mod_exp_cu_65_to_128_bits` |
| `129..=256` bits | `big_mod_exp_cu_129_to_256_bits` |
| `257..=384` bits | `big_mod_exp_cu_257_to_384_bits` |
| `385..=512` bits | `big_mod_exp_cu_385_to_512_bits` |
| `513..=1024` bits | `big_mod_exp_cu_513_to_1024_bits` |
| `1025..=2048` bits | `big_mod_exp_cu_1025_to_2048_bits` |
| `2049..=4096` bits | `big_mod_exp_cu_2049_to_4096_bits` |

The selected bucket constant is the syscall cost. The bucket constants MUST be
priced for the worst-case valid inputs in that modulus-size range, including
the maximum permitted `base_len` and `exponent_len`, returned output allocation,
and slower exponentiation cases. Runtime can vary for inputs with the same
modulus size, and dense exponents can require more multiplications than sparse
exponents in common variable-time exponentiation algorithms. Benchmarks used to
set the constants MUST include these slower valid cases, such as all-ones
exponents.

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
- Each VM abort condition listed above.

### Feature Activation

The syscall MUST be feature-gated and unavailable before activation. Validator
implementations MUST agree on:

- the syscall name and ABI,
- `BIG_MOD_EXP_MAX_BYTES`,
- return and abort behavior,
- the concrete compute cost constants, and
- the arithmetic test vectors.

## Impact

Dapp developers gain a practical primitive for RSA verification and other
number-theoretic cryptography. Programs remain responsible for higher-level
protocol details such as hashing, padding, key validation, and domain
separation.

Validators add a new variable-cost syscall backed by bigint arithmetic. The
bounded input size, deterministic edge-case behavior, and benchmarked compute
cost are required to keep execution predictable.

## Security Considerations

Underpricing is the main risk. Modular exponentiation has input-dependent cost,
especially as modulus size, exponent length, and exponent density change. Since
metering uses only the modulus-size bucket, the compute cost constants MUST be
benchmarked across validator implementations and should leave margin for
worst-case valid inputs in each bucket, including maximum-length exponents,
dense exponents, and odd moduli that are slow for the selected implementation.

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
