# Mission: Fraud Proof System

## Status
Open

## RFC
RFC-0204: L2 Rollup Protocol

## Acceptance Criteria
- [ ] Implement `FraudProof::verify()` method
- [ ] Implement `ExecutionContext::challenge_batch()`
- [ ] Re-execute transaction to verify fraud
- [ ] Slash sequencer bond on fraud proof
- [ ] Revert batch and all descendants
- [ ] Add tests for valid fraud proof
- [ ] Add tests for invalid fraud proof rejection

## Dependencies
- Mission 0204-01 (Rollup Types)
- Mission 0204-03 (Batch Submission)

## Enables
- Mission 0204-05 (Withdrawals)

## Implementation Notes

**Files to Modify:**
- `src/rollup/fraud.rs` - Fraud proof handling

**Fraud Proof Verification:**
```rust
impl FraudProof {
    pub fn verify(&self) -> bool {
        // Re-execute transaction
        let expected_root = execute_transaction_with_proof(
            self.pre_state_root,
            self.transaction_index,
            &self.proof,
        );

        // Verify claimed root is wrong
        self.claimed_post_root != expected_root
    }
}
```

**Challenge Handler:**
```rust
impl ExecutionContext {
    pub fn challenge_batch(
        &mut self,
        batch_number: u64,
        fraud_proof: FraudProof,
    ) -> Result<ExecutionResult, ExecutionError> {
        // 1. Verify fraud proof
        if !self.verify_fraud_proof(&fraud_proof) {
            return Err(ExecutionError::InvalidFraudProof);
        }

        // 2. Slash sequencer stake
        self.slash_sequencer(fraud_proof.sequencer);

        // 3. Revert batch and all descendants
        self.revert_batches_from(batch_number);

        Ok(ExecutionResult {
            gas_used: 50_000,
            logs: vec!["Batch reverted".to_string()],
        })
    }
}
```

**Slashing:** 1/10 of sequencer bond slashed

## Claimant
Open

## Pull Request
TBD

## Commits
TBD

## Completion Date
TBD
