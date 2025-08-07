//! Storage trait abstractions for translation memory and terminology
//! 
//! This module defines the core storage interfaces that can be implemented
//! by different storage backends (DuckDB, Parquet, etc.)

use crate::error::Result;
use crate::models::{TranslationUnit, Terminology, Chunk};
use crate::services::translation_memory::{TranslationMatch, LanguagePair, ChunkLinkType};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Core trait for translation memory storage operations
/// 
/// This trait defines the essential operations needed for storing and retrieving
/// translation units across different storage backends.
#[async_trait]
pub trait TranslationMemoryStorage: Send + Sync {
    /// Initialize the storage schema and perform any necessary setup
    async fn initialize_schema(&self) -> Result<()>;
    
    /// Insert a single translation unit
    async fn insert_translation_unit(&self, unit: &TranslationUnit) -> Result<()>;
    
    /// Insert multiple translation units in a batch operation
    async fn insert_translation_units_batch(&self, units: &[TranslationUnit]) -> Result<usize>;
    
    /// Update an existing translation unit
    async fn update_translation_unit(&self, unit: &TranslationUnit) -> Result<()>;
    
    /// Delete a translation unit by ID
    async fn delete_translation_unit(&self, id: Uuid) -> Result<bool>;
    
    /// Search for exact matches in the translation memory
    async fn search_exact_matches(
        &self,
        source_text: &str,
        language_pair: &LanguagePair,
    ) -> Result<Vec<TranslationMatch>>;
    
    /// Search for fuzzy matches with similarity threshold
    async fn search_fuzzy_matches(
        &self,
        source_text: &str,
        language_pair: &LanguagePair,
        threshold: f32,
    ) -> Result<Vec<TranslationMatch>>;
    
    /// Search for n-gram based matches
    async fn search_ngram_matches(
        &self,
        source_text: &str,
        language_pair: &LanguagePair,
        threshold: f32,
    ) -> Result<Vec<TranslationMatch>>;
    
    /// Get translation units by project ID with optional filtering
    async fn get_translation_units_by_project(
        &self,
        project_id: Uuid,
        language_pair: Option<&LanguagePair>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<TranslationUnit>>;
    
    /// Count total translation units for a project
    async fn count_translation_units(&self, project_id: Uuid) -> Result<u64>;
    
    /// Optimize storage performance (e.g., rebuild indexes, vacuum, etc.)
    async fn optimize_storage(&self) -> Result<()>;
    
    /// Get storage statistics for monitoring
    async fn get_storage_stats(&self) -> Result<TranslationMemoryStorageStats>;
}

/// Core trait for terminology storage operations
/// 
/// This trait defines operations for managing terminology entries
/// across different storage backends.
#[async_trait]
pub trait TerminologyStorage: Send + Sync {
    /// Initialize the terminology storage schema
    async fn initialize_schema(&self) -> Result<()>;
    
    /// Insert a terminology entry
    async fn insert_terminology(&self, terminology: &Terminology, project_id: Uuid) -> Result<()>;
    
    /// Update an existing terminology entry
    async fn update_terminology(&self, terminology: &Terminology, project_id: Uuid) -> Result<bool>;
    
    /// Delete a terminology entry
    async fn delete_terminology(&self, id: Uuid, project_id: Uuid) -> Result<bool>;
    
    /// Get all terminology entries for a project
    async fn get_terms_by_project(&self, project_id: Uuid) -> Result<Vec<Terminology>>;
    
    /// Search terminology entries with text matching
    async fn search_terms(
        &self,
        query: &str,
        project_id: Uuid,
        case_sensitive: bool,
    ) -> Result<Vec<Terminology>>;
    
    /// Update multiple terminology entries in batch
    async fn update_terminology_batch(
        &self,
        project_id: Uuid,
        terms: &[Terminology],
    ) -> Result<usize>;
    
    /// Count terminology entries for a project
    async fn count_terms(&self, project_id: Uuid) -> Result<u64>;
    
    /// Get terminology by term text (for lookup operations)
    async fn get_term_by_text(
        &self,
        term_text: &str,
        project_id: Uuid,
    ) -> Result<Option<Terminology>>;
    
    /// Check if a term exists in the project
    async fn term_exists(
        &self,
        term_text: &str,
        project_id: Uuid,
    ) -> Result<bool>;
    
    /// Get storage statistics for terminology
    async fn get_storage_stats(&self) -> Result<TerminologyStorageStats>;
}

/// Core trait for chunk storage operations
/// 
/// This trait defines operations for managing document chunks
/// and their relationships.
#[async_trait]
pub trait ChunkStorage: Send + Sync {
    /// Initialize the chunk storage schema
    async fn initialize_schema(&self) -> Result<()>;
    
    /// Store a chunk
    async fn store_chunk(&self, chunk: &Chunk) -> Result<()>;
    
    /// Retrieve a chunk by ID
    async fn get_chunk(&self, id: Uuid) -> Result<Option<Chunk>>;
    
    /// Update a chunk
    async fn update_chunk(&self, chunk: &Chunk) -> Result<()>;
    
    /// Delete a chunk
    async fn delete_chunk(&self, id: Uuid) -> Result<()>;
    
    /// Get chunks by chapter ID
    async fn get_chunks_by_chapter(&self, chapter_id: Uuid) -> Result<Vec<Chunk>>;
    
    /// Store multiple chunks in batch
    async fn store_chunks_batch(&self, chunks: &[Chunk]) -> Result<usize>;
    
    /// Link chunks together (for maintaining relationships)
    async fn link_chunks(
        &self,
        chunk_ids: Vec<Uuid>,
        link_type: ChunkLinkType,
    ) -> Result<()>;
    
    /// Get linked chunks for a given chunk
    async fn get_linked_chunks(
        &self,
        chunk_id: Uuid,
        link_type: Option<ChunkLinkType>,
    ) -> Result<Vec<Chunk>>;
    
    /// Count chunks for a chapter
    async fn count_chunks(&self, chapter_id: Uuid) -> Result<u64>;
    
    /// Get storage statistics for chunks
    async fn get_storage_stats(&self) -> Result<ChunkStorageStats>;
}

// ChunkLinkType is imported from crate::services::translation_memory

/// Storage statistics for translation memory
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranslationMemoryStorageStats {
    pub total_translation_units: u64,
    pub unique_language_pairs: u64,
    pub average_confidence_score: f32,
    pub storage_size_bytes: u64,
    pub last_updated: DateTime<Utc>,
    pub optimization_recommended: bool,
}

/// Storage statistics for terminology
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TerminologyStorageStats {
    pub total_terms: u64,
    pub terms_with_definitions: u64,
    pub do_not_translate_count: u64,
    pub storage_size_bytes: u64,
    pub last_updated: DateTime<Utc>,
}

/// Storage statistics for chunks
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChunkStorageStats {
    pub total_chunks: u64,
    pub unique_chapters: u64,
    pub average_chunk_size: f32,
    pub total_links: u64,
    pub storage_size_bytes: u64,
    pub last_updated: DateTime<Utc>,
}

/// Unified storage provider that combines all storage interfaces
/// 
/// This trait allows for implementations that provide all storage
/// capabilities in a single backend.
#[async_trait]
pub trait UnifiedStorageProvider: 
    TranslationMemoryStorage + TerminologyStorage + ChunkStorage 
{
    /// Initialize all schemas at once
    async fn initialize_all_schemas(&self) -> Result<()> {
        TranslationMemoryStorage::initialize_schema(self).await?;
        TerminologyStorage::initialize_schema(self).await?;
        ChunkStorage::initialize_schema(self).await?;
        Ok(())
    }
    
    /// Get comprehensive storage statistics
    async fn get_comprehensive_stats(&self) -> Result<ComprehensiveStorageStats>;
    
    /// Perform comprehensive optimization across all storage types
    async fn optimize_all_storage(&self) -> Result<()>;
    
    /// Execute a transaction across multiple storage operations
    async fn execute_transaction<F, R>(&self, transaction: F) -> Result<R>
    where
        F: Fn() -> Result<R> + Send,
        R: Send;
}

/// Comprehensive storage statistics across all storage types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComprehensiveStorageStats {
    pub translation_memory: TranslationMemoryStorageStats,
    pub terminology: TerminologyStorageStats,
    pub chunks: ChunkStorageStats,
    pub total_storage_size_bytes: u64,
    pub last_optimization: Option<DateTime<Utc>>,
    pub health_score: f32, // 0.0 to 1.0, where 1.0 is optimal
}

/// Storage configuration options
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageConfig {
    /// Maximum number of database connections in the pool
    pub max_connections: usize,
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,
    /// Enable automatic optimization
    pub auto_optimize: bool,
    /// Optimization interval in seconds
    pub optimization_interval_seconds: u64,
    /// Enable compression for storage
    pub enable_compression: bool,
    /// Batch size for bulk operations
    pub batch_size: usize,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            connection_timeout_seconds: 30,
            auto_optimize: true,
            optimization_interval_seconds: 3600, // 1 hour
            enable_compression: true,
            batch_size: 1000,
        }
    }
}

/// Factory trait for creating storage instances
/// 
/// This trait allows for different storage implementations to be
/// created with the same interface.
#[async_trait]
pub trait StorageFactory: Send + Sync {
    type Storage: UnifiedStorageProvider;
    
    /// Create a new storage instance with the given configuration
    async fn create_storage(
        &self,
        db_path: &str,
        config: StorageConfig,
    ) -> Result<Arc<Self::Storage>>;
    
    /// Check if the storage backend is available
    async fn is_available(&self) -> bool;
    
    /// Get the name of this storage backend
    fn backend_name(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_storage_config_defaults() {
        let config = StorageConfig::default();
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.connection_timeout_seconds, 30);
        assert!(config.auto_optimize);
        assert_eq!(config.optimization_interval_seconds, 3600);
        assert!(config.enable_compression);
        assert_eq!(config.batch_size, 1000);
    }
    
    #[test]
    fn test_chunk_link_type_serialization() {
        let link_types = vec![
            ChunkLinkType::LinkedPhrase,
            ChunkLinkType::Unlinked,
            ChunkLinkType::Merged,
        ];
        
        for link_type in link_types {
            let serialized = serde_json::to_string(&link_type).unwrap();
            let deserialized: ChunkLinkType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(link_type, deserialized);
        }
    }
}