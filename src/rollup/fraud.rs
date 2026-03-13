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

//! L2 Rollup Fraud Proof System
//!
//! This module provides fraud proof functionality for challenging invalid batches.

use super::types::{Address, FraudProof, RollupState};

/// Gas cost for processing a fraud proof
pub const FRAUD_PROOF_GAS: u64 = 50_000;

/// Slash percentage (1/10 = 10%)
pub const SLASH_DIVISOR: u64 = 10;

/// Error type for fraud proof operations
#[derive(Debug, Clone, PartialEq)]
pub enum FraudError {
    /// Invalid fraud proof
    InvalidFraudProof,
    /// Batch not yet challengeable
    BatchNotChallengeable,
    /// Batch already finalized
    BatchFinalized,
    /// No sequencer to slash
    NoSequencerToSlash,
}

impl std::fmt::Display for FraudError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FraudError::InvalidFraudProof => write!(f, "Invalid fraud proof"),
            FraudError::BatchNotChallengeable => write!(f, "Batch not yet challengeable"),
            FraudError::BatchFinalized => write!(f, "Batch already finalized"),
            FraudError::NoSequencerToSlash => write!(f, "No sequencer to slash"),
        }
    }
}

impl std::error::Error for FraudError {}

/// Result type for fraud proof operations
pub type FraudResult<T> = Result<T, FraudError>;

/// Result of a successful fraud proof challenge
#[derive(Debug, Clone)]
pub struct ChallengeResult {
    /// Gas used for the challenge
    pub gas_used: u64,
    /// Amount slashed from sequencer
    pub slashed_amount: u64,
    /// Log events
    pub logs: Vec<String>,
}

impl ChallengeResult {
    /// Create a new challenge result
    pub fn new(slashed_amount: u64) -> Self {
        Self {
            gas_used: FRAUD_PROOF_GAS,
            slashed_amount,
            logs: vec!["Batch reverted and sequencer slashed".to_string()],
        }
    }
}

/// Challenge context for handling fraud proofs
///
/// This manages the challenge period and handles fraud proof submissions.
#[derive(Debug, Clone)]
pub struct ChallengeContext {
    /// Current rollup state
    pub state: RollupState,
    /// Sequencer bond amount
    sequencer_bond: u64,
    /// Challenge period (in batches)
    challenge_period: u64,
    /// Batch submission timestamps (batch_number -> timestamp)
    batch_timestamps: std::collections::HashMap<u64, u64>,
    /// Whether a batch has been challenged
    challenged_batches: std::collections::HashSet<u64>,
}

impl ChallengeContext {
    /// Create a new challenge context
    pub fn new(sequencer: Address, sequencer_bond: u64) -> Self {
        let mut state = RollupState::new(sequencer);
        state.sequencer_bonded = true;

        Self {
            state,
            sequencer_bond,
            challenge_period: super::types::CHALLENGE_PERIOD,
            batch_timestamps: std::collections::HashMap::new(),
            challenged_batches: std::collections::HashSet::new(),
        }
    }

    /// Record a batch submission timestamp
    pub fn record_batch(&mut self, batch_number: u64, timestamp: u64) {
        self.batch_timestamps.insert(batch_number, timestamp);
    }

    /// Check if a batch is within the challenge period
    pub fn is_challengeable(&self, batch_number: u64, current_time: u64) -> bool {
        // Check if already challenged
        if self.challenged_batches.contains(&batch_number) {
            return false;
        }

        // Check if past challenge period
        if let Some(&timestamp) = self.batch_timestamps.get(&batch_number) {
            let elapsed = current_time.saturating_sub(timestamp);
            return elapsed >= self.challenge_period;
        }

        false
    }

    /// Submit a fraud proof to challenge a batch
    pub fn challenge_batch(
        &mut self,
        batch_number: u64,
        fraud_proof: FraudProof,
        current_time: u64,
    ) -> FraudResult<ChallengeResult> {
        // 1. Check if batch is challengeable
        if !self.is_challengeable(batch_number, current_time) {
            // Check if already challenged
            if self.challenged_batches.contains(&batch_number) {
                return Err(FraudError::BatchFinalized);
            }
            return Err(FraudError::BatchNotChallengeable);
        }

        // 2. Verify fraud proof
        if !self.verify_fraud_proof(&fraud_proof) {
            return Err(FraudError::InvalidFraudProof);
        }

        // 3. Mark batch as challenged
        self.challenged_batches.insert(batch_number);

        // 4. Slash sequencer
        let slashed_amount = self.slash_sequencer();

        // 5. Revert batch (simplified - just update state)
        self.revert_batch(batch_number);

        Ok(ChallengeResult::new(slashed_amount))
    }

    /// Verify a fraud proof
    ///
    /// This re-executes the transaction and compares the result.
    fn verify_fraud_proof(&self, fraud_proof: &FraudProof) -> bool {
        // The fraud proof is valid if the claimed post-root differs from expected
        // In a real implementation, this would re-execute the transaction
        fraud_proof.verify()
    }

    /// Slash the sequencer's bond
    fn slash_sequencer(&mut self) -> u64 {
        let slash_amount = self.sequencer_bond / SLASH_DIVISOR;
        self.sequencer_bond = self.sequencer_bond.saturating_sub(slash_amount);

        // Unbond the sequencer
        self.state.sequencer_bonded = false;

        slash_amount
    }

    /// Revert a batch (simplified)
    fn revert_batch(&mut self, _batch_number: u64) {
        // In a real implementation, this would:
        // 1. Revert state to pre-state of the batch
        // 2. Remove all batches after this one
        // For now, we just log the revert
        self.state.batch_number = self.state.batch_number.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rollup::types::TxType;

    fn create_test_fraud_proof() -> FraudProof {
        FraudProof::new(
            0, 0, [1u8; 32], [2u8; 32], // Expected
            [3u8; 32], // Claimed (different)
        )
    }

    #[test]
    fn test_challenge_context_new() {
        let ctx = ChallengeContext::new(Address::zero(), 1000);

        assert_eq!(ctx.state.batch_number, 0);
        assert_eq!(ctx.sequencer_bond, 1000);
        assert!(ctx.state.sequencer_bonded);
    }

    #[test]
    fn test_record_batch() {
        let mut ctx = ChallengeContext::new(Address::zero(), 1000);

        ctx.record_batch(0, 100);
        assert!(ctx.batch_timestamps.contains_key(&0));
    }

    #[test]
    fn test_is_challengeable_not_yet() {
        let mut ctx = ChallengeContext::new(Address::zero(), 1000);

        ctx.record_batch(0, 100);
        // Current time is 50, challenge period is 100 - not yet challengeable
        assert!(!ctx.is_challengeable(0, 150));
    }

    #[test]
    fn test_is_challengeable_after_period() {
        let mut ctx = ChallengeContext::new(Address::zero(), 1000);

        ctx.record_batch(0, 100);
        // Current time is 200, challenge period is 100 - now challengeable
        assert!(ctx.is_challengeable(0, 200));
    }

    #[test]
    fn test_is_challengeable_already_challenged() {
        let mut ctx = ChallengeContext::new(Address::zero(), 1000);

        ctx.record_batch(0, 100);
        ctx.challenged_batches.insert(0);

        assert!(!ctx.is_challengeable(0, 200));
    }

    #[test]
    fn test_challenge_batch_success() {
        let mut ctx = ChallengeContext::new(Address::zero(), 1000);

        // Set up state with batch
        ctx.state.batch_number = 1;
        ctx.record_batch(0, 100);

        let fraud_proof = create_test_fraud_proof();
        let result = ctx.challenge_batch(0, fraud_proof, 200).unwrap();

        assert_eq!(result.gas_used, FRAUD_PROOF_GAS);
        assert_eq!(result.slashed_amount, 100); // 1000 / 10
        assert!(ctx.challenged_batches.contains(&0));
    }

    #[test]
    fn test_challenge_batch_not_challengeable() {
        let mut ctx = ChallengeContext::new(Address::zero(), 1000);

        ctx.record_batch(0, 100);

        let fraud_proof = create_test_fraud_proof();
        let result = ctx.challenge_batch(0, fraud_proof, 150);

        assert!(matches!(result, Err(FraudError::BatchNotChallengeable)));
    }

    #[test]
    fn test_challenge_batch_already_challenged() {
        let mut ctx = ChallengeContext::new(Address::zero(), 1000);

        ctx.state.batch_number = 1;
        ctx.record_batch(0, 100);
        ctx.challenged_batches.insert(0);

        let fraud_proof = create_test_fraud_proof();
        let result = ctx.challenge_batch(0, fraud_proof, 200);

        assert!(matches!(result, Err(FraudError::BatchFinalized)));
    }

    #[test]
    fn test_fraud_error_display() {
        assert_eq!(
            format!("{}", FraudError::InvalidFraudProof),
            "Invalid fraud proof"
        );
        assert_eq!(
            format!("{}", FraudError::BatchNotChallengeable),
            "Batch not yet challengeable"
        );
    }

    #[test]
    fn test_challenge_result() {
        let result = ChallengeResult::new(100);

        assert_eq!(result.gas_used, FRAUD_PROOF_GAS);
        assert_eq!(result.slashed_amount, 100);
        assert!(!result.logs.is_empty());
    }
}
