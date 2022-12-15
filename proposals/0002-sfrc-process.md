---
# SFRC: '#0001'

title: Universal IDL Program

authors:
  - Noah Gundotra
  - Armani Ferrante
  - Jeremy P Bogle
  - more to come! you, possibly

category: `Standard`
type: `Program`
status: `Draft`
created: `2022-12-15`
feature: (fill in with feature tracking issues once accepted)

---

## Summary

A new Solana Program that allows developers to publish and manage on-chain IDLs for solana programs.

## Motivation

Why are we doing this? What use cases does it support? What is the expected
outcome?

Most programs on Solana lack information on how to interact and compose with them. Specifically, over 90% of programs on mainnet-beta lack IDLs. This is partly because it is very hard for native programs to deploy Anchor IDLs, because doing so requires adding instructions to their program to write to their IDL to an account owned by their program.

The expected outcome is to have a single program that associates IDLs with  _*any*_ Solana program. Ideally this program will be developed by Labs + external contributors, and then frozen once deployed.

## Alternatives Considered

What alternative designs were considered and what pros/cons does this feature
have relative to them?

#### Alt. 0 - IDL Instructions as Injectable Macro for non-Anchor programs (Noah Gundotra)
Pros:
- Doesn't require changing IDL address for existing anchor programs
- Gives flexibility for non-anchor developers to figure out how to implement
Cons:
- Impossible for frozen programs to implement / update to new versions of IDL spec

#### Alt. 1 - IDL as ELF bytecode (Armani Ferrante)
Pros:
- Can associate IDL with exact version of program
- Standard way to find derived IDL
Cons:
Impossible for frozen programs to implement / update to new versions of IDL spec

#### Alt. 2 - Flat Registry of IDLs + Program Assignment (Jeremy P Bogle)
Pros:
- Anybody can upload an IDL for a program
- Programs can use community defined IDLs for their official IDL
Cons:
- If we decide to add special support for this program in RPC calls, this could become an exploit path for people to easily write lots of data on-chain & cheaply index it

## New Terminology

x

## Detailed Design

<!--Explain the feature as if it was already implemented and you're explaining it
to another Solana core contributor. The generally means:-->

We choose to proceed with Alt. 2 as suggested by Jeremy P Bogle. 
The goal of the program is to:
- Allow anyone to upload IDLs
- Optionally annotate IDLs with slots that describe their target program
- Optionally aggregate these annotated IDLs into a single list account that describes the history of the whole program
- Allow programs to use uploaded IDL lists
We layout a more detailed program interface below.

#### State:
- `IdlAccount`: `{ idl: "not sure, copy from Anchor here", author: Pubkey, program: Pubkey }`
- `IdlAccounts`: `Vec<IdlAccount, SlotRange>`
  - Stores IDL most recent slot range to oldest slot range
  
#### Types:
- `SlotRange`: `{ start: Option(Slot), end: Option(slot)}`
- `Program`: `{ idlAccount: Pubkey, authority: Option(Pubkey) }`

#### Instructions:
- Bah
- Humbug

<!--
- Explain the proposed change and how it works
- Where the feature fits in to the runtime, core, or relevant sub-system
- How this feature was/could be implemented
- Interaction with other features
- Edge cases
-->

## Impact

Anchor developers will have to additionally publish their IDLs to this program.

Developers writing non-anchor programs will have to use other tooling to generate & validate their own IDLs, such as `metaplex-foundation/shank`. Once generated, they will be able to publish their IDL to this pgoram.

## Security Considerations

<!--What security implications/considerations come with implementing this feature?
Are there any implementation-specific guidance or pitfalls?-->

## Drawbacks 

Once this program achieves adoption, it is very likely that client support for programs without IDLs will become more difficult. Additionally, developers managing Anchor programs have to maintain both IDLs while the ecosystem transitions to using this program for IDL discoverability.

<!-- ## Backwards Compatibility *(Optional)* -->
