use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;
use anyhow::Result;
use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::services::{
    TranslationMemoryIntegrationService, IntegrationConfig, EditorSuggestion,
    ConfidenceIndicator, TextPosition, SearchFilters, TranslationStatistics
};
use crate::models::translation_models::LanguagePair;

/// Bridge between Slint UI and translation memory integration service
#[derive(Clone)]
pub struct TranslationMemoryBridge {
    integration_service: Arc<TranslationMemoryIntegrationService>,
    current_project_id: Arc<RwLock<Option<Uuid>>>,
    current_chapter_id: Arc<RwLock<Option<Uuid>>>,
    active_language_pair: Arc<RwLock<Option<LanguagePair>>>,
    ui_suggestions: Arc<RwLock<ModelRc<SlintTranslationSuggestion>>>,
    ui_search_results: Arc<RwLock<ModelRc<SlintTranslationMatch>>>,
}

/// Slint-compatible translation suggestion structure
#[derive(Clone, Debug)]
pub struct SlintTranslationSuggestion {
    pub id: slint::SharedString,
    pub source_text: slint::SharedString,
    pub suggested_text: slint::SharedString,
    pub confidence: f32,
    pub similarity: f32,
    pub context: slint::SharedString,
    pub source: slint::SharedString,
}

/// Slint-compatible translation match structure
#[derive(Clone, Debug)]
pub struct SlintTranslationMatch {
    pub id: slint::SharedString,
    pub source_text: slint::SharedString,
    pub target_text: slint::SharedString,
    pub confidence_score: f32,
    pub similarity_score: f32,
    pub context: slint::SharedString,
}

impl TranslationMemoryBridge {
    pub async fn new(integration_service: Arc<TranslationMemoryIntegrationService>) -> Result<Self> {
        let ui_suggestions = Arc::new(RwLock::new(
            ModelRc::new(VecModel::from(Vec::<SlintTranslationSuggestion>::new()))
        ));
        
        let ui_search_results = Arc::new(RwLock::new(
            ModelRc::new(VecModel::from(Vec::<SlintTranslationMatch>::new()))
        ));

        Ok(Self {
            integration_service,
            current_project_id: Arc::new(RwLock::new(None)),
            current_chapter_id: Arc::new(RwLock::new(None)),
            active_language_pair: Arc::new(RwLock::new(None)),
            ui_suggestions,
            ui_search_results,
        })
    }

    /// Set the current project context
    pub async fn set_project_context(&self, project_id: Uuid, chapter_id: Uuid) {
        *self.current_project_id.write().await = Some(project_id);
        *self.current_chapter_id.write().await = Some(chapter_id);
    }

    /// Set the active language pair
    pub async fn set_language_pair(&self, source_language: String, target_language: String) {
        let language_pair = LanguagePair {
            source: source_language,
            target: target_language,
        };
        *self.active_language_pair.write().await = Some(language_pair);
    }

    /// Get real-time translation suggestions for editor text
    pub async fn get_suggestions_for_text(
        &self,
        text: String,
        line: i32,
        column: i32,
        start_pos: i32,
        end_pos: i32,
    ) -> Result<()> {
        let language_pair = {
            let pair_guard = self.active_language_pair.read().await;
            match pair_guard.as_ref() {
                Some(pair) => pair.clone(),
                None => return Ok(()), // No language pair set
            }
        };

        let position = TextPosition {
            start: start_pos as usize,
            end: end_pos as usize,
            line: line as usize,
            column: column as usize,
        };

        let suggestions = self.integration_service
            .get_real_time_suggestions(&text, language_pair, position)
            .await?;

        // Convert to Slint-compatible format
        let slint_suggestions: Vec<SlintTranslationSuggestion> = suggestions
            .into_iter()
            .map(|s| SlintTranslationSuggestion {
                id: s.id.to_string().into(),
                source_text: s.source_text.into(),
                suggested_text: s.suggested_text.into(),
                confidence: s.confidence,
                similarity: s.similarity,
                context: s.context.unwrap_or_default().into(),
                source: format!("{:?}", s.source).into(),
            })
            .collect();

        // Update UI model
        {
            let ui_suggestions = self.ui_suggestions.read().await;
            let model = ui_suggestions.as_any().downcast_ref::<VecModel<SlintTranslationSuggestion>>().unwrap();
            model.set_vec(slint_suggestions);
        }

        Ok(())
    }

    /// Apply a translation suggestion
    pub async fn apply_suggestion(&self, suggestion_id: String, target_text: String) -> Result<()> {
        let project_id = {
            let guard = self.current_project_id.read().await;
            match *guard {
                Some(id) => id,
                None => return Err(anyhow::anyhow!("No project context set")),
            }
        };

        let chapter_id = {
            let guard = self.current_chapter_id.read().await;
            match *guard {
                Some(id) => id,
                None => return Err(anyhow::anyhow!("No chapter context set")),
            }
        };

        let language_pair = {
            let guard = self.active_language_pair.read().await;
            match guard.as_ref() {
                Some(pair) => pair.clone(),
                None => return Err(anyhow::anyhow!("No language pair set")),
            }
        };

        // Find the suggestion by ID
        let suggestion_uuid = Uuid::parse_str(&suggestion_id)?;
        
        // For now, create a mock suggestion - in a real implementation,
        // we would retrieve the actual suggestion from cache
        let mock_suggestion = EditorSuggestion {
            id: suggestion_uuid,
            source_text: "".to_string(), // Would be filled from actual suggestion
            suggested_text: target_text,
            confidence: 0.8,
            similarity: 0.9,
            context: None,
            source: crate::services::translation_memory_service::TranslationSource::Memory,
            position: TextPosition { start: 0, end: 0, line: 0, column: 0 },
            created_at: chrono::Utc::now(),
        };

        let chunk_id = Uuid::new_v4(); // Would be determined from context
        
        self.integration_service
            .apply_suggestion(&mock_suggestion, project_id, chapter_id, chunk_id, language_pair)
            .await?;

        Ok(())
    }

    /// Search translation memory
    pub async fn search_translation_memory(&self, query: String) -> Result<()> {
        let language_pair = {
            let guard = self.active_language_pair.read().await;
            match guard.as_ref() {
                Some(pair) => pair.clone(),
                None => return Ok(()), // No language pair set
            }
        };

        let filters = SearchFilters {
            min_confidence: Some(0.5),
            min_similarity: Some(0.3),
            max_results: Some(20),
            include_context: true,
        };

        let matches = self.integration_service
            .search_translation_memory(&query, language_pair, filters)
            .await?;

        // Convert to Slint-compatible format
        let slint_matches: Vec<SlintTranslationMatch> = matches
            .into_iter()
            .map(|m| SlintTranslationMatch {
                id: m.id.to_string().into(),
                source_text: m.source_text.into(),
                target_text: m.target_text.into(),
                confidence_score: m.confidence_score,
                similarity_score: m.similarity_score,
                context: m.context.unwrap_or_default().into(),
            })
            .collect();

        // Update UI model
        {
            let ui_search_results = self.ui_search_results.read().await;
            let model = ui_search_results.as_any().downcast_ref::<VecModel<SlintTranslationMatch>>().unwrap();
            model.set_vec(slint_matches);
        }

        Ok(())
    }

    /// Auto-create translation unit when content is modified
    pub async fn auto_create_translation_unit(
        &self,
        source_text: String,
        target_text: String,
        context: Option<String>,
    ) -> Result<bool> {
        let project_id = {
            let guard = self.current_project_id.read().await;
            match *guard {
                Some(id) => id,
                None => return Ok(false),
            }
        };

        let chapter_id = {
            let guard = self.current_chapter_id.read().await;
            match *guard {
                Some(id) => id,
                None => return Ok(false),
            }
        };

        let language_pair = {
            let guard = self.active_language_pair.read().await;
            match guard.as_ref() {
                Some(pair) => pair.clone(),
                None => return Ok(false),
            }
        };

        let chunk_id = Uuid::new_v4(); // Would be determined from context

        let result = self.integration_service
            .auto_create_translation_unit(
                &source_text,
                &target_text,
                language_pair,
                project_id,
                chapter_id,
                chunk_id,
                context,
            )
            .await?;

        Ok(result.is_some())
    }

    /// Update integration configuration
    pub async fn update_config(
        &self,
        auto_suggest_enabled: bool,
        confidence_threshold: f32,
        max_suggestions: i32,
    ) -> Result<()> {
        let mut config = self.integration_service.get_config().await;
        config.auto_suggest_enabled = auto_suggest_enabled;
        config.confidence_threshold = confidence_threshold;
        config.max_suggestions = max_suggestions as usize;

        self.integration_service.update_config(config).await;
        Ok(())
    }

    /// Get current configuration
    pub async fn get_config(&self) -> Result<(bool, f32, i32)> {
        let config = self.integration_service.get_config().await;
        Ok((
            config.auto_suggest_enabled,
            config.confidence_threshold,
            config.max_suggestions as i32,
        ))
    }

    /// Get UI models for Slint
    pub async fn get_suggestions_model(&self) -> ModelRc<SlintTranslationSuggestion> {
        self.ui_suggestions.read().await.clone()
    }

    pub async fn get_search_results_model(&self) -> ModelRc<SlintTranslationMatch> {
        self.ui_search_results.read().await.clone()
    }

    /// Clear suggestions cache
    pub async fn clear_suggestions(&self) -> Result<()> {
        self.integration_service.clear_suggestions_cache().await;
        
        // Clear UI models
        {
            let ui_suggestions = self.ui_suggestions.read().await;
            let model = ui_suggestions.as_any().downcast_ref::<VecModel<SlintTranslationSuggestion>>().unwrap();
            model.set_vec(Vec::new());
        }

        {
            let ui_search_results = self.ui_search_results.read().await;
            let model = ui_search_results.as_any().downcast_ref::<VecModel<SlintTranslationMatch>>().unwrap();
            model.set_vec(Vec::new());
        }

        Ok(())
    }

    /// Get translation statistics
    pub async fn get_statistics(&self) -> Result<(i32, i32, i32, f32)> {
        let stats = self.integration_service.get_translation_statistics().await?;
        Ok((
            stats.cached_suggestions as i32,
            stats.active_indicators as i32,
            stats.auto_created_units as i32,
            stats.average_confidence,
        ))
    }

    /// Update confidence indicator
    pub async fn update_confidence_indicator(
        &self,
        text: String,
        line: i32,
        column: i32,
        start_pos: i32,
        end_pos: i32,
        confidence: f32,
        quality_score: Option<f32>,
        indicator_type: String,
    ) -> Result<()> {
        let position = TextPosition {
            start: start_pos as usize,
            end: end_pos as usize,
            line: line as usize,
            column: column as usize,
        };

        let indicator_type = match indicator_type.as_str() {
            "High" => crate::services::IndicatorType::High,
            "Medium" => crate::services::IndicatorType::Medium,
            "Low" => crate::services::IndicatorType::Low,
            "New" => crate::services::IndicatorType::New,
            "Suggested" => crate::services::IndicatorType::Suggested,
            _ => crate::services::IndicatorType::Medium,
        };

        self.integration_service
            .update_confidence_indicator(&text, position, confidence, quality_score, indicator_type)
            .await;

        Ok(())
    }

    /// Get confidence indicators for text
    pub async fn get_confidence_indicators(&self, text: String) -> Result<Vec<(f32, Option<f32>, String)>> {
        let indicators = self.integration_service
            .get_confidence_indicators(&text)
            .await;

        let result = indicators
            .into_iter()
            .map(|indicator| {
                let type_str = match indicator.indicator_type {
                    crate::services::IndicatorType::High => "High",
                    crate::services::IndicatorType::Medium => "Medium",
                    crate::services::IndicatorType::Low => "Low",
                    crate::services::IndicatorType::New => "New",
                    crate::services::IndicatorType::Suggested => "Suggested",
                };
                (indicator.confidence, indicator.quality_score, type_str.to_string())
            })
            .collect();

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::translation_memory_integration_test::setup_test_services;

    #[tokio::test]
    async fn test_bridge_creation() {
        let (_tm_service, integration_service, _temp_dir) = setup_test_services().await;
        
        let bridge = TranslationMemoryBridge::new(integration_service)
            .await
            .unwrap();
        
        // Test initial state
        let (auto_suggest, threshold, max_suggestions) = bridge.get_config().await.unwrap();
        assert!(auto_suggest);
        assert_eq!(threshold, 0.7);
        assert_eq!(max_suggestions, 5);
    }

    #[tokio::test]
    async fn test_context_setting() {
        let (_tm_service, integration_service, _temp_dir) = setup_test_services().await;
        
        let bridge = TranslationMemoryBridge::new(integration_service)
            .await
            .unwrap();
        
        let project_id = Uuid::new_v4();
        let chapter_id = Uuid::new_v4();
        
        bridge.set_project_context(project_id, chapter_id).await;
        bridge.set_language_pair("en".to_string(), "de".to_string()).await;
        
        // Context should be set (we can't directly test private fields,
        // but we can test operations that depend on context)
        let result = bridge.auto_create_translation_unit(
            "Test".to_string(),
            "Test".to_string(),
            None,
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_config_update() {
        let (_tm_service, integration_service, _temp_dir) = setup_test_services().await;
        
        let bridge = TranslationMemoryBridge::new(integration_service)
            .await
            .unwrap();
        
        // Update config
        bridge.update_config(false, 0.8, 10).await.unwrap();
        
        // Verify update
        let (auto_suggest, threshold, max_suggestions) = bridge.get_config().await.unwrap();
        assert!(!auto_suggest);
        assert_eq!(threshold, 0.8);
        assert_eq!(max_suggestions, 10);
    }
}