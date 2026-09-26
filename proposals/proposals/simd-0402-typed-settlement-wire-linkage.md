---
simd: '<to be assigned by editors>'
title: Typed Settlement Wire Linkage via SPL Memo v2 and Token-2022 Introspection
authors:
  - Abdul Shabazz (veritasvaultone@gmail.com)
category: Standards Track
type: Application
status: Draft
created: 2026-09-26
feature: none
---

## Summary

This proposal standardizes the **Typed Settlement Wire Protocol (X402-TSWP)** for Solana applications, defining a deterministic method to bind external payment references (such as SWIFT ISO 20022 RFC 4122 UUIDv4 UETRs) directly to on-chain state transitions. 

The standard couples:
1. Canonical typed wire memos carried by the official SPL Memo v2 program (`MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr`);
2. Zero-regex on-chain instruction verification via the Solana Instructions Sysvar (`sysvar::instructions`); and
3. Account-level transfer gating using the SPL Token-2022 `RequiredMemoTransfers` extension.

---

## Motivation

Institutional settlement, interbank foreign exchange, and corporate treasury systems mandate end-to-end auditability against unique external transaction identifiers. Under SWIFT CBPR+ (ISO 20022 `pacs.008.001.08`), every transaction requires an immutable 128-bit UETR.

Prior art decentralized applications on Solana face three systemic limitations:
1. **Off-Chain Desynchronization:** Decoupling transaction signatures from banking references into off-chain relational databases creates race conditions, orphan transfers, and non-repudiation vulnerabilities.
2. **Regex Parsing Fragility:** Parsing transaction memos using off-chain regular expressions exposes listeners to spoofing via leading or trailing garbage byte injection.
3. **Execution Pipeline Isolation:** Standard SPL token transfers do not cryptographically verify whether an attached memo succeeded or matched expected parameters within the same atomic transaction pipeline.

This proposal specifies an on-chain execution discipline that guarantees no settlement instruction can finalize without an atomic, byte-exact memo verified directly from execution memory.

---

## Detailed Design

### 1. The Typed Memo Wire Grammar
All memos conform to strict 7-bit ASCII, using standard base-10 representations without leading zeros:

```
X402<Type>:<Payload>
```

| Type Tag | Grammar | Semantics |
|---|---|---|
| `X402W` | `X402W:<window_epoch>:<uetr>` | Net-window settlement binding SWIFT UETR to custody release. |
| `X402L` | `X402L:<lane_id>:<window_epoch>:<nonce>` | Concurrent lane funding memo for 256-lane parallel execution. Max 50 bytes. |
| `X402E` | `X402E:<corridor_id>:<uetr>` | Bilateral cross-border ISO 20022 wire linkage. |
| `X402G` | `X402G:<challenge_uuid>` | Ingress micropayment challenge token. |

### 2. On-Chain Instructions Sysvar Introspection
Smart contracts enforcing this standard do not accept unverified strings from transaction arguments. Instead, the contract inspects the executing transaction's instruction pipeline through the `sysvar::instructions` account.

#### Verification Routine (Rust SBF):
```rust
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::Instruction,
    program_error::ProgramError,
    sysvar::instructions::{
        load_current_index_checked, load_instruction_at_checked, ID as IX_SYSVAR_ID,
    },
};

pub const SPL_MEMO_V2_ID: [u8; 32] = [
    0x05, 0x4a, 0x53, 0x5a, 0x99, 0x29, 0x21, 0x06,
    0x4d, 0x24, 0xe8, 0x71, 0x60, 0xda, 0x38, 0x7c,
    0x7c, 0x35, 0xb5, 0xdd, 0xbc, 0x23, 0x70, 0x57,
    0xbc, 0xa3, 0x3a, 0x5a, 0xfc, 0xe6, 0x45, 0x79,
];

pub fn verify_typed_wire_linkage(
    ix_sysvar: &AccountInfo,
    expected_wire_memo: &[u8],
) -> ProgramResult {
    // 1. Verify the instructions sysvar address
    if ix_sysvar.key != &IX_SYSVAR_ID {
        return Err(ProgramError::InvalidArgument);
    }

    // 2. Fetch current executing instruction index
    let current_index = load_current_index_checked(ix_sysvar)?;
    if current_index == 0 {
        return Err(ProgramError::Custom(0x04)); // MEMO_LINKAGE_MISSING
    }

    // 3. Inspect preceding instruction in the atomic pipeline
    let preceding_ix: Instruction = load_instruction_at_checked(
        (current_index - 1) as usize,
        ix_sysvar,
    )?;

    // 4. Verify program identity (must be official SPL Memo v2)
    if preceding_ix.program_id.to_bytes() != SPL_MEMO_V2_ID {
        return Err(ProgramError::Custom(0x05)); // INVALID_MEMO_PROGRAM
    }

    // 5. Byte-exact comparison against constructed expected wire memo
    if preceding_ix.data != expected_wire_memo {
        return Err(ProgramError::Custom(0x06)); // MEMO_PAYLOAD_MISMATCH
    }

    Ok(())
}
```

### 3. Token-2022 Account Invariant
Institutional settlement vaults MUST initialize the native SPL Token-2022 extension `RequiredMemoTransfers`. This guarantees that even if an actor attempts an out-of-band transfer that bypasses the contract, the Solana runtime rejects the instruction at consensus level:

```rust
spl_token_2022::instruction::enable_required_memo_transfers(
    &spl_token_2022::id(),
    &vault_account,
    &vault_authority,
    &[],
)?;
```

---

## Security Considerations

1. **Replay & Decoy Resistance:** Because the contract inspects the exact instruction data executed within the current transaction envelope, transactions cannot be replayed across different windows or corridors.
2. **Trailing Byte Attacks:** Standard implementations using prefix matching (`starts_with`) are vulnerable to memo extension exploits. This standard mandates strict slice equality (`preceding_ix.data == expected_wire_memo`).
3. **Leading Zero Defense:** On decimal fields (`lane`, `window`, `nonce`), non-canonical representations (e.g., `0151` instead of `151`) trigger `MEMO_PAYLOAD_MISMATCH` to prevent hash splitting.

---

## Reference Implementation & Test Vectors

- **Production SBF Program:** `GhKAvDhsZ4Ph8vySxh9AfKfTM3MGgNCvQdxFqn6nHdRn` (Deployed on Solana Devnet)
- **Token-2022 Protected Vault:** `BnuCTFWFLLXnSPv2Frs42royiTAYG87WP7p1zRLB4ksG`
- **Conformance Test Suite:** Verified 4/4 passing vectors in `@synaptics/x402-tswp` (`test/vectors.test.mjs`)
- **Upstream Standards Document:** IETF Datatracker [`draft-shabazz-http-x402-tswp-00`](https://datatracker.ietf.org/doc/draft-shabazz-http-x402-tswp/)
- **Defensive Prior Art Registration:** Zenodo DOI [`10.5281/zenodo.22979715`](https://doi.org/10.5281/zenodo.22979715)
- **Reference Repository:** [https://github.com/Synaptics-Lab/x402-tswp](https://github.com/Synaptics-Lab/x402-tswp)
