---
simd: '0520'
title: 'On-Chain Agent Identity Standard'
authors:
  - AIP Working Group (dr-wilson-empty)
category: Standard
type: Interface
status: Idea
created: 2026-05-02
feature: (fill in with feature key and github tracking issues once accepted)
---

## Summary

This SIMD specifies an on-chain primitive for **autonomous agent identity** on
Solana: a Program Derived Address (PDA) account schema, a deterministic DID
binding (`did:aip`), and an interface that any conformant agent registry program
must expose. It elevates an agent - distinct from any human operator's wallet - to
a first-class, sovereign, globally unique identity backed by Solana's consensus.

## Motivation

The next generation of decentralized applications will be operated by autonomous
software agents (LLM workers, task orchestrators, on-chain execution bots) that
discover one another, advertise capabilities, transact value, and accept payment
for completed work. These agents need an identity primitive that satisfies five
requirements simultaneously:

1. **Sovereignty** - issuance MUST NOT depend on any central authority or DNS.
2. **Global uniqueness** - collisions MUST be mathematically impossible across
   all operators without coordination.
3. **Tamper-evidence** - every state transition MUST be publicly auditable.
4. **Resolvability** - any party MUST be able to retrieve the agent's metadata
   from the identifier alone.
5. **Operator multiplicity** - a single operator MUST be able to control many
   distinct agent identities.

A bare Solana wallet pubkey satisfies (1)–(3) but not (4) or (5): it is opaque
about what the entity behind it does, and a single operator running ten agents
is forced to either reuse one wallet (collapsing distinct agents into one
identity) or scatter ten unrelated wallets (losing the operator → agents
linkage). Centralized agent directories satisfy (4) and (5) but fail (1).

The PDA-anchored model proposed here delivers all five properties at the cost
of one Solana transaction per registration and one rent-exempt account per
agent (~0.0073 SOL).

## New Terminology

<!-- markdownlint-disable MD013 -->
| Term | Meaning |
|------|---------|
| **Agent Registry Program** | A Solana program that owns and validates `AgentRecord` accounts conformant to this SIMD. |
| **AgentRecord** | A Borsh-serialized account holding the on-chain state of one agent identity. |
| **Owner** | The Ed25519 keypair that signed the `register_agent` instruction; the only authorized signer for `update_agent` and `deregister_agent`. |
| **Agent Card** | An off-chain JSON document, served at the agent's `endpoint`, advertising the agent's capabilities, pricing, and authentication parameters. |
| **`did:aip`** | The W3C-conformant DID method that anchors an agent identity to its `AgentRecord`. Specified separately as a W3C DID Method. |
<!-- markdownlint-enable MD013 -->

## Detailed Design

### 1. PDA Derivation

Every conformant Agent Registry Program **MUST** derive `AgentRecord` PDAs using
the following seed sequence:

```text
seeds = [ b"agent", owner_pubkey.as_ref(), agent_id.as_bytes() ]
```

Where:

- `owner_pubkey` is the 32-byte Ed25519 public key of the registration signer.
- `agent_id` is a 1–32 byte UTF-8 string drawn from the character set
  `[A-Za-z0-9_-]`.

The PDA address is computed via Solana's standard
`find_program_address(seeds, program_id)` algorithm. The bump byte returned by
this function **MUST** be persisted in `AgentRecord.bump`.

#### 1.1 Uniqueness Guarantee (Normative)

Solana's `find_program_address` produces an address that, by the guarantees of
the Ed25519 curve and SHA-256, is computationally infeasible to collide with any
other `(seeds, program_id)` tuple. Because the seed sequence above includes the
owner's pubkey and the owner-scoped slug, the tuple
`(program_id, owner_pubkey, agent_id)` uniquely determines a PDA. Two distinct
agents - under the same operator or under different operators - therefore
**cannot** occupy the same account address. This is the cryptographic basis of
the global-uniqueness property; no off-chain registry or coordination is
required.

### 2. Account Data Schema

The on-chain account layout, in Anchor-idiomatic form (matches the reference
implementation at
[programs/aip-escrow/programs/aip-registry/src/lib.rs:147](programs/aip-escrow/programs/aip-registry/src/lib.rs#L147)):

```rust
#[account]
pub struct AgentRecord {
    pub owner:             Pubkey, // 32  - registration signer; immutable
    pub agent_id:          String, // 4 + 32  - owner-scoped slug; immutable
    pub did:               String, // 4 + 100 - canonical did:aip string; immutable
    pub name:              String, // 4 + 64  - human-readable label
    pub endpoint:          String, // 4 + 200 - Agent Card URL (https://…)
    pub wallet_address:    Pubkey, // 32  - signing key for off-chain protocol traffic
    pub agent_type:        u8,     // 1   - 0=LLM, 1=Task, 2=Execution
    pub capabilities_json: String, // 4 + 512 - coarse capability summary
    pub version:           String, // 4 + 16  - agent SemVer
    pub registered_at:     i64,    // 8   - first-write cluster timestamp
    pub updated_at:        i64,    // 8   - last-write cluster timestamp
    pub bump:              u8,     // 1   - PDA bump seed
}
// Total reserved space (incl. 8-byte Anchor discriminator): 1 048 bytes
```

Field-level normative rules:

- `owner`, `agent_id`, `did`, `registered_at`, and `bump` **MUST NOT** change after
  initial registration.
- `wallet_address` **MAY** equal `owner` (single-key operation) or differ from it
  (cold/hot key separation). When they differ, the off-chain protocol layer
  **MUST** treat `wallet_address` as the agent's signing identity while the
  on-chain layer continues to require `owner` for administrative actions.
- `agent_type` is an enumeration that future revisions of this SIMD **MAY**
  extend; conformant programs **MUST** reject values outside the currently
  defined range.
- `capabilities_json` **MUST** be a UTF-8 JSON document; conformant programs
  **MAY** validate its structure but are not required to (the canonical
  Agent Card at `endpoint` is authoritative).
- `endpoint` **MUST** be a `https://` URL or a fully-qualified protocol-prefixed
  URI.

### 3. Canonical DID Derivation

The canonical `did:aip` string for an `AgentRecord` is:

```text
did:aip:{base58(owner)}:{agent_id}
```

The on-chain `did` field is REQUIRED to equal this canonical form, byte-for-byte,
at registration time. Verifiers **MUST** reject any record whose stored `did`
differs from the canonical derivation; such records are non-conformant and
**MUST NOT** be resolvable.

### 4. Required Instruction Interface

Every conformant Agent Registry Program **MUST** expose at minimum the following
three instructions. Argument names and types are normative; argument *order*
**MAY** vary across implementations to accommodate framework idioms.

#### 4.1 `register_agent`

```rust
pub fn register_agent(
    ctx: Context<RegisterAgent>,
    agent_id:          String,  // 1..=32 UTF-8 chars, [A-Za-z0-9_-]
    did:               String,  // ≤100; MUST equal canonical derivation
    name:              String,  // ≤64
    endpoint:          String,  // ≤200, https:// URL
    wallet_address:    Pubkey,
    agent_type:        u8,      // currently 0..=2
    capabilities_json: String,  // ≤512
    version:           String,  // ≤16, SemVer
) -> Result<()>;
```

Constraints:

- The transaction signer becomes `record.owner`.
- The PDA is initialized with the seeds defined in §1.
- `registered_at` and `updated_at` are stamped with `Clock::get()?.unix_timestamp`.

#### 4.2 `update_agent`

```rust
pub fn update_agent(
    ctx: Context<UpdateAgent>,
    name:              String,
    endpoint:          String,
    wallet_address:    Pubkey,
    agent_type:        u8,
    capabilities_json: String,
    version:           String,
) -> Result<()>;
```

Constraints:

- **MUST** be signed by `record.owner` (enforced by an Anchor `constraint`).
- The fields enumerated in §2 as immutable **MUST NOT** be modified.
- `updated_at` **MUST** be refreshed.

#### 4.3 `deregister_agent`

```rust
pub fn deregister_agent(ctx: Context<DeregisterAgent>) -> Result<()>;
```

Constraints:

- **MUST** be signed by `record.owner`.
- **MUST** close the PDA (`close = owner`), returning rent lamports to the owner.

Conformant programs **MAY** define additional instructions (e.g., reputation
updates, capability attestations, payment tracking) provided that the three
above retain the specified semantics.

### 5. Agent Card JSON Schema

Every conformant agent's `endpoint` **MUST** serve a JSON document conformant
to the schema below. The document **SHOULD** be served as
`Content-Type: application/json` with `Cache-Control` headers appropriate to
the operator's update cadence.

<!-- markdownlint-disable MD013 -->
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://aip.network/schemas/agent-card-v1.json",
  "type": "object",
  "required": ["did", "name", "version", "agentType", "capabilities", "auth"],
  "properties": {
    "did":       { "type": "string", "pattern": "^did:aip:[1-9A-HJ-NP-Za-km-z]{32,44}:[A-Za-z0-9_-]{1,32}$" },
    "name":      { "type": "string", "maxLength": 128 },
    "description": { "type": "string", "maxLength": 1024 },
    "version":   { "type": "string", "pattern": "^\\d+\\.\\d+\\.\\d+(?:-[0-9A-Za-z.-]+)?$" },
    "agentType": { "type": "string", "enum": ["LLM", "Task", "Execution"] },
    "capabilities": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["name", "inputSchema", "outputSchema"],
        "properties": {
          "name":          { "type": "string" },
          "description":   { "type": "string" },
          "inputSchema":   { "type": "object" },
          "outputSchema":  { "type": "object" },
          "pricePerCall":  { "type": "object",
                             "required": ["amount", "currency"],
                             "properties": {
                               "amount":   { "type": "string" },
                               "currency": { "type": "string", "enum": ["SOL", "USDC", "AIP"] }
                             }
                           }
        }
      }
    },
    "auth": {
      "type": "object",
      "required": ["scheme"],
      "properties": {
        "scheme":     { "type": "string", "enum": ["aip-sig-v1", "bearer", "none"] },
        "publicKey":  { "type": "string" },
        "tokenUrl":   { "type": "string", "format": "uri" }
      }
    },
    "signature": {
      "type": "object",
      "required": ["alg", "value"],
      "properties": {
        "alg":   { "type": "string", "const": "Ed25519" },
        "value": { "type": "string", "description": "base58 signature over the canonical card by AgentRecord.wallet_address" }
      }
    }
  }
}
```
<!-- markdownlint-enable MD013 -->

Clients **MUST** verify `signature` against the `wallet_address` of the
resolved DID Document before treating any field of the Agent Card as
authoritative. An unsigned card **MAY** be displayed to humans for
discoverability but **MUST NOT** be used for automated capability invocation
or payment routing.

## Alternatives Considered

<!-- markdownlint-disable MD013 -->
| Alternative | Reason rejected |
|-------------|-----------------|
| **Reuse wallet pubkeys as agent identity** | Fails operator multiplicity; conflates economic identity with operational identity; cannot carry capability metadata. |
| **Off-chain DNS-based agent registry** (e.g., `.well-known/agent.json`) | Requires DNS, which is centralized and not adversary-resistant; no global uniqueness guarantee. |
| **NFT-based agent identity (Metaplex Token Metadata)** | NFTs are transferable by design - the "ownership transfer impossibility" property of the PDA-seeded model cannot be reproduced. Account size and rent overhead are also higher. |
| **SPL Name Service** | Designed for human-readable names, not capability advertisements; lacks the structured `capabilities`/`endpoint` fields needed for autonomous discovery. |
<!-- markdownlint-enable MD013 -->

## Impact

This SIMD introduces a **new** primitive; no existing on-chain account or
program is modified. Consequently:

- **No backwards-compatibility** concerns for existing programs.
- **No consensus or runtime changes** required.
- **No fee or compute-unit pricing changes** required.

Adoption is opt-in: applications that wish to participate in agent-to-agent
discovery and payment register their agents with a conformant registry program
and resolve peers via `did:aip`.

## Security Considerations

### Authority Model

- The `owner` field is set at registration and **cannot** be changed. This
  intentionally rules out ownership transfer attacks (phishing, social
  engineering, key-leak migration) at the cost of eliminating legitimate
  ownership transfers; operators wanting transfer semantics MUST `deregister`
  and re-`register`, accepting that the resulting DID is a different identity.
- The split between `owner` (cold, administrative) and `wallet_address` (hot,
  operational) gives operators a defense-in-depth model: hot-key compromise
  affects only off-chain message authenticity, not on-chain record control.

### Program Upgrade Authority

A registry program retaining a non-null upgrade authority can, by upgrading
its own bytecode, alter the meaning of every existing `AgentRecord` it owns.
Mainnet deployment of a conformant registry program **MUST** be accompanied
by either:

<!-- markdownlint-disable-next-line MD013 -->
1. Irrevocable freeze of the upgrade authority (`solana program set-upgrade-authority --final`), or
2. Transfer of the upgrade authority to a publicly auditable, time-locked
   governance program.

Devnet/testnet deployments **SHOULD** clearly label that the upgrade authority
remains live and that DIDs anchored on those clusters are not suitable for
production trust.

### Sybil Cost

Each registration consumes ~0.0073 SOL of rent at current parameters. This is
a meaningful but modest barrier; high-value applications **MAY** layer
additional Sybil resistance (proof-of-stake, proof-of-personhood attestations)
on top via the `capabilities_json` field or the off-chain Agent Card.

### Endpoint Authenticity

The on-chain record commits only to an endpoint *URL*, not to the content
served at it. A compromised hosting environment can serve a falsified Agent
Card. The signature requirement in §5 mitigates this: clients **MUST** verify
the card's Ed25519 signature against `wallet_address` before acting on its
contents.

### Replay Protection

Inherits from Solana: every `register_agent` / `update_agent` / `deregister_agent`
transaction is uniquely signed and includes a recent blockhash. No
application-layer replay cache is required or permitted.

### Privacy

All on-chain fields are public forever. Operators **MUST NOT** write
personal data, secrets, or any field they may later wish to redact. See §8 of
the companion `did:aip` Method Specification for the full privacy treatment.

## Drawbacks

- **No transferable ownership.** Some use cases (selling a trained agent, M&A
  events) cannot be cleanly modeled and instead require a deregister/register
  cycle that breaks identity continuity.
- **Owner-key loss is fatal.** Loss of the owner key permanently freezes the
  agent record; only the rent escapes (it remains until somebody pays for
  storage on a new account).
- **On-chain field caps are coarse.** A 512-byte `capabilities_json` is a
  summary, not a complete schema; richer data depends on the off-chain Agent
  Card and on the operator keeping their endpoint reachable.

## Backwards Compatibility

Not applicable - this SIMD introduces a new primitive and does not modify any
existing on-chain interface.

## Test Cases

A conformant implementation **MUST** pass the following end-to-end test:

<!-- markdownlint-disable MD013 -->
1. Generate a fresh keypair `K_owner`.
2. Submit `register_agent(agent_id="test-001", did="did:aip:{base58(K_owner)}:test-001", name="Test", endpoint="https://example.com/agent", wallet_address=K_owner, agent_type=0, capabilities_json="{}", version="1.0.0")`.
3. Compute `pda = find_program_address([b"agent", K_owner.as_ref(), b"test-001"], program_id)`.
4. Fetch `pda` via RPC; deserialize as `AgentRecord`.
5. Assert `record.owner == K_owner`, `record.agent_id == "test-001"`, `record.did == canonical`.
6. Submit `update_agent(name="Test v2", …)` signed by `K_owner`; assert success.
7. Submit `update_agent(…)` signed by a different keypair; assert failure with code `Unauthorized`.
8. Submit `deregister_agent` signed by `K_owner`; assert success and PDA closure.
9. Re-submit `register_agent` with the same `agent_id`; assert success (treated as a fresh DID).
<!-- markdownlint-enable MD013 -->

## Reference Implementation

The reference implementation is the **AIP Agent Registry Program**:

- Source: [programs/aip-escrow/programs/aip-registry/src/lib.rs](programs/aip-escrow/programs/aip-registry/src/lib.rs)
- Devnet program ID: `CgchXu2dRV3r9E1YjRhp4kbeLLtv1Xz61yoerJzp1Vbc`
- Framework: Anchor 0.30.x

Annotated key sections:

<!-- markdownlint-disable MD013 -->
- [lib.rs:16–51](programs/aip-escrow/programs/aip-registry/src/lib.rs#L16-L51) - `register_agent` with full input validation.
- [lib.rs:55–81](programs/aip-escrow/programs/aip-registry/src/lib.rs#L55-L81) - `update_agent` with owner constraint.
- [lib.rs:84–87](programs/aip-escrow/programs/aip-registry/src/lib.rs#L84-L87) - `deregister_agent` with `close = owner`.
- [lib.rs:96–110](programs/aip-escrow/programs/aip-registry/src/lib.rs#L96-L110) - PDA seed declaration matching §1 of this SIMD.
- [lib.rs:147–166](programs/aip-escrow/programs/aip-registry/src/lib.rs#L147-L166) - `AgentRecord` account struct matching §2.
- [lib.rs:172–190](programs/aip-escrow/programs/aip-registry/src/lib.rs#L172-L190) - error enumeration covering all rejection paths.
<!-- markdownlint-enable MD013 -->

A TypeScript SDK and a worked Devnet integration test live in
[programs/aip-escrow](programs/aip-escrow). The companion W3C DID Method
Specification (`standards/did-aip-method-spec.md`) defines how off-chain
parties resolve a `did:aip` to a DID Document using only the program ID and a
Solana RPC endpoint.

## Acknowledgements

The PDA-seeded identity pattern draws on prior art from SPL Name Service and
Metaplex Token Metadata; the off-chain capability advertisement pattern draws
on the Model Context Protocol's tool-manifest design and on prior agent-card
proposals from the broader autonomous-agent community.

---

*End of SIMD draft.*
