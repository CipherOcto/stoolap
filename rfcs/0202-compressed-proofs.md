# RFC-0202: Compressed Proof Format for Batch Verification

## Status
Draft

## Summary

Define a compressed proof format that aggregates multiple HexaryProofs into a single STARK proof. Enables efficient batch verification and reduces bandwidth by ~90% for large query results.

## Motivation

HexaryProofs provide compact individual verification (~68 bytes each), but applications often need to verify many rows:

- **Large Queries**: `SELECT * FROM users WHERE region='us'` might return 1000 rows
- **Batch Operations**: Verifying 100 transactions requires 100 proofs
- **State Sync**: Initial download requires proving entire database

Current cost: 1000 rows × 68 bytes = 68 KB
Target cost: 1000 rows in ~10 KB via STARK compression

## Specification

### Data Structures

```rust
/// Compressed proof aggregating multiple HexaryProofs
pub struct CompressedProof {
    pub program_hash: [u8; 32],  // References merkle_batch.cairo
    pub row_count: u64,          // Number of rows proven
    pub root: [u8; 32],          // Expected state root
    pub stark_proof: StarkProof, // STWO proof of batch verification
}

/// Input for batch verification
pub struct BatchVerifyInput {
    pub row_ids: Vec<i64>,
    pub values: Vec<DetermValue>,
    pub proofs: Vec<HexaryProof>,
    pub expected_root: [u8; 32],
}
```

### Cairo Program: `merkle_batch.cairo`

```cairo
// merkle_batch.cairo - Batch verify HexaryProofs

#[derive(Drop, Serde)]
struct ProofLevel {
    bitmap: u16,
    siblings: Array<u8>,  // Flattened hash array
}

#[derive(Drop, Serde)]
struct HexaryProof {
    value_hash: u256,
    levels: Array<ProofLevel>,
    path: Array<u8>,
}

#[derive(Drop, Serde)]
struct BatchInput {
    row_ids: Array<i64>,
    values: Array<u256>,
    proofs: Array<HexaryProof>,
    expected_root: u256,
}

// Batch verify all proofs
fn batch_verify(input: BatchInput, expected_root: u256) -> bool {
    let mut valid = true;
    let mut i = 0;

    // Verify each proof
    while i < input.proofs.len() {
        let proof = input.proofs[i];
        let row_id = input.row_ids[i];
        let value = input.values[i];

        // Hash the value
        let value_hash = hash_value(value);

        // Verify proof against expected root
        if !verify_hexary_proof(proof, value_hash, expected_root) {
            valid = false;
        }

        i += 1;
    }

    valid
}

// Verify single hexary proof
fn verify_hexary_proof(
    proof: HexaryProof,
    value_hash: u256,
    expected_root: u256,
) -> bool {
    let mut current_hash = value_hash;
    let path_nibbles = unpack_nibbles(proof.path);

    let mut level_idx = 0;
    while level_idx < proof.levels.len() {
        let level = proof.levels[level_idx];
        let path_nibble = path_nibbles[level_idx];

        // Reconstruct children hash array
        let children = reconstruct_children(
            level.bitmap,
            level.siblings,
            path_nibble,
            current_hash
        );

        // Hash 16 children
        current_hash = hash_16_children(children);

        level_idx += 1;
    }

    current_hash == expected_root
}

// Hash value to u256
fn hash_value(value: u256) -> u256 {
    poseidon(value)
}

// Unpack nibbles from path
fn unpack_nibbles(path: Array<u8>) -> Array<u8> {
    // Implementation from RFC-0101
    // ...
}

// Reconstruct children array from bitmap and siblings
fn reconstruct_children(
    bitmap: u16,
    siblings: Array<u8>,
    path_nibble: u8,
    path_hash: u256,
) -> Array<u256> {
    // Implementation from RFC-0101
    // ...
}

// Hash 16 children
fn hash_16_children(children: Array<u256>) -> u256 {
    poseidon_batch(children)
}
```

### Proof Generation

```rust
impl RowTrie {
    /// Generate compressed proof for multiple rows
    pub fn get_compressed_proof(
        &self,
        row_ids: &[i64],
    ) -> Option<CompressedProof> {
        // Collect individual proofs
        let mut proofs = Vec::new();
        let mut values = Vec::new();

        for &row_id in row_ids {
            let proof = self.get_hexary_proof(row_id)?;
            let value = self.get_row(row_id)?;
            proofs.push(proof);
            values.push(value);
        }

        // Get batch verification program
        let program = self.get_batch_program()?;

        // Create batch input
        let input = BatchVerifyInput {
            row_ids: row_ids.to_vec(),
            values,
            proofs,
            expected_root: self.get_root(),
        };

        // Serialize input for Cairo
        let cairo_input = serialize_batch_input(&input);

        // Generate STARK proof
        let prover = STWOProver::new();
        let stark_proof = prover.prove(program, &cairo_input).ok()?;

        Some(CompressedProof {
            program_hash: program.hash,
            row_count: row_ids.len() as u64,
            root: self.get_root(),
            stark_proof,
        })
    }
}
```

### Verification

```rust
impl CompressedProof {
    /// Verify compressed proof
    pub fn verify(&self) -> bool {
        // Get program from registry
        let registry = CairoProgramRegistry::get_global();
        let program = match registry.get(&self.program_hash) {
            Some(p) => p,
            None => return false,
        };

        // Verify STARK proof
        let verifier = STWOVerifier::new();
        match verifier.verify(&self.stark_proof) {
            Ok(true) => true,
            _ => false,
        }
    }
}
```

### Encoding Format

#### CompressedProof Encoding

| Field | Size | Description |
|-------|------|-------------|
| program_hash | 32 bytes | Hash of merkle_batch.cairo |
| row_count | 8 bytes | Number of rows in batch |
| root | 32 bytes | Expected state root |
| stark_proof | variable | STWO proof (see RFC-0201) |

### Batch Sizes

Recommended batch sizes based on profiling:

| Rows | Expected Proof Size | Generation Time |
|------|---------------------|-----------------|
| 10 | ~2 KB | ~100ms |
| 100 | ~5 KB | ~500ms |
| 1,000 | ~10 KB | ~1s |
| 10,000 | ~20 KB | ~5s |

### Gas Costs

- Verify compressed proof: 50,000 + (row_count × 100) gas
- Generate proof: Off-chain (free)

## Rationale

### Why STARK Compression?

1. **Constant Size** - Proof size grows logarithmically with batch size
2. **Fast Verification** - Single crypto operation vs N operations
3. **Recursion** - Compressed proofs can be further compressed

### Why Separate Cairo Program?

1. **Upgradability** - Can improve algorithm without consensus change
2. **Auditability** - Single program to review for security
3. **Reuse** - Other components can use same program

### Why Not Simple Aggregation?

- Naive aggregation: Just concatenate proofs (size = N × 68 bytes)
- STARK compression: Prove you verified all proofs (size ≈ 10 KB constant)

## Implementation

### Components

1. **Cairo Program** - `merkle_batch.cairo` implementation
2. **Proof Generation** - `RowTrie::get_compressed_proof()`
3. **Verification** - `CompressedProof::verify()`
4. **Serialization** - CompressedProof encoding/decoding
5. **Integration** - Connect to STWO prover/verifier

### Dependencies

- Requires: RFC-0201 (STWO/Cairo Integration)
- Enables: Efficient large query results, state sync optimization

### Testing Requirements

- Unit tests for Cairo program
- Integration tests for proof generation
- Property tests: valid proofs always verify
- Benchmarks for various batch sizes
- Cross-validate: compressed proof == individual proofs

## Performance Targets

| Metric | Target | Actual (TBD) |
|--------|--------|--------------|
| Proof size (100 rows) | <10 KB | TBD |
| Verification time (100 rows) | <100ms | TBD |
| Generation time (100 rows) | <1s | TBD |
| Compression ratio (100 rows) | >90% | TBD |

## Security Considerations

1. **Proof Validity** - STARK proof must be valid
2. **Program Hash** - Must match approved batch program
3. **Root Match** - Expected root must match chain state
4. **Row Count** - Must match actual number of proofs

## Backward Compatibility

- HexaryProofs remain supported for individual verification
- Compressed proofs are optional optimization
- Clients can choose either format

## Related Use Cases

- [ZK Proofs for Scalability and Privacy](../../docs/use-cases/zk-proofs-scalability.md)

## Related RFCs

- [RFC-0101: Hexary Merkle Proofs](./0101-hexary-merkle-proofs.md)
- [RFC-0201: STWO/Cairo Integration](./0201-stwo-cairo-integration.md)

## Open Questions

1. What's the maximum batch size we should support?
2. Should there be a fee discount for compressed proofs?
3. How do we handle proof generation timeouts for large batches?

## References

- [STWO Prover Documentation](https://github.com/starkware-libs/stwo-cairo)
- [Cairo Language Reference](https://book.cairo-lang.org/)
