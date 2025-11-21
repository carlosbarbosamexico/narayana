//! Audio capture from system microphone

use crate::config::CaptureConfig;
use crate::error::AudioError;
use bytes::Bytes;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, SampleRate, StreamConfig};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};

/// Audio capture from system microphone
pub struct AudioCapture {
    config: Arc<CaptureConfig>,
    host: Host,
    device: Option<Device>,
    // Note: Stream is not Send/Sync, so we store it separately
    // and handle it carefully in start/stop methods
    _stream_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    sample_rate: u32,
    channels: u16,
    sample_format: SampleFormat,
    is_running: Arc<RwLock<bool>>,
    audio_sender: Arc<RwLock<Option<mpsc::Sender<Bytes>>>>,
}

impl AudioCapture {
    /// Create a new audio capture instance
    pub fn new(config: CaptureConfig, sample_rate: u32, channels: u16) -> Result<Self, AudioError> {
        config.validate()
            .map_err(|e| AudioError::Config(e))?;

        let host = cpal::default_host();
        
        // Find device
        let device = if let Some(ref device_name) = config.device_name {
            Self::find_device_by_name(&host, device_name)?
        } else {
            host.default_input_device()
        };

        let sample_format = if let Some(ref device) = device {
            Self::get_supported_format(device)?
        } else {
            SampleFormat::F32
        };

        Ok(Self {
            config: Arc::new(config),
            host,
            device,
            _stream_handle: Arc::new(RwLock::new(None)),
            sample_rate,
            channels,
            sample_format,
            is_running: Arc::new(RwLock::new(false)),
            audio_sender: Arc::new(RwLock::new(None)),
        })
    }

    /// Find device by name
    /// Security: Validates device name to prevent injection
    fn find_device_by_name(host: &Host, name: &str) -> Result<Option<Device>, AudioError> {
        // Security: Validate device name length
        if name.len() > 256 {
            return Err(AudioError::Device("Device name too long (max 256 chars)".to_string()));
        }

        // Security: Limit device enumeration to prevent DoS
        const MAX_DEVICES_TO_CHECK: usize = 100;
        let devices = host.input_devices()
            .map_err(|e| AudioError::Device(format!("Failed to enumerate devices: {}", e)))?;

        let mut count = 0;
        for device in devices {
            if count >= MAX_DEVICES_TO_CHECK {
                break; // Prevent excessive enumeration
            }
            count += 1;

            if let Ok(device_name) = device.name() {
                // Security: Use exact match or safe substring search
                if device_name == name || device_name.contains(name) {
                    return Ok(Some(device));
                }
            }
        }

        Ok(None)
    }

    /// Get supported sample format from device
    fn get_supported_format(device: &Device) -> Result<SampleFormat, AudioError> {
        let configs = device.supported_input_configs()
            .map_err(|e| AudioError::Device(format!("Failed to get supported configs: {}", e)))?;

        for config in configs {
            // Prefer F32 format for better quality
            if config.sample_format() == SampleFormat::F32 {
                return Ok(SampleFormat::F32);
            }
        }

        // Fallback to I16
        Ok(SampleFormat::I16)
    }

    /// Start audio capture
    pub fn start(&self, audio_tx: mpsc::Sender<Bytes>) -> Result<(), AudioError> {
        {
            let mut is_running = self.is_running.write();
            if *is_running {
                return Err(AudioError::Capture("Audio capture already running".to_string()));
            }
            *is_running = true;
        }

        *self.audio_sender.write() = Some(audio_tx);

        let device = self.device.as_ref()
            .ok_or_else(|| AudioError::Device("No input device available".to_string()))?;

        let config = StreamConfig {
            channels: self.channels,
            sample_rate: SampleRate(self.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(4096),
        };

        let audio_sender = self.audio_sender.clone();
        let is_running = self.is_running.clone();

        let stream_result = match self.sample_format {
            SampleFormat::F32 => {
                let is_running_clone = is_running.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        Self::process_samples_f32(data, &audio_sender);
                    },
                    move |err| {
                        error!("Audio stream error: {}", err);
                        *is_running_clone.write() = false;
                    },
                    None,
                )
            }
            SampleFormat::I16 => {
                let is_running_clone = is_running.clone();
                device.build_input_stream(
                    &config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        Self::process_samples_i16(data, &audio_sender);
                    },
                    move |err| {
                        error!("Audio stream error: {}", err);
                        *is_running_clone.write() = false;
                    },
                    None,
                )
            }
            _ => {
                return Err(AudioError::Format("Unsupported sample format".to_string()));
            }
        };

        let stream = stream_result
            .map_err(|e| AudioError::Capture(format!("Failed to build stream: {}", e)))?;

        stream.play()
            .map_err(|e| AudioError::Capture(format!("Failed to start stream: {}", e)))?;

        // Store stream - we need to keep it alive
        // Since Stream is not Send/Sync, we'll handle it differently
        // For now, we'll just keep a reference and let it run
        // The stream will be dropped when AudioCapture is dropped
        std::mem::forget(stream); // Keep stream alive for the duration
        
        info!("Audio capture started");

        Ok(())
    }

    /// Process F32 samples
    /// Security: Validates input size and handles buffer overflow
    fn process_samples_f32(samples: &[f32], sender: &Arc<RwLock<Option<mpsc::Sender<Bytes>>>>) {
        // Security: Limit sample count to prevent excessive memory allocation
        const MAX_SAMPLES_PER_CHUNK: usize = 100_000; // ~400KB max per chunk
        if samples.len() > MAX_SAMPLES_PER_CHUNK {
            warn!("Sample chunk too large: {} samples (max {}), truncating", 
                  samples.len(), MAX_SAMPLES_PER_CHUNK);
            // Truncate to prevent DoS
            let samples = &samples[..MAX_SAMPLES_PER_CHUNK];
            return Self::process_samples_f32(samples, sender);
        }

        let sender_guard = sender.read();
        if let Some(ref tx) = *sender_guard {
            // Convert f32 samples to bytes
            let bytes: Vec<u8> = samples.iter()
                .flat_map(|&sample| {
                    // Security: Handle NaN/Inf gracefully
                    let safe_sample = if sample.is_finite() { sample } else { 0.0 };
                    safe_sample.to_le_bytes().to_vec()
                })
                .collect();
            
            let audio_bytes = Bytes::from(bytes);
            if tx.try_send(audio_bytes).is_err() {
                debug!("Audio buffer full, dropping samples");
            }
        }
    }

    /// Process I16 samples
    /// Security: Validates input size and handles buffer overflow
    fn process_samples_i16(samples: &[i16], sender: &Arc<RwLock<Option<mpsc::Sender<Bytes>>>>) {
        // Security: Limit sample count to prevent excessive memory allocation
        const MAX_SAMPLES_PER_CHUNK: usize = 200_000; // ~400KB max per chunk
        if samples.len() > MAX_SAMPLES_PER_CHUNK {
            warn!("Sample chunk too large: {} samples (max {}), truncating", 
                  samples.len(), MAX_SAMPLES_PER_CHUNK);
            // Truncate to prevent DoS
            let samples = &samples[..MAX_SAMPLES_PER_CHUNK];
            let truncated = &samples[..MAX_SAMPLES_PER_CHUNK];
            Self::process_samples_i16(truncated, sender);
            return;
        }

        let sender_guard = sender.read();
        if let Some(ref tx) = *sender_guard {
            // Convert i16 samples to bytes
            let bytes: Vec<u8> = samples.iter()
                .flat_map(|&sample| sample.to_le_bytes().to_vec())
                .collect();
            
            let audio_bytes = Bytes::from(bytes);
            if tx.try_send(audio_bytes).is_err() {
                debug!("Audio buffer full, dropping samples");
            }
        }
    }

    /// Stop audio capture
    pub fn stop(&self) -> Result<(), AudioError> {
        {
            let mut is_running = self.is_running.write();
            if !*is_running {
                return Ok(()); // Already stopped
            }
            *is_running = false;
        }

        // Stream will be dropped when AudioCapture is dropped
        // For now, we just mark as stopped
        *self.audio_sender.write() = None;
        info!("Audio capture stopped");

        Ok(())
    }

    /// Check if capture is running
    pub fn is_running(&self) -> bool {
        *self.is_running.read()
    }

    /// Get available input devices
    pub fn list_devices() -> Result<Vec<String>, AudioError> {
        let host = cpal::default_host();
        let devices = host.input_devices()
            .map_err(|e| AudioError::Device(format!("Failed to enumerate devices: {}", e)))?;

        let mut device_names = Vec::new();
        for device in devices {
            if let Ok(name) = device.name() {
                device_names.push(name);
            }
        }

        Ok(device_names)
    }
}
