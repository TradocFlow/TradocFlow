//! Translation memory service tests

use tradocflow_translation_memory::services::TranslationMemoryService;
use tradocflow_translation_memory::models::Language;
use tempfile::TempDir;
use uuid::Uuid;

#[tokio::test]
async fn test_translation_memory_service_creation() {
    let temp_dir = TempDir::new().unwrap();
    let project_id = Uuid::new_v4();
    let project_path = temp_dir.path().to_path_buf();
    
    let service = TranslationMemoryService::new(project_id, project_path).await.unwrap();
    
    // Test basic functionality - get_cache_stats returns (usize, usize, Option<DateTime<Utc>>)
    let (translation_units_count, chunks_count, _last_updated) = service.get_cache_stats().await;
    assert_eq!(translation_units_count, 0);
    assert_eq!(chunks_count, 0);
}

#[tokio::test] 
async fn test_translation_memory_basic_operations() {
    let temp_dir = TempDir::new().unwrap();
    let project_id = Uuid::new_v4();
    let project_path = temp_dir.path().to_path_buf();
    
    let service = TranslationMemoryService::new(project_id, project_path).await.unwrap();
    
    // Test getting translation suggestions (should be empty initially)
    // API is: get_translation_suggestions(source_text, target_language, source_language_option)
    let suggestions = service.get_translation_suggestions(
        "Hello world", 
        Language::Spanish,
        Some(Language::English)
    ).await.unwrap();
    assert!(suggestions.is_empty());
    
    // Test cache clearing
    service.clear_cache().await;
    
    // Verify cache is cleared
    let (units_count, chunks_count, _) = service.get_cache_stats().await;
    assert_eq!(units_count, 0);
    assert_eq!(chunks_count, 0);
}