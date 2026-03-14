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

//! Pub/Sub module for distributed cache invalidation
//!
//! This module provides:
//! - EventBus: Local broadcast for same-process cache invalidation
//! - WalPubSub: WAL-based pub/sub for cross-process cache invalidation

pub mod event_bus;
pub mod traits;
pub mod wal_pubsub;

pub use event_bus::{
    DatabaseEvent, EventBus, InvalidationReason, OperationType, PubSubEventType,
    SchemaChangeType,
};
pub use traits::{EventPublisher, EventSubscriber, NoopPublisher, NoopSubscriber};
pub use wal_pubsub::{compute_event_id, IdempotencyTracker, WalPubSub, WalPubSubEntry};

use sha2::{Sha256, Digest};

/// Generate a unique event ID for table modification events
pub fn generate_event_id() -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(&std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .to_le_bytes());
    let result = hasher.finalize();
    result.into()
}
