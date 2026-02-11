//! Broadcast channel for domain events.
//!
//! [`EventBus`] wraps a [`tokio::sync::broadcast`] channel. Every state
//! mutation publishes a [`PoolEvent`] through the bus, and all WebSocket
//! connections subscribe to receive filtered events.

use tokio::sync::broadcast;

use super::PoolEvent;

/// Broadcast bus for [`PoolEvent`]s.
///
/// Backed by a `tokio::broadcast` channel with a configurable capacity
/// (default 10 000). When the ring buffer is full, the oldest events are
/// dropped for lagging receivers.
#[derive(Debug, Clone)]
pub struct EventBus {
    sender: broadcast::Sender<PoolEvent>,
}

impl EventBus {
    /// Creates a new `EventBus` with the given channel capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Publishes an event to all subscribers.
    ///
    /// Returns the number of receivers that received the event.
    /// If there are no active receivers, the event is silently dropped.
    pub fn publish(&self, event: PoolEvent) -> usize {
        self.sender.send(event).unwrap_or(0)
    }

    /// Creates a new receiver that will receive all future events.
    ///
    /// Each WebSocket connection should call this once on connect.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<PoolEvent> {
        self.sender.subscribe()
    }

    /// Returns the current number of active receivers.
    #[must_use]
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::domain::PoolId;
    use chrono::Utc;

    fn make_event(pool_id: PoolId) -> PoolEvent {
        PoolEvent::PoolCreated {
            pool_id,
            pool_type: "constant_product".to_string(),
            token_a: "0xaaa".to_string(),
            token_b: "0xbbb".to_string(),
            fee_tier: 30,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn publish_without_receivers_returns_zero() {
        let bus = EventBus::new(100);
        let count = bus.publish(make_event(PoolId::new()));
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn subscriber_receives_event() {
        let bus = EventBus::new(100);
        let mut rx = bus.subscribe();

        let id = PoolId::new();
        bus.publish(make_event(id));

        let event = rx.recv().await;
        let Ok(event) = event else {
            panic!("expected to receive event");
        };
        assert_eq!(event.pool_id(), id);
    }

    #[tokio::test]
    async fn multiple_subscribers_receive_same_event() {
        let bus = EventBus::new(100);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let id = PoolId::new();
        let count = bus.publish(make_event(id));
        assert_eq!(count, 2);

        let e1 = rx1.recv().await;
        let e2 = rx2.recv().await;
        let Ok(e1) = e1 else {
            panic!("rx1 failed");
        };
        let Ok(e2) = e2 else {
            panic!("rx2 failed");
        };
        assert_eq!(e1.pool_id(), e2.pool_id());
    }

    #[test]
    fn receiver_count_tracks_subscribers() {
        let bus = EventBus::new(100);
        assert_eq!(bus.receiver_count(), 0);

        let _rx1 = bus.subscribe();
        assert_eq!(bus.receiver_count(), 1);

        let _rx2 = bus.subscribe();
        assert_eq!(bus.receiver_count(), 2);

        drop(_rx1);
        assert_eq!(bus.receiver_count(), 1);
    }
}
