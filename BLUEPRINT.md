# The CipherOcto Blueprint

**How ideas become protocol reality.**

This is not documentation. This is process architecture.

---

## Philosophy

CipherOcto is not a repository. It is a protocol for autonomous intelligence collaboration.

Most open-source projects organize files. Successful protocols organize **decision flow**.

This Blueprint defines how work flows through CipherOcto—from idea to protocol evolution.

---

## The Core Separation

We maintain three distinct layers that must never mix:

| Layer | Purpose | Question | Blockchain Analogy |
|-------|---------|----------|-------------------|
| **Use Cases** | Intent | WHY? | Ethereum Vision |
| **RFCs** | Design | WHAT? | EIPs |
| **Missions** | Execution | HOW? | Implementation |

**Mix these layers and governance breaks.**

---

## Governance Stack

```
┌─────────────────────────────────────────────────────────────┐
│                     Idea Emerges                             │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  1️⃣ USE CASES — Intent Layer                               │
│  Location: docs/use-cases/                                  │
│                                                             │
│  Defines:                                                   │
│  - Problems to solve                                        │
│  - Narratives and motivation                                │
│  - Architectural direction                                  │
│                                                             │
│  Characteristics:                                           │
│  - Long-lived                                               │
│  - Descriptive                                              │
│  - Non-actionable                                            │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  2️⃣ RFCs — Protocol Design Layer                           │
│  Location: rfcs/                                            │
│                                                             │
│  Defines:                                                   │
│  - Specifications                                          │
│  - Constraints                                              │
│  - Interfaces                                               │
│  - Expected behavior                                        │
│                                                             │
│  Examples:                                                  │
│  - RFC-0001: Mission Lifecycle                              │
│  - RFC-0002: Agent Manifest Spec                            │
│  - RFC-0003: Storage Provider Protocol                      │
│  - RFC-0101: Hexary Merkle Proofs for Blockchain SQL        │
│  - RFC-0102: Deterministic Value Types                      │
│                                                             │
│  Answer: "What must exist before implementation?"           │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  3️⃣ MISSIONS — Execution Layer                             │
│  Location: missions/                                        │
│                                                             │
│  A mission is a claimable unit of work.                     │
│  - Never conceptual                                         │
│  - Always executable                                         │
│  - Created ONLY after: Use Case → RFC → Mission             │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  4️⃣ AGENTS — Execution Actors                              │
│  Location: agents/                                          │
│                                                             │
│  Agents do NOT decide direction.                            │
│  They implement Missions derived from RFCs.                 │
│  This prevents AI chaos.                                    │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  5️⃣ ROADMAP — Temporal Layer                               │
│  Location: ROADMAP.md                                       │
│                                                             │
│  References:                                                │
│  - Use Cases                                                │
│  - RFC milestones                                           │
│  - Protocol phases                                          │
│                                                             │
│  Roadmap is navigation, NOT backlog.                        │
└─────────────────────────────────────────────────────────────┘
```

---

## Canonical Workflow

```
Idea
 │
 ▼
Use Case (WHY?)
 │
 ▼
RFC Discussion (WHAT?)
 │
 ├─ Draft RFC
 ├─ Community Review
 ├─ Revision
 └─ Accepted RFC
 │
 ▼
Mission Created (HOW?)
 │
 ▼
Agent/Human Claims Mission
 │
 ▼
Implementation (PR)
 │
 ▼
Review & Test
 │
 ▼
Merge
 │
 ▼
Protocol Evolution
```

**This is the only flow. Shortcuts create technical debt.**

---

## Artifact Types

### Use Case

**Location:** `docs/use-cases/`

**Template:**
```markdown
# Use Case: [Title]

## Problem
What problem exists?

## Motivation
Why does this matter for CipherOcto?

## Impact
What changes if this is implemented?

## Related RFCs
- RFC-XXXX
```

---

### RFC (Request for Comments)

**Location:** `rfcs/`

**Template:**
```markdown
# RFC-XXXX: [Title]

## Status
Draft | Accepted | Replaced | Deprecated

## Summary
One-paragraph overview.

## Motivation
Why this RFC?

## Specification
Technical details, constraints, interfaces.

## Rationale
Why this approach over alternatives?

## Implementation
Path to missions.

## Related Use Cases
- [Use Case Name](../../docs/use-cases/...)
```

---

### Mission

**Location:** `missions/`

**Template:**
```markdown
# Mission: [Title]

## Status
Open | Claimed | In Review | Completed | Blocked

## RFC
RFC-XXXX

## Acceptance Criteria
- [ ] Criteria 1
- [ ] Criteria 2

## Claimant
@username

## Pull Request
#

## Notes
Implementation notes, blockers, decisions.
```

---

## Repository Topology

```
cipherocto/
├── BLUEPRINT.md               ← This document
├── START_HERE.md
├── ROLES.md
├── ROADMAP.md
├── docs/
│   └── use-cases/             ← Intent layer
│       ├── blockchain-sql-database.md
│       └── state-verification.md
├── rfcs/                      ← Design layer
│   ├── README.md
│   ├── 0000-template.md
│   ├── 0100-deterministic-types.md
│   ├── 0101-hexary-merkle-proofs.md
│   ├── 0102-blockchain-consensus.md
│   └── archived/
├── missions/                  ← Execution layer
│   ├── open/
│   ├── claimed/
│   ├── with-pr/
│   └── archived/
│       ├── 001-hexary-proof-data-structures.md
│       ├── 002-nibble-packing-utilities.md
│       └── ...
├── agents/
└── crates/
```

---

## Summary

**The CipherOcto Blueprint answers: "What do I do first?"**

- Understand the Use Case (WHY)
- Read the RFC (WHAT)
- Claim the Mission (HOW)

**Everything flows through this structure.**

When in doubt, return to the Blueprint.

---

*"We are not documenting a repository. We are defining how autonomous intelligence collaborates to build infrastructure."*
