//! Translation memory service with thread-safe async operations
//! 
//! This service has been migrated from the main TradocFlow crate with critical
//! thread safety improvements:
//! - Proper connection pooling instead of Arc<RwLock<Connection>>
//! - Async-first database operations with Send + Sync traits
//! - Coordinated caching with database operations
//! - Production-ready error handling and logging

use crate::error::{Result, TranslationMemoryError};
use crate::models::{TranslationUnit, Language, ChunkMetadata as Chunk};
use crate::storage::DuckDBManager;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::path::PathBuf;
use itertools::Itertools;

/// Translation match result from similarity search
/// Migrated from the original service with enhanced metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranslationMatch {
    pub id: Uuid,
    pub source_text: String,
    pub target_text: String,
    pub confidence_score: f32,
    pub similarity_score: f32,
    pub context: Option<String>,
    pub language_pair: LanguagePair,
    pub metadata: TranslationMatchMetadata,
}

/// Language pair for translation operations
/// Enhanced to support both Language enum and string codes for compatibility
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct LanguagePair {
    pub source: Language,
    pub target: Language,
}

impl LanguagePair {
    /// Create from language codes (for compatibility with old API)
    pub fn from_codes(source: &str, target: &str) -> Self {
        Self {
            source: Language::from_code(source).unwrap_or(Language::Custom(source.to_string())),
            target: Language::from_code(target).unwrap_or(Language::Custom(target.to_string())),
        }
    }
    
    /// Create from Language enums
    pub fn new(source: Language, target: Language) -> Self {
        Self { source, target }
    }
    
    /// Check if this is a valid language pair (different languages)
    pub fn is_valid(&self) -> bool {
        self.source != self.target
    }
    
    /// Get the reverse language pair
    pub fn reverse(&self) -> Self {
        Self {
            source: self.target.clone(),
            target: self.source.clone(),
        }
    }
}

impl std::fmt::Display for LanguagePair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} → {}", self.source.code(), self.target.code())
    }
}

/// Metadata for translation matches
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct TranslationMatchMetadata {
    pub translator_id: Option<String>,
    pub reviewer_id: Option<String>,
    pub quality_score: Option<f32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Translation suggestion for user interface
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
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

/// Type of chunk linking operation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ChunkLinkType {
    /// Link chunks into a phrase group
    LinkedPhrase,
    /// Unlink previously linked chunks
    Unlinked,
    /// Merge chunks into a single unit
    Merged,
}

/// Thread-safe in-memory cache for frequently accessed translations
/// Using DashMap for lock-free concurrent access
#[derive(Debug, Default)]
struct TranslationCache {
    // Use DashMap for thread-safe concurrent access without explicit locking
    translation_units: DashMap<String, Vec<TranslationMatch>>,
    chunks: DashMap<Uuid, Chunk>,
    language_pairs: DashMap<LanguagePair, DateTime<Utc>>,
    last_updated: Arc<RwLock<Option<DateTime<Utc>>>>,
    // Cache statistics
    hit_count: Arc<std::sync::atomic::AtomicU64>,
    miss_count: Arc<std::sync::atomic::AtomicU64>,
}

/// Thread-safe translation memory service with async operations
/// 
/// THREAD SAFETY IMPROVEMENTS:
/// - Replaced Arc<RwLock<Connection>> with proper connection pooling via DuckDBManager
/// - All database operations are now truly async with Send + Sync
/// - Cache uses lock-free DashMap for concurrent access
/// - Connection pool prevents blocking on database operations
#[derive(Debug)]
pub struct TranslationMemoryService {
    // Database manager with connection pooling (THREAD SAFETY FIX)
    duckdb_manager: Arc<DuckDBManager>,
    // Thread-safe cache with lock-free concurrent access
    cache: Arc<TranslationCache>,
    project_id: Uuid,
    #[allow(dead_code)]
    project_path: PathBuf,
    // Configuration
    max_search_results: usize,
    min_similarity_threshold: f32,
}

impl TranslationMemoryService {
    /// Create a new translation memory service with proper connection pooling
    /// 
    /// THREAD SAFETY: Uses connection pooling instead of shared connection
    pub async fn new(project_id: Uuid, project_path: PathBuf) -> Result<Self> {
        // Create database manager with connection pooling (THREAD SAFETY FIX)
        let db_path = project_path.join("translation_memory.db");
        let duckdb_manager = DuckDBManager::new(&db_path, Some(10)).await?;
        
        let service = Self {
            duckdb_manager,
            cache: Arc::new(TranslationCache::default()),
            project_id,
            project_path,
            max_search_results: 20,
            min_similarity_threshold: 0.3,
        };
        
        service.initialize().await?;
        Ok(service)
    }
    
    /// Search for translation matches (legacy API for lib.rs compatibility)
    /// 
    /// THREAD SAFETY: Uses connection pool, no blocking operations
    pub async fn search(
        &self,
        query: &str,
        source_lang: Language,
        target_lang: Language,
        threshold: f64,
    ) -> Result<Vec<TranslationUnit>> {
        let language_pair = LanguagePair {
            source: source_lang,
            target: target_lang,
        };
        
        let matches = self
            .search_similar_translations(query, language_pair, Some(threshold as f32))
            .await?;
        
        // Convert matches back to TranslationUnits for compatibility
        let mut units = Vec::new();
        for m in matches {
            // Create a translation unit from the match
            let unit = TranslationUnit {
                id: m.id,
                project_id: self.project_id,
                chapter_id: Uuid::new_v4(), // TODO: Extract from context or metadata
                chunk_id: Uuid::new_v4(),   // TODO: Extract from context or metadata
                source_language: m.language_pair.source,
                source_text: m.source_text,
                target_language: m.language_pair.target,
                target_text: m.target_text,
                confidence_score: m.confidence_score,
                context: m.context,
                metadata: crate::models::TranslationMetadata {
                    translator_id: m.metadata.translator_id,
                    reviewer_id: m.metadata.reviewer_id,
                    quality_score: m.metadata.quality_score,
                    notes: Vec::new(),
                    tags: Vec::new(),
                },
                created_at: m.metadata.created_at,
                updated_at: m.metadata.updated_at,
            };
            units.push(unit);
        }
        
        Ok(units)
    }
    
    /// Initialize the service and create necessary tables
    /// 
    /// THREAD SAFETY: Uses connection pooling for schema initialization
    pub async fn initialize(&self) -> Result<()> {
        log::info!("Initializing translation memory service for project: {}", self.project_id);
        
        // Initialize database schema with connection pooling
        self.duckdb_manager.initialize_schema().await
            .map_err(|e| TranslationMemoryError::DatabaseError(
                format!("Failed to initialize database schema: {}", e)
            ))?;
        
        log::info!("Translation memory service initialized successfully");
        Ok(())
    }
    
    /// Search for similar translations with multiple strategies
    /// 
    /// THREAD SAFETY: Uses connection pool and lock-free cache access
    pub async fn search_similar_translations(
        &self,
        source_text: &str,
        language_pair: LanguagePair,
        min_similarity: Option<f32>,
    ) -> Result<Vec<TranslationMatch>> {
        // Validate input
        if source_text.trim().is_empty() {
            return Err(TranslationMemoryError::ValidationError(
                "Source text cannot be empty".to_string()
            ));
        }
        
        // Check cache first (THREAD SAFETY: using DashMap for lock-free access)
        let cache_key = self.create_cache_key(source_text, &language_pair);
        if let Some(cached_matches) = self.cache.translation_units.get(&cache_key) {
            // Update hit count
            self.cache.hit_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Ok(cached_matches.clone());
        }
        
        // Update miss count
        self.cache.miss_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        let mut matches = Vec::new();
        let threshold = min_similarity.unwrap_or(self.min_similarity_threshold);
        
        log::debug!(
            "Searching for translations: '{}' ({} -> {}) with threshold {}",
            source_text, language_pair.source, language_pair.target, threshold
        );
        
        // Strategy 1: Exact phrase matching (highest priority)
        let exact_matches = self.search_exact_matches(source_text, &language_pair).await?;
        matches.extend(exact_matches);
        
        // Strategy 2: Fuzzy matching with edit distance
        if matches.len() < self.max_search_results {
            let fuzzy_matches = self.search_fuzzy_matches(source_text, &language_pair, threshold).await?;
            matches.extend(fuzzy_matches);
        }
        
        // Strategy 3: N-gram similarity
        if matches.len() < self.max_search_results {
            let ngram_matches = self.search_ngram_matches(source_text, &language_pair, threshold).await?;
            matches.extend(ngram_matches);
        }
        
        // Remove duplicates and sort by similarity score (descending)
        matches.sort_by(|a, b| {
            b.similarity_score
                .partial_cmp(&a.similarity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matches = matches
            .into_iter()
            .unique_by(|m| m.id)
            .take(self.max_search_results)
            .collect();
        
        // Cache the results (THREAD SAFETY: using DashMap for lock-free access)
        self.cache.translation_units.insert(cache_key, matches.clone());
        
        // Update last_updated timestamp
        {
            let mut last_updated = self.cache.last_updated.write().await;
            *last_updated = Some(Utc::now());
        }
        
        log::debug!("Found {} translation matches", matches.len());
        Ok(matches)
    }
    
    /// Add a new translation unit to the database
    /// 
    /// THREAD SAFETY: Uses connection pool for database operations
    pub async fn add_translation_unit(&self, mut unit: TranslationUnit) -> Result<()> {
        // Validate the translation unit
        unit.validate().map_err(|e| {
            TranslationMemoryError::ValidationError(format!("Invalid translation unit: {}", e))
        })?;
        
        // Ensure project ID matches
        if unit.project_id != self.project_id {
            unit.project_id = self.project_id;
        }
        
        log::debug!("Adding translation unit: {} ({} -> {})", 
                   unit.id, unit.source_language, unit.target_language);
        
        // Add to database using connection pool (THREAD SAFETY FIX)
        self.duckdb_manager.insert_translation_unit(&unit).await
            .map_err(|e| TranslationMemoryError::DatabaseError(
                format!("Failed to insert translation unit: {}", e)
            ))?;
        
        // Invalidate relevant cache entries (THREAD SAFETY: lock-free operations)
        self.invalidate_translation_cache(&unit.source_text, &LanguagePair {
            source: unit.source_language.clone(),
            target: unit.target_language.clone(),
        }).await;
        
        log::debug!("Successfully added translation unit: {}", unit.id);
        Ok(())
    }
    
    /// Add multiple translation units in a batch operation
    /// 
    /// THREAD SAFETY: Uses connection pool for batch database operations
    pub async fn add_translation_units_batch(&self, mut units: Vec<TranslationUnit>) -> Result<usize> {
        if units.is_empty() {
            return Ok(0);
        }
        
        log::info!("Adding batch of {} translation units", units.len());
        
        // Validate all units first
        for unit in &mut units {
            unit.validate().map_err(|e| {
                TranslationMemoryError::ValidationError(format!("Invalid translation unit: {}", e))
            })?;
            
            // Ensure project ID matches
            if unit.project_id != self.project_id {
                unit.project_id = self.project_id;
            }
        }
        
        // Batch insert to database using connection pool (THREAD SAFETY FIX)
        let inserted_count = self.duckdb_manager.insert_translation_units_batch(&units).await
            .map_err(|e| TranslationMemoryError::DatabaseError(
                format!("Failed to insert translation units batch: {}", e)
            ))?;
        
        // Invalidate cache entries for all affected language pairs
        let language_pairs: Vec<_> = units
            .iter()
            .map(|u| LanguagePair {
                source: u.source_language.clone(),
                target: u.target_language.clone(),
            })
            .unique()
            .collect();
        
        for pair in language_pairs {
            self.invalidate_cache_for_language_pair(&pair).await;
        }
        
        log::info!("Successfully added {} translation units", inserted_count);
        Ok(inserted_count)
    }
    
    /// Update an existing translation unit
    /// 
    /// THREAD SAFETY: Uses connection pool for database operations
    pub async fn update_translation_unit(&self, unit: TranslationUnit) -> Result<()> {
        // Validate the translation unit
        unit.validate().map_err(|e| {
            TranslationMemoryError::ValidationError(format!("Invalid translation unit: {}", e))
        })?;
        
        log::debug!("Updating translation unit: {}", unit.id);
        
        // Update in database using connection pool (THREAD SAFETY FIX)
        self.duckdb_manager.update_translation_unit(&unit).await
            .map_err(|e| TranslationMemoryError::DatabaseError(
                format!("Failed to update translation unit: {}", e)
            ))?;
        
        // Invalidate relevant cache entries
        self.invalidate_translation_cache(&unit.source_text, &LanguagePair {
            source: unit.source_language.clone(),
            target: unit.target_language.clone(),
        }).await;
        
        log::debug!("Successfully updated translation unit: {}", unit.id);
        Ok(())
    }
    
    /// Delete a translation unit by ID
    /// 
    /// THREAD SAFETY: Uses connection pool for database operations
    pub async fn delete_translation_unit(&self, id: Uuid) -> Result<bool> {
        log::debug!("Deleting translation unit: {}", id);
        
        // Delete from database using connection pool (THREAD SAFETY FIX)
        let deleted = self.duckdb_manager.delete_translation_unit(id).await
            .map_err(|e| TranslationMemoryError::DatabaseError(
                format!("Failed to delete translation unit: {}", e)
            ))?;
        
        if deleted {
            // Invalidate all cache entries (since we don't know which ones are affected)
            self.invalidate_all_cache().await;
            log::debug!("Successfully deleted translation unit: {}", id);
        } else {
            log::warn!("Translation unit not found for deletion: {}", id);
        }
        
        Ok(deleted)
    }
    
    /// Get translation suggestions for a given source text
    /// 
    /// THREAD SAFETY: Uses connection pool and lock-free cache access
    pub async fn get_translation_suggestions(
        &self,
        source_text: &str,
        target_language: Language,
        source_language: Option<Language>,
    ) -> Result<Vec<TranslationSuggestion>> {
        let source_lang = source_language.unwrap_or(Language::English);
        let language_pair = LanguagePair::new(source_lang, target_language);
        
        if !language_pair.is_valid() {
            return Err(TranslationMemoryError::ValidationError(
                "Source and target languages must be different".to_string()
            ));
        }
        
        let matches = self.search_similar_translations(source_text, language_pair, Some(0.5)).await?;
        
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
    
    /// Update chunk linking operations
    /// 
    /// THREAD SAFETY: Uses lock-free cache access for chunks
    pub async fn update_chunk_linking(
        &self,
        chunk_ids: Vec<Uuid>,
        link_type: ChunkLinkType,
    ) -> Result<()> {
        if chunk_ids.len() < 2 && link_type != ChunkLinkType::Unlinked {
            return Err(TranslationMemoryError::ValidationError(
                "At least 2 chunks required for linking".to_string()
            ));
        }
        
        log::debug!("Updating chunk linking: {:?} with type {:?}", chunk_ids, link_type);
        
        // TODO: Implement actual chunk linking when ChunkManager is available
        // For now, just invalidate chunk cache entries
        for chunk_id in &chunk_ids {
            self.cache.chunks.remove(chunk_id);
        }
        
        // Update last_updated timestamp
        {
            let mut last_updated = self.cache.last_updated.write().await;
            *last_updated = Some(Utc::now());
        }
        
        log::debug!("Successfully updated chunk linking for {} chunks", chunk_ids.len());
        Ok(())
    }
    
    /// Add multiple chunks to the translation memory
    /// 
    /// THREAD SAFETY: Uses lock-free cache access for chunks
    pub async fn add_chunks_batch(&self, chunks: Vec<Chunk>) -> Result<()> {
        if chunks.is_empty() {
            return Ok(());
        }
        
        log::debug!("Adding batch of {} chunks", chunks.len());
        
        // TODO: Implement actual chunk storage when ChunkManager is available
        // For now, just add to cache
        for chunk in chunks {
            self.cache.chunks.insert(chunk.id, chunk);
        }
        
        // Update last_updated timestamp
        {
            let mut last_updated = self.cache.last_updated.write().await;
            *last_updated = Some(Utc::now());
        }
        
        log::debug!("Successfully added chunk batch");
        Ok(())
    }
    
    /// Get cache statistics for monitoring (legacy compatibility)
    pub async fn get_cache_stats(&self) -> (usize, usize, Option<DateTime<Utc>>) {
        let last_updated = *self.cache.last_updated.read().await;
        (
            self.cache.translation_units.len(),
            self.cache.chunks.len(),
            last_updated,
        )
    }
    
    /// Clear the cache manually
    pub async fn clear_cache(&self) {
        self.invalidate_all_cache().await;
    }
    
    // Private helper methods
    
    async fn search_exact_matches(
        &self,
        source_text: &str,
        language_pair: &LanguagePair,
    ) -> Result<Vec<TranslationMatch>> {
        // Use database manager with connection pooling (THREAD SAFETY FIX)
        self.duckdb_manager
            .search_exact_matches(source_text, language_pair)
            .await
            .map_err(|e| {
                TranslationMemoryError::DatabaseError(format!(
                    "Failed to search exact matches: {}", e
                ))
            })
    }
    
    async fn search_fuzzy_matches(
        &self,
        source_text: &str,
        language_pair: &LanguagePair,
        threshold: f32,
    ) -> Result<Vec<TranslationMatch>> {
        // Use database manager with connection pooling (THREAD SAFETY FIX)
        self.duckdb_manager
            .search_fuzzy_matches(source_text, language_pair, threshold)
            .await
            .map_err(|e| {
                TranslationMemoryError::DatabaseError(format!(
                    "Failed to search fuzzy matches: {}", e
                ))
            })
    }
    
    async fn search_ngram_matches(
        &self,
        source_text: &str,
        language_pair: &LanguagePair,
        threshold: f32,
    ) -> Result<Vec<TranslationMatch>> {
        // Use database manager with connection pooling (THREAD SAFETY FIX)
        self.duckdb_manager
            .search_ngram_matches(source_text, language_pair, threshold)
            .await
            .map_err(|e| {
                TranslationMemoryError::DatabaseError(format!(
                    "Failed to search n-gram matches: {}", e
                ))
            })
    }
    
    /// Get cache statistics for monitoring
    /// 
    /// Returns: (cache_entries, hit_count, miss_count, hit_ratio, last_updated)
    pub async fn get_detailed_cache_stats(&self) -> (usize, u64, u64, f64, Option<DateTime<Utc>>) {
        let entries = self.cache.translation_units.len() + self.cache.chunks.len();
        let hits = self.cache.hit_count.load(std::sync::atomic::Ordering::Relaxed);
        let misses = self.cache.miss_count.load(std::sync::atomic::Ordering::Relaxed);
        let hit_ratio = if hits + misses > 0 {
            hits as f64 / (hits + misses) as f64
        } else {
            0.0
        };
        let last_updated = *self.cache.last_updated.read().await;
        
        (entries, hits, misses, hit_ratio, last_updated)
    }
    
    /// Get database statistics
    pub async fn get_database_stats(&self) -> Result<crate::storage::duckdb_manager::DatabaseStats> {
        self.duckdb_manager.get_database_stats().await
            .map_err(|e| TranslationMemoryError::DatabaseError(
                format!("Failed to get database stats: {}", e)
            ))
    }
    
    /// Optimize database performance
    pub async fn optimize_database(&self) -> Result<()> {
        log::info!("Optimizing database performance");
        self.duckdb_manager.optimize_database().await
            .map_err(|e| TranslationMemoryError::DatabaseError(
                format!("Failed to optimize database: {}", e)
            ))
    }
    
    /// Get connection pool statistics
    pub async fn get_connection_pool_stats(&self) -> (usize, usize) {
        self.duckdb_manager.get_connection_pool_stats().await
    }
    
    fn create_cache_key(&self, text: &str, language_pair: &LanguagePair) -> String {
        format!("{}:{}:{}", text, language_pair.source.code(), language_pair.target.code())
    }
    
    /// Invalidate specific translation cache entries (THREAD SAFETY: lock-free)
    async fn invalidate_translation_cache(&self, source_text: &str, language_pair: &LanguagePair) {
        let cache_key = self.create_cache_key(source_text, language_pair);
        self.cache.translation_units.remove(&cache_key);
        
        // Update last_updated timestamp
        {
            let mut last_updated = self.cache.last_updated.write().await;
            *last_updated = Some(Utc::now());
        }
    }
    
    /// Invalidate all cache entries for a language pair
    async fn invalidate_cache_for_language_pair(&self, language_pair: &LanguagePair) {
        // Remove all entries that match this language pair
        self.cache.translation_units.retain(|key, _| {
            !key.contains(&format!("{}:{}", language_pair.source, language_pair.target))
        });
        
        self.cache.language_pairs.remove(language_pair);
        
        // Update last_updated timestamp
        {
            let mut last_updated = self.cache.last_updated.write().await;
            *last_updated = Some(Utc::now());
        }
    }
    
    /// Invalidate all cache entries
    async fn invalidate_all_cache(&self) {
        self.cache.translation_units.clear();
        self.cache.chunks.clear();
        self.cache.language_pairs.clear();
        
        // Update last_updated timestamp
        {
            let mut last_updated = self.cache.last_updated.write().await;
            *last_updated = Some(Utc::now());
        }
    }
}

/// Calculate similarity between two strings using Jaccard similarity
pub fn calculate_similarity(text1: &str, text2: &str) -> f32 {
    let words1: std::collections::HashSet<&str> = text1.split_whitespace().collect();
    let words2: std::collections::HashSet<&str> = text2.split_whitespace().collect();
    
    let intersection = words1.intersection(&words2).count();
    let union = words1.union(&words2).count();
    
    if union == 0 {
        return if text1 == text2 { 1.0 } else { 0.0 };
    }
    
    intersection as f32 / union as f32
}

/// Calculate n-gram similarity between two strings
pub fn calculate_ngram_similarity(text1: &str, text2: &str, n: usize) -> f32 {
    let ngrams1 = extract_ngrams(text1, n);
    let ngrams2 = extract_ngrams(text2, n);
    
    let intersection = ngrams1.intersection(&ngrams2).count();
    let union = ngrams1.union(&ngrams2).count();
    
    if union == 0 {
        return if text1 == text2 { 1.0 } else { 0.0 };
    }
    
    intersection as f32 / union as f32
}

fn extract_ngrams(text: &str, n: usize) -> std::collections::HashSet<String> {
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