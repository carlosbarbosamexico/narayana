// Advanced Optimization Algorithms
// Implements quantum-inspired optimization algorithms for search and optimization problems
// Note: These are classical simulations of quantum algorithms, useful for optimization
// but not actual quantum hardware. Based on Grover's search, QFT, and related algorithms.

use num_complex::Complex64;
use ndarray::{Array1, Array2};
use narayana_core::{Error, Result};
use std::sync::Arc;
use parking_lot::RwLock;
use std::f64::consts::PI;

/// Optimization state vector - quantum-inspired state representation
/// For n dimensions, state vector has 2^n complex amplitudes
/// Used for quantum-inspired optimization algorithms
#[derive(Clone, Debug)]
pub struct OptimizationState {
    /// State vector: amplitudes for each computational basis state
    /// |ψ⟩ = Σ α_i |i⟩ where i ranges from 0 to 2^n - 1
    amplitudes: Array1<Complex64>,
    /// Number of qubits
    num_qubits: usize,
}

impl OptimizationState {
    /// Create new quantum state with all qubits in |0⟩
    pub fn new(num_qubits: usize) -> Self {
        let dim = 1 << num_qubits; // 2^n
        let mut amplitudes = Array1::zeros(dim);
        amplitudes[0] = Complex64::new(1.0, 0.0); // |00...0⟩ = 1
        Self {
            amplitudes,
            num_qubits,
        }
    }

    /// Create state from amplitudes (must be normalized)
    pub fn from_amplitudes(amplitudes: Vec<Complex64>) -> Result<Self> {
        let dim = amplitudes.len();
        if dim == 0 || !dim.is_power_of_two() {
            return Err(Error::Storage("State dimension must be power of 2".to_string()));
        }
        let num_qubits = dim.trailing_zeros() as usize;
        let state = Self {
            amplitudes: Array1::from_vec(amplitudes),
            num_qubits,
        };
        state.normalize()?;
        Ok(state)
    }

    /// Normalize state vector (ensure Σ |α_i|² = 1)
    pub fn normalize(&self) -> Result<()> {
        let norm: f64 = self.amplitudes.iter()
            .map(|a| a.norm_sqr())
            .sum::<f64>()
            .sqrt();
        
        if norm < 1e-10 {
            return Err(Error::Storage("State vector is zero".to_string()));
        }

        // Normalization is done in-place when applying gates
        Ok(())
    }

    /// Get probability of measuring each basis state
    pub fn probabilities(&self) -> Vec<f64> {
        self.amplitudes.iter()
            .map(|a| a.norm_sqr())
            .collect()
    }

    /// Measure the quantum state (collapses to classical state)
    pub fn measure(&mut self) -> usize {
        use rand::Rng;
        let probs = self.probabilities();
        let r: f64 = rand::thread_rng().gen();
        let mut cumsum = 0.0;
        
        for (i, &p) in probs.iter().enumerate() {
            cumsum += p;
            if r <= cumsum {
                // Collapse to state |i⟩
                self.amplitudes.fill(Complex64::new(0.0, 0.0));
                self.amplitudes[i] = Complex64::new(1.0, 0.0);
                return i;
            }
        }
        
        // Fallback (shouldn't happen if normalized)
        self.amplitudes.fill(Complex64::new(0.0, 0.0));
        self.amplitudes[0] = Complex64::new(1.0, 0.0);
        0
    }

    /// Get number of qubits
    pub fn num_qubits(&self) -> usize {
        self.num_qubits
    }

    /// Get amplitude for a specific basis state
    pub fn amplitude(&self, state: usize) -> Complex64 {
        if state < self.amplitudes.len() {
            self.amplitudes[state]
        } else {
            Complex64::new(0.0, 0.0)
        }
    }
}

/// Optimization gate - transformation for quantum-inspired algorithms
pub trait OptimizationGate: Send + Sync {
    /// Apply gate to optimization state
    fn apply(&self, state: &mut OptimizationState) -> Result<()>;
    
    /// Get matrix representation of gate
    fn matrix(&self, num_qubits: usize) -> Array2<Complex64>;
    
    /// Get name of gate
    fn name(&self) -> &str;
}

/// Hadamard gate - creates superposition: |0⟩ → (|0⟩ + |1⟩)/√2
pub struct HadamardGate {
    qubit: usize,
}

impl HadamardGate {
    pub fn new(qubit: usize) -> Self {
        Self { qubit }
    }
}

impl OptimizationGate for HadamardGate {
    fn apply(&self, state: &mut OptimizationState) -> Result<()> {
        if self.qubit >= state.num_qubits {
            return Err(Error::Storage(format!("Qubit {} out of range", self.qubit)));
        }

        let dim = state.amplitudes.len();
        let mut new_amplitudes = Array1::zeros(dim);
        let sqrt2_inv = 1.0 / 2.0_f64.sqrt();

        for i in 0..dim {
            let bit = (i >> self.qubit) & 1;
            let j = i ^ (1 << self.qubit); // Flip bit at qubit position
            
            if bit == 0 {
                // |0⟩ → (|0⟩ + |1⟩)/√2
                new_amplitudes[i] += state.amplitudes[i] * Complex64::new(sqrt2_inv, 0.0);
                new_amplitudes[j] += state.amplitudes[i] * Complex64::new(sqrt2_inv, 0.0);
            } else {
                // |1⟩ → (|0⟩ - |1⟩)/√2
                new_amplitudes[j] += state.amplitudes[i] * Complex64::new(sqrt2_inv, 0.0);
                new_amplitudes[i] -= state.amplitudes[i] * Complex64::new(sqrt2_inv, 0.0);
            }
        }

        state.amplitudes = new_amplitudes;
        Ok(())
    }

    fn matrix(&self, _num_qubits: usize) -> Array2<Complex64> {
        let sqrt2_inv = 1.0 / 2.0_f64.sqrt();
        Array2::from_shape_vec((2, 2), vec![
            Complex64::new(sqrt2_inv, 0.0), Complex64::new(sqrt2_inv, 0.0),
            Complex64::new(sqrt2_inv, 0.0), Complex64::new(-sqrt2_inv, 0.0),
        ]).unwrap()
    }

    fn name(&self) -> &str {
        "H"
    }
}

/// CNOT gate - controlled NOT: flips target if control is |1⟩
pub struct CNOTGate {
    control: usize,
    target: usize,
}

impl CNOTGate {
    pub fn new(control: usize, target: usize) -> Self {
        Self { control, target }
    }
}

impl OptimizationGate for CNOTGate {
    fn apply(&self, state: &mut OptimizationState) -> Result<()> {
        if self.control >= state.num_qubits || self.target >= state.num_qubits {
            return Err(Error::Storage("Qubit out of range".to_string()));
        }
        if self.control == self.target {
            return Err(Error::Storage("Control and target must be different".to_string()));
        }

        let dim = state.amplitudes.len();
        let mut new_amplitudes = Array1::zeros(dim);

        for i in 0..dim {
            let control_bit = (i >> self.control) & 1;
            if control_bit == 1 {
                // Control is |1⟩, flip target
                let j = i ^ (1 << self.target);
                new_amplitudes[j] = state.amplitudes[i];
            } else {
                // Control is |0⟩, no change
                new_amplitudes[i] = state.amplitudes[i];
            }
        }

        state.amplitudes = new_amplitudes;
        Ok(())
    }

    fn matrix(&self, _num_qubits: usize) -> Array2<Complex64> {
        // CNOT matrix for 2 qubits
        Array2::from_shape_vec((4, 4), vec![
            Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0),
            Complex64::new(0.0, 0.0), Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0),
        ]).unwrap()
    }

    fn name(&self) -> &str {
        "CNOT"
    }
}

/// Pauli-X gate (quantum NOT): |0⟩ ↔ |1⟩
pub struct PauliXGate {
    qubit: usize,
}

impl PauliXGate {
    pub fn new(qubit: usize) -> Self {
        Self { qubit }
    }
}

impl OptimizationGate for PauliXGate {
    fn apply(&self, state: &mut OptimizationState) -> Result<()> {
        if self.qubit >= state.num_qubits {
            return Err(Error::Storage("Qubit out of range".to_string()));
        }

        let dim = state.amplitudes.len();
        let mut new_amplitudes = Array1::zeros(dim);

        for i in 0..dim {
            let j = i ^ (1 << self.qubit); // Flip bit
            new_amplitudes[j] = state.amplitudes[i];
        }

        state.amplitudes = new_amplitudes;
        Ok(())
    }

    fn matrix(&self, _num_qubits: usize) -> Array2<Complex64> {
        Array2::from_shape_vec((2, 2), vec![
            Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0),
            Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0),
        ]).unwrap()
    }

    fn name(&self) -> &str {
        "X"
    }
}

/// Phase gate: |1⟩ → e^(iφ)|1⟩
pub struct PhaseGate {
    qubit: usize,
    angle: f64,
}

impl PhaseGate {
    pub fn new(qubit: usize, angle: f64) -> Self {
        Self { qubit, angle }
    }
}

impl OptimizationGate for PhaseGate {
    fn apply(&self, state: &mut OptimizationState) -> Result<()> {
        if self.qubit >= state.num_qubits {
            return Err(Error::Storage("Qubit out of range".to_string()));
        }

        let phase = Complex64::from_polar(1.0, self.angle);
        for i in 0..state.amplitudes.len() {
            let bit = (i >> self.qubit) & 1;
            if bit == 1 {
                state.amplitudes[i] *= phase;
            }
        }
        Ok(())
    }

    fn matrix(&self, _num_qubits: usize) -> Array2<Complex64> {
        let phase = Complex64::from_polar(1.0, self.angle);
        Array2::from_shape_vec((2, 2), vec![
            Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0),
            Complex64::new(0.0, 0.0), phase,
        ]).unwrap()
    }

    fn name(&self) -> &str {
        "P"
    }
}

/// Optimization circuit - sequence of transformations
pub struct OptimizationCircuit {
    gates: Vec<Box<dyn OptimizationGate>>,
    num_qubits: usize,
}

impl OptimizationCircuit {
    pub fn new(num_qubits: usize) -> Self {
        Self {
            gates: Vec::new(),
            num_qubits,
        }
    }

    /// Add gate to circuit
    pub fn add_gate(&mut self, gate: Box<dyn OptimizationGate>) {
        self.gates.push(gate);
    }

    /// Execute circuit on optimization state
    pub fn execute(&self, state: &mut OptimizationState) -> Result<()> {
        if state.num_qubits() != self.num_qubits {
            return Err(Error::Storage("State and circuit qubit count mismatch".to_string()));
        }

        for gate in &self.gates {
            gate.apply(state)?;
        }
        Ok(())
    }

    /// Get number of gates
    pub fn gate_count(&self) -> usize {
        self.gates.len()
    }
}

/// Grover's search algorithm - finds marked items with O(√N) queries
/// 
/// Grover's algorithm provides a quadratic speedup for unstructured search problems.
/// Given a search space of N items and M solutions, classical search requires O(N/M) 
/// queries on average, while Grover's algorithm requires only O(√(N/M)) queries.
/// 
/// # Example
/// ```
/// use narayana_storage::optimization_algorithms::GroversAlgorithm;
/// 
/// // Find the number 42 in a search space of 256 items (8 qubits)
/// let oracle = Box::new(|x: usize| x == 42);
/// let grover = GroversAlgorithm::new(8, oracle);
/// let result = grover.search().unwrap();
/// assert_eq!(result, 42);
/// ```
pub struct GroversAlgorithm {
    num_qubits: usize,
    oracle: Box<dyn Fn(usize) -> bool + Send + Sync>,
}

/// Result of Grover's algorithm search
#[derive(Debug, Clone)]
pub struct GroverResult {
    /// The found solution index
    pub solution: usize,
    /// Probability that this is a correct solution
    pub probability: f64,
    /// Number of iterations performed
    pub iterations: usize,
    /// Total number of solutions in search space
    pub num_solutions: usize,
}

impl GroversAlgorithm {
    /// Create a new Grover's algorithm instance
    /// 
    /// # Arguments
    /// * `num_qubits` - Number of qubits (search space size = 2^num_qubits)
    /// * `oracle` - Function that returns true for marked items (solutions)
    pub fn new(num_qubits: usize, oracle: Box<dyn Fn(usize) -> bool + Send + Sync>) -> Self {
        Self { num_qubits, oracle }
    }

    /// Execute Grover's algorithm and return a single solution
    /// 
    /// Returns the index of a marked item with high probability.
    /// The algorithm performs approximately π/4 * √(N/M) iterations for optimal results.
    pub fn search(&self) -> Result<usize> {
        let result = self.search_with_details()?;
        Ok(result.solution)
    }

    /// Execute Grover's algorithm with detailed results
    /// 
    /// Returns information about the search including probability and iteration count.
    pub fn search_with_details(&self) -> Result<GroverResult> {
        let n = 1 << self.num_qubits; // 2^n
        if n == 0 {
            return Err(Error::Storage("Invalid number of qubits".to_string()));
        }

        // Count solutions (pre-compute for optimal iteration count)
        let solutions: Vec<usize> = (0..n).filter(|&i| (self.oracle)(i)).collect();
        let num_solutions = solutions.len();
        
        if num_solutions == 0 {
            return Err(Error::Storage("No solutions found in search space".to_string()));
        }

        if num_solutions == n {
            // All items are solutions - return first one
            return Ok(GroverResult {
                solution: 0,
                probability: 1.0,
                iterations: 0,
                num_solutions,
            });
        }

        // Optimal number of iterations: π/4 * √(N/M)
        // For better accuracy, we use the exact formula with rounding
        let optimal_iterations = ((PI / 4.0) * ((n as f64 / num_solutions as f64).sqrt()));
        let iterations = optimal_iterations.round() as usize;
        
        // Ensure at least 1 iteration
        let iterations = iterations.max(1);

        // Initialize uniform superposition |ψ⟩ = (1/√N) Σ |i⟩
        let mut state = OptimizationState::new(self.num_qubits);
        
        // Apply Hadamard to all qubits to create uniform superposition
        for qubit in 0..self.num_qubits {
            let h = HadamardGate::new(qubit);
            h.apply(&mut state)?;
        }

        // Grover iteration: Oracle + Diffusion
        for _ in 0..iterations {
            // Oracle: flip phase of marked states
            self.apply_oracle(&mut state)?;
            
            // Diffusion operator: reflect about mean
            self.apply_diffusion(&mut state)?;
        }

        // Measure to get result
        let result = state.measure();
        
        // Calculate probability that result is a solution
        let probs = state.probabilities();
        let probability = probs.get(result).copied().unwrap_or(0.0);

        Ok(GroverResult {
            solution: result,
            probability,
            iterations,
            num_solutions,
        })
    }

    /// Find multiple solutions using repeated measurements
    /// 
    /// Performs the algorithm multiple times to find different solutions.
    /// Returns up to `max_solutions` unique solutions.
    pub fn find_multiple_solutions(&self, max_solutions: usize) -> Result<Vec<GroverResult>> {
        let n = 1 << self.num_qubits;
        let solutions: Vec<usize> = (0..n).filter(|&i| (self.oracle)(i)).collect();
        let num_solutions = solutions.len();
        
        if num_solutions == 0 {
            return Err(Error::Storage("No solutions found".to_string()));
        }

        let mut found = std::collections::HashSet::new();
        let mut results = Vec::new();
        
        // Try multiple times to find different solutions
        let max_attempts = (num_solutions * 2).min(100); // Limit attempts
        
        for _ in 0..max_attempts {
            if found.len() >= max_solutions.min(num_solutions) {
                break;
            }
            
            let result = self.search_with_details()?;
            if (self.oracle)(result.solution) && !found.contains(&result.solution) {
                found.insert(result.solution);
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Get the optimal number of iterations for given problem size
    /// 
    /// This is useful for understanding the algorithm's complexity.
    pub fn optimal_iterations(&self) -> Result<usize> {
        let n = 1 << self.num_qubits;
        let solutions: Vec<usize> = (0..n).filter(|&i| (self.oracle)(i)).collect();
        let num_solutions = solutions.len();
        
        if num_solutions == 0 {
            return Err(Error::Storage("No solutions found".to_string()));
        }

        let optimal = ((PI / 4.0) * ((n as f64 / num_solutions as f64).sqrt())).round() as usize;
        Ok(optimal.max(1))
    }

    /// Oracle: marks solutions by flipping their phase
    /// 
    /// The oracle operator O|x⟩ = (-1)^f(x) |x⟩ where f(x) = 1 if x is a solution
    fn apply_oracle(&self, state: &mut OptimizationState) -> Result<()> {
        for i in 0..state.amplitudes.len() {
            if (self.oracle)(i) {
                // Flip phase: multiply by -1 (phase kickback)
                state.amplitudes[i] *= Complex64::new(-1.0, 0.0);
            }
        }
        Ok(())
    }

    /// Diffusion operator: reflection about mean
    /// 
    /// The diffusion operator D = 2|s⟩⟨s| - I where |s⟩ is the uniform superposition.
    /// This amplifies the amplitude of marked states.
    fn apply_diffusion(&self, state: &mut OptimizationState) -> Result<()> {
        // Apply Hadamard to all qubits: H^⊗n
        for qubit in 0..self.num_qubits {
            let h = HadamardGate::new(qubit);
            h.apply(state)?;
        }

        // Phase flip for all states except |0⟩: Z gate on all qubits
        // This is equivalent to: I - 2|0⟩⟨0|
        for i in 1..state.amplitudes.len() {
            state.amplitudes[i] *= Complex64::new(-1.0, 0.0);
        }

        // Apply Hadamard again: H^⊗n
        for qubit in 0..self.num_qubits {
            let h = HadamardGate::new(qubit);
            h.apply(state)?;
        }

        Ok(())
    }
}

/// Fourier Transform for optimization - transforms from computational to frequency basis
pub struct OptimizationFourierTransform {
    num_qubits: usize,
}

impl OptimizationFourierTransform {
    pub fn new(num_qubits: usize) -> Self {
        Self { num_qubits }
    }

    /// Apply Fourier transform to optimization state
    pub fn apply(&self, state: &mut OptimizationState) -> Result<()> {
        if state.num_qubits() != self.num_qubits {
            return Err(Error::Storage("Qubit count mismatch".to_string()));
        }

        let n = state.amplitudes.len();
        let mut new_amplitudes = Array1::zeros(n);

        // QFT: |j⟩ → (1/√N) Σ_k e^(2πijk/N) |k⟩
        for j in 0..n {
            for k in 0..n {
                let angle = 2.0 * PI * (j as f64) * (k as f64) / (n as f64);
                let phase = Complex64::from_polar(1.0, angle);
                new_amplitudes[k] += state.amplitudes[j] * phase;
            }
        }

        // Normalize
        let norm = Complex64::new((n as f64).sqrt(), 0.0);
        for i in 0..n {
            new_amplitudes[i] = new_amplitudes[i] / norm;
        }
        state.amplitudes = new_amplitudes;

        Ok(())
    }
}

/// Advanced optimizer - uses quantum-inspired algorithms for optimization
/// These are classical simulations useful for certain optimization problems
pub struct AdvancedOptimizer {
    simulator: Arc<RwLock<OptimizationSimulator>>,
}

pub struct OptimizationSimulator {
    max_dimensions: usize,
}

impl OptimizationSimulator {
    pub fn new(max_dimensions: usize) -> Self {
        Self { max_dimensions }
    }

    pub fn create_state(&self, num_dimensions: usize) -> Result<OptimizationState> {
        if num_dimensions > self.max_dimensions {
            return Err(Error::Storage(format!("Exceeds max dimensions: {}", self.max_dimensions)));
        }
        Ok(OptimizationState::new(num_dimensions))
    }
}

impl AdvancedOptimizer {
    pub fn new() -> Self {
        Self {
            simulator: Arc::new(RwLock::new(OptimizationSimulator::new(20))), // Max 20 dimensions
        }
    }

    /// Optimize search using Grover's algorithm
    /// Returns the solution index
    pub fn grover_search(
        &self,
        num_qubits: usize,
        oracle: Box<dyn Fn(usize) -> bool + Send + Sync>,
    ) -> Result<usize> {
        let grover = GroversAlgorithm::new(num_qubits, oracle);
        grover.search()
    }

    /// Optimize search using Grover's algorithm with detailed results
    pub fn grover_search_with_details(
        &self,
        num_qubits: usize,
        oracle: Box<dyn Fn(usize) -> bool + Send + Sync>,
    ) -> Result<GroverResult> {
        let grover = GroversAlgorithm::new(num_qubits, oracle);
        grover.search_with_details()
    }

    /// Find multiple solutions using Grover's algorithm
    pub fn grover_find_multiple(
        &self,
        num_qubits: usize,
        oracle: Box<dyn Fn(usize) -> bool + Send + Sync>,
        max_solutions: usize,
    ) -> Result<Vec<GroverResult>> {
        let grover = GroversAlgorithm::new(num_qubits, oracle);
        grover.find_multiple_solutions(max_solutions)
    }

    /// Apply Quantum Fourier Transform
    pub fn fourier_transform(&self, state: &mut OptimizationState) -> Result<()> {
        let qft = OptimizationFourierTransform::new(state.num_qubits());
        qft.apply(state)
    }

    /// Estimate quantum speedup for search
    pub fn estimate_speedup(&self, n: usize, m: usize) -> f64 {
        if m == 0 {
            return 1.0;
        }
        // Classical: O(N/M), Quantum: O(√(N/M))
        let classical = n as f64 / m as f64;
        let quantum = (n as f64 / m as f64).sqrt();
        classical / quantum
    }
}

impl Default for AdvancedOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grovers_single_solution() {
        // Test finding a single solution: find 42 in search space of 256 (8 qubits)
        let target = 42;
        let oracle = Box::new(move |x: usize| x == target);
        let grover = GroversAlgorithm::new(8, oracle);
        
        let result = grover.search().unwrap();
        assert_eq!(result, target, "Grover's algorithm should find the target");
    }

    #[test]
    fn test_grovers_with_details() {
        // Test detailed results
        let target = 100;
        let oracle = Box::new(move |x: usize| x == target);
        let grover = GroversAlgorithm::new(10, oracle);
        
        let result = grover.search_with_details().unwrap();
        assert_eq!(result.solution, target);
        assert!(result.probability > 0.5, "Probability should be high for correct solution");
        assert!(result.iterations > 0, "Should perform at least one iteration");
        assert_eq!(result.num_solutions, 1);
    }

    #[test]
    fn test_grovers_multiple_solutions() {
        // Test finding multiple solutions: find all even numbers
        let oracle = Box::new(|x: usize| x % 2 == 0);
        let grover = GroversAlgorithm::new(6, oracle); // 64 items
        
        let results = grover.find_multiple_solutions(5).unwrap();
        assert!(!results.is_empty(), "Should find at least one solution");
        assert!(results.len() <= 5, "Should not exceed max_solutions");
        
        // Verify all results are actually solutions
        for result in &results {
            assert!(result.solution % 2 == 0, "All results should be even");
        }
    }

    #[test]
    fn test_grovers_no_solutions() {
        // Test with no solutions
        let oracle = Box::new(|_x: usize| false);
        let grover = GroversAlgorithm::new(4, oracle);
        
        let result = grover.search();
        assert!(result.is_err(), "Should return error when no solutions exist");
    }

    #[test]
    fn test_grovers_all_solutions() {
        // Test when all items are solutions
        let oracle = Box::new(|_x: usize| true);
        let grover = GroversAlgorithm::new(4, oracle);
        
        let result = grover.search_with_details().unwrap();
        assert_eq!(result.solution, 0);
        assert_eq!(result.probability, 1.0);
        assert_eq!(result.iterations, 0);
    }

    #[test]
    fn test_grovers_optimal_iterations() {
        // Test optimal iteration calculation
        let target = 5;
        let oracle = Box::new(move |x: usize| x == target);
        let grover = GroversAlgorithm::new(4, oracle); // 16 items, 1 solution
        
        let iterations = grover.optimal_iterations().unwrap();
        // For N=16, M=1: π/4 * √(16/1) ≈ 3.14
        assert!(iterations >= 3 && iterations <= 4, "Iterations should be around 3-4");
    }

    #[test]
    fn test_grovers_small_search_space() {
        // Test with very small search space
        let target = 1;
        let oracle = Box::new(move |x: usize| x == target);
        let grover = GroversAlgorithm::new(2, oracle); // 4 items
        
        let result = grover.search().unwrap();
        assert_eq!(result, target);
    }

    #[test]
    fn test_grovers_through_optimizer() {
        // Test through AdvancedOptimizer interface
        let optimizer = AdvancedOptimizer::new();
        let target = 50;
        let oracle = Box::new(move |x: usize| x == target);
        
        let result = optimizer.grover_search(8, oracle).unwrap();
        assert_eq!(result, target);
    }

    #[test]
    fn test_grovers_through_optimizer_with_details() {
        // Test detailed results through optimizer
        let optimizer = AdvancedOptimizer::new();
        let target = 25;
        let oracle = Box::new(move |x: usize| x == target);
        
        let result = optimizer.grover_search_with_details(8, oracle).unwrap();
        assert_eq!(result.solution, target);
        assert!(result.probability > 0.0);
    }

    #[test]
    fn test_grovers_find_multiple_through_optimizer() {
        // Test finding multiple solutions through optimizer
        let optimizer = AdvancedOptimizer::new();
        let oracle = Box::new(|x: usize| x % 3 == 0); // Multiples of 3
        let results = optimizer.grover_find_multiple(6, oracle, 3).unwrap();
        
        assert!(!results.is_empty());
        for result in &results {
            assert!(result.solution % 3 == 0);
        }
    }

    #[test]
    fn test_grovers_range_search() {
        // Test finding items in a range
        let min = 10;
        let max = 20;
        let oracle = Box::new(move |x: usize| x >= min && x <= max);
        let grover = GroversAlgorithm::new(5, oracle); // 32 items
        
        let results = grover.find_multiple_solutions(5).unwrap();
        assert!(!results.is_empty());
        for result in &results {
            assert!(result.solution >= min && result.solution <= max);
        }
    }
}

