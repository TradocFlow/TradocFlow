//! Demonstration of the storage abstraction layer
//! 
//! This example shows how to use the storage traits to interact with different
//! storage backends in a unified way.

use tradocflow_translation_memory::{
    storage::{
        DuckDBManager, ChunkManager,
        TranslationMemoryStorage, TerminologyStorage, ChunkStorage, UnifiedStorageProvider,
        StorageConfig,
    },
    models::{TranslationUnit, TranslationUnitBuilder, Terminology, Language, Chunk},
    services::translation_memory::LanguagePair,
    error::Result,
};
use std::sync::Arc;
use tempfile::tempdir;
use uuid::Uuid;
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 TradocFlow Translation Memory - Storage Abstraction Demo");
    println!("===========================================================");
    
    // Create a temporary directory for the database
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("demo.db");
    
    println!("📂 Creating database at: {}", db_path.display());
    
    // Create the storage backend (DuckDB)
    let storage = DuckDBManager::new(&db_path, Some(5)).await?;
    println!("✅ DuckDB storage backend created with connection pool");
    
    // Initialize the schema
    UnifiedStorageProvider::initialize_all_schemas(&*storage).await?;
    println!("✅ All schemas initialized (translation memory, terminology, chunks)");
    
    // Demonstrate translation memory operations
    println!("\n📖 Translation Memory Operations:");
    println!("================================");
    
    let project_id = Uuid::new_v4();
    let chapter_id = Uuid::new_v4();
    let chunk_id = Uuid::new_v4();
    
    // Create a sample translation unit using the builder pattern
    let translation_unit = TranslationUnitBuilder::new()
        .project_id(project_id)
        .chapter_id(chapter_id)
        .chunk_id(chunk_id)
        .source_language_enum(Language::English)
        .source_text("Hello, world!")
        .target_language_enum(Language::Spanish)
        .target_text("¡Hola, mundo!")
        .confidence_score(0.95)
        .context("Greeting")
        .build()?;
    
    // Insert translation unit using the trait
    TranslationMemoryStorage::insert_translation_unit(&*storage, &translation_unit).await?;
    println!("✅ Translation unit inserted: '{}' → '{}'", 
             translation_unit.source_text, translation_unit.target_text);
    
    // Search for exact matches
    let language_pair = LanguagePair::new(Language::English, Language::Spanish);
    let matches = TranslationMemoryStorage::search_exact_matches(
        &*storage, 
        "Hello, world!", 
        &language_pair
    ).await?;
    println!("🔍 Exact matches found: {}", matches.len());
    
    // Get translation memory statistics
    let tm_stats = TranslationMemoryStorage::get_storage_stats(&*storage).await?;
    println!("📊 Translation Memory Stats:");
    println!("   - Total units: {}", tm_stats.total_translation_units);
    println!("   - Language pairs: {}", tm_stats.unique_language_pairs);
    println!("   - Average confidence: {:.2}", tm_stats.average_confidence_score);
    println!("   - Storage size: {} bytes", tm_stats.storage_size_bytes);
    
    // Demonstrate terminology operations
    println!("\n📚 Terminology Operations:");
    println!("=========================");
    
    // Create sample terminology
    let term = Terminology {
        id: Uuid::new_v4(),
        term: "API".to_string(),
        definition: Some("Application Programming Interface".to_string()),
        do_not_translate: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    // Insert terminology using the trait
    TerminologyStorage::insert_terminology(&*storage, &term, project_id).await?;
    println!("✅ Terminology inserted: '{}' (do not translate: {})", 
             term.term, term.do_not_translate);
    
    // Search for terms
    let found_terms = TerminologyStorage::search_terms(
        &*storage, 
        "API", 
        project_id, 
        false
    ).await?;
    println!("🔍 Terms found: {}", found_terms.len());
    
    // Check if term exists
    let exists = TerminologyStorage::term_exists(
        &*storage, 
        "API", 
        project_id
    ).await?;
    println!("✅ Term 'API' exists: {}", exists);
    
    // Get terminology statistics
    let term_stats = TerminologyStorage::get_storage_stats(&*storage).await?;
    println!("📊 Terminology Stats:");
    println!("   - Total terms: {}", term_stats.total_terms);
    println!("   - With definitions: {}", term_stats.terms_with_definitions);
    println!("   - Do not translate: {}", term_stats.do_not_translate_count);
    
    // Demonstrate chunk operations
    println!("\n📄 Chunk Operations:");
    println!("====================");
    
    // Create a chunk manager
    let chunk_manager = ChunkManager::new(storage.clone()).await?;
    
    // Create sample chunks (using mock data structure)
    // Note: In a real implementation, you'd use proper Chunk model
    println!("📄 Chunk storage operations would be demonstrated here");
    println!("   (Using mock implementation for demo purposes)");
    
    // Get chunk statistics
    let chunk_stats = ChunkStorage::get_storage_stats(&*storage).await?;
    println!("📊 Chunk Stats:");
    println!("   - Total chunks: {}", chunk_stats.total_chunks);
    println!("   - Unique chapters: {}", chunk_stats.unique_chapters);
    println!("   - Average size: {:.1} chars", chunk_stats.average_chunk_size);
    
    // Demonstrate unified storage operations
    println!("\n🔧 Unified Storage Operations:");
    println!("==============================");
    
    // Get comprehensive statistics
    let comprehensive_stats = UnifiedStorageProvider::get_comprehensive_stats(&*storage).await?;
    println!("📊 Comprehensive Storage Stats:");
    println!("   - Total storage: {} bytes", comprehensive_stats.total_storage_size_bytes);
    println!("   - Health score: {:.2}", comprehensive_stats.health_score);
    println!("   - Last optimization: {:?}", comprehensive_stats.last_optimization);
    
    // Optimize all storage
    UnifiedStorageProvider::optimize_all_storage(&*storage).await?;
    println!("✅ Storage optimization completed");
    
    // Demonstrate transaction
    let transaction_result = UnifiedStorageProvider::execute_transaction(&*storage, || {
        // In a real implementation, this would perform multiple operations
        // within a database transaction
        Ok("Transaction completed successfully".to_string())
    }).await?;
    println!("✅ Transaction result: {}", transaction_result);
    
    println!("\n🎉 Storage abstraction demo completed successfully!");
    println!("✨ All storage operations work through unified trait interfaces");
    
    Ok(())
}