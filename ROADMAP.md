# Stoolap Chain Roadmap

This roadmap outlines the evolution of the Stoolap Chain blockchain SQL database from conception to production.

---

## Current Status: Phase 1 - Foundation ✅

**Completed:** February 2025

The foundation for a blockchain SQL database has been established with hexary Merkle proofs, deterministic types, and block consensus.

---

## Phase 1: Foundation ✅ (COMPLETE)

**Goal:** Establish core cryptographic and consensus infrastructure

### Completed Milestones

| Milestone | RFC | Status | Date |
|-----------|-----|--------|------|
| Deterministic Value Types | RFC-0102 | ✅ Complete | Feb 2025 |
| Hexary Merkle Proofs | RFC-0101 | ✅ Complete | Feb 2025 |
| Blockchain Consensus | RFC-0103 | ✅ Complete | Feb 2025 |
| SHA-256 Migration | - | ✅ Complete | Feb 2025 |

### Deliverables

- ✅ `DetermValue` - Deterministic value types with inline/heap optimization
- ✅ `HexaryProof` - Compact bitmap-based Merkle proofs for 16-way tries
- ✅ `RowTrie` - Hexary Merkle trie with proof generation
- ✅ `ExecutionContext` - Gas-metered transaction execution
- ✅ `Block` - Block structure with operations and state commitments
- ✅ 4,344 passing tests
- ✅ Comprehensive benchmarks

### Performance Achieved

| Metric | Target | Actual |
|--------|--------|--------|
| Proof size (typical) | <100 bytes | ~68 bytes |
| Verification time | <5 μs | ~2-3 μs |
| Batch verification (100) | <50 μs single-threaded | ~50 μs |
| Batch verification (100) | <15 μs parallel (8 cores) | ~15 μs |

---

## Phase 2: Protocol Enhancement (PLANNED)

**Goal:** Extend consensus mechanism with block production and validation

### Planned RFCs

| RFC | Title | Status | Priority |
|-----|-------|--------|----------|
| RFC-0201 | Block Production & Proposer Selection | Draft | High |
| RFC-0202 | Block Validation & Fork Choice | Draft | High |
| RPC-0203 | Network Protocol & Gossip | Draft | Medium |
| RPC-0204 | Signature Schemes & Validator Keys | Draft | High |

### Key Features

- **Block Producers** - Designated nodes propose blocks
- **Validator Set** - Participating nodes validate and sign blocks
- **Fork Choice Rule** - Chain selection in case of conflicts
- **Gossip Protocol** - Block and transaction propagation
- **Finality** - Economic finality for confirmed blocks

### Timeline Estimate

Q2-Q3 2025

---

## Phase 3: Scaling & Optimizations (FUTURE)

**Goal:** Improve throughput, reduce proof sizes, enhance performance

### Planned RFCs

| RFC | Title | Status | Priority |
|-----|-------|--------|----------|
| RFC-0301 | Proof Compression & SNARK Verification | Draft | Medium |
| RFC-0302 | State Pruning & Archive Access | Draft | Low |
| RFC-0303 | Parallel Transaction Execution | Draft | High |
| RFC-0304 | Database Sharding | Draft | Low |

### Key Features

- **SNARK Proofs** - Compress multiple proofs into single verification
- **State Pruning** - Archive old state, reduce storage
- **Parallel Execution** - Execute non-conflicting transactions in parallel
- **Database Sharding** - Horizontal scaling for large datasets

### Timeline Estimate

2026

---

## Phase 4: Advanced Features (FUTURE)

**Goal:** Add advanced blockchain and database capabilities

### Planned RFCs

| RFC | Title | Status | Priority |
|-----|-------|--------|----------|
| RFC-0401 | Cross-Chain Bridges | Draft | Medium |
| RFC-0402 | Light Client SPV Mode | Draft | High |
| RFC-0403 | State Channels | Draft | Medium |
| RFC-0404 | Encrypted Database Operations | Draft | Low |
| RFC-0405 | SQL Smart Contract Language | Draft | High |

### Key Features

- **Cross-Chain Bridges** - Interoperability with other blockchains
- **SPV Mode** - Light client verification without full nodes
- **State Channels** - Off-chain computation with on-chain settlement
- **Encryption** - Confidential database operations
- **Smart Contracts** - SQL-based programmable logic

### Timeline Estimate

2026-2027

---

## Implementation Progress

### Phase 1: Foundation ✅

| Mission | Status | Mission # |
|--------|--------|----------|
| HexaryProof Core Data Structures | ✅ Complete | 001 |
| Nibble Packing/Unpacking Utilities | ✅ Complete | 002 |
| Bitmap Sibling Reconstruction | ✅ Complete | 003 |
| 16-Way Child Hashing | ✅ Complete | 004 |
| HexaryProof Streaming Verification | ✅ Complete | 005 |
| RowTrie Proof Generation | ✅ Complete | 006 |
| Solana-Style Serialization | ✅ Complete | 007 |
| Parallel Batch Verification | ✅ Complete | 008 |
| Verification Fixes and Refinements | ✅ Complete | 009 |
| Integration Tests | ✅ Complete | 010 |
| Benchmarks | ✅ Complete | 011 |
| Documentation and Cleanup | ✅ Complete | 012 |

### Phase 2: Protocol Enhancement (NEXT)

| Mission | Status | Mission # |
|--------|--------|----------|
| TBD | 📋 Planned | - |

---

## Use Cases Driving Development

### Active Use Cases

1. **[Blockchain SQL Database](docs/use-cases/blockchain-sql-database.md)** - Primary motivation
   - Enables verifiable SQL database state
   - Trust minimization through cryptographic proofs
   - Gas-metered transaction execution

2. **[Verifiable State Proofs](docs/use-cases/verifiable-state-proofs.md)** - Core technology
   - Lightweight clients without full nodes
   - Efficient proof verification
   - Standard hexary proof format

---

## Related RFCs

### Accepted RFCs

- **[RFC-0101](rfcs/0101-hexary-merkle-proofs.md)** - Hexary Merkle Proofs (Phase 1)
- **[RFC-0102](rfcs/0102-deterministic-types.md)** - Deterministic Value Types (Phase 1)
- **[RFC-0103](rfcs/0103-blockchain-consensus.md)** - Blockchain Consensus (Phase 1)

### Draft RFCs

- **[RFC-0201](rfcs/0201-block-production.md)** - Block Production (Phase 2)
- **[RFC-0202](rfcs/0202-block-validation.md)** - Block Validation (Phase 2)

---

## Technology Choices

### Why Hexary Tries?

- **Efficiency** - 16-way branching reduces tree depth
- **Proximity** - Related data stored near each other
- **Industry Standard** - Ethereum, Optimism use similar structures

### Why SHA-256?

- **Security** - Cryptographic hash function
- **Standard** - Widely adopted, hardware acceleration
- **Determinism** - Same input always produces same output

### Why Bitmap Encoding?

- **Compactness** - Only store actual siblings, not empty positions
- **Flexibility** - Handles sparse trie structures
- **Efficiency** - 5-10x smaller than binary proofs

---

## Success Metrics

### Phase 1 Targets (ACHIEVED)

| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| Proof size | <100 bytes | ~68 bytes | ✅ |
| Verification time | <5 μs | ~2-3 μs | ✅ |
| Tests passing | >4000 | 4,344 | ✅ |
| Benchmarks created | Yes | Yes | ✅ |

### Phase 2 Targets (FUTURE)

| Metric | Target | Status |
|--------|--------|--------|
| Blocks per second | >100 | 📋 Planned |
| Finality time | <1 minute | 📋 Planned |
| Validator count | >4 | 📋 Planned |

---

## Contributing

See **[BLUEPRINT.md](BLUEPRINT.md)** for the governance process.

**Quick Start:**
1. Read [Use Cases](docs/use-cases/) for WHY
2. Read [RFCs](rfcs/) for WHAT
3. Claim a [Mission](missions/) for HOW

**Never skip layers.** No RFC = No Mission.

---

## Links

- [CipherOcto Blueprint](BLUEPRINT.md)
- [Getting Started](START_HERE.md)
- [Contributor Roles](ROLES.md)
- [Use Cases](docs/use-cases/)
- [RFCs](rfcs/)
- [Missions](missions/)
- [Design Documents](docs/plans/)
- [GitHub Repository](https://github.com/stoolap/stoolap_chain)
