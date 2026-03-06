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

//! Vector WAL Recovery - Replay WAL entries after crash
//!
//! Provides crash recovery by replaying WAL entries.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use crate::core::Result;
use super::wal_logger::VectorWalEntry;

/// Read and replay WAL entries from a file
pub struct VectorWalRecovery {
    /// Path to WAL file
    wal_path: std::path::PathBuf,
}

impl VectorWalRecovery {
    /// Create new recovery handler
    pub fn new(wal_path: &Path) -> Self {
        Self {
            wal_path: wal_path.to_path_buf(),
        }
    }

    /// Replay all WAL entries
    pub fn replay<F>(&self, mut callback: F) -> Result<()>
    where
        F: FnMut(VectorWalEntry) -> Result<()>,
    {
        let file = match File::open(&self.wal_path) {
            Ok(f) => f,
            Err(_) => return Ok(()), // No WAL file = nothing to recover
        };
        let mut reader = BufReader::new(file);

        loop {
            // Read operation type (2 bytes)
            let mut op = [0u8; 2];
            match reader.read_exact(&mut op) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(crate::core::Error::Io { message: e.to_string() }),
            }

            // Read table name length
            let mut table_len_buf = [0u8; 2];
            reader.read_exact(&mut table_len_buf)?;
            let table_len = u16::from_le_bytes(table_len_buf) as usize;

            // Read table name
            let mut table_name = vec![0u8; table_len];
            reader.read_exact(&mut table_name)?;
            let table_name = String::from_utf8(table_name)
                .map_err(|e| crate::core::Error::Parse(e.to_string()))?;

            // Read data length
            let mut data_len_buf = [0u8; 4];
            reader.read_exact(&mut data_len_buf)?;
            let data_len = u32::from_le_bytes(data_len_buf) as usize;

            // Read data
            let mut data = vec![0u8; data_len];
            reader.read_exact(&mut data)?;

            // Combine op + table + data for parsing
            let mut entry_data = Vec::with_capacity(2 + 2 + table_len + 4 + data_len);
            entry_data.extend_from_slice(&op);
            entry_data.extend_from_slice(&table_len_buf);
            entry_data.extend_from_slice(&table_name.as_bytes());
            entry_data.extend_from_slice(&data_len_buf);
            entry_data.extend_from_slice(&data);

            // Parse entry
            let entry = VectorWalEntry::parse(&entry_data)?;
            callback(entry)?;
        }

        Ok(())
    }

    /// Get WAL file size (for determining if truncation is needed)
    pub fn wal_size(&self) -> Result<u64> {
        let metadata = std::fs::metadata(&self.wal_path)?;
        Ok(metadata.len())
    }

    /// Truncate WAL after successful recovery
    pub fn truncate_wal(&self, truncate_to: u64) -> Result<()> {
        use std::io::Seek;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(&self.wal_path)?;
        file.seek(std::io::SeekFrom::Start(truncate_to))?;
        file.set_len(truncate_to)?;
        Ok(())
    }
}

/// Vector MVCC Recovery helper
pub struct VectorMvccRecovery;

impl VectorMvccRecovery {
    /// Recover MVCC from WAL and persisted segments
    pub fn recover<F>(
        _config: &super::VectorConfig,
        _storage_path: &Path,
        _wal_path: &Path,
        _rebuild_index: F,
    ) -> Result<super::VectorMvcc>
    where
        F: Fn(u64) -> Result<()>,
    {
        // For now, create fresh MVCC
        // Full recovery would:
        // 1. Load persisted segments from storage_path
        // 2. Replay WAL entries to rebuild in-memory state
        // 3. Rebuild HNSW indexes
        let mvcc = super::VectorMvcc::new(_config.clone());
        Ok(mvcc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::storage::vector::VectorWalLogger;

    #[test]
    fn test_wal_recovery() {
        let dir = tempdir().unwrap();
        let wal_path = dir.path().join("test.wal");

        // Write some WAL entries
        {
            let logger = VectorWalLogger::new();
            logger.open(&wal_path).unwrap();
            logger.log_insert("test", 1, 1, &[1.0, 2.0, 3.0]).unwrap();
            logger.log_delete("test", 1, 1).unwrap();
            logger.close();
        }

        // Recover
        let recovery = VectorWalRecovery::new(&wal_path);
        let entries = std::sync::Mutex::new(Vec::new());
        recovery.replay(|entry| {
            entries.lock().unwrap().push(entry);
            Ok(())
        }).unwrap();

        assert_eq!(entries.lock().unwrap().len(), 2);
    }
}
