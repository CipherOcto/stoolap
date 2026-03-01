# RFC README

This directory contains Requests for Comments (RFCs) for the Stoolap Chain blockchain SQL database project.

## What is an RFC?

An RFC is a **design specification** that defines:
- WHAT is being built
- Technical specifications and constraints
- Interfaces and expected behavior
- Implementation path

RFCs answer "What must exist before implementation?"

## RFC Process

### 1. Draft RFC
Create RFC from use case motivation:
```bash
cp rfcs/0000-template.md rfcs/0000-my-title.md
```

### 2. Submit PR
Open pull request for discussion

### 3. Review
Community discusses (7-day minimum)

### 4. Decision
Maintainers accept/reject based on consensus

### 5. Outcome
- **Accepted** → Renumber, create missions
- **Rejected** → Move to `archived/` with reasoning
- **Needs Work** → Continue discussion

## RFC Lifecycle

```
Draft → Discussion → Accepted → Implemented → Superseded
                                ↓
                            Rejected → Archived
```

## Accepted RFCs

| Number | Title | Status | Date |
|--------|-------|--------|------|
| [RFC-0101](0101-hexary-merkle-proofs.md) | Hexary Merkle Proofs | Accepted | Feb 2025 |
| [RFC-0102](0102-deterministic-types.md) | Deterministic Value Types | Accepted | Feb 2025 |
| [RFC-0103](0103-blockchain-consensus.md) | Blockchain Consensus | Accepted | Feb 2025 |

## Draft RFCs

Draft RFCs are being discussed and are not yet accepted.

## Archived RFCs

Rejected, superseded, or withdrawn RFCs are stored in `archived/`.

## Numbering

- **Draft RFCs**: Use 0000, 0001, 0002, etc.
- **Accepted RFCs**: Renumbered into ranges (0100-0199 for Phase 1, 0200-0299 for Phase 2, etc.)

## RFC Template

See `0000-template.md` for the RFC template.

## Related

- [Use Cases](../docs/use-cases/) - WHY we build things
- [Missions](../missions/) - HOW we build them
- [BLUEPRINT.md](../BLUEPRINT.md) - Governance process
