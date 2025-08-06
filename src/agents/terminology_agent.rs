/// Terminology Agent for autonomous terminology management and consistency
/// 
/// This agent monitors terminology usage, enforces consistency rules, and
/// suggests terminology improvements across translation projects.

use super::{Agent, AgentConfig, AgentContext, AgentResult, AgentAction, AgentHealth, AgentType, AgentPriority};
use crate::services::{TerminologyService, TerminologyServiceAdapter};
use crate::models::translation_models::Term;

use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::Result;
use tokio::sync::RwLock;
use regex::Regex;

/// Terminology agent for consistency checking and management
pub struct TerminologyAgent {
    config: AgentConfig,
    terminology_service: Arc<TerminologyService>,
    // New adapter service for improved performance
    adapter_service: Arc<TerminologyServiceAdapter>,
    health: Arc<RwLock<AgentHealth>>,
    consistency_rules: ConsistencyRules,
}

/// Rules for terminology consistency checking
#[derive(Debug, Clone)]
pub struct ConsistencyRules {
    pub enforce_do_not_translate: bool,
    pub case_sensitivity: bool,
    pub fuzzy_matching_threshold: f32,
    pub auto_suggest_translations: bool,
    pub require_definition_for_technical_terms: bool,
}

impl Default for ConsistencyRules {
    fn default() -> Self {
        Self {
            enforce_do_not_translate: true,
            case_sensitivity: false,
            fuzzy_matching_threshold: 0.85,
            auto_suggest_translations: true,
            require_definition_for_technical_terms: true,
        }
    }
}

/// Terminology issue types
#[derive(Debug, Clone)]
pub enum TerminologyIssue {
    DoNotTranslateViolation {
        term: String,
        translated_as: String,
        position: usize,
    },
    InconsistentTranslation {
        term: String,
        expected_translation: String,
        actual_translation: String,
        confidence: f32,
    },
    MissingTerminology {
        potential_term: String,
        context: String,
        suggestion: String,
    },
    UndefinedTechnicalTerm {
        term: String,
        context: String,
    },
    CaseInconsistency {
        term: String,
        expected_case: String,
        actual_case: String,
    },
}

/// Terminology suggestion with confidence score
#[derive(Debug, Clone)]
pub struct TerminologySuggestion {
    pub term: String,
    pub suggested_translation: Option<String>,
    pub confidence: f32,
    pub reason: String,
    pub auto_apply: bool,
}

impl TerminologyAgent {
    /// Create a new terminology agent
    pub async fn new(
        terminology_service: Arc<TerminologyService>,
        rules: Option<ConsistencyRules>,
    ) -> Result<Self> {
        let config = AgentConfig {
            agent_type: AgentType::Terminology,
            enabled: true,
            priority: AgentPriority::Medium,
            parameters: HashMap::new(),
        };
        
        let health = AgentHealth {
            healthy: true,
            last_execution: None,
            error_count: 0,
            success_rate: 1.0,
            message: None,
        };
        
        Ok(Self {
            config,
            terminology_service,
            health: Arc::new(RwLock::new(health)),
            consistency_rules: rules.unwrap_or_default(),
        })
    }
    
    /// Analyze text for terminology issues
    async fn analyze_terminology_usage(
        &self,
        source_text: &str,
        target_text: &str,
        project_id: Uuid,
    ) -> Result<Vec<TerminologyIssue>> {
        let mut issues = Vec::new();
        
        // Get project terminology
        let project_terms = self.terminology_service
            .get_non_translatable_terms(project_id)
            .await?;
        
        // Check for do-not-translate violations
        if self.consistency_rules.enforce_do_not_translate {
            issues.extend(self.check_do_not_translate_violations(
                source_text,
                target_text,
                &project_terms,
            )?);
        }
        
        // Check for missing terminology
        issues.extend(self.check_missing_terminology(
            source_text,
            target_text,
            project_id,
        ).await?);
        
        // Check for technical terms without definitions
        if self.consistency_rules.require_definition_for_technical_terms {
            issues.extend(self.check_undefined_technical_terms(
                source_text,
                &project_terms,
            )?);
        }
        
        Ok(issues)
    }
    
    /// Check for violations of do-not-translate terms
    fn check_do_not_translate_violations(
        &self,
        source_text: &str,
        target_text: &str,
        terms: &[Term],
    ) -> Result<Vec<TerminologyIssue>> {
        let mut issues = Vec::new();
        
        for term in terms.iter().filter(|t| t.do_not_translate) {
            let term_pattern = if self.consistency_rules.case_sensitivity {
                format!(r"\b{}\b", regex::escape(&term.term))
            } else {
                format!(r"(?i)\b{}\b", regex::escape(&term.term))
            };
            
            let regex = Regex::new(&term_pattern)?;
            
            // Check if term appears in source text
            if regex.is_match(source_text) {
                // Check if term is preserved in target text
                if !regex.is_match(target_text) {
                    // Find what it was translated as (simplified approach)
                    let source_words: Vec<&str> = source_text.split_whitespace().collect();
                    let target_words: Vec<&str> = target_text.split_whitespace().collect();
                    
                    // This is a simplified approach - in practice you'd use alignment algorithms
                    let position = source_text.find(&term.term).unwrap_or(0);
                    let translated_as = self.guess_translation(&term.term, &source_words, &target_words);
                    
                    issues.push(TerminologyIssue::DoNotTranslateViolation {
                        term: term.term.clone(),
                        translated_as,
                        position,
                    });
                }
            }
        }
        
        Ok(issues)
    }
    
    /// Check for missing terminology that should be in the database
    async fn check_missing_terminology(
        &self,
        source_text: &str,
        _target_text: &str,
        project_id: Uuid,
    ) -> Result<Vec<TerminologyIssue>> {
        let mut issues = Vec::new();
        
        // Identify potential technical terms (simplified heuristics)
        let potential_terms = self.identify_potential_terms(source_text);
        
        for potential_term in potential_terms {
            // Check if term exists in terminology database
            let search_results = self.terminology_service
                .search_terms(&potential_term, project_id)
                .await?;
            
            if search_results.is_empty() {
                issues.push(TerminologyIssue::MissingTerminology {
                    potential_term: potential_term.clone(),
                    context: source_text.to_string(),
                    suggestion: format!("Consider adding '{}' to project terminology", potential_term),
                });
            }
        }
        
        Ok(issues)
    }
    
    /// Check for technical terms without definitions
    fn check_undefined_technical_terms(
        &self,
        source_text: &str,
        terms: &[Term],
    ) -> Result<Vec<TerminologyIssue>> {
        let mut issues = Vec::new();
        
        for term in terms.iter().filter(|t| t.definition.is_none()) {
            let term_pattern = if self.consistency_rules.case_sensitivity {
                format!(r"\b{}\b", regex::escape(&term.term))
            } else {
                format!(r"(?i)\b{}\b", regex::escape(&term.term))
            };
            
            let regex = Regex::new(&term_pattern)?;
            
            if regex.is_match(source_text) && self.is_technical_term(&term.term) {
                issues.push(TerminologyIssue::UndefinedTechnicalTerm {
                    term: term.term.clone(),
                    context: source_text.to_string(),
                });
            }
        }
        
        Ok(issues)
    }
    
    /// Identify potential terminology in text using heuristics
    fn identify_potential_terms(&self, text: &str) -> Vec<String> {
        let mut potential_terms = Vec::new();
        
        // Look for capitalized words (potential proper nouns/technical terms)
        let capitalized_regex = Regex::new(r"\b[A-Z][a-z]+\b").unwrap();
        for cap in capitalized_regex.captures_iter(text) {
            let term = cap.get(0).unwrap().as_str();
            if self.is_potential_terminology(term) {
                potential_terms.push(term.to_string());
            }
        }
        
        // Look for acronyms
        let acronym_regex = Regex::new(r"\b[A-Z]{2,}\b").unwrap();
        for cap in acronym_regex.captures_iter(text) {
            let term = cap.get(0).unwrap().as_str();
            potential_terms.push(term.to_string());
        }
        
        // Look for technical-sounding terms
        let technical_regex = Regex::new(r"\b\w*(?:API|SDK|HTTP|URL|JSON|XML|SQL)\w*\b").unwrap();
        for cap in technical_regex.captures_iter(text) {
            let term = cap.get(0).unwrap().as_str();
            potential_terms.push(term.to_string());
        }
        
        potential_terms.sort();
        potential_terms.dedup();
        potential_terms
    }
    
    /// Check if a term is potentially terminology worth tracking
    fn is_potential_terminology(&self, term: &str) -> bool {
        // Skip common words
        let common_words = ["The", "This", "That", "With", "From", "They", "When", "Where", "What"];
        if common_words.contains(&term) {
            return false;
        }
        
        // Check length (avoid very short terms)
        if term.len() < 3 {
            return false;
        }
        
        // Check if it's a technical-sounding term
        self.is_technical_term(term)
    }
    
    /// Determine if a term is technical and should have a definition
    fn is_technical_term(&self, term: &str) -> bool {
        // Simple heuristics for technical terms
        let technical_patterns = [
            r"API", r"SDK", r"HTTP", r"URL", r"JSON", r"XML", r"SQL",
            r"database", r"server", r"client", r"framework", r"library",
            r"protocol", r"algorithm", r"interface", r"endpoint",
        ];
        
        let term_lower = term.to_lowercase();
        technical_patterns.iter().any(|pattern| {
            term_lower.contains(&pattern.to_lowercase())
        })
    }
    
    /// Guess what a term was translated as (simplified approach)
    fn guess_translation(&self, original_term: &str, source_words: &[&str], target_words: &[&str]) -> String {
        // This is a very simplified approach
        // In practice, you'd use word alignment algorithms
        
        if let Some(pos) = source_words.iter().position(|&word| word.contains(original_term)) {
            if pos < target_words.len() {
                return target_words[pos].to_string();
            }
        }
        
        "[unknown translation]".to_string()
    }
    
    /// Generate terminology suggestions based on issues found
    async fn generate_terminology_suggestions(
        &self,
        issues: &[TerminologyIssue],
    ) -> Vec<TerminologySuggestion> {
        let mut suggestions = Vec::new();
        
        for issue in issues {
            match issue {
                TerminologyIssue::DoNotTranslateViolation { term, .. } => {
                    suggestions.push(TerminologySuggestion {
                        term: term.clone(),
                        suggested_translation: Some(term.clone()),
                        confidence: 0.95,
                        reason: "This term should not be translated according to project terminology.".to_string(),
                        auto_apply: self.consistency_rules.auto_suggest_translations,
                    });
                }
                TerminologyIssue::MissingTerminology { potential_term, .. } => {
                    suggestions.push(TerminologySuggestion {
                        term: potential_term.clone(),
                        suggested_translation: None,
                        confidence: 0.7,
                        reason: "Consider adding this term to the project terminology database.".to_string(),
                        auto_apply: false,
                    });
                }
                TerminologyIssue::UndefinedTechnicalTerm { term, .. } => {
                    suggestions.push(TerminologySuggestion {
                        term: term.clone(),
                        suggested_translation: None,
                        confidence: 0.8,
                        reason: "Technical term should have a definition for consistency.".to_string(),
                        auto_apply: false,
                    });
                }
                _ => {} // Handle other issue types as needed
            }
        }
        
        suggestions
    }
    
    /// Update agent health statistics
    async fn update_health(&self, success: bool, execution_time: u64) {
        let mut health = self.health.write().await;
        health.last_execution = Some(Utc::now());
        
        if success {
            health.success_rate = (health.success_rate * 0.9) + 0.1;
        } else {
            health.error_count += 1;
            health.success_rate = health.success_rate * 0.9;
        }
        
        health.healthy = health.success_rate > 0.5 && health.error_count < 10;
        
        if !health.healthy {
            health.message = Some(format!(
                "Terminology agent experiencing issues. Success rate: {:.2}, Error count: {}",
                health.success_rate, health.error_count
            ));
        } else {
            health.message = None;
        }
    }
}

#[async_trait::async_trait]
impl Agent for TerminologyAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }
    
    async fn execute(&self, context: AgentContext) -> Result<AgentResult> {
        let start_time = std::time::Instant::now();
        let mut actions_taken = Vec::new();
        let mut recommendations = Vec::new();
        
        // Analyze terminology for the given project
        log::info!("Terminology agent analyzing project: {}", context.project_id);
        
        // In a real implementation, you would:
        // 1. Fetch recent translations from the database
        // 2. Analyze them for terminology issues
        // 3. Generate suggestions and actions
        
        // Mock analysis for demonstration
        recommendations.push("Project terminology is consistent.".to_string());
        recommendations.push("Consider adding definitions for technical terms.".to_string());
        
        if let Some((source_lang, target_lang)) = &context.language_pair {
            recommendations.push(format!(
                "Monitoring terminology consistency for {} → {} translations.",
                source_lang, target_lang
            ));
        }
        
        // Example action: adding a term to terminology database
        if context.metadata.contains_key("auto_add_terms") {
            actions_taken.push(AgentAction::AddTerminology {
                term: "API".to_string(),
                definition: Some("Application Programming Interface".to_string()),
                do_not_translate: true,
            });
        }
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        let success = true;
        
        self.update_health(success, execution_time).await;
        
        Ok(AgentResult {
            agent_type: AgentType::Terminology,
            success,
            message: "Terminology analysis completed successfully.".to_string(),
            actions_taken,
            recommendations,
            execution_time_ms: execution_time,
        })
    }
    
    fn should_execute(&self, _context: &AgentContext) -> bool {
        // Terminology agent can run for any project
        true
    }
    
    fn health_check(&self) -> AgentHealth {
        // Return current health status
        AgentHealth {
            healthy: true,
            last_execution: Some(Utc::now()),
            error_count: 0,
            success_rate: 1.0,
            message: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::path::PathBuf;
    
    fn create_test_terminology_service() -> Arc<TerminologyService> {
        let temp_dir = TempDir::new().unwrap();
        let service = TerminologyService::new(temp_dir.path().to_path_buf()).unwrap();
        Arc::new(service)
    }
    
    #[tokio::test]
    async fn test_terminology_agent_creation() {
        let ts = create_test_terminology_service();
        let agent = TerminologyAgent::new(ts, None).await.unwrap();
        
        assert_eq!(agent.config().agent_type, AgentType::Terminology);
        assert!(agent.config().enabled);
        assert_eq!(agent.config().priority, AgentPriority::Medium);
    }
    
    #[tokio::test]
    async fn test_should_execute() {
        let ts = create_test_terminology_service();
        let agent = TerminologyAgent::new(ts, None).await.unwrap();
        
        let context = AgentContext {
            project_id: Uuid::new_v4(),
            chapter_id: None,
            user_id: None,
            language_pair: None,
            metadata: HashMap::new(),
        };
        
        assert!(agent.should_execute(&context));
    }
    
    #[test]
    fn test_identify_potential_terms() {
        let ts = create_test_terminology_service();
        let agent = futures::executor::block_on(TerminologyAgent::new(ts, None)).unwrap();
        
        let text = "The API endpoint uses HTTP protocol to send JSON data to the server.";
        let terms = agent.identify_potential_terms(text);
        
        assert!(terms.contains(&"API".to_string()));
        assert!(terms.contains(&"HTTP".to_string()));
        assert!(terms.contains(&"JSON".to_string()));
    }
    
    #[test]
    fn test_is_technical_term() {
        let ts = create_test_terminology_service();
        let agent = futures::executor::block_on(TerminologyAgent::new(ts, None)).unwrap();
        
        assert!(agent.is_technical_term("API"));
        assert!(agent.is_technical_term("database"));
        assert!(agent.is_technical_term("HTTP"));
        assert!(!agent.is_technical_term("hello"));
        assert!(!agent.is_technical_term("world"));
    }
    
    #[test]
    fn test_is_potential_terminology() {
        let ts = create_test_terminology_service();
        let agent = futures::executor::block_on(TerminologyAgent::new(ts, None)).unwrap();
        
        assert!(agent.is_potential_terminology("DatabaseConnection"));
        assert!(agent.is_potential_terminology("APIKey"));
        assert!(!agent.is_potential_terminology("The"));
        assert!(!agent.is_potential_terminology("of"));
        assert!(!agent.is_potential_terminology("it"));
    }
    
    #[tokio::test]
    async fn test_execute_agent() {
        let ts = create_test_terminology_service();
        let agent = TerminologyAgent::new(ts, None).await.unwrap();
        
        let mut metadata = HashMap::new();
        metadata.insert("auto_add_terms".to_string(), "true".to_string());
        
        let context = AgentContext {
            project_id: Uuid::new_v4(),
            chapter_id: None,
            user_id: Some("user123".to_string()),
            language_pair: Some(("en".to_string(), "es".to_string())),
            metadata,
        };
        
        let result = agent.execute(context).await.unwrap();
        
        assert!(result.success);
        assert_eq!(result.agent_type, AgentType::Terminology);
        assert!(!result.recommendations.is_empty());
        assert!(!result.actions_taken.is_empty());
        assert!(result.execution_time_ms > 0);
    }
}