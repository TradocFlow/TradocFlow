//! Comprehensive test suite for translation memory services
//! 
//! This module provides integration and unit tests for all major components
//! of the translation memory system including services, storage, and utilities.

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod service_tests;

#[cfg(test)]
mod storage_tests;

#[cfg(test)]
mod error_tests;

#[cfg(test)]
mod performance_tests;

// Test utilities and helpers
pub mod test_utils;

use uuid::Uuid;
use chrono::Utc;
use tempfile::TempDir;
use std::sync::Arc;

/// Common test fixtures and setup
pub struct TestFixtures {
    pub temp_dir: TempDir,
    pub project_id: Uuid,
    pub test_db_path: String,
}

impl TestFixtures {
    /// Create new test fixtures with temporary directory
    pub fn new() -> Self {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
        let project_id = Uuid::new_v4();
        let test_db_path = temp_dir.path().join("test.db").to_string_lossy().to_string();
        
        Self {
            temp_dir,
            project_id,
            test_db_path,
        }
    }
}

/// Create test translation units for testing
pub fn create_test_translation_units(count: usize, project_id: Uuid) -> Vec<crate::models::TranslationUnit> {
    (0..count).map(|i| crate::models::TranslationUnit {
        id: Uuid::new_v4(),
        project_id,
        chapter_id: Uuid::new_v4(),
        chunk_id: Uuid::new_v4(),
        source_language: crate::models::Language::English,
        source_text: format!("Test source text {}", i),
        target_language: crate::models::Language::Spanish,
        target_text: format!("Texto de prueba {}", i),
        confidence_score: 0.85 + (i as f32 * 0.01),
        context: Some(format!("Test context {}", i)),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }).collect()
}

/// Create test terminology entries for testing
pub fn create_test_terminology_entries(count: usize) -> Vec<crate::models::Terminology> {
    (0..count).map(|i| crate::models::Terminology {
        id: Uuid::new_v4(),
        term: format!("test_term_{}", i),
        definition: Some(format!("Definition for term {}", i)),
        do_not_translate: i % 2 == 0, // Alternate between translatable and non-translatable
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }).collect()
}