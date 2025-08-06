---
simd: '0215'
title: Homomorphic Hashing of Account State
authors:
  - Brooks Prumo
  - Emanuele Cesena
  - Josh Siegel
  - Sam Kim
category: Standard
type: Core
status: Activated
created: 2024-12-20
feature: LTHasHQX6661DaDD4S6A2TFi6QBuiwXKv66fB1obfHq
---

## Summary

This proposal adds a new hash, **Accounts Lattice Hash**, that uses homomorphic
hashing to maintain a hash of the total account state.  The **Accounts Lattice
Hash** is both fast to update, and secure, enabling (1) every block to contain
the hash of all accounts, instead of only the accounts changed in that block,
and (2) removing the **Epoch Accounts Hash**.


## Motivation

The main goal is to scale Solana to billions accounts, and compute a "hash of
all accounts" in practical time and space.

Currently there are two main kinds of accounts hashes used in Solana:

1. The **Epoch Accounts Hash**, which is a merkle-based hash of the total
   account state.
2. The **Accounts Delta Hash**, which is also a merkle-based hash of accounts
   written in a single block.

Both of these hashes have limitations.  They calculate the root of a Merkle
tree of accounts, sorted by public key, which makes scaling challenging due to
the amount of data required.  This is also why there are two distinct hashes:
the **Epoch Accounts Hash** hashes the every account, but is infrequent; and
the **Accounts Delta Hash** is every block, but only contains the subset of
accounts written in that block.

Ideally every block would contain a hash of the total account state.

Homomorphic hashing functions enable computing the hash of the total account
state by *incrementally* accumulating the hash of each account.  This removes
the need to sort and maintain large intermediate state.  Said another way, the
total account state hash for block `B` can now be computed from block `B - 1`'s
total account state hash plus *only* the accounts that changed in block `B`.


## New Terminology

**Accounts Lattice Hash** is the new "hash of all accounts", built using a
lattice-based homomorphic hashing function.

**Homomorphic hashing** "is the concept to solve the following problem: Given
the hash of an input, along with a small update to the input, how can you
compute the hash of the new input with its update applied, without having to
recompute the entire hash from scratch?" [1].

**Lattice hash** is the specific homomorphic hashing function chosen to compute
both an individual account hash, and the hash of all accounts.


## Detailed Design

### Defining LtHash

**LtHash** is the 2048-byte result of hashing an input with the lattice hashing
function.

A hash function has interface: `init`, `append`, `fini`.  A homomorphic hash
function has two additional methods: `add`, `sub`.  Finally, it is convenient
to have a 32-byte result, so we introduce the method: `out`.

For LtHash:

`init()`: is blake3.init
`append(data)`: is blake3.append(data)
`fini()`: is blake3_xof.fini(2048), i.e. return a 2048-byte output using the
    property that Blake3 is an eXtendable Output Function (XOF).
`add(a, b)`: interpret the hash values `a, b` as arrays of 1024 u16s.
    Wrapping-add each pair of elements in the arrays.
`sub(a, b)`: interpret the hash values `a, b` as arrays of 1024 u16s.
    Wrapping-sub each pair of elements in the arrays.
`out(a)`: blake3.fini( blake3.append( blake3.init(), a ) ), i.e. the 32-byte
    blake3 of the 2048-byte data.


### Calculating LtHash of one account

The LtHash of a single account is defined as:

```
lthash(account) :=
  if account.lamports == 0:
    return 00..00
  else:
    lthash.init()
    lthash.append( account.lamports )
    lthash.append( account.data )
    lthash.append( account.is_executable )
    lthash.append( account.owner )
    lthash.append( account.pubkey )
    return lthash.fini()
```


### Calculating LtHash of all accounts, aka Accounts Lattice Hash

For a set of accounts `A`:

```math
LTHASH(A) = \sum_{a \in A} lthash(a)
```

Specifically, the Accounts Lattice Hash is the sum of the (single) account
lthash for all accounts.


### Incrementally updating Accounts Lattice Hash per block

For a given block, there are a set of accounts modified in that block.  Let
`account` indicate the state of the account *before* modification, and
`account'` indicate the state of the account *after* modification.

To compute the accounts lattice hash for a block, `LTHASH'`, and given the
*prior* block's accounts lattice hash, `LTHASH`:

```
LTHASH' := LTHASH
for each account modified in B:
  LTHASH'.sub( lthash( account ) )
  LTHASH'.add( lthash( account' ) )
return LTHASH'
```

Notes:

- The order in which account lthashes are add/sub is irrelevant.
- If an account is modified by multiple transaction, it is possible to either
  compute its lthashes before/after each individual transaction or the entire
  block.  The final result is the same.
- Every account modified in a block must be reflected in that block's Accounts
  Lattice Hash.  This applies to both on-chain and off-chain modifications.


### Changes to the Bank Hash

The bank hash for each block will now include the Accounts Lattice Hash.  The
Accounts Lattice Hash will be incrementally updated and mixed in *instead of*
the Epoch Accounts Hash (and in every block, not just once per epoch).  It is
safe to replace the Epoch Accounts Hash with the Accounts Lattice Hash since
they are both hashes of the total account state.

Note that this SIMD does *not* make any changes to the Accounts Delta Hash.


### Feature Activation

At feature activation, all nodes must have the Accounts Lattice Hash.  If they
haven't already been computing/updating it, they must now do so.

It is recommended to provide a mechanism for nodes to opt-in to computing the
initial Accounts Lattice Hash and begin updating the value each block *before*
feature activation.  This will enable the cluster to pre-compute the Accounts
Lattice Hash prior to feature activation and avoid a cluster-wide pause while
all nodes calculate the Accounts Lattice Hash.


### Snapshots

Since computing the initial Accounts Lattice Hash is expensive, we want to
avoid this computation when possible.  The Accounts Lattice Hash will be
written to the snapshot, and will be read out at boot time (if present).

It is recommended for nodes to verify the Accounts Lattice Hash read from the
snapshot.

Note that this SIMD does *not* make any changes to the Snapshot Hash.


### Epoch Accounts Hash

As stated above, the Accounts Lattice Hash will replace the Epoch Accounts
Hash.  Thus, the Epoch Accounts Hash will effectively be removed from consensus
at feature activation.  The snapshot format will *not* change to *remove* the
Epoch Accounts Hash field.


## Alternatives Considered

The following alternatives were considered:

- Merkle Trees
  - Pros: well known, can support inclusion proofs
  - Cons: won't scale
- Verkle Trees
  - Pros: scale better, support inclusion proofs
  - Cons: not fast enough for Solana
- Incremental Hash based on XOR
  - Insecure: known sub-exponential attacks exist [3, 4]
- Incremental Hash based on Elliptic Curve, e.g. Ristretto or GLS254
  - Pros: secure incremental hash
  - Cons: not as efficient as LtHash (10-100x slower)


## Impact

Only validators will be impacted.


## Security Considerations

LtHash instantiated with BLAKE3 and a 2048-byte output provides the desired
128-bit security [1, Appendix A].


## Drawbacks

The Accounts Lattice Hash does NOT support inclusion/exclusion proofs.


## Backwards Compatibility *(Optional)*

Incompatible. This changes the bank hash, thus changing consensus.


## Bibliography

1. *Lewi, Kim, Maykov, Weis*, **Securing Update Propagation with Homomorphic
   Hashing**, 2019, [ia.cr/2019/227](https://ia.cr/2019/227)
2. *Bellare, Goldreich, Goldwasser*, **Incremental Cryptography: The Case of
   Hashing and Signing**, 1994,
   [cseweb.ucsd.edu](https://cseweb.ucsd.edu/~mihir/papers/inc1.pdf)
3. *Bellare, Micciancio*, **A new paradigm for collision-free hashing:
   Incrementality at reduced cost**, 1996,
   [cseweb.ucsd.edu](https://cseweb.ucsd.edu/~daniele/papers/IncHash.html)
4. *Wagner*, **A Generalized Birthday Problem**, 2002,
   [crypto2002](https://www.iacr.org/archive/crypto2002/24420288/24420288.pdf)
5. *O'Connor, Ausmasson, Neves, Wilcox-O'Hearn*, **BLAKE3**, 2021,
   [PDF](https://github.com/BLAKE3-team/BLAKE3-specs/blob/master/blake3.pdf)
