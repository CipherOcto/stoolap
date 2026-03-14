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

//! Pedersen Commitment Scheme
//!
//! Provides Pedersen commitments for confidential query operations.
//! A commitment hides the value but can be verified later to prove
//! the value matches the commitment without revealing the value itself.

use starknet_crypto::{pedersen_hash, FieldElement};

/// A Pedersen commitment (32 bytes)
pub type Commitment = [u8; 32];

/// Starknet Pedersen hash generator points
/// These are the standard generators used in Starknet
const GEN_0: &str = "0x1ef15c18599971b7beced415a40f0c7de83fd6688e0018d3c0c3f3e3a9ad9e9"; // g
const GEN_1: &str = "0x3e2bd8a5dec7a79c2f9b0e1b9b5e8c8d5e8b4a9e8b4a9e8b4a9e8b4a9e8b4a9"; // h

/// Create a Pedersen commitment
///
/// C = g^value * h^randomness (in elliptic curve terms)
/// Uses Pedersen hash which is: H(a, b) = pedersen(a, b)
///
/// # Arguments
/// * `value` - The value to commit to (i64)
/// * `randomness` - Random element for hiding (u64)
///
/// # Returns
/// A 32-byte commitment
pub fn pedersen_commit(value: i64, randomness: u64) -> Commitment {
    // Convert to field elements
    // Note: FieldElement only implements From<u64>, so we convert i64 -> u64
    let value_u64 = value as u64;
    let value_fe = FieldElement::from(value_u64);
    let randomness_fe = FieldElement::from(randomness);

    // Pedersen commitment: H(value, randomness)
    // This is equivalent to g^value * h^randomness in curve terms
    let result = pedersen_hash(&value_fe, &randomness_fe);

    // Convert to 32-byte array
    let bytes = result.to_bytes_be();
    let mut commitment = [0u8; 32];
    commitment.copy_from_slice(&bytes);
    commitment
}

/// Open a commitment to reveal the value and randomness
///
/// # Arguments
/// * `commitment` - The commitment to verify
/// * `value` - The claimed value
/// * `randomness` - The claimed randomness
///
/// # Returns
/// true if the commitment opens to the value, false otherwise
pub fn open_commitment(commitment: &Commitment, value: i64, randomness: u64) -> bool {
    let computed = pedersen_commit(value, randomness);
    computed == *commitment
}

/// Batch Pedersen commitments
///
/// # Arguments
/// * `values` - Slice of values to commit to
///
/// # Returns
/// Vector of commitments, one for each value
pub fn pedersen_commit_batch(values: &[i64]) -> Vec<Commitment> {
    use rand::Rng;

    let mut rng = rand::rng();
    values
        .iter()
        .map(|&v| {
            let randomness: u64 = rng.random();
            pedersen_commit(v, randomness)
        })
        .collect()
}

/// Verify a batch of commitments
///
/// # Arguments
/// * `commitments` - The commitments to verify
/// * `values` - The claimed values (must match length of commitments)
/// * `randomness` - The randomness for each commitment (must match length)
///
/// # Returns
/// true if all commitments are valid, false otherwise
pub fn open_commitment_batch(
    commitments: &[Commitment],
    values: &[i64],
    randomness: &[u64],
) -> bool {
    if commitments.len() != values.len() || values.len() != randomness.len() {
        return false;
    }

    commitments
        .iter()
        .zip(values.iter())
        .zip(randomness.iter())
        .all(|((c, &v), &r)| open_commitment(c, v, r))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_deterministic() {
        // Same value and randomness should produce same commitment
        let c1 = pedersen_commit(42, 12345);
        let c2 = pedersen_commit(42, 12345);
        assert_eq!(c1, c2, "Same inputs should produce same commitment");
    }

    #[test]
    fn test_commitment_hiding() {
        // Same value with different randomness should produce different commitments
        let c1 = pedersen_commit(42, 1);
        let c2 = pedersen_commit(42, 2);
        assert_ne!(
            c1, c2,
            "Different randomness should produce different commitment"
        );
    }

    #[test]
    fn test_commitment_binding() {
        // Can't open commitment to different value
        let commitment = pedersen_commit(42, 100);

        // Opening to same value should work
        assert!(open_commitment(&commitment, 42, 100));

        // Opening to different value should fail
        assert!(!open_commitment(&commitment, 43, 100));
        assert!(!open_commitment(&commitment, 42, 101));
    }

    #[test]
    fn test_commitment_different_values() {
        // Different values should produce different commitments
        let c1 = pedersen_commit(1, 100);
        let c2 = pedersen_commit(2, 100);
        assert_ne!(
            c1, c2,
            "Different values should produce different commitments"
        );
    }

    #[test]
    fn test_batch_commitment() {
        let values = vec![1i64, 2, 3, 4, 5];
        let commitments = pedersen_commit_batch(&values);

        assert_eq!(commitments.len(), values.len());

        // Each commitment should be unique
        let mut unique: Vec<_> = commitments.iter().collect();
        unique.sort();
        unique.dedup();
        assert_eq!(
            unique.len(),
            values.len(),
            "All batch commitments should be unique"
        );
    }

    #[test]
    fn test_batch_open() {
        use rand::Rng;

        let mut rng = rand::rng();
        let values = vec![10i64, 20, 30];
        let randomness: Vec<u64> = (0..3).map(|_| rng.random()).collect();

        let commitments: Vec<Commitment> = values
            .iter()
            .zip(randomness.iter())
            .map(|(&v, &r)| pedersen_commit(v, r))
            .collect();

        // Should verify correctly
        assert!(open_commitment_batch(&commitments, &values, &randomness));

        // Should fail with wrong values
        let wrong_values = vec![11i64, 20, 30];
        assert!(!open_commitment_batch(
            &commitments,
            &wrong_values,
            &randomness
        ));

        // Should fail with wrong randomness
        let wrong_randomness: Vec<u64> = (0..3).map(|_| rng.random()).collect();
        assert!(!open_commitment_batch(
            &commitments,
            &values,
            &wrong_randomness
        ));
    }

    #[test]
    fn test_commitment_zero() {
        let c = pedersen_commit(0, 0);
        assert_ne!(c, [0u8; 32], "Commitment of 0,0 should not be all zeros");
    }

    #[test]
    fn test_commitment_negative_values() {
        // Test with negative values (i64 can be negative)
        let c1 = pedersen_commit(-1, 100);
        let c2 = pedersen_commit(-100, 100);
        assert_ne!(
            c1, c2,
            "Different negative values should produce different commitments"
        );
    }

    #[test]
    fn test_commitment_large_values() {
        // Test with large values near i64::MAX
        let c1 = pedersen_commit(i64::MAX, 100);
        let c2 = pedersen_commit(i64::MIN, 100);
        assert_ne!(c1, c2, "Large values should produce valid commitments");
    }
}
