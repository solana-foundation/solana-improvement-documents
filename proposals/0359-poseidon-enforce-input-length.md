---
simd: '0359'
title: Poseidon Syscall - Enforce Input Length
authors:
  - Michal Rostecki
category: Standard
type: Core
status: Idea
created: 2025-09-22
feature:
supersedes:
superseded-by:
extends:
---

## Summary

Require all Poseidon hash inputs to have a fixed length determined by the
number of bytes used by the modulus of a prime field.

For the (currently the only supported) prime field BN254, the modulus is:

$$
p = 21888242871839275222246405745257275088548364400416034343698204186575808495617
$$

The byte representation of that modulus takes 32 bytes:

```
[
  1, 0, 0, 240, 147, 245, 225, 67, 145, 112, 185, 121, 72, 232, 51, 40, 93, 88,
  129, 129, 182, 69, 80, 184, 41, 160, 49, 225, 114, 78, 100, 48
]
```

That means there is no element in BN254 prime field that has byte
representation larger than 32 bytes.

As of today, Poseidon syscall accepts byte slices with length $$n$$, where:

$$
0 \leq n \leq bytelen(p)
$$

The goal of this change is to restrict it to:

$$
n = bytelen(p)
$$

In case of inputs with byte representations having fewer bytes than modulus,
that means a necessity to add explicit padding. For example, the following big
number:

```
115792089237316195423570985008687907853269984665640564039439137263839420088320
```

Which can be represented by these 24 bytes:

```
[
  255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
  255, 255, 255, 255, 255, 255, 255, 255, 255
]
```

When used as a little-endian input for Poseidon on BN254 prime field, will have
to be extended to the following 32 byte representation:

```
[
  255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
  255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0
]
```

## Motivation

The current behavior allowing people to submit slices of different lengths is
confusing and makes the implementation of the hasher less straightforward.

## Alternatives Considered

Leave as it is.

## New Terminology

n/a

## Detailed Design

This exact change is already implemented by
[light-poseidon 0.4.0][light-poseidon].

Validator implementations need to start enforcing the byte length check and
return an error code if length of any of the inputs is different than the byte
length of the modulus of currently used elliptic curve.

[light-poseidon]: https://github.com/Lightprotocol/light-poseidon/releases/tag/v0.4.0

## Impact

It's a consensus breaking change, therefore it needs to be guarded with a
feature flag, implemented by all validator implementations and activated once
all of them are ready.

## Security Considerations

n/a

## Backwards Compatibility

The feature breaks backward compatibility, therefore it needs to be introduced
with a feature gate.

Without the feature enabled, validators must honor the old behavior, where
smaller inputs are accepted and still correctly serialized as big integers.
