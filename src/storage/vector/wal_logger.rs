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

//! Vector WAL Logger - Integrated WAL for vector operations
//!
//! Provides WAL logging for vector insert/update/delete operations.

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

use crate::core::Result;

/// WAL writer for vector operations
pub struct VectorWalLogger {
    file: RwLock<Option<BufWriter<File>>>,
    path: RwLock<Option<std::path::PathBuf>>,
    enabled: AtomicBool,
}

impl VectorWalLogger {
    /// Create new WAL logger
    pub fn new() -> Self {
        Self {
            file: RwLock::new(None),
            path: RwLock::new(None),
            enabled: AtomicBool::new(false),
        }
    }

    /// Open WAL file for appending
    pub fn open(&self, path: &Path) -> Result<()> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;

        let writer = BufWriter::new(file);
        *self.file.write().unwrap() = Some(writer);
        *self.path.write().unwrap() = Some(path.to_path_buf());
        self.enabled.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Enable/disable WAL logging
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    /// Log vector insert
    pub fn log_insert(
        &self,
        table_name: &str,
        vector_id: i64,
        segment_id: u64,
        embedding: &[f32],
    ) -> Result<()> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Ok(());
        }

        let mut data = Vec::with_capacity(8 + 4 + embedding.len() * 4);
        data.extend_from_slice(&vector_id.to_le_bytes());
        data.extend_from_slice(&segment_id.to_le_bytes());
        data.extend_from_slice(&(embedding.len() as u32).to_le_bytes());
        for v in embedding {
            data.extend_from_slice(&v.to_le_bytes());
        }

        self.write_entry(b"VI", table_name, &data)
    }

    /// Log vector delete
    pub fn log_delete(&self, table_name: &str, vector_id: i64, segment_id: u64) -> Result<()> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Ok(());
        }

        let mut data = Vec::with_capacity(16);
        data.extend_from_slice(&vector_id.to_le_bytes());
        data.extend_from_slice(&segment_id.to_le_bytes());

        self.write_entry(b"VD", table_name, &data)
    }

    /// Log segment flush (persist to mmap)
    pub fn log_segment_flush(&self, table_name: &str, segment_id: u64) -> Result<()> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Ok(());
        }

        let mut data = Vec::with_capacity(8);
        data.extend_from_slice(&segment_id.to_le_bytes());

        self.write_entry(b"SF", table_name, &data)
    }

    fn write_entry(&self, op: &[u8], table_name: &str, data: &[u8]) -> Result<()> {
        let mut file_guard = self.file.write().unwrap();
        if let Some(ref mut file) = *file_guard {
            // Write: op(2) + table_len(2) + table_name + data_len(4) + data
            file.write_all(op)?;
            let table_bytes = table_name.as_bytes();
            file.write_all(&(table_bytes.len() as u16).to_le_bytes())?;
            file.write_all(table_bytes)?;
            file.write_all(&(data.len() as u32).to_le_bytes())?;
            file.write_all(data)?;
            file.flush()?;
        }
        Ok(())
    }

    /// Close WAL
    pub fn close(&self) {
        if let Ok(mut guard) = self.file.write() {
            if let Some(ref mut file) = *guard {
                let _ = file.flush();
            }
            *guard = None;
        }
        self.enabled.store(false, Ordering::SeqCst);
    }
}

impl Default for VectorWalLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// WAL entry for recovery
#[derive(Debug)]
pub struct VectorWalEntry {
    pub operation: VectorWalOp,
    pub table_name: String,
    pub vector_id: Option<i64>,
    pub segment_id: Option<u64>,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Copy)]
pub enum VectorWalOp {
    Insert,
    Delete,
    SegmentFlush,
}

impl VectorWalEntry {
    /// Parse WAL entry from bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 6 {
            return Err(crate::core::Error::Parse("WAL entry too short".to_string()));
        }

        let op = match &data[0..2] {
            b"VI" => VectorWalOp::Insert,
            b"VD" => VectorWalOp::Delete,
            b"SF" => VectorWalOp::SegmentFlush,
            _ => return Err(crate::core::Error::Parse("Unknown WAL op".to_string())),
        };

        let mut offset = 2;
        let table_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;

        let table_name = String::from_utf8(data[offset..offset + table_len].to_vec())
            .map_err(|e| crate::core::Error::Parse(e.to_string()))?;
        offset += table_len;

        let data_len = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;

        let payload = &data[offset..offset + data_len];

        match op {
            VectorWalOp::Insert => {
                if payload.len() < 12 {
                    return Err(crate::core::Error::Parse(
                        "Insert payload too short".to_string(),
                    ));
                }
                let vector_id = i64::from_le_bytes([
                    payload[0], payload[1], payload[2], payload[3], payload[4], payload[5],
                    payload[6], payload[7],
                ]);
                let segment_id = u64::from_le_bytes([
                    payload[8],
                    payload[9],
                    payload[10],
                    payload[11],
                    payload[12],
                    payload[13],
                    payload[14],
                    payload[15],
                ]);
                let dim = u32::from_le_bytes([payload[16], payload[17], payload[18], payload[19]])
                    as usize;
                let mut embedding = Vec::with_capacity(dim);
                for i in 0..dim {
                    let val = f32::from_le_bytes([
                        payload[20 + i * 4],
                        payload[21 + i * 4],
                        payload[22 + i * 4],
                        payload[23 + i * 4],
                    ]);
                    embedding.push(val);
                }
                Ok(Self {
                    operation: op,
                    table_name,
                    vector_id: Some(vector_id),
                    segment_id: Some(segment_id),
                    embedding: Some(embedding),
                })
            }
            VectorWalOp::Delete => {
                if payload.len() < 16 {
                    return Err(crate::core::Error::Parse(
                        "Delete payload too short".to_string(),
                    ));
                }
                let vector_id = i64::from_le_bytes([
                    payload[0], payload[1], payload[2], payload[3], payload[4], payload[5],
                    payload[6], payload[7],
                ]);
                let segment_id = u64::from_le_bytes([
                    payload[8],
                    payload[9],
                    payload[10],
                    payload[11],
                    payload[12],
                    payload[13],
                    payload[14],
                    payload[15],
                ]);
                Ok(Self {
                    operation: op,
                    table_name,
                    vector_id: Some(vector_id),
                    segment_id: Some(segment_id),
                    embedding: None,
                })
            }
            VectorWalOp::SegmentFlush => {
                if payload.len() < 8 {
                    return Err(crate::core::Error::Parse(
                        "SegmentFlush payload too short".to_string(),
                    ));
                }
                let segment_id = u64::from_le_bytes([
                    payload[0], payload[1], payload[2], payload[3], payload[4], payload[5],
                    payload[6], payload[7],
                ]);
                Ok(Self {
                    operation: op,
                    table_name,
                    vector_id: None,
                    segment_id: Some(segment_id),
                    embedding: None,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_wal_logger() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.wal");

        let logger = VectorWalLogger::new();
        logger.open(&path).unwrap();

        logger
            .log_insert("test_table", 1, 1, &[1.0, 2.0, 3.0])
            .unwrap();
        logger.log_delete("test_table", 1, 1).unwrap();
        logger.log_segment_flush("test_table", 1).unwrap();

        logger.close();
    }
}
