# Welcome to Stoolap Chain

This is the Stoolap Chain blockchain SQL database project, organized using the **CipherOcto Blueprint** for protocol governance.

## Quick Start

**New to the project?** Start here:

1. **Read this file** → Understand the project
2. **Read `BLUEPRINT.md`** → Understand how we work
3. **Read `ROLES.md`** → Understand how you can contribute
4. **Read `ROADMAP.md`** → Understand where we're going

## What is Stoolap Chain?

Stoolap Chain is a **blockchain SQL database** that combines:
- Traditional SQL database capabilities
- Merkle-patricia trie state verification
- Gas-metered transaction execution
- Deterministic consensus for state transitions

## Project Structure

```
stoolap_chain/
├── BLUEPRINT.md         ← How we govern work
├── START_HERE.md        ← This file
├── ROLES.md             ← How to contribute
├── ROADMAP.md           ← Where we're going
├── docs/use-cases/      ← WHY we build things
├── rfcs/                ← WHAT we're building
├── missions/            ← HOW we build it
├── src/                 ← Implementation
│   ├── consensus/       ← Blockchain consensus (blocks, operations)
│   ├── determ/          ← Deterministic types for blockchain
│   ├── trie/            ← Merkle tries (RowTrie, SchemaTrie, proofs)
│   ├── execution/       ← Transaction execution context
│   └── storage/         ← Traditional SQL storage engine
└── tests/               ← Integration tests
```

## Key Components

| Component | Description |
|-----------|-------------|
| **DetermValue** | Deterministic value types (no Arc, inline text optimization) |
| **RowTrie** | Hexary Merkle trie for row storage with proof generation |
| **HexaryProof** | Compact bitmap-based Merkle proofs for 16-way tries |
| **ExecutionContext** | Gas-metered execution with state snapshots |
| **Block** | Consensus block with operations and state commitments |

## How to Contribute

### As a Human

1. Browse `docs/use-cases/` for what we're solving
2. Check `rfcs/` for active designs
3. Claim a mission from `missions/open/`
4. Implement according to RFC spec
5. Submit PR for review

### As an Agent

1. Read `missions/open/` for available work
2. Claim a mission
3. Implement per RFC spec
4. Write tests
5. Submit PR

## Recent Work

See `ROADMAP.md` for current status. Recent accomplishments include:

- ✅ **RFC-0101: Hexary Merkle Proofs** - Complete (SHA-256, bitmap encoding, batch verification)
- ✅ **RFC-0102: Deterministic Value Types** - Complete (DetermValue with inline/heap text)
- ✅ **RFC-0103: Blockchain Consensus** - Complete (blocks, operations, state roots)

## Getting Started

**Build the project:**
```bash
cargo build
```

**Run tests:**
```bash
cargo test
```

**Run integration tests:**
```bash
cargo test --test blockchain_integration_test
```

## Governance

This project follows the **CipherOcto Blueprint**:

- **Use Cases** define WHY we build something
- **RFCs** define WHAT we're building
- **Missions** define HOW we build it

**Never skip a layer.** No RFC = No Mission.

## Learn More

- `BLUEPRINT.md` - Deep dive on governance
- `ROLES.md` - How to contribute
- `ROADMAP.md` - Current status and direction
- `docs/use-cases/` - Project motivation
- `rfcs/` - Technical specifications
