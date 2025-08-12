use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;
use uuid::Uuid;
use regex::Regex;

use crate::services::document_processing::{
    DocumentProcessingService, DocumentProcessingConfig, ProcessedDocument, 
    ProgressCallback, ImportProgressInfo, ImportStage
};
use crate::TradocumentError;

/// Supported languages for multi-language manual import
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SupportedLanguage {
    English,
    German, 
    Spanish,
    French,
    Dutch,
}

impl SupportedLanguage {
    pub fn code(&self) -> &'static str {
        match self {
            SupportedLanguage::English => "en",
            SupportedLanguage::German => "de", 
            SupportedLanguage::Spanish => "es",
            SupportedLanguage::French => "fr",
            SupportedLanguage::Dutch => "nl",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            SupportedLanguage::English => "English",
            SupportedLanguage::German => "German",
            SupportedLanguage::Spanish => "Spanish", 
            SupportedLanguage::French => "French",
            SupportedLanguage::Dutch => "Dutch",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code.to_lowercase().as_str() {
            "en" | "eng" | "english" => Some(SupportedLanguage::English),
            "de" | "ger" | "german" | "deutsch" => Some(SupportedLanguage::German),
            "es" | "spa" | "spanish" | "español" => Some(SupportedLanguage::Spanish),
            "fr" | "fre" | "french" | "français" => Some(SupportedLanguage::French),
            "nl" | "dut" | "dutch" | "nederlands" => Some(SupportedLanguage::Dutch),
            _ => None,
        }
    }
}

/// Result of folder scanning for multi-language manuals
#[derive(Debug, Clone)]
pub struct FolderScanResult {
    pub total_files_found: usize,
    pub language_files: HashMap<SupportedLanguage, Vec<PathBuf>>,
    pub unmatched_files: Vec<PathBuf>,
    pub conflicts: Vec<LanguageConflict>,
    pub missing_languages: Vec<SupportedLanguage>,
    pub warnings: Vec<String>,
}

/// Represents a conflict where multiple files match the same language
#[derive(Debug, Clone)]
pub struct LanguageConflict {
    pub language: SupportedLanguage,
    pub files: Vec<PathBuf>,
    pub suggested_primary: Option<PathBuf>,
}

/// Configuration for multi-language manual import
#[derive(Debug, Clone)]
pub struct MultiLanguageImportConfig {
    pub required_languages: Vec<SupportedLanguage>,
    pub optional_languages: Vec<SupportedLanguage>, 
    pub allow_partial_import: bool,
    pub recursive_scan: bool,
    pub max_depth: Option<usize>,
    pub processing_config: DocumentProcessingConfig,
    pub resolve_conflicts_automatically: bool,
}

impl Default for MultiLanguageImportConfig {
    fn default() -> Self {
        Self {
            required_languages: vec![SupportedLanguage::English],
            optional_languages: vec![
                SupportedLanguage::German,
                SupportedLanguage::Spanish, 
                SupportedLanguage::French,
                SupportedLanguage::Dutch
            ],
            allow_partial_import: true,
            recursive_scan: true,
            max_depth: Some(3),
            processing_config: DocumentProcessingConfig::default(),
            resolve_conflicts_automatically: true,
        }
    }
}

/// Result of multi-language manual import
#[derive(Debug, Clone)]
pub struct MultiLanguageImportResult {
    pub manual_id: Uuid,
    pub imported_languages: HashMap<SupportedLanguage, ProcessedDocument>,
    pub failed_languages: HashMap<SupportedLanguage, String>,
    pub total_processing_time_ms: u64,
    pub warnings: Vec<String>,
    pub conflicts_resolved: Vec<LanguageConflict>,
}

/// Enhanced multi-language manual import service
pub struct MultiLanguageManualImportService {
    document_processor: DocumentProcessingService,
    language_patterns: Vec<Regex>,
}

impl MultiLanguageManualImportService {
    /// Create a new multi-language manual import service
    pub fn new() -> Result<Self, TradocumentError> {
        let document_processor = DocumentProcessingService::new()?;
        
        // Compile regex patterns for enhanced language detection
        let language_patterns = vec![
            // Standard patterns with separators
            Regex::new(r"[_-](en|eng|english)[_-]?").unwrap(),
            Regex::new(r"[_-](de|ger|german|deutsch)[_-]?").unwrap(), 
            Regex::new(r"[_-](es|spa|spanish|español)[_-]?").unwrap(),
            Regex::new(r"[_-](fr|fre|french|français)[_-]?").unwrap(),
            Regex::new(r"[_-](nl|dut|dutch|nederlands)[_-]?").unwrap(),
            
            // Uppercase variations
            Regex::new(r"[_-](EN|ENG|ENGLISH)[_-]?").unwrap(),
            Regex::new(r"[_-](DE|GER|GERMAN|DEUTSCH)[_-]?").unwrap(),
            Regex::new(r"[_-](ES|SPA|SPANISH|ESPAÑOL)[_-]?").unwrap(), 
            Regex::new(r"[_-](FR|FRE|FRENCH|FRANÇAIS)[_-]?").unwrap(),
            Regex::new(r"[_-](NL|DUT|DUTCH|NEDERLANDS)[_-]?").unwrap(),
            
            // Version-aware patterns
            Regex::new(r"[_-](en|de|es|fr|nl)[_-]?v?\d*[_-]?").unwrap(),
            
            // End-of-filename patterns
            Regex::new(r"[_-](en|de|es|fr|nl)$").unwrap(),
            Regex::new(r"[_-](EN|DE|ES|FR|NL)$").unwrap(),
        ];

        Ok(Self {
            document_processor,
            language_patterns,
        })
    }

    /// Scan a folder for multi-language manual files
    pub fn scan_folder(
        &self,
        folder_path: &Path,
        config: &MultiLanguageImportConfig,
    ) -> Result<FolderScanResult, TradocumentError> {
        let mut language_files: HashMap<SupportedLanguage, Vec<PathBuf>> = HashMap::new();
        let mut unmatched_files = Vec::new();
        let mut total_files_found = 0;

        // Collect all Word document files
        let files = self.collect_document_files(folder_path, config)?;
        total_files_found = files.len();

        // Process each file for language detection
        for file_path in files {
            if let Some(language) = self.detect_language_enhanced(&file_path) {
                language_files.entry(language).or_insert_with(Vec::new).push(file_path);
            } else {
                unmatched_files.push(file_path);
            }
        }

        // Detect conflicts (multiple files per language)
        let mut conflicts = Vec::new();
        for (language, files) in &language_files {
            if files.len() > 1 {
                let suggested_primary = if config.resolve_conflicts_automatically {
                    self.suggest_primary_file(files)
                } else {
                    None
                };
                
                conflicts.push(LanguageConflict {
                    language: language.clone(),
                    files: files.clone(),
                    suggested_primary,
                });
            }
        }

        // Determine missing languages
        let mut missing_languages = Vec::new();
        let found_languages: Vec<&SupportedLanguage> = language_files.keys().collect();
        
        for required_lang in &config.required_languages {
            if !found_languages.contains(&required_lang) {
                missing_languages.push(required_lang.clone());
            }
        }

        // Generate warnings
        let mut warnings = Vec::new();
        if !conflicts.is_empty() {
            warnings.push(format!("Found {} language conflicts that need resolution", conflicts.len()));
        }
        if !missing_languages.is_empty() {
            warnings.push(format!("Missing required languages: {}", 
                missing_languages.iter().map(|l| l.display_name()).collect::<Vec<_>>().join(", ")));
        }
        if !unmatched_files.is_empty() {
            warnings.push(format!("Found {} files with undetectable languages", unmatched_files.len()));
        }

        Ok(FolderScanResult {
            total_files_found,
            language_files,
            unmatched_files,
            conflicts,
            missing_languages,
            warnings,
        })
    }

    /// Import multi-language manual from folder with progress tracking
    pub fn import_multi_language_manual(
        &self,
        folder_path: &Path,
        config: MultiLanguageImportConfig,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<MultiLanguageImportResult, TradocumentError> {
        let start_time = std::time::Instant::now();
        let manual_id = Uuid::new_v4();

        // Send initial progress
        if let Some(ref callback) = progress_callback {
            callback(ImportProgressInfo {
                current_file: "Scanning folder...".to_string(),
                progress_percent: 0,
                message: "Scanning folder for multi-language manual files".to_string(),
                stage: ImportStage::Validating,
                warnings: Vec::new(),
                errors: Vec::new(),
            });
        }

        // Scan folder for files
        let scan_result = self.scan_folder(folder_path, &config)?;

        // Handle missing required languages
        if !config.allow_partial_import && !scan_result.missing_languages.is_empty() {
            return Err(TradocumentError::Validation(
                format!("Missing required languages: {}", 
                    scan_result.missing_languages.iter()
                        .map(|l| l.display_name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            ));
        }

        // Send scanning complete progress
        if let Some(ref callback) = progress_callback {
            callback(ImportProgressInfo {
                current_file: "Scan complete".to_string(),
                progress_percent: 15,
                message: format!("Found files for {} languages", scan_result.language_files.len()),
                stage: ImportStage::Processing,
                warnings: scan_result.warnings.clone(),
                errors: Vec::new(),
            });
        }

        // Resolve conflicts automatically if configured
        let resolved_files = self.resolve_conflicts(&scan_result, &config)?;

        // Import documents for each language
        let mut imported_languages = HashMap::new();
        let mut failed_languages = HashMap::new();
        let total_languages = resolved_files.len();

        for (index, (language, file_path)) in resolved_files.iter().enumerate() {
            let language_progress = ((index as f32 / total_languages as f32) * 70.0) as u8 + 15;

            if let Some(ref callback) = progress_callback {
                callback(ImportProgressInfo {
                    current_file: file_path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    progress_percent: language_progress,
                    message: format!("Importing {} manual...", language.display_name()),
                    stage: ImportStage::Converting,
                    warnings: Vec::new(),
                    errors: Vec::new(),
                });
            }

            // Process individual document
            let mut lang_config = config.processing_config.clone();
            lang_config.target_language = language.code().to_string();

            match self.document_processor.process_document(file_path, lang_config, None) {
                Ok(processed_doc) => {
                    imported_languages.insert(language.clone(), processed_doc);
                }
                Err(e) => {
                    failed_languages.insert(language.clone(), e.to_string());
                }
            }
        }

        let total_processing_time = start_time.elapsed().as_millis() as u64;

        // Send completion progress
        if let Some(ref callback) = progress_callback {
            let stage = if failed_languages.is_empty() {
                ImportStage::Completed
            } else if imported_languages.is_empty() {
                ImportStage::Failed
            } else {
                ImportStage::Completed
            };

            callback(ImportProgressInfo {
                current_file: "Multi-language import complete".to_string(),
                progress_percent: 100,
                message: format!("Successfully imported {} of {} languages", 
                    imported_languages.len(), total_languages),
                stage,
                warnings: scan_result.warnings.clone(),
                errors: failed_languages.values().cloned().collect(),
            });
        }

        Ok(MultiLanguageImportResult {
            manual_id,
            imported_languages,
            failed_languages,
            total_processing_time_ms: total_processing_time,
            warnings: scan_result.warnings,
            conflicts_resolved: scan_result.conflicts,
        })
    }

    /// Enhanced language detection from filename
    fn detect_language_enhanced(&self, file_path: &Path) -> Option<SupportedLanguage> {
        let filename = file_path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Try each regex pattern
        for pattern in &self.language_patterns {
            if let Some(captures) = pattern.captures(&filename) {
                if let Some(lang_match) = captures.get(1) {
                    if let Some(language) = SupportedLanguage::from_code(lang_match.as_str()) {
                        return Some(language);
                    }
                }
            }
        }

        // Fallback to simple contains check
        if filename.contains("english") || filename.contains("_en") || filename.contains("-en") {
            Some(SupportedLanguage::English)
        } else if filename.contains("german") || filename.contains("deutsch") || filename.contains("_de") || filename.contains("-de") {
            Some(SupportedLanguage::German)
        } else if filename.contains("spanish") || filename.contains("español") || filename.contains("_es") || filename.contains("-es") {
            Some(SupportedLanguage::Spanish)
        } else if filename.contains("french") || filename.contains("français") || filename.contains("_fr") || filename.contains("-fr") {
            Some(SupportedLanguage::French)  
        } else if filename.contains("dutch") || filename.contains("nederlands") || filename.contains("_nl") || filename.contains("-nl") {
            Some(SupportedLanguage::Dutch)
        } else {
            None
        }
    }

    /// Collect document files from folder (recursive if configured)
    fn collect_document_files(
        &self,
        folder_path: &Path,
        config: &MultiLanguageImportConfig,
    ) -> Result<Vec<PathBuf>, TradocumentError> {
        let mut files = Vec::new();
        
        if config.recursive_scan {
            self.collect_files_recursive(folder_path, &mut files, 0, config.max_depth)?;
        } else {
            self.collect_files_single_level(folder_path, &mut files)?;
        }

        // Filter for supported document formats
        Ok(files.into_iter()
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.to_lowercase() == "docx")
                    .unwrap_or(false)
            })
            .collect())
    }

    /// Collect files from single directory level
    fn collect_files_single_level(&self, dir_path: &Path, files: &mut Vec<PathBuf>) -> Result<(), TradocumentError> {
        let entries = fs::read_dir(dir_path)
            .map_err(|e| TradocumentError::IoError(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| TradocumentError::IoError(e))?;
            let path = entry.path();
            
            if path.is_file() {
                files.push(path);
            }
        }

        Ok(())
    }

    /// Collect files recursively with depth limit
    fn collect_files_recursive(
        &self,
        dir_path: &Path,
        files: &mut Vec<PathBuf>,
        current_depth: usize,
        max_depth: Option<usize>,
    ) -> Result<(), TradocumentError> {
        // Check depth limit
        if let Some(max) = max_depth {
            if current_depth >= max {
                return Ok(());
            }
        }

        let entries = fs::read_dir(dir_path)
            .map_err(|e| TradocumentError::IoError(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| TradocumentError::IoError(e))?;
            let path = entry.path();

            if path.is_file() {
                files.push(path);
            } else if path.is_dir() {
                self.collect_files_recursive(&path, files, current_depth + 1, max_depth)?;
            }
        }

        Ok(())
    }

    /// Suggest primary file from conflict group
    fn suggest_primary_file(&self, files: &[PathBuf]) -> Option<PathBuf> {
        // Prioritize files by various criteria
        let mut scored_files: Vec<(PathBuf, u32)> = files.iter()
            .map(|file| (file.clone(), self.score_file_priority(file)))
            .collect();

        scored_files.sort_by(|a, b| b.1.cmp(&a.1));
        scored_files.first().map(|(path, _)| path.clone())
    }

    /// Score file priority for conflict resolution
    fn score_file_priority(&self, file_path: &Path) -> u32 {
        let filename = file_path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("")
            .to_lowercase();

        let mut score = 0u32;

        // Prefer files with "manual" in name
        if filename.contains("manual") { score += 10; }
        
        // Prefer files without version numbers (likely main versions)
        if !filename.contains("v1") && !filename.contains("v2") && !filename.contains("version") {
            score += 5;
        }

        // Prefer shorter names (likely less specific)
        if filename.len() < 20 { score += 3; }

        // Check file size (prefer larger files as they're likely more complete)
        if let Ok(metadata) = file_path.metadata() {
            let size_kb = metadata.len() / 1024;
            if size_kb > 100 { score += 5; }
            if size_kb > 500 { score += 3; }
        }

        score
    }

    /// Resolve conflicts by selecting primary files
    fn resolve_conflicts(
        &self,
        scan_result: &FolderScanResult,
        config: &MultiLanguageImportConfig,
    ) -> Result<HashMap<SupportedLanguage, PathBuf>, TradocumentError> {
        let mut resolved_files = HashMap::new();

        for (language, files) in &scan_result.language_files {
            if files.len() == 1 {
                // No conflict, use the single file
                resolved_files.insert(language.clone(), files[0].clone());
            } else if config.resolve_conflicts_automatically {
                // Auto-resolve using priority scoring
                if let Some(primary_file) = self.suggest_primary_file(files) {
                    resolved_files.insert(language.clone(), primary_file);
                }
            } else {
                // Cannot resolve automatically
                return Err(TradocumentError::Validation(
                    format!("Multiple files found for {} language. Manual resolution required.", 
                        language.display_name())
                ));
            }
        }

        Ok(resolved_files)
    }

    /// Get all supported languages
    pub fn supported_languages() -> Vec<SupportedLanguage> {
        vec![
            SupportedLanguage::English,
            SupportedLanguage::German,
            SupportedLanguage::Spanish, 
            SupportedLanguage::French,
            SupportedLanguage::Dutch,
        ]
    }

    /// Validate folder structure and files before import
    pub fn validate_folder_for_import(
        &self,
        folder_path: &Path,
        config: &MultiLanguageImportConfig,
    ) -> Result<FolderScanResult, TradocumentError> {
        if !folder_path.exists() {
            return Err(TradocumentError::IoError(
                std::io::Error::new(std::io::ErrorKind::NotFound, "Folder does not exist")
            ));
        }

        if !folder_path.is_dir() {
            return Err(TradocumentError::Validation(
                "Path is not a directory".to_string()
            ));
        }

        self.scan_folder(folder_path, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;

    #[test]
    fn test_language_detection() {
        let service = MultiLanguageManualImportService::new().unwrap();
        
        // Test various filename patterns
        let test_cases = vec![
            ("manual_en.docx", Some(SupportedLanguage::English)),
            ("manual_DE.docx", Some(SupportedLanguage::German)),
            ("guide-es.docx", Some(SupportedLanguage::Spanish)),
            ("doc_fr_v2.docx", Some(SupportedLanguage::French)),
            ("handbook_nl.docx", Some(SupportedLanguage::Dutch)),
            ("manual_english.docx", Some(SupportedLanguage::English)),
            ("random_file.docx", None),
        ];

        for (filename, expected) in test_cases {
            let path = Path::new(filename);
            let result = service.detect_language_enhanced(&path);
            assert_eq!(result, expected, "Failed for filename: {}", filename);
        }
    }

    #[test]
    fn test_supported_language_conversion() {
        assert_eq!(SupportedLanguage::English.code(), "en");
        assert_eq!(SupportedLanguage::German.code(), "de");
        assert_eq!(SupportedLanguage::from_code("en"), Some(SupportedLanguage::English));
        assert_eq!(SupportedLanguage::from_code("deutsch"), Some(SupportedLanguage::German));
        assert_eq!(SupportedLanguage::from_code("invalid"), None);
    }

    #[test]
    fn test_service_creation() {
        let service = MultiLanguageManualImportService::new();
        assert!(service.is_ok());
    }

    #[test] 
    fn test_default_config() {
        let config = MultiLanguageImportConfig::default();
        assert_eq!(config.required_languages.len(), 1);
        assert_eq!(config.required_languages[0], SupportedLanguage::English);
        assert_eq!(config.optional_languages.len(), 4);
        assert!(config.allow_partial_import);
        assert!(config.recursive_scan);
    }

    #[test]
    fn test_folder_validation() {
        let service = MultiLanguageManualImportService::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let config = MultiLanguageImportConfig::default();

        let result = service.validate_folder_for_import(temp_dir.path(), &config);
        assert!(result.is_ok());
    }
}