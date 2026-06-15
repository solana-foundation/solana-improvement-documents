---
simd: 'XXXX'
title: SHAKE-256 Syscall
authors:
  - (fill in with names of authors)
category: Standard
type: Core
status: Idea
created: 2026-06-15
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Introduce a `sol_shake256` syscall with an interface similar to `sol_sha256`,
but with an explicit output length because SHAKE-256 is an extendable-output
function. The syscall hashes the supplied byte slices and writes the requested
number of output bytes to the caller-provided result buffer.

## Motivation

SHAKE-256 is a standardized hash-based extendable-output function used by
post-quantum cryptographic schemes, including Falcon's HashToPoint operation.
Programs that need SHAKE-256 today must implement it in BPF, which is expensive
in compute units and duplicates a common cryptographic primitive across
programs.

Exposing `sol_shake256` directly gives programs access to SHAKE-256 output at
syscall cost while keeping the ABI aligned with the existing hash syscalls.

## New Terminology

Extendable-output function (XOF): a function that hashes an input message and
can produce an arbitrary number of output bytes.

## Detailed Design

### Syscall Signature

The syscall follows the same input-slice interface as `sol_sha256`,
`sol_keccak256`, and `sol_blake3`, with one additional `result_len` argument:

https://docs.rs/solana-sha256-hasher/3.1.0/src/solana_sha256_hasher/lib.rs.html#62-75

```c
void sol_shake256(
    const SolBytes *bytes,
    uint64_t bytes_len,
    uint8_t *result,
    uint64_t result_len
);
```

The syscall computes SHAKE-256 over the provided byte slices as if they were a
single contiguous input. It writes the first `result_len` bytes of the
SHAKE-256 XOF stream to `result`.

The syscall uses these consensus constants:

```text
MAX_SHAKE256_INPUT_SLICES = 20_000
MAX_SHAKE256_OUTPUT_LEN = 65_536
SHAKE256_RATE_BYTES = 136
SOL_BYTES_SIZE = 16
SOL_BYTES_ALIGN = 8
```

These constants are part of the consensus ABI. Increasing or reducing any of
them requires a separate feature-gated consensus change.

The syscall aborts the virtual machine if any of these conditions are true:

- `bytes` is not aligned to `SOL_BYTES_ALIGN`.
- Not all bytes in `[bytes, bytes + bytes_len * SOL_BYTES_SIZE)` are
  readable.
- Not all bytes in each slice `[bytes[i].addr, bytes[i].addr + bytes[i].len)`
  are readable.
- `result_len > 0` and not all bytes in `[result, result + result_len)` are
  writable.
- `result_len > 0` and `[result, result + result_len)` overlaps the `SolBytes`
  descriptor array or any input slice.
- `bytes_len` exceeds `MAX_SHAKE256_INPUT_SLICES`.
  https://github.com/anza-xyz/agave/blob/289aa4ea46889a1535962b727c0656d4d25527dc/program-runtime/src/execution_budget.rs#L82
- `result_len` exceeds `MAX_SHAKE256_OUTPUT_LEN`.
- Any pointer and length arithmetic overflows, including
  `bytes_len * SOL_BYTES_SIZE`, every range end calculation, and the checked
  accumulation of `total_input_len`.
- Any CU cost calculation overflows, including
  `bytes_len * shake256_slice_cost`, `ceil(total_input_len /
  shake256_input_bytes_per_cu)`, `squeeze_blocks *
  shake256_squeeze_block_cost`, and the checked addition of `preflight_cost`
  or `input_cost` terms.
- Compute budget is exceeded.

A `result_len` of zero is valid and writes no output bytes. In this case
`result` is ignored, the VM does not translate `[result, result)`, and the
output alias check is skipped.

Each `SolBytes` descriptor is exactly `SOL_BYTES_SIZE` bytes and is encoded as
two little-endian unsigned 64-bit values: `addr` followed by `len`. The
descriptor array MUST be aligned to `SOL_BYTES_ALIGN`. Input slices and the
result range are byte slices and MUST NOT require alignment stricter than one
byte.

The output disjointness rule applies only to the descriptor array and input
slices. The result buffer MAY point to any other writable VM memory, including
writable account data, and any bytes written there are normal program-visible
side effects.

### Static Syscall Registration

For SBPFv3 programs, `sol_shake256` MUST follow SIMD-0178 static syscall
registration. This proposal does not define a per-syscall hash seed or override
the global static-syscall hash algorithm. The static syscall hash MUST be the
MurmurHash3_x86_32 value assigned by the SIMD-0178 canonical syscall registry
for the UTF-8 bytes of `sol_shake256`.

If SIMD-0178 does not define a canonical registry algorithm and seed before
activation, this proposal depends on a prerequisite SIMD-0178 amendment. This
proposal MUST NOT introduce a local static-syscall hash seed.

The activation PR MUST include
`assets/simd-xxxx-shake256/static-syscall-hash.txt` with the hash algorithm,
seed, computed hash, active registered syscall hashes, and collision result.
Every validator client MUST reproduce this file in CI and prove that the
registered value does not collide with any active syscall hash.

### Compute Unit Usage

Compute costs extend the existing hash syscall model with mandatory charges for
the output squeeze phase.

https://github.com/anza-xyz/agave/blob/289aa4ea46889a1535962b727c0656d4d25527dc/syscalls/src/lib.rs#L168-L169

Successful calls are charged in two debits:

```text
preflight_cost =
    shake256_base_cost
+ bytes_len * shake256_slice_cost
+ squeeze_blocks * shake256_squeeze_block_cost

input_cost =
    ceil(total_input_len / shake256_input_bytes_per_cu)
```

Where:

```text
squeeze_blocks =
    0, if result_len == 0
    ceil(result_len / SHAKE256_RATE_BYTES), otherwise
```

The total successful-call SHAKE-256 cost is `preflight_cost + input_cost`.
The `shake256_slice_cost` parameter MUST be no lower than the existing
per-slice memory-operation cost used by the fixed-output hash syscalls, and
MUST include descriptor translation, slice-length accumulation, input-range
translation, and output-alias comparison work for each slice.
`shake256_squeeze_block_cost` MUST include the cost of each Keccak-f[1600]
permutation required by the SHAKE-256 squeeze phase, including the first
permutation required for any non-zero output.

The final values for `shake256_base_cost`, `shake256_input_bytes_per_cu`,
`shake256_slice_cost`, and `shake256_squeeze_block_cost` MUST be set by
benchmarking and activated as consensus parameters. `shake256_base_cost`,
`shake256_slice_cost`, and `shake256_squeeze_block_cost` MUST be greater than
zero. `shake256_input_bytes_per_cu` MUST be greater than zero and MUST NOT
exceed `SHAKE256_RATE_BYTES`. The activated parameters MUST satisfy:

```text
ceil(SHAKE256_RATE_BYTES / shake256_input_bytes_per_cu)
    >= shake256_squeeze_block_cost
```

These parameters SHOULD be selected so that a caller cannot produce large XOF
output or many tiny input slices more cheaply through this syscall than through
other runtime-metered work.

### Validation and Metering Order

Implementations MUST validate and meter calls in this order:

1. Validate `bytes_len` and `result_len` against their maximums using checked
   arithmetic for `bytes_len * SOL_BYTES_SIZE`, `squeeze_blocks`, and all
   `preflight_cost` terms. The syscall MUST abort if any of these calculations
   overflow.
2. Deduct `preflight_cost`. If the caller has insufficient remaining CUs, the
   syscall MUST abort.
3. Translate and read the `SolBytes` descriptor array.
4. Compute `total_input_len` with checked addition over every slice length. The
   syscall MUST abort if this accumulation overflows.
5. Compute and deduct `input_cost` using checked arithmetic. If the caller has
   insufficient remaining CUs, the syscall MUST abort.
6. Translate all input slice ranges and, when `result_len > 0`, the output
   range.
7. Validate that the output range is disjoint from the descriptor array and all
   input slices when `result_len > 0`.
8. Absorb all input bytes, squeeze the requested output, and write the output
   bytes.

If validation fails before step 2, the syscall MUST consume zero CUs from the
SHAKE-256 cost formula. This includes overflow while computing
`preflight_cost`. If validation fails after step 2 and before step 5,
`preflight_cost` remains consumed and `input_cost` is not deducted. If
validation fails after step 5, both `preflight_cost` and `input_cost` remain
consumed.

This section defines SHAKE-256-specific metering. Any VM-wide syscall dispatch
cost that applies uniformly to all syscalls MUST be charged identically on
every `sol_shake256` invocation and on every abort path. No additional
SHAKE-256-specific cost may be charged outside the two debits above.

The implementation MUST NOT write any output byte until all input bytes have
been absorbed.

## Alternatives Considered

### BPF Implementation

Programs can implement SHAKE-256 in BPF today, but at a higher CU cost. This
also forces each program to carry its own implementation of a standard
cryptographic primitive.

### Fixed-Length SHAKE Variants

The runtime could expose fixed-length helpers such as `sol_shake256_32` or
`sol_shake256_64`. This would match the fixed-output hash syscall shape, but it
would not serve protocols that require different SHAKE output lengths. An
explicit `result_len` keeps the syscall general while preserving the existing
slice-based input ABI.

### SHAKE-128

The runtime could expose both SHAKE-128 and SHAKE-256. This proposal exposes
only SHAKE-256 because the known Solana use cases require SHAKE-256 and a
single variant keeps the ABI and compute pricing simpler.

### Status Quo

Continue without exposing SHAKE-256. Programs requiring SHAKE-256, including
post-quantum cryptographic protocols, remain unable to access the primitive at
syscall cost.

## Impact

Programs gain access to SHAKE-256 at syscall cost, including variable-length
XOF output. SDKs can add wrappers that mirror existing hash syscall helpers
while requiring callers to pass an output buffer length.

## Security Considerations

The memory-safety surface is the same as the existing hash syscalls, with the
additional need to validate and meter `result_len`. Implementations MUST charge
for both absorbed input bytes and squeezed output bytes, and MUST reject calls
whose `result_len` exceeds `MAX_SHAKE256_OUTPUT_LEN`.

SHAKE-256 itself is standardized in FIPS 202. Implementations MUST use the
FIPS 202 SHAKE-256 function, including the SHAKE domain-separation suffix and
the 136-byte rate. Callers that need protocol-level domain separation MUST
include that domain separation in the input bytes before invoking the syscall.

If a validator client also implements SIMD-0461, the SHAKE-256 primitive used
by `sol_shake256` and the SHAKE-256 primitive used by Falcon HashToPoint MUST
produce identical FIPS 202 SHAKE-256 output for every byte string. Sharing one
implementation is RECOMMENDED, but consensus conformance is defined by
observable output identity.

`MAX_SHAKE256_OUTPUT_LEN` limits XOF output length only. It is independent of
SIMD-0461's `MAX_FALCON_MESSAGE_LEN`, which limits Falcon message input length.
Programs that use both syscalls MUST apply the Falcon message limit before
calling `sol_falcon512_verify`.

## Backwards Compatibility

This is an additive change gated behind a feature flag. Programs that do not
invoke `sol_shake256` are unaffected. Existing syscalls are unchanged.

## Conformance

Implementations MUST include cross-client conformance tests for:

- A pinned FIPS 202 SHAKE-256 known-answer corpus under
  `assets/simd-xxxx-shake256/`. The final accepted SIMD MUST include the exact
  NIST CAVP or ACVTS source URLs, normalized vector files, and SHA-256
  checksums in that directory. The corpus MUST include empty input, short input,
  input exactly one SHAKE-256 rate block, input crossing a rate-block boundary,
  long input, 32-byte output, 64-byte output, and `MAX_SHAKE256_OUTPUT_LEN`
  output. Implementations MUST NOT substitute a client-local corpus.
- Multiple input slices whose concatenation matches the same single-slice test.
- Zero-length output with each of these invalid `result` pointers: null
  (`0x0`), an unmapped non-zero VM address, and a read-only mapped address.
- Rejection of `bytes_len > MAX_SHAKE256_INPUT_SLICES`.
- Rejection of `result_len > MAX_SHAKE256_OUTPUT_LEN`.
- Rejection of `SolBytes` descriptor arrays not aligned to `SOL_BYTES_ALIGN`,
  and acceptance of byte-aligned input and result ranges.
- Rejection of output ranges that overlap the descriptor array or any input
  slice.
- Rejection of checked arithmetic overflow in `total_input_len`.
- Rejection of checked arithmetic overflow in every CU cost term.
- Rejection of invalid consensus cost parameters, including
  `shake256_input_bytes_per_cu == 0`,
  `shake256_input_bytes_per_cu > SHAKE256_RATE_BYTES`, and parameters that do
  not satisfy the rate-block input-cost lower bound.
- Exact expected CU charges for many tiny slices, for non-zero output shorter
  than one SHAKE-256 rate block, for output exactly one rate block, and for
  output crossing a rate-block boundary. Tests MUST assert the exact
  `preflight_cost`, `input_cost`, and total CU delta from the formula using the
  activated consensus cost parameters.
- Zero SHAKE-256 formula CU consumption for validation failures before
  `preflight_cost` deduction, `preflight_cost`-only consumption for validation
  failures before `input_cost` deduction, and full SHAKE-256 formula CU
  consumption for validation failures after `input_cost` deduction.
- If SIMD-0461 is implemented by the same client, the SHAKE-256 primitive used
  by Falcon HashToPoint MUST be tested against the same pinned SHAKE-256 corpus
  used for `sol_shake256`, including the rate-boundary vectors.
- MurmurHash3_x86_32 static syscall registration using the SIMD-0178 canonical
  registry entry and collision-free dispatch with the active syscall table. The
  generated `assets/simd-xxxx-shake256/static-syscall-hash.txt` proof MUST be
  checked into the proposal before acceptance.
