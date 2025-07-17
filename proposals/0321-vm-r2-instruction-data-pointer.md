---
simd: '0321'
title: VM Register 2 Instruction Data Pointer
authors:
  - Joe Caulfield (Anza)
category: Standard
type: Core
status: Review
created: 2025-07-11
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Provide a pointer to instruction data in VM register 2 (`r2`) at program
entrypoint, enabling direct access to instruction data without having to parse
the accounts section of the serialized input region.

## Motivation

Currently, sBPF programs must parse the accounts section of the serialized
input region to locate instruction data. The serialization layout places
accounts before instruction data, requiring programs to iterate through all
accounts before
reaching the instruction data section. This is inefficient for programs that
primarily or exclusively need to access instruction data.

By providing a direct pointer to instruction data in `r2`, programs can
immediately access this data without any parsing overhead, resulting in
improved performance and reduced compute unit consumption.

## New Terminology

* **Instruction data pointer**: A 64-bit pointer (8 bytes) stored in VM
  register 2 that points directly to the start of the instruction data
  section in the input region.

## Detailed Design

When the feature is activated, the VM shall set register 2 (`r2`) to contain a
pointer to the beginning of the instruction data section within the input
region. The instruction data format remains unchanged:

```
[8 bytes: data length (little-endian)][N bytes: instruction data]
```

This pointer in `r2` is made available to all programs, under all loaders,
regardless of whether or not the value is read. Prior to this feature, `r2`
contains uninitialized data at program entrypoint.

Despite technically being a breaking change, mainnet-beta testing with a modified
Agave validator confirms no divergence in execution or consensus. This is
because `r2` can typically only be accessed uninitialized through contrived
examples such as assembly manipulation or compiler bugs. The performance
benefits are considered a reasonable tradeoff. See security section for more
details.

**Register Assignment:**

* `r1`: Input region pointer (existing behavior)
* `r2`: Pointer to instruction data section (new)

**Pointer Details:**

* The pointer in `r2` points to the first byte of the actual instruction data,
  NOT the length field.
* The pointer value in `r2` is stored as a native 64-bit pointer (8 bytes) in
  little-endian format.
* When there is no instruction data (length = 0), `r2` still points to the
  offset immediately proceeding the instruction length counter; in this case,
  the first byte of the program ID, ensuring it will always point to valid,
  readable memory within the bounds of the input region.
* The pointer must always point to valid memory within the input region bounds.

## Alternatives Considered

1. **Provide a pointer to instruction data length**: Store a pointer to the
   instruction data length field in `r2`. However, providing a direct pointer to
   the start of instruction data is more ergonomic.

2. **Provide optional entrypoint parameter**: Allow programs to opt-in via a
   different entrypoint signature. The current approach is simpler as it avoids
   supporting multiple entrypoint signatures and makes the pointer universally
   available. This relies on the assumption that no programs depend on the
   garbage value previously in `r2`.

3. **Modify serialization layout**: The serialization layout will eventually be
   overhauled with ABI v2, a comprehensive upgrade that could resolve this issue
   among many others. Given the significant scope of ABI v2 and potential for
   delays, this targeted optimization provides immediate value and remains
   compatible with ABI v2.

## Impact

On-chain programs are positively impacted by this change. The new `r2` pointer
gives programs the ability to efficiently read instruction data, further
customize their program's control flow and maximize compute unit effiency.
However, any programs that currently depend on the uninitialized/garbage value
in `r2` at entrypoint will break when this feature is activated.

Core contributors must implement this feature, which should be extremely
minimally invasive, depending on the VM implementation.

## Security Considerations

Programs should read and validate the instruction data length (stored at `r2 - 8`)
before accessing data via the `r2` pointer. Failing to check the length could
result in reading unintended memory contents or out-of-bounds access attempts.

Additionally, programs that currently rely on `r2` containing uninitialized or
garbage data at entrypoint will experience breaking changes when this feature
is activated. While it is technically possible with assembly manipulations, no
compiled code uses `r2` with an uninitialized value except in the case of
`sol_log_64_` which is not a direct security concern as logs are not enshrined
by consensus.

## Backwards Compatibility

This feature is only backwards compatible for programs that currently do not
read from `r2` at program entrypoint.

This feature is NOT backwards compatible for any programs that depend on the
uninitialized/garbage data previously in `r2`.
