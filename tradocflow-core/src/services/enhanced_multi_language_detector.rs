use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use std::fs;
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{TradocumentError, Result};

/// Enhanced multi-language detection system for Word document imports
/// Supports various naming patterns and folder scanning capabilities
pub struct EnhancedMultiLanguageDetector {
    /// Supported languages with their patterns
    language_patterns: HashMap<String, LanguagePattern>,
    /// File validation regex patterns
    validation_patterns: Vec<Regex>,
}

/// Language pattern configuration for detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguagePattern {
    /// Language code (e.g., "en", "de", "es")
    pub code: String,
    /// Display name (e.g., "English", "German", "Spanish")
    pub display_name: String,
    /// All possible patterns for this language
    pub patterns: Vec<String>,
    /// ISO language codes that map to this language
    pub iso_codes: Vec<String>,
    /// Full language names that map to this language
    pub full_names: Vec<String>,
}

/// Result of multi-language folder scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderScanResult {
    /// Root folder that was scanned
    pub root_folder: PathBuf,
    /// Documents grouped by detected language
    pub language_groups: HashMap<String, Vec<DetectedLanguageDocument>>,
    /// Files that couldn't be language-detected
    pub unclassified_files: Vec<PathBuf>,
    /// Complete language sets found (all 5 languages present)
    pub complete_sets: Vec<CompleteLanguageSet>,
    /// Missing languages per document group
    pub missing_languages: HashMap<String, Vec<String>>,
    /// Scan statistics
    pub scan_stats: ScanStatistics,
    /// Validation warnings and errors
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// A document with detected language information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedLanguageDocument {
    /// Full file path
    pub file_path: PathBuf,
    /// Detected language code
    pub language_code: String,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,
    /// Detection method used
    pub detection_method: DetectionMethod,
    /// Base document name (without language suffix)
    pub base_name: String,
    /// File size in bytes
    pub file_size: u64,
    /// Last modified timestamp
    pub modified_at: std::time::SystemTime,
}

/// Complete set of 5 language documents (EN, DE, ES, FR, NL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteLanguageSet {
    /// Set identifier (based on base document name)
    pub set_id: String,
    /// Base document name without language codes
    pub base_name: String,
    /// Documents by language code
    pub documents: HashMap<String, DetectedLanguageDocument>,
    /// Total files in set
    pub file_count: usize,
    /// Whether this is a perfect set (all 5 languages)
    pub is_complete: bool,
    /// Missing languages if incomplete
    pub missing_languages: Vec<String>,
}

/// Method used for language detection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DetectionMethod {
    /// Detected from filename pattern (highest confidence)
    FilenamePattern { pattern: String },
    /// Detected from folder structure
    FolderStructure,
    /// Detected from file content analysis
    ContentAnalysis,
    /// Manual assignment
    Manual,
    /// Failed to detect
    Unknown,
}

/// Scanning statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanStatistics {
    pub total_files_scanned: usize,
    pub supported_files_found: usize,
    pub languages_detected: HashMap<String, usize>,
    pub complete_sets_found: usize,
    pub incomplete_sets_found: usize,
    pub scan_duration_ms: u64,
    pub average_confidence: f32,
}

/// Configuration for folder scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderScanConfig {
    /// Whether to scan subdirectories recursively
    pub recursive: bool,
    /// Maximum depth for recursive scanning
    pub max_depth: Option<usize>,
    /// File extensions to include
    pub include_extensions: Vec<String>,
    /// File extensions to exclude
    pub exclude_extensions: Vec<String>,
    /// Patterns to exclude from scanning
    pub exclude_patterns: Vec<String>,
    /// Minimum confidence threshold for detection
    pub min_confidence: f32,
    /// Whether to require complete language sets
    pub require_complete_sets: bool,
    /// Languages that must be present in complete sets
    pub required_languages: Vec<String>,
}

/// Batch import configuration for multi-language processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchImportConfig {
    /// Processing strategy for language sets
    pub processing_strategy: ProcessingStrategy,
    /// How to handle incomplete sets
    pub incomplete_set_handling: IncompleteSetHandling,
    /// Whether to preserve original file structure
    pub preserve_structure: bool,
    /// Parallel processing configuration
    pub parallel_processing: ParallelProcessingConfig,
    /// Memory management settings
    pub memory_config: MemoryConfig,
    /// Error handling strategy
    pub error_handling: ErrorHandlingStrategy,
}

/// Strategy for processing multiple language documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingStrategy {
    /// Process all languages in parallel
    ParallelByLanguage,
    /// Process all documents in parallel
    ParallelByDocument,
    /// Process complete sets sequentially
    SequentialBySets,
    /// Process everything sequentially
    Sequential,
    /// Use adaptive strategy based on system resources
    Adaptive,
}

/// How to handle incomplete language sets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IncompleteSetHandling {
    /// Skip incomplete sets entirely
    Skip,
    /// Process incomplete sets with warnings
    ProcessWithWarnings,
    /// Process incomplete sets as individual documents
    ProcessIndividually,
    /// Fail the entire batch if any set is incomplete
    FailBatch,
}

/// Configuration for parallel processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelProcessingConfig {
    /// Maximum number of concurrent document processors
    pub max_concurrent_documents: usize,
    /// Maximum number of concurrent language processors
    pub max_concurrent_languages: usize,
    /// Processing timeout per document (seconds)
    pub document_timeout_secs: u64,
    /// Whether to use thread pool for CPU-intensive tasks
    pub use_thread_pool: bool,
}

/// Memory management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Maximum memory usage per document (MB)
    pub max_memory_per_document_mb: usize,
    /// Whether to use streaming for large documents
    pub use_streaming: bool,
    /// Buffer size for streaming (KB)
    pub stream_buffer_size_kb: usize,
    /// Whether to cleanup temporary files immediately
    pub immediate_cleanup: bool,
}

/// Strategy for handling errors during batch processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorHandlingStrategy {
    /// Stop on first error (all-or-nothing)
    FailFast,
    /// Continue processing, collect all errors
    ContinueOnError,
    /// Retry failed documents once
    RetryOnce,
    /// Skip problematic documents, warn user
    SkipProblematic,
}

/// Result of batch import processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchImportResult {
    /// Successfully processed complete language sets
    pub successful_sets: Vec<ProcessedLanguageSet>,
    /// Successfully processed individual documents
    pub successful_individual: Vec<ProcessedDocument>,
    /// Failed language sets with errors
    pub failed_sets: Vec<FailedLanguageSet>,
    /// Failed individual documents with errors
    pub failed_individual: Vec<FailedDocument>,
    /// Processing statistics
    pub statistics: BatchProcessingStatistics,
    /// Overall success status
    pub overall_success: bool,
    /// Processing duration
    pub processing_duration_ms: u64,
}

/// A successfully processed language set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedLanguageSet {
    /// Original language set
    pub language_set: CompleteLanguageSet,
    /// Processed documents by language
    pub processed_documents: HashMap<String, ProcessedDocument>,
    /// Set-level processing time
    pub processing_time_ms: u64,
    /// Any warnings during processing
    pub warnings: Vec<String>,
}

/// A processed individual document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedDocument {
    /// Original document information
    pub original: DetectedLanguageDocument,
    /// Extracted markdown content
    pub content: String,
    /// Document title (extracted or derived)
    pub title: String,
    /// Processing messages
    pub messages: Vec<String>,
    /// Processing warnings
    pub warnings: Vec<String>,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Extracted metadata
    pub metadata: DocumentMetadata,
}

/// Failed language set processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedLanguageSet {
    /// Original language set
    pub language_set: CompleteLanguageSet,
    /// Error that caused failure
    pub error: String,
    /// Partial results if any documents succeeded
    pub partial_results: HashMap<String, ProcessedDocument>,
}

/// Failed individual document processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedDocument {
    /// Original document
    pub document: DetectedLanguageDocument,
    /// Error that caused failure
    pub error: String,
}

/// Document metadata extracted during processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// Document format (docx, doc, etc.)
    pub format: String,
    /// Word count
    pub word_count: usize,
    /// Character count
    pub character_count: usize,
    /// Number of images found
    pub image_count: usize,
    /// Number of tables found
    pub table_count: usize,
    /// Document creation date (if available)
    pub created_at: Option<std::time::SystemTime>,
    /// Custom properties from document
    pub custom_properties: HashMap<String, String>,
}

/// Statistics for batch processing
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatchProcessingStatistics {
    pub total_sets_processed: usize,
    pub total_individual_documents: usize,
    pub successful_sets: usize,
    pub successful_individual: usize,
    pub failed_sets: usize,
    pub failed_individual: usize,
    pub total_processing_time_ms: u64,
    pub average_processing_time_per_document_ms: u64,
    pub peak_memory_usage_mb: usize,
    pub total_content_extracted_chars: usize,
    pub languages_processed: HashMap<String, usize>,
}

impl EnhancedMultiLanguageDetector {
    /// Create a new enhanced multi-language detector with default patterns
    pub fn new() -> Self {
        let mut language_patterns = HashMap::new();
        
        // English patterns
        language_patterns.insert("en".to_string(), LanguagePattern {
            code: "en".to_string(),
            display_name: "English".to_string(),
            patterns: vec![
                "_en".to_string(), "-en".to_string(), ".en.".to_string(),
                "_EN".to_string(), "-EN".to_string(), ".EN.".to_string()
            ],
            iso_codes: vec!["en".to_string(), "EN".to_string(), "eng".to_string(), "ENG".to_string()],
            full_names: vec!["english".to_string(), "English".to_string(), "ENGLISH".to_string()],
        });
        
        // German patterns
        language_patterns.insert("de".to_string(), LanguagePattern {
            code: "de".to_string(),
            display_name: "German".to_string(),
            patterns: vec![
                "_de".to_string(), "-de".to_string(), ".de.".to_string(),
                "_DE".to_string(), "-DE".to_string(), ".DE.".to_string()
            ],
            iso_codes: vec!["de".to_string(), "DE".to_string(), "deu".to_string(), "DEU".to_string(), "ger".to_string()],
            full_names: vec!["german".to_string(), "German".to_string(), "GERMAN".to_string(), "deutsch".to_string()],
        });
        
        // Spanish patterns
        language_patterns.insert("es".to_string(), LanguagePattern {
            code: "es".to_string(),
            display_name: "Spanish".to_string(),
            patterns: vec![
                "_es".to_string(), "-es".to_string(), ".es.".to_string(),
                "_ES".to_string(), "-ES".to_string(), ".ES.".to_string(),
                "_esp".to_string(), "-esp".to_string()
            ],
            iso_codes: vec!["es".to_string(), "ES".to_string(), "esp".to_string(), "ESP".to_string(), "spa".to_string()],
            full_names: vec!["spanish".to_string(), "Spanish".to_string(), "SPANISH".to_string(), "español".to_string()],
        });
        
        // French patterns
        language_patterns.insert("fr".to_string(), LanguagePattern {
            code: "fr".to_string(),
            display_name: "French".to_string(),
            patterns: vec![
                "_fr".to_string(), "-fr".to_string(), ".fr.".to_string(),
                "_FR".to_string(), "-FR".to_string(), ".FR.".to_string(),
                "_fra".to_string(), "-fra".to_string()
            ],
            iso_codes: vec!["fr".to_string(), "FR".to_string(), "fra".to_string(), "FRA".to_string(), "fre".to_string()],
            full_names: vec!["french".to_string(), "French".to_string(), "FRENCH".to_string(), "français".to_string()],
        });
        
        // Dutch patterns
        language_patterns.insert("nl".to_string(), LanguagePattern {
            code: "nl".to_string(),
            display_name: "Dutch".to_string(),
            patterns: vec![
                "_nl".to_string(), "-nl".to_string(), ".nl.".to_string(),
                "_NL".to_string(), "-NL".to_string(), ".NL.".to_string(),
                "_nld".to_string(), "-nld".to_string()
            ],
            iso_codes: vec!["nl".to_string(), "NL".to_string(), "nld".to_string(), "NLD".to_string(), "dut".to_string()],
            full_names: vec!["dutch".to_string(), "Dutch".to_string(), "DUTCH".to_string(), "nederlands".to_string()],
        });
        
        // Create validation patterns for common document naming
        let validation_patterns = vec![
            Regex::new(r".*\.(docx|doc|rtf|odt)$").unwrap(), // Document extensions
            Regex::new(r"(?i)manual.*\.(docx|doc)").unwrap(), // Manual documents
            Regex::new(r"(?i).*_(en|de|es|fr|nl)(_.*)?\.(docx|doc)").unwrap(), // Language suffixed
        ];
        
        Self {
            language_patterns,
            validation_patterns,
        }
    }
    
    /// Add or update a language pattern
    pub fn add_language_pattern(&mut self, pattern: LanguagePattern) {
        self.language_patterns.insert(pattern.code.clone(), pattern);
    }
    
    /// Detect language from filename using enhanced pattern matching
    pub fn detect_language_from_filename(&self, filename: &str) -> Option<(String, f32, DetectionMethod)> {
        let filename_lower = filename.to_lowercase();
        
        // Try exact pattern matching first (highest confidence)
        for (lang_code, pattern_config) in &self.language_patterns {
            for pattern in &pattern_config.patterns {
                if filename_lower.contains(pattern) {
                    return Some((
                        lang_code.clone(), 
                        0.95, 
                        DetectionMethod::FilenamePattern { pattern: pattern.clone() }
                    ));
                }
            }
        }
        
        // Try ISO codes (high confidence)
        for (lang_code, pattern_config) in &self.language_patterns {
            for iso_code in &pattern_config.iso_codes {
                if filename_lower.contains(&iso_code.to_lowercase()) {
                    return Some((
                        lang_code.clone(),
                        0.85,
                        DetectionMethod::FilenamePattern { pattern: iso_code.clone() }
                    ));
                }
            }
        }
        
        // Try full language names (medium confidence)
        for (lang_code, pattern_config) in &self.language_patterns {
            for full_name in &pattern_config.full_names {
                if filename_lower.contains(&full_name.to_lowercase()) {
                    return Some((
                        lang_code.clone(),
                        0.75,
                        DetectionMethod::FilenamePattern { pattern: full_name.clone() }
                    ));
                }
            }
        }
        
        None
    }
    
    /// Extract base document name by removing language indicators and version info
    pub fn extract_base_name(&self, filename: &str) -> String {
        let stem = Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(filename);
        
        let mut base_name = stem.to_string();
        
        // Remove language patterns
        for pattern_config in self.language_patterns.values() {
            for pattern in &pattern_config.patterns {
                base_name = base_name.replace(pattern, "");
            }
            for iso_code in &pattern_config.iso_codes {
                base_name = base_name.replace(&format!("_{}", iso_code.to_lowercase()), "");
                base_name = base_name.replace(&format!("-{}", iso_code.to_lowercase()), "");
            }
        }
        
        // Remove version indicators
        let version_patterns = vec![
            Regex::new(r"_v\d+").unwrap(),
            Regex::new(r"-v\d+").unwrap(),
            Regex::new(r"_version_?\d+").unwrap(),
            Regex::new(r"-version-?\d+").unwrap(),
        ];
        
        for pattern in version_patterns {
            base_name = pattern.replace_all(&base_name, "").to_string();
        }
        
        // Clean up multiple separators and trim
        base_name = base_name
            .replace("__", "_")
            .replace("--", "-")
            .trim_end_matches('_')
            .trim_end_matches('-')
            .trim_start_matches('_')
            .trim_start_matches('-')
            .trim()
            .to_string();
        
        if base_name.is_empty() {
            "document".to_string()
        } else {
            base_name
        }
    }
    
    /// Scan a folder recursively for multi-language document sets
    pub async fn scan_folder(&self, folder_path: &Path, config: &FolderScanConfig) -> Result<FolderScanResult> {
        let start_time = std::time::Instant::now();
        let mut scan_result = FolderScanResult {
            root_folder: folder_path.to_path_buf(),
            language_groups: HashMap::new(),
            unclassified_files: Vec::new(),
            complete_sets: Vec::new(),
            missing_languages: HashMap::new(),
            scan_stats: ScanStatistics::default(),
            warnings: Vec::new(),
            errors: Vec::new(),
        };
        
        if !folder_path.exists() || !folder_path.is_dir() {
            scan_result.errors.push(format!("Folder does not exist or is not a directory: {}", folder_path.display()));
            return Ok(scan_result);
        }
        
        // Collect all files to process
        let files = self.collect_files(folder_path, config, &mut scan_result.warnings).await?;
        scan_result.scan_stats.total_files_scanned = files.len();
        
        // Process each file for language detection
        let mut total_confidence = 0.0;
        let mut detected_count = 0;
        
        for file_path in files {
            match self.analyze_file(&file_path).await {
                Ok(detected_doc) => {
                    scan_result.scan_stats.supported_files_found += 1;
                    
                    if detected_doc.confidence >= config.min_confidence {
                        let lang_code = detected_doc.language_code.clone();
                        scan_result.language_groups.entry(lang_code.clone())
                            .or_default()
                            .push(detected_doc);
                        
                        *scan_result.scan_stats.languages_detected.entry(lang_code).or_insert(0) += 1;
                        total_confidence += detected_doc.confidence;
                        detected_count += 1;
                    } else {
                        scan_result.unclassified_files.push(file_path);
                        scan_result.warnings.push(format!(
                            "File {} has low confidence ({:.2}) for language detection", 
                            file_path.display(), detected_doc.confidence
                        ));
                    }
                },
                Err(e) => {
                    scan_result.unclassified_files.push(file_path.clone());
                    scan_result.warnings.push(format!(
                        "Failed to analyze file {}: {}", 
                        file_path.display(), e
                    ));
                }
            }
        }
        
        // Calculate average confidence
        scan_result.scan_stats.average_confidence = if detected_count > 0 {
            total_confidence / detected_count as f32
        } else {
            0.0
        };
        
        // Group documents into complete language sets
        self.group_into_language_sets(&mut scan_result, config).await?;
        
        // Calculate final statistics
        scan_result.scan_stats.scan_duration_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(scan_result)
    }
    
    /// Collect all files in the specified folder based on configuration
    async fn collect_files(&self, folder_path: &Path, config: &FolderScanConfig, warnings: &mut Vec<String>) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        if config.recursive {
            self.collect_files_recursive(folder_path, config, &mut files, 0, warnings).await?;
        } else {
            self.collect_files_single_level(folder_path, config, &mut files, warnings).await?;
        }
        
        Ok(files)
    }
    
    /// Collect files recursively with depth control
    async fn collect_files_recursive(
        &self,
        current_path: &Path,
        config: &FolderScanConfig,
        files: &mut Vec<PathBuf>,
        current_depth: usize,
        warnings: &mut Vec<String>
    ) -> Result<()> {
        // Check depth limit
        if let Some(max_depth) = config.max_depth {
            if current_depth > max_depth {
                return Ok(());
            }
        }
        
        let entries = fs::read_dir(current_path)
            .map_err(|e| TradocumentError::FileError(format!("Cannot read directory {}: {}", current_path.display(), e)))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| TradocumentError::FileError(format!("Directory entry error: {}", e)))?;
            let path = entry.path();
            
            if path.is_dir() {
                // Skip excluded patterns
                if self.should_exclude_path(&path, &config.exclude_patterns) {
                    continue;
                }
                
                // Recurse into subdirectory
                self.collect_files_recursive(&path, config, files, current_depth + 1, warnings).await?;
            } else if path.is_file() {
                if self.should_include_file(&path, config) {
                    files.push(path);
                }
            }
        }
        
        Ok(())
    }
    
    /// Collect files from single level only
    async fn collect_files_single_level(
        &self,
        folder_path: &Path,
        config: &FolderScanConfig,
        files: &mut Vec<PathBuf>,
        _warnings: &mut Vec<String>
    ) -> Result<()> {
        let entries = fs::read_dir(folder_path)
            .map_err(|e| TradocumentError::FileError(format!("Cannot read directory {}: {}", folder_path.display(), e)))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| TradocumentError::FileError(format!("Directory entry error: {}", e)))?;
            let path = entry.path();
            
            if path.is_file() && self.should_include_file(&path, config) {
                files.push(path);
            }
        }
        
        Ok(())
    }
    
    /// Check if a path should be excluded based on patterns
    fn should_exclude_path(&self, path: &Path, exclude_patterns: &[String]) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();
        
        for pattern in exclude_patterns {
            if path_str.contains(&pattern.to_lowercase()) {
                return true;
            }
        }
        
        false
    }
    
    /// Check if a file should be included based on configuration
    fn should_include_file(&self, file_path: &Path, config: &FolderScanConfig) -> bool {
        let filename = file_path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        
        let extension = file_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        // Check excluded extensions
        if config.exclude_extensions.iter().any(|ext| ext.eq_ignore_ascii_case(extension)) {
            return false;
        }
        
        // Check included extensions
        if !config.include_extensions.is_empty() {
            if !config.include_extensions.iter().any(|ext| ext.eq_ignore_ascii_case(extension)) {
                return false;
            }
        }
        
        // Check excluded patterns
        if self.should_exclude_path(file_path, &config.exclude_patterns) {
            return false;
        }
        
        // Validate against document patterns
        for pattern in &self.validation_patterns {
            if pattern.is_match(filename) {
                return true;
            }
        }
        
        // Default to include if no specific patterns matched
        true
    }
    
    /// Analyze a single file and detect language
    async fn analyze_file(&self, file_path: &Path) -> Result<DetectedLanguageDocument> {
        let filename = file_path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        // Get file metadata
        let metadata = fs::metadata(file_path)
            .map_err(|e| TradocumentError::FileError(format!("Cannot read file metadata for {}: {}", file_path.display(), e)))?;
        
        // Detect language from filename
        let (language_code, confidence, detection_method) = self.detect_language_from_filename(&filename)
            .unwrap_or_else(|| ("unknown".to_string(), 0.0, DetectionMethod::Unknown));
        
        let base_name = self.extract_base_name(&filename);
        
        Ok(DetectedLanguageDocument {
            file_path: file_path.to_path_buf(),
            language_code,
            confidence,
            detection_method,
            base_name,
            file_size: metadata.len(),
            modified_at: metadata.modified().unwrap_or(std::time::UNIX_EPOCH),
        })
    }
    
    /// Group detected documents into complete language sets
    async fn group_into_language_sets(&self, scan_result: &mut FolderScanResult, config: &FolderScanConfig) -> Result<()> {
        // Group documents by base name
        let mut base_name_groups: HashMap<String, Vec<&DetectedLanguageDocument>> = HashMap::new();
        
        for docs in scan_result.language_groups.values() {
            for doc in docs {
                base_name_groups.entry(doc.base_name.clone())
                    .or_default()
                    .push(doc);
            }
        }
        
        // Process each base name group
        for (base_name, docs) in base_name_groups {
            let mut documents_by_lang: HashMap<String, DetectedLanguageDocument> = HashMap::new();
            
            for doc in docs {
                if doc.language_code != "unknown" {
                    // If multiple documents for same language, keep the one with higher confidence
                    match documents_by_lang.get(&doc.language_code) {
                        Some(existing) if existing.confidence >= doc.confidence => continue,
                        _ => {
                            documents_by_lang.insert(doc.language_code.clone(), (*doc).clone());
                        }
                    }
                }
            }
            
            if documents_by_lang.is_empty() {
                continue;
            }
            
            // Check if this is a complete set
            let required_languages: HashSet<String> = if config.required_languages.is_empty() {
                vec!["en".to_string(), "de".to_string(), "es".to_string(), "fr".to_string(), "nl".to_string()]
                    .into_iter().collect()
            } else {
                config.required_languages.iter().cloned().collect()
            };
            
            let found_languages: HashSet<String> = documents_by_lang.keys().cloned().collect();
            let missing_languages: Vec<String> = required_languages.difference(&found_languages).cloned().collect();
            
            let is_complete = missing_languages.is_empty();
            
            let language_set = CompleteLanguageSet {
                set_id: Uuid::new_v4().to_string(),
                base_name: base_name.clone(),
                documents: documents_by_lang,
                file_count: docs.len(),
                is_complete,
                missing_languages: missing_languages.clone(),
            };
            
            if is_complete {
                scan_result.scan_stats.complete_sets_found += 1;
            } else {
                scan_result.scan_stats.incomplete_sets_found += 1;
                scan_result.missing_languages.insert(base_name, missing_languages);
            }
            
            scan_result.complete_sets.push(language_set);
        }
        
        Ok(())
    }
    
    /// Get supported languages
    pub fn get_supported_languages(&self) -> Vec<&LanguagePattern> {
        self.language_patterns.values().collect()
    }
    
    /// Get language display name
    pub fn get_language_display_name(&self, language_code: &str) -> Option<String> {
        self.language_patterns.get(language_code)
            .map(|pattern| pattern.display_name.clone())
    }
    
    /// Validate a complete language set for processing readiness
    pub fn validate_language_set(&self, language_set: &CompleteLanguageSet) -> Result<Vec<String>> {
        let mut warnings = Vec::new();
        
        // Check file sizes are reasonable
        for (lang, doc) in &language_set.documents {
            if doc.file_size == 0 {
                warnings.push(format!("Document for language {} is empty", lang));
            } else if doc.file_size > 100 * 1024 * 1024 { // 100MB
                warnings.push(format!("Document for language {} is very large ({} MB)", 
                    lang, doc.file_size / (1024 * 1024)));
            }
            
            if doc.confidence < 0.8 {
                warnings.push(format!("Low confidence ({:.1}%) for language detection of {} document", 
                    doc.confidence * 100.0, lang));
            }
        }
        
        // Check if files have similar modification times (indicating they're related)
        if language_set.documents.len() > 1 {
            let times: Vec<_> = language_set.documents.values()
                .map(|doc| doc.modified_at)
                .collect();
            
            let min_time = times.iter().min().unwrap();
            let max_time = times.iter().max().unwrap();
            
            if let (Ok(min_duration), Ok(max_duration)) = (min_time.duration_since(std::time::UNIX_EPOCH), max_time.duration_since(std::time::UNIX_EPOCH)) {
                let time_diff = max_duration.as_secs() - min_duration.as_secs();
                if time_diff > 7 * 24 * 3600 { // More than a week apart
                    warnings.push(format!("Documents in set have very different modification times (up to {} days apart)", 
                        time_diff / (24 * 3600)));
                }
            }
        }
        
        Ok(warnings)
    }
}

/// Default configuration for folder scanning
impl Default for FolderScanConfig {
    fn default() -> Self {
        Self {
            recursive: true,
            max_depth: Some(3),
            include_extensions: vec!["docx".to_string(), "doc".to_string()],
            exclude_extensions: vec!["tmp".to_string(), "bak".to_string(), "~".to_string()],
            exclude_patterns: vec![
                "$recycle".to_string(),
                ".git".to_string(),
                ".svn".to_string(),
                "__pycache__".to_string(),
                ".DS_Store".to_string(),
                "thumbs.db".to_string(),
            ],
            min_confidence: 0.5,
            require_complete_sets: false,
            required_languages: vec![
                "en".to_string(), "de".to_string(), "es".to_string(), 
                "fr".to_string(), "nl".to_string()
            ],
        }
    }
}

/// Default configuration for batch import
impl Default for BatchImportConfig {
    fn default() -> Self {
        Self {
            processing_strategy: ProcessingStrategy::Adaptive,
            incomplete_set_handling: IncompleteSetHandling::ProcessWithWarnings,
            preserve_structure: true,
            parallel_processing: ParallelProcessingConfig::default(),
            memory_config: MemoryConfig::default(),
            error_handling: ErrorHandlingStrategy::ContinueOnError,
        }
    }
}

/// Default parallel processing configuration
impl Default for ParallelProcessingConfig {
    fn default() -> Self {
        let cpu_count = num_cpus::get();
        Self {
            max_concurrent_documents: cpu_count.max(2),
            max_concurrent_languages: 2,
            document_timeout_secs: 300, // 5 minutes
            use_thread_pool: true,
        }
    }
}

/// Default memory configuration
impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_memory_per_document_mb: 50,
            use_streaming: true,
            stream_buffer_size_kb: 64,
            immediate_cleanup: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;
    
    #[test]
    fn test_language_detection_patterns() {
        let detector = EnhancedMultiLanguageDetector::new();
        
        // Test various filename patterns
        assert_eq!(detector.detect_language_from_filename("manual_EN.docx").unwrap().0, "en");
        assert_eq!(detector.detect_language_from_filename("manual-de.docx").unwrap().0, "de");
        assert_eq!(detector.detect_language_from_filename("manual_ES_v1.docx").unwrap().0, "es");
        assert_eq!(detector.detect_language_from_filename("french_manual.docx").unwrap().0, "fr");
        assert_eq!(detector.detect_language_from_filename("handleiding_NL.docx").unwrap().0, "nl");
        
        // Test confidence levels
        assert!(detector.detect_language_from_filename("manual_EN.docx").unwrap().1 > 0.9);
        assert!(detector.detect_language_from_filename("english_manual.docx").unwrap().1 < 0.9);
        
        // Test undetectable patterns
        assert!(detector.detect_language_from_filename("manual.docx").is_none());
    }
    
    #[test]
    fn test_base_name_extraction() {
        let detector = EnhancedMultiLanguageDetector::new();
        
        assert_eq!(detector.extract_base_name("manual_EN_v1.docx"), "manual");
        assert_eq!(detector.extract_base_name("user_guide-de-v2.docx"), "user_guide");
        assert_eq!(detector.extract_base_name("installation_FR.docx"), "installation");
        assert_eq!(detector.extract_base_name("setup.docx"), "setup");
        assert_eq!(detector.extract_base_name("_en_.docx"), "document"); // Edge case
    }
    
    #[tokio::test]
    async fn test_folder_scanning() {
        let detector = EnhancedMultiLanguageDetector::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Create test files
        let test_files = vec![
            "manual_EN.docx",
            "manual_DE.docx", 
            "manual_ES.docx",
            "manual_FR.docx",
            "manual_NL.docx",
            "guide_EN.docx",
            "guide_DE.docx",
            "other.txt", // Should be excluded
        ];
        
        for filename in &test_files {
            let file_path = temp_dir.path().join(filename);
            let mut file = File::create(&file_path).unwrap();
            writeln!(file, "Test content for {}", filename).unwrap();
        }
        
        let config = FolderScanConfig::default();
        let result = detector.scan_folder(temp_dir.path(), &config).await.unwrap();
        
        // Should find 5 languages
        assert_eq!(result.language_groups.len(), 5);
        
        // Should find 2 complete sets (manual and guide, but guide is incomplete)
        assert_eq!(result.complete_sets.len(), 2);
        
        // One set should be complete (manual), one incomplete (guide)
        let complete_count = result.complete_sets.iter().filter(|set| set.is_complete).count();
        assert_eq!(complete_count, 1);
        
        // Should have good statistics
        assert!(result.scan_stats.total_files_scanned >= 7); // Excludes .txt file
        assert!(result.scan_stats.supported_files_found >= 7);
    }
    
    #[test]
    fn test_language_pattern_customization() {
        let mut detector = EnhancedMultiLanguageDetector::new();
        
        // Add custom language pattern
        let custom_pattern = LanguagePattern {
            code: "pt".to_string(),
            display_name: "Portuguese".to_string(),
            patterns: vec!["_pt".to_string(), "-pt".to_string()],
            iso_codes: vec!["pt".to_string(), "por".to_string()],
            full_names: vec!["portuguese".to_string()],
        };
        
        detector.add_language_pattern(custom_pattern);
        
        // Test detection of custom language
        assert_eq!(detector.detect_language_from_filename("manual_pt.docx").unwrap().0, "pt");
        assert_eq!(detector.get_language_display_name("pt").unwrap(), "Portuguese");
    }
    
    #[test]
    fn test_validation_patterns() {
        let detector = EnhancedMultiLanguageDetector::new();
        let config = FolderScanConfig::default();
        
        // Test file inclusion/exclusion
        assert!(detector.should_include_file(Path::new("document.docx"), &config));
        assert!(detector.should_include_file(Path::new("manual.doc"), &config));
        assert!(!detector.should_include_file(Path::new("image.png"), &config));
        assert!(!detector.should_include_file(Path::new("backup.bak"), &config));
    }
}