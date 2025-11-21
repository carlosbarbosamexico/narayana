//! Basic speech synthesis example

use narayana_spk::{SpeechConfig, SpeechSynthesizer};
use narayana_spk::config::VoiceConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Create speech config (enabled by default for example)
    let mut config = SpeechConfig::default();
    config.enabled = true;
    config.engine = narayana_spk::config::TtsEngine::Native;

    // Create synthesizer
    let synthesizer = SpeechSynthesizer::new(config)?;

    // Synthesize some text
    println!("Synthesizing speech...");
    let text = "Hello, I am a robot. I can speak using text to speech synthesis.";
    
    match synthesizer.speak(text).await {
        Ok(audio) => {
            println!("Successfully synthesized {} bytes of audio", audio.len());
            // In a real application, you would play this audio
        }
        Err(e) => {
            eprintln!("Failed to synthesize speech: {}", e);
        }
    }

    Ok(())
}

