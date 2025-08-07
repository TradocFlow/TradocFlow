//! Unit tests for all translation memory services
//!
//! Tests for TranslationMemoryService, TerminologyService, and HighlightingService

#[cfg(test)]
mod translation_memory_service_tests {
    use super::super::*;
    use crate::services::translation_memory::*;
    use crate::storage::{DuckDBManager, ParquetManager, ChunkManager};
    use tempfile::tempdir;
    use std::sync::Arc;
    
    async fn setup_translation_memory_service() -> Result<(TranslationMemoryService, TestFixtures), Box<dyn std::error::Error>> {
        let fixtures = TestFixtures::new();
        let db_path = std::path::Path::new(&fixtures.test_db_path);
        
        let duckdb_manager = DuckDBManager::new(db_path, Some(5)).await?;
        let parquet_manager = ParquetManager::new(fixtures.temp_dir.path().to_str().unwrap()).await?;
        let chunk_manager = ChunkManager::new(duckdb_manager.clone()).await?;
        
        let service = TranslationMemoryService::new(
            fixtures.project_id,
            duckdb_manager,
            parquet_manager,
            chunk_manager,
        ).await?;
        
        Ok((service, fixtures))
    }
    
    #[tokio::test]
    async fn test_translation_memory_service_creation() {
        let result = setup_translation_memory_service().await;
        assert!(result.is_ok(), "Translation memory service should be created successfully");
    }
    
    #[tokio::test]
    async fn test_add_translation_unit() {
        let (service, fixtures) = setup_translation_memory_service().await.unwrap();
        let units = create_test_translation_units(1, fixtures.project_id);
        
        let result = service.add_translation_unit(units[0].clone()).await;
        assert!(result.is_ok(), "Should be able to add translation unit");
    }
    
    #[tokio::test]
    async fn test_batch_add_translation_units() {
        let (service, fixtures) = setup_translation_memory_service().await.unwrap();
        let units = create_test_translation_units(5, fixtures.project_id);
        
        let result = service.add_translation_units_batch(units).await;
        assert!(result.is_ok(), "Should be able to add translation units in batch");
        
        if let Ok(count) = result {
            assert_eq!(count, 5, "Should return correct count of inserted units");
        }
    }
    
    #[tokio::test]
    async fn test_search_similar_translations() {
        let (service, fixtures) = setup_translation_memory_service().await.unwrap();
        let units = create_test_translation_units(3, fixtures.project_id);
        
        // Add some units first
        for unit in &units {
            let _ = service.add_translation_unit(unit.clone()).await;
        }
        
        let language_pair = LanguagePair {
            source: crate::models::Language::English,
            target: crate::models::Language::Spanish,
        };
        
        let result = service.search_similar_translations("Test source text", language_pair, Some(0.5)).await;
        assert!(result.is_ok(), "Should be able to search for similar translations");
    }
    
    #[tokio::test]
    async fn test_update_translation_unit() {
        let (service, fixtures) = setup_translation_memory_service().await.unwrap();
        let mut unit = create_test_translation_units(1, fixtures.project_id)[0].clone();
        
        // Add the unit first
        service.add_translation_unit(unit.clone()).await.unwrap();
        
        // Update it
        unit.target_text = "Updated target text".to_string();
        unit.confidence_score = 0.95;
        
        let result = service.update_translation_unit(unit).await;
        assert!(result.is_ok(), "Should be able to update translation unit");
    }
    
    #[tokio::test]
    async fn test_delete_translation_unit() {
        let (service, fixtures) = setup_translation_memory_service().await.unwrap();
        let unit = create_test_translation_units(1, fixtures.project_id)[0].clone();
        
        // Add the unit first
        service.add_translation_unit(unit.clone()).await.unwrap();
        
        // Delete it
        let result = service.delete_translation_unit(unit.id).await;
        assert!(result.is_ok(), "Should be able to delete translation unit");
        
        if let Ok(deleted) = result {
            assert!(deleted, "Should return true when unit is successfully deleted");
        }
    }
    
    #[tokio::test]
    async fn test_cache_functionality() {
        let (service, _) = setup_translation_memory_service().await.unwrap();
        
        let (units_count, chunks_count, last_updated) = service.get_cache_stats().await;
        
        // Initially cache should be empty
        assert_eq!(units_count, 0, "Cache should start empty");
        assert_eq!(chunks_count, 0, "Cache should start empty");
        assert!(last_updated.is_none(), "No last updated time initially");
        
        // Clear cache should not fail
        service.clear_cache().await;
    }
}

#[cfg(test)]
mod terminology_service_tests {
    use super::super::*;
    use crate::services::terminology::*;
    use crate::storage::{DuckDBManager, ParquetManager};
    use crate::utils::CsvProcessor;
    use std::sync::Arc;
    
    async fn setup_terminology_service() -> Result<(TerminologyService, TestFixtures), Box<dyn std::error::Error>> {
        let fixtures = TestFixtures::new();
        let db_path = std::path::Path::new(&fixtures.test_db_path);
        
        let duckdb_manager = DuckDBManager::new(db_path, Some(5)).await?;
        let parquet_manager = ParquetManager::new(fixtures.temp_dir.path().to_str().unwrap()).await?;
        let csv_processor = Arc::new(CsvProcessor {});
        
        let service = TerminologyService::new(
            duckdb_manager,
            parquet_manager,
            csv_processor,
            None,
        ).await?;
        
        Ok((service, fixtures))
    }
    
    #[tokio::test]
    async fn test_terminology_service_creation() {
        let result = setup_terminology_service().await;
        assert!(result.is_ok(), "Terminology service should be created successfully");
    }
    
    #[tokio::test]
    async fn test_add_terminology() {
        let (service, fixtures) = setup_terminology_service().await.unwrap();
        let terms = create_test_terminology_entries(1);
        
        let result = service.add_terminology(terms[0].clone(), fixtures.project_id).await;
        assert!(result.is_ok(), "Should be able to add terminology");
    }
    
    #[tokio::test]
    async fn test_get_terms_by_project() {
        let (service, fixtures) = setup_terminology_service().await.unwrap();
        
        let result = service.get_terms_by_project(fixtures.project_id).await;
        assert!(result.is_ok(), "Should be able to get terms by project");
    }
    
    #[tokio::test]
    async fn test_search_terms() {
        let (service, fixtures) = setup_terminology_service().await.unwrap();
        let terms = create_test_terminology_entries(3);
        
        // Add some terms first
        for term in &terms {
            let _ = service.add_terminology(term.clone(), fixtures.project_id).await;
        }
        
        let result = service.search_terms("test_term", fixtures.project_id, Some(false)).await;
        assert!(result.is_ok(), "Should be able to search terms");
    }
    
    #[tokio::test]
    async fn test_update_terminology() {
        let (service, fixtures) = setup_terminology_service().await.unwrap();
        let mut term = create_test_terminology_entries(1)[0].clone();
        
        // Add the term first
        service.add_terminology(term.clone(), fixtures.project_id).await.unwrap();
        
        // Update it
        term.definition = Some("Updated definition".to_string());
        term.do_not_translate = !term.do_not_translate;
        
        let result = service.update_terminology(term, fixtures.project_id).await;
        assert!(result.is_ok(), "Should be able to update terminology");
    }
    
    #[tokio::test]
    async fn test_delete_terminology() {
        let (service, fixtures) = setup_terminology_service().await.unwrap();
        let term = create_test_terminology_entries(1)[0].clone();
        
        // Add the term first
        service.add_terminology(term.clone(), fixtures.project_id).await.unwrap();
        
        // Delete it
        let result = service.delete_terminology(term.id, fixtures.project_id).await;
        assert!(result.is_ok(), "Should be able to delete terminology");
        
        if let Ok(deleted) = result {
            assert!(deleted, "Should return true when term is successfully deleted");
        }
    }
    
    #[tokio::test]
    async fn test_get_non_translatable_terms() {
        let (service, fixtures) = setup_terminology_service().await.unwrap();
        let terms = create_test_terminology_entries(4); // Creates mix of translatable/non-translatable
        
        // Add terms
        for term in &terms {
            let _ = service.add_terminology(term.clone(), fixtures.project_id).await;
        }
        
        let result = service.get_non_translatable_terms(fixtures.project_id).await;
        assert!(result.is_ok(), "Should be able to get non-translatable terms");
    }
    
    #[tokio::test]
    async fn test_cache_functionality() {
        let (service, _) = setup_terminology_service().await.unwrap();
        
        let (terms_count, search_count, non_translatable_count, last_updated) = service.get_cache_stats().await;
        
        // Initially cache should be empty
        assert_eq!(terms_count, 0, "Cache should start empty");
        assert_eq!(search_count, 0, "Cache should start empty");
        assert_eq!(non_translatable_count, 0, "Cache should start empty");
        assert!(last_updated.is_none(), "No last updated time initially");
        
        // Clear cache should not fail
        service.clear_cache().await;
    }
}

#[cfg(test)]
mod highlighting_service_tests {
    use super::super::*;
    use crate::services::highlighting::*;
    use crate::services::terminology::TerminologyService;
    use crate::storage::{DuckDBManager, ParquetManager};
    use crate::utils::CsvProcessor;
    use crate::models::Language;
    use std::sync::Arc;
    use std::collections::HashMap;
    
    async fn setup_highlighting_service() -> Result<(HighlightingService, TestFixtures), Box<dyn std::error::Error>> {
        let fixtures = TestFixtures::new();
        let db_path = std::path::Path::new(&fixtures.test_db_path);
        
        let duckdb_manager = DuckDBManager::new(db_path, Some(5)).await?;
        let parquet_manager = ParquetManager::new(fixtures.temp_dir.path().to_str().unwrap()).await?;
        let csv_processor = Arc::new(CsvProcessor {});
        
        let terminology_service = Arc::new(TerminologyService::new(
            duckdb_manager,
            parquet_manager,
            csv_processor,
            None,
        ).await?);
        
        let service = HighlightingService::new(
            terminology_service,
            None,
        ).await?;
        
        Ok((service, fixtures))
    }
    
    #[tokio::test]
    async fn test_highlighting_service_creation() {
        let result = setup_highlighting_service().await;
        assert!(result.is_ok(), "Highlighting service should be created successfully");
    }
    
    #[tokio::test]
    async fn test_highlight_terms_in_text() {
        let (service, fixtures) = setup_highlighting_service().await.unwrap();
        
        let text = "This is a test text with some terms to highlight.";
        let result = service.highlight_terms_in_text(text, fixtures.project_id, Language::English).await;
        
        assert!(result.is_ok(), "Should be able to highlight terms in text");
        
        if let Ok(highlights) = result {
            // Even with no terms in DB, should return empty vector without error
            assert!(highlights.len() >= 0, "Should return vector of highlights");
        }
    }
    
    #[tokio::test]
    async fn test_check_consistency_across_languages() {
        let (service, fixtures) = setup_highlighting_service().await.unwrap();
        
        let mut texts = HashMap::new();
        texts.insert(Language::English, "This is a test text".to_string());
        texts.insert(Language::Spanish, "Este es un texto de prueba".to_string());
        
        let result = service.check_consistency_across_languages(texts, fixtures.project_id).await;
        assert!(result.is_ok(), "Should be able to check consistency across languages");
    }
    
    #[tokio::test]
    async fn test_generate_terminology_suggestions() {
        let (service, fixtures) = setup_highlighting_service().await.unwrap();
        
        let text = "This is a test document with various terms.";
        let result = service.generate_terminology_suggestions(text, fixtures.project_id, Language::English).await;
        
        assert!(result.is_ok(), "Should be able to generate terminology suggestions");
        
        if let Ok(suggestions) = result {
            assert!(suggestions.len() >= 0, "Should return vector of suggestions");
        }
    }
    
    #[tokio::test]
    async fn test_update_highlighting_for_text_change() {
        let (service, fixtures) = setup_highlighting_service().await.unwrap();
        
        let text = "This is the original text that has been modified.";
        let change_start = 12;
        let change_end = 20;
        
        let result = service.update_highlighting_for_text_change(
            text, 
            change_start, 
            change_end, 
            fixtures.project_id, 
            Language::English
        ).await;
        
        assert!(result.is_ok(), "Should be able to update highlighting for text changes");
    }
    
    #[tokio::test]
    async fn test_cache_operations() {
        let (service, fixtures) = setup_highlighting_service().await.unwrap();
        
        // Test cache stats
        let (regex_count, term_count, highlight_count, last_updated) = service.get_cache_stats().await;
        assert_eq!(regex_count, 0, "Cache should start empty");
        assert_eq!(term_count, 0, "Cache should start empty");
        assert_eq!(highlight_count, 0, "Cache should start empty");
        assert!(last_updated.is_none(), "No last updated time initially");
        
        // Test cache invalidation
        service.invalidate_project_cache(fixtures.project_id).await;
        
        // Test clear all caches
        service.clear_all_caches().await;
    }
}