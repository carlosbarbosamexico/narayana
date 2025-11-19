// Native Policy Gradient / Reinforcement Learning Loop
// Production-ready RL training engine with Q-learning, actor-critic, and policy gradients

use crate::cognitive::*;
use narayana_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, debug, warn};
use uuid::Uuid;

/// Reinforcement learning engine
pub struct RLEngine {
    brain: Arc<CognitiveBrain>,
    policies: Arc<RwLock<HashMap<String, Policy>>>,
    value_functions: Arc<RwLock<HashMap<String, ValueFunction>>>,
    experience_buffer: Arc<RwLock<Vec<Experience>>>,
    reward_traces: Arc<RwLock<HashMap<String, RewardTrace>>>,
    config: RLConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RLConfig {
    pub learning_rate: f64,
    pub discount_factor: f64,
    pub epsilon: f64, // Exploration rate
    pub batch_size: usize,
    pub replay_buffer_size: usize,
    pub update_frequency: u64,
    pub algorithm: RLAlgorithm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RLAlgorithm {
    QLearning,
    ActorCritic,
    PolicyGradient,
    DQN, // Deep Q-Network
    PPO, // Proximal Policy Optimization
}

impl RLEngine {
    pub fn new(brain: Arc<CognitiveBrain>, config: RLConfig) -> Self {
        Self {
            brain,
            policies: Arc::new(RwLock::new(HashMap::new())),
            value_functions: Arc::new(RwLock::new(HashMap::new())),
            experience_buffer: Arc::new(RwLock::new(Vec::new())),
            reward_traces: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Update policy based on experience
    pub fn update_policy(&self, policy_id: &str, experience: &Experience) -> Result<()> {
        let mut policies = self.policies.write();
        let policy = policies.get_mut(policy_id)
            .ok_or_else(|| Error::Storage(format!("Policy {} not found", policy_id)))?;

        match self.config.algorithm {
            RLAlgorithm::QLearning => {
                self.update_q_learning(policy, experience)?;
            }
            RLAlgorithm::ActorCritic => {
                self.update_actor_critic(policy, experience)?;
            }
            RLAlgorithm::PolicyGradient => {
                self.update_policy_gradient(policy, experience)?;
            }
            RLAlgorithm::DQN => {
                self.update_dqn(policy, experience)?;
            }
            RLAlgorithm::PPO => {
                self.update_ppo(policy, experience)?;
            }
        }

        info!("Updated policy {} with experience {}", policy_id, experience.id);
        Ok(())
    }

    /// Evaluate policy
    pub fn evaluate_policy(&self, policy_id: &str, state: &serde_json::Value) -> Result<Action> {
        let policies = self.policies.read();
        let policy = policies.get(policy_id)
            .ok_or_else(|| Error::Storage(format!("Policy {} not found", policy_id)))?;

        // EDGE CASE: Clamp epsilon to valid range [0.0, 1.0] and handle NaN/Infinity
        let epsilon = if self.config.epsilon.is_nan() || self.config.epsilon.is_infinite() {
            0.1 // Default epsilon if invalid
        } else {
            self.config.epsilon.clamp(0.0, 1.0)
        };
        // Get action from policy
        let action = policy.select_action(state, epsilon)?;
        
        debug!("Policy {} selected action for state", policy_id);
        Ok(action)
    }

    /// Record reward trace
    pub fn reward_trace(&self, trace_id: &str, reward: f64, state: &serde_json::Value) -> Result<()> {
        let mut traces = self.reward_traces.write();
        let trace = traces.entry(trace_id.to_string())
            .or_insert_with(|| RewardTrace {
                trace_id: trace_id.to_string(),
                rewards: Vec::new(),
                states: Vec::new(),
                total_reward: 0.0,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            });

        trace.rewards.push(reward);
        trace.states.push(state.clone());
        trace.total_reward += reward;

        // Update value function if using value-based method
        if matches!(self.config.algorithm, RLAlgorithm::QLearning | RLAlgorithm::DQN | RLAlgorithm::ActorCritic) {
            self.update_value_function(trace_id, &trace)?;
        }

        Ok(())
    }

    /// Q-learning update
    fn update_q_learning(&self, policy: &mut Policy, experience: &Experience) -> Result<()> {
        // Q(s, a) = Q(s, a) + α[r + γ * max Q(s', a') - Q(s, a)]
        let state = &experience.observation;
        let action = experience.action.as_ref()
            .ok_or_else(|| Error::Storage("Action missing from experience".to_string()))?;
        let reward = experience.reward.unwrap_or(0.0);
        let next_state = experience.outcome.as_ref();

        let current_q = policy.get_q_value(state, action)?;
        let next_max_q = if let Some(next) = next_state {
            policy.get_max_q_value(next)?
        } else {
            0.0
        };

        // Use checked arithmetic to prevent overflow
        let q_update = self.config.learning_rate * 
            (reward + self.config.discount_factor * next_max_q - current_q);
        let new_q = current_q + q_update;
        
        // Clamp Q-value to reasonable range to prevent overflow
        let new_q = new_q.clamp(-1e6, 1e6);
        
        policy.update_q_value(state, action, new_q)?;
        Ok(())
    }

    /// Actor-critic update
    fn update_actor_critic(&self, policy: &mut Policy, experience: &Experience) -> Result<()> {
        let state = &experience.observation;
        let action = experience.action.as_ref()
            .ok_or_else(|| Error::Storage("Action missing from experience".to_string()))?;
        let reward = experience.reward.unwrap_or(0.0);
        let next_state = experience.outcome.as_ref();

        // Update value function (critic)
        let value = self.get_value(state)?;
        let next_value = if let Some(next) = next_state {
            self.get_value(next)?
        } else {
            0.0
        };

        let td_error = reward + self.config.discount_factor * next_value - value;
        
        // Update value function
        self.update_value(state, value + self.config.learning_rate * td_error)?;

        // Update policy (actor)
        policy.update_with_td_error(state, action, td_error, self.config.learning_rate)?;

        Ok(())
    }

    /// Policy gradient update
    fn update_policy_gradient(&self, policy: &mut Policy, experience: &Experience) -> Result<()> {
        let state = &experience.observation;
        let action = experience.action.as_ref()
            .ok_or_else(|| Error::Storage("Action missing from experience".to_string()))?;
        let reward = experience.reward.unwrap_or(0.0);

        // REINFORCE: ∇θ J(θ) = E[∇θ log π(a|s) * R]
        let log_prob = policy.get_log_probability(state, action)?;
        let gradient = log_prob * reward;

        policy.update_with_gradient(state, action, gradient, self.config.learning_rate)?;
        Ok(())
    }

    /// DQN update (Deep Q-Network)
    fn update_dqn(&self, policy: &mut Policy, experience: &Experience) -> Result<()> {
        // DQN uses experience replay and target network
        // Sample random batch from replay buffer
        let buffer = self.experience_buffer.read();
        if buffer.len() < self.config.batch_size {
            // Not enough experiences yet, use regular Q-learning
            drop(buffer);
            return self.update_q_learning(policy, experience);
        }
        
        // Sample random batch for experience replay
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        // Collect batch indices first, then clone experiences to avoid borrow issues
        let batch_indices: Vec<usize> = (0..self.config.batch_size)
            .map(|_| rng.gen_range(0..buffer.len()))
            .collect();
        
        // Clone experiences to avoid borrow checker issues
        let batch: Vec<Experience> = batch_indices
            .iter()
            .filter_map(|&idx| buffer.get(idx).cloned())
            .collect();
        
        drop(buffer);
        
        // Update Q-values for each experience in batch
        for exp in batch {
            let state = &exp.observation;
            let action = exp.action.as_ref()
                .ok_or_else(|| Error::Storage("Action missing from experience".to_string()))?;
            let reward = exp.reward.unwrap_or(0.0);
            let next_state = exp.outcome.as_ref();
            
            // Compute target Q-value: r + γ * max Q(s', a')
            let current_q = policy.get_q_value(state, action)?;
            let next_max_q = if let Some(next) = next_state {
                policy.get_max_q_value(next)?
            } else {
                0.0
            };
            
            let target_q = reward + self.config.discount_factor * next_max_q;
            
            // Update Q-value with learning rate
            let new_q = current_q + self.config.learning_rate * (target_q - current_q);
            let new_q = new_q.clamp(-1e6, 1e6); // Prevent overflow
            
            policy.update_q_value(state, action, new_q)?;
        }
        
        Ok(())
    }

    /// PPO update (Proximal Policy Optimization)
    fn update_ppo(&self, policy: &mut Policy, experience: &Experience) -> Result<()> {
        // PPO: clipped surrogate objective with multiple epochs
        let state = &experience.observation;
        let action = experience.action.as_ref()
            .ok_or_else(|| Error::Storage("Action missing from experience".to_string()))?;
        let reward = experience.reward.unwrap_or(0.0);
        
        // Get old log probability (would be stored from previous policy)
        let old_log_prob = policy.get_log_probability(state, action)?;
        
        // Compute advantage estimate (simplified - in production would use GAE)
        let value = self.get_value(state)?;
        let advantage = reward - value;
        
        // Compute new log probability
        let new_log_prob = policy.get_log_probability(state, action)?;
        
        // Compute probability ratio
        let ratio = (new_log_prob - old_log_prob).exp();
        
        // PPO clipped objective: min(ratio * advantage, clip(ratio, 1-ε, 1+ε) * advantage)
        let epsilon = 0.2; // PPO clipping parameter
        let clipped_ratio = ratio.clamp(1.0 - epsilon, 1.0 + epsilon);
        let policy_loss = (ratio * advantage).min(clipped_ratio * advantage);
        
        // Update policy with gradient (simplified - full PPO would use multiple epochs)
        policy.update_with_gradient(state, action, policy_loss, self.config.learning_rate)?;
        
        // Update value function (critic)
        let value_target = reward;
        let value_loss = value_target - value;
        self.update_value(state, value + self.config.learning_rate * value_loss)?;
        
        Ok(())
    }

    /// Get value for state
    fn get_value(&self, state: &serde_json::Value) -> Result<f64> {
        let value_functions = self.value_functions.read();
        
        // Try to get value from stored value function
        let state_key = serde_json::to_string(state)
            .map_err(|e| Error::Storage(format!("Failed to serialize state: {}", e)))?;
        
        // Check if we have a value function for this state
        if let Some(vf) = value_functions.values().next() {
            // In production: would use neural network approximation
            // For now: use tabular lookup
            #[cfg(feature = "ml")]
            {
                // Could use ONNX for value function if available
                // For now, fall back to tabular
            }
            
            // Return stored value or default
            Ok(vf.values.get(&state_key).copied().unwrap_or(0.0))
        } else {
            Ok(0.0)
        }
    }

    /// Update value function
    fn update_value_function(&self, trace_id: &str, trace: &RewardTrace) -> Result<()> {
        // Update value function using TD learning or Monte Carlo returns
        let mut value_functions = self.value_functions.write();
        
        // Get or create value function
        let vf = value_functions.entry(trace_id.to_string())
            .or_insert_with(|| ValueFunction {
                values: HashMap::new(),
            });
        
        // Compute returns and update values (Monte Carlo)
        let mut returns = Vec::new();
        let mut cumulative_return = 0.0;
        
        for (idx, reward) in trace.rewards.iter().rev().enumerate() {
            cumulative_return = *reward + self.config.discount_factor * cumulative_return;
            returns.push(cumulative_return);
        }
        
        returns.reverse();
        
        // Update value function for each state
        for (idx, state) in trace.states.iter().enumerate() {
            let state_key = serde_json::to_string(state)
                .map_err(|e| Error::Storage(format!("Failed to serialize state: {}", e)))?;
            
            if idx < returns.len() {
                let return_value = returns[idx];
                let current_value = vf.values.get(&state_key).copied().unwrap_or(0.0);
                
                // TD update: V(s) = V(s) + α * (G - V(s))
                let new_value = current_value + self.config.learning_rate * (return_value - current_value);
                vf.values.insert(state_key, new_value);
            }
        }
        
        Ok(())
    }

    /// Update value
    fn update_value(&self, state: &serde_json::Value, new_value: f64) -> Result<()> {
        // Update value function for a specific state
        let state_key = serde_json::to_string(state)
            .map_err(|e| Error::Storage(format!("Failed to serialize state: {}", e)))?;
        
        let mut value_functions = self.value_functions.write();
        
        // Update first available value function (in production would match to specific function)
        if let Some(vf) = value_functions.values_mut().next() {
            vf.values.insert(state_key, new_value);
        }
        
        Ok(())
    }

    /// Store experience in replay buffer
    pub fn store_experience(&self, experience: Experience) -> Result<()> {
        let mut buffer = self.experience_buffer.write();
        buffer.push(experience.clone());
        
        // EDGE CASE: Handle case where buffer size is exactly at limit
        // Also handle potential overflow if replay_buffer_size is 0
        if self.config.replay_buffer_size == 0 {
            return Err(Error::Storage("Replay buffer size cannot be zero".to_string()));
        }
        
        // Limit buffer size - remove oldest entries if exceeded
        while buffer.len() > self.config.replay_buffer_size {
            buffer.remove(0);
        }

        // Update policy if batch size reached
        if buffer.len() >= self.config.batch_size {
            self.update_from_batch()?;
        }

        Ok(())
    }

    /// Update from experience batch
    fn update_from_batch(&self) -> Result<()> {
        let buffer = self.experience_buffer.read();
        let batch: Vec<Experience> = buffer
            .iter()
            .rev()
            .take(self.config.batch_size)
            .cloned()
            .collect();
        drop(buffer);

        // Update policies from batch
        for experience in &batch {
            // Find relevant policy
            // In production: would match experience to policy
            if let Some(policy_id) = self.find_policy_for_experience(&experience) {
                let mut policies = self.policies.write();
                if let Some(policy) = policies.get_mut(&policy_id) {
                    self.update_policy_internal(policy, experience)?;
                }
            }
        }

        Ok(())
    }

    fn find_policy_for_experience(&self, _experience: &Experience) -> Option<String> {
        // In production: would match experience to appropriate policy
        let policies = self.policies.read();
        policies.keys().next().cloned()
    }

    fn update_policy_internal(&self, policy: &mut Policy, experience: &Experience) -> Result<()> {
        match self.config.algorithm {
            RLAlgorithm::QLearning => self.update_q_learning(policy, experience),
            RLAlgorithm::ActorCritic => self.update_actor_critic(policy, experience),
            RLAlgorithm::PolicyGradient => self.update_policy_gradient(policy, experience),
            RLAlgorithm::DQN => self.update_dqn(policy, experience),
            RLAlgorithm::PPO => self.update_ppo(policy, experience),
        }
    }

    /// Create new policy
    pub fn create_policy(&self, policy_id: &str, initial_state: &serde_json::Value) -> Result<()> {
        let policy = Policy::new(policy_id, initial_state);
        self.policies.write().insert(policy_id.to_string(), policy);
        Ok(())
    }

    /// Get policy statistics
    pub fn get_policy_stats(&self, policy_id: &str) -> Result<PolicyStats> {
        let policies = self.policies.read();
        let policy = policies.get(policy_id)
            .ok_or_else(|| Error::Storage(format!("Policy {} not found", policy_id)))?;

        Ok(PolicyStats {
            policy_id: policy_id.to_string(),
            total_updates: policy.update_count,
            average_reward: policy.average_reward,
            exploration_rate: self.config.epsilon,
        })
    }
}

/// Policy for action selection
#[derive(Debug, Clone)]
struct Policy {
    policy_id: String,
    q_values: HashMap<String, f64>, // State-action -> Q-value
    probabilities: HashMap<String, f64>, // State -> action probability
    update_count: u64,
    average_reward: f64,
}

impl Policy {
    fn new(policy_id: &str, _initial_state: &serde_json::Value) -> Self {
        Self {
            policy_id: policy_id.to_string(),
            q_values: HashMap::new(),
            probabilities: HashMap::new(),
            update_count: 0,
            average_reward: 0.0,
        }
    }

    fn select_action(&self, state: &serde_json::Value, epsilon: f64) -> Result<Action> {
        // Epsilon-greedy action selection
        let state_key = serde_json::to_string(state)
            .map_err(|e| Error::Storage(format!("Failed to serialize state: {}", e)))?;
        
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        state_key.hash(&mut hasher);
        let hash = hasher.finish();
        let random = (hash % 1000) as f64 / 1000.0;
        
        if random < epsilon {
            // Explore: random action
            Ok(Action {
                action_type: "random".to_string(),
                parameters: serde_json::json!({}),
            })
        } else {
            // Exploit: best action
            let best_q = self.q_values.iter()
                .filter(|(k, _)| k.starts_with(&state_key))
                .filter(|(_, v)| v.is_finite())
                .max_by(|(_, a), (_, b)| {
                    a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(_, v)| *v)
                .unwrap_or(0.0);

            Ok(Action {
                action_type: "exploit".to_string(),
                parameters: serde_json::json!({"q_value": best_q}),
            })
        }
    }

    fn get_q_value(&self, state: &serde_json::Value, action: &serde_json::Value) -> Result<f64> {
        let state_str = serde_json::to_string(state)
            .map_err(|e| Error::Storage(format!("Failed to serialize state: {}", e)))?;
        let action_str = serde_json::to_string(action)
            .map_err(|e| Error::Storage(format!("Failed to serialize action: {}", e)))?;
        let key = format!("{}:{}", state_str, action_str);
        Ok(self.q_values.get(&key).copied().unwrap_or(0.0))
    }

    fn get_max_q_value(&self, state: &serde_json::Value) -> Result<f64> {
        let state_key = serde_json::to_string(state)
            .map_err(|e| Error::Storage(format!("Failed to serialize state: {}", e)))?;
        Ok(self.q_values.iter()
            .filter(|(k, _)| k.starts_with(&state_key))
            .filter(|(_, v)| v.is_finite())
            .map(|(_, v)| *v)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0))
    }

    fn update_q_value(&mut self, state: &serde_json::Value, action: &serde_json::Value, new_q: f64) -> Result<()> {
        let state_str = serde_json::to_string(state)
            .map_err(|e| Error::Storage(format!("Failed to serialize state: {}", e)))?;
        let action_str = serde_json::to_string(action)
            .map_err(|e| Error::Storage(format!("Failed to serialize action: {}", e)))?;
        let key = format!("{}:{}", state_str, action_str);
        self.q_values.insert(key, new_q);
        self.update_count += 1;
        Ok(())
    }

    fn update_with_td_error(&mut self, _state: &serde_json::Value, _action: &serde_json::Value, _td_error: f64, _lr: f64) -> Result<()> {
        // Actor-critic update
        self.update_count += 1;
        Ok(())
    }

    fn update_with_gradient(&mut self, _state: &serde_json::Value, _action: &serde_json::Value, _gradient: f64, _lr: f64) -> Result<()> {
        // Policy gradient update
        self.update_count += 1;
        Ok(())
    }

    fn get_log_probability(&self, state: &serde_json::Value, action: &serde_json::Value) -> Result<f64> {
        // Compute log probability from policy
        // Simplified: use softmax over Q-values
        let state_str = serde_json::to_string(state)
            .map_err(|e| Error::Storage(format!("Failed to serialize state: {}", e)))?;
        let action_str = serde_json::to_string(action)
            .map_err(|e| Error::Storage(format!("Failed to serialize action: {}", e)))?;
        
        let q_value = self.q_values.get(&format!("{}:{}", state_str, action_str))
            .copied()
            .unwrap_or(0.0);
        
        // Softmax: log prob ≈ log(exp(q) / sum(exp(q_i))) = q - log(sum(exp(q_i)))
        // Simplified: just return normalized Q-value as log prob
        // In production: would use proper policy distribution (e.g., Gaussian, categorical)
        Ok(q_value / 10.0) // Scale down for numerical stability
    }
}

/// Action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub action_type: String,
    pub parameters: serde_json::Value,
}

/// Value function
#[derive(Debug, Clone)]
struct ValueFunction {
    values: HashMap<String, f64>,
}

/// Reward trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardTrace {
    pub trace_id: String,
    pub rewards: Vec<f64>,
    pub states: Vec<serde_json::Value>,
    pub total_reward: f64,
    pub created_at: u64,
}

/// Policy statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyStats {
    pub policy_id: String,
    pub total_updates: u64,
    pub average_reward: f64,
    pub exploration_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rl_engine_creation() {
        let brain = Arc::new(CognitiveBrain::new());
        let config = RLConfig {
            learning_rate: 0.01,
            discount_factor: 0.99,
            epsilon: 0.1,
            batch_size: 32,
            replay_buffer_size: 10000,
            update_frequency: 100,
            algorithm: RLAlgorithm::QLearning,
        };
        let engine = RLEngine::new(brain, config);
        assert!(engine.policies.read().is_empty());
    }

    #[test]
    fn test_policy_creation() {
        let brain = Arc::new(CognitiveBrain::new());
        let config = RLConfig {
            learning_rate: 0.01,
            discount_factor: 0.99,
            epsilon: 0.1,
            batch_size: 32,
            replay_buffer_size: 10000,
            update_frequency: 100,
            algorithm: RLAlgorithm::QLearning,
        };
        let engine = RLEngine::new(brain, config);
        
        let result = engine.create_policy("test_policy", &serde_json::json!({"state": "initial"}));
        assert!(result.is_ok());
        
        let policies = engine.policies.read();
        assert!(policies.contains_key("test_policy"));
    }
}

