---
simd: '0258'
title: Signatures sysvar for signature introspection
authors:
  - Ahmad Abbasi (Syndica)
category: Standard
type: Core
status: Review
created: 2025-03-14

---

## Summary

Allow the ability to introspect a transaction's signatures within an
instruction via a Signatures sysvar.

## Motivation

As we develop the [Sig validator](https://github.com/Syndica/sig), we've
experimented with novel ways an on-chain protocol can be designed. Many of
those designs require some source of sequencing. Along with the need for a
sequencing mechanism, the ability to query previous sequence "entries" is
also important. Today, the only viable way to accomplish retrieving previous
sequence entries is to use `getSignaturesForAddress` RPC and then parse through
transactions individually to find the ones which create the next/previous
"entry." If you have access to a transaction's signature within an
instruction, you're able to save that into an account which can be used down
the line to retrieve past entries.

## New Terminology

- Signatures sysvar: A Solana runtime sysvar which provides the ability to read
a transaction's signatures within an instruction.

## Detailed Design

In order to introspect transaction signatures within an instruction, a
transaction MUST include the Signatures sysvar
`SysvarSignatures111111111111111111111111111` in the accounts list. The runtime
will recognize this special sysvar, pass the raw transaction signatures to the
instruction so that the bpf program can extract them and use as needed.

A helper function to extract the signatures can be used:

```rust
fn load_signatures(
        signatures_sysvar_account_info: &AccountInfo
    ) -> Result<&[Signature]>;
```

## Alternatives Considered

Currently, no alternative exists to introspect a transaction's signature
from within an instruction.

## Impact

The impact of this will allow for unique and bespoke protocols to emerge on
Solana enabling a new subset of sequence-based and hybrid (on/off-chain)
applications to exist.

## Security Considerations

Because Sysvar(s) are inherently read-only, no major security concerns would
arise from this feature. We already allow instruction data introspection via
`Sysvar1nstructions1111111111111111111111111` sysvar.
