---
simd: '0563'
title: Keccak-f[1600] Syscall
authors:
  - SK, ZZ
category: Standard
type: Core
status: Idea
created: 2026-06-15
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Introduce a `sol_keccak_f1600` syscall that applies the full 24-round
Keccak-f[1600] permutation to a caller-provided 1600-bit state. The syscall is
lower level than a SHAKE or SHA3 hash syscall: programs and SDKs build the
desired sponge mode around it by applying the correct rate, padding, domain
separation suffix, and output handling.

Exposing the permutation instead of one fixed mode supports SHAKE128,
SHAKE256, SHA3-224, SHA3-256, SHA3-384, SHA3-512, legacy Keccak-256, and
protocol-specific Keccak sponge or duplex constructions with one consensus ABI.

## Motivation

The initial motivation for this proposal was SHAKE-256, but SHAKE-256 is only
one mode of the Keccak-f[1600] permutation. FIPS 202 defines the SHA-3 hash
functions and SHAKE XOFs as modes of Keccak-p[1600,24], which is equivalent to
Keccak-f[1600]:

https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.202.pdf

Several cryptographic protocols need different members of the same family or
custom sponge framing:

- ML-DSA, the standardized form of CRYSTALS-Dilithium, uses both SHAKE128 and
  SHAKE256, including an incremental SHAKE API.
  https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.204.pdf
- Falcon uses SHAKE-256 for HashToPoint.
  https://falcon-sign.info/falcon.pdf
- SLH-DSA, the standardized form of SPHINCS+, defines SHAKE parameter sets that
  instantiate its hash and PRF functions with SHAKE256.
  https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.205.pdf
- Bulletproof-style proof systems commonly use transcript frameworks such as
  Merlin/STROBE. Merlin is a typed wrapper around a STROBE object instantiated
  with Keccak-f/1600, with protocol-specific framing and challenge extraction.
  https://merlin.cool/print.html

A `sol_shake256` syscall would help one set of callers but would still leave
SHAKE128, SHA3-512, legacy Keccak-256 compatibility, cSHAKE/KMAC-style modes,
and custom transcript sponges to software implementations. Defining the syscall
at the permutation boundary keeps the runtime ABI small while letting libraries
implement the full Keccak family.

## New Terminology

Keccak-f[1600]: The 24-round permutation over a 1600-bit Keccak state. In FIPS
202 terminology, Keccak-p[1600,24] is equivalent to Keccak-f[1600].

Sponge construction: A construction that absorbs input blocks into a state,
applies a permutation between blocks, and squeezes output from the state.

Rate: The number of state bytes exposed to a sponge mode for absorbing and
squeezing. The remaining bytes are the capacity.

Extendable-output function (XOF): A function that hashes an input message and
can produce an arbitrary number of output bytes.

## Detailed Design

### Syscall Signature

The syscall mutates one Keccak state in place:

```c
void sol_keccak_f1600(uint64_t *state);
```

`state` points to exactly 25 little-endian `uint64_t` lanes, for 200 total
bytes. The lane at `state[x + 5 * y]` is the Keccak lane `A[x,y]`, with
`0 <= x < 5` and `0 <= y < 5`. Validator clients MUST decode and encode these
lanes as little-endian values regardless of host architecture.

Each syscall invocation applies Keccak-p[1600,24], equivalent to
Keccak-f[1600], to the current state exactly once. The final state is written
back to the same memory range.

The syscall does not implement any sponge mode. In particular, it does not:

- absorb input bytes,
- apply `pad10*1`,
- apply SHA3, SHAKE, legacy Keccak, cSHAKE, KMAC, STROBE, or protocol-specific
  domain-separation suffixes,
- enforce a rate or capacity,
- squeeze output bytes.

Those operations are caller or SDK responsibilities. For example, a SHAKE256
wrapper uses a 136-byte rate, the SHAKE domain-separation suffix, FIPS 202
padding, and this syscall for every required Keccak-f[1600] permutation.

The syscall uses these consensus constants:

```text
KECCAK_F1600_STATE_BYTES = 200
KECCAK_F1600_STATE_LANES = 25
KECCAK_F1600_LANE_BYTES = 8
KECCAK_F1600_STATE_ALIGN = 8
KECCAK_F1600_ROUNDS = 24
```

These constants are part of the consensus ABI. Changing any of them requires a
separate feature-gated consensus change.

The syscall aborts the virtual machine if any of these conditions are true:

- `state` is not aligned to `KECCAK_F1600_STATE_ALIGN`.
- Not all bytes in `[state, state + KECCAK_F1600_STATE_BYTES)` are readable and
  writable.
- Any pointer and length arithmetic overflows, including the checked range end
  calculation for `[state, state + KECCAK_F1600_STATE_BYTES)`.
- Compute budget is exceeded.

### Static Syscall Registration

For SBPFv3 programs, `sol_keccak_f1600` MUST follow SIMD-0178 static syscall
registration. This proposal does not define a per-syscall hash seed or override
the global static-syscall hash algorithm. The static syscall hash MUST be the
MurmurHash3_x86_32 value assigned by the SIMD-0178 canonical syscall registry
for the UTF-8 bytes of `sol_keccak_f1600`.

If SIMD-0178 does not define a canonical registry algorithm and seed before
activation, this proposal depends on a prerequisite SIMD-0178 amendment. This
proposal MUST NOT introduce a local static-syscall hash seed.

The activation PR MUST include
`assets/simd-xxxx-keccak-f1600/static-syscall-hash.txt` with the hash
algorithm, seed, computed hash, active registered syscall hashes, and collision
result. Every validator client MUST reproduce this file in CI and prove that
the registered value does not collide with any active syscall hash.

### Compute Unit Usage

Compute costs are based on one full Keccak-f[1600] permutation per syscall
invocation. One full Keccak-f[1600] permutation, including all 24 rounds, is the
unit that MUST be benchmarked.

Successful calls are charged:

```text
keccak_f1600_cost = 130
```

`keccak_f1600_cost` covers fixed syscall-specific work, including validation
and translation of the 200-byte state range, and one complete Keccak-f[1600]
permutation. The activated parameter MUST be greater than zero.

Higher-level SHAKE, SHA3, Keccak-256, or transcript wrappers determine their
total syscall cost by the number of Keccak-f[1600] permutations required by
the mode. For example, a wrapper generally invokes this syscall once for each
padded absorb block and once whenever squeezing must advance to the next rate
block. The exact number is mode-specific and remains outside the syscall ABI.

This parameter SHOULD be selected so that callers cannot obtain many native
Keccak-f[1600] permutations more cheaply through this syscall than through
other runtime-metered work with comparable validator cost.

### Validation and Metering Order

Implementations MUST validate and meter calls in this order:

1. Deduct `keccak_f1600_cost`. If the caller has insufficient remaining CUs,
   the syscall MUST abort.
2. Validate `state` alignment.
3. Translate `[state, state + KECCAK_F1600_STATE_BYTES)` as a readable and
   writable VM memory range. The syscall MUST abort if the range end
   calculation overflows or if the range is not fully readable and writable.
4. Read the state, apply Keccak-f[1600] exactly once, and write the final state
   back to the same range.

If validation fails after step 1, the full `keccak_f1600_cost` remains
consumed.

This section defines Keccak-f[1600]-specific metering. Any VM-wide syscall
dispatch cost that applies uniformly to all syscalls MUST be charged
identically on every `sol_keccak_f1600` invocation and on every abort path. No
additional Keccak-f[1600]-specific cost may be charged outside the debit above.

The implementation MUST NOT write any state byte until the input state has been
fully translated and read.

## Alternatives Considered

### SHAKE-256 Syscall

The runtime could expose `sol_shake256` with an explicit output length. This is
more ergonomic for Falcon and other SHAKE256-only callers, but it does not help
protocols that need SHAKE128, SHA3-512, legacy Keccak-256, cSHAKE/KMAC-style
customization, or STROBE/Merlin-style transcript framing.

### Fixed Family Syscalls

The runtime could expose separate helpers such as `sol_shake128`,
`sol_shake256`, `sol_sha3_512`, and `sol_keccak256_xof`. This would make each
common mode easier to call, but it would expand the syscall ABI and still fail
to cover custom sponge and duplex constructions.

### BPF Implementation

Programs can implement Keccak-f[1600] in BPF today, but at a higher CU cost.
This also forces each program to carry its own implementation of a common
cryptographic primitive and makes it harder for libraries to share one tested
implementation.

### Status Quo

Continue without exposing Keccak-f[1600]. Programs requiring SHAKE,
SHA3-family hashes, or custom Keccak sponge transcripts remain unable to access
the shared expensive primitive at syscall cost.

## Impact

Programs gain access to the native Keccak-f[1600] permutation at syscall cost.
SDKs can add mode-specific wrappers for SHAKE128, SHAKE256, SHA3-224,
SHA3-256, SHA3-384, SHA3-512, legacy Keccak-256, and protocol-specific
transcripts without requiring new runtime syscalls.

Existing `sol_keccak256` behavior is unchanged.

## Security Considerations

This syscall exposes a raw permutation, not a hash function. Callers MUST use a
correct sponge construction for their protocol, including the intended rate,
capacity, padding rule, and domain-separation suffix. A SHA3 wrapper, a SHAKE
wrapper, legacy Keccak-256, and a STROBE transcript all use different framing
rules even though they share Keccak-f[1600].

Implementations MUST implement Keccak-p[1600,24], equivalent to
Keccak-f[1600], exactly as specified by FIPS 202. They MUST NOT substitute a
reduced-round permutation or a hash-mode-specific implementation whose behavior
is only correct for one rate or suffix.

The memory-safety surface is limited to one fixed 200-byte in-place state
buffer. Implementations MUST validate that the range is both readable and
writable before any state byte is written.

Keccak-f[1600] has fixed control flow and is naturally implementable with
bitwise operations. Validator implementations SHOULD avoid data-dependent
branches or memory access patterns based on state contents because some callers
may use the permutation while processing secret signing, PRF, or transcript
state.

FIPS 202 approves Keccak-p[1600,24] in the context of approved modes of
operation such as the SHA-3 functions. Exposing the permutation as a syscall
does not by itself certify arbitrary caller-defined modes.

## Backwards Compatibility

This is an additive change gated behind a feature flag. Programs that do not
invoke `sol_keccak_f1600` are unaffected. Existing syscalls are unchanged.

## Conformance

Implementations MUST include cross-client conformance tests for:

- A pinned Keccak-p[1600,24] / Keccak-f[1600] known-answer corpus under
  `assets/simd-0563-keccak-f1600/`. The final accepted SIMD MUST include the
  exact source URLs, normalized vector files, and SHA-256 checksums in that
  directory. The corpus MUST include the all-zero state, single-bit lane-order
  tests, byte-order tests, high-bit lane tests, random states, and repeated
  permutation tests.
- Exact in-place mutation of the 25 little-endian lane representation where
  `state[x + 5 * y]` maps to Keccak lane `A[x,y]`.
- Rejection of null (`0x0`), unmapped non-zero, and read-only mapped `state`
  pointers.
- Rejection of `state` pointers not aligned to `KECCAK_F1600_STATE_ALIGN`.
- Rejection when the state range is not fully readable and writable.
- Exact expected CU charges for successful calls and for validation failures
  after `keccak_f1600_cost` has been deducted. With the proposed cost,
  successful calls and post-debit validation failures consume exactly 130 CU.
- FIPS 202 wrapper tests, implemented outside the syscall, showing that
  SHA3-224, SHA3-256, SHA3-384, SHA3-512, SHAKE128, and SHAKE256 built on
  `sol_keccak_f1600` match pinned NIST CAVP or ACVTS vectors.
- Legacy Keccak-256 wrapper tests showing that the wrapper built on
  `sol_keccak_f1600` matches the existing `sol_keccak256` syscall for the same
  byte-slice input.
- MurmurHash3_x86_32 static syscall registration using the SIMD-0178 canonical
  registry entry and collision-free dispatch with the active syscall table. The
  generated `assets/simd-xxxx-keccak-f1600/static-syscall-hash.txt` proof MUST
  be checked into the proposal before acceptance.
