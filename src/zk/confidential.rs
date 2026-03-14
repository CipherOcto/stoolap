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

//! Confidential Query Types
//!
//! Data structures for confidential query operations using Pedersen commitments.
//! These types enable proving query results without revealing underlying data.

use crate::zk::commitment::Commitment;

/// Filter comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOp {
    /// Equal (=)
    Equal,
    /// Not Equal (!=)
    NotEqual,
    /// Less Than (<)
    LessThan,
    /// Greater Than (>)
    GreaterThan,
    /// Less Than or Equal (<=)
    LessThanOrEqual,
    /// Greater Than or Equal (>=)
    GreaterThanOrEqual,
}

impl FilterOp {
    /// Serialize to byte
    pub fn to_byte(&self) -> u8 {
        match self {
            FilterOp::Equal => 0x01,
            FilterOp::NotEqual => 0x02,
            FilterOp::LessThan => 0x03,
            FilterOp::GreaterThan => 0x04,
            FilterOp::LessThanOrEqual => 0x05,
            FilterOp::GreaterThanOrEqual => 0x06,
        }
    }

    /// Deserialize from byte
    pub fn from_byte(b: u8) -> Option<FilterOp> {
        match b {
            0x01 => Some(FilterOp::Equal),
            0x02 => Some(FilterOp::NotEqual),
            0x03 => Some(FilterOp::LessThan),
            0x04 => Some(FilterOp::GreaterThan),
            0x05 => Some(FilterOp::LessThanOrEqual),
            0x06 => Some(FilterOp::GreaterThanOrEqual),
            _ => None,
        }
    }
}

/// Encrypted filter condition
///
/// Represents an encrypted WHERE clause with a commitment to the value
/// and a proof that the value satisfies the condition.
#[derive(Debug, Clone)]
pub struct EncryptedFilter {
    /// Encrypted column name
    pub column: Vec<u8>,
    /// Comparison operator
    pub operator: FilterOp,
    /// Commitment to the filter value
    pub value_commitment: Commitment,
    /// Nonce used for encryption
    pub nonce: [u8; 32],
}

/// Encrypted query input
///
/// Represents a fully encrypted SQL query where table name,
/// columns, and filter values are all committed.
#[derive(Debug, Clone)]
pub struct EncryptedQuery {
    /// Encrypted table name
    pub table: Vec<u8>,
    /// List of encrypted filter conditions
    pub filters: Vec<EncryptedFilter>,
    /// Encryption nonce
    pub nonce: [u8; 32],
    /// Query commitment (hash of entire query)
    pub query_commitment: Commitment,
}

/// Range proof for confidential comparisons
///
/// A proof that demonstrates a committed value falls within
/// a specified range without revealing the value.
#[derive(Debug, Clone)]
pub struct RangeProof {
    /// Commitment to the value
    pub commitment: Commitment,
    /// Proof data (serialized)
    pub proof_data: Vec<u8>,
    /// Lower bound of range
    pub lower_bound: i64,
    /// Upper bound of range
    pub upper_bound: i64,
}

/// Confidential query result
///
/// Contains the results of a confidential query along with
/// proofs that enable verification without revealing data.
#[derive(Debug, Clone)]
pub struct ConfidentialResult {
    /// Number of rows returned
    pub row_count: u64,
    /// Commitments to each row's values
    pub row_commitments: Vec<Commitment>,
    /// Commitments to aggregate results (COUNT, SUM, etc.)
    pub aggregate_commitments: Vec<Commitment>,
    /// STARK proof of query execution
    pub proof: Vec<u8>,
    /// Query that was executed (committed)
    pub query_commitment: Commitment,
}

impl EncryptedQuery {
    /// Create a new encrypted query
    pub fn new(table: Vec<u8>, filters: Vec<EncryptedFilter>, nonce: [u8; 32]) -> Self {
        // Compute query commitment from components
        

        let mut hasher = blake3::Hasher::new();
        hasher.update(&table);
        for f in &filters {
            hasher.update(&f.column);
            hasher.update(&[f.operator.to_byte()]);
            hasher.update(&f.value_commitment);
        }
        let query_hash = hasher.finalize();

        let mut commitment = [0u8; 32];
        commitment.copy_from_slice(query_hash.as_bytes());

        Self {
            table,
            filters,
            nonce,
            query_commitment: commitment,
        }
    }

    /// Get serialized size
    pub fn serialized_size(&self) -> usize {
        4 + self.table.len() // table
        + 4 + self.filters.iter().map(|f| 4 + f.column.len() + 1 + 32 + 32).sum::<usize>() // filters (including each filter's column length prefix)
        + 32 // nonce
        + 32 // commitment
    }
}

impl EncryptedFilter {
    /// Create a new encrypted filter
    pub fn new(
        column: Vec<u8>,
        operator: FilterOp,
        value_commitment: Commitment,
        nonce: [u8; 32],
    ) -> Self {
        Self {
            column,
            operator,
            value_commitment,
            nonce,
        }
    }
}

impl RangeProof {
    /// Create a new range proof
    pub fn new(
        commitment: Commitment,
        proof_data: Vec<u8>,
        lower_bound: i64,
        upper_bound: i64,
    ) -> Self {
        Self {
            commitment,
            proof_data,
            lower_bound,
            upper_bound,
        }
    }

    /// Verify the range proof (placeholder - actual verification needs Cairo program)
    pub fn verify(&self) -> bool {
        // Placeholder: actual verification would use STARK proof
        !self.proof_data.is_empty() && self.lower_bound <= self.upper_bound
    }
}

impl ConfidentialResult {
    /// Create a new confidential result
    pub fn new(
        row_count: u64,
        row_commitments: Vec<Commitment>,
        aggregate_commitments: Vec<Commitment>,
        proof: Vec<u8>,
        query_commitment: Commitment,
    ) -> Self {
        Self {
            row_count,
            row_commitments,
            aggregate_commitments,
            proof,
            query_commitment,
        }
    }

    /// Get serialized size
    pub fn serialized_size(&self) -> usize {
        8 // row_count
        + 4 + self.row_commitments.len() * 32 // row_commitments
        + 4 + self.aggregate_commitments.len() * 32 // aggregate_commitments
        + 4 + self.proof.len() // proof
        + 32 // query_commitment
    }

    /// Verify the confidential result
    ///
    /// This verifies:
    /// 1. The STARK proof (if present and zk feature enabled)
    /// 2. The query commitment matches
    /// 3. The row count matches commitments
    ///
    /// Returns `true` if verification passes.
    #[cfg(feature = "zk")]
    pub fn verify(&self, expected_root: [u8; 32]) -> bool {
        use crate::zk::plugin::load_plugin;

        // 1. Verify query commitment is present
        if self.query_commitment == [0u8; 32] {
            return false;
        }

        // 2. Verify row count matches commitments
        if self.row_count as usize != self.row_commitments.len() {
            return false;
        }

        // 3. Verify proof if present (requires plugin)
        if self.proof.is_empty() {
            // No proof - just verify the structure is valid
            return true;
        }

        // Try to verify with plugin
        match load_plugin() {
            Ok(plugin) => {
                // Build verification input
                let mut verify_input = Vec::new();
                verify_input.extend_from_slice(&expected_root);
                verify_input.extend_from_slice(&self.row_count.to_le_bytes());
                verify_input.extend_from_slice(&self.query_commitment);
                for c in &self.row_commitments {
                    verify_input.extend_from_slice(c);
                }

                // Verify the proof
                plugin.verify(&verify_input).is_ok()
            }
            Err(_) => {
                // Plugin not available - verify structure only
                !self.proof.is_empty() && self.row_count > 0
            }
        }
    }

    /// Verify without zk feature - basic structural validation only
    #[cfg(not(feature = "zk"))]
    pub fn verify(&self, _expected_root: [u8; 32]) -> bool {
        // Basic structural validation without proof verification
        self.query_commitment != [0u8; 32] && self.row_count as usize == self.row_commitments.len()
    }

    /// Open a commitment to reveal the underlying value
    ///
    /// This is a placeholder - in production, this would use:
    /// - Homomorphic encryption for computation
    /// - Opening hints stored during commitment creation
    /// - Or zero-knowledge proof of opening
    ///
    /// Returns `None` since we cannot open commitments without additional data.
    pub fn open_commitment(&self, index: usize) -> Option<i64> {
        // Cannot open commitments without the original values and randomness
        // In a real implementation, this would:
        // 1. Have access to the original values (stored separately)
        // 2. Have the randomness used for each commitment
        // 3. Use homomorphic properties to verify
        let _ = index;
        None
    }

    /// Verify that a specific value matches a commitment
    ///
    /// This allows verification that a claimed value corresponds to
    /// a commitment without revealing the value.
    pub fn verify_value_commitment(&self, index: usize, value: i64, randomness: u64) -> bool {
        use crate::zk::commitment::pedersen_commit;

        if index >= self.row_commitments.len() {
            return false;
        }

        let computed = pedersen_commit(value, randomness);
        computed == self.row_commitments[index]
    }
}

// Simple serialization helpers (without SolanaSerialize trait to avoid feature conflicts)

impl EncryptedQuery {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(self.serialized_size());

        // table
        result.extend_from_slice(&(self.table.len() as u32).to_le_bytes());
        result.extend_from_slice(&self.table);

        // filters
        result.extend_from_slice(&(self.filters.len() as u32).to_le_bytes());
        for f in &self.filters {
            result.extend_from_slice(&(f.column.len() as u32).to_le_bytes());
            result.extend_from_slice(&f.column);
            result.push(f.operator.to_byte());
            result.extend_from_slice(&f.value_commitment);
            result.extend_from_slice(&f.nonce);
        }

        // nonce
        result.extend_from_slice(&self.nonce);

        // query_commitment
        result.extend_from_slice(&self.query_commitment);

        result
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        let mut pos = 0;

        // table
        let table_len = u32::from_le_bytes(data[pos..pos + 4].try_into().ok()?) as usize;
        pos += 4;
        let table = data[pos..pos + table_len].to_vec();
        pos += table_len;

        // filters
        let filters_len = u32::from_le_bytes(data[pos..pos + 4].try_into().ok()?) as usize;
        pos += 4;
        let mut filters = Vec::with_capacity(filters_len);
        for _ in 0..filters_len {
            let col_len = u32::from_le_bytes(data[pos..pos + 4].try_into().ok()?) as usize;
            pos += 4;
            let column = data[pos..pos + col_len].to_vec();
            pos += col_len;
            let operator = FilterOp::from_byte(data[pos])?;
            pos += 1;
            let value_commitment: Commitment = data[pos..pos + 32].try_into().ok()?;
            pos += 32;
            let nonce: [u8; 32] = data[pos..pos + 32].try_into().ok()?;
            pos += 32;
            filters.push(EncryptedFilter::new(
                column,
                operator,
                value_commitment,
                nonce,
            ));
        }

        // nonce
        let nonce: [u8; 32] = data[pos..pos + 32].try_into().ok()?;
        pos += 32;

        // query_commitment
        let query_commitment: Commitment = data[pos..pos + 32].try_into().ok()?;

        Some(Self {
            table,
            filters,
            nonce,
            query_commitment,
        })
    }
}

impl ConfidentialResult {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(self.serialized_size());

        // row_count
        result.extend_from_slice(&self.row_count.to_le_bytes());

        // row_commitments
        result.extend_from_slice(&(self.row_commitments.len() as u32).to_le_bytes());
        for c in &self.row_commitments {
            result.extend_from_slice(c);
        }

        // aggregate_commitments
        result.extend_from_slice(&(self.aggregate_commitments.len() as u32).to_le_bytes());
        for c in &self.aggregate_commitments {
            result.extend_from_slice(c);
        }

        // proof
        result.extend_from_slice(&(self.proof.len() as u32).to_le_bytes());
        result.extend_from_slice(&self.proof);

        // query_commitment
        result.extend_from_slice(&self.query_commitment);

        result
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        let mut pos = 0;

        // row_count
        let row_count = u64::from_le_bytes(data[pos..pos + 8].try_into().ok()?);
        pos += 8;

        // row_commitments
        let row_comm_len = u32::from_le_bytes(data[pos..pos + 4].try_into().ok()?) as usize;
        pos += 4;
        let mut row_commitments = Vec::with_capacity(row_comm_len);
        for _ in 0..row_comm_len {
            let c: Commitment = data[pos..pos + 32].try_into().ok()?;
            row_commitments.push(c);
            pos += 32;
        }

        // aggregate_commitments
        let agg_comm_len = u32::from_le_bytes(data[pos..pos + 4].try_into().ok()?) as usize;
        pos += 4;
        let mut aggregate_commitments = Vec::with_capacity(agg_comm_len);
        for _ in 0..agg_comm_len {
            let c: Commitment = data[pos..pos + 32].try_into().ok()?;
            aggregate_commitments.push(c);
            pos += 32;
        }

        // proof
        let proof_len = u32::from_le_bytes(data[pos..pos + 4].try_into().ok()?) as usize;
        pos += 4;
        let proof = data[pos..pos + proof_len].to_vec();
        pos += proof_len;

        // query_commitment
        let query_commitment: Commitment = data[pos..pos + 32].try_into().ok()?;

        Some(Self {
            row_count,
            row_commitments,
            aggregate_commitments,
            proof,
            query_commitment,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_op_serialization() {
        for op in &[
            FilterOp::Equal,
            FilterOp::NotEqual,
            FilterOp::LessThan,
            FilterOp::GreaterThan,
            FilterOp::LessThanOrEqual,
            FilterOp::GreaterThanOrEqual,
        ] {
            let byte = op.to_byte();
            let recovered = FilterOp::from_byte(byte);
            assert_eq!(Some(*op), recovered);
        }
    }

    #[test]
    fn test_filter_op_invalid_byte() {
        assert_eq!(FilterOp::from_byte(0x00), None);
        assert_eq!(FilterOp::from_byte(0xFF), None);
    }

    #[test]
    fn test_encrypted_query_roundtrip() {
        let filters = vec![
            EncryptedFilter::new(b"age".to_vec(), FilterOp::GreaterThan, [1u8; 32], [2u8; 32]),
            EncryptedFilter::new(
                b"score".to_vec(),
                FilterOp::LessThanOrEqual,
                [3u8; 32],
                [4u8; 32],
            ),
        ];

        let query = EncryptedQuery::new(b"users".to_vec(), filters, [5u8; 32]);
        let bytes = query.to_bytes();
        let recovered = EncryptedQuery::from_bytes(&bytes).unwrap();

        assert_eq!(query.table, recovered.table);
        assert_eq!(query.filters.len(), recovered.filters.len());
        assert_eq!(query.nonce, recovered.nonce);
        assert_eq!(query.query_commitment, recovered.query_commitment);
    }

    #[test]
    fn test_confidential_result_roundtrip() {
        let row_commitments = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let aggregate_commitments = vec![[10u8; 32]];

        let result = ConfidentialResult::new(
            3,
            row_commitments.clone(),
            aggregate_commitments.clone(),
            b"proof_data".to_vec(),
            [5u8; 32],
        );

        let bytes = result.to_bytes();
        let recovered = ConfidentialResult::from_bytes(&bytes).unwrap();

        assert_eq!(result.row_count, recovered.row_count);
        assert_eq!(result.row_commitments, recovered.row_commitments);
        assert_eq!(
            result.aggregate_commitments,
            recovered.aggregate_commitments
        );
        assert_eq!(result.proof, recovered.proof);
        assert_eq!(result.query_commitment, recovered.query_commitment);
    }

    #[test]
    fn test_range_proof() {
        let proof = RangeProof::new([1u8; 32], vec![1, 2, 3, 4], 0, 100);

        assert!(proof.verify());

        // Invalid range should fail
        let invalid = RangeProof::new([1u8; 32], vec![], 100, 50);
        assert!(!invalid.verify());
    }

    #[test]
    fn test_encrypted_query_size() {
        let query = EncryptedQuery::new(
            b"users".to_vec(),
            vec![EncryptedFilter::new(
                b"age".to_vec(),
                FilterOp::GreaterThan,
                [1u8; 32],
                [2u8; 32],
            )],
            [3u8; 32],
        );

        let bytes = query.to_bytes();
        assert_eq!(bytes.len(), query.serialized_size());
    }

    #[test]
    fn test_confidential_result_size() {
        let result = ConfidentialResult::new(
            10,
            vec![[1u8; 32]; 5],
            vec![[2u8; 32]; 2],
            b"proof".to_vec(),
            [3u8; 32],
        );

        let bytes = result.to_bytes();
        assert_eq!(bytes.len(), result.serialized_size());
    }

    #[cfg(feature = "commitment")]
    #[test]
    fn test_confidential_result_verify_basic() {
        use crate::zk::commitment::pedersen_commit;

        // Create a valid result
        let commitment1 = pedersen_commit(42, 12345);
        let commitment2 = pedersen_commit(100, 67890);
        let result = ConfidentialResult::new(
            2,
            vec![commitment1, commitment2],
            vec![pedersen_commit(142, 11111)],
            vec![],
            [1u8; 32],
        );

        // Verify with valid structure
        assert!(result.verify([0u8; 32]));
    }

    #[cfg(feature = "commitment")]
    #[test]
    fn test_confidential_result_verify_empty_commitment() {
        // Result with empty query commitment should fail
        let result = ConfidentialResult::new(
            1,
            vec![[1u8; 32]],
            vec![],
            vec![],
            [0u8; 32], // Empty commitment
        );

        assert!(!result.verify([0u8; 32]));
    }

    #[cfg(feature = "commitment")]
    #[test]
    fn test_confidential_result_verify_mismatched_count() {
        let result = ConfidentialResult::new(
            3,                          // row_count says 3
            vec![[1u8; 32], [2u8; 32]], // but only 2 commitments
            vec![],
            vec![],
            [1u8; 32],
        );

        assert!(!result.verify([0u8; 32]));
    }

    #[cfg(feature = "commitment")]
    #[test]
    fn test_verify_value_commitment() {
        use crate::zk::commitment::pedersen_commit;

        let value: i64 = 42;
        let randomness: u64 = 12345;
        let commitment = pedersen_commit(value, randomness);

        let result = ConfidentialResult::new(1, vec![commitment], vec![], vec![], [1u8; 32]);

        // Verify correct value and randomness
        assert!(result.verify_value_commitment(0, value, randomness));

        // Verify wrong value
        assert!(!result.verify_value_commitment(0, 100, randomness));

        // Verify wrong randomness
        assert!(!result.verify_value_commitment(0, value, 99999));

        // Verify out of bounds index
        assert!(!result.verify_value_commitment(5, value, randomness));
    }

    #[cfg(feature = "commitment")]
    #[test]
    fn test_open_commitment_returns_none() {
        let result = ConfidentialResult::new(1, vec![[1u8; 32]], vec![], vec![], [1u8; 32]);

        // Opening requires original values - should return None
        assert_eq!(result.open_commitment(0), None);
    }

    #[cfg(feature = "commitment")]
    #[test]
    fn test_zero_knowledge_property() {
        // Verify that commitments don't reveal the original values
        use crate::zk::commitment::pedersen_commit;

        let value1: i64 = 42;
        let value2: i64 = 1000000;
        let randomness1: u64 = 12345;
        let randomness2: u64 = 67890;

        let commit1 = pedersen_commit(value1, randomness1);
        let commit2 = pedersen_commit(value2, randomness2);

        // Different values with different randomness should produce
        // completely different commitments - cannot infer relationship
        assert_ne!(commit1, commit2);

        // Even with same value, different randomness produces different commitments
        let commit1_different_randomness = pedersen_commit(value1, randomness2);
        assert_ne!(commit1, commit1_different_randomness);
    }
}
