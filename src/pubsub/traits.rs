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

//! Event publisher traits for pub/sub system
//!
//! Provides abstraction over different pub/sub implementations (local broadcast, WAL-based, etc.)

use crate::core::Result;
use crate::pubsub::DatabaseEvent;

/// Trait for event publishing
pub trait EventPublisher: Send + Sync {
    /// Publish an event to all subscribers
    fn publish(&self, event: DatabaseEvent) -> Result<()>;

    /// Subscribe to events - returns a subscriber handle
    fn subscribe(&self) -> Box<dyn EventSubscriber>;

    /// Get the number of subscribers
    fn subscriber_count(&self) -> usize;
}

/// Trait for event subscription
pub trait EventSubscriber: Send {
    /// Try to receive an event without blocking
    fn try_recv(&self) -> Option<DatabaseEvent>;

    /// Check if there are events available
    fn is_ready(&self) -> bool;
}

/// No-op event publisher for when pub/sub is not configured
pub struct NoopPublisher;

impl EventPublisher for NoopPublisher {
    fn publish(&self, _event: DatabaseEvent) -> Result<()> {
        // No-op: do nothing
        Ok(())
    }

    fn subscribe(&self) -> Box<dyn EventSubscriber> {
        Box::new(NoopSubscriber)
    }

    fn subscriber_count(&self) -> usize {
        0
    }
}

/// No-op subscriber that never returns events
pub struct NoopSubscriber;

impl EventSubscriber for NoopSubscriber {
    fn try_recv(&self) -> Option<DatabaseEvent> {
        None
    }

    fn is_ready(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_publisher() {
        let publisher = NoopPublisher;
        assert_eq!(publisher.subscriber_count(), 0);

        let event = DatabaseEvent::TableModified {
            table_name: "test".to_string(),
            operation: crate::pubsub::OperationType::Insert,
            txn_id: 1,
            event_id: [0u8; 32],
        };

        assert!(publisher.publish(event).is_ok());

        let subscriber = publisher.subscribe();
        assert!(subscriber.try_recv().is_none());
    }
}
