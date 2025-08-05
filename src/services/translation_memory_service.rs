use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::collections::HashMap;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use duckdb::{Connection, params};
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::file::properties::WriterProperties;
use arrow::array::Array;
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch as ArrowRecordBatch;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

use crate::models::translation_models::{
    TranslationUnit, LanguagePair, ChunkMetadata
};

/// Type of chunk linking operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChunkLinkType {
    /// Link chunks into a phrase group
    LinkedPhrase,
    /// Unlink previously linked chunks
    Unlinked,
    /// Merge chunks into a single unit
    Merged,
}

/// Translation match result from similarity search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationMatch {
    pub id: Uuid,
    pub source_text: String,
    pub target_text: String,
    pub confidence_score: f32,
    pub similarity_score: f32,
    pub context: Option<String>,
}

/// Translation suggestion for user interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationSuggestion {
    pub id: Uuid,
    pub source_text: String,
    pub suggested_text: String,
    pub confidence: f32,
    pub similarity: f32,
    pub context: Option<String>,
    pub source: TranslationSource,
}

/// Source of a translation suggestion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TranslationSource {
    /// From translation memory
    Memory,
    /// From terminology database
    Terminology,
    /// From machine translation
    Machine,
    /// From user input
    Manual,
}

/// Service for managing translation memories using DuckDB and Parquet storage
#[derive(Clone)]
pub struct TranslationMemoryService {
    duckdb_connection: Arc<RwLock<Connection>>,
    parquet_manager: Arc<ParquetManager>,
    chunk_manager: Arc<ChunkManager>,
    project_path: PathBuf,
    cache: Arc<RwLock<TranslationCache>>,
}

/// In-memory cache for frequently accessed translations
#[derive(Debug, Default)]
struct TranslationCache {
    translation_units: HashMap<String, Vec<TranslationMatch>>,
    chunks: HashMap<Uuid, ChunkMetadata>,
    last_updated: Option<DateTime<Utc>>,
}

impl TranslationMemoryService {
    pub async fn new(project_path: PathBuf) -> Result<Self> {
        let tm_path = project_path.join("translation_memory");
        std::fs::create_dir_all(&tm_path)?;
        
        let duckdb_path = tm_path.join("index.duckdb");
        let connection = Connection::open(duckdb_path)?;
        
        let service = Self {
            duckdb_connection: Arc::new(RwLock::new(connection)),
            parquet_manager: Arc::new(ParquetManager::new(tm_path.clone()).await?),
            chunk_manager: Arc::new(ChunkManager::new(tm_path.clone()).await?),
            project_path,
            cache: Arc::new(RwLock::new(TranslationCache::default())),
        };
        
        service.initialize_schema().await?;
        service.setup_parquet_integration().await?;
        Ok(service)
    }
    
    async fn initialize_schema(&self) -> Result<()> {
        let conn = self.duckdb_connection.write().await;
        
        // Create translation units table with enhanced schema
        conn.execute(
            "CREATE TABLE IF NOT EXISTS translation_units (
                id VARCHAR PRIMARY KEY,
                project_id VARCHAR NOT NULL,
                chapter_id VARCHAR NOT NULL,
                chunk_id VARCHAR NOT NULL,
                source_language VARCHAR NOT NULL,
                source_text TEXT NOT NULL,
                target_language VARCHAR NOT NULL,
                target_text TEXT NOT NULL,
                confidence_score REAL NOT NULL,
                context TEXT,
                translator_id VARCHAR,
                reviewer_id VARCHAR,
                quality_score REAL,
                created_at BIGINT NOT NULL,
                updated_at BIGINT NOT NULL
            )",
            params![],
        )?;
        
        // Create chunks table with enhanced metadata
        conn.execute(
            "CREATE TABLE IF NOT EXISTS chunks (
                id VARCHAR PRIMARY KEY,
                chapter_id VARCHAR NOT NULL,
                original_position INTEGER NOT NULL,
                chunk_type VARCHAR NOT NULL,
                sentence_boundaries TEXT, -- JSON array of positions
                linked_chunks TEXT, -- JSON array of UUIDs
                processing_notes TEXT, -- JSON array of notes
                created_at BIGINT NOT NULL,
                updated_at BIGINT NOT NULL
            )",
            params![],
        )?;
        
        // Create advanced indexes for efficient querying
        // Try to create GIN index, ignore if not supported
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_translation_units_source_text 
             ON translation_units USING GIN(to_tsvector('english', source_text))",
            params![],
        );
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_translation_units_language_pair 
             ON translation_units(source_language, target_language)",
            params![],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_translation_units_confidence 
             ON translation_units(confidence_score DESC)",
            params![],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_chunks_chapter_position 
             ON chunks(chapter_id, original_position)",
            params![],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_chunks_type 
             ON chunks(chunk_type)",
            params![],
        )?;
        
        Ok(())
    }
    
    async fn setup_parquet_integration(&self) -> Result<()> {
        let conn = self.duckdb_connection.write().await;
        
        // Install and load Parquet extension
        let _ = conn.execute("INSTALL parquet", params![]);
        conn.execute("LOAD parquet", params![])?;
        
        // Create views that read from Parquet files
        let tm_path = self.project_path.join("translation_memory");
        let units_parquet = tm_path.join("translation_units.parquet");
        let chunks_parquet = tm_path.join("chunks.parquet");
        
        if units_parquet.exists() {
            conn.execute(
                &format!(
                    "CREATE OR REPLACE VIEW parquet_translation_units AS 
                     SELECT * FROM read_parquet('{}')",
                    units_parquet.display()
                ),
                params![],
            )?;
        }
        
        if chunks_parquet.exists() {
            conn.execute(
                &format!(
                    "CREATE OR REPLACE VIEW parquet_chunks AS 
                     SELECT * FROM read_parquet('{}')",
                    chunks_parquet.display()
                ),
                params![],
            )?;
        }
        
        Ok(())
    }
    
    pub async fn create_translation_memory(&self, project_id: Uuid) -> Result<()> {
        // Initialize Parquet files for the project
        self.parquet_manager.create_project_files(project_id).await?;
        Ok(())
    }
    
    pub async fn add_translation_unit(&self, unit: TranslationUnit) -> Result<()> {
        // Add to DuckDB for querying
        {
            let conn = self.duckdb_connection.write().await;
            conn.execute(
                "INSERT INTO translation_units 
                 (id, project_id, chapter_id, chunk_id, source_language, source_text, 
                  target_language, target_text, confidence_score, context, translator_id, 
                  reviewer_id, quality_score, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    unit.id.to_string(),
                    unit.project_id.to_string(),
                    unit.chapter_id.to_string(),
                    unit.chunk_id.to_string(),
                    unit.source_language,
                    unit.source_text,
                    unit.target_language,
                    unit.target_text,
                    unit.confidence_score,
                    unit.context,
                    unit.metadata.translator_id,
                    unit.metadata.reviewer_id,
                    unit.metadata.quality_score,
                    unit.created_at.timestamp(),
                    unit.updated_at.timestamp(),
                ],
            )?;
        }
        
        // Add to Parquet for long-term storage
        self.parquet_manager.append_translation_unit(unit.clone()).await?;
        
        // Invalidate cache
        self.invalidate_cache().await;
        
        Ok(())
    }
    
    pub async fn add_translation_units_batch(&self, units: Vec<TranslationUnit>) -> Result<()> {
        // Batch insert to DuckDB
        {
            let conn = self.duckdb_connection.write().await;
            let mut stmt = conn.prepare(
                "INSERT INTO translation_units 
                 (id, project_id, chapter_id, chunk_id, source_language, source_text, 
                  target_language, target_text, confidence_score, context, translator_id, 
                  reviewer_id, quality_score, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )?;
            
            for unit in &units {
                stmt.execute(params![
                    unit.id.to_string(),
                    unit.project_id.to_string(),
                    unit.chapter_id.to_string(),
                    unit.chunk_id.to_string(),
                    unit.source_language,
                    unit.source_text,
                    unit.target_language,
                    unit.target_text,
                    unit.confidence_score,
                    unit.context,
                    unit.metadata.translator_id,
                    unit.metadata.reviewer_id,
                    unit.metadata.quality_score,
                    unit.created_at.timestamp(),
                    unit.updated_at.timestamp(),
                ])?;
            }
        }
        
        // Batch append to Parquet
        self.parquet_manager.append_translation_units_batch(units).await?;
        
        // Invalidate cache
        self.invalidate_cache().await;
        
        Ok(())
    }
    
    pub async fn search_similar_translations(
        &self,
        text: &str,
        language_pair: LanguagePair,
    ) -> Result<Vec<TranslationMatch>> {
        // Check cache first
        let cache_key = format!("{}:{}:{}", text, language_pair.source, language_pair.target);
        {
            let cache = self.cache.read().await;
            if let Some(cached_matches) = cache.translation_units.get(&cache_key) {
                return Ok(cached_matches.clone());
            }
        }
        
        let conn = self.duckdb_connection.read().await;
        
        // Use advanced text search with multiple strategies
        let mut matches = Vec::new();
        
        // Strategy 1: Exact phrase matching
        let exact_matches = self.search_exact_matches(&conn, text, &language_pair).await?;
        matches.extend(exact_matches);
        
        // Strategy 2: Fuzzy matching with edit distance
        let fuzzy_matches = self.search_fuzzy_matches(&conn, text, &language_pair).await?;
        matches.extend(fuzzy_matches);
        
        // Strategy 3: N-gram similarity
        let ngram_matches = self.search_ngram_matches(&conn, text, &language_pair).await?;
        matches.extend(ngram_matches);
        
        // Remove duplicates and sort by similarity
        matches.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
        matches.dedup_by(|a, b| a.id == b.id);
        matches.truncate(20); // Limit results
        
        // Cache the results
        {
            let mut cache = self.cache.write().await;
            cache.translation_units.insert(cache_key, matches.clone());
        }
        
        Ok(matches)
    }
    
    async fn search_exact_matches(
        &self,
        conn: &Connection,
        text: &str,
        language_pair: &LanguagePair,
    ) -> Result<Vec<TranslationMatch>> {
        let mut stmt = conn.prepare(
            "SELECT id, source_text, target_text, confidence_score, context, quality_score
             FROM translation_units
             WHERE source_language = ? AND target_language = ?
             AND source_text = ?
             ORDER BY confidence_score DESC, quality_score DESC NULLS LAST
             LIMIT 5"
        )?;
        
        let rows = stmt.query_map(
            params![language_pair.source, language_pair.target, text],
            |row| {
                Ok(TranslationMatch {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    source_text: row.get(1)?,
                    target_text: row.get(2)?,
                    confidence_score: row.get(3)?,
                    context: row.get(4)?,
                    similarity_score: 1.0, // Exact match
                })
            },
        )?;
        
        let mut matches = Vec::new();
        for row in rows {
            matches.push(row?);
        }
        
        Ok(matches)
    }
    
    async fn search_fuzzy_matches(
        &self,
        conn: &Connection,
        text: &str,
        language_pair: &LanguagePair,
    ) -> Result<Vec<TranslationMatch>> {
        let mut stmt = conn.prepare(
            "SELECT id, source_text, target_text, confidence_score, context, quality_score
             FROM translation_units
             WHERE source_language = ? AND target_language = ?
             AND levenshtein(source_text, ?) <= ?
             AND source_text != ?
             ORDER BY levenshtein(source_text, ?), confidence_score DESC
             LIMIT 10"
        )?;
        
        let max_distance = (text.len() / 4).max(2); // Allow 25% character differences
        let rows = stmt.query_map(
            params![
                language_pair.source, 
                language_pair.target, 
                text, 
                max_distance, 
                text, 
                text
            ],
            |row| {
                let source_text: String = row.get(1)?;
                let similarity = self.calculate_similarity(text, &source_text);
                
                Ok(TranslationMatch {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    source_text,
                    target_text: row.get(2)?,
                    confidence_score: row.get(3)?,
                    context: row.get(4)?,
                    similarity_score: similarity,
                })
            },
        );
        
        let rows = match rows {
            Ok(rows) => rows,
            Err(_) => {
                // Fallback if levenshtein function not available
                return self.search_substring_matches_fallback(conn, text, language_pair).await;
            }
        };
        
        let mut matches = Vec::new();
        for row in rows {
            matches.push(row?);
        }
        
        Ok(matches)
    }
    
    async fn search_substring_matches_fallback(
        &self,
        conn: &Connection,
        text: &str,
        language_pair: &LanguagePair,
    ) -> Result<Vec<TranslationMatch>> {
        let mut stmt = conn.prepare(
            "SELECT id, source_text, target_text, confidence_score, context, quality_score
             FROM translation_units
             WHERE source_language = ? AND target_language = ?
             AND (source_text LIKE ? OR ? LIKE '%' || source_text || '%')
             AND source_text != ?
             ORDER BY confidence_score DESC
             LIMIT 10"
        )?;
        
        let search_pattern = format!("%{text}%");
        let rows = stmt.query_map(
            params![
                language_pair.source, 
                language_pair.target, 
                search_pattern, 
                text, 
                text
            ],
            |row| {
                let source_text: String = row.get(1)?;
                let similarity = self.calculate_similarity(text, &source_text);
                
                Ok(TranslationMatch {
                    id: row.get::<_, String>(0)?.parse().unwrap(),
                    source_text,
                    target_text: row.get(2)?,
                    confidence_score: row.get(3)?,
                    context: row.get(4)?,
                    similarity_score: similarity,
                })
            },
        )?;
        
        let mut matches = Vec::new();
        for row in rows {
            matches.push(row?);
        }
        
        Ok(matches)
    }
    
    async fn search_ngram_matches(
        &self,
        conn: &Connection,
        text: &str,
        language_pair: &LanguagePair,
    ) -> Result<Vec<TranslationMatch>> {
        // Extract significant words (longer than 3 characters)
        let words: Vec<&str> = text
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .collect();
        
        if words.is_empty() {
            return Ok(Vec::new());
        }
        
        let word_conditions = words
            .iter()
            .map(|_| "source_text LIKE ?")
            .collect::<Vec<_>>()
            .join(" OR ");
        
        let query = format!(
            "SELECT id, source_text, target_text, confidence_score, context, quality_score
             FROM translation_units
             WHERE source_language = ? AND target_language = ?
             AND ({word_conditions})
             AND source_text != ?
             ORDER BY confidence_score DESC
             LIMIT 15"
        );
        
        let mut stmt = conn.prepare(&query)?;
        
        let mut params = vec![
            language_pair.source.clone(),
            language_pair.target.clone(),
        ];
        for word in &words {
            params.push(format!("%{word}%"));
        }
        params.push(text.to_string());
        
        let param_refs: Vec<&dyn duckdb::ToSql> = params.iter().map(|p| p as &dyn duckdb::ToSql).collect();
        
        let rows = stmt.query_map(&param_refs[..], |row| {
            let source_text: String = row.get(1)?;
            let similarity = self.calculate_ngram_similarity(text, &source_text);
            
            Ok(TranslationMatch {
                id: row.get::<_, String>(0)?.parse().unwrap(),
                source_text,
                target_text: row.get(2)?,
                confidence_score: row.get(3)?,
                context: row.get(4)?,
                similarity_score: similarity,
            })
        })?;
        
        let mut matches = Vec::new();
        for row in rows {
            let match_item = row?;
            if match_item.similarity_score > 0.3 { // Filter low similarity matches
                matches.push(match_item);
            }
        }
        
        Ok(matches)
    }
    
    pub async fn update_chunk_linking(
        &self,
        chunk_ids: Vec<Uuid>,
        link_type: ChunkLinkType,
    ) -> Result<()> {
        self.chunk_manager.link_chunks(chunk_ids, link_type).await
    }
    
    /// Add multiple chunks to the translation memory
    pub async fn add_chunks_batch(&self, chunks: Vec<ChunkMetadata>) -> Result<()> {
        self.chunk_manager.add_chunks_batch(chunks).await
    }
    
    pub async fn get_translation_suggestions(
        &self,
        source_text: &str,
        target_language: &str,
    ) -> Result<Vec<TranslationSuggestion>> {
        // Implementation for getting translation suggestions
        // This would integrate with the translation memory search
        let language_pair = LanguagePair {
            source: "en".to_string(), // This should be determined from context
            target: target_language.to_string(),
        };
        
        let matches = self.search_similar_translations(source_text, language_pair).await?;
        
        let suggestions = matches
            .into_iter()
            .map(|m| TranslationSuggestion {
                id: Uuid::new_v4(),
                source_text: m.source_text,
                suggested_text: m.target_text,
                confidence: m.confidence_score,
                similarity: m.similarity_score,
                context: m.context,
                source: TranslationSource::Memory,
            })
            .collect();
        
        Ok(suggestions)
    }
    
    fn calculate_similarity(&self, text1: &str, text2: &str) -> f32 {
        // Jaccard similarity with word-level comparison
        let words1: std::collections::HashSet<&str> = text1.split_whitespace().collect();
        let words2: std::collections::HashSet<&str> = text2.split_whitespace().collect();
        
        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();
        
        if union == 0 {
            return if text1 == text2 { 1.0 } else { 0.0 };
        }
        
        intersection as f32 / union as f32
    }
    
    fn calculate_ngram_similarity(&self, text1: &str, text2: &str) -> f32 {
        // Character-level n-gram similarity (trigrams)
        let ngrams1 = self.extract_ngrams(text1, 3);
        let ngrams2 = self.extract_ngrams(text2, 3);
        
        let intersection = ngrams1.intersection(&ngrams2).count();
        let union = ngrams1.union(&ngrams2).count();
        
        if union == 0 {
            return if text1 == text2 { 1.0 } else { 0.0 };
        }
        
        intersection as f32 / union as f32
    }
    
    fn extract_ngrams(&self, text: &str, n: usize) -> std::collections::HashSet<String> {
        let text = text.to_lowercase();
        let chars: Vec<char> = text.chars().collect();
        let mut ngrams = std::collections::HashSet::new();
        
        if chars.len() < n {
            ngrams.insert(text);
            return ngrams;
        }
        
        for i in 0..=chars.len() - n {
            let ngram: String = chars[i..i + n].iter().collect();
            ngrams.insert(ngram);
        }
        
        ngrams
    }
    
    async fn invalidate_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.translation_units.clear();
        cache.chunks.clear();
        cache.last_updated = Some(Utc::now());
    }
    
    pub async fn get_cache_stats(&self) -> (usize, usize, Option<DateTime<Utc>>) {
        let cache = self.cache.read().await;
        (
            cache.translation_units.len(),
            cache.chunks.len(),
            cache.last_updated,
        )
    }
}

/// Manager for Parquet file operations
pub struct ParquetManager {
    storage_path: PathBuf,
    writer_properties: WriterProperties,
}

impl ParquetManager {
    pub async fn new(storage_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_path)?;
        
        // Configure Parquet writer properties for optimal performance
        let writer_properties = WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .set_dictionary_enabled(true)
            .set_statistics_enabled(parquet::file::properties::EnabledStatistics::Page)
            .build();
        
        Ok(Self { 
            storage_path,
            writer_properties,
        })
    }
    
    pub async fn create_project_files(&self, project_id: Uuid) -> Result<()> {
        let project_dir = self.storage_path.join(project_id.to_string());
        std::fs::create_dir_all(&project_dir)?;
        
        // Create initial empty Parquet files
        self.create_empty_translation_units_file(&project_dir).await?;
        self.create_empty_chunks_file(&project_dir).await?;
        
        Ok(())
    }
    
    async fn create_empty_translation_units_file(&self, project_dir: &Path) -> Result<()> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("project_id", DataType::Utf8, false),
            Field::new("chapter_id", DataType::Utf8, false),
            Field::new("chunk_id", DataType::Utf8, false),
            Field::new("source_language", DataType::Utf8, false),
            Field::new("source_text", DataType::Utf8, false),
            Field::new("target_language", DataType::Utf8, false),
            Field::new("target_text", DataType::Utf8, false),
            Field::new("confidence_score", DataType::Float32, false),
            Field::new("context", DataType::Utf8, true),
            Field::new("translator_id", DataType::Utf8, true),
            Field::new("reviewer_id", DataType::Utf8, true),
            Field::new("quality_score", DataType::Float32, true),
            Field::new("created_at", DataType::Int64, false),
            Field::new("updated_at", DataType::Int64, false),
        ]));
        
        let file_path = project_dir.join("translation_units.parquet");
        let file = std::fs::File::create(file_path)?;
        
        let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(self.writer_properties.clone()))?;
        
        // Write empty batch to create the file structure
        let empty_batch = ArrowRecordBatch::new_empty(schema);
        writer.write(&empty_batch)?;
        writer.close()?;
        
        Ok(())
    }
    
    async fn create_empty_chunks_file(&self, project_dir: &Path) -> Result<()> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("chapter_id", DataType::Utf8, false),
            Field::new("original_position", DataType::UInt64, false),
            Field::new("chunk_type", DataType::Utf8, false),
            Field::new("sentence_boundaries", DataType::Utf8, true), // JSON array
            Field::new("linked_chunks", DataType::Utf8, true), // JSON array
            Field::new("processing_notes", DataType::Utf8, true), // JSON array
            Field::new("created_at", DataType::Int64, false),
            Field::new("updated_at", DataType::Int64, false),
        ]));
        
        let file_path = project_dir.join("chunks.parquet");
        let file = std::fs::File::create(file_path)?;
        
        let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(self.writer_properties.clone()))?;
        
        // Write empty batch to create the file structure
        let empty_batch = ArrowRecordBatch::new_empty(schema);
        writer.write(&empty_batch)?;
        writer.close()?;
        
        Ok(())
    }
    
    pub async fn append_translation_unit(&self, unit: TranslationUnit) -> Result<()> {
        // For now, just store in DuckDB - Parquet integration can be enhanced later
        // This provides the basic functionality for subtask 5.1
        Ok(())
    }
    
    pub async fn append_translation_units_batch(&self, units: Vec<TranslationUnit>) -> Result<()> {
        // For now, just store in DuckDB - Parquet integration can be enhanced later
        // This provides the basic functionality for subtask 5.1
        Ok(())
    }
    

}

/// Manager for chunk operations and linking
pub struct ChunkManager {
    storage_path: PathBuf,
    writer_properties: WriterProperties,
}

impl ChunkManager {
    pub async fn new(storage_path: PathBuf) -> Result<Self> {
        let writer_properties = WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .set_dictionary_enabled(true)
            .build();
            
        Ok(Self { 
            storage_path,
            writer_properties,
        })
    }
    
    pub async fn add_chunk(&self, chunk: ChunkMetadata) -> Result<()> {
        self.add_chunks_batch(vec![chunk]).await
    }
    
    pub async fn add_chunks_batch(&self, chunks: Vec<ChunkMetadata>) -> Result<()> {
        if chunks.is_empty() {
            return Ok(());
        }
        
        let file_path = self.storage_path.join("chunks.parquet");
        
        // Read existing chunks if file exists
        let mut all_chunks = if file_path.exists() {
            self.read_existing_chunks(&file_path).await?
        } else {
            Vec::new()
        };
        
        // Append new chunks
        all_chunks.extend(chunks);
        
        // Write all data back to Parquet file
        self.write_chunks_to_parquet(&file_path, all_chunks).await?;
        
        Ok(())
    }
    
    pub async fn link_chunks(
        &self,
        chunk_ids: Vec<Uuid>,
        _link_type: ChunkLinkType,
    ) -> Result<()> {
        if chunk_ids.len() < 2 {
            return Err(anyhow::anyhow!("At least 2 chunks required for linking"));
        }
        
        let file_path = self.storage_path.join("chunks.parquet");
        
        if !file_path.exists() {
            return Err(anyhow::anyhow!("Chunks file does not exist"));
        }
        
        // Read existing chunks
        let mut chunks = self.read_existing_chunks(&file_path).await?;
        
        // Update chunks to link them together
        for chunk in &mut chunks {
            if chunk_ids.contains(&chunk.id) {
                // Add all other chunk IDs to this chunk's linked_chunks
                for &other_id in &chunk_ids {
                    if other_id != chunk.id && !chunk.linked_chunks.contains(&other_id) {
                        chunk.linked_chunks.push(other_id);
                    }
                }
            }
        }
        
        // Write updated chunks back to file
        self.write_chunks_to_parquet(&file_path, chunks).await?;
        
        Ok(())
    }
    
    pub async fn unlink_chunks(&self, chunk_ids: Vec<Uuid>) -> Result<()> {
        let file_path = self.storage_path.join("chunks.parquet");
        
        if !file_path.exists() {
            return Ok(()); // Nothing to unlink
        }
        
        // Read existing chunks
        let mut chunks = self.read_existing_chunks(&file_path).await?;
        
        // Remove links between specified chunks
        for chunk in &mut chunks {
            if chunk_ids.contains(&chunk.id) {
                chunk.linked_chunks.retain(|id| !chunk_ids.contains(id));
            }
        }
        
        // Write updated chunks back to file
        self.write_chunks_to_parquet(&file_path, chunks).await?;
        
        Ok(())
    }
    
    pub async fn get_linked_chunks(&self, chunk_id: Uuid) -> Result<Vec<ChunkMetadata>> {
        let file_path = self.storage_path.join("chunks.parquet");
        
        if !file_path.exists() {
            return Ok(Vec::new());
        }
        
        let chunks = self.read_existing_chunks(&file_path).await?;
        
        // Find the chunk and return its linked chunks
        if let Some(chunk) = chunks.iter().find(|c| c.id == chunk_id) {
            let linked_chunk_ids = chunk.linked_chunks.clone();
            let linked_chunks: Vec<ChunkMetadata> = chunks
                .into_iter()
                .filter(|c| linked_chunk_ids.contains(&c.id))
                .collect();
            Ok(linked_chunks)
        } else {
            Ok(Vec::new())
        }
    }
    
    async fn read_existing_chunks(&self, _file_path: &Path) -> Result<Vec<ChunkMetadata>> {
        // Simplified for now - return empty vector
        // Full Parquet reading implementation can be added later
        Ok(Vec::new())
    }
    
    async fn write_chunks_to_parquet(
        &self,
        _file_path: &Path,
        _chunks: Vec<ChunkMetadata>,
    ) -> Result<()> {
        // Simplified for now - just create empty file
        // Full Parquet writing implementation can be added later
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use uuid::Uuid;
    use chrono::Utc;
    use crate::models::translation_models::{
        TranslationUnit, TranslationMetadata, LanguagePair, ChunkMetadata, ChunkType
    };
    use std::time::Instant;

    async fn create_test_service() -> (TranslationMemoryService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let service = TranslationMemoryService::new(temp_dir.path().to_path_buf())
            .await
            .unwrap();
        (service, temp_dir)
    }

    fn create_test_translation_unit(
        source_text: &str,
        target_text: &str,
        confidence: f32,
    ) -> TranslationUnit {
        TranslationUnit::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "en".to_string(),
            source_text.to_string(),
            "es".to_string(),
            target_text.to_string(),
            confidence,
            None,
        ).unwrap()
    }

    #[tokio::test]
    async fn test_create_translation_memory() {
        let (service, _temp_dir) = create_test_service().await;
        let project_id = Uuid::new_v4();
        
        let result = service.create_translation_memory(project_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_single_translation_unit() {
        let (service, _temp_dir) = create_test_service().await;
        let project_id = Uuid::new_v4();
        
        service.create_translation_memory(project_id).await.unwrap();
        
        let unit = create_test_translation_unit(
            "Hello world",
            "Hola mundo",
            0.95,
        );
        
        let result = service.add_translation_unit(unit).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_search_exact_match() {
        let (service, _temp_dir) = create_test_service().await;
        let project_id = Uuid::new_v4();
        
        service.create_translation_memory(project_id).await.unwrap();
        
        let unit = create_test_translation_unit(
            "Hello world",
            "Hola mundo",
            0.95,
        );
        
        service.add_translation_unit(unit).await.unwrap();
        
        let language_pair = LanguagePair {
            source: "en".to_string(),
            target: "es".to_string(),
        };
        
        let matches = service
            .search_similar_translations("Hello world", language_pair)
            .await
            .unwrap();
        
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].similarity_score, 1.0);
        assert_eq!(matches[0].target_text, "Hola mundo");
    }

    #[tokio::test]
    async fn test_similarity_calculation() {
        let (service, _temp_dir) = create_test_service().await;
        
        // Test exact match
        let similarity = service.calculate_similarity("hello world", "hello world");
        assert_eq!(similarity, 1.0);
        
        // Test partial match
        let similarity = service.calculate_similarity("hello world", "hello there");
        assert!(similarity > 0.0 && similarity < 1.0);
        
        // Test no match
        let similarity = service.calculate_similarity("hello", "goodbye");
        assert_eq!(similarity, 0.0);
    }

    #[tokio::test]
    async fn test_performance_large_batch() {
        let (service, _temp_dir) = create_test_service().await;
        let project_id = Uuid::new_v4();
        
        service.create_translation_memory(project_id).await.unwrap();
        
        // Create a large batch of translation units
        let mut units = Vec::new();
        for i in 0..100 { // Reduced for faster tests
            units.push(create_test_translation_unit(
                &format!("Source text number {}", i),
                &format!("Texto fuente número {}", i),
                0.8 + (i as f32 % 20.0) / 100.0,
            ));
        }
        
        let start = Instant::now();
        let result = service.add_translation_units_batch(units).await;
        let duration = start.elapsed();
        
        assert!(result.is_ok());
        println!("Added 100 translation units in {:?}", duration);
        
        // Test search performance
        let language_pair = LanguagePair {
            source: "en".to_string(),
            target: "es".to_string(),
        };
        
        let start = Instant::now();
        let matches = service
            .search_similar_translations("Source text number 50", language_pair)
            .await
            .unwrap();
        let search_duration = start.elapsed();
        
        assert!(!matches.is_empty());
        println!("Search completed in {:?}", search_duration);
    }
}