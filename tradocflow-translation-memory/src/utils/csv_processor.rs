//! CSV processing utilities for terminology and translation data

use crate::error::{Result, TranslationMemoryError};
use crate::models::{TranslationUnit, TerminologyCsvRecord, Term};
use std::path::Path;

#[cfg(feature = "terminology-csv")]
use std::fs::File;
#[cfg(feature = "terminology-csv")]
use csv::{Reader, Writer, StringRecord};
#[cfg(feature = "terminology-csv")]
use std::collections::HashMap;

/// Utility for processing CSV files with translation and terminology data
#[derive(Debug)]
pub struct CsvProcessor;

impl CsvProcessor {
    /// Create a new CSV processor
    pub fn new() -> Self {
        Self
    }

    /// Import translation units from CSV
    pub async fn import_from_csv(_file_path: &str) -> Result<Vec<TranslationUnit>> {
        // TODO: Implement translation unit CSV import when needed
        Ok(Vec::new())
    }
    
    /// Export translation units to CSV (static method)
    pub async fn export_translation_units_to_csv(_units: &[TranslationUnit], _file_path: &str) -> Result<()> {
        // TODO: Implement translation unit CSV export when needed
        Ok(())
    }
    
    /// Validate CSV format for terminology files
    #[cfg(feature = "terminology-csv")]
    pub async fn validate_csv_format(file_path: &str) -> Result<bool> {
        if !Path::new(file_path).exists() {
            return Err(TranslationMemoryError::FileOperationError(
                format!("CSV file not found: {}", file_path)
            ));
        }

        let file = File::open(file_path)?;
        let mut reader = Reader::from_reader(file);
        
        // Check if we can read at least the headers
        match reader.headers() {
            Ok(headers) => {
                // Validate that we have at least a 'term' column
                let has_term_column = headers.iter().any(|header| 
                    header.trim().to_lowercase() == "term"
                );
                
                if !has_term_column {
                    return Err(TranslationMemoryError::DataValidation(
                        "CSV file must contain a 'term' column".to_string()
                    ));
                }
                
                // Try to read first record to validate format
                for (i, result) in reader.records().enumerate() {
                    match result {
                        Ok(_) => {
                            // Only validate first few records for performance
                            if i >= 10 {
                                break;
                            }
                        }
                        Err(e) => {
                            return Err(TranslationMemoryError::ParsingError(
                                format!("Invalid CSV format at record {}: {}", i + 1, e)
                            ));
                        }
                    }
                }
                
                Ok(true)
            }
            Err(e) => {
                Err(TranslationMemoryError::ParsingError(
                    format!("Failed to read CSV headers: {}", e)
                ))
            }
        }
    }

    #[cfg(not(feature = "terminology-csv"))]
    pub async fn validate_csv_format(_file_path: &str) -> Result<bool> {
        Err(TranslationMemoryError::UnsupportedOperation(
            "CSV functionality is not enabled. Enable 'terminology-csv' feature".to_string()
        ))
    }
    
    /// Parse CSV file and return terminology records
    #[cfg(feature = "terminology-csv")]
    pub async fn parse_csv(&self, file_path: &Path) -> Result<Vec<TerminologyCsvRecord>> {
        if !file_path.exists() {
            return Err(TranslationMemoryError::FileOperationError(
                format!("CSV file not found: {}", file_path.display())
            ));
        }

        let file = File::open(file_path)?;
        let mut reader = Reader::from_reader(file);
        let mut records = Vec::new();
        
        // Get headers to map column positions
        let headers = reader.headers()?;
        let header_map = self.create_header_map(headers);
        
        for (row_num, result) in reader.records().enumerate() {
            match result {
                Ok(record) => {
                    match self.parse_record_to_terminology(&record, &header_map) {
                        Ok(terminology_record) => {
                            records.push(terminology_record);
                        }
                        Err(e) => {
                            // Log parsing errors but continue processing
                            log::warn!("Failed to parse CSV record at row {}: {}", row_num + 2, e);
                        }
                    }
                }
                Err(e) => {
                    return Err(TranslationMemoryError::ParsingError(
                        format!("CSV parsing error at row {}: {}", row_num + 2, e)
                    ));
                }
            }
        }
        
        Ok(records)
    }

    #[cfg(not(feature = "terminology-csv"))]
    pub async fn parse_csv(&self, _file_path: &Path) -> Result<Vec<TerminologyCsvRecord>> {
        Err(TranslationMemoryError::UnsupportedOperation(
            "CSV functionality is not enabled. Enable 'terminology-csv' feature".to_string()
        ))
    }
    
    /// Export terminology to CSV
    #[cfg(feature = "terminology-csv")]
    pub async fn export_to_csv(&self, terms: &[Term], file_path: &Path) -> Result<()> {
        let file = File::create(file_path)?;
        let mut writer = Writer::from_writer(file);
        
        // Write headers
        writer.write_record(&[
            "term",
            "definition", 
            "do_not_translate",
            "category",
            "notes"
        ])?;
        
        // Write term data
        for term in terms {
            let csv_record = TerminologyCsvRecord::from_term(term);
            
            writer.write_record(&[
                &csv_record.term,
                csv_record.definition.as_deref().unwrap_or(""),
                csv_record.do_not_translate.as_deref().unwrap_or("false"),
                csv_record.category.as_deref().unwrap_or(""),
                csv_record.notes.as_deref().unwrap_or(""),
            ])?;
        }
        
        writer.flush()?;
        Ok(())
    }

    #[cfg(not(feature = "terminology-csv"))]
    pub async fn export_to_csv(&self, _terms: &[Term], _file_path: &Path) -> Result<()> {
        Err(TranslationMemoryError::UnsupportedOperation(
            "CSV functionality is not enabled. Enable 'terminology-csv' feature".to_string()
        ))
    }

    /// Get CSV export statistics
    pub fn get_export_stats(&self, terms: &[Term]) -> CsvExportStats {
        let total_terms = terms.len();
        let do_not_translate_count = terms.iter().filter(|t| t.do_not_translate).count();
        let with_definition_count = terms.iter().filter(|t| t.definition.is_some()).count();
        
        CsvExportStats {
            total_terms,
            do_not_translate_count,
            with_definition_count,
            translatable_count: total_terms - do_not_translate_count,
        }
    }

    // Private helper methods

    #[cfg(feature = "terminology-csv")]
    fn create_header_map(&self, headers: &StringRecord) -> HashMap<String, usize> {
        let mut map = HashMap::new();
        
        for (index, header) in headers.iter().enumerate() {
            let normalized = header.trim().to_lowercase();
            map.insert(normalized, index);
        }
        
        map
    }

    #[cfg(feature = "terminology-csv")]
    fn parse_record_to_terminology(
        &self,
        record: &StringRecord,
        header_map: &HashMap<String, usize>,
    ) -> Result<TerminologyCsvRecord> {
        // Get term (required field)
        let term = self.get_field_value(record, header_map, "term")
            .ok_or_else(|| TranslationMemoryError::DataValidation(
                "Missing required 'term' field".to_string()
            ))?;
        
        if term.trim().is_empty() {
            return Err(TranslationMemoryError::DataValidation(
                "Term cannot be empty".to_string()
            ));
        }
        
        // Get optional fields
        let definition = self.get_field_value(record, header_map, "definition")
            .filter(|s| !s.trim().is_empty());
        
        let do_not_translate = self.get_field_value(record, header_map, "do_not_translate")
            .filter(|s| !s.trim().is_empty());
        
        let category = self.get_field_value(record, header_map, "category")
            .filter(|s| !s.trim().is_empty());
        
        let notes = self.get_field_value(record, header_map, "notes")
            .filter(|s| !s.trim().is_empty());
        
        Ok(TerminologyCsvRecord {
            term,
            definition,
            do_not_translate,
            category,
            notes,
        })
    }

    #[cfg(feature = "terminology-csv")]
    fn get_field_value(
        &self,
        record: &StringRecord,
        header_map: &HashMap<String, usize>,
        field_name: &str,
    ) -> Option<String> {
        header_map.get(field_name)
            .and_then(|&index| record.get(index))
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty())
    }
}

impl Default for CsvProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for CSV export operations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CsvExportStats {
    pub total_terms: usize,
    pub do_not_translate_count: usize,
    pub translatable_count: usize,
    pub with_definition_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_csv_processor_creation() {
        let processor = CsvProcessor::new();
        let default_processor = CsvProcessor::default();
        
        // Both should be Debug-printable
        println!("Processor: {:?}", processor);
        println!("Default: {:?}", default_processor);
    }

    #[cfg(feature = "terminology-csv")]
    #[tokio::test]
    async fn test_parse_valid_csv() {
        let csv_content = r#"term,definition,do_not_translate
API,Application Programming Interface,true
database,A structured collection of data,false
JSON,JavaScript Object Notation,true"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(csv_content.as_bytes()).unwrap();
        
        let processor = CsvProcessor::new();
        let result = processor.parse_csv(temp_file.path()).await;
        
        assert!(result.is_ok());
        let records = result.unwrap();
        assert_eq!(records.len(), 3);
        
        assert_eq!(records[0].term, "API");
        assert_eq!(records[0].do_not_translate, Some("true".to_string()));
        
        assert_eq!(records[1].term, "database");
        assert_eq!(records[1].do_not_translate, Some("false".to_string()));
    }

    #[cfg(feature = "terminology-csv")]
    #[tokio::test]
    async fn test_validate_csv_format() {
        let valid_csv = r#"term,definition
API,Application Programming Interface
database,A structured collection of data"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(valid_csv.as_bytes()).unwrap();
        
        let result = CsvProcessor::validate_csv_format(temp_file.path().to_str().unwrap()).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[cfg(feature = "terminology-csv")]
    #[tokio::test]
    async fn test_validate_invalid_csv_format() {
        let invalid_csv = r#"name,description
API,Application Programming Interface"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(invalid_csv.as_bytes()).unwrap();
        
        let result = CsvProcessor::validate_csv_format(temp_file.path().to_str().unwrap()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_export_stats() {
        let terms = vec![
            Term::new("API".to_string(), Some("Application Programming Interface".to_string()), true).unwrap(),
            Term::new("database".to_string(), None, false).unwrap(),
            Term::new("JSON".to_string(), Some("JavaScript Object Notation".to_string()), true).unwrap(),
        ];
        
        let processor = CsvProcessor::new();
        let stats = processor.get_export_stats(&terms);
        
        assert_eq!(stats.total_terms, 3);
        assert_eq!(stats.do_not_translate_count, 2);
        assert_eq!(stats.translatable_count, 1);
        assert_eq!(stats.with_definition_count, 2);
    }
}