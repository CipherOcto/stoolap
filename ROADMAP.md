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

## Phase 2: Zero-Knowledge Proofs (PLANNED)

**Goal:** Integrate STWO/Cairo for proof compression, confidential queries, and L2 scaling

### Planned RFCs

| RFC | Title | Status | Priority |
|-----|-------|--------|----------|
| RFC-0201 | STWO and Cairo Integration | Draft | High |
| RFC-0202 | Compressed Proof Format | Draft | High |
| RFC-0203 | Confidential Query Operations | Draft | Medium |
| RFC-0204 | L2 Rollup Protocol | Draft | High |

### Key Features

- **STWO Integration** - Circle STARK prover/verifier in Rust
- **Cairo Programs** - Full Cairo programs for core operations
- **Proof Compression** - Aggregate HexaryProofs into single STARK proof
- **Confidential Queries** - Private database operations with ZK proofs
- **L2 Rollup** - Off-chain execution with on-chain verification

### Timeline Estimate

Q2-Q3 2025

---

## Phase 3: Protocol Enhancement (FUTURE)

**Goal:** Extend consensus mechanism with block production and validation

### Planned RFCs

| RFC | Title | Status | Priority |
|-----|-------|--------|----------|
| RFC-0301 | Block Production & Proposer Selection | Draft | High |
| RFC-0302 | Block Validation & Fork Choice | Draft | High |
| RFC-0303 | Network Protocol & Gossip | Draft | Medium |
| RFC-0304 | Signature Schemes & Validator Keys | Draft | High |

### Key Features

- **Block Producers** - Designated nodes propose blocks
- **Validator Set** - Participating nodes validate and sign blocks
- **Fork Choice Rule** - Chain selection in case of conflicts
- **Gossip Protocol** - Block and transaction propagation
- **Finality** - Economic finality for confirmed blocks

### Timeline Estimate

Q4 2025 - Q1 2026

---

## Phase 4: Advanced Optimizations (FUTURE)

**Goal:** Further scalability and advanced features

### Planned RFCs

| RFC | Title | Status | Priority |
|-----|-------|--------|----------|
| RFC-0401 | State Pruning & Archive Access | Draft | Low |
| RFC-0402 | Parallel Transaction Execution | Draft | High |
| RFC-0403 | Database Sharding | Draft | Low |
| RFC-0404 | Cross-Chain Bridges | Draft | Medium |
| RFC-0405 | Light Client SPV Mode | Draft | High |

### Key Features

- **State Pruning** - Archive old state, reduce storage
- **Parallel Execution** - Execute non-conflicting transactions in parallel
- **Database Sharding** - Horizontal scaling for large datasets
- **Cross-Chain Bridges** - Interoperability with other blockchains
- **SPV Mode** - Light client verification without full nodes

### Timeline Estimate

2026

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

3. **[ZK Proofs for Scalability and Privacy](docs/use-cases/zk-proofs-scalability.md)** - Next phase
   - Proof compression for bandwidth efficiency
   - Confidential queries for private data operations
   - L2 rollup for high-throughput applications

---

## Related RFCs

### Accepted RFCs

- **[RFC-0101](rfcs/0101-hexary-merkle-proofs.md)** - Hexary Merkle Proofs (Phase 1)
- **[RFC-0102](rfcs/0102-deterministic-types.md)** - Deterministic Value Types (Phase 1)
- **[RFC-0103](rfcs/0103-blockchain-consensus.md)** - Blockchain Consensus (Phase 1)

### Draft RFCs

- **[RFC-0201](rfcs/0201-stwo-cairo-integration.md)** - STWO/Cairo Integration (Phase 2)
- **[RFC-0202](rfcs/0202-compressed-proofs.md)** - Compressed Proofs (Phase 2)
- **[RFC-0203](rfcs/0203-confidential-queries.md)** - Confidential Queries (Phase 2)
- **[RFC-0204](rfcs/0204-l2-rollup.md)** - L2 Rollup (Phase 2)

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

### Why STWO and Cairo?

- **STWO** - Circle STARK prover/verifier written in Rust
- **Cairo** - Turing-complete language for provable programs
- **Native Integration** - Direct Rust linking for performance
- **Cairo VM Verification** - On-chain verification via Cairo contracts

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
| Proof size (100 rows) | <10 KB | 📋 Planned |
| Verification time | <100ms | 📋 Planned |
| L2 TPS | >1000 | 📋 Planned |
| Finality time | <1 minute | 📋 Planned |

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
