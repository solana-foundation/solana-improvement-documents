---
simd: '0166'
title: Dynamic stack frames in SBF
authors:
  - Alexander Meißner
  - Alessandro Decina
  - Lucas Steuernagel
category: Standard
type: Core
status: Draft
created: 2024-08-19T00:00:00.000Z
feature: null
supersedes: null
superseded-by: null
extends: null
---

## Summary

The SVM currently allocates a fixed amount of stack space to each function 
frame. We propose allowing programs to dynamically manage their stack space 
through the introduction of an explicit stack pointer register. 

## Motivation

The SVM allocates a fixed amount of memory to hold a program’s stack. Within 
the stack region, the virtual machine reserves 4096 bytes of stack space for 
each function frame. This is simultaneously limiting for functions that 
require more space, and wasteful for functions that require less space.

For well optimized programs that don’t allocate large amounts of stack, the 
virtual machine currently still reserves 4096 bytes of stack for each 
function call, leading to suboptimal memory usage, which may cause 
unnecessary page faults.

On the other hand, some programs are known to create large function frames - 
this seems common with programs that serialize a lot of data - and they have 
to jump through hoops to avoid overflowing the stack. The virtual machine 
detects when a stack overflow occurs, and it does so by implementing a stack 
frame gaps system whereby it inserts a virtual sentinel frame following a 
valid function frame. If the sentinel frame is accessed, the executing program 
is aborted. This system is fragile and is incompatible with direct mapping - 
a feature we expect to enable soon. 

The changes proposed in this document would allow us to optimize stack memory 
usage and remove the fragile stack frame gaps system. Note that we do not 
propose to remove the existing maximum stack space limit: stack space stays 
unchanged, what changes is how it is partitioned internally.

## Alternatives Considered

To cope with the SBF limitation of 4096 bytes for the frame size, we could 
have increased such a number. Even though this would solve the original 
problem, it would supply an unnecessary amount of memory to functions even 
when they do not need them. In addition, such a solution would increase 
pressure on the total memory available for the call stack. Either we would 
need to increase the total allocation for the virtual machine or decrease the 
maximum call depth.

## New Terminology

None.

## Detailed Design

Bringing dynamic stack frames to the Solana Bytecode Format and its 
corresponding virtual machine entails changes in several aspects of the 
execution environment.

### SBF architecture modifications


We will introduce a new register R11 in the virtual machine, which is going 
to hold the stack pointer. The program must only write to such a register and 
modify it through the `add64 reg, imm` (op code `0x07`) instruction. The 
verifier must enforce these constraints on deployed programs. For further 
information about the  changes in the ISA, refer to the 
[SPF spec document](https://github.com/solana-labs/rbpf/blob/main/doc/bytecode.md).

The R11 register must work in tandem with the R10 (frame pointer) register. 
The former is write-only to the program, and the latter is read-only to the 
program, forming a common design pattern in hardware engineering. More 
details of this usage are in the following section.

### Changes in the execution environment

The R10 register must continue to hold the frame pointer, but we will manage 
it differently. With fixed frames, when there is a function call we add 4096 
to R10 and subtract it when the function returns. In the new scheme, we must 
assign the value of R11 to R10 at function calls, and save R10’s former value 
so that we can restore it when the function returns.

The introduction of dynamic stack frames will change the direction of stack 
growth. Presently, we stack frames on top of each other, but the memory usage 
in them grows downward. In the new frame setting, both the placement of new 
frames and the memory usage inside frames must be downward.

The stack frame gaps feature, which creates a memory layout where frames are 
interleaved with equally sized gaps, are not compatible with dynamic stack 
frames and must be deactivated.

### Changes in code generation

In the compiler side, dynamic stack frames allow for some optimizations. 
First, when a function does not need any stack allocated variable, code 
generation must not create any instruction to modify R11. In addition, we 
can stop using R5 as a stack spill register when a function call receives 
more than five arguments. With dynamic stack frames, the compiler must use 
registers R1 to R5 for the first five arguments and place remainder arguments 
in the callee frame, instead of placing them in the caller’s frame. This new 
call convention obviates the need to use R5 for retrieving the caller’s frame 
pointer address to access those parameters.

### Identification of programs

As per the description in SIMD-0161, programs compiled with dynamic stack 
frames must contain the XX flag on their ELF header `e_flags` field.

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

We also expect some improvements in program execution. For functions with no 
stack usage, we will not emit the additional instruction that modifies R11, 
saving some execution time. Furthermore, for function calls that handle more 
than five arguments, there will be one less store and one less load operation 
due to the new call convention.

## Security Considerations

Stack gaps will be disabled for dynamic stack frames to work. Stack gaps could 
detect invalid accesses between two function frames, if the accessed address 
would fall between them. With dynamic stack frames, all stack access will be 
valid, provided that their address is within the allowed range. We already 
allow functions to read and modify the memory inside the frame of other 
functions, so removing the stack gaps should not bring any security 
implications.

Although one can change R11 to any value that fits in a 64-bit integer, every 
memory access is verified, so there is no risk of invalid accesses from a 
corrupt register.

## Drawbacks

Programs will consume more compute units, as most functions will include two 
extra instructions: one to increment the stack pointer and another one to 
decrement it.
