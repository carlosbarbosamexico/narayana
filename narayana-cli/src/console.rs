// NarayanaDB Interactive Console
// Rails console-like REPL for interactive database access

use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::{json, Value};
use std::io::{self, Write, BufRead, BufReader};
use std::time::{Duration, SystemTime};
use std::collections::HashMap;

pub struct InteractiveConsole {
    client: Client,
    server_url: String,
    pub current_database: Option<String>,
    history: Vec<String>,
    vars: HashMap<String, Value>,
    prompt: String,
}

impl InteractiveConsole {
    pub fn new(server_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            server_url,
            current_database: None,
            history: Vec::new(),
            vars: HashMap::new(),
            prompt: "narayana".to_string(),
        }
    }

    /// Start the interactive console
    pub async fn run(&mut self) -> Result<()> {
        self.print_banner();
        self.print_help();

        let stdin = io::stdin();
        let mut stdin = BufReader::new(stdin.lock());

        loop {
            // Update prompt with current database
            self.update_prompt();
            print!("{}> ", self.prompt);
            io::stdout().flush()?;

            let mut line = String::new();
            stdin.read_line(&mut line)?;

            let line = line.trim().to_string();

            if line.is_empty() {
                continue;
            }

            // Add to history
            if !self.history.contains(&line) {
                self.history.push(line.clone());
            }

            // Handle commands
            match self.handle_command(&line).await {
                Ok(CommandResult::Continue) => continue,
                Ok(CommandResult::Exit) => break,
                Ok(CommandResult::Success(msg)) => {
                    if !msg.is_empty() {
                        println!("✅ {}", msg);
                    }
                }
                Ok(CommandResult::Error(msg)) => {
                    println!("❌ Error: {}", msg);
                }
                Ok(CommandResult::Output(output)) => {
                    println!("{}", output);
                }
                Err(e) => {
                    println!("❌ Error: {}", e);
                }
            }
        }

        println!("\n👋 Goodbye!");
        Ok(())
    }

    fn print_banner(&self) {
        println!("\n╔═══════════════════════════════════════════════════════════════╗");
        println!("║                  NarayanaDB Interactive Console               ║");
        println!("║           The Fastest Columnar Database - Just Works™         ║");
        println!("╚═══════════════════════════════════════════════════════════════╝");
        println!();
        println!("Connected to: {}", self.server_url);
        println!("Type 'help' for available commands, 'exit' to quit");
        println!();
    }

    fn print_help(&self) {
        println!("📚 Available Commands:");
        println!("  help, ?           - Show this help message");
        println!("  exit, quit, q     - Exit the console");
        println!("  clear, cls        - Clear the screen");
        println!("  databases, dbs    - List all databases");
        println!("  use <db>          - Switch to a database");
        println!("  tables, tbls      - List all tables");
        println!("  describe <table>  - Show table schema");
        println!("  query <sql>       - Execute a query");
        println!("  history           - Show command history");
        println!("  var <name>        - Show variable value");
        println!("  vars              - List all variables");
        println!("  save <var>        - Save last result to variable");
        println!("");
        println!("💡 Examples:");
        println!("  use mydb");
        println!("  tables");
        println!("  describe users");
        println!("  query SELECT * FROM users LIMIT 10");
        println!("  save result");
        println!("  var result");
        println!("");
    }

    fn update_prompt(&mut self) {
        if let Some(ref db) = self.current_database {
            self.prompt = format!("narayana[{}]", db);
        } else {
            self.prompt = "narayana".to_string();
        }
    }

    async fn handle_command(&mut self, line: &str) -> Result<CommandResult> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            return Ok(CommandResult::Continue);
        }

        let command = parts[0].to_lowercase();

        match command.as_str() {
            "exit" | "quit" | "q" => Ok(CommandResult::Exit),
            "help" | "?" => {
                self.print_help();
                Ok(CommandResult::Continue)
            }
            "clear" | "cls" => {
                print!("\x1B[2J\x1B[1;1H");
                io::stdout().flush()?;
                Ok(CommandResult::Continue)
            }
            "databases" | "dbs" => self.list_databases().await,
            "use" => {
                if parts.len() < 2 {
                    return Ok(CommandResult::Error("Usage: use <database_name>".to_string()));
                }
                self.use_database(parts[1]).await
            }
            "tables" | "tbls" => self.list_tables().await,
            "describe" | "desc" => {
                if parts.len() < 2 {
                    return Ok(CommandResult::Error("Usage: describe <table_name>".to_string()));
                }
                self.describe_table(parts[1]).await
            }
            "query" => {
                if parts.len() < 2 {
                    return Ok(CommandResult::Error("Usage: query <sql_query>".to_string()));
                }
                let query = parts[1..].join(" ");
                self.execute_query(&query).await
            }
            "history" => {
                println!("📜 Command History:");
                for (i, cmd) in self.history.iter().enumerate() {
                    println!("  {}: {}", i + 1, cmd);
                }
                Ok(CommandResult::Continue)
            }
            "var" => {
                if parts.len() < 2 {
                    return Ok(CommandResult::Error("Usage: var <variable_name>".to_string()));
                }
                self.show_var(parts[1])
            }
            "vars" => {
                if self.vars.is_empty() {
                    println!("No variables set");
                } else {
                    println!("📦 Variables:");
                    for (name, _) in &self.vars {
                        println!("  ${}", name);
                    }
                }
                Ok(CommandResult::Continue)
            }
            "save" => {
                if parts.len() < 2 {
                    return Ok(CommandResult::Error("Usage: save <variable_name>".to_string()));
                }
                Ok(CommandResult::Error("Save last result feature coming soon".to_string()))
            }
            _ => {
                // Try to execute as a query
                if line.starts_with("SELECT") || line.starts_with("select") ||
                   line.starts_with("SHOW") || line.starts_with("show") ||
                   line.starts_with("DESCRIBE") || line.starts_with("describe") ||
                   line.starts_with("USE") || line.starts_with("use") {
                    self.execute_query(line).await
                } else {
                    Ok(CommandResult::Error(format!("Unknown command: {}. Type 'help' for available commands.", command)))
                }
            }
        }
    }

    async fn list_databases(&self) -> Result<CommandResult> {
        let url = format!("{}/api/v1/databases", self.server_url);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let data: Value = response.json().await?;
            
            if let Some(databases) = data.get("databases").and_then(|v| v.as_array()) {
                println!("📚 Databases:");
                for db in databases {
                    if let Some(name) = db.get("name").and_then(|v| v.as_str()) {
                        let current = if self.current_database.as_ref() == Some(&name.to_string()) {
                            " (current)"
                        } else {
                            ""
                        };
                        println!("  • {}{}", name, current);
                    }
                }
            }
            Ok(CommandResult::Continue)
        } else {
            Ok(CommandResult::Error(format!("HTTP {}: {}", response.status(), response.text().await?)))
        }
    }

    async fn use_database(&mut self, db_name: &str) -> Result<CommandResult> {
        // Verify database exists
        let url = format!("{}/api/v1/databases/{}", self.server_url, db_name);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            self.current_database = Some(db_name.to_string());
            Ok(CommandResult::Success(format!("Switched to database '{}'", db_name)))
        } else {
            Ok(CommandResult::Error(format!("Database '{}' not found", db_name)))
        }
    }

    async fn list_tables(&self) -> Result<CommandResult> {
        let url = if let Some(ref db) = self.current_database {
            format!("{}/api/v1/databases/{}/tables", self.server_url, db)
        } else {
            format!("{}/api/v1/tables", self.server_url)
        };

        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let data: Value = response.json().await?;
            
            if let Some(tables) = data.get("tables").and_then(|v| v.as_array()) {
                println!("📊 Tables:");
                for table in tables {
                    if let Some(name) = table.get("name").and_then(|v| v.as_str()) {
                        println!("  • {}", name);
                    }
                }
            }
            Ok(CommandResult::Continue)
        } else {
            Ok(CommandResult::Error(format!("HTTP {}: {}", response.status(), response.text().await?)))
        }
    }

    async fn describe_table(&self, table_name: &str) -> Result<CommandResult> {
        let url = if let Some(ref db) = self.current_database {
            format!("{}/api/v1/databases/{}/tables/{}", self.server_url, db, table_name)
        } else {
            format!("{}/api/v1/tables/{}", self.server_url, table_name)
        };

        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let data: Value = response.json().await?;
            
            if let Some(schema) = data.get("schema") {
                println!("📋 Table: {}", table_name);
                println!("   Schema:");
                
                if let Some(fields) = schema.get("fields").and_then(|v| v.as_array()) {
                    println!("   {:<20} {:<15} {:<10}", "Field", "Type", "Nullable");
                    println!("   {}", "-".repeat(45));
                    for field in fields {
                        let name = field.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let type_str = field.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        let nullable = field.get("nullable").and_then(|v| v.as_bool()).unwrap_or(false);
                        println!("   {:<20} {:<15} {}", name, type_str, if nullable { "Yes" } else { "No" });
                    }
                }
            }
            Ok(CommandResult::Continue)
        } else {
            Ok(CommandResult::Error(format!("HTTP {}: {}", response.status(), response.text().await?)))
        }
    }

    async fn execute_query(&self, query: &str) -> Result<CommandResult> {
        // Handle USE command
        if query.to_uppercase().starts_with("USE ") {
            let parts: Vec<&str> = query.split_whitespace().collect();
            if parts.len() >= 2 {
                // This would need mutable self, so handle separately
                return Ok(CommandResult::Error("Use 'use <db>' command instead".to_string()));
            }
        }

        let url = if let Some(ref db) = self.current_database {
            format!("{}/api/v1/databases/{}/query", self.server_url, db)
        } else {
            format!("{}/api/v1/query", self.server_url)
        };

        let payload = json!({
            "query": query
        });

        let start = std::time::Instant::now();
        let response = self.client.post(&url).json(&payload).send().await?;
        let elapsed = start.elapsed();

        let status = response.status();
        if status.is_success() {
            let data: Value = response.json().await?;
            
            // Pretty print result
            let output = self.format_query_result(&data, elapsed);
            Ok(CommandResult::Output(output))
        } else {
            let error_text = response.text().await?;
            Ok(CommandResult::Error(format!("HTTP {}: {}", status, error_text)))
        }
    }

    fn format_query_result(&self, data: &Value, elapsed: Duration) -> String {
        let mut output = String::new();

        // Format time
        let time_str = if elapsed.as_millis() > 0 {
            format!("{}ms", elapsed.as_millis())
        } else {
            format!("{:.2}μs", elapsed.as_micros() as f64)
        };

        // Format rows
        if let Some(rows) = data.get("rows").and_then(|v| v.as_array()) {
            if rows.is_empty() {
                output.push_str(&format!("✅ Query executed successfully ({}ms)\n", elapsed.as_millis()));
                output.push_str("📊 Result: 0 rows\n");
            } else {
                // Get column names from first row
                if let Some(first_row) = rows.first().and_then(|v| v.as_object()) {
                    let columns: Vec<&str> = first_row.keys().map(|k| k.as_str()).collect();
                    
                    // Calculate column widths
                    let mut widths: Vec<usize> = columns.iter().map(|c| c.len()).collect();
                    
                    for row in rows {
                        if let Some(obj) = row.as_object() {
                            for (i, col) in columns.iter().enumerate() {
                                if let Some(val) = obj.get(*col) {
                                    let val_str = format!("{}", val);
                                    widths[i] = widths[i].max(val_str.len().min(50)); // Max 50 chars per cell
                                }
                            }
                        }
                    }

                    // Print header
                    output.push_str("┌");
                    for (i, (col, &width)) in columns.iter().zip(widths.iter()).enumerate() {
                        if i > 0 { output.push_str("┬"); }
                        output.push_str(&format!("{:─<1$}", "", width + 2));
                    }
                    output.push_str("┐\n");

                    output.push_str("│");
                    for (col, &width) in columns.iter().zip(widths.iter()) {
                        output.push_str(&format!(" {:1$} │", col, width));
                    }
                    output.push_str("\n");

                    output.push_str("├");
                    for (i, &width) in widths.iter().enumerate() {
                        if i > 0 { output.push_str("┼"); }
                        output.push_str(&format!("{:─<1$}", "", width + 2));
                    }
                    output.push_str("┤\n");

                    // Print rows
                    for row in rows.iter().take(100) { // Limit to 100 rows
                        if let Some(obj) = row.as_object() {
                            output.push_str("│");
                            for (col, &width) in columns.iter().zip(widths.iter()) {
                                let val = obj.get(*col).map(|v| format!("{}", v)).unwrap_or_default();
                                let display_val = if val.len() > 50 {
                                    format!("{}...", &val[..47])
                                } else {
                                    val
                                };
                                output.push_str(&format!(" {:1$} │", display_val, width));
                            }
                            output.push_str("\n");
                        }
                    }

                    output.push_str("└");
                    for (i, &width) in widths.iter().enumerate() {
                        if i > 0 { output.push_str("┴"); }
                        output.push_str(&format!("{:─<1$}", "", width + 2));
                    }
                    output.push_str("┘\n");

                    output.push_str(&format!("✅ {} row(s) in {}ms\n", rows.len(), elapsed.as_millis()));
                    
                    if rows.len() == 100 {
                        output.push_str("⚠️  Showing first 100 rows\n");
                    }
                }
            }
        } else {
            // Not a query result, just print the data
            output.push_str(&format!("✅ Query executed successfully ({}ms)\n", elapsed.as_millis()));
            output.push_str(&serde_json::to_string_pretty(data).unwrap_or_default());
            output.push_str("\n");
        }

        output
    }

    fn show_var(&self, var_name: &str) -> Result<CommandResult> {
        if let Some(value) = self.vars.get(var_name) {
            Ok(CommandResult::Output(serde_json::to_string_pretty(value)?))
        } else {
            Ok(CommandResult::Error(format!("Variable '{}' not found", var_name)))
        }
    }
}

enum CommandResult {
    Continue,
    Exit,
    Success(String),
    Error(String),
    Output(String),
}

