# narayana-spk: Speech Synthesis for Robots

Text-to-speech (TTS) capabilities for robots, integrated with the Narayana cognitive architecture.

## Features

- **Native TTS Engines**: Platform-specific TTS (macOS NSSpeechSynthesizer, Linux espeak-ng, Windows SAPI)
- **Optional API TTS**: Support for OpenAI TTS, Google Cloud TTS, Amazon Polly (when implemented)
- **Brain Integration**: Plugs into narayana-wld as a ProtocolAdapter
- **Configurable**: Off by default, can be enabled per CPL/brain
- **Caching**: Audio caching for frequently spoken text
- **Queue Management**: Async speech synthesis with request queuing

## Configuration

Speech synthesis is **disabled by default**. To enable:

```rust
use narayana_spk::SpeechConfig;

let mut config = SpeechConfig::default();
config.enabled = true;
config.engine = narayana_spk::config::TtsEngine::Native;
config.voice.language = "en-US".to_string();
config.rate = 150; // Words per minute
config.volume = 0.8; // 0.0 to 1.0
```

## Integration with narayana-wld

The `SpeechAdapter` implements `ProtocolAdapter` and can be registered with the world broker:

```rust
use narayana_spk::SpeechAdapter;
use narayana_wld::world_broker::WorldBroker;

let config = SpeechConfig::default();
let adapter = SpeechAdapter::new(config)?;
let broker = WorldBroker::new();
adapter.start(broker.handle()).await?;
```

## CPL Settings

CPLs (Conscience Persistent Loops) can have speech settings that cascade to their brain:

```rust
// CPL speech setting would be stored in the cognitive brain
// and automatically applied when the speech adapter starts
```

## Usage

### Basic Synthesis

```rust
use narayana_spk::{SpeechSynthesizer, SpeechConfig};

let config = SpeechConfig {
    enabled: true,
    ..Default::default()
};

let synthesizer = SpeechSynthesizer::new(config)?;
let audio = synthesizer.speak("Hello, world!").await?;
```

### With Custom Voice

```rust
use narayana_spk::config::VoiceConfig;

let voice = VoiceConfig {
    language: "en-US".to_string(),
    name: Some("Alex".to_string()),
    ..Default::default()
};

let audio = synthesizer.speak_with_config("Hello!", &voice).await?;
```

## Platform Support

- **macOS**: Uses NSSpeechSynthesizer (built-in)
- **Linux**: Requires espeak-ng (`sudo apt-get install espeak-ng`)
- **Windows**: Uses SAPI (built-in)

## CPL Integration

CPLs can enable speech synthesis and configure it:

```rust
use narayana_storage::conscience_persistent_loop::CPLConfig;
use narayana_spk::cpl_integration::create_speech_adapter_from_cpl;

let mut cpl_config = CPLConfig::default();
cpl_config.enable_speech = true;
cpl_config.speech_config = Some(serde_json::json!({
    "rate": 150,
    "volume": 0.8,
    "voice": {
        "language": "en-US"
    }
}));

// Create adapter from CPL config
if let Ok(Some(adapter)) = create_speech_adapter_from_cpl(&cpl_config) {
    // Use adapter with world broker
}
```

## Future Features

- OpenAI TTS API integration
- Google Cloud TTS integration
- Amazon Polly integration
- Piper TTS (local neural TTS)
- SSML support
- Audio format conversion
- Real-time streaming synthesis
- macOS AVAudioEngine integration for better audio output

## Security

- Input validation (text length limits, sanitization)
- HTTPS-only for API endpoints
- Rate limiting for API calls
- Cache size limits to prevent DoS
- Text sanitization (removes control characters)

