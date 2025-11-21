//! Avatar adapter for narayana-wld integration

use crate::config::AvatarConfig;
use crate::error::AvatarError;
use crate::avatar_broker::AvatarBroker;
use narayana_wld::protocol_adapters::ProtocolAdapter;
use narayana_wld::world_broker::WorldBrokerHandle;
use narayana_wld::event_transformer::{WorldEvent, WorldAction};
use narayana_core::Error;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use parking_lot::RwLock as SyncRwLock;
use tracing::{info, warn, debug};

/// Avatar adapter implementing ProtocolAdapter for narayana-wld
pub struct AvatarAdapter {
    broker: Arc<RwLock<AvatarBroker>>,
    event_sender: Arc<SyncRwLock<Option<broadcast::Sender<WorldEvent>>>>,  // Sync for subscribe_events
    is_running: Arc<RwLock<bool>>,
    processing_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl AvatarAdapter {
    /// Create a new avatar adapter
    pub fn new(config: AvatarConfig) -> Result<Self, Error> {
        config.validate()
            .map_err(|e| Error::Storage(format!("Invalid avatar config: {}", e)))?;

        let broker = Arc::new(RwLock::new(
            AvatarBroker::new(config.clone())
                .map_err(|e| Error::Storage(format!("Failed to create avatar broker: {}", e)))?
        ));

        Ok(Self {
            broker,
            event_sender: Arc::new(SyncRwLock::new(None)),
            is_running: Arc::new(RwLock::new(false)),
            processing_handle: Arc::new(RwLock::new(None)),
        })
    }
}

#[async_trait]
impl ProtocolAdapter for AvatarAdapter {
    fn protocol_name(&self) -> &str {
        "avatar"
    }

    async fn start(&self, broker: WorldBrokerHandle) -> Result<(), Error> {
        // Check if already running
        {
            let mut is_running = self.is_running.write().await;
            if *is_running {
                return Err(Error::Storage("Avatar adapter already running".to_string()));
            }
            *is_running = true;
        }

        info!("Starting avatar adapter");

        // Initialize broker (clone Arc before await)
        {
            let broker_arc = Arc::clone(&self.broker);
            let broker = broker_arc.read().await;
            broker.initialize().await
                .map_err(|e| Error::Storage(format!("Failed to initialize avatar broker: {}", e)))?;
        }

        // Create event channel
        const EVENT_BUFFER_SIZE: usize = 1000;
        let (sender, _receiver) = broadcast::channel(EVENT_BUFFER_SIZE);
        
        // Set event sender (sync lock for non-async subscribe_events)
        *self.event_sender.write() = Some(sender.clone());

        // Subscribe to actions from broker (world actions that affect avatar)
        let action_receiver = broker.subscribe_actions();
        let _broker_weak = Arc::downgrade(&self.broker);
        let _event_sender_weak = Arc::downgrade(&self.event_sender);

        // Spawn task to process world actions
        // Note: This task listens to actions but actual processing happens in send_action()
        // This is kept for future extensibility (e.g., automatic emotion detection)
        let handle = tokio::spawn(async move {
            let mut action_receiver = action_receiver;
            
            loop {
                tokio::select! {
                    result = action_receiver.recv() => {
                        match result {
                            Ok(_action) => {
                                // Actions are processed via send_action() method
                                // This loop is kept for future use (e.g., monitoring, logging)
                                debug!("Received world action (processed via send_action)");
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                // Channel closed
                                debug!("World action channel closed, stopping listener");
                                break;
                            }
                            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                                warn!("World action receiver lagged, skipped {} messages", skipped);
                                // Continue receiving
                            }
                        }
                    }
                }
            }
            debug!("World action listener task stopped");
        });

        *self.processing_handle.write().await = Some(handle);

        // Start avatar stream if enabled (clone Arc before await)
        {
            let broker_arc = Arc::clone(&self.broker);
            let broker = broker_arc.read().await;
            match broker.start_stream().await {
                Ok(client_url) => {
                    info!("Avatar stream started: {}", client_url);
                    
                    // Emit event with proper timestamp handling
                    let timestamp = chrono::Utc::now()
                        .timestamp_nanos_opt()
                        .unwrap_or_else(|| {
                            // Fallback to timestamp_millis if nanos not available
                            chrono::Utc::now().timestamp_millis() as i64 * 1_000_000
                        }) as u64;
                    
                    let event = WorldEvent::SensorData {
                        source: "avatar".to_string(),
                        data: json!({
                            "type": "stream_started",
                            "client_url": client_url,
                            "timestamp": timestamp,
                        }),
                        timestamp,
                    };
                    
                    if sender.send(event).is_err() {
                        warn!("Failed to send avatar event (channel full)");
                    }
                }
                Err(e) => {
                    warn!("Failed to start avatar stream: {} (adapter will continue without stream)", e);
                    // Don't fail the entire start() - adapter can still process actions
                }
            }
        }

        info!("Avatar adapter started successfully");
        Ok(())
    }

    async fn stop(&self) -> Result<(), Error> {
        {
            let mut is_running = self.is_running.write().await;
            if !*is_running {
                return Ok(()); // Already stopped
            }
            *is_running = false;
        }

        // Stop avatar stream (clone Arc before await)
        {
            let broker_arc = Arc::clone(&self.broker);
            {
                let broker = broker_arc.read().await;
                if let Err(e) = broker.stop_stream().await {
                    warn!("Failed to stop avatar stream: {}", e);
                }
            } // Drop guard
        }

        // Stop processing task
        let handle_opt = {
            let mut guard = self.processing_handle.write().await;
            guard.take()
        };
        
        if let Some(handle) = handle_opt {
            handle.abort();
            // Wait for task to finish (with timeout)
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(2), // Give it more time to clean up
                handle
            ).await;
            // Note: We ignore timeout errors here - task might take longer, but abort() should stop it
        }

        // Clear event sender
        *self.event_sender.write() = None;

        info!("Avatar adapter stopped");
        Ok(())
    }

    async fn send_action(&self, action: WorldAction) -> Result<(), Error> {
        // Handle avatar commands
        match action {
            WorldAction::ActuatorCommand { target, command } => {
                if target == "avatar" || target.starts_with("avatar_") {
                    debug!("Received avatar command: {:?}", command);
                    
                    // Validate command JSON size to prevent DoS
                    let command_size = match serde_json::to_string(&command) {
                        Ok(s) => s.len(),
                        Err(e) => {
                            warn!("Failed to serialize command for size check: {}", e);
                            // Use a safe estimate: assume it's large if we can't serialize
                            const MAX_COMMAND_SIZE: usize = 10_000;
                            return Err(Error::Storage(format!("Command too large or invalid (max {} bytes)", MAX_COMMAND_SIZE)));
                        }
                    };
                    const MAX_COMMAND_SIZE: usize = 10_000; // 10KB max
                    if command_size > MAX_COMMAND_SIZE {
                        warn!("Avatar command too large ({} bytes, max {} bytes), ignoring", command_size, MAX_COMMAND_SIZE);
                        return Ok(());
                    }
                    
                    // Parse avatar commands
                    if let Some(cmd_type) = command.get("type").and_then(|v| v.as_str()) {
                        // Validate cmd_type length
                        if cmd_type.len() > 64 {
                            warn!("Command type too long, ignoring");
                            return Ok(());
                        }
                        
                        match cmd_type {
                            "expression" => {
                                if let (Some(expr_str), intensity_opt) = (
                                    command.get("expression").and_then(|v| v.as_str()),
                                    command.get("intensity").and_then(|v| v.as_f64())
                                ) {
                                    // Validate expression string
                                    if expr_str.len() > 256 {
                                        warn!("Expression string too long, ignoring");
                                        return Ok(());
                                    }
                                    
                                    // Validate intensity
                                    let intensity = if let Some(i) = intensity_opt {
                                        if !i.is_finite() || !(0.0..=1.0).contains(&i) {
                                            warn!("Invalid intensity value, using default 0.7");
                                            0.7
                                        } else {
                                            i
                                        }
                                    } else {
                                        0.7
                                    };
                                    
                                    let expression = parse_expression(expr_str);
                                    let broker_arc = Arc::clone(&self.broker);
                                    {
                                        let broker = broker_arc.read().await;
                                        if let Err(e) = broker.set_expression(expression, intensity).await {
                                            warn!("Failed to set expression: {}", e);
                                        }
                                    } // Drop lock after await
                                }
                            }
                            "gesture" => {
                                if let (Some(gesture_str), duration_opt) = (
                                    command.get("gesture").and_then(|v| v.as_str()),
                                    command.get("duration_ms").and_then(|v| v.as_u64())
                                ) {
                                    // Validate gesture string
                                    if gesture_str.len() > 256 {
                                        warn!("Gesture string too long, ignoring");
                                        return Ok(());
                                    }
                                    
                                    // Validate duration (prevent excessive values)
                                    const MAX_DURATION_MS: u64 = 300_000; // 5 minutes max
                                    let duration = duration_opt.unwrap_or(1000).min(MAX_DURATION_MS);
                                    
                                    let gesture = parse_gesture(gesture_str);
                                    let broker_arc = Arc::clone(&self.broker);
                                    {
                                        let broker = broker_arc.read().await;
                                        if let Err(e) = broker.set_gesture(gesture, duration).await {
                                            warn!("Failed to set gesture: {}", e);
                                        }
                                    } // Drop lock after await
                                }
                            }
                            _ => {
                                warn!("Unknown avatar command type: {}", cmd_type);
                            }
                        }
                    }
                }
            }
            _ => {
                // Other actions not handled by avatar adapter
            }
        }

        Ok(())
    }

    fn subscribe_events(&self) -> broadcast::Receiver<WorldEvent> {
        // Use read() on parking_lot::RwLock (non-blocking for readers, but can't use async)
        // This is a limitation - for proper async support, would need a different pattern
        let sender_guard = self.event_sender.read();
        if let Some(ref sender) = *sender_guard {
            sender.subscribe()
        } else {
            // Return closed channel if not started
            let (_, receiver) = broadcast::channel(1);
            receiver
        }
    }
}


fn parse_expression(s: &str) -> crate::config::Expression {
    // Validate input size to prevent DoS
    const MAX_EXPRESSION_STRING_LEN: usize = 256;
    let s_trimmed = s.trim();
    if s_trimmed.len() > MAX_EXPRESSION_STRING_LEN {
        warn!("Expression string too long ({} chars, max {}), truncating", s_trimmed.len(), MAX_EXPRESSION_STRING_LEN);
        let truncated = s_trimmed.chars().take(MAX_EXPRESSION_STRING_LEN).collect::<String>();
        return crate::config::Expression::Custom(truncated);
    }
    
    match s_trimmed.to_lowercase().as_str() {
        "neutral" => crate::config::Expression::Neutral,
        "happy" => crate::config::Expression::Happy,
        "sad" => crate::config::Expression::Sad,
        "angry" => crate::config::Expression::Angry,
        "surprised" => crate::config::Expression::Surprised,
        "thinking" => crate::config::Expression::Thinking,
        "confused" => crate::config::Expression::Confused,
        "excited" => crate::config::Expression::Excited,
        "tired" => crate::config::Expression::Tired,
        "recognition" => crate::config::Expression::Recognition,
        other => {
            // Validate custom expression characters (alphanumeric, dash, underscore only)
            if other.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                crate::config::Expression::Custom(other.to_string())
            } else {
                warn!("Invalid characters in expression string, using neutral");
                crate::config::Expression::Neutral
            }
        }
    }
}

fn parse_gesture(s: &str) -> crate::config::Gesture {
    // Validate input size to prevent DoS
    const MAX_GESTURE_STRING_LEN: usize = 256;
    let s_trimmed = s.trim();
    if s_trimmed.len() > MAX_GESTURE_STRING_LEN {
        warn!("Gesture string too long ({} chars, max {}), truncating", s_trimmed.len(), MAX_GESTURE_STRING_LEN);
        let truncated = s_trimmed.chars().take(MAX_GESTURE_STRING_LEN).collect::<String>();
        return crate::config::Gesture::Custom(truncated);
    }
    
    match s_trimmed.to_lowercase().as_str() {
        "none" => crate::config::Gesture::None,
        "wave" => crate::config::Gesture::Wave,
        "point" => crate::config::Gesture::Point,
        "nod" => crate::config::Gesture::Nod,
        "shake" => crate::config::Gesture::Shake,
        "thumbs_up" | "thumbsup" => crate::config::Gesture::ThumbsUp,
        other => {
            // Validate custom gesture characters (alphanumeric, dash, underscore only)
            if other.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                crate::config::Gesture::Custom(other.to_string())
            } else {
                warn!("Invalid characters in gesture string, using None");
                crate::config::Gesture::None
            }
        }
    }
}

