use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use uuid::Uuid;
use anyhow::Result;
use slint::{ModelRc, VecModel, SharedString};

use crate::services::{
    TerminologyHighlightingService, TerminologyService, 
    TermHighlight, HighlightType, TerminologySuggestion
};

// Slint-compatible structures
#[derive(Debug, Clone)]
pub struct SlintTermHighlight {
    pub term_id: SharedString,
    pub term: SharedString,
    pub start_position: i32,
    pub end_position: i32,
    pub highlight_type: SharedString,
    pub definition: SharedString,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct SlintTermSuggestion {
    pub original_text: SharedString,
    pub suggested_term: SharedString,
    pub definition: SharedString,
    pub confidence: f32,
    pub position: i32,
    pub reason: SharedString,
}

#[derive(Debug, Clone)]
pub struct SlintConsistencyResult {
    pub term: SharedString,
    pub language: SharedString,
    pub expected_term: SharedString,
    pub found_terms: ModelRc<SharedString>,
    pub positions: ModelRc<i32>,
}

/// Bridge between Slint UI and Rust terminology services
pub struct TerminologyBridge {
    highlighting_service: Arc<TerminologyHighlightingService>,
    current_project_id: Arc<Mutex<Option<Uuid>>>,
    current_language: Arc<Mutex<String>>,
    highlights_cache: Arc<Mutex<HashMap<String, Vec<TermHighlight>>>>,
}

impl TerminologyBridge {
    pub fn new(terminology_service: Arc<TerminologyService>) -> Self {
        let highlighting_service = Arc::new(
            TerminologyHighlightingService::new(terminology_service)
        );
        
        Self {
            highlighting_service,
            current_project_id: Arc::new(Mutex::new(None)),
            current_language: Arc::new(Mutex::new("en".to_string())),
            highlights_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Set the current project context
    pub fn set_project(&self, project_id: Uuid) {
        let mut current = self.current_project_id.lock().unwrap();
        *current = Some(project_id);
        
        // Clear cache when project changes
        self.highlights_cache.lock().unwrap().clear();
    }
    
    /// Set the current language context
    pub fn set_language(&self, language: String) {
        let mut current = self.current_language.lock().unwrap();
        *current = language;
        
        // Clear cache when language changes
        self.highlights_cache.lock().unwrap().clear();
    }
    
    /// Analyze text and return highlights for Slint UI
    pub async fn analyze_text_for_highlights(
        &self,
        text: String,
    ) -> Result<ModelRc<SlintTermHighlight>> {
        let project_id = {
            let current = self.current_project_id.lock().unwrap();
            match *current {
                Some(id) => id,
                None => return Ok(ModelRc::new(VecModel::from(vec![]))),
            }
        };
        
        let language = {
            let current = self.current_language.lock().unwrap();
            current.clone()
        };
        
        // Check cache first
        let cache_key = format!("{}:{}:{}", project_id, language, text.len());
        {
            let cache = self.highlights_cache.lock().unwrap();
            if let Some(cached_highlights) = cache.get(&cache_key) {
                let slint_highlights: Vec<SlintTermHighlight> = cached_highlights
                    .iter()
                    .map(|h| self.convert_highlight_to_slint(h))
                    .collect();
                return Ok(ModelRc::new(VecModel::from(slint_highlights)));
            }
        }
        
        // Analyze text
        let highlights = self.highlighting_service
            .highlight_terms_in_text(&text, project_id, &language)
            .await?;
        
        // Cache results
        {
            let mut cache = self.highlights_cache.lock().unwrap();
            cache.insert(cache_key, highlights.clone());
        }
        
        // Convert to Slint format
        let slint_highlights: Vec<SlintTermHighlight> = highlights
            .iter()
            .map(|h| self.convert_highlight_to_slint(h))
            .collect();
        
        Ok(ModelRc::new(VecModel::from(slint_highlights)))
    }
    
    /// Generate terminology suggestions for text
    pub async fn generate_suggestions(
        &self,
        text: String,
    ) -> Result<ModelRc<SlintTermSuggestion>> {
        let project_id = {
            let current = self.current_project_id.lock().unwrap();
            match *current {
                Some(id) => id,
                None => return Ok(ModelRc::new(VecModel::from(vec![]))),
            }
        };
        
        let language = {
            let current = self.current_language.lock().unwrap();
            current.clone()
        };
        
        let suggestions = self.highlighting_service
            .generate_terminology_suggestions(&text, project_id, &language)
            .await?;
        
        let slint_suggestions: Vec<SlintTermSuggestion> = suggestions
            .iter()
            .map(|s| self.convert_suggestion_to_slint(s))
            .collect();
        
        Ok(ModelRc::new(VecModel::from(slint_suggestions)))
    }
    
    /// Check consistency across multiple language texts
    pub async fn check_consistency(
        &self,
        texts: HashMap<String, String>,
    ) -> Result<ModelRc<SlintConsistencyResult>> {
        let project_id = {
            let current = self.current_project_id.lock().unwrap();
            match *current {
                Some(id) => id,
                None => return Ok(ModelRc::new(VecModel::from(vec![]))),
            }
        };
        
        let consistency_results = self.highlighting_service
            .check_consistency_across_languages(texts, project_id)
            .await?;
        
        let slint_results: Vec<SlintConsistencyResult> = consistency_results
            .iter()
            .flat_map(|result| {
                result.inconsistencies.iter().map(|inconsistency| {
                    SlintConsistencyResult {
                        term: result.term.clone().into(),
                        language: inconsistency.language.clone().into(),
                        expected_term: inconsistency.expected_term.clone().into(),
                        found_terms: ModelRc::new(VecModel::from(
                            inconsistency.found_terms.iter()
                                .map(|s| SharedString::from(s.clone()))
                                .collect::<Vec<SharedString>>()
                        )),
                        positions: ModelRc::new(VecModel::from(
                            inconsistency.positions.iter()
                                .map(|&pos| pos as i32)
                                .collect::<Vec<i32>>()
                        )),
                    }
                })
            })
            .collect();
        
        Ok(ModelRc::new(VecModel::from(slint_results)))
    }
    
    /// Update highlighting for real-time text changes
    pub async fn update_highlighting_for_change(
        &self,
        text: String,
        change_start: usize,
        change_end: usize,
    ) -> Result<ModelRc<SlintTermHighlight>> {
        let project_id = {
            let current = self.current_project_id.lock().unwrap();
            match *current {
                Some(id) => id,
                None => return Ok(ModelRc::new(VecModel::from(vec![]))),
            }
        };
        
        let language = {
            let current = self.current_language.lock().unwrap();
            current.clone()
        };
        
        let highlights = self.highlighting_service
            .update_highlighting_for_text_change(&text, change_start, change_end, project_id, &language)
            .await?;
        
        let slint_highlights: Vec<SlintTermHighlight> = highlights
            .iter()
            .map(|h| self.convert_highlight_to_slint(h))
            .collect();
        
        Ok(ModelRc::new(VecModel::from(slint_highlights)))
    }
    
    /// Apply a terminology suggestion to text
    pub fn apply_suggestion(
        &self,
        text: &str,
        suggestion: &SlintTermSuggestion,
    ) -> String {
        let position = suggestion.position as usize;
        let original_len = suggestion.original_text.len();
        
        if position + original_len <= text.len() {
            let mut result = text.to_string();
            result.replace_range(position..position + original_len, &suggestion.suggested_term);
            result
        } else {
            text.to_string()
        }
    }
    
    /// Invalidate cache for a project
    pub fn invalidate_cache(&self, project_id: Uuid) {
        self.highlighting_service.invalidate_cache(project_id);
        
        let mut cache = self.highlights_cache.lock().unwrap();
        cache.retain(|key, _| !key.starts_with(&project_id.to_string()));
    }
    
    /// Convert internal TermHighlight to Slint format
    fn convert_highlight_to_slint(&self, highlight: &TermHighlight) -> SlintTermHighlight {
        SlintTermHighlight {
            term_id: SharedString::from(highlight.term_id.to_string()),
            term: SharedString::from(highlight.term.clone()),
            start_position: highlight.start_position as i32,
            end_position: highlight.end_position as i32,
            highlight_type: SharedString::from(match highlight.highlight_type {
                HighlightType::DoNotTranslate => "do_not_translate",
                HighlightType::Inconsistent => "inconsistent",
                HighlightType::Suggestion => "suggestion",
                HighlightType::Validated => "validated",
            }),
            definition: SharedString::from(highlight.definition.clone().unwrap_or_default()),
            confidence: highlight.confidence,
        }
    }
    
    /// Convert internal TerminologySuggestion to Slint format
    fn convert_suggestion_to_slint(&self, suggestion: &TerminologySuggestion) -> SlintTermSuggestion {
        SlintTermSuggestion {
            original_text: SharedString::from(suggestion.original_text.clone()),
            suggested_term: SharedString::from(suggestion.suggested_term.clone()),
            definition: SharedString::from(suggestion.definition.clone().unwrap_or_default()),
            confidence: suggestion.confidence,
            position: suggestion.position as i32,
            reason: SharedString::from(suggestion.reason.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_terminology_bridge_creation() {
        let temp_dir = TempDir::new().unwrap();
        let terminology_service = Arc::new(
            TerminologyService::new(temp_dir.path().to_path_buf()).unwrap()
        );
        let bridge = TerminologyBridge::new(terminology_service);
        
        // Test that bridge is created successfully
        assert!(bridge.current_project_id.lock().unwrap().is_none());
        assert_eq!(*bridge.current_language.lock().unwrap(), "en");
    }

    #[tokio::test]
    async fn test_set_project_and_language() {
        let temp_dir = TempDir::new().unwrap();
        let terminology_service = Arc::new(
            TerminologyService::new(temp_dir.path().to_path_buf()).unwrap()
        );
        let bridge = TerminologyBridge::new(terminology_service);
        
        let project_id = Uuid::new_v4();
        bridge.set_project(project_id);
        bridge.set_language("de".to_string());
        
        assert_eq!(*bridge.current_project_id.lock().unwrap(), Some(project_id));
        assert_eq!(*bridge.current_language.lock().unwrap(), "de");
    }

    #[test]
    fn test_apply_suggestion() {
        let temp_dir = TempDir::new().unwrap();
        let terminology_service = Arc::new(
            TerminologyService::new(temp_dir.path().to_path_buf()).unwrap()
        );
        let bridge = TerminologyBridge::new(terminology_service);
        
        let suggestion = SlintTermSuggestion {
            original_text: SharedString::from("color"),
            suggested_term: SharedString::from("colour"),
            definition: SharedString::from(""),
            confidence: 0.9,
            position: 5,
            reason: SharedString::from("British spelling"),
        };
        
        let text = "The color is red";
        let result = bridge.apply_suggestion(text, &suggestion);
        assert_eq!(result, "The colour is red");
    }
}