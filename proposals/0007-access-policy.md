---
simd: "0007"
title: SIMD Access Policy
authors:
- Jacob Creech (Solana Foundation)
- Ben Hawkins (Solana Foundation)
category: Meta
type: Meta
status: Living
created: 2022-12-10
---

## Summary

Two levels of access, listed in order of increasing privilege:

1. **Write**
2. **Maintain**

Each level implies the capabilities of the one beneath it. A contributor with
_Write_ access can triage, approve, and merge pull requests once required
status‑checks pass, while a contributor with _Maintain_ access can additionally
administer repository settings and manage access.

## Motivation

The SIMD process values openness while safeguarding core infrastructure. Two
clearly‑defined levels minimise administrative overhead yet retain sufficient
control to keep the repository healthy.

## Alternatives Considered

* A three‑tier scheme (Triage / Write / Maintain) was previously documented.
  Consolidating Triage and Write avoids confusion and matches GitHub’s current
  merge‑gate automation where contributors need _Write_ to land changes.
* Other open‑source projects (e.g. EIP and PEP) use committee‑based models;
  those approaches carry higher governance overhead for our current scale.

## Detailed Design

### Level 1 – Write Access

**Requirement**  One voucher from a Level 2 member **or** two vouchers from
existing Level 1 members.

Write maps to GitHub’s built‑in 
[**Write** role](https://docs.github.com/en/organizations/managing-user-access-to-your-organizations-repositories/repository-roles-for-an-organization#permissions-for-each-role):

* Open pull requests and **self‑merge** once mandatory status checks 
(e.g. Jump + Anza approvals) are green. Write‑level contributors may not merge
pull requests opened by other users; that capability is reserved for Level 2 maintainers.
* Manage issues, labels, and project boards.
* Create and push branches.

### Level 2 – Maintain Access

**Requirement**  One voucher from an existing Level 2 member.

Maintain maps to GitHub’s 
[**Maintain** role](https://docs.github.com/en/organizations/managing-user-access-to-your-organizations-repositories/repository-roles-for-an-organization#permissions-for-each-role):

* All capabilities of Level 1, **plus the ability to merge any pull request** once required status checks are satisfied.
* Manage repository settings, branch‑protection rules, and workflows.
* Grant or revoke Write and Maintain access according to this SIMD.

### Vouching Process

1. Open an issue in the SIMD repository titled
   `Request Access (Level X) for <GitHub‑username>` and include a public contact
   method (e.g. Discord handle).
2. Supporters comment **“I vouch”** on the issue.
3. When the issue meets the requirement above, a Level 2 maintainer adds the
   user to the appropriate GitHub team.

### Access Removal Process

1. Open an issue titled `Revoke Access (Level X) for <GitHub‑username>`.
2. Collect supporting comments:
   * Revoking **Level 2** – requires support from two other Level 2 members.
   * Revoking **Level 1** – requires support from one Level 2 member **or** two
     Level 1 members.
   * Self‑revocation needs no additional support.
3. A Level 2 maintainer removes the user from the corresponding team.

## Impact

A lean two‑tier policy lowers the barrier to contribution while preserving
clear custodianship of the SIMD process.

## Security Considerations

Should a malicious actor gain access, community members must invoke the Access
Removal Process promptly. Maintainers may also tighten branch‑protection rules
or revoke credentials as warranted.