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

//! Tests for RowTrie implementation

use crate::determ::DetermRow;
use crate::determ::DetermValue;
use crate::trie::row_trie::RowTrie;

#[test]
fn test_row_trie_insert() {
    let mut trie = RowTrie::new();

    // Create a test row
    let row = DetermRow::from_values(vec![
        DetermValue::integer(1),
        DetermValue::text("Alice"),
        DetermValue::integer(30),
    ]);

    // Insert the row
    let (root_hash, diff) = trie.insert(1, row.clone());

    // Verify the diff
    assert_eq!(diff.inserted.len(), 1);
    assert_eq!(diff.inserted[0].0, 1);
    assert_eq!(diff.updated.len(), 0);
    assert_eq!(diff.deleted.len(), 0);

    // Verify root hash is not zero
    assert_ne!(root_hash, [0u8; 32]);

    // Verify we can get the root
    let stored_root = trie.get_root();
    assert_eq!(stored_root, root_hash);

    // Verify row count
    assert_eq!(trie.len(), 1);

    // Insert another row
    let row2 = DetermRow::from_values(vec![
        DetermValue::integer(2),
        DetermValue::text("Bob"),
        DetermValue::integer(25),
    ]);

    let (root_hash2, _diff2) = trie.insert(2, row2);

    // Root should change
    assert_ne!(root_hash2, root_hash);
    assert_eq!(trie.len(), 2);

    // Update existing row
    let row_updated = DetermRow::from_values(vec![
        DetermValue::integer(1),
        DetermValue::text("Alice Updated"),
        DetermValue::integer(31),
    ]);

    let (root_hash3, diff3) = trie.insert(1, row_updated);

    // Debug output
    println!("After second insert, root_hash2: {:?}", root_hash2);
    println!("Before update, len: {}", trie.len());

    // Verify update diff
    assert_eq!(diff3.inserted.len(), 0);
    assert_eq!(diff3.updated.len(), 1);
    assert_eq!(diff3.updated[0].0, 1);
    assert_ne!(diff3.updated[0].1, diff3.updated[0].2); // old_hash != new_hash
    assert_eq!(diff3.deleted.len(), 0);

    // Root should change on update
    assert_ne!(root_hash3, root_hash2);
    assert_eq!(trie.len(), 2); // Count should not increase
}

#[test]
fn test_row_trie_get() {
    let mut trie = RowTrie::new();

    // Create test rows
    let row1 = DetermRow::from_values(vec![
        DetermValue::integer(1),
        DetermValue::text("Alice"),
        DetermValue::integer(30),
    ]);

    let row2 = DetermRow::from_values(vec![
        DetermValue::integer(2),
        DetermValue::text("Bob"),
        DetermValue::integer(25),
    ]);

    // Insert rows
    trie.insert(1, row1.clone());
    trie.insert(2, row2.clone());

    // Get existing row
    let retrieved = trie.get(1);
    assert!(retrieved.is_some());
    let retrieved_row = retrieved.unwrap();
    assert_eq!(retrieved_row.len(), 3);
    assert_eq!(retrieved_row[0], DetermValue::integer(1));
    assert_eq!(retrieved_row[1], DetermValue::text("Alice"));
    assert_eq!(retrieved_row[2], DetermValue::integer(30));

    // Get non-existing row
    let non_existing = trie.get(999);
    assert!(non_existing.is_none());

    // Get hash only
    let hash = trie.get_hash(1);
    assert!(hash.is_some());
    assert_ne!(hash.unwrap(), [0u8; 32]);

    // Get hash of non-existing row
    let hash_non_existing = trie.get_hash(999);
    assert!(hash_non_existing.is_none());
}

#[test]
fn test_row_trie_get_hexary_proof() {
    let mut trie = RowTrie::new();

    // Create test rows
    let row1 = DetermRow::from_values(vec![
        DetermValue::integer(1),
        DetermValue::text("Alice"),
        DetermValue::integer(30),
    ]);

    let row2 = DetermRow::from_values(vec![
        DetermValue::integer(2),
        DetermValue::text("Bob"),
        DetermValue::integer(25),
    ]);

    // Insert rows
    trie.insert(1, row1);
    trie.insert(2, row2);

    // Verify both rows exist
    assert!(trie.get(1).is_some(), "Row 1 should exist");
    assert!(trie.get(2).is_some(), "Row 2 should exist");

    // Get hexary proof for existing row
    let proof = trie.get_hexary_proof(1);
    assert!(proof.is_some());

    let proof = proof.unwrap();
    assert_eq!(proof.root, trie.get_root());

    // Verify the proof
    assert!(proof.verify(), "Proof should verify for row 1");

    // Also test proof for row 2
    let proof2 = trie.get_hexary_proof(2);
    assert!(proof2.is_some());
    let proof2 = proof2.unwrap();
    assert!(proof2.verify(), "Proof should verify for row 2");

    // Get proof for non-existing row
    let proof_non_existing = trie.get_hexary_proof(999);
    assert!(proof_non_existing.is_none());
}

#[test]
fn test_row_trie_delete() {
    let mut trie = RowTrie::new();

    // Create test rows
    let row1 = DetermRow::from_values(vec![
        DetermValue::integer(1),
        DetermValue::text("Alice"),
        DetermValue::integer(30),
    ]);

    let row2 = DetermRow::from_values(vec![
        DetermValue::integer(2),
        DetermValue::text("Bob"),
        DetermValue::integer(25),
    ]);

    // Insert rows
    let (root1, _) = trie.insert(1, row1);
    let (root2, _) = trie.insert(2, row2);

    assert_eq!(trie.len(), 2);

    // Delete first row
    let (root3, diff) = trie.delete(1);

    // Verify the diff
    assert_eq!(diff.inserted.len(), 0);
    assert_eq!(diff.updated.len(), 0);
    assert_eq!(diff.deleted.len(), 1);
    assert_eq!(diff.deleted[0].0, 1);
    assert_eq!(diff.deleted[0].1, root1); // Should match the inserted row's hash

    // Root should change
    assert_ne!(root3, root2);

    // Row count should decrease
    assert_eq!(trie.len(), 1);

    // Deleted row should not be retrievable
    assert!(trie.get(1).is_none());
    assert!(trie.get_hash(1).is_none());

    // Other row should still be accessible
    assert!(trie.get(2).is_some());

    // Delete non-existing row
    let (root4, diff2) = trie.delete(999);

    // Should return no changes
    assert_eq!(diff2.inserted.len(), 0);
    assert_eq!(diff2.updated.len(), 0);
    assert_eq!(diff2.deleted.len(), 0);

    // Root should not change
    assert_eq!(root4, root3);
    assert_eq!(trie.len(), 1);
}
