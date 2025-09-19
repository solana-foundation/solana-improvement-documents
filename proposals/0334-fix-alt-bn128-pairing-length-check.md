---
simd: "0334"
title: Fix alt_bn128_pairing syscall length check
authors:
  - Stanislav Ladyzhenskiy
category: Standard
type: Core
status: Review
created: 2025-08-11
feature:
supersedes:
superseded-by:
extends:
---

## Summary

The `alt_bn128_pairing` syscall takes a byte slice as input,
interprets the bytes as an array of pairs of g1 and g2 points on bn128 elliptic
curve, and applies a pairing operation. If the byte slice input has an improper
length, the function should terminate early. Specifically, if the byte slice
length is not a multiple of 192 (the sum of the lengths of g1 and g2 points),
the function should terminate early with an error.

However, the current code does not perform this check correctly.

This document proposes to fix this length check by checking for the correct
length.

## Motivation

The `alt_bn128_pairing` function still works with the incorrect length check
because it only processes multiples of 192 bytes and discards the rest.
However, there could be successful inputs that are not multiples of 192.
This could make the application logic harder to debug.

## Alternatives Considered

Leave as is.

## New Terminology

N/A

## Detailed Design

Currently, the code checks than `checked_rem` of the input length and
`ALT_BN128_PAIRING_ELEMENT_LEN` (which is 192) is not `None`.
However, `checked_rem` returns `None` when the rhs is 0,
which never happens in this context.

```rust
pub fn alt_bn128_pairing(input: &[u8]) -> Result<Vec<u8>, AltBn128Error> {
    if input
        .len()
        .checked_rem(consts::ALT_BN128_PAIRING_ELEMENT_LEN)
        .is_none()
    {
        return Err(AltBn128Error::InvalidInputData);
    }

    // logic omitted...
}
```

The correct logic should check that the reminder is 0.

```rust
pub fn alt_bn128_pairing(input: &[u8]) -> Result<Vec<u8>, AltBn128Error> {
    if input.len() % ALT_BN128_PAIRING_ELEMENT_LEN != 0 {
        return Err(AltBn128Error::InvalidInputData);
    }

    // logic omitted...
}
```

## Impact

This fix will prevent accidental misuse of the `alt_bn128_pairing`
syscall function and make programs easier to debug.

## Security Considerations

This does update the behavior of the syscall function and therefore should be
properly feature-gated.

## Drawbacks _(Optional)_

None
