---
simd: '0007'
title: SIMD Access Policy
authors:
- Jacob Creech (Solana Foundation)
category: Meta
type: Meta
status: Living
created: 2022-12-10
---

## Summary

3 levels of access, in the order of increasing access:

1. Triage

    Requirement: One voucher from anyone with level 2 or above access.

2. Write

    Requirement: One voucher from anyone with level 3 access or two from
anyone with level 2 access. If only vouched by those with level 2 access, the
user must have Triage access to be promoted.

3. Maintain

    Requirement: One voucher from anyone with level 3 access. This permission
is usually reserved for those maintaining the SIMD process.

Each of the levels implies having the previous levels - e.g. level 2 implies
level 1. A contributor with Write access will be able to triage pull requests
in accordance with Github access policies.

## Motivation

Shamelessly borrowing from [Mozilla's access 
policy](https://www.mozilla.org/en-US/about/governance/policies/commit/access-policy/)
: 

```
There are two sorts of control which can be used to stop people checking in -
technical and social.

A "full technical" implementation would have per-directory permissions
everywhere, but would lead to a greatly-increased management overhead for IT,
vouchers and developers alike.
A "full social" implementation would just have a single permission which gave
you complete access to everything, but (depending on the height of the barrier
to that permission) there is a risk of making developer's lives more difficult
when they are excluded, or of giving the untrustworthy or incompetent power to
mess things up.
Therefore, a good policy balances the use of technical and social controls to
minimize both management overhead and risk to the development process.
```

## Alternatives Considered

[EIP](https://github.com/ethereum/EIPs) run under two separate levels,
contributors and editors. Editors are a select group of people that review each
EIP and have write access. Instead of starting with a select group of editors
who have exclusive write access, we could begin with an easier way to achieve
write access and gradually tighten permissions as needed.

[RFCs](https://www.rust-lang.org/governance) have sub-teams that review and
triage new proposed changes. This is a possibility, but would require more
granular labeling of different SIMDs.

[PEP](https://peps.python.org/pep-0013/) has an election of a council that
reviews and triages new proposed changes. The PEP approach may result in an
increase in governance overhead.

## Detailed Design

### Level 1 - Triage Access

Requirements: One voucher from any user with level 2 or above access.

Contributors with Triage access will have the associated [triage Github access
policy](https://docs.github.com/en/organizations/managing-user-access-to-your-organizations-repositories/repository-roles-for-an-organization#permissions-for-each-role)
for the [Solana Improvement
Documents](https://github.com/solana-foundation/solana-improvement-documents)
repository.

### Level 2 - Write Access

Requirements: One voucher from anyone with level 3 access or two vouchers from
anyone from level 2. Applicants must have level 1 access to apply for level 2.

Contributors with Write access will have the associated [write Github access
policy](https://docs.github.com/en/organizations/managing-user-access-to-your-organizations-repositories/repository-roles-for-an-organization#permissions-for-each-role)
for the [Solana Improvement
Documents](https://github.com/solana-foundation/solana-improvement-documents)
repository.

Most notably these permissions allow the contributor to approve and merge pull
requests.

### Level 3 - Maintain Access

Requirements: One voucher from anyone with level 3 access.

Contributors will have the associated [maintain Github access
policy](https://docs.github.com/en/organizations/managing-user-access-to-your-organizations-repositories/repository-roles-for-an-organization#permissions-for-each-role)
for the [Solana Improvement
Documents](https://github.com/solana-foundation/solana-improvement-documents)
repository.

Users with Maintain access are responsible for managing the SIMD repository and
granting user access based on the above requirements.

### Vouching Process 

To receive a level of access as defined in SIMD-0007, follow these steps:

1. Open an issue on the [Solana Improvement
Documents](https://github.com/solana-foundation/solana-improvement-documents)
repository with a title in the following format: "Request Access (Level [X])
for [Username]". Add information that can help identify you such as your 
discord id in the issue.
2. Gather your vouchers to add a comment on the issue expressing their support.
3. Once the issue has received enough support, notify the users with Maintain
access by adding a comment tagging
[`@SIMD-Maintainers`](https://github.com/orgs/solana-foundation/teams/simd-maintainers)
and you will be granted the requested level of access.

### Access Removal Process

In the event that a user requires their access to be removed, follow these
steps:

1. Open an issue on the [Solana Improvement
Documents](https://github.com/solana-foundation/solana-improvement-documents)
repository with the title in the following format: "Revoke Access (Level X) for
[Username]".
2. Other users with the appropriate level of access should comment on the issue
to express their support for the removal of access.
3. Once the issue has received enough support, the user's access will be
revoked.

Requirements:

- If a user's level 3 access is being revoked, support from at least two other
users with level 3 access is required.
- If a user's level 1 or 2 access is being revoked, support from at least two 
other users with level 2 or one user with level 3 is required.
- If a user opens the issue to revoke their own access, no support from others
is required.

## Impact

Setting a permissive way of contributing to the SIMD process will help remove
engineering overhead.

## Security Considerations

In the event of a malicious actor gaining any level of access, users must
follow the Access Removal Process to revoke that actor's access.