# Mission: Rollup Batch Submission

## Status
Completed

## RFC
RFC-0204: L2 Rollup Protocol

## Acceptance Criteria
- [x] Implement `SubmissionContext::submit_batch()`
- [x] Verify sequencer authorization
- [x] Verify batch number sequence
- [x] Verify STARK proof (stub with plugin)
- [x] Update rollup state
- [x] Add gas metering for submission
- [x] Add tests for valid submission
- [x] Add tests for unauthorized sequencer rejection

## Dependencies
- RFC-0203 (Confidential Queries) - Complete
- Mission 0204-01 (Rollup Types) - Complete
- Mission 0204-02 (Batch Execution) - Complete

## Enables
- Mission 0204-04 (Fraud Proofs)

## Implementation Notes

**Files Created:**
- `src/rollup/submission.rs` - Batch submission logic

**Gas Cost:** 100,000 gas per batch

**Types Added:**
- `SubmissionError` - Error enum for submission
- `SubmissionResult_` - Result of batch submission
- `SubmissionContext` - Context for batch submissions

**Methods:**
```rust
impl SubmissionContext {
    pub fn new(sequencer: Address) -> Self
    pub fn authorize_sequencer(&mut self, address: Address)
    pub fn remove_sequencer(&mut self, address: &Address)
    pub fn is_authorized_sequencer(&self, address: &Address) -> bool
    pub fn get_next_batch_number(&self) -> u64
    pub fn get_last_batch_hash(&self) -> [u8; 32]
    pub fn submit_batch(&mut self, batch: RollupBatch) -> Result<SubmissionResult_>
}
```

**Tests (8 new, 27 total):**
- test_submission_context_new
- test_submit_batch_success
- test_submit_batch_unauthorized
- test_submit_batch_invalid_number
- test_submit_batch_wrong_parent
- test_sequencer_authorization
- test_submission_error_display
- test_submission_result

**Features:**
- `zk` - Optional, enables STARK proof verification via plugin

## Claimant
Claude Agent

## Pull Request
TBD

## Commits
- feat: Implement L2 rollup data structures (mission 0204-01)
- feat: Implement rollup batch execution (mission 0204-02)
- feat: Implement rollup batch submission (mission 0204-03)

## Completion Date
2026-03-02
