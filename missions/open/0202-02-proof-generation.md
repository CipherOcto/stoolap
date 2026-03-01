# Mission: Compressed Proof Generation

## Status
Open

## RFC
RFC-0202: Compressed Proof Format for Batch Verification

## Acceptance Criteria
- [ ] Implement `RowTrie::get_compressed_proof()`
- [ ] Collect individual HexaryProofs for batch
- [ ] Serialize batch input for Cairo
- [ ] Generate STARK proof via STWOProver
- [ ] Add error handling for generation failures
- [ ] Add tests for various batch sizes (10, 100, 1000)
- [ ] Benchmark proof generation time

## Dependencies
- RFC-0201 (STWO Integration) - Complete
- Mission 0201-06 (Core Cairo Programs)
- Mission 0202-01 (Compressed Proof Types)

## Enables
- Mission 0202-03 (Proof Verification)

## Implementation Notes

**Files to Modify:**
- `src/trie/row_trie.rs` - Add compressed proof method

**Implementation:**
```rust
impl RowTrie {
    pub fn get_compressed_proof(
        &self,
        row_ids: &[i64],
    ) -> Option<CompressedProof> {
        // 1. Collect individual proofs
        let mut proofs = Vec::new();
        let mut values = Vec::new();

        for &row_id in row_ids {
            let proof = self.get_hexary_proof(row_id)?;
            let value = self.get_row(row_id)?;
            proofs.push(proof);
            values.push(value);
        }

        // 2. Get batch program
        let program = self.get_batch_program()?;

        // 3. Create batch input
        let input = BatchVerifyInput {
            row_ids: row_ids.to_vec(),
            values,
            proofs,
            expected_root: self.get_root(),
        };

        // 4. Serialize and generate proof
        let cairo_input = serialize_batch_input(&input);
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

**Tests:**
- Small batch (10 rows)
- Medium batch (100 rows)
- Large batch (1000 rows)
- Empty batch (error case)

## Claimant
Open

## Pull Request
TBD

## Commits
TBD

## Completion Date
TBD
