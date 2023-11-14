---
simd: '0087'
title: Explicit Sign-Extension in SBPFv2
authors:
  - Liam Heeger (@CantelopePeel, @lheeger-jump)
category: Standard
type: Core
status: Draft
created: 2023-11-14
feature: TBD
---

## Summary

Add a sign extension instruction to SBPFv2 and replace implicit sign extension
in certain SBPF instructions with zero extension. 

## Motivation

Currently many 32-bit ALU instructions in SBPF perform implicit sign extension 
on inputs where it is not required or necessary.

Removing implicit sign extension simplifies the virtual machine in both JIT and
interpreted approaches and thereby improves performance.

## Alternatives Considered

NA

## New Terminology

NA

## Detailed Design

The instruction set changes proposed are two fold: 

1. Add a new instruction to SBPFv2 for explicit sign extension.
1. Change existing behavior of instructions in SBPFv2 to no longer do implicit
sign extension.

It is important to note that these changes do not affect existing programs and
only affect the new SBPFv2 instruction set architecture.

Appendix 1 contains additional functions used in the specification of 
instructions.

The new instruction is specified as follows:

---

### `SEXT64_REG` - Sign Extension 64-Bit Instruction

Opcode: `0x87`

Description: \
Sign extend the lower 32-bit value in `dst_reg` to 64-bit and store into
`dst_reg`. This instruction discards and does not consider any value in the
upper 32-bits of the `dst_reg`.

Input Constraints: \
$\mathtt{dst\_reg} \neq \mathtt{r10}$

Operation:

```
if dst_reg[31] = 0 {
  dst_reg[63:32] := 0
} else {
  dst_reg[63:32] := 1
}
```

---

The changes to existing instructions is as follows:

---

### `OR64_IMM` - Bitwise Or 64-bit Immediate

Opcode: `0x47`

Description: \
Zero-extend `imm` to 64-bits. Perform bitwise or on the zero-extended immediate
with `dst_reg`. Store this value in dst_reg.

Input Constraints: \
$\mathtt{dst\_reg} \neq \mathtt{r10}$

Operation:

```
dst_reg := dst_reg | ZeroExtend(imm)
```

---

### `AND64_IMM` - Bitwise And 64-bit Immediate

Opcode: `0x57`

Description: \
Zero-extend `imm` to 64-bits. Perform bitwise and on the zero-extended immediate
with `dst_reg`. Store this value in dst_reg.

Input Constraints: \
$\mathtt{dst\_reg} \neq \mathtt{r10}$

Operation:

```
dst_reg := dst_reg & ZeroExtend(imm)
```

---

### `XOR64_IMM` - Bitwise Xor 64-bit Immediate

Opcode: `0xA7`

Description: \
Zero-extend `imm` to 64-bits. Perform bitwise xor on the zero-extended immediate
with `dst_reg`. Store this value in dst_reg.

Input Constraints: \
$\mathtt{dst\_reg} \neq \mathtt{r10}$

Operation:

```
dst_reg := dst_reg ^ ZeroExtend(imm)
```

---

### `MOV64_IMM` - Move 64-bit Immediate

Opcode: `0xB7`

Description: \
Zero-extend `imm` to 64-bits. Perform bitwise xor on the zero-extended immediate
with `dst_reg`. Store this value in dst_reg.

Input Constraints: \
$\mathtt{dst\_reg} \neq \mathtt{r10}$

Operation:

```
dst_reg := dst_reg ^ ZeroExtend(imm)
```

---

## Impact

This proposal affects changes to Program Runtime v2 and the tooling being built
for developers. It does not affect developers directly, 

## Security Considerations

All clients must carefully implement the changes to the instruction set and
should fuzz test and cross validate the changes.

## Drawbacks

This will require work from both the compiler teams and runtime teams to 
implement within SBPFv2.

It will reuse the old NEG64 instruction opcode from SBPFv1 in SBPFv2. 

## Backwards Compatibility 

SBPFv2 is a new version of the SBPF ISA and does not require backwards 
compatibility with SBPFv1.

## Appendix 1: Functions for Instruction Operations

### Zero Extension Function

```
ZeroExtend(x: u32) -> u64 {
  y: u64 := 0
  y[31:0] := x
  return y
}
```
