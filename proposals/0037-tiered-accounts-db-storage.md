---
simd: '0037'
title: Tiered Accounts DB Storage
authors:
  - Yueh-Hsuan Chiang
category: Standard
type: Core
status: Draft
created: 2023-02-13
---

## Summary

This proposal presents a hierarchical storage architecture that enables
the accounts database to efficiently manage a large number of accounts.
It stores active accounts in a format that is optimized for performance
while accommodating the storage of inactive accounts in a compact format
that conserves disk space.

## Motivation
The existing accounts db employs AppendVec, a memory-mapped file format
and optimized for efficient runtime performance.  However, as the number of
accounts continues to expand, a storage solution is required to accommodate
billions of accounts while still maintaining performance.

## Workload
The accounts DB serves both reads and writes.  It is a read-write-balanced to
write-heavy workload.  Writes to the accounts DB constantly happen as a result
of committing transactions/blocks, and the accounts DB serves the following
read workload:

* Point-lookups from transactions: transactions also read relevant account
data / meta.  Lookups are in general random, with some accounts that are
accessed more frequently than the others in a short period of time.

* Scans for recent slots for hash calculation: periodic hash calculation scans
all accounts from the recent slots.  Currently, hash calculation happens for
every 100 blocks, and we plan to reduce the frequency to once per 25k blocks
in near future with incremental accounts hashes and incremental snapshots.

* Not-found queries for account creation: each account creation checks
whether the given address already exists in the accounts DB.  This requires
an efficient way to support "not-found" queries.

* Hot accounts: we observed that some accounts are updated more frequently than
others, and each update to one account's data usually only changes a small
portion of the data instead of the entire data.

## New Terminology
- Hot account: an account that is recently/frequently updated.
- Cold account: an account that hasn't been updated for a while.

## Detailed Design
The proposed design uses a extensible blocked-based file format that
can be used for both cold and hot storage.  The format includes
five sections: Account Data Blocks, Account Metas Block, Owners Block,
Account Index Block, and Footer.

The Footer includes information about to read the file.  Specifically,
it includes the file format version and the format of each section.
This allows cold and hot storage shares the same file layout but optimize
differently in each section.  For instance, cold storage has its account
data blocks compressed while hot storage stores its account data blocks
in uncompressed and aligned format in order to make it memory-mapped
possible.

    +------------------------------------------------------------------------------+
    | Account Data Blocks      | Account data in compressed blocks.                |
    | (compressed)             | Optional fields such as rent_epoch are also       |
    |                          | stored here.                                      |
    +------------------------------------------------------------------------------+
    | Account Metas Block      | Fixed-size entries for storing accounts metadata. |
    | (can be compressed)      | Metadata also includes information about how to   |
    |                          | access its owner and account data.                |
    +------------------------------------------------------------------------------+
    | Owners Block             | 32 byte (Pubkey) per entry                        |
    | (uncompressed)           | Each account meta entry has an index pointing     |
    |                          | to its owner entry.                               |
    +------------------------------------------------------------------------------+
    | Account Index Block      | Index for accessing accounts meta block           |
    | (uncompressed)           | Footer has an enum entry describing the format    |
    |                          | of this block.                                    |
    +------------------------------------------------------------------------------+
    | Footer                   | Stores information about how to read the file.    |
    | (uncompressed)           | (e.g. the offset and format of each block.)       |
    +------------------------------------------------------------------------------+

The blocks are stored in reverse order to make the writer single-pass.
The single-pass writer design also provides optimization opportunities
such as sorting the data before persisting, pairing entries to avoid
possible padding when the target format requires alignment.
Once a file is written, it becomes immutable.

In the rest of the sections, we will begin with sections where both cold and hot
storages share in common, then we will go deeper in each individual format.

### Footer
The footer block includes information about how to read the file.  Both cold
and hot storage share the same footer format.

The footer has two parts:  footer body and footer tail.

    +----------------------------------------------------------------------------------+
    | Footer                                                                           |
    +----------------------------------------------------------------------------------+
    |   Footer Body (Size specified in the Footer Tail to allow extensions)            |
    +----------------------------------------------------------------------------------+
    |   Footer Tail (Fixed 88-bytes)                                                   |
    +----------------------------------------------------------------------------------+

Footer Tail is the first part to read when opening the accounts file.
It is a fixed 88-byte block that includes a magic number (8 bytes), the min and max
account addresses (32 bytes each), the format version of the file, and the size of
the entire footer.

The magic number indicates whether the file is a complete accounts data storage
file without truncation.  The min and max account addresses allow the reader
to quickly know whether the account of interest falls-in the account range of
this file.  The format version allows the reader to correctly parse the file
using the right verion.  The footer size tells the reader how to seek to
the offset of the Footer Body.
    
    +----------------------------------------------------------------------------------+
    | Footer Tail (88 bytes)                                                           |
    +------------------------------------+---------------------------------------------+
    | footer_size (8 bytes)              | The size of the footer block.               |
    | format_version (8 bytes)           | The format version of this file.            |
    | min_account_address (32 bytes)     | The minimum account address in this file.   |
    | max_account_address (32 bytes)     | The maximum account address in this file.   |
    | magic_number (8 bytes)             | A predefined const magic number to indicate |
    |                                    | the file type and make sure the file itself |
    |                                    | is completed without truncation.            |
    +------------------------------------+---------------------------------------------+

The Footer Body describes the structure of the file, including the offset and format of
each section.

    +----------------------------------------------------------------------------------+
    | Footer Body                                                                      |
    +----------------------------------------------------------------------------------+
    | (data_block_offset) (0 bytes)      | (This offset is omitted as it's always 0.)  |
    | data_block_format (8 bytes)        | The format of the account data blocks.      |
    | data_block_size (8 bytes)          | The size of the account data block.         |
    |                                    | (Defult: 4k, uncompressed)                  |
    |                                    |                                             |
    | account_metas_offset (8 bytes)     | The offset of the account metas block.      |
    | account_meta_count (8 bytes)       | The number of account meta entries.         |
    | account_meta_entry_size (8 bytes)  | The size of each account meta entry.        |
    | account_metas_format (8 bytes)     | The format of the account meta section.     |
    |                                    |                                             |
    | owners_offset (8 bytes)            | The offset of the owners block.             |
    | owner_count (8 bytes)              | The number of unique owners in this file.   |
    | owner_entry_size (8 bytes)         | The size of each owner entry in bytes.      |
    |                                    |                                             |
    | account_index_offset (8 bytes)     | The offset of the account address block.    |
    | account_index_format (8 bytes)     | Describe the format of the account index    |
    |                                    | block.                                      |
    |                                    |                                             |
    | file_hash (32 bytes)               | The accounts hash of the entire file.       |
    |                                    |                                             |
    | optional_field_version  (8 bytes)  | The version of the account optional fields. |
    +----------------------------------------------------------------------------------+

### Account Index Block
This block includes information about how to load an account given its address.
Its offset / format information is described in `account_index_offset` /
`account_index_format` fields in the Footer Body.

As an ideal account index format itself can already be an independent proposal,
here we only introduce one possible format to help illustrate how Account Index
Block might look like.

In the basic format, the account index block is sorted by the account's address
to allow binary search.  It has two sections: `account_addrsses` and `meta_indices`.
`account_addresses` section stores every pubkeys in sorted order.  Each account
address entry has a correspinding entry in the `meta_indices` section.

    +------------------------------------+---------------------------------------------+
    | Account Index Block (Binary Search Format) -- 40 bytes per account               |
    +------------------------------------+---------------------------------------------+
    | addresses (32-byte each)           | Each entry is 32-byte (Pubkey).             |
    |                                    | Entries are sorted in alphabetical order.   |
    +------------------------------------+---------------------------------------------+
    | meta_indices (4-byte each)         | Indices to access the account meta.         |
    |                                    | The Nth entry in addresses corresponds      |
    |                                    | Nth entry in meta_offsets.                  |
    +------------------------------------+---------------------------------------------+

### Owners Block
The owners block includes one address for each unique owner.  As multiple accounts
might share the same owner, storing unique owners in one block and accessing them
using local indices would save disk space.  In addition, as addresses are
uncompressible, having them stored separately in one block can make the compression
easier for account metas block.

    +----------------------------------------------------------------------------------+
    | Owners Block -- 32 bytes per unique owner                                        |
    +------------------------------------+---------------------------------------------+
    | owner_addresses                    | 32-byte (Pubkey) each.                      |
    |                                    | Each account-meta entry has a local index   |
    |                                    | pointing to its owner's address.            |
    +------------------------------------+---------------------------------------------+

### Account Metas Block
Account Metas block contains one entry for each account in this file.
Each account-meta is a fixed-size entry that includes its account's metadata and
how to access its account data, owner, and optional fields.

The size of each account meta entry is described in the `account_meta_entry_size`
field in the footer.  The `account_metas_format` field in the footer also provides
information about how to correctly read each account-meta entry.

We make the format flexible so that cold and hot storage can use different
account meta storage format.

#### Hot Storage Account Meta Format
All entries in hot storage file format are uncompressed and aligned.  As a result,
each account data will have its own aligned and uncompressed block.  Thus, we don't
need to store `intra_block_offset` and `uncompressed_data_len`.

Instead, we store `data_block_offset / 8` and `padding_bytes` in one 8 bytes unit.
As the offset of each data block is aligned, we use 7 bytes to store
`data_block_offset / 8` and 1 byte to store the number of `padding_bytes`.  Again,
the length of one acount data can be derived by the offsets of two consecutive
account data blocks and the number of padding bytes.

    +--------------------------------------------------------------------------------+
    | Account Meta Entry -- 24 bytes per account                                     |
    +--------------------------------------------------------------------------------+
    | lamport (8 bytes)              | The lamport balance of this account.          |
    |                                |                                               |
    | data_block_offset (7 bytes)    | Value * 8 is the offset of its data block.    |
    | padding_info (1 bytes)         | 3 bits for the number of padding bytes.       |
    |                                | 2 bits to describe whether the account:       |
    |                                | - 00: has its own account data block.         |
    |                                | - 01: the 1st account in a shared data block. |
    |                                | - 02: the 2nd account in a shared data block. |
    |                                |                                               |
    | owner_index (4 bytes)          | The index of the account's owner address in   |
    |                                | the owners block.                             |
    | flags (4 bytes)                | Flags include executable and whether this     |
    |                                | account has a particular optional field.      |
    +--------------------------------------------------------------------------------+
    
#### Cold Storage Account Meta Format
In cold storage file format, one account data block may contain account data and
optional fields for multiple accounts, and each account data block is compressed.

As a result, to access the account data of one account, it requires the offset
of the compressed data block, the intra offset to its account data after
decompression, and the length of the account data after decompression.

For account data that exceeds the configured data block size (default 4K), its
account data and optional fields will have their own data block that exceeds
the data block size.  For such blob accounts, their `intra_block_offset` will
be 0, and their `uncompressed_data_len` will be u16::MAX.

The `flags` field is used to store one account's boolean attributes (such as
whether its account data is executable) and whether the account has a particular
optional field.

    +--------------------------------------------------------------------------------+
    | Account Meta Entry -- 28 bytes per account                                     |
    +--------------------------------------------------------------------------------+
    | lamport (8 bytes)              | The lamport balance of this account.          |
    |                                |                                               |
    | data_block_offset (8 bytes)    | The offset to the account's data block.       |
    | intra_block_offset (2 bytes)   | The inner-block offset to the accounts' data  |
    |                                | after decompressing its data block.           |
    | uncompressed_data_len (2 bytes)| The length of the uncompressed_data.          |
    |                                | If this value is u16::MAX, then the size is   |
    |                                | the entire data block minus optional fields.  |
    |                                |                                               |
    | owner_index (4 bytes)          | The index of the account's owner address in   |
    |                                | the owners block.                             |
    | flags (4 bytes)                | Flags include executable and whether this     |
    |                                | account has a particular optional field.      |
    +--------------------------------------------------------------------------------+

### Account Data Blocks
An account data block consists of one or more account data entries.  Each account
data entry includes the data and the optional fields of one account.

    +--------------------------------------------------------------------------------+
    | Account Data Entry                                                             |
    +--------------------------------------------------------------------------------+
    | account_data               |                                                   |
    |                            |                                                   |
    | optional_field_1           |                                                   |
    | ...                        |                                                   |
    | optional_field_n           |                                                   |
    +--------------------------------------------------------------------------------+

#### Hot Storage Account Data Blocks
For hot account storage, there're two types of account data block.

The first type is simple: an account data block that contains only one account entry.
The only difference is that there're 0-7 padding bytes between the account data
and the optional fields.

    +--------------------------------------------------------------------------------+
    | Single Account Data Block                                                      |
    +--------------------------------------------------------------------------------+
    | account_data               |                                                   |
    | [0-7 bytes padding]        |                                                   |
    | optional_field_1           |                                                   |
    | ...                        |                                                   |
    | optional_field_n           |                                                   |
    +--------------------------------------------------------------------------------+

In the second type, one account data block contains two account data entries, where
the lengths of two account data together is a multiple of 8. This optimization is to
avoid the extra padding bytes for alignment.
    
    +--------------------------------------------------------------------------------+
    | Dual Account Data Block                                                        |
    +--------------------------------------------------------------------------------+
    | account_data_a             |                                                   |
    | account_data_b             |                                                   |
    | optional_field_a_1         |                                                   |
    | ...                        |                                                   |
    | optional_field_a_n         |                                                   |
    | optional_field_b_1         |                                                   |
    | ...                        |                                                   |
    | optional_field_b_n         |                                                   |
    +--------------------------------------------------------------------------------+

#### Cold Storage Account Data Blocks
    
    +--------------------------------------------------------------------------------+
    | Account Data Block (Compressed)                                                |
    +--------------------------------------------------------------------------------+
    | Account Data Entry 1                                                           |
    | Account Data Entry 2                                                           |
    | ...                                                                            |
    | Account Data Entry N                                                           |
    +--------------------------------------------------------------------------------+

    +--------------------------------------------------------------------------------+
    | Account Data Entry                                                             |
    +--------------------------------------------------------------------------------+
    | account_data       |                                                           |
    | optional_field_1   |                                                           |
    | optional_field_2   | If the associated bit in account meta's "flags" is on,    |
    | ...                | then the exact number of bytes is reserved for this field.|
    | optional_field_n   | Otherwise, 0 bytes is used.                               |
    +--------------------------------------------------------------------------------+

## Impact
- Allows Acccounts DB to handle larger amount of accounts.
- Keep hot-accounts performant by using aligned / memory-mapped file format, while
  optimizing storage size for cold accounts using a compressed format.
- Use fewer number of bytes for each account
 - Deprecate 48 bytes of account hash, write version, and rent epoch.
 - Introduce optional fields.  None field only requires 1 bit in the file.
 - Minimize padding by grouping two account data entries which lengths together
   is a multiple of 8.

## Backwards Compatibility
While the existing AppendVec and the proposed can support the existing AccountsDB
API, additional steps are needed in snapshot format to handle the migration period.

As the proposed format includes extra fields describing the format of each block
section, it is extensible to allow different implementations of different block
sections that ensures backward compatibility.
