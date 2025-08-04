use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use duckdb::{Connection, params};
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::file::properties::WriterProperties;
use arrow::array::{StringArray, BooleanArray, Int64Array, RecordBatch};
use arrow::datatypes::{DataType, Field, Schema};

use crate::models::Term;

/// Service for managing terminology databases and CSV import/export
pub struct TerminologyService {
    terminology_repository: Arc<TerminologyRepository>,
    csv_processor: Arc<CsvProcessor>,
    parquet_converter: Arc<ParquetConverter>,
    project_path: PathBuf,
}

impl TerminologyService {
    pub fn new(project_path: PathBuf) -> Result<Self> {
        let terminology_path = project_path.join("terminology");
        std::fs::create_dir_all(&terminology_path)?;
        
        let service = Self {
            terminology_repository: Arc::new(TerminologyRepository::new(terminology_path.clone())?),
            csv_processor: Arc::new(CsvProcessor::new()),
            parquet_converter: Arc::new(ParquetConverter::new(terminology_path.clone())?),
            project_path,
        };
        
        Ok(service)
    }
    
    pub async fn import_terminology_csv(
        &self,
        file_path: &Path,
        project_id: Uuid,
    ) -> Result<TerminologyImportResult> {
        let start_time = std::time::Instant::now();
        
        // Parse CSV file
        let terms = self.csv_processor.parse_csv(file_path).await?;
        
        // Validate terms and detect conflicts
        let validation_result = self.validate_terms(&terms, project_id).await?;
        
        // Convert to Parquet format
        let parquet_path = self.parquet_converter
            .convert_terms_to_parquet(&terms, project_id)
            .await?;
        
        // Store in repository
        for term in &terms {
            self.terminology_repository.add_term(term.clone(), project_id).await?;
        }
        
        let processing_time = start_time.elapsed();
        
        Ok(TerminologyImportResult {
            imported_count: terms.len(),
            conflicts: validation_result.conflicts,
            warnings: validation_result.warnings,
            parquet_file: parquet_path,
            processing_time_ms: processing_time.as_millis() as u64,
        })
    }
    
    pub async fn get_non_translatable_terms(&self, project_id: Uuid) -> Result<Vec<Term>> {
        let terms = self.terminology_repository
            .get_terms_by_project(project_id)
            .await?
            .into_iter()
            .filter(|term| term.do_not_translate)
            .collect::<Vec<_>>();
        Ok(terms)
    }
    
    pub async fn update_terminology(&self, project_id: Uuid, terms: Vec<Term>) -> Result<()> {
        // Update repository
        for term in &terms {
            self.terminology_repository.update_term(term.clone(), project_id).await?;
        }
        
        // Refresh Parquet files
        self.parquet_converter
            .refresh_parquet_files(project_id, &terms)
            .await?;
        
        Ok(())
    }
    
    pub async fn export_terminology_csv(&self, project_id: Uuid) -> Result<PathBuf> {
        let terms = self.terminology_repository
            .get_terms_by_project(project_id)
            .await?;
        
        let export_path = self.project_path
            .join("terminology")
            .join(format!("export_{}.csv", project_id));
        
        self.csv_processor.export_to_csv(&terms, &export_path).await?;
        
        Ok(export_path)
    }
    
    pub async fn search_terms(&self, query: &str, project_id: Uuid) -> Result<Vec<Term>> {
        self.terminology_repository.search_terms(query, project_id).await
    }
    
    pub async fn get_term_suggestions(&self, text: &str, project_id: Uuid) -> Result<Vec<TermSuggestion>> {
        let terms = self.terminology_repository
            .get_terms_by_project(project_id)
            .await?;
        
        let mut suggestions = Vec::new();
        
        for term in terms {
            if text.to_lowercase().contains(&term.term.to_lowercase()) {
                suggestions.push(TermSuggestion {
                    term: term.term.clone(),
                    definition: term.definition.clone(),
                    do_not_translate: term.do_not_translate,
                    confidence: self.calculate_term_confidence(&term.term, text),
                    position: text.to_lowercase().find(&term.term.to_lowercase()),
                });
            }
        }
        
        // Sort by confidence
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        
        Ok(suggestions)
    }
    
    async fn validate_terms(
        &self,
        terms: &[Term],
        project_id: Uuid,
    ) -> Result<TermValidationResult> {
        let existing_terms = self.terminology_repository
            .get_terms_by_project(project_id)
            .await?;
        
        let mut conflicts = Vec::new();
        let mut warnings = Vec::new();
        
        for term in terms {
            // Check for conflicts with existing terms
            if let Some(existing) = existing_terms.iter().find(|t| t.term == term.term) {
                if existing.definition != term.definition {
                    conflicts.push(TermConflict {
                        term: term.term.clone(),
                        existing_definition: existing.definition.clone(),
                        new_definition: term.definition.clone(),
                    });
                }
            }
            
            // Check for potential issues
            if term.term.is_empty() {
                warnings.push(format!("Empty term found"));
            }
            
            if term.term.len() > 100 {
                warnings.push(format!("Term '{}' is very long", term.term));
            }
        }
        
        Ok(TermValidationResult { conflicts, warnings })
    }
    
    fn calculate_term_confidence(&self, term: &str, text: &str) -> f32 {
        // Simple confidence calculation based on exact match and context
        let term_lower = term.to_lowercase();
        let text_lower = text.to_lowercase();
        
        if text_lower.contains(&term_lower) {
            // Higher confidence for exact word boundaries
            let word_boundary_regex = regex::Regex::new(&format!(r"\b{}\b", regex::escape(&term_lower))).unwrap();
            if word_boundary_regex.is_match(&text_lower) {
                0.9
            } else {
                0.7
            }
        } else {
            0.0
        }
    }
}

/// Repository for terminology data access
pub struct TerminologyRepository {
    connection: Arc<Mutex<Connection>>,
    storage_path: PathBuf,
}

impl TerminologyRepository {
    pub fn new(storage_path: PathBuf) -> Result<Self> {
        let db_path = storage_path.join("terminology.duckdb");
        let connection = Connection::open(db_path)?;
        
        let repo = Self {
            connection: Arc::new(Mutex::new(connection)),
            storage_path,
        };
        
        repo.initialize_schema()?;
        Ok(repo)
    }
    
    fn initialize_schema(&self) -> Result<()> {
        let conn = self.connection.lock().unwrap();
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS terms (
                id VARCHAR PRIMARY KEY,
                project_id VARCHAR NOT NULL,
                term VARCHAR NOT NULL,
                definition TEXT,
                do_not_translate BOOLEAN DEFAULT FALSE,
                created_at BIGINT NOT NULL,
                updated_at BIGINT NOT NULL,
                UNIQUE(project_id, term)
            )",
            params![],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_terms_project_id ON terms(project_id)",
            params![],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_terms_term ON terms(term)",
            params![],
        )?;
        
        Ok(())
    }
    
    pub async fn add_term(&self, term: Term, project_id: Uuid) -> Result<()> {
        let conn = self.connection.lock().unwrap();
        
        conn.execute(
            "INSERT OR REPLACE INTO terms 
             (id, project_id, term, definition, do_not_translate, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                term.id.to_string(),
                project_id.to_string(),
                term.term,
                term.definition,
                term.do_not_translate,
                term.created_at.timestamp(),
                term.updated_at.timestamp(),
            ],
        )?;
        
        Ok(())
    }
    
    pub async fn update_term(&self, term: Term, project_id: Uuid) -> Result<()> {
        self.add_term(term, project_id).await // Uses INSERT OR REPLACE
    }
    
    pub async fn get_terms_by_project(&self, project_id: Uuid) -> Result<Vec<Term>> {
        let conn = self.connection.lock().unwrap();
        
        let mut stmt = conn.prepare(
            "SELECT id, term, definition, do_not_translate, created_at, updated_at
             FROM terms WHERE project_id = ?"
        )?;
        
        let rows = stmt.query_map(params![project_id.to_string()], |row| {
            Ok(Term {
                id: row.get::<_, String>(0)?.parse().unwrap(),
                term: row.get(1)?,
                definition: row.get(2)?,
                do_not_translate: row.get(3)?,
                created_at: DateTime::from_timestamp(row.get(4)?, 0).unwrap(),
                updated_at: DateTime::from_timestamp(row.get(5)?, 0).unwrap(),
            })
        })?;
        
        let mut terms = Vec::new();
        for row in rows {
            terms.push(row?);
        }
        
        Ok(terms)
    }
    
    pub async fn search_terms(&self, query: &str, project_id: Uuid) -> Result<Vec<Term>> {
        let conn = self.connection.lock().unwrap();
        
        let mut stmt = conn.prepare(
            "SELECT id, term, definition, do_not_translate, created_at, updated_at
             FROM terms 
             WHERE project_id = ? AND (term LIKE ? OR definition LIKE ?)
             ORDER BY term"
        )?;
        
        let search_pattern = format!("%{}%", query);
        let rows = stmt.query_map(
            params![project_id.to_string(), search_pattern, search_pattern],
            |row| {
                Ok(Term {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    term: row.get(1)?,
                    definition: row.get(2)?,
                    do_not_translate: row.get(3)?,
                    created_at: DateTime::from_timestamp(row.get(4)?, 0).unwrap(),
                    updated_at: DateTime::from_timestamp(row.get(5)?, 0).unwrap(),
                })
            },
        )?;
        
        let mut terms = Vec::new();
        for row in rows {
            terms.push(row?);
        }
        
        Ok(terms)
    }
}

/// CSV processor for importing and exporting terminology
pub struct CsvProcessor;

impl CsvProcessor {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn parse_csv(&self, file_path: &Path) -> Result<Vec<Term>> {
        let content = std::fs::read_to_string(file_path)?;
        let mut reader = csv::Reader::from_reader(content.as_bytes());
        
        let mut terms = Vec::new();
        
        for result in reader.records() {
            let record = result?;
            
            if record.len() >= 2 {
                let term = Term {
                    id: Uuid::new_v4(),
                    term: record.get(0).unwrap_or("").to_string(),
                    definition: record.get(1).map(|s| s.to_string()),
                    do_not_translate: record.get(2)
                        .map(|s| s.to_lowercase() == "true" || s == "1")
                        .unwrap_or(false),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };
                
                if !term.term.is_empty() {
                    terms.push(term);
                }
            }
        }
        
        Ok(terms)
    }
    
    pub async fn export_to_csv(&self, terms: &[Term], file_path: &Path) -> Result<()> {
        let mut writer = csv::Writer::from_path(file_path)?;
        
        // Write header
        writer.write_record(&["term", "definition", "do_not_translate"])?;
        
        // Write terms
        for term in terms {
            writer.write_record(&[
                &term.term,
                term.definition.as_deref().unwrap_or(""),
                &term.do_not_translate.to_string(),
            ])?;
        }
        
        writer.flush()?;
        Ok(())
    }
}

/// Parquet converter for efficient terminology storage
pub struct ParquetConverter {
    storage_path: PathBuf,
}

impl ParquetConverter {
    pub fn new(storage_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_path)?;
        Ok(Self { storage_path })
    }
    
    pub async fn convert_terms_to_parquet(
        &self,
        terms: &[Term],
        project_id: Uuid,
    ) -> Result<PathBuf> {
        let file_path = self.storage_path
            .join(format!("terms_{}.parquet", project_id));
        
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("term", DataType::Utf8, false),
            Field::new("definition", DataType::Utf8, true),
            Field::new("do_not_translate", DataType::Boolean, false),
            Field::new("created_at", DataType::Int64, false),
            Field::new("updated_at", DataType::Int64, false),
        ]));
        
        let file = std::fs::File::create(&file_path)?;
        let props = WriterProperties::builder().build();
        let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;
        
        // Convert terms to Arrow arrays
        let ids: Vec<String> = terms.iter().map(|t| t.id.to_string()).collect();
        let term_names: Vec<String> = terms.iter().map(|t| t.term.clone()).collect();
        let definitions: Vec<Option<String>> = terms.iter()
            .map(|t| t.definition.clone())
            .collect();
        let do_not_translate: Vec<bool> = terms.iter()
            .map(|t| t.do_not_translate)
            .collect();
        let created_at: Vec<i64> = terms.iter()
            .map(|t| t.created_at.timestamp())
            .collect();
        let updated_at: Vec<i64> = terms.iter()
            .map(|t| t.updated_at.timestamp())
            .collect();
        
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(ids)),
                Arc::new(StringArray::from(term_names)),
                Arc::new(StringArray::from(definitions)),
                Arc::new(BooleanArray::from(do_not_translate)),
                Arc::new(Int64Array::from(created_at)),
                Arc::new(Int64Array::from(updated_at)),
            ],
        )?;
        
        writer.write(&batch)?;
        writer.close()?;
        
        Ok(file_path)
    }
    
    pub async fn refresh_parquet_files(
        &self,
        project_id: Uuid,
        terms: &[Term],
    ) -> Result<()> {
        self.convert_terms_to_parquet(terms, project_id).await?;
        Ok(())
    }
}

// Supporting types and structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminologyImportResult {
    pub imported_count: usize,
    pub conflicts: Vec<TermConflict>,
    pub warnings: Vec<String>,
    pub parquet_file: PathBuf,
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermValidationResult {
    pub conflicts: Vec<TermConflict>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermConflict {
    pub term: String,
    pub existing_definition: Option<String>,
    pub new_definition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermSuggestion {
    pub term: String,
    pub definition: Option<String>,
    pub do_not_translate: bool,
    pub confidence: f32,
    pub position: Option<usize>,
}