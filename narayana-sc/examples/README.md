# narayana-sc Examples

This directory contains example programs demonstrating how to use the narayana-sc audio capture and analysis library.

## Examples

### 1. `basic_audio_capture.rs`
**Purpose**: Basic audio capture from system microphone with real-time analysis.

**Features**:
- Audio capture from system microphone
- Real-time audio analysis (FFT, energy, pitch, spectral features)
- Integration with WorldBroker for event handling
- Voice-to-text support (if LLM integration enabled)
- Event subscription and processing

**Usage**:
```bash
cargo run --example basic_audio_capture --package narayana-sc
```

**Note**: Requires an audio input device (microphone) to be available.

### 2. `audio_analysis.rs`
**Purpose**: Direct audio analysis without capture - analyzes pre-generated audio data.

**Features**:
- Analyze silence
- Analyze pure tones (sine waves)
- Analyze multiple frequencies
- Analyze white noise
- Demonstrates FFT, energy, pitch detection, spectral analysis

**Usage**:
```bash
cargo run --example audio_analysis --package narayana-sc
```

**Note**: No audio device required - uses generated audio data.

### 3. `comprehensive_capture.rs`
**Purpose**: Full-featured audio capture with all advanced features enabled.

**Features**:
- Comprehensive audio processing pipeline
- Noise reduction
- Automatic gain control (AGC)
- Echo cancellation
- Voice activity detection
- Complete statistics tracking
- All analysis features enabled

**Usage**:
```bash
cargo run --example comprehensive_capture --package narayana-sc
```

**Note**: Uses simulated audio data for demonstration.

### 4. `cpl_integration.rs`
**Purpose**: Integration with Conscience Persistent Loop (CPL) for brain-controlled audio processing.

**Features**:
- CPL configuration with audio settings
- Audio config extraction from CPL
- WorldBroker integration
- Event handling and processing
- Real-time audio event monitoring

**Usage**:
```bash
cargo run --example cpl_integration --package narayana-sc
```

**Note**: Requires CPL and WorldBroker to be properly configured.

## Running Examples

### Run a specific example
```bash
cargo run --example <example_name> --package narayana-sc
```

### Run with output
```bash
RUST_LOG=info cargo run --example basic_audio_capture --package narayana-sc
```

### Run with debug output
```bash
RUST_LOG=debug cargo run --example basic_audio_capture --package narayana-sc
```

## Example Output

### basic_audio_capture
```
INFO Starting basic audio capture example...
INFO Audio configuration validated
INFO Audio adapter created successfully
INFO Audio adapter registered with world broker
INFO World broker started. Capturing audio for 10 seconds...
INFO Received audio event: ...
INFO Audio energy: 0.45
INFO Detected pitch: 440.23 Hz
INFO Dominant frequencies: [440.0, 880.0, ...]
```

### audio_analysis
```
Audio Analysis Example
======================

1. Analyzing silence...
  Energy: 0.000000
  Zero-crossing rate: 0.000000
  Spectral centroid: 0.00 Hz

2. Analyzing pure tone (440 Hz)...
  Energy: 0.125000
  Zero-crossing rate: 0.002197
  Spectral centroid: 440.00 Hz
  Detected pitch: 440.12 Hz (expected ~440.00 Hz)
  Error: 0.12 Hz
```

## Requirements

### For examples requiring audio capture:
- Audio input device (microphone)
- Proper audio permissions (macOS/Linux)
- Audio drivers installed

### For all examples:
- Rust 1.70+
- narayana-sc dependencies
- tokio runtime

## Troubleshooting

### "Failed to create audio adapter"
- Check if audio device is available
- Verify audio permissions
- Check audio driver installation

### "No audio device found"
- Connect a microphone
- Check system audio settings
- Verify device permissions

### "Invalid audio config"
- Check configuration values
- Ensure sample rate is valid
- Verify buffer sizes are reasonable

## Next Steps

After running the examples:
1. Modify configurations to suit your needs
2. Integrate with your own applications
3. Enable LLM integration for voice-to-text
4. Customize analysis parameters
5. Add custom event handlers

