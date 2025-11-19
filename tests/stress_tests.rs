use narayana_core::{schema::{Schema, Field, DataType}, types::TableId, column::Column};
use narayana_storage::{ColumnStore, InMemoryColumnStore};
use narayana_query::vectorized::VectorizedOps;
use std::time::Instant;

#[tokio::test]
async fn test_stress_many_tables() {
    let store = InMemoryColumnStore::new();
    
    let start = Instant::now();
    for i in 1..=1000 {
        let table_id = TableId(i);
        let schema = Schema::new(vec![
            Field {
                name: "id".to_string(),
                data_type: DataType::Int64,
                nullable: false,
                default_value: None,
            },
        ]);
        store.create_table(table_id, schema).await.unwrap();
    }
    let duration = start.elapsed();
    
    println!("Created 1000 tables in {:?}", duration);
    assert!(duration.as_secs() < 10);
}

#[tokio::test]
async fn test_stress_large_dataset() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(1);
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);

    store.create_table(table_id, schema).await.unwrap();
    
    let start = Instant::now();
    let large_data: Vec<i64> = (0..10_000_000).collect();
    let columns = vec![Column::Int64(large_data)];
    store.write_columns(table_id, columns).await.unwrap();
    let write_duration = start.elapsed();
    
    println!("Wrote 10M rows in {:?}", write_duration);
    
    let start = Instant::now();
    let _read = store.read_columns(table_id, vec![0], 0, 10_000_000).await.unwrap();
    let read_duration = start.elapsed();
    
    println!("Read 10M rows in {:?}", read_duration);
    assert!(write_duration.as_secs() < 30);
    assert!(read_duration.as_secs() < 30);
}

#[tokio::test]
async fn test_stress_many_columns() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(1);
    
    let fields: Vec<Field> = (0..1000).map(|i| Field {
        name: format!("col_{}", i),
        data_type: DataType::Int32,
        nullable: false,
        default_value: None,
    }).collect();
    
    let schema = Schema::new(fields);
    store.create_table(table_id, schema).await.unwrap();
    
    let start = Instant::now();
    let columns: Vec<Column> = (0..1000).map(|_| Column::Int32((0..1000).collect())).collect();
    store.write_columns(table_id, columns).await.unwrap();
    let duration = start.elapsed();
    
    println!("Wrote 1000 columns with 1000 rows each in {:?}", duration);
    assert!(duration.as_secs() < 60);
}

#[tokio::test]
async fn test_stress_vectorized_large() {
    let start = Instant::now();
    let data: Vec<i64> = (0..10_000_000).collect();
    let column = Column::Int64(data);
    
    let value = serde_json::Value::Number(5_000_000.into());
    let mask = VectorizedOps::compare_eq(&column, &value);
    let filtered = VectorizedOps::filter(&column, &mask);
    
    let duration = start.elapsed();
    println!("Filtered 10M rows in {:?}", duration);
    assert_eq!(filtered.len(), 1);
    assert!(duration.as_secs() < 5);
}

#[tokio::test]
async fn test_stress_vectorized_sum_large() {
    let start = Instant::now();
    let data: Vec<i64> = (0..10_000_000).collect();
    let column = Column::Int64(data);
    
    let sum = VectorizedOps::sum(&column);
    let duration = start.elapsed();
    
    println!("Summed 10M rows in {:?}", duration);
    assert!(sum.is_some());
    assert!(duration.as_secs() < 5);
}

#[tokio::test]
async fn test_stress_vectorized_min_max_large() {
    let start = Instant::now();
    let data: Vec<i64> = (0..10_000_000).collect();
    let column = Column::Int64(data);
    
    let min = VectorizedOps::min(&column);
    let max = VectorizedOps::max(&column);
    let duration = start.elapsed();
    
    println!("Min/Max 10M rows in {:?}", duration);
    assert_eq!(min, Some(serde_json::Value::Number(0.into())));
    assert_eq!(max, Some(serde_json::Value::Number(9_999_999.into())));
    assert!(duration.as_secs() < 5);
}

#[tokio::test]
async fn test_stress_rapid_operations() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(1);
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);

    store.create_table(table_id, schema).await.unwrap();
    
    let start = Instant::now();
    for i in 0..10000 {
        let columns = vec![Column::Int64(vec![i as i64])];
        let _ = store.write_columns(table_id, columns).await;
    }
    let duration = start.elapsed();
    
    println!("10K rapid writes in {:?}", duration);
    assert!(duration.as_secs() < 30);
}

#[tokio::test]
async fn test_stress_create_delete_cycle() {
    let store = InMemoryColumnStore::new();
    
    let start = Instant::now();
    for i in 1..=100 {
        let table_id = TableId(i);
        let schema = Schema::new(vec![
            Field {
                name: "id".to_string(),
                data_type: DataType::Int64,
                nullable: false,
                default_value: None,
            },
        ]);
        store.create_table(table_id, schema).await.unwrap();
        store.delete_table(table_id).await.unwrap();
    }
    let duration = start.elapsed();
    
    println!("100 create/delete cycles in {:?}", duration);
    assert!(duration.as_secs() < 10);
}

#[tokio::test]
async fn test_stress_string_operations() {
    let start = Instant::now();
    let data: Vec<String> = (0..1_000_000).map(|i| format!("string_{}", i)).collect();
    let column = Column::String(data);
    
    let value = serde_json::Value::String("string_500000".to_string());
    let mask = VectorizedOps::compare_eq(&column, &value);
    let filtered = VectorizedOps::filter(&column, &mask);
    
    let duration = start.elapsed();
    println!("Filtered 1M strings in {:?}", duration);
    assert_eq!(filtered.len(), 1);
    assert!(duration.as_secs() < 10);
}

#[tokio::test]
async fn test_stress_mixed_operations() {
    let store = InMemoryColumnStore::new();
    
    let start = Instant::now();
    for i in 1..=100 {
        let table_id = TableId(i);
        let schema = Schema::new(vec![
            Field {
                name: "id".to_string(),
                data_type: DataType::Int64,
                nullable: false,
                default_value: None,
            },
        ]);
        store.create_table(table_id, schema).await.unwrap();
        
        let columns = vec![Column::Int64((0..1000).collect())];
        store.write_columns(table_id, columns).await.unwrap();
        
        let _ = store.read_columns(table_id, vec![0], 0, 1000).await;
        let _ = store.get_schema(table_id).await;
    }
    let duration = start.elapsed();
    
    println!("100 mixed operations in {:?}", duration);
    assert!(duration.as_secs() < 30);
}

#[tokio::test]
async fn test_stress_complex_filter() {
    let data: Vec<i32> = (0..1_000_000).collect();
    let column = Column::Int32(data);
    
    let start = Instant::now();
    
    // Multiple comparisons
    let value1 = serde_json::Value::Number(100000.into());
    let mask1 = VectorizedOps::compare_gt(&column, &value1);
    
    let value2 = serde_json::Value::Number(200000.into());
    let mask2 = VectorizedOps::compare_lt(&column, &value2);
    
    // Combine masks (AND operation)
    let combined_mask: Vec<bool> = mask1.iter().zip(mask2.iter())
        .map(|(a, b)| *a && *b)
        .collect();
    
    let filtered = VectorizedOps::filter(&column, &combined_mask);
    
    let duration = start.elapsed();
    println!("Complex filter on 1M rows in {:?}", duration);
    assert_eq!(filtered.len(), 100000);
    assert!(duration.as_secs() < 5);
}

#[tokio::test]
async fn test_stress_memory_pressure() {
    let store = InMemoryColumnStore::new();
    let table_id = TableId(1);
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);

    store.create_table(table_id, schema).await.unwrap();
    
    // Write many batches
    for batch in 0..100 {
        let start_idx = batch * 10000;
        let end_idx = start_idx + 10000;
        let data: Vec<i64> = (start_idx..end_idx).collect();
        let columns = vec![Column::Int64(data)];
        store.write_columns(table_id, columns).await.unwrap();
    }
    
    // Read all back
    let _ = store.read_columns(table_id, vec![0], 0, 1_000_000).await.unwrap();
}

