# Mission: L2 to L1 Withdrawals

## Status
Open

## RFC
RFC-0204: L2 Rollup Protocol

## Acceptance Criteria
- [ ] Implement `ExecutionContext::initiate_withdrawal()`
- [ ] Implement `ExecutionContext::finalize_withdrawal()`
- [ ] Enforce challenge period
- [ ] Transfer funds to recipient
- [ ] Add tests for withdrawal initiation
- [ ] Add tests for withdrawal finalization
- [ ] Add tests for premature finalization rejection

## Dependencies
- Mission 0204-01 (Rollup Types)
- Mission 0204-03 (Batch Submission)

## Enables
- RFC-0204 completion

## Implementation Notes

**Files to Modify:**
- `src/rollup/withdrawal.rs` - Withdrawal handling

**Withdrawal Initiation:**
```rust
impl ExecutionContext {
    pub fn initiate_withdrawal(
        &mut self,
        recipient: Address,
        amount: u64,
    ) -> Result<ExecutionResult, ExecutionError> {
        let withdrawal = Withdrawal {
            recipient,
            amount,
            batch_number: self.rollup_state.batch_number + CHALLENGE_PERIOD,
        };

        self.rollup_state.pending_withdrawals.push(withdrawal);

        Ok(ExecutionResult {
            gas_used: 20_000,
            logs: vec!["Withdrawal initiated".to_string()],
        })
    }
}
```

**Withdrawal Finalization:**
```rust
impl ExecutionContext {
    pub fn finalize_withdrawal(
        &mut self,
        withdrawal_id: u64,
    ) -> Result<ExecutionResult, ExecutionError> {
        let withdrawal = self.rollup_state.pending_withdrawals
            .get(withdrawal_id as usize)
            .ok_or(ExecutionError::InvalidWithdrawal)?;

        // Verify challenge period passed
        if withdrawal.batch_number > self.rollup_state.batch_number {
            return Err(ExecutionError::ChallengePeriodNotPassed);
        }

        // Transfer funds to recipient
        self.transfer_balance(withdrawal.recipient, withdrawal.amount)?;

        // Remove withdrawal
        self.rollup_state.pending_withdrawals.remove(withdrawal_id as usize);

        Ok(ExecutionResult {
            gas_used: 30_000,
            logs: vec!["Withdrawal finalized".to_string()],
        })
    }
}
```

**Challenge Period:** 100 batches

**Gas Costs:**
- Initiate: 20,000 gas
- Finalize: 30,000 gas

## Claimant
Open

## Pull Request
TBD

## Commits
TBD

## Completion Date
TBD
