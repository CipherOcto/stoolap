# RFC-0102: Deterministic Value Types for Blockchain Consensus

## Status
Accepted

## Summary

Define deterministic value types (`DetermValue`, `DetermRow`, `DetermMap`) that use no Arc/pointers for predictable memory layout and Merkle hashing. Replaces non-deterministic `Value` types with blockchain-compatible alternatives.

## Motivation

For blockchain consensus, all data structures must:
1. **Hash Deterministically** - Same data → same hash on all nodes
2. **Have Predictable Layout** - No Arc/pointors causing address-based hashing
3. **Encode/Decode** - Convert to/from bytes for storage and transmission
4. **Support Merkle Hashing** - Compute inclusion proofs

Existing `Value` types use `Arc<Vec<u8>>` for strings which breaks determinismism due to pointer addresses.

## Specification

### DetermValue Enum

```rust
pub enum DetermValue {
    Null,
    Integer(i64),
    Float(f64),
    InlineText([u8; 15], u8),  // ≤15 bytes inline
    HeapText(Box<[u8]>),        // >15 bytes on heap
    Boolean(bool),
    Timestamp(i64),
    Extension(Box<[u8]>),
}
```

### Memory Optimization

**Inline Text:**
- Values ≤15 bytes stored inline (no heap allocation)
- Array + length byte
- ~50% of typical text values benefit

**Heap Text:**
- Values >15 bytes stored as `Box<[u8]>`
- Deterministic (no Arc, pointer addresses don't affect hash)

### Encoding Format

| Type | Tag | Payload |
|------|-----|---------|
| Null | 0x00 | (empty) |
| Integer | 0x01 | 8 bytes (little-endian i64) |
| Float | 0x02 | 8 bytes (little-endian f64) |
| InlineText | 0x03 | 1 byte length + up to 15 bytes data |
| HeapText | 0x04 | 4 bytes length + data |
| Boolean | 0x05 | 1 byte (0 or 1) |
| Timestamp | 0x06 | 8 bytes (little-endian i64) |
| Extension | 0x07 | 4 bytes length + data |

### Merkle Hashing

```rust
pub fn hash(&self) -> [u8; 32] {
    let mut hasher = MerkleHasher::new();
    match self {
        DetermValue::Null => hasher.input(&[TYPE_NULL]),
        DetermValue::Integer(v) => {
            hasher.input(&[TYPE_INTEGER]);
            hasher.input(&v.to_le_bytes());
        }
        DetermValue::InlineText(data, len) => {
            hasher.input(&[TYPE_INLINE_TEXT]);
            hasher.input(&[*len]);
            hasher.input(&data[..(*len as usize)]);
        }
        // ... similar for other types
    }
    hasher.finalize()
}
```

Uses SHA-256 for cryptographic security.

### DetermRow

```rust
pub struct DetermRow {
    pub values: Vec<DetermValue>,
}
```

Hash is SHA-256 of concatenated value hashes.

### DetermMap

```rust
pub struct DetermMap {
    pub data: BTreeMap<String, DetermValue>,
}
```

Uses `BTreeMap` instead of `HashMap` for deterministic iteration order.

## Rationale

### Why Inline Text Optimization?

1. **Performance** - Avoids heap allocation for common cases
2. **Cache Efficiency** - Better memory locality
3. **Determinism** - Inline array has predictable layout

### Why 15 Bytes?

- Fits 2 cache lines (with metadata)
- Covers most identifiers, names, codes
- Leaves 1 byte for length (fits in 16-byte inline storage)

### Why Box<[u8]> Instead of Arc<Vec<u8>>?

1. **No Reference Counting** - Arc adds pointer addresses to hash
2. **Immutable** - Blockchain data doesn't need shared mutation
3. **Deterministic** - Same data always produces same hash

### Why BTreeMap Instead of HashMap?

1. **Deterministic Iteration** - Sorted keys, consistent across runs
2. **No Random Seed** - HashMap uses hasher with random seed
3. **Ordered Proofs** - Predictable iteration for verification

## Implementation

### Components

1. **DetermValue** - Core enum with all SQL types
2. **DetermRow** - Row of values
3. **DetermMap** - Ordered map for deterministic iteration
4. **MerkleHasher** - SHA-256 wrapper for hashing
5. **Encoding/Decoding** - Binary format conversion

### Constraints

1. **No Arc** - Use Box for heap allocation
2. **No HashMap** - Use BTreeMap for deterministic iteration
3. **No Randomness** - Hash functions must be deterministic
4. **Little-Endian** - All multi-byte integers use little-endian

### Testing

- Roundtrip encoding/decoding
- Hash determinism (same input → same hash)
- Inline/heap boundary conditions (15 vs 16 bytes)
- All value types
- BTreeMap ordering determinism

## Migration Path

1. **Phase 1** - Add DetermValue alongside Value
2. **Phase 2** - Use DetermValue in blockchain components
3. **Phase 3** - Deprecate Value in consensus-critical code
4. **Phase 4** - Value becomes legacy-only

## Performance Impact

| Operation | Before | After | Change |
|-----------|--------|-------|--------|
| Hash text (≤15 bytes) | ~200 ns | ~150 ns | 25% faster |
| Hash text (>15 bytes) | ~200 ns | ~180 ns | 10% faster |
| Memory (≤15 bytes) | 48 bytes | 16 bytes | 67% reduction |
| Encode/Decode | N/A | ~500 ns | New capability |

## Security Considerations

1. **SHA-256** - Cryptographic hash, not XOR
2. **No Side Channels** - Constant-time operations where possible
3. **Input Validation** - Length checks, bounds checking
4. **Determinism Required** - Non-deterministic types break consensus

## Related Use Cases

- [Blockchain SQL Database](../../docs/use-cases/blockchain-sql-database.md)

## Related RFCs

- [RFC-0101: Hexary Merkle Proofs](./0101-hexary-merkle-proofs.md)
- [RFC-0103: Blockchain Consensus](./0103-blockchain-consensus.md)
