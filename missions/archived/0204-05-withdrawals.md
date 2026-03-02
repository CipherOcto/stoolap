# Mission: L2 to L1 Withdrawals

## Status
Completed

## RFC
RFC-0204: L2 Rollup Protocol

## Acceptance Criteria
- [x] Implement `WithdrawalContext::initiate_withdrawal()`
- [x] Implement `WithdrawalContext::finalize_withdrawal()`
- [x] Enforce challenge period
- [x] Transfer funds to recipient (stub)
- [x] Add tests for withdrawal initiation
- [x] Add tests for withdrawal finalization
- [x] Add tests for premature finalization rejection

## Dependencies
- Mission 0204-01 (Rollup Types) - Complete
- Mission 0204-03 (Batch Submission) - Complete
- Mission 0204-04 (Fraud Proofs) - Complete

## Enables
- RFC-0204 completion

## Implementation Notes

**Files Created:**
- `src/rollup/withdrawal.rs` - Withdrawal handling

**Challenge Period:** 100 batches

**Gas Costs:**
- Initiate: 20,000 gas
- Finalize: 30,000 gas

**Types Added:**
- `WithdrawalError` - Error enum for withdrawal operations
- `WithdrawalOpResult` - Result of a withdrawal operation
- `WithdrawalContext` - Context for managing withdrawals

**Methods:**
```rust
impl WithdrawalContext {
    pub fn new(sequencer: Address) -> Self
    pub fn record_batch(&mut self, batch_number: u64, timestamp: u64)
    pub fn initiate_withdrawal(&mut self, recipient: Address, amount: u64) -> Result<WithdrawalOpResult>
    pub fn finalize_withdrawal(&mut self, withdrawal_id: u64, current_time: u64) -> Result<WithdrawalOpResult>
    pub fn get_pending_withdrawals(&self, recipient: &Address) -> Vec<&Withdrawal>
    pub fn pending_count(&self) -> usize
}
```

**Tests (9 new, 46 total):**
- test_withdrawal_context_new
- test_initiate_withdrawal
- test_initiate_multiple_withdrawals
- test_finalize_withdrawal_not_found
- test_finalize_withdrawal_too_early
- test_finalize_withdrawal_success
- test_finalize_already_finalized
- test_get_pending_withdrawals
- test_withdrawal_error_display

## Claimant
Claude Agent

## Pull Request
TBD

## Commits
- feat: Implement L2 rollup data structures (mission 0204-01)
- feat: Implement rollup batch execution (mission 0204-02)
- feat: Implement rollup batch submission (mission 0204-03)
- feat: Implement fraud proof system (mission 0204-04)
- feat: Implement L2 to L1 withdrawals (mission 0204-05)

## Completion Date
2026-03-02
