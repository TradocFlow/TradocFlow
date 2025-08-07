//! TradocFlow Translation Memory
//! 
//! A high-performance translation memory and terminology management system
//! built with Rust, DuckDB, and Parquet for efficient storage and retrieval
//! of translation units and terminology data.

pub mod error;
pub mod models;
pub mod services;
pub mod storage;
pub mod utils;

// Re-export key types for easier access
pub use error::{TranslationMemoryError, Result};
pub use models::{
    TranslationUnit, 
    TranslationUnitBuilder, 
    TranslationMetadata,
    TranslationMatch,
    TranslationSuggestion,
    MatchType,
    MatchScore,
    Term,
    TerminologyCsvRecord,
    TerminologyImportResult,
    TerminologyImportError,
    ConflictResolution,
    TerminologyValidationConfig,
    ChunkMetadata,
    ChunkBuilder,
    ChunkType,
    LanguagePair,
    ValidationError,
    TranslationStatus,
    Language,
    Domain,
    Quality,
    Metadata,
};
pub use services::{
    translation_memory::TranslationMemoryService,
    terminology::TerminologyService,
    highlighting::HighlightingService,
};
pub use storage::chunk_manager::ChunkManager;

#[cfg(feature = "duckdb-storage")]
pub use storage::duckdb_manager::DuckDBManager;

#[cfg(feature = "parquet-export")]
pub use storage::parquet_manager::ParquetManager;

/// The main translation memory facade providing a unified API
/// for all translation memory and terminology operations.
pub struct TradocFlowTranslationMemory {
    tm_service: TranslationMemoryService,
    terminology_service: TerminologyService,
    highlighting_service: HighlightingService,
}

impl TradocFlowTranslationMemory {
    /// Create a new instance with default configuration
    pub async fn new(db_path: &str) -> Result<Self> {
        use uuid::Uuid;
        use std::path::PathBuf;
        
        let project_id = Uuid::new_v4();
        let project_path = PathBuf::from(db_path).parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf();
            
        // Create services with thread-safe implementations
        let tm_service = TranslationMemoryService::new(project_id, project_path.clone()).await?;
        
        // Create CSV processor for terminology service
        let csv_processor = std::sync::Arc::new(crate::utils::CsvProcessor::new());
        let terminology_service = TerminologyService::new(csv_processor, None).await?;
        
        // Create highlighting service with Arc reference to terminology service
        let terminology_arc = std::sync::Arc::new(terminology_service);
        let highlighting_service = HighlightingService::new(
            terminology_arc.clone(),
            None
        ).await?;
        
        Ok(Self {
            tm_service,
            terminology_service: std::sync::Arc::try_unwrap(terminology_arc).unwrap_or_else(|arc| (*arc).clone()),
            highlighting_service,
        })
    }

    /// Create a new instance with default configuration
    /// Requires the `duckdb-storage` feature
    #[cfg(feature = "duckdb-storage")]
    pub async fn new_with_duckdb(db_path: &str) -> Result<Self> {
        Self::new_with_features(db_path, true).await
    }

    /// Create a new instance with configurable feature usage
    #[cfg(feature = "duckdb-storage")]
    async fn new_with_features(db_path: &str, _enable_parquet: bool) -> Result<Self> {
        use std::path::PathBuf;
        use uuid::Uuid;
        
        let project_id = Uuid::new_v4();
        let project_path = PathBuf::from(db_path).parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf();
        
        // Create services with correct constructors
        let tm_service = TranslationMemoryService::new(project_id, project_path.clone()).await?;
        
        // Create CSV processor for terminology service
        let csv_processor = std::sync::Arc::new(crate::utils::CsvProcessor::new());
        let terminology_service = TerminologyService::new(csv_processor, None).await?;
        
        // Create highlighting service with Arc reference to terminology service
        let terminology_arc = std::sync::Arc::new(terminology_service);
        let highlighting_service = HighlightingService::new(
            terminology_arc.clone(),
            None
        ).await?;
        
        Ok(Self {
            tm_service,
            terminology_service: std::sync::Arc::try_unwrap(terminology_arc).unwrap_or_else(|arc| (*arc).clone()),
            highlighting_service,
        })
    }
    
    /// Get the translation memory service
    pub fn translation_memory(&self) -> &TranslationMemoryService {
        &self.tm_service
    }
    
    /// Get the terminology service
    pub fn terminology(&self) -> &TerminologyService {
        &self.terminology_service
    }
    
    /// Get the highlighting service
    pub fn highlighting(&self) -> &HighlightingService {
        &self.highlighting_service
    }
    
    /// Perform a comprehensive search across both translation memory and terminology
    pub async fn comprehensive_search(&self, query: &str, source_lang: Language, target_lang: Language) -> Result<ComprehensiveSearchResult> {
        let tm_matches = self.tm_service.search(query, source_lang.clone(), target_lang.clone(), 0.7).await?;
        let terminology_matches = self.terminology_service.search_terms(query, source_lang, target_lang).await?;
        
        Ok(ComprehensiveSearchResult {
            translation_matches: tm_matches,
            terminology_matches,
        })
    }
    
    /// Initialize the database schema and perform any necessary migrations
    pub async fn initialize(&self) -> Result<()> {
        self.tm_service.initialize().await?;
        self.terminology_service.initialize().await?;
        Ok(())
    }
}

/// Result of a comprehensive search across translation memory and terminology
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComprehensiveSearchResult {
    pub translation_matches: Vec<TranslationUnit>,
    pub terminology_matches: Vec<Term>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_basic_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_tm.db");
        
        let tm = TradocFlowTranslationMemory::new(db_path.to_str().unwrap()).await.unwrap();
        tm.initialize().await.unwrap();
        
        // Basic smoke test
        let result = tm.comprehensive_search(
            "Hello world",
            Language::English,
            Language::Spanish
        ).await.unwrap();
        
        assert!(result.translation_matches.is_empty());
        assert!(result.terminology_matches.is_empty());
    }
}