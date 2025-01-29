---
simd: "0222"
title: Fix alt-bn128 multiplication syscall length
authors:
  - Sam Kim
category: Standard
type: Core
status: Implemented
created: 2025-01-10
feature: bn2puAyxUx6JUabAxYdKdJ5QHbNNmKw8dCGuGCyRrFN
supersedes:
superseded-by:
extends:
---

## Summary

The `alt_bn128_multiplication` syscall takes in a byte slice as input,
interprets the bytes as a bn128 elliptic curve point/scalar pair, and applies
point-scalar multiplication. If the byte slice input has improper length then
the function terminates early. Specifically, if the byte slice has length
greater than 128, then the function terminates early with an error.

However, a bn128 curve point is 64 bytes and a scalar is 32 bytes. This means
that the function should check if the byte slice is 96 bytes in length rather
than 128 bytes.

This document proposes to fix this length check by checking for the correct
length.

## Motivation

The `alt_bn128_multiplication` function still works with the incorrect 128
length bound since a correct input of 96 bytes is still less than 128 bytes.
However, there could be successful inputs that are greater than 96 bytes and
smaller than 128 bytes in length. This could cause application logic harder to
debug.

## Alternatives Considered

Leave as is.

## New Terminology

N/A

## Detailed Design

The fix is simple.

Currently, the constant `ALT_BN128_MULTIPLICATION_INPUT_LEN`, which is set to
128 is used to sanity check the length of the input.

```rust

pub fn alt_bn128_multiplication(input: &[u8]) -> Result<Vec<u8>, AltBn128Error> {
    if input.len() > ALT_BN128_MULTIPLICATION_INPUT_LEN {
        return Err(AltBn128Error::InvalidInputData);

    // logic omitted...
}
```

A fix would entail updating the `ALT_BN128_MULTIPLICATION_INPUT_LEN` constant to
the correct length of 96.

## Impact

This fix will prevent accidental misuse of the `alt_bn128_multiplication`
syscall function and make programs easier to debug.

## Security Considerations

This does update the behavior of the syscall function and therefore, should be
properly feature-gated.

## Drawbacks _(Optional)_

None
