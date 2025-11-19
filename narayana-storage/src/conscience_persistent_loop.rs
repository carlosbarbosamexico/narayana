// Conscience Persistent Loop (CPL)
// Main orchestrator for cognitive consciousness systems
// Integrates Global Workspace, Background Daemon, Working Memory, Memory Bridge,
// Narrative Generator, Attention Router, and Dreaming Loop

use crate::cognitive::{CognitiveBrain, CognitiveEvent, Memory, Experience, Thought};
use crate::global_workspace::GlobalWorkspace;
use crate::background_daemon::BackgroundDaemon;
use crate::working_memory::WorkingMemoryScratchpad;
use crate::memory_bridge::MemoryBridge;
use crate::narrative_generator::NarrativeGenerator;
use crate::attention_router::AttentionRouter;
use crate::dreaming_loop::DreamingLoop;
use crate::genetics::GeneticSystem;
use crate::traits_equations::TraitCalculator;
use crate::talking_cricket::{TalkingCricket, TalkingCricketConfig};
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{info, debug, warn, error};
use uuid::Uuid;

/// Configuration for the Conscience Persistent Loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CPLConfig {
    /// Loop frequency in milliseconds
    pub loop_interval_ms: u64,
    /// Enable global workspace model
    pub enable_global_workspace: bool,
    /// Enable background daemon
    pub enable_background_daemon: bool,
    /// Enable dreaming loop
    pub enable_dreaming: bool,
    /// Working memory capacity (Miller's 7±2)
    pub working_memory_capacity: usize,
    /// Attention router enabled
    pub enable_attention: bool,
    /// Narrative generator enabled
    pub enable_narrative: bool,
    /// Memory bridge enabled
    pub enable_memory_bridge: bool,
    /// Persistence enabled
    pub enable_persistence: bool,
    /// Persistence directory
    pub persistence_dir: Option<String>,
    /// Enable genetics system
    pub enable_genetics: bool,
    /// Genetic mutation rate (0.0-1.0)
    pub genetic_mutation_rate: f64,
    /// Evolution frequency (iterations between evolution cycles)
    pub evolution_frequency: u64,
    /// Trait environmental weight (0.0-1.0, balance between genes and environment)
    pub trait_environmental_weight: f64,
    /// Enable Talking Cricket moral guide (optional)
    pub enable_talking_cricket: bool,
    /// Talking Cricket LLM enabled
    pub talking_cricket_llm_enabled: bool,
    /// Talking Cricket veto threshold (0.0-1.0)
    pub talking_cricket_veto_threshold: f64,
    /// Talking Cricket evolution frequency (iterations between evolution cycles)
    pub talking_cricket_evolution_frequency: u64,
}

impl Default for CPLConfig {
    fn default() -> Self {
        Self {
            loop_interval_ms: 100, // 100ms default loop
            enable_global_workspace: true,
            enable_background_daemon: true,
            enable_dreaming: true,
            working_memory_capacity: 7, // Miller's magic number
            enable_attention: true,
            enable_narrative: true,
            enable_memory_bridge: true,
            enable_persistence: true,
            persistence_dir: Some("data/cpl".to_string()),
            enable_genetics: true,
            genetic_mutation_rate: 0.01,
            evolution_frequency: 1000,
            trait_environmental_weight: 0.3,
            enable_talking_cricket: false, // Default: disabled (optional)
            talking_cricket_llm_enabled: false,
            talking_cricket_veto_threshold: 0.3,
            talking_cricket_evolution_frequency: 1000,
        }
    }
}

/// Conscience Persistent Loop - Main orchestrator
pub struct ConsciencePersistentLoop {
    id: String,
    brain: Arc<CognitiveBrain>,
    config: CPLConfig,
    
    // Core cognitive systems (stored as Option<Arc<T>> to allow cloning across await points)
    global_workspace: Arc<RwLock<Option<Arc<GlobalWorkspace>>>>,
    background_daemon: Arc<RwLock<Option<Arc<BackgroundDaemon>>>>,
    working_memory: Arc<WorkingMemoryScratchpad>,
    memory_bridge: Arc<RwLock<Option<Arc<MemoryBridge>>>>,
    narrative_generator: Arc<RwLock<Option<Arc<NarrativeGenerator>>>>,
    attention_router: Arc<RwLock<Option<Arc<AttentionRouter>>>>,
    dreaming_loop: Arc<RwLock<Option<Arc<DreamingLoop>>>>,
    
    // Genetics system
    genetics_system: Arc<RwLock<Option<Arc<GeneticSystem>>>>,
    
    // Talking Cricket (optional moral guide)
    talking_cricket: Arc<RwLock<Option<Arc<TalkingCricket>>>>,
    
    // State management
    is_running: Arc<RwLock<bool>>,
    loop_count: Arc<RwLock<u64>>,
    last_persist: Arc<RwLock<u64>>,
    
    // Event channel for CPL events
    event_sender: broadcast::Sender<CPLEvent>,
    
    // Persistence
    persistence_path: Option<String>,
}

/// CPL-specific events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CPLEvent {
    LoopIteration { iteration: u64, timestamp: u64 },
    GlobalWorkspaceBroadcast { content_id: String, priority: f64 },
    MemoryConsolidated { memory_id: String },
    NarrativeUpdated { narrative_id: String },
    AttentionShifted { from: String, to: String },
    DreamingCycle { experiences_replayed: usize },
    BackgroundProcessCompleted { process_type: String },
    TalkingCricketAssessment { action_id: String, moral_score: f64, should_veto: bool },
}

impl ConsciencePersistentLoop {
    /// Create a new CPL instance
    pub fn new(brain: Arc<CognitiveBrain>, config: CPLConfig) -> Self {
        let id = Uuid::new_v4().to_string();
        let (sender, _) = broadcast::channel(1000);
        
        let working_memory = Arc::new(WorkingMemoryScratchpad::new(
            config.working_memory_capacity,
            brain.clone(),
        ));
        
        Self {
            id: id.clone(),
            brain: brain.clone(),
            config: config.clone(),
            global_workspace: Arc::new(RwLock::new(None)),
            background_daemon: Arc::new(RwLock::new(None)),
            working_memory: working_memory.clone(),
            memory_bridge: Arc::new(RwLock::new(None)),
            narrative_generator: Arc::new(RwLock::new(None)),
            attention_router: Arc::new(RwLock::new(None)),
            dreaming_loop: Arc::new(RwLock::new(None)),
            genetics_system: Arc::new(RwLock::new(None)),
            talking_cricket: Arc::new(RwLock::new(None)),
            is_running: Arc::new(RwLock::new(false)),
            loop_count: Arc::new(RwLock::new(0)),
            last_persist: Arc::new(RwLock::new(0)),
            event_sender: sender,
            persistence_path: config.persistence_dir.clone(),
        }
    }
    
    /// Initialize all cognitive systems
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing CPL {}", self.id);
        
        // Validate config
        if self.config.loop_interval_ms == 0 {
            return Err(Error::Storage("Loop interval must be > 0".to_string()));
        }
        if self.config.working_memory_capacity == 0 {
            return Err(Error::Storage("Working memory capacity must be > 0".to_string()));
        }
        
        // Initialize Global Workspace
        if self.config.enable_global_workspace {
            let gw = Arc::new(GlobalWorkspace::new(self.brain.clone(), self.event_sender.clone()));
            *self.global_workspace.write() = Some(gw);
            info!("Global Workspace initialized");
        }
        
        // Initialize Background Daemon
        if self.config.enable_background_daemon {
            let daemon = Arc::new(BackgroundDaemon::new(
                self.brain.clone(),
                self.event_sender.clone(),
            ));
            *self.background_daemon.write() = Some(daemon);
            info!("Background Daemon initialized");
        }
        
        // Initialize Memory Bridge
        if self.config.enable_memory_bridge {
            let bridge = Arc::new(MemoryBridge::new(
                self.brain.clone(),
                self.working_memory.clone(),
                self.event_sender.clone(),
            ));
            *self.memory_bridge.write() = Some(bridge);
            info!("Memory Bridge initialized");
        }
        
        // Initialize Narrative Generator
        if self.config.enable_narrative {
            let narrative = Arc::new(NarrativeGenerator::new(
                self.brain.clone(),
                self.event_sender.clone(),
            ));
            *self.narrative_generator.write() = Some(narrative);
            info!("Narrative Generator initialized");
        }
        
        // Initialize Attention Router
        if self.config.enable_attention {
            let attention = Arc::new(AttentionRouter::new(
                self.brain.clone(),
                self.event_sender.clone(),
            ));
            *self.attention_router.write() = Some(attention);
            info!("Attention Router initialized");
        }
        
        // Initialize Dreaming Loop
        if self.config.enable_dreaming {
            let dreaming = Arc::new(DreamingLoop::new(
                self.brain.clone(),
                self.event_sender.clone(),
            ));
            *self.dreaming_loop.write() = Some(dreaming);
            info!("Dreaming Loop initialized");
        }
        
        // Initialize Genetics System
        if self.config.enable_genetics {
            use crate::genetics::GeneticConfig;
            let genetic_config = GeneticConfig {
                mutation_rate: self.config.genetic_mutation_rate,
                crossover_rate: 0.7,
                population_size: 50,
                selection_pressure: 0.5,
                enable_evolution: true,
            };
            let genetic_system = Arc::new(GeneticSystem::new(genetic_config));
            
            // Create trait calculator
            let trait_calculator = Arc::new(TraitCalculator::new(
                genetic_system.clone(),
                self.config.trait_environmental_weight,
            ));
            
            // Set genetics in brain
            self.brain.set_genetics(genetic_system.clone(), trait_calculator);
            
            *self.genetics_system.write() = Some(genetic_system);
            info!("Genetics System initialized");
        }
        
        // Initialize Talking Cricket (optional moral guide)
        if self.config.enable_talking_cricket {
            let tc_config = TalkingCricketConfig {
                llm_enabled: self.config.talking_cricket_llm_enabled,
                veto_threshold: self.config.talking_cricket_veto_threshold,
                evolution_frequency: self.config.talking_cricket_evolution_frequency,
                principles_table: "talking_cricket_principles".to_string(),
            };
            
            let talking_cricket = TalkingCricket::new(self.brain.clone(), tc_config);
            
            // Set trait calculator and genetic system if available
            if let Some(trait_calc) = self.brain.get_trait_calculator() {
                talking_cricket.set_trait_calculator(trait_calc);
            }
            if let Some(genetic_sys) = self.genetics_system.read().as_ref().map(|g| g.clone()) {
                talking_cricket.set_genetic_system(genetic_sys);
            }
            
            // Load principles from database
            let tc_arc = Arc::new(talking_cricket);
            if let Err(e) = tc_arc.load_principles_from_db().await {
                warn!("Failed to load Talking Cricket principles: {}", e);
            }
            
            *self.talking_cricket.write() = Some(tc_arc.clone());
            tc_arc.attach_to_cpl()?;
            info!("Talking Cricket initialized");
        }
        
        // Load persisted state if available
        if self.config.enable_persistence {
            if let Err(e) = self.load_state().await {
                warn!("Failed to load persisted state: {}", e);
            }
        }
        
        info!("CPL {} initialized successfully", self.id);
        Ok(())
    }
    
    /// Start the persistent loop
    /// Note: This should be called with Arc<Self> for proper Send semantics
    pub async fn start(self: Arc<Self>) -> Result<()> {
        // Validate state
        if *self.is_running.read() {
            return Err(Error::Storage("CPL is already running".to_string()));
        }
        
        // Validate config before starting
        if self.config.loop_interval_ms == 0 {
            return Err(Error::Storage("Loop interval must be > 0".to_string()));
        }
        
        *self.is_running.write() = true;
        info!("Starting CPL {}", self.id);
        
        let interval_duration = Duration::from_millis(self.config.loop_interval_ms);
        let mut interval_timer = interval(interval_duration);
        
        // Spawn the main loop
        let cpl_for_loop = self.clone();
        tokio::spawn(async move {
            cpl_for_loop.run_loop(interval_timer).await;
        });
        
        Ok(())
    }
    
    /// Stop the persistent loop
    pub async fn stop(&self) -> Result<()> {
        *self.is_running.write() = false;
        info!("Stopping CPL {}", self.id);
        
        // Persist state before stopping
        if self.config.enable_persistence {
            if let Err(e) = self.save_state().await {
                error!("Failed to persist state on stop: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Attach Talking Cricket to this CPL
    pub async fn attach_talking_cricket(&self, tc: Arc<TalkingCricket>) -> Result<()> {
        // Set trait calculator and genetic system if available
        if let Some(trait_calc) = self.brain.get_trait_calculator() {
            tc.set_trait_calculator(trait_calc);
        }
        if let Some(genetic_sys) = self.genetics_system.read().as_ref().map(|g| g.clone()) {
            tc.set_genetic_system(genetic_sys);
        }
        
        // Load principles
        if let Err(e) = tc.load_principles_from_db().await {
            warn!("Failed to load Talking Cricket principles: {}", e);
        }
        
        *self.talking_cricket.write() = Some(tc.clone());
        tc.attach_to_cpl()?;
        info!("Talking Cricket attached to CPL {}", self.id);
        Ok(())
    }
    
    /// Detach Talking Cricket from this CPL
    pub async fn detach_talking_cricket(&self) -> Result<()> {
        if let Some(tc) = self.talking_cricket.read().as_ref().map(|tc| tc.clone()) {
            tc.detach_from_cpl()?;
            // Save principles before detaching
            if let Err(e) = tc.save_principles_to_db().await {
                warn!("Failed to save Talking Cricket principles: {}", e);
            }
        }
        *self.talking_cricket.write() = None;
        info!("Talking Cricket detached from CPL {}", self.id);
        Ok(())
    }
    
    /// Main loop execution
    async fn run_loop(&self, mut interval_timer: tokio::time::Interval) {
        while *self.is_running.read() {
            interval_timer.tick().await;
            
            let iteration = {
                let mut count = self.loop_count.write();
                *count += 1;
                *count
            };
            
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            debug!("CPL {} iteration {}", self.id, iteration);
            
            // Emit loop iteration event (ignore send errors - subscribers may have dropped)
            let _ = self.event_sender.send(CPLEvent::LoopIteration {
                iteration,
                timestamp: now,
            });
            
            // Edge case: Prevent infinite loops - cap iteration count
            if iteration > 1_000_000 {
                warn!("CPL {} reached maximum iterations, stopping", self.id);
                *self.is_running.write() = false;
                break;
            }
            
            // Execute cognitive systems in order
            // Note: We clone references to avoid holding locks across await points
            
            // 0. Genetics Processing (trait recalculation, periodic evolution)
            {
                let genetics_opt = {
                    let guard = self.genetics_system.read();
                    guard.as_ref().map(|g| g.clone())
                };
                if let Some(genetics) = genetics_opt {
                    // Recalculate traits from genes + environment
                    if let Some(calc) = self.brain.get_trait_calculator() {
                        if let Err(e) = calc.recalculate_all() {
                            warn!("Trait recalculation error: {}", e);
                        }
                    }
                    
                    // Periodic evolution
                    if iteration % self.config.evolution_frequency == 0 {
                        if let Err(e) = genetics.evolve() {
                            warn!("Evolution error: {}", e);
                        }
                    }
                }
            }
            
            // 1. Background Daemon (unconscious processes)
            {
                let daemon_opt = {
                    let guard = self.background_daemon.read();
                    guard.as_ref().map(|d| d.clone())
                };
                if let Some(daemon) = daemon_opt {
                    if let Err(e) = daemon.process().await {
                        warn!("Background daemon error: {}", e);
                    }
                }
            }
            
            // 2. Attention Router (allocate attention)
            {
                let attention_opt = {
                    let guard = self.attention_router.read();
                    guard.as_ref().map(|a| a.clone())
                };
                if let Some(attention) = attention_opt {
                    if let Err(e) = attention.route_attention().await {
                        warn!("Attention router error: {}", e);
                    }
                }
            }
            
            // 2.5. Talking Cricket (moral assessment - optional)
            {
                let tc_opt = {
                    let guard = self.talking_cricket.read();
                    guard.as_ref().map(|tc| tc.clone())
                };
                if let Some(tc) = tc_opt {
                    // Periodic principle evolution
                    if iteration % self.config.talking_cricket_evolution_frequency == 0 {
                        if let Err(e) = tc.evolve_principles().await {
                            warn!("Talking Cricket evolution error: {}", e);
                        }
                    }
                    // Note: Actual action assessment happens in motor interface
                    // This is just for periodic evolution
                }
            }
            
            // 3. Global Workspace (conscious broadcast)
            {
                let gw_opt = {
                    let guard = self.global_workspace.read();
                    guard.as_ref().map(|g| g.clone())
                };
                if let Some(gw) = gw_opt {
                    if let Err(e) = gw.process_broadcast().await {
                        warn!("Global workspace error: {}", e);
                    }
                }
            }
            
            // 4. Working Memory (scratchpad updates)
            if let Err(e) = self.working_memory.update().await {
                warn!("Working memory error: {}", e);
            }
            
            // 5. Memory Bridge (episodic-semantic conversion)
            {
                let bridge_opt = {
                    let guard = self.memory_bridge.read();
                    guard.as_ref().map(|b| b.clone())
                };
                if let Some(bridge) = bridge_opt {
                    if let Err(e) = bridge.process_bridge().await {
                        warn!("Memory bridge error: {}", e);
                    }
                }
            }
            
            // 6. Narrative Generator (sense of self)
            {
                let narrative_opt = {
                    let guard = self.narrative_generator.read();
                    guard.as_ref().map(|n| n.clone())
                };
                if let Some(narrative) = narrative_opt {
                    if let Err(e) = narrative.update_narrative().await {
                        warn!("Narrative generator error: {}", e);
                    }
                }
            }
            
            // 7. Dreaming Loop (offline replay, less frequent)
            if iteration % 10 == 0 {
                let dreaming_opt = {
                    let guard = self.dreaming_loop.read();
                    guard.as_ref().map(|d| d.clone())
                };
                if let Some(dreaming) = dreaming_opt {
                    if let Err(e) = dreaming.replay_experiences().await {
                        warn!("Dreaming loop error: {}", e);
                    }
                }
            }
            
            // Periodic persistence
            if self.config.enable_persistence {
                let should_persist = {
                    let last_persist = *self.last_persist.read();
                    now.saturating_sub(last_persist) > 60
                };
                
                if should_persist {
                    // Persist every minute
                    if let Err(e) = self.save_state().await {
                        error!("Failed to persist state: {}", e);
                    } else {
                        *self.last_persist.write() = now;
                    }
                }
            }
        }
        
        info!("CPL {} loop stopped", self.id);
    }
    
    
    /// Save CPL state to disk
    async fn save_state(&self) -> Result<()> {
        if let Some(ref path) = self.persistence_path {
            // Create directory if it doesn't exist
            if let Err(e) = tokio::fs::create_dir_all(path).await {
                return Err(Error::Storage(format!("Failed to create persistence directory: {}", e)));
            }
            
            // Save state (simplified - would serialize full state)
            // SECURITY: Prevent path traversal attacks
            use crate::security_utils::SecurityUtils;
            let safe_id = self.id.replace("..", "").replace("/", "_").replace("\\", "_");
            let state_file = format!("{}/cpl_{}.state", path, safe_id);
            
            // SECURITY: Validate path to prevent directory traversal
            let state_path = std::path::Path::new(&state_file);
            if let Some(parent) = state_path.parent() {
                if let Err(e) = SecurityUtils::validate_path(parent, &safe_id) {
                    return Err(Error::Storage(format!("Invalid persistence path: {}", e)));
                }
            }
            // Get genome for persistence
            let genome = if let Some(genetics) = self.genetics_system.read().as_ref() {
                Some(genetics.get_genome())
            } else {
                None
            };
            
            let state = CPLState {
                id: self.id.clone(),
                loop_count: *self.loop_count.read(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                genome,
            };
            
            let state_json = serde_json::to_string(&state)
                .map_err(|e| Error::Serialization(format!("Failed to serialize state: {}", e)))?;
            
            tokio::fs::write(&state_file, state_json).await
                .map_err(|e| Error::Storage(format!("Failed to write state file: {}", e)))?;
            
            debug!("Saved CPL state to {}", state_file);
        }
        
        Ok(())
    }
    
    /// Load CPL state from disk
    async fn load_state(&self) -> Result<()> {
        if let Some(ref path) = self.persistence_path {
            // SECURITY: Prevent path traversal attacks
            use crate::security_utils::SecurityUtils;
            let safe_id = self.id.replace("..", "").replace("/", "_").replace("\\", "_");
            let state_file = format!("{}/cpl_{}.state", path, safe_id);
            
            // SECURITY: Validate path to prevent directory traversal
            let state_path = std::path::Path::new(&state_file);
            if let Some(parent) = state_path.parent() {
                if let Err(e) = SecurityUtils::validate_path(parent, &safe_id) {
                    return Err(Error::Storage(format!("Invalid persistence path: {}", e)));
                }
            }
            
            if let Ok(state_json) = tokio::fs::read_to_string(&state_file).await {
                let state: CPLState = serde_json::from_str(&state_json)
                    .map_err(|e| Error::Deserialization(format!("Failed to deserialize state: {}", e)))?;
                
                *self.loop_count.write() = state.loop_count;
                
                // Restore genome if available
                if let Some(genome) = state.genome {
                    if self.config.enable_genetics {
                        use crate::genetics::GeneticConfig;
                        let genetic_config = GeneticConfig {
                            mutation_rate: self.config.genetic_mutation_rate,
                            crossover_rate: 0.7,
                            population_size: 50,
                            selection_pressure: 0.5,
                            enable_evolution: true,
                        };
                        let genetic_system = Arc::new(GeneticSystem::from_genome(genome, genetic_config));
                        
                        // Create trait calculator
                        let trait_calculator = Arc::new(TraitCalculator::new(
                            genetic_system.clone(),
                            self.config.trait_environmental_weight,
                        ));
                        
                        // Set genetics in brain
                        self.brain.set_genetics(genetic_system.clone(), trait_calculator);
                        
                        *self.genetics_system.write() = Some(genetic_system);
                        
                        // Recalculate traits from loaded genome + current environment
                        if let Some(calc) = self.brain.get_trait_calculator() {
                            if let Err(e) = calc.recalculate_all() {
                                warn!("Failed to recalculate traits on load: {}", e);
                            }
                        }
                        
                        debug!("Restored genome from persisted state");
                    }
                }
                
                debug!("Loaded CPL state from {}", state_file);
            }
        }
        
        Ok(())
    }
    
    /// Get CPL ID
    pub fn id(&self) -> &str {
        &self.id
    }
    
    /// Get brain reference
    pub fn brain(&self) -> &Arc<CognitiveBrain> {
        &self.brain
    }
    
    /// Get working memory
    pub fn working_memory(&self) -> &Arc<WorkingMemoryScratchpad> {
        &self.working_memory
    }
    
    /// Get event receiver for CPL events
    pub fn subscribe_events(&self) -> broadcast::Receiver<CPLEvent> {
        self.event_sender.subscribe()
    }
    
    /// Check if CPL is running
    pub fn is_running(&self) -> bool {
        *self.is_running.read()
    }
}

/// CPL state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CPLState {
    id: String,
    loop_count: u64,
    timestamp: u64,
    genome: Option<crate::genetics::Genome>, // Persist genome
}

