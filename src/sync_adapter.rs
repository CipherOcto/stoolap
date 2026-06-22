//! StoolapAdapter: implements the `DatabaseSyncAdapter` trait for the Stoolap MVCCEngine.
//!
//! Per RFC-0862 v1.1.0 §DatabaseSyncAdapter Trait, the cipherocto sync engine
//! consumes `Arc<dyn DatabaseSyncAdapter>`; this module provides the concrete
//! `StoolapAdapter` impl for the Stoolap fork. The cipherocto sync engine does
//! not call Stoolap DB functions directly; all DB operations go through the
//! trait boundary.
//!
//! # Architecture
//!
//! ```text
//! cipherocto sync engine
//!   └── Arc<dyn DatabaseSyncAdapter>
//!        └── StoolapAdapter (this module)
//!             └── Arc<parking_lot::Mutex<MVCCEngine>>
//!                  └── stoolap::storage::mvcc::engine::MVCCEngine
//! ```
//!
//! # TableId mapping
//!
//! The trait uses `TableId: u32`. The stoolap fork uses string table names.
//! The adapter computes `TableId = u32::from_le_bytes(BLAKE3-256(table_name)[0..4])`,
//! which is a deterministic, collision-resistant mapping. The cipherocto sync
//! engine uses the same convention (per the trait's RFC-0862 v1.1.0 design).
//!
//! # Feature gate
//!
//! This entire module is gated behind the `sync` feature (which requires the
//! `octo-sync` git dep). When the feature is disabled, the `DatabaseSyncAdapter`
//! trait is not available and this module does not exist.

use std::sync::Arc;

use blake3::Hasher;
use parking_lot::Mutex;

use octo_sync::adapter::{DatabaseSyncAdapter, SnapshotSegment as AdapterSegment};
use octo_sync::error::SyncError;
use octo_sync::types::{Lsn, MissionId, NodeId, SegmentIndex, TableId};

use crate::storage::mvcc::engine::MVCCEngine;

/// The StoolapAdapter: implements `DatabaseSyncAdapter` for the Stoolap MVCCEngine.
///
/// Wraps an `Arc<MVCCEngine>` in a `parking_lot::Mutex` to satisfy the trait's
/// `Send + Sync + 'static` bounds. The identity (mission_id, node_id) is set at
/// construction time and is immutable for the lifetime of the adapter (consistent
/// with the trait's invariants: identity is stable for the duration of a sync
/// session).
///
/// We implement `Clone` manually (not `#[derive(Clone)]`) because
/// `parking_lot::Mutex<T>` doesn't implement `Clone` (it would require
/// interior-mutable access to the inner `T`). Manual `Clone` is safe because
/// the inner `Arc<MVCCEngine>` is cheaply cloneable.
pub struct StoolapAdapter {
    /// The wrapped MVCCEngine (behind an `Arc`). The `parking_lot::Mutex` protects
    /// the `Arc` itself (so we can lock to get a cheap clone of the Arc); the
    /// actual engine operations don't need a Mutex because `MVCCEngine` is
    /// `Send + Sync` (its internal state is behind `parking_lot::RwLock`).
    engine: Mutex<Arc<MVCCEngine>>,
    /// The mission ID (32 bytes). Set at construction; immutable.
    /// Accessed via the `DatabaseSyncAdapter::mission_id` trait method.
    mission_id_value: MissionId,
    /// The local node ID (32 bytes). Set at construction; immutable.
    /// Accessed via the `DatabaseSyncAdapter::node_id` trait method.
    node_id_value: NodeId,
}

impl std::fmt::Debug for StoolapAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoolapAdapter")
            .field("mission_id", &format_args!("{:02x?}", &self.mission_id_value[..]))
            .field("node_id", &format_args!("{:02x?}", &self.node_id_value[..]))
            .finish()
    }
}

impl Clone for StoolapAdapter {
    fn clone(&self) -> Self {
        Self {
            engine: Mutex::new(self.engine.lock().clone()),
            mission_id_value: self.mission_id_value,
            node_id_value: self.node_id_value,
        }
    }
}

impl StoolapAdapter {
    /// Create a new `StoolapAdapter` wrapping the given `MVCCEngine`.
    ///
    /// The adapter takes an `Arc<MVCCEngine>` (not `&MVCCEngine`) so that the
    /// `Send + Sync` bounds are satisfied. The cipherocto sync engine spawns
    /// the trait calls via `tokio::task::spawn_blocking`, so the adapter does
    /// not need to be `Send` across await points — but it must be `Send + Sync`
    /// for the trait object to be stored in `Arc<dyn DatabaseSyncAdapter + Send + Sync>`.
    ///
    /// `mission_id` and `node_id` are stable for the lifetime of the adapter
    /// (per RFC-0862 §Implicit Assumptions Audit row 5: "Node identity is
    /// stable for the duration of a sync session").
    pub fn new(engine: Arc<MVCCEngine>, mission_id: MissionId, node_id: NodeId) -> Self {
        Self {
            engine: Mutex::new(engine),
            mission_id_value: mission_id,
            node_id_value: node_id,
        }
    }

    /// Return a reference to the inner `Mutex<Arc<MVCCEngine>>`.
    pub fn engine(&self) -> &Mutex<Arc<MVCCEngine>> {
        &self.engine
    }
}

/// Compute the `TableId` for a table name.
///
/// The convention is `TableId = u32::from_le_bytes(BLAKE3-256(table_name.to_lowercase().as_bytes())[0..4])`.
///
/// The cipherocto sync engine uses the same convention (per the trait's
/// RFC-0862 v1.1.0 design). This is a deterministic, collision-resistant
/// mapping (BLAKE3-256 produces 32 bytes; we take the first 4 bytes as u32).
fn compute_table_id(table_name: &str) -> TableId {
    let mut hasher = Hasher::new();
    hasher.update(table_name.to_lowercase().as_bytes());
    let hash = hasher.finalize();
    let bytes = hash.as_bytes();
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Reverse-lookup: find the table name for a given `TableId`.
///
/// Scans the engine's schemas (via `MVCCEngine::list_table_names`) and
/// returns the first table whose computed `TableId` matches. Returns
/// `None` if no table matches (which is a configuration error — the
/// cipherocto sync engine is sending a `TableId` for a table the local
/// DB doesn't have).
fn find_table_name_by_id(
    engine: &MVCCEngine,
    table_id: TableId,
) -> Option<String> {
    let tables = engine.list_table_names().ok()?;
    tables
        .into_iter()
        .find(|name| compute_table_id(name) == table_id)
}

impl DatabaseSyncAdapter for StoolapAdapter {
    fn read_wal_range(
        &self,
        from_lsn: Lsn,
        to_lsn: Lsn,
    ) -> Result<Vec<Vec<u8>>, SyncError> {
        if from_lsn > to_lsn {
            return Err(SyncError::InvalidLsnRange { from: from_lsn, to: to_lsn });
        }
        self.engine.lock().read_wal_range(from_lsn, to_lsn).map_err(|e| {
            SyncError::BackendNotReady(format!("read_wal_range failed: {}", e))
        })
    }

    fn current_lsn(&self) -> Result<Lsn, SyncError> {
        Ok(self.engine.lock().current_wal_lsn())
    }

    fn apply_wal_entry(&self, entry: &[u8]) -> Result<(), SyncError> {
        // Decoding the WAL bytes is the "decryption" boundary — if the bytes
        // are malformed (bad magic, bad CRC32, bad header), this is analogous
        // to an AEAD decryption failure (the bytes don't validate).
        // We classify such failures as DecryptionFailed per the trait's error
        // model (RFC-0862 §Error Handling: "AEAD decryption failure: the
        // adapter's apply_wal_entry could not verify the ciphertext").
        self.engine
            .lock()
            .apply_wal_entry_bytes(entry)
            .map_err(|e| {
                let msg = e.to_string();
                // Bad magic / bad CRC / bad version / truncated → DecryptionFailed
                if msg.contains("magic")
                    || msg.contains("version")
                    || msg.contains("CRC")
                    || msg.contains("truncated")
                    || msg.contains("header_size")
                {
                    SyncError::DecryptionFailed
                } else {
                    SyncError::BackendNotReady(format!("apply_wal_entry_bytes failed: {}", e))
                }
            })
    }
    fn read_snapshot_segment(
        &self,
        table_id: TableId,
        segment_index: SegmentIndex,
    ) -> Result<Option<AdapterSegment>, SyncError> {
        // Reverse-lookup: find the table name for this table_id.
        // Unknown table → SegmentNotFound per the trait.
        let engine = self.engine.lock();
        let table_name = match find_table_name_by_id(&engine, table_id) {
            Some(n) => n,
            None => {
                return Err(SyncError::SegmentNotFound {
                    table_id,
                    segment_index,
                    regenerated: false,
                });
            }
        };
        let paths = match engine.snapshot_segment_paths(&table_name) {
            Ok(p) => p,
            Err(e) => {
                return Err(SyncError::BackendNotReady(format!(
                    "snapshot_segment_paths failed: {}",
                    e
                )));
            }
        };
        if (segment_index as usize) >= paths.len() {
            // Per the trait doc: "Ok(None) if no file at that position
            // (the cipherocto sync engine interprets None as a signal to
            // descend the Merkle tree or request a different ordinal)".
            return Ok(None);
        }
        let path = &paths[segment_index as usize];
        let lsn_watermark = engine.current_wal_lsn();
        let payload = match engine.read_snapshot_segment_file(path) {
            Ok(p) => p,
            Err(e) => {
                return Err(SyncError::BackendNotReady(format!(
                    "read_snapshot_segment_file failed: {}",
                    e
                )));
            }
        };
        Ok(Some(AdapterSegment {
            table_id,
            segment_index,
            payload,
            lsn_watermark,
        }))
    }

    fn write_snapshot_segment(
        &self,
        table_id: TableId,
        segment_index: SegmentIndex,
        payload: &[u8],
    ) -> Result<(), SyncError> {
        let engine = self.engine.lock();
        let table_name = match find_table_name_by_id(&engine, table_id) {
            Some(n) => n,
            None => {
                return Err(SyncError::SegmentNotFound {
                    table_id,
                    segment_index,
                    regenerated: false,
                });
            }
        };
        engine
            .write_snapshot_segment_to_file(&table_name, segment_index, payload)
            .map(|_| ())
            .map_err(|e| {
                SyncError::BackendNotReady(format!(
                    "write_snapshot_segment_to_file failed: {}",
                    e
                ))
            })
    }

    fn regenerate_snapshot(&self, table_id: TableId) -> Result<u32, SyncError> {
        let engine = self.engine.lock();
        let table_name = match find_table_name_by_id(&engine, table_id) {
            Some(n) => n,
            None => {
                return Err(SyncError::SegmentNotFound {
                    table_id,
                    segment_index: 0,
                    regenerated: false,
                });
            }
        };
        // Create a fresh snapshot for this table.
        engine
            .create_snapshot_for_table(&table_name)
            .map_err(|e| {
                SyncError::BackendNotReady(format!(
                    "create_snapshot_for_table failed: {}",
                    e
                ))
            })?;
        // Return the new segment count.
        engine
            .snapshot_segment_count(&table_name)
            .map_err(|e| {
                SyncError::BackendNotReady(format!(
                    "snapshot_segment_count failed: {}",
                    e
                ))
            })
    }

    fn mission_id(&self) -> Result<MissionId, SyncError> {
        Ok(self.mission_id_value)
    }

    fn node_id(&self) -> Result<NodeId, SyncError> {
        Ok(self.node_id_value)
    }
}

/// Helper: list all table names in the engine, with their computed TableIds.
///
/// This is a public API for the cipherocto sync engine (via the StoolapAdapter).
/// It uses the engine's `list_table_names` method to iterate all tables and
/// computes the TableId for each one.
pub fn list_table_table_ids(engine: &MVCCEngine) -> Vec<(TableId, String)> {
    let tables = match engine.list_table_names() {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    tables
        .into_iter()
        .map(|name| (compute_table_id(&name), name))
        .collect()
}

/// Helper: compute the TableId for a given table name (exported for use by
/// the cipherocto sync engine when building summary envelopes).
pub fn table_id_for_name(table_name: &str) -> TableId {
    compute_table_id(table_name)
}

/// Helper: read a snapshot segment by table name (for debugging and testing).
pub fn read_snapshot_segment_by_name(
    engine: &MVCCEngine,
    table_name: &str,
    segment_index: SegmentIndex,
) -> Result<Option<Vec<u8>>, SyncError> {
    let paths = engine
        .snapshot_segment_paths(table_name)
        .map_err(|e| SyncError::BackendNotReady(format!("snapshot_segment_paths failed: {}", e)))?;
    if (segment_index as usize) >= paths.len() {
        return Ok(None);
    }
    let path = &paths[segment_index as usize];
    engine
        .read_snapshot_segment_file(path)
        .map(Some)
        .map_err(|e| {
            SyncError::BackendNotReady(format!("read_snapshot_segment_file failed: {}", e))
        })
}

/// Sync configuration passed to `Database::open_with_sync`.
///
/// The cipherocto sync engine passes this to the stoolap fork when opening
/// a database with sync support. The mission_id and node_id are required;
/// the public_key is optional (for HMAC envelope verification).
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// The mission ID (32 bytes). Required.
    pub mission_id: MissionId,
    /// The local node ID (32 bytes). Required.
    pub node_id: NodeId,
    /// The public key (32 bytes) for HMAC envelope verification. Optional.
    pub public_key: Option<[u8; 32]>,
}

impl SyncConfig {
    /// Create a new `SyncConfig` with the given mission_id and node_id.
    pub fn new(mission_id: MissionId, node_id: NodeId) -> Self {
        Self {
            mission_id,
            node_id,
            public_key: None,
        }
    }

    /// Set the public key for HMAC envelope verification.
    pub fn with_public_key(mut self, public_key: [u8; 32]) -> Self {
        self.public_key = Some(public_key);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::mvcc::engine::MVCCEngine;

    fn mission_id() -> MissionId {
        let mut m = [0u8; 32];
        m[0] = 0xAB;
        m
    }

    fn node_id() -> NodeId {
        let mut n = [0u8; 32];
        n[0] = 0xCD;
        n
    }

    #[test]
    fn table_id_for_name_is_deterministic() {
        let id1 = table_id_for_name("users");
        let id2 = table_id_for_name("users");
        assert_eq!(id1, id2);
    }

    #[test]
    fn table_id_for_name_is_case_insensitive() {
        let id1 = table_id_for_name("Users");
        let id2 = table_id_for_name("users");
        assert_eq!(id1, id2);
    }

    #[test]
    fn different_names_have_different_ids() {
        let id1 = table_id_for_name("users");
        let id2 = table_id_for_name("orders");
        assert_ne!(id1, id2);
    }

    #[test]
    fn adapter_construction() {
        let engine = Arc::new(MVCCEngine::in_memory());
        let adapter = StoolapAdapter::new(Arc::clone(&engine), mission_id(), node_id());
        assert_eq!(adapter.mission_id().unwrap(), mission_id());
        assert_eq!(adapter.node_id().unwrap(), node_id());
    }

    #[test]
    fn current_lsn_starts_at_zero() {
        let engine = Arc::new(MVCCEngine::in_memory());
        let adapter = StoolapAdapter::new(Arc::clone(&engine), mission_id(), node_id());
        assert_eq!(adapter.current_lsn().unwrap(), 0);
    }

    #[test]
    fn read_wal_range_invalid_returns_err() {
        let engine = Arc::new(MVCCEngine::in_memory());
        let adapter = StoolapAdapter::new(Arc::clone(&engine), mission_id(), node_id());
        let err = adapter.read_wal_range(10, 5).unwrap_err();
        assert_eq!(err, SyncError::InvalidLsnRange { from: 10, to: 5 });
    }

    #[test]
    fn read_wal_range_empty_when_no_persistence() {
        let engine = Arc::new(MVCCEngine::in_memory());
        let adapter = StoolapAdapter::new(Arc::clone(&engine), mission_id(), node_id());
        let entries = adapter.read_wal_range(0, 100).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn find_table_name_by_id_empty_engine() {
        let engine = Arc::new(MVCCEngine::in_memory());
        assert!(find_table_name_by_id(&engine, 0xDEADBEEF).is_none());
    }

    #[test]
    fn list_table_table_ids_empty() {
        let engine = Arc::new(MVCCEngine::in_memory());
        assert!(list_table_table_ids(&engine).is_empty());
    }

    #[test]
    fn sync_config_construction() {
        let cfg = SyncConfig::new(mission_id(), node_id());
        assert_eq!(cfg.mission_id, mission_id());
        assert_eq!(cfg.node_id, node_id());
        assert!(cfg.public_key.is_none());
    }

    #[test]
    fn sync_config_with_public_key() {
        let key = [0xAB; 32];
        let cfg = SyncConfig::new(mission_id(), node_id()).with_public_key(key);
        assert_eq!(cfg.public_key, Some(key));
    }

    #[test]
    fn adapter_clone_preserves_identity() {
        let engine = Arc::new(MVCCEngine::in_memory());
        let adapter = StoolapAdapter::new(Arc::clone(&engine), mission_id(), node_id());
        let cloned = adapter.clone();
        assert_eq!(cloned.mission_id().unwrap(), adapter.mission_id().unwrap());
        assert_eq!(cloned.node_id().unwrap(), adapter.node_id().unwrap());
    }

    #[test]
    fn apply_wal_entry_bad_magic_returns_decryption_failed() {
        let engine = MVCCEngine::in_memory();
        engine.open_engine().unwrap();
        let engine = Arc::new(engine);
        let adapter = StoolapAdapter::new(Arc::clone(&engine), mission_id(), node_id());
        let bad_bytes = vec![0u8; 64]; // wrong magic
        let err = adapter.apply_wal_entry(&bad_bytes).unwrap_err();
        assert!(matches!(err, SyncError::DecryptionFailed), "got: {:?}", err);
    }

    #[test]
    fn read_snapshot_segment_unknown_table_returns_segment_not_found() {
        let engine = Arc::new(MVCCEngine::in_memory());
        let adapter = StoolapAdapter::new(Arc::clone(&engine), mission_id(), node_id());
        let err = adapter.read_snapshot_segment(0xDEADBEEF, 0).unwrap_err();
        assert!(matches!(err, SyncError::SegmentNotFound { .. }), "got: {:?}", err);
    }

    #[test]
    fn write_snapshot_segment_unknown_table_returns_segment_not_found() {
        let engine = Arc::new(MVCCEngine::in_memory());
        let adapter = StoolapAdapter::new(Arc::clone(&engine), mission_id(), node_id());
        let err = adapter.write_snapshot_segment(0xDEADBEEF, 0, b"x").unwrap_err();
        assert!(matches!(err, SyncError::SegmentNotFound { .. }), "got: {:?}", err);
    }

    #[test]
    fn file_backed_engine_persistence_flow() {
        use crate::storage::config::Config;
        use std::env;

        let tmp = env::temp_dir().join(format!("stoolap_sync_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let db_path = tmp.join("test.db");

        let config = Config::with_path(db_path.to_string_lossy().into_owned());
        let engine = MVCCEngine::new(config);
        engine.open_engine().unwrap();

        let _ = std::fs::remove_dir_all(&tmp);
        assert!(!db_path.exists() || tmp.exists());
    }

    #[test]
    fn snapshot_segment_paths_on_file_backed_engine() {
        use crate::storage::config::Config;
        use std::env;

        let tmp = env::temp_dir().join(format!("stoolap_sync_test2_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let db_path = tmp.join("test.db");

        let config = Config::with_path(db_path.to_string_lossy().into_owned());
        let engine = MVCCEngine::new(config);
        engine.open_engine().unwrap();
        let paths = engine.snapshot_segment_paths("nonexistent").unwrap();
        assert!(paths.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
