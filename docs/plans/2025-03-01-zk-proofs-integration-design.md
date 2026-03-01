# ZK Proofs Integration Design

> **Status:** Design Phase
> **Related RFCs:** TBD
> **Related Missions:** TBD

## Summary

Stoolap Chain will integrate comprehensive zero-knowledge proof support using STWO (Circle STARK prover/verifier in Rust) and Cairo (provable programming language). This enables three major capabilities: proof compression, confidential queries, and L2 scaling.

## Architecture Overview

### Core Capabilities

1. **Proof Compression** - Aggregate multiple HexaryProofs into single STARK proofs
2. **Confidential Queries** - Private database queries with ZK proofs of correctness
3. **L2 Scaling** - Batch transactions off-chain, post only validity proofs on-chain

### Integration Approach

- **STWO Integration**: Native Rust integration where Stoolap Chain directly links to STWO libraries
- **Cairo Programs**: Full Cairo programs for core operations (hash calculations, state transitions)
- **Verification**: Cairo contracts running on Cairo VM - keeps consensus simple while enabling programmability

### Technology Stack

- **STWO** - Rust prover/verifier framework for Circle STARKs
- **Cairo** - Turing-complete language for provable programs
- **Sierra** - Intermediate representation for Cairo compilation
- **CASM** - Cairo Assembly Machine (final executable format)

## Components and Data Flow

### Prover Layer (Rust + Cairo)

- `CairoProgramRegistry` - Manages compiled Cairo programs (Sierra → CASM)
- `STWOProver` - Wraps STWO's prover API, takes Cairo execution trace and inputs
- `ProofCompressor` - Aggregates HexaryProofs, generates compressed STARK proof
- `ConfidentialQueryProver` - Generates proofs for private query results

### Verification Layer (Cairo Contracts)

- `ProofVerifier.cairo` - On-chain contract to verify STARK proofs
- `CompressedProofVerifier.cairo` - Verifies aggregated proof commitments
- `ConfidentialQueryVerifier.cairo` - Verifies query result proofs without revealing data

### Integration Layer (Stoolap Core)

- `ZKOperation` enum - New operation variants for ZK-related actions
- `ProofStorage` - Stores and indexes submitted STARK proofs
- `StateCommitmentBridge` - Connects RowTrie state to Cairo execution context

### Data Flow

```
Client Request → Cairo Program Compilation → STWO Prover → STARK Proof
                                                      ↓
                                              On-Chain Cairo Contract
                                                      ↓
                                              Verification & State Update
```

## Sequential Implementation Phases

### Phase 1: Proof Compression (Foundation)

**Goal**: Build STWO integration infrastructure and proof compression

**Deliverables**:
- STWO Rust integration with Cairo program compilation pipeline
- `ProofCompressor` that takes N HexaryProofs and outputs one STARK proof
- `CompressedProofVerifier.cairo` contract
- Single STARK proof verifies 100+ row inclusions vs 100 separate Merkle proofs

**Success Metrics**:
- Proof generation <1s for 100 rows
- Verification time <100ms on Cairo VM
- Proof size <10KB for compressed batch of 100

### Phase 2: Confidential Queries (Privacy)

**Goal**: Enable private database queries with ZK proofs

**Deliverables**:
- Cairo programs for encrypted query execution
- Private input/output handling using pedersen commitments
- `ConfidentialQueryProver` and verifier contract
- Users prove query results without revealing data to chain

**Success Metrics**:
- Confidential query proof <500ms to generate
- Verification <150ms on-chain
- Zero knowledge property: Verifier learns nothing about inputs

### Phase 3: L2 Scaling (Throughput)

**Goal**: Rollup transactions off-chain with validity proofs

**Deliverables**:
- Transaction batch structure with Cairo state transition logic
- Rollup operator processing batches off-chain
- `RollupVerifier.cairo` posting batch state roots
- 1000+ TPS via off-chain execution with on-chain proofs

**Success Metrics**:
- 1000+ transactions per second
- Finality time <1 minute
- Batch proof size <50KB for 1000 transactions

## Cairo Program Structure

### Core Cairo Programs

1. **`state_transition.cairo`** - Validates RowTrie state transitions
   - Input: Previous root hash, operation list, new root hash
   - Output: STARK proof of valid transition
   - Uses Cairo's built-in poseidon hash for state hashing

2. **`hexary_verify.cairo`** - Hexary proof verification in Cairo
   - Input: Row ID, value, HexaryProof, expected root
   - Output: Boolean validity + STARK proof
   - Enables compressed proof batches

3. **`confidential_query.cairo`** - Private query execution
   - Input: Encrypted query, pedersen commitment of data
   - Output: Query result + proof of correct execution
   - Zero-knowledge property: Verifier learns nothing about inputs

### Compilation Pipeline

```
.cairo source → Sierra (IR) → CASM (executable) → STWO Prover
```

### Program Storage

- Compiled CASM stored in `CairoProgramRegistry`
- Identified by blake3 hash of source code
- Chain maintains allowlist of approved program hashes

### Gas Costs

- Proof generation: Off-chain (free)
- Proof verification: On-chain Cairo VM execution (gas metered)
- Estimated: ~50K-200K gas per STARK verification (vs ~5K for single Merkle proof)

## Error Handling

### Proof Generation Failures

- Cairo compilation errors → Return to client with source location
- STWO prover failures → Retry with different parameters, timeout after 30s
- Out of memory during proving → Split into smaller batches

### Verification Failures

- Invalid STARK proof → Reject operation, don't charge gas
- Program hash not in allowlist → Reject with "unauthorized program"
- Cairo VM execution error → Revert block (consensus failure)

### Fallback Behavior

- If STWO unavailable → Fall back to raw HexaryProof verification
- If proof too large → Reject with "proof exceeds maximum size"

## Testing Strategy

### Unit Tests

- Cairo program compilation tests
- STWO prover wrapper tests (mock prover)
- Proof storage and retrieval tests

### Integration Tests

- End-to-end proof generation and verification
- Cairo VM execution tests
- Program registry management tests

### Property-Based Tests

- Valid proofs always verify
- Invalid proofs (tampered inputs) always reject
- Proof size bounded by N × log(M) where N=operations, M=state size

### Benchmarks

- Proof generation time (target: <1s for 100 rows)
- Verification time (target: <100ms on Cairo VM)
- Proof size (target: <10KB for compressed batch of 100)

## Security Considerations

- Cryptographic assumptions: Circle STARK security, Cairo soundness
- Attack vectors: Malicious Cairo programs, proof tampering, DoS via large proofs
- Mitigation: Program allowlist, proof size limits, gas metering for verification

## Related Work

- [STWO Repository](https://github.com/starkware-libs/stwo-cairo) - Prover/verifier framework
- [Cairo Language](https://github.com/starkware-libs/cairo) - Provable programming language
- [RFC-0101](../../rfcs/0101-hexary-merkle-proofs.md) - Hexary Merkle Proofs (base for compression)
- [RFC-0103](../../rfcs/0103-blockchain-consensus.md) - Blockchain Consensus (integration point)

## Open Questions

1. Should Cairo programs be upgradeable? If so, what's the governance process?
2. What's the maximum proof size the chain should accept?
3. Should there be a bond/slash mechanism for submitting invalid proofs?
4. How do we handle STWO protocol upgrades?
