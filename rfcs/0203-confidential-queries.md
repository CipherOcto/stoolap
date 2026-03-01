# RFC-0203: Confidential Query Operations

## Status
Draft

## Summary

Define confidential query operations that enable proving SQL query results without revealing underlying data. Uses pedersen commitments and ZK proofs to provide privacy while maintaining verifiability.

## Motivation

Current blockchain SQL database is fully transparent:
- All data is public on-chain
- All queries reveal access patterns
- No way to prove "value > X" without revealing value

This prevents use cases like:
- **Credit Checks**: Prove "score > 700" without revealing score
- **Medical Eligibility**: Verify condition without revealing diagnosis
- **Business Analytics**: Aggregate insights without leaking customer data

Confidential queries enable:
- **Data Minimization**: Reveal only what's necessary
- **Selective Disclosure**: Prove properties without revealing values
- **Compliance**: GDPR-friendly data verification

## Specification

### Data Structures

```rust
/// Pedersen commitment to a value
pub type Commitment = [u8; 32];

/// Encrypted query input
pub struct EncryptedQuery {
    pub table: String,           // Encrypted table name
    pub filters: Vec<EncryptedFilter>, // Encrypted WHERE clauses
    pub nonce: [u8; 32],         // Encryption nonce
}

/// Encrypted filter condition
pub struct EncryptedFilter {
    pub column: String,          // Encrypted column name
    pub operator: FilterOp,      // Comparison operator
    pub value_commitment: Commitment, // Committed value
    pub proof: RangeProof,       // Proof that value is in range
}

/// Filter operators
pub enum FilterOp {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
}

/// Confidential query result
pub struct ConfidentialResult {
    pub row_count: u64,          // Number of matching rows
    pub value_commitments: Vec<Commitment>, // Committed result values
    pub proof: StarkProof,       // ZK proof of correct execution
    pub opening_key: Option<[u8; 32]>, // If holder can open results
}

/// Range proof (Bulgarian or similar)
pub struct RangeProof {
    pub proof: Vec<u8>,          // Serialized proof
    pub commitment: Commitment,  // Committed value
    pub min: i64,                // Minimum value (public)
    pub max: i64,                // Maximum value (public)
}
```

### Cairo Program: `confidential_query.cairo`

```cairo
// confidential_query.cairo - Execute queries with encrypted inputs

#[derive(Drop, Serde)]
struct EncryptedFilter {
    column_hash: u256,           // Hash of column name
    operator: u8,                // 0=EQ, 1=LT, 2=GT, etc.
    value_commitment: u256,      // Pedersen commitment
    range_proof: Array<u8>,      // Range proof bytes
}

#[derive(Drop, Serde)]
struct EncryptedQuery {
    table_hash: u256,            // Hash of table name
    filters: Array<EncryptedFilter>,
    nonce: u256,
}

#[derive(Drop, Serde)]
struct QueryResult {
    row_count: u64,
    value_commitments: Array<u256>,
}

// Execute confidential query
fn execute_confidential_query(
    query: EncryptedQuery,
    state_root: u256,
) -> QueryResult {
    // Decrypt query using private key (embedded in program)
    let decrypted = decrypt_query(query);

    // Execute query on state
    let mut row_count = 0;
    let mut commitments = ArrayTrait::new();

    // Iterate through rows (simplified)
    // In reality, this would use Merkle proofs to access state
    let mut row_idx = 0;
    while row_idx < MAX_ROWS {
        if matches_filters(decrypted.filters, row_idx, state_root) {
            let value = get_row_value(row_idx, state_root);
            commitments.append(pedersen_commit(value));
            row_count += 1;
        }
        row_idx += 1;
    }

    QueryResult {
        row_count,
        value_commitments: commitments,
    }
}

// Check if row matches all filters
fn matches_filters(
    filters: Array<EncryptedFilter>,
    row_idx: u64,
    state_root: u256,
) -> bool {
    let mut i = 0;
    while i < filters.len() {
        let filter = filters[i];
        let value = get_column_value(row_idx, filter.column_hash, state_root);

        if !matches_filter(filter, value) {
            return false;
        }

        i += 1;
    }

    true
}

// Check if value matches single filter
fn matches_filter(filter: EncryptedFilter, value: i64) -> bool {
    // Open commitment and compare
    let committed_value = open_commitment(filter.value_commitment);

    match filter.operator {
        0 => value == committed_value,      // EQ
        1 => value < committed_value,       // LT
        2 => value > committed_value,       // GT
        3 => value <= committed_value,      // LTE
        4 => value >= committed_value,      // GTE
        5 => value != committed_value,      // NEQ
        _ => false,
    }
}

// Pedersen commitment
fn pedersen_commit(value: i64) -> u256 {
    // Use built-in pedersen hash
    pedersen(value, 0)
}

// Open commitment (requires witness)
fn open_commitment(commitment: u256) -> i64 {
    // In reality, requires the random value used in commitment
    // This is a simplified version
    commitment
}

// Get row value from state
fn get_row_value(row_idx: u64, state_root: u256) -> i64 {
    // Use Merkle proof to access state
    // Simplified here
    0
}

// Get column value from row
fn get_column_value(row_idx: u64, column_hash: u256, state_root: u256) -> i64 {
    // Use Merkle proof to access state
    // Simplified here
    0
}
```

### Commitment Scheme

```rust
/// Pedersen commitment to a value
pub fn pedersen_commit(value: i64, randomness: u64) -> Commitment {
    // C = g^value * h^randomness
    let g = GENERATOR_G;
    let h = GENERATOR_H;
    let point = g.mul(value).add(h.mul(randomness));
    point.to_bytes()
}

/// Open commitment
pub fn open_commitment(
    commitment: &Commitment,
    value: i64,
    randomness: u64,
) -> bool {
    pedersen_commit(value, randomness) == *commitment
}

/// Batch commitment (more efficient)
pub fn pedersen_commit_batch(values: &[i64]) -> Vec<Commitment> {
    values.iter()
        .map(|&v| pedersen_commit(v, thread_rng().gen()))
        .collect()
}
```

### Range Proof

```rust
/// Prove value is in range [min, max] without revealing value
pub fn prove_range(
    value: i64,
    min: i64,
    max: i64,
    randomness: u64,
) -> RangeProof {
    // Use Bulletproofs or Bulgarian proof
    let commitment = pedersen_commit(value, randomness);
    let proof = bulletproof_prove(value, min, max, randomness);

    RangeProof {
        proof,
        commitment,
        min,
        max,
    }
}

/// Verify range proof
pub fn verify_range(proof: &RangeProof) -> bool {
    bulletproof_verify(
        &proof.proof,
        proof.commitment,
        proof.min,
        proof.max,
    )
}
```

### Query Execution

```rust
impl RowTrie {
    /// Execute confidential query
    pub fn execute_confidential_query(
        &self,
        query: EncryptedQuery,
    ) -> Result<ConfidentialResult, QueryError> {
        // Decrypt query
        let decrypted = decrypt_query(&query)?;

        // Collect matching rows
        let mut row_count = 0;
        let mut commitments = Vec::new();

        for (&row_id, row) in self.iter() {
            if matches_filters(&decrypted.filters, row) {
                commitments.push(pedersen_commit(row.value, random()));
                row_count += 1;
            }
        }

        // Create result
        let result = QueryResult {
            row_count,
            value_commitments: commitments.clone(),
        };

        // Get program
        let program = self.get_confidential_program()?;

        // Serialize input
        let cairo_input = serialize_confidential_input(query, result, self.get_root());

        // Generate proof
        let prover = STWOProver::new();
        let stark_proof = prover.prove(program, &cairo_input)?;

        Ok(ConfidentialResult {
            row_count,
            value_commitments: commitments,
            proof: stark_proof,
            opening_key: None, // Query holder can't open results
        })
    }
}
```

### Verification

```rust
impl ConfidentialResult {
    /// Verify confidential result
    pub fn verify(&self, expected_root: [u8; 32]) -> bool {
        // Verify STARK proof
        let registry = CairoProgramRegistry::get_global();
        let program = match registry.get_confidential_program() {
            Some(p) => p,
            None => return false,
        };

        let verifier = STWOVerifier::new();
        verifier.verify(&self.proof).unwrap_or(false)
    }

    /// Open commitment (requires opening key)
    pub fn open_commitment(
        &self,
        index: usize,
        opening_key: [u8; 32],
    ) -> Option<i64> {
        // Derive randomness from opening key
        let randomness = derive_randomness(opening_key, index as u64);

        // Brute-force or use opening hint
        // In reality, would store opening hints
        None
    }
}
```

### Encryption Scheme

- **Algorithm**: X25519 + ChaCha20-Poly1305
- **Key Derivation**: ECDH from client public key
- **Nonce**: 256-bit random nonce per query

## Rationale

### Why Pedersen Commitments?

1. **Privacy** - Hiding under discrete log assumption
2. **Additivity** - C(v1) + C(v2) = C(v1 + v2)
3. **Efficiency** - Simple elliptic curve operations

### Why Range Proofs?

1. **Bound Checks** - Prove "value > X" without revealing value
2. **Efficiency** - Single proof for range, not per comparison
3. **Standard** - Well-studied construction

### Why Separate Opening Keys?

1. **Selective Disclosure** - Data owner controls who can see results
2. **Revocation** - Can rotate opening keys
3. **Auditability** - Can prove you have access without revealing data

## Implementation

### Components

1. **Cairo Program** - `confidential_query.cairo`
2. **Encryption** - Query encryption/decryption
3. **Commitments** - Pedersen commitment scheme
4. **Range Proofs** - Bulletproofs implementation
5. **Execution** - Query execution with encrypted inputs
6. **Verification** - Result verification

### Dependencies

- Requires: RFC-0201 (STWO/Cairo), RFC-0202 (Compressed Proofs)
- Enables: Private database operations

### Testing Requirements

- Unit tests for commitment scheme
- Integration tests for query execution
- Property tests: proofs verify, invalid proofs reject
- Zero-knowledge tests: proof reveals nothing
- Benchmarks for query performance

## Performance Targets

| Metric | Target | Actual (TBD) |
|--------|--------|--------------|
| Query execution (1000 rows) | <500ms | TBD |
| Proof generation | <1s | TBD |
| Verification | <150ms | TBD |
| Proof size | <20 KB | TBD |

## Security Considerations

1. **Zero-Knowledge** - Proof must reveal nothing about inputs
2. **Binding** - Commitments can't be opened to different values
3. **Hiding** - Commitments reveal nothing about committed value
4. **Correctness** - Query must execute correctly on encrypted data
5. **Key Management** - Opening keys must be protected

## Privacy Considerations

- **Access Patterns**: Query execution time may leak information
- **Metadata**: Table and column names are still visible
- **Traffic Analysis**: Query frequency may reveal patterns

## Backward Compatibility

- Traditional queries remain supported
- Confidential queries are opt-in
- No breaking changes to existing operations

## Related Use Cases

- [ZK Proofs for Scalability and Privacy](../../docs/use-cases/zk-proofs-scalability.md)

## Related RFCs

- [RFC-0201: STWO/Cairo Integration](./0201-stwo-cairo-integration.md)
- [RFC-0202: Compressed Proofs](./0202-compressed-proofs.md)

## Open Questions

1. How do we handle table/column name privacy?
2. Should opening keys be recoverable?
3. What's the key rotation mechanism?
4. How do we prevent timing attacks?

## References

- [Pedersen Commitments](https://link.springer.com/chapter/10.1007/3-540-46766-1_9)
- [Bulletproofs](https://eprint.iacr.org/2017/1066)
- [Zero-Knowledge SQL](https://eprint.iacr.org/2019/1264)
