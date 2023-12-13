---
simd: 'XXXX'
title: extenable output (XOF) hashing support
authors:
  - Ralph Ankele
category: Standard
type: Core
status: Draft
created: 2023-11-30
feature: (fill in with feature tracking issues once accepted)
---

## Summary

This proposal introduces three new concepts to the Solana runtime:
- Support extendable Output Functions (XOF) hasing, based on cSHAKE  
- Support [cSHAKE](https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-185.pdf) as a customable 
version of [SHAKE](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.202.pdf)
- Support the [STROBE protocol](https://strobe.sourceforge.io/papers/strobe-latest.pdf) based on cSHAKE

Using the above new concepts would enable regular Solana programs to:
- Use [merlin transcripts](https://merlin.cool/index.html), automating the Fiat-Shamir transform for 
zero-knowledge proofs, which turns interactive proofs into non-interactive proofs
- Use the widely used [BulletProofs](https://github.com/dalek-cryptography/bulletproofs) zero-knowledge library  

## Motivation

Implementing [cSHAKE](https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-185.pdf) within Solana 
offers several advantages. Firstly, cSHAKE is a variant of the SHA3 extendable-output function (XOF) that allows 
users to customize the output length and incorporate personalized domain separation parameters. It operates by 
taking a message `N` and a customization string `S` as input, enabling users to generate hash outputs of variable 
lengths `L` and tailor the hashing process based on specific application requirements. Extendable-output function's 
such as cSHAKE can be used for:
- *Customized hashing:* cSHAKE's ability to produce variable-length hash outputs makes it valuable for 
applications requiring flexible and tailored hashing functions, such as in blockchain protocols, where different 
data structures might require different hash lengths.
- *Domain Separation:* It is beneficial in situations where secure and domain-separated hashing is necessary, like 
in cryptographic protocols and systems where unique hashing based on different contexts or domains is crucial.
- *Protocols:* It is useful in protocols such as the [STROBE protocol](https://strobe.sourceforge.io/papers/strobe-latest.pdf), which is a versatile protocol framework used to construct 
cryptographic primitives by composing different operations in a sequence known as a protocol transcript. It allows 
for flexible and efficient design of cryptographic protocols by assembling operations like hashing, encryption, 
and authentication in a customizable manner. 

Integrating cSHAKE would enhance Solana's cryptography toolkit, enabling developers to create more secure and 
flexible applications on the platform. By incorporating cSHAKE, Solana can leverage the full potential of the 
[BulletProofs](https://github.com/dalek-cryptography/bulletproofs) zero-knowledge proof library, enabling the 
efficient generation and verification of non-interactive proofs. Applications involving privacy-preserving 
transactions, such as confidential asset transfers, can leverage Bulletproofs for efficient range proofs, while 
cSHAKE provides customizable hashing for enhanced security.

Overall, integrating cSHAKE and enabling to build the Bulletproofs zero-knowledge proof library into Solana's 
infrastructure broadens the platform's cryptographic capabilities, fostering enhanced privacy, security, and 
flexibility for a wide array of decentralized applications and use cases.

## Alternatives Considered
Rewriting the BulletProof zero-knowledge library such that the merlin transcripts are 
not based on any extendable output function. However, that would change the security 
guarantees, and is most probably more complicated to implement. 

Another alternative is to implement the BulletProof zero-knowledge library as a native 
program entirely, however, this would limit the use cases that can additionally be enabled 
by supporting the customable extendable output functin cSHAKE, and the merlin transcripts. 
Though, supporting a native zero-knowledge proof library would likely be more efficient. 

## New Terminology

None.

## Detailed Design

### cSHAKE

cSHAKE is a customable variant of SHAKE, which is SHA3 with infinite output. Basically, cSHAKE 
differs from SHA3/Keccak by 
- infinite output (infinite squeeze)
- different domain seperation (SHA3 appends `01` after the input, SHAKE appends `1111`)

There are two variants of [cSHAKE](https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-185.pdf), `cSHAKE128` and `cSHAKE256`, providing `128`-bit and `256`-bit of security, 
respectively. Both functions take four parameters, `cSHAKE(X, L, N, S)` where
- `X`: is the input string, which can be *any* length (`0..2^2040-1`)
- `L`: the output length in bits
- `N`: the function name as a bit string
- `S`: the customization bit string 

cSHAKE can be defined in terms of SHAKE or Keccak as follows:
```
cSHAKE-128(X, L, N, S):
if N == "" and S == "":
  return SHAKE-128(X, L)
else:
  return Keccak[256](bytepad(encode_string(N) || encode_string(S), 168) || X || 00, L)
```
```
cSHAKE-256(X, L, N, S):
if N == "" and S == "":
  return SHAKE-256(X, L)
else:
  return Keccak[512](bytepad(encode_string(N) || encode_string(S), 136) || X || 00, L)
```

#### Implementation Details

An third-party Rust implementation of cSHAKE (suggested by the Keccak designers on their 
[website](https://keccak.team/software.html)) is available at 
[https://github.com/quininer/sp800-185](https://github.com/quininer/sp800-185/blob/master/src/cshake.rs). 

SHA3/Keccak are already implemented in the Solana runtime, in the [bpf_loader](https://github.com/solana-labs/solana/blob/master/programs/bpf_loader/src/syscalls/mod.rs#L205) as a [syscall](https://github.com/solana-labs/solana/blob/master/sdk/program/src/syscalls/definitions.rs#L46) for hashing. The implementation 
of Keccak is in [solana/sdk/program/src/keccak.rs](https://github.com/solana-labs/solana/blob/master/sdk/program/src/keccak.rs).  

When implementing cSHAKE the Keccak implementation can be used as a template and the domain separations needs 
to be adapted. Moreover, the squeeze function needs to be adapted to allow for infinite squeezing. In more details, a new syscall needs to be added in [solana/sdk/program/src/syscalls/definition.rs](https://github.com/solana-labs/solana/blob/master/sdk/program/src/syscalls/definitions.rs#L46) as follows:
```
define_syscall!(fn sol_cshake128(vals: *const u8, val_len: u64, func_name: *const u8, cust_string: *const u8, hash_result: *mut u8) -> u64);
define_syscall!(fn sol_cshake256(vals: *const u8, val_len: u64, func_name: *const u8, cust_string: *const u8, hash_result: *mut u8) -> u64);
```
Additionally, the support for using cSHAKE as a xof-hasing function can be added in [solana/sdk/program/bpf_loader/src/syscalls/mod.rs](https://github.com/solana-labs/solana/blob/master/programs/bpf_loader/src/syscalls/mod.rs#L149) by adding a new `HasherImpl` implementation for cSHAKE.

```
impl HasherImpl for cShake128Hasher {
  ...
}
```

The cSHAKE implementation then differs from Keccak by modifying the domain separation and the squeeze function from the existing Keccak implementation in [solana/sdk/program/src/keccak.rs](https://github.com/solana-labs/solana/blob/master/sdk/program/src/keccak.rs), or exchanging the `Keccak` implemenation with a `cSHAKE` implementation.

### STROBE

Strobe is a protocol framework based on the duplex Sponge construction. `Strobe-f-λ/b` is a Strobe instance 
with a targeted security level of `λ` bits. The capacity `c = 2λ`, the bandwidth `b=r+c`, where `r` is the rate, 
and `F=f(b)` is the sponge function, based on Keccak-f. 

Strobe can be instantiated with cSHAKE and specifies two instances based on cSHAKE:
- `Strobe-128/1600()`
- `Strobe-256/1600()`

where the  initial state of `Strobe-f-λ/b` is
```
  S_0 := F(bytepad(encode_string("") || encode_string("STROBEv1.0.2"), r/8))
       = F([[1, r/8, 1, 0, 1, 96]] || "STROBEv1.0.2" || (r/8-18) * [[0]])
```

For `Strobe-128/1600()` any data squeezed is of the form `cSHAKE128(X)` and for `Strobe-256/1600()` it is of the form 
`cSHAKE256(X)`. 

#### Implementation Details

The Strobe designers released an official implementation in C available at [https://sourceforge.net/p/strobe](https://sourceforge.net/p/strobe/code/ci/master/tree/). Moreover, a minimal Strobe-128 implementation in Rust is available 
in the [source code](https://github.com/zkcrypto/merlin/blob/main/src/strobe.rs) for the merlin crate. 

The STROBE protocol framework can be build on top of the current Keccak implementation or on top of the cSHAKE implementation, 
as outlined in the Strobe-128 [implementation](https://github.com/zkcrypto/merlin/blob/main/src/strobe.rs) of the merlin 
crate. Additionally to hashing, that is currently implemented for Keccak, additional sponge functions need to be implemented, 
such as: 
- Adding associated data 
- Key Addition
- Extract hash/pseudorandom data (PRF)

which should all be available in Keccak/cSHAKE, by using the `Absorb` and `Squeeze` functions. In more details, for adding 
associated data the functions `AD` and `meta_AD` need to be implemented, that absorbs data into the state. `meta_AD` 
describes the protocols interpretation of the operation. The function `KEY` adds a cryptographic key to the state by 
absorbing the key. The function `PRF` extracts pseudorandom data from the state, by squeezing data. Additionally, Strobe 
operations are defined by flags as outlined in the specifications at [https://strobe.sourceforge.io/specs/#ops.flags](https://strobe.sourceforge.io/specs/#ops.flags).  

For all the above functions, syscalls need to be defined in 
[solana/sdk/program/src/syscalls/definition.rs](https://github.com/solana-labs/solana/blob/master/sdk/program/src/syscalls/definitions.rs#L46) as follows:
```
define_syscall!(fn sol_strobe128_ad(...) -> u64);
define_syscall!(fn sol_strobe128_meta_ad(...) -> u64);
define_syscall!(fn sol_strobe128_key(...) -> u64);
define_syscall!(fn sol_strobe128_prf(...) -> u64);

define_syscall!(fn sol_strobe256_ad(...) -> u64);
define_syscall!(fn sol_strobe256_meta_ad(...) -> u64);
define_syscall!(fn sol_strobe256_key(...) -> u64);
define_syscall!(fn sol_strobe256_prf(...) -> u64);
```

The `AD`, `meta_AD`, `KEY` and `PRF` functions can be build by using the `Absorb` and `Squeeze` functions from a cSHAKE
implementation as defined in [cSHAKE](#implementation-details) above, and need to be added to `solana/sdk/program/src/strobe.rs`
Moreover, a trait needs to be build for the Strobe functions in [solana/sdk/program/bpf_loader/src/syscalls/mod.rs](https://github.com/solana-labs/solana/blob/master/programs/bpf_loader/src/syscalls/mod.rs#L135) by adding a new `StrobeImpl` similar to
the `HasherImpl` used for Keccak hashing.

```
pub trait StrobeImpl {
  ...

  fn ad(...)
  fn meta_ad(...)
  fn key(...)
  fn prf(...)
}
```

### Merlin and BulletProofs

With the sycalls for [cSHAKE](#implementation-details) and [STROBE](#implementation-details-1) in place, the 
[merlin transcripts](https://merlin.cool/index.html) can straight forward 
be implemented in regular Solana programs. This further enables developers to use the 
[BulletProofs](https://github.com/dalek-cryptography/bulletproofs) zero-knowledge proof library.

## Impact

This proposal would enable dapp developers and core contributors to use the extendable output function 
[cSHAKE](https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-185.pdf). Moreover, it would allow 
them to easier build applications based on the [BulletProofs](https://github.com/dalek-cryptography/bulletproofs) 
zero-knowledge proof library.  

## Security Considerations

### cSHAKE
The cSHAKE functions support variable output lengths of `L` bits. Keep in mind that the security of 
e.g. `cSHAKE128` is `min(2^(L/2), 2^128)` for collision attacks and `min(2^L, 2^128)` for 
preimage attacks, where `L` is the number of output bits. While a longer output does not 
improve the security, as shorter output lenght might decrease the security. 

For a given choice of the function name `N` and the customizable string `S`, `cSHAKE128(X, L, N, S)`
has the same security properties as `SHAKE128(X, L)`. Note, that the customizeable string `S` 
should never be under attacker control. It should be a fixed constant or random value set by the 
protocol or application. An attacker controlled customizable string `S` could lead to related-key attacks 
or void any security proof as an attacker could force two outputs of the hash function to be 
the same, by using identical customizable strings.  

### STROBE

Strobe is a framework to create symmetric protocols, so cryptographic keys need to be pre-shared. Moreover, 
the padding in Strobe should be carefully implemented as outlined in the [specification](https://strobe.sourceforge.io/specs/#ops.impl.runf). Additionally, when using Strobe with cSHAKE, the [NIST separation string](https://strobe.sourceforge.io/specs/#ops.impl.init) `N=""` should be set to the empty string as Strobe was not designed by NIST. 

<!---
## Drawbacks *(Optional)*

Why should we not do this?

## Backwards Compatibility *(Optional)*

Does the feature introduce any breaking changes? All incompatibilities and
consequences should be listed.
-->