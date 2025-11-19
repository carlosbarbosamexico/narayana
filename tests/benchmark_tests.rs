use criterion::{black_box, criterion_group, criterion_main, Criterion};
use narayana_core::column::Column;
use narayana_query::vectorized::VectorizedOps;
use narayana_storage::compression::{create_compressor, create_decompressor};
use narayana_core::types::CompressionType;

fn bench_vectorized_filter(c: &mut Criterion) {
    let data: Vec<i32> = (0..100000).collect();
    let column = Column::Int32(data);
    let mask: Vec<bool> = (0..100000).map(|i| i % 2 == 0).collect();
    
    c.bench_function("vectorized_filter_100k", |b| {
        b.iter(|| {
            VectorizedOps::filter(black_box(&column), black_box(&mask))
        })
    });
}

fn bench_vectorized_sum(c: &mut Criterion) {
    let data: Vec<i64> = (0..1000000).collect();
    let column = Column::Int64(data);
    
    c.bench_function("vectorized_sum_1m", |b| {
        b.iter(|| {
            VectorizedOps::sum(black_box(&column))
        })
    });
}

fn bench_vectorized_compare(c: &mut Criterion) {
    let data: Vec<i32> = (0..100000).collect();
    let column = Column::Int32(data);
    let value = serde_json::Value::Number(50000.into());
    
    c.bench_function("vectorized_compare_eq_100k", |b| {
        b.iter(|| {
            VectorizedOps::compare_eq(black_box(&column), black_box(&value))
        })
    });
}

fn bench_compression_lz4(c: &mut Criterion) {
    let data: Vec<u8> = (0..100000).map(|i| (i % 256) as u8).collect();
    let compressor = create_compressor(CompressionType::LZ4);
    let decompressor = create_decompressor(CompressionType::LZ4);
    
    c.bench_function("compress_lz4_100k", |b| {
        b.iter(|| {
            let compressed = compressor.compress(black_box(&data)).unwrap();
            black_box(compressed)
        })
    });
    
    let compressed = compressor.compress(&data).unwrap();
    c.bench_function("decompress_lz4_100k", |b| {
        b.iter(|| {
            decompressor.decompress(black_box(&compressed), data.len()).unwrap()
        })
    });
}

fn bench_compression_zstd(c: &mut Criterion) {
    let data: Vec<u8> = (0..100000).map(|i| (i % 256) as u8).collect();
    let compressor = create_compressor(CompressionType::Zstd);
    let decompressor = create_decompressor(CompressionType::Zstd);
    
    c.bench_function("compress_zstd_100k", |b| {
        b.iter(|| {
            let compressed = compressor.compress(black_box(&data)).unwrap();
            black_box(compressed)
        })
    });
    
    let compressed = compressor.compress(&data).unwrap();
    c.bench_function("decompress_zstd_100k", |b| {
        b.iter(|| {
            decompressor.decompress(black_box(&compressed), data.len()).unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_vectorized_filter,
    bench_vectorized_sum,
    bench_vectorized_compare,
    bench_compression_lz4,
    bench_compression_zstd
);
criterion_main!(benches);

