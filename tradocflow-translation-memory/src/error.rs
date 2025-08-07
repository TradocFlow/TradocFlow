//! Error types for the translation memory system

use thiserror::Error;

/// Result type alias for translation memory operations
pub type Result<T> = std::result::Result<T, TranslationMemoryError>;

/// Comprehensive error type for translation memory operations
#[derive(Error, Debug)]
pub enum TranslationMemoryError {
    // Note: DuckDB and Parquet dependencies are temporarily disabled
    // #[cfg(feature = "duckdb-storage")]
    // #[error("Database error: {0}")]
    // Database(#[from] duckdb::Error),
    
    // #[cfg(feature = "parquet-export")]
    // #[error("Parquet error: {0}")]
    // Parquet(#[from] parquet::errors::ParquetError),
    
    // #[cfg(any(feature = "duckdb-storage", feature = "parquet-export"))]
    // #[error("Arrow error: {0}")]
    // Arrow(#[from] arrow::error::ArrowError),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[cfg(feature = "terminology-csv")]
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
    
    #[error("Invalid language code: {0}")]
    InvalidLanguage(String),
    
    #[error("Invalid domain: {0}")]
    InvalidDomain(String),
    
    #[error("Translation unit not found: {0}")]
    TranslationUnitNotFound(uuid::Uuid),
    
    #[error("Terminology entry not found: {0}")]
    TerminologyNotFound(uuid::Uuid),
    
    #[error("Chunk not found: {0}")]
    ChunkNotFound(uuid::Uuid),
    
    #[error("Invalid match score: {0} (must be between 0.0 and 1.0)")]
    InvalidMatchScore(f64),
    
    #[error("Invalid quality score: {0} (must be between 0 and 100)")]
    InvalidQuality(u8),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Search query too short: minimum length is {min}, got {actual}")]
    QueryTooShort { min: usize, actual: usize },
    
    #[error("Search query too long: maximum length is {max}, got {actual}")]
    QueryTooLong { max: usize, actual: usize },
    
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
    
    #[error("Concurrent access error: {0}")]
    ConcurrentAccess(String),
    
    #[error("Schema migration error: {0}")]
    SchemaMigration(String),
    
    #[error("Data validation error: {0}")]
    DataValidation(String),
    
    #[error("Resource exhaustion: {0}")]
    ResourceExhaustion(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Authentication error: {0}")]
    Authentication(String),
    
    #[error("Authorization error: {0}")]
    Authorization(String),
    
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Import error: {0}")]
    ImportError(String),
    
    #[error("Export error: {0}")]
    ExportError(String),
    
    #[error("Cache error: {0}")]
    CacheError(String),
    
    #[error("Compression error: {0}")]
    CompressionError(String),
    
    #[error("Threading error: {0}")]
    ThreadingError(String),
    
    #[error("Timeout error: {0}")]
    TimeoutError(String),
    
    #[error("Connection pool error: {0}")]
    ConnectionPoolError(String),
    
    #[error("File operation error: {0}")]
    FileOperationError(String),
    
    #[error("Parsing error: {0}")]
    ParsingError(String),
    
    #[error("Encoding error: {0}")]
    EncodingError(String),
    
    #[error("Generic error: {0}")]
    Generic(String),
}

impl TranslationMemoryError {
    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::DatabaseError(_) | Self::StorageError(_) => true,
            Self::Io(_) | Self::Network(_) => true,
            Self::ConcurrentAccess(_) | Self::ResourceExhaustion(_) => true,
            Self::TimeoutError(_) | Self::ConnectionPoolError(_) => true,
            Self::ThreadingError(_) => true,
            _ => false,
        }
    }
    
    /// Check if the error is a user input error
    pub fn is_user_error(&self) -> bool {
        match self {
            Self::InvalidLanguage(_) | Self::InvalidDomain(_) => true,
            Self::InvalidMatchScore(_) | Self::InvalidQuality(_) => true,
            Self::QueryTooShort { .. } | Self::QueryTooLong { .. } => true,
            Self::DataValidation(_) | Self::ValidationError(_) => true,
            Self::ImportError(_) | Self::ExportError(_) => true,
            Self::ParsingError(_) | Self::EncodingError(_) => true,
            Self::FileOperationError(_) => true,
            _ => false,
        }
    }
    
    /// Get error category for logging and monitoring
    pub fn category(&self) -> ErrorCategory {
        match self {
            // Temporarily disabled error variants
            // #[cfg(feature = "duckdb-storage")]
            // Self::Database(_) => ErrorCategory::Storage,
            // #[cfg(feature = "parquet-export")]
            // Self::Parquet(_) => ErrorCategory::Storage,
            // #[cfg(any(feature = "duckdb-storage", feature = "parquet-export"))]
            // Self::Arrow(_) => ErrorCategory::Storage,
            Self::DatabaseError(_) | Self::StorageError(_) => ErrorCategory::Storage,
            Self::Io(_) | Self::Network(_) => ErrorCategory::IO,
            #[cfg(feature = "terminology-csv")]
            Self::Serialization(_) | Self::Csv(_) => ErrorCategory::Serialization,
            #[cfg(not(feature = "terminology-csv"))]
            Self::Serialization(_) => ErrorCategory::Serialization,
            Self::InvalidLanguage(_) | Self::InvalidDomain(_) => ErrorCategory::Validation,
            Self::InvalidMatchScore(_) | Self::InvalidQuality(_) => ErrorCategory::Validation,
            Self::QueryTooShort { .. } | Self::QueryTooLong { .. } => ErrorCategory::Validation,
            Self::DataValidation(_) | Self::ValidationError(_) => ErrorCategory::Validation,
            Self::TranslationUnitNotFound(_) => ErrorCategory::NotFound,
            Self::TerminologyNotFound(_) => ErrorCategory::NotFound,
            Self::ChunkNotFound(_) => ErrorCategory::NotFound,
            Self::NotFound(_) => ErrorCategory::NotFound,
            Self::Configuration(_) => ErrorCategory::Configuration,
            Self::UnsupportedOperation(_) => ErrorCategory::Logic,
            Self::ConcurrentAccess(_) | Self::ThreadingError(_) => ErrorCategory::Concurrency,
            Self::SchemaMigration(_) => ErrorCategory::Migration,
            Self::ResourceExhaustion(_) | Self::ConnectionPoolError(_) => ErrorCategory::Resource,
            Self::Authentication(_) | Self::Authorization(_) => ErrorCategory::Security,
            Self::ImportError(_) | Self::ExportError(_) => ErrorCategory::IO,
            Self::CacheError(_) => ErrorCategory::Resource,
            Self::CompressionError(_) => ErrorCategory::Storage,
            Self::TimeoutError(_) => ErrorCategory::IO,
            Self::FileOperationError(_) => ErrorCategory::IO,
            Self::ParsingError(_) | Self::EncodingError(_) => ErrorCategory::Serialization,
            Self::Regex(_) => ErrorCategory::Logic,
            Self::Generic(_) => ErrorCategory::Unknown,
        }
    }
    
    /// Create a context-aware error with additional details
    pub fn with_context(self, context: &str) -> Self {
        match self {
            Self::Generic(msg) => Self::Generic(format!("{}: {}", context, msg)),
            Self::DatabaseError(msg) => Self::DatabaseError(format!("{}: {}", context, msg)),
            Self::StorageError(msg) => Self::StorageError(format!("{}: {}", context, msg)),
            Self::ValidationError(msg) => Self::ValidationError(format!("{}: {}", context, msg)),
            Self::ImportError(msg) => Self::ImportError(format!("{}: {}", context, msg)),
            Self::ExportError(msg) => Self::ExportError(format!("{}: {}", context, msg)),
            other => other,
        }
    }
    
    /// Create a database error with context
    pub fn database_error(msg: &str, context: &str) -> Self {
        Self::DatabaseError(format!("{}: {}", context, msg))
    }
    
    /// Create a storage error with context
    pub fn storage_error(msg: &str, context: &str) -> Self {
        Self::StorageError(format!("{}: {}", context, msg))
    }
    
    /// Create a validation error with context
    pub fn validation_error(msg: &str, context: &str) -> Self {
        Self::ValidationError(format!("{}: {}", context, msg))
    }
    
    /// Create a not found error with context
    pub fn not_found(entity: &str, id: &str) -> Self {
        Self::NotFound(format!("{} with ID {} not found", entity, id))
    }
}

/// Error categories for monitoring and alerting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Storage,
    IO,
    Serialization,
    Validation,
    NotFound,
    Configuration,
    Logic,
    Concurrency,
    Migration,
    Resource,
    Security,
    Unknown,
}

impl ErrorCategory {
    /// Get the severity level for this error category
    pub fn severity(&self) -> Severity {
        match self {
            Self::Security => Severity::Critical,
            Self::Storage | Self::Resource => Severity::High,
            Self::IO | Self::Concurrency | Self::Migration => Severity::Medium,
            Self::Validation | Self::NotFound => Severity::Low,
            Self::Serialization | Self::Configuration | Self::Logic => Severity::Medium,
            Self::Unknown => Severity::Medium,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_categorization() {
        let err = TranslationMemoryError::InvalidLanguage("invalid".to_string());
        assert_eq!(err.category(), ErrorCategory::Validation);
        assert!(err.is_user_error());
        assert!(!err.is_recoverable());
        
        let db_err = TranslationMemoryError::DatabaseError("connection failed".to_string());
        assert_eq!(db_err.category(), ErrorCategory::Storage);
        assert!(!db_err.is_user_error());
        assert!(db_err.is_recoverable());
    }
    
    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
    }
    
    #[test]
    fn test_error_context() {
        let base_err = TranslationMemoryError::Generic("test error".to_string());
        let with_context = base_err.with_context("service operation");
        match with_context {
            TranslationMemoryError::Generic(msg) => {
                assert!(msg.contains("service operation"));
                assert!(msg.contains("test error"));
            }
            _ => panic!("Expected Generic error"),
        }
    }
    
    #[test]
    fn test_context_aware_constructors() {
        let db_err = TranslationMemoryError::database_error("connection failed", "initialization");
        match db_err {
            TranslationMemoryError::DatabaseError(msg) => {
                assert!(msg.contains("initialization"));
                assert!(msg.contains("connection failed"));
            }
            _ => panic!("Expected DatabaseError"),
        }
        
        let not_found = TranslationMemoryError::not_found("Translation unit", "12345");
        match not_found {
            TranslationMemoryError::NotFound(msg) => {
                assert!(msg.contains("Translation unit"));
                assert!(msg.contains("12345"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }
    
    #[test]
    fn test_recoverable_errors() {
        let recoverable_errors = vec![
            TranslationMemoryError::DatabaseError("test".to_string()),
            TranslationMemoryError::StorageError("test".to_string()),
            TranslationMemoryError::TimeoutError("test".to_string()),
            TranslationMemoryError::ConnectionPoolError("test".to_string()),
        ];
        
        for err in recoverable_errors {
            assert!(err.is_recoverable(), "Error should be recoverable: {:?}", err);
        }
        
        let non_recoverable_errors = vec![
            TranslationMemoryError::ValidationError("test".to_string()),
            TranslationMemoryError::InvalidLanguage("test".to_string()),
            TranslationMemoryError::ParsingError("test".to_string()),
        ];
        
        for err in non_recoverable_errors {
            assert!(!err.is_recoverable(), "Error should not be recoverable: {:?}", err);
        }
    }
    
    #[test]
    fn test_user_errors() {
        let user_errors = vec![
            TranslationMemoryError::ValidationError("test".to_string()),
            TranslationMemoryError::InvalidLanguage("test".to_string()),
            TranslationMemoryError::ParsingError("test".to_string()),
            TranslationMemoryError::ImportError("test".to_string()),
        ];
        
        for err in user_errors {
            assert!(err.is_user_error(), "Error should be user error: {:?}", err);
        }
        
        let system_errors = vec![
            TranslationMemoryError::DatabaseError("test".to_string()),
            TranslationMemoryError::ThreadingError("test".to_string()),
            TranslationMemoryError::ConnectionPoolError("test".to_string()),
        ];
        
        for err in system_errors {
            assert!(!err.is_user_error(), "Error should not be user error: {:?}", err);
        }
    }
}