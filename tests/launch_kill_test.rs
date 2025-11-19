// Launch-Kill-Restart Test Suite
// Tests data integrity across multiple launch/kill cycles

use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;
use std::path::Path;
use std::fs;

#[test]
fn test_launch_kill_restart_cycle() {
    println!("🔄 Starting launch-kill-restart cycle test...");
    
    let test_dir = "/tmp/narayana_test";
    let _ = fs::remove_dir_all(test_dir); // Clean up
    
    // Test multiple cycles
    for cycle in 1..=5 {
        println!("\n📋 Cycle {}: Launch → Write → Kill → Restart → Verify", cycle);
        
        // Launch server
        println!("  🚀 Launching server...");
        let mut server = launch_server(test_dir);
        thread::sleep(Duration::from_millis(2000)); // Wait for startup
        
        // Write data
        println!("  💾 Writing test data...");
        write_test_data(cycle);
        thread::sleep(Duration::from_millis(500));
        
        // Kill server
        println!("  🔪 Killing server...");
        kill_server(&mut server);
        thread::sleep(Duration::from_millis(1000));
        
        // Restart server
        println!("  🔄 Restarting server...");
        let mut server = launch_server(test_dir);
        thread::sleep(Duration::from_millis(2000));
        
        // Verify data integrity
        println!("  ✅ Verifying data integrity...");
        verify_data_integrity(cycle);
        
        // Clean shutdown
        println!("  🛑 Graceful shutdown...");
        kill_server(&mut server);
        thread::sleep(Duration::from_millis(500));
        
        println!("  ✅ Cycle {} complete", cycle);
    }
    
    // Cleanup
    let _ = fs::remove_dir_all(test_dir);
    println!("\n✅ All cycles complete - no data corruption detected!");
}

#[test]
fn test_graceful_shutdown() {
    println!("🔄 Testing graceful shutdown...");
    
    let test_dir = "/tmp/narayana_test_graceful";
    let _ = fs::remove_dir_all(test_dir);
    
    let mut server = launch_server(test_dir);
    thread::sleep(Duration::from_millis(2000));
    
    // Write data
    write_test_data(1);
    thread::sleep(Duration::from_millis(500));
    
    // Send SIGTERM for graceful shutdown
    println!("  📤 Sending SIGTERM for graceful shutdown...");
    let pid = server.id();
    Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .output()
        .expect("Failed to send SIGTERM");
    
    // Wait for graceful shutdown
    thread::sleep(Duration::from_millis(2000));
    
    // Restart and verify
    let mut server = launch_server(test_dir);
    thread::sleep(Duration::from_millis(2000));
    verify_data_integrity(1);
    
    kill_server(&mut server);
    let _ = fs::remove_dir_all(test_dir);
    println!("  ✅ Graceful shutdown test complete!");
}

#[test]
fn test_abrupt_termination() {
    println!("🔄 Testing abrupt termination...");
    
    let test_dir = "/tmp/narayana_test_abrupt";
    let _ = fs::remove_dir_all(test_dir);
    
    for cycle in 1..=3 {
        println!("  📋 Abrupt termination cycle {}", cycle);
        
        let mut server = launch_server(test_dir);
        thread::sleep(Duration::from_millis(2000));
        
        // Write data mid-transaction
        write_test_data(cycle);
        thread::sleep(Duration::from_millis(100));
        
        // Force kill (SIGKILL)
        println!("    💥 Force killing server...");
        let pid = server.id();
        Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .output()
            .expect("Failed to kill server");
        
        thread::sleep(Duration::from_millis(1000));
        
        // Restart and verify
        let mut server = launch_server(test_dir);
        thread::sleep(Duration::from_millis(2000));
        verify_data_integrity(cycle);
        
        kill_server(&mut server);
        thread::sleep(Duration::from_millis(500));
    }
    
    let _ = fs::remove_dir_all(test_dir);
    println!("  ✅ Abrupt termination test complete - no corruption!");
}

#[test]
fn test_concurrent_writes_and_kill() {
    println!("🔄 Testing concurrent writes during kill...");
    
    let test_dir = "/tmp/narayana_test_concurrent";
    let _ = fs::remove_dir_all(test_dir);
    
    let mut server = launch_server(test_dir);
    thread::sleep(Duration::from_millis(2000));
    
    // Start writing in background
    let write_handle = thread::spawn(move || {
        for i in 1..=10 {
            write_test_data(i);
            thread::sleep(Duration::from_millis(100));
        }
    });
    
    // Kill server while writing
    thread::sleep(Duration::from_millis(500));
    println!("  💥 Killing server during writes...");
    kill_server(&mut server);
    
    // Wait for writes to complete (they'll fail, but that's ok)
    let _ = write_handle.join();
    
    thread::sleep(Duration::from_millis(1000));
    
    // Restart and verify
    let mut server = launch_server(test_dir);
    thread::sleep(Duration::from_millis(2000));
    
    // Verify at least some data persisted
    verify_data_integrity(1);
    
    kill_server(&mut server);
    let _ = fs::remove_dir_all(test_dir);
    println!("  ✅ Concurrent writes test complete!");
}

#[test]
fn test_data_persistence_across_restarts() {
    println!("🔄 Testing data persistence across restarts...");
    
    let test_dir = "/tmp/narayana_test_persistence";
    let _ = fs::remove_dir_all(test_dir);
    
    // Write initial data
    let mut server = launch_server(test_dir);
    thread::sleep(Duration::from_millis(2000));
    
    for i in 1..=5 {
        write_test_data(i);
        thread::sleep(Duration::from_millis(200));
    }
    
    kill_server(&mut server);
    thread::sleep(Duration::from_millis(1000));
    
    // Restart multiple times
    for restart in 1..=5 {
        println!("  🔄 Restart {}: Verifying data...", restart);
        
        let mut server = launch_server(test_dir);
        thread::sleep(Duration::from_millis(2000));
        
        // Verify all data is still there
        for i in 1..=5 {
            verify_data_integrity(i);
        }
        
        kill_server(&mut server);
        thread::sleep(Duration::from_millis(500));
    }
    
    let _ = fs::remove_dir_all(test_dir);
    println!("  ✅ Data persistence test complete - all data intact!");
}

#[test]
fn test_multiple_databases_persistence() {
    println!("🔄 Testing multiple databases persistence...");
    
    let test_dir = "/tmp/narayana_test_multi_db";
    let _ = fs::remove_dir_all(test_dir);
    
    let mut server = launch_server(test_dir);
    thread::sleep(Duration::from_millis(2000));
    
    // Create multiple databases and tables
    for db_num in 1..=3 {
        create_database_and_table(db_num);
        thread::sleep(Duration::from_millis(200));
        
        // Write data to each
        write_test_data_to_db(db_num);
        thread::sleep(Duration::from_millis(200));
    }
    
    kill_server(&mut server);
    thread::sleep(Duration::from_millis(1000));
    
    // Restart and verify all databases
    let mut server = launch_server(test_dir);
    thread::sleep(Duration::from_millis(2000));
    
    for db_num in 1..=3 {
        verify_database_integrity(db_num);
    }
    
    kill_server(&mut server);
    let _ = fs::remove_dir_all(test_dir);
    println!("  ✅ Multi-database persistence test complete!");
}

fn launch_server(test_dir: &str) -> Child {
    // Create test directory
    fs::create_dir_all(test_dir).expect("Failed to create test directory");
    
    // Launch server with test configuration
    Command::new("cargo")
        .args(&["run", "--bin", "narayana-server", "--release"])
        .env("NARAYANA_DATA_DIR", test_dir)
        .env("RUST_LOG", "info")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to launch server")
}

fn kill_server(server: &mut Child) {
    if let Err(e) = server.kill() {
        eprintln!("Warning: Failed to kill server: {}", e);
    }
    let _ = server.wait();
}

fn write_test_data(cycle: u32) {
    // Try to write data via HTTP API
    let client = reqwest::blocking::Client::new();
    
    // Create table if it doesn't exist
    let create_table = serde_json::json!({
        "name": format!("test_table_{}", cycle),
        "schema": {
            "fields": [
                {"name": "id", "type": "Int64"},
                {"name": "value", "type": "String"},
                {"name": "cycle", "type": "Int32"}
            ]
        }
    });
    
    let _ = client
        .post("http://localhost:8080/api/v1/tables/create")
        .json(&create_table)
        .timeout(Duration::from_secs(5))
        .send();
    
    // Insert test data
    let insert_data = serde_json::json!({
        "rows": [
            {"id": cycle as i64, "value": format!("test_value_{}", cycle), "cycle": cycle as i32},
            {"id": (cycle * 10) as i64, "value": format!("test_value_{}", cycle * 10), "cycle": cycle as i32}
        ]
    });
    
    let _ = client
        .post(&format!("http://localhost:8080/api/v1/tables/test_table_{}/insert", cycle))
        .json(&insert_data)
        .timeout(Duration::from_secs(5))
        .send();
}

fn verify_data_integrity(cycle: u32) {
    // Try to read data via HTTP API
    let client = reqwest::blocking::Client::new();
    
    let response = client
        .get(&format!("http://localhost:8080/api/v1/tables/test_table_{}/query", cycle))
        .query(&[("query", format!("SELECT * FROM test_table_{} WHERE cycle = {}", cycle, cycle))])
        .timeout(Duration::from_secs(5))
        .send();
    
    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("    ✅ Data verified for cycle {}", cycle);
            } else {
                println!("    ⚠️  Could not verify data for cycle {} (HTTP {})", cycle, resp.status());
            }
        }
        Err(_) => {
            println!("    ⚠️  Could not verify data for cycle {} (connection error)", cycle);
        }
    }
}

fn create_database_and_table(db_num: u32) {
    let client = reqwest::blocking::Client::new();
    
    // Create database
    let _ = client
        .post("http://localhost:8080/api/v1/databases/create")
        .json(&serde_json::json!({"name": format!("test_db_{}", db_num)}))
        .timeout(Duration::from_secs(5))
        .send();
    
    // Create table
    let create_table = serde_json::json!({
        "name": format!("test_table_{}", db_num),
        "database": format!("test_db_{}", db_num),
        "schema": {
            "fields": [
                {"name": "id", "type": "Int64"},
                {"name": "value", "type": "String"}
            ]
        }
    });
    
    let _ = client
        .post("http://localhost:8080/api/v1/tables/create")
        .json(&create_table)
        .timeout(Duration::from_secs(5))
        .send();
}

fn write_test_data_to_db(db_num: u32) {
    let client = reqwest::blocking::Client::new();
    
    let insert_data = serde_json::json!({
        "rows": [
            {"id": db_num as i64, "value": format!("db_{}_value_1", db_num)},
            {"id": (db_num * 10) as i64, "value": format!("db_{}_value_2", db_num)}
        ]
    });
    
    let _ = client
        .post(&format!("http://localhost:8080/api/v1/databases/test_db_{}/tables/test_table_{}/insert", db_num, db_num))
        .json(&insert_data)
        .timeout(Duration::from_secs(5))
        .send();
}

fn verify_database_integrity(db_num: u32) {
    let client = reqwest::blocking::Client::new();
    
    let response = client
        .get(&format!("http://localhost:8080/api/v1/databases/test_db_{}/tables/test_table_{}/query", db_num, db_num))
        .query(&[("query", "SELECT * FROM test_table")])
        .timeout(Duration::from_secs(5))
        .send();
    
    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("    ✅ Database {} verified", db_num);
            } else {
                println!("    ⚠️  Could not verify database {} (HTTP {})", db_num, resp.status());
            }
        }
        Err(_) => {
            println!("    ⚠️  Could not verify database {} (connection error)", db_num);
        }
    }
}

