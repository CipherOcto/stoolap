# Roles in Stoolap Chain

This document defines the roles and responsibilities in the Stoolap Chain project following the CipherOcto Blueprint.

---

## Core Principle

**Humans govern. Agents implement.**

| Activity | Human | Agent |
|----------|-------|-------|
| Define Use Cases | ✓ | ✗ |
| Write RFCs | ✓ | ✗ |
| Accept RFCs | ✓ | ✗ |
| Create Missions | ✓ | ✓ |
| Claim Missions | ✓ | ✓ |
| Implement RFCs | ✓ | ✓ |
| Review PRs | ✓ | ✗ |
| Merge to main | ✓ | ✗ |

---

## Human Roles

### Maintainer

**Responsibilities:**
- Accept/reject RFCs
- Review and merge PRs
- Enforce the Blueprint workflow
- Guide project direction

**Requirements:**
- Deep understanding of the codebase
- Commitment to the Blueprint process
- Active participation in RFC discussions

### Contributor

**Responsibilities:**
- Write RFCs for proposed changes
- Claim and complete missions
- Submit PRs for review
- Participate in RFC discussions

**Requirements:**
- Follow the Blueprint workflow
- Implement according to RFC specs
- Write tests for all code

### RFC Author

**Responsibilities:**
- Draft RFC from use case motivation
- Address community feedback
- Revise based on review
- Create missions after acceptance

---

## Agent Capabilities

### What Agents CAN Do

| Capability | Description |
|------------|-------------|
| Read Missions | Browse `missions/open/` |
| Claim Missions | Move mission to `missions/claimed/` |
| Implement Specs | Execute according to RFC |
| Write Tests | Ensure quality and coverage |
| Submit PRs | Standard contribution flow |
| Update Missions | Move through lifecycle (claimed → with-pr → archived) |

### What Agents CANNOT Do

| Restriction | Reason |
|-------------|--------|
| Create Use Cases | Human direction required for intent |
| Accept RFCs | Governance decision requires human judgment |
| Bypass Missions | Must follow Blueprint workflow |
| Skip RFC Process | Shortcuts create technical debt |

---

## Mission Workflow

```
┌──────────────────┐
│   RFC Accepted   │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│  Mission Created │  → missions/open/
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│     Claimed      │  → missions/claimed/  (Human or Agent)
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│  PR Submitted    │  → missions/with-pr/
└────────┬─────────┘
         │
         ├─ Accept → Archive (completed)
         └─ Reject → Return to claimed
```

### Mission Timeouts

| State | Timeout | Action |
|-------|---------|--------|
| Claimed | 14 days | Return to open |
| With PR | 7 days | Follow up or close |

---

## RFC Process

### Submission

1. Draft RFC from use case motivation
2. Submit PR for discussion
3. Add to `rfcs/` as `XXXX-title.md` (draft number)

### Review

1. Community discussion (7-day minimum)
2. Maintainer feedback
3. Revisions as needed

### Acceptance

1. At least 2 maintainer approvals
2. No blocking objections
3. Renumber to final RFC number
4. Create missions from accepted RFC

### Outcomes

| Outcome | Action |
|----------|--------|
| Accepted | Renumber, create missions |
| Rejected | Move to `rfcs/archived/` with reasoning |
| Needs Work | Continue discussion |

---

## Code Review Process

### PR Submission

1. Reference the mission being completed
2. Link to the RFC being implemented
3. Include tests
4. Ensure all CI checks pass

### Review Criteria

- Matches RFC specification
- Tests pass
- No unnecessary changes
- Follows coding standards

### Approval

- At least 1 maintainer approval
- All CI checks pass
- No blocking review comments

---

## Decision Matrix

| Question | Who Decides | Based On |
|----------|-------------|----------|
| Is this worth building? | Humans | Use Cases |
| What should we build? | Humans | RFCs |
| How do we build it? | Humans + Agents | Missions |
| Is the implementation correct? | Humans | Code Review |
| Is the RFC accepted? | Maintainers | RFC Process |

---

## Summary

**The Blueprint creates clarity:**

- Use Cases tell us WHY
- RFCs tell us WHAT
- Missions tell us HOW

**Humans provide direction. Agents (and humans) provide execution.**

When in doubt, refer to the appropriate layer of the Blueprint.
