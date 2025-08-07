//! Storage layer for translation memory
//! 
//! Migrated from the main TradocFlow crate with thread safety improvements:
//! - Connection pooling for DuckDB operations
//! - Async-first design with Send + Sync traits
//! - Lock-free cache coordination

pub mod traits;
pub mod duckdb_manager;
pub mod parquet_manager;
pub mod chunk_manager;

// Re-export storage trait abstractions
pub use traits::{
    TranslationMemoryStorage, 
    TerminologyStorage, 
    ChunkStorage,
    UnifiedStorageProvider,
    StorageFactory,
    StorageConfig,
    TranslationMemoryStorageStats,
    TerminologyStorageStats,
    ChunkStorageStats,
    ComprehensiveStorageStats,
};

// Re-export concrete storage implementations
pub use duckdb_manager::DuckDBManager;
pub use parquet_manager::ParquetManager;
pub use chunk_manager::{ChunkManager, ChunkManagementStats};