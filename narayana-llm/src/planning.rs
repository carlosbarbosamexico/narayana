use crate::config::*;
use crate::error::Result;
use crate::manager::LLMManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub goal: String,
    pub steps: Vec<PlanStep>,
    pub constraints: Vec<String>,
    pub status: PlanStatus,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub description: String,
    pub dependencies: Vec<String>,
    pub status: StepStatus,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    Created,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

pub struct PlanningSystem {
    plans: Arc<RwLock<HashMap<String, Plan>>>,
}

impl PlanningSystem {
    pub fn new() -> Self {
        Self {
            plans: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate a plan to achieve a goal
    pub async fn generate_plan(
        &self,
        llm_manager: &LLMManager,
        goal: &str,
        constraints: &[String],
    ) -> Result<String> {
        // Input validation
        if goal.is_empty() {
            return Err(crate::error::LLMError::InvalidResponse("Goal cannot be empty".to_string()));
        }
        
        if goal.len() > 10000 {
            return Err(crate::error::LLMError::InvalidResponse("Goal too long (max 10000 chars)".to_string()));
        }
        
        if constraints.len() > 100 {
            return Err(crate::error::LLMError::InvalidResponse("Too many constraints (max 100)".to_string()));
        }
        
        // Validate constraint sizes
        for constraint in constraints {
            if constraint.len() > 1000 {
                return Err(crate::error::LLMError::InvalidResponse("Constraint too long (max 1000 chars)".to_string()));
            }
        }
        
        let constraints_text = if constraints.is_empty() {
            "None".to_string()
        } else {
            constraints.iter().take(100).map(|s| {
                if s.len() > 1000 {
                    &s[..1000]
                } else {
                    s.as_str()
                }
            }).collect::<Vec<_>>().join(", ")
        };

        let prompt = format!(
            "Create a detailed plan to achieve this goal:\n\nGoal: {}\n\nConstraints: {}\n\nBreak down the goal into specific, actionable steps. Each step should be clear and measurable. List the steps in order, and note any dependencies between steps.",
            goal, constraints_text
        );

        let response = llm_manager
            .chat(vec![Message {
                role: MessageRole::User,
                content: prompt,
            }], None)
            .await?;

        // Parse response and create plan structure
        let plan_id = uuid::Uuid::new_v4().to_string();
        let steps: Vec<PlanStep> = self.parse_steps_from_response(&response);

        let plan = Plan {
            id: plan_id.clone(),
            goal: goal.to_string(),
            steps,
            constraints: constraints.iter().take(100).cloned().collect(), // Limit constraints
            status: PlanStatus::Created,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        self.plans.write().insert(plan_id.clone(), plan);

        Ok(plan_id)
    }

    /// Refine a plan based on feedback
    pub async fn refine_plan(
        &self,
        llm_manager: &LLMManager,
        plan_id: &str,
        feedback: &str,
    ) -> Result<()> {
        // Input validation
        if plan_id.is_empty() || plan_id.len() > 200 {
            return Err(crate::error::LLMError::InvalidResponse("Invalid plan ID".to_string()));
        }
        
        if feedback.len() > 100000 {
            return Err(crate::error::LLMError::InvalidResponse("Feedback too long (max 100KB)".to_string()));
        }
        
        let plan = self
            .plans
            .read()
            .get(plan_id)
            .cloned()
            .ok_or_else(|| {
                crate::error::LLMError::InvalidResponse(format!("Plan {} not found", plan_id))
            })?;

        let plan_text = format!(
            "Current plan:\nGoal: {}\nSteps:\n{}",
            plan.goal,
            plan.steps
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{}. {}", i + 1, s.description))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let prompt = format!(
            "{}\n\nFeedback: {}\n\nRefine the plan based on this feedback. Provide an updated version of the plan with improved steps.",
            plan_text, feedback
        );

        let response = llm_manager
            .chat(vec![Message {
                role: MessageRole::User,
                content: prompt,
            }], None)
            .await?;

        // Update plan with refined steps
        let mut plans = self.plans.write();
        if let Some(plan) = plans.get_mut(plan_id) {
            plan.steps = self.parse_steps_from_response(&response);
        }

        Ok(())
    }

    /// Get a plan by ID
    pub fn get_plan(&self, plan_id: &str) -> Option<Plan> {
        self.plans.read().get(plan_id).cloned()
    }

    fn parse_steps_from_response(&self, response: &str) -> Vec<PlanStep> {
        // Simple parsing - in production, use more sophisticated parsing
        let lines: Vec<&str> = response.lines().collect();
        let mut steps = Vec::new();

        for (_i, line) in lines.iter().enumerate() {
            if line.trim().starts_with(char::is_numeric) || line.contains("Step") {
                let step_id = uuid::Uuid::new_v4().to_string();
                steps.push(PlanStep {
                    id: step_id,
                    description: line.trim().to_string(),
                    dependencies: Vec::new(),
                    status: StepStatus::Pending,
                    result: None,
                });
            }
        }

        if steps.is_empty() {
            // Fallback: treat entire response as one step
            steps.push(PlanStep {
                id: uuid::Uuid::new_v4().to_string(),
                description: response.to_string(),
                dependencies: Vec::new(),
                status: StepStatus::Pending,
                result: None,
            });
        }

        steps
    }
}

