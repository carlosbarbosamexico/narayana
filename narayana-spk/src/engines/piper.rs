//! Piper TTS engine implementation
//! Local neural TTS using Piper

use crate::config::VoiceConfig;
use crate::error::SpeechError;
use crate::engines::TtsEngine;
use async_trait::async_trait;
use bytes::Bytes;
use std::path::PathBuf;
use std::process::Command;
use tracing::{debug, warn};

/// Piper TTS engine
pub struct PiperTtsEngine {
    piper_path: PathBuf,
    model_path: Option<PathBuf>,
    voices_dir: Option<PathBuf>,
    rate: u32,   // Speech rate (0-500 WPM)
    volume: f32, // Volume (0.0-1.0)
    pitch: f32,  // Pitch (-1.0 to 1.0)
}

impl PiperTtsEngine {
    /// Create a new Piper TTS engine
    pub fn new(
        piper_path: Option<PathBuf>,
        model_path: Option<PathBuf>,
        voices_dir: Option<PathBuf>,
    ) -> Result<Self, SpeechError> {
        Self::new_with_config(piper_path, model_path, voices_dir, 150, 0.8, 0.0)
    }
    
    /// Create a new Piper TTS engine with rate/volume/pitch
    pub fn new_with_config(
        piper_path: Option<PathBuf>,
        model_path: Option<PathBuf>,
        voices_dir: Option<PathBuf>,
        rate: u32,
        volume: f32,
        pitch: f32,
    ) -> Result<Self, SpeechError> {
        // Try to find piper executable
        let piper_path = if let Some(path) = piper_path {
            if !path.exists() {
                return Err(SpeechError::Engine(format!("Piper executable not found at: {:?}", path)));
            }
            path
        } else {
            // Try to find piper in PATH
            let output = Command::new("which")
                .arg("piper")
                .output()
                .ok();
            
            if let Some(output) = output {
                if output.status.success() {
                    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    PathBuf::from(path_str)
                } else {
                    return Err(SpeechError::Engine(
                        "Piper TTS not found. Please install piper-tts or provide piper_path in config".to_string()
                    ));
                }
            } else {
                return Err(SpeechError::Engine(
                    "Piper TTS not found. Please install piper-tts or provide piper_path in config".to_string()
                ));
            }
        };

        Ok(Self {
            piper_path,
            model_path,
            voices_dir,
            rate,
            volume,
            pitch,
        })
    }

    /// Find model file for voice
    fn find_model_file(&self, voice_config: &VoiceConfig) -> Result<PathBuf, SpeechError> {
        // If model_path is explicitly set, use it
        if let Some(ref model_path) = self.model_path {
            if model_path.exists() {
                return Ok(model_path.clone());
            }
        }

        // Try to find model in voices directory
        if let Some(ref voices_dir) = self.voices_dir {
            // Validate voices_dir is absolute to prevent path traversal
            if !voices_dir.is_absolute() {
                return Err(SpeechError::Engine("Voices directory must be an absolute path".to_string()));
            }
            
            // Sanitize voice name/language to prevent path traversal
            let sanitize_model_name = |name: &str| -> String {
                name.chars()
                    .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
                    .take(256) // Limit length
                    .collect()
            };
            
            // Look for model files matching voice name or language
            let model_name = if let Some(ref name) = voice_config.name {
                format!("{}.onnx", sanitize_model_name(name))
            } else {
                format!("{}.onnx", sanitize_model_name(&voice_config.language.replace("-", "_")))
            };

            // Validate model_name doesn't contain path traversal
            if model_name.contains("..") || model_name.contains('/') || model_name.contains('\\') {
                return Err(SpeechError::Engine("Invalid model name (path traversal detected)".to_string()));
            }

            let model_path = voices_dir.join(&model_name);
            
            // Validate that the resolved path is still within voices_dir (prevent path traversal)
            if let (Ok(canon_voices), Ok(canon_model)) = (voices_dir.canonicalize(), model_path.canonicalize()) {
                if !canon_model.starts_with(&canon_voices) {
                    return Err(SpeechError::Engine("Model path traversal detected".to_string()));
                }
            }
            
            if model_path.exists() {
                return Ok(model_path);
            }

            // Try common model names
            let common_models = vec![
                "en_US-lessac-medium.onnx",
                "en_US-lessac-high.onnx",
                "en_US-libritts-high.onnx",
            ];

            for model in common_models {
                let model_path = voices_dir.join(model);
                if model_path.exists() {
                    return Ok(model_path);
                }
            }
        }

        Err(SpeechError::Engine(format!(
            "Piper model not found for voice: {:?}. Please provide model_path or voices_dir in config",
            voice_config.name
        )))
    }
}

#[async_trait]
impl TtsEngine for PiperTtsEngine {
    async fn synthesize(&self, text: &str, config: &VoiceConfig) -> Result<Bytes, SpeechError> {
        use tempfile::NamedTempFile;
        use std::io::Read;

        // Validate input
        if text.is_empty() {
            return Err(SpeechError::Engine("Text cannot be empty".to_string()));
        }

        if text.len() > 100_000 {
            return Err(SpeechError::Engine("Text too long (max 100KB)".to_string()));
        }

        // Find model file
        let model_path = self.find_model_file(config)?;

        // Create temporary output file
        let temp_file = NamedTempFile::new()
            .map_err(|e| SpeechError::Engine(format!("Failed to create temp file: {}", e)))?;
        let output_path = temp_file.path().to_str()
            .ok_or_else(|| SpeechError::Engine("Invalid temp file path".to_string()))?;

        // Sanitize text - remove all control characters and shell metacharacters
        // This prevents command injection when passing text to piper
        let sanitized_text: String = text
            .chars()
            .filter(|c| {
                // Allow printable ASCII and common Unicode characters
                // Block control chars (except newline, tab, carriage return)
                // Block shell metacharacters
                match *c {
                    '\n' | '\r' | '\t' => true, // Allow whitespace
                    c if c.is_control() => false, // Block other control chars
                    ';' | '|' | '&' | '$' | '`' | '(' | ')' | '<' | '>' | '\\' | '"' | '\'' => false, // Block shell metacharacters
                    _ => true, // Allow other characters
                }
            })
            .collect();

        // Execute piper command
        // piper --model model.onnx --output_file output.wav --text "text"
        // Apply rate control via --length_scale (inverse relationship: lower = faster)
        let mut cmd = Command::new(&self.piper_path);
        cmd.arg("--model")
            .arg(model_path.to_str().ok_or_else(|| {
                SpeechError::Engine("Invalid model path".to_string())
            })?)
            .arg("--output_file")
            .arg(output_path)
            .arg("--text")
            .arg(&sanitized_text);
        
        // Apply rate control via --length_scale (inverse relationship: lower = faster)
        // Map rate 0-500 WPM to length_scale 2.0-0.5 (default ~1.0 for 150 WPM)
        let length_scale = if self.rate <= 150 {
            // 0-150 WPM: length_scale 2.0 to 1.0 (slower to normal)
            2.0 - (self.rate as f32 / 150.0)
        } else {
            // 150-500 WPM: length_scale 1.0 to 0.5 (normal to faster)
            1.0 - ((self.rate - 150) as f32 / 350.0) * 0.5
        }.clamp(0.5, 2.0);
        
        cmd.arg("--length_scale").arg(length_scale.to_string());
        
        // Note: Volume and pitch would require audio post-processing
        // Piper doesn't directly support these via command line
        
        let output = cmd.output()
            .map_err(|e| SpeechError::Engine(format!("Failed to execute piper: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(SpeechError::Engine(format!("Piper synthesis failed: {}", error_msg)));
        }

        // Read the audio file
        let mut audio_data = Vec::new();
        std::fs::File::open(output_path)
            .and_then(|mut f| f.read_to_end(&mut audio_data))
            .map_err(|e| SpeechError::Engine(format!("Failed to read audio file: {}", e)))?;

        // Validate audio size
        const MAX_AUDIO_SIZE: usize = 10 * 1024 * 1024; // 10MB
        if audio_data.len() > MAX_AUDIO_SIZE {
            return Err(SpeechError::Engine(format!(
                "Generated audio too large ({} bytes, max {} bytes)",
                audio_data.len(), MAX_AUDIO_SIZE
            )));
        }

        Ok(Bytes::from(audio_data))
    }

    async fn list_voices(&self) -> Result<Vec<String>, SpeechError> {
        // Try to list voices from voices directory
        if let Some(ref voices_dir) = self.voices_dir {
            let mut voices = Vec::new();
            
            if let Ok(entries) = std::fs::read_dir(voices_dir) {
                for entry in entries.flatten() {
                    if let Some(file_name) = entry.file_name().to_str() {
                        if file_name.ends_with(".onnx") {
                            let voice_name = file_name.trim_end_matches(".onnx").to_string();
                            if voice_name.len() <= 256 {
                                voices.push(voice_name);
                            }
                        }
                    }
                }
            }

            if !voices.is_empty() {
                return Ok(voices);
            }
        }

        // Return default voices if none found
        Ok(vec![
            "en_US-lessac-medium".to_string(),
            "en_US-lessac-high".to_string(),
            "en_US-libritts-high".to_string(),
        ])
    }

    fn is_available(&self) -> bool {
        // Check if piper executable exists and is executable
        self.piper_path.exists() && 
        std::fs::metadata(&self.piper_path)
            .map(|m| {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    m.permissions().mode() & 0o111 != 0
                }
                #[cfg(not(unix))]
                {
                    true // On Windows, just check if file exists
                }
            })
            .unwrap_or(false)
    }

    fn name(&self) -> &str {
        "Piper TTS"
    }
}


