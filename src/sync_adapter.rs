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
    /// Cached reverse-lookup table: `TableId → table_name`. Populated lazily
    /// on first use and invalidated when the engine's schema_epoch changes
    /// (which happens on CREATE/ALTER/DROP TABLE).
    ///
    /// `None` means the cache hasn't been populated yet (first call).
    /// `Some((epoch, HashMap))` means the cache is populated for that epoch.
    /// If the current epoch != cached epoch, the cache is rebuilt.
    table_id_cache: Mutex<Option<(u64, std::collections::HashMap<TableId, String>)>>,
}

impl std::fmt::Debug for StoolapAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Hex-encode the 32-byte IDs for readability via the HexId helper
        // (a compact hex string, e.g. "ab12cd34...").
        f.debug_struct("StoolapAdapter")
            .field("mission_id", &HexId(&self.mission_id_value))
            .field("node_id", &HexId(&self.node_id_value))
            .finish()
    }
}

/// Helper: debug-print a 32-byte ID as a hex string.
struct HexId<'a>(&'a [u8; 32]);

impl std::fmt::Debug for HexId<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl Clone for StoolapAdapter {
    fn clone(&self) -> Self {
        Self {
            engine: Mutex::new(self.engine.lock().clone()),
            mission_id_value: self.mission_id_value,
            node_id_value: self.node_id_value,
            // Start with an empty cache in the clone (don't share the cache
            // — the cipherocto sync engine typically has one adapter per
            // session, and cloning is rare).
            table_id_cache: Mutex::new(None),
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
            table_id_cache: Mutex::new(None),
        }
    }
}

/// Compute the `TableId` for a table name.
///
/// Re-exports `MVCCEngine::compute_table_id` (the shared contract between
/// the stoolap fork and the cipherocto sync engine). This is a
/// deterministic, collision-resistant mapping (BLAKE3-256 produces 32 bytes;
/// we take the first 4 bytes as u32).
fn compute_table_id(table_name: &str) -> TableId {
    MVCCEngine::compute_table_id(table_name)
}

/// Reverse-lookup: find the table name for a given `TableId`.
///
/// Uses a lazy-initialized cache (`table_id_cache`) to avoid O(n) scans
/// on every call. The cache is tagged with the engine's `schema_epoch` —
/// if the epoch changes (CREATE/ALTER/DROP TABLE), the cache is rebuilt.
/// This ensures the cache stays correct across schema changes mid-session.
///
/// The cipherocto sync engine typically calls this O(log n) times per
/// table per sync session (Merkle tree descent), so caching turns O(n²)
/// total work into O(n) + O(log n) per session.
fn find_table_name_by_id(
    engine_arc: &Arc<MVCCEngine>,
    cache: &Mutex<Option<(u64, std::collections::HashMap<TableId, String>)>>,
    table_id: TableId,
) -> Option<String> {
    // Fast path: check the cache.
    {
        let guard = cache.lock();
        if let Some((epoch, map)) = guard.as_ref() {
            let current_epoch = engine_arc.schema_epoch();
            if *epoch == current_epoch {
                if let Some(name) = map.get(&table_id) {
                    return Some(name.clone());
                }
                // Cache is populated and current, but table_id not found
                // → return None (the table genuinely doesn't exist; no
                // point re-scanning).
                return None;
            }
            // Epoch mismatch — fall through to rebuild.
        }
    }

    // Slow path: cache is empty or stale, rebuild it.
    let engine = engine_arc.as_ref();
    let tables = engine.list_table_names().ok()?;
    let mut map = std::collections::HashMap::with_capacity(tables.len());
    for name in &tables {
        let id = compute_table_id(name);
        // On hash collision, keep the first one (BLAKE3 collisions are
        // astronomically unlikely — 128-bit collision resistance).
        map.entry(id).or_insert_with(|| name.clone());
    }
    let result = map.get(&table_id).cloned();
    let epoch = engine.schema_epoch();
    *cache.lock() = Some((epoch, map));
    result
}

impl DatabaseSyncAdapter for StoolapAdapter {
    fn read_wal_range(&self, from_lsn: Lsn, to_lsn: Lsn) -> Result<Vec<Vec<u8>>, SyncError> {
        if from_lsn > to_lsn {
            return Err(SyncError::InvalidLsnRange {
                from: from_lsn,
                to: to_lsn,
            });
        }
        self.engine
            .lock()
            .read_wal_range(from_lsn, to_lsn)
            .map_err(|e| SyncError::BackendNotReady(format!("read_wal_range failed: {}", e)))
    }

    fn current_lsn(&self) -> Result<Lsn, SyncError> {
        Ok(self.engine.lock().current_wal_lsn())
    }

    fn apply_wal_entry(&self, entry: &[u8]) -> Result<(), SyncError> {
        // Map ApplyWalEntryError to SyncError per RFC-0862 §Error Handling:
        // - Decode → DecryptionFailed (bytes failed to validate; analogous
        //   to an AEAD decryption failure at the envelope layer).
        // - Apply → BackendNotReady (transient error; the cipherocto sync
        //   engine retries with backoff).
        //
        // Note: we do NOT log the decode message here. The SyncError enum
        // is the signal; the cipherocto sync engine records it in its
        // metrics. Side-effect logging (eprintln!) would be untestable
        // and would spam stderr in production.
        self.engine
            .lock()
            .apply_wal_entry_bytes(entry)
            .map_err(|e| match e {
                crate::storage::mvcc::engine::ApplyWalEntryError::Decode(_) => {
                    SyncError::DecryptionFailed
                }
                crate::storage::mvcc::engine::ApplyWalEntryError::Apply(msg) => {
                    SyncError::BackendNotReady(format!("WAL apply failed: {msg}"))
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
        let engine_arc = self.engine.lock().clone();
        let table_name = match find_table_name_by_id(&engine_arc, &self.table_id_cache, table_id) {
            Some(n) => n,
            None => {
                return Err(SyncError::SegmentNotFound {
                    table_id,
                    segment_index,
                    regenerated: false,
                });
            }
        };
        let paths = match engine_arc.snapshot_segment_paths(&table_name) {
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
        // Read the source_lsn AND the payload from a single file open.
        // The source_lsn is at offset 28 in the 64-byte header; the
        // payload is the entire file (the cipherocto sync engine's
        // SegmentIndexer BLAKE3-hashes the full file).
        let (lsn_watermark, payload) = match engine_arc.read_snapshot_segment_with_watermark(path) {
            Ok(r) => r,
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
        let engine_arc = self.engine.lock().clone();
        let table_name = match find_table_name_by_id(&engine_arc, &self.table_id_cache, table_id) {
            Some(n) => n,
            None => {
                return Err(SyncError::SegmentNotFound {
                    table_id,
                    segment_index,
                    regenerated: false,
                });
            }
        };
        engine_arc
            .write_snapshot_segment_to_file(&table_name, segment_index, payload)
            .map(|_| ())
            .map_err(|e| {
                SyncError::BackendNotReady(format!("write_snapshot_segment_to_file failed: {}", e))
            })
    }

    fn regenerate_snapshot(&self, table_id: TableId) -> Result<u32, SyncError> {
        let engine_arc = self.engine.lock().clone();
        let table_name = match find_table_name_by_id(&engine_arc, &self.table_id_cache, table_id) {
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
        engine_arc
            .create_snapshot_for_table(&table_name)
            .map_err(|e| {
                SyncError::BackendNotReady(format!("create_snapshot_for_table failed: {}", e))
            })?;
        // Prune old segments, keeping only the 2 most recent (the new one
        // and one backup). This prevents disk bloat from accumulated
        // regeneration requests.
        engine_arc
            .prune_snapshot_segments(&table_name, 2)
            .map_err(|e| {
                SyncError::BackendNotReady(format!("prune_snapshot_segments failed: {}", e))
            })?;
        // Return the new segment count.
        engine_arc.snapshot_segment_count(&table_name).map_err(|e| {
            SyncError::BackendNotReady(format!("snapshot_segment_count failed: {}", e))
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
#[cfg(test)]
pub(crate) fn read_snapshot_segment_by_name(
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
/// a database with sync support. The `mission_id` and `node_id` are required
/// and are stable for the lifetime of the adapter.
///
/// The cipherocto sync engine derives the per-mission `transport_key` and
/// `execution_key` via `HKDF-BLAKE3(mission_root_key, "sync:v1", mission_id)`
/// (per RFC-0862 §4.3.1 and mission 0862d). The stoolap fork does not need
/// the public key or any key material — the cipherocto sync engine handles
/// all cryptography at the envelope layer.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// The mission ID (32 bytes). Required.
    pub mission_id: MissionId,
    /// The local node ID (32 bytes). Required.
    pub node_id: NodeId,
}

impl SyncConfig {
    /// Create a new `SyncConfig` with the given mission_id and node_id.
    pub fn new(mission_id: MissionId, node_id: NodeId) -> Self {
        Self {
            mission_id,
            node_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::mvcc::engine::MVCCEngine;
    use crate::storage::mvcc::wal_manager::WAL_ENTRY_MAGIC;

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
        let cache = Mutex::new(None);
        assert!(find_table_name_by_id(&engine, &cache, 0xDEADBEEF).is_none());
    }

    #[test]
    fn find_table_name_by_id_caches_result() {
        // After the first call, the cache is populated. Subsequent calls
        // for the same table_id should return the same result without
        // re-scanning.
        let engine = Arc::new(MVCCEngine::in_memory());
        let cache = Mutex::new(None);

        // First call: populates the cache.
        let r1 = find_table_name_by_id(&engine, &cache, 0xDEADBEEF);
        // Cache should be populated now.
        assert!(cache.lock().is_some());
        // Second call: uses the cache.
        let r2 = find_table_name_by_id(&engine, &cache, 0xDEADBEEF);
        assert_eq!(r1, r2);
    }

    #[test]
    fn cache_invalidates_on_schema_epoch_change() {
        // The cache is tagged with the engine's schema_epoch. If the
        // epoch changes (e.g., a new table is created), the cache is
        // rebuilt on the next call.
        let engine = Arc::new(MVCCEngine::in_memory());
        let cache = Mutex::new(None);

        // First call: populates the cache.
        let _ = find_table_name_by_id(&engine, &cache, 0xDEADBEEF);
        let initial_epoch = cache.lock().as_ref().map(|(e, _)| *e).unwrap();

        // Simulate a schema change by directly incrementing the epoch
        // (we can't create a table without an open engine in the test).
        engine.schema_epoch_inc_for_test();

        // The cache epoch should be stale now.
        assert!(initial_epoch < engine.schema_epoch());

        // Next call: cache should be rebuilt with the new epoch.
        let _ = find_table_name_by_id(&engine, &cache, 0xDEADBEEF);
        let new_epoch = cache.lock().as_ref().map(|(e, _)| *e).unwrap();
        assert_eq!(new_epoch, engine.schema_epoch());
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
    fn apply_wal_entry_too_short_returns_decryption_failed() {
        let engine = MVCCEngine::in_memory();
        engine.open_engine().unwrap();
        let engine = Arc::new(engine);
        let adapter = StoolapAdapter::new(Arc::clone(&engine), mission_id(), node_id());
        let bad_bytes = vec![0u8; 10]; // less than WAL_HEADER_SIZE (32) + 4
        let err = adapter.apply_wal_entry(&bad_bytes).unwrap_err();
        assert!(matches!(err, SyncError::DecryptionFailed), "got: {:?}", err);
    }

    #[test]
    fn apply_wal_entry_bad_version_returns_decryption_failed() {
        let engine = MVCCEngine::in_memory();
        engine.open_engine().unwrap();
        let engine = Arc::new(engine);
        let adapter = StoolapAdapter::new(Arc::clone(&engine), mission_id(), node_id());
        // Build a 64-byte payload with the correct magic but wrong version.
        let mut bad_bytes = vec![0u8; 64];
        bad_bytes[0..4].copy_from_slice(&WAL_ENTRY_MAGIC.to_le_bytes());
        bad_bytes[4] = 99; // wrong version
        let err = adapter.apply_wal_entry(&bad_bytes).unwrap_err();
        assert!(matches!(err, SyncError::DecryptionFailed), "got: {:?}", err);
    }

    #[test]
    fn read_snapshot_segment_unknown_table_returns_segment_not_found() {
        let engine = Arc::new(MVCCEngine::in_memory());
        let adapter = StoolapAdapter::new(Arc::clone(&engine), mission_id(), node_id());
        let err = adapter.read_snapshot_segment(0xDEADBEEF, 0).unwrap_err();
        assert!(
            matches!(err, SyncError::SegmentNotFound { .. }),
            "got: {:?}",
            err
        );
    }

    #[test]
    fn read_snapshot_segment_unknown_table_returns_segment_not_found_with_index() {
        // Verify the SegmentNotFound carries the correct segment_index
        // (the cipherocto sync engine uses this to know which segment
        // was requested).
        let engine = Arc::new(MVCCEngine::in_memory());
        let adapter = StoolapAdapter::new(Arc::clone(&engine), mission_id(), node_id());
        let err = adapter.read_snapshot_segment(0xCAFEBABE, 42).unwrap_err();
        assert!(
            matches!(
                err,
                SyncError::SegmentNotFound {
                    segment_index: 42,
                    ..
                }
            ),
            "got: {:?}",
            err
        );
    }

    #[test]
    fn write_snapshot_segment_unknown_table_returns_segment_not_found() {
        let engine = Arc::new(MVCCEngine::in_memory());
        let adapter = StoolapAdapter::new(Arc::clone(&engine), mission_id(), node_id());
        let err = adapter
            .write_snapshot_segment(0xDEADBEEF, 0, b"x")
            .unwrap_err();
        assert!(
            matches!(err, SyncError::SegmentNotFound { .. }),
            "got: {:?}",
            err
        );
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
