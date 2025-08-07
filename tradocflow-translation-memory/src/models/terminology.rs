//! Terminology management models
//! 
//! Extracted from the original TradocFlow core translation models

use super::common::ValidationError;
use crate::error::{Result, TranslationMemoryError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A terminology entry represents a term with definition and metadata
/// Extracted from the original TradocFlow core translation models
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Term {
    /// Unique identifier
    pub id: Uuid,
    
    /// The term text
    pub term: String,
    
    /// Optional definition or explanation
    pub definition: Option<String>,
    
    /// Whether this term should not be translated
    pub do_not_translate: bool,
    
    /// When this entry was created
    pub created_at: DateTime<Utc>,
    
    /// When this entry was last modified
    pub updated_at: DateTime<Utc>,
}

/// CSV record structure for terminology import/export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminologyCsvRecord {
    /// The term text
    pub term: String,
    
    /// Optional definition
    pub definition: Option<String>,
    
    /// Do not translate flag as string ("true"/"false" or "yes"/"no")
    pub do_not_translate: Option<String>,
    
    /// Optional category for organization
    pub category: Option<String>,
    
    /// Optional notes
    pub notes: Option<String>,
}

/// Result of terminology import operation
#[derive(Debug, Clone)]
pub struct TerminologyImportResult {
    /// Successfully imported terms
    pub successful_imports: Vec<Term>,
    
    /// Failed imports with error details
    pub failed_imports: Vec<TerminologyImportError>,
    
    /// Terms that were duplicates
    pub duplicate_terms: Vec<String>,
    
    /// Total number of records processed
    pub total_processed: usize,
}

/// Error details for failed terminology imports
#[derive(Debug, Clone)]
pub struct TerminologyImportError {
    /// Row number in the source file
    pub row_number: usize,
    
    /// The term that failed to import
    pub term: String,
    
    /// The validation error that occurred
    pub error: ValidationError,
}

/// Conflict resolution strategies for duplicate terms
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictResolution {
    /// Skip the conflicting term
    Skip,
    
    /// Overwrite the existing term
    Overwrite,
    
    /// Merge definitions and notes
    Merge,
    
    /// Create a variant with suffix
    CreateVariant,
}

/// Configuration for terminology validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminologyValidationConfig {
    /// Whether term matching is case sensitive
    pub case_sensitive: bool,
    
    /// Whether to allow duplicate terms
    pub allow_duplicates: bool,
    
    /// How to resolve conflicts
    pub conflict_resolution: ConflictResolution,
    
    /// Maximum allowed term length
    pub max_term_length: usize,
    
    /// Maximum allowed definition length
    pub max_definition_length: usize,
    
    /// Required fields that must be present
    pub required_fields: Vec<String>,
}

impl Term {
    /// Create a new term with validation
    pub fn new(
        term: String,
        definition: Option<String>,
        do_not_translate: bool,
    ) -> Result<Self> {
        // Validate term
        if term.trim().is_empty() {
            return Err(TranslationMemoryError::DataValidation(
                "Term cannot be empty".to_string()
            ));
        }

        // Validate definition if provided
        if let Some(ref def) = definition {
            if def.trim().is_empty() {
                return Err(TranslationMemoryError::DataValidation(
                    "Definition cannot be empty if provided".to_string()
                ));
            }
        }

        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            term: term.trim().to_string(),
            definition: definition.map(|d| d.trim().to_string()),
            do_not_translate,
            created_at: now,
            updated_at: now,
        })
    }
    
    /// Update the term definition
    pub fn update_definition(&mut self, definition: Option<String>) -> Result<()> {
        if let Some(ref def) = definition {
            if def.trim().is_empty() {
                return Err(TranslationMemoryError::DataValidation(
                    "Definition cannot be empty if provided".to_string()
                ));
            }
        }

        self.definition = definition.map(|d| d.trim().to_string());
        self.updated_at = Utc::now();
        Ok(())
    }
    
    /// Update the do_not_translate flag
    pub fn set_do_not_translate(&mut self, do_not_translate: bool) {
        self.do_not_translate = do_not_translate;
        self.updated_at = Utc::now();
    }
    
    /// Validate the term
    pub fn validate(&self) -> Result<()> {
        if self.term.trim().is_empty() {
            return Err(TranslationMemoryError::DataValidation(
                "Term cannot be empty".to_string()
            ));
        }

        if let Some(ref def) = self.definition {
            if def.trim().is_empty() {
                return Err(TranslationMemoryError::DataValidation(
                    "Definition cannot be empty if provided".to_string()
                ));
            }
        }

        Ok(())
    }
}

impl TerminologyCsvRecord {
    /// Convert CSV record to Term model
    pub fn to_term(&self) -> Result<Term> {
        // Parse do_not_translate field
        let do_not_translate = match self.do_not_translate.as_deref() {
            Some("true") | Some("yes") | Some("1") | Some("True") | Some("Yes") => true,
            Some("false") | Some("no") | Some("0") | Some("False") | Some("No") => false,
            None => false,
            Some(value) => {
                return Err(TranslationMemoryError::DataValidation(
                    format!("Invalid do_not_translate value: '{}'. Expected true/false, yes/no, or 1/0", value)
                ));
            }
        };

        // Combine definition and notes if both exist
        let definition = match (&self.definition, &self.notes) {
            (Some(def), Some(notes)) if !def.trim().is_empty() && !notes.trim().is_empty() => {
                Some(format!("{}

Notes: {}", def.trim(), notes.trim()))
            }
            (Some(def), _) if !def.trim().is_empty() => Some(def.trim().to_string()),
            (_, Some(notes)) if !notes.trim().is_empty() => Some(format!("Notes: {}", notes.trim())),
            _ => None,
        };

        Term::new(self.term.clone(), definition, do_not_translate)
    }

    /// Create CSV record from Term model
    pub fn from_term(term: &Term) -> Self {
        let (definition, notes) = if let Some(ref def) = term.definition {
            if def.contains("\n\nNotes: ") {
                let parts: Vec<&str> = def.splitn(2, "\n\nNotes: ").collect();
                (
                    if parts[0].is_empty() { None } else { Some(parts[0].to_string()) },
                    Some(parts.get(1).unwrap_or(&"").to_string()),
                )
            } else if def.starts_with("Notes: ") {
                (None, Some(def.strip_prefix("Notes: ").unwrap_or(def).to_string()))
            } else {
                (Some(def.clone()), None)
            }
        } else {
            (None, None)
        };

        Self {
            term: term.term.clone(),
            definition,
            do_not_translate: Some(if term.do_not_translate { "true" } else { "false" }.to_string()),
            category: None, // Could be extended in the future
            notes,
        }
    }
}

impl TerminologyImportResult {
    /// Create a new empty result
    pub fn new() -> Self {
        Self {
            successful_imports: Vec::new(),
            failed_imports: Vec::new(),
            duplicate_terms: Vec::new(),
            total_processed: 0,
        }
    }

    /// Get the number of successful imports
    pub fn success_count(&self) -> usize {
        self.successful_imports.len()
    }

    /// Get the number of failed imports
    pub fn error_count(&self) -> usize {
        self.failed_imports.len()
    }

    /// Get the number of duplicate terms
    pub fn duplicate_count(&self) -> usize {
        self.duplicate_terms.len()
    }

    /// Check if there were any errors
    pub fn has_errors(&self) -> bool {
        !self.failed_imports.is_empty()
    }

    /// Check if there were any duplicates
    pub fn has_duplicates(&self) -> bool {
        !self.duplicate_terms.is_empty()
    }
}

impl Default for TerminologyImportResult {
    fn default() -> Self {
        Self::new()
    }
}

impl ConflictResolution {
    /// Get all available conflict resolution strategies
    pub fn all() -> Vec<ConflictResolution> {
        vec![
            ConflictResolution::Skip,
            ConflictResolution::Overwrite,
            ConflictResolution::Merge,
            ConflictResolution::CreateVariant,
        ]
    }

    /// Get strategy description
    pub fn description(&self) -> &'static str {
        match self {
            ConflictResolution::Skip => "Skip conflicting terms",
            ConflictResolution::Overwrite => "Overwrite existing terms",
            ConflictResolution::Merge => "Merge definitions and notes",
            ConflictResolution::CreateVariant => "Create variants with suffixes",
        }
    }
}

impl Default for TerminologyValidationConfig {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            allow_duplicates: false,
            conflict_resolution: ConflictResolution::Skip,
            max_term_length: 200,
            max_definition_length: 1000,
            required_fields: vec!["term".to_string()],
        }
    }
}

impl TerminologyValidationConfig {
    /// Validate a term against this configuration
    pub fn validate_term(&self, term: &Term) -> Result<()> {
        // Check term length
        if term.term.len() > self.max_term_length {
            return Err(TranslationMemoryError::DataValidation(
                format!("Term exceeds maximum length of {} characters", self.max_term_length)
            ));
        }

        // Check definition length
        if let Some(ref definition) = term.definition {
            if definition.len() > self.max_definition_length {
                return Err(TranslationMemoryError::DataValidation(
                    format!("Definition exceeds maximum length of {} characters", self.max_definition_length)
                ));
            }
        }

        // Check required fields
        for field in &self.required_fields {
            match field.as_str() {
                "term" => {
                    if term.term.trim().is_empty() {
                        return Err(TranslationMemoryError::DataValidation(
                            "Term is required".to_string()
                        ));
                    }
                }
                "definition" => {
                    if term.definition.is_none() || term.definition.as_ref().unwrap().trim().is_empty() {
                        return Err(TranslationMemoryError::DataValidation(
                            "Definition is required".to_string()
                        ));
                    }
                }
                _ => {} // Unknown field, ignore
            }
        }

        Ok(())
    }

    /// Check if two terms are considered duplicates based on configuration
    pub fn are_duplicates(&self, term1: &str, term2: &str) -> bool {
        if self.case_sensitive {
            term1 == term2
        } else {
            term1.to_lowercase() == term2.to_lowercase()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_term_creation() {
        let term = Term::new(
            "API".to_string(),
            Some("Application Programming Interface".to_string()),
            true,
        );
        
        assert!(term.is_ok());
        let term = term.unwrap();
        assert_eq!(term.term, "API");
        assert_eq!(term.definition, Some("Application Programming Interface".to_string()));
        assert!(term.do_not_translate);
    }
    
    #[test]
    fn test_term_validation_empty_term() {
        let result = Term::new(
            "".to_string(),
            Some("Definition".to_string()),
            false,
        );
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_term_validation_empty_definition() {
        let result = Term::new(
            "Term".to_string(),
            Some("".to_string()),
            false,
        );
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_term_update_definition() {
        let mut term = Term::new(
            "API".to_string(),
            Some("Old definition".to_string()),
            false,
        ).unwrap();
        
        let result = term.update_definition(Some("New definition".to_string()));
        assert!(result.is_ok());
        assert_eq!(term.definition, Some("New definition".to_string()));
        
        // Test removing definition
        let result = term.update_definition(None);
        assert!(result.is_ok());
        assert_eq!(term.definition, None);
    }
    
    #[test]
    fn test_terminology_csv_record_conversion() {
        let csv_record = TerminologyCsvRecord {
            term: "API".to_string(),
            definition: Some("Application Programming Interface".to_string()),
            do_not_translate: Some("true".to_string()),
            category: Some("Technical".to_string()),
            notes: Some("Commonly used in software development".to_string()),
        };
        
        let term = csv_record.to_term().unwrap();
        assert_eq!(term.term, "API");
        assert!(term.do_not_translate);
        assert!(term.definition.as_ref().unwrap().contains("Application Programming Interface"));
        assert!(term.definition.as_ref().unwrap().contains("Commonly used in software development"));
        
        // Test conversion back to CSV
        let csv_back = TerminologyCsvRecord::from_term(&term);
        assert_eq!(csv_back.term, "API");
        assert_eq!(csv_back.do_not_translate, Some("true".to_string()));
    }
    
    #[test]
    fn test_terminology_csv_boolean_parsing() {
        let test_cases = vec![
            ("true", true),
            ("false", false),
            ("yes", true),
            ("no", false),
            ("1", true),
            ("0", false),
            ("True", true),
            ("False", false),
        ];
        
        for (input, expected) in test_cases {
            let csv_record = TerminologyCsvRecord {
                term: "Test".to_string(),
                definition: None,
                do_not_translate: Some(input.to_string()),
                category: None,
                notes: None,
            };
            
            let term = csv_record.to_term().unwrap();
            assert_eq!(term.do_not_translate, expected, "Failed for input: {}", input);
        }
    }
    
    #[test]
    fn test_terminology_csv_invalid_boolean() {
        let csv_record = TerminologyCsvRecord {
            term: "Test".to_string(),
            definition: None,
            do_not_translate: Some("invalid".to_string()),
            category: None,
            notes: None,
        };
        
        let result = csv_record.to_term();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_terminology_validation_config() {
        let config = TerminologyValidationConfig::default();
        
        // Test valid term
        let valid_term = Term::new(
            "API".to_string(),
            Some("Application Programming Interface".to_string()),
            false,
        ).unwrap();
        assert!(config.validate_term(&valid_term).is_ok());
        
        // Test term too long
        let long_term = Term::new(
            "A".repeat(config.max_term_length + 1),
            None,
            false,
        ).unwrap();
        assert!(config.validate_term(&long_term).is_err());
        
        // Test duplicate detection
        assert!(config.are_duplicates("API", "api")); // Case insensitive by default
        
        let case_sensitive_config = TerminologyValidationConfig {
            case_sensitive: true,
            ..Default::default()
        };
        assert!(!case_sensitive_config.are_duplicates("API", "api")); // Case sensitive
    }
    
    #[test]
    fn test_terminology_import_result() {
        let mut result = TerminologyImportResult::new();
        assert_eq!(result.success_count(), 0);
        assert_eq!(result.error_count(), 0);
        assert!(!result.has_errors());
        
        result.successful_imports.push(Term::new("Test".to_string(), None, false).unwrap());
        assert_eq!(result.success_count(), 1);
        
        result.failed_imports.push(TerminologyImportError {
            row_number: 1,
            term: "Invalid".to_string(),
            error: ValidationError::InvalidTerm("Test error".to_string()),
        });
        assert_eq!(result.error_count(), 1);
        assert!(result.has_errors());
    }
    
    #[test]
    fn test_conflict_resolution_strategies() {
        let strategies = ConflictResolution::all();
        assert_eq!(strategies.len(), 4);
        assert!(strategies.contains(&ConflictResolution::Skip));
        assert!(strategies.contains(&ConflictResolution::Overwrite));
        assert!(strategies.contains(&ConflictResolution::Merge));
        assert!(strategies.contains(&ConflictResolution::CreateVariant));
        
        assert_eq!(ConflictResolution::Skip.description(), "Skip conflicting terms");
        assert_eq!(ConflictResolution::Overwrite.description(), "Overwrite existing terms");
    }
}