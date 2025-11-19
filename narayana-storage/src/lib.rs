pub mod column_store;
pub mod persistent_column_store;
pub mod compression;
pub mod block;
pub mod writer;
pub mod reader;
pub mod index;
pub mod cache;
pub mod performance;
pub mod ultra_performance;
pub mod sharding;
pub mod transaction_engine;
pub mod encryption;
pub mod key_management;
pub mod quantum_sync;
pub mod consensus;
pub mod network_sync;
pub mod network_sync_impl;
pub mod columnar_format;
pub mod database_manager;
pub mod true_columnar;
pub mod advanced_indexing;
pub mod advanced_indexing_impl;
pub mod ai_optimized;
pub mod vector_search;
pub mod small_writes;
pub mod advanced_joins;
pub mod auto_increment;
pub mod mutable_data;
pub mod webhooks;
pub mod self_healing;
pub mod cognitive;
pub mod persistent_memory_store;
pub mod parallel_thoughts;
pub mod native_cache;
pub mod infinite_context;
pub mod quantum_sync_enhanced;
pub mod auto_scaling;
pub mod advanced_load_balancer;
pub mod persistence;
pub mod human_search;
pub mod query_learning;
pub mod predictive_scaling;
pub mod dynamic_schema;
pub mod dynamic_output;
pub mod migration_free;
pub mod dynamic_thoughts;
pub mod bug_detection;
pub mod security_utils;
pub mod security_limits;
pub mod native_events;
pub mod workers;
pub mod threading;
pub mod quantum_optimization;
pub mod optimization_algorithms;
pub mod gpu_execution;
pub mod thought_kernel;
pub mod reinforcement_learning;
pub mod hnsw;
pub mod sensory_streams;
pub mod cognitive_graph;
pub mod model_registry;
pub mod thought_serialization;
pub mod autonomous_schema;
pub mod embedded;
pub mod conscience_persistent_loop;
pub mod global_workspace;
pub mod background_daemon;
pub mod working_memory;
pub mod memory_bridge;
pub mod narrative_generator;
pub mod attention_router;
pub mod dreaming_loop;
pub mod cpl_manager;
pub mod genetics;
pub mod traits_equations;
pub mod talking_cricket;

// Test modules
#[cfg(test)]
mod query_learning_tests;
#[cfg(test)]
mod thought_tracking_tests;
#[cfg(test)]
mod rl_tests;
#[cfg(test)]
mod sensory_tests;
#[cfg(test)]
mod cpl_tests;

pub use column_store::{ColumnStore, InMemoryColumnStore};
pub use compression::{Compressor, Decompressor};
pub use block::{Block, BlockMetadata};
pub use writer::ColumnWriter;
pub use reader::ColumnReader;

// GPU execution exports
pub use gpu_execution::{
    Backend, GpuEngine, GpuTensor, GpuColumn, GpuMask, GpuBackend,
    GpuEmbeddingStore, CpuBackend, MetalBackend, CudaBackend, VulkanBackend,
};
pub use dynamic_output::DynamicOutputManager;

#[cfg(test)]
mod tests {
    use super::*;
    use narayana_core::{schema::{Schema, Field, DataType}, types::TableId, column::Column};

    #[tokio::test]
    async fn test_column_store() {
        let store = column_store::InMemoryColumnStore::new();
        let table_id = TableId(1);
        
        let schema = Schema::new(vec![
            Field {
                name: "id".to_string(),
                data_type: DataType::Int64,
                nullable: false,
                default_value: None,
            },
            Field {
                name: "value".to_string(),
                data_type: DataType::Float64,
                nullable: false,
                default_value: None,
            },
        ]);

        store.create_table(table_id, schema.clone()).await.unwrap();
        
        let columns = vec![
            Column::Int64(vec![1, 2, 3, 4, 5]),
            Column::Float64(vec![1.1, 2.2, 3.3, 4.4, 5.5]),
        ];

        store.write_columns(table_id, columns.clone()).await.unwrap();
        
        let read_columns = store.read_columns(table_id, vec![0, 1], 0, 5).await.unwrap();
        assert_eq!(read_columns.len(), 2);
    }

    #[test]
    fn test_compression() {
        let compressor = compression::create_compressor(narayana_core::types::CompressionType::LZ4);
        let data = b"test data for compression";
        let compressed = compressor.compress(data).unwrap();
        assert!(compressed.len() < data.len() || compressed.len() == data.len());
        
        let decompressor = compression::create_decompressor(narayana_core::types::CompressionType::LZ4);
        let decompressed = decompressor.decompress(&compressed, data.len()).unwrap();
        assert_eq!(decompressed, data);
    }
}
