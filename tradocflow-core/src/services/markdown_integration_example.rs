// Integration example showing how to use the new markdown processing services
// together to create a complete markdown editor backend

use std::sync::Arc;
use std::path::Path;
use uuid::Uuid;
use tokio::sync::mpsc;

use super::{
    MarkdownTextProcessor, MarkdownProcessor, DocumentStateManager, MarkdownFormat, AutoSaveConfig,
    DocumentChange, ValidationError,
};
use super::markdown_processor::ProcessingStatistics as MarkdownProcessingStatistics;
use super::document_processing::ProcessingStatistics;

/// Complete markdown editor backend that integrates all services
pub struct MarkdownEditorBackend {
    state_manager: Arc<DocumentStateManager>,
    change_receiver: Option<mpsc::UnboundedReceiver<DocumentChange>>,
}

impl MarkdownEditorBackend {
    /// Create a new markdown editor backend
    pub async fn new(document_id: Option<Uuid>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let state_manager = Arc::new(DocumentStateManager::new(document_id).await?);
        
        // Subscribe to changes for UI updates
        let change_receiver = state_manager.subscribe_to_changes().await;
        
        Ok(Self {
            state_manager,
            change_receiver: Some(change_receiver),
        })
    }

    /// Load a markdown file
    pub async fn load_file(&self, file_path: &Path) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.state_manager.load_from_file(file_path).await?;
        Ok(self.state_manager.get_content().await)
    }

    /// Save the current document
    pub async fn save_file(&self, file_path: Option<&Path>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.state_manager.save_to_file(file_path).await?;
        Ok(())
    }

    /// Get current document content
    pub async fn get_content(&self) -> String {
        self.state_manager.get_content().await
    }

    /// Set document content
    pub async fn set_content(&self, content: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.state_manager.set_content(content).await?;
        Ok(())
    }

    /// Insert text at position (for UI integration)
    pub async fn insert_text(&self, position: usize, text: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.state_manager.insert_text(position, text).await?;
        Ok(())
    }

    /// Delete text range (for UI integration)
    pub async fn delete_range(&self, start: usize, end: usize) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.state_manager.delete_range(start, end).await?)
    }

    /// Apply markdown formatting to a text selection
    pub async fn apply_formatting(&self, start: usize, end: usize, format: MarkdownFormat) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // This is a simplified example - in practice you'd integrate more tightly
        // with the text processor to handle cursors and selections properly
        
        let content = self.state_manager.get_content().await;
        let selected_text = &content[start..end];
        
        // Format the text based on the format type
        let formatted_text = match format {
            MarkdownFormat::Bold => format!("**{}**", selected_text),
            MarkdownFormat::Italic => format!("*{}*", selected_text),
            MarkdownFormat::Code => format!("`{}`", selected_text),
            MarkdownFormat::Heading { level } => {
                let hash_count = "#".repeat(level as usize);
                format!("{} {}", hash_count, selected_text)
            },
            MarkdownFormat::Link { url, title: _ } => {
                format!("[{}]({})", selected_text, url)
            },
            _ => selected_text.to_string(), // Handle other formats as needed
        };
        
        // Replace the selected text
        self.state_manager.delete_range(start, end).await?;
        self.state_manager.insert_text(start, &formatted_text).await?;
        
        Ok(())
    }

    /// Validate the current document
    pub async fn validate_document(&self) -> Result<Vec<ValidationError>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.state_manager.validate().await?)
    }

    /// Get document statistics
    pub async fn get_statistics(&self) -> Result<ProcessingStatistics, Box<dyn std::error::Error + Send + Sync>> {
        // Convert from markdown ProcessingStatistics to document ProcessingStatistics
        let markdown_stats = self.state_manager.get_statistics().await?;
        Ok(ProcessingStatistics {
            total_files: 1, // Single document
            total_processing_time_ms: 0, // TODO: Track processing time
            average_processing_time_ms: 0, // TODO: Calculate average
            total_warnings: markdown_stats.warning_count,
            total_content_size: markdown_stats.character_count,
            supported_formats: vec!["markdown".to_string()],
        })
    }

    /// Configure auto-save
    pub async fn configure_auto_save(&self, config: AutoSaveConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.state_manager.configure_auto_save(config).await?;
        Ok(())
    }

    /// Create a version snapshot
    pub async fn create_snapshot(&self, description: String) -> Result<Uuid, Box<dyn std::error::Error + Send + Sync>> {
        let version = self.state_manager.create_version_snapshot(description, None).await?;
        Ok(version.id)
    }

    /// Restore from a version
    pub async fn restore_version(&self, version_id: Uuid) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.state_manager.restore_from_version(version_id).await?;
        Ok(())
    }

    /// Get change receiver for UI updates
    pub fn take_change_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<DocumentChange>> {
        self.change_receiver.take()
    }

    /// Check if document is modified
    pub async fn is_modified(&self) -> bool {
        self.state_manager.get_state().await.is_modified
    }

    /// Get memory usage information
    pub async fn get_memory_usage(&self) -> super::MemoryUsageInfo {
        self.state_manager.get_memory_usage().await
    }

    /// Optimize memory usage
    pub async fn optimize_memory(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.state_manager.optimize_memory().await?;
        Ok(())
    }
}

/// Example of how to integrate with a Slint UI callback system
pub struct SlintIntegration {
    backend: Arc<MarkdownEditorBackend>,
}

impl SlintIntegration {
    pub fn new(backend: Arc<MarkdownEditorBackend>) -> Self {
        Self { backend }
    }

    /// Handle text insertion from UI
    pub async fn handle_text_insert(&self, position: i32, text: String) -> Result<(), String> {
        self.backend
            .insert_text(position as usize, &text)
            .await
            .map_err(|e| e.to_string())
    }

    /// Handle text deletion from UI
    pub async fn handle_text_delete(&self, start: i32, end: i32) -> Result<String, String> {
        self.backend
            .delete_range(start as usize, end as usize)
            .await
            .map_err(|e| e.to_string())
    }

    /// Handle formatting from ribbon buttons
    pub async fn handle_bold_formatting(&self, start: i32, end: i32) -> Result<(), String> {
        self.backend
            .apply_formatting(start as usize, end as usize, MarkdownFormat::Bold)
            .await
            .map_err(|e| e.to_string())
    }

    /// Handle italic formatting
    pub async fn handle_italic_formatting(&self, start: i32, end: i32) -> Result<(), String> {
        self.backend
            .apply_formatting(start as usize, end as usize, MarkdownFormat::Italic)
            .await
            .map_err(|e| e.to_string())
    }

    /// Handle heading formatting
    pub async fn handle_heading_formatting(&self, start: i32, end: i32, level: i32) -> Result<(), String> {
        let level = (level as u8).clamp(1, 6);
        self.backend
            .apply_formatting(start as usize, end as usize, MarkdownFormat::Heading { level })
            .await
            .map_err(|e| e.to_string())
    }

    /// Handle link creation
    pub async fn handle_link_creation(&self, start: i32, end: i32, url: String) -> Result<(), String> {
        self.backend
            .apply_formatting(
                start as usize, 
                end as usize, 
                MarkdownFormat::Link { url, title: None }
            )
            .await
            .map_err(|e| e.to_string())
    }

    /// Get content for UI display
    pub async fn get_content(&self) -> String {
        self.backend.get_content().await
    }

    /// Handle file operations
    pub async fn handle_file_open(&self, file_path: String) -> Result<String, String> {
        self.backend
            .load_file(Path::new(&file_path))
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn handle_file_save(&self, file_path: Option<String>) -> Result<(), String> {
        let path = file_path.as_ref().map(|p| Path::new(p));
        self.backend
            .save_file(path)
            .await
            .map_err(|e| e.to_string())
    }

    /// Get validation errors for UI display
    pub async fn get_validation_errors(&self) -> Result<Vec<String>, String> {
        let errors = self.backend
            .validate_document()
            .await
            .map_err(|e| e.to_string())?;
        
        Ok(errors.into_iter().map(|e| e.message).collect())
    }

    /// Get document statistics for status bar
    pub async fn get_document_stats(&self) -> Result<DocumentStats, String> {
        let stats = self.backend
            .get_statistics()
            .await
            .map_err(|e| e.to_string())?;
        
        Ok(DocumentStats {
            word_count: 0, // TODO: Calculate from content
            character_count: stats.total_content_size,
            line_count: 0, // TODO: Calculate from content 
            link_count: 0, // TODO: Calculate from content
            image_count: 0, // TODO: Calculate from content
        })
    }
}

/// Simplified stats structure for UI display
#[derive(Debug, Clone)]
pub struct DocumentStats {
    pub word_count: usize,
    pub character_count: usize,
    pub line_count: usize,
    pub link_count: usize,
    pub image_count: usize,
}

/// Example of advanced text processing with the services
pub struct AdvancedMarkdownProcessor {
    text_processor: MarkdownTextProcessor,
    markdown_processor: MarkdownProcessor,
}

impl AdvancedMarkdownProcessor {
    pub fn new() -> Self {
        Self {
            text_processor: MarkdownTextProcessor::new(),
            markdown_processor: MarkdownProcessor::new(),
        }
    }

    /// Process markdown with advanced features
    pub async fn process_markdown(&mut self, content: &str) -> Result<ProcessedMarkdownResult, Box<dyn std::error::Error + Send + Sync>> {
        // Set content in text processor
        self.text_processor.set_content(content.to_string());
        
        // Parse with markdown processor
        let ast = self.markdown_processor.parse(content).await?;
        
        // Validate the content
        let validation_errors = self.markdown_processor.validate(content).await?;
        
        // Generate statistics
        let statistics = self.markdown_processor.generate_statistics(content).await?;
        
        // Find and highlight formatting
        let full_range = super::TextRange::new(0, content.len());
        let format_detections = self.markdown_processor.detect_formatting(content, full_range);
        
        Ok(ProcessedMarkdownResult {
            ast,
            validation_errors,
            statistics,
            format_detections,
        })
    }

    /// Find and replace with advanced options
    pub fn find_and_replace(&mut self, pattern: &str, replacement: &str, case_sensitive: bool) -> Result<u32, super::TextProcessorError> {
        let options = super::FindReplaceOptions {
            case_sensitive,
            whole_word: false,
            use_regex: false,
            multiline: true,
            scope: super::SearchScope::EntireDocument,
        };
        
        self.text_processor.replace_all(pattern, replacement, &options)
    }

    /// Apply formatting with conflict detection
    pub async fn apply_smart_formatting(&mut self, format: MarkdownFormat) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Collect cursor data to avoid borrowing conflicts
        let cursors_data: Vec<_> = self.text_processor.get_cursors().to_vec();
        let _content = self.text_processor.get_content();
        
        for cursor in cursors_data {
            if let Some(selection) = &cursor.selection {
                let range = super::TextRange::new(selection.start.offset, selection.end.offset);
                let current_content = self.text_processor.get_content();
                
                // Apply formatting with conflict resolution
                let result = self.markdown_processor
                    .apply_formatting_to_range(&current_content, range, format.clone())
                    .await?;
                
                // Update the text processor with the result
                self.text_processor.set_content(result);
            }
        }
        
        Ok(())
    }

    /// Get current content
    pub fn get_content(&self) -> &str {
        self.text_processor.get_content()
    }

    /// Undo last operation
    pub fn undo(&mut self) -> Result<bool, super::TextProcessorError> {
        self.text_processor.undo()
    }

    /// Redo last undone operation
    pub fn redo(&mut self) -> Result<bool, super::TextProcessorError> {
        self.text_processor.redo()
    }
}

/// Result of advanced markdown processing
pub struct ProcessedMarkdownResult {
    pub ast: super::MarkdownNode,
    pub validation_errors: Vec<ValidationError>,
    pub statistics: MarkdownProcessingStatistics,
    pub format_detections: Vec<super::FormatDetection>,
}

/// Example of performance monitoring
pub struct PerformanceMonitor {
    start_time: std::time::Instant,
    operation_name: String,
}

impl PerformanceMonitor {
    pub fn new(operation_name: impl Into<String>) -> Self {
        Self {
            start_time: std::time::Instant::now(),
            operation_name: operation_name.into(),
        }
    }

    pub fn finish(self) -> std::time::Duration {
        let duration = self.start_time.elapsed();
        println!("Operation '{}' completed in {:?}", self.operation_name, duration);
        duration
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

    #[tokio::test]
    async fn test_complete_workflow() {
        let backend = MarkdownEditorBackend::new(None).await.unwrap();
        
        // Set initial content
        let content = "# Test Document\n\nThis is a test paragraph.";
        backend.set_content(content.to_string()).await.unwrap();
        
        // Apply formatting
        backend.apply_formatting(16, 32, MarkdownFormat::Bold).await.unwrap();
        
        // Validate the result
        let final_content = backend.get_content().await;
        assert!(final_content.contains("**This is a test**"));
        
        // Check statistics
        let stats = backend.get_statistics().await.unwrap();
        assert_eq!(stats.heading_count.get(&1), Some(&1));
        assert!(stats.word_count > 0);
        
        // Create snapshot
        let snapshot_id = backend.create_snapshot("After formatting".to_string()).await.unwrap();
        
        // Modify content
        backend.set_content("# Modified Document".to_string()).await.unwrap();
        
        // Restore snapshot
        backend.restore_version(snapshot_id).await.unwrap();
        
        // Verify restoration
        let restored_content = backend.get_content().await;
        assert!(restored_content.contains("**This is a test**"));
    }

    #[tokio::test]
    async fn test_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");
        
        // Create test file
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "# Original Content\n\nThis is the original content.").unwrap();
        
        let backend = MarkdownEditorBackend::new(None).await.unwrap();
        
        // Load file
        let content = backend.load_file(&file_path).await.unwrap();
        assert!(content.contains("# Original Content"));
        
        // Modify content
        backend.set_content("# Modified Content\n\nThis is modified.".to_string()).await.unwrap();
        
        // Save file
        backend.save_file(Some(&file_path)).await.unwrap();
        
        // Verify file was saved
        let saved_content = std::fs::read_to_string(&file_path).unwrap();
        assert!(saved_content.contains("# Modified Content"));
    }

    #[tokio::test]
    async fn test_advanced_processing() {
        let mut processor = AdvancedMarkdownProcessor::new();
        
        let content = "This is **bold** and *italic* text with some normal text.";
        let result = processor.process_markdown(content).await.unwrap();
        
        // Check that formatting was detected
        assert!(!result.format_detections.is_empty());
        
        // Check statistics
        assert!(result.statistics.word_count > 0);
        assert_eq!(result.statistics.character_count, content.len());
        
        // Test find and replace
        let replaced_count = processor.find_and_replace("text", "content", false).unwrap();
        assert_eq!(replaced_count, 2);
        
        let final_content = processor.get_content();
        assert!(final_content.contains("content"));
        assert!(!final_content.contains("text"));
    }

    #[tokio::test]
    async fn test_slint_integration() {
        let backend = Arc::new(MarkdownEditorBackend::new(None).await.unwrap());
        let integration = SlintIntegration::new(backend);
        
        // Test text operations
        integration.handle_text_insert(0, "Hello ".to_string()).await.unwrap();
        integration.handle_text_insert(6, "World".to_string()).await.unwrap();
        
        let content = integration.get_content().await;
        assert_eq!(content, "Hello World");
        
        // Test formatting
        integration.handle_bold_formatting(0, 5).await.unwrap();
        
        let formatted_content = integration.get_content().await;
        assert!(formatted_content.contains("**Hello**"));
        
        // Test statistics
        let stats = integration.get_document_stats().await.unwrap();
        assert_eq!(stats.word_count, 2);
    }

    #[tokio::test]
    async fn test_performance_monitoring() {
        let backend = MarkdownEditorBackend::new(None).await.unwrap();
        
        let monitor = PerformanceMonitor::new("large_document_processing");
        
        // Create large content
        let large_content = "# Large Document\n\n".to_string() + &"This is a paragraph.\n\n".repeat(1000);
        backend.set_content(large_content).await.unwrap();
        
        let duration = monitor.finish();
        assert!(duration.as_millis() < 1000); // Should be fast
        
        // Test memory usage
        let memory_info = backend.get_memory_usage().await;
        assert!(memory_info.content_size > 10000); // Large content
        
        // Optimize memory
        backend.optimize_memory().await.unwrap();
    }

    #[tokio::test]
    async fn test_auto_save_configuration() {
        let backend = MarkdownEditorBackend::new(None).await.unwrap();
        
        let auto_save_config = AutoSaveConfig {
            enabled: true,
            interval_seconds: 10,
            max_idle_time_seconds: 60,
            save_on_change_count: Some(5),
            backup_directory: None,
            max_backup_files: 5,
            compress_backups: false,
        };
        
        backend.configure_auto_save(auto_save_config).await.unwrap();
        
        // The auto-save will run in the background
        // In a real application, you'd test this with actual file I/O
    }
}