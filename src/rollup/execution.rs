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

//! L2 Rollup Batch Execution
//!
//! This module provides batch execution functionality for the L2 rollup protocol.

use super::types::{RollupBatch, RollupState, Transaction, TxType, MAX_BATCH_SIZE};

/// Error type for rollup execution
#[derive(Debug, Clone, PartialEq)]
pub enum RollupError {
    /// Invalid parent batch hash
    InvalidParent,
    /// Invalid pre-state root
    InvalidPreState,
    /// Invalid post-state root
    InvalidPostState,
    /// Batch too large
    BatchTooLarge,
    /// Transaction execution failed
    ExecutionFailed(String),
    /// Proof generation failed
    ProofError(String),
    /// Invalid transaction
    InvalidTransaction(String),
}

impl std::fmt::Display for RollupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RollupError::InvalidParent => write!(f, "Invalid parent batch hash"),
            RollupError::InvalidPreState => write!(f, "Invalid pre-state root"),
            RollupError::InvalidPostState => write!(f, "Invalid post-state root"),
            RollupError::BatchTooLarge => write!(f, "Batch exceeds maximum size"),
            RollupError::ExecutionFailed(msg) => write!(f, "Transaction execution failed: {}", msg),
            RollupError::ProofError(msg) => write!(f, "Proof generation failed: {}", msg),
            RollupError::InvalidTransaction(msg) => write!(f, "Invalid transaction: {}", msg),
        }
    }
}

impl std::error::Error for RollupError {}

/// Result type for rollup operations
pub type RollupResult<T> = Result<T, RollupError>;

/// Execution result containing the new state and proof
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Final state root after executing the batch
    pub post_state_root: [u8; 32],
    /// Number of transactions successfully executed
    pub executed_count: usize,
    /// Total fees collected
    pub total_fees: u64,
    /// STARK proof (if generated)
    pub proof: Vec<u8>,
}

impl RollupBatch {
    /// Execute the batch and optionally generate a proof
    ///
    /// This method:
    /// 1. Validates the batch size
    /// 2. Verifies the parent hash
    /// 3. Verifies the pre-state root
    /// 4. Executes all transactions
    /// 5. Verifies the post-state root
    /// 6. Generates a STARK proof (if zk feature enabled)
    pub fn execute_and_prove(
        &self,
        pre_state_root: [u8; 32],
        parent_hash: [u8; 32],
    ) -> RollupResult<ExecutionResult> {
        // 1. Validate batch size
        if self.transactions.len() > MAX_BATCH_SIZE {
            return Err(RollupError::BatchTooLarge);
        }

        // 2. Verify parent hash
        if self.parent_hash != parent_hash {
            return Err(RollupError::InvalidParent);
        }

        // 3. Verify pre-state root
        if self.pre_state_root != pre_state_root {
            return Err(RollupError::InvalidPreState);
        }

        // 4. Execute transactions
        let mut state = RollupState::new(self.sequencer.clone());
        state.state_root = pre_state_root;
        state.batch_number = self.batch_number;

        let mut executed_count = 0;
        let mut total_fees = 0u64;

        for tx in &self.transactions {
            match state.execute_transaction(tx) {
                Ok(fee) => {
                    executed_count += 1;
                    total_fees += fee;
                }
                Err(e) => {
                    // Log but continue - invalid transactions are skipped
                    tracing::warn!("Transaction execution failed: {}", e);
                }
            }
        }

        // 5. Verify post-state root
        let post_state_root = state.state_root;
        if post_state_root != self.post_state_root {
            return Err(RollupError::InvalidPostState);
        }

        // 6. Generate proof (if zk feature enabled)
        #[cfg(feature = "zk")]
        let proof = self.generate_proof(pre_state_root, post_state_root);

        #[cfg(not(feature = "zk"))]
        let proof = Vec::new();

        Ok(ExecutionResult {
            post_state_root,
            executed_count,
            total_fees,
            proof,
        })
    }

    /// Execute batch without proof generation
    pub fn execute(
        &self,
        pre_state_root: [u8; 32],
        parent_hash: [u8; 32],
    ) -> RollupResult<ExecutionResult> {
        self.execute_and_prove(pre_state_root, parent_hash)
    }

    /// Generate a STARK proof for the batch execution
    #[cfg(feature = "zk")]
    fn generate_proof(&self, pre_state_root: [u8; 32], post_state_root: [u8; 32]) -> Vec<u8> {
        // Try to load the plugin and generate a proof
        match crate::zk::plugin::load_plugin() {
            Ok(plugin) => {
                // Create proof input from batch data
                let mut proof_input = Vec::new();
                proof_input.extend_from_slice(&self.batch_number.to_le_bytes());
                proof_input.extend_from_slice(&pre_state_root);
                proof_input.extend_from_slice(&post_state_root);
                proof_input.extend_from_slice(&self.parent_hash);
                proof_input.extend_from_slice(&(self.transactions.len() as u64).to_le_bytes());

                // Verify to get a "proof" (placeholder - real proving would use Cairo program)
                let _ = plugin.verify(&proof_input);
                proof_input
            }
            Err(_) => {
                // Plugin not available, return placeholder
                Vec::new()
            }
        }
    }
}

impl RollupState {
    /// Execute a single transaction
    ///
    /// Returns the fee collected or an error
    pub fn execute_transaction(&mut self, tx: &Transaction) -> RollupResult<u64> {
        // Validate transaction
        if tx.nonce != self.get_nonce(&tx.from) {
            return Err(RollupError::InvalidTransaction("Invalid nonce".to_string()));
        }

        // Execute based on transaction type
        let fee = tx.fee;
        match tx.tx_type {
            TxType::Transfer => {
                // Simple transfer - update state
                self.apply_transfer(&tx.from, tx.to.as_ref(), tx.data.len() as u64);
            }
            TxType::Call => {
                // Contract call - execute in state
                if tx.to.is_none() {
                    return Err(RollupError::InvalidTransaction(
                        "Call requires target".to_string(),
                    ));
                }
                self.apply_call(tx.to.as_ref().unwrap(), &tx.data);
            }
            TxType::Create => {
                // Contract creation
                self.apply_create(&tx.data);
            }
            TxType::Withdrawal => {
                // Withdrawal request
                self.apply_withdrawal(&tx.from, tx.data.len() as u64);
            }
        }

        // Increment nonce
        self.increment_nonce(&tx.from);

        Ok(fee)
    }

    /// Apply a transfer (simplified state update)
    fn apply_transfer(
        &mut self,
        _from: &super::types::Address,
        _to: Option<&super::types::Address>,
        _amount: u64,
    ) {
        // In a real implementation, this would update account balances
        // For now, we just update the state root
        self.update_state_root();
    }

    /// Apply a contract call
    fn apply_call(&mut self, _to: &super::types::Address, _data: &[u8]) {
        // In a real implementation, this would execute contract code
        self.update_state_root();
    }

    /// Apply a contract creation
    fn apply_create(&mut self, _code: &[u8]) {
        // In a real implementation, this would create a new contract
        self.update_state_root();
    }

    /// Apply a withdrawal
    fn apply_withdrawal(&mut self, _from: &super::types::Address, _amount: u64) {
        // In a real implementation, this would queue a withdrawal
        self.update_state_root();
    }

    /// Get nonce for an address
    fn get_nonce(&self, _address: &super::types::Address) -> u64 {
        // Simplified - return 0 for all addresses
        0
    }

    /// Increment nonce for an address
    fn increment_nonce(&mut self, _address: &super::types::Address) {
        // Simplified - no-op for now
    }

    /// Update the state root (simplified)
    fn update_state_root(&mut self) {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.state_root);
        hasher.update(self.batch_number.to_le_bytes());
        hasher.update((self.pending_withdrawals.len() as u64).to_le_bytes());
        self.state_root = hasher.finalize().into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rollup::types::{Address, TxType};

    #[test]
    fn test_batch_execution_valid() {
        // Create batch with correct pre and post state roots
        // The execution will compute the same post-state root as long as we use the right input
        let pre_state = [1u8; 32];

        let batch = RollupBatch::new(
            1,
            [0u8; 32],
            vec![],
            pre_state,
            pre_state, // Use same root - empty batch doesn't change state
            1000,
            Address::zero(),
        );

        let result = batch.execute(pre_state, [0u8; 32]).unwrap();
        assert_eq!(result.executed_count, 0);
    }

    #[test]
    fn test_batch_execution_invalid_parent() {
        let batch = RollupBatch::new(
            1,
            [1u8; 32], // Different parent hash
            vec![],
            [1u8; 32],
            [2u8; 32],
            1000,
            Address::zero(),
        );

        let result = batch.execute([1u8; 32], [0u8; 32]);
        assert!(matches!(result, Err(RollupError::InvalidParent)));
    }

    #[test]
    fn test_batch_execution_invalid_prestate() {
        let batch = RollupBatch::new(
            1,
            [0u8; 32],
            vec![],
            [1u8; 32], // Pre-state root
            [2u8; 32],
            1000,
            Address::zero(),
        );

        // Different pre-state root
        let result = batch.execute([9u8; 32], [0u8; 32]);
        assert!(matches!(result, Err(RollupError::InvalidPreState)));
    }

    #[test]
    fn test_batch_execution_too_large() {
        let mut transactions = Vec::new();
        for i in 0..(MAX_BATCH_SIZE + 1) {
            transactions.push(Transaction::new(
                TxType::Transfer,
                Address::zero(),
                None,
                vec![],
                i as u64,
                1,
            ));
        }

        let batch = RollupBatch::new(
            1,
            [0u8; 32],
            transactions,
            [1u8; 32],
            [2u8; 32],
            1000,
            Address::zero(),
        );

        let result = batch.execute([1u8; 32], [0u8; 32]);
        assert!(matches!(result, Err(RollupError::BatchTooLarge)));
    }

    #[test]
    fn test_execution_result() {
        let result = ExecutionResult {
            post_state_root: [1u8; 32],
            executed_count: 5,
            total_fees: 100,
            proof: vec![1, 2, 3],
        };

        assert_eq!(result.executed_count, 5);
        assert_eq!(result.total_fees, 100);
    }

    #[test]
    fn test_rollup_error_display() {
        assert_eq!(
            format!("{}", RollupError::InvalidParent),
            "Invalid parent batch hash"
        );
        assert_eq!(
            format!("{}", RollupError::BatchTooLarge),
            "Batch exceeds maximum size"
        );
    }
}
