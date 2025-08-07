/// Quality Agent for autonomous quality assurance and validation
/// 
/// This agent performs comprehensive quality checks on translations,
/// validates consistency, and ensures adherence to quality standards.

use super::{Agent, AgentConfig, AgentContext, AgentResult, AgentAction, AgentHealth, AgentType, AgentPriority};

use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use anyhow::Result;
use tokio::sync::RwLock;
use regex::Regex;

/// Quality agent for comprehensive quality assurance
pub struct QualityAgent {
    config: AgentConfig,
    health: Arc<RwLock<AgentHealth>>,
    quality_standards: QualityStandards,
}

/// Quality standards and thresholds
#[derive(Debug, Clone)]
pub struct QualityStandards {
    pub minimum_confidence_score: f32,
    pub maximum_length_ratio: f32,
    pub minimum_readability_score: f32,
    pub require_spell_check: bool,
    pub require_grammar_check: bool,
    pub require_cultural_validation: bool,
    pub minimum_consistency_score: f32,
}

impl Default for QualityStandards {
    fn default() -> Self {
        Self {
            minimum_confidence_score: 0.8,
            maximum_length_ratio: 2.5,
            minimum_readability_score: 0.7,
            require_spell_check: true,
            require_grammar_check: true,
            require_cultural_validation: false,
            minimum_consistency_score: 0.85,
        }
    }
}

/// Quality issue severity levels
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum QualitySeverity {
    Info = 1,
    Warning = 2,
    Error = 3,
    Critical = 4,
}

/// Quality issue types
#[derive(Debug, Clone)]
pub enum QualityIssue {
    LowConfidence {
        confidence: f32,
        threshold: f32,
        severity: QualitySeverity,
    },
    LengthMismatch {
        source_length: usize,
        target_length: usize,
        ratio: f32,
        severity: QualitySeverity,
    },
    SpellingError {
        word: String,
        suggestions: Vec<String>,
        position: usize,
        severity: QualitySeverity,
    },
    GrammarError {
        error_type: String,
        description: String,
        position: usize,
        severity: QualitySeverity,
    },
    ConsistencyViolation {
        expected: String,
        actual: String,
        similarity: f32,
        severity: QualitySeverity,
    },
    ReadabilityIssue {
        score: f32,
        threshold: f32,
        suggestions: Vec<String>,
        severity: QualitySeverity,
    },
    CulturalIssue {
        issue_type: String,
        description: String,
        suggestion: String,
        severity: QualitySeverity,
    },
    FormatInconsistency {
        expected_format: String,
        actual_format: String,
        severity: QualitySeverity,
    },
}

/// Quality metrics for a translation
#[derive(Debug, Clone)]
pub struct QualityMetrics {
    pub overall_score: f32,
    pub confidence_score: f32,
    pub readability_score: f32,
    pub consistency_score: f32,
    pub length_ratio: f32,
    pub issue_count: usize,
    pub critical_issues: usize,
    pub warnings: usize,
}

impl QualityAgent {
    /// Create a new quality agent
    pub async fn new(standards: Option<QualityStandards>) -> Result<Self> {
        let config = AgentConfig {
            agent_type: AgentType::Quality,
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
            health: Arc::new(RwLock::new(health)),
            quality_standards: standards.unwrap_or_default(),
        })
    }
    
    /// Perform comprehensive quality analysis
    async fn analyze_quality(
        &self,
        source_text: &str,
        target_text: &str,
        confidence_score: f32,
        language_pair: &(String, String),
    ) -> Result<(Vec<QualityIssue>, QualityMetrics)> {
        let mut issues = Vec::new();
        
        // Check confidence score
        if confidence_score < self.quality_standards.minimum_confidence_score {
            issues.push(QualityIssue::LowConfidence {
                confidence: confidence_score,
                threshold: self.quality_standards.minimum_confidence_score,
                severity: if confidence_score < 0.5 { QualitySeverity::Critical } else { QualitySeverity::Warning },
            });
        }
        
        // Check length ratio
        let length_ratio = self.calculate_length_ratio(source_text, target_text);
        if length_ratio > self.quality_standards.maximum_length_ratio {
            issues.push(QualityIssue::LengthMismatch {
                source_length: source_text.len(),
                target_length: target_text.len(),
                ratio: length_ratio,
                severity: if length_ratio > 5.0 { QualitySeverity::Error } else { QualitySeverity::Warning },
            });
        }
        
        // Check readability
        let readability_score = self.calculate_readability_score(target_text);
        if readability_score < self.quality_standards.minimum_readability_score {
            issues.push(QualityIssue::ReadabilityIssue {
                score: readability_score,
                threshold: self.quality_standards.minimum_readability_score,
                suggestions: self.generate_readability_suggestions(target_text),
                severity: QualitySeverity::Warning,
            });
        }
        
        // Check spelling if enabled
        if self.quality_standards.require_spell_check {
            issues.extend(self.check_spelling(target_text, &language_pair.1).await?);
        }
        
        // Check grammar if enabled
        if self.quality_standards.require_grammar_check {
            issues.extend(self.check_grammar(target_text, &language_pair.1).await?);
        }
        
        // Check format consistency
        issues.extend(self.check_format_consistency(source_text, target_text)?);
        
        // Check cultural appropriateness if enabled
        if self.quality_standards.require_cultural_validation {
            issues.extend(self.check_cultural_appropriateness(target_text, &language_pair.1).await?);
        }
        
        // Calculate metrics
        let metrics = self.calculate_quality_metrics(&issues, confidence_score, readability_score, length_ratio);
        
        Ok((issues, metrics))
    }
    
    /// Calculate length ratio between source and target text
    fn calculate_length_ratio(&self, source_text: &str, target_text: &str) -> f32 {
        if source_text.is_empty() {
            return if target_text.is_empty() { 1.0 } else { f32::INFINITY };
        }
        target_text.len() as f32 / source_text.len() as f32
    }
    
    /// Calculate readability score (simplified implementation)
    fn calculate_readability_score(&self, text: &str) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        
        let sentences = text.split(&['.', '!', '?'][..]).count().max(1);
        let words = text.split_whitespace().count();
        let characters = text.chars().count();
        
        // Simplified Flesch-like score
        let avg_sentence_length = words as f32 / sentences as f32;
        let avg_word_length = characters as f32 / words.max(1) as f32;
        
        // Normalize to 0-1 range (higher is better)
        let complexity = (avg_sentence_length / 20.0) + (avg_word_length / 10.0);
        (2.0 - complexity).max(0.0).min(1.0)
    }
    
    /// Generate readability improvement suggestions
    fn generate_readability_suggestions(&self, text: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        let sentences = text.split(&['.', '!', '?'][..]).count();
        let words = text.split_whitespace().count();
        
        if sentences > 0 {
            let avg_sentence_length = words as f32 / sentences as f32;
            if avg_sentence_length > 25.0 {
                suggestions.push("Consider breaking long sentences into shorter ones.".to_string());
            }
        }
        
        // Check for complex words (simplified heuristic)
        let complex_words: Vec<&str> = text
            .split_whitespace()
            .filter(|word| word.len() > 12)
            .collect();
        
        if !complex_words.is_empty() {
            suggestions.push("Consider using simpler alternatives for complex words.".to_string());
        }
        
        // Check for passive voice (very simplified)
        if text.contains(" was ") || text.contains(" were ") || text.contains(" been ") {
            suggestions.push("Consider using active voice where appropriate.".to_string());
        }
        
        if suggestions.is_empty() {
            suggestions.push("Text readability is acceptable.".to_string());
        }
        
        suggestions
    }
    
    /// Check spelling (simplified implementation)
    async fn check_spelling(&self, text: &str, language: &str) -> Result<Vec<QualityIssue>> {
        let mut issues = Vec::new();
        
        // This is a simplified implementation
        // In practice, you would integrate with a real spell checker
        
        let suspicious_patterns = [
            r"\b\w*\d\w*\b", // Words with numbers (might be typos)
            r"\b\w{20,}\b",  // Very long words (might be typos)
        ];
        
        for pattern in &suspicious_patterns {
            let regex = Regex::new(pattern)?;
            for mat in regex.find_iter(text) {
                issues.push(QualityIssue::SpellingError {
                    word: mat.as_str().to_string(),
                    suggestions: vec!["[manual review needed]".to_string()],
                    position: mat.start(),
                    severity: QualitySeverity::Warning,
                });
            }
        }
        
        Ok(issues)
    }
    
    /// Check grammar (simplified implementation)
    async fn check_grammar(&self, text: &str, language: &str) -> Result<Vec<QualityIssue>> {
        let mut issues = Vec::new();
        
        // This is a very simplified implementation
        // In practice, you would integrate with a grammar checker like LanguageTool
        
        // Check for double spaces
        if text.contains("  ") {
            issues.push(QualityIssue::GrammarError {
                error_type: "spacing".to_string(),
                description: "Multiple consecutive spaces found".to_string(),
                position: text.find("  ").unwrap_or(0),
                severity: QualitySeverity::Info,
            });
        }
        
        // Check for missing punctuation at end
        if !text.is_empty() && !text.ends_with(&['.', '!', '?', ':', ';'][..]) {
            issues.push(QualityIssue::GrammarError {
                error_type: "punctuation".to_string(),
                description: "Sentence may be missing ending punctuation".to_string(),
                position: text.len(),
                severity: QualitySeverity::Warning,
            });
        }
        
        Ok(issues)
    }
    
    /// Check format consistency between source and target
    fn check_format_consistency(&self, source_text: &str, target_text: &str) -> Result<Vec<QualityIssue>> {
        let mut issues = Vec::new();
        
        // Check HTML tags consistency
        let html_tag_regex = Regex::new(r"<[^>]+>")?;
        let source_tags: Vec<&str> = html_tag_regex.find_iter(source_text).map(|m| m.as_str()).collect();
        let target_tags: Vec<&str> = html_tag_regex.find_iter(target_text).map(|m| m.as_str()).collect();
        
        if source_tags.len() != target_tags.len() {
            issues.push(QualityIssue::FormatInconsistency {
                expected_format: format!("{} HTML tags", source_tags.len()),
                actual_format: format!("{} HTML tags", target_tags.len()),
                severity: QualitySeverity::Error,
            });
        }
        
        // Check placeholder consistency (simplified)
        let placeholder_regex = Regex::new(r"\{\w+\}")?;
        let source_placeholders: Vec<&str> = placeholder_regex.find_iter(source_text).map(|m| m.as_str()).collect();
        let target_placeholders: Vec<&str> = placeholder_regex.find_iter(target_text).map(|m| m.as_str()).collect();
        
        if source_placeholders.len() != target_placeholders.len() {
            issues.push(QualityIssue::FormatInconsistency {
                expected_format: format!("{} placeholders", source_placeholders.len()),
                actual_format: format!("{} placeholders", target_placeholders.len()),
                severity: QualitySeverity::Error,
            });
        }
        
        Ok(issues)
    }
    
    /// Check cultural appropriateness (simplified implementation)
    async fn check_cultural_appropriateness(&self, text: &str, language: &str) -> Result<Vec<QualityIssue>> {
        let mut issues = Vec::new();
        
        // This is a very simplified implementation
        // In practice, you would use cultural validation databases and rules
        
        // Check for potentially sensitive terms (example for demonstration)
        let sensitive_patterns = match language {
            "ar" | "fa" | "ur" => vec!["alcohol", "pork"], // Islamic cultural considerations
            "hi" | "ta" | "te" => vec!["beef"], // Hindu cultural considerations
            _ => vec![],
        };
        
        for pattern in sensitive_patterns {
            if text.to_lowercase().contains(pattern) {
                issues.push(QualityIssue::CulturalIssue {
                    issue_type: "cultural_sensitivity".to_string(),
                    description: format!("Term '{}' may be culturally sensitive", pattern),
                    suggestion: "Consider cultural context and alternative wording".to_string(),
                    severity: QualitySeverity::Warning,
                });
            }
        }
        
        Ok(issues)
    }
    
    /// Calculate overall quality metrics
    fn calculate_quality_metrics(
        &self,
        issues: &[QualityIssue],
        confidence_score: f32,
        readability_score: f32,
        length_ratio: f32,
    ) -> QualityMetrics {
        let critical_issues = issues.iter()
            .filter(|issue| self.get_issue_severity(issue) == QualitySeverity::Critical)
            .count();
        
        let warnings = issues.iter()
            .filter(|issue| matches!(self.get_issue_severity(issue), QualitySeverity::Warning))
            .count();
        
        // Calculate consistency score (simplified)
        let consistency_score = if issues.iter().any(|issue| {
            matches!(issue, QualityIssue::ConsistencyViolation { .. })
        }) {
            0.5
        } else {
            0.9
        };
        
        // Calculate overall score
        let mut overall_score = confidence_score * 0.3
            + readability_score * 0.2
            + consistency_score * 0.2;
        
        // Penalize for issues
        if critical_issues > 0 {
            overall_score *= 0.5;
        } else if warnings > 0 {
            overall_score *= 0.8;
        }
        
        // Penalize for length ratio issues
        if length_ratio > self.quality_standards.maximum_length_ratio {
            overall_score *= 0.9;
        }
        
        overall_score = overall_score.max(0.0).min(1.0);
        
        QualityMetrics {
            overall_score,
            confidence_score,
            readability_score,
            consistency_score,
            length_ratio,
            issue_count: issues.len(),
            critical_issues,
            warnings,
        }
    }
    
    /// Get severity of a quality issue
    fn get_issue_severity(&self, issue: &QualityIssue) -> QualitySeverity {
        match issue {
            QualityIssue::LowConfidence { severity, .. } => severity.clone(),
            QualityIssue::LengthMismatch { severity, .. } => severity.clone(),
            QualityIssue::SpellingError { severity, .. } => severity.clone(),
            QualityIssue::GrammarError { severity, .. } => severity.clone(),
            QualityIssue::ConsistencyViolation { severity, .. } => severity.clone(),
            QualityIssue::ReadabilityIssue { severity, .. } => severity.clone(),
            QualityIssue::CulturalIssue { severity, .. } => severity.clone(),
            QualityIssue::FormatInconsistency { severity, .. } => severity.clone(),
        }
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
                "Quality agent experiencing issues. Success rate: {:.2}, Error count: {}",
                health.success_rate, health.error_count
            ));
        } else {
            health.message = None;
        }
    }
}

#[async_trait::async_trait]
impl Agent for QualityAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }
    
    async fn execute(&self, context: AgentContext) -> Result<AgentResult> {
        let start_time = std::time::Instant::now();
        let mut actions_taken = Vec::new();
        let mut recommendations = Vec::new();
        
        log::info!("Quality agent analyzing project: {}", context.project_id);
        
        // In a real implementation, you would:
        // 1. Fetch translations to analyze from the database
        // 2. Run quality checks on each translation
        // 3. Generate quality reports and suggestions
        // 4. Flag issues that need human review
        
        // Mock quality analysis for demonstration
        recommendations.push("Overall translation quality is good.".to_string());
        recommendations.push("Found minor formatting inconsistencies to review.".to_string());
        recommendations.push("Consider spell-checking target language text.".to_string());
        
        if let Some((source_lang, target_lang)) = &context.language_pair {
            recommendations.push(format!(
                "Quality standards enforced for {} → {} translations.",
                source_lang, target_lang
            ));
        }
        
        // Example quality issue flagging
        if context.metadata.contains_key("flag_quality_issues") {
            actions_taken.push(AgentAction::MarkQualityIssue {
                chunk_id: Uuid::new_v4(),
                issue_type: "readability".to_string(),
                severity: "warning".to_string(),
                description: "Text readability could be improved".to_string(),
            });
        }
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        let success = true;
        
        self.update_health(success, execution_time).await;
        
        Ok(AgentResult {
            agent_type: AgentType::Quality,
            success,
            message: "Quality analysis completed successfully.".to_string(),
            actions_taken,
            recommendations,
            execution_time_ms: execution_time,
        })
    }
    
    fn should_execute(&self, context: &AgentContext) -> bool {
        // Quality agent should run when there's content to analyze
        context.chapter_id.is_some() || context.language_pair.is_some()
    }
    
    fn health_check(&self) -> AgentHealth {
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
    
    #[tokio::test]
    async fn test_quality_agent_creation() {
        let agent = QualityAgent::new(None).await.unwrap();
        
        assert_eq!(agent.config().agent_type, AgentType::Quality);
        assert!(agent.config().enabled);
        assert_eq!(agent.config().priority, AgentPriority::High);
    }
    
    #[test]
    fn test_calculate_length_ratio() {
        let agent = futures::executor::block_on(QualityAgent::new(None)).unwrap();
        
        assert_eq!(agent.calculate_length_ratio("hello", "hello"), 1.0);
        assert_eq!(agent.calculate_length_ratio("hi", "hello"), 2.5);
        assert_eq!(agent.calculate_length_ratio("", "hello"), f32::INFINITY);
        assert_eq!(agent.calculate_length_ratio("", ""), 1.0);
    }
    
    #[test]
    fn test_calculate_readability_score() {
        let agent = futures::executor::block_on(QualityAgent::new(None)).unwrap();
        
        let score = agent.calculate_readability_score("Hello world. This is a test.");
        assert!(score > 0.0 && score <= 1.0);
        
        let empty_score = agent.calculate_readability_score("");
        assert_eq!(empty_score, 0.0);
    }
    
    #[test]
    fn test_generate_readability_suggestions() {
        let agent = futures::executor::block_on(QualityAgent::new(None)).unwrap();
        
        let long_sentence = "This is a very long sentence with many words that continues for a very long time and probably should be broken into shorter sentences for better readability and comprehension by the reader.";
        let suggestions = agent.generate_readability_suggestions(long_sentence);
        
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains("shorter")));
    }
    
    #[tokio::test]
    async fn test_check_format_consistency() {
        let agent = QualityAgent::new(None).await.unwrap();
        
        let source = "Hello <b>world</b> {name}!";
        let target_good = "Hola <b>mundo</b> {name}!";
        let target_bad = "Hola mundo!";
        
        let issues_good = agent.check_format_consistency(source, target_good).unwrap();
        assert!(issues_good.is_empty());
        
        let issues_bad = agent.check_format_consistency(source, target_bad).unwrap();
        assert!(!issues_bad.is_empty());
    }
    
    #[tokio::test]
    async fn test_should_execute() {
        let agent = QualityAgent::new(None).await.unwrap();
        
        let context_with_chapter = AgentContext {
            project_id: Uuid::new_v4(),
            chapter_id: Some(Uuid::new_v4()),
            user_id: None,
            language_pair: None,
            metadata: HashMap::new(),
        };
        
        assert!(agent.should_execute(&context_with_chapter));
        
        let context_without_content = AgentContext {
            project_id: Uuid::new_v4(),
            chapter_id: None,
            user_id: None,
            language_pair: None,
            metadata: HashMap::new(),
        };
        
        assert!(!agent.should_execute(&context_without_content));
    }
    
    #[tokio::test]
    async fn test_execute_agent() {
        let agent = QualityAgent::new(None).await.unwrap();
        
        let mut metadata = HashMap::new();
        metadata.insert("flag_quality_issues".to_string(), "true".to_string());
        
        let context = AgentContext {
            project_id: Uuid::new_v4(),
            chapter_id: Some(Uuid::new_v4()),
            user_id: Some("user123".to_string()),
            language_pair: Some(("en".to_string(), "es".to_string())),
            metadata,
        };
        
        let result = agent.execute(context).await.unwrap();
        
        assert!(result.success);
        assert_eq!(result.agent_type, AgentType::Quality);
        assert!(!result.recommendations.is_empty());
        assert!(!result.actions_taken.is_empty());
        assert!(result.execution_time_ms > 0);
    }
}