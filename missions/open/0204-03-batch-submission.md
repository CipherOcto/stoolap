# Mission: Rollup Batch Submission

## Status
Open

## RFC
RFC-0204: L2 Rollup Protocol

## Acceptance Criteria
- [ ] Implement `ExecutionContext::submit_rollup_batch()`
- [ ] Verify sequencer authorization
- [ ] Verify batch number sequence
- [ ] Verify STARK proof
- [ ] Update rollup state
- [ ] Add gas metering for submission
- [ ] Add tests for valid submission
- [ ] Add tests for unauthorized sequencer rejection

## Dependencies
- RFC-0203 (Confidential Queries) - Complete
- Mission 0201-05 (Prover Interface)
- Mission 0204-01 (Rollup Types)
- Mission 0204-02 (Batch Execution)

## Enables
- Mission 0204-04 (Fraud Proofs)

## Implementation Notes

**Files to Modify:**
- `src/consensus/execution.rs` - Add rollup operations

**Implementation:**
```rust
impl ExecutionContext {
    pub fn submit_rollup_batch(
        &mut self,
        batch: RollupBatch,
        proof: StarkProof,
    ) -> Result<ExecutionResult, ExecutionError> {
        // 1. Verify sequencer is authorized
        if !self.is_authorized_sequencer(batch.sequencer) {
            return Err(ExecutionError::UnauthorizedSequencer);
        }

        // 2. Verify batch number
        let expected_number = self.get_next_batch_number();
        if batch.batch_number != expected_number {
            return Err(ExecutionError::InvalidBatchNumber);
        }

        // 3. Verify proof
        let program = self.get_rollup_program()?;
        let verifier = STWOVerifier::new();
        if !verifier.verify(&proof)? {
            return Err(ExecutionError::InvalidProof);
        }

        // 4. Update rollup state
        self.rollup_state = RollupState {
            batch_number: batch.batch_number,
            state_root: batch.post_state_root,
            pending_withdrawals: Vec::new(),
            sequencer: batch.sequencer,
        };

        Ok(ExecutionResult {
            gas_used: 100_000,
            logs: vec!["Batch submitted".to_string()],
        })
    }
}
```

**Gas Cost:** 100,000 gas per batch

## Claimant
Open

## Pull Request
TBD

## Commits
TBD

## Completion Date
TBD
