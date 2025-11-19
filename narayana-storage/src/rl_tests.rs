#[cfg(test)]
mod rl_tests {
    use crate::reinforcement_learning::{RLEngine, RLConfig, RLAlgorithm, RewardTrace};
    use crate::cognitive::{CognitiveBrain, Experience};
    use serde_json::json;
    use std::sync::Arc;

    fn create_test_experience() -> Experience {
        use std::collections::HashMap;
        use crate::cognitive::Pattern;
        use crate::cognitive::PatternType;
        
        Experience {
            id: "exp1".to_string(),
            event_type: "test".to_string(),
            observation: json!({"state": "s1"}),
            action: Some(json!({"action": "a1"})),
            outcome: Some(json!({"state": "s2"})),
            reward: Some(1.0),
            timestamp: 0,
            context: HashMap::new(),
            patterns: Vec::new(),
            embedding: None,
        }
    }

    #[test]
    fn test_dqn_experience_replay() {
        let brain = Arc::new(CognitiveBrain::new());
        let config = RLConfig {
            learning_rate: 0.01,
            discount_factor: 0.99,
            epsilon: 0.1,
            batch_size: 4,
            replay_buffer_size: 100,
            update_frequency: 1,
            algorithm: RLAlgorithm::DQN,
        };
        
        let engine = RLEngine::new(brain, config);
        
        // Store multiple experiences to fill replay buffer
        for i in 0..10 {
            let mut exp = create_test_experience();
            exp.id = format!("exp_{}", i);
            exp.observation = json!({"state": format!("s{}", i)});
            engine.store_experience(exp).unwrap();
        }
        
        // Create a policy to update
        engine.create_policy("test_policy", &json!({"state": "initial"})).unwrap();
        
        // DQN should use experience replay, not just the latest experience
        let mut exp = create_test_experience();
        exp.reward = Some(10.0); // High reward to test replay
        engine.store_experience(exp).unwrap();
        
        // If experience replay is working, the policy should have been updated
        // from multiple experiences in the batch
        let stats = engine.get_policy_stats("test_policy").unwrap();
        assert!(stats.total_updates > 0);
    }

    #[test]
    fn test_ppo_clipped_objective() {
        let brain = Arc::new(CognitiveBrain::new());
        let config = RLConfig {
            learning_rate: 0.01,
            discount_factor: 0.99,
            epsilon: 0.1,
            batch_size: 1,
            replay_buffer_size: 100,
            update_frequency: 1,
            algorithm: RLAlgorithm::PPO,
        };
        
        let engine = RLEngine::new(brain, config);
        engine.create_policy("test_policy", &json!({"state": "initial"})).unwrap();
        
        // Store experience with reward
        let mut exp = create_test_experience();
        exp.reward = Some(5.0);
        engine.store_experience(exp).unwrap();
        
        // PPO should update both policy and value function
        let stats = engine.get_policy_stats("test_policy").unwrap();
        assert!(stats.total_updates > 0);
    }

    #[test]
    fn test_value_function_updates() {
        let brain = Arc::new(CognitiveBrain::new());
        let config = RLConfig {
            learning_rate: 0.1,
            discount_factor: 0.9,
            epsilon: 0.1,
            batch_size: 1,
            replay_buffer_size: 100,
            update_frequency: 1,
            algorithm: RLAlgorithm::ActorCritic,
        };
        
        let engine = RLEngine::new(brain, config);
        
        // Create reward trace
        let trace = RewardTrace {
            trace_id: "trace1".to_string(),
            rewards: vec![1.0, 2.0, 3.0],
            states: vec![
                json!({"state": "s1"}),
                json!({"state": "s2"}),
                json!({"state": "s3"}),
            ],
            total_reward: 6.0,
            created_at: 0,
        };
        
        // update_value_function is private, test via public API that uses it
        // Store experiences that create traces
        for i in 0..3 {
            let mut exp = create_test_experience();
            exp.id = format!("exp_{}", i);
            exp.reward = Some(trace.rewards[i]);
            exp.observation = trace.states[i].clone();
            engine.store_experience(exp).unwrap();
        }
        
        // Value function should have been updated with returns
        // (would need access to internal value functions to verify exact values)
    }

    #[test]
    fn test_experience_replay_buffer_limit() {
        let brain = Arc::new(CognitiveBrain::new());
        let config = RLConfig {
            learning_rate: 0.01,
            discount_factor: 0.99,
            epsilon: 0.1,
            batch_size: 1,
            replay_buffer_size: 5, // Small buffer
            update_frequency: 1,
            algorithm: RLAlgorithm::DQN,
        };
        
        let engine = RLEngine::new(brain, config);
        
        // Store more experiences than buffer size
        for i in 0..10 {
            let mut exp = create_test_experience();
            exp.id = format!("exp_{}", i);
            engine.store_experience(exp).unwrap();
        }
        
        // Buffer should not exceed size limit
        // (Would need internal access to verify, but should not panic)
    }
}

