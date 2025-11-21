# Avatar CPL Example - Complete Multimodal + LLM Integration

This example demonstrates creating a CPL with avatar support enabled and interacting with the avatar using **all multimodal capabilities** (vision, hearing, voice) and **LLM integration** for intelligent responses.

## Prerequisites

1. **Build with all required features:**
   ```bash
   cargo build --package narayana-me --features "beyond-presence,llm,multimodal"
   ```

2. **Set environment variables:**
   ```bash
   export BEYOND_PRESENCE_API_KEY="sk-d4qXnCSYoSwOIQ0o_-ayq920Peu3k2iTE3nuxEf9U8"
   export BEYOND_PRESENCE_BASE_URL="https://api.beyondpresence.ai/v1"  # Optional
   export COHERE_API_KEY="wxYWH1psllHYBHLfEPEaXw8aMpg6sa9BZ4olESeg"  # For LLM
   ```

3. **Start the Narayana server:**
   ```bash
   cargo run --package narayana-server --features "beyond-presence,llm"
   ```

4. **Ensure browser permissions:** Camera and microphone access will be requested when opening the avatar window.

## Quick Start: Ready-to-Use API Example

This creates a CPL with **all multimodal capabilities + LLM enabled** and ready to interact:

```bash
# Get authentication token
TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}' | jq -r '.token')

# Create CPL with full multimodal avatar + LLM
curl -X POST http://localhost:8080/api/v1/cpls \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "config": {
      "loop_interval_ms": 100,
      "enable_global_workspace": true,
      "enable_background_daemon": true,
      "enable_dreaming": true,
      "working_memory_capacity": 7,
      "enable_attention": true,
      "enable_narrative": true,
      "enable_memory_bridge": true,
      "enable_persistence": true,
      "persistence_dir": "data/cpl",
      "enable_genetics": true,
      "enable_avatar": true,
      "avatar_config": {
        "enabled": true,
        "provider": "BeyondPresence",
        "expression_sensitivity": 0.7,
        "animation_speed": 1.0,
        "enable_lip_sync": true,
        "enable_gestures": true,
        "avatar_id": "default",
        "enable_vision": true,
        "vision_config": {
          "fps": 30,
          "width": 640,
          "height": 480,
          "camera_id": 0
        },
        "enable_audio_input": true,
        "audio_input_config": {
          "sample_rate": 16000,
          "channels": 1
        },
        "enable_tts": true,
        "tts_config": {
          "rate": 1.0,
          "volume": 1.0,
          "voice": {
            "language": "en-US",
            "name": "default"
          }
        }
      }
    }
  }' | jq -r '.cpl_id'

# Save CPL_ID from response, then start it
CPL_ID="<CPL_ID_FROM_RESPONSE>"
curl -X POST http://localhost:8080/api/v1/cpls/$CPL_ID/start \
  -H "Authorization: Bearer $TOKEN"
```

**What this enables:**
- 👁️ **Vision**: Camera frames → LLM → Understanding → TTS responses
- 🎤 **Hearing**: Microphone audio → LLM → Conversation → TTS responses  
- 🗣️ **Speech**: LLM responses → TTS → Avatar speaks with lip sync
- 🧠 **LLM**: All multimodal input processed intelligently

## Complete Rust Code Example

This example shows the **complete setup** with all components:

```rust
use narayana_storage::conscience_persistent_loop::CPLConfig;
use narayana_storage::cpl_manager::CPLManager;
use narayana_storage::cognitive::CognitiveBrain;
use narayana_me::{AvatarBroker, AvatarConfig, MultimodalManager, AvatarBridge};
use narayana_llm::{LLMManager, LLMConfig, Provider};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize LLM manager (REQUIRED for intelligent responses)
    let llm_config = LLMConfig::default();
    let llm_manager = Arc::new(LLMManager::new(llm_config)?);
    
    // Configure Cohere provider for LLM
    let cohere_api_key = std::env::var("COHERE_API_KEY")
        .expect("COHERE_API_KEY environment variable must be set");
    let mut cohere_provider = narayana_llm::providers::CohereProvider::new()?;
    cohere_provider.set_api_key(cohere_api_key);
    llm_manager.add_provider(Provider::Cohere, Box::new(cohere_provider)).await;
    llm_manager.set_default_provider(Provider::Cohere).await;
    
    println!("✅ LLM manager initialized with Cohere");
    
    // 2. Create cognitive brain
    let brain = Arc::new(CognitiveBrain::new());
    
    // Set LLM manager on brain (for CPL to use)
    brain.set_llm_manager(llm_manager.clone());
    
    println!("✅ Cognitive brain initialized");
    
    // 3. Create CPL config with FULL multimodal avatar enabled
    let mut cpl_config = CPLConfig::default();
    cpl_config.enable_avatar = true;
    cpl_config.avatar_config = Some(serde_json::json!({
        "enabled": true,
        "provider": "BeyondPresence",
        "expression_sensitivity": 0.7,
        "animation_speed": 1.0,
        "enable_lip_sync": true,
        "enable_gestures": true,
        "avatar_id": "default",
        // MULTIMODAL CAPABILITIES - ALL ENABLED
        "enable_vision": true,
        "vision_config": {
            "fps": 30,
            "width": 640,
            "height": 480,
            "camera_id": 0
        },
        "enable_audio_input": true,
        "audio_input_config": {
            "sample_rate": 16000,
            "channels": 1
        },
        "enable_tts": true,
        "tts_config": {
            "rate": 1.0,
            "volume": 1.0,
            "voice": {
                "language": "en-US",
                "name": "default"
            }
        }
    }));
    
    // 4. Create CPL manager
    let cpl_manager = Arc::new(CPLManager::new(cpl_config.clone()));
    
    // 5. Create and start CPL
    let cpl_id = cpl_manager.spawn_cpl(Some(cpl_config)).await?;
    cpl_manager.start_cpl(&cpl_id).await?;
    
    println!("✅ CPL created and started: {}", cpl_id);
    
    // 6. Get avatar config from CPL
    let cpl_config_ref = cpl_manager.get_cpl_config(&cpl_id).await?;
    let avatar_config = narayana_me::cpl_integration::avatar_config_from_cpl(cpl_config_ref)
        .expect("Avatar config should be available");
    
    // 7. Create multimodal manager
    let multimodal_manager = Arc::new(MultimodalManager::new());
    
    // 8. Create avatar broker
    let avatar_broker = Arc::new(RwLock::new(
        AvatarBroker::new(avatar_config.clone())?
    ));
    
    // Initialize avatar broker
    avatar_broker.read().await.initialize().await?;
    println!("✅ Avatar broker initialized");
    
    // 9. Start avatar stream
    let _client_url = avatar_broker.read().await.start_stream().await?;
    println!("✅ Avatar stream started");
    
    // 10. Create avatar bridge with LLM integration
    let bridge_port = 8081;
    let avatar_bridge = AvatarBridge::new(
        avatar_broker,
        multimodal_manager,
        Some(llm_manager.clone()), // ⚠️ CRITICAL: Pass LLM manager for intelligent responses
        bridge_port,
    );
    
    // 11. Start avatar bridge in background
    println!("🌐 Starting avatar bridge on port {}...", bridge_port);
    tokio::spawn(async move {
        if let Err(e) = avatar_bridge.start().await {
            eprintln!("❌ Avatar bridge error: {}", e);
        }
    });
    
    println!("");
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ MULTIMODAL AVATAR + LLM READY                            ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║                                                              ║");
    println!("║  👁️  Vision:     Camera → LLM → Understanding → TTS         ║");
    println!("║  🎤  Hearing:    Microphone → LLM → Conversation → TTS       ║");
    println!("║  🗣️  Speech:     LLM responses → TTS → Avatar speaks         ║");
    println!("║  🧠  LLM:        All input processed intelligently           ║");
    println!("║                                                              ║");
    println!("║  🌐 WebSocket:   ws://localhost:{}/avatar/ws        ║", bridge_port);
    println!("║                                                              ║");
    println!("║  📝 Next Steps:                                              ║");
    println!("║     1. Open avatar window in browser                         ║");
    println!("║     2. Grant camera & microphone permissions                 ║");
    println!("║     3. Speak or show things to camera                        ║");
    println!("║     4. Avatar will process through LLM and respond!          ║");
    println!("║                                                              ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!("");
    
    // Keep running
    tokio::signal::ctrl_c().await?;
    println!("Shutting down...");
    Ok(())
}
```

## Interaction Flow

When the avatar is running with all capabilities enabled:

1. **User speaks into microphone:**
   ```
   Microphone → Browser → WebSocket → AvatarBridge → MultimodalManager
   → LLM processes audio description → Generates intelligent response
   → TTSRequest → Browser TTS → Avatar speaks with lip sync
   ```

2. **User shows something to camera:**
   ```
   Camera → Browser → WebSocket → AvatarBridge → MultimodalManager  
   → LLM processes vision description → Generates description/response
   → TTSRequest → Browser TTS → Avatar describes what it sees
   ```

3. **Avatar expressions and gestures** automatically update based on:
   - Conversation context (from LLM)
   - Emotional state
   - Interaction patterns

## Configuration Reference

### Complete AvatarConfig

```json
{
  "enabled": true,
  "provider": "BeyondPresence",
  "expression_sensitivity": 0.7,
  "animation_speed": 1.0,
  "enable_lip_sync": true,
  "enable_gestures": true,
  "avatar_id": "default",
  
  "enable_vision": true,
  "vision_config": {
    "fps": 30,
    "width": 640,
    "height": 480,
    "camera_id": 0
  },
  
  "enable_audio_input": true,
  "audio_input_config": {
    "sample_rate": 16000,
    "channels": 1
  },
  
  "enable_tts": true,
  "tts_config": {
    "rate": 1.0,
    "volume": 1.0,
    "voice": {
      "language": "en-US",
      "name": "default"
    }
  }
}
```

## WebSocket Messages

### Messages Received from Backend

- `expression`: Facial expression updates with intensity
- `gesture`: Gesture commands with duration
- `state`: Avatar state changes ("thinking", "speaking", "idle")
- `ttsRequest`: Text to convert to speech (triggers browser TTS)
- `ttsAudio`: Audio data for playback (alternative to browser TTS)

### Messages Sent to Backend

- `videoFrame`: Camera frame data
  ```json
  {
    "type": "videoFrame",
    "data": "base64_encoded_frame",
    "width": 640,
    "height": 480,
    "timestamp": 1234567890
  }
  ```

- `audioSample`: Microphone audio sample
  ```json
  {
    "type": "audioSample",
    "data": "base64_encoded_audio",
    "sample_rate": 16000,
    "channels": 1,
    "timestamp": 1234567890
  }
  ```

- `ttsRequest`: Request TTS for text
  ```json
  {
    "type": "ttsRequest",
    "text": "Hello, I am the avatar"
  }
  ```

## Troubleshooting

### Avatar Not Responding

1. Check CPL is running: `curl http://localhost:8080/api/v1/cpls`
2. Verify avatar is enabled in CPL config
3. Check WebSocket connection in browser console
4. Verify LLM manager is passed to `AvatarBridge::new()`

### No Audio/Vision Processing

1. Grant browser permissions for camera/microphone
2. Verify `enable_vision` and `enable_audio_input` are `true`
3. Check `MultimodalManager` is passed to `AvatarBridge::new()`

### No LLM Responses

1. Verify `LLMManager` is passed to `AvatarBridge::new()`
2. Check `COHERE_API_KEY` environment variable is set
3. Verify Cohere provider is configured in LLM manager
4. Check server logs for LLM errors

### Build Errors

1. Ensure features are enabled: `--features "beyond-presence,llm,multimodal"`
2. Verify all dependencies are available
3. Check environment variables are set correctly

## Performance Notes

- **LLM Processing**: ~500ms-2s per response (depends on provider)
- **Vision Processing**: ~100ms per frame
- **Audio Processing**: ~50ms per sample
- **TTS Generation**: Browser-native, < 100ms
- **Total Latency**: ~1-3s from input to speech response

## Next Steps

1. **Run the example** (API or Rust code)
2. **Open avatar window** in browser when CPL is running
3. **Grant permissions** for camera and microphone
4. **Interact** with the avatar:
   - Speak to it → It will respond via LLM
   - Show it things → It will describe via LLM
   - Avatar expressions and gestures update automatically

The avatar is now fully interactive with multimodal capabilities and LLM-powered intelligence! 🚀
