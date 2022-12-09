---
simd: '0002'
title: Ledger-CAR file format
authors:
  - Richard Patel (Jump Crypto)
  - Peter Rabbitson (Protocol Labs)
  - Jonathan Victor (Protocol Labs)
category: Interface
type: Standards Track
status: Draft
created: 2022-11-02
---

## Summary

Ledger-CAR is a streaming file format for Solana ledger data.

Ledger data broadly contains all block information required to validate all state transitions between two slots.

Specifically, this includes two types of data:
* Proof-of-History parameters
* Transactions (both user txs and consensus txs)

To support distribution over public networks, Ledger-CAR files are generated deterministically and are stored data via content-addressable IPLD objects.

## Specification

### Solana Entries

The canonical representation of block data is the "entries" format.
Each entry contains proof-of-history tick parameters and a subslice of transactions recorded in the block.

Entries are serialized as Bincode little-endian encoding.

```python
class Entry:
  # PoH parameters
  num_hashes: int
  prev_hash: Hash
  # Block data
  transactions: list[Tx]

class Block:
  slot: int
  entries: list[Entry]
```

### CARv1 File Format

The **Content-addressable ARchive** is a streaming container format that serves as the basis of the Ledger-CAR file format.
[CARv1] files store a number of binary blobs of arbitrary content and size.

Each blob is prefixed by its [CIDv1] (content identifier) which serves as a globally unique identifier.
The CID consists of a cryptographic hash of the blob it is referring to, prefixed by a [multicodec] type identifier.

Content addressability stems from the fact that a CID unambiguously commits to the exact binary content of a blob.
This allows for storage and distribution of blobs via untrusted peers.

  [CARv1]: https://ipld.io/specs/transport/car/carv1/
  [multicodec]: https://github.com/multiformats/multicodec
  [CIDv1]: https://docs.ipfs.tech/concepts/content-addressing/


### Determinism

Ledger-CAR files are reproducible and deterministic.
Independent node operators would generate byte-by-byte identical CAR files for the same extent of ledger history regardless of where that data is sourced from.

The generation procedure must be free of ambiguity, reflected in all aspects of the file design below.

### CARv1 Header

The header of the Ledger-CAR file is set to the following CBOR data.

```json
{
  "roots": ["bafkqaaa"],
  "version": 1
}
```

Rationale: The CAR file does not have a single root so we place the "empty" multihash instead,
as recommended by the [CARv1 spec](https://ipld.io/specs/transport/car/carv1/#number-of-roots).

This implies that any CARv1 file starts with the following byte content (hex).

```
19 a2 65 72 6f 6f 74 73
81 d8 2a 45 00 01 55 00
00 67 76 65 72 73 69 6f
6e 01
```

### IPLD schemas

Each Solana block is first converted into a data structure according to the following [IPLD schemas].
IPLD schemas are a neutral interface definition language based operating on a JSON-like data model.

  [IPLD schemas]: https://ipld.io/docs/schemas/

```ipld
type Block struct {
  slot      Int
  entries   [ &Entry ]
  shredding [ Shredding ]
} representation map

type Shredding struct {
  entryEndIdx Int
  shredEndIdx Int
} representation tuple

type Entry struct {
  numHashes  Int
  hash       Hash
  txs        [ &Transaction ]
} representation tuple

type Hash bytes

type Transaction bytes
```

The resulting JSON-like data structure is then converted into a set of blobs, beginning with the `Block` struct.

The reference operator (`&`) marks the locations where subtrees of the structure are split out into separate blobs. (specifically, every `Entry` and `Transaction` struct instance)
The original position of the split out object is replaced with a CID link.

Data is encoded into blobs by encoding the wire representation data model (as indicated by the schema) with [DAG-CBOR] codec.

  [DAG-CBOR]: https://ipld.io/specs/codecs/dag-cbor/spec/

### Multicodec IDs

The type of each blob is defined by the outermost structure and identified by the multicodec ID.

For the Ledger-CAR types, the IDs are defined as follows.
- `Transaction` (legacy or v0 transaction format): `0x5B00`
- `Block`: `0x5BCB`
- `Entry`: `0x5BCE`

### CAR blob order

Blobs stored in the Ledger-CAR file follow a strict order.

For each block, the order of blobs matches that of depth-first traversal over the aforementioned IPLD representation.

In the case of CARs storing multiple blocks, blocks should be ordered by block number.

**Listing 1: Example block IPLD data**

```
Block {
  slot: 42,
  entries: [
    Entry {
      numHashes: 100,
      hash: 'foo',
      txs: [
        Transaction: 'tx1',
        Transaction: 'tx2',
      ]
    }
    Entry {
      numHashes: 101,
      hash: 'bar',
      txs: [
        Transaction: 'tx3',
      ]
    }
  ]
}
```

**Listing 2: Example block in CAR blob order**

The block in _listing 1_ corresponds to the following ordered vector of blobs.

1. ```
   Transaction: 'tx1'
   ```
2. ```
   Transaction: 'tx2'
   ```
3. ```
   Entry {
     numHashes: 100,
     hash: 'foo',
     txs: [ CID_1, CID_2 ]
   }
   ```
4. ```
   Transaction: 'tx3'
   ```
5. ```
   Entry {
     numHashes: 101,
     hash: 'bar',
     txs: [ CID_4 ]
   }
   ```
6. ```
   Block {
     slot: 42,
     entries: [ CID_3, CID_5 ]
   }
   ```

### CID Format

CIDs in Ledger-CAR must use the following parameters:
- multibase prefix: `b` (base32)
- multihash type: `0x12` (sha2-256)
- multihash length: `0x20` (32 bytes)

## Motivation

Transparency and verifiability of transaction data are fundamental security properties of public blockchains.
In practice, these directly depend on public availability of block data.

Existing methods for storing distributing historical block data bear various shortcomings, such as the use of trusted peers or private networks.

This warrants the creation of file format designed to support ledger archival and public distribution.
This new format should be generic/neutral (to support a variety of storage backends) and simple to implement (read: based on existing standards).
Additionally, any node operator that owns an existing local copy of ledger data should be able to independently convert their data into this new format to support public distribution efforts.

## Alternatives Considered

The following notable alternatives are currently available.

Solutions based on the Geyser plugin interface v1.14 are not up for consideration as they lack PoH information.

### Alternative: Turbine packet captures

**Summary**

The Turbine peer-to-peer protocol shreds block data into network packets with erasure coding.
This involves splitting serialized entries into ~1.5kB size "shreds" designed to be transmitted as UDP packets.
Each shred is signed by the leader that produced it.

**Creation**

Using standard packet capture tooling such as libpcap, such Turbine traffic can be recorded from a network socket for long-term storage.

**Overhead**

The internet headers contained in typical traffic captures add up to at least 134 bytes of overhead per packet. (88 byte shred header, 18 byte Ethernet header, 20 byte IPv4 header, 8 byte UDP header).
Assuming a 1500 byte average packet size, this works out to at least ~9% storage overhead.
Unless the packet capture was filtered, error correction codes and unrelated traffic further increase this ratio.

**Considerations**

Current tooling can only capture live traffic and cannot discern orphaned blocks or malicious traffic from actual ledger data.
Thus, parsing and filtering for ledger data adds significant complexity.

### Alternative: Blockstore

**Summary**

Blockstore is the validator database implementation of the Solana Labs validator.

It stores a rolling window of shreds in a RocksDB database.
This database format offers streaming of block data and an integrated index to efficiently query individual shreds.

**Creation**

A blockstore database of a Solana Labs validator can be captured by creating a point-in-time copy of its `rocksdb` directory.
Although recommended, it is not necessary to stop the validator to capture such data, as RocksDB can recover from databases that were not gracefully shut down.

**Overhead**

The overhead of blockstore databases is not trivial to quantify.
Solana Labs validators use them to store various other data relevant to the operation of a validator.

**Considerations**

Blockstore databases are inherently implementation-defined and depend on the RocksDB database engine.
