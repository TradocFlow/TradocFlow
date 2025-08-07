use super::export_service::*;
use crate::{
    export_engine::ExportFormat,
    models::{document::Document, project::Project},
};
use chrono::Utc;
use std::{collections::HashMap, path::PathBuf};
use tempfile::TempDir;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[tokio::test]
async fn test_export_service_creation() {
    let service = ExportService::new();
    let (queued, active) = service.get_queue_status();
    assert_eq!(queued, 0);
    assert_eq!(active, 0);
}

#[tokio::test]
async fn test_export_request_validation() {
    // Test empty languages
    let config = ExportConfiguration {
        format: ExportFormat::Pdf,
        layout: ExportLayout::SingleLanguage,
        languages: vec![],
        include_metadata: true,
        include_table_of_contents: true,
        custom_css_path: None,
        template_options: HashMap::new(),
    };

    let result = ExportService::validate_export_config(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("At least one language"));
}

#[tokio::test]
async fn test_export_request_validation_invalid_css() {
    let config = ExportConfiguration {
        format: ExportFormat::Html,
        layout: ExportLayout::SingleLanguage,
        languages: vec!["en".to_string()],
        include_metadata: true,
        include_table_of_contents: true,
        custom_css_path: Some(PathBuf::from("/nonexistent/file.css")),
        template_options: HashMap::new(),
    };

    let result = ExportService::validate_export_config(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Custom CSS file not found"));
}

#[tokio::test]
async fn test_export_queue_management() {
    let service = ExportService::new();
    let temp_dir = TempDir::new().unwrap();

    let request = create_test_export_request(temp_dir.path().to_path_buf());
    let export_id = service.queue_export(request).await.unwrap();

    // Check that the export was queued
    let status = service.get_export_status(export_id);
    assert!(status.is_some());
    
    let job = status.unwrap();
    assert_eq!(job.request.id, export_id);
    assert!(matches!(job.status, ExportStatus::Queued | ExportStatus::InProgress));
}

#[tokio::test]
async fn test_export_cancellation() {
    let service = ExportService::new();
    let temp_dir = TempDir::new().unwrap();

    let request = create_test_export_request(temp_dir.path().to_path_buf());
    let export_id = service.queue_export(request).await.unwrap();

    // Cancel the export
    service.cancel_export(export_id).await.unwrap();

    // Check that the export is no longer in the queue
    let status = service.get_export_status(export_id);
    assert!(status.is_none());
}

#[tokio::test]
async fn test_export_progress_subscription() {
    let service = ExportService::new();
    let mut progress_receiver = service.subscribe_to_progress();

    // This test would need a mock export job to generate progress updates
    // For now, we just verify that the subscription works
    assert!(progress_receiver.try_recv().is_err()); // No messages yet
}

#[tokio::test]
async fn test_export_history() {
    let service = ExportService::new();
    
    // Initially, history should be empty
    let history = service.get_export_history(None);
    assert_eq!(history.total_exports, 0);
    assert_eq!(history.successful_exports, 0);
    assert_eq!(history.failed_exports, 0);
    assert!(history.exports.is_empty());
}

#[tokio::test]
async fn test_export_history_with_limit() {
    let service = ExportService::new();
    
    // Test with limit
    let history = service.get_export_history(Some(10));
    assert_eq!(history.total_exports, 0);
    assert!(history.exports.len() <= 10);
}

#[tokio::test]
async fn test_multiple_export_requests() {
    let service = ExportService::new();
    let temp_dir = TempDir::new().unwrap();

    // Queue multiple exports
    let mut export_ids = Vec::new();
    for i in 0..3 {
        let mut request = create_test_export_request(temp_dir.path().to_path_buf());
        request.filename_prefix = format!("test_{}", i);
        let export_id = service.queue_export(request).await.unwrap();
        export_ids.push(export_id);
    }

    // Check queue status
    let (queued, active) = service.get_queue_status();
    assert!(queued + active >= 3);

    // Verify all exports are tracked
    for export_id in export_ids {
        let status = service.get_export_status(export_id);
        assert!(status.is_some());
    }
}

#[tokio::test]
async fn test_export_configuration_formats() {
    let temp_dir = TempDir::new().unwrap();

    // Test PDF format
    let pdf_config = ExportConfiguration {
        format: ExportFormat::Pdf,
        layout: ExportLayout::SideBySide,
        languages: vec!["en".to_string(), "es".to_string()],
        include_metadata: true,
        include_table_of_contents: true,
        custom_css_path: None,
        template_options: HashMap::new(),
    };
    assert!(ExportService::validate_export_config(&pdf_config).is_ok());

    // Test HTML format
    let html_config = ExportConfiguration {
        format: ExportFormat::Html,
        layout: ExportLayout::Sequential,
        languages: vec!["en".to_string()],
        include_metadata: false,
        include_table_of_contents: false,
        custom_css_path: None,
        template_options: HashMap::new(),
    };
    assert!(ExportService::validate_export_config(&html_config).is_ok());

    // Test Both format
    let both_config = ExportConfiguration {
        format: ExportFormat::Both,
        layout: ExportLayout::SingleLanguage,
        languages: vec!["en".to_string(), "fr".to_string(), "de".to_string()],
        include_metadata: true,
        include_table_of_contents: true,
        custom_css_path: None,
        template_options: HashMap::new(),
    };
    assert!(ExportService::validate_export_config(&both_config).is_ok());
}

#[tokio::test]
async fn test_export_layout_options() {
    let layouts = vec![
        ExportLayout::SingleLanguage,
        ExportLayout::SideBySide,
        ExportLayout::Sequential,
    ];

    for layout in layouts {
        let config = ExportConfiguration {
            format: ExportFormat::Pdf,
            layout,
            languages: vec!["en".to_string(), "es".to_string()],
            include_metadata: true,
            include_table_of_contents: true,
            custom_css_path: None,
            template_options: HashMap::new(),
        };
        assert!(ExportService::validate_export_config(&config).is_ok());
    }
}

#[tokio::test]
async fn test_export_cleanup() {
    let service = ExportService::new();
    
    // Test cleanup of old exports (should not fail even with empty queue)
    let cleaned = service.cleanup_old_exports(30).await.unwrap();
    assert_eq!(cleaned, 0);
}

#[tokio::test]
async fn test_export_format_extension_mapping() {
    assert_eq!(ExportService::get_format_extension(&ExportFormat::Pdf), "pdf");
    assert_eq!(ExportService::get_format_extension(&ExportFormat::Html), "html");
    assert_eq!(ExportService::get_format_extension(&ExportFormat::Both), "mixed");
}

// Helper function to create a test export request
fn create_test_export_request(output_directory: PathBuf) -> ExportRequest {
    ExportRequest {
        id: Uuid::new_v4(),
        project_id: Uuid::new_v4(),
        document_id: Some(Uuid::new_v4()),
        config: ExportConfiguration {
            format: ExportFormat::Pdf,
            layout: ExportLayout::SingleLanguage,
            languages: vec!["en".to_string()],
            include_metadata: true,
            include_table_of_contents: true,
            custom_css_path: None,
            template_options: HashMap::new(),
        },
        output_directory,
        filename_prefix: "test_document".to_string(),
        requested_at: Utc::now(),
        requested_by: "test_user".to_string(),
    }
}

// Integration test for the complete export workflow
#[tokio::test]
async fn test_complete_export_workflow() {
    let service = ExportService::new();
    let temp_dir = TempDir::new().unwrap();

    // Create a test export request
    let request = create_test_export_request(temp_dir.path().to_path_buf());
    let export_id = request.id;

    // Queue the export
    let queued_id = service.queue_export(request).await.unwrap();
    assert_eq!(export_id, queued_id);

    // Check initial status
    let initial_status = service.get_export_status(export_id);
    assert!(initial_status.is_some());

    // Wait a bit for processing to potentially start
    sleep(Duration::from_millis(100)).await;

    // Check queue status
    let (queued, active) = service.get_queue_status();
    assert!(queued + active >= 1);

    // The export might complete quickly or still be processing
    // We just verify that the system is tracking it properly
    let final_status = service.get_export_status(export_id);
    if let Some(status) = final_status {
        assert!(matches!(
            status.status,
            ExportStatus::Queued | ExportStatus::InProgress | ExportStatus::Completed | ExportStatus::Failed
        ));
    }
}

#[tokio::test]
async fn test_concurrent_export_limit() {
    let service = ExportService::new();
    let temp_dir = TempDir::new().unwrap();

    // Queue more exports than the concurrent limit
    let mut export_ids = Vec::new();
    for i in 0..5 {
        let mut request = create_test_export_request(temp_dir.path().to_path_buf());
        request.filename_prefix = format!("concurrent_test_{}", i);
        let export_id = service.queue_export(request).await.unwrap();
        export_ids.push(export_id);
    }

    // Check that the service respects the concurrent limit
    let (queued, active) = service.get_queue_status();
    assert!(active <= 3); // max_concurrent_jobs is set to 3
    assert_eq!(queued + active, 5);
}