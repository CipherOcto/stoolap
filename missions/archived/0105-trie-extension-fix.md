# Mission: Trie Extension Key Slicing Fix

## Status
Completed

## RFC
RFC-0105

## Acceptance Criteria

- [x] Modify `do_get_hash()` Extension case to slice key and reset depth to 0
- [x] Modify `do_get()` Extension case to use the same key slicing logic
- [x] Modify `do_get_hexary_proof()` Extension case to use the same key slicing logic
- [x] All existing tests continue to pass (1934 tests)
- [x] Regression test for sequential row IDs 1-10 now passes
- [x] Extended regression test for sequential row IDs 1-100 now passes
- [x] Hexary proof benchmarks exist (benches/hexary_proof.rs)
- [x] Mock STARK prover ready - benchmarks would use mock, not real

## Claimant
Claude Agent

## Pull Request
# (merged)

## Notes

### Root Cause
The insertion code was incorrectly incrementing depth (`depth + prefix.len()`) after slicing the key (`&key[prefix.len()..]`). After slicing, the key indices shift, so using `key[depth]` accessed the wrong position.

### Fix Applied
1. **Insertion** (do_insert_static): Reset depth to 0 after slicing key in Extension case
2. **Lookup** (do_get_hash, do_get, do_get_hexary_proof): Slice key and reset depth to 0 to match insertion
3. **Leaf cases**: Use `key.is_empty() || key.iter().all(|x| x == 0)` check

### Files Modified
- `src/trie/row_trie.rs`:
  - Line ~395: Insertion extension case - changed depth to 0
  - Line ~558: do_get_hash extension case - added key slicing
  - Line ~580: do_get extension case - added key slicing
  - Line ~606: do_get extension case - added key slicing
  - Line ~679: do_get_hexary_proof leaf case - simplified check
  - Line ~730: do_get_hexary_proof extension case - added key slicing

### Test Results
- All 1934 tests pass (including zk feature)
- Regression test: `test_sequential_row_ids_1_to_10` passes
- Regression test: `test_sequential_row_ids_1_to_100` passes

### Additional Fix (2026-03-01)
A second bug was discovered when testing 100 sequential rows: the extension split case
was not properly handling both old and new paths. When inserting a Row that diverges from
an existing extension prefix, the old child was being discarded.

**Fix Applied**: Modified extension split to place both old child (at branch[prefix[0]])
and new leaf (at branch[key[depth]]) in the branch node.

### Context
This fix resolves the sequential row ID lookup bug, enabling the compressed proof generation functionality in mission 0202-02 to work correctly with sequential row IDs 1-10.
