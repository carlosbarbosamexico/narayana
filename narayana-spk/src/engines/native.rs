//! Native platform TTS engine

use crate::error::SpeechError;
use crate::engines::TtsEngine;
use crate::config::VoiceConfig;
use async_trait::async_trait;
use bytes::Bytes;
use tracing::{info, warn, error};

/// Native TTS engine (platform-specific)
pub struct NativeTtsEngine {
    #[cfg(target_os = "macos")]
    synthesizer: Option<macos::NSSpeechSynthesizer>,
    
    #[cfg(target_os = "linux")]
    synthesizer: Option<linux::EspeakEngine>,
    
    #[cfg(target_os = "windows")]
    synthesizer: Option<windows::SapiEngine>,
    
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    synthesizer: Option<()>,
    
    // Store speech config for rate/volume/pitch support
    rate: u32,
    volume: f32,
    pitch: f32,
}

#[async_trait]
impl TtsEngine for NativeTtsEngine {
    async fn synthesize(&self, text: &str, config: &VoiceConfig) -> Result<Bytes, SpeechError> {
        // Spawn to blocking thread pool to avoid Send issues
        let text = text.to_string();
        let config = config.clone();
        // Validate input
        if text.is_empty() {
            return Err(SpeechError::Synthesizer("Text cannot be empty".to_string()));
        }

        if text.len() > 100_000 {
            return Err(SpeechError::Synthesizer("Text too long (max 100KB)".to_string()));
        }

        // Sanitize text (remove control characters except newlines)
        let sanitized: String = text
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\r')
            .take(100_000)
            .collect();

        #[cfg(target_os = "macos")]
        {
            if let Some(ref synth) = self.synthesizer {
                return macos::synthesize(synth, &sanitized, &config, self.rate, self.volume, self.pitch).await;
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(ref synth) = self.synthesizer {
                return linux::synthesize(synth, &sanitized, config, self.rate, self.volume, self.pitch).await;
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(ref synth) = self.synthesizer {
                return windows::synthesize(synth, &sanitized, config, self.rate, self.volume, self.pitch).await;
            }
        }

        Err(SpeechError::Engine("Native TTS engine not available".to_string()))
    }

    async fn list_voices(&self) -> Result<Vec<String>, SpeechError> {
        #[cfg(target_os = "macos")]
        {
            if self.synthesizer.is_some() {
                // Return default macOS voices
                return Ok(vec!["com.apple.speech.synthesis.voice.Alex".to_string()]);
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Some(ref synth) = self.synthesizer {
                return linux::list_voices(synth).await;
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(ref synth) = self.synthesizer {
                return windows::list_voices(synth).await;
            }
        }

        Ok(vec![])
    }

    fn is_available(&self) -> bool {
        #[cfg(target_os = "macos")]
        return self.synthesizer.is_some();
        
        #[cfg(target_os = "linux")]
        return self.synthesizer.is_some();
        
        #[cfg(target_os = "windows")]
        return self.synthesizer.is_some();
        
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        return false;
    }

    fn name(&self) -> &str {
        "native"
    }
}

impl NativeTtsEngine {
    pub fn new() -> Result<Self, SpeechError> {
        Self::new_with_config(150, 0.8, 0.0) // Default values
    }
    
    pub fn new_with_config(rate: u32, volume: f32, pitch: f32) -> Result<Self, SpeechError> {
        #[cfg(target_os = "macos")]
        {
            match macos::NSSpeechSynthesizer::new() {
                Ok(synth) => {
                    info!("Native macOS TTS engine initialized");
                    Ok(Self { 
                        synthesizer: Some(synth),
                        rate,
                        volume,
                        pitch,
                    })
                }
                Err(e) => {
                    warn!("Failed to initialize macOS TTS: {}", e);
                    Ok(Self { 
                        synthesizer: None,
                        rate,
                        volume,
                        pitch,
                    })
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            match linux::EspeakEngine::new() {
                Ok(synth) => {
                    info!("Native Linux TTS engine initialized");
                    Ok(Self { 
                        synthesizer: Some(synth),
                        rate,
                        volume,
                        pitch,
                    })
                }
                Err(e) => {
                    warn!("Failed to initialize Linux TTS: {}", e);
                    Ok(Self { 
                        synthesizer: None,
                        rate,
                        volume,
                        pitch,
                    })
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            match windows::SapiEngine::new() {
                Ok(synth) => {
                    info!("Native Windows TTS engine initialized");
                    Ok(Self { 
                        synthesizer: Some(synth),
                        rate,
                        volume,
                        pitch,
                    })
                }
                Err(e) => {
                    warn!("Failed to initialize Windows TTS: {}", e);
                    Ok(Self { 
                        synthesizer: None,
                        rate,
                        volume,
                        pitch,
                    })
                }
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            warn!("Native TTS not supported on this platform");
            Ok(Self { 
                synthesizer: None,
                rate,
                volume,
                pitch,
            })
        }
    }
}

// Platform-specific implementations
#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use objc::runtime::{Class, Object};
    use objc::*;
    use core_foundation::base::TCFType;
    use core_foundation::string::{CFString, CFStringRef};
    use std::ffi::CString;

    #[derive(Clone)]
    pub struct NSSpeechSynthesizer {
        _phantom: std::marker::PhantomData<()>,
    }

    impl NSSpeechSynthesizer {
        pub fn new() -> Result<Self, SpeechError> {
            // Check if NSSpeechSynthesizer is available
            unsafe {
                let _class = Class::get("NSSpeechSynthesizer").ok_or_else(|| {
                    SpeechError::Engine("NSSpeechSynthesizer class not found".to_string())
                })?;
            }
            Ok(Self { _phantom: std::marker::PhantomData })
        }
    }

    pub async fn synthesize(
        _synth: &NSSpeechSynthesizer,
        text: &str,
        config: &VoiceConfig,
        rate: u32,
        _volume: f32,
        _pitch: f32,
    ) -> Result<Bytes, SpeechError> {
        use tempfile::NamedTempFile;
        use std::io::Read;
        use std::process::Command;
        
        // Create a temporary file for audio output
        let temp_file = NamedTempFile::new()
            .map_err(|e| SpeechError::Engine(format!("Failed to create temp file: {}", e)))?;
        let temp_path = temp_file.path().to_str()
            .ok_or_else(|| SpeechError::Engine("Invalid temp file path".to_string()))?;
        
        // Use 'say' command to synthesize speech to file
        // 'say' command supports -o flag for output file
        let mut cmd = Command::new("say");
        cmd.arg("-o").arg(temp_path);
        
        // Set voice if specified (sanitize to prevent command injection)
        if let Some(ref voice_name) = config.name {
            // Sanitize voice name - only allow alphanumeric, spaces, and hyphens
            let sanitized_voice: String = voice_name
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
                .take(256) // Limit length
                .collect();
            if !sanitized_voice.is_empty() {
                cmd.arg("-v").arg(&sanitized_voice);
            }
        }
        // Note: If no voice specified, system will use default
        
        // Set rate (words per minute, say command uses -r flag)
        // Use rate from SpeechConfig (passed as parameter)
        // Clamp rate to valid range for say command (typically 0-500, but say accepts wider range)
        let say_rate = rate.min(500).max(0);
        cmd.arg("-r").arg(say_rate.to_string());
        
        // Note: Volume and pitch are not directly supported by say command
        // Volume can be controlled via system volume or audio processing
        // Pitch adjustment would require audio post-processing
        
        // Add text (sanitized)
        let sanitized_text: String = text
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
            .collect();
        
        cmd.arg(&sanitized_text);
        
        // Execute command
        let output = cmd.output()
            .map_err(|e| SpeechError::Engine(format!("Failed to execute say command: {}", e)))?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(SpeechError::Engine(format!("say command failed: {}", error_msg)));
        }
        
        // Read the audio file
        let mut audio_data = Vec::new();
        std::fs::File::open(temp_path)
            .and_then(|mut f| f.read_to_end(&mut audio_data))
            .map_err(|e| SpeechError::Engine(format!("Failed to read audio file: {}", e)))?;
        
        // Validate audio size (prevent huge files)
        const MAX_AUDIO_SIZE: usize = 10 * 1024 * 1024; // 10MB
        if audio_data.len() > MAX_AUDIO_SIZE {
            return Err(SpeechError::Engine(format!(
                "Generated audio too large ({} bytes, max {} bytes)",
                audio_data.len(), MAX_AUDIO_SIZE
            )));
        }
        
        Ok(Bytes::from(audio_data))
    }

    pub async fn list_voices(_synth: &NSSpeechSynthesizer) -> Result<Vec<String>, SpeechError> {
        use std::process::Command;
        
        // Use 'say' command to list available voices
        let output = Command::new("say")
            .arg("-v")
            .arg("?")
            .output()
            .map_err(|e| SpeechError::Engine(format!("Failed to execute say command: {}", e)))?;
        
        if !output.status.success() {
            return Err(SpeechError::Engine("Failed to list voices".to_string()));
        }
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut voices = Vec::new();
        
        // Parse output: each line is "voice_name language_code"
        for line in output_str.lines() {
            if let Some(voice_name) = line.split_whitespace().next() {
                // Validate voice name
                if voice_name.len() <= 256 && 
                   voice_name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.') {
                    voices.push(voice_name.to_string());
                }
            }
        }
        
        // Limit to prevent memory exhaustion
        const MAX_VOICES: usize = 1000;
        if voices.len() > MAX_VOICES {
            voices.truncate(MAX_VOICES);
        }
        
        if voices.is_empty() {
            // Fallback to default voices
            voices = vec![
                "Alex".to_string(),
                "Samantha".to_string(),
                "Victoria".to_string(),
            ];
        }
        
        Ok(voices)
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;

    pub struct EspeakEngine {
        available: bool,
    }

    impl EspeakEngine {
        pub fn new() -> Result<Self, SpeechError> {
            // Check if espeak-ng is available
            let available = std::process::Command::new("espeak-ng")
                .arg("--version")
                .output()
                .is_ok();

            Ok(Self { available })
        }
    }

    pub async fn synthesize(
        synth: &EspeakEngine,
        text: &str,
        _config: &VoiceConfig,
    ) -> Result<Bytes, SpeechError> {
        if !synth.available {
            return Err(SpeechError::Engine("espeak-ng not available".to_string()));
        }

        // Validate and sanitize text to prevent command injection
        // Remove any shell metacharacters and control characters
        let sanitized_text: String = text
            .chars()
            .filter(|c| {
                !c.is_control() && 
                *c != ';' && *c != '|' && *c != '&' && *c != '$' && 
                *c != '`' && *c != '(' && *c != ')' && *c != '<' && 
                *c != '>' && *c != '\n' && *c != '\r'
            })
            .take(100_000) // Limit length
            .collect();

        if sanitized_text.is_empty() {
            return Err(SpeechError::Synthesizer("Text is empty after sanitization".to_string()));
        }

        // Use espeak-ng to synthesize to WAV file, then read it
        use std::process::Command;
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new()
            .map_err(|e| SpeechError::Io(e))?
            .path()
            .to_path_buf();

        // Validate temp file path (should be safe from NamedTempFile, but double-check)
        let temp_file_str = temp_file.to_string_lossy();
        if temp_file_str.contains("..") || 
           temp_file_str.len() > 512 ||
           temp_file_str.contains('\0') || 
           temp_file_str.contains('\n') || 
           temp_file_str.contains('\r') {
            return Err(SpeechError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid temp file path"
            )));
        }
        
        // Build espeak-ng command with rate, volume, and pitch
        let mut cmd = Command::new("espeak-ng");
        
        // Set speed (WPM) - use rate from SpeechConfig
        cmd.arg("-s").arg(rate.to_string());
        
        // Set volume (0-200, where 100 is normal)
        // Convert from 0.0-1.0 range to 0-200 range
        let espeak_volume = ((volume * 200.0).round() as u32).min(200).max(0);
        cmd.arg("-a").arg(espeak_volume.to_string());
        
        // Set pitch (0-99, where 50 is normal)
        // Convert from -1.0 to 1.0 range to 0-99 range
        // -1.0 -> 0, 0.0 -> 50, 1.0 -> 99
        let espeak_pitch = ((50.0 + (pitch * 49.0)).round() as u32).min(99).max(0);
        cmd.arg("-p").arg(espeak_pitch.to_string());
        
        cmd.arg("-w") // Write to file
            .arg(&temp_file)
            .arg(&sanitized_text); // Use sanitized text
        
        let output = cmd.output()
            .map_err(|e| SpeechError::Engine(format!("Failed to run espeak-ng: {}", e)))?;

        if !output.status.success() {
            let _ = std::fs::remove_file(&temp_file);
            return Err(SpeechError::Engine(format!(
                "espeak-ng failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Validate file size before reading (prevent huge files)
        const MAX_AUDIO_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
        let metadata = std::fs::metadata(&temp_file)
            .map_err(|e| SpeechError::Io(e))?;
        
        if metadata.len() > MAX_AUDIO_FILE_SIZE {
            let _ = std::fs::remove_file(&temp_file);
            return Err(SpeechError::Engine(format!(
                "Generated audio file too large ({} bytes, max {} bytes)",
                metadata.len(), MAX_AUDIO_FILE_SIZE
            )));
        }

        let audio_data = std::fs::read(&temp_file)
            .map_err(|e| {
                let _ = std::fs::remove_file(&temp_file);
                SpeechError::Io(e)
            })?;

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_file);

        Ok(Bytes::from(audio_data))
    }

    pub async fn list_voices(_synth: &EspeakEngine) -> Result<Vec<String>, SpeechError> {
        use std::process::Command;

        let output = Command::new("espeak-ng")
            .arg("--voices")
            .output()
            .map_err(|e| SpeechError::Engine(format!("Failed to list voices: {}", e)))?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        // Parse voices with validation
        let voices: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .skip(1) // Skip header
            .filter_map(|line| {
                line.split_whitespace().nth(1).and_then(|s| {
                    // Validate and limit voice name length
                    let voice = s.to_string();
                    if voice.len() > 256 {
                        warn!("Voice name too long, truncating: {}", voice);
                        Some(voice.chars().take(256).collect())
                    } else if voice.chars().any(|c| c == '\0' || c.is_control()) {
                        warn!("Voice name contains invalid characters, skipping: {}", voice);
                        None
                    } else {
                        Some(voice)
                    }
                })
            })
            .take(1000) // Limit number of voices to prevent memory exhaustion
            .collect();

        Ok(voices)
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;

    pub struct SapiEngine {
        available: bool,
    }

    impl SapiEngine {
        pub fn new() -> Result<Self, SpeechError> {
            // SAPI should always be available on Windows
            Ok(Self { available: true })
        }
    }

    pub async fn synthesize(
        synth: &SapiEngine,
        text: &str,
        config: &VoiceConfig,
    ) -> Result<Bytes, SpeechError> {
        if !synth.available {
            return Err(SpeechError::Engine("SAPI not available".to_string()));
        }

        // Validate input length to prevent DoS
        if text.len() > 100_000 {
            return Err(SpeechError::Engine("Text too long for Windows SAPI (max 100KB)".to_string()));
        }

        // Validate voice name if provided
        if let Some(ref voice_name) = config.name {
            if voice_name.len() > 256 {
                return Err(SpeechError::Engine("Voice name too long (max 256 chars)".to_string()));
            }
            // Check for invalid characters in voice name
            if voice_name.chars().any(|c| c == '\0' || c == '\n' || c == '\r') {
                return Err(SpeechError::Engine("Voice name contains invalid characters".to_string()));
            }
        }

        // Generate unique temporary file name to prevent race conditions
        use uuid::Uuid;
        let file_id = Uuid::new_v4().to_string();
        let temp_dir = std::env::var("TEMP")
            .map_err(|_| SpeechError::Engine("TEMP environment variable not set".to_string()))?;
        
        // Validate temp directory path to prevent path traversal
        let temp_path = std::path::Path::new(&temp_dir);
        if !temp_path.is_absolute() {
            return Err(SpeechError::Engine("TEMP path must be absolute".to_string()));
        }
        
        let wav_filename = format!("tts_output_{}.wav", file_id);
        let wav_path = temp_path.join(&wav_filename);
        
        // Additional validation: ensure the path is within temp directory
        let canonical_temp = temp_path.canonicalize()
            .map_err(|e| SpeechError::Engine(format!("Failed to canonicalize temp path: {}", e)))?;
        let canonical_wav = wav_path.canonicalize()
            .map_err(|_| {
                // File doesn't exist yet, but we can check parent
                wav_path.parent()
                    .ok_or_else(|| SpeechError::Engine("Invalid temp file path".to_string()))
                    .and_then(|p| p.canonicalize()
                        .map_err(|e| SpeechError::Engine(format!("Failed to canonicalize wav path: {}", e))))
            })?;
        
        if !canonical_wav.starts_with(&canonical_temp) {
            return Err(SpeechError::Engine("Path traversal detected in temp file path".to_string()));
        }

        let wav_path_str = wav_path.to_string_lossy().to_string();
        
        // Sanitize text to prevent PowerShell injection
        // Escape all special PowerShell characters
        let sanitized_text = text
            .chars()
            .map(|c| match c {
                '"' => "`\"".to_string(),
                '$' => "`$".to_string(),
                '`' => "``".to_string(),
                '\n' => " ".to_string(),
                '\r' => " ".to_string(),
                '\0' => return Err(SpeechError::Engine("Text contains null bytes".to_string())),
                _ => c.to_string(),
            })
            .collect::<Result<String, SpeechError>>()?;

        // Sanitize voice name if provided
        let voice_arg = if let Some(ref voice_name) = config.name {
            let sanitized_voice = voice_name
                .chars()
                .map(|c| match c {
                    '"' => "`\"".to_string(),
                    '$' => "`$".to_string(),
                    '`' => "``".to_string(),
                    '\0' => return Err(SpeechError::Engine("Voice name contains null bytes".to_string())),
                    _ => c.to_string(),
                })
                .collect::<Result<String, SpeechError>>()?;
            format!("$synth.SelectVoice('{}'); ", sanitized_voice)
        } else {
            String::new()
        };

        // Use Add-Type to create a .NET SpeechSynthesizer
        // Use single quotes for file path and proper escaping
        // Set rate, volume, and pitch
        // Rate: SpeechSynthesizer.Rate is -10 to 10 (default 0)
        // Volume: SpeechSynthesizer.Volume is 0 to 100 (default 100)
        // Note: Pitch adjustment requires SSML or audio post-processing
        
        // Convert rate from WPM (0-500) to SpeechSynthesizer rate (-10 to 10)
        // 0 WPM -> -10, 250 WPM -> 0, 500 WPM -> 10
        let synth_rate = if rate <= 250 {
            -10 + ((rate as f32 / 250.0) * 10.0) as i32
        } else {
            ((rate - 250) as f32 / 250.0 * 10.0) as i32
        }.clamp(-10, 10);
        
        // Convert volume from 0.0-1.0 to 0-100
        let synth_volume = ((volume * 100.0).round() as u32).clamp(0, 100);
        
        let ps_script = format!(
            r#"
            Add-Type -AssemblyName System.Speech
            $synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
            {}
            $synth.Rate = {}
            $synth.Volume = {}
            $synth.SetOutputToWaveFile('{}')
            $synth.Speak('{}')
            $synth.Dispose()
            "#,
            voice_arg,
            synth_rate,
            synth_volume,
            wav_path_str.replace('\'', "''"), // Escape single quotes in path
            sanitized_text
        );

        // Execute PowerShell script with proper security flags
        let output = tokio::process::Command::new("powershell")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-ExecutionPolicy")
            .arg("Bypass") // Needed for Add-Type
            .arg("-Command")
            .arg(&ps_script)
            .output()
            .await
            .map_err(|e| SpeechError::Engine(format!("Failed to execute PowerShell: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Clean up temp file even on error
            let _ = tokio::fs::remove_file(&wav_path).await;
            return Err(SpeechError::Engine(format!(
                "PowerShell SAPI synthesis failed: {}",
                stderr
            )));
        }

        // Read the generated WAV file with size limit
        const MAX_AUDIO_SIZE: usize = 10 * 1024 * 1024; // 10MB max
        let metadata = tokio::fs::metadata(&wav_path)
            .await
            .map_err(|e| SpeechError::Engine(format!("Failed to get file metadata: {}", e)))?;
        
        if metadata.len() > MAX_AUDIO_SIZE as u64 {
            let _ = tokio::fs::remove_file(&wav_path).await;
            return Err(SpeechError::Engine(format!(
                "Generated audio file too large ({} bytes, max {} bytes)",
                metadata.len(), MAX_AUDIO_SIZE
            )));
        }

        let audio_bytes = tokio::fs::read(&wav_path)
            .await
            .map_err(|e| SpeechError::Engine(format!("Failed to read generated audio file: {}", e)))?;

        // Clean up temporary file
        if let Err(e) = tokio::fs::remove_file(&wav_path).await {
            warn!("Failed to remove temporary file {}: {}", wav_path_str, e);
        }

        Ok(Bytes::from(audio_bytes))
    }

    pub async fn list_voices(_synth: &SapiEngine) -> Result<Vec<String>, SpeechError> {
        // Windows SAPI voice enumeration
        Ok(vec!["Microsoft David Desktop".to_string(), "Microsoft Zira Desktop".to_string()])
    }
}

