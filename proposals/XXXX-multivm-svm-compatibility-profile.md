---
Title: "sRFC: Multi-VM SVM Compatibility Profile"
Author: Hilal Agil (@hilarl) <hilal@tenzro.com>
Status: Draft
Created: 2026-05-02
---

# sRFC: Multi-VM SVM Compatibility Profile

## Summary

This sRFC proposes a **declarative compatibility profile** that any
non-Solana chain running the Solana Virtual Machine (SVM) — whether
as a standalone L1, an L2 settling to another chain, or one VM among
several in a multi-VM runtime — publishes alongside its genesis to
describe **which parts of the Solana mainnet specification it
preserves, subsets, or diverges from**. It defines three conformance
levels (`svm-strict`, `svm-subset`, `svm-derived`) and a fixed set of
**required declarations** covering syscalls, BPF Loader versions,
ComputeBudget defaults, SPL Token semantics, ATA derivation, Address
Lookup Tables, Versioned Transactions, and genesis hash strategy.
The goal is to give wallet, RPC, indexer, and SDK authors an
unambiguous, machine-readable basis for deciding whether existing
Solana tooling will work against a given SVM chain — and where it
will not.

## Motivation

In 2026 the SVM has become a portable execution environment. Beyond
Solana mainnet itself, multiple production and pre-production chains
run SVM in some form:

- **Eclipse Mainnet** — SVM L2 settling to Ethereum, with Celestia
  data availability and documented divergence from Solana mainnet
  (notably: ETH-denominated fees rather than SOL).
- **Tenzro Network** — multi-VM L1 where SVM is one of three
  executors (alongside EVM via revm and Canton/DAML), implemented on
  top of `solana-sbpf` (the Anza fork of `solana_rbpf`).
- Several earlier and announced projects, including SOON, Termina,
  Atlas, MagicBlock ephemeral rollups, and Sonic SVM.

These chains do not all support the same syscalls, the same SPL Token
Program version, the same ATA derivation, or the same ComputeBudget
defaults. Some declare their divergence in product documentation;
others do not. The result is **silent compatibility breakage**: an
SDK that works on Solana mainnet may produce transactions that fail
on a self-described "SVM-compatible" chain, with no machine-readable
signal explaining why.

There is currently **no formal compatibility profile** that lets a
chain author make explicit, normative claims about which parts of the
Solana mainnet specification they preserve. This sRFC proposes one.

The proposal is intentionally a **discussion-starter for application
standards**, not a normative core protocol change to Solana mainnet.
sRFCs do not have a formal acceptance gate; the value here is in
producing a shared vocabulary that wallet and tooling authors can
point to when deciding what to support.

The terms **MUST**, **SHOULD**, **MAY**, **MUST NOT**, **SHOULD NOT**,
and **REQUIRED** in this document are to be interpreted as described
in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119) and
[RFC 8174](https://www.rfc-editor.org/rfc/rfc8174).

## Specification

### 1. Conformance Levels

A chain claiming SVM compatibility **MUST** declare exactly one of
three conformance levels in its public chain metadata:

#### 1.1. `svm-strict`

The chain preserves Solana mainnet's observable behavior to the
extent that any program deployed on Solana mainnet, compiled against
the same Anza Agave runtime, and submitted with an unmodified
mainnet-targeting client, would execute with **byte-identical state
transitions and account writes** modulo:

- block hash, slot number, and chain-identifying fields,
- the set of validator/leader keys, and
- chain-specific genesis allocations.

A `svm-strict` chain **MUST**:

- accept the full Solana mainnet syscall set as of the declared Anza
  Agave version,
- accept BPF Loader v1, v2, and v3 with the same activation rules as
  mainnet,
- expose `Token` (SPL Token v3) and `Token-2022` programs at their
  canonical mainnet addresses with semantics unchanged,
- derive Associated Token Accounts via the canonical
  `AssociatedTokenAccount` program at its canonical mainnet address,
- expose ComputeBudget Program at its canonical mainnet address with
  the same default and maximum CU values,
- support Address Lookup Tables and Versioned Transactions
  (v0 messages) per current mainnet behavior.

#### 1.2. `svm-subset`

The chain preserves Solana mainnet semantics for a **declared subset**
of the surface area but explicitly omits or restricts other parts.
The subset **MUST** be enumerated in the declarations (Section 2);
unenumerated surface area is treated as "not supported."

A `svm-subset` chain **MUST NOT** silently change semantics of a
declared-supported feature. If `Token-2022` is declared supported,
all of its extensions either work as on mainnet or each
unsupported-extension is enumerated.

#### 1.3. `svm-derived`

The chain runs an SVM-derived runtime but knowingly diverges from
Solana mainnet semantics for one or more in-scope features. Examples
include: a different SPL Token decimals truncation rule, a
non-canonical ATA derivation, a different fee asset, or
host-extension syscalls not present on mainnet.

A `svm-derived` chain **MUST** enumerate every divergence in its
declarations and **MUST NOT** market itself as
"Solana-compatible" without the `-derived` qualifier in machine-
readable contexts (chain metadata, CAIP-2 namespace registry,
explorer self-description).

### 2. Required Declarations

Every SVM-compatible chain (any conformance level) **MUST** publish
the following declarations in a stable, machine-readable form. The
recommended form is a JSON document at a well-known path on the
chain's canonical RPC or documentation host (e.g.
`/.well-known/svm-profile.json`). The exact transport is out of
scope; what matters is that the declarations are stable, versioned,
and addressable.

#### 2.1. `conformance_level`

One of `"svm-strict"`, `"svm-subset"`, `"svm-derived"`.

#### 2.2. `agave_baseline`

The Anza Agave version against which the conformance claim is made,
as a semantic version string (e.g. `"2.1.0"`). Conformance is always
relative to a specific upstream baseline.

#### 2.3. `genesis_hash_strategy`

One of:

- `"mainnet-equal"` — the chain's genesis hash equals Solana mainnet
  genesis hash. Reserved for forks that explicitly intend to be
  state-equivalent at some block height.
- `"custom"` — the chain has its own genesis hash distinct from
  Solana mainnet. **REQUIRED** for any chain that is not a literal
  state fork.
- `"derived"` — the chain's genesis hash is computed from a
  declared seed (e.g. an Ethereum block hash, a CAIP-2 chain
  identifier). The derivation rule **MUST** be documented.

A chain that uses Solana mainnet's genesis hash but is not a state
fork (i.e., has different starting balances) **MUST NOT** declare
`mainnet-equal`. This is the primary anti-confusion guarantee.

#### 2.4. `syscalls`

An object enumerating which Solana syscalls are supported. Each
syscall is named by its canonical symbol (e.g. `sol_log_`,
`sol_invoke_signed_c`, `sol_set_return_data`). For each syscall the
chain **MUST** record one of:

- `"supported"` — semantics match the declared `agave_baseline`.
- `"unsupported"` — calling this syscall causes program execution
  to fail (the chain **MUST** specify the failure mode: trap, error
  return, or unconditional revert).
- `"divergent"` — the syscall is callable but its semantics differ
  from `agave_baseline`. A `divergence_note` field **MUST** describe
  the difference.

Chains **SHOULD** also declare any non-Solana host-extension syscalls
they expose (e.g. for cross-VM messaging) under a separate
`host_extensions` array. Programs portable across SVM chains **SHOULD
NOT** rely on host extensions.

#### 2.5. `bpf_loader_versions`

An array listing accepted BPF Loader versions, each entry one of
`"v1"`, `"v2"` (`BPFLoader2`), `"v3"` (`BPFLoaderUpgradeable`), and
optionally future versions. A `svm-strict` chain **MUST** accept all
three of v1, v2, v3.

#### 2.6. `compute_budget`

An object describing ComputeBudget Program defaults and limits:

- `default_cu_per_ix`: default compute unit limit per instruction
  (Solana mainnet currently 200,000).
- `max_cu_per_tx`: maximum compute unit limit per transaction
  (Solana mainnet currently 1,400,000).
- `set_compute_unit_limit_supported`: boolean.
- `set_compute_unit_price_supported`: boolean.
- `request_heap_frame_supported`: boolean.

If any of these values diverge from the chain's declared
`agave_baseline`, conformance level **MUST NOT** be `svm-strict`.

#### 2.7. `spl_token`

An object describing SPL Token Program support. Required fields:

- `program_id`: the address at which the Token Program is deployed.
  A `svm-strict` chain **MUST** publish this at the canonical
  mainnet address `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`.
- `version`: one of `"v3"`, `"token-2022"`, or `"both"`.
- `decimals_truncation_rule`: one of `"native"` (mints define
  arbitrary decimals; no chain-level truncation), `"fixed-{N}"`
  (the chain enforces `N` SPL decimals regardless of the underlying
  asset's native decimals), or `"none"`.
  - A chain whose native asset has more decimals than the SPL Token
    Program can express (Solana SPL is u64 amounts; an
    18-decimal-native asset like a typical EVM token requires
    truncation when reflected as SPL) **MUST** declare its
    truncation rule explicitly.
  - Truncation **MUST** round toward zero (truncation, not
    rounding) to prevent inflation attacks via repeated round-trip.
- `freeze_authority_supported`: boolean. If `false`, attempts to
  set `freeze_authority` on a mint **MUST** be rejected at
  instruction-decode time, not silently accepted.
- `mint_authority_supported`: boolean. Same rejection requirement
  as above.
- `transfer_fee_supported` (Token-2022 only): boolean.
- `interest_bearing_supported` (Token-2022 only): boolean.
- `confidential_transfer_supported` (Token-2022 only): boolean.

#### 2.8. `ata_derivation`

An object describing how Associated Token Account addresses are
derived. Required fields:

- `algorithm`: one of `"canonical-ata-program"` (Solana mainnet's
  `find_program_address` against the Associated Token Account
  Program at `ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL`), or
  `"chain-specific"` (the chain uses its own deterministic
  derivation).
- If `chain-specific`, the chain **MUST** publish:
  - `seed_layout`: byte-level description of the preimage,
  - `hash_function`: hash primitive (e.g. SHA-256, Blake3,
    Keccak-256),
  - `program_id_referenced`: which program address (if any) is
    included in the preimage.

A `svm-strict` chain **MUST** use `canonical-ata-program`. A chain
using `chain-specific` derivation **MUST NOT** claim
`svm-strict`.

#### 2.9. `address_lookup_tables`

An object: `{ "supported": bool, "behavior": "mainnet-equivalent" | "subset" | "divergent" }`.
If `subset` or `divergent`, a free-form `notes` field describes the
restriction.

#### 2.10. `versioned_transactions`

An object: `{ "v0_supported": bool, "legacy_supported": bool }`.
A `svm-strict` chain **MUST** support both.

#### 2.11. `fee_asset`

The SPL mint or native asset used to pay transaction fees, by
canonical identifier. Solana mainnet uses native SOL with a fixed
mint pseudo-address. Chains that use a different asset (ETH on
Eclipse, native chain token elsewhere) **MUST** declare it.

#### 2.12. `caip2_chain`

The chain's CAIP-2 chain identifier (`namespace:reference`). Per
[CAIP-30](https://chainagnostic.org/CAIPs/caip-30) the `solana`
namespace is reserved for Solana mainnet, devnet, and testnet.
Other SVM chains **MUST NOT** use the `solana:` namespace; they
**SHOULD** register a distinct namespace (or reuse `eip155:` for
chains that anchor to an Ethereum chain ID and expose SVM as a
secondary VM, with the SVM-profile declarations supplementing the
EVM identification).

### 3. Cross-Chain Identity

A chain's `caip2_chain` value **MUST** be sufficient, when combined
with the published profile document, to determine whether a given
piece of Solana tooling will work without modification. Wallet and
SDK authors **SHOULD** treat the absence of a profile document as
equivalent to `svm-derived` with all features declared `unsupported`
unless documented otherwise.

### 4. Versioning of the Profile Itself

The profile document **MUST** carry a `profile_schema_version`
field. This sRFC, if adopted, defines `profile_schema_version: 1`.
Future revisions of this profile (additional declarations as the
SVM evolves) increment that version monotonically.

### 5. Testing Vectors

A chain claiming any conformance level **SHOULD** publish a
machine-runnable test vector set that exercises each declared
feature. Reference test vectors **SHOULD** be derived from
`solana-program-test` and `mollusk-svm` where applicable.

## Rationale

**Why a profile and not a hard standard.** The SVM is not a single
specification; it is the observable behavior of a specific Anza Agave
build at a specific commit, which itself evolves through SIMDs. Any
attempt to freeze "SVM compatibility" as a hard standard would
either fork from Solana mainnet immediately (wrong incentive) or
require non-Solana chains to ratchet-step-lock to every Anza release
(also wrong incentive). A profile lets each chain make its own
trade-off explicit and lets tooling authors test against the
declarations rather than guessing.

**Why three levels and not two or five.** Two levels (compatible /
incompatible) is too coarse — Eclipse and a deliberate fork of
Solana with renamed precompiles are clearly not equivalent in
practice. Five or more levels invite bikeshedding without adding
information. Three levels carve out a meaningful spectrum:
state-fork-equivalent, declared-subset, and intentionally-divergent.

**Why declarations and not capability negotiation.** SVM transactions
are submitted offline against a chain's RPC; there is no negotiation
phase where a client could ask the chain "do you support
`sol_invoke_signed_c`?" The declaration document substitutes for that
negotiation and makes the answer cacheable.

**Why allow `svm-derived` at all.** Chains running SVM as one of
several VMs (Tenzro Network is the example the author is most
familiar with) frequently have valid reasons to deviate from the
canonical Solana surface — for instance, mapping SPL token amounts
into an 18-decimal native ledger requires a documented truncation
rule, and ATA derivation in such an environment may need to bridge
the SPL keying scheme into the chain's native account space. Marking
these chains as `svm-derived` (rather than refusing them the SVM
label entirely) is more honest than the current state of affairs.

## Drawbacks

- Adds a documentation burden on every SVM-compatible chain.
- The set of declarations is necessarily incomplete on day one; any
  fixed list will need extension as the SVM evolves (handled via
  `profile_schema_version`).
- Wallet authors who do not read the profile document will be no
  better off than they are today; tooling adoption is a
  prerequisite for any practical benefit.

## Alternatives Considered

**SIMD instead of sRFC.** The SVM specification proper is governed
by SIMDs. A SIMD could mandate this profile inside the Anza Agave
runtime, but that would only constrain Solana mainnet itself, which
already implements every declared feature. The audience for this
proposal is the set of chains that consume the Anza Agave codebase
or its derivatives downstream. sRFCs are the documented venue for
application-layer standards that span beyond Solana mainnet itself.

**Capability negotiation at runtime.** Rejected — see Rationale.

**No standard at all (status quo).** Rejected — silent
compatibility breakage is the current outcome and is increasingly
costly as the SVM ecosystem grows.

## Prior Art

- **Solana Labs / Anza Agave** is the canonical SVM reference
  implementation; this profile is defined relative to it.
- **Eclipse Mainnet** publishes a "Differences between Eclipse and
  Solana" page documenting fee-asset and runtime divergences from
  mainnet; this profile generalizes that disclosure pattern.
- **Tenzro Network** ships SVM as one of three VMs and uses
  `solana-sbpf` for BPF execution; the author's reference
  implementation (Section "Reference Implementation") is a
  `svm-derived` instance with a small declared syscall subset and
  a chain-specific ATA derivation.
- **CAIP-2 / CAIP-30** establish the precedent that the `solana`
  namespace is reserved for Solana itself, leaving non-Solana SVM
  chains to use distinct namespaces.
- **Ethereum's EIP-1474 / EIP-1193** play a comparable role for
  Ethereum tooling: they describe what an Ethereum-JSON-RPC-
  compatible endpoint must expose, leaving chain-specific
  divergence to higher-layer documents. The SVM ecosystem lacks an
  equivalent reference.

## Unresolved Questions

1. **Where should the profile document live?** This sRFC proposes
   `/.well-known/svm-profile.json` on the canonical RPC host but
   does not mandate it. Alternatives: a registry maintained by the
   Solana Foundation, a CAIP-extension schema, or per-chain
   self-publication only.
2. **Should the profile be required or recommended?** As written,
   the profile is required for any chain making a conformance
   claim, but a chain can simply not make a claim and avoid the
   declarations. Whether ecosystem tooling should treat absence-
   of-profile as a hard signal is a wallet/SDK policy question
   beyond this sRFC.
3. **How should `Token-2022` extensions be enumerated?** The list
   is large and growing. A separate sub-profile may be warranted
   if the extension surface continues to expand.
4. **Interaction with future Firedancer.** Firedancer is a
   separate validator client implementing the same SVM
   specification. The profile is tied to "Anza Agave version" only
   for the baseline declaration; the actual conformance set should
   be observable behavior, not implementation. This may need
   re-wording once Firedancer parity is observed in production.
5. **Cross-chain ATA conventions.** Chains that bridge SPL tokens
   from Solana mainnet face an awkward choice between preserving
   canonical ATA derivation (and accepting the program-address
   collision risk if they don't deploy the canonical Associated
   Token Account program) or using a chain-specific derivation.
   This sRFC does not resolve the trade-off; it only requires the
   choice be declared.

## Reference Implementation

The canonical SVM reference is **Solana Labs / Anza Agave**
(<https://github.com/anza-xyz/agave>). Any conformance check is
made against an Agave commit.

A worked `svm-derived` example is available in **Tenzro Network**:

- SVM executor: `crates/tenzro-vm/src/svm/executor.rs` —
  `solana-sbpf`-based BPF interpretation, four registered syscalls
  (`sol_log_`, `sol_log_64_`, `sol_sha256`, `abort`), a strict
  subset of mainnet.
- SPL Token adapter: `crates/tenzro-vm/src/svm/spl_adapter.rs` —
  declares `decimals_truncation_rule: "fixed-9"` (truncating from
  the chain's 18-decimal native TNZO), `freeze_authority_supported:
  false`, `mint_authority_supported: true`.
- ATA derivation: `chain-specific`, `algorithm: "SHA-256"`,
  `seed_layout: "tenzro-ata: || owner || mint || program_id"`.
  This **does not** match canonical ATA derivation; per Section 2.8
  the chain therefore cannot claim `svm-strict`.
- Repository: <https://github.com/tenzro/tenzro-network>.

The author proposes Tenzro as **one example among several** of a
chain that this profile lets honestly self-describe its
non-mainnet status. The intent is for Eclipse, SOON, Termina,
Atlas, MagicBlock, Sonic, and any future SVM chain to publish their
own profile documents on the same schema.

## Acknowledgements

This proposal benefited from the design conversations behind
Eclipse's "Differences between Eclipse and Solana" documentation
and from the Anza Agave team's work standardizing the SVM through
the SIMD process. Errors are the author's.

## Author

Hilal Agil — <hilal@tenzro.com> — GitHub
[@hilarl](https://github.com/hilarl)
