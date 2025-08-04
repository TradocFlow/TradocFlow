#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use uuid::Uuid;
    use chrono::Utc;
    use crate::models::translation_models::{
        TranslationUnit, TranslationMetadata, LanguagePair, ChunkMetadata, ChunkType
    };
    use std::time::Instant;

    async fn create_test_service() -> (TranslationMemoryService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let service = TranslationMemoryService::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();
        (service, temp_dir)
    }

    fn create_test_translation_unit(
        source_text: &str,
        target_text: &str,
        confidence: f32,
    ) -> TranslationUnit {
        TranslationUnit::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "en".to_string(),
            source_text.to_string(),
            "es".to_string(),
            target_text.to_string(),
            confidence,
            None,
        ).unwrap()
    }

    #[tokio::test]
    async fn test_create_translation_memory() {
        let (service, _temp_dir) = create_test_service().await;
        let project_id = Uuid::new_v4();
        
        let result = service.create_translation_memory(project_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_single_translation_unit() {
        let (service, _temp_dir) = create_test_service().await;
        let project_id = Uuid::new_v4();
        
        service.create_translation_memory(project_id).await.unwrap();
        
        let unit = create_test_translation_unit(
            "Hello world",
            "Hola mundo",
            0.95,
        );
        
        let result = service.add_translation_unit(unit).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_translation_units_batch() {
        let (service, _temp_dir) = create_test_service().await;
        let project_id = Uuid::new_v4();
        
        service.create_translation_memory(project_id).await.unwrap();
        
        let units = vec![
            create_test_translation_unit("Hello", "Hola", 0.9),
            create_test_translation_unit("World", "Mundo", 0.95),
            create_test_translation_unit("Good morning", "Buenos días", 0.88),
        ];
        
        let result = service.add_translation_units_batch(units).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_search_exact_match() {
        let (service, _temp_dir) = create_test_service().await;
        let project_id = Uuid::new_v4();
        
        service.create_translation_memory(project_id).await.unwrap();
        
        let unit = create_test_translation_unit(
            "Hello world",
            "Hola mundo",
            0.95,
        );
        
        service.add_translation_unit(unit).await.unwrap();
        
        let language_pair = LanguagePair {
            source: "en".to_string(),
            target: "es".to_string(),
        };
        
        let matches = service
            .search_similar_translations("Hello world", language_pair)
            .await
            .unwrap();
        
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].similarity_score, 1.0);
        assert_eq!(matches[0].target_text, "Hola mundo");
    }

    #[tokio::test]
    async fn test_search_fuzzy_match() {
        let (service, _temp_dir) = create_test_service().await;
        let project_id = Uuid::new_v4();
        
        service.create_translation_memory(project_id).await.unwrap();
        
        let units = vec![
            create_test_translation_unit("Hello world", "Hola mundo", 0.95),
            create_test_translation_unit("Hello there", "Hola allí", 0.9),
            create_test_translation_unit("Good morning world", "Buenos días mundo", 0.88),
        ];
        
        service.add_translation_units_batch(units).await.unwrap();
        
        let language_pair = LanguagePair {
            source: "en".to_string(),
            target: "es".to_string(),
        };
        
        let matches = service
            .search_similar_translations("Hello", language_pair)
            .await
            .unwrap();
        
        assert!(!matches.is_empty());
        // Should find matches containing "Hello"
        assert!(matches.iter().any(|m| m.source_text.contains("Hello")));
    }

    #[tokio::test]
    async fn test_similarity_calculation() {
        let (service, _temp_dir) = create_test_service().await;
        
        // Test exact match
        let similarity = service.calculate_similarity("hello world", "hello world");
        assert_eq!(similarity, 1.0);
        
        // Test partial match
        let similarity = service.calculate_similarity("hello world", "hello there");
        assert!(similarity > 0.0 && similarity < 1.0);
        
        // Test no match
        let similarity = service.calculate_similarity("hello", "goodbye");
        assert_eq!(similarity, 0.0);
    }

    #[tokio::test]
    async fn test_ngram_similarity() {
        let (service, _temp_dir) = create_test_service().await;
        
        // Test similar strings
        let similarity = service.calculate_ngram_similarity("hello", "hallo");
        assert!(similarity > 0.5);
        
        // Test very different strings
        let similarity = service.calculate_ngram_similarity("hello", "xyz");
        assert!(similarity < 0.3);
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let (service, _temp_dir) = create_test_service().await;
        let project_id = Uuid::new_v4();
        
        service.create_translation_memory(project_id).await.unwrap();
        
        let unit = create_test_translation_unit("Test cache", "Probar caché", 0.9);
        service.add_translation_unit(unit).await.unwrap();
        
        let language_pair = LanguagePair {
            source: "en".to_string(),
            target: "es".to_string(),
        };
        
        // First search - should populate cache
        let start = Instant::now();
        let matches1 = service
            .search_similar_translations("Test cache", language_pair.clone())
            .await
            .unwrap();
        let first_duration = start.elapsed();
        
        // Second search - should use cache
        let start = Instant::now();
        let matches2 = service
            .search_similar_translations("Test cache", language_pair)
            .await
            .unwrap();
        let second_duration = start.elapsed();
        
        assert_eq!(matches1.len(), matches2.len());
        // Cache should make second search faster (though this might not always be true in tests)
        println!("First search: {:?}, Second search: {:?}", first_duration, second_duration);
        
        // Check cache stats
        let (cache_translations, cache_chunks, last_updated) = service.get_cache_stats().await;
        assert!(cache_translations > 0);
        assert!(last_updated.is_some());
    }

    #[tokio::test]
    async fn test_chunk_management() {
        let (service, _temp_dir) = create_test_service().await;
        
        let chunk1 = ChunkMetadata::new(
            0,
            vec![0, 10, 20],
            ChunkType::Sentence,
        ).unwrap();
        
        let chunk2 = ChunkMetadata::new(
            1,
            vec![0, 15],
            ChunkType::Sentence,
        ).unwrap();
        
        let chunk1_id = chunk1.id;
        let chunk2_id = chunk2.id;
        
        // Add chunks
        service.chunk_manager.add_chunks_batch(vec![chunk1, chunk2]).await.unwrap();
        
        // Link chunks
        service.chunk_manager.link_chunks(
            vec![chunk1_id, chunk2_id],
            ChunkLinkType::Sequential,
        ).await.unwrap();
        
        // Get linked chunks
        let linked = service.chunk_manager.get_linked_chunks(chunk1_id).await.unwrap();
        assert_eq!(linked.len(), 1);
        assert_eq!(linked[0].id, chunk2_id);
    }

    #[tokio::test]
    async fn test_performance_large_batch() {
        let (service, _temp_dir) = create_test_service().await;
        let project_id = Uuid::new_v4();
        
        service.create_translation_memory(project_id).await.unwrap();
        
        // Create a large batch of translation units
        let mut units = Vec::new();
        for i in 0..1000 {
            units.push(create_test_translation_unit(
                &format!("Source text number {}", i),
                &format!("Texto fuente número {}", i),
                0.8 + (i as f32 % 20) / 100.0, // Vary confidence scores
            ));
        }
        
        let start = Instant::now();
        let result = service.add_translation_units_batch(units).await;
        let duration = start.elapsed();
        
        assert!(result.is_ok());
        println!("Added 1000 translation units in {:?}", duration);
        
        // Test search performance
        let language_pair = LanguagePair {
            source: "en".to_string(),
            target: "es".to_string(),
        };
        
        let start = Instant::now();
        let matches = service
            .search_similar_translations("Source text number 500", language_pair)
            .await
            .unwrap();
        let search_duration = start.elapsed();
        
        assert!(!matches.is_empty());
        println!("Search completed in {:?}", search_duration);
        
        // Performance assertions - these are rough guidelines
        assert!(duration.as_millis() < 5000, "Batch insert took too long: {:?}", duration);
        assert!(search_duration.as_millis() < 1000, "Search took too long: {:?}", search_duration);
    }

    #[tokio::test]
    async fn test_parquet_file_creation() {
        let (service, temp_dir) = create_test_service().await;
        let project_id = Uuid::new_v4();
        
        service.create_translation_memory(project_id).await.unwrap();
        
        // Check that Parquet files are created
        let tm_path = temp_dir.path().join("translation_memory");
        let units_file = tm_path.join("translation_units.parquet");
        let chunks_file = tm_path.join("chunks.parquet");
        
        assert!(units_file.exists(), "Translation units Parquet file should exist");
        assert!(chunks_file.exists(), "Chunks Parquet file should exist");
        
        // Add some data and verify it's written to Parquet
        let unit = create_test_translation_unit("Test parquet", "Probar parquet", 0.9);
        service.add_translation_unit(unit).await.unwrap();
        
        // File should still exist and have content
        assert!(units_file.exists());
        let metadata = std::fs::metadata(&units_file).unwrap();
        assert!(metadata.len() > 0, "Parquet file should have content");
    }

    #[tokio::test]
    async fn test_duckdb_integration() {
        let (service, _temp_dir) = create_test_service().await;
        let project_id = Uuid::new_v4();
        
        service.create_translation_memory(project_id).await.unwrap();
        
        // Add translation units
        let units = vec![
            create_test_translation_unit("Database test", "Prueba de base de datos", 0.95),
            create_test_translation_unit("Integration test", "Prueba de integración", 0.9),
        ];
        
        service.add_translation_units_batch(units).await.unwrap();
        
        // Test that DuckDB can query the data
        let conn = service.duckdb_connection.read().await;
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM translation_units").unwrap();
        let count: i64 = stmt.query_row(params![], |row| row.get(0)).unwrap();
        
        assert_eq!(count, 2);
        
        // Test language pair filtering
        let mut stmt = conn.prepare(
            "SELECT COUNT(*) FROM translation_units WHERE source_language = ? AND target_language = ?"
        ).unwrap();
        let count: i64 = stmt.query_row(params!["en", "es"], |row| row.get(0)).unwrap();
        
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_translation_suggestions() {
        let (service, _temp_dir) = create_test_service().await;
        let project_id = Uuid::new_v4();
        
        service.create_translation_memory(project_id).await.unwrap();
        
        let units = vec![
            create_test_translation_unit("Good morning", "Buenos días", 0.95),
            create_test_translation_unit("Good evening", "Buenas tardes", 0.9),
            create_test_translation_unit("Good night", "Buenas noches", 0.92),
        ];
        
        service.add_translation_units_batch(units).await.unwrap();
        
        let suggestions = service
            .get_translation_suggestions("Good afternoon", "es")
            .await
            .unwrap();
        
        assert!(!suggestions.is_empty());
        // Should find suggestions with "Good" in them
        assert!(suggestions.iter().any(|s| s.source_text.contains("Good")));
    }
}