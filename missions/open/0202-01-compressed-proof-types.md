# Mission: Compressed Proof Types

## Status
Open

## RFC
RFC-0202: Compressed Proof Format for Batch Verification

## Acceptance Criteria
- [ ] Define `CompressedProof` struct
- [ ] Define `BatchVerifyInput` struct
- [ ] Implement SolanaSerialize for CompressedProof
- [ ] Add compression ratio calculation utility
- [ ] Add tests for proof encoding/decoding
- [ ] Add tests for size limits

## Dependencies
- RFC-0201 (STWO Integration) - Complete
- Mission 0201-04 (Stark Proof Types)

## Enables
- Mission 0202-02 (Proof Generation)

## Implementation Notes

**Files to Create:**
- `src/zk/compressed.rs` - Compressed proof types

**Data Structures:**
```rust
pub struct CompressedProof {
    pub program_hash: [u8; 32],  // merkle_batch.cairo
    pub row_count: u64,
    pub root: [u8; 32],
    pub stark_proof: StarkProof,
}

pub struct BatchVerifyInput {
    pub row_ids: Vec<i64>,
    pub values: Vec<DetermValue>,
    pub proofs: Vec<HexaryProof>,
    pub expected_root: [u8; 32],
}
```

**Encoding Format:**
| Field | Size | Description |
|-------|------|-------------|
| program_hash | 32 bytes | Hash of merkle_batch.cairo |
| row_count | 8 bytes | Number of rows in batch |
| root | 32 bytes | Expected state root |
| stark_proof | variable | STWO proof |

**Compression Ratio:**
```rust
impl CompressedProof {
    pub fn compression_ratio(&self) -> f64 {
        let original_size = self.row_count as usize * 68; // Avg HexaryProof size
        let compressed_size = self.stark_proof.proof.len() + 72; // + header
        original_size as f64 / compressed_size as f64
    }
}
```

## Claimant
Open

## Pull Request
TBD

## Commits
TBD

## Completion Date
TBD
