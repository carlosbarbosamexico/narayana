//! Configuration for avatar rendering

use serde::{Deserialize, Serialize};

/// Avatar rendering configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AvatarConfig {
    /// Enable avatar rendering (off by default)
    pub enabled: bool,

    /// Avatar provider type
    pub provider: AvatarProviderType,

    /// Provider-specific configuration (JSON)
    pub provider_config: Option<serde_json::Value>,

    /// Expression sensitivity (0.0-1.0, default 0.7)
    pub expression_sensitivity: f64,

    /// Animation speed multiplier (0.5-2.0, default 1.0)
    pub animation_speed: f64,

    /// WebSocket port for streaming (default: auto-assigned)
    pub websocket_port: Option<u16>,

    /// Enable lip sync
    pub enable_lip_sync: bool,

    /// Enable gestures
    pub enable_gestures: bool,

    /// Avatar model/ID (provider-specific)
    pub avatar_id: Option<String>,

    /// Enable vision (camera input)
    pub enable_vision: bool,

    /// Enable audio input (microphone/hearing)
    pub enable_audio_input: bool,

    /// Enable text-to-speech (voice output)
    pub enable_tts: bool,

    /// Vision configuration (camera ID, resolution, FPS)
    pub vision_config: Option<serde_json::Value>,

    /// Audio input configuration (sample rate, channels, device)
    pub audio_input_config: Option<serde_json::Value>,

    /// TTS configuration (voice, rate, volume)
    pub tts_config: Option<serde_json::Value>,
}

/// Avatar provider type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AvatarProviderType {
    /// Beyond Presence Genesis 1.0 (hyper-realistic)
    BeyondPresence,
    /// LiveAvatar (hyper-realistic)
    LiveAvatar,
    /// Ready Player Me (customizable)
    ReadyPlayerMe,
    /// Avatar SDK (selfie-based)
    AvatarSDK,
    /// OpenAvatarChat (open source)
    OpenAvatarChat,
}

impl Default for AvatarConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Off by default
            provider: AvatarProviderType::BeyondPresence,
            provider_config: None,
            expression_sensitivity: 0.7,
            animation_speed: 1.0,
            websocket_port: None,
            enable_lip_sync: true,
            enable_gestures: true,
            avatar_id: None,
            enable_vision: false,
            enable_audio_input: false,
            enable_tts: false,
            vision_config: None,
            audio_input_config: None,
            tts_config: None,
        }
    }
}

impl AvatarConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if !(0.0..=1.0).contains(&self.expression_sensitivity) {
            return Err("Expression sensitivity must be between 0.0 and 1.0".to_string());
        }

        if !(0.5..=2.0).contains(&self.animation_speed) {
            return Err("Animation speed must be between 0.5 and 2.0".to_string());
        }

        if let Some(port) = self.websocket_port {
            if port == 0 {
                return Err("WebSocket port cannot be 0".to_string());
            }
            if port > 65535 {
                return Err("WebSocket port out of range (max 65535)".to_string());
            }
        }

        if let Some(ref avatar_id) = self.avatar_id {
            if avatar_id.is_empty() {
                return Err("Avatar ID cannot be empty if provided".to_string());
            }
            if avatar_id.len() > 256 {
                return Err("Avatar ID too long (max 256 chars)".to_string());
            }
            if avatar_id.chars().any(|c| c == '\0' || c.is_control()) {
                return Err("Avatar ID contains invalid characters".to_string());
            }
        }

        // Validate provider config if present
        if let Some(ref provider_config) = self.provider_config {
            // Ensure it's a valid JSON object
            if !provider_config.is_object() {
                return Err("Provider config must be a JSON object".to_string());
            }
            
            // Validate provider config size to prevent DoS
            let config_size = serde_json::to_string(provider_config)
                .map(|s| s.len())
                .unwrap_or(0);
            const MAX_PROVIDER_CONFIG_SIZE: usize = 100_000; // 100KB max
            if config_size > MAX_PROVIDER_CONFIG_SIZE {
                return Err(format!("Provider config too large (max {} bytes)", MAX_PROVIDER_CONFIG_SIZE));
            }
            
            // Validate depth to prevent stack overflow from deeply nested JSON
            let depth = count_json_depth(provider_config);
            const MAX_JSON_DEPTH: usize = 32;
            if depth > MAX_JSON_DEPTH {
                return Err(format!("Provider config too deeply nested (max depth {})", MAX_JSON_DEPTH));
            }
        }

        Ok(())
    }
}

/// Count JSON depth to prevent stack overflow from deeply nested structures
fn count_json_depth(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Object(map) => {
            if map.is_empty() {
                1
            } else {
                1 + map.values().map(count_json_depth).max().unwrap_or(0)
            }
        }
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                1
            } else {
                1 + arr.iter().map(count_json_depth).max().unwrap_or(0)
            }
        }
        _ => 1,
    }
}

/// Facial expression types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Expression {
    /// Neutral/resting face
    Neutral,
    /// Happy/smiling
    Happy,
    /// Sad/frowning
    Sad,
    /// Angry
    Angry,
    /// Surprised
    Surprised,
    /// Thinking/contemplating
    Thinking,
    /// Confused
    Confused,
    /// Excited
    Excited,
    /// Tired/sleepy
    Tired,
    /// Recognition/understanding
    Recognition,
    /// Custom expression (string identifier)
    Custom(String),
}

/// Gesture types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Gesture {
    /// No gesture
    None,
    /// Wave hand
    Wave,
    /// Point
    Point,
    /// Nod head
    Nod,
    /// Shake head
    Shake,
    /// Thumbs up
    ThumbsUp,
    /// Custom gesture (string identifier)
    Custom(String),
}

/// Emotion types for CPL integration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Emotion {
    /// Joy
    Joy,
    /// Sadness
    Sadness,
    /// Anger
    Anger,
    /// Fear
    Fear,
    /// Surprise
    Surprise,
    /// Disgust
    Disgust,
    /// Contempt
    Contempt,
    /// Neutral
    Neutral,
    /// Interest/curiosity
    Interest,
    /// Recognition/understanding
    Recognition,
    /// Thinking/processing
    Thinking,
}

impl Emotion {
    /// Convert emotion to expression
    pub fn to_expression(&self) -> Expression {
        match self {
            Emotion::Joy => Expression::Happy,
            Emotion::Sadness => Expression::Sad,
            Emotion::Anger => Expression::Angry,
            Emotion::Surprise => Expression::Surprised,
            Emotion::Disgust => Expression::Confused,
            Emotion::Neutral => Expression::Neutral,
            Emotion::Recognition => Expression::Recognition,
            Emotion::Thinking => Expression::Thinking,
            Emotion::Interest => Expression::Thinking,
            _ => Expression::Neutral,
        }
    }
}


