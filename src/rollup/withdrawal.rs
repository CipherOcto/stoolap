// Copyright 2025 Stoolap Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! L2 to L1 Withdrawals
//!
//! This module provides withdrawal functionality for moving funds from L2 to L1.

use super::types::{Address, RollupState, Withdrawal};

/// Gas cost for initiating a withdrawal
pub const WITHDRAWAL_INITIATE_GAS: u64 = 20_000;

/// Gas cost for finalizing a withdrawal
pub const WITHDRAWAL_FINALIZE_GAS: u64 = 30_000;

/// Error type for withdrawal operations
#[derive(Debug, Clone, PartialEq)]
pub enum WithdrawalError {
    /// Withdrawal not found
    WithdrawalNotFound,
    /// Challenge period not passed
    ChallengePeriodNotPassed,
    /// Withdrawal already finalized
    AlreadyFinalized,
    /// Insufficient balance
    InsufficientBalance,
}

impl std::fmt::Display for WithdrawalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WithdrawalError::WithdrawalNotFound => write!(f, "Withdrawal not found"),
            WithdrawalError::ChallengePeriodNotPassed => write!(f, "Challenge period not passed"),
            WithdrawalError::AlreadyFinalized => write!(f, "Withdrawal already finalized"),
            WithdrawalError::InsufficientBalance => write!(f, "Insufficient balance"),
        }
    }
}

impl std::error::Error for WithdrawalError {}

/// Result type for withdrawal operations
pub type WithdrawalResult<T> = Result<T, WithdrawalError>;

/// Result of a withdrawal operation
#[derive(Debug, Clone)]
pub struct WithdrawalOpResult {
    /// Gas used for the operation
    pub gas_used: u64,
    /// Withdrawal ID (for initiation)
    pub withdrawal_id: Option<u64>,
    /// Log events
    pub logs: Vec<String>,
}

impl WithdrawalOpResult {
    /// Create a new withdrawal result
    pub fn new(gas_used: u64, withdrawal_id: Option<u64>, logs: Vec<String>) -> Self {
        Self {
            gas_used,
            withdrawal_id,
            logs,
        }
    }
}

/// Withdrawal context for handling L2 to L1 withdrawals
///
/// This manages pending withdrawals and handles finalization after the challenge period.
#[derive(Debug, Clone)]
pub struct WithdrawalContext {
    /// Current rollup state
    pub state: RollupState,
    /// Next withdrawal ID
    next_withdrawal_id: u64,
    /// Batch timestamps for challenge period checking
    batch_timestamps: std::collections::HashMap<u64, u64>,
}

impl WithdrawalContext {
    /// Create a new withdrawal context
    pub fn new(sequencer: Address) -> Self {
        Self {
            state: RollupState::new(sequencer),
            next_withdrawal_id: 0,
            batch_timestamps: std::collections::HashMap::new(),
        }
    }

    /// Record a batch submission timestamp
    pub fn record_batch(&mut self, batch_number: u64, timestamp: u64) {
        self.batch_timestamps.insert(batch_number, timestamp);
        self.state.batch_number = batch_number + 1;
    }

    /// Initiate a withdrawal from L2 to L1
    ///
    /// This creates a pending withdrawal that can be finalized after the challenge period.
    pub fn initiate_withdrawal(
        &mut self,
        recipient: Address,
        amount: u64,
    ) -> WithdrawalResult<WithdrawalOpResult> {
        // Calculate the batch number after which this withdrawal can be finalized
        // This is current batch + challenge period
        let available_after_batch = self.state.batch_number + super::types::CHALLENGE_PERIOD;

        // Create the withdrawal
        let withdrawal = Withdrawal::new(
            self.next_withdrawal_id,
            recipient,
            amount,
            available_after_batch,
            0, // Timestamp will be set when batch is recorded
        );

        self.next_withdrawal_id += 1;

        // Add to pending withdrawals
        self.state.pending_withdrawals.push(withdrawal.clone());

        Ok(WithdrawalOpResult::new(
            WITHDRAWAL_INITIATE_GAS,
            Some(withdrawal.withdrawal_id),
            vec![format!(
                "Withdrawal {} initiated, available after batch {}",
                withdrawal.withdrawal_id, available_after_batch
            )],
        ))
    }

    /// Finalize a pending withdrawal
    ///
    /// This can only be called after the challenge period has passed.
    pub fn finalize_withdrawal(
        &mut self,
        withdrawal_id: u64,
        current_time: u64,
    ) -> WithdrawalResult<WithdrawalOpResult> {
        // Find the withdrawal
        let withdrawal_idx = self
            .state
            .pending_withdrawals
            .iter()
            .position(|w| w.withdrawal_id == withdrawal_id)
            .ok_or(WithdrawalError::WithdrawalNotFound)?;

        // Clone needed data before mutable borrow
        let (is_finalized, batch_number, recipient) = {
            let w = &self.state.pending_withdrawals[withdrawal_idx];
            (w.finalized, w.batch_number, w.recipient.clone())
        };

        // Check if already finalized
        if is_finalized {
            return Err(WithdrawalError::AlreadyFinalized);
        }

        // Check if challenge period has passed
        if !self.can_finalize_withdrawal(batch_number, current_time) {
            return Err(WithdrawalError::ChallengePeriodNotPassed);
        }

        // Mark as finalized
        self.state.pending_withdrawals[withdrawal_idx].finalize([0u8; 32]);

        Ok(WithdrawalOpResult::new(
            WITHDRAWAL_FINALIZE_GAS,
            None,
            vec![format!(
                "Withdrawal {} finalized for recipient {:?}",
                withdrawal_id, recipient
            )],
        ))
    }

    /// Check if a withdrawal can be finalized
    fn can_finalize_withdrawal(&self, available_after_batch: u64, _current_time: u64) -> bool {
        // Finalization is allowed after the challenge period batches have passed
        // This means the current batch number must be greater than available_after_batch
        available_after_batch <= self.state.batch_number
    }

    /// Get pending withdrawals for an address
    pub fn get_pending_withdrawals(&self, recipient: &Address) -> Vec<&Withdrawal> {
        self.state
            .pending_withdrawals
            .iter()
            .filter(|w| w.recipient == *recipient && !w.finalized)
            .collect()
    }

    /// Get the number of pending withdrawals
    pub fn pending_count(&self) -> usize {
        self.state
            .pending_withdrawals
            .iter()
            .filter(|w| !w.finalized)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_withdrawal_context_new() {
        let ctx = WithdrawalContext::new(Address::zero());

        assert_eq!(ctx.next_withdrawal_id, 0);
        assert!(ctx.state.pending_withdrawals.is_empty());
    }

    #[test]
    fn test_initiate_withdrawal() {
        let mut ctx = WithdrawalContext::new(Address::zero());

        let result = ctx
            .initiate_withdrawal(Address::new([1u8; 20]), 1000)
            .unwrap();

        assert_eq!(result.gas_used, WITHDRAWAL_INITIATE_GAS);
        assert_eq!(result.withdrawal_id, Some(0));
        assert_eq!(ctx.pending_count(), 1);
    }

    #[test]
    fn test_initiate_multiple_withdrawals() {
        let mut ctx = WithdrawalContext::new(Address::zero());

        ctx.initiate_withdrawal(Address::new([1u8; 20]), 100)
            .unwrap();
        ctx.initiate_withdrawal(Address::new([2u8; 20]), 200)
            .unwrap();
        ctx.initiate_withdrawal(Address::new([3u8; 20]), 300)
            .unwrap();

        assert_eq!(ctx.next_withdrawal_id, 3);
        assert_eq!(ctx.pending_count(), 3);
    }

    #[test]
    fn test_finalize_withdrawal_not_found() {
        let mut ctx = WithdrawalContext::new(Address::zero());

        let result = ctx.finalize_withdrawal(0, 1000);
        assert!(matches!(result, Err(WithdrawalError::WithdrawalNotFound)));
    }

    #[test]
    fn test_finalize_withdrawal_too_early() {
        let mut ctx = WithdrawalContext::new(Address::zero());

        // Record batch 0 at time 100
        ctx.record_batch(0, 100);

        // Initiate withdrawal
        ctx.initiate_withdrawal(Address::new([1u8; 20]), 1000)
            .unwrap();

        // Try to finalize immediately (before challenge period)
        let result = ctx.finalize_withdrawal(0, 150);
        assert!(matches!(
            result,
            Err(WithdrawalError::ChallengePeriodNotPassed)
        ));
    }

    #[test]
    fn test_finalize_withdrawal_success() {
        let mut ctx = WithdrawalContext::new(Address::zero());

        // Record batches to establish state
        for i in 0..10 {
            ctx.record_batch(i, (i * 10) as u64);
        }

        // Initiate withdrawal - available after batch 10 + 100 = 110
        ctx.initiate_withdrawal(Address::new([1u8; 20]), 1000)
            .unwrap();

        // Record more batches to pass challenge period
        // Need batch_number > 110
        for i in 10..120 {
            ctx.record_batch(i, (i * 10) as u64);
        }

        // Now batch_number = 120, can finalize
        let result = ctx.finalize_withdrawal(0, 10000).unwrap();

        assert_eq!(result.gas_used, WITHDRAWAL_FINALIZE_GAS);
    }

    #[test]
    fn test_finalize_already_finalized() {
        let mut ctx = WithdrawalContext::new(Address::zero());

        // Record batches to establish state
        for i in 0..10 {
            ctx.record_batch(i, (i * 10) as u64);
        }

        // Initiate withdrawal - available after batch 10 + 100 = 110
        ctx.initiate_withdrawal(Address::new([1u8; 20]), 1000)
            .unwrap();

        // Record more batches to pass challenge period
        for i in 10..120 {
            ctx.record_batch(i, (i * 10) as u64);
        }

        // Finalize once
        ctx.finalize_withdrawal(0, 10000).unwrap();

        // Try to finalize again - should fail with AlreadyFinalized
        let result = ctx.finalize_withdrawal(0, 10000);
        assert!(matches!(result, Err(WithdrawalError::AlreadyFinalized)));
    }

    #[test]
    fn test_get_pending_withdrawals() {
        let mut ctx = WithdrawalContext::new(Address::zero());

        let addr1 = Address::new([1u8; 20]);
        let addr2 = Address::new([2u8; 20]);

        ctx.initiate_withdrawal(addr1.clone(), 100).unwrap();
        ctx.initiate_withdrawal(addr2.clone(), 200).unwrap();
        ctx.initiate_withdrawal(addr1.clone(), 300).unwrap();

        let pending = ctx.get_pending_withdrawals(&addr1);
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn test_withdrawal_error_display() {
        assert_eq!(
            format!("{}", WithdrawalError::WithdrawalNotFound),
            "Withdrawal not found"
        );
        assert_eq!(
            format!("{}", WithdrawalError::ChallengePeriodNotPassed),
            "Challenge period not passed"
        );
    }
}
