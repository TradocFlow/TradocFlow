use std::collections::HashMap;
use uuid::Uuid;
use crate::Result;
use crate::services::{
    sentence_alignment_service::{SentenceAlignmentService, AlignmentConfig, LanguageProfile},
    text_structure_analyzer::{TextStructureAnalyzer, StructureAnalysisConfig},
    alignment_cache_service::{AlignmentCacheService, AlignmentCacheConfig},
    multi_pane_alignment_service::{MultiPaneAlignmentService, MultiPaneAlignmentConfig},
    alignment_api_service::{
        AlignmentApiService, AlignmentApiConfig, AddPaneRequest, UpdatePaneRequest,
        SyncCursorRequest, UserCorrectionRequest
    }
};

/// Comprehensive integration tests for the sentence alignment system
pub struct AlignmentIntegrationTests;

impl AlignmentIntegrationTests {
    /// Test the complete sentence alignment pipeline
    pub async fn test_complete_alignment_pipeline() -> Result<()> {
        println!("Testing complete sentence alignment pipeline...");

        // Create sentence alignment service
        let alignment_service = SentenceAlignmentService::new(AlignmentConfig::default());

        // Test data
        let english_text = "Hello world! How are you today? I hope you're doing well. This is a test of the sentence alignment system.";
        let spanish_text = "¡Hola mundo! ¿Cómo estás hoy? Espero que estés bien. Esta es una prueba del sistema de alineación de oraciones.";

        // Test sentence boundary detection
        let english_boundaries = alignment_service.detect_sentence_boundaries(english_text, "en").await?;
        let spanish_boundaries = alignment_service.detect_sentence_boundaries(spanish_text, "es").await?;

        assert!(!english_boundaries.is_empty(), "English boundaries should not be empty");
        assert!(!spanish_boundaries.is_empty(), "Spanish boundaries should not be empty");
        assert_eq!(english_boundaries.len(), 4, "Should detect 4 English sentences");
        assert_eq!(spanish_boundaries.len(), 4, "Should detect 4 Spanish sentences");

        println!("✓ Sentence boundary detection working correctly");

        // Test sentence alignment
        let alignments = alignment_service.align_sentences(
            english_text,
            spanish_text,
            "en",
            "es"
        ).await?;

        assert!(!alignments.is_empty(), "Alignments should not be empty");
        assert_eq!(alignments.len(), 4, "Should have 4 sentence alignments");

        // Verify alignment quality
        for alignment in &alignments {
            assert!(alignment.alignment_confidence > 0.0, "Alignment confidence should be positive");
            assert!(alignment.alignment_confidence <= 1.0, "Alignment confidence should be <= 1.0");
        }

        println!("✓ Sentence alignment working correctly");

        // Test quality indicators
        let quality_indicators = alignment_service.calculate_quality_indicators(&alignments).await?;
        assert!(quality_indicators.overall_quality >= 0.0, "Overall quality should be non-negative");
        assert!(quality_indicators.overall_quality <= 1.0, "Overall quality should be <= 1.0");

        println!("✓ Quality indicators calculation working correctly");

        // Test synchronization
        let mut pane_contents = HashMap::new();
        pane_contents.insert("en".to_string(), english_text.to_string());
        pane_contents.insert("es".to_string(), spanish_text.to_string());

        let sync_positions = alignment_service.synchronize_sentence_boundaries(
            &pane_contents,
            50, // Cursor position in middle of text
            "en"
        ).await?;

        assert!(sync_positions.contains_key("en"), "Should have English sync position");
        assert!(sync_positions.contains_key("es"), "Should have Spanish sync position");

        println!("✓ Sentence synchronization working correctly");
        Ok(())
    }

    /// Test text structure analysis
    pub async fn test_text_structure_analysis() -> Result<()> {
        println!("Testing text structure analysis...");

        let config = StructureAnalysisConfig::default();
        let analyzer = TextStructureAnalyzer::new(config)?;

        let test_text = r#"# Main Heading

This is a paragraph with some content.

## Sub Heading

- First list item
- Second list item
- Third list item

```rust
let x = 5;
println!("Hello: {}", x);
```

Another paragraph with different content.

| Column 1 | Column 2 |
|----------|----------|
| Data 1   | Data 2   |

> This is a blockquote
> with multiple lines.

1. Ordered list item
2. Another ordered item

Final paragraph."#;

        let analysis_result = analyzer.analyze_structure(test_text, Some("en")).await?;

        // Verify different structure types were detected
        let structure_types: Vec<String> = analysis_result.structures.iter()
            .map(|s| format!("{:?}", s.structure_type))
            .collect();

        assert!(structure_types.iter().any(|t| t.contains("Heading")), "Should detect headings");
        assert!(structure_types.iter().any(|t| t.contains("Paragraph")), "Should detect paragraphs");
        assert!(structure_types.iter().any(|t| t.contains("List")), "Should detect lists");
        assert!(structure_types.iter().any(|t| t.contains("CodeBlock")), "Should detect code blocks");
        assert!(structure_types.iter().any(|t| t.contains("Table")), "Should detect tables");
        assert!(structure_types.iter().any(|t| t.contains("Quote")), "Should detect quotes");

        // Verify statistics
        assert!(analysis_result.statistics.total_elements > 0, "Should have detected elements");
        assert!(analysis_result.statistics.word_count > 0, "Should count words");
        assert!(analysis_result.statistics.character_count > 0, "Should count characters");

        // Verify language features
        assert_eq!(analysis_result.language_specific_features.detected_language, "en");
        assert!(analysis_result.language_specific_features.confidence > 0.0);

        println!("✓ Text structure analysis working correctly");
        Ok(())
    }

    /// Test caching system
    pub async fn test_caching_system() -> Result<()> {
        println!("Testing caching system...");

        let config = AlignmentCacheConfig::default();
        let cache_service = AlignmentCacheService::new(config);

        // Generate test data
        let alignments = vec![
            Self::create_test_alignment("Hello world.", "Hola mundo.", "en", "es"),
            Self::create_test_alignment("How are you?", "¿Cómo estás?", "en", "es"),
        ];

        let quality_indicator = crate::services::sentence_alignment_service::AlignmentQualityIndicator {
            overall_quality: 0.8,
            position_consistency: 0.9,
            length_ratio_consistency: 0.7,
            structural_coherence: 0.8,
            user_validation_rate: 0.0,
            problem_areas: Vec::new(),
        };

        let statistics = crate::services::sentence_alignment_service::AlignmentStatistics {
            total_sentences: 2,
            aligned_sentences: 2,
            validated_alignments: 0,
            average_confidence: 0.8,
            alignment_accuracy: 0.8,
            processing_time_ms: 50,
            language_pair: ("en".to_string(), "es".to_string()),
        };

        let cache_key = "test_cache_key".to_string();

        // Test cache storage
        cache_service.store_alignments(
            cache_key.clone(),
            alignments.clone(),
            quality_indicator.clone(),
            statistics.clone(),
        ).await?;

        // Test cache retrieval
        let cached_result = cache_service.get_alignments(&cache_key).await?;
        assert!(cached_result.is_some(), "Should retrieve cached data");

        let (cached_alignments, cached_quality, cached_stats) = cached_result.unwrap();
        assert_eq!(cached_alignments.len(), 2, "Should have cached 2 alignments");
        assert_eq!(cached_quality.overall_quality, 0.8, "Should have cached quality");
        assert_eq!(cached_stats.total_sentences, 2, "Should have cached statistics");

        // Test cache invalidation
        let removed_count = cache_service.invalidate_language_pair("en", "es").await?;
        assert!(removed_count > 0, "Should have removed cache entries");

        // Verify data is gone
        let after_invalidation = cache_service.get_alignments(&cache_key).await?;
        assert!(after_invalidation.is_none(), "Cache should be empty after invalidation");

        // Test cache statistics
        let cache_stats = cache_service.get_statistics().await;
        assert!(cache_stats.total_entries >= 0, "Cache stats should be valid");

        println!("✓ Caching system working correctly");
        Ok(())
    }

    /// Test multi-pane alignment service
    pub async fn test_multi_pane_service() -> Result<()> {
        println!("Testing multi-pane alignment service...");

        let config = MultiPaneAlignmentConfig::default();
        let service = MultiPaneAlignmentService::new(config)?;

        // Add multiple panes
        let pane1_id = service.add_pane(
            "en".to_string(),
            "Hello world. How are you today?".to_string(),
            true,
        ).await?;

        let pane2_id = service.add_pane(
            "es".to_string(),
            "Hola mundo. ¿Cómo estás hoy?".to_string(),
            false,
        ).await?;

        let pane3_id = service.add_pane(
            "fr".to_string(),
            "Bonjour le monde. Comment allez-vous aujourd'hui?".to_string(),
            false,
        ).await?;

        // Verify panes were added
        let active_panes = service.get_active_panes().await;
        assert_eq!(active_panes.len(), 3, "Should have 3 active panes");

        // Test cursor synchronization
        let sync_positions = service.synchronize_cursor_position(pane1_id, 15).await?;
        assert_eq!(sync_positions.len(), 3, "Should sync all 3 panes");

        // Test content update
        service.update_pane_content(
            pane1_id,
            "Updated hello world. How are you today? Fine, thanks.".to_string(),
            Some(25),
        ).await?;

        // Test quality monitoring
        let quality_result = service.perform_quality_monitoring().await?;
        assert!(quality_result.overall_quality >= 0.0, "Overall quality should be valid");
        assert!(!quality_result.alignment_qualities.is_empty(), "Should have alignment qualities");

        // Test sync state
        let sync_state = service.get_sync_state().await;
        assert!(!sync_state.synchronized_positions.is_empty(), "Should have sync positions");

        // Test performance metrics
        let performance = service.get_performance_metrics().await;
        assert!(performance.alignment_time_ms >= 0.0, "Alignment time should be valid");

        // Test pane removal
        service.remove_pane(pane3_id).await?;
        let active_panes_after = service.get_active_panes().await;
        assert_eq!(active_panes_after.len(), 2, "Should have 2 panes after removal");

        println!("✓ Multi-pane alignment service working correctly");
        Ok(())
    }

    /// Test API service
    pub async fn test_api_service() -> Result<()> {
        println!("Testing API service...");

        let multi_pane_config = MultiPaneAlignmentConfig::default();
        let api_config = AlignmentApiConfig::default();
        let api_service = AlignmentApiService::new(multi_pane_config, api_config)?;

        // Test add pane API
        let add_request = AddPaneRequest {
            language: "en".to_string(),
            content: "Hello world! This is a test.".to_string(),
            is_source: true,
        };

        let add_response = api_service.add_pane(add_request).await?;
        assert!(add_response.success, "Add pane should succeed");
        
        let pane_id = add_response.pane_id;

        // Test add second pane
        let add_request2 = AddPaneRequest {
            language: "es".to_string(),
            content: "¡Hola mundo! Esta es una prueba.".to_string(),
            is_source: false,
        };

        let add_response2 = api_service.add_pane(add_request2).await?;
        assert!(add_response2.success, "Add second pane should succeed");

        // Test update pane content API
        let update_request = UpdatePaneRequest {
            pane_id,
            content: "Updated hello world! This is a test. How are you?".to_string(),
            cursor_position: Some(20),
        };

        let update_response = api_service.update_pane_content(update_request).await?;
        assert!(update_response.success, "Update pane should succeed");

        // Test cursor synchronization API
        let sync_request = SyncCursorRequest {
            source_pane_id: pane_id,
            cursor_position: 25,
        };

        let sync_response = api_service.synchronize_cursor(sync_request).await?;
        assert!(sync_response.success, "Cursor sync should succeed");
        assert!(!sync_response.synchronized_positions.is_empty(), "Should have sync positions");

        // Test system status API
        let system_status = api_service.get_system_status().await?;
        assert_eq!(system_status.active_panes.len(), 2, "Should have 2 active panes");
        assert!(system_status.quality_monitoring.overall_quality >= 0.0, "Quality should be valid");

        // Test health check
        let health_status = api_service.health_check().await?;
        assert!(!matches!(health_status, crate::services::alignment_api_service::HealthStatus::Critical), 
                "Health should not be critical");

        // Test API statistics
        let api_stats = api_service.get_api_statistics().await;
        assert!(api_stats.contains_key("total_requests"), "Should have request stats");
        assert!(api_stats["total_requests"] > 0, "Should have processed requests");

        println!("✓ API service working correctly");
        Ok(())
    }

    /// Test error handling and edge cases
    pub async fn test_error_handling() -> Result<()> {
        println!("Testing error handling and edge cases...");

        let config = MultiPaneAlignmentConfig::default();
        let service = MultiPaneAlignmentService::new(config)?;

        // Test adding too many panes
        for i in 0..5 {
            let result = service.add_pane(
                "en".to_string(),
                format!("Content {}", i),
                i == 0,
            ).await;

            if i < 4 {
                assert!(result.is_ok(), "Should succeed for pane {}", i);
            } else {
                assert!(result.is_err(), "Should fail when exceeding max panes");
            }
        }

        // Test removing non-existent pane
        let fake_id = Uuid::new_v4();
        let remove_result = service.remove_pane(fake_id).await;
        assert!(remove_result.is_err(), "Should fail to remove non-existent pane");

        // Test updating non-existent pane
        let update_result = service.update_pane_content(
            fake_id,
            "New content".to_string(),
            None,
        ).await;
        assert!(update_result.is_err(), "Should fail to update non-existent pane");

        // Test synchronization with non-existent pane
        let sync_result = service.synchronize_cursor_position(fake_id, 10).await;
        assert!(sync_result.is_err(), "Should fail to sync with non-existent pane");

        // Test unsupported language
        let unsupported_result = service.add_pane(
            "xyz".to_string(),
            "Unsupported language content".to_string(),
            true,
        ).await;
        assert!(unsupported_result.is_err(), "Should fail for unsupported language");

        println!("✓ Error handling working correctly");
        Ok(())
    }

    /// Test performance under load
    pub async fn test_performance() -> Result<()> {
        println!("Testing performance under load...");

        let config = MultiPaneAlignmentConfig::default();
        let service = MultiPaneAlignmentService::new(config)?;

        // Add panes
        let pane1_id = service.add_pane(
            "en".to_string(),
            "Performance test content. ".repeat(100), // Large content
            true,
        ).await?;

        let pane2_id = service.add_pane(
            "es".to_string(),
            "Contenido de prueba de rendimiento. ".repeat(100), // Large content
            false,
        ).await?;

        let start_time = std::time::Instant::now();

        // Perform multiple synchronizations
        for i in 0..50 {
            let _ = service.synchronize_cursor_position(pane1_id, i * 10).await?;
        }

        let sync_duration = start_time.elapsed();

        // Update content multiple times
        let start_time = std::time::Instant::now();

        for i in 0..20 {
            let _ = service.update_pane_content(
                pane1_id,
                format!("Updated content iteration {}. {}", i, "More content. ".repeat(50)),
                Some(i * 5),
            ).await?;
        }

        let update_duration = start_time.elapsed();

        // Check performance metrics
        let performance = service.get_performance_metrics().await;
        
        println!("  • Sync operations (50): {:?}", sync_duration);
        println!("  • Update operations (20): {:?}", update_duration);
        println!("  • Average alignment time: {:.2}ms", performance.alignment_time_ms);
        println!("  • Cache hit rate: {:.1}%", performance.cache_hit_rate);

        // Performance assertions
        assert!(sync_duration.as_millis() < 5000, "Sync operations should complete within 5 seconds");
        assert!(update_duration.as_millis() < 10000, "Update operations should complete within 10 seconds");

        println!("✓ Performance tests completed successfully");
        Ok(())
    }

    /// Run all integration tests
    pub async fn run_all_tests() -> Result<()> {
        println!("🚀 Starting comprehensive sentence alignment integration tests...\n");

        Self::test_complete_alignment_pipeline().await?;
        println!();

        Self::test_text_structure_analysis().await?;
        println!();

        Self::test_caching_system().await?;
        println!();

        Self::test_multi_pane_service().await?;
        println!();

        Self::test_api_service().await?;
        println!();

        Self::test_error_handling().await?;
        println!();

        Self::test_performance().await?;
        println!();

        println!("✅ All integration tests passed successfully!");
        println!("🎉 Sentence alignment system is working correctly!");

        Ok(())
    }

    // Helper function to create test alignments
    fn create_test_alignment(
        source_text: &str,
        target_text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> crate::services::sentence_alignment_service::SentenceAlignment {
        use crate::services::sentence_alignment_service::{
            SentenceAlignment, SentenceBoundary, BoundaryType, AlignmentMethod, ValidationStatus
        };
        use tokio::time::Instant;

        SentenceAlignment {
            id: Uuid::new_v4(),
            source_sentence: SentenceBoundary {
                start_offset: 0,
                end_offset: source_text.len(),
                text: source_text.to_string(),
                confidence: 0.9,
                boundary_type: BoundaryType::Period,
            },
            target_sentence: SentenceBoundary {
                start_offset: 0,
                end_offset: target_text.len(),
                text: target_text.to_string(),
                confidence: 0.9,
                boundary_type: BoundaryType::Period,
            },
            source_language: source_lang.to_string(),
            target_language: target_lang.to_string(),
            alignment_confidence: 0.8,
            alignment_method: AlignmentMethod::PositionBased,
            validation_status: ValidationStatus::Pending,
            created_at: Instant::now(),
            last_validated: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sentence_boundary_detection() {
        AlignmentIntegrationTests::test_complete_alignment_pipeline().await.unwrap();
    }

    #[tokio::test]
    async fn test_structure_analysis() {
        AlignmentIntegrationTests::test_text_structure_analysis().await.unwrap();
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        AlignmentIntegrationTests::test_caching_system().await.unwrap();
    }

    #[tokio::test]
    async fn test_multi_pane_functionality() {
        AlignmentIntegrationTests::test_multi_pane_service().await.unwrap();
    }

    #[tokio::test]
    async fn test_api_functionality() {
        AlignmentIntegrationTests::test_api_service().await.unwrap();
    }

    #[tokio::test]
    async fn test_error_cases() {
        AlignmentIntegrationTests::test_error_handling().await.unwrap();
    }

    #[tokio::test]
    async fn test_performance_load() {
        AlignmentIntegrationTests::test_performance().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Run manually for full integration test
    async fn run_full_integration_test_suite() {
        AlignmentIntegrationTests::run_all_tests().await.unwrap();
    }
}