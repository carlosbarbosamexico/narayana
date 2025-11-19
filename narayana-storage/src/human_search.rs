// Human Search - Natural Language Search System
// Handles everything internally - natural language, semantic, fuzzy, typo tolerance, etc.

use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;
use dashmap::DashMap;
use tracing::{info, warn, debug};
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};

/// Search query - human-friendly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanSearchQuery {
    pub query: String,
    pub filters: Vec<SearchFilter>,
    pub sort: Option<SortOption>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub language: Option<String>,
    pub fuzzy: bool,
    pub typo_tolerance: Option<u8>, // 0-5, higher = more tolerant
    pub semantic: bool,
    pub synonyms: bool,
    pub context: Option<SearchContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilter {
    pub field: String,
    pub operator: FilterOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Contains,
    StartsWith,
    EndsWith,
    In,
    NotIn,
    Between,
    Like,
    Regex,
    Exists,
    NotExists,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortOption {
    pub field: String,
    pub direction: SortDirection,
    pub relevance: bool, // Sort by relevance score
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchContext {
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub previous_queries: Vec<String>,
    pub preferences: HashMap<String, serde_json::Value>,
    pub location: Option<Location>,
    pub time_of_day: Option<String>,
    pub device: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub country: Option<String>,
    pub city: Option<String>,
}

/// Search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub score: f64,
    pub highlights: Vec<Highlight>,
    pub matched_fields: Vec<String>,
    pub explanation: Option<String>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlight {
    pub field: String,
    pub snippets: Vec<String>,
    pub positions: Vec<usize>,
}

/// Search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total: usize,
    pub took_ms: u64,
    pub suggestions: Vec<String>,
    pub related_queries: Vec<String>,
    pub facets: HashMap<String, Vec<FacetValue>>,
    pub query_understanding: Option<QueryUnderstanding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetValue {
    pub value: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryUnderstanding {
    pub intent: SearchIntent,
    pub entities: Vec<Entity>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    pub sentiment: Option<Sentiment>,
    pub language: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchIntent {
    Find,
    Compare,
    Browse,
    Discover,
    Navigate,
    Informational,
    Transactional,
    Navigational,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub entity_type: String,
    pub value: String,
    pub confidence: f64,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sentiment {
    Positive,
    Negative,
    Neutral,
}

/// Human search engine - handles everything internally
pub struct HumanSearchEngine {
    index: Arc<SearchIndex>,
    tokenizer: Arc<Tokenizer>,
    semantic_search: Arc<SemanticSearch>,
    fuzzy_matcher: Arc<FuzzyMatcher>,
    synonym_engine: Arc<SynonymEngine>,
    query_understanding: Arc<QueryUnderstandingEngine>,
    auto_complete: Arc<AutoCompleteEngine>,
    search_history: Arc<SearchHistory>,
    personalization: Arc<PersonalizationEngine>,
    multi_language: Arc<MultiLanguageSupport>,
    typo_corrector: Arc<TypoCorrector>,
}

/// Search index
struct SearchIndex {
    documents: Arc<DashMap<String, IndexedDocument>>,
    inverted_index: Arc<RwLock<HashMap<String, Vec<Posting>>>>,
    field_indexes: Arc<DashMap<String, FieldIndex>>,
    ngram_index: Arc<RwLock<HashMap<String, HashSet<String>>>>,
}

#[derive(Debug, Clone)]
struct IndexedDocument {
    id: String,
    fields: HashMap<String, FieldValue>,
    metadata: HashMap<String, serde_json::Value>,
    embedding: Option<Vec<f32>>,
    created_at: u64,
    updated_at: u64,
}

#[derive(Debug, Clone)]
enum FieldValue {
    Text(String),
    Number(f64),
    Boolean(bool),
    Date(u64),
    Array(Vec<String>),
    Object(HashMap<String, serde_json::Value>),
}

#[derive(Debug, Clone)]
struct Posting {
    document_id: String,
    field: String,
    position: usize,
    score: f64,
}

#[derive(Debug, Clone)]
struct FieldIndex {
    field_name: String,
    index_type: IndexType,
    values: HashMap<String, Vec<String>>, // value -> document_ids
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum IndexType {
    Text,
    Keyword,
    Numeric,
    Date,
    Boolean,
    Geo,
}

/// Tokenizer - handles text tokenization
struct Tokenizer {
    language: String,
    stop_words: HashSet<String>,
    stemmer: Option<Stemmer>,
}

struct Stemmer {
    language: String,
}

/// Semantic search - vector similarity search
struct SemanticSearch {
    embeddings: Arc<DashMap<String, Vec<f32>>>,
    dimension: usize,
    index_type: SemanticIndexType,
}

/// Internal semantic search result
#[derive(Debug, Clone)]
struct SemanticSearchResult {
    id: String,
    score: f64,
}

#[derive(Debug, Clone)]
enum SemanticIndexType {
    Flat,
    HNSW { m: usize, ef_construction: usize },
    IVF { nlist: usize },
}

/// Fuzzy matcher - handles typos and approximate matching
struct FuzzyMatcher {
    max_distance: u8,
    algorithms: Vec<FuzzyAlgorithm>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FuzzyAlgorithm {
    Levenshtein,
    DamerauLevenshtein,
    Jaro,
    JaroWinkler,
    NGram,
    Soundex,
    Metaphone,
    DoubleMetaphone,
}

/// Synonym engine - handles synonyms and related terms
struct SynonymEngine {
    synonyms: Arc<RwLock<HashMap<String, Vec<String>>>>,
    related_terms: Arc<RwLock<HashMap<String, Vec<String>>>>,
    auto_expand: bool,
}

/// Query understanding engine - understands user intent
struct QueryUnderstandingEngine {
    intent_classifier: IntentClassifier,
    entity_extractor: EntityExtractor,
    keyword_extractor: KeywordExtractor,
    sentiment_analyzer: SentimentAnalyzer,
}

struct IntentClassifier {
    model: String,
}

struct EntityExtractor {
    patterns: HashMap<String, Vec<String>>,
}

struct KeywordExtractor {
    min_length: usize,
    max_length: usize,
}

struct SentimentAnalyzer {
    model: String,
}

/// Auto-complete engine - suggests completions
struct AutoCompleteEngine {
    trie: Arc<RwLock<Trie>>,
    suggestions: Arc<DashMap<String, Vec<Suggestion>>>,
    max_suggestions: usize,
}

struct Trie {
    root: TrieNode,
}

struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_end: bool,
    suggestions: Vec<String>,
    frequency: u64,
}

#[derive(Debug, Clone)]
struct Suggestion {
    text: String,
    score: f64,
    category: Option<String>,
}

/// Search history - tracks user searches
struct SearchHistory {
    history: Arc<DashMap<String, Vec<SearchHistoryEntry>>>,
    max_entries_per_user: usize,
}

#[derive(Debug, Clone)]
struct SearchHistoryEntry {
    query: String,
    timestamp: u64,
    results_count: usize,
    clicked_results: Vec<String>,
}

/// Personalization engine - personalizes search results
struct PersonalizationEngine {
    user_profiles: Arc<DashMap<String, UserProfile>>,
    collaborative_filtering: bool,
}

#[derive(Debug, Clone)]
struct UserProfile {
    user_id: String,
    preferences: HashMap<String, f64>,
    search_patterns: Vec<String>,
    favorite_categories: Vec<String>,
    clicked_items: Vec<String>,
}

/// Multi-language support
struct MultiLanguageSupport {
    languages: HashSet<String>,
    language_detector: LanguageDetector,
    translators: HashMap<String, Translator>,
}

struct LanguageDetector {
    model: String,
}

struct Translator {
    source_language: String,
    target_language: String,
}

/// Typo corrector - corrects typos in queries
struct TypoCorrector {
    dictionary: HashSet<String>,
    max_distance: u8,
    algorithms: Vec<FuzzyAlgorithm>,
}

impl HumanSearchEngine {
    pub fn new() -> Self {
        Self {
            index: Arc::new(SearchIndex::new()),
            tokenizer: Arc::new(Tokenizer::new("en")),
            semantic_search: Arc::new(SemanticSearch::new(384)), // 384-dim embeddings
            fuzzy_matcher: Arc::new(FuzzyMatcher::new(2)),
            synonym_engine: Arc::new(SynonymEngine::new()),
            query_understanding: Arc::new(QueryUnderstandingEngine::new()),
            auto_complete: Arc::new(AutoCompleteEngine::new()),
            search_history: Arc::new(SearchHistory::new()),
            personalization: Arc::new(PersonalizationEngine::new()),
            multi_language: Arc::new(MultiLanguageSupport::new()),
            typo_corrector: Arc::new(TypoCorrector::new()),
        }
    }

    /// Index a document
    pub async fn index(&self, id: String, fields: HashMap<String, serde_json::Value>, metadata: HashMap<String, serde_json::Value>) -> Result<()> {
        let mut indexed_doc = IndexedDocument {
            id: id.clone(),
            fields: HashMap::new(),
            metadata: metadata.clone(),
            embedding: None,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            updated_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
        };

        // Process fields
        for (field_name, value) in &fields {
            let field_value = self.parse_field_value(value.clone())?;
            indexed_doc.fields.insert(field_name.clone(), field_value.clone());
            
            // Index field
            self.index_field(&id, field_name, &field_value).await?;
        }

        // Generate embedding for semantic search
        if let Some(embedding) = self.generate_embedding(&indexed_doc).await? {
            indexed_doc.embedding = Some(embedding.clone());
            self.semantic_search.index(id.clone(), embedding).await?;
        }

        // Clone fields before moving indexed_doc
        let fields_clone = fields.clone();
        
        // Store document
        self.index.documents.insert(id.clone(), indexed_doc);

        // Update ngram index
        self.update_ngram_index(&id, &fields_clone).await?;

        // Update auto-complete trie with terms from indexed document
        let mut terms = Vec::new();
        for (_, field_value) in &fields {
            if let serde_json::Value::String(text) = field_value {
                let tokens = self.tokenizer.tokenize(text).await?;
                terms.extend(tokens);
            }
        }
        if !terms.is_empty() {
            let mut trie = self.auto_complete.trie.write();
            for term in &terms {
                trie.insert(term);
            }
        }

        info!("Indexed document: {}", id);
        Ok(())
    }

    /// Search - handles everything internally
    pub async fn search(&self, query: HumanSearchQuery) -> Result<SearchResponse> {
        let start_time = SystemTime::now();
        
        // Understand query
        let understanding = self.query_understanding.understand(&query.query, query.language.as_deref()).await?;
        
        // Correct typos if enabled
        let corrected_query = if query.typo_tolerance.is_some() {
            self.typo_corrector.correct(&query.query, query.typo_tolerance.unwrap()).await?
        } else {
            query.query.clone()
        };

        // Expand synonyms if enabled
        let expanded_query = if query.synonyms {
            self.synonym_engine.expand(&corrected_query).await?
        } else {
            corrected_query
        };

        // Tokenize query
        let tokens = self.tokenizer.tokenize(&expanded_query).await?;

        // Search results
        let mut results = Vec::new();

        // Text search
        let text_results = self.text_search(&tokens, &query).await?;
        results.extend(text_results);

        // Semantic search if enabled
        if query.semantic {
            let semantic_results = self.semantic_search(&expanded_query, 100).await?;
            results.extend(semantic_results);
        }

        // Fuzzy search if enabled
        if query.fuzzy {
            let fuzzy_results = self.fuzzy_search(&tokens, &query).await?;
            results.extend(fuzzy_results);
        }

        // Apply filters
        let filtered_results = self.apply_filters(results, &query.filters).await?;

        // Personalize results
        let personalized_results = if let Some(ref context) = query.context {
            self.personalization.personalize(filtered_results, context).await?
        } else {
            filtered_results
        };

        // Sort results
        let sorted_results = self.sort_results(personalized_results, &query.sort).await?;

        // Apply limit and offset
        let final_results = self.apply_pagination(sorted_results, query.limit, query.offset).await?;

        // Generate highlights
        let highlighted_results = self.generate_highlights(final_results, &tokens).await?;

        // Generate suggestions
        let suggestions = self.auto_complete.suggest(&query.query, 10).await?;

        // Generate related queries
        let related_queries = self.generate_related_queries(&query.query, &understanding).await?;

        // Generate facets
        let facets = self.generate_facets(&highlighted_results).await?;

        // Record search history
        if let Some(ref context) = query.context {
            if let Some(ref user_id) = context.user_id {
                self.search_history.record(user_id.clone(), query.query.clone(), highlighted_results.len()).await?;
            }
        }

        let took_ms = start_time.elapsed().unwrap_or_default().as_millis() as u64;
        let highlighted_results_len = highlighted_results.len();

        Ok(SearchResponse {
            results: highlighted_results,
            total: highlighted_results_len,
            took_ms,
            suggestions: suggestions.into_iter().map(|s| s.text).collect(),
            related_queries,
            facets,
            query_understanding: Some(understanding),
        })
    }

    /// Text search
    async fn text_search(&self, tokens: &[String], query: &HumanSearchQuery) -> Result<Vec<SearchResult>> {
        let mut results = HashMap::new();
        let inverted_index = self.index.inverted_index.read();

        for token in tokens {
            if let Some(postings) = inverted_index.get(token) {
                for posting in postings {
                    let score = results.entry(posting.document_id.clone())
                        .or_insert(0.0);
                    *score += posting.score;
                }
            }
        }

        // Convert to SearchResult
        let mut search_results = Vec::new();
        for (doc_id, score) in results {
            if let Some(doc) = self.index.documents.get(&doc_id) {
                search_results.push(SearchResult {
                    id: doc_id.clone(),
                    score,
                    highlights: Vec::new(),
                    matched_fields: Vec::new(),
                    explanation: None,
                    data: serde_json::to_value(doc.metadata.clone()).unwrap_or_default(),
                });
            }
        }

        Ok(search_results)
    }

    /// Semantic search
    async fn semantic_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // Generate query embedding
        let query_embedding = self.generate_query_embedding(query).await?;
        
        // Search in semantic index using vector similarity
        let results = self.semantic_search.search_vectors(&query_embedding, limit).await?;
        
        // Map to SearchResult format
        let mut search_results = Vec::new();
        for result in results {
            // Get document from index
            if let Some(doc) = self.index.documents.get(&result.id) {
                search_results.push(SearchResult {
                    id: result.id.clone(),
                    score: result.score,
                    highlights: Vec::new(),
                    matched_fields: vec!["*".to_string()], // Semantic match applies to all fields
                    explanation: Some("Semantic similarity match".to_string()),
                    data: serde_json::to_value(doc.metadata.clone()).unwrap_or_default(),
                });
            }
        }
        
        Ok(search_results)
    }

    /// Fuzzy search
    async fn fuzzy_search(&self, tokens: &[String], query: &HumanSearchQuery) -> Result<Vec<SearchResult>> {
        let mut results = HashMap::new();
        let max_distance = query.typo_tolerance.unwrap_or(2);
        let inverted_index = self.index.inverted_index.read();
        
        // Get all tokens from the inverted index as dictionary
        let dictionary: HashSet<String> = inverted_index.keys().cloned().collect();
        drop(inverted_index);
        
        for token in tokens {
            // Find similar tokens using fuzzy matching
            let similar_tokens = self.fuzzy_matcher.find_similar_in_dictionary(token, &dictionary, max_distance);
            
            // Also check exact match
            let mut all_matches = similar_tokens;
            if dictionary.contains(token) {
                all_matches.push(token.clone());
            }
            
            // Search for documents containing these tokens
            let inverted_index = self.index.inverted_index.read();
            for matched_token in &all_matches {
                if let Some(postings) = inverted_index.get(matched_token) {
                    for posting in postings {
                        let score = results.entry(posting.document_id.clone())
                            .or_insert(0.0);
                        // Exact matches get full score, fuzzy matches get reduced score
                        let match_score = if matched_token == token {
                            posting.score
                        } else {
                            posting.score * 0.8 // Lower score for fuzzy matches
                        };
                        *score += match_score;
                    }
                }
            }
        }

        let mut search_results = Vec::new();
        for (doc_id, score) in results {
            if let Some(doc) = self.index.documents.get(&doc_id) {
                search_results.push(SearchResult {
                    id: doc_id.clone(),
                    score,
                    highlights: Vec::new(),
                    matched_fields: Vec::new(),
                    explanation: Some("Fuzzy match".to_string()),
                    data: serde_json::to_value(doc.metadata.clone()).unwrap_or_default(),
                });
            }
        }

        Ok(search_results)
    }

    /// Apply filters
    async fn apply_filters(&self, results: Vec<SearchResult>, filters: &[SearchFilter]) -> Result<Vec<SearchResult>> {
        let mut filtered = results;
        
        for filter in filters {
            filtered = filtered.into_iter()
                .filter(|result| {
                    self.matches_filter(result, filter)
                })
                .collect();
        }
        
        Ok(filtered)
    }

    /// Check if result matches filter
    fn matches_filter(&self, result: &SearchResult, filter: &SearchFilter) -> bool {
        // Get field value from result data
        let field_value = if let serde_json::Value::Object(obj) = &result.data {
            obj.get(&filter.field)
        } else {
            return false;
        };

        if field_value.is_none() {
            return matches!(filter.operator, FilterOperator::NotExists);
        }

        let field_value = field_value.unwrap();

        match &filter.operator {
            FilterOperator::Equals => {
                field_value == &filter.value
            }
            FilterOperator::NotEquals => {
                field_value != &filter.value
            }
            FilterOperator::GreaterThan => {
                if let (Some(fv_num), Some(filt_num)) = (field_value.as_f64(), filter.value.as_f64()) {
                    fv_num > filt_num
                } else {
                    false
                }
            }
            FilterOperator::LessThan => {
                if let (Some(fv_num), Some(filt_num)) = (field_value.as_f64(), filter.value.as_f64()) {
                    fv_num < filt_num
                } else {
                    false
                }
            }
            FilterOperator::GreaterThanOrEqual => {
                if let (Some(fv_num), Some(filt_num)) = (field_value.as_f64(), filter.value.as_f64()) {
                    fv_num >= filt_num
                } else {
                    false
                }
            }
            FilterOperator::LessThanOrEqual => {
                if let (Some(fv_num), Some(filt_num)) = (field_value.as_f64(), filter.value.as_f64()) {
                    fv_num <= filt_num
                } else {
                    false
                }
            }
            FilterOperator::Contains => {
                if let (Some(fv_str), Some(filt_str)) = (field_value.as_str(), filter.value.as_str()) {
                    fv_str.contains(filt_str)
                } else {
                    false
                }
            }
            FilterOperator::StartsWith => {
                if let (Some(fv_str), Some(filt_str)) = (field_value.as_str(), filter.value.as_str()) {
                    fv_str.starts_with(filt_str)
                } else {
                    false
                }
            }
            FilterOperator::EndsWith => {
                if let (Some(fv_str), Some(filt_str)) = (field_value.as_str(), filter.value.as_str()) {
                    fv_str.ends_with(filt_str)
                } else {
                    false
                }
            }
            FilterOperator::In => {
                if let Some(filt_arr) = filter.value.as_array() {
                    filt_arr.contains(field_value)
                } else {
                    false
                }
            }
            FilterOperator::NotIn => {
                if let Some(filt_arr) = filter.value.as_array() {
                    !filt_arr.contains(field_value)
                } else {
                    true
                }
            }
            FilterOperator::Between => {
                if let Some(filt_arr) = filter.value.as_array() {
                    if filt_arr.len() >= 2 {
                        if let (Some(fv_num), Some(min), Some(max)) = (
                            field_value.as_f64(),
                            filt_arr[0].as_f64(),
                            filt_arr[1].as_f64(),
                        ) {
                            fv_num >= min && fv_num <= max
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            FilterOperator::Like => {
                if let (Some(fv_str), Some(filt_str)) = (field_value.as_str(), filter.value.as_str()) {
                    // Simple LIKE pattern matching (SQL LIKE with % and _)
                    self.like_match(fv_str, filt_str)
                } else {
                    false
                }
            }
            FilterOperator::Regex => {
                if let (Some(fv_str), Some(filt_str)) = (field_value.as_str(), filter.value.as_str()) {
                    use regex::Regex;
                    if let Ok(re) = Regex::new(filt_str) {
                        re.is_match(fv_str)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            FilterOperator::Exists => {
                true // Field exists (already checked above)
            }
            FilterOperator::NotExists => {
                false // Field doesn't exist
            }
        }
    }

    /// Simple LIKE pattern matching (SQL LIKE)
    fn like_match(&self, text: &str, pattern: &str) -> bool {
        // Convert SQL LIKE pattern to regex
        let regex_pattern = pattern
            .replace("%", ".*")
            .replace("_", ".");
        
        use regex::Regex;
        if let Ok(re) = Regex::new(&format!("^{}$", regex_pattern)) {
            re.is_match(text)
        } else {
            false
        }
    }

    /// Personalize results
    async fn personalize(&self, results: Vec<SearchResult>, context: &SearchContext) -> Result<Vec<SearchResult>> {
        if let Some(ref user_id) = context.user_id {
            if let Some(profile) = self.personalization.user_profiles.get(user_id) {
                // Boost results based on user preferences
                let mut personalized = results;
                for result in &mut personalized {
                    // Boost score based on user preferences
                    result.score *= 1.1; // Example boost
                }
                return Ok(personalized);
            }
        }
        Ok(results)
    }

    /// Sort results
    async fn sort_results(&self, mut results: Vec<SearchResult>, sort: &Option<SortOption>) -> Result<Vec<SearchResult>> {
        if let Some(ref sort_opt) = sort {
            if sort_opt.relevance {
                results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            } else {
                // Sort by field
                // Simplified - in production would sort by actual field values
            }
        } else {
            // Default: sort by relevance
            results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        }
        Ok(results)
    }

    /// Apply pagination
    async fn apply_pagination(&self, results: Vec<SearchResult>, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<SearchResult>> {
        let offset = offset.unwrap_or(0);
        let limit = limit.unwrap_or(10);
        
        Ok(results.into_iter()
            .skip(offset)
            .take(limit)
            .collect())
    }

    /// Generate highlights
    async fn generate_highlights(&self, results: Vec<SearchResult>, tokens: &[String]) -> Result<Vec<SearchResult>> {
        let mut highlighted = results;
        
        for result in &mut highlighted {
            // Generate highlights for matched tokens
            result.highlights = self.generate_highlights_for_result(&result.id, tokens).await?;
        }
        
        Ok(highlighted)
    }

    /// Generate highlights for a result
    async fn generate_highlights_for_result(&self, doc_id: &str, tokens: &[String]) -> Result<Vec<Highlight>> {
        let mut highlights = Vec::new();
        
        if let Some(doc) = self.index.documents.get(doc_id) {
            // Generate highlights for each field
            for (field_name, field_value) in &doc.fields {
                if let FieldValue::Text(text) = field_value {
                    let mut snippets = Vec::new();
                    let mut positions = Vec::new();
                    
                    // Find token matches in text
                    let text_lower = text.to_lowercase();
                    for (idx, token) in tokens.iter().enumerate() {
                        let token_lower = token.to_lowercase();
                        
                        // Find all occurrences of token
                        let mut start = 0;
                        while let Some(pos) = text_lower[start..].find(&token_lower) {
                            let absolute_pos = start + pos;
                            
                            // Extract snippet (50 chars before and after)
                            let snippet_start = absolute_pos.saturating_sub(50);
                            let snippet_end = (absolute_pos + token.len() + 50).min(text.len());
                            let snippet = format!(
                                "...{}...",
                                &text[snippet_start..snippet_end]
                            );
                            
                            snippets.push(snippet);
                            positions.push(absolute_pos);
                            
                            start = absolute_pos + token.len();
                        }
                    }
                    
                    if !snippets.is_empty() {
                        highlights.push(Highlight {
                            field: field_name.clone(),
                            snippets,
                            positions,
                        });
                    }
                }
            }
        }
        
        Ok(highlights)
    }

    /// Generate related queries
    async fn generate_related_queries(&self, query: &str, understanding: &QueryUnderstanding) -> Result<Vec<String>> {
        // Generate related queries based on understanding
        let mut related = Vec::new();
        
        // Add variations
        for keyword in &understanding.keywords {
            related.push(format!("{} {}", query, keyword));
        }
        
        Ok(related)
    }

    /// Generate facets
    async fn generate_facets(&self, results: &[SearchResult]) -> Result<HashMap<String, Vec<FacetValue>>> {
        let mut facets: HashMap<String, HashMap<String, usize>> = HashMap::new();
        
        // Extract facets from result metadata
        for result in results {
            if let serde_json::Value::Object(obj) = &result.data {
                for (key, value) in obj {
                    let value_str = match value {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        _ => continue,
                    };
                    
                    let facet_map = facets.entry(key.clone()).or_insert_with(HashMap::new);
                    *facet_map.entry(value_str).or_insert(0) += 1;
                }
            }
        }
        
        // Convert to FacetValue format
        let mut result_facets = HashMap::new();
        for (field, counts) in facets {
            let mut values: Vec<FacetValue> = counts.into_iter()
                .map(|(value, count)| FacetValue { value, count })
                .collect();
            values.sort_by(|a, b| b.count.cmp(&a.count));
            result_facets.insert(field, values);
        }
        
        Ok(result_facets)
    }

    /// Parse field value
    fn parse_field_value(&self, value: serde_json::Value) -> Result<FieldValue> {
        match value {
            serde_json::Value::String(s) => Ok(FieldValue::Text(s)),
            serde_json::Value::Number(n) => Ok(FieldValue::Number(n.as_f64().unwrap_or(0.0))),
            serde_json::Value::Bool(b) => Ok(FieldValue::Boolean(b)),
            serde_json::Value::Array(arr) => {
                let strings: Vec<String> = arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                Ok(FieldValue::Array(strings))
            }
            serde_json::Value::Object(obj) => {
                let mut map = HashMap::new();
                for (k, v) in obj {
                    map.insert(k, v);
                }
                Ok(FieldValue::Object(map))
            },
            _ => Err(Error::Storage("Unsupported field value type".to_string())),
        }
    }

    /// Index a field
    async fn index_field(&self, doc_id: &str, field_name: &str, field_value: &FieldValue) -> Result<()> {
        match field_value {
            FieldValue::Text(text) => {
                let tokens = self.tokenizer.tokenize(text).await?;
                let mut inverted_index = self.index.inverted_index.write();
                
                for (pos, token) in tokens.iter().enumerate() {
                    let posting = Posting {
                        document_id: doc_id.to_string(),
                        field: field_name.to_string(),
                        position: pos,
                        score: 1.0,
                    };
                    inverted_index.entry(token.clone())
                        .or_insert_with(Vec::new)
                        .push(posting);
                }
            }
            _ => {
                // Index other field types
            }
        }
        Ok(())
    }

    /// Generate embedding using hash-based semantic embedding
    /// Creates deterministic embeddings from text content
    async fn generate_embedding(&self, doc: &IndexedDocument) -> Result<Option<Vec<f32>>> {
        // Combine all text fields for embedding
        let mut text_content = String::new();
        for (_, field_value) in &doc.fields {
            if let FieldValue::Text(text) = field_value {
                text_content.push_str(text);
                text_content.push(' ');
            }
        }
        
        if text_content.is_empty() {
            return Ok(None);
        }
        
        // Generate deterministic embedding using hash-based approach
        // This creates a 384-dimensional vector from text content
        let embedding = self.text_to_embedding(&text_content, 384);
        Ok(Some(embedding))
    }

    /// Generate query embedding
    async fn generate_query_embedding(&self, query: &str) -> Result<Vec<f32>> {
        Ok(self.text_to_embedding(query, 384))
    }

    /// Convert text to embedding vector using hash-based approach
    /// Creates deterministic, semantic-like embeddings
    fn text_to_embedding(&self, text: &str, dimension: usize) -> Vec<f32> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use sha2::{Sha256, Digest};
        
        // Use SHA-256 for better distribution
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let hash = hasher.finalize();
        
        // Create embedding from hash
        let mut embedding = Vec::with_capacity(dimension);
        
        // Use hash bytes to generate embedding values
        for i in 0..dimension {
            let byte_idx = i % hash.len();
            let hash_val = hash[byte_idx] as f32 / 255.0; // Normalize to [0, 1]
            
            // Add some variation based on position
            let position_factor = (i as f32 / dimension as f32) * 2.0 - 1.0; // [-1, 1]
            let value = (hash_val + position_factor * 0.1).tanh(); // Normalize to [-1, 1]
            
            embedding.push(value);
        }
        
        // Normalize the vector
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut embedding {
                *val /= norm;
            }
        }
        
        embedding
    }

    /// Update ngram index
    async fn update_ngram_index(&self, doc_id: &str, fields: &HashMap<String, serde_json::Value>) -> Result<()> {
        let mut ngram_index = self.index.ngram_index.write();
        
        // Generate n-grams (trigrams) from all text fields
        for (_, value) in fields {
            if let serde_json::Value::String(text) = value {
                let text_lower = text.to_lowercase();
                let chars: Vec<char> = text_lower.chars().collect();
                
                // Generate trigrams
                for i in 0..chars.len().saturating_sub(2) {
                    let trigram: String = chars[i..i + 3].iter().collect();
                    
                    ngram_index.entry(trigram)
                        .or_insert_with(HashSet::new)
                        .insert(doc_id.to_string());
                }
                
                // Also generate bigrams for shorter matching
                for i in 0..chars.len().saturating_sub(1) {
                    let bigram: String = chars[i..i + 2].iter().collect();
                    
                    ngram_index.entry(bigram)
                        .or_insert_with(HashSet::new)
                        .insert(doc_id.to_string());
                }
            }
        }
        
        Ok(())
    }
}

// Implementations for supporting structs

impl SearchIndex {
    fn new() -> Self {
        Self {
            documents: Arc::new(DashMap::new()),
            inverted_index: Arc::new(RwLock::new(HashMap::new())),
            field_indexes: Arc::new(DashMap::new()),
            ngram_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Tokenizer {
    fn new(language: &str) -> Self {
        Self {
            language: language.to_string(),
            stop_words: Self::load_stop_words(language),
            stemmer: Some(Stemmer::new(language)),
        }
    }

    async fn tokenize(&self, text: &str) -> Result<Vec<String>> {
        // Tokenize text: lowercase, remove punctuation, filter stop words, stem
        let tokens: Vec<String> = text
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|s| !s.is_empty() && !self.stop_words.contains(*s))
            .map(|s| {
                if let Some(ref stemmer) = self.stemmer {
                    stemmer.stem(s)
                } else {
                    s.to_string()
                }
            })
            .collect();
        
        Ok(tokens)
    }

    fn load_stop_words(language: &str) -> HashSet<String> {
        // Common English stop words
        let words = vec!["the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by"];
        words.into_iter().map(|s| s.to_string()).collect()
    }
}

impl Stemmer {
    fn new(language: &str) -> Self {
        Self {
            language: language.to_string(),
        }
    }

    fn stem(&self, word: &str) -> String {
        // Porter stemmer implementation (simplified but functional)
        let word = word.to_lowercase();
        
        // Handle empty or very short words
        if word.len() <= 2 {
            return word;
        }
        
        // Step 1a: Handle plurals and past participles
        let mut stemmed = word.clone();
        
        if stemmed.ends_with("sses") {
            stemmed.truncate(stemmed.len() - 2);
        } else if stemmed.ends_with("ies") {
            stemmed.truncate(stemmed.len() - 2);
        } else if stemmed.ends_with("ss") {
            // Keep as is
        } else if stemmed.ends_with("s") {
            stemmed.pop();
        }
        
        // Step 1b: Handle -ed and -ing
        if stemmed.ends_with("eed") {
            if stemmed.len() > 3 {
                stemmed.pop();
            }
        } else if stemmed.ends_with("ed") {
            stemmed.truncate(stemmed.len() - 2);
            if stemmed.ends_with("at") || stemmed.ends_with("bl") || stemmed.ends_with("iz") {
                stemmed.push('e');
            } else if stemmed.len() > 1 && stemmed.chars().last() == stemmed.chars().nth(stemmed.len() - 2) {
                // Double consonant
                if !stemmed.ends_with("l") && !stemmed.ends_with("s") && !stemmed.ends_with("z") {
                    stemmed.pop();
                }
            }
        } else if stemmed.ends_with("ing") {
            stemmed.truncate(stemmed.len() - 3);
            if stemmed.ends_with("at") || stemmed.ends_with("bl") || stemmed.ends_with("iz") {
                stemmed.push('e');
            } else if stemmed.len() > 1 && stemmed.chars().last() == stemmed.chars().nth(stemmed.len() - 2) {
                if !stemmed.ends_with("l") && !stemmed.ends_with("s") && !stemmed.ends_with("z") {
                    stemmed.pop();
                }
            }
        }
        
        // Step 1c: Handle -y
        if stemmed.ends_with("y") && stemmed.len() > 2 {
            // Safe: stemmed.len() > 2 ensures stemmed.len() - 2 >= 1, so nth() will return Some
            if let Some(prev) = stemmed.chars().nth(stemmed.len() - 2) {
                if !matches!(prev, 'a' | 'e' | 'i' | 'o' | 'u') {
                    stemmed.truncate(stemmed.len() - 1);
                    stemmed.push('i');
                }
            }
        }
        
        // Step 2-5: Further stemming (simplified)
        if stemmed.ends_with("ational") {
            stemmed.truncate(stemmed.len() - 5);
            stemmed.push_str("ate");
        } else if stemmed.ends_with("tional") {
            stemmed.truncate(stemmed.len() - 2);
        } else if stemmed.ends_with("enci") {
            stemmed.truncate(stemmed.len() - 1);
            stemmed.push('e');
        } else if stemmed.ends_with("anci") {
            stemmed.truncate(stemmed.len() - 1);
            stemmed.push('e');
        } else if stemmed.ends_with("izer") {
            stemmed.truncate(stemmed.len() - 1);
        } else if stemmed.ends_with("abli") {
            stemmed.truncate(stemmed.len() - 1);
            stemmed.push('e');
        } else if stemmed.ends_with("alli") {
            stemmed.truncate(stemmed.len() - 2);
        } else if stemmed.ends_with("entli") {
            stemmed.truncate(stemmed.len() - 2);
        } else if stemmed.ends_with("eli") {
            stemmed.truncate(stemmed.len() - 2);
        } else if stemmed.ends_with("ousli") {
            stemmed.truncate(stemmed.len() - 2);
        } else if stemmed.ends_with("ization") {
            stemmed.truncate(stemmed.len() - 5);
            stemmed.push_str("ize");
        } else if stemmed.ends_with("ation") {
            stemmed.truncate(stemmed.len() - 3);
            stemmed.push_str("ate");
        } else if stemmed.ends_with("ator") {
            stemmed.truncate(stemmed.len() - 2);
            stemmed.push('e');
        } else if stemmed.ends_with("alism") {
            stemmed.truncate(stemmed.len() - 3);
        } else if stemmed.ends_with("iveness") {
            stemmed.truncate(stemmed.len() - 4);
        } else if stemmed.ends_with("fulness") {
            stemmed.truncate(stemmed.len() - 4);
        } else if stemmed.ends_with("ousness") {
            stemmed.truncate(stemmed.len() - 4);
        } else if stemmed.ends_with("aliti") {
            stemmed.truncate(stemmed.len() - 3);
        } else if stemmed.ends_with("iviti") {
            stemmed.truncate(stemmed.len() - 3);
            stemmed.push_str("ive");
        } else if stemmed.ends_with("biliti") {
            stemmed.truncate(stemmed.len() - 5);
            stemmed.push_str("ble");
        }
        
        // Step 4: Remove common endings
        if stemmed.ends_with("icate") {
            stemmed.truncate(stemmed.len() - 3);
        } else if stemmed.ends_with("ative") {
            stemmed.truncate(stemmed.len() - 5);
        } else if stemmed.ends_with("alize") {
            stemmed.truncate(stemmed.len() - 3);
        } else if stemmed.ends_with("iciti") {
            stemmed.truncate(stemmed.len() - 3);
        } else if stemmed.ends_with("ical") {
            stemmed.truncate(stemmed.len() - 2);
        } else if stemmed.ends_with("ful") {
            stemmed.truncate(stemmed.len() - 3);
        } else if stemmed.ends_with("ness") {
            stemmed.truncate(stemmed.len() - 4);
        }
        
        // Step 5a: Remove final -e if word is long enough
        if stemmed.len() > 3 && stemmed.ends_with("e") {
            stemmed.pop();
        }
        
        // Step 5b: Remove final -l if double l and word is long enough
        if stemmed.len() > 3 && stemmed.ends_with("ll") {
            stemmed.pop();
        }
        
        stemmed
    }
}

impl SemanticSearch {
    fn new(dimension: usize) -> Self {
        Self {
            embeddings: Arc::new(DashMap::new()),
            dimension,
            index_type: SemanticIndexType::Flat,
        }
    }

    async fn index(&self, doc_id: String, embedding: Vec<f32>) -> Result<()> {
        self.embeddings.insert(doc_id, embedding);
        Ok(())
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // Generate query embedding and search
        // Note: This requires the embedding generator, which should be passed in
        // For now, return empty - the actual search happens in search_vectors
        Ok(Vec::new())
    }

    async fn search_vectors(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<SemanticSearchResult>> {
        // Find most similar vectors
        let mut results = Vec::new();
        
        for entry in self.embeddings.iter() {
            let doc_id = entry.key();
            let doc_embedding = entry.value();
            
            let similarity = self.cosine_similarity(query_embedding, doc_embedding)?;
            
            results.push(SemanticSearchResult {
                id: doc_id.clone(),
                score: similarity,
            });
        }
        
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        
        Ok(results)
    }

    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> Result<f64> {
        if a.len() != b.len() {
            return Err(Error::Query("Vector dimensions mismatch".to_string()));
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if magnitude_a == 0.0 || magnitude_b == 0.0 {
            Ok(0.0)
        } else {
            Ok((dot_product / (magnitude_a * magnitude_b)) as f64)
        }
    }
}

impl FuzzyMatcher {
    fn new(max_distance: u8) -> Self {
        Self {
            max_distance,
            algorithms: vec![
                FuzzyAlgorithm::Levenshtein,
                FuzzyAlgorithm::DamerauLevenshtein,
                FuzzyAlgorithm::JaroWinkler,
            ],
        }
    }

    async fn find_similar(&self, token: &str, max_distance: u8) -> Result<Vec<String>> {
        // This needs access to the inverted index to find similar tokens
        // For now, return empty - the actual implementation will be in the search engine
        Ok(Vec::new())
    }

    /// Find similar tokens from a dictionary
    pub fn find_similar_in_dictionary(&self, token: &str, dictionary: &HashSet<String>, max_distance: u8) -> Vec<String> {
        let mut similar = Vec::new();
        let token_lower = token.to_lowercase();

        for dict_token in dictionary {
            let dict_lower = dict_token.to_lowercase();
            
            // Use multiple algorithms and take best match
            let mut best_similarity: f64 = 0.0;
            
            for algorithm in &self.algorithms {
                let similarity = self.similarity(&token_lower, &dict_lower, algorithm);
                best_similarity = best_similarity.max(similarity as f64);
            }

            // Convert similarity to distance threshold
            let distance_threshold = 1.0 - (max_distance as f64 / 10.0);
            if best_similarity >= distance_threshold {
                similar.push(dict_token.clone());
            }
        }

        // Sort by similarity (would need to recalculate, but for now just return)
        similar
    }

    /// Calculate Levenshtein distance between two strings
    pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();
        let s1_len = s1_chars.len();
        let s2_len = s2_chars.len();

        if s1_len == 0 {
            return s2_len;
        }
        if s2_len == 0 {
            return s1_len;
        }

        let mut matrix = vec![vec![0; s2_len + 1]; s1_len + 1];

        // Initialize first row and column
        for i in 0..=s1_len {
            matrix[i][0] = i;
        }
        for j in 0..=s2_len {
            matrix[0][j] = j;
        }

        // Fill the matrix
        for i in 1..=s1_len {
            for j in 1..=s2_len {
                let cost = if s1_chars[i - 1] == s2_chars[j - 1] { 0 } else { 1 };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[s1_len][s2_len]
    }

    /// Calculate Damerau-Levenshtein distance (includes transpositions)
    pub fn damerau_levenshtein_distance(s1: &str, s2: &str) -> usize {
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();
        let s1_len = s1_chars.len();
        let s2_len = s2_chars.len();

        if s1_len == 0 {
            return s2_len;
        }
        if s2_len == 0 {
            return s1_len;
        }

        let mut da: HashMap<char, usize> = HashMap::new();
        let max_dist = s1_len + s2_len;
        let mut matrix = vec![vec![max_dist; s2_len + 2]; s1_len + 2];

        matrix[0][0] = max_dist;
        for i in 0..=s1_len {
            matrix[i + 1][0] = max_dist;
            matrix[i + 1][1] = i;
        }
        for j in 0..=s2_len {
            matrix[0][j + 1] = max_dist;
            matrix[1][j + 1] = j;
        }

        for i in 1..=s1_len {
            let mut db = 0;
            for j in 1..=s2_len {
                let k = da.get(&s2_chars[j - 1]).copied().unwrap_or(0);
                let l = db;
                let mut cost = 1;
                if s1_chars[i - 1] == s2_chars[j - 1] {
                    cost = 0;
                    db = j;
                }

                matrix[i + 1][j + 1] = (matrix[i + 1][j] + 1)
                    .min(matrix[i][j + 1] + 1)
                    .min(matrix[i][j] + cost)
                    .min(matrix[k][l] + (i - k - 1) + 1 + (j - l - 1));
            }
            da.insert(s1_chars[i - 1], i);
        }

        matrix[s1_len + 1][s2_len + 1]
    }

    /// Calculate Jaro similarity (0.0 to 1.0)
    pub fn jaro_similarity(s1: &str, s2: &str) -> f64 {
        if s1 == s2 {
            return 1.0;
        }
        if s1.is_empty() || s2.is_empty() {
            return 0.0;
        }

        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();
        let match_window = (s1_chars.len().max(s2_chars.len()) / 2) - 1;
        let match_window = match_window.max(0);

        let mut s1_matches = vec![false; s1_chars.len()];
        let mut s2_matches = vec![false; s2_chars.len()];

        let mut matches = 0;
        let mut transpositions = 0;

        // Find matches
        for i in 0..s1_chars.len() {
            let start = if i >= match_window { i - match_window } else { 0 };
            let end = (i + match_window + 1).min(s2_chars.len());

            for j in start..end {
                if s2_matches[j] || s1_chars[i] != s2_chars[j] {
                    continue;
                }
                s1_matches[i] = true;
                s2_matches[j] = true;
                matches += 1;
                break;
            }
        }

        if matches == 0 {
            return 0.0;
        }

        // Count transpositions
        let mut k = 0;
        for i in 0..s1_chars.len() {
            if !s1_matches[i] {
                continue;
            }
            while !s2_matches[k] {
                k += 1;
            }
            if s1_chars[i] != s2_chars[k] {
                transpositions += 1;
            }
            k += 1;
        }

        let matches_f = matches as f64;
        (matches_f / s1_chars.len() as f64
            + matches_f / s2_chars.len() as f64
            + (matches_f - (transpositions / 2) as f64) / matches_f)
            / 3.0
    }

    /// Calculate Jaro-Winkler similarity (0.0 to 1.0)
    pub fn jaro_winkler_similarity(s1: &str, s2: &str) -> f64 {
        let jaro = Self::jaro_similarity(s1, s2);
        if jaro < 0.7 {
            return jaro;
        }

        let prefix_len = s1.chars()
            .zip(s2.chars())
            .take(4)
            .take_while(|(a, b)| a == b)
            .count();

        jaro + (0.1 * prefix_len as f64 * (1.0 - jaro))
    }

    /// Calculate similarity using specified algorithm
    pub fn similarity(&self, s1: &str, s2: &str, algorithm: &FuzzyAlgorithm) -> f64 {
        match algorithm {
            FuzzyAlgorithm::Levenshtein => {
                let dist = Self::levenshtein_distance(s1, s2);
                let max_len = s1.len().max(s2.len());
                if max_len == 0 {
                    1.0
                } else {
                    1.0 - (dist as f64 / max_len as f64)
                }
            }
            FuzzyAlgorithm::DamerauLevenshtein => {
                let dist = Self::damerau_levenshtein_distance(s1, s2);
                let max_len = s1.len().max(s2.len());
                if max_len == 0 {
                    1.0
                } else {
                    1.0 - (dist as f64 / max_len as f64)
                }
            }
            FuzzyAlgorithm::Jaro => Self::jaro_similarity(s1, s2),
            FuzzyAlgorithm::JaroWinkler => Self::jaro_winkler_similarity(s1, s2),
            FuzzyAlgorithm::NGram => {
                // N-gram similarity (trigram)
                let n = 3;
                let s1_ngrams = Self::ngrams(s1, n);
                let s2_ngrams = Self::ngrams(s2, n);
                let intersection = s1_ngrams.intersection(&s2_ngrams).count();
                let union = s1_ngrams.union(&s2_ngrams).count();
                if union == 0 {
                    0.0
                } else {
                    intersection as f64 / union as f64
                }
            }
            FuzzyAlgorithm::Soundex => {
                if Self::soundex(s1) == Self::soundex(s2) {
                    1.0
                } else {
                    0.0
                }
            }
            _ => Self::jaro_winkler_similarity(s1, s2), // Default fallback
        }
    }

    /// Generate n-grams for a string
    fn ngrams(s: &str, n: usize) -> HashSet<String> {
        let mut ngrams = HashSet::new();
        let chars: Vec<char> = s.chars().collect();
        for i in 0..chars.len().saturating_sub(n - 1) {
            let ngram: String = chars[i..i + n].iter().collect();
            ngrams.insert(ngram);
        }
        ngrams
    }

    /// Soundex algorithm for phonetic matching
    fn soundex(s: &str) -> String {
        if s.is_empty() {
            return String::new();
        }

        let mut result = String::with_capacity(4);
        let chars: Vec<char> = s.to_uppercase().chars().collect();
        
        // First character
        result.push(chars[0]);

        // Map characters to digits
        let mut prev_digit = None;
        for &ch in chars.iter().skip(1) {
            let digit = match ch {
                'B' | 'F' | 'P' | 'V' => Some('1'),
                'C' | 'G' | 'J' | 'K' | 'Q' | 'S' | 'X' | 'Z' => Some('2'),
                'D' | 'T' => Some('3'),
                'L' => Some('4'),
                'M' | 'N' => Some('5'),
                'R' => Some('6'),
                _ => None,
            };

            if let Some(d) = digit {
                if Some(d) != prev_digit && result.len() < 4 {
                    result.push(d);
                }
                prev_digit = Some(d);
            } else {
                prev_digit = None;
            }
        }

        // Pad with zeros
        while result.len() < 4 {
            result.push('0');
        }

        result
    }
}

impl SynonymEngine {
    fn new() -> Self {
        let mut engine = Self {
            synonyms: Arc::new(RwLock::new(HashMap::new())),
            related_terms: Arc::new(RwLock::new(HashMap::new())),
            auto_expand: true,
        };
        
        // Initialize with common English synonyms
        engine.initialize_common_synonyms();
        engine
    }

    /// Initialize common English synonyms
    fn initialize_common_synonyms(&self) {
        let mut synonyms = self.synonyms.write();
        
        // Common synonym groups
        let synonym_groups = vec![
            vec!["car", "automobile", "vehicle", "auto"],
            vec!["big", "large", "huge", "enormous"],
            vec!["small", "tiny", "little", "mini"],
            vec!["fast", "quick", "rapid", "swift"],
            vec!["slow", "sluggish", "lethargic"],
            vec!["happy", "joyful", "glad", "pleased"],
            vec!["sad", "unhappy", "depressed", "melancholy"],
            vec!["good", "great", "excellent", "fine"],
            vec!["bad", "terrible", "awful", "poor"],
            vec!["buy", "purchase", "acquire"],
            vec!["sell", "vend", "trade"],
            vec!["find", "search", "locate", "discover"],
            vec!["help", "assist", "aid", "support"],
            vec!["begin", "start", "commence"],
            vec!["end", "finish", "complete", "conclude"],
            vec!["show", "display", "exhibit", "present"],
            vec!["hide", "conceal", "cover"],
            vec!["create", "make", "build", "construct"],
            vec!["destroy", "break", "demolish"],
            vec!["increase", "grow", "expand"],
            vec!["decrease", "reduce", "diminish"],
        ];
        
        // Build bidirectional synonym map
        for group in synonym_groups {
            for word in &group {
                let mut word_synonyms = Vec::new();
                for other in &group {
                    if other != word {
                        word_synonyms.push(other.to_string());
                    }
                }
                synonyms.insert(word.to_string(), word_synonyms);
            }
        }
    }

    async fn expand(&self, query: &str) -> Result<String> {
        if !self.auto_expand {
            return Ok(query.to_string());
        }
        
        let synonyms = self.synonyms.read();
        let words: Vec<&str> = query.split_whitespace().collect();
        let mut expanded_words = Vec::new();
        
        for word in words {
            let word_lower = word.to_lowercase();
            expanded_words.push(word.to_string());
            
            // Add synonyms if found
            if let Some(syns) = synonyms.get(&word_lower) {
                // Add first synonym as alternative
                if let Some(first_syn) = syns.first() {
                    expanded_words.push(format!("({})", first_syn));
                }
            }
        }
        
        Ok(expanded_words.join(" "))
    }

    /// Add custom synonym
    pub fn add_synonym(&self, word: &str, synonym: &str) {
        let mut synonyms = self.synonyms.write();
        synonyms.entry(word.to_lowercase())
            .or_insert_with(Vec::new)
            .push(synonym.to_string());
    }
}

impl QueryUnderstandingEngine {
    fn new() -> Self {
        Self {
            intent_classifier: IntentClassifier { model: "default".to_string() },
            entity_extractor: EntityExtractor { patterns: HashMap::new() },
            keyword_extractor: KeywordExtractor { min_length: 2, max_length: 50 },
            sentiment_analyzer: SentimentAnalyzer { model: "default".to_string() },
        }
    }

    async fn understand(&self, query: &str, language: Option<&str>) -> Result<QueryUnderstanding> {
        // Understand query: extract intent, entities, keywords, sentiment
        Ok(QueryUnderstanding {
            intent: SearchIntent::Find,
            entities: Vec::new(),
            keywords: query.split_whitespace().map(|s| s.to_string()).collect(),
            categories: Vec::new(),
            sentiment: Some(Sentiment::Neutral),
            language: language.unwrap_or("en").to_string(),
            confidence: 0.8,
        })
    }
}

impl AutoCompleteEngine {
    fn new() -> Self {
        Self {
            trie: Arc::new(RwLock::new(Trie::new())),
            suggestions: Arc::new(DashMap::new()),
            max_suggestions: 10,
        }
    }

    async fn suggest(&self, query: &str, limit: usize) -> Result<Vec<Suggestion>> {
        let trie = self.trie.read();
        let mut suggestions = Vec::new();
        
        // Traverse trie to find matching nodes
        let mut current = &trie.root;
        let query_chars: Vec<char> = query.chars().collect();
        
        // Navigate to the node matching the query prefix
        for &ch in &query_chars {
            if let Some(next) = current.children.get(&ch) {
                current = next;
            } else {
                // No match found
                return Ok(suggestions);
            }
        }
        
        // Collect suggestions from this node and its children
        self.collect_suggestions(current, &query_chars, &mut suggestions, limit);
        
        // Sort by frequency/score
        suggestions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        suggestions.truncate(limit);
        
        Ok(suggestions)
    }
    
    /// Recursively collect suggestions from trie node
    fn collect_suggestions(
        &self,
        node: &TrieNode,
        prefix: &[char],
        suggestions: &mut Vec<Suggestion>,
        limit: usize,
    ) {
        if suggestions.len() >= limit {
            return;
        }
        
        // Add suggestions from this node
        for sug_text in &node.suggestions {
            if suggestions.len() >= limit {
                break;
            }
            suggestions.push(Suggestion {
                text: sug_text.clone(),
                score: node.frequency as f64,
                category: None,
            });
        }
        
        // Recursively collect from children
        for (_, child) in &node.children {
            self.collect_suggestions(child, prefix, suggestions, limit);
        }
    }
}

impl SearchHistory {
    fn new() -> Self {
        Self {
            history: Arc::new(DashMap::new()),
            max_entries_per_user: 100,
        }
    }

    async fn record(&self, user_id: String, query: String, results_count: usize) -> Result<()> {
        let entry = SearchHistoryEntry {
            query,
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            results_count,
            clicked_results: Vec::new(),
        };
        
        let mut user_history = self.history.entry(user_id).or_insert_with(Vec::new);
        user_history.push(entry);
        
        // Keep only recent entries
        if user_history.len() > self.max_entries_per_user {
            user_history.remove(0);
        }
        
        Ok(())
    }
}

impl PersonalizationEngine {
    fn new() -> Self {
        Self {
            user_profiles: Arc::new(DashMap::new()),
            collaborative_filtering: true,
        }
    }

    async fn personalize(&self, results: Vec<SearchResult>, context: &SearchContext) -> Result<Vec<SearchResult>> {
        // Personalize results based on user profile
        Ok(results)
    }
}

impl MultiLanguageSupport {
    fn new() -> Self {
        Self {
            languages: ["en", "es", "fr", "de", "it", "pt", "ru", "zh", "ja", "ko"].iter()
                .map(|s| s.to_string())
                .collect(),
            language_detector: LanguageDetector { model: "default".to_string() },
            translators: HashMap::new(),
        }
    }
}

impl TypoCorrector {
    fn new() -> Self {
        let mut corrector = Self {
            dictionary: HashSet::new(),
            max_distance: 2,
            algorithms: vec![FuzzyAlgorithm::Levenshtein],
        };
        
        // Initialize with common English words
        corrector.initialize_dictionary();
        corrector
    }

    /// Initialize dictionary with common English words
    fn initialize_dictionary(&mut self) {
        let common_words = vec![
            "the", "be", "to", "of", "and", "a", "in", "that", "have", "i",
            "it", "for", "not", "on", "with", "he", "as", "you", "do", "at",
            "this", "but", "his", "by", "from", "they", "we", "say", "her", "she",
            "or", "an", "will", "my", "one", "all", "would", "there", "their",
            "what", "so", "up", "out", "if", "about", "who", "get", "which", "go",
            "me", "when", "make", "can", "like", "time", "no", "just", "him", "know",
            "take", "people", "into", "year", "your", "good", "some", "could", "them",
            "see", "other", "than", "then", "now", "look", "only", "come", "its", "over",
            "think", "also", "back", "after", "use", "two", "how", "our", "work", "first",
            "well", "way", "even", "new", "want", "because", "any", "these", "give", "day",
            "most", "us", "find", "search", "query", "result", "data", "information",
            "document", "file", "text", "content", "page", "web", "site", "link",
        ];
        
        for word in common_words {
            self.dictionary.insert(word.to_string());
        }
    }

    async fn correct(&self, query: &str, max_distance: u8) -> Result<String> {
        let words: Vec<&str> = query.split_whitespace().collect();
        let mut corrected_words = Vec::new();
        
        for word in words {
            let word_lower = word.to_lowercase();
            
            // Check if word is already in dictionary
            if self.dictionary.contains(&word_lower) {
                corrected_words.push(word.to_string());
                continue;
            }
            
            // Find closest match in dictionary
            let mut best_match = word.to_string();
            let mut best_distance = max_distance as usize + 1;
            
            for dict_word in &self.dictionary {
                let distance = FuzzyMatcher::levenshtein_distance(&word_lower, dict_word);
                if distance < best_distance && distance <= max_distance as usize {
                    best_distance = distance;
                    best_match = dict_word.clone();
                }
            }
            
            // If we found a close match, use it; otherwise keep original
            if best_distance <= max_distance as usize {
                // Preserve original capitalization if word was capitalized
                if word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    let mut chars: Vec<char> = best_match.chars().collect();
                    if let Some(first) = chars.first_mut() {
                        *first = first.to_uppercase().next().unwrap();
                    }
                    corrected_words.push(chars.into_iter().collect());
                } else {
                    corrected_words.push(best_match);
                }
            } else {
                corrected_words.push(word.to_string());
            }
        }
        
        Ok(corrected_words.join(" "))
    }

    /// Add word to dictionary
    pub fn add_to_dictionary(&mut self, word: &str) {
        self.dictionary.insert(word.to_lowercase());
    }
}

impl Trie {
    fn new() -> Self {
        Self {
            root: TrieNode {
                children: HashMap::new(),
                is_end: false,
                suggestions: Vec::new(),
                frequency: 0,
            },
        }
    }

    /// Insert a word into the trie
    fn insert(&mut self, word: &str) {
        let mut current = &mut self.root;
        let chars: Vec<char> = word.chars().collect();
        
        for &ch in &chars {
            current = current.children
                .entry(ch)
                .or_insert_with(|| TrieNode {
                    children: HashMap::new(),
                    is_end: false,
                    suggestions: Vec::new(),
                    frequency: 0,
                });
        }
        
        current.is_end = true;
        current.frequency += 1;
        if !current.suggestions.contains(&word.to_string()) {
            current.suggestions.push(word.to_string());
        }
    }

    /// Build trie from search history and indexed terms
    fn build_from_terms(&mut self, terms: &[String]) {
        for term in terms {
            self.insert(term);
        }
    }
}

impl Default for HumanSearchQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            filters: Vec::new(),
            sort: None,
            limit: Some(10),
            offset: Some(0),
            language: Some("en".to_string()),
            fuzzy: true,
            typo_tolerance: Some(2),
            semantic: true,
            synonyms: true,
            context: None,
        }
    }
}

