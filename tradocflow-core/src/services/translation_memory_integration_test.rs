use std::sync::Arc;

use crate::services::{
    TranslationMemoryService, TranslationMemoryIntegrationService
};
use crate::models::translation_models::LanguagePair;

/// Integration tests for translation memory editor integration
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_test_services() -> (Arc<TranslationMemoryService>, Arc<TranslationMemoryIntegrationService>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().to_path_buf();
        
        let tm_service = Arc::new(
            TranslationMemoryService::new(project_path.clone())
                .await
                .unwrap()
        );
        
        let integration_service = Arc::new(
            TranslationMemoryIntegrationService::new(tm_service.clone())
                .await
                .unwrap()
        );
        
        (tm_service, integration_service, temp_dir)
    }

    pub async fn create_sample_translation_units(tm_service: &TranslationMemoryService) -> Vec<TranslationUnit> {
        let project_id = Uuid::new_v4();
        let chapter_id = Uuid::new_v4();
        let chunk_id = Uuid::new_v4();
        
        let units = vec![
            TranslationUnit::new(
                project_id,
                chapter_id,
                chunk_id,
                "en".to_string(),
                "Hello world".to_string(),
                "de".to_string(),
                "Hallo Welt".to_string(),
                0.9,
                Some("Greeting".to_string()),
            ).unwrap(),
            TranslationUnit::new(
                project_id,
                chapter_id,
                chunk_id,
                "en".to_string(),
                "Good morning".to_string(),
                "de".to_string(),
                "Guten Morgen".to_string(),
                0.85,
                Some("Greeting".to_string()),
            ).unwrap(),
            TranslationUnit::new(
                project_id,
                chapter_id,
                chunk_id,
                "en".to_string(),
                "How are you?".to_string(),
                "de".to_string(),
                "Wie geht es dir?".to_string(),
                0.8,
                Some("Question".to_string()),
            ).unwrap(),
        ];
        
        // Add units to translation memory
        for unit in &units {
            tm_service.add_translation_unit(unit.clone()).await.unwrap();
        }
        
        units
    }

    #[tokio::test]
    async fn test_real_time_suggestions() {
        let (_tm_service, integration_service, _temp_dir) = setup_test_services().await;
        
        let language_pair = LanguagePair {
            source: "en".to_string(),
            target: "de".to_string(),
        };
        
        let position = TextPosition {
            start: 0,
            end: 11,
            line: 1,
            column: 1,
        };
        
        // Test with empty text
        let suggestions = integration_service
            .get_real_time_suggestions("", language_pair.clone(), position.clone())
            .await
            .unwrap();
        
        assert!(suggestions.is_empty());
        
        // Test with actual text (no matches expected since we haven't added any units)
        let suggestions = integration_service
            .get_real_time_suggestions("Hello world", language_pair, position)
            .await
            .unwrap();
        
        // Should be empty since no translation units exist yet
        assert!(suggestions.is_empty());
    }

    #[tokio::test]
    async fn test_real_time_suggestions_with_data() {
        let (tm_service, integration_service, _temp_dir) = setup_test_services().await;
        
        // Create sample data
        create_sample_translation_units(&tm_service).await;
        
        let language_pair = LanguagePair {
            source: "en".to_string(),
            target: "de".to_string(),
        };
        
        let position = TextPosition {
            start: 0,
            end: 11,
            line: 1,
            column: 1,
        };
        
        // Test with exact match
        let suggestions = integration_service
            .get_real_time_suggestions("Hello world", language_pair.clone(), position.clone())
            .await
            .unwrap();
        
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].source_text, "Hello world");
        assert_eq!(suggestions[0].suggested_text, "Hallo Welt");
        
        // Test with partial match
        let suggestions = integration_service
            .get_real_time_suggestions("Hello", language_pair, position)
            .await
            .unwrap();
        
        // Should find suggestions based on similarity
        assert!(!suggestions.is_empty());
    }

    #[tokio::test]
    async fn test_apply_suggestion() {
        let (tm_service, integration_service, _temp_dir) = setup_test_services().await;
        
        // Create sample data
        let units = create_sample_translation_units(&tm_service).await;
        let sample_unit = &units[0];
        
        let language_pair = LanguagePair {
            source: "en".to_string(),
            target: "de".to_string(),
        };
        
        let position = TextPosition {
            start: 0,
            end: 11,
            line: 1,
            column: 1,
        };
        
        // Get suggestions
        let suggestions = integration_service
            .get_real_time_suggestions("Hello world", language_pair.clone(), position.clone())
            .await
            .unwrap();
        
        assert!(!suggestions.is_empty());
        
        // Apply the first suggestion
        let suggestion = &suggestions[0];
        let project_id = Uuid::new_v4();
        let chapter_id = Uuid::new_v4();
        let chunk_id = Uuid::new_v4();
        
        let applied_unit = integration_service
            .apply_suggestion(suggestion, project_id, chapter_id, chunk_id, language_pair)
            .await
            .unwrap();
        
        assert_eq!(applied_unit.source_text, suggestion.source_text);
        assert_eq!(applied_unit.target_text, suggestion.suggested_text);
        assert_eq!(applied_unit.confidence_score, suggestion.confidence);
    }

    #[tokio::test]
    async fn test_auto_create_translation_unit() {
        let (_tm_service, integration_service, _temp_dir) = setup_test_services().await;
        
        let language_pair = LanguagePair {
            source: "en".to_string(),
            target: "de".to_string(),
        };
        
        let project_id = Uuid::new_v4();
        let chapter_id = Uuid::new_v4();
        let chunk_id = Uuid::new_v4();
        
        // Test auto-creation with new content
        let result = integration_service
            .auto_create_translation_unit(
                "New text to translate",
                "Neuer Text zum Übersetzen",
                language_pair,
                project_id,
                chapter_id,
                chunk_id,
                Some("Test context".to_string()),
            )
            .await
            .unwrap();
        
        assert!(result.is_some());
        let unit = result.unwrap();
        assert_eq!(unit.source_text, "New text to translate");
        assert_eq!(unit.target_text, "Neuer Text zum Übersetzen");
        assert!(unit.confidence_score > 0.0 && unit.confidence_score <= 1.0);
    }

    #[tokio::test]
    async fn test_search_translation_memory() {
        let (tm_service, integration_service, _temp_dir) = setup_test_services().await;
        
        // Create sample data
        create_sample_translation_units(&tm_service).await;
        
        let language_pair = LanguagePair {
            source: "en".to_string(),
            target: "de".to_string(),
        };
        
        let filters = SearchFilters {
            min_confidence: Some(0.8),
            min_similarity: Some(0.5),
            max_results: Some(10),
            include_context: true,
        };
        
        // Search for existing content
        let results = integration_service
            .search_translation_memory("Hello", language_pair, filters)
            .await
            .unwrap();
        
        assert!(!results.is_empty());
        
        // All results should meet the confidence threshold
        for result in &results {
            assert!(result.confidence_score >= 0.8);
        }
    }

    #[tokio::test]
    async fn test_confidence_indicators() {
        let (_tm_service, integration_service, _temp_dir) = setup_test_services().await;
        
        let position = TextPosition {
            start: 0,
            end: 11,
            line: 1,
            column: 1,
        };
        
        let text = "Test text";
        
        // Add confidence indicator
        integration_service
            .update_confidence_indicator(
                text,
                position.clone(),
                0.9,
                Some(0.85),
                IndicatorType::High,
            )
            .await;
        
        // Retrieve indicators
        let indicators = integration_service
            .get_confidence_indicators(text)
            .await;
        
        assert_eq!(indicators.len(), 1);
        assert_eq!(indicators[0].confidence, 0.9);
        assert_eq!(indicators[0].quality_score, Some(0.85));
        assert_eq!(indicators[0].indicator_type, IndicatorType::High);
    }

    #[tokio::test]
    async fn test_config_management() {
        let (_tm_service, integration_service, _temp_dir) = setup_test_services().await;
        
        // Get default config
        let default_config = integration_service.get_config().await;
        assert!(default_config.auto_suggest_enabled);
        assert_eq!(default_config.confidence_threshold, 0.7);
        assert_eq!(default_config.max_suggestions, 5);
        
        // Update config
        let mut new_config = default_config.clone();
        new_config.confidence_threshold = 0.8;
        new_config.max_suggestions = 10;
        new_config.auto_suggest_enabled = false;
        
        integration_service.update_config(new_config.clone()).await;
        
        // Verify config was updated
        let updated_config = integration_service.get_config().await;
        assert!(!updated_config.auto_suggest_enabled);
        assert_eq!(updated_config.confidence_threshold, 0.8);
        assert_eq!(updated_config.max_suggestions, 10);
    }

    #[tokio::test]
    async fn test_suggestions_caching() {
        let (tm_service, integration_service, _temp_dir) = setup_test_services().await;
        
        // Create sample data
        create_sample_translation_units(&tm_service).await;
        
        let language_pair = LanguagePair {
            source: "en".to_string(),
            target: "de".to_string(),
        };
        
        let position = TextPosition {
            start: 0,
            end: 11,
            line: 1,
            column: 1,
        };
        
        let text = "Hello world";
        
        // First request - should populate cache
        let suggestions1 = integration_service
            .get_real_time_suggestions(text, language_pair.clone(), position.clone())
            .await
            .unwrap();
        
        // Check cache
        let cached = integration_service
            .get_cached_suggestions(text, &language_pair)
            .await;
        
        assert!(cached.is_some());
        let cached_suggestions = cached.unwrap();
        assert_eq!(cached_suggestions.len(), suggestions1.len());
        
        // Clear cache
        integration_service.clear_suggestions_cache().await;
        
        let cached_after_clear = integration_service
            .get_cached_suggestions(text, &language_pair)
            .await;
        
        assert!(cached_after_clear.is_none());
    }

    #[tokio::test]
    async fn test_translation_statistics() {
        let (_tm_service, integration_service, _temp_dir) = setup_test_services().await;
        
        // Get initial statistics
        let stats = integration_service
            .get_translation_statistics()
            .await
            .unwrap();
        
        assert_eq!(stats.cached_suggestions, 0);
        assert_eq!(stats.active_indicators, 0);
        
        // Add some data and check statistics update
        let position = TextPosition {
            start: 0,
            end: 11,
            line: 1,
            column: 1,
        };
        
        integration_service
            .update_confidence_indicator(
                "test text",
                position,
                0.9,
                None,
                IndicatorType::High,
            )
            .await;
        
        let updated_stats = integration_service
            .get_translation_statistics()
            .await
            .unwrap();
        
        assert_eq!(updated_stats.active_indicators, 1);
    }

    #[tokio::test]
    async fn test_confidence_calculation() {
        let (_tm_service, integration_service, _temp_dir) = setup_test_services().await;
        
        // Test confidence calculation with various text pairs
        let test_cases = vec![
            ("Hello", "Hallo", 0.1, 0.9), // Similar length, should have decent confidence
            ("Hello world", "Hallo Welt", 0.1, 0.9), // Good match
            ("A", "Ein sehr langer deutscher Satz", 0.1, 0.9), // Very different lengths
            ("", "", 0.1, 0.9), // Empty strings
        ];
        
        for (source, target, min_expected, max_expected) in test_cases {
            let confidence = integration_service.calculate_auto_confidence(source, target);
            assert!(
                confidence >= min_expected && confidence <= max_expected,
                "Confidence {} for '{}' -> '{}' not in range [{}, {}]",
                confidence, source, target, min_expected, max_expected
            );
        }
    }
}

/// Performance tests for translation memory integration
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_suggestion_performance() {
        let (tm_service, integration_service, _temp_dir) = setup_test_services().await;
        
        // Create a large number of translation units
        let project_id = Uuid::new_v4();
        let chapter_id = Uuid::new_v4();
        let chunk_id = Uuid::new_v4();
        
        let mut units = Vec::new();
        for i in 0..1000 {
            let unit = TranslationUnit::new(
                project_id,
                chapter_id,
                chunk_id,
                "en".to_string(),
                format!("Test sentence number {}", i),
                "de".to_string(),
                format!("Testsatz Nummer {}", i),
                0.8,
                Some(format!("Context {}", i)),
            ).unwrap();
            units.push(unit);
        }
        
        // Batch add units
        tm_service.add_translation_units_batch(units).await.unwrap();
        
        let language_pair = LanguagePair {
            source: "en".to_string(),
            target: "de".to_string(),
        };
        
        let position = TextPosition {
            start: 0,
            end: 20,
            line: 1,
            column: 1,
        };
        
        // Measure suggestion retrieval time
        let start = Instant::now();
        let suggestions = integration_service
            .get_real_time_suggestions("Test sentence", language_pair, position)
            .await
            .unwrap();
        let duration = start.elapsed();
        
        println!("Retrieved {} suggestions in {:?}", suggestions.len(), duration);
        
        // Should complete within reasonable time (adjust threshold as needed)
        assert!(duration.as_millis() < 1000, "Suggestion retrieval took too long: {:?}", duration);
        assert!(!suggestions.is_empty(), "Should find suggestions");
    }

    #[tokio::test]
    async fn test_search_performance() {
        let (tm_service, integration_service, _temp_dir) = setup_test_services().await;
        
        // Create sample data
        create_sample_translation_units(&tm_service).await;
        
        let language_pair = LanguagePair {
            source: "en".to_string(),
            target: "de".to_string(),
        };
        
        let filters = SearchFilters::default();
        
        // Measure search time
        let start = Instant::now();
        let results = integration_service
            .search_translation_memory("Hello", language_pair, filters)
            .await
            .unwrap();
        let duration = start.elapsed();
        
        println!("Search returned {} results in {:?}", results.len(), duration);
        
        // Should complete quickly
        assert!(duration.as_millis() < 500, "Search took too long: {:?}", duration);
    }
}

/// Helper function to setup test services (used by other test modules)
pub async fn setup_test_services() -> (Arc<TranslationMemoryService>, Arc<TranslationMemoryIntegrationService>, tempfile::TempDir) {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let project_path = temp_dir.path().to_path_buf();
    
    let tm_service = Arc::new(
        TranslationMemoryService::new(project_path.clone())
            .await
            .unwrap()
    );
    
    let integration_service = Arc::new(
        TranslationMemoryIntegrationService::new(tm_service.clone())
            .await
            .unwrap()
    );
    
    (tm_service, integration_service, temp_dir)
}