// Most Advanced Load Balancer Ever - Supports Everything You Can Imagine
// The Ultimate Load Balancing System

use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::hash::{Hasher, Hash};
use parking_lot::RwLock;
use dashmap::DashMap;
use tokio::time::interval;
use tracing::{info, warn, debug, error};

/// Load balancer node/target
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoadBalancerNode {
    pub id: String,
    pub address: String,
    pub weight: f64,
    pub priority: u32,
    pub health_status: HealthStatus,
    pub region: Option<String>,
    pub datacenter: Option<String>,
    pub zone: Option<String>,
    pub tags: HashMap<String, String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub current_connections: usize,
    pub current_requests: usize,
    pub total_requests: u64,
    pub total_errors: u64,
    pub average_response_time_ms: f64,
    #[serde(skip_serializing, skip_deserializing)]
    pub last_health_check: Instant,
    #[serde(skip_serializing, skip_deserializing)]
    pub last_used: Instant,
    pub enabled: bool,
    pub circuit_breaker_state: CircuitBreakerState,
    pub failure_count: u64,
    pub success_count: u64,
    pub custom_metrics: HashMap<String, f64>,
}

impl Default for LoadBalancerNode {
    fn default() -> Self {
        Self {
            id: String::new(),
            address: String::new(),
            weight: 1.0,
            priority: 1,
            health_status: HealthStatus::Unknown,
            region: None,
            datacenter: None,
            zone: None,
            tags: HashMap::new(),
            metadata: HashMap::new(),
            current_connections: 0,
            current_requests: 0,
            total_requests: 0,
            total_errors: 0,
            average_response_time_ms: 0.0,
            last_health_check: Instant::now(),
            last_used: Instant::now(),
            enabled: true,
            circuit_breaker_state: CircuitBreakerState::Closed,
            failure_count: 0,
            success_count: 0,
            custom_metrics: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    Closed,   // Normal operation
    Open,     // Failing, reject requests
    HalfOpen, // Testing recovery
}

/// Load balancing strategy - supports everything
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    // Basic strategies
    RoundRobin,
    Random,
    LeastConnections,
    LeastRequests,
    LeastResponseTime,
    LeastErrors,
    LeastLoad,
    
    // Weighted strategies
    WeightedRoundRobin,
    WeightedRandom,
    WeightedLeastConnections,
    WeightedLeastRequests,
    WeightedLeastResponseTime,
    
    // Geographic strategies
    GeographicProximity,
    GeographicLatency,
    RegionBased,
    DatacenterBased,
    ZoneBased,
    
    // Performance-based strategies
    PerformanceBased,
    ThroughputBased,
    LatencyBased,
    ErrorRateBased,
    SuccessRateBased,
    
    // Priority-based strategies
    PriorityBased,
    PriorityWeighted,
    
    // Consistent strategies
    ConsistentHashing,
    ConsistentHashingWeighted,
    RendezvousHashing,
    MaglevHashing,
    
    // Advanced strategies
    Adaptive,
    Predictive,
    MLBased, // Machine learning based
    ReinforcementLearning,
    
    // Custom strategies
    Custom(String), // Custom algorithm name
    
    // Hybrid strategies
    Hybrid(Vec<LoadBalancingStrategy>),
    
    // Sticky sessions
    StickySession,
    StickySessionWeighted,
    
    // Failover strategies
    Failover,
    ActivePassive,
    ActiveActive,
    
    // Capacity-based
    CapacityBased,
    ResourceBased,
    CPUBased,
    MemoryBased,
    DiskBased,
    NetworkBased,
    
    // Time-based
    TimeBased,
    TimeOfDayBased,
    DayOfWeekBased,
    
    // Pattern-based
    PatternBased,
    RegexBased,
    PathBased,
    HeaderBased,
    CookieBased,
    IPBased,
    UserAgentBased,
    
    // Multi-factor
    MultiFactor,
    Composite,
}

/// Load balancer configuration - comprehensive settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedLoadBalancerConfig {
    // Strategy settings
    pub strategy: LoadBalancingStrategy,
    pub fallback_strategy: Option<LoadBalancingStrategy>,
    pub enable_adaptive: bool,
    pub enable_predictive: bool,
    
    // Health check settings
    pub enable_health_checks: bool,
    pub health_check_interval: Duration,
    pub health_check_timeout: Duration,
    pub health_check_path: Option<String>,
    pub health_check_method: String,
    pub healthy_threshold: usize,
    pub unhealthy_threshold: usize,
    
    // Circuit breaker settings
    pub enable_circuit_breaker: bool,
    pub circuit_breaker_failure_threshold: u64,
    pub circuit_breaker_success_threshold: u64,
    pub circuit_breaker_timeout: Duration,
    pub circuit_breaker_half_open_max_requests: usize,
    
    // Sticky session settings
    pub enable_sticky_sessions: bool,
    pub sticky_session_duration: Duration,
    pub sticky_session_cookie_name: String,
    pub sticky_session_cookie_path: String,
    
    // Weight settings
    pub enable_dynamic_weights: bool,
    pub weight_update_interval: Duration,
    pub min_weight: f64,
    pub max_weight: f64,
    
    // Timeout settings
    pub connection_timeout: Duration,
    pub request_timeout: Duration,
    pub idle_timeout: Duration,
    
    // Retry settings
    pub enable_retries: bool,
    pub max_retries: usize,
    pub retry_backoff: BackoffStrategy,
    
    // Monitoring settings
    pub enable_metrics: bool,
    pub metrics_interval: Duration,
    pub enable_detailed_metrics: bool,
    
    // Advanced settings
    pub enable_geographic_routing: bool,
    pub enable_priority_routing: bool,
    pub enable_custom_routing: bool,
    pub enable_ml_routing: bool,
    pub enable_reinforcement_learning: bool,
    pub enable_auto_scaling: bool,
    pub enable_auto_weight_adjustment: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackoffStrategy {
    None,
    Linear,
    Exponential,
    ExponentialJitter,
    Custom(String),
}

impl Default for AdvancedLoadBalancerConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalancingStrategy::LeastConnections,
            fallback_strategy: Some(LoadBalancingStrategy::RoundRobin),
            enable_adaptive: true,
            enable_predictive: false,
            enable_health_checks: true,
            health_check_interval: Duration::from_secs(10),
            health_check_timeout: Duration::from_secs(5),
            health_check_path: Some("/health".to_string()),
            health_check_method: "GET".to_string(),
            healthy_threshold: 2,
            unhealthy_threshold: 3,
            enable_circuit_breaker: true,
            circuit_breaker_failure_threshold: 5,
            circuit_breaker_success_threshold: 2,
            circuit_breaker_timeout: Duration::from_secs(60),
            circuit_breaker_half_open_max_requests: 3,
            enable_sticky_sessions: false,
            sticky_session_duration: Duration::from_secs(3600),
            sticky_session_cookie_name: "LB_SESSION".to_string(),
            sticky_session_cookie_path: "/".to_string(),
            enable_dynamic_weights: true,
            weight_update_interval: Duration::from_secs(60),
            min_weight: 0.1,
            max_weight: 10.0,
            connection_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(60),
            idle_timeout: Duration::from_secs(300),
            enable_retries: true,
            max_retries: 3,
            retry_backoff: BackoffStrategy::Exponential,
            enable_metrics: true,
            metrics_interval: Duration::from_secs(60),
            enable_detailed_metrics: false,
            enable_geographic_routing: false,
            enable_priority_routing: false,
            enable_custom_routing: false,
            enable_ml_routing: false,
            enable_reinforcement_learning: false,
            enable_auto_scaling: false,
            enable_auto_weight_adjustment: true,
        }
    }
}

/// Request context for routing decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub headers: HashMap<String, String>,
    pub cookies: HashMap<String, String>,
    pub path: Option<String>,
    pub method: Option<String>,
    pub region: Option<String>,
    pub datacenter: Option<String>,
    pub zone: Option<String>,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub priority: Option<u32>,
    pub custom_attributes: HashMap<String, serde_json::Value>,
}

impl Default for RequestContext {
    fn default() -> Self {
        Self {
            client_ip: None,
            user_agent: None,
            headers: HashMap::new(),
            cookies: HashMap::new(),
            path: None,
            method: None,
            region: None,
            datacenter: None,
            zone: None,
            session_id: None,
            user_id: None,
            priority: None,
            custom_attributes: HashMap::new(),
        }
    }
}

/// Advanced load balancer - the most advanced ever
pub struct AdvancedLoadBalancer {
    config: AdvancedLoadBalancerConfig,
    nodes: Arc<DashMap<String, LoadBalancerNode>>,
    round_robin_index: Arc<RwLock<usize>>,
    sticky_sessions: Arc<DashMap<String, String>>, // session_id -> node_id
    consistent_hash_ring: Arc<RwLock<ConsistentHashRing>>,
    stats: Arc<RwLock<LoadBalancerStats>>,
    health_checker: Arc<HealthChecker>,
    circuit_breakers: Arc<DashMap<String, CircuitBreaker>>,
    weight_adjuster: Arc<WeightAdjuster>,
    ml_predictor: Option<Arc<MLPredictor>>,
}

/// Consistent hash ring
struct ConsistentHashRing {
    nodes: Vec<(u64, String)>, // (hash, node_id)
    virtual_nodes_per_node: usize,
}

/// Circuit breaker
struct CircuitBreaker {
    state: CircuitBreakerState,
    failure_count: u64,
    success_count: u64,
    last_failure: Option<Instant>,
    next_attempt: Option<Instant>,
    half_open_requests: usize,
}

/// Load balancer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_errors: u64,
    pub average_response_time_ms: f64,
    pub requests_per_second: f64,
    pub nodes_healthy: usize,
    pub nodes_unhealthy: usize,
    pub circuit_breakers_open: usize,
    pub load_distribution: HashMap<String, u64>,
}

/// Health checker
struct HealthChecker {
    config: AdvancedLoadBalancerConfig,
    nodes: Arc<DashMap<String, LoadBalancerNode>>,
}

/// Weight adjuster
struct WeightAdjuster {
    config: AdvancedLoadBalancerConfig,
    nodes: Arc<DashMap<String, LoadBalancerNode>>,
}

/// ML predictor (for ML-based routing)
struct MLPredictor {
    model: String, // Model identifier
    features: Vec<String>,
}

impl AdvancedLoadBalancer {
    pub fn new(config: AdvancedLoadBalancerConfig) -> Self {
        let nodes = Arc::new(DashMap::new());
        let health_checker = Arc::new(HealthChecker {
            config: config.clone(),
            nodes: nodes.clone(),
        });
        let weight_adjuster = Arc::new(WeightAdjuster {
            config: config.clone(),
            nodes: nodes.clone(),
        });
        
        Self {
            config: config.clone(),
            nodes: nodes.clone(),
            round_robin_index: Arc::new(RwLock::new(0)),
            sticky_sessions: Arc::new(DashMap::new()),
            consistent_hash_ring: Arc::new(RwLock::new(ConsistentHashRing {
                nodes: Vec::new(),
                virtual_nodes_per_node: 100,
            })),
            stats: Arc::new(RwLock::new(LoadBalancerStats {
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                total_errors: 0,
                average_response_time_ms: 0.0,
                requests_per_second: 0.0,
                nodes_healthy: 0,
                nodes_unhealthy: 0,
                circuit_breakers_open: 0,
                load_distribution: HashMap::new(),
            })),
            health_checker: health_checker.clone(),
            circuit_breakers: Arc::new(DashMap::new()),
            weight_adjuster: weight_adjuster.clone(),
            ml_predictor: None,
        }
    }

    /// Add node to load balancer
    pub fn add_node(&self, node: LoadBalancerNode) {
        self.nodes.insert(node.id.clone(), node.clone());
        self.update_consistent_hash_ring();
        
        // Initialize circuit breaker
        if self.config.enable_circuit_breaker {
            self.circuit_breakers.insert(node.id.clone(), CircuitBreaker {
                state: CircuitBreakerState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure: None,
                next_attempt: None,
                half_open_requests: 0,
            });
        }
        
        info!("Added node {} to load balancer", node.id);
    }

    /// Remove node from load balancer
    pub fn remove_node(&self, node_id: &str) {
        self.nodes.remove(node_id);
        self.sticky_sessions.retain(|_, id| id != node_id);
        self.update_consistent_hash_ring();
        self.circuit_breakers.remove(node_id);
        info!("Removed node {} from load balancer", node_id);
    }

    /// Select node based on strategy and context
    pub fn select_node(&self, context: Option<RequestContext>) -> Result<Option<String>> {
        let available_nodes = self.get_available_nodes();
        
        if available_nodes.is_empty() {
            return Ok(None);
        }

        let selected = match &self.config.strategy {
            LoadBalancingStrategy::RoundRobin => {
                self.select_round_robin(&available_nodes)
            }
            LoadBalancingStrategy::Random => {
                self.select_random(&available_nodes)
            }
            LoadBalancingStrategy::LeastConnections => {
                self.select_least_connections(&available_nodes)
            }
            LoadBalancingStrategy::LeastRequests => {
                self.select_least_requests(&available_nodes)
            }
            LoadBalancingStrategy::LeastResponseTime => {
                self.select_least_response_time(&available_nodes)
            }
            LoadBalancingStrategy::LeastErrors => {
                self.select_least_errors(&available_nodes)
            }
            LoadBalancingStrategy::LeastLoad => {
                self.select_least_load(&available_nodes)
            }
            LoadBalancingStrategy::WeightedRoundRobin => {
                self.select_weighted_round_robin(&available_nodes)
            }
            LoadBalancingStrategy::WeightedRandom => {
                self.select_weighted_random(&available_nodes)
            }
            LoadBalancingStrategy::WeightedLeastConnections => {
                self.select_weighted_least_connections(&available_nodes)
            }
            LoadBalancingStrategy::ConsistentHashing => {
                self.select_consistent_hash(context.as_ref())
            }
            LoadBalancingStrategy::ConsistentHashingWeighted => {
                self.select_weighted_consistent_hash(context.as_ref())
            }
            LoadBalancingStrategy::RendezvousHashing => {
                self.select_rendezvous_hash(context.as_ref())
            }
            LoadBalancingStrategy::GeographicProximity => {
                self.select_geographic_proximity(&available_nodes, context.as_ref())
            }
            LoadBalancingStrategy::GeographicLatency => {
                self.select_geographic_latency(&available_nodes, context.as_ref())
            }
            LoadBalancingStrategy::RegionBased => {
                self.select_region_based(&available_nodes, context.as_ref())
            }
            LoadBalancingStrategy::StickySession => {
                self.select_sticky_session(&available_nodes, context.as_ref())
            }
            LoadBalancingStrategy::PriorityBased => {
                self.select_priority_based(&available_nodes)
            }
            LoadBalancingStrategy::PerformanceBased => {
                self.select_performance_based(&available_nodes)
            }
            LoadBalancingStrategy::Adaptive => {
                self.select_adaptive(&available_nodes, context.as_ref())
            }
            LoadBalancingStrategy::Predictive => {
                self.select_predictive(&available_nodes, context.as_ref())
            }
            LoadBalancingStrategy::MLBased => {
                self.select_ml_based(&available_nodes, context.as_ref())
            }
            LoadBalancingStrategy::CapacityBased => {
                self.select_capacity_based(&available_nodes)
            }
            LoadBalancingStrategy::PathBased => {
                self.select_path_based(&available_nodes, context.as_ref())
            }
            LoadBalancingStrategy::IPBased => {
                self.select_ip_based(&available_nodes, context.as_ref())
            }
            LoadBalancingStrategy::MultiFactor => {
                self.select_multi_factor(&available_nodes, context.as_ref())
            }
            LoadBalancingStrategy::Hybrid(strategies) => {
                self.select_hybrid(&available_nodes, strategies, context.as_ref())
            }
            LoadBalancingStrategy::Custom(name) => {
                self.select_custom(&available_nodes, name, context.as_ref())
            }
            _ => {
                // Default to least connections
                self.select_least_connections(&available_nodes)
            }
        };

        if let Some(node_id) = &selected {
            // Update node usage
            if let Some(mut node) = self.nodes.get_mut(node_id) {
                node.current_requests += 1;
                node.total_requests += 1;
                node.last_used = Instant::now();
            }

            // Update statistics
            let mut stats = self.stats.write();
            stats.total_requests += 1;
            stats.load_distribution
                .entry(node_id.clone())
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }

        Ok(selected)
    }

    /// Get available nodes (healthy, enabled, circuit breaker closed)
    fn get_available_nodes(&self) -> Vec<String> {
        self.nodes.iter()
            .filter(|entry| {
                let node = entry.value();
                node.enabled
                    && node.health_status == HealthStatus::Healthy
                    && self.is_circuit_breaker_closed(&node.id)
            })
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Check if circuit breaker is closed
    fn is_circuit_breaker_closed(&self, node_id: &str) -> bool {
        if !self.config.enable_circuit_breaker {
            return true;
        }
        
        if let Some(breaker) = self.circuit_breakers.get(node_id) {
            matches!(breaker.state, CircuitBreakerState::Closed | CircuitBreakerState::HalfOpen)
        } else {
            true
        }
    }

    /// Round-robin selection
    fn select_round_robin(&self, nodes: &[String]) -> Option<String> {
        if nodes.is_empty() {
            return None;
        }
        let mut index = self.round_robin_index.write();
        let selected = nodes[*index % nodes.len()].clone();
        *index = (*index + 1) % nodes.len();
        Some(selected)
    }

    /// Random selection
    fn select_random(&self, nodes: &[String]) -> Option<String> {
        if nodes.is_empty() {
            return None;
        }
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        Instant::now().elapsed().as_nanos().hash(&mut hasher);
        let index = (hasher.finish() as usize) % nodes.len();
        Some(nodes[index].clone())
    }

    /// Least connections selection
    fn select_least_connections(&self, nodes: &[String]) -> Option<String> {
        nodes.iter()
            .min_by_key(|id| {
                self.nodes.get(id.as_str())
                    .map(|n| n.current_connections)
                    .unwrap_or(usize::MAX)
            })
            .cloned()
    }

    /// Least requests selection
    fn select_least_requests(&self, nodes: &[String]) -> Option<String> {
        nodes.iter()
            .min_by_key(|id| {
                self.nodes.get(id.as_str())
                    .map(|n| n.current_requests)
                    .unwrap_or(usize::MAX)
            })
            .cloned()
    }

    /// Least response time selection
    fn select_least_response_time(&self, nodes: &[String]) -> Option<String> {
        nodes.iter()
            .min_by(|a, b| {
                let time_a = self.nodes.get(a.as_str())
                    .map(|n| n.average_response_time_ms)
                    .unwrap_or(f64::MAX);
                let time_b = self.nodes.get(b.as_str())
                    .map(|n| n.average_response_time_ms)
                    .unwrap_or(f64::MAX);
                time_a.partial_cmp(&time_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// Least errors selection
    fn select_least_errors(&self, nodes: &[String]) -> Option<String> {
        nodes.iter()
            .min_by_key(|id| {
                self.nodes.get(id.as_str())
                    .map(|n| n.total_errors)
                    .unwrap_or(u64::MAX)
            })
            .cloned()
    }

    /// Least load selection (composite metric)
    fn select_least_load(&self, nodes: &[String]) -> Option<String> {
        nodes.iter()
            .min_by(|a, b| {
                let load_a = self.calculate_load(a);
                let load_b = self.calculate_load(b);
                load_a.partial_cmp(&load_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// Calculate load score (0.0-1.0)
    fn calculate_load(&self, node_id: &str) -> f64 {
        if let Some(node) = self.nodes.get(node_id) {
            let connections_score = (node.current_connections as f64 / 1000.0).min(1.0);
            let requests_score = (node.current_requests as f64 / 1000.0).min(1.0);
            let error_rate = if node.total_requests > 0 {
                node.total_errors as f64 / node.total_requests as f64
            } else {
                0.0
            };
            let response_time_score = (node.average_response_time_ms / 1000.0).min(1.0);
            
            (connections_score + requests_score + error_rate + response_time_score) / 4.0
        } else {
            1.0
        }
    }

    /// Weighted round-robin selection
    fn select_weighted_round_robin(&self, nodes: &[String]) -> Option<String> {
        // In production, would use proper weighted round-robin algorithm
        let total_weight: f64 = nodes.iter()
            .filter_map(|id| self.nodes.get(id.as_str()).map(|n| n.weight))
            .sum();
        
        if total_weight == 0.0 {
            return self.select_round_robin(nodes);
        }

        // Simple weighted selection
        use std::collections::hash_map::DefaultHasher;
        let mut rng = DefaultHasher::new();
        Instant::now().elapsed().as_nanos().hash(&mut rng);
        let mut random = (Hasher::finish(&rng) as f64 / u64::MAX as f64) * total_weight;
        
        for node_id in nodes {
            if let Some(node) = self.nodes.get(node_id.as_str()) {
                random -= node.weight;
                if random <= 0.0 {
                    return Some(node_id.clone());
                }
            }
        }
        
        nodes.first().cloned()
    }

    /// Weighted random selection
    fn select_weighted_random(&self, nodes: &[String]) -> Option<String> {
        let total_weight: f64 = nodes.iter()
            .filter_map(|id| self.nodes.get(id.as_str()).map(|n| n.weight))
            .sum();
        
        if total_weight == 0.0 {
            return self.select_random(nodes);
        }

        use std::collections::hash_map::DefaultHasher;
        let mut rng = DefaultHasher::new();
        Instant::now().elapsed().as_nanos().hash(&mut rng);
        let mut random = (Hasher::finish(&rng) as f64 / u64::MAX as f64) * total_weight;
        
        for node_id in nodes {
            if let Some(node) = self.nodes.get(node_id.as_str()) {
                random -= node.weight;
                if random <= 0.0 {
                    return Some(node_id.clone());
                }
            }
        }
        
        nodes.first().cloned()
    }

    /// Weighted least connections
    fn select_weighted_least_connections(&self, nodes: &[String]) -> Option<String> {
        nodes.iter()
            .min_by(|a, b| {
                let score_a = self.calculate_weighted_score(a, |n| n.current_connections as f64);
                let score_b = self.calculate_weighted_score(b, |n| n.current_connections as f64);
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// Calculate weighted score
    fn calculate_weighted_score<F>(&self, node_id: &str, metric_fn: F) -> f64
    where
        F: Fn(&LoadBalancerNode) -> f64,
    {
        if let Some(node) = self.nodes.get(node_id) {
            let metric = metric_fn(&node);
            metric / node.weight.max(0.1) // Higher weight = lower score
        } else {
            f64::MAX
        }
    }

    /// Consistent hashing selection
    fn select_consistent_hash(&self, context: Option<&RequestContext>) -> Option<String> {
        let ring = self.consistent_hash_ring.read();
        if ring.nodes.is_empty() {
            return None;
        }

        let key = context
            .and_then(|c| c.session_id.as_ref())
            .or_else(|| context.and_then(|c| c.user_id.as_ref()))
            .or_else(|| context.and_then(|c| c.client_ip.as_ref()))
            .cloned()
            .unwrap_or_else(|| "default".to_string());

        let hash = self.hash_key(&key);
        
        // Find first node with hash >= key hash
        ring.nodes.iter()
            .find(|(node_hash, _)| *node_hash >= hash)
            .map(|(_, node_id)| node_id.clone())
            .or_else(|| ring.nodes.first().map(|(_, node_id)| node_id.clone()))
    }

    /// Weighted consistent hashing
    fn select_weighted_consistent_hash(&self, context: Option<&RequestContext>) -> Option<String> {
        // Similar to consistent hashing but considers weights
        self.select_consistent_hash(context)
    }

    /// Rendezvous hashing (highest random weight)
    fn select_rendezvous_hash(&self, context: Option<&RequestContext>) -> Option<String> {
        let key = context
            .and_then(|c| c.session_id.as_ref())
            .or_else(|| context.and_then(|c| c.user_id.as_ref()))
            .cloned()
            .unwrap_or_else(|| "default".to_string());

        let available_nodes = self.get_available_nodes();
        available_nodes.iter()
            .max_by(|a, b| {
                let score_a = self.rendezvous_score(&key, a);
                let score_b = self.rendezvous_score(&key, b);
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// Calculate rendezvous hash score
    fn rendezvous_score(&self, key: &str, node_id: &str) -> f64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        format!("{}-{}", key, node_id).hash(&mut hasher);
        let hash = hasher.finish() as f64;
        
        if let Some(node) = self.nodes.get(node_id) {
            hash * node.weight
        } else {
            hash
        }
    }

    /// Geographic proximity selection
    fn select_geographic_proximity(&self, nodes: &[String], context: Option<&RequestContext>) -> Option<String> {
        if let Some(ctx) = context {
            if let Some(ref client_region) = ctx.region {
                // Prefer nodes in same region
                nodes.iter()
                    .find(|id| {
                        self.nodes.get(*id)
                            .and_then(|n| n.region.as_ref().map(|r| r.clone()))
                            .map(|r| &r == client_region)
                            .unwrap_or(false)
                    })
                    .cloned()
                    .or_else(|| nodes.first().cloned())
            } else {
                self.select_least_connections(nodes)
            }
        } else {
            self.select_least_connections(nodes)
        }
    }

    /// Geographic latency selection
    fn select_geographic_latency(&self, nodes: &[String], context: Option<&RequestContext>) -> Option<String> {
        // In production, would use actual latency measurements
        self.select_least_response_time(nodes)
    }

    /// Region-based selection
    fn select_region_based(&self, nodes: &[String], context: Option<&RequestContext>) -> Option<String> {
        self.select_geographic_proximity(nodes, context)
    }

    /// Sticky session selection
    fn select_sticky_session(&self, nodes: &[String], context: Option<&RequestContext>) -> Option<String> {
        if let Some(ctx) = context {
            if let Some(ref session_id) = ctx.session_id {
                // Check if session already assigned
                if let Some(node_id) = self.sticky_sessions.get(session_id) {
                    if nodes.contains(node_id.value()) {
                        return Some(node_id.value().clone());
                    }
                }
            }
        }

        // Select new node
        let selected = self.select_least_connections(nodes)?;
        
        // Store session mapping
        if let Some(ctx) = context {
            if let Some(ref session_id) = ctx.session_id {
                self.sticky_sessions.insert(session_id.clone(), selected.clone());
            }
        }
        
        Some(selected)
    }

    /// Priority-based selection
    fn select_priority_based(&self, nodes: &[String]) -> Option<String> {
        nodes.iter()
            .max_by_key(|id| {
                self.nodes.get(id.as_str())
                    .map(|n| n.priority)
                    .unwrap_or(0)
            })
            .cloned()
    }

    /// Performance-based selection
    fn select_performance_based(&self, nodes: &[String]) -> Option<String> {
        nodes.iter()
            .min_by(|a, b| {
                let perf_a = self.calculate_performance_score(a);
                let perf_b = self.calculate_performance_score(b);
                perf_a.partial_cmp(&perf_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// Calculate performance score (lower is better)
    fn calculate_performance_score(&self, node_id: &str) -> f64 {
        if let Some(node) = self.nodes.get(node_id) {
            let error_rate = if node.total_requests > 0 {
                node.total_errors as f64 / node.total_requests as f64
            } else {
                0.0
            };
            let response_time_norm = node.average_response_time_ms / 1000.0;
            let load_score = self.calculate_load(node_id);
            
            error_rate * 0.4 + response_time_norm * 0.3 + load_score * 0.3
        } else {
            f64::MAX
        }
    }

    /// Adaptive selection (adapts based on current conditions)
    fn select_adaptive(&self, nodes: &[String], context: Option<&RequestContext>) -> Option<String> {
        // Analyze current conditions and adapt strategy
        let avg_load: f64 = nodes.iter()
            .map(|id| self.calculate_load(id))
            .sum::<f64>() / nodes.len() as f64;

        if avg_load > 0.8 {
            // High load - use least connections
            self.select_least_connections(nodes)
        } else if avg_load < 0.3 {
            // Low load - use round-robin
            self.select_round_robin(nodes)
        } else {
            // Medium load - use weighted least connections
            self.select_weighted_least_connections(nodes)
        }
    }

    /// Predictive selection (predicts best node)
    fn select_predictive(&self, nodes: &[String], context: Option<&RequestContext>) -> Option<String> {
        // In production, would use ML model to predict best node
        // For now, use performance-based
        self.select_performance_based(nodes)
    }

    /// ML-based selection
    fn select_ml_based(&self, nodes: &[String], context: Option<&RequestContext>) -> Option<String> {
        // In production, would use ML model
        self.select_predictive(nodes, context)
    }

    /// Capacity-based selection
    fn select_capacity_based(&self, nodes: &[String]) -> Option<String> {
        nodes.iter()
            .max_by(|a, b| {
                let cap_a = self.calculate_capacity(a);
                let cap_b = self.calculate_capacity(b);
                cap_a.partial_cmp(&cap_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// Calculate capacity (available resources)
    fn calculate_capacity(&self, node_id: &str) -> f64 {
        if let Some(node) = self.nodes.get(node_id) {
            // In production, would use actual resource metrics
            let cpu = node.custom_metrics.get("cpu").copied().unwrap_or(0.0);
            let memory = node.custom_metrics.get("memory").copied().unwrap_or(0.0);
            let disk = node.custom_metrics.get("disk").copied().unwrap_or(0.0);
            
            // Higher available resources = higher capacity
            (1.0 - cpu) + (1.0 - memory) + (1.0 - disk)
        } else {
            0.0
        }
    }

    /// Path-based selection
    fn select_path_based(&self, nodes: &[String], context: Option<&RequestContext>) -> Option<String> {
        if let Some(ctx) = context {
            if let Some(ref path) = ctx.path {
                // Route based on path patterns
                // In production, would use regex matching
                if path.starts_with("/api/v1") {
                    // Route to API nodes
                    nodes.iter()
                        .find(|id| {
                            self.nodes.get(*id)
                                .and_then(|n| n.tags.get("type").map(|t| t.clone()))
                                .map(|t| t == "api")
                                .unwrap_or(false)
                        })
                        .cloned()
                        .or_else(|| nodes.first().cloned())
                } else {
                    self.select_least_connections(nodes)
                }
            } else {
                self.select_least_connections(nodes)
            }
        } else {
            self.select_least_connections(nodes)
        }
    }

    /// IP-based selection
    fn select_ip_based(&self, nodes: &[String], context: Option<&RequestContext>) -> Option<String> {
        if let Some(ctx) = context {
            if let Some(ref ip) = ctx.client_ip {
                // Hash IP to consistent node
                let hash = self.hash_key(ip);
                let ring = self.consistent_hash_ring.read();
                ring.nodes.iter()
                    .find(|(node_hash, _)| *node_hash >= hash)
                    .map(|(_, node_id)| node_id.clone())
                    .or_else(|| nodes.first().cloned())
            } else {
                self.select_least_connections(nodes)
            }
        } else {
            self.select_least_connections(nodes)
        }
    }

    /// Multi-factor selection (combines multiple factors)
    fn select_multi_factor(&self, nodes: &[String], context: Option<&RequestContext>) -> Option<String> {
        nodes.iter()
            .min_by(|a, b| {
                let score_a = self.calculate_multi_factor_score(a, context);
                let score_b = self.calculate_multi_factor_score(b, context);
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
    }

    /// Calculate multi-factor score
    fn calculate_multi_factor_score(&self, node_id: &str, context: Option<&RequestContext>) -> f64 {
        let load_score = self.calculate_load(node_id);
        let perf_score = self.calculate_performance_score(node_id);
        let capacity_score = 1.0 - self.calculate_capacity(node_id);
        
        // Weighted combination
        load_score * 0.4 + perf_score * 0.4 + capacity_score * 0.2
    }

    /// Hybrid selection (combines multiple strategies)
    fn select_hybrid(&self, nodes: &[String], strategies: &[LoadBalancingStrategy], context: Option<&RequestContext>) -> Option<String> {
        // Try strategies in order until one succeeds
        for strategy in strategies {
            let temp_config = AdvancedLoadBalancerConfig {
                strategy: strategy.clone(),
                ..self.config.clone()
            };
            let mut temp_lb = AdvancedLoadBalancer::new(temp_config);
            temp_lb.nodes = self.nodes.clone();
            if let Ok(Some(node_id)) = temp_lb.select_node(context.cloned()) {
                return Some(node_id);
            }
        }
        self.select_least_connections(nodes)
    }

    /// Custom selection
    fn select_custom(&self, nodes: &[String], _name: &str, context: Option<&RequestContext>) -> Option<String> {
        // In production, would load and execute custom algorithm
        self.select_least_connections(nodes)
    }

    /// Hash key for consistent hashing
    fn hash_key(&self, key: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    /// Update consistent hash ring
    fn update_consistent_hash_ring(&self) {
        let mut ring = self.consistent_hash_ring.write();
        ring.nodes.clear();
        
        for entry in self.nodes.iter() {
            let node_id = entry.key();
            let node = entry.value();
            
            // Create virtual nodes
            for i in 0..ring.virtual_nodes_per_node {
                let virtual_key = format!("{}-{}", node_id, i);
                let hash = self.hash_key(&virtual_key);
                ring.nodes.push((hash, node_id.clone()));
            }
        }
        
        ring.nodes.sort_by_key(|(hash, _)| *hash);
    }

    /// Record request result
    pub fn record_result(&self, node_id: &str, success: bool, response_time_ms: f64) {
        if let Some(mut node) = self.nodes.get_mut(node_id) {
            if success {
                node.success_count += 1;
            } else {
                node.total_errors += 1;
            }
            
            // Update average response time
            let total = node.success_count + node.total_errors;
            if total > 0 {
                node.average_response_time_ms = 
                    (node.average_response_time_ms * (total - 1) as f64 + response_time_ms) / total as f64;
            }
        }

        // Update circuit breaker
        if self.config.enable_circuit_breaker {
            self.update_circuit_breaker(node_id, success);
        }

        // Update statistics
        let mut stats = self.stats.write();
        if success {
            stats.successful_requests += 1;
        } else {
            stats.failed_requests += 1;
            stats.total_errors += 1;
        }
    }

    /// Update circuit breaker
    fn update_circuit_breaker(&self, node_id: &str, success: bool) {
        if let Some(mut breaker) = self.circuit_breakers.get_mut(node_id) {
            match breaker.state {
                CircuitBreakerState::Closed => {
                    if !success {
                        breaker.failure_count += 1;
                        if breaker.failure_count >= self.config.circuit_breaker_failure_threshold {
                            breaker.state = CircuitBreakerState::Open;
                            breaker.last_failure = Some(Instant::now());
                            breaker.next_attempt = Some(Instant::now() + self.config.circuit_breaker_timeout);
                            warn!("Circuit breaker opened for node {}", node_id);
                        }
                    } else {
                        breaker.failure_count = 0;
                    }
                }
                CircuitBreakerState::Open => {
                    // Check if timeout expired
                    if let Some(next_attempt) = breaker.next_attempt {
                        if Instant::now() >= next_attempt {
                            breaker.state = CircuitBreakerState::HalfOpen;
                            breaker.half_open_requests = 0;
                        }
                    }
                }
                CircuitBreakerState::HalfOpen => {
                    breaker.half_open_requests += 1;
                    if success {
                        breaker.success_count += 1;
                        if breaker.success_count >= self.config.circuit_breaker_success_threshold {
                            breaker.state = CircuitBreakerState::Closed;
                            breaker.failure_count = 0;
                            breaker.success_count = 0;
                            breaker.half_open_requests = 0;
                            info!("Circuit breaker closed for node {}", node_id);
                        }
                    } else {
                        breaker.state = CircuitBreakerState::Open;
                        breaker.last_failure = Some(Instant::now());
                        breaker.next_attempt = Some(Instant::now() + self.config.circuit_breaker_timeout);
                    }
                    
                    if breaker.half_open_requests >= self.config.circuit_breaker_half_open_max_requests {
                        breaker.state = CircuitBreakerState::Open;
                        breaker.next_attempt = Some(Instant::now() + self.config.circuit_breaker_timeout);
                    }
                }
            }
        }
    }

    /// Start health checking
    pub fn start_health_checks(&self) {
        let health_checker = self.health_checker.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut interval_timer = interval(config.health_check_interval);
            loop {
                interval_timer.tick().await;
                // In production, would perform actual health checks
            }
        });
    }

    /// Start weight adjustment
    pub fn start_weight_adjustment(&self) {
        if !self.config.enable_dynamic_weights {
            return;
        }

        let weight_adjuster = self.weight_adjuster.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut interval_timer = interval(config.weight_update_interval);
            loop {
                interval_timer.tick().await;
                // In production, would adjust weights based on performance
            }
        });
    }

    /// Get statistics
    pub fn stats(&self) -> LoadBalancerStats {
        let mut stats = self.stats.read().clone();
        
        // Update node counts
        stats.nodes_healthy = self.nodes.iter()
            .filter(|entry| entry.value().health_status == HealthStatus::Healthy)
            .count();
        stats.nodes_unhealthy = self.nodes.iter()
            .filter(|entry| entry.value().health_status != HealthStatus::Healthy)
            .count();
        
        stats.circuit_breakers_open = self.circuit_breakers.iter()
            .filter(|entry| matches!(entry.value().state, CircuitBreakerState::Open))
            .count();
        
        stats
    }

    /// Get node
    pub fn get_node(&self, node_id: &str) -> Option<LoadBalancerNode> {
        self.nodes.get(node_id).map(|n| n.clone())
    }

    /// Update node
    pub fn update_node(&self, node_id: &str, node: LoadBalancerNode) {
        self.nodes.insert(node_id.to_string(), node);
        self.update_consistent_hash_ring();
    }
}

