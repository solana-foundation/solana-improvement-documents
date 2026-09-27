---
simd: '0671'
title: Typed Settlement Wire Linkage
authors:
  - Abdul Shabazz (veritasvaultone@gmail.com)
category: Standard
type: Interface
status: Draft
created: 2026-09-26
---

## Summary

This proposal standardizes the Typed Settlement Wire Protocol (X402-TSWP)
for Solana applications, defining a deterministic method to bind external
payment references (such as SWIFT ISO 20022 RFC 4122 UUIDv4 UETRs) directly
to on-chain state transitions.

The standard couples:

1. Canonical typed wire memos carried by the official SPL Memo v2 program
   (`MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr`);
2. Zero-regex on-chain instruction verification via the Solana Instructions
   Sysvar (`sysvar::instructions`); and
3. Account-level transfer gating using the SPL Token-2022
   `RequiredMemoTransfers` extension.

## Motivation

Institutional settlement, interbank foreign exchange, and corporate treasury
systems mandate end-to-end auditability against unique external transaction
identifiers. Under SWIFT CBPR+ (ISO 20022 `pacs.008.001.08`), every
transaction requires an immutable 128-bit UETR.

Prior art decentralized applications on Solana face three limitations:

1. Off-Chain Desynchronization: Decoupling transaction signatures from
   banking references into off-chain relational databases creates race
   conditions, orphan transfers, and non-repudiation vulnerabilities.
2. Regex Parsing Fragility: Parsing transaction memos using off-chain
   regular expressions exposes listeners to spoofing via leading or
   trailing garbage byte injection.
3. Execution Pipeline Isolation: Standard SPL token transfers do not
   cryptographically verify whether an attached memo succeeded or matched
   expected parameters within the same atomic transaction pipeline.

This proposal specifies an on-chain execution discipline that guarantees
no settlement instruction can finalize without an atomic, byte-exact memo
verified directly from execution memory.

## Alternatives Considered

1. Off-chain database indexing: Storing external payment references solely
   in off-chain relational databases decoupled from the ledger. This was
   rejected due to race conditions, synchronization latency, and absence
   of cryptographic non-repudiation.
2. Custom instruction data fields: Requiring every protocol contract to
   embed bespoke external identifier fields. This was rejected because it
   breaks composability with standard SPL Token transfer instructions and
   third-party liquidity vaults.
3. Unverified memo logging: Emitting unvalidated memos alongside transfers.
   This was rejected because trailing-byte injection and regex ambiguity
   expose accounting engines to spoofing.

## New Terminology

- X402-TSWP: Typed Settlement Wire Protocol, defining canonical ASCII
  prefixes for institutional memo payloads.
- UETR: Unique End-to-end Transaction Reference (128-bit UUIDv4 mandated
  under ISO 20022 standard).
- Instructions Sysvar: The Solana native introspection sysvar
  (`sysvar::instructions`) exposing the current transaction's execution
  pipeline.

## Detailed Design

### 1. The Typed Memo Wire Grammar

All memos conform to strict 7-bit ASCII, using standard base-10
representations without leading zeros:

```text
X402<Type>:<Payload>
```

- `X402W:<window_epoch>:<uetr>`: Net-window settlement binding SWIFT UETR.
- `X402L:<lane_id>:<window_epoch>:<nonce>`: Concurrent lane funding memo.
- `X402E:<corridor_id>:<uetr>`: Bilateral cross-border ISO 20022 linkage.
- `X402G:<challenge_uuid>`: Ingress micropayment challenge token.

### 2. On-Chain Instructions Sysvar Introspection

Smart contracts enforcing this standard do not accept unverified strings
from transaction arguments. Instead, the contract inspects the executing
transaction instruction pipeline through the `sysvar::instructions` account.

```rust
pub fn verify_typed_wire_linkage(
    ix_sysvar: &AccountInfo,
    expected_wire_memo: &[u8],
) -> ProgramResult {
    if ix_sysvar.key != &IX_SYSVAR_ID {
        return Err(ProgramError::InvalidArgument);
    }
    let current_index = load_current_index_checked(ix_sysvar)?;
    if current_index == 0 {
        return Err(ProgramError::Custom(0x04));
    }
    let preceding_ix: Instruction = load_instruction_at_checked(
        (current_index - 1) as usize,
        ix_sysvar,
    )?;
    if preceding_ix.program_id.to_bytes() != SPL_MEMO_V2_ID {
        return Err(ProgramError::Custom(0x05));
    }
    if preceding_ix.data != expected_wire_memo {
        return Err(ProgramError::Custom(0x06));
    }
    Ok(())
}
```

### 3. Token-2022 Account Invariant

Institutional settlement vaults initialize the native SPL Token-2022
extension `RequiredMemoTransfers`. This guarantees that even if an actor
attempts an out-of-band transfer that bypasses the contract, the Solana
runtime rejects the instruction at consensus level:

```rust
spl_token_2022::instruction::enable_required_memo_transfers(
    &spl_token_2022::id(),
    &vault_account,
    &vault_authority,
    &[],
)?;
```

## Impact

This proposal introduces no breaking changes to the Solana core protocol
or existing SPL programs. It establishes an interoperable interface standard
that enables enterprise banking engines, payment processors, and audit
gateways to safely execute cross-rail DvP settlement on Solana.

## Security Considerations

1. Replay & Decoy Resistance: Because the contract inspects the exact
   instruction data executed within the current transaction envelope,
   transactions cannot be replayed across different windows or corridors.
2. Trailing Byte Attacks: Standard implementations using prefix matching
   are vulnerable to memo extension exploits. This standard mandates strict
   slice equality (`preceding_ix.data == expected_wire_memo`).
3. Leading Zero Defense: On decimal fields, non-canonical representations
   trigger errors to prevent state hash collisions.
