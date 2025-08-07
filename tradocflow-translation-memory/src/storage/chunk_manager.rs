//! Chunk storage manager with DuckDB backend integration
//! 
//! This manager coordinates chunk storage operations using DuckDB for persistence
//! and provides chunk-specific functionality like linking and relationship management.

use crate::error::Result;
use crate::models::ChunkMetadata as Chunk;
use crate::services::translation_memory::ChunkLinkType;
use crate::storage::traits::{ChunkStorage, ChunkStorageStats};
use crate::storage::DuckDBManager;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

/// Manager for chunk storage operations with DuckDB backend
#[derive(Debug)]
pub struct ChunkManager {
    /// DuckDB manager for persistent storage
    duckdb_manager: Arc<DuckDBManager>,
}

impl ChunkManager {
    /// Create a new chunk manager with DuckDB backend
    pub async fn new(duckdb_manager: Arc<DuckDBManager>) -> Result<Self> {
        Ok(Self {
            duckdb_manager,
        })
    }
    
    /// Create a new chunk manager without DuckDB (for backwards compatibility)
    pub async fn new_standalone() -> Result<Self> {
        // Create a temporary DuckDB manager for standalone usage
        let temp_path = std::env::temp_dir().join("chunk_manager_standalone.db");
        let duckdb_manager = DuckDBManager::new(&temp_path, Some(5)).await?;
        
        Ok(Self {
            duckdb_manager,
        })
    }
    
    /// Initialize the chunk storage schema
    pub async fn initialize_schema(&self) -> Result<()> {
        // Delegate to DuckDB manager's chunk schema initialization
        // (this is already part of the translation memory schema)
        self.duckdb_manager.initialize_schema().await
    }
    
    /// Get the underlying DuckDB manager (for advanced operations)
    pub fn duckdb_manager(&self) -> &Arc<DuckDBManager> {
        &self.duckdb_manager
    }
}

#[async_trait]
impl ChunkStorage for ChunkManager {
    async fn initialize_schema(&self) -> Result<()> {
        self.initialize_schema().await
    }
    
    async fn store_chunk(&self, chunk: &Chunk) -> Result<()> {
        ChunkStorage::store_chunk(self.duckdb_manager.as_ref(), chunk).await
    }
    
    async fn get_chunk(&self, id: Uuid) -> Result<Option<Chunk>> {
        ChunkStorage::get_chunk(self.duckdb_manager.as_ref(), id).await
    }
    
    async fn update_chunk(&self, chunk: &Chunk) -> Result<()> {
        ChunkStorage::update_chunk(self.duckdb_manager.as_ref(), chunk).await
    }
    
    async fn delete_chunk(&self, id: Uuid) -> Result<()> {
        ChunkStorage::delete_chunk(self.duckdb_manager.as_ref(), id).await
    }
    
    async fn get_chunks_by_chapter(&self, chapter_id: Uuid) -> Result<Vec<Chunk>> {
        ChunkStorage::get_chunks_by_chapter(self.duckdb_manager.as_ref(), chapter_id).await
    }
    
    async fn store_chunks_batch(&self, chunks: &[Chunk]) -> Result<usize> {
        ChunkStorage::store_chunks_batch(self.duckdb_manager.as_ref(), chunks).await
    }
    
    async fn link_chunks(
        &self,
        chunk_ids: Vec<Uuid>,
        link_type: ChunkLinkType,
    ) -> Result<()> {
        ChunkStorage::link_chunks(self.duckdb_manager.as_ref(), chunk_ids, link_type).await
    }
    
    async fn get_linked_chunks(
        &self,
        chunk_id: Uuid,
        link_type: Option<ChunkLinkType>,
    ) -> Result<Vec<Chunk>> {
        ChunkStorage::get_linked_chunks(self.duckdb_manager.as_ref(), chunk_id, link_type).await
    }
    
    async fn count_chunks(&self, chapter_id: Uuid) -> Result<u64> {
        ChunkStorage::count_chunks(self.duckdb_manager.as_ref(), chapter_id).await
    }
    
    async fn get_storage_stats(&self) -> Result<ChunkStorageStats> {
        ChunkStorage::get_storage_stats(self.duckdb_manager.as_ref()).await
    }
}

// Additional convenience methods for chunk management
impl ChunkManager {
    /// Add multiple chunks in a batch operation (legacy method name)
    pub async fn add_chunks_batch(&self, chunks: Vec<Chunk>) -> Result<usize> {
        self.store_chunks_batch(&chunks).await
    }
    
    /// Get all chunks for a specific chapter with optional filtering
    pub async fn get_chapter_chunks_filtered(
        &self,
        chapter_id: Uuid,
        chunk_type: Option<crate::models::ChunkType>,
    ) -> Result<Vec<Chunk>> {
        let chunks = self.get_chunks_by_chapter(chapter_id).await?;
        
        // Filter by chunk type if specified
        if let Some(_filter_type) = chunk_type {
            Ok(chunks.into_iter().filter(|_chunk| {
                // Assume chunks have a chunk_type field for filtering
                // This would be based on the actual Chunk model implementation
                true // Placeholder - implement based on actual Chunk model
            }).collect())
        } else {
            Ok(chunks)
        }
    }
    
    /// Get chunks that are linked to a specific chunk
    pub async fn get_related_chunks(
        &self,
        chunk_id: Uuid,
        max_depth: u32,
    ) -> Result<Vec<Chunk>> {
        let mut related_chunks = Vec::new();
        let mut processed_ids = std::collections::HashSet::new();
        let mut current_level = vec![chunk_id];
        
        for _ in 0..max_depth {
            if current_level.is_empty() {
                break;
            }
            
            let mut next_level = Vec::new();
            
            for chunk_id in current_level {
                if processed_ids.contains(&chunk_id) {
                    continue;
                }
                
                processed_ids.insert(chunk_id);
                
                // Get all linked chunks (regardless of link type)
                let linked = self.get_linked_chunks(chunk_id, None).await?;
                
                for linked_chunk in linked {
                    if !processed_ids.contains(&linked_chunk.id) {
                        next_level.push(linked_chunk.id);
                        related_chunks.push(linked_chunk);
                    }
                }
            }
            
            current_level = next_level;
        }
        
        Ok(related_chunks)
    }
    
    /// Link chunks in a sequence (preserving order)
    pub async fn link_chunks_sequentially(&self, chunk_ids: Vec<Uuid>) -> Result<()> {
        if chunk_ids.len() < 2 {
            return Ok(());
        }
        
        // Link each consecutive pair
        for window in chunk_ids.windows(2) {
            self.link_chunks(vec![window[0], window[1]], ChunkLinkType::LinkedPhrase).await?;
        }
        
        Ok(())
    }
    
    /// Unlink all chunks in a group
    pub async fn unlink_chunk_group(&self, chunk_ids: Vec<Uuid>) -> Result<()> {
        self.link_chunks(chunk_ids, ChunkLinkType::Unlinked).await
    }
    
    /// Get comprehensive chunk statistics for monitoring
    pub async fn get_comprehensive_stats(&self) -> Result<ChunkManagementStats> {
        let storage_stats = self.get_storage_stats().await?;
        
        // Get additional statistics specific to chunk management
        Ok(ChunkManagementStats {
            storage_stats,
            linked_chunk_groups: 0, // TODO: Implement actual counting
            orphaned_chunks: 0,     // TODO: Implement actual counting
            average_links_per_chunk: 0.0, // TODO: Implement actual calculation
        })
    }
}

/// Extended statistics for chunk management operations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChunkManagementStats {
    /// Base storage statistics
    pub storage_stats: ChunkStorageStats,
    /// Number of linked chunk groups
    pub linked_chunk_groups: u64,
    /// Number of orphaned chunks (no links)
    pub orphaned_chunks: u64,
    /// Average number of links per chunk
    pub average_links_per_chunk: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_chunk_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let duckdb_manager = DuckDBManager::new(&db_path, Some(5)).await.unwrap();
        let chunk_manager = ChunkManager::new(duckdb_manager).await.unwrap();
        
        // Test schema initialization
        let result = chunk_manager.initialize_schema().await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_standalone_chunk_manager() {
        let chunk_manager = ChunkManager::new_standalone().await.unwrap();
        
        // Test schema initialization
        let result = chunk_manager.initialize_schema().await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_chunk_storage_stats() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let duckdb_manager = DuckDBManager::new(&db_path, Some(5)).await.unwrap();
        let chunk_manager = ChunkManager::new(duckdb_manager).await.unwrap();
        
        let stats = chunk_manager.get_storage_stats().await.unwrap();
        assert!(stats.total_chunks >= 0);
        assert!(stats.unique_chapters >= 0);
        assert!(stats.average_chunk_size >= 0.0);
    }
}