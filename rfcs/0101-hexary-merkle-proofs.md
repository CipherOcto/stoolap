# RFC-0101: Hexary Merkle Proofs for Blockchain SQL

## Status
Accepted

## Summary

Define a compact, verifiable Merkle proof format for 16-way hexary tries. Replaces inefficient binary-tree proof format with bitmap-based sibling encoding and nibble-packed paths.

## Motivation

Existing Merkle proof formats (`MerkleProof` in this codebase) assume binary trees with left/right branching. This is incompatible with hexary tries like `RowTrie` which use 16-way branching for efficiency.

Binary proofs for hexary tries require:
- Storing all 15 sibling hashes per level
- Complex path encoding
- Proof sizes 5-10x larger than necessary

## Specification

### Data Structures

```rust
pub struct ProofLevel {
    pub bitmap: u16,           // 16-bit bitmap of existing children
    pub siblings: Vec<[u8; 32]>, // Only non-path children
}

pub struct HexaryProof {
    pub value_hash: [u8; 32],  // Hash of proven value
    pub levels: Vec<ProofLevel>, // Proof levels from root to leaf
    pub root: [u8; 32],         // Expected Merkle root
    pub path: Vec<u8>,          // Nibble path (2 nibbles/byte, LSB first)
}
```

### Encoding Format

#### Bitmap Encoding
- 16-bit bitmap indicates which child positions (0-15) have hashes
- Only store sibling hashes (not the path child)
- Reduces proof size by ~10x for sparse tries

#### Nibble Path Encoding
- 2 nibbles packed per byte (LSB first)
- Odd-length paths: last byte uses only low nibble
- Example: `[5, 12, 3]` → `[0x35, 0xC3, 0x03]`

#### Extension Node Handling
- Extension nodes are flattened during proof generation
- Prefix nibbles added directly to path
- Simplifies verification logic

### Verification Algorithm

```rust
fn verify(&self) -> bool {
    let mut current_hash = self.value_hash;
    let path_nibbles = unpack_nibbles(&self.path);

    for (level_idx, level) in self.levels.iter().enumerate() {
        let path_nibble = path_nibbles[level_idx];
        let children = reconstruct_children(level.bitmap, &level.siblings, path_nibble, current_hash);
        current_hash = hash_16_children(&children);
    }

    current_hash == self.root
}
```

### Serialization Format

Solana-style binary format:

| Field | Size | Description |
|-------|------|-------------|
| value_hash | 32 bytes | Hash of proven value |
| root | 32 bytes | Expected Merkle root |
| path_len | 1 byte | Length of path |
| path | variable | Packed nibble path |
| levels_len | 1 byte | Number of levels |
| levels | variable | Proof levels |

Each level:
| Field | Size | Description |
|-------|------|-------------|
| bitmap | 2 bytes | 16-bit child bitmap (little-endian) |
| sibling_count | 1 byte | Number of siblings |
| siblings | variable | Sibling hashes (32 bytes each) |

### Batch Verification

```rust
#[cfg(feature = "parallel")]
pub fn verify_batch(proofs: &[HexaryProof]) -> bool {
    proofs.par_iter().all(|p| p.verify())
}

pub fn verify_batch_sequential(proofs: &[HexaryProof]) -> bool {
    proofs.iter().all(|p| p.verify())
}
```

## Rationale

### Why Bitmap Encoding?

1. **Compactness** - Only store actual siblings, not empty positions
2. **Clarity** - Bitmap clearly indicates which children exist
3. **Extensibility** - Handles cases with >2 non-empty children (deletions)
4. **Industry Standard** - Similar to Ethereum's hexary proof approach

### Why Nibble Packing?

1. **2x Reduction** - Half the bytes compared to byte-per-nibble
2. **Fast Operations** - Simple bit operations
3. **No Padding Ambiguity** - Path length tracked separately

### Why Extension Flattening?

1. **Simpler Verification** - Verifiers don't need extension logic
2. **Smaller Proofs** - No extension node overhead
3. **Implementation Detail** - Extensions are storage optimization, not semantic

## Implementation

### Core Components

1. **Data Structures** - `HexaryProof`, `ProofLevel`
2. **Nibble Utilities** - `pack_nibbles()`, `unpack_nibbles()`
3. **Reconstruction** - `reconstruct_children()`
4. **Hashing** - `hash_16_children()`
5. **Verification** - `HexaryProof::verify()`
6. **Proof Generation** - `RowTrie::get_hexary_proof()`
7. **Serialization** - `SolanaSerialize` trait
8. **Batch Verification** - Parallel and sequential

### Testing Requirements

- Unit tests for each component
- Roundtrip serialization tests
- Proof generation and verification tests
- Extension flattening tests
- Batch verification tests
- Benchmarks for performance validation

## Performance Targets

| Metric | Target |
|--------|--------|
| Proof size (typical) | <100 bytes |
| Verification time | <5 μs |
| Batch verification (100) | <50 μs single-threaded |
| Batch verification (100) | <15 μs parallel (8 cores) |

## Security Considerations

1. **Streaming Verification** - Fails immediately on mismatch, prevents attack amplification
2. **SHA-256** - Cryptographic hash function, not XOR
3. **Deterministic** - Same inputs always produce same outputs
4. **No Early Exit** - Full verification even if some levels pass

## Backward Compatibility

This RFC introduces `HexaryProof` alongside existing `MerkleProof`. The old type is deprecated but remains for backward compatibility during transition period.

## Related Use Cases

- [Verifiable State Proofs](../../docs/use-cases/verifiable-state-proofs.md)
- [Blockchain SQL Database](../../docs/use-cases/blockchain-sql-database.md)

## Related RFCs

- [RFC-0102: Deterministic Value Types](./0102-deterministic-types.md)
- [RFC-0103: Blockchain Consensus](./0103-blockchain-consensus.md)
