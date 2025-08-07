//! Unit tests for storage layer components
//!
//! Tests for DuckDBManager, ParquetManager, and ChunkManager

#[cfg(test)]
mod duckdb_manager_tests {
    use super::super::*;
    use crate::storage::duckdb_manager::*;
    use crate::models::{Language, TranslationUnit};
    use crate::services::translation_memory::LanguagePair;
    use tempfile::tempdir;
    use std::path::Path;
    
    #[tokio::test]
    async fn test_duckdb_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let result = DuckDBManager::new(&db_path, Some(5)).await;
        assert!(result.is_ok(), "DuckDB manager should be created successfully");
        
        let manager = result.unwrap();
        assert_eq!(manager.db_path(), &db_path);
    }
    
    #[tokio::test]
    async fn test_schema_initialization() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let manager = DuckDBManager::new(&db_path, None).await.unwrap();
        let result = manager.initialize_schema().await;
        
        assert!(result.is_ok(), "Schema initialization should succeed");
        
        // Second call should also succeed (idempotent)
        let result2 = manager.initialize_schema().await;
        assert!(result2.is_ok(), "Second schema initialization should also succeed");
    }
    
    #[tokio::test]
    async fn test_translation_memory_schema() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let manager = DuckDBManager::new(&db_path, None).await.unwrap();
        let result = manager.initialize_translation_memory_schema().await;
        
        assert!(result.is_ok(), "Translation memory schema initialization should succeed");
    }
    
    #[tokio::test]
    async fn test_terminology_schema() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let manager = DuckDBManager::new(&db_path, None).await.unwrap();
        let result = manager.initialize_terminology_schema().await;
        
        assert!(result.is_ok(), "Terminology schema initialization should succeed");
    }
    
    #[tokio::test]
    async fn test_translation_unit_operations() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let manager = DuckDBManager::new(&db_path, None).await.unwrap();
        manager.initialize_schema().await.unwrap();
        
        let fixtures = TestFixtures::new();
        let units = create_test_translation_units(3, fixtures.project_id);
        
        // Test single insert
        let result = manager.insert_translation_unit(&units[0]).await;
        assert!(result.is_ok(), "Should be able to insert translation unit");
        
        // Test batch insert
        let batch_result = manager.insert_translation_units_batch(&units[1..]).await;
        assert!(batch_result.is_ok(), "Should be able to batch insert translation units");
        
        // Test update
        let update_result = manager.update_translation_unit(&units[0]).await;
        assert!(update_result.is_ok(), "Should be able to update translation unit");
        
        // Test delete
        let delete_result = manager.delete_translation_unit(units[0].id).await;
        assert!(delete_result.is_ok(), "Should be able to delete translation unit");
        assert!(delete_result.unwrap(), "Delete should return true for successful deletion");
    }
    
    #[tokio::test]
    async fn test_translation_search_operations() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let manager = DuckDBManager::new(&db_path, None).await.unwrap();
        manager.initialize_schema().await.unwrap();
        
        let language_pair = LanguagePair {
            source: Language::English,
            target: Language::Spanish,
        };
        
        // Test exact match search
        let exact_result = manager.search_exact_matches("test text", &language_pair).await;
        assert!(exact_result.is_ok(), "Exact match search should succeed");
        
        // Test fuzzy match search
        let fuzzy_result = manager.search_fuzzy_matches("test text", &language_pair, 0.7).await;
        assert!(fuzzy_result.is_ok(), "Fuzzy match search should succeed");
        
        // Test n-gram match search
        let ngram_result = manager.search_ngram_matches("test text", &language_pair, 0.6).await;
        assert!(ngram_result.is_ok(), "N-gram match search should succeed");
    }
    
    #[tokio::test]
    async fn test_terminology_operations() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let manager = DuckDBManager::new(&db_path, None).await.unwrap();
        manager.initialize_schema().await.unwrap();
        
        let fixtures = TestFixtures::new();
        let terms = create_test_terminology_entries(2);
        
        // Test insert terminology
        let insert_result = manager.insert_terminology(&terms[0], fixtures.project_id).await;
        assert!(insert_result.is_ok(), "Should be able to insert terminology");
        
        // Test update terminology
        let update_result = manager.update_terminology(&terms[0], fixtures.project_id).await;
        assert!(update_result.is_ok(), "Should be able to update terminology");
        assert!(update_result.unwrap(), "Update should return true for successful update");
        
        // Test get terms by project
        let get_result = manager.get_terms_by_project(fixtures.project_id).await;
        assert!(get_result.is_ok(), "Should be able to get terms by project");
        
        // Test search terms
        let search_result = manager.search_terms("test", fixtures.project_id, false).await;
        assert!(search_result.is_ok(), "Should be able to search terms");
        
        // Test batch update
        let batch_update_result = manager.update_terminology_batch(fixtures.project_id, &terms).await;
        assert!(batch_update_result.is_ok(), "Should be able to batch update terminology");
        
        // Test delete terminology
        let delete_result = manager.delete_terminology(terms[0].id, fixtures.project_id).await;
        assert!(delete_result.is_ok(), "Should be able to delete terminology");
        assert!(delete_result.unwrap(), "Delete should return true for successful deletion");
    }
    
    #[tokio::test]
    async fn test_database_utilities() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let manager = DuckDBManager::new(&db_path, None).await.unwrap();
        manager.initialize_schema().await.unwrap();
        
        // Test custom query execution
        let query_result = manager.execute_query("SELECT 1").await;
        assert!(query_result.is_ok(), "Should be able to execute custom queries");
        
        // Test database statistics
        let stats_result = manager.get_database_stats().await;
        assert!(stats_result.is_ok(), "Should be able to get database statistics");
        
        let stats = stats_result.unwrap();
        assert!(stats.translation_units_count >= 0, "Stats should have valid counts");
        assert!(stats.terminology_entries_count >= 0, "Stats should have valid counts");
        
        // Test database optimization
        let optimize_result = manager.optimize_database().await;
        assert!(optimize_result.is_ok(), "Database optimization should succeed");
        
        // Test connection pool stats
        let (available, max) = manager.get_connection_pool_stats().await;
        assert!(available <= max, "Available connections should not exceed max");
        assert!(max > 0, "Max connections should be positive");
    }
}

#[cfg(test)]
mod parquet_manager_tests {
    use super::super::*;
    use crate::storage::parquet_manager::*;
    use crate::models::{Language, TranslationUnit};
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_parquet_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let result = ParquetManager::new(temp_dir.path().to_str().unwrap()).await;
        
        assert!(result.is_ok(), "Parquet manager should be created successfully");
        
        let manager = result.unwrap();
        assert_eq!(manager.base_path(), temp_dir.path());
    }
    
    #[tokio::test]
    async fn test_project_files_creation() {
        let temp_dir = tempdir().unwrap();
        let manager = ParquetManager::new(temp_dir.path().to_str().unwrap()).await.unwrap();
        let fixtures = TestFixtures::new();
        
        let result = manager.create_project_files(fixtures.project_id).await;
        assert!(result.is_ok(), "Should be able to create project files");
        
        // Verify directories were created
        let project_dir = temp_dir.path().join(format!("project_{}", fixtures.project_id));
        assert!(project_dir.exists(), "Project directory should exist");
        assert!(project_dir.join("translation_units").exists(), "Translation units directory should exist");
        assert!(project_dir.join("terminology").exists(), "Terminology directory should exist");
        assert!(project_dir.join("chunks").exists(), "Chunks directory should exist");
        assert!(project_dir.join("metadata").exists(), "Metadata directory should exist");
    }
    
    #[tokio::test]
    async fn test_translation_unit_operations() {
        let temp_dir = tempdir().unwrap();
        let manager = ParquetManager::new(temp_dir.path().to_str().unwrap()).await.unwrap();
        let fixtures = TestFixtures::new();
        
        manager.create_project_files(fixtures.project_id).await.unwrap();
        let units = create_test_translation_units(3, fixtures.project_id);
        
        // Test single append
        let append_result = manager.append_translation_unit(&units[0]).await;
        assert!(append_result.is_ok(), "Should be able to append translation unit");
        
        // Test batch append
        let batch_result = manager.append_translation_units_batch(&units).await;
        assert!(batch_result.is_ok(), "Should be able to batch append translation units");
        
        // Test update
        let update_result = manager.update_translation_unit(&units[0]).await;
        assert!(update_result.is_ok(), "Should be able to update translation unit");
        
        // Test delete
        let delete_result = manager.delete_translation_unit(units[0].id).await;
        assert!(delete_result.is_ok(), "Should be able to delete translation unit");
    }
    
    #[tokio::test]
    async fn test_terminology_operations() {
        let temp_dir = tempdir().unwrap();
        let manager = ParquetManager::new(temp_dir.path().to_str().unwrap()).await.unwrap();
        let fixtures = TestFixtures::new();
        
        manager.create_project_files(fixtures.project_id).await.unwrap();
        let terms = create_test_terminology_entries(3);
        
        // Test convert terms to parquet
        let convert_result = manager.convert_terms_to_parquet(&terms, fixtures.project_id).await;
        assert!(convert_result.is_ok(), "Should be able to convert terms to Parquet");
        
        // Test update terminology
        let update_result = manager.update_terminology(&terms[0], fixtures.project_id).await;
        assert!(update_result.is_ok(), "Should be able to update terminology");
        
        // Test delete terminology
        let delete_result = manager.delete_terminology(terms[0].id, fixtures.project_id).await;
        assert!(delete_result.is_ok(), "Should be able to delete terminology");
        
        // Test refresh parquet files
        let refresh_result = manager.refresh_parquet_files(fixtures.project_id, &terms).await;
        assert!(refresh_result.is_ok(), "Should be able to refresh Parquet files");
    }
    
    #[tokio::test]
    async fn test_generic_parquet_operations() {
        let temp_dir = tempdir().unwrap();
        let manager = ParquetManager::new(temp_dir.path().to_str().unwrap()).await.unwrap();
        
        let test_data = b"test data for parquet";
        let filename = "test.parquet";
        
        // Test export
        let export_result = manager.export_to_parquet(test_data, filename).await;
        assert!(export_result.is_ok(), "Should be able to export to Parquet");
        
        // Test import
        let import_result = manager.import_from_parquet(filename).await;
        assert!(import_result.is_ok(), "Should be able to import from Parquet");
        
        let imported_data = import_result.unwrap();
        assert_eq!(imported_data, test_data, "Imported data should match exported data");
    }
    
    #[tokio::test]
    async fn test_storage_statistics() {
        let temp_dir = tempdir().unwrap();
        let manager = ParquetManager::new(temp_dir.path().to_str().unwrap()).await.unwrap();
        
        // Test storage stats
        let stats_result = manager.get_storage_stats().await;
        assert!(stats_result.is_ok(), "Should be able to get storage statistics");
        
        let stats = stats_result.unwrap();
        assert_eq!(stats.total_files, 0, "Should start with no files");
        assert_eq!(stats.total_size_bytes, 0, "Should start with zero size");
        assert_eq!(stats.project_count, 0, "Should start with no projects");
        
        // Test compression stats
        let compression_stats = manager.get_compression_stats(Some(ParquetFileType::TranslationUnits)).await;
        assert!(compression_stats.is_ok(), "Should be able to get compression statistics");
        
        let overall_stats = manager.get_compression_stats(None).await;
        assert!(overall_stats.is_ok(), "Should be able to get overall compression statistics");
    }
    
    #[tokio::test]
    async fn test_maintenance_operations() {
        let temp_dir = tempdir().unwrap();
        let manager = ParquetManager::new(temp_dir.path().to_str().unwrap()).await.unwrap();
        let fixtures = TestFixtures::new();
        
        // Test file cleanup
        let cleanup_result = manager.cleanup_old_files(30).await;
        assert!(cleanup_result.is_ok(), "Should be able to cleanup old files");
        assert_eq!(cleanup_result.unwrap(), 0, "Should have no files to clean initially");
        
        // Test project optimization
        let optimize_result = manager.optimize_project_files(fixtures.project_id).await;
        assert!(optimize_result.is_ok(), "Should be able to optimize project files");
        
        // Test project file metadata
        let metadata_result = manager.get_project_file_metadata(fixtures.project_id).await;
        assert!(metadata_result.is_ok(), "Should be able to get project file metadata");
        
        let metadata = metadata_result.unwrap();
        assert_eq!(metadata.len(), 0, "Should start with no metadata");
    }
}

#[cfg(test)]
mod chunk_manager_tests {
    use super::super::*;
    use crate::storage::{ChunkManager, DuckDBManager};
    use crate::models::ChunkMetadata;
    use tempfile::tempdir;
    use uuid::Uuid;
    
    #[tokio::test]
    async fn test_chunk_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let duckdb_manager = DuckDBManager::new(&db_path, Some(5)).await.unwrap();
        let result = ChunkManager::new(duckdb_manager).await;
        
        assert!(result.is_ok(), "Chunk manager should be created successfully");
    }
    
    #[tokio::test]
    async fn test_chunk_operations() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let duckdb_manager = DuckDBManager::new(&db_path, Some(5)).await.unwrap();
        let chunk_manager = ChunkManager::new(duckdb_manager).await.unwrap();
        
        // Create test chunk metadata
        let chunk = ChunkMetadata {
            id: Uuid::new_v4(),
            original_position: 0,
            sentence_boundaries: vec![0, 10, 25],
            linked_chunks: vec![],
            chunk_type: crate::models::ChunkType::Sentence,
            processing_notes: vec!["Test chunk".to_string()],
        };
        
        // Test store chunk
        let store_result = chunk_manager.store_chunk(&chunk).await;
        assert!(store_result.is_ok(), "Should be able to store chunk");
        
        // Test get chunk
        let get_result = chunk_manager.get_chunk(chunk.id).await;
        assert!(get_result.is_ok(), "Should be able to get chunk");
        
        // Test update chunk
        let update_result = chunk_manager.update_chunk(&chunk).await;
        assert!(update_result.is_ok(), "Should be able to update chunk");
        
        // Test delete chunk
        let delete_result = chunk_manager.delete_chunk(chunk.id).await;
        assert!(delete_result.is_ok(), "Should be able to delete chunk");
    }
}