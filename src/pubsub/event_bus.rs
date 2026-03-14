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

//! EventBus: Local broadcast for same-process cache invalidation
//!
//! Uses parking_lot Mutex and Vec for simple intra-process event distribution.

use parking_lot::Mutex;
use std::fmt;
use std::sync::Arc;

/// Local broadcast for same-process cache invalidation
#[derive(Clone)]
pub struct EventBus {
    inner: Arc<Mutex<EventBusInner>>,
}

struct EventBusInner {
    subscribers: Vec<channel::Sender<DatabaseEvent>>,
}

mod channel {
    use std::sync::mpsc;
    pub type Sender<T> = mpsc::Sender<T>;
    pub type Receiver<T> = mpsc::Receiver<T>;
    pub type SendError<T> = mpsc::SendError<T>;

    pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
        mpsc::channel()
    }
}

impl EventBus {
    /// Create a new EventBus
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(EventBusInner {
                subscribers: Vec::new(),
            })),
        }
    }

    /// Subscribe to events - returns a receiver
    pub fn subscribe(&self) -> channel::Receiver<DatabaseEvent> {
        let (tx, rx) = channel::channel();
        self.inner.lock().subscribers.push(tx);
        rx
    }

    /// Publish an event to all subscribers
    pub fn publish(&self, event: DatabaseEvent) -> Result<(), ()> {
        let mut inner = self.inner.lock();

        // Collect dead subscribers to remove
        let mut dead_indices = Vec::new();

        for (i, tx) in inner.subscribers.iter().enumerate() {
            if tx.send(event.clone()).is_err() {
                dead_indices.push(i);
            }
        }

        // Remove dead subscribers (in reverse order to maintain indices)
        for i in dead_indices.into_iter().rev() {
            inner.subscribers.remove(i);
        }

        Ok(())
    }

    /// Get the number of subscribers
    pub fn subscriber_count(&self) -> usize {
        self.inner.lock().subscribers.len()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for EventBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventBus")
            .field("subscriber_count", &self.subscriber_count())
            .finish()
    }
}

/// Database events for pub/sub
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DatabaseEvent {
    /// Key invalidated (revoked, rotated, budget changed)
    KeyInvalidated {
        key_hash: Vec<u8>,
        reason: InvalidationReason,
        /// Updated rate limits for cross-process sync
        rpm_limit: Option<u32>,
        tpm_limit: Option<u32>,
        /// Event ID for idempotency
        event_id: [u8; 32],
    },
    /// Table modified (for query cache invalidation)
    TableModified {
        table_name: String,
        operation: OperationType,
        txn_id: i64,
        event_id: [u8; 32],
    },
    /// Schema changed (DDL)
    SchemaChanged {
        table_name: String,
        change_type: SchemaChangeType,
        event_id: [u8; 32],
    },
    /// Transaction committed
    TransactionCommited {
        txn_id: i64,
        affected_tables: Vec<String>,
        event_id: [u8; 32],
    },
}

impl DatabaseEvent {
    /// Get the channel name for routing
    pub fn channel_name(&self) -> String {
        match self {
            DatabaseEvent::KeyInvalidated { .. } => "key:invalidate".to_string(),
            DatabaseEvent::TableModified { table_name, .. } => {
                format!("table:{}", table_name)
            }
            DatabaseEvent::SchemaChanged { table_name, .. } => {
                format!("schema:{}", table_name)
            }
            DatabaseEvent::TransactionCommited { .. } => "txn:commit".to_string(),
        }
    }

    /// Get the pub/sub event type for WAL entries
    pub fn pub_sub_type(&self) -> PubSubEventType {
        match self {
            DatabaseEvent::KeyInvalidated { .. } => PubSubEventType::KeyInvalidated,
            DatabaseEvent::TableModified { .. } => PubSubEventType::CacheCleared,
            DatabaseEvent::SchemaChanged { .. } => PubSubEventType::SchemaChanged,
            DatabaseEvent::TransactionCommited { .. } => PubSubEventType::CacheCleared,
        }
    }

    /// Get the event ID
    pub fn event_id(&self) -> [u8; 32] {
        match self {
            DatabaseEvent::KeyInvalidated { event_id, .. } => *event_id,
            DatabaseEvent::TableModified { event_id, .. } => *event_id,
            DatabaseEvent::SchemaChanged { event_id, .. } => *event_id,
            DatabaseEvent::TransactionCommited { event_id, .. } => *event_id,
        }
    }
}

/// Invalidation reason for KeyInvalidated events
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InvalidationReason {
    /// API key revoked
    Revoke,
    /// Key rotation
    Rotate,
    /// Balance changed
    UpdateBudget,
    /// RPM/TPM changed
    UpdateRateLimit,
    /// TTL expired
    Expire,
    /// Table DDL change
    SchemaChange,
}

impl fmt::Display for InvalidationReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvalidationReason::Revoke => write!(f, "REVOKE"),
            InvalidationReason::Rotate => write!(f, "ROTATE"),
            InvalidationReason::UpdateBudget => write!(f, "UPDATE_BUDGET"),
            InvalidationReason::UpdateRateLimit => write!(f, "UPDATE_RATE_LIMIT"),
            InvalidationReason::Expire => write!(f, "EXPIRE"),
            InvalidationReason::SchemaChange => write!(f, "SCHEMA_CHANGE"),
        }
    }
}

/// Operation type for TableModified events
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OperationType {
    Insert,
    Update,
    Delete,
}

impl fmt::Display for OperationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationType::Insert => write!(f, "INSERT"),
            OperationType::Update => write!(f, "UPDATE"),
            OperationType::Delete => write!(f, "DELETE"),
        }
    }
}

/// Schema change type for SchemaChanged events
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SchemaChangeType {
    CreateTable,
    DropTable,
    AlterTable,
}

impl fmt::Display for SchemaChangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemaChangeType::CreateTable => write!(f, "CREATE_TABLE"),
            SchemaChangeType::DropTable => write!(f, "DROP_TABLE"),
            SchemaChangeType::AlterTable => write!(f, "ALTER_TABLE"),
        }
    }
}

/// Pub/sub event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PubSubEventType {
    KeyInvalidated,
    BudgetUpdated,
    RateLimitUpdated,
    SchemaChanged,
    CacheCleared,
}

impl PubSubEventType {
    /// Convert to byte for serialization
    pub fn to_u8(&self) -> u8 {
        match self {
            PubSubEventType::KeyInvalidated => 0,
            PubSubEventType::BudgetUpdated => 1,
            PubSubEventType::RateLimitUpdated => 2,
            PubSubEventType::SchemaChanged => 3,
            PubSubEventType::CacheCleared => 4,
        }
    }

    /// Create from byte
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(PubSubEventType::KeyInvalidated),
            1 => Some(PubSubEventType::BudgetUpdated),
            2 => Some(PubSubEventType::RateLimitUpdated),
            3 => Some(PubSubEventType::SchemaChanged),
            4 => Some(PubSubEventType::CacheCleared),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_bus_new_and_subscribe() {
        let bus = EventBus::new();
        let _rx = bus.subscribe();
    }

    #[test]
    fn test_event_bus_publish_subscribe() {
        let bus = EventBus::new();
        let rx = bus.subscribe();

        let event = DatabaseEvent::KeyInvalidated {
            key_hash: vec![1, 2, 3],
            reason: InvalidationReason::Revoke,
            rpm_limit: None,
            tpm_limit: None,
            event_id: [0u8; 32],
        };

        bus.publish(event.clone()).unwrap();

        let received = rx.recv().unwrap();
        assert_eq!(received, event);
    }

    #[test]
    fn test_channel_name() {
        let event = DatabaseEvent::KeyInvalidated {
            key_hash: vec![1, 2, 3],
            reason: InvalidationReason::Revoke,
            rpm_limit: None,
            tpm_limit: None,
            event_id: [0u8; 32],
        };
        assert_eq!(event.channel_name(), "key:invalidate");

        let event = DatabaseEvent::TableModified {
            table_name: "users".to_string(),
            operation: OperationType::Update,
            txn_id: 1,
            event_id: [0u8; 32],
        };
        assert_eq!(event.channel_name(), "table:users");
    }

    #[test]
    fn test_event_id() {
        let event_id = [1u8; 32];
        let event = DatabaseEvent::KeyInvalidated {
            key_hash: vec![1, 2, 3],
            reason: InvalidationReason::Revoke,
            rpm_limit: None,
            tpm_limit: None,
            event_id,
        };
        assert_eq!(event.event_id(), event_id);
    }
}
