//! WebSocket bridge for streaming avatar to web clients

use crate::avatar_broker::AvatarBroker;
use crate::multimodal::MultimodalManager;
#[cfg(feature = "llm")]
use narayana_llm::LLMManager;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use narayana_core::Error;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{info, debug, warn, error};
use uuid::Uuid;

/// WebSocket bridge for avatar streaming
pub struct AvatarBridge {
    broker: Arc<RwLock<AvatarBroker>>,
    clients: Arc<RwLock<Vec<broadcast::Sender<BridgeMessage>>>>,
    multimodal_manager: Arc<MultimodalManager>,
    #[cfg(feature = "llm")]
    llm_manager: Option<Arc<LLMManager>>,
    port: u16,
}

/// Messages sent to connected clients
#[derive(Debug, Clone, serde::Serialize)]
pub enum BridgeMessage {
    /// Expression change
    Expression {
        emotion: String,
        intensity: f64,
    },
    /// Gesture
    Gesture {
        gesture: String,
        duration_ms: u64,
    },
    /// Stream state
    State {
        state: String, // "thinking", "speaking", "idle"
    },
    /// Stream URL update
    StreamUrl {
        url: String,
    },
    /// Audio data (for client-side lip sync fallback)
    Audio {
        data: Vec<u8>,
    },
    /// TTS audio output (for avatar speech)
    TTSAudio {
        data: Vec<u8>,
        format: String, // "wav", "pcm", "opus"
    },
    /// TTS request (text to convert to speech)
    TTSRequest {
        text: String,
    },
}

/// Messages received from clients
#[derive(Debug, Clone, serde::Deserialize)]
pub enum ClientMessage {
    /// Video frame from camera
    VideoFrame {
        data: Vec<u8>,
        width: u32,
        height: u32,
        timestamp: u64,
    },
    /// Audio sample from microphone
    AudioSample {
        data: Vec<u8>,
        sample_rate: u32,
        channels: u8,
        timestamp: u64,
    },
    /// Request TTS audio for text
    TTSRequest {
        text: String,
    },
}

impl AvatarBridge {
    pub fn new(
        broker: Arc<RwLock<AvatarBroker>>,
        multimodal_manager: Arc<MultimodalManager>,
        #[cfg(feature = "llm")]
        llm_manager: Option<Arc<LLMManager>>,
        port: u16,
    ) -> Self {
        Self {
            broker,
            clients: Arc::new(RwLock::new(Vec::new())),
            multimodal_manager,
            #[cfg(feature = "llm")]
            llm_manager,
            port,
        }
    }

    pub async fn start(&self) -> Result<(), Error> {
        let port = self.port;
        let app = Router::new()
            .route("/avatar/ws", get(websocket_handler))
            .with_state(BridgeState {
                broker: Arc::clone(&self.broker),
                clients: Arc::clone(&self.clients),
                multimodal_manager: Arc::clone(&self.multimodal_manager),
                #[cfg(feature = "llm")]
                llm_manager: self.llm_manager.clone(),
            });
        let addr = format!("0.0.0.0:{}", port);
        info!("Starting avatar bridge on {}", addr);
        let listener = tokio::net::TcpListener::bind(&addr).await
            .map_err(|e| Error::Storage(format!("Failed to bind to {}: {}", addr, e)))?;
        axum::serve(listener, app).await
            .map_err(|e| Error::Storage(format!("Avatar bridge server error: {}", e)))?;
        Ok(())
    }

    pub async fn broadcast(&self, message: BridgeMessage) {
        const MAX_CLIENTS: usize = 10_000;
        
        // Collect clients to process (clone senders to avoid holding lock during send)
        let client_senders: Vec<_> = {
            let clients = self.clients.read().await;
            if clients.len() > MAX_CLIENTS {
                warn!("Too many clients connected ({}), limiting broadcast", clients.len());
            }
            clients.iter().take(MAX_CLIENTS).cloned().collect()
        };
        
        // Broadcast to all clients without holding lock
        let mut disconnected_indices = Vec::new();
        for (idx, client) in client_senders.iter().enumerate() {
            if client.send(message.clone()).is_err() {
                disconnected_indices.push(idx);
            }
        }
        
        // Remove disconnected clients by checking receiver_count (race-safe)
        if !disconnected_indices.is_empty() {
            let mut clients_mut = self.clients.write().await;
            // Remove in reverse order to preserve indices
            clients_mut.retain(|c| c.receiver_count() > 1); // Keep only active receivers (> 1 because we count ourselves)
        }
    }
}

/// Bridge state passed to handlers
#[derive(Clone)]
struct BridgeState {
    broker: Arc<RwLock<AvatarBroker>>,
    clients: Arc<RwLock<Vec<broadcast::Sender<BridgeMessage>>>>,
    multimodal_manager: Arc<MultimodalManager>,
    #[cfg(feature = "llm")]
    llm_manager: Option<Arc<LLMManager>>,
}

/// WebSocket handler
async fn websocket_handler(ws: WebSocketUpgrade, State(state): State<BridgeState>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: BridgeState) {
    use futures_util::StreamExt;
    
    let client_id = Uuid::new_v4();
    info!("New avatar client connected: {}", client_id);

    // Create channel for this client
    let (tx, mut rx) = broadcast::channel::<BridgeMessage>(100);
    
    // Add to clients list (with size limit)
    {
        let mut clients = state.clients.write().await;
        const MAX_CLIENTS: usize = 10_000;
        if clients.len() >= MAX_CLIENTS {
            warn!("Maximum client limit reached ({}), rejecting new connection", MAX_CLIENTS);
            return; // Reject connection
        }
        clients.push(tx.clone());
    }

    // Split socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();
    info!("Client {}: Socket split successfully", client_id);
    
    // Spawn task to send messages to client
    info!("Client {}: Starting send task", client_id);
    let mut send_task = tokio::spawn(async move {
        use futures_util::SinkExt;
        
        // Send welcome message immediately
        let welcome_msg = BridgeMessage::State {
            state: "connected".to_string(),
        };
        if let Ok(welcome_json) = serde_json::to_string(&welcome_msg) {
            match sender.send(axum::extract::ws::Message::Text(welcome_json)).await {
                Ok(_) => {
                    info!("Client {}: Welcome message sent successfully", client_id);
                }
                Err(e) => {
                    warn!("Client {}: Failed to send welcome message: {}, closing connection", client_id, e);
                    return;
                }
            }
        } else {
            warn!("Client {}: Failed to serialize welcome message", client_id);
        }
        
        loop {
            // Use timeout to prevent hanging on receive
            match tokio::time::timeout(
                std::time::Duration::from_secs(300), // 5 minute timeout
                rx.recv()
            ).await {
                Ok(Ok(msg)) => {
                    let json = match serde_json::to_string(&msg) {
                        Ok(j) => j,
                        Err(e) => {
                            warn!("Failed to serialize bridge message: {}", e);
                            continue;
                        }
                    };
                    
                    // Validate message size before sending
                    const MAX_WS_MESSAGE_SIZE: usize = 1_000_000; // 1MB max
                    if json.len() > MAX_WS_MESSAGE_SIZE {
                        warn!("Message too large to send ({} bytes), skipping", json.len());
                        continue;
                    }
                    
                    if sender.send(axum::extract::ws::Message::Text(json)).await.is_err() {
                        debug!("Client {}: WebSocket send failed, closing sender task", client_id);
                        break;
                    }
                }
                Ok(Err(broadcast::error::RecvError::Closed)) => {
                    debug!("Client {}: Bridge message channel closed, closing sender task", client_id);
                    break;
                }
                Ok(Err(broadcast::error::RecvError::Lagged(skipped))) => {
                    warn!("Client {}: Bridge message channel lagged, skipped {} messages", client_id, skipped);
                    // Continue, but client might have missed some updates
                }
                Err(_) => {
                    // Timeout - send ping to keep connection alive
                    debug!("Client {}: Send task timeout, sending ping", client_id);
                    // Ping is handled automatically by WebSocket library
                }
            }
        }
    });

    // Subscribe to TTS audio from MultimodalManager
    let mut tts_audio_receiver = state.multimodal_manager.subscribe_tts_audio();

    // Spawn task to receive messages from client
    info!("Client {}: Starting receive task", client_id);
    let broker_arc = Arc::clone(&state.broker);
    let multimodal_manager_arc = Arc::clone(&state.multimodal_manager);
    #[cfg(feature = "llm")]
    let llm_manager_arc = state.llm_manager.as_ref().map(Arc::clone);
    let clients_for_recv_task = Arc::clone(&state.clients);
    
    let mut recv_task = tokio::spawn(async move {
        loop {
            match tokio::time::timeout(
                std::time::Duration::from_secs(300), // 5 minute timeout
                receiver.next()
            ).await {
                Ok(Some(Ok(msg))) => {
                    match msg {
                        Message::Text(text) => {
                            const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;
                            if text.len() > MAX_MESSAGE_SIZE {
                                warn!("Client {}: WebSocket message too large ({} bytes, max {} bytes), closing connection", client_id, text.len(), MAX_MESSAGE_SIZE);
                                break;
                            }
                            
                            debug!("Client {}: Received text message: {} bytes", client_id, text.len());
                            
                            match serde_json::from_str::<ClientMessage>(&text) {
                                Ok(client_msg) => {
                                    match client_msg {
                                        ClientMessage::VideoFrame { data, width, height, timestamp } => {
                                            debug!("Client {}: Received video frame: {}x{} @ {}", client_id, width, height, data.len());
                                            const MAX_FRAME_SIZE: usize = 10 * 1024 * 1024;
                                            if data.len() > MAX_FRAME_SIZE {
                                                warn!("Client {}: Video frame too large ({} bytes), ignoring", client_id, data.len());
                                                continue;
                                            }
                                            
                                            let frame = crate::multimodal::VisionFrame {
                                                data: data.clone(),
                                                width,
                                                height,
                                                timestamp,
                                            };
                                            if let Err(e) = multimodal_manager_arc.send_vision_frame(frame) {
                                                warn!("Client {}: Failed to send vision frame to multimodal manager: {}", client_id, e);
                                            }
                                            
                                            #[cfg(feature = "llm")]
                                            if let Some(ref llm) = llm_manager_arc {
                                                let vision_description = format!("Camera frame received: {}x{} pixels, {} bytes", width, height, data.len());
                                                let llm_clone = Arc::clone(llm);
                                                let clients_clone = Arc::clone(&clients_for_recv_task);
                                                tokio::spawn(async move {
                                                    use narayana_llm::MessageRole;
                                                    let messages = vec![
                                                        narayana_llm::Message {
                                                            role: MessageRole::System,
                                                            content: "You are a helpful avatar assistant with vision capabilities. Describe what you see naturally.".to_string(),
                                                        },
                                                        narayana_llm::Message {
                                                            role: MessageRole::User,
                                                            content: format!("What do you see? {}", vision_description),
                                                        },
                                                    ];
                                                    
                                                    match llm_clone.chat(messages, None).await {
                                                        Ok(response) => {
                                                            debug!("Client {}: LLM vision response: {}", client_id, response);
                                                            let tts_msg = BridgeMessage::TTSRequest {
                                                                text: response,
                                                            };
                                                            let clients = clients_clone.read().await;
                                                            for client in clients.iter() {
                                                                let _ = client.send(tts_msg.clone());
                                                            }
                                                        }
                                                        Err(e) => {
                                                            warn!("Client {}: LLM chat error: {}", client_id, e);
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                        ClientMessage::AudioSample { data, sample_rate, channels, timestamp } => {
                                            debug!("Client {}: Received audio sample: {}Hz, {}ch @ {}", client_id, sample_rate, channels, data.len());
                                            const MAX_AUDIO_SIZE: usize = 1 * 1024 * 1024;
                                            if data.len() > MAX_AUDIO_SIZE {
                                                warn!("Client {}: Audio sample too large ({} bytes), ignoring", client_id, data.len());
                                                continue;
                                            }
                                            
                                            let audio_samples: Vec<f32> = data.iter()
                                                .map(|&byte| (byte as f32 - 128.0) / 128.0)
                                                .collect();
                                            
                                            let frame = crate::multimodal::AudioSample {
                                                data: audio_samples,
                                                sample_rate,
                                                channels,
                                                timestamp,
                                            };
                                            if let Err(e) = multimodal_manager_arc.send_audio_input(frame) {
                                                warn!("Client {}: Failed to send audio input to multimodal manager: {}", client_id, e);
                                            }
                                            
                                            #[cfg(feature = "llm")]
                                            if let Some(ref llm) = llm_manager_arc {
                                                let audio_text = format!("Audio input received: {}Hz, {}ch, {} bytes", sample_rate, channels, data.len());
                                                let llm_clone = Arc::clone(llm);
                                                let clients_clone = Arc::clone(&clients_for_recv_task);
                                                tokio::spawn(async move {
                                                    use narayana_llm::MessageRole;
                                                    let messages = vec![
                                                        narayana_llm::Message {
                                                            role: MessageRole::System,
                                                            content: "You are a helpful avatar assistant. Respond naturally and concisely.".to_string(),
                                                        },
                                                        narayana_llm::Message {
                                                            role: MessageRole::User,
                                                            content: audio_text,
                                                        },
                                                    ];
                                                    
                                                    match llm_clone.chat(messages, None).await {
                                                        Ok(response) => {
                                                            debug!("Client {}: LLM generated response: {}", client_id, response);
                                                            let tts_msg = BridgeMessage::TTSRequest {
                                                                text: response,
                                                            };
                                                            let clients = clients_clone.read().await;
                                                            for client in clients.iter() {
                                                                let _ = client.send(tts_msg.clone());
                                                            }
                                                        }
                                                        Err(e) => {
                                                            warn!("Client {}: LLM chat error: {}", client_id, e);
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                        ClientMessage::TTSRequest { text } => {
                                            debug!("Client {}: Received TTS request (chat message): {} chars", client_id, text.len());
                                            // Validate text length
                                            const MAX_TEXT_LENGTH: usize = 10_000; // 10K chars max
                                            if text.len() > MAX_TEXT_LENGTH {
                                                warn!("Client {}: TTS text too long ({} chars), ignoring", client_id, text.len());
                                                continue;
                                            }
                                            
                                            // Process text through LLM if available
                                            #[cfg(feature = "llm")]
                                            if let Some(ref llm) = llm_manager_arc {
                                                let text_clone = text.clone();
                                                let llm_clone = Arc::clone(llm);
                                                let clients_clone = Arc::clone(&clients_for_recv_task);
                                                let client_id_clone = client_id;
                                                tokio::spawn(async move {
                                                    use narayana_llm::MessageRole;
                                                    let messages = vec![
                                                        narayana_llm::Message {
                                                            role: MessageRole::System,
                                                            content: "You are a helpful avatar assistant. Respond naturally and concisely to user messages.".to_string(),
                                                        },
                                                        narayana_llm::Message {
                                                            role: MessageRole::User,
                                                            content: text_clone,
                                                        },
                                                    ];
                                                    
                                                    match llm_clone.chat(messages, None).await {
                                                        Ok(response) => {
                                                            debug!("Client {}: LLM generated response: {}", client_id_clone, response);
                                                            let tts_msg = BridgeMessage::TTSRequest {
                                                                text: response,
                                                            };
                                                            let clients = clients_clone.read().await;
                                                            for client in clients.iter() {
                                                                let _ = client.send(tts_msg.clone());
                                                            }
                                                        }
                                                        Err(e) => {
                                                            warn!("Client {}: LLM chat error: {}", client_id_clone, e);
                                                        }
                                                    }
                                                });
                                            }
                                            #[cfg(not(feature = "llm"))]
                                            {
                                                debug!("Client {}: LLM not available, echoing message", client_id);
                                                // Echo back if no LLM
                                                let echo_msg = BridgeMessage::TTSRequest {
                                                    text: format!("Echo: {}", text),
                                                };
                                                let clients = clients_for_recv_task.read().await;
                                                for client in clients.iter() {
                                                    let _ = client.send(echo_msg.clone());
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    debug!("Client {}: Failed to parse client message: {}, treating as legacy format", client_id, e);
                                }
                            }
                        }
                        Message::Binary(data) => {
                            const MAX_BINARY_SIZE: usize = 10 * 1024 * 1024;
                            if data.len() > MAX_BINARY_SIZE {
                                warn!("Client {}: WebSocket binary message too large ({} bytes, max {} bytes), closing connection", client_id, data.len(), MAX_BINARY_SIZE);
                                break;
                            }
                            debug!("Client {}: Received binary message: {} bytes", client_id, data.len());
                        }
                        Message::Close(close_frame) => {
                            info!("Client {}: Disconnected gracefully. Close frame: {:?}", client_id, close_frame);
                            break;
                        }
                        Message::Ping(_) => {
                            debug!("Client {}: Received ping", client_id);
                        }
                        Message::Pong(_) => {
                            debug!("Client {}: Received pong", client_id);
                        }
                    }
                }
                Ok(Some(Err(e))) => {
                    warn!("Client {}: WebSocket receive error: {}", client_id, e);
                    break;
                }
                Ok(None) => {
                    break;
                }
                Err(_) => {
                    warn!("Client {}: WebSocket receive timeout, closing connection", client_id);
                    break;
                }
            }
        }
    });

    tokio::select! {
        result = &mut send_task => {
            match result {
                Ok(_) => {
                    debug!("Client {}: Send task completed normally", client_id);
                }
                Err(e) => {
                    warn!("Client {}: Send task panicked or was aborted: {}", client_id, e);
                }
            }
            recv_task.abort();
        }
        result = &mut recv_task => {
            match result {
                Ok(_) => {
                    debug!("Client {}: Receive task completed normally", client_id);
                }
                Err(e) => {
                    warn!("Client {}: Receive task panicked or was aborted: {}", client_id, e);
                }
            }
            send_task.abort();
        }
    }

    {
        let mut clients = state.clients.write().await;
        let initial_len = clients.len();
        clients.retain(|c| c.receiver_count() > 1);
        let final_len = clients.len();
        if initial_len != final_len {
            debug!("Cleaned up {} disconnected client(s) for client {}", initial_len - final_len, client_id);
        }
    }

    info!("Client {}: Avatar client disconnected", client_id);
}
