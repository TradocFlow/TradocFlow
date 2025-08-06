use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::services::translation_memory_adapter::{
    TranslationMemoryAdapter, TranslationMatch, TranslationSource
};
use crate::models::{
    document::TranslationUnit,
    translation_models::LanguagePair
};

/// Configuration for translation memory integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    pub auto_suggest_enabled: bool,
    pub confidence_threshold: f32,
    pub max_suggestions: usize,
    pub debounce_delay_ms: u64,
    pub auto_create_units: bool,
    pub similarity_threshold: f32,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            auto_suggest_enabled: true,
            confidence_threshold: 0.7,
            max_suggestions: 5,
            debounce_delay_ms: 500,
            auto_create_units: true,
            similarity_threshold: 0.3,
        }
    }
}

/// Real-time translation suggestion for editor integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSuggestion {
    pub id: Uuid,
    pub source_text: String,
    pub suggested_text: String,
    pub confidence: f32,
    pub similarity: f32,
    pub context: Option<String>,
    pub source: TranslationSource,
    pub position: TextPosition,
    pub created_at: DateTime<Utc>,
}

/// Text position information for suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPosition {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

/// Translation confidence indicator for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceIndicator {
    pub position: TextPosition,
    pub confidence: f32,
    pub quality_score: Option<f32>,
    pub indicator_type: IndicatorType,
}

/// Types of confidence indicators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IndicatorType {
    High,      // Green - high confidence
    Medium,    // Yellow - medium confidence  
    Low,       // Red - low confidence
    New,       // Blue - new translation
    Suggested, // Purple - suggested by TM
}

/// Service for integrating translation memory with the editor
#[derive(Clone)]
pub struct TranslationMemoryIntegrationService {
    translation_memory: Arc<TranslationMemoryAdapter>,
    config: Arc<RwLock<IntegrationConfig>>,
    active_suggestions: Arc<RwLock<HashMap<String, Vec<EditorSuggestion>>>>,
    confidence_indicators: Arc<RwLock<HashMap<String, Vec<ConfidenceIndicator>>>>,
    pending_translations: Arc<RwLock<HashMap<String, PendingTranslation>>>,
}

/// Pending translation for auto-creation
#[derive(Debug, Clone)]
struct PendingTranslation {
    source_text: String,
    target_text: String,
    language_pair: LanguagePair,
    confidence: f32,
    context: Option<String>,
    created_at: DateTime<Utc>,
}

impl TranslationMemoryIntegrationService {
    pub async fn new(translation_memory: Arc<TranslationMemoryAdapter>) -> Result<Self> {
        Ok(Self {
            translation_memory,
            config: Arc::new(RwLock::new(IntegrationConfig::default())),
            active_suggestions: Arc::new(RwLock::new(HashMap::new())),
            confidence_indicators: Arc::new(RwLock::new(HashMap::new())),
            pending_translations: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get real-time translation suggestions for editor text
    pub async fn get_real_time_suggestions(
        &self,
        text: &str,
        language_pair: LanguagePair,
        position: TextPosition,
    ) -> Result<Vec<EditorSuggestion>> {
        let config = self.config.read().await;
        
        if !config.auto_suggest_enabled || text.trim().is_empty() {
            return Ok(Vec::new());
        }

        // Search translation memory for similar text
        let matches = self.translation_memory
            .search_similar_translations(text, language_pair.clone())
            .await?;

        let mut suggestions = Vec::new();
        
        for (index, tm_match) in matches.iter().enumerate() {
            if index >= config.max_suggestions {
                break;
            }

            // Filter by similarity threshold
            if tm_match.similarity_score < config.similarity_threshold {
                continue;
            }

            let suggestion = EditorSuggestion {
                id: Uuid::new_v4(),
                source_text: tm_match.source_text.clone(),
                suggested_text: tm_match.target_text.clone(),
                confidence: tm_match.confidence_score,
                similarity: tm_match.similarity_score,
                context: tm_match.context.clone(),
                source: TranslationSource::Memory,
                position: position.clone(),
                created_at: Utc::now(),
            };

            suggestions.push(suggestion);
        }

        // Cache suggestions for this text
        let cache_key = format!("{}:{}-{}", text, language_pair.source, language_pair.target);
        {
            let mut active_suggestions = self.active_suggestions.write().await;
            active_suggestions.insert(cache_key, suggestions.clone());
        }

        Ok(suggestions)
    }

    /// Apply a translation suggestion to create a new translation unit
    pub async fn apply_suggestion(
        &self,
        suggestion: &EditorSuggestion,
        project_id: Uuid,
        chapter_id: Uuid,
        chunk_id: Uuid,
        language_pair: LanguagePair,
    ) -> Result<TranslationUnit> {
        // Create translation unit from suggestion
        let translation_unit = TranslationUnit::new(
            project_id,
            chapter_id,
            chunk_id,
            language_pair.source,
            suggestion.source_text.clone(),
            language_pair.target,
            suggestion.suggested_text.clone(),
            suggestion.confidence,
            suggestion.context.clone(),
        )?;

        // Add to translation memory
        self.translation_memory.add_translation_unit(translation_unit.clone()).await?;

        // Update confidence indicators
        self.update_confidence_indicator(
            &suggestion.source_text,
            suggestion.position.clone(),
            suggestion.confidence,
            None,
            IndicatorType::Suggested,
        ).await;

        Ok(translation_unit)
    }

    /// Automatically create translation unit when content is modified
    pub async fn auto_create_translation_unit(
        &self,
        source_text: &str,
        target_text: &str,
        language_pair: LanguagePair,
        project_id: Uuid,
        chapter_id: Uuid,
        chunk_id: Uuid,
        context: Option<String>,
    ) -> Result<Option<TranslationUnit>> {
        let config = self.config.read().await;
        
        if !config.auto_create_units || source_text.trim().is_empty() || target_text.trim().is_empty() {
            return Ok(None);
        }

        // Check if we already have a similar translation
        let existing_matches = self.translation_memory
            .search_similar_translations(source_text, language_pair.clone())
            .await?;

        // If we have an exact match, don't create a new unit
        if existing_matches.iter().any(|m| m.source_text == source_text && m.target_text == target_text) {
            return Ok(None);
        }

        // Calculate confidence based on text length and complexity
        let confidence = self.calculate_auto_confidence(source_text, target_text);

        // Create new translation unit
        let translation_unit = TranslationUnit::new(
            project_id,
            chapter_id,
            chunk_id,
            language_pair.source,
            source_text.to_string(),
            language_pair.target,
            target_text.to_string(),
            confidence,
            context,
        )?;

        // Add to translation memory
        self.translation_memory.add_translation_unit(translation_unit.clone()).await?;

        Ok(Some(translation_unit))
    }

    /// Search translation memory with advanced filtering
    pub async fn search_translation_memory(
        &self,
        query: &str,
        language_pair: LanguagePair,
        filters: SearchFilters,
    ) -> Result<Vec<TranslationMatch>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut matches = self.translation_memory
            .search_similar_translations(query, language_pair)
            .await?;

        // Apply filters
        if let Some(min_confidence) = filters.min_confidence {
            matches.retain(|m| m.confidence_score >= min_confidence);
        }

        if let Some(min_similarity) = filters.min_similarity {
            matches.retain(|m| m.similarity_score >= min_similarity);
        }

        if let Some(max_results) = filters.max_results {
            matches.truncate(max_results);
        }

        // Sort by relevance (combination of confidence and similarity)
        matches.sort_by(|a, b| {
            let score_a = (a.confidence_score + a.similarity_score) / 2.0;
            let score_b = (b.confidence_score + b.similarity_score) / 2.0;
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(matches)
    }

    /// Update confidence indicator for a text position
    pub async fn update_confidence_indicator(
        &self,
        text: &str,
        position: TextPosition,
        confidence: f32,
        quality_score: Option<f32>,
        indicator_type: IndicatorType,
    ) {
        let indicator = ConfidenceIndicator {
            position,
            confidence,
            quality_score,
            indicator_type,
        };

        let mut indicators = self.confidence_indicators.write().await;
        indicators.entry(text.to_string())
            .or_insert_with(Vec::new)
            .push(indicator);
    }

    /// Get confidence indicators for text
    pub async fn get_confidence_indicators(&self, text: &str) -> Vec<ConfidenceIndicator> {
        let indicators = self.confidence_indicators.read().await;
        indicators.get(text).cloned().unwrap_or_default()
    }

    /// Update integration configuration
    pub async fn update_config(&self, new_config: IntegrationConfig) {
        let mut config = self.config.write().await;
        *config = new_config;
    }

    /// Get current configuration
    pub async fn get_config(&self) -> IntegrationConfig {
        self.config.read().await.clone()
    }

    /// Clear cached suggestions
    pub async fn clear_suggestions_cache(&self) {
        let mut suggestions = self.active_suggestions.write().await;
        suggestions.clear();
    }

    /// Get cached suggestions for text
    pub async fn get_cached_suggestions(&self, text: &str, language_pair: &LanguagePair) -> Option<Vec<EditorSuggestion>> {
        let cache_key = format!("{}:{}-{}", text, language_pair.source, language_pair.target);
        let suggestions = self.active_suggestions.read().await;
        suggestions.get(&cache_key).cloned()
    }

    /// Calculate automatic confidence score based on text characteristics
    pub fn calculate_auto_confidence(&self, source_text: &str, target_text: &str) -> f32 {
        let mut confidence = 0.5; // Base confidence

        // Length similarity factor
        let source_len = source_text.len() as f32;
        let target_len = target_text.len() as f32;
        let length_ratio = if source_len > 0.0 {
            (target_len / source_len).min(source_len / target_len)
        } else {
            0.0
        };
        confidence += (length_ratio - 0.5) * 0.2;

        // Word count similarity
        let source_words = source_text.split_whitespace().count() as f32;
        let target_words = target_text.split_whitespace().count() as f32;
        let word_ratio = if source_words > 0.0 {
            (target_words / source_words).min(source_words / target_words)
        } else {
            0.0
        };
        confidence += (word_ratio - 0.5) * 0.2;

        // Complexity factor (longer texts get slightly lower confidence)
        if source_text.len() > 100 {
            confidence -= 0.1;
        }

        // Ensure confidence is within valid range
        confidence.max(0.1).min(0.9)
    }

    /// Get translation statistics for analytics
    pub async fn get_translation_statistics(&self) -> Result<TranslationStatistics> {
        // This would integrate with the translation memory service to get stats
        let (cache_entries, indicator_entries, _) = {
            let suggestions = self.active_suggestions.read().await;
            let indicators = self.confidence_indicators.read().await;
            let pending = self.pending_translations.read().await;
            (suggestions.len(), indicators.len(), pending.len())
        };

        Ok(TranslationStatistics {
            cached_suggestions: cache_entries,
            active_indicators: indicator_entries,
            auto_created_units: 0, // Would need to track this
            average_confidence: 0.0, // Would calculate from actual data
        })
    }
}

/// Search filters for translation memory queries
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub min_confidence: Option<f32>,
    pub min_similarity: Option<f32>,
    pub max_results: Option<usize>,
    pub include_context: bool,
}

/// Translation statistics for analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationStatistics {
    pub cached_suggestions: usize,
    pub active_indicators: usize,
    pub auto_created_units: usize,
    pub average_confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_integration_service_creation() {
        let tm_service = Arc::new(
            TranslationMemoryAdapter::new(PathBuf::from("/tmp/test"))
                .await
                .unwrap()
        );
        
        let integration_service = TranslationMemoryIntegrationService::new(tm_service)
            .await
            .unwrap();
        
        let config = integration_service.get_config().await;
        assert!(config.auto_suggest_enabled);
        assert_eq!(config.confidence_threshold, 0.7);
    }

    #[tokio::test]
    async fn test_confidence_calculation() {
        let tm_service = Arc::new(
            TranslationMemoryService::new(PathBuf::from("/tmp/test"))
                .await
                .unwrap()
        );
        
        let integration_service = TranslationMemoryIntegrationService::new(tm_service)
            .await
            .unwrap();
        
        let confidence = integration_service.calculate_auto_confidence(
            "Hello world",
            "Hola mundo"
        );
        
        assert!(confidence > 0.0 && confidence <= 1.0);
    }

    #[tokio::test]
    async fn test_config_update() {
        let tm_service = Arc::new(
            TranslationMemoryService::new(PathBuf::from("/tmp/test"))
                .await
                .unwrap()
        );
        
        let integration_service = TranslationMemoryIntegrationService::new(tm_service)
            .await
            .unwrap();
        
        let mut new_config = IntegrationConfig::default();
        new_config.confidence_threshold = 0.8;
        new_config.max_suggestions = 10;
        
        integration_service.update_config(new_config.clone()).await;
        
        let updated_config = integration_service.get_config().await;
        assert_eq!(updated_config.confidence_threshold, 0.8);
        assert_eq!(updated_config.max_suggestions, 10);
    }
}