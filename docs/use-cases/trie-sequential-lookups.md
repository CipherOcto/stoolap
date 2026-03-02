# Use Case: Trie Sequential Row Lookups

## Status
**RESOLVED** - Fixed in commit 89aba23

## Problem

The hexary trie implementation failed to correctly retrieve sequential row IDs beyond the first two entries. When inserting rows with sequential IDs (1, 2, 3, ...), the trie could successfully retrieve rows 1 and 2, but lookup failed for row 3 and beyond.

This was a critical correctness bug that violated the fundamental contract of a key-value store: **if you insert a value, you must be able to retrieve it**.

### Actual Root Cause

The bug was in the insertion code itself, not just the lookup:

- **Insertion**: Sliced the key (`&key[prefix.len()..]`) but also incremented depth (`depth + prefix.len()`)
- **After slicing**: Using `key[depth]` accessed the wrong position because the key indices had shifted

For example:
- Original key: `[0, 1, 0, 0, ...]`, depth = 0, prefix = `[0]`
- After slicing: key = `[1, 0, 0, ...]`, depth = 1
- Branch used: `key[1]` = `0` but should be `key[0]` = `1`

## Fix Applied

### Changes to `src/trie/row_trie.rs`:

1. **Insertion** (line ~395): Reset depth to 0 after slicing key in Extension case
2. **Lookup functions** (do_get_hash, do_get, do_get_hexary_proof): Slice key and reset depth to 0 to match insertion
3. **Leaf cases**: Simplified check to `key.is_empty() || key.iter().all(|x| x == 0)`

## Test Results

- **1934 tests pass** (all tests including zk feature)
- Regression test: `test_sequential_row_ids_1_to_10` passes
- Regression test: `test_sequential_row_ids_1_to_100` passes
- Sequential row IDs 1-100 are now fully retrievable

### Additional Fix (2026-03-01)

A second bug was discovered when testing 100 sequential rows: the extension split case
was not properly handling both old and new paths. The old child was being discarded when
splitting an extension node during insertion.

**Fix Applied**: Modified extension split to place both old child (at branch[prefix[0]])
and new leaf (at branch[key[depth]]) in the branch node.

## Impact

After fix:

1. ✅ **Mission 0202-02 works** - All compressed proof tests pass with sequential row IDs
2. ✅ **Batch operations work correctly** - Sequential row IDs 1-100 are retrievable
3. ✅ **Data integrity maintained** - All inserted rows become retrievable
4. ✅ **Proof generation reliable** - Compressed proofs work with sequential rows
5. ✅ **100-row regression test passes** - Edge case for extension splitting fixed

## Related RFCs

- [RFC-0101: Hexary Merkle Proofs](../../rfcs/0101-hexary-merkle-proofs.md) - Trie structure definition
- [RFC-0105: Trie Extension Key Slicing](./0105-trie-extension-key-slicing.md) - Fix specification
- [RFC-0202: Compressed Proofs](../../rfcs/0202-compressed-proofs.md) - Now works correctly

## Related Missions

- [Mission 0105: Trie Extension Key Slicing Fix](../../missions/open/0105-trie-extension-fix.md) - Completed
