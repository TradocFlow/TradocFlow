use std::sync::Arc;
use std::time::Instant;
use tokio::task::JoinSet;
use futures::stream::{FuturesUnordered, StreamExt};
use tokio::sync::{Semaphore, mpsc};
use serde::{Deserialize, Serialize};

use crate::{TradocumentError, Result};
use crate::services::simplified_document_import_service::{SimplifiedDocumentImportService, ImportConfig as SimplifiedImportConfig};
use crate::services::enhanced_multi_language_detector::{
    EnhancedMultiLanguageDetector, CompleteLanguageSet, DetectedLanguageDocument,
    BatchImportConfig, BatchImportResult, ProcessedDocument, ProcessedLanguageSet,
    FailedDocument, FailedLanguageSet, DocumentMetadata, BatchProcessingStatistics,
    ProcessingStrategy, IncompleteSetHandling, ErrorHandlingStrategy,
    ParallelProcessingConfig, MemoryConfig
};

/// Enhanced batch processor for multi-language document sets
/// Handles parallel processing, memory management, and error recovery
pub struct EnhancedBatchProcessor {
    /// Language detector for document classification
    detector: Arc<EnhancedMultiLanguageDetector>,
    /// Document import service for actual conversion
    import_service: Arc<SimplifiedDocumentImportService>,
    /// Processing configuration
    config: BatchImportConfig,
}

/// Progress information for batch processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProcessingProgress {
    /// Current processing stage
    pub stage: ProcessingStage,
    /// Overall progress percentage (0-100)
    pub overall_progress: u8,
    /// Number of language sets completed
    pub sets_completed: usize,
    /// Total number of language sets to process
    pub total_sets: usize,
    /// Number of individual documents completed
    pub documents_completed: usize,
    /// Total number of documents to process
    pub total_documents: usize,
    /// Current operation description
    pub current_operation: String,
    /// Processing warnings accumulated so far
    pub warnings: Vec<String>,
    /// Processing errors encountered
    pub errors: Vec<String>,
    /// Estimated time remaining (seconds)
    pub estimated_time_remaining_secs: Option<u64>,
}

/// Processing stage indicator
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProcessingStage {
    /// Initializing and validating input
    Initializing,
    /// Validating documents and language sets
    Validating,
    /// Processing documents in parallel
    Processing,
    /// Finalizing results and cleanup
    Finalizing,
    /// Processing completed
    Completed,
    /// Processing failed
    Failed,
}

/// Progress callback type for UI integration
pub type BatchProgressCallback = Arc<dyn Fn(BatchProcessingProgress) + Send + Sync>;

/// Task information for parallel processing
#[derive(Debug)]
struct ProcessingTask {
    task_id: String,
    task_type: TaskType,
    language_set: Option<CompleteLanguageSet>,
    document: Option<DetectedLanguageDocument>,
    priority: u8, // Higher number = higher priority
}

/// Type of processing task
#[derive(Debug, PartialEq)]
enum TaskType {
    /// Process a complete language set
    LanguageSet,
    /// Process an individual document
    IndividualDocument,
}

/// Task result from parallel processing
#[derive(Debug)]
enum TaskResult {
    /// Successfully processed language set
    ProcessedSet(ProcessedLanguageSet),
    /// Successfully processed individual document
    ProcessedDocument(ProcessedDocument),
    /// Failed to process language set
    FailedSet(FailedLanguageSet),
    /// Failed to process individual document
    FailedDocument(FailedDocument),
}

/// Memory usage tracker for processing
#[derive(Debug, Default)]
struct MemoryTracker {
    current_usage_mb: usize,
    peak_usage_mb: usize,
    document_count: usize,
}

impl EnhancedBatchProcessor {
    /// Create a new enhanced batch processor
    pub fn new(config: BatchImportConfig) -> Self {
        let detector = Arc::new(EnhancedMultiLanguageDetector::new());
        let import_service = Arc::new(SimplifiedDocumentImportService::new());
        
        Self {
            detector,
            import_service,
            config,
        }
    }
    
    /// Create with custom detector and import service
    pub fn with_services(
        detector: Arc<EnhancedMultiLanguageDetector>,
        import_service: Arc<SimplifiedDocumentImportService>,
        config: BatchImportConfig
    ) -> Self {
        Self {
            detector,
            import_service,
            config,
        }
    }
    
    /// Process multiple language sets with progress tracking
    pub async fn process_language_sets(
        &self,
        language_sets: Vec<CompleteLanguageSet>,
        individual_documents: Vec<DetectedLanguageDocument>,
        progress_callback: Option<BatchProgressCallback>,
    ) -> Result<BatchImportResult> {
        let start_time = Instant::now();
        
        // Initialize progress tracking
        let total_sets = language_sets.len();
        let total_documents = individual_documents.len() + 
            language_sets.iter().map(|set| set.documents.len()).sum::<usize>();
        
        let mut progress = BatchProcessingProgress {
            stage: ProcessingStage::Initializing,
            overall_progress: 0,
            sets_completed: 0,
            total_sets,
            documents_completed: 0,
            total_documents,
            current_operation: "Initializing batch processing...".to_string(),
            warnings: Vec::new(),
            errors: Vec::new(),
            estimated_time_remaining_secs: None,
        };
        
        self.report_progress(&progress, &progress_callback);
        
        // Validate input
        progress.stage = ProcessingStage::Validating;
        progress.current_operation = "Validating language sets and documents...".to_string();
        progress.overall_progress = 5;
        self.report_progress(&progress, &progress_callback);
        
        let validation_result = self.validate_input(&language_sets, &individual_documents);
        progress.warnings.extend(validation_result.warnings);
        if !validation_result.errors.is_empty() {
            progress.errors.extend(validation_result.errors);
            progress.stage = ProcessingStage::Failed;
            self.report_progress(&progress, &progress_callback);
            
            return match self.config.error_handling {
                ErrorHandlingStrategy::FailFast => {
                    Err(TradocumentError::Validation(format!(
                        "Input validation failed: {}", 
                        progress.errors.join("; ")
                    )))
                },
                _ => {
                    // Return partial result with validation errors
                    Ok(BatchImportResult {
                        successful_sets: Vec::new(),
                        successful_individual: Vec::new(),
                        failed_sets: Vec::new(),
                        failed_individual: Vec::new(),
                        statistics: BatchProcessingStatistics::default(),
                        overall_success: false,
                        processing_duration_ms: start_time.elapsed().as_millis() as u64,
                    })
                }
            };
        }
        
        // Determine processing strategy
        let strategy = self.determine_processing_strategy(&language_sets, &individual_documents);
        
        progress.stage = ProcessingStage::Processing;
        progress.current_operation = format!("Processing with {:?} strategy...", strategy);
        progress.overall_progress = 10;
        self.report_progress(&progress, &progress_callback);
        
        // Execute processing based on strategy
        let processing_result = match strategy {
            ProcessingStrategy::ParallelByLanguage => {
                self.process_parallel_by_language(language_sets, individual_documents, &progress_callback).await
            },
            ProcessingStrategy::ParallelByDocument => {
                self.process_parallel_by_document(language_sets, individual_documents, &progress_callback).await
            },
            ProcessingStrategy::SequentialBySets => {
                self.process_sequential_by_sets(language_sets, individual_documents, &progress_callback).await
            },
            ProcessingStrategy::Sequential => {
                self.process_sequential(language_sets, individual_documents, &progress_callback).await
            },
            ProcessingStrategy::Adaptive => {
                self.process_adaptive(language_sets, individual_documents, &progress_callback).await
            },
        };
        
        // Finalize results
        progress.stage = ProcessingStage::Finalizing;
        progress.current_operation = "Finalizing results and cleaning up...".to_string();
        progress.overall_progress = 95;
        self.report_progress(&progress, &progress_callback);
        
        let mut result = processing_result?;
        result.processing_duration_ms = start_time.elapsed().as_millis() as u64;
        
        // Update final progress
        progress.stage = ProcessingStage::Completed;
        progress.overall_progress = 100;
        progress.current_operation = "Processing completed".to_string();
        progress.sets_completed = result.successful_sets.len();
        progress.documents_completed = result.successful_individual.len() + 
            result.successful_sets.iter().map(|set| set.processed_documents.len()).sum::<usize>();
        self.report_progress(&progress, &progress_callback);
        
        Ok(result)
    }
    
    /// Validate input language sets and documents
    fn validate_input(
        &self,
        language_sets: &[CompleteLanguageSet],
        individual_documents: &[DetectedLanguageDocument]
    ) -> ValidationResult {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        
        // Validate language sets
        for language_set in language_sets {
            match self.detector.validate_language_set(language_set) {
                Ok(set_warnings) => warnings.extend(set_warnings),
                Err(e) => errors.push(format!(
                    "Language set '{}' validation failed: {}", 
                    language_set.base_name, e
                )),
            }
        }
        
        // Validate individual documents
        for doc in individual_documents {
            if doc.file_size == 0 {
                warnings.push(format!(
                    "Document {} is empty", 
                    doc.file_path.display()
                ));
            }
            
            if !doc.file_path.exists() {
                errors.push(format!(
                    "Document {} no longer exists", 
                    doc.file_path.display()
                ));
            }
        }
        
        // Check for memory constraints
        let total_size: u64 = language_sets.iter()
            .flat_map(|set| set.documents.values())
            .chain(individual_documents.iter())
            .map(|doc| doc.file_size)
            .sum();
        
        let estimated_memory_mb = (total_size / (1024 * 1024)) * 3; // Rough estimate: 3x file size
        let max_allowed_mb = (self.config.memory_config.max_memory_per_document_mb * 
            std::cmp::max(language_sets.len(), individual_documents.len())) as u64;
        
        if estimated_memory_mb > max_allowed_mb {
            warnings.push(format!(
                "Estimated memory usage ({} MB) may exceed limits ({} MB). Consider using streaming mode.",
                estimated_memory_mb, max_allowed_mb
            ));
        }
        
        ValidationResult { warnings, errors }
    }
    
    /// Determine the optimal processing strategy
    fn determine_processing_strategy(
        &self,
        language_sets: &[CompleteLanguageSet],
        individual_documents: &[DetectedLanguageDocument]
    ) -> ProcessingStrategy {
        match self.config.processing_strategy {
            ProcessingStrategy::Adaptive => {
                let total_documents = language_sets.iter().map(|set| set.documents.len()).sum::<usize>() + 
                    individual_documents.len();
                let total_file_size: u64 = language_sets.iter()
                    .flat_map(|set| set.documents.values())
                    .chain(individual_documents.iter())
                    .map(|doc| doc.file_size)
                    .sum();
                
                let avg_file_size_mb = total_file_size / (1024 * 1024 * total_documents as u64).max(1);
                let cpu_count = num_cpus::get();
                
                // Adaptive logic
                if total_documents <= 2 {
                    ProcessingStrategy::Sequential
                } else if total_documents <= cpu_count * 2 && avg_file_size_mb < 10 {
                    ProcessingStrategy::ParallelByDocument
                } else if language_sets.len() > 1 {
                    ProcessingStrategy::ParallelByLanguage
                } else {
                    ProcessingStrategy::SequentialBySets
                }
            },
            strategy => strategy
        }
    }
    
    /// Process language sets in parallel by language
    async fn process_parallel_by_language(
        &self,
        language_sets: Vec<CompleteLanguageSet>,
        individual_documents: Vec<DetectedLanguageDocument>,
        progress_callback: &Option<BatchProgressCallback>,
    ) -> Result<BatchImportResult> {
        let mut successful_sets = Vec::new();
        let mut successful_individual = Vec::new();
        let mut failed_sets = Vec::new();
        let mut failed_individual = Vec::new();
        let mut memory_tracker = MemoryTracker::default();
        
        // Create semaphore for language-based concurrency control
        let language_semaphore = Arc::new(Semaphore::new(
            self.config.parallel_processing.max_concurrent_languages
        ));
        
        // Process language sets
        let mut set_futures = FuturesUnordered::new();
        
        for language_set in language_sets {
            let detector = Arc::clone(&self.detector);
            let import_service = Arc::clone(&self.import_service);
            let semaphore = Arc::clone(&language_semaphore);
            let config = self.config.clone();
            
            set_futures.push(async move {
                let _permit = semaphore.acquire().await.unwrap();
                Self::process_language_set_parallel(
                    detector, 
                    import_service, 
                    language_set, 
                    &config
                ).await
            });
        }
        
        // Process individual documents
        let mut doc_futures = FuturesUnordered::new();
        
        for document in individual_documents {
            let import_service = Arc::clone(&self.import_service);
            let semaphore = Arc::clone(&language_semaphore);
            let config = self.config.clone();
            
            doc_futures.push(async move {
                let _permit = semaphore.acquire().await.unwrap();
                Self::process_individual_document(import_service, document, &config).await
            });
        }
        
        // Collect results from language sets
        while let Some(result) = set_futures.next().await {
            match result {
                Ok(TaskResult::ProcessedSet(processed_set)) => {
                    memory_tracker.peak_usage_mb = memory_tracker.peak_usage_mb.max(
                        processed_set.processed_documents.len() * 10 // Estimate 10MB per doc
                    );
                    successful_sets.push(processed_set);
                },
                Ok(TaskResult::FailedSet(failed_set)) => {
                    failed_sets.push(failed_set);
                },
                _ => unreachable!("Unexpected result type from language set processing"),
            }
        }
        
        // Collect results from individual documents
        while let Some(result) = doc_futures.next().await {
            match result {
                Ok(TaskResult::ProcessedDocument(processed_doc)) => {
                    memory_tracker.peak_usage_mb = memory_tracker.peak_usage_mb.max(10); // Estimate
                    successful_individual.push(processed_doc);
                },
                Ok(TaskResult::FailedDocument(failed_doc)) => {
                    failed_individual.push(failed_doc);
                },
                _ => unreachable!("Unexpected result type from individual document processing"),
            }
        }
        
        let statistics = self.calculate_statistics(
            &successful_sets, 
            &successful_individual,
            &failed_sets,
            &failed_individual,
            memory_tracker.peak_usage_mb
        );
        
        Ok(BatchImportResult {
            successful_sets,
            successful_individual,
            failed_sets,
            failed_individual,
            statistics,
            overall_success: failed_sets.is_empty() && failed_individual.is_empty(),
            processing_duration_ms: 0, // Will be set by caller
        })
    }
    
    /// Process a language set in parallel by language
    async fn process_language_set_parallel(
        detector: Arc<EnhancedMultiLanguageDetector>,
        import_service: Arc<SimplifiedDocumentImportService>,
        language_set: CompleteLanguageSet,
        config: &BatchImportConfig,
    ) -> Result<TaskResult> {
        let start_time = Instant::now();
        let mut processed_documents = std::collections::HashMap::new();
        let mut warnings = Vec::new();
        
        // Validate the language set first
        match detector.validate_language_set(&language_set) {
            Ok(set_warnings) => warnings.extend(set_warnings),
            Err(e) => {
                return Ok(TaskResult::FailedSet(FailedLanguageSet {
                    language_set,
                    error: e.to_string(),
                    partial_results: std::collections::HashMap::new(),
                }));
            }
        }
        
        // Process documents in the set in parallel
        let mut document_futures = FuturesUnordered::new();
        
        for (lang_code, document) in language_set.documents.clone() {
            let import_service_clone = Arc::clone(&import_service);
            let config_clone = config.clone();
            
            document_futures.push(async move {
                (lang_code, Self::process_single_document(
                    import_service_clone, 
                    document, 
                    &config_clone
                ).await)
            });
        }
        
        // Collect results
        let mut processing_errors = Vec::new();
        
        while let Some((lang_code, result)) = document_futures.next().await {
            match result {
                Ok(processed_doc) => {
                    processed_documents.insert(lang_code, processed_doc);
                },
                Err(e) => {
                    processing_errors.push(format!("{}: {}", lang_code, e));
                }
            }
        }
        
        // Handle errors based on strategy
        if !processing_errors.is_empty() {
            match config.error_handling {
                ErrorHandlingStrategy::FailFast => {
                    return Ok(TaskResult::FailedSet(FailedLanguageSet {
                        language_set,
                        error: processing_errors.join("; "),
                        partial_results: processed_documents,
                    }));
                },
                ErrorHandlingStrategy::ContinueOnError | 
                ErrorHandlingStrategy::SkipProblematic => {
                    warnings.extend(processing_errors.iter().map(|e| format!("Error processing document: {}", e)));
                },
                ErrorHandlingStrategy::RetryOnce => {
                    // TODO: Implement retry logic
                    warnings.extend(processing_errors.iter().map(|e| format!("Error processing document (no retry implemented): {}", e)));
                }
            }
        }
        
        let processing_time = start_time.elapsed().as_millis() as u64;
        
        Ok(TaskResult::ProcessedSet(ProcessedLanguageSet {
            language_set,
            processed_documents,
            processing_time_ms: processing_time,
            warnings,
        }))
    }
    
    /// Process an individual document
    async fn process_individual_document(
        import_service: Arc<SimplifiedDocumentImportService>,
        document: DetectedLanguageDocument,
        config: &BatchImportConfig,
    ) -> Result<TaskResult> {
        match Self::process_single_document(import_service, document.clone(), config).await {
            Ok(processed_doc) => Ok(TaskResult::ProcessedDocument(processed_doc)),
            Err(e) => Ok(TaskResult::FailedDocument(FailedDocument {
                document,
                error: e.to_string(),
            }))
        }
    }
    
    /// Process a single document using the import service
    async fn process_single_document(
        import_service: Arc<SimplifiedDocumentImportService>,
        document: DetectedLanguageDocument,
        config: &BatchImportConfig,
    ) -> Result<ProcessedDocument> {
        let start_time = Instant::now();
        
        // Create import configuration
        let import_config = SimplifiedImportConfig {
            preserve_formatting: true,
            extract_images: false, // TODO: Make configurable
            chapter_mode: false,
            target_language: document.language_code.clone(),
        };
        
        // Import the document
        let import_result = import_service.import_document(&document.file_path, &import_config).await?;
        
        // Extract metadata (basic implementation)
        let metadata = DocumentMetadata {
            format: document.file_path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("unknown")
                .to_string(),
            word_count: import_result.content.split_whitespace().count(),
            character_count: import_result.content.len(),
            image_count: 0, // TODO: Extract from import result
            table_count: 0, // TODO: Extract from import result
            created_at: None, // TODO: Extract from document properties
            custom_properties: std::collections::HashMap::new(),
        };
        
        let processing_time = start_time.elapsed().as_millis() as u64;
        
        Ok(ProcessedDocument {
            original: document,
            content: import_result.content,
            title: import_result.title,
            messages: import_result.messages,
            warnings: import_result.warnings,
            processing_time_ms: processing_time,
            metadata,
        })
    }
    
    /// Placeholder for other processing strategies
    async fn process_parallel_by_document(
        &self,
        language_sets: Vec<CompleteLanguageSet>,
        individual_documents: Vec<DetectedLanguageDocument>,
        _progress_callback: &Option<BatchProgressCallback>,
    ) -> Result<BatchImportResult> {
        // TODO: Implement parallel by document strategy
        self.process_sequential(language_sets, individual_documents, _progress_callback).await
    }
    
    async fn process_sequential_by_sets(
        &self,
        language_sets: Vec<CompleteLanguageSet>,
        individual_documents: Vec<DetectedLanguageDocument>,
        _progress_callback: &Option<BatchProgressCallback>,
    ) -> Result<BatchImportResult> {
        // TODO: Implement sequential by sets strategy
        self.process_sequential(language_sets, individual_documents, _progress_callback).await
    }
    
    async fn process_sequential(
        &self,
        language_sets: Vec<CompleteLanguageSet>,
        individual_documents: Vec<DetectedLanguageDocument>,
        _progress_callback: &Option<BatchProgressCallback>,
    ) -> Result<BatchImportResult> {
        // TODO: Implement sequential processing strategy
        // For now, return empty result
        Ok(BatchImportResult {
            successful_sets: Vec::new(),
            successful_individual: Vec::new(),
            failed_sets: Vec::new(),
            failed_individual: Vec::new(),
            statistics: BatchProcessingStatistics::default(),
            overall_success: true,
            processing_duration_ms: 0,
        })
    }
    
    async fn process_adaptive(
        &self,
        language_sets: Vec<CompleteLanguageSet>,
        individual_documents: Vec<DetectedLanguageDocument>,
        progress_callback: &Option<BatchProgressCallback>,
    ) -> Result<BatchImportResult> {
        // Use parallel by language as the adaptive strategy for now
        self.process_parallel_by_language(language_sets, individual_documents, progress_callback).await
    }
    
    /// Calculate processing statistics
    fn calculate_statistics(
        &self,
        successful_sets: &[ProcessedLanguageSet],
        successful_individual: &[ProcessedDocument],
        failed_sets: &[FailedLanguageSet],
        failed_individual: &[FailedDocument],
        peak_memory_mb: usize,
    ) -> BatchProcessingStatistics {
        let total_successful_docs = successful_sets.iter()
            .map(|set| set.processed_documents.len())
            .sum::<usize>() + successful_individual.len();
        
        let total_processing_time_ms = successful_sets.iter()
            .map(|set| set.processing_time_ms)
            .sum::<u64>() + successful_individual.iter()
            .map(|doc| doc.processing_time_ms)
            .sum::<u64>();
        
        let average_processing_time = if total_successful_docs > 0 {
            total_processing_time_ms / total_successful_docs as u64
        } else {
            0
        };
        
        let total_content_chars = successful_sets.iter()
            .flat_map(|set| set.processed_documents.values())
            .chain(successful_individual.iter())
            .map(|doc| doc.content.len())
            .sum();
        
        let mut languages_processed = std::collections::HashMap::new();
        for set in successful_sets {
            for lang in set.processed_documents.keys() {
                *languages_processed.entry(lang.clone()).or_insert(0) += 1;
            }
        }
        for doc in successful_individual {
            *languages_processed.entry(doc.original.language_code.clone()).or_insert(0) += 1;
        }
        
        BatchProcessingStatistics {
            total_sets_processed: successful_sets.len() + failed_sets.len(),
            total_individual_documents: successful_individual.len() + failed_individual.len(),
            successful_sets: successful_sets.len(),
            successful_individual: successful_individual.len(),
            failed_sets: failed_sets.len(),
            failed_individual: failed_individual.len(),
            total_processing_time_ms,
            average_processing_time_per_document_ms: average_processing_time,
            peak_memory_usage_mb: peak_memory_mb,
            total_content_extracted_chars: total_content_chars,
            languages_processed,
        }
    }
    
    /// Report progress to callback
    fn report_progress(
        &self,
        progress: &BatchProcessingProgress,
        callback: &Option<BatchProgressCallback>
    ) {
        if let Some(ref callback) = callback {
            callback(progress.clone());
        }
    }
}

/// Result of input validation
#[derive(Debug)]
struct ValidationResult {
    warnings: Vec<String>,
    errors: Vec<String>,
}

impl Default for EnhancedBatchProcessor {
    fn default() -> Self {
        Self::new(BatchImportConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;
    
    #[tokio::test]
    async fn test_batch_processor_creation() {
        let processor = EnhancedBatchProcessor::default();
        assert!(!processor.config.preserve_structure);
    }
    
    #[tokio::test]
    async fn test_processing_strategy_determination() {
        let processor = EnhancedBatchProcessor::default();
        
        // Test with small number of documents
        let strategy = processor.determine_processing_strategy(&vec![], &vec![]);
        assert_eq!(strategy, ProcessingStrategy::Sequential);
    }
    
    #[tokio::test]
    async fn test_input_validation() {
        let processor = EnhancedBatchProcessor::default();
        let temp_dir = TempDir::new().unwrap();
        
        // Create a test document
        let file_path = temp_dir.path().join("test_en.docx");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Test content").unwrap();
        
        let detector = EnhancedMultiLanguageDetector::new();
        let test_doc = detector.analyze_file(&file_path).await.unwrap();
        
        let result = processor.validate_input(&vec![], &vec![test_doc]);
        assert!(result.errors.is_empty());
    }
}