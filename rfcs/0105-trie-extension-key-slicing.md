# RFC-0105: Trie Extension Key Slicing Fix

## Status
**Accepted** - Implemented in commit 89aba23

## Summary

Fixes the depth handling bug in hexary trie extension node traversal. The insertion code was slicing the key but incorrectly incrementing depth, causing lookup failures for sequential row IDs.

## Motivation

The original trie implementation had a subtle bug:

| Operation | Prefix Check | Key Slicing | Depth |
|-----------|--------------|-------------|-------|
| Insertion | `key.starts_with(&prefix)` | `&key[prefix.len()..]` | ❌ `depth + prefix.len()` |
| Lookup | `key[depth..]` | No slicing | `depth` |

**The Bug**: After slicing the key in insertion, the depth was still incremented. This caused subsequent branch navigation to use `key[depth]` which accessed the wrong position after slicing.

For example:
- Original key: `[0, 1, 0, 0, ...]`, depth = 0, prefix = `[0]`
- After slicing: key = `[1, 0, 0, ...]`, depth = 1 (WRONG!)
- Branch used: `key[1]` = `0` but should be `key[0]` = `1`

## Specification

### The Fix

**1. Insertion** - Reset depth to 0 after slicing:
```rust
// Before (buggy):
&key[prefix.len()..],
depth + prefix.len()

// After (fixed):
&key[prefix.len()..],
0  // Reset depth - key is now relative
```

**2. Lookup** - Slice key and reset depth to match:
```rust
// Extension case in do_get_hash, do_get, do_get_hexary_proof:
// Before:
key,  // Don't slice
depth + prefix.len()

// After:
&key[depth + prefix.len()..],  // Slice past the prefix
0  // Reset depth - key is now relative
```

**3. Leaf Cases** - Simplified check after slicing:
```rust
// After extension slicing, key contains remaining nibbles
if key.is_empty() || key.iter().all(|&x| x == 0) {
    // Found the leaf
}
```

### Affected Functions

All three traversal functions needed changes:

1. **`do_get_hash`** - Used by `get_hash()` for proof generation
2. **`do_get`** - Used by `get()` for row retrieval
3. **`do_get_hexary_proof`** - Used by `get_hexary_proof()` for proof generation

## Implementation

### Files Modified

- `src/trie/row_trie.rs`:
  - Line ~395: Insertion extension case - reset depth to 0
  - Line ~558: do_get_hash extension case - add key slicing
  - Line ~580: do_get extension case - add key slicing
  - Line ~606: do_get extension case - add key slicing
  - Line ~679: do_get_hexary_proof leaf case - simplified check
  - Line ~730: do_get_hexary_proof extension case - add key slicing

### Test Results

- **1934 tests pass** (all tests including zk feature)
- Regression test: `test_sequential_row_ids_1_to_10` passes
- Regression test: `test_sequential_row_ids_1_to_100` passes
- Sequential row IDs 1-100 now fully retrievable

### Additional Fix (2026-03-01)

A second bug was discovered when testing 100 sequential rows: the extension split case
was not properly handling both old and new paths. When inserting a Row that diverges from
an existing extension prefix, the old child was being discarded.

**The Second Bug**: In the else branch that splits an extension:
```rust
// Old code - BUG: only inserted new row, discarded old child
let child = Box::new(Self::do_insert_static(Some(*child), &prefix[1..], depth + 1, row_id, row));
// Only placed at prefix[0], never handled new row's position
```

**The Fix**: Place both old child AND new leaf in the branch:
```rust
// Old path: at branch[prefix[0]]
if let RowNode::Branch { ref mut children, .. } = branch {
    children[prefix[0] as usize] = Some(child.clone());
}
// New path: at branch[key[depth]]
if let RowNode::Branch { ref mut children, .. } = branch {
    children[key[depth] as usize] = Some(Box::new(new_leaf));
}
```

### Known Limitation

- **RESOLVED**: Extended test for 100 sequential rows now passes

## Rationale

### Why This Approach?

1. **Fixes Root Cause** - Corrects the actual bug in insertion (depth increment)
2. **Consistent Behavior** - All three functions use same key slicing logic
3. **Minimal Code Change** - Only changes required lines, no restructuring

## Performance Considerations

- **Time complexity**: No change - still O(depth) for lookup
- **Space complexity**: No change - same number of recursive calls
- **Performance**: Identical to current implementation

## Security Considerations

- **Correctness**: This fixes broken behavior that could return wrong data
- **Determinism**: Preserves deterministic behavior
- **Data Integrity**: All inserted data is now correctly retrievable

## Related Use Cases

- [Trie Sequential Row Lookups](../../docs/use-cases/trie-sequential-lookups.md)

## Related RFCs

- [RFC-0101: Hexary Merkle Proofs](./0101-hexary-merkle-proofs.md)
- [RFC-0202: Compressed Proofs](./0202-compressed-proofs.md)

## Related Missions

- [Mission 0105: Trie Extension Key Slicing Fix](../../missions/open/0105-trie-extension-fix.md) - Completed
