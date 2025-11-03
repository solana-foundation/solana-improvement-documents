---
simd: '0360'
title: Epoch Stake Root (ESR) Sysvar
authors:
  - Alexander Atamanov (alexander@twinelabs.xyz)
category: Standard
type: Core
status: Idea
created: 2025-01-14
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

Introduce a new sysvar, Epoch Stake Root (ESR), that calculates a Merkle
root commitment to the finalized validator set and effective stake
distribution for the next epoch (N+1) and makes it available on-chain. The ESR
is written exactly once per epoch at the epoch boundary when the runtime
finalizes the upcoming epoch's effective stakes. The sysvar also includes a
minimum inclusion threshold and dual stake totals to support percentage
calculations.

---

## Motivation

Light clients and external verifiers need a compact, trust-minimized way to
follow validator-set transitions across epoch boundaries. Today,
reconstructing the validator set for epoch N+1 requires scanning thousands of
stake/vote accounts during epoch N and re-implementing stake warmup/cooldown
and activation/deactivation rate limits. This is data- and compute-intensive
and undermines lightweight clients.

ESR collapses this into a single, consensus-committed 32-byte root with
minimal metadata, enabling efficient verification on constrained devices and
cross-chain verifiers without duplicating runtime logic.

---

## New Terminology

- **Effective stake:** Activated stake per vote account after applying
  warmup/cooldown and global activation/deactivation rate limits at the epoch
  boundary.
- **Validator identifier:** Vote account public key (`vote_pubkey`).
- **Minimum inclusion threshold:** Validators with effective stake below this
  threshold are excluded from the ESR tree. Their stake still contributes to
  the global epoch total for percentage-of-all calculations.

---

## Detailed Design

### Sysvar

- **Name:** `EpochStakeRoot` (ESR)
- **Address:** `ESR111111111111111111111111111111111111111` (fixed sysvar
  address, base58).
- **Write semantics:** Runtime-owned; updated only at epoch transition
  `N → N+1`.
- **Access:** Read-only to programs; cached similarly to other sysvars.

Data layout (little-endian unless stated otherwise):

```text
struct EpochStakeRootV1 {
    u8     version;              // = 2 (ESRv1.1)
    u64    epoch;                // epoch the root applies to (N+1)
    [u8;32] root;                // Merkle root over INCLUDED leaves
    u64    leaf_count;           // number of INCLUDED validators
                                 // (stake >= min_stake_lamports)
    u128   total_active_stake;   // sum of ALL effective stakes across
                                 // ALL validators
    u128   tree_lamports;        // sum of INCLUDED stakes
                                 // (>= min_stake_lamports)
    u128   min_stake_lamports;   // threshold; below this stake excluded
                                 // from ESR tree
    u8     hash_id;              // 1 = SHA-256, 2 = Poseidon
                                 // (BN254 params)
}
```

Notes:

- `total_active_stake` is the canonical epoch total and must match the input
  used by leader-schedule generation.
- `tree_lamports` enables percentage calculations over the included subset.
- Inclusion threshold affects which leaves are in the Merkle tree but does not
  change the leaf hashing format.
- Two hash functions are supported:
  - SHA-256 (`hash_id: 1`) for broad compatibility with light clients and
    cross-chain verifiers, particularly Ethereum and other EVM-based chains.
  - Poseidon (`hash_id: 2`) for ZK-friendly verification using Solana's
    existing `solana-poseidon` implementation with BN254 curve parameters,
    enabling efficient inclusion proofs in zero-knowledge circuits for
    ZK-rollups and privacy-preserving applications.

### Merkle construction

#### SHA-256 (hash_id = 1)

- Leaf preimage (domain separated):

```text
leaf = SHA256(
  "ESRv1" ||              // ASCII, 5 bytes (domain separator)
  epoch_le_u64 ||         // epoch N+1 (8 bytes)
  vote_pubkey_32 ||       // validator vote account pubkey (32 bytes)
  stake_le_u128           // effective stake in lamports (16 bytes)
)
```

- Inner node: `SHA256(left || right)`.

#### Poseidon (hash_id = 2)

- Uses `light-poseidon` crate directly (already a dependency in Agave) for
  native runtime execution
- Leaf preimage (byte-based encoding with little-endian):

```rust
use light_poseidon::{Poseidon, PoseidonBytesHasher};
use ark_bn254::Fr;

// Create hasher for 4 inputs
let mut hasher = Poseidon::<Fr>::new_circom(4).unwrap();
leaf = hasher.hash_bytes_le(&[
    b"ESRv1",                 // Domain separator (5 bytes)
    &epoch.to_le_bytes(),     // Epoch N+1 (8 bytes)
    vote_pubkey.as_ref(),     // Vote pubkey (32 bytes)
    &stake.to_le_bytes(),     // Effective stake (16 bytes for u128)
]).unwrap()
```

- Inner node:

```rust
// Create hasher for 2 inputs (left and right child)
let mut hasher = Poseidon::<Fr>::new_circom(2).unwrap();
// Note: left and right are &[u8; 32]
hasher.hash_bytes_le(&[&left, &right]).unwrap()
```

- Configuration: BN254 curve with x^5 S-box, Circom-compatible parameters with
  width based on input count.

#### Common rules (both hash functions)

- Ordering: leaves are sorted lexicographically by `vote_pubkey` bytes
  (ascending) before tree construction.
- Inclusion rule: only validators with
  `effective_stake >= min_stake_lamports` are included.
- Root: standard binary Merkle root over ordered leaves.
- Proof format (for clients): `(leaf, index_u64, [sibling_hashes])`.
- Odd number of nodes: when a level has an odd number of nodes, the last node
  is duplicated (hashed with itself) to form a pair.

### Runtime algorithm

At each epoch boundary:

1. Finalize effective stakes for all vote accounts (existing logic).
2. Compute `total_active_stake` across all validators.
3. Filter entries where `effective_stake >= min_stake_lamports`.
4. Sort included entries by `vote_pubkey` ascending.
5. Select hash function based on network configuration (SHA-256 or Poseidon).
6. Compute leaves and the Merkle root using the selected hash function.
7. Compute `tree_lamports` as the sum of included stakes.
8. Write the `EpochStakeRootV1` struct to the ESR sysvar with appropriate `hash_id`.

Frequency: once per epoch (~2 days).  
Cost: `O(V)` hashing for `V` validators; negligible vs existing
epoch-transition work and trivially parallelizable. Since this runs in the
runtime (not as a program):

- SHA-256: Native execution using standard crypto libraries
- Poseidon: Native execution using `light-poseidon` crate, which is
  significantly faster than SHA-256 in native code (~18μs for 1 input, ~37μs
  for 3 inputs on modern hardware)

---

## Impact

Programs may read ESR to reference the committed validator distribution for
epoch `epoch` in constant time.

Light clients:

1. Obtain a (possibly untrusted) list of `(vote_pubkey, effective_stake)` for
   epoch `epoch` (e.g., via RPC snapshot, gossip, or archival node).
2. For any validator of interest with `stake >= min_stake_lamports`, verify
   Merkle inclusion against `root` using the provided proof (index and sibling
   hashes).
3. Compute percentages:
   - `percent_of_all_stake = stake / total_active_stake`.
   - `percent_of_included_set = stake / tree_lamports`.

Merkle proofs can be obtained from RPC nodes or computed locally from the full
validator list.

Continuity of trust across epochs is derived from vote signatures on the bank
hash that commits ESR (see Security Considerations).

---

## Rationale

- **Committing to the next-epoch (`N+1`) set:** Consumers must transition
  trust across epoch boundaries; ESR binds the validator set and weights that
  will be active in `N+1` at the moment `N` finalizes.
- **Minimal on-chain surface:** A single root plus summary metadata avoids
  large arrays or per-validator accounts.
- **Deterministic ordering and domain separation:** Prevent ambiguity and
  cross-epoch collisions.
- **Thresholding:** Reduces ESR size while preserving verifiable linkage to
  the full epoch total via `total_active_stake`.
- **Dual hash function support:**
  - **SHA-256:** Universal compatibility with existing infrastructure,
    hardware acceleration, and broad ecosystem support.
  - **Poseidon:** ZK-friendly for efficient inclusion proofs in zero-knowledge
    circuits. Uses `light-poseidon` v0.4.0 (already a dependency in Agave)
    with BN254 curve parameters. Poseidon is ~100x more efficient than SHA-256
    in ZK circuits, making it ideal for privacy-preserving bridges and
    ZK-rollup integration. Following
    [SIMD-0359](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0359-poseidon-enforce-input-length.md),
    strict input length validation is enforced.
- **Leveraging existing infrastructure:** Both hash functions are already
  available in the runtime - SHA-256 through standard libraries and Poseidon
  through the `light-poseidon` crate that's already used for the syscall
  implementation.
- **Future flexibility:** The `hash_id` field allows networks to choose the
  appropriate hash function based on their verification requirements without
  protocol changes.

---

## Backwards compatibility

- No changes to stake/vote programs or leader-schedule computation.
- Existing sysvars are unchanged; programs not using ESR are unaffected.
- Versioning (`version`, `hash_id`) allows future upgrades (e.g., different hash).

---

## Alternatives Considered

- Publish full validator list on-chain each epoch: higher storage/bandwidth;
  unnecessary for verification—root suffices.
- Commit to a single hash of the concatenated list (no Merkle): `O(V)`
  verification cost and no efficient inclusion proofs.
- Piggyback on existing sysvars (e.g., `StakeHistory`, `LeaderSchedule`): do
  not expose a cryptographic commitment to the entire `(validator → stake)`
  mapping.

---

## Drawbacks

- Introduces one additional hashing pass per epoch transition (~O(V) compute).
- Consumers still need data availability of the underlying validator list
  (from RPC/gossip/snapshots) to obtain or verify inclusion proofs, which is
  standard in light-client designs.
- Validators below `min_stake_lamports` cannot be proven via ESR, though they
  remain in `total_active_stake` for economic calculations.

---

## Security Considerations

- **State commitment:** ESR is stored in bank state and thus included in the
  bank hash.
- **Consensus proof:** Supermajority vote signatures on the bank hash provide
  a consensus proof for ESR, enabling cross-chain/off-chain verification.
- **Continuity:** Verifiers chain trust from epoch `N → N+1` by verifying
  (i) bank hash via validator signatures, (ii) ESR value included in that
  bank, and (iii) Merkle proofs for specific validators.
- **Replay/versioning:** `version` and `hash_id` fields allow future hash
  upgrades.
- **Canonical inputs:** Effective stakes must exactly match leader-schedule
  inputs to avoid interpretational forks. Runtime instrumentation should
  monitor for root divergences across nodes during epoch transitions.
- **Threshold safety:** Validators below `min_stake_lamports` are excluded
  from the tree but their stakes remain in `total_active_stake`, preserving
  the canonical epoch total for leader schedule and economic calculations.
- **Input validation:** When using Poseidon (hash_id=2), strict input length
  validation is enforced per SIMD-0359. Each input to the hash function must
  be ≤32 bytes to prevent potential security vulnerabilities.

---

## Performance and resource impact

- **Memory:** Reuses the epoch-stakes snapshot already present at epoch boundary.
- **Compute:** ~`V` leaf hashes + ~`V` inner hashes. With ~1–3k validators:
  - SHA-256: ~3-9ms total using native crypto libraries
  - Poseidon: ~18-54ms total using `light-poseidon` (still negligible for epoch transition)
  - Both are easily parallelized using Rayon or similar
- **Storage:** One small sysvar struct (~100 bytes); negligible.

---

## Testing plan

- Unit tests: ordering, leaf hashing format, domain separation, root stability
  across nodes, thresholding behavior, empty/small/large sets.
- Integration: end-to-end epoch transitions producing ESR and verifying
  against independently recomputed roots; feature-gated
  activation/deactivation across simulated epochs.
- Cluster tests: multi-node devnet clusters with real epoch transitions to
  verify ESR consistency and client-side proof validation.
- Fuzzing: serialization/endianness and proof verification harnesses.
- Edge cases: 0 validators, all stakes below threshold, maximum validator
  counts (~100k+).

---

## Activation

- Land behind a feature gate; enable on devnet, then testnet.
- After sufficient epochs without divergences and with instrumentation, roll
  out to mainnet-beta.

---

## Reference pseudocode

```rust
use light_poseidon::{Poseidon, PoseidonBytesHasher};
use ark_bn254::Fr;
use sha2::{Sha256, Digest};

fn write_esr_sysvar(epoch_n_plus_1: u64,
                    stakes_all: Vec<(Pubkey, u128)>,
                    min_stake_lamports: u128,
                    use_poseidon: bool) {
    // Canonical total for epoch and leader schedule
    let total_active_stake: u128 = stakes_all.iter().map(|(_, s)| *s).sum();

    // Apply inclusion threshold
    let mut included: Vec<(Pubkey, u128)> = stakes_all
        .into_iter()
        .filter(|(_, s)| *s >= min_stake_lamports)
        .collect();

    // Deterministic ordering
    included.sort_by(|(a,_),(b,_)| a.as_ref().cmp(b.as_ref()));

    // Compute leaf hashes based on selected hash function
    let (leaves, hash_id) = if use_poseidon {
        // Poseidon using light-poseidon crate
        let leaves: Vec<[u8; 32]> = included.iter().map(|(pk, stake)| {
            // Create hasher for 4 inputs
            let mut hasher = Poseidon::<Fr>::new_circom(4).unwrap();
            // Note: light-poseidon v0.4.0 enforces strict input length validation
            // Each input slice must be ≤32 bytes (per SIMD-0359)
            hasher.hash_bytes_le(&[
                b"ESRv1",                      // Domain separator (5 bytes)
                &epoch_n_plus_1.to_le_bytes(), // Epoch (8 bytes)
                pk.as_ref(),                   // Vote pubkey (32 bytes)
                &stake.to_le_bytes(),          // Stake (16 bytes for u128)
            ]).unwrap()
        }).collect();
        (leaves, 2) // hash_id = 2 for Poseidon
    } else {
        // SHA-256 with byte concatenation
        let leaves: Vec<[u8; 32]> = included.iter().map(|(pk, stake)| {
            let mut hasher = Sha256::new();
            hasher.update(b"ESRv1");                      // Domain separator
            hasher.update(&epoch_n_plus_1.to_le_bytes()); // Epoch
            hasher.update(pk.as_ref());                   // Vote pubkey
            hasher.update(&stake.to_le_bytes());          // Stake (u128)
            hasher.finalize().into()
        }).collect();
        (leaves, 1) // hash_id = 1 for SHA-256
    };

    // Build Merkle root using appropriate hash function
    let root = if use_poseidon {
        merkle_root_poseidon(&leaves)
    } else {
        merkle_root_sha256(&leaves)
    };
    
    let tree_lamports: u128 = included.iter().map(|(_, s)| *s).sum();

    let esr = EpochStakeRootV1 {
        version: 2, // ESRv1.1
        epoch: epoch_n_plus_1,
        root,
        leaf_count: included.len() as u64,
        total_active_stake,
        tree_lamports,
        min_stake_lamports,
        hash_id,
    };

    sysvar_write(SysvarId::EpochStakeRoot, &esr);
}

// Helper function to build Merkle root from leaves (SHA-256)
fn merkle_root_sha256(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }
    
    let mut current_level = leaves.to_vec();
    
    while current_level.len() > 1 {
        let mut next_level = Vec::new();
        
        for pair in current_level.chunks(2) {
            let hash = if pair.len() > 1 {
                let mut hasher = Sha256::new();
                hasher.update(&pair[0]);
                hasher.update(&pair[1]);
                hasher.finalize().into()
            } else {
                // Odd node: hash with itself
                let mut hasher = Sha256::new();
                hasher.update(&pair[0]);
                hasher.update(&pair[0]);
                hasher.finalize().into()
            };
            next_level.push(hash);
        }
        
        current_level = next_level;
    }
    
    current_level[0]
}

// Helper function to build Merkle root from leaves (Poseidon)
fn merkle_root_poseidon(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }
    
    let mut current_level = leaves.to_vec();
    
    while current_level.len() > 1 {
        let mut next_level = Vec::new();
        
        for pair in current_level.chunks(2) {
            let hash = if pair.len() > 1 {
                // Hash two nodes together
                let mut hasher = Poseidon::<Fr>::new_circom(2).unwrap();
                hasher.hash_bytes_le(&[&pair[0], &pair[1]]).unwrap()
            } else {
                // Odd node: hash with itself
                let mut hasher = Poseidon::<Fr>::new_circom(2).unwrap();
                hasher.hash_bytes_le(&[&pair[0], &pair[0]]).unwrap()
            };
            next_level.push(hash);
        }
        
        current_level = next_level;
    }
    
    current_level[0]
}

// Merkle proof structure
struct MerkleProof {
    siblings: Vec<Hash>,  // Sibling hashes on path to root
}

fn verify_validator_in_esr(esr: &EpochStakeRootV1,
                           vote_pubkey: Pubkey,
                           stake: u128,
                           proof: &MerkleProof,
                           index: u64) -> bool {
    if stake < esr.min_stake_lamports {
        return false; // not included in ESR tree
    }
    
    // Reconstruct leaf hash based on hash_id
    let mut current_hash = match esr.hash_id {
        1 => {
            // SHA-256 with byte concatenation
            let mut hasher = Sha256::new();
            hasher.update(b"ESRv1");                    // Domain separator
            hasher.update(&esr.epoch.to_le_bytes());    // Epoch
            hasher.update(vote_pubkey.as_ref());        // Vote pubkey
            hasher.update(&stake.to_le_bytes());        // Stake (u128)
            hasher.finalize().into()
        },
        2 => {
            // Poseidon using light-poseidon
            let mut hasher = Poseidon::<Fr>::new_circom(4).unwrap();
            hasher.hash_bytes_le(&[
                b"ESRv1",                    // Domain separator (5 bytes)
                &esr.epoch.to_le_bytes(),    // Epoch (8 bytes)
                vote_pubkey.as_ref(),        // Vote pubkey (32 bytes)
                &stake.to_le_bytes(),        // Stake (16 bytes for u128)
            ]).unwrap()
        },
        _ => return false, // Unknown hash_id
    };
    
    // Traverse up the tree using sibling hashes
    let mut current_index = index;
    for sibling in &proof.siblings {
        current_hash = match esr.hash_id {
            1 => {
                // SHA-256
                let mut hasher = Sha256::new();
                if current_index % 2 == 0 {
                    hasher.update(&current_hash);
                    hasher.update(sibling);
                } else {
                    hasher.update(sibling);
                    hasher.update(&current_hash);
                }
                hasher.finalize().into()
            },
            2 => {
                // Poseidon
                let mut hasher = Poseidon::<Fr>::new_circom(2).unwrap();
                if current_index % 2 == 0 {
                    hasher.hash_bytes_le(&[&current_hash, sibling]).unwrap()
                } else {
                    hasher.hash_bytes_le(&[sibling, &current_hash]).unwrap()
                }
            },
            _ => return false,
        };
        current_index /= 2;
    }
    
    current_hash == esr.root
}
```

**Implementation notes:**

- Uses `sha2` crate for SHA-256 hashing (standard in Rust ecosystem)
- Uses `light-poseidon` v0.4.0 with `ark-bn254` for BN254 field arithmetic
  (already dependencies in Agave)
- The `new_circom(n)` constructor creates a Poseidon hasher for `n` inputs
  with Circom-compatible parameters
- `hash_bytes_le(&[&[u8]])` takes a slice of byte slices and handles field
  element conversion internally
- Each input slice must be ≤32 bytes to fit within the BN254 field (enforced
  by SIMD-0359)
- All multi-byte integers use little-endian encoding (`to_le_bytes()`) for
  consistency with Solana's native endianness
- Domain separator `"ESRv1"` (5 ASCII bytes) prevents cross-protocol hash
  collisions
- The Poseidon implementation automatically handles field element conversions
  and modular arithmetic

---

## Interaction with other proposals

### SIMD-0326 (Alpenglow consensus protocol)

Technical compatibility analysis:

**Consensus layer separation:**

- ESR operates as a data commitment layer independent of consensus mechanism
- Sysvar write occurs at epoch boundary regardless of consensus protocol
- Bank hash inclusion remains unchanged between TowerBFT and Alpenglow

**Validator set constraints:**

- Alpenglow enforces maximum 2,000 validators via admission control
- ESR tree construction scales linearly with validator count
- Minimum inclusion threshold (`min_stake_lamports`) aligns with Alpenglow's
  Validator Admission Ticket (VAT) mechanism

**Stake calculation compatibility:**

- Both proposals use identical effective stake definitions from existing stake
  program
- `total_active_stake` computation remains canonical for leader schedule
  generation
- Alpenglow's validator removal (zero rewards) reflected in next epoch's ESR

**Finality implications:**

- Alpenglow fast-finalization (80% stake, 1 round) reduces ESR commitment
  latency
- Slow-finalization (60% stake, 2 rounds) provides equivalent security
  guarantees
- Certificate-based consensus enables potential proof aggregation with ESR
  Merkle proofs

**Implementation considerations:**

- ESR computation occurs after Alpenglow's epoch validator admission
- No modification required to ESR's Merkle construction or hash functions
- Vote account changes (off-chain votes in Alpenglow) do not affect ESR
  calculation

---

## References

- SIMD process and format guidance:
  [Solana Improvement Documents – SIMD Process](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0001-simd-process.md)
- SIMD-0326 Alpenglow consensus:
  [SIMD-0326](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0326-alpenglow.md)
- SIMD-0359 Poseidon input length enforcement:
  [SIMD-0359](https://github.com/solana-foundation/solana-improvement-documents/blob/main/proposals/0359-poseidon-enforce-input-length.md)
- Solana epoch and stake mechanics:
  [Solana Documentation - Staking](https://docs.solana.com/staking)
- Merkle tree construction:
  [RFC 6962 - Certificate Transparency](https://datatracker.ietf.org/doc/html/rfc6962)
- Poseidon hash function:
  [Poseidon: A New Hash Function for Zero-Knowledge Proof Systems](https://eprint.iacr.org/2019/458.pdf)
- light-poseidon implementation:
  [Light Protocol Poseidon](https://github.com/Lightprotocol/light-poseidon)
- BN254 curve and field parameters:
  [EIP-197: Precompiled contracts for optimal ate pairing check on the
  elliptic curve alt_bn128](https://eips.ethereum.org/EIPS/eip-197)


