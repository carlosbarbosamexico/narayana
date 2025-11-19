// Real performance benchmarks to verify claims
// Tests actual throughput and latency

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use narayana_storage::cognitive::{CognitiveBrain, MemoryType};
use narayana_storage::vector_search::{VectorIndex, IndexType, Embedding};
use narayana_storage::hnsw::HNSWIndex;
use std::collections::HashMap;
use std::time::Instant;

/// Benchmark memory operations
fn benchmark_memory_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_operations");
    
    let brain = CognitiveBrain::new();
    
    // Benchmark memory storage
    group.bench_function("store_memory", |b| {
        b.iter(|| {
            let _ = brain.store_memory(
                MemoryType::Episodic,
                serde_json::json!({"event": "test"}),
                None,
                vec!["test".to_string()],
                None,
            );
        });
    });
    
    // Benchmark memory retrieval
    let memory_id = brain.store_memory(
        MemoryType::Episodic,
        serde_json::json!({"event": "test"}),
        None,
        vec!["test".to_string()],
        None,
    ).unwrap();
    
    group.bench_function("retrieve_memory", |b| {
        b.iter(|| {
            let _ = brain.retrieve_memories_by_type(
                MemoryType::Episodic,
                Some("test"),
                None,
                10,
            );
        });
    });
    
    group.finish();
}

/// Benchmark vector search operations
fn benchmark_vector_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("vector_search");
    
    let dimension = 384;
    let num_vectors = 10000;
    
    // Benchmark HNSW index
    group.bench_with_input(
        BenchmarkId::new("hnsw_search", num_vectors),
        &num_vectors,
        |b, &n| {
            let index = HNSWIndex::new(16, 200, dimension);
            
            // Add vectors
            for i in 0..n {
                let vector: Vec<f32> = (0..dimension)
                    .map(|_| rand::random::<f32>())
                    .collect();
                index.insert(i, vector).unwrap();
            }
            
            let query: Vec<f32> = (0..dimension)
                .map(|_| rand::random::<f32>())
                .collect();
            
            b.iter(|| {
                let _ = index.search(black_box(&query), 10);
            });
        },
    );
    
    // Benchmark flat index (linear search)
    group.bench_with_input(
        BenchmarkId::new("flat_search", num_vectors),
        &num_vectors,
        |b, &n| {
            let index = VectorIndex::new(dimension, IndexType::Flat);
            
            // Add vectors
            for i in 0..n {
                let embedding = Embedding {
                    id: i,
                    vector: (0..dimension)
                        .map(|_| rand::random::<f32>())
                        .collect(),
                    metadata: HashMap::new(),
                    timestamp: 0,
                };
                index.add(embedding).unwrap();
            }
            
            let query: Vec<f32> = (0..dimension)
                .map(|_| rand::random::<f32>())
                .collect();
            
            b.iter(|| {
                let _ = index.search(black_box(&query), 10);
            });
        },
    );
    
    group.finish();
}

/// Benchmark throughput (operations per second)
fn benchmark_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    group.sample_size(100);
    
    let brain = CognitiveBrain::new();
    
    // Benchmark memory write throughput
    group.bench_function("memory_write_throughput", |b| {
        let mut count = 0;
        b.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                let _ = brain.store_memory(
                    MemoryType::Episodic,
                    serde_json::json!({"count": count}),
                    None,
                    vec![],
                    None,
                );
                count += 1;
            }
            start.elapsed()
        });
    });
    
    group.finish();
}

/// Benchmark latency (time per operation)
fn benchmark_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency");
    group.sample_size(1000);
    
    let brain = CognitiveBrain::new();
    
    // Store some memories first
    for i in 0..100 {
        let _ = brain.store_memory(
            MemoryType::Episodic,
            serde_json::json!({"id": i}),
            None,
            vec![],
            None,
        );
    }
    
    // Benchmark memory read latency
    group.bench_function("memory_read_latency", |b| {
        b.iter(|| {
            let _ = brain.retrieve_memories_by_type(
                MemoryType::Episodic,
                None,
                None,
                10,
            );
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_memory_operations,
    benchmark_vector_search,
    benchmark_throughput,
    benchmark_latency
);
criterion_main!(benches);

