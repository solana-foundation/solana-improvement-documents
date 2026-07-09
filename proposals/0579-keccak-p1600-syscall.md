---
simd: '0579'
title: Keccak-p[1600] Syscall
authors:
  - David Rubin (Jump), SK, ZZ
category: Standard
type: Core
status: Idea
created: 2026-07-98
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Introduces a `sol_keccak_p1600` syscall that applies $Keccak\-p[1600, rounds]$
to a caller-provided 1600-bit (200 byte) state. The caller may chose between
either 24 rounds, or 12 rounds, allowing for a wider range of supported
usecases, such as SHAKE, TurboSHAKE, and Sponge/Duplex constructions.

## Motivation

The initial motivation for this proposal was for using SHAKE-256 in on-chain
programs, and was further expanded to support newer Keccak-based constructions
that use fewer rounds. RFC 9861 defines TurboSHAKE128, TurboSHAKE256, KT128, 
and KT256 on top of $Keccak-p[1600,12]$:

https://www.rfc-editor.org/rfc/rfc9861.html

The Keccak team feels confident that halving the round count of their core
permutation does not impact the security guarantees of it.

Many protocols use custom framing and their own domain seperation, so
we expose the commonly shared core permutation. This allows for common
constructions such as SHAKE256 to be cheaply implemented in BPF, i.e.:

```py
def append(state, data, sz):
  cursor = last_padding
  for i in range(sz):
    state[cursor] ^= data[i]
    cursor += 1
    if (cursor == RATE):
      sol_keccak_p1600(state)
      cursor = 0
  last_padding = cursor
```

as predictable linear memory accesses and XORs are cheap.

Protocols such as [Merlin](https://merlin.cool/), that have different bit-flags
and seperators, also use the core $Keccak\-p[1600, rounds]$ function and 
would benefit from this syscall.

## New Terminology

$Keccak\-p[1600, rounds]$: The Keccak permutation over a 1600-bit (200 byte)
state using the first `rounds` round constants specified in FIPS 202.

## Detailed Design

### Syscall Signature

The syscall mutates one Keccak state in place:

```rust
define_syscall!(fn sol_keccak_p1600(
    state: *mut u64,
    rounds: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
) -> u64)
```

`state` points at 25 little-endian `u64` words, for a total of 200 bytes.
Given `x` and `y` within $[0, 5)$, `state[x + 5 * y]` is the Keccak lane
`A[x,y]`.

`rounds` is the number of rounds to apply. It MUST be either `12` or `24`.
Each syscall invocation applies $Keccak\-p[1600, rounds]$ to the provided
state once, with the after-state written back to the same memory object.

Returns 0 in all cases.

### Compute Unit Usage

We define the `base_cost` to be `10` and the `round_cost` to be `5`.

We describe the compute cost of one innvocation as:

```
cost(rounds) = base_cost + (rounds * round_cost)
```

with these proposed values, `rounds = 24` costs `130` CUs and `rounds = 12`
costs `70` CUs.

### Validation Behaviour

An implementation MUST validate and meter calls in this order. If any
condition is not met, the virtual machine is aborted.

1. Validate that `rounds` is not either `12` or `24`.
2. Deduct `cost(rounds)` CUs from the budget. If the caller has insufficient
remaining CUs, the virtual machine is aborted.
3. Validate that `state` is aligned to `8`.
4. Translate `[state, state + 200)` as a writable memory region. If any
computation overflows the syscall MUST abort.
5. Applies $Keccak\-p[1600, rounds]$ to the state and writes it back to
the aforementioned writable memory region.

## Alternatives Considered

### SHAKE-256 Syscall

The runtime could expose a `sol_shake256` syscall which would incorporate
the rate and domain seperation, however this would prevent other protocols
from re-using the underlying core permutation.

### BPF Implementation

Programs can implement $Keccak\-p[1600, rounds]$ in BPF today, but at a
higher CU cost. This also forces each program to carry its own implementation
of a common cryptographic primitive, and makes it harder for libraries to
share one well-tested implementation.

## Impact

Programs gain access to a faster and better tested $Keccak\-p[1600, rounds]$
that can be re-used across many modern constructions and protocols.

Existing `sol_keccak256` behavior is unchanged.

## Security Considerations

As this syscall exposes the underlying permutation, a caller must make
sure to correctly implement the domain seperation and tagging that is
required of the construction they are implementing.

The API `sol_keccak_p1600` exposes is safe and obvious to use.

## Backwards Compatibility

This is an additive change gated behind a feature flag. Programs that do not
invoke `sol_keccak_p1600` are unaffected. Existing syscalls are unchanged.

## Conformance

Each validator implementation MUST include tests or fixtures that demonstrate:

- Correct usage with known-answer-tests, that will be provided with the
final accepted SIMD.
- Rejection when `rounds` is not either `12` or `24`.
- Rejection when `state` is not aligned to `8`.
- Rejection when `state` is an invalid address, as described earlier.
- Rejection when there are not enough CUs left to fulfill the syscall.
