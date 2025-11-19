// Genetics System
// Hybrid biological-style genetics with evolutionary optimization
// Supports genes, alleles, inheritance, mutation, crossover, and population-based evolution

use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};
use uuid::Uuid;
use rand::Rng;

/// Allele - variant of a gene
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Allele {
    Dominant,
    Recessive,
}

impl Allele {
    /// Random allele
    pub fn random() -> Self {
        if rand::thread_rng().gen_bool(0.5) {
            Allele::Dominant
        } else {
            Allele::Recessive
        }
    }
    
    /// From string representation
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "D" | "dominant" => Some(Allele::Dominant),
            "r" | "recessive" => Some(Allele::Recessive),
            _ => None,
        }
    }
}

/// Gene - basic genetic unit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gene {
    pub id: String,
    pub name: String,
    pub allele1: Allele,
    pub allele2: Allele,
    pub trait_name: String, // Which trait this gene influences
    pub effect_strength: f64, // How much this gene affects the trait (0.0-1.0)
}

impl Gene {
    /// Create new gene
    pub fn new(name: String, trait_name: String, effect_strength: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            allele1: Allele::random(),
            allele2: Allele::random(),
            trait_name,
            effect_strength: effect_strength.max(0.0).min(1.0),
        }
    }
    
    /// Get expressed value (Mendelian genetics)
    /// Dominant alleles mask recessive ones
    pub fn expressed_value(&self) -> f64 {
        let has_dominant = self.allele1 == Allele::Dominant || self.allele2 == Allele::Dominant;
        
        if has_dominant {
            // Dominant expression
            self.effect_strength
        } else {
            // Recessive expression (weaker)
            self.effect_strength * 0.5
        }
    }
    
    /// Mutate this gene
    pub fn mutate(&mut self, mutation_rate: f64) {
        // SECURITY: Validate mutation rate
        let mutation_rate = mutation_rate.max(0.0).min(1.0);
        if mutation_rate <= 0.0 {
            return;
        }
        
        let mut rng = rand::thread_rng();
        
        if rng.gen_bool(mutation_rate) {
            self.allele1 = Allele::random();
        }
        if rng.gen_bool(mutation_rate) {
            self.allele2 = Allele::random();
        }
        
        // Small chance to change effect strength
        let strength_mutation_rate = (mutation_rate * 0.1).max(0.0).min(1.0);
        if strength_mutation_rate > 0.0 && rng.gen_bool(strength_mutation_rate) {
            let change = rng.gen_range(-0.1..=0.1);
            // SECURITY: Validate effect_strength after mutation
            let new_strength = self.effect_strength + change;
            self.effect_strength = if new_strength.is_nan() || new_strength.is_infinite() {
                self.effect_strength // Keep old value if invalid
            } else {
                new_strength.max(0.0).min(1.0)
            };
        }
    }
}

/// Genome - complete genetic makeup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genome {
    pub id: String,
    pub genes: HashMap<String, Gene>, // gene_name -> Gene
    pub created_at: u64,
    pub generation: u64,
    pub parent_ids: Vec<String>, // IDs of parent genomes (for evolution tracking)
}

impl Genome {
    /// Create new random genome
    pub fn new() -> Self {
        let mut genes = HashMap::new();
        
        // Create default cognitive trait genes
        let trait_genes = vec![
            ("attention_span", "attention_span", 0.7),
            ("memory_capacity", "memory_capacity", 0.8),
            ("curiosity", "curiosity", 0.6),
            ("creativity", "creativity", 0.5),
            ("social_affinity", "social_affinity", 0.6),
            ("risk_taking", "risk_taking", 0.4),
            ("patience", "patience", 0.7),
            ("learning_rate", "learning_rate", 0.8),
            ("moral_sensitivity", "moral_receptivity", 0.6),
        ];
        
        for (gene_name, trait_name, effect) in trait_genes {
            let gene = Gene::new(gene_name.to_string(), trait_name.to_string(), effect);
            genes.insert(gene_name.to_string(), gene);
        }
        
        Self {
            id: Uuid::new_v4().to_string(),
            genes,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            generation: 0,
            parent_ids: Vec::new(),
        }
    }
    
    /// Get genetic contribution to a trait
    pub fn get_trait_genetic_value(&self, trait_name: &str) -> f64 {
        let mut total = 0.0;
        let mut count = 0;
        
        for gene in self.genes.values() {
            if gene.trait_name == trait_name {
                let value = gene.expressed_value();
                // SECURITY: Filter NaN/Inf values
                if value.is_finite() && value >= 0.0 && value <= 1.0 {
                    total += value;
                    count += 1;
                }
            }
        }
        
        // SECURITY: Prevent division by zero
        if count > 0 {
            let result = total / count as f64;
            // SECURITY: Validate result
            if result.is_finite() && result >= 0.0 && result <= 1.0 {
                result
            } else {
                0.5 // Default neutral value if invalid
            }
        } else {
            0.5 // Default neutral value
        }
    }
    
    /// Mutate genome
    pub fn mutate(&mut self, mutation_rate: f64) {
        for gene in self.genes.values_mut() {
            gene.mutate(mutation_rate);
        }
    }
    
    /// Crossover with another genome (genetic recombination)
    pub fn crossover(&self, other: &Genome, crossover_rate: f64) -> Genome {
        // SECURITY: Validate crossover rate
        let crossover_rate = crossover_rate.max(0.0).min(1.0);
        
        let mut rng = rand::thread_rng();
        let mut new_genes = HashMap::new();
        
        // SECURITY: Limit gene count to prevent DoS
        const MAX_GENES: usize = 1000;
        let max_genes = self.genes.len().max(other.genes.len()).min(MAX_GENES);
        
        for (gene_name, gene1) in &self.genes {
            if new_genes.len() >= max_genes {
                break;
            }
            
            if let Some(gene2) = other.genes.get(gene_name) {
                if rng.gen_bool(crossover_rate) {
                    // Take alleles from parent 2
                    let mut new_gene = gene1.clone();
                    new_gene.allele1 = gene2.allele1.clone();
                    new_gene.allele2 = gene2.allele2.clone();
                    new_genes.insert(gene_name.clone(), new_gene);
                } else {
                    // Keep parent 1
                    new_genes.insert(gene_name.clone(), gene1.clone());
                }
            } else {
                // Gene only in parent 1
                new_genes.insert(gene_name.clone(), gene1.clone());
            }
        }
        
        // Add genes only in parent 2
        for (gene_name, gene2) in &other.genes {
            if new_genes.len() >= max_genes {
                break;
            }
            if !new_genes.contains_key(gene_name) {
                new_genes.insert(gene_name.clone(), gene2.clone());
            }
        }
        
        // SECURITY: Prevent generation overflow
        let new_generation = self.generation
            .saturating_add(other.generation)
            .saturating_add(1)
            .min(u64::MAX);
        
        Genome {
            id: Uuid::new_v4().to_string(),
            genes: new_genes,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            generation: new_generation,
            parent_ids: vec![self.id.clone(), other.id.clone()],
        }
    }
}

impl Default for Genome {
    fn default() -> Self {
        Self::new()
    }
}

/// Genetic System - manages genetics and evolution
pub struct GeneticSystem {
    genome: Arc<RwLock<Genome>>,
    population: Arc<RwLock<Vec<Genome>>>,
    fitness_scores: Arc<RwLock<HashMap<String, f64>>>, // genome_id -> fitness
    config: GeneticConfig,
    evolution_history: Arc<RwLock<Vec<EvolutionRecord>>>,
}

/// Configuration for genetic system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneticConfig {
    pub mutation_rate: f64,
    pub crossover_rate: f64,
    pub population_size: usize,
    pub selection_pressure: f64, // How strongly to favor fit individuals (0.0-1.0)
    pub enable_evolution: bool,
}

impl Default for GeneticConfig {
    fn default() -> Self {
        Self {
            mutation_rate: 0.01, // 1% mutation rate
            crossover_rate: 0.7,  // 70% crossover rate
            population_size: 50,
            selection_pressure: 0.5,
            enable_evolution: true,
        }
    }
}

impl GeneticConfig {
    /// Validate and sanitize config values
    pub fn validate(&mut self) {
        // SECURITY: Clamp all rates to valid ranges
        self.mutation_rate = self.mutation_rate.max(0.0).min(1.0);
        self.crossover_rate = self.crossover_rate.max(0.0).min(1.0);
        self.selection_pressure = self.selection_pressure.max(0.0).min(1.0);
        
        // SECURITY: Limit population size to prevent DoS
        const MAX_POPULATION: usize = 10000;
        const MIN_POPULATION: usize = 2;
        self.population_size = self.population_size.max(MIN_POPULATION).min(MAX_POPULATION);
    }
}

/// Evolution record for tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionRecord {
    pub generation: u64,
    pub timestamp: u64,
    pub best_fitness: f64,
    pub average_fitness: f64,
    pub genome_id: String,
}

impl GeneticSystem {
    /// Create new genetic system with random genome
    pub fn new(mut config: GeneticConfig) -> Self {
        // SECURITY: Validate config
        config.validate();
        
        let genome = Genome::new();
        
        // Initialize population
        let mut population = Vec::new();
        for _ in 0..config.population_size {
            population.push(Genome::new());
        }
        
        Self {
            genome: Arc::new(RwLock::new(genome)),
            population: Arc::new(RwLock::new(population)),
            fitness_scores: Arc::new(RwLock::new(HashMap::new())),
            config,
            evolution_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Create from existing genome
    pub fn from_genome(genome: Genome, mut config: GeneticConfig) -> Self {
        // SECURITY: Validate config
        config.validate();
        
        let mut population = Vec::new();
        for _ in 0..config.population_size {
            population.push(genome.clone());
        }
        
        Self {
            genome: Arc::new(RwLock::new(genome)),
            population: Arc::new(RwLock::new(population)),
            fitness_scores: Arc::new(RwLock::new(HashMap::new())),
            config,
            evolution_history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Get current genome
    pub fn get_genome(&self) -> Genome {
        self.genome.read().clone()
    }
    
    /// Get genetic value for a trait
    pub fn get_trait_genetic_value(&self, trait_name: &str) -> f64 {
        self.genome.read().get_trait_genetic_value(trait_name)
    }
    
    /// Set fitness for a genome (used in evolution)
    pub fn set_fitness(&self, genome_id: &str, fitness: f64) {
        // SECURITY: Validate fitness value (filter NaN/Inf)
        let valid_fitness = if fitness.is_finite() {
            fitness.max(0.0) // Allow any positive value
        } else {
            0.0 // Default to 0 for invalid values
        };
        
        // SECURITY: Limit fitness scores map size to prevent DoS
        let mut scores = self.fitness_scores.write();
        if scores.len() < 100000 {
            scores.insert(genome_id.to_string(), valid_fitness);
        } else {
            // Remove oldest entries if map is too large
            let keys: Vec<String> = scores.keys().cloned().take(1000).collect();
            for key in keys {
                scores.remove(&key);
            }
            scores.insert(genome_id.to_string(), valid_fitness);
        }
    }
    
    /// Evolve population (selection, crossover, mutation)
    pub fn evolve(&self) -> Result<()> {
        if !self.config.enable_evolution {
            return Ok(());
        }
        
        let population = self.population.read();
        let fitness_scores = self.fitness_scores.read();
        
        if population.is_empty() {
            return Ok(());
        }
        
        // Calculate fitness for all genomes (use stored or default)
        let mut genome_fitness: Vec<(usize, f64)> = population
            .iter()
            .enumerate()
            .map(|(i, genome)| {
                let fitness = fitness_scores
                    .get(&genome.id)
                    .copied()
                    .unwrap_or(0.5); // Default fitness
                // SECURITY: Filter NaN/Inf fitness values
                let valid_fitness = if fitness.is_finite() && fitness >= 0.0 {
                    fitness
                } else {
                    0.5 // Default if invalid
                };
                (i, valid_fitness)
            })
            .collect();
        
        // SECURITY: Ensure we have valid fitness values
        if genome_fitness.is_empty() {
            return Ok(());
        }
        
        // Sort by fitness (descending)
        genome_fitness.sort_by(|a, b| {
            // SECURITY: Safe comparison with NaN handling
            b.1.partial_cmp(&a.1).unwrap_or_else(|| {
                if a.1.is_nan() && b.1.is_nan() {
                    std::cmp::Ordering::Equal
                } else if a.1.is_nan() {
                    std::cmp::Ordering::Less
                } else if b.1.is_nan() {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            })
        });
        
        // Selection: keep top individuals
        let selection_count = (population.len() as f64 * (1.0 - self.config.selection_pressure)) as usize;
        let selection_count = selection_count.max(2).min(population.len()); // At least 2, at most population size
        
        // SECURITY: Ensure we have enough selected individuals
        if selection_count == 0 || selection_count > genome_fitness.len() {
            return Err(Error::Storage("Invalid selection count".to_string()));
        }
        
        let selected_indices: Vec<usize> = genome_fitness
            .iter()
            .take(selection_count)
            .map(|(i, _)| *i)
            .collect();
        
        drop(population);
        drop(fitness_scores);
        
        // Create new population through crossover and mutation
        let mut new_population = Vec::new();
        let old_population = self.population.read();
        
        // Keep best individuals (elitism)
        for &idx in &selected_indices {
            new_population.push(old_population[idx].clone());
        }
        
        // Generate rest through crossover
        let mut rng = rand::thread_rng();
        // SECURITY: Prevent infinite loop
        let max_iterations = self.config.population_size * 2;
        let mut iterations = 0;
        
        while new_population.len() < self.config.population_size && iterations < max_iterations {
            iterations += 1;
            
            // SECURITY: Validate indices before access
            if selected_indices.is_empty() || old_population.is_empty() {
                break;
            }
            
            let parent1_idx = selected_indices[rng.gen_range(0..selected_indices.len())];
            let parent2_idx = selected_indices[rng.gen_range(0..selected_indices.len())];
            
            // SECURITY: Bounds check
            if parent1_idx >= old_population.len() || parent2_idx >= old_population.len() {
                continue;
            }
            
            let child = old_population[parent1_idx].crossover(
                &old_population[parent2_idx],
                self.config.crossover_rate,
            );
            
            // Mutate child
            let mut child = child;
            child.mutate(self.config.mutation_rate);
            
            new_population.push(child);
        }
        
        // SECURITY: Ensure we have a valid population
        if new_population.is_empty() {
            return Err(Error::Storage("Failed to generate new population".to_string()));
        }
        
        drop(old_population);
        
        // Update population
        *self.population.write() = new_population;
        
        // Update best genome
        if let Some((best_idx, _)) = genome_fitness.first() {
            let population = self.population.read();
            // SECURITY: Bounds check
            if *best_idx < population.len() {
                let best_genome = population[*best_idx].clone();
                drop(population);
                *self.genome.write() = best_genome;
            }
        }
        
        // Record evolution
        // SECURITY: Prevent division by zero
        let avg_fitness = if genome_fitness.is_empty() {
            0.0
        } else {
            let sum: f64 = genome_fitness.iter()
                .map(|(_, f)| if f.is_finite() { *f } else { 0.0 })
                .sum();
            let count = genome_fitness.len() as f64;
            if count > 0.0 {
                sum / count
            } else {
                0.0
            }
        };
        
        let best_fitness = genome_fitness.first()
            .map(|(_, f)| if f.is_finite() { *f } else { 0.0 })
            .unwrap_or(0.0);
        
        let record = EvolutionRecord {
            generation: self.genome.read().generation,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            best_fitness,
            average_fitness: avg_fitness,
            genome_id: self.genome.read().id.clone(),
        };
        
        self.evolution_history.write().push(record);
        
        // Keep history bounded
        let mut history = self.evolution_history.write();
        const MAX_HISTORY: usize = 100;
        if history.len() > MAX_HISTORY {
            history.remove(0);
        }
        
        info!("Evolved population: best fitness={:.3}, avg={:.3}", best_fitness, avg_fitness);
        
        Ok(())
    }
    
    /// Get evolution history
    pub fn get_evolution_history(&self) -> Vec<EvolutionRecord> {
        self.evolution_history.read().clone()
    }
    
    /// Get config
    pub fn get_config(&self) -> GeneticConfig {
        self.config.clone()
    }
}

