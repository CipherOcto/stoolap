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

//! L2 Rollup Data Types
//!
//! Core data structures for the L2 rollup protocol.

use serde::{Deserialize, Serialize};

/// Address size in bytes (Ethereum-style address)
pub const ADDRESS_SIZE: usize = 20;

/// Rollup protocol constants
pub const CHALLENGE_PERIOD: u64 = 100; // Blocks before batch can be finalized
pub const MAX_BATCH_SIZE: usize = 10000; // Maximum transactions per batch
pub const BATCH_INTERVAL: u64 = 10; // Blocks between batch submissions
pub const SEQUENCER_BOND: u64 = 100_000; // Minimum bond amount for sequencer

/// Ethereum-style address
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address(pub [u8; ADDRESS_SIZE]);

impl Address {
    /// Create a new address from bytes
    pub fn new(bytes: [u8; ADDRESS_SIZE]) -> Self {
        Self(bytes)
    }

    /// Create a zero address
    pub fn zero() -> Self {
        Self([0u8; ADDRESS_SIZE])
    }

    /// Check if address is zero
    pub fn is_zero(&self) -> bool {
        self.0 == [0u8; ADDRESS_SIZE]
    }

    /// Get address as bytes
    pub fn as_bytes(&self) -> &[u8; ADDRESS_SIZE] {
        &self.0
    }
}

impl Default for Address {
    fn default() -> Self {
        Self::zero()
    }
}

/// A batch of transactions to be submitted to L1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollupBatch {
    /// Unique batch number
    pub batch_number: u64,
    /// Hash of the parent batch
    pub parent_hash: [u8; 32],
    /// Transactions in this batch
    pub transactions: Vec<Transaction>,
    /// State root before this batch
    pub pre_state_root: [u8; 32],
    /// State root after this batch
    pub post_state_root: [u8; 32],
    /// Timestamp when batch was created
    pub timestamp: u64,
    /// Sequencer that created this batch
    pub sequencer: Address,
    /// Aggregated signature (if using multi-party)
    pub signature: Option<Vec<u8>>,
}

impl RollupBatch {
    /// Create a new rollup batch
    pub fn new(
        batch_number: u64,
        parent_hash: [u8; 32],
        transactions: Vec<Transaction>,
        pre_state_root: [u8; 32],
        post_state_root: [u8; 32],
        timestamp: u64,
        sequencer: Address,
    ) -> Self {
        Self {
            batch_number,
            parent_hash,
            transactions,
            pre_state_root,
            post_state_root,
            timestamp,
            sequencer,
            signature: None,
        }
    }

    /// Get the number of transactions in the batch
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }

    /// Compute batch hash using SHA256
    pub fn hash(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.batch_number.to_le_bytes());
        hasher.update(self.parent_hash);
        hasher.update(self.pre_state_root);
        hasher.update(self.post_state_root);
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(self.sequencer.as_bytes());
        hasher.update((self.transactions.len() as u64).to_le_bytes());
        hasher.finalize().into()
    }

    /// Get serialized size
    pub fn serialized_size(&self) -> usize {
        8 // batch_number
        + 32 // parent_hash
        + 4 + self.transactions.iter().map(|t| t.serialized_size()).sum::<usize>() // transactions
        + 32 // pre_state_root
        + 32 // post_state_root
        + 8 // timestamp
        + ADDRESS_SIZE // sequencer
        + 4 + self.signature.as_ref().map(|s| s.len()).unwrap_or(0) // signature
    }
}

/// A simple transaction type for the rollup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction type
    pub tx_type: TxType,
    /// Sender address
    pub from: Address,
    /// Recipient address (None for contract creation)
    pub to: Option<Address>,
    /// Transaction data/payload
    pub data: Vec<u8>,
    /// Transaction nonce
    pub nonce: u64,
    /// Fee for the transaction
    pub fee: u64,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(
        tx_type: TxType,
        from: Address,
        to: Option<Address>,
        data: Vec<u8>,
        nonce: u64,
        fee: u64,
    ) -> Self {
        Self {
            tx_type,
            from,
            to,
            data,
            nonce,
            fee,
        }
    }

    /// Get serialized size
    pub fn serialized_size(&self) -> usize {
        1 // tx_type
        + ADDRESS_SIZE // from
        + 1 + self.to.as_ref().map(|_| ADDRESS_SIZE).unwrap_or(0) // to
        + 4 + self.data.len() // data
        + 8 // nonce
        + 8 // fee
    }
}

/// Transaction types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxType {
    /// Simple value transfer
    Transfer = 0,
    /// Contract call
    Call = 1,
    /// Contract creation
    Create = 2,
    /// Batch withdrawal
    Withdrawal = 3,
}

impl TxType {
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(TxType::Transfer),
            1 => Some(TxType::Call),
            2 => Some(TxType::Create),
            3 => Some(TxType::Withdrawal),
            _ => None,
        }
    }

    pub fn to_byte(&self) -> u8 {
        match self {
            TxType::Transfer => 0,
            TxType::Call => 1,
            TxType::Create => 2,
            TxType::Withdrawal => 3,
        }
    }
}

/// Current state of the L2 rollup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollupState {
    /// Current batch number
    pub batch_number: u64,
    /// Current state root
    pub state_root: [u8; 32],
    /// Pending withdrawals
    pub pending_withdrawals: Vec<Withdrawal>,
    /// Active sequencer
    pub sequencer: Address,
    /// Whether the sequencer is bonded
    pub sequencer_bonded: bool,
    /// Last finalized batch number
    pub finalized_batch: u64,
}

impl RollupState {
    /// Create a new rollup state
    pub fn new(sequencer: Address) -> Self {
        Self {
            batch_number: 0,
            state_root: [0u8; 32],
            pending_withdrawals: Vec::new(),
            sequencer,
            sequencer_bonded: false,
            finalized_batch: 0,
        }
    }

    /// Check if a batch can be finalized
    pub fn can_finalize(&self, batch_number: u64) -> bool {
        batch_number > self.finalized_batch && self.batch_number >= batch_number + CHALLENGE_PERIOD
    }
}

/// A withdrawal request from L2 to L1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Withdrawal {
    /// Unique withdrawal ID
    pub withdrawal_id: u64,
    /// Recipient address on L1
    pub recipient: Address,
    /// Amount being withdrawn
    pub amount: u64,
    /// Batch number where withdrawal was included
    pub batch_number: u64,
    /// Timestamp of withdrawal
    pub timestamp: u64,
    /// Whether withdrawal has been finalized
    pub finalized: bool,
    /// L1 transaction hash (after finalization)
    pub l1_tx_hash: Option<[u8; 32]>,
}

impl Withdrawal {
    /// Create a new withdrawal
    pub fn new(
        withdrawal_id: u64,
        recipient: Address,
        amount: u64,
        batch_number: u64,
        timestamp: u64,
    ) -> Self {
        Self {
            withdrawal_id,
            recipient,
            amount,
            batch_number,
            timestamp,
            finalized: false,
            l1_tx_hash: None,
        }
    }

    /// Finalize the withdrawal
    pub fn finalize(&mut self, l1_tx_hash: [u8; 32]) {
        self.finalized = true;
        self.l1_tx_hash = Some(l1_tx_hash);
    }
}

/// Operations that can be performed on the rollup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollupOperation {
    /// Submit a new batch
    SubmitBatch {
        /// The batch being submitted
        batch: RollupBatch,
        /// STARK proof for the batch (requires zk feature, otherwise empty)
        proof: Vec<u8>,
    },
    /// Challenge a submitted batch (fraud proof)
    ChallengeBatch {
        /// Batch being challenged
        batch_number: u64,
        /// Fraud proof
        proof: FraudProof,
    },
    /// Finalize a withdrawal
    FinalizeWithdrawal {
        /// Withdrawal to finalize
        withdrawal_id: u64,
    },
    /// Update sequencer
    UpdateSequencer {
        /// New sequencer address
        new_sequencer: Address,
        /// Bond amount
        bond: u64,
    },
}

/// Fraud proof for challenging invalid batches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudProof {
    /// Batch number being challenged
    pub batch_number: u64,
    /// Index of the fraudulent transaction
    pub transaction_index: u64,
    /// State root before the transaction
    pub pre_state_root: [u8; 32],
    /// Expected state root after the transaction
    pub expected_post_root: [u8; 32],
    /// Claimed state root (incorrect)
    pub claimed_post_root: [u8; 32],
    /// Merkle proof for the transaction
    pub proof: Vec<u8>,
    /// Input that caused the fraud
    pub input: Vec<u8>,
    /// Expected output
    pub expected_output: Vec<u8>,
    /// Claimed output (incorrect)
    pub claimed_output: Vec<u8>,
}

impl FraudProof {
    /// Create a new fraud proof
    pub fn new(
        batch_number: u64,
        transaction_index: u64,
        pre_state_root: [u8; 32],
        expected_post_root: [u8; 32],
        claimed_post_root: [u8; 32],
    ) -> Self {
        Self {
            batch_number,
            transaction_index,
            pre_state_root,
            expected_post_root,
            claimed_post_root,
            proof: Vec::new(),
            input: Vec::new(),
            expected_output: Vec::new(),
            claimed_output: Vec::new(),
        }
    }

    /// Verify the fraud proof
    pub fn verify(&self) -> bool {
        // The claimed post root must differ from expected
        self.claimed_post_root != self.expected_post_root
    }
}

// Serialization helpers

impl RollupBatch {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(self.serialized_size());

        result.extend_from_slice(&self.batch_number.to_le_bytes());
        result.extend_from_slice(&self.parent_hash);
        result.extend_from_slice(&(self.transactions.len() as u32).to_le_bytes());
        for tx in &self.transactions {
            result.push(tx.tx_type.to_byte());
            result.extend_from_slice(tx.from.as_bytes());
            result.push(if tx.to.is_some() { 1 } else { 0 });
            if let Some(ref to) = tx.to {
                result.extend_from_slice(to.as_bytes());
            }
            result.extend_from_slice(&(tx.data.len() as u32).to_le_bytes());
            result.extend_from_slice(&tx.data);
            result.extend_from_slice(&tx.nonce.to_le_bytes());
            result.extend_from_slice(&tx.fee.to_le_bytes());
        }
        result.extend_from_slice(&self.pre_state_root);
        result.extend_from_slice(&self.post_state_root);
        result.extend_from_slice(&self.timestamp.to_le_bytes());
        result.extend_from_slice(self.sequencer.as_bytes());
        result.push(if self.signature.is_some() { 1 } else { 0 });
        if let Some(ref sig) = self.signature {
            result.extend_from_slice(&(sig.len() as u32).to_le_bytes());
            result.extend_from_slice(sig);
        } else {
            result.extend_from_slice(&0u32.to_le_bytes());
        }

        result
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        let mut pos = 0;

        let batch_number = u64::from_le_bytes(data[pos..pos + 8].try_into().ok()?);
        pos += 8;

        let mut parent_hash = [0u8; 32];
        parent_hash.copy_from_slice(&data[pos..pos + 32]);
        pos += 32;

        let tx_count = u32::from_le_bytes(data[pos..pos + 4].try_into().ok()?) as usize;
        pos += 4;

        let mut transactions = Vec::with_capacity(tx_count);
        for _ in 0..tx_count {
            let tx_type = TxType::from_byte(data[pos])?;
            pos += 1;

            let mut from_bytes = [0u8; 20];
            from_bytes.copy_from_slice(&data[pos..pos + 20]);
            pos += 20;
            let from = Address::new(from_bytes);

            let has_to = data[pos] != 0;
            pos += 1;

            let to = if has_to {
                let mut to_bytes = [0u8; 20];
                to_bytes.copy_from_slice(&data[pos..pos + 20]);
                pos += 20;
                Some(Address::new(to_bytes))
            } else {
                None
            };

            let data_len = u32::from_le_bytes(data[pos..pos + 4].try_into().ok()?) as usize;
            pos += 4;
            let tx_data = data[pos..pos + data_len].to_vec();
            pos += data_len;

            let nonce = u64::from_le_bytes(data[pos..pos + 8].try_into().ok()?);
            pos += 8;

            let fee = u64::from_le_bytes(data[pos..pos + 8].try_into().ok()?);
            pos += 8;

            transactions.push(Transaction::new(tx_type, from, to, tx_data, nonce, fee));
        }

        let mut pre_state_root = [0u8; 32];
        pre_state_root.copy_from_slice(&data[pos..pos + 32]);
        pos += 32;

        let mut post_state_root = [0u8; 32];
        post_state_root.copy_from_slice(&data[pos..pos + 32]);
        pos += 32;

        let timestamp = u64::from_le_bytes(data[pos..pos + 8].try_into().ok()?);
        pos += 8;

        let mut sequencer_bytes = [0u8; 20];
        sequencer_bytes.copy_from_slice(&data[pos..pos + 20]);
        pos += 20;
        let sequencer = Address::new(sequencer_bytes);

        let has_sig = data[pos] != 0;
        pos += 1;

        let signature = if has_sig {
            let sig_len = u32::from_le_bytes(data[pos..pos + 4].try_into().ok()?) as usize;
            pos += 4;
            Some(data[pos..pos + sig_len].to_vec())
        } else {
            None
        };

        Some(RollupBatch {
            batch_number,
            parent_hash,
            transactions,
            pre_state_root,
            post_state_root,
            timestamp,
            sequencer,
            signature,
        })
    }
}

impl Withdrawal {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(&self.withdrawal_id.to_le_bytes());
        result.extend_from_slice(self.recipient.as_bytes());
        result.extend_from_slice(&self.amount.to_le_bytes());
        result.extend_from_slice(&self.batch_number.to_le_bytes());
        result.extend_from_slice(&self.timestamp.to_le_bytes());
        result.push(if self.finalized { 1 } else { 0 });
        if let Some(ref hash) = self.l1_tx_hash {
            result.push(1);
            result.extend_from_slice(hash);
        } else {
            result.push(0);
        }
        result
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        let mut pos = 0;

        let withdrawal_id = u64::from_le_bytes(data[pos..pos + 8].try_into().ok()?);
        pos += 8;

        let mut recipient_bytes = [0u8; 20];
        recipient_bytes.copy_from_slice(&data[pos..pos + 20]);
        pos += 20;

        let amount = u64::from_le_bytes(data[pos..pos + 8].try_into().ok()?);
        pos += 8;

        let batch_number = u64::from_le_bytes(data[pos..pos + 8].try_into().ok()?);
        pos += 8;

        let timestamp = u64::from_le_bytes(data[pos..pos + 8].try_into().ok()?);
        pos += 8;

        let finalized = data[pos] != 0;
        pos += 1;

        let l1_tx_hash = if data[pos] != 0 {
            pos += 1;
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&data[pos..pos + 32]);
            Some(hash)
        } else {
            None
        };

        Some(Withdrawal {
            withdrawal_id,
            recipient: Address::new(recipient_bytes),
            amount,
            batch_number,
            timestamp,
            finalized,
            l1_tx_hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address() {
        let addr = Address::new([1u8; 20]);
        assert!(!addr.is_zero());

        let zero = Address::zero();
        assert!(zero.is_zero());
    }

    #[test]
    fn test_rollup_batch_new() {
        let batch = RollupBatch::new(
            1,
            [0u8; 32],
            vec![],
            [1u8; 32],
            [2u8; 32],
            1000,
            Address::zero(),
        );

        assert_eq!(batch.batch_number, 1);
        assert_eq!(batch.len(), 0);
        assert!(batch.is_empty());
    }

    #[test]
    fn test_rollup_batch_hash() {
        let batch = RollupBatch::new(
            1,
            [0u8; 32],
            vec![],
            [1u8; 32],
            [2u8; 32],
            1000,
            Address::zero(),
        );

        let hash1 = batch.hash();
        let hash2 = batch.hash();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_rollup_batch_roundtrip() {
        // Test with empty transactions first
        let batch = RollupBatch::new(
            42,
            [1u8; 32],
            vec![],
            [4u8; 32],
            [5u8; 32],
            2000,
            Address::new([6u8; 20]),
        );

        let bytes = batch.to_bytes();
        eprintln!("Bytes length: {}", bytes.len());
        let recovered = RollupBatch::from_bytes(&bytes).unwrap();

        assert_eq!(batch.batch_number, recovered.batch_number);
        assert_eq!(batch.parent_hash, recovered.parent_hash);
        assert_eq!(batch.transactions.len(), recovered.transactions.len());
        assert_eq!(batch.pre_state_root, recovered.pre_state_root);
        assert_eq!(batch.post_state_root, recovered.post_state_root);
    }

    #[test]
    fn test_rollup_batch_with_tx_roundtrip() {
        // Test with a transaction
        let batch = RollupBatch::new(
            42,
            [1u8; 32],
            vec![Transaction::new(
                TxType::Transfer,
                Address::new([2u8; 20]),
                None,
                vec![1, 2, 3],
                5,
                100,
            )],
            [4u8; 32],
            [5u8; 32],
            2000,
            Address::new([6u8; 20]),
        );

        let bytes = batch.to_bytes();
        eprintln!("Bytes with tx length: {}", bytes.len());
        let recovered = RollupBatch::from_bytes(&bytes).unwrap();

        assert_eq!(batch.batch_number, recovered.batch_number);
        assert_eq!(batch.parent_hash, recovered.parent_hash);
        assert_eq!(batch.transactions.len(), recovered.transactions.len());
        assert_eq!(batch.pre_state_root, recovered.pre_state_root);
        assert_eq!(batch.post_state_root, recovered.post_state_root);
    }

    #[test]
    fn test_withdrawal_new() {
        let withdrawal = Withdrawal::new(1, Address::new([1u8; 20]), 1000, 42, 1000);

        assert_eq!(withdrawal.withdrawal_id, 1);
        assert_eq!(withdrawal.amount, 1000);
        assert!(!withdrawal.finalized);
    }

    #[test]
    fn test_withdrawal_finalize() {
        let mut withdrawal = Withdrawal::new(1, Address::new([1u8; 20]), 1000, 42, 1000);

        withdrawal.finalize([9u8; 32]);

        assert!(withdrawal.finalized);
        assert!(withdrawal.l1_tx_hash.is_some());
    }

    #[test]
    fn test_withdrawal_roundtrip() {
        let mut withdrawal = Withdrawal::new(1, Address::new([1u8; 20]), 1000, 42, 1000);
        withdrawal.finalize([9u8; 32]);

        let bytes = withdrawal.to_bytes();
        let recovered = Withdrawal::from_bytes(&bytes).unwrap();

        assert_eq!(withdrawal.withdrawal_id, recovered.withdrawal_id);
        assert_eq!(withdrawal.amount, recovered.amount);
        assert_eq!(withdrawal.finalized, recovered.finalized);
    }

    #[test]
    fn test_fraud_proof_verify() {
        let proof = FraudProof::new(
            1, 0, [1u8; 32], [2u8; 32], [3u8; 32], // Different from expected
        );

        assert!(proof.verify());
    }

    #[test]
    fn test_fraud_proof_same_roots() {
        let same_root = [1u8; 32];
        let proof = FraudProof::new(
            1, 0, same_root, same_root, // Same as claimed
            same_root,
        );

        assert!(!proof.verify());
    }

    #[test]
    fn test_rollup_state_new() {
        let state = RollupState::new(Address::new([1u8; 20]));

        assert_eq!(state.batch_number, 0);
        assert_eq!(state.state_root, [0u8; 32]);
        assert!(state.pending_withdrawals.is_empty());
        assert!(!state.sequencer_bonded);
    }

    #[test]
    fn test_rollup_state_can_finalize() {
        let mut state = RollupState::new(Address::new([1u8; 20]));
        state.batch_number = 200;
        state.finalized_batch = 50;

        // Can finalize batch 100 (since 200 >= 100 + 100 = 200)
        assert!(state.can_finalize(100));

        // Cannot finalize already finalized batch
        assert!(!state.can_finalize(50));

        // Cannot finalize future batch (needs batch_number + CHALLENGE_PERIOD <= batch_number)
        assert!(!state.can_finalize(150));
    }

    #[test]
    fn test_tx_type_serialization() {
        for tx_type in &[
            TxType::Transfer,
            TxType::Call,
            TxType::Create,
            TxType::Withdrawal,
        ] {
            let byte = tx_type.to_byte();
            let recovered = TxType::from_byte(byte);
            assert_eq!(Some(*tx_type), recovered);
        }
    }
}
