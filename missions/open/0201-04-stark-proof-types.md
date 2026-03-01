# Mission: Stark Proof Types

## Status
Open

## RFC
RFC-0201: STWO and Cairo Integration for Zero-Knowledge Proofs

## Acceptance Criteria
- [ ] Define `StarkProof` struct with all required fields
- [ ] Define `ZKOperation` enum with all variants
- [ ] Implement `StarkProof` serialization/deserialization
- [ ] Add SolanaSerialize trait implementation for StarkProof
- [ ] Add comprehensive tests for proof encoding/decoding
- [ ] Add proof size limit constants

## Dependencies
- Mission 0201-01 (STWO Dependency)
- Mission 0201-02 (Cairo Program Types)

## Enables
- Mission 0201-05 (Prover Interface)

## Implementation Notes

**Files to Create:**
- `src/zk/proof.rs` - STARK proof types

**Data Structures:**
```rust
pub struct StarkProof {
    pub program_hash: CairoProgramHash,
    pub inputs: Vec<u8>,
    pub outputs: Vec<u8>,
    pub proof: Vec<u8>,
    pub public_inputs: Vec<u8>,
}

pub enum ZKOperation {
    SubmitProof {
        proof: StarkProof,
        gas_limit: u64,
    },
    RegisterProgram {
        program: CairoProgram,
    },
    VerifyProof {
        proof: StarkProof,
    },
}
```

**Serialization Format:**
| Field | Size | Description |
|-------|------|-------------|
| program_hash | 32 bytes | Blake3 hash |
| inputs_len | 4 bytes | Length |
| inputs | variable | Inputs |
| outputs_len | 4 bytes | Length |
| outputs | variable | Outputs |
| proof_len | 4 bytes | Length |
| proof | variable | STWO proof |
| public_inputs_len | 4 bytes | Length |
| public_inputs | variable | Public inputs |

## Claimant
Open

## Pull Request
TBD

## Commits
TBD

## Completion Date
TBD
