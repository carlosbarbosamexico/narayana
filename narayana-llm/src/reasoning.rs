use crate::config::*;
use crate::error::Result;
use crate::manager::LLMManager;

pub struct ReasoningSystem;

impl ReasoningSystem {
    pub fn new() -> Self {
        Self
    }

    /// Chain of thought reasoning - structured step-by-step reasoning
    pub async fn chain_of_thought_reasoning(
        &self,
        llm_manager: &LLMManager,
        problem: &str,
        steps: &[&str],
    ) -> Result<String> {
        // Input validation
        if problem.is_empty() {
            return Err(crate::error::LLMError::InvalidResponse("Problem cannot be empty".to_string()));
        }
        
        if problem.len() > 10000 {
            return Err(crate::error::LLMError::InvalidResponse("Problem too long (max 10000 chars)".to_string()));
        }
        
        if steps.len() > 50 {
            return Err(crate::error::LLMError::InvalidResponse("Too many steps (max 50)".to_string()));
        }
        
        let steps_text = steps
            .iter()
            .take(50)
            .enumerate()
            .map(|(i, step)| {
                let truncated = if step.len() > 1000 {
                    &step[..1000]
                } else {
                    step
                };
                format!("Step {}: {}", i + 1, truncated)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Solve this problem using chain of thought reasoning:\n\nProblem: {}\n\nFollow these steps:\n{}\n\nProvide your reasoning step by step, then give a final answer.",
            problem, steps_text
        );

        let response = llm_manager
            .chat(vec![Message {
                role: MessageRole::User,
                content: prompt,
            }], None)
            .await?;

        Ok(response)
    }

    /// Tree of thoughts - explore multiple reasoning paths
    pub async fn tree_of_thoughts(
        &self,
        llm_manager: &LLMManager,
        problem: &str,
        branches: usize,
    ) -> Result<Vec<String>> {
        // Input validation
        if problem.is_empty() {
            return Err(crate::error::LLMError::InvalidResponse("Problem cannot be empty".to_string()));
        }
        
        if problem.len() > 10000 {
            return Err(crate::error::LLMError::InvalidResponse("Problem too long (max 10000 chars)".to_string()));
        }
        
        // Limit branches to prevent resource exhaustion
        let branches = branches.min(10);
        
        let mut results = Vec::new();

        for i in 0..branches {
            let prompt = format!(
                "Problem: {}\n\nApproach this problem from a different angle (approach {} of {}). Provide your reasoning and solution.",
                problem, i + 1, branches
            );

            let response = llm_manager
                .chat(vec![Message {
                    role: MessageRole::User,
                    content: prompt,
                }], None)
                .await?;

            results.push(response);
        }

        Ok(results)
    }

    /// Generate hypothesis from observation
    pub async fn generate_hypothesis(
        &self,
        llm_manager: &LLMManager,
        observation: &str,
        context: Option<&str>,
    ) -> Result<String> {
        // Input validation
        if observation.is_empty() {
            return Err(crate::error::LLMError::InvalidResponse("Observation cannot be empty".to_string()));
        }
        
        if observation.len() > 10000 {
            return Err(crate::error::LLMError::InvalidResponse("Observation too long (max 10000 chars)".to_string()));
        }
        
        let context_text = context
            .map(|c| {
                let truncated = if c.len() > 10000 {
                    &c[..10000]
                } else {
                    c
                };
                format!("\n\nContext: {}", truncated)
            })
            .unwrap_or_default();

        let prompt = format!(
            "Based on this observation: {}{}\n\nGenerate a hypothesis that could explain this observation. Provide reasoning for why this hypothesis might be valid.",
            observation, context_text
        );

        let response = llm_manager
            .chat(vec![Message {
                role: MessageRole::User,
                content: prompt,
            }], None)
            .await?;

        Ok(response)
    }
}

