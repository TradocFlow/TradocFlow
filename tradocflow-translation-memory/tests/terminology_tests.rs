//! Terminology service tests

use tradocflow_translation_memory::services::TerminologyService;
use tradocflow_translation_memory::utils::CsvProcessor;
use tradocflow_translation_memory::models::{Term, TerminologyValidationConfig as ModelValidationConfig};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_terminology_service_creation() {
    let csv_processor = Arc::new(CsvProcessor::new());
    let validation_config = None; // Using default validation config from service
    
    let service = TerminologyService::new(csv_processor, validation_config).await.unwrap();
    
    // Test basic functionality
    let project_id = Uuid::new_v4();
    let terms = service.get_terms_by_project(project_id).await.unwrap();
    assert!(terms.is_empty());
}

#[tokio::test]
async fn test_terminology_crud_operations() {
    let csv_processor = Arc::new(CsvProcessor::new());
    let service = TerminologyService::new(csv_processor, None).await.unwrap();
    let project_id = Uuid::new_v4();
    
    // Test adding a term
    let term = Term::new(
        "API".to_string(),
        Some("Application Programming Interface".to_string()),
        true,
    ).unwrap();
    
    service.add_terminology(term.clone(), project_id).await.unwrap();
    
    // Test retrieving terms
    let terms = service.get_terms_by_project(project_id).await.unwrap();
    assert_eq!(terms.len(), 1);
    assert_eq!(terms[0].term, "API");
    assert!(terms[0].do_not_translate);
    
    // Test searching terms
    let search_results = service.search_terms_by_project("API", project_id, None).await.unwrap();
    assert_eq!(search_results.len(), 1);
    assert_eq!(search_results[0].term, "API");
    
    // Test updating a term
    let mut updated_term = terms[0].clone();
    updated_term.definition = Some("Updated definition".to_string());
    service.update_terminology(updated_term, project_id).await.unwrap();
    
    let terms_after_update = service.get_terms_by_project(project_id).await.unwrap();
    assert_eq!(terms_after_update[0].definition, Some("Updated definition".to_string()));
    
    // Test deleting a term
    let deleted = service.delete_terminology(terms[0].id, project_id).await.unwrap();
    assert!(deleted);
    
    let terms_after_delete = service.get_terms_by_project(project_id).await.unwrap();
    assert!(terms_after_delete.is_empty());
}

#[tokio::test]
async fn test_cache_functionality() {
    let csv_processor = Arc::new(CsvProcessor::new());
    let service = TerminologyService::new(csv_processor, None).await.unwrap();
    
    // Test cache stats
    let (terms_count, search_count, non_translatable_count, last_updated) = service.get_cache_stats().await;
    assert_eq!(terms_count, 0);
    assert_eq!(search_count, 0);
    assert_eq!(non_translatable_count, 0);
    assert!(last_updated.is_none());
}