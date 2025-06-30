---
simd: '0166'
title: SBPF Dynamic stack frames
authors:
  - Alexander Meißner
  - Alessandro Decina
  - Lucas Steuernagel
category: Standard
type: Core
status: Implemented
created: 2024-08-19T00:00:00.000Z
feature: JE86WkYvTrzW8HgNmrHY7dFYpCmSptUpKupbo2AdQ9cG
supersedes: null
superseded-by: null
extends: null
---

## Summary

The SVM currently allocates a fixed amount of stack space to each function 
frame. We propose allowing programs to dynamically manage their stack space 
through the introduction of an explicit stack pointer.

## Motivation

The SVM allocates a fixed amount of memory to hold a program’s stack. Within 
the stack region, the virtual machine reserves 4096 bytes of stack space for 
each function frame. This is simultaneously limiting for functions that 
require more space, and wasteful for functions that require less space.

For well optimized programs that don’t allocate large amounts of stack, the 
virtual machine currently still reserves 4096 bytes of stack for each function 
call, leading to suboptimal memory usage, which may cause unnecessary page 
faults.

On the other hand, some programs are known to create large function frames - 
this seems common with programs that serialize a lot of data - and they have 
to jump through hoops to avoid overflowing the stack. The virtual machine 
detects when a stack overflow occurs, and it does so by implementing a stack 
frame gaps system whereby it inserts a virtual sentinel frame following a 
valid function frame. If the sentinel frame is accessed, the executing program 
is aborted. This system is fragile and is incompatible with direct mapping - a 
feature we expect to enable soon.

The changes proposed in this document would allow us to optimize stack memory 
usage and remove the fragile stack frame gaps system. Note that we do not 
propose to remove the existing maximum stack space limit: stack space stays 
unchanged, what changes is how it is partitioned internally.

## Alternatives Considered

To cope with the SBF limitation of 4096 bytes for the frame size, we could 
have increased such a number. Even though this would solve the original 
problem, it would supply functions with an unnecessary amount of memory. In 
addition, such a solution would increase pressure on the total memory 
available for the call stack. Either we would need to increase the total 
allocation for the virtual machine or decrease the maximum call depth.

## New Terminology

None.

## Detailed Design

Bringing dynamic stack frames to the Solana Bytecode Format and its 
corresponding virtual machine entails changes in several aspects of the 
execution environment.

### Changes in the execution environment

We will repurpose the existing R10 register from a frame pointer to a stack 
pointer. In other words, it must stop representing the highest address 
accessible in a frame, and must now point to the lowest address in a frame.

Such a change entails a change in the direction of stack growth. Presently, we 
stack frames on top of each other, but the memory usage within them grows 
downward. In the new frame setting, both the placement of new frames and the 
memory usage inside frames must be downward.

Functions in SBF must alter the stack pointer using the `add64 reg, imm` 
(opcode `0x07`) instruction only, allowing them to request any desirable 
amount of stack space, provided that it meets the required alignment (refer to 
the following section).

The stack frame gaps feature, which creates a memory layout where frames are 
interleaved with equally sized gaps, are not compatible with dynamic stack 
frames and must be deactivated.

The VM must maintain a shadow stack to reset the stack pointer to the value 
in usage at the caller function when the callee returns via either the `exit` 
(opcode `0x95`) or the `return` instruction (opcode `0x9D`). This setting 
entails that a function does not need to manually adjust the stack pointer 
to its original value with a second `add64 reg, imm` instruction at its end.

### Stack alignment

We want to enforce that the stack pointer remains aligned, therefore R10 must 
only be incremented or decremented by a multiple of 64. Large alignments might 
seem wasteful, but enforcing a sufficiently big alignment will spark 
innovation in interpreters and JITs, ultimately leading to much better 
performance and thus lower costs.

Based on the current AVX-512 instructions available on Intel and AMD 
processors, the stack alignment must be 64 bytes. Even if current interpreters 
do not take advantage of these vectorized instructions, we believe that future 
generation interpreters might be able to vectorize SBF programs to speed up 
common operations, such as copying or comparing public keys and signatures. 
An unaligned stack prohibits such innovations.

### Changes in the verifier

The verifier must now allow R10 to be the destination register of the 
`add64 reg, imm` (opcode `0x07`) instruction.

The verifier must throw `VerifierError::UnalignedImmediate` when the immediate 
value of `add64 reg, imm` (opcode `0x07`) is not a multiple of 64 and the 
destination register is R10. The error must only be raised when both 
conditions happen simultaneously.

### Changes in code generation

In the compiler side, dynamic stack frames allow for some optimizations. 
First, when a function does not need any stack allocated variable, code 
generation must not create any instruction to modify R10. In addition, we can 
stop using R5 as a stack spill register when a function call receives more 
than five arguments. With dynamic stack frames, the compiler must use 
registers R1 to R5 for the first five arguments and place remainder arguments 
in the caller frame, easily retrieving them in the callee as an offset from 
the stack pointer. This new call convention obviates the need to use R5 for 
retrieving the caller’s frame pointer address to access those parameters.

### Identification of programs

As per the description in SIMD-0161, programs compiled with dynamic stack 
frames must contain the `0x01` flag on their ELF header `e_flags` field.

## Impact

We foresee a positive impact in smart contract development. Developers won’t 
need to worry about exceeding the maximum frame space allowed for a function 
and won’t face any case of stack access violation if their code follows 
conventional Rust safety rules. Likewise, when we update the Rust version of 
our platform tools, developers will not have the burden of modifying their 
contract just because the newer version is using more stack than the previous 
one, often reaching the 4096 bytes limit. Refer to issues 
[#1186](https://github.com/anza-xyz/agave/issues/1186) and 
[#1158](https://github.com/anza-xyz/agave/issues/1158).

We also expect some improvements in program execution. For function calls that 
handle more than five arguments, there will be one less store and one less 
load operation due to the new call convention.

## Security Considerations

Stack gaps will be disabled for dynamic stack frames to work. Stack gaps could 
detect invalid accesses between two function frames, if the accessed address 
would fall between them. With dynamic stack frames, all stack access will be 
valid, provided that their address is within the allowed range. We already 
allow functions to read and modify the memory inside the frame of other 
functions, so removing the stack gaps should not bring any security 
implications.

Although one can change R10 to almost any value that fits in a 64-bit integer 
with `add64 reg, imm`, every memory access is verified, so there is no risk of 
invalid accesses from a corrupt register.

## Drawbacks

Programs will consume negligibly more compute units, as most functions will 
include two extra instructions: one to increment the stack pointer and another 
one to decrement it.
