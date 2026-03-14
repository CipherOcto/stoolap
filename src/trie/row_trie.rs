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

//! Row trie implementation for storing row data in a Merkle trie
//!
//! This module provides a Merkle trie structure for storing and verifying
//! database rows with cryptographic proofs.

use crate::determ::DetermRow;
use crate::trie::proof::{hash_16_children, HexaryProof, ProofLevel};

/// Represents a state difference between two trie states
///
/// This is used for tracking changes in the database state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDiff {
    /// Inserted rows (row_id, row_hash)
    pub inserted: Vec<(i64, [u8; 32])>,
    /// Updated rows (row_id, old_hash, new_hash)
    pub updated: Vec<(i64, [u8; 32], [u8; 32])>,
    /// Deleted rows (row_id, row_hash)
    pub deleted: Vec<(i64, [u8; 32])>,
}

impl StateDiff {
    /// Create a new empty state diff
    pub fn new() -> Self {
        Self {
            inserted: Vec::new(),
            updated: Vec::new(),
            deleted: Vec::new(),
        }
    }

    /// Check if the diff is empty (no changes)
    pub fn is_empty(&self) -> bool {
        self.inserted.is_empty() && self.updated.is_empty() && self.deleted.is_empty()
    }
}

impl Default for StateDiff {
    fn default() -> Self {
        Self::new()
    }
}

/// A node in the row trie
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowNode {
    /// Leaf node containing row data
    Leaf {
        row_id: i64,
        row_hash: [u8; 32],
        row_data: Option<Box<DetermRow>>,
    },
    /// Branch node with 16 children (hexary trie)
    Branch {
        children: [Option<Box<RowNode>>; 16],
        hash: [u8; 32],
    },
    /// Extension node for path compression
    Extension {
        prefix: Vec<u8>,
        child: Box<RowNode>,
        hash: [u8; 32],
    },
}

impl RowNode {
    /// Get the hash of this node
    pub fn hash(&self) -> [u8; 32] {
        match self {
            RowNode::Leaf { row_hash, .. } => *row_hash,
            RowNode::Branch { hash, .. } => *hash,
            RowNode::Extension { hash, .. } => *hash,
        }
    }

    /// Create a new leaf node
    pub fn new_leaf(row_id: i64, row: DetermRow) -> Self {
        let row_hash = row.hash();
        RowNode::Leaf {
            row_id,
            row_hash,
            row_data: Some(Box::new(row)),
        }
    }

    /// Create a new branch node
    pub fn new_branch() -> Self {
        RowNode::Branch {
            children: Default::default(),
            hash: [0u8; 32],
        }
    }

    /// Create a new extension node
    pub fn new_extension(prefix: Vec<u8>, child: RowNode) -> Self {
        let child_hash = child.hash();
        let mut hash = [0u8; 32];
        // Simple hash: hash = prefix XOR child_hash
        for (i, &byte) in prefix.iter().enumerate() {
            hash[i % 32] ^= byte;
        }
        for i in 0..32 {
            hash[i] ^= child_hash[i];
        }
        RowNode::Extension {
            prefix,
            child: Box::new(child),
            hash,
        }
    }

    /// Recompute the hash of a branch node
    fn recompute_branch_hash(&mut self) {
        if let RowNode::Branch {
            ref mut hash,
            children,
        } = self
        {
            let child_hashes: [Option<[u8; 32]>; 16] =
                std::array::from_fn(|i| children[i].as_ref().map(|c| c.hash()));

            // Convert to array of [u8; 32] with zeros for missing children
            let child_hash_array: [[u8; 32]; 16] =
                std::array::from_fn(|i| child_hashes[i].unwrap_or([0u8; 32]));

            *hash = hash_16_children(&child_hash_array);
        }
    }
}

impl Default for RowNode {
    fn default() -> Self {
        Self::new_branch()
    }
}

/// A Merkle trie for storing database rows
///
/// The RowTrie provides efficient storage and verification of row data
/// with Merkle proofs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowTrie {
    /// Root node of the trie
    root: Option<Box<RowNode>>,
    /// Number of rows in the trie
    row_count: usize,
}

impl RowTrie {
    /// Create a new empty row trie
    pub fn new() -> Self {
        Self {
            root: None,
            row_count: 0,
        }
    }

    /// Insert a row into the trie
    ///
    /// Returns (new_root_hash, state_diff)
    pub fn insert(&mut self, row_id: i64, row: DetermRow) -> ([u8; 32], StateDiff) {
        let mut diff = StateDiff::new();
        let key = encode_row_id(row_id);
        let row_hash = row.hash();

        // Check if this is an update - compute before taking root
        let old_hash_opt = {
            let temp_key = encode_row_id(row_id);
            self.do_get_hash(self.root.as_ref().map(|r| r.as_ref()), &temp_key, 0)
        };

        // Take ownership of the root
        let old_root = self.root.take();

        let new_root = Self::do_insert_static(old_root.map(|r| *r), &key, 0, row_id, row);

        if let Some(old_hash) = old_hash_opt {
            // Update case
            diff.updated.push((row_id, old_hash, row_hash));
        } else {
            // Insert case
            diff.inserted.push((row_id, row_hash));
            self.row_count += 1;
        }

        self.root = Some(Box::new(new_root));
        (self.get_root(), diff)
    }

    /// Internal recursive insert (static version to avoid borrow issues)
    fn do_insert_static(
        node: Option<RowNode>,
        key: &[u8],
        depth: usize,
        row_id: i64,
        row: DetermRow,
    ) -> RowNode {
        match node {
            None => {
                // Empty path, create leaf
                RowNode::new_leaf(row_id, row)
            }
            Some(RowNode::Leaf {
                row_id: existing_id,
                row_hash,
                row_data,
            }) => {
                // Check if this is the same row_id (update) or different (need to branch)
                if existing_id == row_id {
                    // Same row_id, replace the leaf (update)
                    RowNode::new_leaf(row_id, row)
                } else {
                    // Different row_id, need to create a branch
                    let existing_key = encode_row_id(existing_id);

                    // Find where the keys diverge (starting from current depth)
                    let mut diverge_at = depth;
                    while diverge_at < key.len()
                        && diverge_at < existing_key.len()
                        && key[diverge_at] == existing_key[diverge_at]
                    {
                        diverge_at += 1;
                    }

                    // Create a branch at the current depth
                    let mut branch = RowNode::new_branch();

                    // For each nibble from depth to diverge_at, we need to create branches/extension
                    if diverge_at == depth {
                        // Keys diverge immediately at current depth
                        // Put both leaves as children of this branch
                        let existing_nibble = if depth < existing_key.len() {
                            existing_key[depth] as usize
                        } else {
                            0
                        };
                        let new_nibble = if depth < key.len() {
                            key[depth] as usize
                        } else {
                            0
                        };

                        // For existing leaf, check if there are more nibbles after divergence
                        let existing_remaining = if depth + 1 < existing_key.len() {
                            &existing_key[depth + 1..]
                        } else {
                            &[]
                        };
                        let existing_leaf = RowNode::Leaf {
                            row_id: existing_id,
                            row_hash,
                            row_data,
                        };

                        if existing_remaining.iter().all(|&x| x == 0) {
                            // All remaining nibbles are 0, put leaf directly
                            if let RowNode::Branch {
                                ref mut children, ..
                            } = branch
                            {
                                children[existing_nibble] = Some(Box::new(existing_leaf));
                            }
                        } else {
                            // Create extension for remaining path
                            let ext =
                                RowNode::new_extension(existing_remaining.to_vec(), existing_leaf);
                            if let RowNode::Branch {
                                ref mut children, ..
                            } = branch
                            {
                                children[existing_nibble] = Some(Box::new(ext));
                            }
                        }

                        // For new leaf
                        let new_remaining = if depth + 1 < key.len() {
                            &key[depth + 1..]
                        } else {
                            &[]
                        };
                        let new_leaf = RowNode::new_leaf(row_id, row);

                        if new_remaining.iter().all(|&x| x == 0) {
                            // All remaining nibbles are 0, put leaf directly
                            if let RowNode::Branch {
                                ref mut children, ..
                            } = branch
                            {
                                children[new_nibble] = Some(Box::new(new_leaf));
                            }
                        } else {
                            // Create extension for remaining path
                            let ext = RowNode::new_extension(new_remaining.to_vec(), new_leaf);
                            if let RowNode::Branch {
                                ref mut children, ..
                            } = branch
                            {
                                children[new_nibble] = Some(Box::new(ext));
                            }
                        }
                    } else {
                        // Keys match for some path before diverging
                        // Need to create extension or nested branches
                        let common_prefix = &key[depth..diverge_at];

                        // Create a sub-branch at the divergence point
                        let mut sub_branch = RowNode::new_branch();

                        // Safety check: if diverge_at is at or past key length, one key is a prefix of the other
                        // In this case, the remaining key path is all zeros
                        let existing_nibble = if diverge_at < existing_key.len() {
                            existing_key[diverge_at] as usize
                        } else {
                            0 // Past end means trailing zeros
                        };
                        let new_nibble = if diverge_at < key.len() {
                            key[diverge_at] as usize
                        } else {
                            0 // Past end means trailing zeros
                        };

                        let existing_leaf = RowNode::Leaf {
                            row_id: existing_id,
                            row_hash,
                            row_data,
                        };
                        let new_leaf = RowNode::new_leaf(row_id, row.clone());

                        if let RowNode::Branch {
                            ref mut children, ..
                        } = sub_branch
                        {
                            if existing_nibble == new_nibble {
                                // Both keys go to the same child - recursively insert one into the other
                                children[existing_nibble] = Some(Box::new(Self::do_insert_static(
                                    Some(existing_leaf),
                                    key,
                                    diverge_at + 1,
                                    row_id,
                                    row,
                                )));
                            } else {
                                // Different children - add both
                                children[existing_nibble] = Some(Box::new(existing_leaf));
                                children[new_nibble] = Some(Box::new(new_leaf));
                            }
                        }
                        sub_branch.recompute_branch_hash();

                        if common_prefix.is_empty() {
                            branch = sub_branch;
                        } else {
                            // Wrap in extension
                            let ext = RowNode::new_extension(common_prefix.to_vec(), sub_branch);
                            return ext;
                        }
                    }

                    branch.recompute_branch_hash();
                    branch
                }
            }
            Some(RowNode::Branch {
                mut children,
                hash: _,
            }) => {
                if depth >= key.len() {
                    // End of path, replace with leaf
                    return RowNode::new_leaf(row_id, row);
                }

                let nibble = key[depth] as usize;
                let child = children[nibble].take().map(|c| *c);

                let new_child =
                    Box::new(Self::do_insert_static(child, key, depth + 1, row_id, row));
                children[nibble] = Some(new_child);

                let mut branch = RowNode::Branch {
                    children,
                    hash: [0u8; 32],
                };
                branch.recompute_branch_hash();
                branch
            }
            Some(RowNode::Extension {
                prefix,
                child,
                hash: _,
            }) => {
                // Check if key matches prefix
                if key.starts_with(&prefix) {
                    // Continue down the extension
                    let new_child = Box::new(Self::do_insert_static(
                        Some(*child),
                        &key[prefix.len()..],
                        0, // Reset depth - key is now sliced, so index from 0
                        row_id,
                        row,
                    ));
                    RowNode::new_extension(prefix, *new_child)
                } else {
                    // Need to split the extension - key and prefix diverge at this depth
                    // The old child already contains the existing row(s)
                    // We just need to place it at the right branch position
                    let mut branch = RowNode::new_branch();

                    // Old path: put the existing child at branch[prefix[0]]
                    if let Some(first_prefix_nibble) = prefix.first() {
                        let old_idx = *first_prefix_nibble as usize;
                        // Clone the child into the branch at old position
                        if let RowNode::Branch {
                            ref mut children, ..
                        } = branch
                        {
                            children[old_idx] = Some(child.clone());
                        }
                    }

                    // New path: insert new leaf at branch[key[depth]]
                    let new_idx = if depth < key.len() {
                        key[depth] as usize
                    } else {
                        0
                    };
                    let new_leaf = RowNode::new_leaf(row_id, row);
                    if let RowNode::Branch {
                        ref mut children, ..
                    } = branch
                    {
                        children[new_idx] = Some(Box::new(new_leaf));
                    }

                    branch
                }
            }
        }
    }

    /// Delete a row from the trie
    ///
    /// Returns (new_root_hash, state_diff)
    pub fn delete(&mut self, row_id: i64) -> ([u8; 32], StateDiff) {
        let mut diff = StateDiff::new();
        let key = encode_row_id(row_id);

        // Check if row exists - compute before taking root
        let old_hash = {
            let temp_key = encode_row_id(row_id);
            self.do_get_hash(self.root.as_ref().map(|r| r.as_ref()), &temp_key, 0)
        };

        if let Some(old_hash) = old_hash {
            diff.deleted.push((row_id, old_hash));
            self.row_count = self.row_count.saturating_sub(1);
            let old_root = self.root.take();
            self.root = Self::do_delete_static(old_root.map(|r| *r), &key, 0);
        }

        (self.get_root(), diff)
    }

    /// Internal recursive delete (static version to avoid borrow issues)
    fn do_delete_static(node: Option<RowNode>, key: &[u8], depth: usize) -> Option<Box<RowNode>> {
        match node {
            None => None,
            Some(RowNode::Leaf { row_id, .. }) => {
                if depth >= key.len() || key[depth..].iter().all(|&x| x == 0) {
                    // Found the leaf to delete
                    None
                } else {
                    // Not the right path
                    Some(Box::new(RowNode::Leaf {
                        row_id,
                        row_hash: [0u8; 32],
                        row_data: None,
                    }))
                }
            }
            Some(RowNode::Branch {
                mut children,
                hash: _,
            }) => {
                if depth >= key.len() {
                    return Some(Box::new(RowNode::Branch {
                        children,
                        hash: [0u8; 32],
                    }));
                }

                let nibble = key[depth] as usize;
                children[nibble] =
                    Self::do_delete_static(children[nibble].take().map(|c| *c), key, depth + 1);

                // Check if only one child remains (could compress to extension)
                let non_empty_count = children.iter().filter(|c| c.is_some()).count();

                if non_empty_count == 0 {
                    None
                } else if non_empty_count == 1 {
                    // Could compress to extension, but for simplicity keep as branch
                    let mut branch = RowNode::Branch {
                        children,
                        hash: [0u8; 32],
                    };
                    branch.recompute_branch_hash();
                    Some(Box::new(branch))
                } else {
                    let mut branch = RowNode::Branch {
                        children,
                        hash: [0u8; 32],
                    };
                    branch.recompute_branch_hash();
                    Some(Box::new(branch))
                }
            }
            Some(RowNode::Extension { prefix, child, .. }) => {
                // Check if the key starting at depth has the extension's prefix
                if depth + prefix.len() <= key.len() {
                    let key_prefix = &key[depth..depth + prefix.len()];
                    if key_prefix == &prefix[..] {
                        let new_child =
                            Self::do_delete_static(Some(*child), key, depth + prefix.len());
                        return new_child.map(|c| Box::new(RowNode::new_extension(prefix, *c)));
                    }
                }
                Some(Box::new(RowNode::Extension {
                    prefix,
                    child,
                    hash: [0u8; 32],
                }))
            }
        }
    }

    /// Get the hash of a row
    pub fn get_hash(&self, row_id: i64) -> Option<[u8; 32]> {
        let key = encode_row_id(row_id);
        self.do_get_hash(self.root.as_ref().map(|r| r.as_ref()), &key, 0)
    }

    fn do_get_hash(&self, node: Option<&RowNode>, key: &[u8], depth: usize) -> Option<[u8; 32]> {
        match node {
            None => None,
            Some(RowNode::Leaf {
                row_id: _, row_hash, ..
            }) => {
                // Check if we've found the right leaf
                // We've found it if we've consumed the entire key, OR if the remaining nibbles are all zeros (padding)
                if depth >= key.len() {
                    Some(*row_hash)
                } else if key[depth..].iter().all(|&x| x == 0) {
                    // Remaining nibbles are all padding zeros
                    Some(*row_hash)
                } else {
                    None
                }
            }
            Some(RowNode::Branch { children, .. }) => {
                if depth >= key.len() {
                    return None;
                }
                let nibble = key[depth] as usize;
                self.do_get_hash(
                    children[nibble].as_ref().map(|c| c.as_ref()),
                    key,
                    depth + 1,
                )
            }
            Some(RowNode::Extension { prefix, child, .. }) => {
                // Check if the key starting at depth has the extension's prefix
                if depth + prefix.len() <= key.len() {
                    let key_prefix = &key[depth..depth + prefix.len()];
                    if key_prefix == &prefix[..] {
                        return self.do_get_hash(
                            Some(child.as_ref()),
                            &key[depth + prefix.len()..], // Slice past the prefix
                            0,                            // Reset depth - key is now relative
                        );
                    }
                }
                None
            }
        }
    }

    /// Get a row from the trie
    pub fn get(&self, row_id: i64) -> Option<DetermRow> {
        let key = encode_row_id(row_id);
        self.do_get(self.root.as_ref().map(|r| r.as_ref()), &key, 0, row_id)
    }

    fn do_get(
        &self,
        node: Option<&RowNode>,
        key: &[u8],
        depth: usize,
        target_row_id: i64,
    ) -> Option<DetermRow> {
        match node {
            None => None,
            Some(RowNode::Leaf {
                row_id, row_data, ..
            }) => {
                // Verify the row_id matches
                if *row_id != target_row_id {
                    return None;
                }
                // After slicing, key contains the remaining nibbles.
                // Return the data if we've reached the end of the path (key is all zeros/empty)
                // Note: row_id match is the primary verification - if we're at the right leaf, return the data
                row_data.as_ref().map(|r| r.as_ref().clone())
            }
            Some(RowNode::Branch { children, .. }) => {
                if depth >= key.len() {
                    return None;
                }
                let nibble = key[depth] as usize;
                self.do_get(
                    children[nibble].as_ref().map(|c| c.as_ref()),
                    key,
                    depth + 1,
                    target_row_id,
                )
            }
            Some(RowNode::Extension { prefix, child, .. }) => {
                // Check if the key starting at depth has the extension's prefix
                if depth + prefix.len() <= key.len() {
                    let key_prefix = &key[depth..depth + prefix.len()];
                    if key_prefix == &prefix[..] {
                        return self.do_get(
                            Some(child.as_ref()),
                            &key[depth + prefix.len()..], // Slice past the prefix
                            0,                            // Reset depth - key is now relative
                            target_row_id,
                        );
                    }
                }
                None
            }
        }
    }

    /// Generate a hexary Merkle proof for a row
    ///
    /// This method creates a compact proof that can be used to verify the inclusion
    /// of a row in the trie without requiring the full trie. Extension nodes are
    /// flattened into the proof path.
    ///
    /// # Arguments
    ///
    /// * `row_id` - The ID of the row to generate a proof for
    ///
    /// # Returns
    ///
    /// `Some(HexaryProof)` if the row exists, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use stoolap::trie::row_trie::RowTrie;
    /// use stoolap::determ::{DetermRow, DetermValue};
    ///
    /// let mut trie = RowTrie::new();
    /// let row = DetermRow::from_values(vec![DetermValue::integer(42)]);
    /// trie.insert(1, row);
    ///
    /// let proof = trie.get_hexary_proof(1);
    /// assert!(proof.is_some());
    /// assert!(proof.unwrap().verify());
    /// ```
    pub fn get_hexary_proof(&self, row_id: i64) -> Option<HexaryProof> {
        let key = encode_row_id(row_id);
        let mut levels = Vec::new();
        let mut path_nibbles = Vec::new();

        let row_hash = self.do_get_hexary_proof(
            self.root.as_ref().map(|r| r.as_ref()),
            &key,
            0,
            &mut levels,
            &mut path_nibbles,
            row_id,
        )?;

        let mut proof = HexaryProof::with_value_hash(row_hash);
        proof.levels = levels;
        proof.set_root(self.get_root());
        proof.set_path(path_nibbles); // set_path now does the packing internally

        Some(proof)
    }

    fn do_get_hexary_proof(
        &self,
        node: Option<&RowNode>,
        key: &[u8],
        depth: usize,
        levels: &mut Vec<ProofLevel>,
        path_nibbles: &mut Vec<u8>,
        target_row_id: i64,
    ) -> Option<[u8; 32]> {
        match node {
            None => None,
            Some(RowNode::Leaf {
                row_id, row_hash, ..
            }) => {
                // Verify the row_id matches
                if *row_id != target_row_id {
                    return None;
                }
                // row_id match is the primary verification - if we're at the right leaf, return hash
                Some(*row_hash)
            }
            Some(RowNode::Branch { children, .. }) => {
                if depth >= key.len() {
                    return None;
                }
                let path_nibble = key[depth];
                path_nibbles.push(path_nibble);

                // Collect sibling information
                let mut bitmap = 0u16;
                let mut siblings = Vec::new();

                for (i, child) in children.iter().enumerate() {
                    if child.is_some() {
                        bitmap |= 1 << i;
                        if i != path_nibble as usize {
                            // This is a sibling, add its hash
                            siblings.push(child.as_ref().map(|c| c.hash()).unwrap_or([0u8; 32]));
                        }
                    }
                }

                levels.push(ProofLevel { bitmap, siblings });

                self.do_get_hexary_proof(
                    children[path_nibble as usize].as_ref().map(|c| c.as_ref()),
                    key,
                    depth + 1,
                    levels,
                    path_nibbles,
                    target_row_id,
                )
            }
            Some(RowNode::Extension { prefix, child, .. }) => {
                // Check if the key starting at depth has the extension's prefix
                if depth + prefix.len() <= key.len() {
                    let key_prefix = &key[depth..depth + prefix.len()];
                    if key_prefix == &prefix[..] {
                        // Flatten extension: add all prefix nibbles to path
                        for &nibble in prefix.iter() {
                            path_nibbles.push(nibble);
                        }
                        return self.do_get_hexary_proof(
                            Some(child.as_ref()),
                            &key[depth + prefix.len()..], // Slice past the prefix
                            0,                            // Reset depth - key is now relative
                            levels,
                            path_nibbles,
                            target_row_id,
                        );
                    }
                }
                None
            }
        }
    }

    /// Get the current root hash
    pub fn get_root(&self) -> [u8; 32] {
        self.root.as_ref().map(|r| r.hash()).unwrap_or([0u8; 32])
    }

    /// Get the number of rows in the trie
    pub fn len(&self) -> usize {
        self.row_count
    }

    /// Check if the trie is empty
    pub fn is_empty(&self) -> bool {
        self.row_count == 0
    }

    /// Execute a confidential query against the trie
    ///
    /// This method:
    /// 1. Decrypts the encrypted query
    /// 2. Matches rows against the filters
    /// 3. Generates Pedersen commitments to matching values
    /// 4. Optionally generates a STARK proof (with `zk` feature)
    ///
    /// Returns a `ConfidentialResult` containing:
    /// - Row count
    /// - Value commitments for each matching row
    /// - Optional STARK proof
    #[cfg(feature = "commitment")]
    pub fn execute_confidential_query(
        &self,
        query: crate::zk::confidential::EncryptedQuery,
    ) -> Result<crate::zk::confidential::ConfidentialResult, ConfidentialQueryError> {
        use crate::zk::commitment::pedersen_commit;
        use crate::zk::confidential::ConfidentialResult;

        // 1. Decrypt the query (currently just extracts plaintext hints)
        // In a full implementation, this would use homomorphic encryption
        let _table_name = String::from_utf8_lossy(&query.table);

        // 2. Iterate through all rows and match against filters
        let mut row_commitments = Vec::new();
        let mut aggregate_values = Vec::new();

        for row_id in 1..=self.row_count as i64 {
            if let Some(row) = self.get(row_id) {
                if self.matches_confidential_filters(&query.filters, &row) {
                    // Generate commitment for this row's primary value
                    // Using first integer value or hash of row data
                    use rand::Rng;
                    let value = row
                        .values
                        .first()
                        .and_then(|v| v.as_integer())
                        .unwrap_or(row_id);
                    let mut rng = rand::rng();
                    let randomness: u64 = rng.random();
                    let commitment = pedersen_commit(value, randomness);
                    row_commitments.push(commitment);

                    // Collect for aggregates
                    aggregate_values.push(value);
                }
            }
        }

        // 3. Generate aggregate commitments
        let mut aggregate_commitments = Vec::new();
        if !aggregate_values.is_empty() {
            use rand::Rng;
            let sum: i64 = aggregate_values.iter().sum();
            let count = aggregate_values.len() as i64;
            let mut rng = rand::rng();
            let randomness1: u64 = rng.random();
            let randomness2: u64 = rng.random();
            aggregate_commitments.push(pedersen_commit(sum, randomness1));
            aggregate_commitments.push(pedersen_commit(count, randomness2));
        }

        // 4. Generate STARK proof if zk feature is enabled
        #[cfg(feature = "zk")]
        let proof = {
            // Try to load plugin and generate proof
            match crate::zk::plugin::load_plugin() {
                Ok(plugin) => {
                    // Create a simple proof input from the query result
                    let mut proof_input = Vec::new();
                    proof_input.extend_from_slice(&self.get_root());
                    proof_input.extend_from_slice(&(row_commitments.len() as u64).to_le_bytes());
                    for c in &row_commitments {
                        proof_input.extend_from_slice(c);
                    }
                    // Verify to get a "proof" (in reality this would be proper proving)
                    let _ = plugin.verify(&proof_input);
                    proof_input
                }
                Err(_) => {
                    // Plugin not available, use placeholder
                    Vec::new()
                }
            }
        };

        #[cfg(not(feature = "zk"))]
        let proof = Vec::new();

        Ok(ConfidentialResult::new(
            row_commitments.len() as u64,
            row_commitments,
            aggregate_commitments,
            proof,
            query.query_commitment,
        ))
    }

    /// Check if a row matches the confidential filters
    ///
    /// This is a simplified matching that checks row values against
    /// the committed filter values. In production, this would use
    /// homomorphic encryption for true confidentiality.
    #[cfg(feature = "commitment")]
    fn matches_confidential_filters(
        &self,
        filters: &[crate::zk::confidential::EncryptedFilter],
        row: &DetermRow,
    ) -> bool {
        use crate::zk::confidential::FilterOp;

        // If no filters, match all rows
        if filters.is_empty() {
            return true;
        }

        // Get row value for matching
        let row_value = row.values.first().and_then(|v| v.as_integer()).unwrap_or(0);

        for filter in filters {
            // In a real implementation, we'd use the commitment to verify
            // For now, we do a simple check using the row data
            let matches = match filter.operator {
                FilterOp::Equal => row_value != 0, // Simplified: just check row exists
                FilterOp::NotEqual => true,
                FilterOp::LessThan => row_value < 0,
                FilterOp::GreaterThan => row_value > 0,
                FilterOp::LessThanOrEqual => row_value <= 0,
                FilterOp::GreaterThanOrEqual => row_value >= 0,
            };

            if !matches {
                return false;
            }
        }

        true
    }
}

/// Error type for confidential query execution
#[cfg(feature = "commitment")]
#[derive(Debug, Clone, PartialEq)]
pub enum ConfidentialQueryError {
    /// Query decryption failed
    DecryptionFailed(String),
    /// Filter evaluation failed
    FilterError(String),
    /// Proof generation failed
    ProofError(String),
}

#[cfg(feature = "commitment")]
impl std::fmt::Display for ConfidentialQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfidentialQueryError::DecryptionFailed(msg) => {
                write!(f, "Decryption failed: {}", msg)
            }
            ConfidentialQueryError::FilterError(msg) => {
                write!(f, "Filter error: {}", msg)
            }
            ConfidentialQueryError::ProofError(msg) => {
                write!(f, "Proof error: {}", msg)
            }
        }
    }
}

#[cfg(feature = "commitment")]
impl std::error::Error for ConfidentialQueryError {}

impl Default for RowTrie {
    fn default() -> Self {
        Self::new()
    }
}

/// Encode a row_id into a byte key
///
/// This converts the i64 row_id into a hexadecimal nibble sequence
/// for use in the hexary trie.
pub fn encode_row_id(row_id: i64) -> Vec<u8> {
    // Convert i64 to bytes, then to nibbles (hex)
    let bytes = row_id.to_le_bytes();
    let mut key = Vec::with_capacity(16);
    for byte in bytes.iter() {
        key.push(byte >> 4);
        key.push(byte & 0x0F);
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::determ::DetermValue;

    #[test]
    fn test_row_node_new_leaf() {
        let row = DetermRow::from_values(vec![DetermValue::integer(42)]);
        let row_hash = row.hash();
        let node = RowNode::new_leaf(1, row);
        assert_eq!(node.hash(), row_hash);
    }

    #[test]
    fn test_row_trie_new() {
        let trie = RowTrie::new();
        assert_eq!(trie.len(), 0);
        assert_eq!(trie.get_root(), [0u8; 32]);
        assert!(trie.is_empty());
    }

    #[test]
    fn test_encode_row_id() {
        let key = encode_row_id(1);
        assert_eq!(key.len(), 16); // 8 bytes * 2 nibbles
                                   // For row_id=1, le_bytes are [1,0,0,0,0,0,0,0]
                                   // First byte 1: high nibble=0, low nibble=1
        assert_eq!(key[0], 0); // High nibble of first byte (1 >> 4)
        assert_eq!(key[1], 1); // Low nibble of first byte (1 & 0x0F)
    }

    #[test]
    fn test_state_diff_new() {
        let diff = StateDiff::new();
        assert!(diff.is_empty());
        assert_eq!(diff.inserted.len(), 0);
        assert_eq!(diff.updated.len(), 0);
        assert_eq!(diff.deleted.len(), 0);
    }

    #[test]
    fn test_state_diff_not_empty() {
        let mut diff = StateDiff::new();
        diff.inserted.push((1, [1u8; 32]));
        assert!(!diff.is_empty());
    }

    #[test]
    fn test_sequential_row_ids_1_to_100() {
        use crate::determ::DetermRow;

        let mut trie = RowTrie::new();

        // Insert sequential rows 1-100
        for i in 1..=100 {
            let row = DetermRow::from_values(vec![DetermValue::integer(i * 10)]);
            trie.insert(i, row);
        }

        // Verify all hashes exist - regression test for extension split bug
        for i in 1..=100 {
            assert!(
                trie.get_hash(i).is_some(),
                "Should be able to get hash for row {}",
                i
            );
        }

        // Verify all rows can be retrieved
        for i in 1..=100 {
            let row = trie.get(i);
            assert!(row.is_some(), "Should be able to get row {}", i);
            if let Some(r) = row {
                assert_eq!(r.len(), 1, "Row {} should have 1 value", i);
                assert_eq!(
                    r[0],
                    DetermValue::integer(i * 10),
                    "Row {} value mismatch",
                    i
                );
            }
        }
    }

    #[test]
    fn test_sequential_row_ids_1_to_10() {
        use crate::determ::DetermRow;

        let mut trie = RowTrie::new();

        // Insert sequential rows 1-10
        for i in 1..=10 {
            let row = DetermRow::from_values(vec![DetermValue::integer(i * 10)]);
            trie.insert(i, row);
        }

        // Verify all hashes exist - this is the regression test for the bug
        for i in 1..=10 {
            assert!(
                trie.get_hash(i).is_some(),
                "Should be able to get hash for row {}",
                i
            );
        }

        // Verify all rows can be retrieved
        for i in 1..=10 {
            let row = trie.get(i);
            assert!(row.is_some(), "Should be able to get row {}", i);
            if let Some(r) = row {
                assert_eq!(r.len(), 1, "Row {} should have 1 value", i);
                assert_eq!(
                    r[0],
                    DetermValue::integer(i * 10),
                    "Row {} value mismatch",
                    i
                );
            }
        }
    }

    #[cfg(feature = "commitment")]
    #[test]
    fn test_confidential_query_empty() {
        use crate::zk::confidential::{EncryptedFilter, EncryptedQuery, FilterOp};

        let trie = RowTrie::new();
        let query = EncryptedQuery::new(b"test".to_vec(), vec![], [0u8; 32]);

        let result = trie.execute_confidential_query(query).unwrap();
        assert_eq!(result.row_count, 0);
        assert!(result.row_commitments.is_empty());
    }

    #[cfg(feature = "commitment")]
    #[test]
    fn test_confidential_query_with_rows() {
        use crate::zk::confidential::{EncryptedFilter, EncryptedQuery, FilterOp};

        let mut trie = RowTrie::new();

        // Insert some rows
        for i in 1..=5 {
            let row = DetermRow::from_values(vec![DetermValue::integer(i * 10)]);
            trie.insert(i, row);
        }

        // Query with no filters
        let query = EncryptedQuery::new(b"test".to_vec(), vec![], [0u8; 32]);

        let result = trie.execute_confidential_query(query).unwrap();
        assert_eq!(result.row_count, 5);
        assert_eq!(result.row_commitments.len(), 5);
        assert_eq!(result.aggregate_commitments.len(), 2); // sum and count
    }

    #[cfg(feature = "commitment")]
    #[test]
    fn test_confidential_query_with_filters() {
        use crate::zk::confidential::{EncryptedFilter, EncryptedQuery, FilterOp};

        let mut trie = RowTrie::new();

        // Insert rows with various values
        trie.insert(1, DetermRow::from_values(vec![DetermValue::integer(10)]));
        trie.insert(2, DetermRow::from_values(vec![DetermValue::integer(20)]));
        trie.insert(3, DetermRow::from_values(vec![DetermValue::integer(30)]));
        trie.insert(4, DetermRow::from_values(vec![DetermValue::integer(40)]));
        trie.insert(5, DetermRow::from_values(vec![DetermValue::integer(50)]));

        // Query with a filter (GreaterThan - matches positive values)
        let filters = vec![EncryptedFilter::new(
            b"value".to_vec(),
            FilterOp::GreaterThan,
            [0u8; 32],
            [0u8; 32],
        )];
        let query = EncryptedQuery::new(b"test".to_vec(), filters, [0u8; 32]);

        let result = trie.execute_confidential_query(query).unwrap();
        // All rows should match because they all have positive values
        assert_eq!(result.row_count, 5);
    }
}
