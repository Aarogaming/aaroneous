//! crates/ipc_bus/src/universal_event_bus.rs
//! High-Performance Typed Topic Event Bus with Lock-Free Sequence Barriers.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// A typed event envelope with monotonic sequencing and microsecond timestamps
#[derive(Debug, Clone, PartialEq)]
pub struct EventEnvelope<T> {
    pub topic: String,
    pub sequence_id: u64,
    pub timestamp_us: u64,
    pub payload: T,
}

/// Thread-safe lock-free sequence counter for monotonic ordering
#[derive(Debug, Default)]
pub struct SequenceBarrier {
    cursor: AtomicU64,
}

impl SequenceBarrier {
    pub fn new(initial: u64) -> Self {
        Self {
            cursor: AtomicU64::new(initial),
        }
    }

    pub fn next(&self) -> u64 {
        self.cursor.fetch_add(1, Ordering::SeqCst)
    }

    pub fn current(&self) -> u64 {
        self.cursor.load(Ordering::SeqCst)
    }
}

/// Generic bounded channel consumer handle for a registered subscriber
pub struct EventSubscriber<T> {
    pub subscriber_id: String,
    pub topic: String,
    receiver: std::sync::mpsc::Receiver<EventEnvelope<T>>,
}

impl<T> EventSubscriber<T> {
    pub fn try_recv(&self) -> Option<EventEnvelope<T>> {
        self.receiver.try_recv().ok()
    }

    pub fn recv_timeout(&self, timeout: std::time::Duration) -> Option<EventEnvelope<T>> {
        self.receiver.recv_timeout(timeout).ok()
    }
}

type TopicSenderList<T> = Vec<std::sync::mpsc::Sender<EventEnvelope<T>>>;
type SubscriberRegistry<T> = Arc<Mutex<HashMap<String, TopicSenderList<T>>>>;

/// Multi-producer multi-consumer typed event bus with topic routing
#[derive(Clone)]
pub struct UniversalEventBus<T: Clone> {
    barrier: Arc<SequenceBarrier>,
    subscribers: SubscriberRegistry<T>,
}

impl<T: Clone> Default for UniversalEventBus<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> UniversalEventBus<T> {
    pub fn new() -> Self {
        Self {
            barrier: Arc::new(SequenceBarrier::default()),
            subscribers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Subscribes to an event topic (supports exact match or prefix wildcard like "telemetry.*")
    pub fn subscribe(&self, topic: &str, subscriber_id: &str) -> EventSubscriber<T> {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut guard = self.subscribers.lock().unwrap_or_else(|p| p.into_inner());
        guard.entry(topic.to_string()).or_default().push(tx);

        EventSubscriber {
            subscriber_id: subscriber_id.to_string(),
            topic: topic.to_string(),
            receiver: rx,
        }
    }

    /// Publishes a typed payload to a topic, matching exact and wildcard topic subscribers
    pub fn publish(&self, topic: &str, payload: T, timestamp_us: u64) -> u64 {
        let seq = self.barrier.next();
        let envelope = EventEnvelope {
            topic: topic.to_string(),
            sequence_id: seq,
            timestamp_us,
            payload,
        };

        let mut guard = self.subscribers.lock().unwrap_or_else(|p| p.into_inner());
        for (pattern, senders) in guard.iter_mut() {
            let is_match = pattern == topic
                || (pattern.ends_with(".*") && topic.starts_with(&pattern[..pattern.len() - 2]))
                || pattern == "*";

            if is_match {
                senders.retain(|tx| tx.send(envelope.clone()).is_ok());
            }
        }

        seq
    }

    /// Publishes a batch of typed events in a single locked pass, reducing lock contention under high telemetry throughput
    pub fn publish_batch<I>(&self, events: I) -> Vec<u64>
    where
        I: IntoIterator<Item = (String, T, u64)>,
    {
        let mut sequences = Vec::new();
        let mut envelopes = Vec::new();

        for (topic, payload, timestamp_us) in events {
            let seq = self.barrier.next();
            sequences.push(seq);
            envelopes.push((
                topic.clone(),
                EventEnvelope {
                    topic,
                    sequence_id: seq,
                    timestamp_us,
                    payload,
                },
            ));
        }

        let mut guard = self.subscribers.lock().unwrap_or_else(|p| p.into_inner());
        for (pattern, senders) in guard.iter_mut() {
            for (topic, envelope) in &envelopes {
                let is_match = pattern == topic
                    || (pattern.ends_with(".*") && topic.starts_with(&pattern[..pattern.len() - 2]))
                    || pattern == "*";

                if is_match {
                    senders.retain(|tx| tx.send(envelope.clone()).is_ok());
                }
            }
        }

        sequences
    }

    /// Returns the total active subscriber count for a specific topic pattern
    pub fn subscriber_count(&self, topic: &str) -> usize {
        let guard = self.subscribers.lock().unwrap_or_else(|p| p.into_inner());
        guard.get(topic).map(|v| v.len()).unwrap_or(0)
    }

    pub fn current_sequence(&self) -> u64 {
        self.barrier.current()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_bus_publish_subscribe() {
        let bus = UniversalEventBus::<String>::new();
        let sub1 = bus.subscribe("telemetry.power", "sub1");
        let sub2 = bus.subscribe("telemetry.power", "sub2");

        let seq = bus.publish("telemetry.power", "watts:42.0".to_string(), 1000);
        assert_eq!(seq, 0);

        let evt1 = sub1.try_recv().expect("sub1 received");
        let evt2 = sub2.try_recv().expect("sub2 received");

        assert_eq!(evt1.sequence_id, 0);
        assert_eq!(evt1.payload, "watts:42.0");
        assert_eq!(evt2.payload, "watts:42.0");
    }

    #[test]
    fn test_wildcard_topic_subscription() {
        let bus = UniversalEventBus::<String>::new();
        let wild_sub = bus.subscribe("telemetry.*", "wildcard_listener");
        let all_sub = bus.subscribe("*", "global_listener");
        let explicit_sub = bus.subscribe("system.heartbeat", "hb_listener");

        assert_eq!(bus.subscriber_count("telemetry.*"), 1);
        assert_eq!(bus.subscriber_count("*"), 1);

        bus.publish("telemetry.thermal", "temp:55C".to_string(), 2000);
        bus.publish("system.heartbeat", "alive".to_string(), 2001);

        let wild_evt = wild_sub.try_recv().expect("wildcard got thermal");
        assert_eq!(wild_evt.payload, "temp:55C");
        assert!(wild_sub.try_recv().is_none());

        let all_evt1 = all_sub.try_recv().expect("global got thermal");
        let all_evt2 = all_sub.try_recv().expect("global got heartbeat");
        assert_eq!(all_evt1.payload, "temp:55C");
        assert_eq!(all_evt2.payload, "alive");

        let exp_evt = explicit_sub.try_recv().expect("explicit got heartbeat");
        assert_eq!(exp_evt.payload, "alive");
    }

    #[test]
    fn test_publish_batch() {
        let bus = UniversalEventBus::<String>::new();
        let sub = bus.subscribe("sensor.*", "batch_sub");

        let batch = vec![
            ("sensor.temp".to_string(), "24C".to_string(), 100),
            ("sensor.humidity".to_string(), "45%".to_string(), 101),
        ];

        let seqs = bus.publish_batch(batch);
        assert_eq!(seqs.len(), 2);
        assert_eq!(seqs, vec![0, 1]);

        let e1 = sub.try_recv().expect("e1");
        let e2 = sub.try_recv().expect("e2");

        assert_eq!(e1.payload, "24C");
        assert_eq!(e2.payload, "45%");
    }
}
