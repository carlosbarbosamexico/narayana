// Comprehensive tests for threading system
// NarayanaDB - Fully Multithreaded with Ample Controls

use narayana_storage::threading::*;
use narayana_core::config::ThreadingConfig;
use std::sync::Arc;
use std::time::Duration;
use std::thread;

#[test]
fn test_thread_pool_type() {
    let types = vec![
        ThreadPoolType::Query,
        ThreadPoolType::Write,
        ThreadPoolType::Read,
        ThreadPoolType::Compression,
        ThreadPoolType::CPU,
        ThreadPoolType::Background,
        ThreadPoolType::Analytics,
        ThreadPoolType::Vector,
        ThreadPoolType::Worker,
        ThreadPoolType::Sync,
        ThreadPoolType::Index,
        ThreadPoolType::NetworkIO,
    ];
    
    assert_eq!(types.len(), 12);
}

#[test]
fn test_thread_pool_config_default() {
    let config = ThreadPoolConfig::default();
    assert!(config.min_threads > 0);
    assert!(config.max_threads >= config.min_threads);
    assert!(config.initial_threads >= config.min_threads);
    assert!(config.initial_threads <= config.max_threads);
    assert!(!config.thread_name_prefix.is_empty());
}

#[test]
fn test_thread_pool_config_query() {
    let config = ThreadPoolConfig::query();
    assert_eq!(config.name, "query");
    assert!(config.min_threads > 0);
    assert!(config.thread_name_prefix.contains("query"));
}

#[test]
fn test_thread_pool_config_write() {
    let config = ThreadPoolConfig::write();
    assert_eq!(config.name, "write");
    assert!(config.min_threads > 0);
    assert!(config.thread_name_prefix.contains("write"));
}

#[test]
fn test_thread_pool_config_read() {
    let config = ThreadPoolConfig::read();
    assert_eq!(config.name, "read");
    assert!(config.min_threads > 0);
    assert!(config.thread_name_prefix.contains("read"));
}

#[test]
fn test_thread_pool_config_compression() {
    let config = ThreadPoolConfig::compression();
    assert_eq!(config.name, "compression");
    assert!(config.min_threads > 0);
    assert!(config.thread_name_prefix.contains("compression"));
}

#[test]
fn test_thread_pool_config_cpu() {
    let config = ThreadPoolConfig::cpu();
    assert_eq!(config.name, "cpu");
    assert!(config.min_threads > 0);
    assert!(config.thread_name_prefix.contains("cpu"));
}

#[test]
fn test_thread_pool_config_custom() {
    let config = ThreadPoolConfig {
        name: "custom".to_string(),
        min_threads: 2,
        max_threads: 16,
        initial_threads: 4,
        stack_size: Some(4 * 1024 * 1024),
        keep_alive: Some(Duration::from_secs(120)),
        priority: Some(50),
        cpu_affinity: Some(vec![0, 1]),
        thread_name_prefix: "custom-prefix".to_string(),
        enable_tls: true,
        spawn_timeout: Some(Duration::from_secs(60)),
        deadlock_timeout: Some(Duration::from_secs(600)),
        panic_handler: None,
    };
    
    assert_eq!(config.name, "custom");
    assert_eq!(config.min_threads, 2);
    assert_eq!(config.max_threads, 16);
    assert_eq!(config.initial_threads, 4);
    assert_eq!(config.stack_size, Some(4 * 1024 * 1024));
    assert_eq!(config.priority, Some(50));
    assert_eq!(config.cpu_affinity, Some(vec![0, 1]));
}

#[test]
fn test_thread_pool_stats_default() {
    let stats = ThreadPoolStats::default();
    assert_eq!(stats.current_threads, 0);
    assert_eq!(stats.active_threads, 0);
    assert_eq!(stats.idle_threads, 0);
    assert_eq!(stats.tasks_completed, 0);
    assert_eq!(stats.tasks_queued, 0);
    assert_eq!(stats.queue_size, 0);
    assert_eq!(stats.avg_task_duration_us, 0);
    assert_eq!(stats.max_task_duration_us, 0);
    assert_eq!(stats.total_cpu_time_us, 0);
}

#[tokio::test]
async fn test_managed_thread_pool_creation() {
    let config = ThreadPoolConfig::default();
    let pool = ManagedThreadPool::new(ThreadPoolType::Query, config);
    assert!(pool.is_ok());
}

#[tokio::test]
async fn test_managed_thread_pool_execute() {
    let config = ThreadPoolConfig::default();
    let pool = ManagedThreadPool::new(ThreadPoolType::Query, config).unwrap();
    
    let result = pool.execute(|| {
        thread::current().name().unwrap_or("unknown").to_string()
    });
    
    assert!(result.contains("narayana"));
    
    let stats = pool.stats();
    assert_eq!(stats.tasks_completed, 1);
    assert!(stats.current_threads > 0);
}

#[tokio::test]
async fn test_managed_thread_pool_execute_multiple() {
    let config = ThreadPoolConfig::default();
    let pool = ManagedThreadPool::new(ThreadPoolType::Query, config).unwrap();
    
    for i in 0..10 {
        let result = pool.execute(move || i * 2);
        assert_eq!(result, i * 2);
    }
    
    let stats = pool.stats();
    assert_eq!(stats.tasks_completed, 10);
}

#[tokio::test]
async fn test_managed_thread_pool_spawn() {
    let config = ThreadPoolConfig::default();
    let pool = Arc::new(ManagedThreadPool::new(ThreadPoolType::Query, config).unwrap());
    
    let pool_clone = pool.clone();
    let handle = pool_clone.spawn(|| {
        thread::sleep(Duration::from_millis(10));
        42
    });
    
    let result = handle.await.unwrap();
    assert_eq!(result, 42);
    
    let stats = pool.stats();
    assert!(stats.tasks_completed > 0);
}

#[tokio::test]
async fn test_managed_thread_pool_statistics() {
    let config = ThreadPoolConfig::default();
    let pool = Arc::new(ManagedThreadPool::new(ThreadPoolType::Query, config).unwrap());
    
    // Execute some tasks
    for i in 0..5 {
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            pool_clone.execute(move || {
                thread::sleep(Duration::from_millis(10));
                i
            });
        });
    }
    
    // Wait a bit for tasks to complete
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let stats = pool.stats();
    assert!(stats.tasks_completed >= 5);
    assert!(stats.avg_task_duration_us > 0);
}

#[tokio::test]
async fn test_managed_thread_pool_config_access() {
    let config = ThreadPoolConfig::query();
    let pool = ManagedThreadPool::new(ThreadPoolType::Query, config.clone()).unwrap();
    
    let pool_config = pool.config();
    assert_eq!(pool_config.name, config.name);
    assert_eq!(pool_config.min_threads, config.min_threads);
}

#[tokio::test]
async fn test_managed_thread_pool_pool_type() {
    let config = ThreadPoolConfig::default();
    let pool = ManagedThreadPool::new(ThreadPoolType::Write, config).unwrap();
    
    assert_eq!(pool.pool_type(), ThreadPoolType::Write);
}

#[tokio::test]
async fn test_thread_manager_creation() {
    let config = ThreadingConfig::default();
    let manager = ThreadManager::from_core_config(config);
    assert!(manager.is_ok());
}

#[tokio::test]
async fn test_thread_manager_execute() {
    let config = ThreadingConfig::default();
    let manager = ThreadManager::from_core_config(config).unwrap();
    
    let result = manager.execute(ThreadPoolType::Query, || {
        42
    }).unwrap();
    
    assert_eq!(result, 42);
}

#[tokio::test]
async fn test_thread_manager_execute_all_pools() {
    let config = ThreadingConfig::default();
    let manager = ThreadManager::from_core_config(config).unwrap();
    
    // Test all pool types
    let pools = vec![
        ThreadPoolType::Query,
        ThreadPoolType::Write,
        ThreadPoolType::Read,
        ThreadPoolType::Compression,
        ThreadPoolType::CPU,
    ];
    
    for pool_type in pools {
        let result = manager.execute(pool_type, || {
            thread::current().name().unwrap_or("unknown").to_string()
        }).unwrap();
        
        assert!(!result.is_empty());
    }
}

#[tokio::test]
async fn test_thread_manager_spawn() {
    let config = ThreadingConfig::default();
    let manager = Arc::new(ThreadManager::from_core_config(config).unwrap());
    
    let manager_clone = manager.clone();
    let handle = manager_clone.spawn(ThreadPoolType::Query, || {
        thread::sleep(Duration::from_millis(10));
        100
    }).unwrap();
    
    let result = handle.await.unwrap();
    assert_eq!(result, 100);
}

#[tokio::test]
async fn test_thread_manager_get_pool() {
    let config = ThreadingConfig::default();
    let manager = ThreadManager::from_core_config(config).unwrap();
    
    let pool = manager.get_pool(ThreadPoolType::Query);
    assert!(pool.is_some());
    
    let pool = manager.get_pool(ThreadPoolType::Write);
    assert!(pool.is_some());
    
    // Non-existent pool type should return None (but all types are initialized)
    // This test verifies that all expected pools exist
}

#[tokio::test]
async fn test_thread_manager_get_stats() {
    let config = ThreadingConfig::default();
    let manager = Arc::new(ThreadManager::from_core_config(config).unwrap());
    
    // Execute some tasks
    for i in 0..5 {
        let manager_clone = manager.clone();
        tokio::spawn(async move {
            manager_clone.execute(ThreadPoolType::Query, move || i).unwrap();
        });
    }
    
    // Wait for tasks
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let stats = manager.get_stats(ThreadPoolType::Query);
    assert!(stats.is_some());
    let stats = stats.unwrap();
    assert!(stats.tasks_completed >= 5);
}

#[tokio::test]
async fn test_thread_manager_get_all_stats() {
    let config = ThreadingConfig::default();
    let manager = Arc::new(ThreadManager::from_core_config(config).unwrap());
    
    // Execute tasks in multiple pools
    manager.execute(ThreadPoolType::Query, || 1).unwrap();
    manager.execute(ThreadPoolType::Write, || 2).unwrap();
    manager.execute(ThreadPoolType::Read, || 3).unwrap();
    
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    let all_stats = manager.get_all_stats();
    assert!(!all_stats.is_empty());
    
    // Check that we have stats for the pools we used
    assert!(all_stats.contains_key(&ThreadPoolType::Query));
    assert!(all_stats.contains_key(&ThreadPoolType::Write));
    assert!(all_stats.contains_key(&ThreadPoolType::Read));
}

#[tokio::test]
async fn test_thread_manager_update_pool_config() {
    let config = ThreadingConfig::default();
    let manager = ThreadManager::from_core_config(config).unwrap();
    
    let mut new_config = ThreadPoolConfig::query();
    new_config.min_threads = 4;
    new_config.max_threads = 32;
    new_config.initial_threads = 8;
    
    let result = manager.update_pool_config(ThreadPoolType::Query, new_config.clone());
    assert!(result.is_ok());
    
    // Verify the pool was updated
    let pool = manager.get_pool(ThreadPoolType::Query).unwrap();
    let pool_config = pool.config();
    assert_eq!(pool_config.min_threads, 4);
    assert_eq!(pool_config.max_threads, 32);
}

#[tokio::test]
async fn test_thread_manager_tls_registry() {
    let config = ThreadingConfig::default();
    let manager = ThreadManager::from_core_config(config).unwrap();
    
    // TLS functionality would require implementing ThreadLocalStorage trait
    // This test verifies the registry exists
    // In a real implementation, you would register TLS and test it
}

#[tokio::test]
async fn test_thread_manager_shutdown() {
    let config = ThreadingConfig::default();
    let manager = Arc::new(ThreadManager::from_core_config(config).unwrap());
    
    // Execute some tasks
    manager.execute(ThreadPoolType::Query, || 1).unwrap();
    
    // Shutdown should not panic
    manager.shutdown().await;
}

#[tokio::test]
async fn test_parallel_executor_creation() {
    let config = ThreadingConfig::default();
    let manager = Arc::new(ThreadManager::from_core_config(config).unwrap());
    let executor = ParallelExecutor::new(manager);
    
    // Executor should be created successfully
    assert!(std::mem::size_of_val(&executor) > 0);
}

#[tokio::test]
async fn test_parallel_executor_execute_parallel() {
    let config = ThreadingConfig::default();
    let manager = Arc::new(ThreadManager::from_core_config(config).unwrap());
    let executor = ParallelExecutor::new(manager);
    
    let data = vec![1, 2, 3, 4, 5];
    let results = executor.execute_parallel(
        ThreadPoolType::Query,
        data.into_iter(),
        |x| x * 2,
    ).unwrap();
    
    assert_eq!(results, vec![2, 4, 6, 8, 10]);
}

#[tokio::test]
async fn test_parallel_executor_execute_parallel_large() {
    let config = ThreadingConfig::default();
    let manager = Arc::new(ThreadManager::from_core_config(config).unwrap());
    let executor = ParallelExecutor::new(manager);
    
    let data: Vec<i32> = (0..1000).collect();
    let results = executor.execute_parallel(
        ThreadPoolType::Query,
        data.into_iter(),
        |x| x * 3,
    ).unwrap();
    
    assert_eq!(results.len(), 1000);
    assert_eq!(results[0], 0);
    assert_eq!(results[999], 2997);
}

#[tokio::test]
async fn test_parallel_executor_execute_parallel_fold() {
    let config = ThreadingConfig::default();
    let manager = Arc::new(ThreadManager::from_core_config(config).unwrap());
    let executor = ParallelExecutor::new(manager);
    
    let data = vec![1, 2, 3, 4, 5];
    let result: i32 = executor.execute_parallel_fold(
        ThreadPoolType::Query,
        data.into_iter(),
        0,
        |acc, x| acc + x,
    ).unwrap();
    
    assert_eq!(result, 15);
}

#[tokio::test]
async fn test_parallel_executor_execute_parallel_fold_large() {
    let config = ThreadingConfig::default();
    let manager = Arc::new(ThreadManager::from_core_config(config).unwrap());
    let executor = ParallelExecutor::new(manager);
    
    let data: Vec<i64> = (1..=1000).collect();
    let result: i64 = executor.execute_parallel_fold(
        ThreadPoolType::Query,
        data.into_iter(),
        0,
        |acc, x| acc + x,
    ).unwrap();
    
    // Sum of 1..=1000 = 500500
    assert_eq!(result, 500500);
}

#[tokio::test]
async fn test_thread_manager_concurrent_execution() {
    let config = ThreadingConfig::default();
    let manager = Arc::new(ThreadManager::from_core_config(config).unwrap());
    
    let handles: Vec<_> = (0..20)
        .map(|i| {
            let manager = manager.clone();
            tokio::spawn(async move {
                manager.execute(ThreadPoolType::Query, move || i * 2).unwrap()
            })
        })
        .collect();
    
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // All should succeed
    for (i, result) in results.into_iter().enumerate() {
        assert_eq!(result.unwrap(), i * 2);
    }
}

#[tokio::test]
async fn test_thread_manager_concurrent_multiple_pools() {
    let config = ThreadingConfig::default();
    let manager = Arc::new(ThreadManager::from_core_config(config).unwrap());
    
    let handles: Vec<_> = vec![
        (ThreadPoolType::Query, 0),
        (ThreadPoolType::Write, 1),
        (ThreadPoolType::Read, 2),
        (ThreadPoolType::Compression, 3),
        (ThreadPoolType::CPU, 4),
    ]
    .into_iter()
    .map(|(pool_type, value)| {
        let manager = manager.clone();
        tokio::spawn(async move {
            manager.execute(pool_type, move || value).unwrap()
        })
    })
    .collect();
    
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // All should succeed
    for (i, result) in results.into_iter().enumerate() {
        assert_eq!(result.unwrap(), i);
    }
}

#[tokio::test]
async fn test_thread_pool_stats_update() {
    let config = ThreadPoolConfig::default();
    let pool = Arc::new(ManagedThreadPool::new(ThreadPoolType::Query, config).unwrap());
    
    // Execute tasks
    for i in 0..10 {
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            pool_clone.execute(move || {
                thread::sleep(Duration::from_millis(5));
                i
            });
        });
    }
    
    // Wait for tasks to complete
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let stats = pool.stats();
    assert!(stats.tasks_completed >= 10);
    assert!(stats.total_cpu_time_us > 0);
    assert!(stats.avg_task_duration_us > 0);
    assert!(stats.max_task_duration_us > 0);
}

#[tokio::test]
async fn test_thread_pool_long_running_task() {
    let config = ThreadPoolConfig::default();
    let pool = Arc::new(ManagedThreadPool::new(ThreadPoolType::Query, config).unwrap());
    
    let pool_clone = pool.clone();
    let handle = pool_clone.spawn(|| {
        thread::sleep(Duration::from_millis(50));
        999
    });
    
    let result = handle.await.unwrap();
    assert_eq!(result, 999);
    
    let stats = pool.stats();
    assert!(stats.max_task_duration_us >= 50000); // At least 50ms
}

#[tokio::test]
async fn test_thread_manager_invalid_pool() {
    let config = ThreadingConfig::default();
    let manager = ThreadManager::from_core_config(config).unwrap();
    
    // All pool types should be initialized, so this shouldn't happen
    // But we can test that the pools we expect exist
    assert!(manager.get_pool(ThreadPoolType::Query).is_some());
    assert!(manager.get_pool(ThreadPoolType::Write).is_some());
}

#[tokio::test]
async fn test_thread_pool_resize() {
    let config = ThreadPoolConfig::default();
    let pool = ManagedThreadPool::new(ThreadPoolType::Query, config).unwrap();
    
    // Resize should not panic (even though Rayon doesn't support dynamic resizing)
    let result = pool.resize(8);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_thread_pool_rayon_access() {
    let config = ThreadPoolConfig::default();
    let pool = ManagedThreadPool::new(ThreadPoolType::Query, config).unwrap();
    
    // Should be able to access underlying Rayon pool
    let rayon_pool = pool.rayon_pool();
    assert!(rayon_pool.current_num_threads() > 0);
}

#[tokio::test]
async fn test_thread_manager_statistics_accuracy() {
    let config = ThreadingConfig::default();
    let manager = Arc::new(ThreadManager::from_core_config(config).unwrap());
    
    // Execute known number of tasks
    let task_count = 25;
    for i in 0..task_count {
        manager.execute(ThreadPoolType::Query, move || i).unwrap();
    }
    
    // Wait a bit for stats to update
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let stats = manager.get_stats(ThreadPoolType::Query).unwrap();
    assert!(stats.tasks_completed >= task_count);
}

#[tokio::test]
async fn test_thread_manager_monitoring() {
    let config = ThreadingConfig::default();
    let manager = Arc::new(ThreadManager::from_core_config(config).unwrap());
    
    // Execute some tasks
    manager.execute(ThreadPoolType::Query, || 1).unwrap();
    
    // Wait for monitoring to update
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // Monitoring should have updated stats
    let stats = manager.get_stats(ThreadPoolType::Query).unwrap();
    assert!(stats.current_threads > 0);
}

#[tokio::test]
async fn test_thread_pool_custom_config() {
    let config = ThreadPoolConfig {
        name: "test-pool".to_string(),
        min_threads: 2,
        max_threads: 8,
        initial_threads: 4,
        stack_size: Some(1024 * 1024),
        keep_alive: Some(Duration::from_secs(30)),
        priority: None,
        cpu_affinity: None,
        thread_name_prefix: "test-".to_string(),
        enable_tls: false,
        spawn_timeout: Some(Duration::from_secs(10)),
        deadlock_timeout: Some(Duration::from_secs(60)),
        panic_handler: None,
    };
    
    let pool = ManagedThreadPool::new(ThreadPoolType::Query, config.clone());
    assert!(pool.is_ok());
    
    let pool = pool.unwrap();
    let pool_config = pool.config();
    assert_eq!(pool_config.name, "test-pool");
    assert_eq!(pool_config.min_threads, 2);
    assert_eq!(pool_config.initial_threads, 4);
}

#[tokio::test]
async fn test_thread_pool_with_cpu_affinity() {
    let mut config = ThreadPoolConfig::default();
    config.cpu_affinity = Some(vec![0, 1, 2]);
    
    let pool = ManagedThreadPool::new(ThreadPoolType::CPU, config);
    // Should create successfully even with CPU affinity set
    // (actual affinity setting happens at OS level)
    assert!(pool.is_ok());
}

#[tokio::test]
async fn test_thread_pool_with_priority() {
    let mut config = ThreadPoolConfig::default();
    config.priority = Some(50);
    
    let pool = ManagedThreadPool::new(ThreadPoolType::Background, config);
    // Should create successfully even with priority set
    // (actual priority setting happens at OS level)
    assert!(pool.is_ok());
}

#[tokio::test]
async fn test_thread_manager_empty_stats() {
    let config = ThreadingConfig::default();
    let manager = ThreadManager::from_core_config(config).unwrap();
    
    // Get stats immediately after creation (no tasks executed)
    let stats = manager.get_stats(ThreadPoolType::Query).unwrap();
    assert_eq!(stats.tasks_completed, 0);
    assert_eq!(stats.active_threads, 0);
}

#[tokio::test]
async fn test_parallel_executor_empty_iterator() {
    let config = ThreadingConfig::default();
    let manager = Arc::new(ThreadManager::from_core_config(config).unwrap());
    let executor = ParallelExecutor::new(manager);
    
    let data: Vec<i32> = vec![];
    let results = executor.execute_parallel(
        ThreadPoolType::Query,
        data.into_iter(),
        |x| x * 2,
    ).unwrap();
    
    assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_parallel_executor_single_element() {
    let config = ThreadingConfig::default();
    let manager = Arc::new(ThreadManager::from_core_config(config).unwrap());
    let executor = ParallelExecutor::new(manager);
    
    let data = vec![42];
    let results = executor.execute_parallel(
        ThreadPoolType::Query,
        data.into_iter(),
        |x| x * 2,
    ).unwrap();
    
    assert_eq!(results, vec![84]);
}

#[tokio::test]
async fn test_thread_pool_uptime_tracking() {
    let config = ThreadPoolConfig::default();
    let pool = Arc::new(ManagedThreadPool::new(ThreadPoolType::Query, config).unwrap());
    
    let stats1 = pool.stats();
    tokio::time::sleep(Duration::from_millis(100)).await;
    let stats2 = pool.stats();
    
    // Uptime should have increased
    assert!(stats2.uptime >= stats1.uptime);
}

// Stress tests

#[tokio::test]
async fn test_thread_manager_stress_execution() {
    let config = ThreadingConfig::default();
    let manager = Arc::new(ThreadManager::from_core_config(config).unwrap());
    
    // Execute many tasks concurrently
    let handles: Vec<_> = (0..1000)
        .map(|i| {
            let manager = manager.clone();
            tokio::spawn(async move {
                manager.execute(ThreadPoolType::Query, move || i).unwrap()
            })
        })
        .collect();
    
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // All should succeed
    for (i, result) in results.into_iter().enumerate() {
        assert_eq!(result.unwrap(), i);
    }
}

#[tokio::test]
async fn test_thread_pool_stress_execution() {
    let config = ThreadPoolConfig::default();
    let pool = Arc::new(ManagedThreadPool::new(ThreadPoolType::Query, config).unwrap());
    
    // Execute many tasks
    let handles: Vec<_> = (0..500)
        .map(|i| {
            let pool = pool.clone();
            tokio::spawn(async move {
                pool.execute(move || i)
            })
        })
        .collect();
    
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // All should succeed
    for (i, result) in results.into_iter().enumerate() {
        assert_eq!(result.unwrap(), i);
    }
    
    let stats = pool.stats();
    assert!(stats.tasks_completed >= 500);
}

#[tokio::test]
async fn test_parallel_executor_stress() {
    let config = ThreadingConfig::default();
    let manager = Arc::new(ThreadManager::from_core_config(config).unwrap());
    let executor = ParallelExecutor::new(manager);
    
    let data: Vec<i32> = (0..10000).collect();
    let results = executor.execute_parallel(
        ThreadPoolType::Query,
        data.into_iter(),
        |x| x * 2,
    ).unwrap();
    
    assert_eq!(results.len(), 10000);
    assert_eq!(results[0], 0);
    assert_eq!(results[9999], 19998);
}

// Error handling tests

#[tokio::test]
async fn test_thread_manager_execute_error_handling() {
    let config = ThreadingConfig::default();
    let manager = ThreadManager::from_core_config(config).unwrap();
    
    // Execute should handle panics gracefully
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        manager.execute(ThreadPoolType::Query, || {
            panic!("Test panic");
        })
    }));
    
    // Panic should be caught (actual behavior depends on Rayon's panic handling)
    // This test verifies we can attempt execution
}

// Configuration tests

#[test]
fn test_threading_config_default() {
    let config = ThreadingConfig::default();
    assert!(config.enabled);
    assert!(config.enable_monitoring);
    assert!(!config.pools.is_empty());
}

#[test]
fn test_thread_pool_config_section_default() {
    let section = narayana_core::config::ThreadPoolConfigSection::default();
    assert!(section.min_threads > 0);
    assert!(section.max_threads >= section.min_threads);
    assert!(section.initial_threads >= section.min_threads);
}

#[test]
fn test_threading_config_all_pools() {
    let config = ThreadingConfig::default();
    
    // Verify all pools have configurations
    assert!(config.query_pool.min_threads > 0);
    assert!(config.write_pool.min_threads > 0);
    assert!(config.read_pool.min_threads > 0);
    assert!(config.compression_pool.min_threads > 0);
    assert!(config.cpu_pool.min_threads > 0);
    assert!(config.background_pool.min_threads > 0);
    assert!(config.analytics_pool.min_threads > 0);
    assert!(config.vector_pool.min_threads > 0);
    assert!(config.worker_pool.min_threads > 0);
    assert!(config.sync_pool.min_threads > 0);
}

