// NarayanaDB Command Line Interface
// Comprehensive CLI for server management, database operations, and more

mod console;

use clap::{Parser, Subcommand};
use narayana_core::banner;
use narayana_core::schema::{Schema, Field, DataType};
use serde_json::json;
use std::io::{self, Write};
use std::process::Command;
use tokio::process::Command as TokioCommand;
use std::process::Stdio;
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "narayana")]
#[command(about = "NarayanaDB Command Line Interface - The Fastest Columnar Database", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(long, default_value = "http://localhost:8080", global = true)]
    server: String,
    
    #[arg(long, short, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start NarayanaDB server
    Start {
        /// HTTP port
        #[arg(long, default_value = "8080")]
        http_port: u16,
        
        /// gRPC port
        #[arg(long, default_value = "50051")]
        grpc_port: u16,
        
        /// GraphQL port
        #[arg(long, default_value = "4000")]
        graphql_port: u16,
        
        /// Data directory
        #[arg(long, default_value = "./data")]
        data_dir: String,
        
        /// Log level (error, warn, info, debug, trace)
        #[arg(long, default_value = "info")]
        log_level: String,
        
        /// Configuration file path
        #[arg(long, short)]
        config: Option<String>,
        
        /// Run in background (daemon mode)
        #[arg(long, short)]
        daemon: bool,
    },
    
    /// Stop NarayanaDB server
    Stop {
        /// Force stop (kill process)
        #[arg(long, short)]
        force: bool,
    },
    
    /// Show server status
    Status {
        /// Show detailed status
        #[arg(long, short)]
        detailed: bool,
    },
    
    /// Show server health
    Health,
    
    /// Show server logs
    Logs {
        /// Number of lines to show
        #[arg(long, short, default_value = "100")]
        lines: usize,
        
        /// Follow logs (like tail -f)
        #[arg(long, short)]
        follow: bool,
    },
    
    /// Database operations
    #[command(subcommand)]
    Database(DatabaseCommands),
    
    /// Table operations
    #[command(subcommand)]
    Table(TableCommands),
    
    /// Query operations
    Query {
        /// Query string
        query: String,
        
        /// Database name
        #[arg(long, short)]
        database: Option<String>,
        
        /// Output format (json, table, csv)
        #[arg(long, short, default_value = "table")]
        format: String,
    },
    
    /// Configuration management
    #[command(subcommand)]
    Config(ConfigCommands),
    
    /// Interactive console (REPL) - Like Rails console
    Console {
        /// Server URL
        #[arg(long, default_value = "http://localhost:8080")]
        server: Option<String>,
        
        /// Database to use on startup
        #[arg(long, short)]
        database: Option<String>,
    },
    
    /// Show server metrics
    Metrics {
        /// Metric name filter
        #[arg(long, short)]
        filter: Option<String>,
    },
    
    /// Manage webhooks
    #[command(subcommand)]
    Webhook(WebhookCommands),
    
    /// Backup and restore
    #[command(subcommand)]
    Backup(BackupCommands),
    
    /// Show version information
    Version,
    
    /// Show help and examples
    Help,
}

#[derive(Subcommand)]
enum DatabaseCommands {
    /// Create a new database
    Create {
        name: String,
    },
    
    /// List all databases
    List,
    
    /// Drop a database
    Drop {
        name: String,
        /// Force drop (no confirmation)
        #[arg(long, short)]
        force: bool,
    },
    
    /// Show database info
    Info {
        name: String,
    },
}

#[derive(Subcommand)]
enum TableCommands {
    /// Create a new table
    Create {
        /// Table name
        name: String,
        
        /// Database name
        #[arg(long, short)]
        database: Option<String>,
        
        /// Schema file (JSON)
        #[arg(long, short)]
        schema_file: Option<String>,
        
        /// Schema definition (inline JSON)
        #[arg(long)]
        schema: Option<String>,
    },
    
    /// List tables
    List {
        /// Database name
        #[arg(long, short)]
        database: Option<String>,
    },
    
    /// Show table schema
    Schema {
        /// Table name
        name: String,
        
        /// Database name
        #[arg(long, short)]
        database: Option<String>,
    },
    
    /// Drop a table
    Drop {
        /// Table name
        name: String,
        
        /// Database name
        #[arg(long, short)]
        database: Option<String>,
        
        /// Force drop (no confirmation)
        #[arg(long, short)]
        force: bool,
    },
    
    /// Insert data into a table
    Insert {
        /// Table name
        name: String,
        
        /// Database name
        #[arg(long, short)]
        database: Option<String>,
        
        /// Data file (JSON)
        #[arg(long, short)]
        file: Option<String>,
        
        /// Data (inline JSON)
        #[arg(long)]
        data: Option<String>,
    },
    
    /// Show table statistics
    Stats {
        /// Table name
        name: String,
        
        /// Database name
        #[arg(long, short)]
        database: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Show current configuration
    Show,
    
    /// Set configuration value
    Set {
        key: String,
        value: String,
    },
    
    /// Get configuration value
    Get {
        key: String,
    },
    
    /// Load configuration from file
    Load {
        file: String,
    },
    
    /// Save configuration to file
    Save {
        file: String,
    },
}

#[derive(Subcommand)]
enum WebhookCommands {
    /// Create a webhook
    Create {
        /// Webhook URL
        url: String,
        
        /// Event types (comma-separated)
        #[arg(long, short)]
        events: Option<String>,
        
        /// Scope (global, database, table)
        #[arg(long, short, default_value = "global")]
        scope: String,
    },
    
    /// List webhooks
    List,
    
    /// Delete a webhook
    Delete {
        webhook_id: String,
    },
}

#[derive(Subcommand)]
enum BackupCommands {
    /// Create a backup
    Create {
        /// Backup name
        name: Option<String>,
        
        /// Backup directory
        #[arg(long, short, default_value = "./backups")]
        dir: String,
    },
    
    /// List backups
    List {
        /// Backup directory
        #[arg(long, short, default_value = "./backups")]
        dir: String,
    },
    
    /// Restore from backup
    Restore {
        /// Backup name
        name: String,
        
        /// Backup directory
        #[arg(long, short, default_value = "./backups")]
        dir: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging if verbose
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    }

    match cli.command {
        Commands::Start { http_port, grpc_port, graphql_port, data_dir, log_level, config, daemon } => {
            start_server(http_port, grpc_port, graphql_port, data_dir, log_level, config, daemon).await?;
        }
        Commands::Stop { force } => {
            stop_server(force).await?;
        }
        Commands::Status { detailed } => {
            show_status(&cli.server, detailed).await?;
        }
        Commands::Health => {
            check_health(&cli.server).await?;
        }
        Commands::Logs { lines, follow } => {
            show_logs(lines, follow).await?;
        }
        Commands::Database(cmd) => {
            handle_database_command(&cli.server, cmd).await?;
        }
        Commands::Table(cmd) => {
            handle_table_command(&cli.server, cmd).await?;
        }
        Commands::Query { query, database, format } => {
            execute_query(&cli.server, &query, database.as_deref(), &format).await?;
        }
        Commands::Config(cmd) => {
            handle_config_command(&cli.server, cmd).await?;
        }
        Commands::Metrics { filter } => {
            show_metrics(&cli.server, filter.as_deref()).await?;
        }
        Commands::Webhook(cmd) => {
            handle_webhook_command(&cli.server, cmd).await?;
        }
        Commands::Backup(cmd) => {
            handle_backup_command(cmd).await?;
        }
        Commands::Console { server, database } => {
            let server_url = server.as_deref().unwrap_or(&cli.server);
            let mut console = console::InteractiveConsole::new(server_url.to_string());
            
            // Set database if provided
            if let Some(ref db) = database {
                console.current_database = Some(db.clone());
            }
            
            console.run().await?;
        }
        Commands::Version => {
            show_version();
        }
        Commands::Help => {
            show_help();
        }
    }

    Ok(())
}

/// Start NarayanaDB server
async fn start_server(
    http_port: u16,
    grpc_port: u16,
    graphql_port: u16,
    data_dir: String,
    log_level: String,
    config: Option<String>,
    daemon: bool,
) -> anyhow::Result<()> {
    // Show banner
    banner::print_colored_banner();
    
    println!("🚀 Starting NarayanaDB server...");
    println!("   HTTP Port:    {}", http_port);
    println!("   gRPC Port:     {}", grpc_port);
    println!("   GraphQL Port:  {}", graphql_port);
    println!("   Data Dir:      {}", data_dir);
    println!("   Log Level:     {}", log_level);
    if let Some(ref cfg) = config {
        println!("   Config File:   {}", cfg);
    }
    println!();

    // Set environment variables
    std::env::set_var("NARAYANA_HTTP_PORT", http_port.to_string());
    std::env::set_var("NARAYANA_GRPC_PORT", grpc_port.to_string());
    std::env::set_var("NARAYANA_GRAPHQL_PORT", graphql_port.to_string());
    std::env::set_var("NARAYANA_DATA_DIR", &data_dir);
    std::env::set_var("NARAYANA_LOG_LEVEL", &log_level);
    if let Some(ref cfg) = config {
        std::env::set_var("NARAYANA_CONFIG_FILE", cfg);
    }

    // SECURITY: Validate inputs to prevent command injection
    // Ports are already validated by clap (u16 type)
    // Data directory path validation
    if data_dir.contains("..") || data_dir.contains("//") || data_dir.contains("\\\\") {
        return Err(anyhow::anyhow!("Invalid data directory path"));
    }
    
    // Log level validation
    let valid_log_levels = ["error", "warn", "info", "debug", "trace"];
    if !valid_log_levels.contains(&log_level.as_str()) {
        return Err(anyhow::anyhow!("Invalid log level: {}", log_level));
    }
    
    if daemon {
        // Run in background
        // SECURITY: Fixed command injection - use hardcoded command, validate inputs
        let mut cmd = TokioCommand::new("cargo");
        // SECURITY: Use fixed arguments, no user input in command args
        cmd.args(&["run", "--bin", "narayana-server", "--release"]);
        cmd.env("NARAYANA_HTTP_PORT", http_port.to_string());
        cmd.env("NARAYANA_GRPC_PORT", grpc_port.to_string());
        cmd.env("NARAYANA_GRAPHQL_PORT", graphql_port.to_string());
        // SECURITY: Data dir already validated above
        cmd.env("NARAYANA_DATA_DIR", &data_dir);
        // SECURITY: Log level already validated above
        cmd.env("NARAYANA_LOG_LEVEL", &log_level);
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
        
        let child = cmd.spawn()?;
        println!("✅ NarayanaDB started in background (PID: {:?})", child.id());
        println!("   Use 'narayana status' to check server status");
    } else {
        // Run in foreground
        // SECURITY: Fixed command injection - use hardcoded command, validate inputs
        let mut cmd = TokioCommand::new("cargo");
        // SECURITY: Use fixed arguments, no user input in command args
        cmd.args(&["run", "--bin", "narayana-server", "--release"]);
        cmd.env("NARAYANA_HTTP_PORT", http_port.to_string());
        cmd.env("NARAYANA_GRPC_PORT", grpc_port.to_string());
        cmd.env("NARAYANA_GRAPHQL_PORT", graphql_port.to_string());
        // SECURITY: Data dir already validated above
        cmd.env("NARAYANA_DATA_DIR", &data_dir);
        // SECURITY: Log level already validated above
        cmd.env("NARAYANA_LOG_LEVEL", &log_level);
        
        let status = cmd.status().await?;
        if !status.success() {
            return Err(anyhow::anyhow!("Server failed to start"));
        }
    }

    Ok(())
}

/// Stop NarayanaDB server
async fn stop_server(force: bool) -> anyhow::Result<()> {
    println!("🛑 Stopping NarayanaDB server...");
    
    // SECURITY: Fixed command injection - use hardcoded commands, validate PID
    // Find server process
    let output = Command::new("pgrep")
        .arg("-f")
        .arg("narayana-server") // SECURITY: Fixed string, no user input
        .output()?;
    
    if output.stdout.is_empty() {
        println!("⚠️  No running NarayanaDB server found");
        return Ok(());
    }
    
    let pid_str = String::from_utf8(output.stdout)?;
    let pid_str = pid_str.trim();
    
    // SECURITY: Validate PID is numeric only (prevents command injection)
    if !pid_str.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow::anyhow!("Invalid PID format"));
    }
    
    let pid = pid_str.parse::<u32>()?;
    
    // SECURITY: Validate PID is reasonable (not 0 or system PIDs)
    if pid == 0 || pid < 100 {
        return Err(anyhow::anyhow!("Invalid PID: {}", pid));
    }
    
    if force {
        // SECURITY: Use fixed arguments, no user input
        Command::new("kill")
            .arg("-9")
            .arg(pid.to_string()) // SECURITY: Validated numeric PID
            .status()?;
        println!("✅ Server forcefully stopped (PID: {})", pid);
    } else {
        // SECURITY: Use fixed arguments, no user input
        Command::new("kill")
            .arg(pid.to_string()) // SECURITY: Validated numeric PID
            .status()?;
        println!("✅ Server stopped gracefully (PID: {})", pid);
    }
    
    Ok(())
}

/// Show server status
async fn show_status(server: &str, detailed: bool) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    
    match client.get(&format!("{}/health", server)).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let health: serde_json::Value = response.json().await?;
                println!("✅ NarayanaDB is running");
                println!("   Server: {}", server);
                
                if detailed {
                    println!("\n📊 Detailed Status:");
                    println!("{}", serde_json::to_string_pretty(&health)?);
                }
            } else {
                println!("⚠️  Server responded with error: {}", response.status());
            }
        }
        Err(_) => {
            println!("❌ NarayanaDB is not running");
            println!("   Server: {}", server);
            println!("\n💡 Start server with: narayana start");
        }
    }
    
    Ok(())
}

/// Check server health
async fn check_health(server: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    
    match client.get(&format!("{}/health", server)).send().await {
        Ok(response) => {
            if response.status().is_success() {
                let health: serde_json::Value = response.json().await?;
                println!("✅ Server is healthy");
                println!("{}", serde_json::to_string_pretty(&health)?);
            } else {
                println!("❌ Server health check failed: {}", response.status());
                std::process::exit(1);
            }
        }
        Err(e) => {
            println!("❌ Cannot connect to server: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}

/// Show server logs
async fn show_logs(lines: usize, _follow: bool) -> anyhow::Result<()> {
    // In production, would read from log file or journald
    println!("📋 Showing last {} lines of logs...", lines);
    println!("(Log viewing not fully implemented - would read from log file)");
    Ok(())
}

/// Handle database commands
async fn handle_database_command(server: &str, cmd: DatabaseCommands) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    
    match cmd {
        DatabaseCommands::Create { name } => {
            let response = client
                .post(&format!("{}/api/v1/databases", server))
                .json(&json!({ "name": name }))
                .send()
                .await?;
            
            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                println!("✅ Database '{}' created", name);
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("❌ Failed to create database: {}", response.status());
            }
        }
        DatabaseCommands::List => {
            let response = client
                .get(&format!("{}/api/v1/databases", server))
                .send()
                .await?;
            
            if response.status().is_success() {
                let databases: serde_json::Value = response.json().await?;
                println!("📊 Databases:");
                println!("{}", serde_json::to_string_pretty(&databases)?);
            } else {
                println!("❌ Failed to list databases: {}", response.status());
            }
        }
        DatabaseCommands::Drop { name, force } => {
            if !force {
                print!("⚠️  Are you sure you want to drop database '{}'? (yes/no): ", name);
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim().to_lowercase() != "yes" {
                    println!("❌ Cancelled");
                    return Ok(());
                }
            }
            
            let response = client
                .delete(&format!("{}/api/v1/databases/{}", server, name))
                .send()
                .await?;
            
            if response.status().is_success() {
                println!("✅ Database '{}' dropped", name);
            } else {
                println!("❌ Failed to drop database: {}", response.status());
            }
        }
        DatabaseCommands::Info { name } => {
            let response = client
                .get(&format!("{}/api/v1/databases/{}", server, name))
                .send()
                .await?;
            
            if response.status().is_success() {
                let info: serde_json::Value = response.json().await?;
                println!("📊 Database Info:");
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("❌ Failed to get database info: {}", response.status());
            }
        }
    }
    
    Ok(())
}

/// Handle table commands
async fn handle_table_command(server: &str, cmd: TableCommands) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    
    match cmd {
        TableCommands::Create { name, database, schema_file, schema } => {
            let schema_obj = if let Some(file) = schema_file {
                let content = std::fs::read_to_string(file)?;
                serde_json::from_str(&content)?
            } else if let Some(schema_str) = schema {
                serde_json::from_str(&schema_str)?
            } else {
                // Default schema
                json!({
                    "fields": [
                        {
                            "name": "id",
                            "data_type": "Int64",
                            "nullable": false
                        }
                    ]
                })
            };
            
            let url = if let Some(db) = database {
                format!("{}/api/v1/databases/{}/tables", server, db)
            } else {
                format!("{}/api/v1/tables", server)
            };
            
            let response = client
                .post(&url)
                .json(&json!({
                    "name": name,
                    "schema": schema_obj
                }))
                .send()
                .await?;
            
            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                println!("✅ Table '{}' created", name);
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("❌ Failed to create table: {}", response.status());
            }
        }
        TableCommands::List { database } => {
            let url = if let Some(db) = database {
                format!("{}/api/v1/databases/{}/tables", server, db)
            } else {
                format!("{}/api/v1/tables", server)
            };
            
            let response = client.get(&url).send().await?;
            
            if response.status().is_success() {
                let tables: serde_json::Value = response.json().await?;
                println!("📊 Tables:");
                println!("{}", serde_json::to_string_pretty(&tables)?);
            } else {
                println!("❌ Failed to list tables: {}", response.status());
            }
        }
        TableCommands::Schema { name, database } => {
            let url = if let Some(db) = database {
                format!("{}/api/v1/databases/{}/tables/{}/schema", server, db, name)
            } else {
                format!("{}/api/v1/tables/{}/schema", server, name)
            };
            
            let response = client.get(&url).send().await?;
            
            if response.status().is_success() {
                let schema: serde_json::Value = response.json().await?;
                println!("📋 Table Schema:");
                println!("{}", serde_json::to_string_pretty(&schema)?);
            } else {
                println!("❌ Failed to get schema: {}", response.status());
            }
        }
        TableCommands::Drop { name, database, force } => {
            if !force {
                print!("⚠️  Are you sure you want to drop table '{}'? (yes/no): ", name);
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim().to_lowercase() != "yes" {
                    println!("❌ Cancelled");
                    return Ok(());
                }
            }
            
            let url = if let Some(db) = database {
                format!("{}/api/v1/databases/{}/tables/{}", server, db, name)
            } else {
                format!("{}/api/v1/tables/{}", server, name)
            };
            
            let response = client.delete(&url).send().await?;
            
            if response.status().is_success() {
                println!("✅ Table '{}' dropped", name);
            } else {
                println!("❌ Failed to drop table: {}", response.status());
            }
        }
        TableCommands::Insert { name, database, file, data } => {
            let data_obj: serde_json::Value = if let Some(file_path) = file {
                let content = std::fs::read_to_string(file_path)?;
                serde_json::from_str(&content)?
            } else if let Some(data_str) = data {
                serde_json::from_str(&data_str)?
            } else {
                return Err(anyhow::anyhow!("Either --file or --data must be provided"));
            };
            
            let url = if let Some(db) = database {
                format!("{}/api/v1/databases/{}/tables/{}/insert", server, db, name)
            } else {
                format!("{}/api/v1/tables/{}/insert", server, name)
            };
            
            let response = client
                .post(&url)
                .json(&data_obj)
                .send()
                .await?;
            
            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                println!("✅ Data inserted into table '{}'", name);
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("❌ Failed to insert data: {}", response.status());
            }
        }
        TableCommands::Stats { name, database } => {
            let url = if let Some(db) = database {
                format!("{}/api/v1/databases/{}/tables/{}/stats", server, db, name)
            } else {
                format!("{}/api/v1/tables/{}/stats", server, name)
            };
            
            let response = client.get(&url).send().await?;
            
            if response.status().is_success() {
                let stats: serde_json::Value = response.json().await?;
                println!("📊 Table Statistics:");
                println!("{}", serde_json::to_string_pretty(&stats)?);
            } else {
                println!("❌ Failed to get stats: {}", response.status());
            }
        }
    }
    
    Ok(())
}

/// Execute query
async fn execute_query(server: &str, query: &str, database: Option<&str>, format: &str) -> anyhow::Result<()> {
    // SECURITY: Validate server URL to prevent SSRF in CLI
    // For CLI, we trust the server URL, but still validate format
    if !server.starts_with("http://") && !server.starts_with("https://") {
        return Err(anyhow::anyhow!("Server URL must start with http:// or https://"));
    }
    
    // SECURITY: Validate query length to prevent DoS
    if query.len() > 1_000_000 {
        return Err(anyhow::anyhow!("Query length {} exceeds maximum (1MB)", query.len()));
    }
    
    let client = reqwest::Client::new();
    
    let url = if let Some(db) = database {
        // SECURITY: Validate database name to prevent injection
        if db.contains('/') || db.contains('\\') || db.contains("..") {
            return Err(anyhow::anyhow!("Invalid database name: '{}'", db));
        }
        format!("{}/api/v1/databases/{}/query", server, db)
    } else {
        format!("{}/api/v1/query", server)
    };
    
    let response = client
        .post(&url)
        .json(&json!({ "query": query }))
        .send()
        .await?;
    
    if response.status().is_success() {
        let result: serde_json::Value = response.json().await?;
        
        match format {
            "json" => {
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            "csv" => {
                // Simple CSV output (would need proper implementation)
                println!("CSV format not fully implemented");
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            _ => {
                // Table format (would need proper table formatting)
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
    } else {
        println!("❌ Query failed: {}", response.status());
    }
    
    Ok(())
}

/// Handle config commands
async fn handle_config_command(server: &str, cmd: ConfigCommands) -> anyhow::Result<()> {
    match cmd {
        ConfigCommands::Show => {
            let client = reqwest::Client::new();
            let response = client.get(&format!("{}/api/v1/config", server)).send().await?;
            
            if response.status().is_success() {
                let config: serde_json::Value = response.json().await?;
                println!("⚙️  Configuration:");
                println!("{}", serde_json::to_string_pretty(&config)?);
            } else {
                println!("❌ Failed to get config: {}", response.status());
            }
        }
        ConfigCommands::Set { key, value } => {
            let client = reqwest::Client::new();
            let response = client
                .put(&format!("{}/api/v1/config/{}", server, key))
                .json(&json!({ "value": value }))
                .send()
                .await?;
            
            if response.status().is_success() {
                println!("✅ Configuration updated: {} = {}", key, value);
            } else {
                println!("❌ Failed to set config: {}", response.status());
            }
        }
        ConfigCommands::Get { key } => {
            let client = reqwest::Client::new();
            let response = client.get(&format!("{}/api/v1/config/{}", server, key)).send().await?;
            
            if response.status().is_success() {
                let value: serde_json::Value = response.json().await?;
                println!("{}", serde_json::to_string_pretty(&value)?);
            } else {
                println!("❌ Failed to get config: {}", response.status());
            }
        }
        ConfigCommands::Load { file } => {
            println!("📂 Loading configuration from: {}", file);
            // Implementation would load and apply config
        }
        ConfigCommands::Save { file } => {
            println!("💾 Saving configuration to: {}", file);
            // Implementation would save current config
        }
    }
    
    Ok(())
}

/// Show metrics
async fn show_metrics(server: &str, filter: Option<&str>) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let mut url = format!("{}/api/v1/metrics", server);
    
    if let Some(f) = filter {
        url.push_str(&format!("?filter={}", f));
    }
    
    let response = client.get(&url).send().await?;
    
    if response.status().is_success() {
        let metrics: serde_json::Value = response.json().await?;
        println!("📊 Metrics:");
        println!("{}", serde_json::to_string_pretty(&metrics)?);
    } else {
        println!("❌ Failed to get metrics: {}", response.status());
    }
    
    Ok(())
}

/// Handle webhook commands
async fn handle_webhook_command(server: &str, cmd: WebhookCommands) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    
    match cmd {
        WebhookCommands::Create { url, events, scope } => {
            let response = client
                .post(&format!("{}/api/v1/webhooks", server))
                .json(&json!({
                    "url": url,
                    "events": events.map(|e| e.split(',').map(|s| s.to_string()).collect::<Vec<_>>()),
                    "scope": scope
                }))
                .send()
                .await?;
            
            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                println!("✅ Webhook created");
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("❌ Failed to create webhook: {}", response.status());
            }
        }
        WebhookCommands::List => {
            let response = client.get(&format!("{}/api/v1/webhooks", server)).send().await?;
            
            if response.status().is_success() {
                let webhooks: serde_json::Value = response.json().await?;
                println!("🔔 Webhooks:");
                println!("{}", serde_json::to_string_pretty(&webhooks)?);
            } else {
                println!("❌ Failed to list webhooks: {}", response.status());
            }
        }
        WebhookCommands::Delete { webhook_id } => {
            let response = client
                .delete(&format!("{}/api/v1/webhooks/{}", server, webhook_id))
                .send()
                .await?;
            
            if response.status().is_success() {
                println!("✅ Webhook deleted");
            } else {
                println!("❌ Failed to delete webhook: {}", response.status());
            }
        }
    }
    
    Ok(())
}

/// Handle backup commands
async fn handle_backup_command(cmd: BackupCommands) -> anyhow::Result<()> {
    match cmd {
        BackupCommands::Create { name, dir } => {
            let backup_name = name.unwrap_or_else(|| {
                chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string()
            });
            println!("💾 Creating backup: {}", backup_name);
            println!("   Directory: {}", dir);
            // Implementation would create backup
        }
        BackupCommands::List { dir } => {
            println!("📋 Backups in: {}", dir);
            // Implementation would list backups
        }
        BackupCommands::Restore { name, dir } => {
            println!("🔄 Restoring backup: {}", name);
            println!("   Directory: {}", dir);
            // Implementation would restore backup
        }
    }
    
    Ok(())
}

/// Show version
fn show_version() {
    banner::print_colored_banner();
    println!("Version: 0.1.0");
    println!("License: Apache-2.0");
    println!("GitHub: https://github.com/carlosbarbosa/narayana");
}

/// Show help and examples
fn show_help() {
    println!("NarayanaDB CLI - Examples");
    println!();
    println!("Server Management:");
    println!("  narayana start                    # Start server");
    println!("  narayana start --daemon           # Start in background");
    println!("  narayana stop                     # Stop server");
    println!("  narayana status                   # Check status");
    println!("  narayana health                   # Health check");
    println!();
    println!("Database Operations:");
    println!("  narayana database create mydb    # Create database");
    println!("  narayana database list           # List databases");
    println!("  narayana database drop mydb      # Drop database");
    println!();
    println!("Table Operations:");
    println!("  narayana table create users      # Create table");
    println!("  narayana table list              # List tables");
    println!("  narayana table schema users      # Show schema");
    println!("  narayana table insert users --file data.json");
    println!();
    println!("Query:");
    println!("  narayana query \"SELECT * FROM users\"");
    println!();
    println!("For more information, see: https://github.com/carlosbarbosa/narayana");
}
