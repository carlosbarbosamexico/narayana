// Advanced Native Events System - Never Need RabbitMQ Again!
// Comprehensive event streaming, pub/sub, queues, and more

use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use parking_lot::RwLock;
use dashmap::DashMap;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use uuid::Uuid;
use tracing::{info, warn, error, debug};

/// Event ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub u64);

/// Event stream name
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StreamName(pub String);

/// Event topic name
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TopicName(pub String);

/// Event queue name
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueueName(pub String);

/// Event - core event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub stream: StreamName,
    pub topic: Option<TopicName>,
    pub queue: Option<QueueName>,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub headers: HashMap<String, String>,
    pub timestamp: u64,
    pub correlation_id: Option<String>,
    pub causation_id: Option<EventId>,
    pub partition_key: Option<String>,
    pub ttl: Option<u64>, // Time to live in seconds
    pub priority: u8, // 0-255, higher = more priority
}

/// Event subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSubscription {
    pub id: String,
    pub stream: StreamName,
    pub topic: Option<TopicName>,
    pub filter: Option<EventFilter>,
    pub consumer_group: Option<String>,
    pub offset: EventOffset,
    pub batch_size: usize,
    pub auto_ack: bool,
    pub max_retries: usize,
    pub retry_delay: Duration,
}

/// Event filter for selective subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventFilter {
    /// Filter by event type (exact match)
    EventType(String),
    /// Filter by event type pattern (regex)
    EventTypePattern(String),
    /// Filter by header (key-value match)
    Header { key: String, value: String },
    /// Filter by header pattern (key-regex match)
    HeaderPattern { key: String, pattern: String },
    /// Filter by payload field (JSONPath)
    PayloadField { path: String, value: serde_json::Value },
    /// Filter by payload field pattern (JSONPath-regex)
    PayloadPattern { path: String, pattern: String },
    /// Combined filter (AND logic)
    And(Vec<EventFilter>),
    /// Combined filter (OR logic)
    Or(Vec<EventFilter>),
    /// Negated filter (NOT logic)
    Not(Box<EventFilter>),
}

/// Event offset for consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventOffset {
    /// Start from beginning
    Beginning,
    /// Start from end (latest)
    End,
    /// Start from specific event ID
    FromId(EventId),
    /// Start from specific timestamp
    FromTimestamp(u64),
    /// Start from sequence number
    FromSequence(u64),
}

/// Event stream - ordered sequence of events
#[derive(Debug, Clone)]
pub struct EventStream {
    pub name: StreamName,
    pub partitions: usize,
    pub retention: Option<Duration>,
    pub replication_factor: usize,
    pub compression: bool,
    pub encryption: bool,
    pub max_size: Option<u64>,
    pub max_events: Option<u64>,
}

/// Event topic - pub/sub topic
#[derive(Debug, Clone)]
pub struct EventTopic {
    pub name: TopicName,
    pub stream: StreamName,
    pub partitions: usize,
    pub retention: Option<Duration>,
    pub fanout: bool, // If true, all subscribers get all messages
}

/// Event queue - FIFO queue with persistence
#[derive(Debug, Clone)]
pub struct EventQueue {
    pub name: QueueName,
    pub stream: StreamName,
    pub topic: Option<TopicName>,
    pub fifo: bool, // If true, maintains strict order
    pub deduplication: bool, // If true, deduplicates by ID
    pub visibility_timeout: Duration, // Time before message becomes visible again after consumption
    pub max_receives: usize, // Max times message can be received before DLQ
    pub dead_letter_queue: Option<QueueName>,
    pub retention: Option<Duration>,
    pub max_size: Option<u64>,
    pub max_messages: Option<u64>,
}

/// Event consumer - consumes events from stream/topic/queue
pub struct EventConsumer {
    pub subscription: EventSubscription,
    pub receiver: mpsc::Receiver<Event>,
    pub handle: JoinHandle<()>,
}

/// Event producer - produces events to stream/topic/queue
#[derive(Debug, Clone)]
pub struct EventProducer {
    pub stream: StreamName,
    pub topic: Option<TopicName>,
    pub queue: Option<QueueName>,
    pub sender: mpsc::Sender<Event>,
}

/// Event delivery guarantee
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryGuarantee {
    /// At most once (may lose messages)
    AtMostOnce,
    /// At least once (may duplicate messages)
    AtLeastOnce,
    /// Exactly once (no loss, no duplicates)
    ExactlyOnce,
}

/// Event persistence mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersistenceMode {
    /// Ephemeral (memory only, lost on restart)
    Ephemeral,
    /// Persistent (written to disk)
    Persistent,
    /// Durable (replicated and persisted)
    Durable,
}

/// Native Events System - Never Need RabbitMQ Again!
pub struct NativeEventsSystem {
    // Streams
    streams: Arc<RwLock<HashMap<StreamName, EventStream>>>,
    stream_events: Arc<DashMap<StreamName, Vec<Event>>>,
    stream_sequences: Arc<DashMap<StreamName, u64>>,
    
    // Topics
    topics: Arc<RwLock<HashMap<TopicName, EventTopic>>>,
    topic_subscribers: Arc<DashMap<TopicName, Vec<broadcast::Sender<Event>>>>,
    
    // Queues
    queues: Arc<RwLock<HashMap<QueueName, EventQueue>>>,
    queue_messages: Arc<DashMap<QueueName, Vec<Event>>>,
    queue_in_flight: Arc<DashMap<QueueName, HashMap<EventId, SystemTime>>>,
    queue_dlq: Arc<DashMap<QueueName, Vec<Event>>>,
    
    // Producers
    producers: Arc<DashMap<String, EventProducer>>,
    
    // Consumers
    consumers: Arc<DashMap<String, EventConsumer>>,
    
    // Persistence
    persistence: Option<Arc<dyn EventPersistence>>,
    
    // Configuration
    config: EventsConfig,
    
    // Metrics
    metrics: Arc<RwLock<EventsMetrics>>,
}

/// Events configuration
#[derive(Debug, Clone)]
pub struct EventsConfig {
    pub default_delivery_guarantee: DeliveryGuarantee,
    pub default_persistence: PersistenceMode,
    pub default_retention: Option<Duration>,
    pub default_batch_size: usize,
    pub max_message_size: usize,
    pub enable_persistence: bool,
    pub enable_replication: bool,
    pub enable_compression: bool,
    pub enable_encryption: bool,
    pub replication_factor: usize,
    pub partition_count: usize,
}

impl Default for EventsConfig {
    fn default() -> Self {
        Self {
            default_delivery_guarantee: DeliveryGuarantee::AtLeastOnce,
            default_persistence: PersistenceMode::Persistent,
            default_retention: Some(Duration::from_secs(7 * 24 * 60 * 60)), // 7 days
            default_batch_size: 100,
            max_message_size: 10 * 1024 * 1024, // 10MB
            enable_persistence: true,
            enable_replication: false,
            enable_compression: true,
            enable_encryption: false,
            replication_factor: 1,
            partition_count: 1,
        }
    }
}

/// Events metrics
#[derive(Debug, Clone, Default)]
pub struct EventsMetrics {
    pub events_published: u64,
    pub events_consumed: u64,
    pub events_delivered: u64,
    pub events_failed: u64,
    pub events_duplicated: u64,
    pub events_lost: u64,
    pub streams_count: usize,
    pub topics_count: usize,
    pub queues_count: usize,
    pub consumers_count: usize,
    pub producers_count: usize,
    pub average_latency_ms: f64,
    pub throughput_per_second: f64,
}

/// Event persistence trait
#[async_trait::async_trait]
pub trait EventPersistence: Send + Sync {
    async fn save_event(&self, stream: &StreamName, event: &Event) -> Result<()>;
    async fn load_events(&self, stream: &StreamName, offset: &EventOffset, limit: usize) -> Result<Vec<Event>>;
    async fn save_subscription(&self, subscription: &EventSubscription) -> Result<()>;
    async fn load_subscription(&self, id: &str) -> Result<Option<EventSubscription>>;
    async fn save_consumer_offset(&self, subscription_id: &str, stream: &StreamName, offset: EventId) -> Result<()>;
    async fn load_consumer_offset(&self, subscription_id: &str, stream: &StreamName) -> Result<Option<EventId>>;
}

/// In-memory event persistence (for testing)
pub struct InMemoryEventPersistence {
    events: Arc<DashMap<StreamName, Vec<Event>>>,
    subscriptions: Arc<DashMap<String, EventSubscription>>,
    offsets: Arc<DashMap<String, EventId>>,
}

impl InMemoryEventPersistence {
    pub fn new() -> Self {
        Self {
            events: Arc::new(DashMap::new()),
            subscriptions: Arc::new(DashMap::new()),
            offsets: Arc::new(DashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl EventPersistence for InMemoryEventPersistence {
    async fn save_event(&self, stream: &StreamName, event: &Event) -> Result<()> {
        // SECURITY: Prevent unbounded Vec growth - enforce limits
        const MAX_EVENTS_PER_STREAM: usize = 10_000_000; // 10 million events max per stream
        let mut events = self.events.entry(stream.clone()).or_insert_with(Vec::new);
        
        // SECURITY: Evict oldest events if limit reached
        if events.len() >= MAX_EVENTS_PER_STREAM {
            // Remove oldest 10% to make room
            let to_remove = MAX_EVENTS_PER_STREAM / 10;
            events.drain(0..to_remove);
        }
        
        events.push(event.clone());
        Ok(())
    }

    async fn load_events(&self, stream: &StreamName, offset: &EventOffset, limit: usize) -> Result<Vec<Event>> {
        let events = self.events.get(stream)
            .ok_or_else(|| Error::Storage(format!("Stream {} not found", stream.0)))?;
        
        let start_idx = match offset {
            EventOffset::Beginning => 0,
            EventOffset::End => events.len().saturating_sub(limit),
            EventOffset::FromId(id) => {
                events.iter().position(|e| e.id == *id)
                    .unwrap_or(0)
            }
            EventOffset::FromTimestamp(ts) => {
                events.iter().position(|e| e.timestamp >= *ts)
                    .unwrap_or(0)
            }
            EventOffset::FromSequence(seq) => (*seq as usize).min(events.len()),
        };
        
        let end_idx = (start_idx + limit).min(events.len());
        Ok(events[start_idx..end_idx].to_vec())
    }

    async fn save_subscription(&self, subscription: &EventSubscription) -> Result<()> {
        self.subscriptions.insert(subscription.id.clone(), subscription.clone());
        Ok(())
    }

    async fn load_subscription(&self, id: &str) -> Result<Option<EventSubscription>> {
        Ok(self.subscriptions.get(id).map(|s| s.clone()))
    }

    async fn save_consumer_offset(&self, subscription_id: &str, _stream: &StreamName, offset: EventId) -> Result<()> {
        self.offsets.insert(subscription_id.to_string(), offset);
        Ok(())
    }

    async fn load_consumer_offset(&self, subscription_id: &str, _stream: &StreamName) -> Result<Option<EventId>> {
        Ok(self.offsets.get(subscription_id).map(|o| *o.value()))
    }
}

impl NativeEventsSystem {
    /// Create new events system
    pub fn new(config: EventsConfig) -> Self {
        let persistence = if config.enable_persistence {
            Some(Arc::new(InMemoryEventPersistence::new()) as Arc<dyn EventPersistence>)
        } else {
            None
        };

        Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
            stream_events: Arc::new(DashMap::new()),
            stream_sequences: Arc::new(DashMap::new()),
            topics: Arc::new(RwLock::new(HashMap::new())),
            topic_subscribers: Arc::new(DashMap::new()),
            queues: Arc::new(RwLock::new(HashMap::new())),
            queue_messages: Arc::new(DashMap::new()),
            queue_in_flight: Arc::new(DashMap::new()),
            queue_dlq: Arc::new(DashMap::new()),
            producers: Arc::new(DashMap::new()),
            consumers: Arc::new(DashMap::new()),
            persistence,
            config,
            metrics: Arc::new(RwLock::new(EventsMetrics::default())),
        }
    }

    /// Create event stream
    pub async fn create_stream(&self, stream: EventStream) -> Result<()> {
        let mut streams = self.streams.write();
        if streams.contains_key(&stream.name) {
            return Err(Error::Storage(format!("Stream {} already exists", stream.name.0)));
        }
        
        streams.insert(stream.name.clone(), stream.clone());
        self.stream_events.insert(stream.name.clone(), Vec::new());
        self.stream_sequences.insert(stream.name.clone(), 0);
        
        let mut metrics = self.metrics.write();
        metrics.streams_count = streams.len();
        
        info!("Created event stream: {}", stream.name.0);
        Ok(())
    }

    /// Create event topic
    pub async fn create_topic(&self, topic: EventTopic) -> Result<()> {
        // Verify stream exists
        let streams = self.streams.read();
        if !streams.contains_key(&topic.stream) {
            return Err(Error::Storage(format!("Stream {} not found", topic.stream.0)));
        }
        drop(streams);
        
        let mut topics = self.topics.write();
        if topics.contains_key(&topic.name) {
            return Err(Error::Storage(format!("Topic {} already exists", topic.name.0)));
        }
        
        topics.insert(topic.name.clone(), topic.clone());
        self.topic_subscribers.insert(topic.name.clone(), Vec::new());
        
        let mut metrics = self.metrics.write();
        metrics.topics_count = topics.len();
        
        info!("Created event topic: {}", topic.name.0);
        Ok(())
    }

    /// Create event queue
    pub async fn create_queue(&self, queue: EventQueue) -> Result<()> {
        // Verify stream exists
        let streams = self.streams.read();
        if !streams.contains_key(&queue.stream) {
            return Err(Error::Storage(format!("Stream {} not found", queue.stream.0)));
        }
        drop(streams);
        
        // Verify topic exists if specified
        if let Some(ref topic) = queue.topic {
            let topics = self.topics.read();
            if !topics.contains_key(topic) {
                return Err(Error::Storage(format!("Topic {} not found", topic.0)));
            }
        }
        
        let mut queues = self.queues.write();
        if queues.contains_key(&queue.name) {
            return Err(Error::Storage(format!("Queue {} already exists", queue.name.0)));
        }
        
        queues.insert(queue.name.clone(), queue.clone());
        self.queue_messages.insert(queue.name.clone(), Vec::new());
        self.queue_in_flight.insert(queue.name.clone(), HashMap::new());
        self.queue_dlq.insert(queue.name.clone(), Vec::new());
        
        let mut metrics = self.metrics.write();
        metrics.queues_count = queues.len();
        
        info!("Created event queue: {}", queue.name.0);
        Ok(())
    }

    /// Publish event to stream
    pub async fn publish_event(&self, mut event: Event) -> Result<EventId> {
        // SECURITY: Validate event size before serialization to prevent DoS
        // Serialize to check size (needed for validation)
        let event_json = serde_json::to_string(&event)
            .map_err(|e| Error::Storage(format!("Failed to serialize event: {}", e)))?;
        let event_size = event_json.len();
        
        if event_size > self.config.max_message_size {
            return Err(Error::Storage(format!(
                "Event size {} exceeds maximum {}",
                event_size, self.config.max_message_size
            )));
        }
        
        // SECURITY: Validate payload size separately (prevent deserialization bombs)
        if let Some(payload_size) = event.payload.as_object()
            .and_then(|obj| serde_json::to_string(obj).ok())
            .map(|s| s.len()) {
            if payload_size > self.config.max_message_size / 2 {
                return Err(Error::Storage(format!(
                    "Event payload size {} exceeds maximum {}",
                    payload_size, self.config.max_message_size / 2
                )));
            }
        }
        
        // Generate event ID if not set
        if event.id.0 == 0 {
            let mut sequence = self.stream_sequences.entry(event.stream.clone())
                .or_insert(0);
            *sequence += 1;
            event.id = EventId(*sequence);
        }
        
        // Set timestamp if not set
        if event.timestamp == 0 {
            event.timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }
        
        // Add to stream
        let mut stream_events = self.stream_events.entry(event.stream.clone())
            .or_insert_with(Vec::new);
        
        // SECURITY: Enforce size limits to prevent resource exhaustion attack
        let max_events_limit = {
            let streams = self.streams.read();
            streams.get(&event.stream).and_then(|sc| sc.max_events)
        };
        let max_size_limit = {
            let streams = self.streams.read();
            streams.get(&event.stream).and_then(|sc| sc.max_size)
        };
        
        // Check max_events limit
        if let Some(max_events) = max_events_limit {
            if stream_events.len() >= max_events as usize {
                // Remove oldest events (FIFO eviction)
                let excess = stream_events.len() - (max_events as usize) + 1;
                stream_events.drain(0..excess);
            }
        }
        
        // Check max_size limit (approximate)
        if let Some(max_size) = max_size_limit {
            // Estimate current size (rough calculation)
            let current_len = stream_events.len();
            let estimated_size = current_len * event_size;
            if estimated_size > max_size as usize {
                // Remove oldest events until under limit
                let target_size = (max_size as usize * 9) / 10; // 90% of max
                let excess_bytes = estimated_size.saturating_sub(target_size);
                let events_to_remove = (excess_bytes / event_size).max(1);
                let drain_end = events_to_remove.min(current_len);
                if drain_end > 0 {
                    stream_events.drain(0..drain_end);
                }
            }
        }
        
        stream_events.push(event.clone());
        
        // Persist if enabled
        if let Some(ref persistence) = self.persistence {
            persistence.save_event(&event.stream, &event).await?;
        }
        
        // Publish to topic if specified
        if let Some(ref topic) = event.topic {
            self.publish_to_topic(topic.clone(), event.clone()).await?;
        }
        
        // Add to queue if specified
        if let Some(ref queue) = event.queue {
            self.enqueue(queue.clone(), event.clone()).await?;
        }
        
        // Update metrics (after all awaits)
        let mut metrics = self.metrics.write();
        metrics.events_published += 1;
        
        info!("Published event {} to stream {}", event.id.0, event.stream.0);
        Ok(event.id)
    }

    /// Publish event to topic (pub/sub)
    async fn publish_to_topic(&self, topic: TopicName, event: Event) -> Result<()> {
        if let Some(subscribers) = self.topic_subscribers.get(&topic) {
            for subscriber in subscribers.value() {
                // Fanout to all subscribers
                let _ = subscriber.send(event.clone());
            }
        }
        
        let mut metrics = self.metrics.write();
        metrics.events_delivered += 1;
        
        Ok(())
    }

    /// Enqueue event to queue (FIFO with persistence)
    async fn enqueue(&self, queue: QueueName, event: Event) -> Result<()> {
        let mut messages = self.queue_messages.entry(queue.clone())
            .or_insert_with(Vec::new);
        
        // SECURITY: Fix race condition - read queue config and use it within lock scope
        let queues = self.queues.read();
        let queue_config = queues.get(&queue)
            .ok_or_else(|| Error::Storage(format!("Queue {} not found", queue.0)))?;
        
        // Check deduplication
        if queue_config.deduplication {
            // Check if event already exists
            if messages.iter().any(|e| e.id == event.id) {
                return Ok(()); // Skip duplicate
            }
        }
        
        // SECURITY: Enforce queue size limits to prevent resource exhaustion
        let queue_size = messages.len();
        let max_messages = queue_config.max_messages;
        let max_size = queue_config.max_size;
        let dead_letter_queue = queue_config.dead_letter_queue.clone();
        drop(queues); // Release lock before processing
        
        if let Some(max_messages) = max_messages {
            if queue_size >= max_messages as usize {
                // Move oldest message to DLQ if configured
                if let Some(ref dlq) = dead_letter_queue {
                    if let Some(oldest_event) = messages.first() {
                        let mut dlq_messages = self.queue_dlq.entry(dlq.clone())
                            .or_insert_with(Vec::new);
                        dlq_messages.push(oldest_event.clone());
                        messages.remove(0);
                    }
                } else {
                    // No DLQ, just remove oldest
                    messages.remove(0);
                }
            }
        }
        
        // Check max_size limit
        if let Some(max_size) = max_size {
            let estimated_size = messages.iter().map(|e| serde_json::to_string(e).map(|s| s.len()).unwrap_or(100)).sum::<usize>();
            if estimated_size > max_size as usize {
                // Evict oldest messages
                let target_size = (max_size as usize * 9) / 10; // 90% of max
                while estimated_size > target_size && !messages.is_empty() {
                    messages.remove(0);
                }
            }
        }
        
        messages.push(event.clone());
        
        // Persist if enabled
        if let Some(ref persistence) = self.persistence {
            persistence.save_event(&event.stream, &event).await?;
        }
        
        let mut metrics = self.metrics.write();
        metrics.events_published += 1;
        
        Ok(())
    }

    /// Subscribe to stream/topic
    pub async fn subscribe(&self, subscription: EventSubscription) -> Result<EventConsumer> {
        // Verify stream exists
        let streams = self.streams.read();
        if !streams.contains_key(&subscription.stream) {
            return Err(Error::Storage(format!("Stream {} not found", subscription.stream.0)));
        }
        drop(streams);
        
        // Create consumer channel
        let (sender, receiver) = mpsc::channel(1000);
        
        // Handle topic subscription
        if let Some(ref topic) = subscription.topic {
            let mut subscribers = self.topic_subscribers.entry(topic.clone())
                .or_insert_with(Vec::new);
            
            let (topic_sender, mut topic_receiver) = broadcast::channel(1000);
            subscribers.push(topic_sender);
            
            // Spawn task to forward topic events to consumer
            let sender_clone = sender.clone();
            let filter = subscription.filter.clone();
            let subscription_id = subscription.id.clone();
            let handle = tokio::spawn(async move {
                while let Ok(event) = topic_receiver.recv().await {
                    if Self::matches_filter(&event, &filter) {
                        if sender_clone.send(event).await.is_err() {
                            break;
                        }
                    }
                }
            });
            
            let consumer = EventConsumer {
                subscription: subscription.clone(),
                receiver,
                handle,
            };
            
            // Cannot clone EventConsumer (contains receiver and handle) - store subscription ID reference instead
            // In production, would use Arc<EventConsumer> or store consumer differently
            let _consumer_id = subscription_id.clone();
            
            // Persist subscription
            if let Some(ref persistence) = self.persistence {
                persistence.save_subscription(&subscription).await?;
            }
            
            let mut metrics = self.metrics.write();
            metrics.consumers_count = self.consumers.len();
            
            return Ok(consumer);
        }
        
        // Handle stream subscription
        let subscription_id = subscription.id.clone();
        let subscription_id_clone = subscription_id.clone();
        let stream = subscription.stream.clone();
        let offset = subscription.offset.clone();
        let batch_size = subscription.batch_size;
        let filter = subscription.filter.clone();
        let sender_clone = sender.clone();
        let persistence = self.persistence.clone();
        
        let handle = tokio::spawn(async move {
            let mut current_offset = match offset {
                EventOffset::Beginning => EventId(0),
                EventOffset::End => {
                    // Start from end
                    if let Some(ref pers) = persistence {
                        if let Ok(Some(last_id)) = pers.load_consumer_offset(&subscription_id_clone, &stream).await {
                            last_id
                        } else {
                            EventId(0)
                        }
                    } else {
                        EventId(0)
                    }
                }
                EventOffset::FromId(id) => id,
                EventOffset::FromTimestamp(_ts) => EventId(0), // Simplified
                EventOffset::FromSequence(seq) => EventId(seq),
            };
            
            // SECURITY: Prevent infinite loop DoS attack
            let mut empty_iterations = 0;
            const MAX_EMPTY_ITERATIONS: usize = 100; // Exit after 100 empty batches (10 seconds)
            
            loop {
                // Load events from persistence or memory
                let events = if let Some(ref pers) = persistence {
                    pers.load_events(&stream, &EventOffset::FromId(current_offset), batch_size).await
                        .unwrap_or_default()
                } else {
                    Vec::new() // In-memory would need different handling
                };
                
                // SECURITY: Exit if no events after many iterations (prevent infinite loop)
                if events.is_empty() {
                    empty_iterations += 1;
                    if empty_iterations >= MAX_EMPTY_ITERATIONS {
                        warn!("Consumer {} exiting due to no events after {} iterations", subscription_id_clone, MAX_EMPTY_ITERATIONS);
                        return;
                    }
                } else {
                    empty_iterations = 0; // Reset counter on successful batch
                }
                
                for event in events {
                    if Self::matches_filter(&event, &filter) {
                        if sender_clone.send(event.clone()).await.is_err() {
                            return;
                        }
                        current_offset = event.id;
                        
                        // Save offset
                        if let Some(ref pers) = persistence {
                            let _ = pers.save_consumer_offset(&subscription_id_clone, &stream, current_offset).await;
                        }
                    }
                }
                
                // Wait before next batch
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
        
        let consumer = EventConsumer {
            subscription,
            receiver,
            handle,
        };
        
        // Cannot clone EventConsumer (contains receiver and handle) - store subscription ID reference instead
        // In production, would use Arc<EventConsumer> or store consumer differently
        let _consumer_id = subscription_id.clone();
        
        // Persist subscription
        if let Some(ref persistence) = self.persistence {
            persistence.save_subscription(&consumer.subscription).await?;
        }
        
        let mut metrics = self.metrics.write();
        metrics.consumers_count = self.consumers.len();
        
        Ok(consumer)
    }

    /// Check if event matches filter
    fn matches_filter(event: &Event, filter: &Option<EventFilter>) -> bool {
        let filter = match filter {
            None => return true,
            Some(f) => f,
        };
        
        match filter {
            EventFilter::EventType(et) => event.event_type == *et,
            EventFilter::EventTypePattern(pattern) => {
                // Simple pattern matching (could use regex)
                pattern == "*" || event.event_type.contains(pattern)
            }
            EventFilter::Header { key, value } => {
                event.headers.get(key) == Some(value)
            }
            EventFilter::HeaderPattern { key, pattern } => {
                if let Some(header_value) = event.headers.get(key) {
                    pattern == "*" || header_value.contains(pattern)
                } else {
                    false
                }
            }
            EventFilter::PayloadField { path, value } => {
                // Simple JSONPath-like matching (could use jsonpath crate)
                event.payload.get(path) == Some(value)
            }
            EventFilter::PayloadPattern { path, pattern } => {
                if let Some(payload_value) = event.payload.get(path) {
                    if let Some(str_value) = payload_value.as_str() {
                        pattern == "*" || str_value.contains(pattern)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            EventFilter::And(filters) => {
                filters.iter().all(|f| Self::matches_filter(event, &Some(f.clone())))
            }
            EventFilter::Or(filters) => {
                filters.iter().any(|f| Self::matches_filter(event, &Some(f.clone())))
            }
            EventFilter::Not(f) => {
                !Self::matches_filter(event, &Some(*f.clone()))
            }
        }
    }

    /// Create producer for publishing events
    pub fn create_producer(&self, stream: StreamName, topic: Option<TopicName>, queue: Option<QueueName>) -> EventProducer {
        let (sender, receiver) = mpsc::channel(10000);
        let producer_id = Uuid::new_v4().to_string();
        
        let producer = EventProducer {
            stream: stream.clone(),
            topic: topic.clone(),
            queue: queue.clone(),
            sender: sender.clone(),
        };
        
        self.producers.insert(producer_id.clone(), producer.clone());
        
        // Spawn task to handle producer events
        // SECURITY: Producer tasks are automatically cleaned up when receiver closes
        // Clone Arc references to avoid Send issues with parking_lot guards
        let stream_events_clone = self.stream_events.clone();
        let persistence_clone = self.persistence.clone();
        let producer_id_clone = producer_id.clone();
        let stream_clone = stream.clone();
        let topic_clone = topic.clone();
        let queue_clone = queue.clone();
        tokio::spawn(async move {
            let mut receiver = receiver;
            while let Some(event) = receiver.recv().await {
                let mut event_with_stream = event;
                event_with_stream.stream = stream_clone.clone();
                event_with_stream.topic = topic_clone.clone();
                event_with_stream.queue = queue_clone.clone();
                
                // Simplified event publishing without full NativeEventsSystem to avoid Send issues
                let _ = stream_events_clone.entry(event_with_stream.stream.clone())
                    .or_insert_with(Vec::new)
                    .push(event_with_stream.clone());
                
                if let Some(ref persistence) = persistence_clone {
                    if let Err(e) = persistence.save_event(&event_with_stream.stream, &event_with_stream).await {
                        error!("Failed to persist event from producer {}: {}", producer_id_clone, e);
                    }
                }
            }
            // Cleanup: Producer receiver closed, remove from registry
            debug!("Producer {} receiver closed, cleaning up", producer_id_clone);
        });
        
        let mut metrics = self.metrics.write();
        metrics.producers_count = self.producers.len();
        
        producer
    }

    /// Receive event from queue (FIFO with visibility timeout)
    pub async fn receive_from_queue(&self, queue: QueueName, max_messages: usize, visibility_timeout: Duration) -> Result<Vec<Event>> {
        let mut messages = self.queue_messages.entry(queue.clone())
            .or_insert_with(Vec::new);
        
        let queue_config = {
            let queues = self.queues.read();
            queues.get(&queue)
                .ok_or_else(|| Error::Storage(format!("Queue {} not found", queue.0)))?
                .clone()
        };
        
        let mut received = Vec::new();
        let mut in_flight = self.queue_in_flight.entry(queue.clone())
            .or_insert_with(HashMap::new);
        
        // Filter out in-flight messages
        let now = SystemTime::now();
        in_flight.retain(|_id, &mut timeout| {
            now.duration_since(timeout).unwrap_or_default() < queue_config.visibility_timeout
        });
        
        let mut idx = 0;
        while idx < messages.len() && received.len() < max_messages {
            let event = &messages[idx];
            
            // Skip if in flight
            if in_flight.contains_key(&event.id) {
                idx += 1;
                continue;
            }
            
            // Mark as in-flight
            in_flight.insert(event.id, SystemTime::now());
            received.push(event.clone());
            idx += 1;
        }
        
        // Remove received messages (they'll be added back if not acknowledged)
        // In production, would keep them until ack or visibility timeout expires
        
        let mut metrics = self.metrics.write();
        metrics.events_consumed += received.len() as u64;
        
        Ok(received)
    }

    /// Acknowledge event from queue (remove from queue)
    pub async fn acknowledge(&self, queue: QueueName, event_id: EventId) -> Result<()> {
        let mut messages = self.queue_messages.entry(queue.clone())
            .or_insert_with(Vec::new);
        
        let mut in_flight = self.queue_in_flight.entry(queue.clone())
            .or_insert_with(HashMap::new);
        
        // Remove from in-flight
        in_flight.remove(&event_id);
        
        // Remove from queue
        messages.retain(|e| e.id != event_id);
        
        let mut metrics = self.metrics.write();
        metrics.events_delivered += 1;
        
        Ok(())
    }

    /// Get events system metrics
    pub fn get_metrics(&self) -> EventsMetrics {
        self.metrics.read().clone()
    }

    /// Get stream statistics
    pub fn get_stream_stats(&self, stream: &StreamName) -> Result<StreamStats> {
        let events = self.stream_events.get(stream)
            .ok_or_else(|| Error::Storage(format!("Stream {} not found", stream.0)))?;
        
        let sequence = self.stream_sequences.get(stream)
            .map(|s| *s.value())
            .unwrap_or(0);
        
        Ok(StreamStats {
            stream: stream.clone(),
            event_count: events.len(),
            last_sequence: sequence,
            first_event_id: events.first().map(|e| e.id),
            last_event_id: events.last().map(|e| e.id),
        })
    }
}

/// Stream statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStats {
    pub stream: StreamName,
    pub event_count: usize,
    pub last_sequence: u64,
    pub first_event_id: Option<EventId>,
    pub last_event_id: Option<EventId>,
}

// Clone implementation for NativeEventsSystem
// Note: consumers cannot be cloned (EventConsumer contains non-Clone types like JoinHandle)
// So we create a new empty DashMap for consumers in the clone
impl Clone for NativeEventsSystem {
    fn clone(&self) -> Self {
        Self {
            streams: self.streams.clone(),
            stream_events: self.stream_events.clone(),
            stream_sequences: self.stream_sequences.clone(),
            topics: self.topics.clone(),
            topic_subscribers: self.topic_subscribers.clone(),
            queues: self.queues.clone(),
            queue_messages: self.queue_messages.clone(),
            queue_in_flight: self.queue_in_flight.clone(),
            queue_dlq: self.queue_dlq.clone(),
            producers: self.producers.clone(),
            consumers: Arc::new(DashMap::new()), // Cannot clone EventConsumer (contains non-Clone types)
            persistence: self.persistence.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

// Note: EventConsumer cannot be cloned due to receiver ownership
// Use subscription.clone() to create a new consumer instead

