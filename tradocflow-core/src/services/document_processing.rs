use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;

use crate::services::simplified_document_import_service::{
    SimplifiedDocumentImportService, ImportConfig, FileValidationResult
};
use crate::TradocumentError;

/// Progress callback type for UI integration
pub type ProgressCallback = Arc<dyn Fn(ImportProgressInfo) + Send + Sync>;

/// Enhanced progress information for UI display
#[derive(Debug, Clone)]
pub struct ImportProgressInfo {
    pub current_file: String,
    pub progress_percent: u8,
    pub message: String,
    pub stage: ImportStage,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// Different stages of the import process
#[derive(Debug, Clone, PartialEq)]
pub enum ImportStage {
    Validating,
    Processing,
    Converting,
    Finalizing,
    Completed,
    Failed,
}

/// Configuration for the document processing
#[derive(Debug, Clone)]
pub struct DocumentProcessingConfig {
    pub preserve_formatting: bool,
    pub extract_images: bool,
    pub target_language: String,
    pub timeout_seconds: u64,
    pub max_file_size_mb: u64,
}

impl Default for DocumentProcessingConfig {
    fn default() -> Self {
        Self {
            preserve_formatting: true,
            extract_images: false,
            target_language: "en".to_string(),
            timeout_seconds: 300, // 5 minutes
            max_file_size_mb: 50,  // 50 MB
        }
    }
}

/// Document processing service with UI integration
pub struct DocumentProcessingService {
    import_service: SimplifiedDocumentImportService,
    runtime: Runtime,
}

impl DocumentProcessingService {
    /// Create a new document processing service
    pub fn new() -> Result<Self, TradocumentError> {
        let runtime = Runtime::new()
            .map_err(|e| TradocumentError::IoError(e))?;
            
        Ok(Self {
            import_service: SimplifiedDocumentImportService::new(),
            runtime,
        })
    }

    /// Process a single document file with progress callbacks
    pub fn process_document(
        &self,
        file_path: &Path,
        config: DocumentProcessingConfig,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<ProcessedDocument, TradocumentError> {
        let file_path = file_path.to_path_buf();
        let filename = file_path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Send initial progress
        if let Some(ref callback) = progress_callback {
            callback(ImportProgressInfo {
                current_file: filename.clone(),
                progress_percent: 0,
                message: "Starting import...".to_string(),
                stage: ImportStage::Validating,
                warnings: Vec::new(),
                errors: Vec::new(),
            });
        }

        // Validate file first
        let validation_result = self.validate_file(&file_path)?;
        if !validation_result.is_valid {
            return Err(TradocumentError::Validation(
                format!("File validation failed: {}", validation_result.errors.join(", "))
            ));
        }

        // Send validation complete progress
        if let Some(ref callback) = progress_callback {
            callback(ImportProgressInfo {
                current_file: filename.clone(),
                progress_percent: 25,
                message: "File validation completed".to_string(),
                stage: ImportStage::Processing,
                warnings: validation_result.warnings.clone(),
                errors: Vec::new(),
            });
        }

        // Create import config
        let import_config = ImportConfig {
            preserve_formatting: config.preserve_formatting,
            extract_images: config.extract_images,
            chapter_mode: false,
            target_language: config.target_language.clone(),
        };

        // Process the document asynchronously with timeout
        let result = self.runtime.block_on(async {
            // Create a timeout for the operation
            let import_future = self.import_service.import_document(&file_path, &import_config);
            
            match tokio::time::timeout(
                Duration::from_secs(config.timeout_seconds),
                import_future
            ).await {
                Ok(result) => result,
                Err(_) => Err(TradocumentError::DocumentImport(
                    "Import operation timed out".to_string()
                ))
            }
        })?;

        // Send processing complete progress
        if let Some(ref callback) = progress_callback {
            callback(ImportProgressInfo {
                current_file: filename.clone(),
                progress_percent: 75,
                message: "Document processing completed".to_string(),
                stage: ImportStage::Converting,
                warnings: result.warnings.clone(),
                errors: Vec::new(),
            });
        }

        // Convert to ProcessedDocument
        let processed_doc = ProcessedDocument {
            id: Uuid::new_v4(),
            filename: result.filename,
            title: result.title,
            content: result.content,
            language: result.language,
            processing_time_ms: result.processing_time_ms,
            messages: result.messages,
            warnings: result.warnings,
            file_size: validation_result.file_size,
            detected_format: validation_result.detected_format,
        };

        // Send completion progress
        if let Some(ref callback) = progress_callback {
            callback(ImportProgressInfo {
                current_file: filename.clone(),
                progress_percent: 100,
                message: "Import completed successfully".to_string(),
                stage: ImportStage::Completed,
                warnings: processed_doc.warnings.clone(),
                errors: Vec::new(),
            });
        }

        Ok(processed_doc)
    }

    /// Process multiple documents with batch progress reporting
    pub fn process_documents_batch(
        &self,
        file_paths: Vec<std::path::PathBuf>,
        config: DocumentProcessingConfig,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<BatchProcessResult, TradocumentError> {
        let total_files = file_paths.len();
        let mut successful_imports = Vec::new();
        let mut failed_imports = Vec::new();
        let start_time = std::time::Instant::now();

        for (index, file_path) in file_paths.iter().enumerate() {
            let filename = file_path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Calculate overall progress
            let overall_progress = ((index as f32 / total_files as f32) * 100.0) as u8;

            // Send batch progress
            if let Some(ref callback) = progress_callback {
                callback(ImportProgressInfo {
                    current_file: filename.clone(),
                    progress_percent: overall_progress,
                    message: format!("Processing file {} of {}: {}", index + 1, total_files, filename),
                    stage: ImportStage::Processing,
                    warnings: Vec::new(),
                    errors: Vec::new(),
                });
            }

            // Process individual file
            match self.process_document(file_path, config.clone(), None) {
                Ok(processed_doc) => {
                    successful_imports.push(processed_doc);
                }
                Err(e) => {
                    failed_imports.push(BatchImportError {
                        filename,
                        error: e.to_string(),
                    });
                }
            }
        }

        let processing_time = start_time.elapsed().as_millis() as u64;

        // Send final progress
        if let Some(ref callback) = progress_callback {
            let stage = if failed_imports.is_empty() {
                ImportStage::Completed
            } else if successful_imports.is_empty() {
                ImportStage::Failed
            } else {
                ImportStage::Completed
            };

            callback(ImportProgressInfo {
                current_file: "Batch Complete".to_string(),
                progress_percent: 100,
                message: format!("Processed {} of {} files successfully", 
                    successful_imports.len(), total_files),
                stage,
                warnings: Vec::new(),
                errors: failed_imports.iter().map(|e| e.error.clone()).collect(),
            });
        }

        Ok(BatchProcessResult {
            total_files,
            successful_imports,
            failed_imports,
            processing_time_ms: processing_time,
        })
    }

    /// Validate a file before processing
    pub fn validate_file(&self, file_path: &Path) -> Result<FileValidationResult, TradocumentError> {
        self.import_service.validate_file(file_path)
    }

    /// Get supported file formats
    pub fn supported_formats(&self) -> &[String] {
        self.import_service.supported_formats()
    }

    /// Check if a file format is supported
    pub fn is_format_supported(&self, filename: &str) -> bool {
        self.import_service.is_format_supported(filename)
    }

    /// Detect file format from content
    pub fn detect_format(&self, file_path: &Path) -> Result<String, TradocumentError> {
        self.import_service.detect_format_from_content(file_path)
    }

    /// Get processing statistics
    pub fn get_processing_statistics(&self, results: &[ProcessedDocument]) -> ProcessingStatistics {
        let total_files = results.len();
        let total_processing_time: u64 = results.iter().map(|r| r.processing_time_ms).sum();
        let total_warnings = results.iter().map(|r| r.warnings.len()).sum();
        let average_processing_time = if total_files > 0 {
            total_processing_time / total_files as u64
        } else {
            0
        };
        let total_content_size: usize = results.iter().map(|r| r.content.len()).sum();

        ProcessingStatistics {
            total_files,
            total_processing_time_ms: total_processing_time,
            average_processing_time_ms: average_processing_time,
            total_warnings,
            total_content_size,
            supported_formats: self.supported_formats().to_vec(),
        }
    }
}

/// Result of processing a single document
#[derive(Debug, Clone)]
pub struct ProcessedDocument {
    pub id: Uuid,
    pub filename: String,
    pub title: String,
    pub content: String,
    pub language: String,
    pub processing_time_ms: u64,
    pub messages: Vec<String>,
    pub warnings: Vec<String>,
    pub file_size: u64,
    pub detected_format: Option<String>,
}

/// Result of batch processing multiple documents
#[derive(Debug, Clone)]
pub struct BatchProcessResult {
    pub total_files: usize,
    pub successful_imports: Vec<ProcessedDocument>,
    pub failed_imports: Vec<BatchImportError>,
    pub processing_time_ms: u64,
}

/// Error information for failed batch imports
#[derive(Debug, Clone)]
pub struct BatchImportError {
    pub filename: String,
    pub error: String,
}

/// Statistics about document processing operations
#[derive(Debug, Clone)]
pub struct ProcessingStatistics {
    pub total_files: usize,
    pub total_processing_time_ms: u64,
    pub average_processing_time_ms: u64,
    pub total_warnings: usize,
    pub total_content_size: usize,
    pub supported_formats: Vec<String>,
}

/// Thread-safe document processor for UI integration
#[derive(Clone)]
pub struct ThreadSafeDocumentProcessor {
    processor: Arc<Mutex<DocumentProcessingService>>,
}

impl ThreadSafeDocumentProcessor {
    /// Create a new thread-safe document processor
    pub fn new() -> Result<Self, TradocumentError> {
        let processor = DocumentProcessingService::new()?;
        Ok(Self {
            processor: Arc::new(Mutex::new(processor)),
        })
    }

    /// Process document in a separate thread with progress callbacks
    pub fn process_document_async(
        &self,
        file_path: std::path::PathBuf,
        config: DocumentProcessingConfig,
        progress_callback: ProgressCallback,
    ) -> mpsc::Receiver<Result<ProcessedDocument, TradocumentError>> {
        let (sender, receiver) = mpsc::channel();
        let processor = Arc::clone(&self.processor);

        thread::spawn(move || {
            let result = {
                let processor_guard = processor.lock().unwrap();
                processor_guard.process_document(&file_path, config, Some(progress_callback))
            };
            
            let _ = sender.send(result);
        });

        receiver
    }

    /// Process multiple documents in a separate thread
    pub fn process_documents_batch_async(
        &self,
        file_paths: Vec<std::path::PathBuf>,
        config: DocumentProcessingConfig,
        progress_callback: ProgressCallback,
    ) -> mpsc::Receiver<Result<BatchProcessResult, TradocumentError>> {
        let (sender, receiver) = mpsc::channel();
        let processor = Arc::clone(&self.processor);

        thread::spawn(move || {
            let result = {
                let processor_guard = processor.lock().unwrap();
                processor_guard.process_documents_batch(file_paths, config, Some(progress_callback))
            };
            
            let _ = sender.send(result);
        });

        receiver
    }

    /// Validate file synchronously (safe for UI thread)
    pub fn validate_file_sync(&self, file_path: &Path) -> Result<FileValidationResult, TradocumentError> {
        let processor = self.processor.lock().unwrap();
        processor.validate_file(file_path)
    }

    /// Get supported formats (safe for UI thread)
    pub fn supported_formats(&self) -> Vec<String> {
        let processor = self.processor.lock().unwrap();
        processor.supported_formats().to_vec()
    }

    /// Check format support (safe for UI thread)
    pub fn is_format_supported(&self, filename: &str) -> bool {
        let processor = self.processor.lock().unwrap();
        processor.is_format_supported(filename)
    }
    
    /// Process document synchronously (blocking operation)
    pub fn process_document_sync(
        &self,
        file_path: &Path,
        config: DocumentProcessingConfig,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<ProcessedDocument, TradocumentError> {
        let processor = self.processor.lock().unwrap();
        processor.process_document(file_path, config, progress_callback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_document_processor_creation() {
        let processor = DocumentProcessingService::new();
        assert!(processor.is_ok());
    }

    #[test]
    fn test_supported_formats() {
        let processor = DocumentProcessingService::new().unwrap();
        let formats = processor.supported_formats();
        assert!(formats.contains(&"docx".to_string()));
        assert!(formats.contains(&"txt".to_string()));
        assert!(formats.contains(&"md".to_string()));
    }

    #[test]
    fn test_format_support_detection() {
        let processor = DocumentProcessingService::new().unwrap();
        assert!(processor.is_format_supported("document.docx"));
        assert!(processor.is_format_supported("file.txt"));
        assert!(!processor.is_format_supported("image.png"));
    }

    #[test]
    fn test_file_validation() {
        let processor = DocumentProcessingService::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Test content").unwrap();

        let validation = processor.validate_file(&file_path).unwrap();
        assert!(validation.is_valid);
        assert_eq!(validation.detected_format, Some("txt".to_string()));
        assert!(validation.file_size > 0);
    }

    #[test]
    fn test_thread_safe_processor() {
        let processor = ThreadSafeDocumentProcessor::new();
        assert!(processor.is_ok());
    }

    #[test]
    fn test_processing_config_default() {
        let config = DocumentProcessingConfig::default();
        assert_eq!(config.target_language, "en");
        assert!(config.preserve_formatting);
        assert!(!config.extract_images);
        assert_eq!(config.timeout_seconds, 300);
        assert_eq!(config.max_file_size_mb, 50);
    }
}