# Mission: Core Cairo Programs

## Status
Open

## RFC
RFC-0201: STWO and Cairo Integration for Zero-Knowledge Proofs

## Acceptance Criteria
- [ ] Write `state_transition.cairo` program
- [ ] Write `hexary_verify.cairo` program
- [ ] Write `merkle_batch.cairo` program
- [ ] Compile all programs to CASM
- [ ] Add programs to bundled resources
- [ ] Add tests verifying program functionality
- [ ] Document program interfaces

## Dependencies
- Mission 0201-03 (Cairo Compiler Integration)
- Mission 0201-05 (Prover Interface)

## Enables
- Mission 0202-02 (Compressed Proof Generation)
- Mission 0203-02 (Confidential Query Execution)

## Implementation Notes

**Files to Create:**
- `cairo/state_transition.cairo` - State transition verification
- `cairo/hexary_verify.cairo` - Hexary proof verification
- `cairo/merkle_batch.cairo` - Batch proof verification
- `src/zk/bundled.rs` - Bundled program registry

**Cairo Programs:**

1. **state_transition.cairo**
   - Input: prev_root, operations[], new_root
   - Output: valid (bool)
   - Functions: hash_operation, apply_operation, verify_root

2. **hexary_verify.cairo**
   - Input: row_id, value, proof, expected_root
   - Output: valid (bool)
   - Functions: verify_hexary_proof, hash_16_children

3. **merkle_batch.cairo**
   - Input: proofs[], expected_root
   - Output: valid (bool), count (u64)
   - Functions: batch_verify, verify_single_proof

**Build Process:**
```rust
// Build script to compile Cairo programs
fn build_cairo_programs() {
    let programs = vec![
        "cairo/state_transition.cairo",
        "cairo/hexary_verify.cairo",
        "cairo/merkle_batch.cairo",
    ];

    for program in programs {
        compile_cairo_program(program);
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
