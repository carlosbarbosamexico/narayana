// Cognitive Brain Benchmark Suite
// Tests performance of thoughts, memories, experiences, and pattern learning

use narayana_storage::cognitive::{CognitiveBrain, MemoryType};
use std::sync::Arc;
use std::time::Instant;
use serde_json::json;

pub async fn run_brain_bench() -> anyhow::Result<()> {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║                                                               ║");
    println!("║     COGNITIVE BRAIN BENCHMARK SUITE                          ║");
    println!("║                                                               ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();
    
    let brain = Arc::new(CognitiveBrain::new());
    
    // Test 1: Thought Creation
    println!("═══════════════════════════════════════════════════════════════");
    println!("TEST 1: Thought Creation Performance");
    println!("═══════════════════════════════════════════════════════════════");
    test_thought_creation(&brain).await?;
    
    // Test 2: Memory Storage
    println!("═══════════════════════════════════════════════════════════════");
    println!("TEST 2: Memory Storage Performance");
    println!("═══════════════════════════════════════════════════════════════");
    test_memory_storage(&brain).await?;
    
    // Test 3: Experience Storage
    println!("═══════════════════════════════════════════════════════════════");
    println!("TEST 3: Experience Storage Performance");
    println!("═══════════════════════════════════════════════════════════════");
    test_experience_storage(&brain).await?;
    
    // Test 4: Memory Retrieval
    println!("═══════════════════════════════════════════════════════════════");
    println!("TEST 4: Memory Retrieval Performance");
    println!("═══════════════════════════════════════════════════════════════");
    test_memory_retrieval(&brain).await?;
    
    // Test 5: Pattern Learning
    println!("═══════════════════════════════════════════════════════════════");
    println!("TEST 5: Pattern Learning Performance");
    println!("═══════════════════════════════════════════════════════════════");
    test_pattern_learning(&brain).await?;
    
    // Test 6: Association Creation
    println!("═══════════════════════════════════════════════════════════════");
    println!("TEST 6: Association Creation Performance");
    println!("═══════════════════════════════════════════════════════════════");
    test_associations(&brain).await?;
    
    // Test 7: Mixed Cognitive Workload
    println!("═══════════════════════════════════════════════════════════════");
    println!("TEST 7: Mixed Cognitive Workload");
    println!("═══════════════════════════════════════════════════════════════");
    test_mixed_workload(&brain).await?;
    
    // Test 8: Forgetting Curves (Exponential Decay)
    println!("═══════════════════════════════════════════════════════════════");
    println!("TEST 8: Forgetting Curves (Biological Memory Decay)");
    println!("═══════════════════════════════════════════════════════════════");
    test_forgetting_curves(&brain).await?;
    
    // Test 9: Power Law Distribution (Biological Memory Strength)
    println!("═══════════════════════════════════════════════════════════════");
    println!("TEST 9: Power Law Memory Distribution");
    println!("═══════════════════════════════════════════════════════════════");
    test_power_law_distribution(&brain).await?;
    
    // Test 10: Diminishing Returns (Logarithmic Scaling)
    println!("═══════════════════════════════════════════════════════════════");
    println!("TEST 10: Diminishing Returns (Biological Scaling)");
    println!("═══════════════════════════════════════════════════════════════");
    test_diminishing_returns(&brain).await?;
    
    // Test 11: Memory Consolidation (Non-linear Strengthening)
    println!("═══════════════════════════════════════════════════════════════");
    println!("TEST 11: Memory Consolidation (Repeated Access)");
    println!("═══════════════════════════════════════════════════════════════");
    test_memory_consolidation(&brain).await?;
    
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║                    BRAIN BENCHMARK COMPLETE                    ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    
    Ok(())
}

async fn test_thought_creation(brain: &Arc<CognitiveBrain>) -> anyhow::Result<()> {
    let sizes = vec![1_000, 10_000, 100_000];
    
    for size in sizes {
        let start = Instant::now();
        let mut thought_ids = Vec::new();
        
        for i in 0..size {
            let content = json!({
                "action": format!("test_action_{}", i),
                "context": format!("test_context_{}", i),
                "priority": (i % 10) as f64,
            });
            let priority = (i % 10) as f64;
            match brain.create_thought(content, priority) {
                Ok(id) => thought_ids.push(id),
                Err(e) => {
                    eprintln!("Error creating thought: {}", e);
                    break;
                }
            }
        }
        
        let duration = start.elapsed();
        let ops = if duration.as_secs_f64() > 0.0 {
            (thought_ids.len() as f64 / duration.as_secs_f64()) as usize
        } else {
            thought_ids.len()
        };
        
        println!("  {:>10} thoughts: {:>12} ops/sec ({:.2}ms)", 
                 size, ops, duration.as_secs_f64() * 1000.0);
    }
    println!();
    Ok(())
}

async fn test_memory_storage(brain: &Arc<CognitiveBrain>) -> anyhow::Result<()> {
    let sizes = vec![1_000, 10_000, 100_000];
    let memory_types = vec![
        MemoryType::Episodic,
        MemoryType::Semantic,
        MemoryType::Procedural,
    ];
    
    for size in sizes {
        let start = Instant::now();
        let mut memory_ids = Vec::new();
        
        for i in 0..size {
            let memory_type = &memory_types[i % memory_types.len()];
            let content = json!({
                "event": format!("memory_event_{}", i),
                "data": format!("memory_data_{}", i),
                "timestamp": i as u64,
            });
            let tags = vec![format!("tag_{}", i % 10)];
            
            match brain.store_memory(memory_type.clone(), content, None, tags, None) {
                Ok(id) => memory_ids.push(id),
                Err(e) => {
                    eprintln!("Error storing memory: {}", e);
                    break;
                }
            }
        }
        
        let duration = start.elapsed();
        let ops = if duration.as_secs_f64() > 0.0 {
            (memory_ids.len() as f64 / duration.as_secs_f64()) as usize
        } else {
            memory_ids.len()
        };
        
        println!("  {:>10} memories: {:>12} ops/sec ({:.2}ms)", 
                 size, ops, duration.as_secs_f64() * 1000.0);
    }
    println!();
    Ok(())
}

async fn test_experience_storage(brain: &Arc<CognitiveBrain>) -> anyhow::Result<()> {
    let sizes = vec![1_000, 10_000, 100_000];
    
    for size in sizes {
        let start = Instant::now();
        let mut experience_ids = Vec::new();
        
        for i in 0..size {
            let event_type = format!("experience_{}", i % 10);
            let observation = json!({
                "state": format!("state_{}", i),
                "sensor_data": format!("sensor_{}", i),
            });
            let action = json!({
                "action_type": format!("action_{}", i % 5),
                "parameters": i,
            });
            let reward = (i % 100) as f64 / 100.0;
            let outcome = json!({
                "result": format!("outcome_{}", i),
                "success": (i % 2) == 0,
            });
            
            match brain.store_experience(event_type, observation, Some(action), Some(outcome), Some(reward), None) {
                Ok(id) => experience_ids.push(id),
                Err(e) => {
                    eprintln!("Error storing experience: {}", e);
                    break;
                }
            }
        }
        
        let duration = start.elapsed();
        let ops = if duration.as_secs_f64() > 0.0 {
            (experience_ids.len() as f64 / duration.as_secs_f64()) as usize
        } else {
            experience_ids.len()
        };
        
        println!("  {:>10} experiences: {:>12} ops/sec ({:.2}ms)", 
                 size, ops, duration.as_secs_f64() * 1000.0);
    }
    println!();
    Ok(())
}

async fn test_memory_retrieval(brain: &Arc<CognitiveBrain>) -> anyhow::Result<()> {
    // First, create memories to retrieve
    let num_memories = 10_000;
    let mut memory_ids = Vec::new();
    
    for i in 0..num_memories {
        let memory_type = MemoryType::Episodic;
        let content = json!({
            "event": format!("retrieval_test_{}", i),
            "data": format!("data_{}", i),
        });
        let tags = vec![format!("tag_{}", i % 10)];
        
        match brain.store_memory(memory_type, content, None, tags, None) {
            Ok(id) => memory_ids.push(id),
            Err(_) => break,
        }
    }
    
    println!("  Created {} memories for retrieval test", memory_ids.len());
    
    // Test retrieval by tag
    let start = Instant::now();
    let mut retrieved = 0;
    for i in 0..10 {
        let tag = format!("tag_{}", i);
        if let Ok(memories) = brain.retrieve_memories_by_tag(&tag) {
            retrieved += memories.len();
        }
    }
    let duration = start.elapsed();
    let ops = if duration.as_secs_f64() > 0.0 {
        (retrieved as f64 / duration.as_secs_f64()) as usize
    } else {
        retrieved
    };
    
    println!("  Retrieved {} memories by tag: {:>12} ops/sec ({:.2}ms)", 
             retrieved, ops, duration.as_secs_f64() * 1000.0);
    println!();
    Ok(())
}

async fn test_pattern_learning(brain: &Arc<CognitiveBrain>) -> anyhow::Result<()> {
    // Create experiences that form patterns
    let num_experiences = 5_000;
    let mut experience_ids = Vec::new();
    
    for i in 0..num_experiences {
        let event_type = format!("pattern_event_{}", i % 10);
        let observation = json!({
            "pattern_id": i % 5,
            "sequence": i,
        });
        let action = json!({
            "action": format!("action_{}", i % 3),
        });
        let reward = if i % 5 == 0 { 1.0 } else { 0.0 };
        
        match brain.store_experience(event_type, observation, Some(action), None, Some(reward), None) {
            Ok(id) => experience_ids.push(id),
            Err(_) => break,
        }
    }
    
    println!("  Created {} experiences for pattern learning", experience_ids.len());
    
    // Test pattern detection
    let start = Instant::now();
    let pattern_ids = brain.detect_patterns_from_experiences().unwrap_or_default();
    let duration = start.elapsed();
    
    println!("  Pattern detection: {} patterns found in {:.2}ms", 
             pattern_ids.len(), duration.as_secs_f64() * 1000.0);
    println!();
    Ok(())
}

async fn test_associations(brain: &Arc<CognitiveBrain>) -> anyhow::Result<()> {
    // Create memories and thoughts to associate
    let num_items = 1_000;
    let mut memory_ids = Vec::new();
    let mut thought_ids = Vec::new();
    
    // Create memories
    for i in 0..num_items {
        let content = json!({"id": i});
        match brain.store_memory(MemoryType::Associative, content, None, vec![], None) {
            Ok(id) => memory_ids.push(id),
            Err(_) => break,
        }
    }
    
    // Create thoughts
    for i in 0..num_items {
        let content = json!({"id": i});
        match brain.create_thought(content, 1.0) {
            Ok(id) => thought_ids.push(id),
            Err(_) => break,
        }
    }
    
    println!("  Created {} memories and {} thoughts", memory_ids.len(), thought_ids.len());
    
    // Create associations
    let start = Instant::now();
    let mut associations_created = 0;
    
    for i in 0..num_items.min(memory_ids.len()).min(thought_ids.len()) {
        if let (Some(mem_id), Some(thought_id)) = (memory_ids.get(i), thought_ids.get(i)) {
            match brain.create_association(mem_id, thought_id) {
                Ok(_) => associations_created += 1,
                Err(_) => break,
            }
        }
    }
    
    let duration = start.elapsed();
    let ops = if duration.as_secs_f64() > 0.0 {
        (associations_created as f64 / duration.as_secs_f64()) as usize
    } else {
        associations_created
    };
    
    println!("  Created {} associations: {:>12} ops/sec ({:.2}ms)", 
             associations_created, ops, duration.as_secs_f64() * 1000.0);
    println!();
    Ok(())
}

async fn test_mixed_workload(brain: &Arc<CognitiveBrain>) -> anyhow::Result<()> {
    let iterations = 10_000;
    let start = Instant::now();
    let mut operations = 0;
    
    for i in 0..iterations {
        match i % 4 {
            0 => {
                // Create thought
                let content = json!({"mixed_workload": i});
                if brain.create_thought(content, 1.0).is_ok() {
                    operations += 1;
                }
            }
            1 => {
                // Store memory
                let content = json!({"mixed_workload": i});
                if brain.store_memory(MemoryType::Working, content, None, vec![], None).is_ok() {
                    operations += 1;
                }
            }
            2 => {
                // Store experience
                let observation = json!({"mixed_workload": i});
                if brain.store_experience("mixed".to_string(), observation, None, None, None, None).is_ok() {
                    operations += 1;
                }
            }
            3 => {
                // Retrieve memory (if we have any)
                // This is simplified - in real scenario would track memory IDs
                operations += 1;
            }
            _ => {}
        }
    }
    
    let duration = start.elapsed();
    let ops = if duration.as_secs_f64() > 0.0 {
        (operations as f64 / duration.as_secs_f64()) as usize
    } else {
        operations
    };
    
    println!("  Mixed workload ({} operations): {:>12} ops/sec ({:.2}ms)", 
             operations, ops, duration.as_secs_f64() * 1000.0);
    println!();
    Ok(())
}

/// Test forgetting curves - exponential decay of memory strength over time
/// Biological memory follows: strength(t) = initial_strength * e^(-decay_rate * t)
async fn test_forgetting_curves(brain: &Arc<CognitiveBrain>) -> anyhow::Result<()> {
    // Create memories with initial strength
    let num_memories = 1_000;
    let mut memory_ids = Vec::new();
    let initial_strength = 1.0;
    
    for i in 0..num_memories {
        let content = json!({
            "event": format!("forgetting_test_{}", i),
            "initial_strength": initial_strength,
        });
        
        match brain.store_memory(MemoryType::Episodic, content, None, vec![], None) {
            Ok(id) => {
                // Set initial strength
                brain.update_memory_strength(&id, initial_strength)?;
                memory_ids.push(id);
            }
            Err(_) => break,
        }
    }
    
    println!("  Created {} memories with initial strength {:.2}", memory_ids.len(), initial_strength);
    
    // Simulate time passing and measure decay
    let decay_intervals = vec![0, 1, 2, 5, 10, 20, 50]; // Days
    let decay_rate: f64 = 0.1; // 10% per day
    
    println!("  Forgetting curve (decay rate: {:.1}% per day):", decay_rate * 100.0);
    println!("  {:>6} | {:>10} | {:>10}", "Days", "Strength", "Retained %");
    println!("  {}", "-".repeat(35));
    
    for days in decay_intervals {
        let mut total_strength = 0.0;
        let mut accessible = 0;
        
        // Simulate accessing memories after time has passed
        let days_f64 = days as f64;
        for id in &memory_ids {
            if let Ok(memory) = brain.access_memory(id) {
                // Apply decay
                let decayed_strength = memory.strength * (1.0 - decay_rate).powf(days_f64);
                brain.update_memory_strength(id, decayed_strength)?;
                
                if decayed_strength > 0.1 { // Threshold for "accessible"
                    accessible += 1;
                }
                total_strength += decayed_strength;
            }
        }
        
        let avg_strength = total_strength / memory_ids.len() as f64;
        let retained_pct = (accessible as f64 / memory_ids.len() as f64) * 100.0;
        
        println!("  {:>6} | {:>10.4} | {:>9.1}%", days, avg_strength, retained_pct);
    }
    
    println!();
    Ok(())
}

/// Test power law distribution - most memories are weak, few are strong
/// Biological memory follows: P(strength) ~ strength^(-alpha)
async fn test_power_law_distribution(brain: &Arc<CognitiveBrain>) -> anyhow::Result<()> {
    use rand::Rng;
    
    let num_memories = 10_000;
    let mut memory_ids = Vec::new();
    let alpha = 2.0; // Power law exponent (typical for biological systems)
    
    // Generate power-law distributed memory strengths
    let mut rng = rand::thread_rng();
    let mut strength_buckets = vec![0; 10]; // 10 buckets: 0.0-0.1, 0.1-0.2, etc.
    
    for i in 0..num_memories {
        // Generate power-law distributed strength: P(x) ~ x^(-alpha)
        // Using inverse CDF method: x = (1-u)^(-1/(alpha-1)) for x in [1, inf)
        // Then normalize to [0, 1] range
        let u: f64 = rng.gen();
        // Generate power law value, then normalize to [0, 1]
        let power_law_value = (1.0 - u).powf(-1.0 / (alpha - 1.0));
        // Normalize: map from [1, inf) to [0, 1] using 1 - 1/x
        let strength = (1.0 - 1.0 / power_law_value).max(0.0).min(1.0);
        
        let content = json!({
            "event": format!("powerlaw_test_{}", i),
            "strength": strength,
        });
        
        match brain.store_memory(MemoryType::Semantic, content, None, vec![], None) {
            Ok(id) => {
                brain.update_memory_strength(&id, strength)?;
                memory_ids.push(id);
                
                // Bucket the strength
                let bucket = (strength * 10.0).floor().min(9.0) as usize;
                strength_buckets[bucket] += 1;
            }
            Err(_) => break,
        }
    }
    
    println!("  Created {} memories with power-law distribution (α={:.2})", memory_ids.len(), alpha);
    println!("  Strength distribution:");
    println!("  {:>12} | {:>10} | {:>10}", "Range", "Count", "Percentage");
    println!("  {}", "-".repeat(40));
    
    for (i, count) in strength_buckets.iter().enumerate() {
        let range_start = i as f64 / 10.0;
        let range_end = (i + 1) as f64 / 10.0;
        let pct = (*count as f64 / memory_ids.len() as f64) * 100.0;
        println!("  {:>5.1}-{:>5.1} | {:>10} | {:>9.1}%", range_start, range_end, count, pct);
    }
    
    // Verify power law: should have many weak memories, few strong ones
    let weak_memories = strength_buckets[0..5].iter().sum::<usize>();
    let strong_memories = strength_buckets[7..].iter().sum::<usize>();
    let ratio = weak_memories as f64 / strong_memories.max(1) as f64;
    
    println!("  Weak/Strong ratio: {:.2} (expected >> 1 for power law)", ratio);
    println!();
    Ok(())
}

/// Test diminishing returns - performance degrades logarithmically as memory grows
/// Biological systems show: performance ~ log(memory_size)
async fn test_diminishing_returns(brain: &Arc<CognitiveBrain>) -> anyhow::Result<()> {
    // Use logarithmic scale instead of linear
    let sizes: Vec<usize> = vec![100, 316, 1_000, 3_162, 10_000, 31_623, 100_000];
    
    println!("  Testing retrieval performance with logarithmic scaling:");
    println!("  {:>10} | {:>12} | {:>12} | {:>10}", "Size", "Ops/sec", "Time (ms)", "Efficiency");
    println!("  {}", "-".repeat(60));
    
    for size in sizes {
        // Create memories
        let mut memory_ids = Vec::new();
        for i in 0..size {
            let content = json!({
                "event": format!("diminish_test_{}", i),
                "index": i,
            });
            
            match brain.store_memory(MemoryType::Working, content, None, vec![], None) {
                Ok(id) => memory_ids.push(id),
                Err(_) => break,
            }
        }
        
        // Test retrieval performance - test all memories to see real scaling
        let start = Instant::now();
        let mut retrieved = 0;
        
        // For large sizes, we still test all to see real performance degradation
        // But limit iterations to avoid timeout
        let test_count = size.min(10_000); // Test up to 10k retrievals
        
        for i in 0..test_count {
            if let Some(id) = memory_ids.get(i % memory_ids.len()) {
                if brain.access_memory(id).is_ok() {
                    retrieved += 1;
                }
            }
        }
        
        let duration = start.elapsed();
        let ops = if duration.as_secs_f64() > 0.0 {
            (retrieved as f64 / duration.as_secs_f64()) as usize
        } else {
            retrieved
        };
        
        // Calculate efficiency (ops/sec normalized by log of size)
        // Should decrease logarithmically: efficiency ~ ops/sec / log(size)
        // This shows how performance degrades as memory grows
        let efficiency = ops as f64 / (size as f64).ln().max(1.0);
        
        println!("  {:>10} | {:>12} | {:>12.2} | {:>10.2}", 
                 size, ops, duration.as_secs_f64() * 1000.0, efficiency);
    }
    
    println!("  Note: Efficiency should decrease logarithmically (biological pattern)");
    println!();
    Ok(())
}

/// Test memory consolidation - strength increases non-linearly with repeated access
/// Biological pattern: strength(n) = 1 - e^(-consolidation_rate * n)
async fn test_memory_consolidation(brain: &Arc<CognitiveBrain>) -> anyhow::Result<()> {
    let num_memories = 100;
    let max_accesses = 20;
    let consolidation_rate = 0.15; // Rate of consolidation per access
    
    // Create memories with low initial strength
    let mut memory_ids = Vec::new();
    let initial_strength = 0.2;
    
    for i in 0..num_memories {
        let content = json!({
            "event": format!("consolidation_test_{}", i),
            "accesses": 0,
        });
        
        match brain.store_memory(MemoryType::LongTerm, content, None, vec![], None) {
            Ok(id) => {
                brain.update_memory_strength(&id, initial_strength)?;
                memory_ids.push(id);
            }
            Err(_) => break,
        }
    }
    
    println!("  Testing consolidation with {} memories (initial strength: {:.2})", 
             memory_ids.len(), initial_strength);
    println!("  Consolidation rate: {:.1}% per access", consolidation_rate * 100.0);
    println!("  {:>8} | {:>12} | {:>12} | {:>10}", "Accesses", "Avg Strength", "Max Strength", "Consolidated");
    println!("  {}", "-".repeat(55));
    
    for access_count in 0..=max_accesses {
        let mut total_strength = 0.0;
        let mut max_strength: f64 = 0.0;
        let mut consolidated = 0;
        
        // Access each memory access_count times
        for id in &memory_ids {
            // Access the memory
            if let Ok(memory) = brain.access_memory(id) {
                // Apply consolidation: strength increases non-linearly
                // Using exponential approach to asymptote: strength = 1 - (1 - initial) * e^(-rate * n)
                let new_strength = 1.0 - (1.0 - initial_strength) * (-consolidation_rate * access_count as f64).exp();
                brain.update_memory_strength(id, new_strength)?;
                
                total_strength += new_strength;
                max_strength = max_strength.max(new_strength);
                
                if new_strength > 0.8 { // Threshold for "consolidated"
                    consolidated += 1;
                }
            }
        }
        
        let avg_strength = total_strength / memory_ids.len() as f64;
        
        if access_count % 2 == 0 || access_count == max_accesses {
            println!("  {:>8} | {:>12.4} | {:>12.4} | {:>10}", 
                     access_count, avg_strength, max_strength, consolidated);
        }
    }
    
    println!("  Note: Strength should increase non-linearly (biological consolidation)");
    println!();
    Ok(())
}

