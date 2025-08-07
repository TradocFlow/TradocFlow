//! Error handling tests for the translation memory system

#[cfg(test)]
mod error_handling_tests {
    use crate::error::*;
    
    #[test]
    fn test_error_categorization() {
        // Test database errors
        let db_err = TranslationMemoryError::DatabaseError("connection failed".to_string());
        assert_eq!(db_err.category(), ErrorCategory::Storage);
        assert!(db_err.is_recoverable());
        assert!(!db_err.is_user_error());
        
        // Test validation errors
        let val_err = TranslationMemoryError::ValidationError("invalid input".to_string());
        assert_eq!(val_err.category(), ErrorCategory::Validation);
        assert!(!val_err.is_recoverable());
        assert!(val_err.is_user_error());
        
        // Test I/O errors
        let io_err = TranslationMemoryError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound, 
            "file not found"
        ));
        assert_eq!(io_err.category(), ErrorCategory::IO);
        assert!(io_err.is_recoverable());
        assert!(!io_err.is_user_error());
        
        // Test timeout errors
        let timeout_err = TranslationMemoryError::TimeoutError("operation timed out".to_string());
        assert_eq!(timeout_err.category(), ErrorCategory::IO);
        assert!(timeout_err.is_recoverable());
        assert!(!timeout_err.is_user_error());
    }
    
    #[test]
    fn test_error_severity() {
        let security_category = ErrorCategory::Security;
        assert_eq!(security_category.severity(), Severity::Critical);
        
        let storage_category = ErrorCategory::Storage;
        assert_eq!(storage_category.severity(), Severity::High);
        
        let validation_category = ErrorCategory::Validation;
        assert_eq!(validation_category.severity(), Severity::Low);
        
        let unknown_category = ErrorCategory::Unknown;
        assert_eq!(unknown_category.severity(), Severity::Medium);
    }
    
    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
        
        let mut severities = vec![Severity::Low, Severity::Critical, Severity::Medium, Severity::High];
        severities.sort();
        assert_eq!(severities, vec![Severity::Low, Severity::Medium, Severity::High, Severity::Critical]);
    }
    
    #[test]
    fn test_error_context() {
        let base_err = TranslationMemoryError::Generic("base error".to_string());
        let contextual_err = base_err.with_context("operation failed");
        
        match contextual_err {
            TranslationMemoryError::Generic(msg) => {
                assert!(msg.contains("operation failed"));
                assert!(msg.contains("base error"));
            }
            _ => panic!("Expected Generic error with context"),
        }
    }
    
    #[test]
    fn test_context_aware_constructors() {
        // Test database error constructor
        let db_err = TranslationMemoryError::database_error("timeout", "connection pool");
        match db_err {
            TranslationMemoryError::DatabaseError(msg) => {
                assert!(msg.contains("connection pool"));
                assert!(msg.contains("timeout"));
            }
            _ => panic!("Expected DatabaseError"),
        }
        
        // Test storage error constructor
        let storage_err = TranslationMemoryError::storage_error("disk full", "parquet write");
        match storage_err {
            TranslationMemoryError::StorageError(msg) => {
                assert!(msg.contains("parquet write"));
                assert!(msg.contains("disk full"));
            }
            _ => panic!("Expected StorageError"),
        }
        
        // Test validation error constructor
        let val_err = TranslationMemoryError::validation_error("empty field", "term validation");
        match val_err {
            TranslationMemoryError::ValidationError(msg) => {
                assert!(msg.contains("term validation"));
                assert!(msg.contains("empty field"));
            }
            _ => panic!("Expected ValidationError"),
        }
        
        // Test not found error constructor
        let not_found_err = TranslationMemoryError::not_found("Translation unit", "12345");
        match not_found_err {
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
            TranslationMemoryError::ThreadingError("test".to_string()),
            TranslationMemoryError::Io(std::io::Error::new(std::io::ErrorKind::Other, "test")),
            TranslationMemoryError::Network("test".to_string()),
            TranslationMemoryError::ConcurrentAccess("test".to_string()),
            TranslationMemoryError::ResourceExhaustion("test".to_string()),
        ];
        
        for err in recoverable_errors {
            assert!(err.is_recoverable(), "Error should be recoverable: {:?}", err);
        }
    }
    
    #[test]
    fn test_non_recoverable_errors() {
        let non_recoverable_errors = vec![
            TranslationMemoryError::ValidationError("test".to_string()),
            TranslationMemoryError::InvalidLanguage("test".to_string()),
            TranslationMemoryError::ParsingError("test".to_string()),
            TranslationMemoryError::Configuration("test".to_string()),
            TranslationMemoryError::UnsupportedOperation("test".to_string()),
            TranslationMemoryError::Authentication("test".to_string()),
            TranslationMemoryError::Authorization("test".to_string()),
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
            TranslationMemoryError::InvalidDomain("test".to_string()),
            TranslationMemoryError::InvalidMatchScore(1.5),
            TranslationMemoryError::InvalidQuality(101),
            TranslationMemoryError::QueryTooShort { min: 3, actual: 1 },
            TranslationMemoryError::QueryTooLong { max: 100, actual: 150 },
            TranslationMemoryError::DataValidation("test".to_string()),
            TranslationMemoryError::ImportError("test".to_string()),
            TranslationMemoryError::ExportError("test".to_string()),
            TranslationMemoryError::ParsingError("test".to_string()),
            TranslationMemoryError::EncodingError("test".to_string()),
            TranslationMemoryError::FileOperationError("test".to_string()),
        ];
        
        for err in user_errors {
            assert!(err.is_user_error(), "Error should be user error: {:?}", err);
        }
    }
    
    #[test]
    fn test_system_errors() {
        let system_errors = vec![
            TranslationMemoryError::DatabaseError("test".to_string()),
            TranslationMemoryError::StorageError("test".to_string()),
            TranslationMemoryError::ThreadingError("test".to_string()),
            TranslationMemoryError::ConnectionPoolError("test".to_string()),
            TranslationMemoryError::TimeoutError("test".to_string()),
            TranslationMemoryError::CacheError("test".to_string()),
            TranslationMemoryError::CompressionError("test".to_string()),
            TranslationMemoryError::Network("test".to_string()),
            TranslationMemoryError::ConcurrentAccess("test".to_string()),
            TranslationMemoryError::ResourceExhaustion("test".to_string()),
        ];
        
        for err in system_errors {
            assert!(!err.is_user_error(), "Error should not be user error: {:?}", err);
        }
    }
    
    #[test]
    fn test_all_error_categories_covered() {
        // Test that all error variants have proper category mapping
        let test_errors = vec![
            (TranslationMemoryError::DatabaseError("test".to_string()), ErrorCategory::Storage),
            (TranslationMemoryError::StorageError("test".to_string()), ErrorCategory::Storage),
            (TranslationMemoryError::Io(std::io::Error::new(std::io::ErrorKind::Other, "test")), ErrorCategory::IO),
            (TranslationMemoryError::Network("test".to_string()), ErrorCategory::IO),
            (TranslationMemoryError::Serialization(serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::Other, "test"))), ErrorCategory::Serialization),
            (TranslationMemoryError::ValidationError("test".to_string()), ErrorCategory::Validation),
            (TranslationMemoryError::NotFound("test".to_string()), ErrorCategory::NotFound),
            (TranslationMemoryError::Configuration("test".to_string()), ErrorCategory::Configuration),
            (TranslationMemoryError::UnsupportedOperation("test".to_string()), ErrorCategory::Logic),
            (TranslationMemoryError::ConcurrentAccess("test".to_string()), ErrorCategory::Concurrency),
            (TranslationMemoryError::ThreadingError("test".to_string()), ErrorCategory::Concurrency),
            (TranslationMemoryError::SchemaMigration("test".to_string()), ErrorCategory::Migration),
            (TranslationMemoryError::ResourceExhaustion("test".to_string()), ErrorCategory::Resource),
            (TranslationMemoryError::ConnectionPoolError("test".to_string()), ErrorCategory::Resource),
            (TranslationMemoryError::CacheError("test".to_string()), ErrorCategory::Resource),
            (TranslationMemoryError::Authentication("test".to_string()), ErrorCategory::Security),
            (TranslationMemoryError::Authorization("test".to_string()), ErrorCategory::Security),
            (TranslationMemoryError::CompressionError("test".to_string()), ErrorCategory::Storage),
            (TranslationMemoryError::TimeoutError("test".to_string()), ErrorCategory::IO),
            (TranslationMemoryError::FileOperationError("test".to_string()), ErrorCategory::IO),
            (TranslationMemoryError::ImportError("test".to_string()), ErrorCategory::IO),
            (TranslationMemoryError::ExportError("test".to_string()), ErrorCategory::IO),
            (TranslationMemoryError::ParsingError("test".to_string()), ErrorCategory::Serialization),
            (TranslationMemoryError::EncodingError("test".to_string()), ErrorCategory::Serialization),
            (TranslationMemoryError::Generic("test".to_string()), ErrorCategory::Unknown),
        ];
        
        for (error, expected_category) in test_errors {
            assert_eq!(error.category(), expected_category, "Error category mismatch for: {:?}", error);
        }
    }
    
    #[test]
    fn test_error_display_formatting() {
        let errors = vec![
            TranslationMemoryError::DatabaseError("connection failed".to_string()),
            TranslationMemoryError::ValidationError("invalid input".to_string()),
            TranslationMemoryError::NotFound("entity not found".to_string()),
            TranslationMemoryError::QueryTooShort { min: 3, actual: 1 },
            TranslationMemoryError::QueryTooLong { max: 100, actual: 150 },
            TranslationMemoryError::InvalidMatchScore(1.5),
            TranslationMemoryError::InvalidQuality(101),
        ];
        
        for error in errors {
            let display_string = format!("{}", error);
            assert!(!display_string.is_empty(), "Error display should not be empty");
            assert!(display_string.len() > 10, "Error display should be descriptive");
        }
    }
}