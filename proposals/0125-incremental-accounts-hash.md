---
simd: '0125'
title: Incremental Accounts Hash
authors:
  - Emanuele Cesena
  - Liam Heeger
  - Sam Kim
  - Tom Pointon
  - Josh Siegel
  - Jeff Washington
  - Haoran Yi
category: Standard
type: Core
status: Draft
created: 2023-03-14
feature:
supersedes:
extends:
---

## Summary

We define a new **Incremental Accounts Hash** to compute a "hash of all
accounts", based on the hash function [LtHash](https://ia.cr/2019/227) [1].

This new hash is very efficient to compute, and can replace both the [**Epoch
Accounts
Hash**](https://docs.solanalabs.com/implemented-proposals/epoch_accounts_hash)
and the **Delta Accounts Hash**, in an effort to scale Solana to billions of
accounts.

## Motivation

Our main goal is to scale Solana to billions accounts, and compute a "hash of
all accounts" in a practical time and memory.

The current solution is to calculate the root of a Merkle tree of all accounts,
sorted by public key. Scaling to billions of accounts will make the Merkle tree
hard to manage (TBs of data), and sorting all the accounts extremely challenging
(overall, an n*log(n) complexity).

Moreover, the current solution requires to distinguish between the **Epoch
Accounts Hash** and the **Delta Accounts Hash**. The latter is computed for
every slot "just because" the former can't be computed at the speed of Solana
slots. If we could compute a "hash of all accounts" fast enough, we wouldn't
need two different hashes.

Incremental hash functions allow to compute the hash of a set (e.g., Solana
accounts) by *incrementally* accumulate the hash of each element, thus removing
the need to sort and maintain large intermediate state.

This work proposes a specific incremental hash suitable for Solana needs, and
defines how to compute it and incorporating it in the existing protocol.

## Alternatives Considered

We considered the following alternatives:

- Merkle Trees.
  - Pros: well known, can support inclusion proofs.
  - Cons: won't scale.
- Verkle Trees.
  - Pros: scale better, support inclusion proofs.
  - Cons: not fast enough for Solana.
- Incremental Hash based on XOR.
  - Insecure: known sub-exponential attacks exist [3, 4].
- Incremental Hash based on Elliptic Curve, e.g. Ristretto or GLS254.
  - Pros: secure incremental hash.
  - Cons: not as efficient as LtHash (10-100x slower).

## New Terminology

**Incremental Accounts Hash** is the new "hash of all accounts", built using an
incremental hash.

**Incremental hash** [2] is a cryptographic hash that is easy to update upon
modification of the underlying message. In our context, the message is the set
of all accounts. As each individual account changes as a result of executing
transactions, we want to compute a new hash value of the entire set of accounts
much more efficiently than recomputing it from scratch.

**LtHash** is the specific incremental hash chosen to compute both: 1) the
individual account hash, and 2) the Incremental Accounts Hash (sum of the
individual account hash, for all accounts).

## Detailed Design

### LtHash (WIP)

We slightly deviate from the original paper [1] replacing BLAKE2x with BLAKE3
[5]. We use BLAKE3 simply because the current account hash is based on BLAKE3.

A hash function has interface: init, append, fini. An incremental hash function
has 2 additional methods: add, sub. Finally, the output of LtHash is 2048-byte
but it's convenient to also have a 32-byte output so we introduce the method:
out.

For LtHash:

- init(): is blake3.init
- append(data): is blake3.append(data)
- fini(): is blake3_xof.fini(2048), i.e. return a 2048-byte output using the
  property that Blake3 is an eXtendable Output Function (XOF).
- add(a, b): interpret the hash values a, b as arrays of 1024 u16. Add each pair
  of elements in the arrays.
- sub(a, b): interpret the hash values a, b as arrays of 1024 u16. Sub each pair
  of elements in the arrays.
- out(a): blake3.fini( blake3.append( blake3.init(), a ) ), i.e. the 32-byte
  blake3 of the 2048-byte data.

### (Single) Account Hash (WIP)

The hash of a single account is defined as:

```
hash(account) :=
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

### Incremental Accounts Hash (WIP)

For a set of accounts A:

```
hash(A) := \sum_{a \in A} hash(a)
```

Specifically, the Incremental Accounts Hash or "hash of all accounts" is the sum
of the (single) account hash for all accounts.

A validator can compute the initial value for the Incremental Accounts Hash at
boot, while loading the accounts state from snapshot.

At slot S, a list of txs is executed, making changes on a set of writeable
accounts. Given the Incremental Accounts Hash before S, we can compute the new
Incremental Accounts Hash after S as follow:

- for each account that's modified within S:
  - compute the hash before changes
  - compute the hash after changes
- sub all the hashes before
- add all the hashes after

Notes:

- Before hashes could be pre-computed and stored in the account metadata db. Or
  not. We consider it an implementation choice.
- The order in which account hashes are add/sub is irrelevant, again this is
  left as an implementation choice
- If an account is modified by multiple txs, it's possible to either compute its
  hashes before/after each individual txs or the entire slot. The final result
  is the same, so again this is left as an implementation choice
- Accounts that are created have before hash = 0. Analogously accounts that are
  close have after hash = 0. Add/sub will work as expected in these edge cases,
  or they could be optimized. Again this is left as an implementation choice

### Current Workflow

Lifecycle of accounts:

1. Validator starts up at a given rooted slot
    1. Needs every account
        1. An account: pubkey + account state: primarily data, lamports, owner.
            1. A hash can be created from the account data
    2. A hash and capitalization from the entire accounts state is produced
        1. The hash and capitalization can be verified against the cluster and
           the snapshot it came from (this is for security and sanity).
2. Validator executes a transaction
    1. Many accounts are read only
    2. Some accounts are writable and updated by the transaction (the same
       account can be written multiple times per transaction).
3. At the end of a slot, all written accounts, in their final state, are held
   together.
    1. Final state means the final values of the fields: data, lamports, ...
       after all transactions have run in this slot and things like rewards and
       fees were dealt with.
4. A Delta Accounts Hash is created
    1. A hash is created per account. 
        1. This is done async in the bg today: each time account state is
           written, a bg thread hashes the new state to have the hash ready when
           it is needed later.
    2. The list of all accounts written in this slot (final state of each
       account) are sorted in pubkey order.
    3. The hashes, in pubkey order, are hashed together in a merkle tree. Notes:
        1. Accounts with lamports=0 ARE included in this merkle tree.
        2. This list has to be de-duped
            1. If the same account is written 2 times in this slot, the hash for
               the account state after the final write is used here.
    4. The merkle tree root hash is called the Delta Accounts Hash.
5. The Delta Accounts Hash is hashed with other data to become the Bank’s Block
   Hash.
6. Once an account is written in this slot, any children slots will load the
   account from this slot.
    1. Note that this is true immediately, even before this slot has become a
       root.
7. When the slot becomes a root (when cluster agrees), the modified accounts
   from this slot are lazily written to more permanent storage.
    1. If the slot does not become a root, then all changes to these accounts
       are discarded.

Verifying the entire account state:

1. An “Accounts Hash” is a hash of the entire account state.
2. There are ~3 times we calculate an Accounts Hash
    1. At startup, to verify entire account state matches expected
    2. Prior to creating a snapshot. A snapshot is a file containing almost
       everything needed to start a validator from a given slot:
        1. All accounts state
        2. A persisted bank at the slot of the snapshot
        3. The status cache
        4. It does NOT contain genesis.
        5. ...
    3. Once per epoch for the Epoch Accounts Hash
        1. Hash is calculated as of (rooted) slot = 25% of epoch
        2. Hash is included in bank’s block hash when slot = 75% of epoch
        3. Goal of this is to make sure, with consensus, at least once per
           epoch, that every validator’s account state matches expected
3. The calculation of the Epoch Accounts Hash today is:
    1. Source data:
        1. Each account at its highest slot number
        2. Excluding accounts where lamports=0 at their highest slot number
    2. Accounts are sorted by pubkey (computationally, this is the most
       expensive and worst scalable part)
    3. A merkle tree of width 16 is calculated
    4. The merkle root is the Epoch Accounts Hash.

### New Workflow

Using the incremental hash:

1. Instead of a merkle tree to hash all accounts, use an incremental hash
    1. Benefit: data that was previously accumulated can be subtracted out, and
       newly written data can then be accumulated in
    2. We start each slot knowing the Incremental Accounts Hash as of the end of
       the last slot.
        1. We update the Incremental Accounts Hash as follows. For each account
           modified during the slot, we compute the account hash before and
           after changes. We subtract the hash before and add the hash after
           into the Incremental Accounts Hash.
        2. This update is O(# accounts written in the slot (1000s)) instead of
           O(# non-zero lamport accounts in the entire system (50B)).
        3. Said in another way, this update costs approximately like a Delta
           Accounts Hash, but provides a hash of the state of all accounts as
           the Epoch Accounts Hash.
2. In the Bank’s Block Hash, we replace the Delta Accounts Hash with the new
   Incremental Accounts Hash
    1. The actual Incremental Accounts Hash is 2KiB long. For simplicity we
       include a 32-byte long BLAKE3(Incremental Accounts Hash) into the Bank’s
       Block Hash.
    2. The entire account state is thus agreed upon by every slot instead of
       once per epoch
    3. In principle, a validator could run a full hash of all accounts
       accumulation in the background on its own schedule to verify no disk bit
       flips have occurred, or any other unexpected change.
4. We also replace the Epoch Accounts Hash with the new Incremental Accounts
   Hash
    1. The actual Incremental Accounts Hash is 2KiB long. For simplicity we use
       a 32-byte long BLAKE3(Incremental Accounts Hash) as a replacement of the
       Epoch Accounts Hash.

### Replacing Epoch & Delta Accounts Hash

This section describes the details of replacing just the Epoch Accounts Hash
with the Incremental Accounts Hash.

Pros:

- Simplicity: one way to hash
- Performs better than merkle tree
- Scales independently of total # accounts
   - O(# accounts written per slot)

Cons:

- Changes to all validators
- For a while we'll have to support and compute both
- Versioning issue with snapshots
- This adds work that must be done per slot to update the accumulated hash
- At the current scale, performance is worse than merkle tree and may affect
  block time (see Performance section below for details on why we think it's ok)

Result:

1. Bank.accounts delta hash: incremental
2. Bank.epoch accounts hash: incremental
3. Bank.parent’s hash of all accounts: incremental
4. Snapshot.hash of all accounts: incremental

### Perfomance

Some performance result - rust code from Solana runtime.

|  | 10_000 accounts | 100_000 accounts |
| --- | --- | --- |
| #[bench] test_accounts_delta_hash (unmodified) | 2.2ms | 25ms |
| without accumulate_account_hashes | 1.559ms | 17ms |
| with blake3_xof (2048 bytes) | 1.563ms | 18ms |

The first row shows the existing bench run “as is”, with 100k accounts (as in
the current bench) or 10k (trivial change to the bench code).

The second row shows that 70% of the computation is blake3, and therefore 30% is
sort + Merkle tree.

The third row shows the cost of switching from blake3 (256-bit) to blake3_xof
(2048-bytes) that’s needed for the incremental hash. The difference is minimal.
Accumulating the hashes is also negligible (it’s only 1024 u16 adds, and they
get vectorized).

**What does this mean for the incremental hash and for a transition period with
both delta and incremental hash?**

- The cost of the incremental hash would be roughly 1.4-1.5x, i.e. 3ms at 10k
  accounts
    - We need to compute a hash of the account before (0.7x) and after (0.7x)
      executing the tx
    - As explained above, this assumes we won’t store/cache any hash at all
    - Some of this cost can be easily hidden, for example by computing the
      account hash while  loading the account, in parallel to executing the tx
      (this means that the total latency could be reduced to 70%)
- During a transition period where both hashes are computed:
    - Trivial: the cost is 2.5x
    - Reusing the long blake3 computation for both hashes can reduce to 1.7x
      (but probably not worth if if we’re dealing with 10k accounts and the
      transition period is not super long)

### Implementation Notes

#### Efficient Caching of BLAKE3 State for Extending Output (Optional)

Implementors of this specification may wish to cache or store the hash of the
accounts. Doing this for the full 2KiB of BLAKE3 extended output could be
prohibitively expensive. 

To avoid this high storage cost we recommend retaining the BLAKE3 state
necessary to recompute the extended output. This is comprised of:

- Root input chaining value (32 bytes): The chaining value used as input to the
  root compression function in the BLAKE3 algorithm.
- The input data length in bytes modulo 64 bytes (1 byte): Note, if the
  `data_len % 64 == 0 && data_len != 0` then this length is 64. Alternatively,
  implementors already have the total input data length, and can just retain
  this, computing the modulo on the fly.
- Given the length value above, retain the last data_len modulo 64 bytes (note
  the exception above) of the input (max 64-bytes).

Together this state can be used to recompute the 2KiB extended BLAKE3 hash. [4,
Section 2.6]

## Impact

This change only impacts validators. It'll be a transparent change for any other
user.

## Security Considerations

LtHash instantiated with BLAKE3 and a 2048-byte output provides the desired
128-bit security [1, Appendix A].

## Drawbacks

The Incremental Accounts Hash does NOT support inclusion/exclusion proofs.

## Backwards Compatibility

The Incremental Accounts Hash replaces both the Epoch Accounts Hash and the
Delta Accounts Hash, and therefore is a breaking change.

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
