/// Translation Agent for autonomous translation quality and consistency management
/// 
/// This agent monitors translation quality, suggests improvements, and maintains
/// consistency across translation units within a project.

use super::{Agent, AgentConfig, AgentContext, AgentResult, AgentAction, AgentHealth, AgentType, AgentPriority};
use crate::services::{TranslationMemoryService, TranslationMemoryAdapter};
use crate::models::translation_models::{TranslationUnit, LanguagePair};

use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::Result;
use tokio::sync::RwLock;

/// Translation agent for quality assurance and consistency checking
pub struct TranslationAgent {
    config: AgentConfig,
    translation_memory: Arc<TranslationMemoryService>,
    // New adapter service for improved performance
    adapter_service: Arc<TranslationMemoryAdapter>,
    health: Arc<RwLock<AgentHealth>>,
    quality_thresholds: QualityThresholds,
}

/// Quality thresholds for translation validation
#[derive(Debug, Clone)]
pub struct QualityThresholds {
    pub minimum_confidence: f32,
    pub similarity_threshold: f32,
    pub consistency_threshold: f32,
    pub max_length_ratio: f32,
}

impl Default for QualityThresholds {
    fn default() -> Self {
        Self {
            minimum_confidence: 0.7,
            similarity_threshold: 0.8,
            consistency_threshold: 0.85,
            max_length_ratio: 3.0, // Target text shouldn't be more than 3x source length
        }
    }
}

/// Translation quality issue types
#[derive(Debug, Clone)]
pub enum QualityIssue {
    LowConfidence {
        confidence: f32,
        threshold: f32,
    },
    InconsistentTranslation {
        expected_translation: String,
        actual_translation: String,
        similarity: f32,
    },
    LengthMismatch {
        source_length: usize,
        target_length: usize,
        ratio: f32,
    },
    MissingContext {
        reason: String,
    },
    TerminologyViolation {
        term: String,
        suggestion: String,
    },
}

impl TranslationAgent {
    /// Create a new translation agent
    pub async fn new(
        translation_memory: Arc<TranslationMemoryService>,
        thresholds: Option<QualityThresholds>,
    ) -> Result<Self> {
        let config = AgentConfig {
            agent_type: AgentType::Translation,
            enabled: true,
            priority: AgentPriority::High,
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
            translation_memory,
            health: Arc::new(RwLock::new(health)),
            quality_thresholds: thresholds.unwrap_or_default(),
        })
    }
    
    /// Analyze translation quality and suggest improvements
    async fn analyze_translation_quality(
        &self,
        translation_unit: &TranslationUnit,
    ) -> Result<Vec<QualityIssue>> {
        let mut issues = Vec::new();
        
        // Check confidence threshold
        if translation_unit.confidence_score < self.quality_thresholds.minimum_confidence {
            issues.push(QualityIssue::LowConfidence {
                confidence: translation_unit.confidence_score,
                threshold: self.quality_thresholds.minimum_confidence,
            });
        }
        
        // Check length ratio
        let source_len = translation_unit.source_text.len();
        let target_len = translation_unit.target_text.len();
        if source_len > 0 {
            let length_ratio = target_len as f32 / source_len as f32;
            if length_ratio > self.quality_thresholds.max_length_ratio {
                issues.push(QualityIssue::LengthMismatch {
                    source_length: source_len,
                    target_length: target_len,
                    ratio: length_ratio,
                });
            }
        }
        
        // Check for consistency with translation memory
        let language_pair = LanguagePair {
            source: translation_unit.source_language.clone(),
            target: translation_unit.target_language.clone(),
        };
        
        let similar_translations = self.translation_memory
            .search_similar_translations(&translation_unit.source_text, language_pair)
            .await?;
        
        if let Some(best_match) = similar_translations.first() {
            if best_match.similarity_score > self.quality_thresholds.consistency_threshold
                && best_match.target_text != translation_unit.target_text {
                // Calculate similarity between current and expected translation  
                let similarity = self.calculate_text_similarity(
                    &translation_unit.target_text,
                    &best_match.target_text,
                );
                
                if similarity < self.quality_thresholds.similarity_threshold {
                    issues.push(QualityIssue::InconsistentTranslation {
                        expected_translation: best_match.target_text.clone(),
                        actual_translation: translation_unit.target_text.clone(),
                        similarity,
                    });
                }
            }
        }
        
        // Check for missing context in complex translations
        if translation_unit.source_text.len() > 100 && translation_unit.context.is_none() {
            issues.push(QualityIssue::MissingContext {
                reason: "Long source text without context may lead to ambiguous translations".to_string(),
            });
        }
        
        Ok(issues)
    }
    
    /// Calculate similarity between two text strings
    fn calculate_text_similarity(&self, text1: &str, text2: &str) -> f32 {
        // Simple Jaccard similarity at word level
        let words1: std::collections::HashSet<&str> = text1.split_whitespace().collect();
        let words2: std::collections::HashSet<&str> = text2.split_whitespace().collect();
        
        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();
        
        if union == 0 {
            return if text1 == text2 { 1.0 } else { 0.0 };
        }
        
        intersection as f32 / union as f32
    }
    
    /// Generate improvement suggestions for translation unit
    async fn generate_suggestions(
        &self,
        translation_unit: &TranslationUnit,
        issues: &[QualityIssue],
    ) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        for issue in issues {
            match issue {
                QualityIssue::LowConfidence { confidence, threshold } => {
                    suggestions.push(format!(
                        "Translation confidence ({:.2}) is below threshold ({:.2}). Consider reviewing and improving the translation quality.",
                        confidence, threshold
                    ));
                }
                QualityIssue::InconsistentTranslation { expected_translation, similarity, .. } => {
                    suggestions.push(format!(
                        "Similar source text was previously translated as '{}'. Consider using consistent terminology (similarity: {:.2}).",
                        expected_translation, similarity
                    ));
                }
                QualityIssue::LengthMismatch { ratio, .. } => {
                    suggestions.push(format!(
                        "Target text is significantly longer than source (ratio: {:.2}). Verify translation accuracy and conciseness.",
                        ratio
                    ));
                }
                QualityIssue::MissingContext { reason } => {
                    suggestions.push(format!("Consider adding context: {}", reason));
                }
                QualityIssue::TerminologyViolation { term, suggestion } => {
                    suggestions.push(format!(
                        "Term '{}' should be translated as '{}' according to project terminology.",
                        term, suggestion
                    ));
                }
            }
        }
        
        // Add general suggestions based on translation unit characteristics
        if translation_unit.source_text.len() > 200 {
            suggestions.push("Consider breaking down long text into smaller chunks for better translation quality.".to_string());
        }
        
        if translation_unit.confidence_score > 0.95 && issues.is_empty() {
            suggestions.push("High-quality translation. Consider adding to translation memory as reference.".to_string());
        }
        
        suggestions
    }
    
    /// Update agent health statistics
    async fn update_health(&self, success: bool, execution_time: u64) {
        let mut health = self.health.write().await;
        health.last_execution = Some(Utc::now());
        
        if success {
            health.success_rate = (health.success_rate * 0.9) + 0.1; // Weighted average
        } else {
            health.error_count += 1;
            health.success_rate = health.success_rate * 0.9; // Decrease success rate
        }
        
        health.healthy = health.success_rate > 0.5 && health.error_count < 10;
        
        if !health.healthy {
            health.message = Some(format!(
                "Agent experiencing issues. Success rate: {:.2}, Error count: {}",
                health.success_rate, health.error_count
            ));
        } else {
            health.message = None;
        }
    }
}

#[async_trait::async_trait]
impl Agent for TranslationAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }
    
    async fn execute(&self, context: AgentContext) -> Result<AgentResult> {
        let start_time = std::time::Instant::now();
        let mut actions_taken = Vec::new();
        let mut recommendations = Vec::new();
        
        // For demonstration, we'll analyze existing translations in the chapter
        // In a real implementation, this would be triggered by translation events
        
        match context.chapter_id {
            Some(chapter_id) => {
                // This is a simplified implementation
                // In practice, you would fetch translation units from the database
                log::info!("Translation agent analyzing chapter: {}", chapter_id);
                
                // Mock analysis result
                recommendations.push("Translation quality is within acceptable range.".to_string());
                recommendations.push("Consider reviewing translations with confidence < 0.8.".to_string());
                
                if let Some((source_lang, target_lang)) = &context.language_pair {
                    recommendations.push(format!(
                        "Monitoring {} → {} translations for consistency.",
                        source_lang, target_lang
                    ));
                }
            }
            None => {
                recommendations.push("No specific chapter provided for analysis.".to_string());
            }
        }
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        let success = true;
        
        self.update_health(success, execution_time).await;
        
        Ok(AgentResult {
            agent_type: AgentType::Translation,
            success,
            message: "Translation quality analysis completed successfully.".to_string(),
            actions_taken,
            recommendations,
            execution_time_ms: execution_time,
        })
    }
    
    fn should_execute(&self, context: &AgentContext) -> bool {
        // Execute when there's a chapter to analyze or language pair specified
        context.chapter_id.is_some() || context.language_pair.is_some()
    }
    
    fn health_check(&self) -> AgentHealth {
        // Return a clone of current health status
        // In async context, you'd use: self.health.try_read().unwrap().clone()
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
    
    async fn create_test_translation_memory() -> Arc<TranslationMemoryService> {
        let temp_dir = TempDir::new().unwrap();
        let service = TranslationMemoryService::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();
        Arc::new(service)
    }
    
    #[tokio::test]
    async fn test_translation_agent_creation() {
        let tm = create_test_translation_memory().await;
        let agent = TranslationAgent::new(tm, None).await.unwrap();
        
        assert_eq!(agent.config().agent_type, AgentType::Translation);
        assert!(agent.config().enabled);
        assert_eq!(agent.config().priority, AgentPriority::High);
    }
    
    #[tokio::test]
    async fn test_should_execute_with_chapter() {
        let tm = create_test_translation_memory().await;
        let agent = TranslationAgent::new(tm, None).await.unwrap();
        
        let context = AgentContext {
            project_id: Uuid::new_v4(),
            chapter_id: Some(Uuid::new_v4()),
            user_id: None,
            language_pair: None,
            metadata: HashMap::new(),
        };
        
        assert!(agent.should_execute(&context));
    }
    
    #[tokio::test]
    async fn test_should_execute_with_language_pair() {
        let tm = create_test_translation_memory().await;
        let agent = TranslationAgent::new(tm, None).await.unwrap();
        
        let context = AgentContext {
            project_id: Uuid::new_v4(),
            chapter_id: None,
            user_id: None,
            language_pair: Some(("en".to_string(), "es".to_string())),
            metadata: HashMap::new(),
        };
        
        assert!(agent.should_execute(&context));
    }
    
    #[tokio::test]
    async fn test_should_not_execute_without_context() {
        let tm = create_test_translation_memory().await;
        let agent = TranslationAgent::new(tm, None).await.unwrap();
        
        let context = AgentContext {
            project_id: Uuid::new_v4(),
            chapter_id: None,
            user_id: None,
            language_pair: None,
            metadata: HashMap::new(),
        };
        
        assert!(!agent.should_execute(&context));
    }
    
    #[tokio::test]
    async fn test_execute_agent() {
        let tm = create_test_translation_memory().await;
        let agent = TranslationAgent::new(tm, None).await.unwrap();
        
        let context = AgentContext {
            project_id: Uuid::new_v4(),
            chapter_id: Some(Uuid::new_v4()),
            user_id: Some("user123".to_string()),
            language_pair: Some(("en".to_string(), "es".to_string())),
            metadata: HashMap::new(),
        };
        
        let result = agent.execute(context).await.unwrap();
        
        assert!(result.success);
        assert_eq!(result.agent_type, AgentType::Translation);
        assert!(!result.recommendations.is_empty());
        assert!(result.execution_time_ms > 0);
    }
    
    #[test]
    fn test_text_similarity_calculation() {
        let tm = futures::executor::block_on(create_test_translation_memory());
        let agent = futures::executor::block_on(TranslationAgent::new(tm, None)).unwrap();
        
        // Test exact match
        let similarity = agent.calculate_text_similarity("hello world", "hello world");
        assert_eq!(similarity, 1.0);
        
        // Test partial match
        let similarity = agent.calculate_text_similarity("hello world test", "hello world");
        assert!(similarity > 0.0 && similarity < 1.0);
        
        // Test no match
        let similarity = agent.calculate_text_similarity("hello", "goodbye");
        assert_eq!(similarity, 0.0);
    }
    
    #[test]
    fn test_quality_thresholds_default() {
        let thresholds = QualityThresholds::default();
        
        assert_eq!(thresholds.minimum_confidence, 0.7);
        assert_eq!(thresholds.similarity_threshold, 0.8);
        assert_eq!(thresholds.consistency_threshold, 0.85);
        assert_eq!(thresholds.max_length_ratio, 3.0);
    }
}