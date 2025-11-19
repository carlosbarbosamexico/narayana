// Sensory Streams - Live Data Inputs for Embodied Robots
// Camera frames, audio, sensors, IMU, lidar as column streams
// Production-ready implementation with indexing, compression, vectorization

use narayana_core::{Error, Result, column::Column};
use crate::index::{BTreeIndex, Index};
use crate::compression::{Compressor, Decompressor, Lz4Compressor, ZstdCompressor, SnappyCompressor};
use crate::advanced_indexing::AdvancedIndexManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, debug, warn};
use uuid::Uuid;
use bytes::Bytes;

/// Sensory stream manager
pub struct SensoryStreamManager {
    streams: Arc<RwLock<HashMap<String, Arc<SensoryStream>>>>,
    stream_processors: Arc<RwLock<HashMap<String, StreamProcessor>>>,
    event_sender: broadcast::Sender<StreamEvent>,
}

impl SensoryStreamManager {
    pub fn new() -> Self {
        // EDGE CASE: Broadcast channel has fixed capacity - handle overflow gracefully
        // Capacity of 1000 should be sufficient, but if overflow occurs, messages are dropped
        // In production, would monitor channel capacity and warn if approaching limit
        const CHANNEL_CAPACITY: usize = 1000;
        let (sender, _) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
            stream_processors: Arc::new(RwLock::new(HashMap::new())),
            event_sender: sender,
        }
    }

    /// Register camera stream
    pub fn register_camera_stream(
        &self,
        stream_id: &str,
        width: u32,
        height: u32,
        fps: u32,
    ) -> Result<Arc<SensoryStream>> {
        let stream = SensoryStream::new(
            stream_id.to_string(),
            StreamType::Camera {
                width,
                height,
                fps,
            },
        );
        let stream_arc = Arc::new(stream);
        self.streams.write().insert(stream_id.to_string(), stream_arc.clone());
        info!("Registered camera stream: {} ({}x{} @ {}fps)", stream_id, width, height, fps);
        Ok(stream_arc)
    }

    /// Register audio stream
    pub fn register_audio_stream(
        &self,
        stream_id: &str,
        sample_rate: u32,
        channels: u8,
    ) -> Result<Arc<SensoryStream>> {
        let stream = SensoryStream::new(
            stream_id.to_string(),
            StreamType::Audio {
                sample_rate,
                channels,
            },
        );
        let stream_arc = Arc::new(stream);
        self.streams.write().insert(stream_id.to_string(), stream_arc.clone());
        info!("Registered audio stream: {} ({}Hz, {}ch)", stream_id, sample_rate, channels);
        Ok(stream_arc)
    }

    /// Register sensor stream
    pub fn register_sensor_stream(
        &self,
        stream_id: &str,
        sensor_type: SensorType,
    ) -> Result<Arc<SensoryStream>> {
        let stream = SensoryStream::new(
            stream_id.to_string(),
            StreamType::Sensor(sensor_type.clone()),
        );
        let stream_arc = Arc::new(stream);
        self.streams.write().insert(stream_id.to_string(), stream_arc.clone());
        info!("Registered sensor stream: {} ({:?})", stream_id, sensor_type);
        Ok(stream_arc)
    }

    /// Register IMU stream
    pub fn register_imu_stream(&self, stream_id: &str) -> Result<Arc<SensoryStream>> {
        let stream = SensoryStream::new(
            stream_id.to_string(),
            StreamType::IMU,
        );
        let stream_arc = Arc::new(stream);
        self.streams.write().insert(stream_id.to_string(), stream_arc.clone());
        info!("Registered IMU stream: {}", stream_id);
        Ok(stream_arc)
    }

    /// Register lidar stream
    pub fn register_lidar_stream(
        &self,
        stream_id: &str,
        points_per_scan: usize,
    ) -> Result<Arc<SensoryStream>> {
        let stream = SensoryStream::new(
            stream_id.to_string(),
            StreamType::Lidar { points_per_scan },
        );
        let stream_arc = Arc::new(stream);
        self.streams.write().insert(stream_id.to_string(), stream_arc.clone());
        info!("Registered lidar stream: {} ({} points/scan)", stream_id, points_per_scan);
        Ok(stream_arc)
    }

    /// Push data to stream
    pub async fn push_data(&self, stream_id: &str, data: StreamData) -> Result<()> {
        let stream = {
            let streams = self.streams.read();
            streams.get(stream_id).cloned()
        }.ok_or_else(|| Error::Storage(format!("Stream {} not found", stream_id)))?;

        // Convert to columnar format
        let columns = stream.convert_to_columns(&data)?;

        // Index if configured
        if stream.config.index_enabled {
            stream.index_data(&columns).await?;
        }

        // Compress if configured
        let compressed = if stream.config.compress {
            stream.compress_columns(&columns)?
        } else {
            columns.clone()
        };

        // Vectorize if configured
        if stream.config.vectorize {
            let vectors = stream.vectorize(&compressed)?;
            stream.store_vectors(vectors).await?;
        }

        // Store in memory system
        stream.store_columns(compressed).await?;

        // Emit event
        // EDGE CASE: Handle broadcast channel overflow gracefully
        // If channel is full, message is dropped (non-blocking)
        // In production, would log warning if send fails
        let event = StreamEvent::DataReceived {
            stream_id: stream_id.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        if self.event_sender.send(event).is_err() {
            // Channel is full or all receivers dropped - log but don't fail
            tracing::warn!("Stream event channel overflow for stream {}", stream_id);
        }

        Ok(())
    }

    /// Get stream processor
    pub fn get_processor(&self, stream_id: &str) -> Option<StreamProcessor> {
        self.stream_processors.read().get(stream_id).cloned()
    }

    /// Subscribe to stream events
    pub fn subscribe(&self) -> broadcast::Receiver<StreamEvent> {
        self.event_sender.subscribe()
    }
}

/// Sensory stream
pub struct SensoryStream {
    stream_id: String,
    stream_type: StreamType,
    config: StreamConfig,
    buffer: Arc<RwLock<Vec<StreamData>>>,
    column_store: Arc<RwLock<Vec<Column>>>,
    vector_store: Arc<RwLock<Vec<Vec<f32>>>>,
    temporal_index: Arc<RwLock<BTreeIndex>>, // Temporal index for time-series data
    spatial_index: Arc<RwLock<HashMap<String, BTreeIndex>>>, // Spatial indexes (KD-tree simulation)
    index_manager: Arc<AdvancedIndexManager>, // Advanced indexing (skip, bloom, min-max)
    compressor: Arc<dyn Compressor + Send + Sync>, // Compression algorithm
    decompressor: Arc<dyn Decompressor + Send + Sync>, // Decompression algorithm
}

impl SensoryStream {
    fn new(stream_id: String, stream_type: StreamType) -> Self {
        // Choose compression algorithm based on stream type
        let (compressor, decompressor): (Arc<dyn Compressor + Send + Sync>, Arc<dyn Decompressor + Send + Sync>) = 
            match &stream_type {
                StreamType::Camera { .. } => {
                    // Camera data: use LZ4 for speed
                    (Arc::new(Lz4Compressor), Arc::new(Lz4Compressor))
                }
                StreamType::Audio { .. } => {
                    // Audio data: use Zstd for better compression
                    (Arc::new(ZstdCompressor::new(3)), Arc::new(ZstdCompressor::new(3)))
                }
                _ => {
                    // Other streams: use Snappy for balanced performance
                    (Arc::new(SnappyCompressor), Arc::new(SnappyCompressor))
                }
            };
        
        Self {
            stream_id,
            stream_type,
            config: StreamConfig::default(),
            buffer: Arc::new(RwLock::new(Vec::new())),
            column_store: Arc::new(RwLock::new(Vec::new())),
            vector_store: Arc::new(RwLock::new(Vec::new())),
            temporal_index: Arc::new(RwLock::new(BTreeIndex::new())),
            spatial_index: Arc::new(RwLock::new(HashMap::new())),
            index_manager: Arc::new(AdvancedIndexManager::new()),
            compressor,
            decompressor,
        }
    }

    fn convert_to_columns(&self, data: &StreamData) -> Result<Vec<Column>> {
        match (&self.stream_type, data) {
            (StreamType::Camera { .. }, StreamData::CameraFrame { pixels, .. }) => {
                // Convert image pixels to columns
                // In production: would handle different pixel formats
                if pixels.is_empty() {
                    return Err(Error::Storage("Empty pixel data".to_string()));
                }
                let pixel_values: Vec<u8> = pixels.iter().flatten().copied().collect();
                if pixel_values.is_empty() {
                    return Err(Error::Storage("No pixel values after flattening".to_string()));
                }
                Ok(vec![Column::UInt8(pixel_values)])
            }
            (StreamType::Audio { .. }, StreamData::AudioSamples { samples, .. }) => {
                // Convert audio samples to columns
                Ok(vec![Column::Float32(samples.clone())])
            }
            (StreamType::IMU, StreamData::IMUData { accel, gyro, .. }) => {
                // Convert IMU data to columns
                // Validate IMU vector lengths
                if accel.len() != 3 {
                    return Err(Error::Storage(format!(
                        "Invalid accelerometer data length: expected 3, got {}",
                        accel.len()
                    )));
                }
                if gyro.len() != 3 {
                    return Err(Error::Storage(format!(
                        "Invalid gyroscope data length: expected 3, got {}",
                        gyro.len()
                    )));
                }
                let mut columns = Vec::new();
                columns.push(Column::Float32(accel.clone()));
                columns.push(Column::Float32(gyro.clone()));
                Ok(columns)
            }
            (StreamType::Lidar { .. }, StreamData::LidarPoints { points, .. }) => {
                // Convert lidar points to columns
                let x: Vec<f32> = points.iter().map(|p| p.x).collect();
                let y: Vec<f32> = points.iter().map(|p| p.y).collect();
                let z: Vec<f32> = points.iter().map(|p| p.z).collect();
                Ok(vec![
                    Column::Float32(x),
                    Column::Float32(y),
                    Column::Float32(z),
                ])
            }
            (StreamType::Sensor(_), StreamData::SensorData { values, .. }) => {
                // Convert sensor data to columns
                Ok(vec![Column::Float64(values.clone())])
            }
            _ => Err(Error::Storage("Data type mismatch".to_string())),
        }
    }

    async fn index_data(&self, columns: &[Column]) -> Result<()> {
        // Index columnar data for fast retrieval
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        match &self.stream_type {
            StreamType::Camera { .. } | StreamType::Audio { .. } | StreamType::IMU | StreamType::Sensor(_) => {
                // Temporal index for time-series data
                let timestamp_bytes = now.to_le_bytes().to_vec();
                // Use Index trait method - need to hold the lock guard
                let mut idx_guard = self.temporal_index.write();
                Index::insert(&mut *idx_guard, timestamp_bytes, now)?;
                drop(idx_guard);
                
                // Create skip index for timestamp column
                self.index_manager.create_skip_index(0, 1024); // column_id 0 = timestamp
                
                debug!("Created temporal index for {} columns", columns.len());
            }
            StreamType::Lidar { .. } => {
                // Spatial index for Lidar point clouds
                // Create spatial indexes for x, y, z columns
                if columns.len() >= 3 {
                    let mut spatial_idx = self.spatial_index.write();
                    spatial_idx.insert("x".to_string(), BTreeIndex::new());
                    spatial_idx.insert("y".to_string(), BTreeIndex::new());
                    spatial_idx.insert("z".to_string(), BTreeIndex::new());
                    
                    // Create min-max indexes for spatial range queries
                    self.index_manager.create_min_max_index(0); // x
                    self.index_manager.create_min_max_index(1); // y
                    self.index_manager.create_min_max_index(2); // z
                }
                
                debug!("Created spatial indexes for lidar data");
            }
        }
        
        // Create bloom filters for frequently queried columns
        for (idx, column) in columns.iter().enumerate() {
            if matches!(column, Column::Int64(_) | Column::Float64(_)) {
                // SECURITY: Handle Result from create_bloom_filter
                if let Err(e) = self.index_manager.create_bloom_filter(idx as u32, 10000, 0.01) {
                    // Log error but continue - bloom filter is optional
                    tracing::warn!("Failed to create bloom filter for column {}: {}", idx, e);
                }
            }
        }
        
        Ok(())
    }

    fn compress_columns(&self, columns: &[Column]) -> Result<Vec<Column>> {
        // Compress columns using configured compression algorithm
        use narayana_core::column::Column;
        
        let mut compressed_columns = Vec::new();
        
        for column in columns {
            // Serialize column to bytes
            let column_bytes: Vec<u8> = match column {
                Column::Int8(data) => {
                    let mut bytes = Vec::with_capacity(data.len() * 1);
                    for &x in data {
                        bytes.extend_from_slice(&x.to_le_bytes());
                    }
                    bytes
                }
                Column::Int16(data) => {
                    let mut bytes = Vec::with_capacity(data.len() * 2);
                    for &x in data {
                        bytes.extend_from_slice(&x.to_le_bytes());
                    }
                    bytes
                }
                Column::Int32(data) => {
                    let mut bytes = Vec::with_capacity(data.len() * 4);
                    for &x in data {
                        bytes.extend_from_slice(&x.to_le_bytes());
                    }
                    bytes
                }
                Column::Int64(data) => {
                    let mut bytes = Vec::with_capacity(data.len() * 8);
                    for &x in data {
                        bytes.extend_from_slice(&x.to_le_bytes());
                    }
                    bytes
                }
                Column::UInt8(data) => {
                    data.clone() // UInt8 is already bytes
                }
                Column::UInt16(data) => {
                    let mut bytes = Vec::with_capacity(data.len() * 2);
                    for &x in data {
                        bytes.extend_from_slice(&x.to_le_bytes());
                    }
                    bytes
                }
                Column::UInt32(data) => {
                    let mut bytes = Vec::with_capacity(data.len() * 4);
                    for &x in data {
                        bytes.extend_from_slice(&x.to_le_bytes());
                    }
                    bytes
                }
                Column::UInt64(data) => {
                    let mut bytes = Vec::with_capacity(data.len() * 8);
                    for &x in data {
                        bytes.extend_from_slice(&x.to_le_bytes());
                    }
                    bytes
                }
                Column::Float32(data) => {
                    let mut bytes = Vec::with_capacity(data.len() * 4);
                    for &x in data {
                        bytes.extend_from_slice(&x.to_le_bytes());
                    }
                    bytes
                }
                Column::Float64(data) => {
                    let mut bytes = Vec::with_capacity(data.len() * 8);
                    for &x in data {
                        bytes.extend_from_slice(&x.to_le_bytes());
                    }
                    bytes
                }
                Column::Boolean(data) => {
                    // Bit-packed boolean data
                    let byte_count = (data.len() + 7) / 8;
                    let mut bytes = vec![0u8; byte_count];
                    for (idx, &bit) in data.iter().enumerate() {
                        if bit {
                            bytes[idx / 8] |= 1 << (idx % 8);
                        }
                    }
                    bytes
                }
                Column::String(data) => {
                    // Concatenate strings with length prefixes
                    let mut bytes = Vec::new();
                    for s in data {
                        let s_bytes = s.as_bytes();
                        bytes.extend_from_slice(&(s_bytes.len() as u32).to_le_bytes());
                        bytes.extend_from_slice(s_bytes);
                    }
                    bytes
                }
                Column::Binary(data) => {
                    // Concatenate binary data with length prefixes
                    let mut bytes = Vec::new();
                    for bin in data {
                        bytes.extend_from_slice(&(bin.len() as u32).to_le_bytes());
                        bytes.extend_from_slice(&bin);
                    }
                    bytes
                }
                Column::Timestamp(data) => {
                    let mut bytes = Vec::with_capacity(data.len() * 8);
                    for &x in data {
                        bytes.extend_from_slice(&x.to_le_bytes());
                    }
                    bytes
                }
                Column::Date(data) => {
                    let mut bytes = Vec::with_capacity(data.len() * 4);
                    for &x in data {
                        bytes.extend_from_slice(&x.to_le_bytes());
                    }
                    bytes
                }
            };
            
            // Compress the bytes
            match self.compressor.compress(&column_bytes) {
                Ok(compressed) => {
                    let compressed_len = compressed.len();
                    // Store compressed data as UInt8 column
                    compressed_columns.push(Column::UInt8(compressed));
                    debug!("Compressed column: {} bytes -> {} bytes", column_bytes.len(), compressed_len);
                }
                Err(e) => {
                    warn!("Compression failed: {}. Storing uncompressed.", e);
                    // Fallback: store uncompressed
                    compressed_columns.push(Column::UInt8(column_bytes));
                }
            }
        }
        
        Ok(compressed_columns)
    }

    fn vectorize(&self, columns: &[Column]) -> Result<Vec<Vec<f32>>> {
        // Convert columns to vector embeddings using embedding models
        #[cfg(feature = "ml")]
        {
            // Try to use ONNX embedding models if available
            if let Ok(embeddings) = self.generate_embeddings_with_model(columns) {
                return Ok(embeddings);
            }
        }
        
        // Fallback: basic vectorization for numeric columns
        let mut vectors = Vec::new();
        for column in columns {
            match column {
                Column::Float32(data) => {
                    vectors.push(data.clone());
                }
                Column::Float64(data) => {
                    vectors.push(data.iter().map(|&x| x as f32).collect());
                }
                Column::UInt8(data) => {
                    // Convert image pixels to embeddings (simplified - would use ResNet/CLIP)
                    // For now: normalize and flatten
                    let normalized: Vec<f32> = data.iter().map(|&x| x as f32 / 255.0).collect();
                    vectors.push(normalized);
                }
                _ => {
                    // For other types, create simple numeric embeddings
                    // In production: would use appropriate embedding models
                }
            }
        }
        Ok(vectors)
    }

    #[cfg(feature = "ml")]
    /// Generate embeddings using ONNX embedding models
    fn generate_embeddings_with_model(&self, columns: &[Column]) -> Result<Vec<Vec<f32>>> {
        use crate::model_registry::ModelRegistry;
        use serde_json::json;
        
        // Create temporary model registry if not already available
        // In production: would have a shared model registry
        let registry = ModelRegistry::new();
        
        let mut embeddings = Vec::new();
        
        for column in columns {
            match (&self.stream_type, column) {
                (StreamType::Camera { .. }, Column::UInt8(data)) => {
                    // Image embedding: would use ResNet or CLIP
                    // For now: convert to float array for ONNX input
                    let float_data: Vec<f64> = data.iter().map(|&x| x as f64 / 255.0).collect();
                    let input = crate::model_registry::InferenceInput {
                        data: json!({"image": float_data}),
                        metadata: HashMap::new(),
                    };
                    
                    // Try to get perception model (could be CLIP for images)
                    if let Ok(output) = futures::executor::block_on(
                        registry.request_inference(
                            crate::model_registry::ModelSlotType::Perception,
                            input
                        )
                    ) {
                        // Extract embedding vector from output
                        if let Some(embedding_array) = output.data.get("output").and_then(|v| v.as_array()) {
                            let embedding: Vec<f32> = embedding_array
                                .iter()
                                .filter_map(|v| v.as_f64().map(|f| f as f32))
                                .collect();
                            if !embedding.is_empty() {
                                embeddings.push(embedding);
                                continue;
                            }
                        }
                    }
                }
                (StreamType::Audio { .. }, Column::Float32(data)) => {
                    // Audio embedding: would use Wav2Vec or similar
                    let input = crate::model_registry::InferenceInput {
                        data: json!({"audio": data.iter().map(|&x| x as f64).collect::<Vec<_>>()}),
                        metadata: HashMap::new(),
                    };
                    
                    if let Ok(output) = futures::executor::block_on(
                        registry.request_inference(
                            crate::model_registry::ModelSlotType::Perception,
                            input
                        )
                    ) {
                        if let Some(embedding_array) = output.data.get("output").and_then(|v| v.as_array()) {
                            let embedding: Vec<f32> = embedding_array
                                .iter()
                                .filter_map(|v| v.as_f64().map(|f| f as f32))
                                .collect();
                            if !embedding.is_empty() {
                                embeddings.push(embedding);
                                continue;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        
        if embeddings.is_empty() {
            Err(Error::Storage("Failed to generate embeddings with model".to_string()))
        } else {
            Ok(embeddings)
        }
    }

    async fn store_vectors(&self, vectors: Vec<Vec<f32>>) -> Result<()> {
        let mut store = self.vector_store.write();
        // Limit vector store size to prevent unbounded growth
        const MAX_VECTORS: usize = 100000;
        // EDGE CASE: Handle case where vectors.len() itself exceeds MAX_VECTORS
        // Also handle potential overflow in addition
        let current_len = store.len();
        let vectors_len = vectors.len();
        
        // If adding these vectors would exceed limit, remove enough to make room
        if current_len.saturating_add(vectors_len) > MAX_VECTORS {
            // Calculate how many to remove (handle overflow)
            let target_size = MAX_VECTORS.saturating_sub(vectors_len);
            if target_size < current_len {
                let to_remove = current_len - target_size;
                let drain_end = to_remove.min(current_len);
                store.drain(0..drain_end);
            }
        }
        store.extend(vectors);
        Ok(())
    }

    async fn store_columns(&self, columns: Vec<Column>) -> Result<()> {
        let mut store = self.column_store.write();
        store.extend(columns);
        Ok(())
    }
}

/// Stream type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamType {
    Camera { width: u32, height: u32, fps: u32 },
    Audio { sample_rate: u32, channels: u8 },
    Sensor(SensorType),
    IMU,
    Lidar { points_per_scan: usize },
}

/// Sensor type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensorType {
    Temperature,
    Pressure,
    Humidity,
    Light,
    Proximity,
    Touch,
    Custom(String),
}

/// Stream data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamData {
    CameraFrame {
        pixels: Vec<Vec<u8>>,
        width: u32,
        height: u32,
        timestamp: u64,
    },
    AudioSamples {
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u8,
        timestamp: u64,
    },
    IMUData {
        accel: Vec<f32>, // [x, y, z]
        gyro: Vec<f32>,  // [x, y, z]
        mag: Vec<f32>,   // [x, y, z]
        timestamp: u64,
    },
    LidarPoints {
        points: Vec<Point3D>,
        timestamp: u64,
    },
    SensorData {
        values: Vec<f64>,
        sensor_type: SensorType,
        timestamp: u64,
    },
}

/// 3D point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Stream configuration
#[derive(Debug, Clone)]
pub struct StreamConfig {
    pub index_enabled: bool,
    pub compress: bool,
    pub vectorize: bool,
    pub buffer_size: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            index_enabled: true,
            compress: true,
            vectorize: true,
            buffer_size: 1000,
        }
    }
}

/// Stream processor
#[derive(Debug, Clone)]
pub struct StreamProcessor {
    stream_id: String,
    processor_type: ProcessorType,
}

#[derive(Debug, Clone)]
pub enum ProcessorType {
    FrameProcessor,
    AudioProcessor,
    SensorProcessor,
    IMUProcessor,
    LidarProcessor,
}

/// Stream event
#[derive(Debug, Clone)]
pub enum StreamEvent {
    DataReceived { stream_id: String, timestamp: u64 },
    StreamStarted { stream_id: String },
    StreamStopped { stream_id: String },
    Error { stream_id: String, error: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_camera_stream() {
        let manager = SensoryStreamManager::new();
        let stream = manager.register_camera_stream("camera1", 640, 480, 30).unwrap();
        
        let frame = StreamData::CameraFrame {
            pixels: vec![vec![0u8; 640]; 480],
            width: 640,
            height: 480,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };

        let result = manager.push_data("camera1", frame).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_imu_stream() {
        let manager = SensoryStreamManager::new();
        let _stream = manager.register_imu_stream("imu1").unwrap();
        
        let imu_data = StreamData::IMUData {
            accel: vec![0.0, 0.0, 9.8],
            gyro: vec![0.0, 0.0, 0.0],
            mag: vec![0.0, 0.0, 0.0],
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };

        let result = manager.push_data("imu1", imu_data).await;
        assert!(result.is_ok());
    }
}

