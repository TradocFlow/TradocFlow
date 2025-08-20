//! Parquet file manager for long-term storage with thread-safe async operations

use crate::error::{Result, TranslationMemoryError};
use crate::models::{TranslationUnit, Terminology, Language};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

/// Parquet file metadata for tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParquetFileMetadata {
    pub file_path: PathBuf,
    pub file_type: ParquetFileType,
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub record_count: u64,
    pub file_size_bytes: u64,
    pub compression_type: String,
}

/// Types of Parquet files managed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ParquetFileType {
    TranslationUnits,
    Terminology,
    Chunks,
    Metadata,
}

/// Parquet file organization structure
#[derive(Debug)]
struct ParquetFileStructure {
    project_files: HashMap<Uuid, Vec<ParquetFileMetadata>>,
    #[allow(dead_code)]
    active_writers: HashMap<String, DateTime<Utc>>,
    compression_stats: HashMap<ParquetFileType, CompressionStats>,
}

/// Compression statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct CompressionStats {
    pub total_files: u64,
    pub total_original_size: u64,
    pub total_compressed_size: u64,
    pub average_compression_ratio: f32,
}

impl Default for ParquetFileStructure {
    fn default() -> Self {
        Self {
            project_files: HashMap::new(),
            active_writers: HashMap::new(),
            compression_stats: HashMap::new(),
        }
    }
}

/// Manager for Parquet file operations with async support
#[derive(Debug)]
pub struct ParquetManager {
    base_path: PathBuf,
    file_structure: Arc<RwLock<ParquetFileStructure>>,
    default_compression: String,
}

impl ParquetManager {
    /// Create a new Parquet manager
    pub async fn new(base_path: &str) -> Result<Arc<Self>> {
        let base_path = PathBuf::from(base_path);
        
        // Create base directory if it doesn't exist
        tokio::fs::create_dir_all(&base_path).await
            .map_err(|e| TranslationMemoryError::StorageError(format!("Failed to create base directory: {}", e)))?;
        
        let manager = Arc::new(Self {
            base_path,
            file_structure: Arc::new(RwLock::new(ParquetFileStructure::default())),
            default_compression: "snappy".to_string(),
        });
        
        // Initialize file structure
        manager.scan_existing_files().await?;
        
        Ok(manager)
    }
    
    /// Create project-specific Parquet files structure
    pub async fn create_project_files(&self, project_id: Uuid) -> Result<()> {
        let project_dir = self.base_path.join(format!("project_{}", project_id));
        tokio::fs::create_dir_all(&project_dir).await
            .map_err(|e| TranslationMemoryError::StorageError(format!("Failed to create project directory: {}", e)))?;
        
        // Create subdirectories for different data types
        let subdirs = ["translation_units", "terminology", "chunks", "metadata"];
        for subdir in &subdirs {
            let subdir_path = project_dir.join(subdir);
            tokio::fs::create_dir_all(&subdir_path).await
                .map_err(|e| TranslationMemoryError::StorageError(format!("Failed to create subdirectory {}: {}", subdir, e)))?;
        }
        
        log::info!("Created Parquet file structure for project: {}", project_id);
        Ok(())
    }
    
    /// Append a translation unit to Parquet storage
    pub async fn append_translation_unit(&self, unit: &TranslationUnit) -> Result<()> {
        let file_path = self.get_translation_units_file_path(unit.project_id, Some(unit.target_language.clone())).await?;
        
        // Mock append operation - in real implementation, this would use Arrow/Parquet libraries
        log::debug!("Appending translation unit {} to Parquet file: {:?}", unit.id, file_path);
        
        // Simulate file write operation
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        
        // Update metadata
        self.update_file_metadata(file_path, ParquetFileType::TranslationUnits, 1).await?;
        
        Ok(())
    }
    
    /// Append multiple translation units in batch
    pub async fn append_translation_units_batch(&self, units: &[TranslationUnit]) -> Result<()> {
        if units.is_empty() {
            return Ok(());
        }
        
        let project_id = units[0].project_id;
        let target_language = units[0].target_language.clone();
        
        let file_path = self.get_translation_units_file_path(project_id, Some(target_language)).await?;
        
        log::debug!("Batch appending {} translation units to Parquet file: {:?}", units.len(), file_path);
        
        // Mock batch append - simulate processing time based on batch size
        tokio::time::sleep(tokio::time::Duration::from_millis(units.len() as u64)).await;
        
        // Update metadata
        self.update_file_metadata(file_path, ParquetFileType::TranslationUnits, units.len() as u64).await?;
        
        Ok(())
    }
    
    /// Update an existing translation unit in Parquet storage
    pub async fn update_translation_unit(&self, unit: &TranslationUnit) -> Result<()> {
        let file_path = self.get_translation_units_file_path(unit.project_id, Some(unit.target_language.clone())).await?;
        
        log::debug!("Updating translation unit {} in Parquet file: {:?}", unit.id, file_path);
        
        // Mock update operation - in real implementation, this might involve reading, modifying, and rewriting
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        
        Ok(())
    }
    
    /// Delete a translation unit from Parquet storage
    pub async fn delete_translation_unit(&self, id: Uuid) -> Result<()> {
        log::debug!("Deleting translation unit {} from Parquet storage", id);
        
        // Mock deletion - in real implementation, this would involve rewriting files without the deleted record
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        Ok(())
    }
    
    /// Convert terminology entries to Parquet format
    pub async fn convert_terms_to_parquet(&self, terms: &[Terminology], project_id: Uuid) -> Result<()> {
        let file_path = self.get_terminology_file_path(project_id).await?;
        
        log::debug!("Converting {} terms to Parquet format for project: {}", terms.len(), project_id);
        
        // Mock conversion operation
        tokio::time::sleep(tokio::time::Duration::from_millis(terms.len() as u64 * 2)).await;
        
        // Update metadata
        self.update_file_metadata(file_path, ParquetFileType::Terminology, terms.len() as u64).await?;
        
        Ok(())
    }
    
    /// Update terminology in Parquet storage
    pub async fn update_terminology(&self, terminology: &Terminology, project_id: Uuid) -> Result<()> {
        let file_path = self.get_terminology_file_path(project_id).await?;
        
        log::debug!("Updating terminology '{}' in Parquet file: {:?}", terminology.term, file_path);
        
        // Mock update operation
        tokio::time::sleep(tokio::time::Duration::from_millis(3)).await;
        
        Ok(())
    }
    
    /// Delete terminology from Parquet storage
    pub async fn delete_terminology(&self, id: Uuid, project_id: Uuid) -> Result<()> {
        log::debug!("Deleting terminology {} from Parquet storage for project: {}", id, project_id);
        
        // Mock deletion
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        
        Ok(())
    }
    
    /// Refresh Parquet files for updated terminology
    pub async fn refresh_parquet_files(&self, project_id: Uuid, terms: &[Terminology]) -> Result<()> {
        log::debug!("Refreshing Parquet files for project: {} with {} terms", project_id, terms.len());
        
        // Mock refresh operation - in real implementation, this would rebuild optimized Parquet files
        tokio::time::sleep(tokio::time::Duration::from_millis(terms.len() as u64 * 3)).await;
        
        Ok(())
    }
    
    /// Export data to Parquet format (generic method)
    pub async fn export_to_parquet(&self, data: &[u8], filename: &str) -> Result<()> {
        let file_path = self.base_path.join(filename);
        
        log::debug!("Exporting {} bytes to Parquet file: {:?}", data.len(), file_path);
        
        // Mock export - in real implementation, would serialize data to Parquet format
        tokio::fs::write(&file_path, data).await
            .map_err(|e| TranslationMemoryError::StorageError(format!("Failed to write Parquet file: {}", e)))?;
        
        Ok(())
    }
    
    /// Import data from Parquet format (generic method)
    pub async fn import_from_parquet(&self, filename: &str) -> Result<Vec<u8>> {
        let file_path = self.base_path.join(filename);
        
        log::debug!("Importing from Parquet file: {:?}", file_path);
        
        // Mock import - in real implementation, would deserialize Parquet data
        let data = tokio::fs::read(&file_path).await
            .map_err(|e| TranslationMemoryError::StorageError(format!("Failed to read Parquet file: {}", e)))?;
        
        Ok(data)
    }
    
    /// Compact and optimize Parquet files for a project
    pub async fn optimize_project_files(&self, project_id: Uuid) -> Result<()> {
        log::info!("Optimizing Parquet files for project: {}", project_id);
        
        // Mock optimization - in real implementation, would:
        // 1. Read all files for the project
        // 2. Merge small files
        // 3. Recompress with optimal settings
        // 4. Update indexes
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        Ok(())
    }
    
    /// Get compression statistics for monitoring
    pub async fn get_compression_stats(&self, file_type: Option<ParquetFileType>) -> Result<CompressionStats> {
        let file_structure = self.file_structure.read().await;
        
        if let Some(file_type) = file_type {
            Ok(file_structure.compression_stats.get(&file_type).cloned().unwrap_or_default())
        } else {
            // Calculate overall stats
            let mut overall_stats = CompressionStats::default();
            for stats in file_structure.compression_stats.values() {
                overall_stats.total_files += stats.total_files;
                overall_stats.total_original_size += stats.total_original_size;
                overall_stats.total_compressed_size += stats.total_compressed_size;
            }
            
            if overall_stats.total_original_size > 0 {
                overall_stats.average_compression_ratio = 
                    overall_stats.total_compressed_size as f32 / overall_stats.total_original_size as f32;
            }
            
            Ok(overall_stats)
        }
    }
    
    /// Get file metadata for a project
    pub async fn get_project_file_metadata(&self, project_id: Uuid) -> Result<Vec<ParquetFileMetadata>> {
        let file_structure = self.file_structure.read().await;
        Ok(file_structure.project_files.get(&project_id).cloned().unwrap_or_default())
    }
    
    /// Clean up old or orphaned Parquet files
    pub async fn cleanup_old_files(&self, max_age_days: u32) -> Result<usize> {
        let cutoff_date = Utc::now() - chrono::Duration::days(max_age_days as i64);
        let mut cleaned_count = 0;
        
        log::info!("Cleaning up Parquet files older than {} days", max_age_days);
        
        let mut file_structure = self.file_structure.write().await;
        
        for (_project_id, files) in file_structure.project_files.iter_mut() {
            files.retain(|file| {
                if file.last_modified < cutoff_date {
                    log::debug!("Marking file for cleanup: {:?}", file.file_path);
                    cleaned_count += 1;
                    false
                } else {
                    true
                }
            });
        }
        
        log::info!("Cleaned up {} old Parquet files", cleaned_count);
        Ok(cleaned_count)
    }
    
    /// Get base path
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
    
    /// Get storage usage statistics
    pub async fn get_storage_stats(&self) -> Result<StorageStats> {
        let file_structure = self.file_structure.read().await;
        
        let mut total_files = 0;
        let mut total_size = 0;
        let mut project_count = 0;
        
        for files in file_structure.project_files.values() {
            project_count += 1;
            total_files += files.len();
            total_size += files.iter().map(|f| f.file_size_bytes).sum::<u64>();
        }
        
        Ok(StorageStats {
            total_files,
            total_size_bytes: total_size,
            project_count,
            compression_ratio: if total_size > 0 {
                file_structure.compression_stats.values()
                    .map(|s| s.average_compression_ratio)
                    .sum::<f32>() / file_structure.compression_stats.len() as f32
            } else {
                0.0
            },
            last_optimization: Utc::now(), // Mock value
        })
    }
    
    // Private helper methods
    
    async fn get_translation_units_file_path(&self, project_id: Uuid, language: Option<Language>) -> Result<PathBuf> {
        let project_dir = self.base_path.join(format!("project_{}", project_id)).join("translation_units");
        
        let filename = if let Some(lang) = language {
            format!("units_{}.parquet", lang)
        } else {
            "units_all.parquet".to_string()
        };
        
        Ok(project_dir.join(filename))
    }
    
    async fn get_terminology_file_path(&self, project_id: Uuid) -> Result<PathBuf> {
        let project_dir = self.base_path.join(format!("project_{}", project_id)).join("terminology");
        Ok(project_dir.join("terms.parquet"))
    }
    
    #[allow(dead_code)]
    async fn get_chunks_file_path(&self, project_id: Uuid) -> Result<PathBuf> {
        let project_dir = self.base_path.join(format!("project_{}", project_id)).join("chunks");
        Ok(project_dir.join("chunks.parquet"))
    }
    
    async fn update_file_metadata(
        &self, 
        file_path: PathBuf, 
        file_type: ParquetFileType, 
        record_count: u64
    ) -> Result<()> {
        let mut file_structure = self.file_structure.write().await;
        
        // Extract project ID from file path
        let project_id = self.extract_project_id_from_path(&file_path)?;
        
        // Update or create metadata
        let metadata = ParquetFileMetadata {
            file_path: file_path.clone(),
            file_type: file_type.clone(),
            created_at: Utc::now(),
            last_modified: Utc::now(),
            record_count,
            file_size_bytes: 1024 * record_count, // Mock file size
            compression_type: self.default_compression.clone(),
        };
        
        let project_files = file_structure.project_files.entry(project_id).or_insert_with(Vec::new);
        
        // Update existing or add new
        if let Some(existing) = project_files.iter_mut().find(|f| f.file_path == file_path) {
            existing.last_modified = metadata.last_modified;
            existing.record_count += record_count;
            existing.file_size_bytes = 1024 * existing.record_count;
        } else {
            project_files.push(metadata);
        }
        
        // Update compression stats
        let stats = file_structure.compression_stats.entry(file_type).or_insert_with(CompressionStats::default);
        stats.total_files += 1;
        stats.total_original_size += record_count * 2048; // Mock original size
        stats.total_compressed_size += record_count * 1024; // Mock compressed size
        stats.average_compression_ratio = stats.total_compressed_size as f32 / stats.total_original_size as f32;
        
        Ok(())
    }
    
    fn extract_project_id_from_path(&self, file_path: &Path) -> Result<Uuid> {
        // Extract project ID from path like "base/project_{uuid}/..."
        let path_str = file_path.to_string_lossy();
        if let Some(start) = path_str.find("project_") {
            let id_start = start + "project_".len();
            if let Some(end) = path_str[id_start..].find('/') {
                let id_str = &path_str[id_start..id_start + end];
                return Uuid::parse_str(id_str)
                    .map_err(|e| TranslationMemoryError::ValidationError(format!("Invalid project ID in path: {}", e)).into());
            }
        }
        
        Err(TranslationMemoryError::ValidationError("Could not extract project ID from file path".to_string()).into())
    }
    
    async fn scan_existing_files(&self) -> Result<()> {
        // Mock implementation - in real version would scan directory structure
        log::debug!("Scanning existing Parquet files in: {:?}", self.base_path);
        
        // Simulate scanning time
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        
        Ok(())
    }
}

/// Storage statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub project_count: usize,
    pub compression_ratio: f32,
    pub last_optimization: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::models::Language;
    
    #[tokio::test]
    async fn test_parquet_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let manager = ParquetManager::new(temp_dir.path().to_str().unwrap()).await.unwrap();
        
        assert_eq!(manager.base_path(), temp_dir.path());
    }
    
    #[tokio::test]
    async fn test_create_project_files() {
        let temp_dir = tempdir().unwrap();
        let manager = ParquetManager::new(temp_dir.path().to_str().unwrap()).await.unwrap();
        let project_id = Uuid::new_v4();
        
        let result = manager.create_project_files(project_id).await;
        assert!(result.is_ok());
        
        // Check that directories were created
        let project_dir = temp_dir.path().join(format!("project_{}", project_id));
        assert!(project_dir.exists());
        assert!(project_dir.join("translation_units").exists());
        assert!(project_dir.join("terminology").exists());
        assert!(project_dir.join("chunks").exists());
        assert!(project_dir.join("metadata").exists());
    }
    
    #[tokio::test]
    async fn test_translation_unit_operations() {
        let temp_dir = tempdir().unwrap();
        let manager = ParquetManager::new(temp_dir.path().to_str().unwrap()).await.unwrap();
        let project_id = Uuid::new_v4();
        
        manager.create_project_files(project_id).await.unwrap();
        
        // Create mock translation unit using the builder pattern
        let unit = crate::models::TranslationUnitBuilder::new()
            .project_id(project_id)
            .chapter_id(Uuid::new_v4())
            .chunk_id(Uuid::new_v4())
            .source_language_enum(Language::English)
            .source_text("Hello world")
            .target_language_enum(Language::Spanish)
            .target_text("Hola mundo")
            .confidence_score(0.95)
            .build()
            .unwrap();
        
        // Test append
        let result = manager.append_translation_unit(&unit).await;
        assert!(result.is_ok());
        
        // Test batch append
        let units = vec![unit.clone(), unit.clone()];
        let result = manager.append_translation_units_batch(&units).await;
        assert!(result.is_ok());
        
        // Test update
        let result = manager.update_translation_unit(&unit).await;
        assert!(result.is_ok());
        
        // Test delete
        let result = manager.delete_translation_unit(unit.id).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_terminology_operations() {
        let temp_dir = tempdir().unwrap();
        let manager = ParquetManager::new(temp_dir.path().to_str().unwrap()).await.unwrap();
        let project_id = Uuid::new_v4();
        
        manager.create_project_files(project_id).await.unwrap();
        
        // Create mock terminology
        let terms = vec![
            Terminology {
                id: Uuid::new_v4(),
                term: "API".to_string(),
                definition: Some("Application Programming Interface".to_string()),
                do_not_translate: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        ];
        
        // Test convert terms to parquet
        let result = manager.convert_terms_to_parquet(&terms, project_id).await;
        assert!(result.is_ok());
        
        // Test update terminology
        let result = manager.update_terminology(&terms[0], project_id).await;
        assert!(result.is_ok());
        
        // Test delete terminology
        let result = manager.delete_terminology(terms[0].id, project_id).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_storage_stats() {
        let temp_dir = tempdir().unwrap();
        let manager = ParquetManager::new(temp_dir.path().to_str().unwrap()).await.unwrap();
        
        let stats = manager.get_storage_stats().await.unwrap();
        assert_eq!(stats.total_files, 0);
        assert_eq!(stats.total_size_bytes, 0);
        assert_eq!(stats.project_count, 0);
    }
    
    #[tokio::test]
    async fn test_compression_stats() {
        let temp_dir = tempdir().unwrap();
        let manager = ParquetManager::new(temp_dir.path().to_str().unwrap()).await.unwrap();
        
        let stats = manager.get_compression_stats(Some(ParquetFileType::TranslationUnits)).await.unwrap();
        assert_eq!(stats.total_files, 0);
        
        let overall_stats = manager.get_compression_stats(None).await.unwrap();
        assert_eq!(overall_stats.total_files, 0);
    }
    
    #[tokio::test]
    async fn test_file_cleanup() {
        let temp_dir = tempdir().unwrap();
        let manager = ParquetManager::new(temp_dir.path().to_str().unwrap()).await.unwrap();
        
        let cleaned_count = manager.cleanup_old_files(30).await.unwrap();
        assert_eq!(cleaned_count, 0); // No old files to clean
    }
    
    #[tokio::test]
    async fn test_project_optimization() {
        let temp_dir = tempdir().unwrap();
        let manager = ParquetManager::new(temp_dir.path().to_str().unwrap()).await.unwrap();
        let project_id = Uuid::new_v4();
        
        let result = manager.optimize_project_files(project_id).await;
        assert!(result.is_ok());
    }
}