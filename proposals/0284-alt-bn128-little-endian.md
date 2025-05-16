---
simd: '0284'
title: Alt-BN128 Little Endian compatibility
authors:
  - Dean Little - Blueshift
category: Standard
type: Core/Networking/Interface/Meta
status: Idea
created: 2025-05-15
feature: 
---

## Summary

Extend the Alt-BN128 syscalls to add little endian support.

## Motivation

All prominent ZK teams on Solana primarily use Rust with ark-bn254 in their
workflows; the same crate used in the Alt-BN128 syscalls implementation in 
agave. The original implementor designed the syscalls to mimic the 
functionality of Ethereum which encodes uint256 values in Big Endian. 

This creates an unnecessarily complicated API for anyone trying to build zero
knowledge proofs on Solana leading to many wasted hours of debugging endianness
and encoding issues. As it's relatively easy to fix, we should do something
about it.

## New Terminology

N/A

## Detailed Design

There are two Alt-BN128 syscalls: 

`sol_alt_bn128_group_op` and `sol_alt_bn128_compression`. 

Each takes in an argument to determine which operation they are performing as
their first parameter.

In the case of `sol_alt_bn128_group_op`:

```rust
pub const ALT_BN128_ADD: u64 = 0;
pub const ALT_BN128_SUB: u64 = 1;
pub const ALT_BN128_MUL: u64 = 2;
pub const ALT_BN128_PAIRING: u64 = 3;
```

In the case of `sol_alt_bn128_compression`:

```rust
pub const ALT_BN128_G1_COMPRESS: u64 = 0;
pub const ALT_BN128_G1_DECOMPRESS: u64 = 1;
pub const ALT_BN128_G2_COMPRESS: u64 = 2;
pub const ALT_BN128_G2_DECOMPRESS: u64 = 3;
```

This SIMD proposes we include four new values for each of these syscalls with a
bitmask of `0x80` to signal their little endian equivalents:

```rust
pub const ALT_BN128_ADD_LE: u64 = ALT_BN128_ADD | 0x80;
pub const ALT_BN128_SUB_LE: u64 = ALT_BN128_SUB | 0x80;
pub const ALT_BN128_MUL_LE: u64 = ALT_BN128_MUL | 0x80;
pub const ALT_BN128_PAIRING_LE: u64 = ALT_BN128_PAIRING | 0x80;
```

In the case of `sol_alt_bn128_compression`

```rust
pub const ALT_BN128_G1_COMPRESS_LE: u64 = ALT_BN128_G1_COMPRESS | 0x80;
pub const ALT_BN128_G1_DECOMPRESS_LE: u64 = ALT_BN128_G1_DECOMPRESS | 0x80;
pub const ALT_BN128_G2_COMPRESS_LE: u64 = ALT_BN128_G2_COMPRESS | 0x80;
pub const ALT_BN128_G2_DECOMPRESS_LE: u64 = ALT_BN128_G2_DECOMPRESS | 0x80;
```

These options could then be added to the relevant SDKs (solana-bn254), 
potentially with an optional feature flag to enable/disable big and little
endian variants.

## Alternatives Considered

Overhaul Rust-based ZK tooling itself to become more Ethereum-compatible.

## Impact

Working with ZK proofs will become much easier, as the most widely-used 
tooling and the system API will finally be compatible without any additional
work or confusion.

## Security Considerations

None
