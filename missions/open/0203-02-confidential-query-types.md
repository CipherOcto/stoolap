# Mission: Confidential Query Types

## Status
Open

## RFC
RFC-0203: Confidential Query Operations

## Acceptance Criteria
- [ ] Define `EncryptedQuery` struct
- [ ] Define `EncryptedFilter` struct
- [ ] Define `FilterOp` enum
- [ ] Define `ConfidentialResult` struct
- [ ] Define `RangeProof` struct
- [ ] Implement SolanaSerialize for all types
- [ ] Add tests for query encoding/decoding

## Dependencies
- RFC-0201 (STWO Integration) - Complete
- Mission 0203-01 (Commitment Scheme)

## Enables
- Mission 0203-03 (Query Execution)

## Implementation Notes

**Files to Create:**
- `src/zk/confidential.rs` - Confidential query types

**Data Structures:**
```rust
pub struct EncryptedQuery {
    pub table: String,
    pub filters: Vec<EncryptedFilter>,
    pub nonce: [u8; 32],
}

pub struct EncryptedFilter {
    pub column: String,
    pub operator: FilterOp,
    pub value_commitment: Commitment,
    pub proof: RangeProof,
}

pub enum FilterOp {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
}

pub struct ConfidentialResult {
    pub row_count: u64,
    pub value_commitments: Vec<Commitment>,
    pub proof: StarkProof,
    pub opening_key: Option<[u8; 32]>,
}

pub struct RangeProof {
    pub proof: Vec<u8>,
    pub commitment: Commitment,
    pub min: i64,
    pub max: i64,
}
```

**Encryption:**
```rust
pub fn encrypt_query(query: &Query, public_key: &[u8]) -> EncryptedQuery {
    // X25519 + ChaCha20-Poly1305
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
