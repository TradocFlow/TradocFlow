//! Terminology service for managing terminology databases with async operations

use crate::error::{Result, TranslationMemoryError};
use crate::models::{Terminology, Language, TerminologyImportResult as ModelImportResult};
// Temporarily disable storage dependencies due to version conflicts
// use crate::storage::{DuckDBManager, ParquetManager};
use crate::utils::CsvProcessor;
use std::sync::Arc;
use std::path::Path;
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};


/// Service-specific terminology import result with additional details
/// Uses the model's TerminologyImportResult as base
#[derive(Debug, Clone)]
pub struct ServiceTerminologyImportResult {
    pub base_result: ModelImportResult,
    pub conflicts: Vec<TermConflict>,
    pub warnings: Vec<String>,
    pub processing_time_ms: u64,
}

/// Term validation result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TermValidationResult {
    pub conflicts: Vec<TermConflict>,
    pub warnings: Vec<String>,
    pub duplicates: Vec<String>,
}

/// Term conflict information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TermConflict {
    pub term: String,
    pub existing_definition: Option<String>,
    pub new_definition: Option<String>,
    pub conflict_type: ConflictType,
}

/// Type of terminology conflict
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ConflictType {
    Definition,
    DoNotTranslate,
    Duplicate,
}

/// Terminology suggestion for user interface
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TermSuggestion {
    pub term: String,
    pub definition: Option<String>,
    pub do_not_translate: bool,
    pub confidence: f32,
    pub position: Option<usize>,
    pub context: Option<String>,
}

/// Error details for failed terminology imports
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TerminologyImportError {
    pub row_number: usize,
    pub term: String,
    pub error_message: String,
    pub error_type: ImportErrorType,
}

/// Type of import error
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ImportErrorType {
    Validation,
    Duplicate,
    Format,
    Database,
}

/// Terminology validation configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TerminologyValidationConfig {
    pub case_sensitive: bool,
    pub allow_duplicates: bool,
    pub max_term_length: usize,
    pub max_definition_length: usize,
    pub required_fields: Vec<String>,
}

impl Default for TerminologyValidationConfig {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            allow_duplicates: false,
            max_term_length: 200,
            max_definition_length: 1000,
            required_fields: vec!["term".to_string()],
        }
    }
}

/// In-memory cache for terminology operations
#[derive(Debug, Default)]
struct TerminologyCache {
    terms_by_project: HashMap<Uuid, Vec<Terminology>>,
    search_results: HashMap<String, Vec<Terminology>>,
    non_translatable_terms: HashMap<Uuid, Vec<Terminology>>,
    last_updated: Option<DateTime<Utc>>,
}

/// Thread-safe terminology service with async operations  
#[derive(Debug, Clone)]
pub struct TerminologyService {
    // Temporarily disable storage managers due to version conflicts
    // duckdb_manager: Arc<DuckDBManager>,
    // parquet_manager: Arc<ParquetManager>,
    csv_processor: Arc<CsvProcessor>,
    cache: Arc<RwLock<TerminologyCache>>,
    validation_config: TerminologyValidationConfig,
    // In-memory storage for terms until database is available
    in_memory_storage: Arc<RwLock<HashMap<Uuid, Vec<Terminology>>>>,
}

impl TerminologyService {
    /// Search for terminology matches (legacy API for lib.rs compatibility)
    pub async fn search_terms(
        &self,
        _query: &str,
        _source_lang: Language,
        _target_lang: Language,
    ) -> Result<Vec<crate::models::Term>> {
        // TODO: Implement terminology search when storage is available
        Ok(Vec::new())
    }
    /// Create a new terminology service
    pub async fn new(
        csv_processor: Arc<CsvProcessor>,
        validation_config: Option<TerminologyValidationConfig>,
    ) -> Result<Self> {
        let service = Self {
            csv_processor,
            cache: Arc::new(RwLock::new(TerminologyCache::default())),
            validation_config: validation_config.unwrap_or_default(),
            in_memory_storage: Arc::new(RwLock::new(HashMap::new())),
        };
        
        service.initialize().await?;
        Ok(service)
    }
    
    /// Initialize the service and create necessary tables
    pub async fn initialize(&self) -> Result<()> {
        // TODO: Initialize database schema when DuckDB is available
        Ok(())
    }
    
    /// Import terminology from CSV file with comprehensive validation
    pub async fn import_terminology_csv(
        &self,
        file_path: &Path,
        project_id: Uuid,
    ) -> Result<ServiceTerminologyImportResult> {
        let start_time = std::time::Instant::now();
        
        // Parse CSV file
        let csv_records = self.csv_processor.parse_csv(file_path).await?;
        
        // Convert to terminology objects
        let mut terms = Vec::new();
        let mut failed_terms = Vec::new();
        
        for (row_number, record) in csv_records.iter().enumerate() {
            match record.to_term() {
                Ok(term) => {
                    // Convert Term to Terminology for service use
                    let terminology = Terminology {
                        id: term.id,
                        term: term.term.clone(),
                        definition: term.definition,
                        do_not_translate: term.do_not_translate,
                        created_at: term.created_at,
                        updated_at: term.updated_at,
                    };
                    
                    if let Err(validation_error) = self.validate_terminology(&terminology) {
                        failed_terms.push(TerminologyImportError {
                            row_number: row_number + 1,
                            term: term.term.clone(),
                            error_message: validation_error.to_string(),
                            error_type: ImportErrorType::Validation,
                        });
                    } else {
                        terms.push(terminology);
                    }
                },
                Err(e) => {
                    failed_terms.push(TerminologyImportError {
                        row_number: row_number + 1,
                        term: record.term.clone(),
                        error_message: e.to_string(),
                        error_type: ImportErrorType::Format,
                    });
                }
            }
        }
        
        // Validate terms and detect conflicts
        let validation_result = self.validate_terms(&terms, project_id).await?;
        
        // Filter out conflicting terms if not allowed
        let final_terms = if self.validation_config.allow_duplicates {
            terms
        } else {
            terms.into_iter()
                .filter(|term| !validation_result.duplicates.contains(&term.term))
                .collect()
        };
        
        // Store valid terms
        let mut _imported_count = 0;
        for term in &final_terms {
            match self.add_terminology_internal(term.clone(), project_id).await {
                Ok(_) => _imported_count += 1,
                Err(e) => {
                    failed_terms.push(TerminologyImportError {
                        row_number: 0, // Unknown row at this point
                        term: term.term.clone(),
                        error_message: e.to_string(),
                        error_type: ImportErrorType::Database,
                    });
                }
            }
        }
        
        // TODO: Convert to Parquet format when storage manager is available
        
        // Invalidate cache
        self.invalidate_cache().await;
        
        let processing_time = start_time.elapsed();
        
        let base_result = ModelImportResult {
            successful_imports: final_terms,
            failed_imports: failed_terms.into_iter().map(|err| crate::models::TerminologyImportError {
                row_number: err.row_number,
                term: err.term,
                error: crate::models::ValidationError::InvalidTerm(err.error_message),
            }).collect(),
            duplicate_terms: Vec::new(), // TODO: Extract duplicates from validation
            total_processed: csv_records.len(),
        };
        
        Ok(ServiceTerminologyImportResult {
            base_result,
            conflicts: validation_result.conflicts,
            warnings: validation_result.warnings,
            processing_time_ms: processing_time.as_millis() as u64,
        })
    }
    
    /// Export terminology to CSV file
    pub async fn export_terminology_csv(
        &self,
        project_id: Uuid,
        output_path: &Path,
    ) -> Result<usize> {
        let terms = self.get_terms_by_project(project_id).await?;
        self.csv_processor.export_to_csv(&terms, output_path).await?;
        Ok(terms.len())
    }
    
    /// Get all terminology entries for a project
    pub async fn get_terms_by_project(&self, project_id: Uuid) -> Result<Vec<Terminology>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached_terms) = cache.terms_by_project.get(&project_id) {
                return Ok(cached_terms.clone());
            }
        }
        
        // Fetch from in-memory storage
        let terms = {
            let storage = self.in_memory_storage.read().await;
            storage.get(&project_id).cloned().unwrap_or_default()
        };
        
        // Cache the results
        {
            let mut cache = self.cache.write().await;
            cache.terms_by_project.insert(project_id, terms.clone());
            cache.last_updated = Some(Utc::now());
        }
        
        Ok(terms)
    }
    
    /// Get non-translatable terms for a project
    pub async fn get_non_translatable_terms(&self, project_id: Uuid) -> Result<Vec<Terminology>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached_terms) = cache.non_translatable_terms.get(&project_id) {
                return Ok(cached_terms.clone());
            }
        }
        
        let all_terms = self.get_terms_by_project(project_id).await?;
        let non_translatable: Vec<Terminology> = all_terms
            .into_iter()
            .filter(|term| term.do_not_translate)
            .collect();
        
        // Cache the results
        {
            let mut cache = self.cache.write().await;
            cache.non_translatable_terms.insert(project_id, non_translatable.clone());
        }
        
        Ok(non_translatable)
    }
    
    /// Search for terminology entries (alternative method signature)
    pub async fn search_terms_by_project(
        &self,
        query: &str,
        project_id: Uuid,
        case_sensitive: Option<bool>,
    ) -> Result<Vec<Terminology>> {
        let cache_key = format!("{}:{}", project_id, query);
        
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(cached_results) = cache.search_results.get(&cache_key) {
                return Ok(cached_results.clone());
            }
        }
        
        let case_sensitive = case_sensitive.unwrap_or(self.validation_config.case_sensitive);
        
        // Get all terms for the project and filter by search query
        let all_terms = self.get_terms_by_project(project_id).await?;
        let query_lower = query.to_lowercase();
        
        let results: Vec<Terminology> = all_terms.into_iter()
            .filter(|term| {
                if case_sensitive {
                    term.term.contains(query) || 
                    term.definition.as_ref().map_or(false, |def| def.contains(query))
                } else {
                    term.term.to_lowercase().contains(&query_lower) ||
                    term.definition.as_ref().map_or(false, |def| def.to_lowercase().contains(&query_lower))
                }
            })
            .collect();
        
        // Cache the results
        {
            let mut cache = self.cache.write().await;
            cache.search_results.insert(cache_key, results.clone());
        }
        
        Ok(results)
    }
    
    /// Get term suggestions for text analysis
    pub async fn get_term_suggestions(
        &self,
        text: &str,
        project_id: Uuid,
    ) -> Result<Vec<TermSuggestion>> {
        let terms = self.get_terms_by_project(project_id).await?;
        let mut suggestions = Vec::new();
        
        for term in terms {
            if let Some(confidence) = self.calculate_term_confidence(&term.term, text) {
                if confidence > 0.5 {
                    let position = if self.validation_config.case_sensitive {
                        text.find(&term.term)
                    } else {
                        text.to_lowercase().find(&term.term.to_lowercase())
                    };
                    
                    suggestions.push(TermSuggestion {
                        term: term.term.clone(),
                        definition: term.definition.clone(),
                        do_not_translate: term.do_not_translate,
                        confidence,
                        position,
                        context: Some(self.extract_context(text, position, &term.term)),
                    });
                }
            }
        }
        
        // Sort by confidence
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(suggestions)
    }
    
    /// Add a new terminology entry
    pub async fn add_terminology(&self, terminology: Terminology, project_id: Uuid) -> Result<()> {
        // Validate the terminology
        self.validate_terminology(&terminology)?;
        
        // Check for duplicates if not allowed
        if !self.validation_config.allow_duplicates {
            let existing_terms = self.get_terms_by_project(project_id).await?;
            for existing_term in &existing_terms {
                if self.are_duplicate_terms(&terminology.term, &existing_term.term) {
                    return Err(TranslationMemoryError::ValidationError(
                        format!("Term '{}' already exists in project", terminology.term)
                    ).into());
                }
            }
        }
        
        self.add_terminology_internal(terminology, project_id).await
    }
    
    /// Update an existing terminology entry
    pub async fn update_terminology(
        &self,
        mut terminology: Terminology,
        project_id: Uuid,
    ) -> Result<()> {
        // Validate the terminology
        self.validate_terminology(&terminology)?;
        
        // Update timestamp
        terminology.updated_at = Utc::now();
        
        // Store ID for error message before move
        let terminology_id = terminology.id;
        
        // Update in in-memory storage
        let updated = {
            let mut storage = self.in_memory_storage.write().await;
            if let Some(project_terms) = storage.get_mut(&project_id) {
                if let Some(existing_pos) = project_terms.iter().position(|t| t.id == terminology_id) {
                    project_terms[existing_pos] = terminology;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };
        
        if !updated {
            return Err(TranslationMemoryError::NotFound(
                format!("Terminology with ID {} not found", terminology_id)
            ));
        }
        
        // Invalidate cache
        self.invalidate_cache().await;
        
        Ok(())
    }
    
    /// Delete a terminology entry
    pub async fn delete_terminology(&self, id: Uuid, project_id: Uuid) -> Result<bool> {
        // Delete from in-memory storage
        let deleted = {
            let mut storage = self.in_memory_storage.write().await;
            if let Some(project_terms) = storage.get_mut(&project_id) {
                if let Some(pos) = project_terms.iter().position(|t| t.id == id) {
                    project_terms.remove(pos);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };
        
        if deleted {
            // Invalidate cache
            self.invalidate_cache().await;
        }
        
        Ok(deleted)
    }
    
    /// Update terminology for a project (batch operation)
    pub async fn update_project_terminology(
        &self,
        project_id: Uuid,
        mut terms: Vec<Terminology>,
    ) -> Result<usize> {
        // Validate all terms first
        for term in &terms {
            self.validate_terminology(term)?;
        }
        
        // Update timestamps
        let now = Utc::now();
        for term in &mut terms {
            term.updated_at = now;
        }
        
        // Batch update in in-memory storage
        let updated_count = {
            let mut storage = self.in_memory_storage.write().await;
            let project_terms = storage.entry(project_id).or_insert_with(Vec::new);
            
            let mut count = 0;
            for new_term in terms {
                if let Some(existing_pos) = project_terms.iter().position(|t| t.id == new_term.id) {
                    project_terms[existing_pos] = new_term;
                    count += 1;
                } else {
                    // If not found by ID, try to find by term text for updates
                    if let Some(existing_pos) = project_terms.iter().position(|t| 
                        self.are_duplicate_terms(&t.term, &new_term.term)
                    ) {
                        project_terms[existing_pos] = new_term;
                        count += 1;
                    } else {
                        // Add as new term
                        project_terms.push(new_term);
                        count += 1;
                    }
                }
            }
            count
        };
        
        // Invalidate cache
        self.invalidate_cache().await;
        
        Ok(updated_count)
    }
    
    /// Get cache statistics for monitoring
    pub async fn get_cache_stats(&self) -> (usize, usize, usize, Option<DateTime<Utc>>) {
        let cache = self.cache.read().await;
        (
            cache.terms_by_project.len(),
            cache.search_results.len(),
            cache.non_translatable_terms.len(),
            cache.last_updated,
        )
    }
    
    /// Clear the cache manually
    pub async fn clear_cache(&self) {
        self.invalidate_cache().await;
    }
    
    // Private helper methods
    
    async fn add_terminology_internal(
        &self,
        terminology: Terminology,
        project_id: Uuid,
    ) -> Result<()> {
        // Add to in-memory storage
        {
            let mut storage = self.in_memory_storage.write().await;
            let project_terms = storage.entry(project_id).or_insert_with(Vec::new);
            
            // Check for duplicates if not allowed
            if !self.validation_config.allow_duplicates {
                if project_terms.iter().any(|t| self.are_duplicate_terms(&t.term, &terminology.term)) {
                    return Err(TranslationMemoryError::ValidationError(
                        format!("Term '{}' already exists in project", terminology.term)
                    ));
                }
            }
            
            // Update existing term or add new one
            if let Some(existing_pos) = project_terms.iter().position(|t| t.id == terminology.id) {
                project_terms[existing_pos] = terminology;
            } else {
                project_terms.push(terminology);
            }
        }
        
        // Invalidate cache
        self.invalidate_cache().await;
        
        Ok(())
    }
    
    async fn validate_terms(
        &self,
        terms: &[Terminology],
        project_id: Uuid,
    ) -> Result<TermValidationResult> {
        let existing_terms = self.get_terms_by_project(project_id).await?;
        
        let mut conflicts = Vec::new();
        let mut warnings = Vec::new();
        let mut duplicates = Vec::new();
        
        for term in terms {
            // Check for conflicts with existing terms
            for existing in &existing_terms {
                if self.are_duplicate_terms(&term.term, &existing.term) {
                    if term.definition != existing.definition {
                        conflicts.push(TermConflict {
                            term: term.term.clone(),
                            existing_definition: existing.definition.clone(),
                            new_definition: term.definition.clone(),
                            conflict_type: ConflictType::Definition,
                        });
                    }
                    
                    if term.do_not_translate != existing.do_not_translate {
                        conflicts.push(TermConflict {
                            term: term.term.clone(),
                            existing_definition: Some(existing.do_not_translate.to_string()),
                            new_definition: Some(term.do_not_translate.to_string()),
                            conflict_type: ConflictType::DoNotTranslate,
                        });
                    }
                    
                    duplicates.push(term.term.clone());
                }
            }
            
            // Generate warnings
            if term.term.is_empty() {
                warnings.push("Empty term found".to_string());
            }
            
            if term.term.len() > self.validation_config.max_term_length {
                warnings.push(format!("Term '{}' exceeds maximum length", term.term));
            }
            
            if let Some(ref definition) = term.definition {
                if definition.len() > self.validation_config.max_definition_length {
                    warnings.push(format!("Definition for '{}' exceeds maximum length", term.term));
                }
            }
        }
        
        Ok(TermValidationResult {
            conflicts,
            warnings,
            duplicates,
        })
    }
    
    fn validate_terminology(&self, terminology: &Terminology) -> Result<()> {
        // Check required fields
        if terminology.term.trim().is_empty() {
            return Err(TranslationMemoryError::ValidationError("Term cannot be empty".to_string()).into());
        }
        
        // Check term length
        if terminology.term.len() > self.validation_config.max_term_length {
            return Err(TranslationMemoryError::ValidationError(
                format!("Term exceeds maximum length of {} characters", self.validation_config.max_term_length)
            ).into());
        }
        
        // Check definition length
        if let Some(ref definition) = terminology.definition {
            if definition.len() > self.validation_config.max_definition_length {
                return Err(TranslationMemoryError::ValidationError(
                    format!("Definition exceeds maximum length of {} characters", self.validation_config.max_definition_length)
                ).into());
            }
        }
        
        Ok(())
    }
    
    fn are_duplicate_terms(&self, term1: &str, term2: &str) -> bool {
        if self.validation_config.case_sensitive {
            term1 == term2
        } else {
            term1.to_lowercase() == term2.to_lowercase()
        }
    }
    
    fn calculate_term_confidence(&self, term: &str, text: &str) -> Option<f32> {
        let term_lower = term.to_lowercase();
        let text_lower = text.to_lowercase();
        
        if text_lower.contains(&term_lower) {
            // Use regex for word boundary matching
            let pattern = format!(r"\b{}\b", regex::escape(&term_lower));
            match regex::Regex::new(&pattern) {
                Ok(regex) => {
                    if regex.is_match(&text_lower) {
                        Some(0.9) // High confidence for exact word boundary match
                    } else {
                        Some(0.7) // Lower confidence for substring match
                    }
                }
                Err(_) => {
                    // Fallback to basic substring match if regex fails
                    Some(0.6)
                }
            }
        } else {
            None
        }
    }
    
    fn extract_context(&self, text: &str, position: Option<usize>, term: &str) -> String {
        if let Some(pos) = position {
            let start = pos.saturating_sub(30);
            let end = std::cmp::min(pos + term.len() + 30, text.len());
            let context = &text[start..end];
            
            if start > 0 {
                format!("...{}", context)
            } else if end < text.len() {
                format!("{}...", context)
            } else {
                context.to_string()
            }
        } else {
            "No context available".to_string()
        }
    }
    
    async fn invalidate_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.terms_by_project.clear();
        cache.search_results.clear();
        cache.non_translatable_terms.clear();
        cache.last_updated = Some(Utc::now());
    }
}