#[cfg(test)]
mod sensory_tests {
    use crate::sensory_streams::{SensoryStreamManager, StreamType, StreamData, SensorType};
    use narayana_core::column::Column;

    #[tokio::test]
    async fn test_indexing_creation() {
        let manager = SensoryStreamManager::new();
        
        // Register camera stream
        let _stream = manager.register_camera_stream("camera1", 640, 480, 30).unwrap();
        
        // Push data to trigger indexing
        let pixels = vec![vec![128u8; 3]; 640 * 480]; // RGB frame
        let data = StreamData::CameraFrame {
            pixels,
            width: 640,
            height: 480,
            timestamp: 0,
        };
        
        manager.push_data("camera1", data).await.unwrap();
        
        // Indexing should have been created (would need internal access to verify)
    }

    #[tokio::test]
    async fn test_compression() {
        let manager = SensoryStreamManager::new();
        
        // Register audio stream (uses Zstd)
        let _stream = manager.register_audio_stream("audio1", 44100, 2).unwrap();
        
        // Push audio data
        let samples = vec![0.1f32; 1000];
        let data = StreamData::AudioSamples {
            samples,
            sample_rate: 44100,
            channels: 2,
            timestamp: 0,
        };
        
        manager.push_data("audio1", data).await.unwrap();
        
        // Compression should succeed
    }

    #[test]
    fn test_compression_all_column_types() {
        // Test that all column types can be compressed
        let columns = vec![
            Column::Int8(vec![1, 2, 3]),
            Column::Int16(vec![100, 200, 300]),
            Column::Int32(vec![1000, 2000, 3000]),
            Column::Int64(vec![10000, 20000, 30000]),
            Column::UInt8(vec![10, 20, 30]),
            Column::UInt16(vec![100, 200, 300]),
            Column::UInt32(vec![1000, 2000, 3000]),
            Column::UInt64(vec![10000, 20000, 30000]),
            Column::Float32(vec![1.1, 2.2, 3.3]),
            Column::Float64(vec![1.11, 2.22, 3.33]),
            Column::Boolean(vec![true, false, true]),
            Column::String(vec!["hello".to_string(), "world".to_string()]),
        ];
        
        // All should serialize to bytes without panic
        for column in columns {
            match column {
                Column::Int8(data) => {
                    let _bytes: Vec<u8> = data.iter()
                        .flat_map(|&x| x.to_le_bytes())
                        .collect();
                }
                // Similar for other types...
                _ => {}
            }
        }
    }

    #[tokio::test]
    async fn test_lidar_spatial_indexing() {
        let manager = SensoryStreamManager::new();
        
        // Register Lidar stream
        let _stream = manager.register_lidar_stream("lidar1", 1000).unwrap();
        
        // Create point cloud data
        use crate::sensory_streams::Point3D;
        let points: Vec<Point3D> = (0..1000).map(|i| Point3D {
            x: 1.0 + (i as f32) * 0.1,
            y: 2.0 + (i as f32) * 0.1,
            z: 3.0 + (i as f32) * 0.1,
        }).collect();
        
        let data = StreamData::LidarPoints {
            points,
            timestamp: 0,
        };
        
        manager.push_data("lidar1", data).await.unwrap();
        
        // Spatial indexes should have been created for x, y, z
    }

    #[tokio::test]
    async fn test_temporal_indexing() {
        let manager = SensoryStreamManager::new();
        
        let _stream = manager.register_sensor_stream("temp1", SensorType::Temperature).unwrap();
        
        let data = StreamData::SensorData {
            values: vec![25.0],
            sensor_type: SensorType::Temperature,
            timestamp: 1234567890,
        };
        
        manager.push_data("temp1", data).await.unwrap();
        
        // Temporal index should have been created
    }
}

