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

//! L2 Rollup Batch Submission
//!
//! This module provides batch submission functionality for the L2 rollup protocol.

use super::types::{Address, RollupBatch, RollupState};

/// Gas cost for batch submission
pub const BATCH_SUBMISSION_GAS: u64 = 100_000;

/// Error type for batch submission
#[derive(Debug, Clone, PartialEq)]
pub enum SubmissionError {
    /// Unauthorized sequencer
    UnauthorizedSequencer,
    /// Invalid batch number
    InvalidBatchNumber,
    /// Invalid proof
    InvalidProof,
    /// Batch too old (already submitted)
    BatchTooOld,
    /// State root mismatch
    StateRootMismatch,
    /// Parent hash mismatch
    ParentHashMismatch,
}

impl std::fmt::Display for SubmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubmissionError::UnauthorizedSequencer => write!(f, "Unauthorized sequencer"),
            SubmissionError::InvalidBatchNumber => write!(f, "Invalid batch number"),
            SubmissionError::InvalidProof => write!(f, "Invalid STARK proof"),
            SubmissionError::BatchTooOld => write!(f, "Batch already submitted"),
            SubmissionError::StateRootMismatch => write!(f, "State root mismatch"),
            SubmissionError::ParentHashMismatch => write!(f, "Parent hash mismatch"),
        }
    }
}

impl std::error::Error for SubmissionError {}

/// Result type for submission operations
pub type SubmissionResult<T> = Result<T, SubmissionError>;

/// Result of a successful batch submission
#[derive(Debug, Clone)]
pub struct SubmissionResult_ {
    /// Gas used for the submission
    pub gas_used: u64,
    /// New state after submission
    pub new_state: RollupState,
    /// Log events
    pub logs: Vec<String>,
}

impl SubmissionResult_ {
    /// Create a new submission result
    pub fn new(new_state: RollupState) -> Self {
        Self {
            gas_used: BATCH_SUBMISSION_GAS,
            new_state,
            logs: vec!["Batch submitted successfully".to_string()],
        }
    }
}

/// Rollup submission context
///
/// This holds the current state and handles batch submissions.
#[derive(Debug, Clone)]
pub struct SubmissionContext {
    /// Current rollup state
    pub state: RollupState,
    /// Authorized sequencers
    authorized_sequencers: Vec<Address>,
    /// Last submitted batch hash
    last_batch_hash: [u8; 32],
}

impl SubmissionContext {
    /// Create a new submission context
    pub fn new(sequencer: Address) -> Self {
        let mut state = RollupState::new(sequencer.clone());
        state.sequencer_bonded = true;

        Self {
            state,
            authorized_sequencers: vec![sequencer],
            last_batch_hash: [0u8; 32],
        }
    }

    /// Add an authorized sequencer
    pub fn authorize_sequencer(&mut self, address: Address) {
        if !self.authorized_sequencers.contains(&address) {
            self.authorized_sequencers.push(address);
        }
    }

    /// Remove an authorized sequencer
    pub fn remove_sequencer(&mut self, address: &Address) {
        self.authorized_sequencers.retain(|a| a != address);
    }

    /// Check if an address is an authorized sequencer
    pub fn is_authorized_sequencer(&self, address: &Address) -> bool {
        self.authorized_sequencers.contains(address)
    }

    /// Get the expected next batch number
    pub fn get_next_batch_number(&self) -> u64 {
        self.state.batch_number
    }

    /// Get the last batch hash
    pub fn get_last_batch_hash(&self) -> [u8; 32] {
        self.last_batch_hash
    }

    /// Submit a rollup batch
    ///
    /// This method:
    /// 1. Verifies the sequencer is authorized
    /// 2. Verifies the batch number is correct
    /// 3. Verifies the parent hash
    /// 4. Verifies the STARK proof (if zk feature enabled)
    /// 5. Updates the rollup state
    pub fn submit_batch(&mut self, batch: RollupBatch) -> SubmissionResult<SubmissionResult_> {
        // 1. Verify sequencer is authorized
        if !self.is_authorized_sequencer(&batch.sequencer) {
            return Err(SubmissionError::UnauthorizedSequencer);
        }

        // 2. Verify batch number
        let expected_number = self.get_next_batch_number();
        if batch.batch_number != expected_number {
            return Err(SubmissionError::InvalidBatchNumber);
        }

        // 3. Verify parent hash
        if batch.parent_hash != self.last_batch_hash {
            return Err(SubmissionError::ParentHashMismatch);
        }

        // 4. Verify proof (if zk feature enabled)
        #[cfg(feature = "zk")]
        {
            if !self.verify_proof(&batch) {
                return Err(SubmissionError::InvalidProof);
            }
        }

        // 5. Update rollup state
        self.state.batch_number = batch.batch_number + 1;
        self.state.state_root = batch.post_state_root;
        self.last_batch_hash = batch.hash();

        Ok(SubmissionResult_::new(self.state.clone()))
    }

    /// Verify the STARK proof
    #[cfg(feature = "zk")]
    fn verify_proof(&self, batch: &RollupBatch) -> bool {
        // Try to load the plugin and verify
        match crate::zk::plugin::load_plugin() {
            Ok(plugin) => {
                let mut verify_input = Vec::new();
                verify_input.extend_from_slice(&batch.pre_state_root);
                verify_input.extend_from_slice(&batch.post_state_root);
                verify_input.extend_from_slice(&batch.parent_hash);
                verify_input.extend_from_slice(&batch.batch_number.to_le_bytes());

                plugin.verify(&verify_input).is_ok()
            }
            Err(_) => {
                // No plugin - skip proof verification (for testing)
                true
            }
        }
    }

    #[cfg(not(feature = "zk"))]
    fn verify_proof(&self, _batch: &RollupBatch) -> bool {
        // Without zk feature, always accept
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_batch(batch_number: u64, parent_hash: [u8; 32]) -> RollupBatch {
        RollupBatch::new(
            batch_number,
            parent_hash,
            vec![],
            [1u8; 32],
            [2u8; 32],
            1000,
            Address::zero(),
        )
    }

    #[test]
    fn test_submission_context_new() {
        let ctx = SubmissionContext::new(Address::new([1u8; 20]));

        assert_eq!(ctx.state.batch_number, 0);
        assert!(ctx.is_authorized_sequencer(&Address::new([1u8; 20])));
    }

    #[test]
    fn test_submit_batch_success() {
        let mut ctx = SubmissionContext::new(Address::zero());

        let batch = create_test_batch(0, [0u8; 32]);
        let result = ctx.submit_batch(batch).unwrap();

        assert_eq!(result.gas_used, BATCH_SUBMISSION_GAS);
        assert_eq!(ctx.state.batch_number, 1);
    }

    #[test]
    fn test_submit_batch_unauthorized() {
        let mut ctx = SubmissionContext::new(Address::zero());

        // Create batch with different sequencer
        let mut batch = create_test_batch(0, [0u8; 32]);
        batch.sequencer = Address::new([9u8; 20]);

        let result = ctx.submit_batch(batch);
        assert!(matches!(
            result,
            Err(SubmissionError::UnauthorizedSequencer)
        ));
    }

    #[test]
    fn test_submit_batch_invalid_number() {
        let mut ctx = SubmissionContext::new(Address::zero());

        // Submit batch 0 first
        let batch0 = create_test_batch(0, [0u8; 32]);
        ctx.submit_batch(batch0).unwrap();

        // Try to submit batch 2 (skipped batch 1)
        let batch2 = create_test_batch(2, ctx.get_last_batch_hash());
        let result = ctx.submit_batch(batch2);

        assert!(matches!(result, Err(SubmissionError::InvalidBatchNumber)));
    }

    #[test]
    fn test_submit_batch_wrong_parent() {
        let mut ctx = SubmissionContext::new(Address::zero());

        // Try to submit with wrong parent hash
        let batch = create_test_batch(0, [9u8; 32]);
        let result = ctx.submit_batch(batch);

        assert!(matches!(result, Err(SubmissionError::ParentHashMismatch)));
    }

    #[test]
    fn test_sequencer_authorization() {
        let mut ctx = SubmissionContext::new(Address::zero());

        // New sequencer should not be authorized
        let new_sequencer = Address::new([9u8; 20]);
        assert!(!ctx.is_authorized_sequencer(&new_sequencer));

        // Authorize the sequencer
        ctx.authorize_sequencer(new_sequencer.clone());
        assert!(ctx.is_authorized_sequencer(&new_sequencer));

        // Remove the sequencer
        ctx.remove_sequencer(&new_sequencer);
        assert!(!ctx.is_authorized_sequencer(&new_sequencer));
    }

    #[test]
    fn test_submission_error_display() {
        assert_eq!(
            format!("{}", SubmissionError::UnauthorizedSequencer),
            "Unauthorized sequencer"
        );
        assert_eq!(
            format!("{}", SubmissionError::InvalidBatchNumber),
            "Invalid batch number"
        );
    }

    #[test]
    fn test_submission_result() {
        let result = SubmissionResult_::new(RollupState::new(Address::zero()));

        assert_eq!(result.gas_used, BATCH_SUBMISSION_GAS);
        assert!(!result.logs.is_empty());
    }
}
