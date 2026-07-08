---
simd: '0563'
title: Keccak-p[1600] Syscall
authors:
  - SK, ZZ
category: Standard
type: Core
status: Idea
created: 2026-06-15
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Introduce a `sol_keccak_p1600` syscall that applies Keccak-p[1600, `rounds`]
to a caller-provided 1600-bit state. The caller supplies the round count:
`rounds = 24` applies the full Keccak-f[1600] permutation used by SHA-3 and
SHAKE, while TurboSHAKE wrappers can request the 12-round permutation required
by their mode.

Exposing the permutation instead of one fixed mode supports SHAKE128,
SHAKE256, SHA3-224, SHA3-256, SHA3-384, SHA3-512, legacy Keccak-256, and
TurboSHAKE, as well as protocol-specific Keccak sponge or duplex constructions
with one consensus ABI.

## Motivation

The initial motivation for this proposal was SHAKE-256, but SHAKE-256 is only
one mode of the Keccak-f[1600] permutation. FIPS 202 defines the SHA-3 hash
functions and SHAKE XOFs as modes of Keccak-p[1600,24], which is equivalent to
Keccak-f[1600]:

https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.202.pdf

Newer Keccak-based XOFs use the same 1600-bit state and sponge construction
with fewer rounds. RFC 9861 defines TurboSHAKE128, TurboSHAKE256, KT128, and
KT256 on top of Keccak-p[1600,12]:

https://www.rfc-editor.org/rfc/rfc9861.html

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
SHAKE128, SHA3-512, legacy Keccak-256 compatibility, TurboSHAKE,
cSHAKE/KMAC-style modes, and custom transcript sponges to software
implementations. Defining the syscall at the permutation boundary keeps the
runtime ABI small while letting libraries implement the full Keccak family.

## New Terminology

Keccak-p[1600, `rounds`]: The Keccak permutation over a 1600-bit state using
the last `rounds` rounds of the Keccak-p round sequence specified by FIPS 202.
This proposal supports only `rounds = 12` and `rounds = 24`.

Keccak-f[1600]: The 24-round permutation over a 1600-bit Keccak state. In FIPS
202 terminology, Keccak-p[1600,24] is equivalent to Keccak-f[1600].

TurboSHAKE: A family of XOFs defined by RFC 9861 using Keccak-p[1600,12] with
mode-specific rate, padding, and domain-separation rules.

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
void sol_keccak_p1600(uint64_t *state, uint64_t rounds);
```

`state` points to exactly 25 little-endian `uint64_t` lanes, for 200 total
bytes. The lane at `state[x + 5 * y]` is the Keccak lane `A[x,y]`, with
`0 <= x < 5` and `0 <= y < 5`. Validator clients MUST decode and encode these
lanes as little-endian values regardless of host architecture.

`rounds` is the number of Keccak-p rounds to apply. It MUST be either
`KECCAK_P1600_TURBO_ROUNDS` or `KECCAK_P1600_FULL_ROUNDS`. Each syscall
invocation applies Keccak-p[1600, `rounds`] to the current state exactly once,
using the FIPS 202 round constants and ordering for the last `rounds` rounds.
The final state is written back to the same memory range.

For `rounds = 24`, the syscall applies Keccak-p[1600,24], equivalent to
Keccak-f[1600]. For `rounds = 12`, it applies the reduced-round permutation
used by TurboSHAKE and KangarooTwelve.

The syscall does not implement any sponge mode. In particular, it does not:

- absorb input bytes,
- apply `pad10*1`,
- apply SHA3, SHAKE, TurboSHAKE, legacy Keccak, cSHAKE, KMAC, STROBE, or
  protocol-specific domain-separation suffixes,
- enforce a rate or capacity,
- squeeze output bytes.

Those operations are caller or SDK responsibilities. For example, a SHAKE256
wrapper uses a 136-byte rate, the SHAKE domain-separation suffix, FIPS 202
padding, `rounds = 24`, and this syscall for every required permutation. A
TurboSHAKE256 wrapper uses a 136-byte rate, the TurboSHAKE
domain-separation byte and padding from RFC 9861, `rounds = 12`, and this
syscall for every required permutation.

The syscall uses these consensus constants:

```text
KECCAK_P1600_STATE_BYTES = 200
KECCAK_P1600_STATE_LANES = 25
KECCAK_P1600_LANE_BYTES = 8
KECCAK_P1600_STATE_ALIGN = 8
KECCAK_P1600_TURBO_ROUNDS = 12
KECCAK_P1600_FULL_ROUNDS = 24
```

These constants are part of the consensus ABI. Changing any of them requires a
separate feature-gated consensus change.

The syscall aborts the virtual machine if any of these conditions are true:

- `state` is not aligned to `KECCAK_P1600_STATE_ALIGN`.
- `rounds` is neither `KECCAK_P1600_TURBO_ROUNDS` nor
  `KECCAK_P1600_FULL_ROUNDS`.
- Not all bytes in `[state, state + KECCAK_P1600_STATE_BYTES)` are readable and
  writable.
- Any pointer and length arithmetic overflows, including the checked range end
  calculation for `[state, state + KECCAK_P1600_STATE_BYTES)`.
- Compute budget is exceeded.

### Static Syscall Registration

For SBPFv3 programs, `sol_keccak_p1600` MUST follow SIMD-0178 static syscall
registration. This proposal does not define a per-syscall hash seed or override
the global static-syscall hash algorithm. The static syscall hash MUST be the
MurmurHash3_x86_32 value assigned by the SIMD-0178 canonical syscall registry
for the UTF-8 bytes of `sol_keccak_p1600`.

If SIMD-0178 does not define a canonical registry algorithm and seed before
activation, this proposal depends on a prerequisite SIMD-0178 amendment. This
proposal MUST NOT introduce a local static-syscall hash seed.

The activation PR MUST include
`assets/simd-xxxx-keccak-p1600/static-syscall-hash.txt` with the hash
algorithm, seed, computed hash, active registered syscall hashes, and collision
result. Every validator client MUST reproduce this file in CI and prove that
the registered value does not collide with any active syscall hash.

### Compute Unit Usage

Compute costs are based on the requested number of Keccak-p[1600] rounds per
syscall invocation. A 24-round invocation remains the unit that MUST be
benchmarked against full Keccak-f[1600] implementations, and 12-round
invocations MUST be benchmarked to verify the reduced-round cost.

Successful calls are charged:

```text
keccak_p1600_base_cost = 10
keccak_p1600_round_cost = 5
keccak_p1600_cost(rounds) =
    keccak_p1600_base_cost + rounds * keccak_p1600_round_cost
```

With these proposed values, `rounds = 24` costs 130 CU and `rounds = 12` costs
70 CU.

`keccak_p1600_base_cost` covers fixed syscall-specific work, including
validation and translation of the 200-byte state range. `keccak_p1600_round_cost`
covers one Keccak-p[1600] round. The activated parameters MUST be greater than
zero, and the 24-round cost MUST remain greater than the 12-round cost.

Higher-level SHAKE, SHA3, TurboSHAKE, Keccak-256, or transcript wrappers
determine their total syscall cost by the number of Keccak-p[1600] permutations
and rounds per permutation required by the mode. For example, a wrapper
generally invokes this syscall once for each padded absorb block and once
whenever squeezing must advance to the next rate block. The exact number is
mode-specific and remains outside the syscall ABI.

These parameters SHOULD be selected so that callers cannot obtain many native
Keccak-p[1600] rounds more cheaply through this syscall than through other
runtime-metered work with comparable validator cost.

### Validation and Metering Order

Implementations MUST validate and meter calls in this order:

1. Validate that `rounds` is either `KECCAK_P1600_TURBO_ROUNDS` or
   `KECCAK_P1600_FULL_ROUNDS`. If not, the syscall MUST abort.
2. Deduct `keccak_p1600_cost(rounds)`. If the caller has insufficient remaining
   CUs, the syscall MUST abort.
3. Validate `state` alignment.
4. Translate `[state, state + KECCAK_P1600_STATE_BYTES)` as a readable and
   writable VM memory range. The syscall MUST abort if the range end
   calculation overflows or if the range is not fully readable and writable.
5. Read the state, apply Keccak-p[1600, `rounds`] exactly once, and write the
   final state back to the same range.

If validation fails after step 2, the full `keccak_p1600_cost(rounds)` remains
consumed.

This section defines Keccak-p[1600]-specific metering. Any VM-wide syscall
dispatch cost that applies uniformly to all syscalls MUST be charged
identically on every `sol_keccak_p1600` invocation and on every abort path. No
additional Keccak-p[1600]-specific cost may be charged outside the debit above.

The implementation MUST NOT write any state byte until the input state has been
fully translated and read.

## Alternatives Considered

### SHAKE-256 Syscall

The runtime could expose `sol_shake256` with an explicit output length. This is
more ergonomic for Falcon and other SHAKE256-only callers, but it does not help
protocols that need SHAKE128, SHA3-512, TurboSHAKE, legacy Keccak-256,
cSHAKE/KMAC-style customization, or STROBE/Merlin-style transcript framing.

### Fixed Family Syscalls

The runtime could expose separate helpers such as `sol_shake128`,
`sol_shake256`, `sol_turboshake256`, `sol_sha3_512`, and
`sol_keccak256_xof`. This would make each common mode easier to call, but it
would expand the syscall ABI and still fail to cover custom sponge and duplex
constructions.

### BPF Implementation

Programs can implement Keccak-p[1600] in BPF today, but at a higher CU cost.
This also forces each program to carry its own implementation of a common
cryptographic primitive and makes it harder for libraries to share one tested
implementation.

### Status Quo

Continue without exposing Keccak-p[1600]. Programs requiring SHAKE,
TurboSHAKE, SHA3-family hashes, or custom Keccak sponge transcripts remain
unable to access the shared expensive primitive at syscall cost.

## Impact

Programs gain access to the native Keccak-p[1600] permutation at syscall cost.
SDKs can add mode-specific wrappers for SHAKE128, SHAKE256, TurboSHAKE128,
TurboSHAKE256, SHA3-224, SHA3-256, SHA3-384, SHA3-512, legacy Keccak-256, and
protocol-specific transcripts without requiring new runtime syscalls.

Existing `sol_keccak256` behavior is unchanged.

## Security Considerations

This syscall exposes a raw permutation, not a hash function. Callers MUST use a
correct sponge construction for their protocol, including the intended rate,
capacity, padding rule, and domain-separation suffix. A SHA3 wrapper, a SHAKE
wrapper, a TurboSHAKE wrapper, legacy Keccak-256, and a STROBE transcript all
use different framing rules even though they share the same 1600-bit state.

Implementations MUST implement Keccak-p[1600, `rounds`] exactly as specified by
FIPS 202, including the FIPS 202 definition that Keccak-p[1600, `rounds`] uses
the last `rounds` rounds from the 24-round sequence. They MUST NOT silently
force `rounds = 24`, silently substitute a different reduced-round schedule, or
use a hash-mode-specific implementation whose behavior is only correct for one
rate or suffix.

Reduced-round permutations are not interchangeable with full-round SHA-3 or
SHAKE. Callers MUST use `rounds = 24` for FIPS 202 SHA3 and SHAKE wrappers and
MUST use `rounds = 12` for RFC 9861 TurboSHAKE and KangarooTwelve wrappers.
No other round counts are accepted by this syscall.

The memory-safety surface is limited to one fixed 200-byte in-place state
buffer. Implementations MUST validate that the range is both readable and
writable before any state byte is written.

Keccak-p[1600] has fixed control flow for a fixed `rounds` value and is
naturally implementable with bitwise operations. Branching on the public
`rounds` value is allowed. Validator implementations SHOULD avoid
data-dependent branches or memory access patterns based on state contents
because some callers may use the permutation while processing secret signing,
PRF, or transcript state.

FIPS 202 approves Keccak-p[1600,24] in the context of approved modes of
operation such as the SHA-3 functions. RFC 9861 defines TurboSHAKE and
KangarooTwelve on top of Keccak-p[1600,12]. Exposing the permutation as a
syscall does not by itself certify arbitrary caller-defined modes.

## Backwards Compatibility

This is an additive change gated behind a feature flag. Programs that do not
invoke `sol_keccak_p1600` are unaffected. Existing syscalls are unchanged.

## Conformance

Implementations MUST include cross-client conformance tests for:

- A pinned Keccak-p[1600,24] / Keccak-f[1600] known-answer corpus under
  `assets/simd-0563-keccak-p1600/`. The final accepted SIMD MUST include the
  exact source URLs, normalized vector files, and SHA-256 checksums in that
  directory. The corpus MUST include the all-zero state, single-bit lane-order
  tests, byte-order tests, high-bit lane tests, random states, and repeated
  permutation tests.
- A pinned Keccak-p[1600,12] known-answer corpus under the same directory,
  including all-zero state, random states, and repeated permutation tests for
  TurboSHAKE-compatible round scheduling.
- Exact in-place mutation of the 25 little-endian lane representation where
  `state[x + 5 * y]` maps to Keccak lane `A[x,y]`.
- Rejection of null (`0x0`), unmapped non-zero, and read-only mapped `state`
  pointers.
- Rejection of `state` pointers not aligned to `KECCAK_P1600_STATE_ALIGN`.
- Rejection of any `rounds` value other than `KECCAK_P1600_TURBO_ROUNDS` and
  `KECCAK_P1600_FULL_ROUNDS`, including `0`, `1`, `13`, `23`, and `25`.
- Rejection when the state range is not fully readable and writable.
- Exact expected CU charges for successful calls and for validation failures
  after `keccak_p1600_cost(rounds)` has been deducted. With the proposed cost,
  successful 24-round calls and post-debit validation failures consume exactly
  130 CU, and successful 12-round calls and post-debit validation failures
  consume exactly 70 CU.
- FIPS 202 wrapper tests, implemented outside the syscall, showing that
  SHA3-224, SHA3-256, SHA3-384, SHA3-512, SHAKE128, and SHAKE256 built on
  `sol_keccak_p1600` with `rounds = 24` match pinned NIST CAVP or ACVTS
  vectors.
- RFC 9861 wrapper tests, implemented outside the syscall, showing that
  TurboSHAKE128, TurboSHAKE256, KT128, and KT256 built on `sol_keccak_p1600`
  with `rounds = 12` match pinned RFC vectors.
- Legacy Keccak-256 wrapper tests showing that the wrapper built on
  `sol_keccak_p1600` with `rounds = 24` matches the existing `sol_keccak256`
  syscall for the same byte-slice input.
- MurmurHash3_x86_32 static syscall registration using the SIMD-0178 canonical
  registry entry and collision-free dispatch with the active syscall table. The
  generated `assets/simd-xxxx-keccak-p1600/static-syscall-hash.txt` proof MUST
  be checked into the proposal before acceptance.
