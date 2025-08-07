/// Agent system module for autonomous translation workflow management
/// 
/// This module provides intelligent agents that can manage various aspects
/// of the translation workflow including quality assurance, terminology 
/// consistency, and translation memory optimization.

pub mod translation_agent;
pub mod terminology_agent;
pub mod quality_agent;
pub mod workflow_agent;

pub use translation_agent::TranslationAgent;
pub use terminology_agent::TerminologyAgent;
pub use quality_agent::QualityAgent;
pub use workflow_agent::WorkflowAgent;

use std::collections::HashMap;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use anyhow::Result;

/// Agent capabilities and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_type: AgentType,
    pub enabled: bool,
    pub priority: AgentPriority,
    pub parameters: HashMap<String, String>,
}

/// Types of available agents
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentType {
    Translation,
    Terminology,
    Quality,
    Workflow,
}

/// Agent priority levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum AgentPriority {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

/// Agent execution context
#[derive(Debug, Clone)]
pub struct AgentContext {
    pub project_id: Uuid,
    pub chapter_id: Option<Uuid>,
    pub user_id: Option<String>,
    pub language_pair: Option<(String, String)>,
    pub metadata: HashMap<String, String>,
}

/// Base trait for all agents
pub trait Agent: Send + Sync {
    /// Get agent configuration
    fn config(&self) -> &AgentConfig;
    
    /// Execute agent logic
    async fn execute(&self, context: AgentContext) -> Result<AgentResult>;
    
    /// Check if agent should run for given context
    fn should_execute(&self, context: &AgentContext) -> bool;
    
    /// Get agent health status
    fn health_check(&self) -> AgentHealth;
}

/// Agent execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub agent_type: AgentType,
    pub success: bool,
    pub message: String,
    pub actions_taken: Vec<AgentAction>,
    pub recommendations: Vec<String>,
    pub execution_time_ms: u64,
}

/// Actions that agents can take
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentAction {
    UpdateTranslation {
        chunk_id: Uuid,
        old_text: String,
        new_text: String,
        confidence: f32,
    },
    AddTerminology {
        term: String,
        definition: Option<String>,
        do_not_translate: bool,
    },
    MarkQualityIssue {
        chunk_id: Uuid,
        issue_type: String,
        severity: String,
        description: String,
    },
    UpdateWorkflowStatus {
        chapter_id: Uuid,
        old_status: String,
        new_status: String,
    },
    NotifyUser {
        user_id: String,
        message: String,
        priority: AgentPriority,
    },
}

/// Agent health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHealth {
    pub healthy: bool,
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
    pub error_count: u32,
    pub success_rate: f32,
    pub message: Option<String>,
}

/// Agent orchestrator for managing multiple agents
pub struct AgentOrchestrator {
    agents: Vec<Box<dyn Agent>>,
    config: OrchestratorConfig,
}

/// Configuration for agent orchestrator
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub max_concurrent_agents: usize,
    pub execution_timeout_ms: u64,
    pub retry_attempts: u32,
    pub enable_parallel_execution: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_agents: 4,
            execution_timeout_ms: 30000, // 30 seconds
            retry_attempts: 3,
            enable_parallel_execution: true,
        }
    }
}

impl AgentOrchestrator {
    /// Create new agent orchestrator
    pub fn new(config: OrchestratorConfig) -> Self {
        Self {
            agents: Vec::new(),
            config,
        }
    }
    
    /// Add an agent to the orchestrator
    pub fn add_agent(&mut self, agent: Box<dyn Agent>) {
        self.agents.push(agent);
    }
    
    /// Execute all applicable agents for the given context
    pub async fn execute_agents(&self, context: AgentContext) -> Result<Vec<AgentResult>> {
        let applicable_agents: Vec<&Box<dyn Agent>> = self.agents
            .iter()
            .filter(|agent| agent.should_execute(&context) && agent.config().enabled)
            .collect();
        
        if applicable_agents.is_empty() {
            return Ok(Vec::new());
        }
        
        // Sort agents by priority (highest first)
        let mut sorted_agents = applicable_agents;
        sorted_agents.sort_by(|a, b| b.config().priority.partial_cmp(&a.config().priority).unwrap());
        
        let mut results = Vec::new();
        
        if self.config.enable_parallel_execution {
            // Execute agents in parallel (respecting max_concurrent_agents)
            use futures::stream::{StreamExt, FuturesUnordered};
            
            let mut futures = FuturesUnordered::new();
            let mut agent_iter = sorted_agents.into_iter();
            
            // Start initial batch
            for _ in 0..self.config.max_concurrent_agents.min(agent_iter.len()) {
                if let Some(agent) = agent_iter.next() {
                    let context_clone = context.clone();
                    futures.push(async move {
                        agent.execute(context_clone).await
                    });
                }
            }
            
            // Process results and start new agents
            while let Some(result) = futures.next().await {
                match result {
                    Ok(agent_result) => results.push(agent_result),
                    Err(e) => {
                        log::warn!("Agent execution failed: {}", e);
                        // Continue with other agents
                    }
                }
                
                // Start next agent if available
                if let Some(agent) = agent_iter.next() {
                    let context_clone = context.clone();
                    futures.push(async move {
                        agent.execute(context_clone).await
                    });
                }
            }
        } else {
            // Execute agents sequentially
            for agent in sorted_agents {
                match agent.execute(context.clone()).await {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        log::warn!("Agent {} execution failed: {}", 
                                 format!("{:?}", agent.config().agent_type), e);
                        // Continue with other agents
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// Get health status of all agents
    pub fn get_agents_health(&self) -> Vec<(AgentType, AgentHealth)> {
        self.agents
            .iter()
            .map(|agent| (agent.config().agent_type.clone(), agent.health_check()))
            .collect()
    }
    
    /// Get agent by type
    pub fn get_agent(&self, agent_type: &AgentType) -> Option<&Box<dyn Agent>> {
        self.agents.iter().find(|agent| &agent.config().agent_type == agent_type)
    }
    
    /// Enable/disable an agent
    pub fn set_agent_enabled(&mut self, agent_type: &AgentType, enabled: bool) {
        // Note: This would require making Agent trait methods mutable
        // For now, agents are configured at creation time
        log::info!("Agent {:?} enabled status would be set to: {}", agent_type, enabled);
    }
}

impl Clone for AgentContext {
    fn clone(&self) -> Self {
        Self {
            project_id: self.project_id,
            chapter_id: self.chapter_id,
            user_id: self.user_id.clone(),
            language_pair: self.language_pair.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_agent_priority_ordering() {
        assert!(AgentPriority::Critical > AgentPriority::High);
        assert!(AgentPriority::High > AgentPriority::Medium);
        assert!(AgentPriority::Medium > AgentPriority::Low);
    }
    
    #[test]
    fn test_agent_config_creation() {
        let config = AgentConfig {
            agent_type: AgentType::Translation,
            enabled: true,
            priority: AgentPriority::High,
            parameters: HashMap::new(),
        };
        
        assert_eq!(config.agent_type, AgentType::Translation);
        assert!(config.enabled);
        assert_eq!(config.priority, AgentPriority::High);
    }
    
    #[test]
    fn test_agent_context_creation() {
        let context = AgentContext {
            project_id: Uuid::new_v4(),
            chapter_id: Some(Uuid::new_v4()),
            user_id: Some("user123".to_string()),
            language_pair: Some(("en".to_string(), "es".to_string())),
            metadata: HashMap::new(),
        };
        
        assert!(context.chapter_id.is_some());
        assert!(context.user_id.is_some());
        assert!(context.language_pair.is_some());
    }
    
    #[test]
    fn test_orchestrator_creation() {
        let config = OrchestratorConfig::default();
        let orchestrator = AgentOrchestrator::new(config);
        
        assert_eq!(orchestrator.agents.len(), 0);
        assert_eq!(orchestrator.config.max_concurrent_agents, 4);
    }
}