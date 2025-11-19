// Best Autocomplete Ever - Makes NarayanaDB a No-Brainer to Use
// Intelligent, context-aware, learning autocomplete system

use narayana_core::{Error, Result, schema::{Schema, Field, DataType}};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;
use dashmap::DashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Autocomplete suggestion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Suggestion {
    pub text: String,
    pub display_text: String,
    pub suggestion_type: SuggestionType,
    pub relevance_score: f64,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SuggestionType {
    Keyword,
    Table,
    Column,
    Function,
    Variable,
    Alias,
    Join,
    Aggregate,
    WindowFunction,
    DataType,
    Operator,
    Clause,
    Option,
    Parameter,
}

/// Autocomplete context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutocompleteContext {
    pub current_query: String,
    pub cursor_position: usize,
    pub current_word: String,
    pub previous_words: Vec<String>,
    pub next_words: Vec<String>,
    pub current_clause: Option<ClauseType>,
    pub available_tables: Vec<String>,
    pub current_table: Option<String>,
    pub available_columns: Vec<String>,
    pub query_history: Vec<String>,
    pub user_preferences: HashMap<String, String>,
}

impl Default for AutocompleteContext {
    fn default() -> Self {
        Self {
            current_query: String::new(),
            cursor_position: 0,
            current_word: String::new(),
            previous_words: Vec::new(),
            next_words: Vec::new(),
            current_clause: None,
            available_tables: Vec::new(),
            current_table: None,
            available_columns: Vec::new(),
            query_history: Vec::new(),
            user_preferences: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClauseType {
    Select,
    From,
    Where,
    Join,
    GroupBy,
    OrderBy,
    Having,
    Limit,
    Insert,
    Update,
    Delete,
    Create,
    Alter,
    Drop,
}

/// Autocomplete manager - the best autocomplete ever
pub struct AutocompleteManager {
    // Schema information
    schemas: Arc<DashMap<String, Schema>>,
    
    // Keyword dictionary
    keywords: Arc<RwLock<HashSet<String>>>,
    
    // Function registry
    functions: Arc<DashMap<String, FunctionInfo>>,
    
    // User learning - tracks what users type
    user_patterns: Arc<DashMap<String, UserPattern>>,
    
    // Popular queries
    popular_queries: Arc<RwLock<Vec<PopularQuery>>>,
    
    // Fuzzy matching index
    fuzzy_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
    
    // Configuration
    config: AutocompleteConfig,
    
    // Statistics
    stats: Arc<RwLock<AutocompleteStats>>,
}

/// Function information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: DataType,
    pub category: FunctionCategory,
    pub examples: Vec<String>,
    pub usage_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FunctionCategory {
    Aggregate,
    String,
    Math,
    Date,
    Window,
    Array,
    JSON,
    Conversion,
    Conditional,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub parameter_type: DataType,
    pub required: bool,
    pub default_value: Option<String>,
    pub description: String,
}

/// User pattern - learns from user behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPattern {
    pub pattern: String,
    pub frequency: u64,
    pub last_used: u64,
    pub context: HashMap<String, String>,
    pub suggestions: Vec<String>,
}

/// Popular query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopularQuery {
    pub query: String,
    pub frequency: u64,
    pub last_used: u64,
    pub success_rate: f64,
}

/// Autocomplete configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutocompleteConfig {
    pub max_suggestions: usize,
    pub enable_fuzzy_matching: bool,
    pub fuzzy_threshold: f64,
    pub enable_learning: bool,
    pub enable_context_aware: bool,
    pub enable_smart_suggestions: bool,
    pub enable_syntax_highlighting: bool,
    pub enable_parameter_hints: bool,
    pub enable_query_templates: bool,
    pub enable_shortcuts: bool,
    pub min_relevance_score: f64,
}

impl Default for AutocompleteConfig {
    fn default() -> Self {
        Self {
            max_suggestions: 20,
            enable_fuzzy_matching: true,
            fuzzy_threshold: 0.7,
            enable_learning: true,
            enable_context_aware: true,
            enable_smart_suggestions: true,
            enable_syntax_highlighting: true,
            enable_parameter_hints: true,
            enable_query_templates: true,
            enable_shortcuts: true,
            min_relevance_score: 0.3,
        }
    }
}

/// Autocomplete statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutocompleteStats {
    pub total_suggestions: u64,
    pub accepted_suggestions: u64,
    pub average_relevance: f64,
    pub learning_patterns: usize,
    pub popular_queries_count: usize,
}

impl AutocompleteManager {
    pub fn new(config: AutocompleteConfig) -> Self {
        let mut manager = Self {
            schemas: Arc::new(DashMap::new()),
            keywords: Arc::new(RwLock::new(Self::init_keywords())),
            functions: Arc::new(DashMap::new()),
            user_patterns: Arc::new(DashMap::new()),
            popular_queries: Arc::new(RwLock::new(Vec::new())),
            fuzzy_index: Arc::new(RwLock::new(HashMap::new())),
            config: config.clone(),
            stats: Arc::new(RwLock::new(AutocompleteStats {
                total_suggestions: 0,
                accepted_suggestions: 0,
                average_relevance: 0.0,
                learning_patterns: 0,
                popular_queries_count: 0,
            })),
        };
        
        // Initialize functions
        manager.init_functions();
        
        manager
    }

    /// Get autocomplete suggestions - the best ever
    pub fn get_suggestions(&self, context: AutocompleteContext) -> Result<Vec<Suggestion>> {
        let mut suggestions = Vec::new();
        
        // 1. Context-aware suggestions based on current clause
        if self.config.enable_context_aware {
            suggestions.extend(self.get_context_aware_suggestions(&context)?);
        }
        
        // 2. Keyword suggestions
        suggestions.extend(self.get_keyword_suggestions(&context)?);
        
        // 3. Table/column suggestions
        suggestions.extend(self.get_schema_suggestions(&context)?);
        
        // 4. Function suggestions
        suggestions.extend(self.get_function_suggestions(&context)?);
        
        // 5. Smart suggestions based on user patterns
        if self.config.enable_learning {
            suggestions.extend(self.get_learning_suggestions(&context)?);
        }
        
        // 6. Popular query suggestions
        suggestions.extend(self.get_popular_query_suggestions(&context)?);
        
        // 7. Fuzzy matching suggestions
        if self.config.enable_fuzzy_matching {
            suggestions.extend(self.get_fuzzy_suggestions(&context)?);
        }
        
        // Sort by relevance
        suggestions.sort_by(|a, b| {
            b.relevance_score.partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Filter by minimum relevance
        suggestions.retain(|s| s.relevance_score >= self.config.min_relevance_score);
        
        // Limit results
        suggestions.truncate(self.config.max_suggestions);
        
        // Update statistics
        let mut stats = self.stats.write();
        stats.total_suggestions += suggestions.len() as u64;
        if !suggestions.is_empty() {
            let avg_relevance: f64 = suggestions.iter().map(|s| s.relevance_score).sum::<f64>() / suggestions.len() as f64;
            stats.average_relevance = (stats.average_relevance + avg_relevance) / 2.0;
        }
        
        Ok(suggestions)
    }

    /// Context-aware suggestions
    fn get_context_aware_suggestions(&self, context: &AutocompleteContext) -> Result<Vec<Suggestion>> {
        let mut suggestions = Vec::new();
        
        match context.current_clause {
            Some(ClauseType::Select) => {
                // Suggest columns from current table
                if let Some(ref table) = context.current_table {
                    if let Some(schema) = self.schemas.get(table) {
                        for field in &schema.fields {
                            suggestions.push(Suggestion {
                                text: field.name.clone(),
                                display_text: format!("{} ({})", field.name, self.format_type(&field.data_type)),
                                suggestion_type: SuggestionType::Column,
                                relevance_score: 0.9,
                                description: Some(format!("Column from {}", table)),
                                icon: Some("column".to_string()),
                                metadata: HashMap::new(),
                            });
                        }
                    }
                }
                // Suggest aggregate functions
                suggestions.extend(self.get_aggregate_functions());
            }
            Some(ClauseType::From) => {
                // Suggest tables
                for table in &context.available_tables {
                    suggestions.push(Suggestion {
                        text: table.clone(),
                        display_text: table.clone(),
                        suggestion_type: SuggestionType::Table,
                        relevance_score: 0.9,
                        description: None,
                        icon: Some("table".to_string()),
                        metadata: HashMap::new(),
                    });
                }
            }
            Some(ClauseType::Where) => {
                // Suggest columns and operators
                suggestions.extend(self.get_column_suggestions(context)?);
                suggestions.extend(self.get_operator_suggestions());
            }
            Some(ClauseType::Join) => {
                // Suggest tables and join types
                suggestions.extend(self.get_join_suggestions(context)?);
            }
            Some(ClauseType::OrderBy) => {
                // Suggest columns
                suggestions.extend(self.get_column_suggestions(context)?);
            }
            Some(ClauseType::GroupBy) => {
                // Suggest columns
                suggestions.extend(self.get_column_suggestions(context)?);
            }
            _ => {}
        }
        
        Ok(suggestions)
    }

    /// Keyword suggestions
    fn get_keyword_suggestions(&self, context: &AutocompleteContext) -> Result<Vec<Suggestion>> {
        let mut suggestions = Vec::new();
        let keywords = self.keywords.read();
        let current_word_lower = context.current_word.to_lowercase();
        
        for keyword in keywords.iter() {
            if keyword.starts_with(&current_word_lower) || current_word_lower.is_empty() {
                let relevance = if keyword.starts_with(&current_word_lower) {
                    0.8
                } else {
                    0.5
                };
                
                suggestions.push(Suggestion {
                    text: keyword.to_uppercase(),
                    display_text: keyword.to_uppercase(),
                    suggestion_type: SuggestionType::Keyword,
                    relevance_score: relevance,
                    description: Some(self.get_keyword_description(keyword)),
                    icon: Some("keyword".to_string()),
                    metadata: HashMap::new(),
                });
            }
        }
        
        Ok(suggestions)
    }

    /// Schema suggestions (tables, columns)
    fn get_schema_suggestions(&self, context: &AutocompleteContext) -> Result<Vec<Suggestion>> {
        let mut suggestions = Vec::new();
        let current_word_lower = context.current_word.to_lowercase();
        
        // Table suggestions
        for table in &context.available_tables {
            if table.to_lowercase().contains(&current_word_lower) || current_word_lower.is_empty() {
                let relevance = if table.to_lowercase().starts_with(&current_word_lower) {
                    0.9
                } else {
                    0.7
                };
                
                suggestions.push(Suggestion {
                    text: table.clone(),
                    display_text: format!("📊 {}", table),
                    suggestion_type: SuggestionType::Table,
                    relevance_score: relevance,
                    description: None,
                    icon: Some("table".to_string()),
                    metadata: HashMap::new(),
                });
            }
        }
        
        // Column suggestions
        for column in &context.available_columns {
            if column.to_lowercase().contains(&current_word_lower) || current_word_lower.is_empty() {
                let relevance = if column.to_lowercase().starts_with(&current_word_lower) {
                    0.85
                } else {
                    0.65
                };
                
                suggestions.push(Suggestion {
                    text: column.clone(),
                    display_text: format!("📋 {}", column),
                    suggestion_type: SuggestionType::Column,
                    relevance_score: relevance,
                    description: None,
                    icon: Some("column".to_string()),
                    metadata: HashMap::new(),
                });
            }
        }
        
        Ok(suggestions)
    }

    /// Function suggestions
    fn get_function_suggestions(&self, context: &AutocompleteContext) -> Result<Vec<Suggestion>> {
        let mut suggestions = Vec::new();
        let current_word_lower = context.current_word.to_lowercase();
        
        for entry in self.functions.iter() {
            let name = entry.key();
            let func_info = entry.value();
            if name.to_lowercase().starts_with(&current_word_lower) || current_word_lower.is_empty() {
                let relevance = if name.to_lowercase().starts_with(&current_word_lower) {
                    0.85 + (func_info.usage_count as f64 / 1000.0).min(0.1)
                } else {
                    0.6
                };
                
                suggestions.push(Suggestion {
                    text: format!("{}({})", name, self.format_parameters(&func_info.parameters)),
                    display_text: format!("🔧 {} - {}", name, func_info.description),
                    suggestion_type: SuggestionType::Function,
                    relevance_score: relevance,
                    description: Some(func_info.description.clone()),
                    icon: Some("function".to_string()),
                    metadata: HashMap::from([
                        ("category".to_string(), format!("{:?}", func_info.category)),
                        ("return_type".to_string(), format!("{:?}", func_info.return_type)),
                    ]),
                });
            }
        }
        
        Ok(suggestions)
    }

    /// Learning-based suggestions
    fn get_learning_suggestions(&self, context: &AutocompleteContext) -> Result<Vec<Suggestion>> {
        let mut suggestions = Vec::new();
        
        // Find similar patterns
        for entry in self.user_patterns.iter() {
            let pattern = entry.key();
            let user_pattern = entry.value();
            if Self::pattern_matches(&context.current_query, pattern) {
                for suggestion_text in &user_pattern.suggestions {
                    suggestions.push(Suggestion {
                        text: suggestion_text.clone(),
                        display_text: format!("💡 {}", suggestion_text),
                        suggestion_type: SuggestionType::Variable,
                        relevance_score: 0.7 + (user_pattern.frequency as f64 / 100.0).min(0.2),
                        description: Some("Based on your usage patterns".to_string()),
                        icon: Some("learning".to_string()),
                        metadata: HashMap::new(),
                    });
                }
            }
        }
        
        Ok(suggestions)
    }

    /// Popular query suggestions
    fn get_popular_query_suggestions(&self, context: &AutocompleteContext) -> Result<Vec<Suggestion>> {
        let mut suggestions = Vec::new();
        let popular = self.popular_queries.read();
        
        for query in popular.iter().take(5) {
            if query.query.contains(&context.current_word) || context.current_word.is_empty() {
                suggestions.push(Suggestion {
                    text: query.query.clone(),
                    display_text: format!("⭐ {}", query.query),
                    suggestion_type: SuggestionType::Clause,
                    relevance_score: 0.6 + (query.success_rate * 0.3),
                    description: Some(format!("Used {} times", query.frequency)),
                    icon: Some("popular".to_string()),
                    metadata: HashMap::new(),
                });
            }
        }
        
        Ok(suggestions)
    }

    /// Fuzzy matching suggestions
    fn get_fuzzy_suggestions(&self, context: &AutocompleteContext) -> Result<Vec<Suggestion>> {
        let mut suggestions = Vec::new();
        
        if context.current_word.len() < 2 {
            return Ok(suggestions);
        }
        
        // Fuzzy match against all known identifiers
        let all_identifiers = self.get_all_identifiers();
        
        for identifier in all_identifiers {
            let similarity = Self::fuzzy_match(&context.current_word, &identifier);
            if similarity >= self.config.fuzzy_threshold {
                suggestions.push(Suggestion {
                    text: identifier.clone(),
                    display_text: format!("🔍 {} (fuzzy match)", identifier),
                    suggestion_type: SuggestionType::Variable,
                    relevance_score: similarity * 0.6,
                    description: Some(format!("Fuzzy match: {:.0}%", similarity * 100.0)),
                    icon: Some("fuzzy".to_string()),
                    metadata: HashMap::new(),
                });
            }
        }
        
        Ok(suggestions)
    }

    /// Helper methods
    fn get_column_suggestions(&self, context: &AutocompleteContext) -> Result<Vec<Suggestion>> {
        self.get_schema_suggestions(context)
    }

    fn get_operator_suggestions(&self) -> Vec<Suggestion> {
        vec![
            Suggestion {
                text: "=".to_string(),
                display_text: "= (equals)".to_string(),
                suggestion_type: SuggestionType::Operator,
                relevance_score: 0.9,
                description: Some("Equals operator".to_string()),
                icon: Some("operator".to_string()),
                metadata: HashMap::new(),
            },
            Suggestion {
                text: "!=".to_string(),
                display_text: "!= (not equals)".to_string(),
                suggestion_type: SuggestionType::Operator,
                relevance_score: 0.9,
                description: Some("Not equals operator".to_string()),
                icon: Some("operator".to_string()),
                metadata: HashMap::new(),
            },
            Suggestion {
                text: ">".to_string(),
                display_text: "> (greater than)".to_string(),
                suggestion_type: SuggestionType::Operator,
                relevance_score: 0.9,
                description: Some("Greater than operator".to_string()),
                icon: Some("operator".to_string()),
                metadata: HashMap::new(),
            },
            Suggestion {
                text: "<".to_string(),
                display_text: "< (less than)".to_string(),
                suggestion_type: SuggestionType::Operator,
                relevance_score: 0.9,
                description: Some("Less than operator".to_string()),
                icon: Some("operator".to_string()),
                metadata: HashMap::new(),
            },
            Suggestion {
                text: "LIKE".to_string(),
                display_text: "LIKE (pattern match)".to_string(),
                suggestion_type: SuggestionType::Operator,
                relevance_score: 0.9,
                description: Some("Pattern matching operator".to_string()),
                icon: Some("operator".to_string()),
                metadata: HashMap::new(),
            },
        ]
    }

    fn get_join_suggestions(&self, context: &AutocompleteContext) -> Result<Vec<Suggestion>> {
        let mut suggestions = Vec::new();
        
        suggestions.push(Suggestion {
            text: "INNER JOIN".to_string(),
            display_text: "INNER JOIN".to_string(),
            suggestion_type: SuggestionType::Join,
            relevance_score: 0.9,
            description: Some("Inner join - returns matching rows".to_string()),
            icon: Some("join".to_string()),
            metadata: HashMap::new(),
        });
        
        suggestions.push(Suggestion {
            text: "LEFT JOIN".to_string(),
            display_text: "LEFT JOIN".to_string(),
            suggestion_type: SuggestionType::Join,
            relevance_score: 0.9,
            description: Some("Left join - returns all left rows".to_string()),
            icon: Some("join".to_string()),
            metadata: HashMap::new(),
        });
        
        // Add table suggestions
        suggestions.extend(self.get_schema_suggestions(context)?);
        
        Ok(suggestions)
    }

    fn get_aggregate_functions(&self) -> Vec<Suggestion> {
        vec![
            Suggestion {
                text: "COUNT(*)".to_string(),
                display_text: "COUNT(*) - Count rows".to_string(),
                suggestion_type: SuggestionType::Aggregate,
                relevance_score: 0.95,
                description: Some("Count all rows".to_string()),
                icon: Some("aggregate".to_string()),
                metadata: HashMap::new(),
            },
            Suggestion {
                text: "SUM(".to_string(),
                display_text: "SUM() - Sum values".to_string(),
                suggestion_type: SuggestionType::Aggregate,
                relevance_score: 0.95,
                description: Some("Sum column values".to_string()),
                icon: Some("aggregate".to_string()),
                metadata: HashMap::new(),
            },
            Suggestion {
                text: "AVG(".to_string(),
                display_text: "AVG() - Average values".to_string(),
                suggestion_type: SuggestionType::Aggregate,
                relevance_score: 0.95,
                description: Some("Average column values".to_string()),
                icon: Some("aggregate".to_string()),
                metadata: HashMap::new(),
            },
            Suggestion {
                text: "MAX(".to_string(),
                display_text: "MAX() - Maximum value".to_string(),
                suggestion_type: SuggestionType::Aggregate,
                relevance_score: 0.95,
                description: Some("Maximum column value".to_string()),
                icon: Some("aggregate".to_string()),
                metadata: HashMap::new(),
            },
            Suggestion {
                text: "MIN(".to_string(),
                display_text: "MIN() - Minimum value".to_string(),
                suggestion_type: SuggestionType::Aggregate,
                relevance_score: 0.95,
                description: Some("Minimum column value".to_string()),
                icon: Some("aggregate".to_string()),
                metadata: HashMap::new(),
            },
        ]
    }

    /// Initialize keywords
    fn init_keywords() -> HashSet<String> {
        let mut keywords = HashSet::new();
        keywords.insert("select".to_string());
        keywords.insert("from".to_string());
        keywords.insert("where".to_string());
        keywords.insert("join".to_string());
        keywords.insert("inner".to_string());
        keywords.insert("left".to_string());
        keywords.insert("right".to_string());
        keywords.insert("full".to_string());
        keywords.insert("outer".to_string());
        keywords.insert("on".to_string());
        keywords.insert("group".to_string());
        keywords.insert("by".to_string());
        keywords.insert("order".to_string());
        keywords.insert("having".to_string());
        keywords.insert("limit".to_string());
        keywords.insert("offset".to_string());
        keywords.insert("insert".to_string());
        keywords.insert("into".to_string());
        keywords.insert("values".to_string());
        keywords.insert("update".to_string());
        keywords.insert("set".to_string());
        keywords.insert("delete".to_string());
        keywords.insert("create".to_string());
        keywords.insert("table".to_string());
        keywords.insert("alter".to_string());
        keywords.insert("drop".to_string());
        keywords.insert("as".to_string());
        keywords.insert("and".to_string());
        keywords.insert("or".to_string());
        keywords.insert("not".to_string());
        keywords.insert("in".to_string());
        keywords.insert("like".to_string());
        keywords.insert("between".to_string());
        keywords.insert("is".to_string());
        keywords.insert("null".to_string());
        keywords.insert("distinct".to_string());
        keywords.insert("union".to_string());
        keywords.insert("all".to_string());
        keywords
    }

    /// Initialize functions
    fn init_functions(&self) {
        // Aggregate functions
        self.functions.insert("COUNT".to_string(), FunctionInfo {
            name: "COUNT".to_string(),
            description: "Count rows or non-null values".to_string(),
            parameters: vec![ParameterInfo {
                name: "expr".to_string(),
                parameter_type: DataType::Int64,
                required: false,
                default_value: None,
                description: "Expression to count".to_string(),
            }],
            return_type: DataType::Int64,
            category: FunctionCategory::Aggregate,
            examples: vec!["COUNT(*)".to_string(), "COUNT(column)".to_string()],
            usage_count: 1000,
        });

        self.functions.insert("SUM".to_string(), FunctionInfo {
            name: "SUM".to_string(),
            description: "Sum of values".to_string(),
            parameters: vec![ParameterInfo {
                name: "column".to_string(),
                parameter_type: DataType::Float64,
                required: true,
                default_value: None,
                description: "Column to sum".to_string(),
            }],
            return_type: DataType::Float64,
            category: FunctionCategory::Aggregate,
            examples: vec!["SUM(price)".to_string()],
            usage_count: 800,
        });

        // String functions
        self.functions.insert("UPPER".to_string(), FunctionInfo {
            name: "UPPER".to_string(),
            description: "Convert string to uppercase".to_string(),
            parameters: vec![ParameterInfo {
                name: "str".to_string(),
                parameter_type: DataType::String,
                required: true,
                default_value: None,
                description: "String to convert".to_string(),
            }],
            return_type: DataType::String,
            category: FunctionCategory::String,
            examples: vec!["UPPER(name)".to_string()],
            usage_count: 500,
        });

        // Math functions
        self.functions.insert("ROUND".to_string(), FunctionInfo {
            name: "ROUND".to_string(),
            description: "Round number to specified decimal places".to_string(),
            parameters: vec![
                ParameterInfo {
                    name: "number".to_string(),
                    parameter_type: DataType::Float64,
                    required: true,
                    default_value: None,
                    description: "Number to round".to_string(),
                },
                ParameterInfo {
                    name: "decimals".to_string(),
                    parameter_type: DataType::Int32,
                    required: false,
                    default_value: Some("0".to_string()),
                    description: "Number of decimal places".to_string(),
                },
            ],
            return_type: DataType::Float64,
            category: FunctionCategory::Math,
            examples: vec!["ROUND(price, 2)".to_string()],
            usage_count: 600,
        });
    }

    /// Learn from user input
    pub fn learn_from_input(&self, query: &str, accepted_suggestion: Option<&str>) {
        if !self.config.enable_learning {
            return;
        }

        // Update user patterns
        let pattern_key = Self::extract_pattern(query);
        let mut pattern = self.user_patterns.entry(pattern_key.clone())
            .or_insert_with(|| UserPattern {
                pattern: pattern_key.clone(),
                frequency: 0,
                last_used: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                context: HashMap::new(),
                suggestions: Vec::new(),
            });
        
        pattern.frequency += 1;
        pattern.last_used = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        if let Some(suggestion) = accepted_suggestion {
            if !pattern.suggestions.contains(&suggestion.to_string()) {
                pattern.suggestions.push(suggestion.to_string());
            }
        }

        // Update popular queries
        let mut popular = self.popular_queries.write();
        if let Some(pop_query) = popular.iter_mut().find(|q| q.query == query) {
            pop_query.frequency += 1;
            pop_query.last_used = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        } else {
            popular.push(PopularQuery {
                query: query.to_string(),
                frequency: 1,
                last_used: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                success_rate: 1.0,
            });
            popular.sort_by(|a, b| b.frequency.cmp(&a.frequency));
            popular.truncate(100); // Keep top 100
        }

        // Update statistics
        if let Some(_) = accepted_suggestion {
            self.stats.write().accepted_suggestions += 1;
        }
    }

    /// Register schema
    pub fn register_schema(&self, table_name: String, schema: Schema) {
        self.schemas.insert(table_name, schema);
    }

    /// Helper methods
    fn get_keyword_description(&self, keyword: &str) -> String {
        match keyword.to_lowercase().as_str() {
            "select" => "Select columns from table".to_string(),
            "from" => "Specify table to query".to_string(),
            "where" => "Filter rows".to_string(),
            "join" => "Join tables".to_string(),
            "group" => "Group rows".to_string(),
            "order" => "Sort results".to_string(),
            _ => format!("SQL keyword: {}", keyword),
        }
    }

    fn format_type(&self, data_type: &DataType) -> String {
        format!("{:?}", data_type)
    }

    fn format_parameters(&self, params: &[ParameterInfo]) -> String {
        params.iter()
            .map(|p| {
                if p.required {
                    format!("{}", p.name)
                } else {
                    format!("[{}]", p.name)
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn pattern_matches(query: &str, pattern: &str) -> bool {
        query.contains(pattern) || pattern.contains(query)
    }

    fn fuzzy_match(s1: &str, s2: &str) -> f64 {
        // Simple Levenshtein-based fuzzy matching
        let s1_lower = s1.to_lowercase();
        let s2_lower = s2.to_lowercase();
        
        if s1_lower == s2_lower {
            return 1.0;
        }
        
        if s2_lower.contains(&s1_lower) || s1_lower.contains(&s2_lower) {
            return 0.8;
        }
        
        // Simple character overlap
        let s1_chars: HashSet<char> = s1_lower.chars().collect();
        let s2_chars: HashSet<char> = s2_lower.chars().collect();
        let intersection = s1_chars.intersection(&s2_chars).count();
        let union = s1_chars.union(&s2_chars).count();
        
        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    fn extract_pattern(query: &str) -> String {
        // Extract pattern from query (simplified)
        query.split_whitespace().take(3).collect::<Vec<_>>().join(" ")
    }

    fn get_all_identifiers(&self) -> Vec<String> {
        let mut identifiers = Vec::new();
        
        // Add tables
        for table in self.schemas.iter() {
            identifiers.push(table.key().clone());
        }
        
        // Add columns
        for schema in self.schemas.iter() {
            for field in &schema.value().fields {
                identifiers.push(field.name.clone());
            }
        }
        
        // Add functions
        for func in self.functions.iter() {
            identifiers.push(func.key().clone());
        }
        
        identifiers
    }

    /// Get statistics
    pub fn stats(&self) -> AutocompleteStats {
        self.stats.read().clone()
    }
}

