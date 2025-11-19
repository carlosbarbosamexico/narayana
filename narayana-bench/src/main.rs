mod native_bench;
mod brain_bench;

use narayana_core::{schema::{Schema, Field, DataType}, types::TableId, column::Column};
use narayana_storage::{ColumnStore, column_store::InMemoryColumnStore};
use narayana_query::vectorized::VectorizedOps;
use std::time::Instant;
use tracing_subscriber;
use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(name = "narayana-bench")]
#[command(about = "NarayanaDB Performance Benchmarks")]
struct Cli {
    #[command(subcommand)]
    command: Option<BenchCommand>,
    
    /// Number of writes
    #[arg(long, default_value = "1000")]
    writes: usize,
    
    /// Number of reads
    #[arg(long, default_value = "500")]
    reads: usize,
}

#[derive(clap::Subcommand)]
enum BenchCommand {
    /// Run native benchmark (direct storage, no HTTP)
    Native {
        /// Number of writes
        #[arg(long, default_value = "1000")]
        writes: usize,
        
        /// Number of reads
        #[arg(long, default_value = "500")]
        reads: usize,
    },
    /// Run full benchmark suite
    Full,
    /// Run comprehensive benchmark suite (all scenarios)
    Comprehensive,
    /// Run cognitive brain benchmark suite
    Brain,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    
    match cli.command {
        Some(BenchCommand::Native { writes, reads }) => {
            native_bench::run_native_bench(writes, reads).await?;
        }
        Some(BenchCommand::Full) => {
            println!("NarayanaDB Performance Benchmarks");
            println!("==================================\n");

            // Benchmark 1: Columnar write performance
            benchmark_write_performance().await?;

            // Benchmark 2: Columnar read performance
            benchmark_read_performance().await?;

            // Benchmark 3: Vectorized operations
            benchmark_vectorized_ops()?;

            // Benchmark 4: Compression performance
            benchmark_compression()?;
        }
        Some(BenchCommand::Comprehensive) => {
            native_bench::run_comprehensive_bench().await?;
        }
        Some(BenchCommand::Brain) => {
            brain_bench::run_brain_bench().await?;
        }
        None => {
            // Default: run native benchmark with CLI args
            native_bench::run_native_bench(cli.writes, cli.reads).await?;
        }
    }

    Ok(())
}

async fn benchmark_write_performance() -> anyhow::Result<()> {
    println!("Benchmark 1: Columnar Write Performance");
    println!("----------------------------------------");

    let store = InMemoryColumnStore::new();
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

    store.create_table(table_id, schema).await?;

    let sizes = vec![10_000, 100_000, 1_000_000, 10_000_000];
    
    for size in sizes {
        let ids: Vec<i64> = (0..size as i64).collect();
        let values: Vec<f64> = (0..size).map(|i| i as f64 * 1.5).collect();
        
        let columns = vec![
            Column::Int64(ids),
            Column::Float64(values),
        ];

        let start = Instant::now();
        store.write_columns(table_id, columns).await?;
        let duration = start.elapsed();

        let throughput = size as f64 / duration.as_secs_f64();
        println!("  {} rows: {:?} ({:.2} rows/sec)", 
                 size, duration, throughput);
    }

    println!();
    Ok(())
}

async fn benchmark_read_performance() -> anyhow::Result<()> {
    println!("Benchmark 2: Columnar Read Performance");
    println!("----------------------------------------");

    let store = InMemoryColumnStore::new();
    let table_id = TableId(2);
    
    let schema = Schema::new(vec![
        Field {
            name: "id".to_string(),
            data_type: DataType::Int64,
            nullable: false,
            default_value: None,
        },
    ]);

    store.create_table(table_id, schema).await?;

    // Write test data
    let size = 1_000_000;
    let ids: Vec<i64> = (0..size as i64).collect();
    store.write_columns(table_id, vec![Column::Int64(ids)]).await?;

    let start = Instant::now();
    let columns = store.read_columns(table_id, vec![0], 0, size).await?;
    let duration = start.elapsed();

    let throughput = size as f64 / duration.as_secs_f64();
    println!("  Read {} rows: {:?} ({:.2} rows/sec)", 
             size, duration, throughput);
    println!("  Columns read: {}", columns.len());

    println!();
    Ok(())
}

fn benchmark_vectorized_ops() -> anyhow::Result<()> {
    println!("Benchmark 3: Vectorized Operations");
    println!("-----------------------------------");

    let sizes = vec![10_000, 100_000, 1_000_000];
    
    for size in sizes {
        let data: Vec<i64> = (0..size as i64).collect();
        let column = Column::Int64(data);
        let value = serde_json::Value::Number((size / 2).into());

        // Benchmark filter
        let start = Instant::now();
        let mask = VectorizedOps::compare_eq(&column, &value);
        let filter_duration = start.elapsed();
        let filtered = VectorizedOps::filter(&column, &mask);
        
        println!("  Size {}: Filter {:?} (filtered {} rows)", 
                 size, filter_duration, filtered.len());

        // Benchmark aggregation
        let start = Instant::now();
        let sum = VectorizedOps::sum(&column);
        let agg_duration = start.elapsed();
        println!("  Size {}: Sum {:?}", size, agg_duration);
    }

    println!();
    Ok(())
}

fn benchmark_compression() -> anyhow::Result<()> {
    println!("Benchmark 4: Compression Performance");
    println!("-------------------------------------");

    let data_size = 1_000_000;
    let test_data: Vec<u8> = (0..data_size).map(|i| (i % 256) as u8).collect();

    let compressors = vec![
        ("LZ4", narayana_core::types::CompressionType::LZ4),
        ("Zstd", narayana_core::types::CompressionType::Zstd),
        ("Snappy", narayana_core::types::CompressionType::Snappy),
    ];

    for (name, comp_type) in compressors {
        let compressor = narayana_storage::compression::create_compressor(comp_type);
        let decompressor = narayana_storage::compression::create_decompressor(comp_type);

        let start = Instant::now();
        let compressed = compressor.compress(&test_data)?;
        let compress_time = start.elapsed();

        let start = Instant::now();
        let decompressed = decompressor.decompress(&compressed, data_size)?;
        let decompress_time = start.elapsed();

        let ratio = compressed.len() as f64 / data_size as f64;
        println!("  {}: compress {:?}, decompress {:?}, ratio: {:.2}%", 
                 name, compress_time, decompress_time, ratio * 100.0);
    }

    println!();
    Ok(())
}

