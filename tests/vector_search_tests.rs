// Tests for vector search

use narayana_storage::vector_search::*;

#[test]
fn test_vector_index_creation() {
    let index = VectorIndex::new(128, IndexType::Flat);
    assert_eq!(index.dimension, 128);
}

#[test]
fn test_vector_index_add() {
    let index = VectorIndex::new(128, IndexType::Flat);
    let embedding = Embedding {
        id: 1,
        vector: vec![0.0f32; 128],
        metadata: std::collections::HashMap::new(),
        timestamp: 0,
    };
    
    index.add(embedding).unwrap();
}

#[test]
fn test_vector_index_add_wrong_dimension() {
    let index = VectorIndex::new(128, IndexType::Flat);
    let embedding = Embedding {
        id: 1,
        vector: vec![0.0f32; 64], // Wrong dimension
        metadata: std::collections::HashMap::new(),
        timestamp: 0,
    };
    
    let result = index.add(embedding);
    assert!(result.is_err());
}

#[test]
fn test_vector_index_search() {
    let index = VectorIndex::new(3, IndexType::Flat);
    
    // Add some embeddings
    let embedding1 = Embedding {
        id: 1,
        vector: vec![1.0f32, 0.0, 0.0],
        metadata: std::collections::HashMap::new(),
        timestamp: 0,
    };
    let embedding2 = Embedding {
        id: 2,
        vector: vec![0.0f32, 1.0, 0.0],
        metadata: std::collections::HashMap::new(),
        timestamp: 0,
    };
    
    index.add(embedding1).unwrap();
    index.add(embedding2).unwrap();
    
    // Search for similar vector
    let query = vec![1.0f32, 0.0, 0.0];
    let results = index.search(&query, 1).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, 1);
}

#[test]
fn test_vector_store_creation() {
    let store = VectorStore::new();
    // Should create successfully
}

#[test]
fn test_vector_store_create_index() {
    let store = VectorStore::new();
    store.create_index("test".to_string(), 128, IndexType::Flat);
    // Should create successfully
}

#[test]
fn test_vector_store_add_embedding() {
    let store = VectorStore::new();
    store.create_index("test".to_string(), 128, IndexType::Flat);
    
    let embedding = Embedding {
        id: 1,
        vector: vec![0.0f32; 128],
        metadata: std::collections::HashMap::new(),
        timestamp: 0,
    };
    
    store.add_embedding("test", embedding).unwrap();
}

#[test]
fn test_vector_store_search() {
    let store = VectorStore::new();
    store.create_index("test".to_string(), 3, IndexType::Flat);
    
    let embedding = Embedding {
        id: 1,
        vector: vec![1.0f32, 0.0, 0.0],
        metadata: std::collections::HashMap::new(),
        timestamp: 0,
    };
    
    store.add_embedding("test", embedding).unwrap();
    
    let query = vec![1.0f32, 0.0, 0.0];
    let results = store.search("test", &query, 1).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_hybrid_search_creation() {
    let store = VectorStore::new();
    let hybrid = HybridSearch::new(store);
    // Should create successfully
}

#[test]
fn test_hybrid_search_with_filters() {
    let store = VectorStore::new();
    store.create_index("test".to_string(), 3, IndexType::Flat);
    
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("status".to_string(), serde_json::json!("active"));
    
    let embedding = Embedding {
        id: 1,
        vector: vec![1.0f32, 0.0, 0.0],
        metadata,
        timestamp: 0,
    };
    
    store.add_embedding("test", embedding).unwrap();
    
    let hybrid = HybridSearch::new(store);
    let mut filters = std::collections::HashMap::new();
    filters.insert("status".to_string(), serde_json::json!("active"));
    
    let query = vec![1.0f32, 0.0, 0.0];
    let results = hybrid.search_with_filters("test", &query, 1, filters).unwrap();
    assert_eq!(results.len(), 1);
}

