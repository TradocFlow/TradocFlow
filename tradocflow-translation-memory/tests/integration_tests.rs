//! Integration tests for translation memory crate

use tradocflow_translation_memory::*;
use tempfile::TempDir;

#[tokio::test]
async fn test_basic_integration() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_tm.db");
    
    let tm = TradocFlowTranslationMemory::new(db_path.to_str().unwrap()).await.unwrap();
    tm.initialize().await.unwrap();
    
    // Test basic functionality
    assert!(tm.translation_memory().search("test", Language::English, Language::Spanish, 0.7).await.unwrap().is_empty());
}

#[tokio::test]
async fn test_comprehensive_search() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_tm.db");
    
    let tm = TradocFlowTranslationMemory::new(db_path.to_str().unwrap()).await.unwrap();
    tm.initialize().await.unwrap();
    
    let result = tm.comprehensive_search("hello", Language::English, Language::Spanish).await.unwrap();
    assert!(result.translation_matches.is_empty());
    assert!(result.terminology_matches.is_empty());
}