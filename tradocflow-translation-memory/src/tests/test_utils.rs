//! Test utilities and helper functions for the translation memory test suite

use uuid::Uuid;
use chrono::Utc;
use std::path::Path;
use tempfile::TempDir;

/// Test utilities for creating mock data and test fixtures
pub struct TestUtils;

impl TestUtils {
    /// Create a temporary directory for tests
    pub fn create_temp_dir() -> TempDir {
        tempfile::tempdir().expect("Failed to create temporary directory")
    }
    
    /// Create a test database path in a temporary directory
    pub fn create_test_db_path(temp_dir: &TempDir) -> String {
        temp_dir.path().join("test.db").to_string_lossy().to_string()
    }
    
    /// Generate a UUID for testing purposes
    pub fn generate_test_uuid() -> Uuid {
        Uuid::new_v4()
    }
    
    /// Create test CSV content for terminology import testing
    pub fn create_test_csv_content() -> String {
        vec![
            "term,definition,do_not_translate,category,notes",
            "API,Application Programming Interface,true,technical,Core term",
            "JSON,JavaScript Object Notation,true,technical,Data format",
            "HTTP,HyperText Transfer Protocol,false,technical,Web protocol",
            "database,System for storing data,false,general,Storage system",
            "cache,Temporary storage,false,technical,Performance optimization"
        ].join("\n")
    }
    
    /// Create test TMX (Translation Memory eXchange) content
    pub fn create_test_tmx_content() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <tmx version="1.4">
          <header>
            <prop type="x-filename">test.tmx</prop>
          </header>
          <body>
            <tu tuid="1">
              <tuv xml:lang="en">
                <seg>Hello world</seg>
              </tuv>
              <tuv xml:lang="es">
                <seg>Hola mundo</seg>
              </tuv>
            </tu>
            <tu tuid="2">
              <tuv xml:lang="en">
                <seg>Good morning</seg>
              </tuv>
              <tuv xml:lang="es">
                <seg>Buenos días</seg>
              </tuv>
            </tu>
          </body>
        </tmx>"#.to_string()
    }
    
    /// Write test content to a temporary file
    pub async fn write_test_file(temp_dir: &TempDir, filename: &str, content: &str) -> Result<String, std::io::Error> {
        let file_path = temp_dir.path().join(filename);
        tokio::fs::write(&file_path, content).await?;
        Ok(file_path.to_string_lossy().to_string())
    }
    
    /// Create test data for performance benchmarking
    pub fn create_large_dataset(size: usize, project_id: Uuid) -> Vec<crate::models::TranslationUnit> {
        (0..size).map(|i| {
            let source_variations = [
                "This is test sentence number",
                "Here we have example text",
                "Sample content for testing purposes",
                "Demo translation unit data",
                "Benchmark test content item"
            ];
            
            let target_variations = [
                "Esta es la oración de prueba número",
                "Aquí tenemos texto de ejemplo",
                "Contenido de muestra para propósitos de prueba",
                "Datos de unidad de traducción demo",
                "Elemento de contenido de prueba de referencia"
            ];
            
            let source_base = source_variations[i % source_variations.len()];
            let target_base = target_variations[i % target_variations.len()];
            
            crate::models::TranslationUnit {
                id: Uuid::new_v4(),
                project_id,
                chapter_id: Uuid::new_v4(),
                chunk_id: Uuid::new_v4(),
                source_language: crate::models::Language::English,
                source_text: format!("{} {}", source_base, i + 1),
                target_language: crate::models::Language::Spanish,
                target_text: format!("{} {}", target_base, i + 1),
                confidence_score: 0.7 + (i as f32 % 30) / 100.0, // Vary between 0.7 and 0.99
                context: if i % 3 == 0 { 
                    Some(format!("Context for item {}", i)) 
                } else { 
                    None 
                },
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        }).collect()
    }
    
    /// Create test terminology dataset for performance testing
    pub fn create_large_terminology_dataset(size: usize) -> Vec<crate::models::Terminology> {
        let term_prefixes = ["tech_", "business_", "legal_", "medical_", "general_"];
        let definitions = [
            "Technical term for system component",
            "Business process or methodology",
            "Legal concept or procedure",
            "Medical terminology or process",
            "General purpose term"
        ];
        
        (0..size).map(|i| {
            let prefix = term_prefixes[i % term_prefixes.len()];
            let definition = definitions[i % definitions.len()];
            
            crate::models::Terminology {
                id: Uuid::new_v4(),
                term: format!("{}{}", prefix, i),
                definition: Some(format!("{} {}", definition, i)),
                do_not_translate: i % 3 == 0, // Every third term is non-translatable
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        }).collect()
    }
    
    /// Validate test results and assert conditions
    pub fn assert_performance_within_bounds(
        operation_name: &str,
        duration_ms: u128,
        expected_max_ms: u128
    ) {
        assert!(
            duration_ms <= expected_max_ms,
            "{} took {}ms, expected <= {}ms",
            operation_name,
            duration_ms,
            expected_max_ms
        );
        
        println!("{}: {}ms (limit: {}ms)", operation_name, duration_ms, expected_max_ms);
    }
    
    /// Create test configuration for services
    pub fn create_test_terminology_config() -> crate::services::terminology::TerminologyValidationConfig {
        crate::services::terminology::TerminologyValidationConfig {
            case_sensitive: false,
            allow_duplicates: false,
            max_term_length: 100,
            max_definition_length: 500,
            required_fields: vec!["term".to_string()],
        }
    }
    
    /// Create test highlighting configuration
    pub fn create_test_highlighting_config() -> crate::services::highlighting::HighlightingConfig {
        crate::services::highlighting::HighlightingConfig {
            case_sensitive: false,
            word_boundaries_only: true,
            min_confidence_threshold: 0.6,
            max_context_length: 100,
            highlight_overlaps: false,
            include_variations: true,
        }
    }
    
    /// Compare two translation units for testing equality
    pub fn assert_translation_units_equal(
        unit1: &crate::models::TranslationUnit,
        unit2: &crate::models::TranslationUnit,
        ignore_timestamps: bool
    ) {
        assert_eq!(unit1.id, unit2.id);
        assert_eq!(unit1.project_id, unit2.project_id);
        assert_eq!(unit1.source_text, unit2.source_text);
        assert_eq!(unit1.target_text, unit2.target_text);
        assert_eq!(unit1.source_language, unit2.source_language);
        assert_eq!(unit1.target_language, unit2.target_language);
        assert!((unit1.confidence_score - unit2.confidence_score).abs() < 0.001);
        assert_eq!(unit1.context, unit2.context);
        
        if !ignore_timestamps {
            assert_eq!(unit1.created_at, unit2.created_at);
            assert_eq!(unit1.updated_at, unit2.updated_at);
        }
    }
    
    /// Compare two terminology entries for testing equality
    pub fn assert_terminology_equal(
        term1: &crate::models::Terminology,
        term2: &crate::models::Terminology,
        ignore_timestamps: bool
    ) {
        assert_eq!(term1.id, term2.id);
        assert_eq!(term1.term, term2.term);
        assert_eq!(term1.definition, term2.definition);
        assert_eq!(term1.do_not_translate, term2.do_not_translate);
        
        if !ignore_timestamps {
            assert_eq!(term1.created_at, term2.created_at);
            assert_eq!(term1.updated_at, term2.updated_at);
        }
    }
}

/// Async test helper macros
#[macro_export]
macro_rules! async_test_timeout {
    ($test_fn:expr, $timeout_secs:expr) => {
        tokio::time::timeout(
            tokio::time::Duration::from_secs($timeout_secs),
            $test_fn
        ).await.expect("Test timed out")
    };
}

/// Test result assertion helper
#[macro_export]
macro_rules! assert_result_ok {
    ($result:expr, $message:expr) => {
        match $result {
            Ok(val) => val,
            Err(e) => panic!("{}: {:?}", $message, e),
        }
    };
}

/// Test error assertion helper
#[macro_export]
macro_rules! assert_result_err {
    ($result:expr, $expected_error_type:ty) => {
        match $result {
            Ok(_) => panic!("Expected error of type {}, but got Ok", stringify!($expected_error_type)),
            Err(e) => {
                // Additional error type checking would be implemented here
                // once the error types are properly defined
            },
        }
    };
}

#[cfg(test)]
mod test_utils_tests {
    use super::*;
    
    #[test]
    fn test_create_temp_dir() {
        let temp_dir = TestUtils::create_temp_dir();
        assert!(temp_dir.path().exists());
    }
    
    #[test]
    fn test_generate_test_uuid() {
        let uuid1 = TestUtils::generate_test_uuid();
        let uuid2 = TestUtils::generate_test_uuid();
        assert_ne!(uuid1, uuid2);
    }
    
    #[test]
    fn test_create_test_csv_content() {
        let csv_content = TestUtils::create_test_csv_content();
        assert!(csv_content.contains("term,definition"));
        assert!(csv_content.contains("API,Application Programming Interface"));
    }
    
    #[test]
    fn test_create_test_tmx_content() {
        let tmx_content = TestUtils::create_test_tmx_content();
        assert!(tmx_content.contains("<?xml version=\"1.0\""));
        assert!(tmx_content.contains("<tmx version=\"1.4\">"));
        assert!(tmx_content.contains("Hello world"));
    }
    
    #[test]
    fn test_create_large_dataset() {
        let project_id = TestUtils::generate_test_uuid();
        let dataset = TestUtils::create_large_dataset(100, project_id);
        
        assert_eq!(dataset.len(), 100);
        assert!(dataset.iter().all(|unit| unit.project_id == project_id));
        assert!(dataset.iter().all(|unit| unit.source_language == crate::models::Language::English));
        assert!(dataset.iter().all(|unit| unit.target_language == crate::models::Language::Spanish));
    }
    
    #[test]
    fn test_create_large_terminology_dataset() {
        let dataset = TestUtils::create_large_terminology_dataset(50);
        
        assert_eq!(dataset.len(), 50);
        assert!(dataset.iter().any(|term| term.do_not_translate));
        assert!(dataset.iter().any(|term| !term.do_not_translate));
        assert!(dataset.iter().all(|term| term.definition.is_some()));
    }
}