use std::collections::{HashMap, VecDeque, BTreeMap};
use std::sync::{Arc, Mutex, RwLock, mpsc};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::ops::Range;
use regex::Regex;
use uuid::Uuid;

use super::markdown_text_processor::{MarkdownTextProcessor, DocumentStats};
use super::markdown_processor::{MarkdownProcessor, MarkdownAST, ValidationResult};

/// Performance optimization engine for markdown editor
/// Provides lazy loading, efficient search, caching, and background processing
pub struct MarkdownPerformanceOptimizer {
    /// Document cache for fast access
    document_cache: Arc<RwLock<DocumentCache>>,
    /// Search index for fast text search
    search_index: Arc<RwLock<SearchIndex>>,
    /// Background task manager
    task_manager: BackgroundTaskManager,
    /// Performance metrics
    metrics: Arc<Mutex<PerformanceMetrics>>,
    /// Configuration
    config: OptimizationConfig,
}

/// Document cache for storing parsed content and metadata
#[derive(Debug, Clone)]
struct DocumentCache {
    /// Cached document chunks
    chunks: HashMap<ChunkId, DocumentChunk>,
    /// Cache metadata
    metadata: CacheMetadata,
    /// LRU tracking for eviction
    lru_tracker: VecDeque<ChunkId>,
    /// Total memory usage in bytes
    memory_usage: usize,
}

/// A document chunk for lazy loading
#[derive(Debug, Clone)]
struct DocumentChunk {
    /// Chunk identifier
    id: ChunkId,
    /// Content range in the document
    range: Range<usize>,
    /// Cached content
    content: String,
    /// Parsed AST (if available)
    ast: Option<MarkdownAST>,
    /// Validation result (if available)
    validation: Option<ValidationResult>,
    /// Last access time
    last_accessed: u64,
    /// Access count
    access_count: usize,
    /// Memory size in bytes
    memory_size: usize,
}

/// Chunk identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ChunkId(u64);

/// Cache metadata
#[derive(Debug, Clone)]
struct CacheMetadata {
    /// Total document size
    total_size: usize,
    /// Number of chunks
    chunk_count: usize,
    /// Last update timestamp
    last_updated: u64,
    /// Cache hit ratio
    hit_ratio: f32,
    /// Total hits
    total_hits: u64,
    /// Total misses
    total_misses: u64,
}

/// Search index for fast text operations
#[derive(Debug)]
struct SearchIndex {
    /// Word-based index for fast search
    word_index: HashMap<String, Vec<TextMatch>>,
    /// N-gram index for fuzzy search
    ngram_index: HashMap<String, Vec<TextMatch>>,
    /// Line-based index for navigation
    line_index: BTreeMap<usize, LineInfo>,
    /// Search cache for recent queries
    search_cache: HashMap<String, SearchResult>,
    /// Index metadata
    metadata: SearchIndexMetadata,
}

/// Text match information
#[derive(Debug, Clone)]
struct TextMatch {
    /// Position in document
    position: Range<usize>,
    /// Line number
    line: usize,
    /// Column position
    column: usize,
    /// Context around match
    context: String,
    /// Match score (for fuzzy search)
    score: f32,
}

/// Line information for navigation
#[derive(Debug, Clone)]
struct LineInfo {
    /// Line number
    line_number: usize,
    /// Start position in document
    start_position: usize,
    /// End position in document
    end_position: usize,
    /// Line content
    content: String,
    /// Line type (heading, list, etc.)
    line_type: LineType,
}

/// Types of lines for navigation
#[derive(Debug, Clone, PartialEq, Eq)]
enum LineType {
    Text,
    Heading(usize),
    ListItem,
    CodeBlock,
    BlockQuote,
    Table,
    HorizontalRule,
}

/// Search result with caching
#[derive(Debug, Clone)]
struct SearchResult {
    /// Query that generated this result
    query: String,
    /// Matches found
    matches: Vec<TextMatch>,
    /// Search options used
    options: SearchOptions,
    /// Result timestamp
    timestamp: u64,
    /// Time taken to generate result (ms)
    search_time_ms: u64,
}

/// Search options for optimization
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct SearchOptions {
    /// Case sensitive search
    case_sensitive: bool,
    /// Use regex
    use_regex: bool,
    /// Whole words only
    whole_words: bool,
    /// Fuzzy search threshold
    fuzzy_threshold: Option<f32>,
    /// Maximum results
    max_results: Option<usize>,
}

/// Search index metadata
#[derive(Debug, Clone)]
struct SearchIndexMetadata {
    /// Total words indexed
    word_count: usize,
    /// Total n-grams indexed
    ngram_count: usize,
    /// Index size in bytes
    index_size: usize,
    /// Last rebuild timestamp
    last_rebuilt: u64,
    /// Index version
    version: u64,
}

/// Background task manager
struct BackgroundTaskManager {
    /// Running tasks
    tasks: HashMap<Uuid, BackgroundTask>,
    /// Task queue
    task_queue: VecDeque<PendingTask>,
    /// Worker threads
    workers: Vec<WorkerThread>,
    /// Task results
    results: Arc<Mutex<HashMap<Uuid, TaskResult>>>,
    /// Shutdown signal
    shutdown_sender: Option<mpsc::Sender<()>>,
}

/// Background task
struct BackgroundTask {
    /// Task ID
    id: Uuid,
    /// Task type
    task_type: TaskType,
    /// Task status
    status: TaskStatus,
    /// Start time
    start_time: Instant,
    /// Progress percentage
    progress: f32,
    /// Cancellation token
    cancel_sender: mpsc::Sender<()>,
}

/// Types of background tasks
#[derive(Debug, Clone, PartialEq, Eq)]
enum TaskType {
    IndexRebuild,
    CacheWarmup,
    DocumentValidation,
    SearchPrecomputation,
    MemoryOptimization,
    ChunkPreload,
}

/// Task status
#[derive(Debug, Clone, PartialEq, Eq)]
enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Pending task
struct PendingTask {
    /// Task type
    task_type: TaskType,
    /// Task data
    data: TaskData,
    /// Priority
    priority: TaskPriority,
    /// Created timestamp
    created_at: Instant,
}

/// Task data payload
#[derive(Debug, Clone)]
enum TaskData {
    IndexRebuild { content: String },
    CacheWarmup { chunk_ranges: Vec<Range<usize>> },
    Validation { content: String },
    SearchPrecomputation { queries: Vec<String> },
    MemoryOptimization { target_size: usize },
    ChunkPreload { chunk_ids: Vec<ChunkId> },
}

/// Task priority levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Worker thread
struct WorkerThread {
    /// Thread ID
    id: Uuid,
    /// Thread handle
    handle: thread::JoinHandle<()>,
    /// Task receiver
    task_receiver: mpsc::Receiver<PendingTask>,
}

/// Task result
#[derive(Debug, Clone)]
struct TaskResult {
    /// Task ID
    task_id: Uuid,
    /// Success status
    success: bool,
    /// Result data
    data: Option<serde_json::Value>,
    /// Error message
    error: Option<String>,
    /// Execution time
    execution_time: Duration,
}

/// Performance metrics tracking
#[derive(Debug, Clone)]
struct PerformanceMetrics {
    /// Cache metrics
    cache_hits: u64,
    cache_misses: u64,
    cache_evictions: u64,
    /// Search metrics
    search_count: u64,
    total_search_time_ms: u64,
    average_search_time_ms: f32,
    /// Memory metrics
    peak_memory_usage: usize,
    current_memory_usage: usize,
    memory_allocations: u64,
    /// Background task metrics
    tasks_completed: u64,
    tasks_failed: u64,
    total_task_time_ms: u64,
}

/// Configuration for performance optimization
#[derive(Debug, Clone)]
pub struct OptimizationConfig {
    /// Maximum cache size in MB
    pub max_cache_size_mb: usize,
    /// Chunk size for lazy loading
    pub chunk_size_bytes: usize,
    /// Number of background worker threads
    pub worker_thread_count: usize,
    /// Search index rebuild threshold (document changes)
    pub index_rebuild_threshold: usize,
    /// Cache eviction strategy
    pub cache_eviction_strategy: CacheEvictionStrategy,
    /// Search optimization settings
    pub search_optimization: SearchOptimizationConfig,
    /// Memory optimization settings
    pub memory_optimization: MemoryOptimizationConfig,
}

/// Cache eviction strategies
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheEvictionStrategy {
    /// Least Recently Used
    LRU,
    /// Least Frequently Used
    LFU,
    /// Time-based expiration
    TTL(Duration),
    /// Size-based with priority
    SizePriority,
}

/// Search optimization configuration
#[derive(Debug, Clone)]
pub struct SearchOptimizationConfig {
    /// Enable fuzzy search
    pub enable_fuzzy_search: bool,
    /// N-gram size for fuzzy search
    pub ngram_size: usize,
    /// Maximum search cache size
    pub max_search_cache_size: usize,
    /// Search result cache TTL
    pub search_cache_ttl_seconds: u64,
    /// Precompute common searches
    pub precompute_searches: bool,
}

/// Memory optimization configuration
#[derive(Debug, Clone)]
pub struct MemoryOptimizationConfig {
    /// Enable automatic garbage collection
    pub auto_gc: bool,
    /// GC trigger threshold (memory usage %)
    pub gc_threshold: f32,
    /// Enable memory compression
    pub enable_compression: bool,
    /// Compress chunks older than threshold
    pub compression_age_threshold_seconds: u64,
}

/// Result type for performance operations
pub type PerformanceResult<T> = Result<T, PerformanceError>;

/// Performance optimization errors
#[derive(Debug, thiserror::Error)]
pub enum PerformanceError {
    #[error("Cache error: {0}")]
    CacheError(String),
    #[error("Search index error: {0}")]
    SearchIndexError(String),
    #[error("Background task error: {0}")]
    BackgroundTaskError(String),
    #[error("Memory optimization error: {0}")]
    MemoryError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl MarkdownPerformanceOptimizer {
    /// Create new performance optimizer
    pub fn new(config: OptimizationConfig) -> PerformanceResult<Self> {
        let document_cache = Arc::new(RwLock::new(DocumentCache::new()));
        let search_index = Arc::new(RwLock::new(SearchIndex::new()));
        let task_manager = BackgroundTaskManager::new(config.worker_thread_count)?;
        let metrics = Arc::new(Mutex::new(PerformanceMetrics::new()));

        Ok(Self {
            document_cache,
            search_index,
            task_manager,
            metrics,
            config,
        })
    }

    /// Optimize document for performance
    pub fn optimize_document(&mut self, processor: &mut MarkdownProcessor) -> PerformanceResult<()> {
        let content = processor.content().to_string();
        
        // Create chunks for lazy loading
        self.create_document_chunks(&content)?;
        
        // Rebuild search index in background
        self.schedule_index_rebuild(content.clone())?;
        
        // Warm up cache for visible content
        self.schedule_cache_warmup(vec![0..content.len().min(self.config.chunk_size_bytes)])?;
        
        Ok(())
    }

    /// Get document chunk with lazy loading
    pub fn get_chunk(&self, range: Range<usize>) -> PerformanceResult<String> {
        let chunk_id = self.range_to_chunk_id(&range);
        
        // Try cache first
        if let Some(chunk) = self.get_cached_chunk(chunk_id) {
            self.update_metrics_cache_hit();
            return Ok(chunk.content);
        }
        
        self.update_metrics_cache_miss();
        
        // Load chunk on demand
        self.load_chunk_on_demand(chunk_id, range)
    }

    /// Perform optimized search
    pub fn search_optimized(&self, query: &str, options: SearchOptions) -> PerformanceResult<Vec<TextMatch>> {
        let search_key = self.generate_search_key(query, &options);
        
        // Check search cache
        if let Some(cached_result) = self.get_cached_search_result(&search_key) {
            return Ok(cached_result.matches);
        }
        
        let start_time = Instant::now();
        
        // Perform search using index
        let matches = if options.fuzzy_threshold.is_some() {
            self.fuzzy_search(query, &options)?
        } else if options.use_regex {
            self.regex_search(query, &options)?
        } else {
            self.exact_search(query, &options)?
        };
        
        let search_time = start_time.elapsed();
        
        // Cache result
        self.cache_search_result(search_key, query.to_string(), matches.clone(), options, search_time);
        
        // Update metrics
        self.update_search_metrics(search_time);
        
        Ok(matches)
    }

    /// Get document statistics with caching
    pub fn get_stats_cached(&self, processor: &mut MarkdownTextProcessor) -> PerformanceResult<DocumentStats> {
        let content_hash = self.calculate_content_hash(processor.content());
        
        // Check if stats are cached
        if let Some(cached_stats) = self.get_cached_stats(content_hash) {
            return Ok(cached_stats);
        }
        
        // Calculate stats and cache
        let stats = processor.get_stats().clone();
        self.cache_stats(content_hash, stats.clone());
        
        Ok(stats)
    }

    /// Preload chunks for better performance
    pub fn preload_chunks(&mut self, ranges: Vec<Range<usize>>) -> PerformanceResult<()> {
        let chunk_ids: Vec<ChunkId> = ranges.iter()
            .map(|range| self.range_to_chunk_id(range))
            .collect();
        
        self.schedule_chunk_preload(chunk_ids)?;
        Ok(())
    }

    /// Get performance metrics
    pub fn get_metrics(&self) -> PerformanceResult<PerformanceMetrics> {
        let metrics = self.metrics.lock()
            .map_err(|_| PerformanceError::MemoryError("Failed to acquire metrics lock".to_string()))?;
        Ok(metrics.clone())
    }

    /// Optimize memory usage
    pub fn optimize_memory(&mut self) -> PerformanceResult<()> {
        // Calculate current memory usage
        let current_usage = self.calculate_memory_usage()?;
        let max_size = self.config.max_cache_size_mb * 1024 * 1024;
        
        if current_usage > max_size {
            let target_size = (max_size as f32 * 0.8) as usize; // Target 80% of max
            self.schedule_memory_optimization(target_size)?;
        }
        
        Ok(())
    }

    /// Clear all caches
    pub fn clear_caches(&mut self) -> PerformanceResult<()> {
        {
            let mut cache = self.document_cache.write()
                .map_err(|_| PerformanceError::CacheError("Failed to acquire cache lock".to_string()))?;
            cache.clear();
        }
        
        {
            let mut index = self.search_index.write()
                .map_err(|_| PerformanceError::SearchIndexError("Failed to acquire index lock".to_string()))?;
            index.clear();
        }
        
        Ok(())
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> PerformanceResult<CacheStats> {
        let cache = self.document_cache.read()
            .map_err(|_| PerformanceError::CacheError("Failed to acquire cache lock".to_string()))?;
        
        Ok(CacheStats {
            total_chunks: cache.chunks.len(),
            memory_usage_bytes: cache.memory_usage,
            hit_ratio: cache.metadata.hit_ratio,
            total_hits: cache.metadata.total_hits,
            total_misses: cache.metadata.total_misses,
        })
    }

    // Private helper methods

    fn create_document_chunks(&self, content: &str) -> PerformanceResult<()> {
        let mut cache = self.document_cache.write()
            .map_err(|_| PerformanceError::CacheError("Failed to acquire cache lock".to_string()))?;
        
        let chunk_size = self.config.chunk_size_bytes;
        let mut chunk_id = 0u64;
        
        for (i, chunk_start) in (0..content.len()).step_by(chunk_size).enumerate() {
            let chunk_end = (chunk_start + chunk_size).min(content.len());
            let chunk_content = content[chunk_start..chunk_end].to_string();
            
            let chunk = DocumentChunk {
                id: ChunkId(chunk_id),
                range: chunk_start..chunk_end,
                content: chunk_content.clone(),
                ast: None,
                validation: None,
                last_accessed: current_timestamp(),
                access_count: 0,
                memory_size: chunk_content.len(),
            };
            
            cache.chunks.insert(ChunkId(chunk_id), chunk);
            cache.memory_usage += chunk_content.len();
            chunk_id += 1;
        }
        
        cache.metadata.chunk_count = cache.chunks.len();
        cache.metadata.total_size = content.len();
        cache.metadata.last_updated = current_timestamp();
        
        Ok(())
    }

    fn get_cached_chunk(&self, chunk_id: ChunkId) -> Option<DocumentChunk> {
        if let Ok(mut cache) = self.document_cache.write() {
            if let Some(chunk) = cache.chunks.get_mut(&chunk_id) {
                chunk.last_accessed = current_timestamp();
                chunk.access_count += 1;
                
                // Update LRU tracker
                cache.lru_tracker.retain(|&id| id != chunk_id);
                cache.lru_tracker.push_back(chunk_id);
                
                return Some(chunk.clone());
            }
        }
        None
    }

    fn load_chunk_on_demand(&self, chunk_id: ChunkId, range: Range<usize>) -> PerformanceResult<String> {
        // This would typically load from disk or regenerate the chunk
        // For now, return empty string as placeholder
        Ok(String::new())
    }

    fn range_to_chunk_id(&self, range: &Range<usize>) -> ChunkId {
        let chunk_index = range.start / self.config.chunk_size_bytes;
        ChunkId(chunk_index as u64)
    }

    fn exact_search(&self, query: &str, options: &SearchOptions) -> PerformanceResult<Vec<TextMatch>> {
        let index = self.search_index.read()
            .map_err(|_| PerformanceError::SearchIndexError("Failed to acquire index lock".to_string()))?;
        
        let search_term = if options.case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };
        
        let matches = index.word_index.get(&search_term)
            .cloned()
            .unwrap_or_default();
        
        Ok(matches)
    }

    fn regex_search(&self, pattern: &str, options: &SearchOptions) -> PerformanceResult<Vec<TextMatch>> {
        // Placeholder for regex search implementation
        Ok(Vec::new())
    }

    fn fuzzy_search(&self, query: &str, options: &SearchOptions) -> PerformanceResult<Vec<TextMatch>> {
        let index = self.search_index.read()
            .map_err(|_| PerformanceError::SearchIndexError("Failed to acquire index lock".to_string()))?;
        
        let threshold = options.fuzzy_threshold.unwrap_or(0.8);
        let mut matches = Vec::new();
        
        // Simple fuzzy search using n-grams
        for ngram in self.generate_ngrams(query, self.config.search_optimization.ngram_size) {
            if let Some(ngram_matches) = index.ngram_index.get(&ngram) {
                for text_match in ngram_matches {
                    if text_match.score >= threshold {
                        matches.push(text_match.clone());
                    }
                }
            }
        }
        
        // Sort by score and limit results
        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        if let Some(max_results) = options.max_results {
            matches.truncate(max_results);
        }
        
        Ok(matches)
    }

    fn generate_ngrams(&self, text: &str, n: usize) -> Vec<String> {
        let mut ngrams = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        
        for i in 0..=chars.len().saturating_sub(n) {
            let ngram: String = chars[i..i + n].iter().collect();
            ngrams.push(ngram);
        }
        
        ngrams
    }

    fn generate_search_key(&self, query: &str, options: &SearchOptions) -> String {
        format!("{:?}_{}", options, query)
    }

    fn get_cached_search_result(&self, key: &str) -> Option<SearchResult> {
        if let Ok(index) = self.search_index.read() {
            if let Some(result) = index.search_cache.get(key) {
                // Check if cache entry is still valid
                let ttl = Duration::from_secs(self.config.search_optimization.search_cache_ttl_seconds);
                let age = Duration::from_millis(current_timestamp() - result.timestamp);
                
                if age < ttl {
                    return Some(result.clone());
                }
            }
        }
        None
    }

    fn cache_search_result(&self, key: String, query: String, matches: Vec<TextMatch>, options: SearchOptions, search_time: Duration) {
        if let Ok(mut index) = self.search_index.write() {
            let result = SearchResult {
                query,
                matches,
                options,
                timestamp: current_timestamp(),
                search_time_ms: search_time.as_millis() as u64,
            };
            
            index.search_cache.insert(key, result);
            
            // Limit cache size
            if index.search_cache.len() > self.config.search_optimization.max_search_cache_size {
                // Remove oldest entries
                let mut entries: Vec<_> = index.search_cache.iter().collect();
                entries.sort_by_key(|(_, result)| result.timestamp);
                
                for (key, _) in entries.iter().take(index.search_cache.len() / 4) {
                    index.search_cache.remove(*key);
                }
            }
        }
    }

    fn calculate_content_hash(&self, content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    fn get_cached_stats(&self, _content_hash: u64) -> Option<DocumentStats> {
        // Placeholder for stats caching
        None
    }

    fn cache_stats(&self, _content_hash: u64, _stats: DocumentStats) {
        // Placeholder for stats caching
    }

    fn calculate_memory_usage(&self) -> PerformanceResult<usize> {
        let cache = self.document_cache.read()
            .map_err(|_| PerformanceError::CacheError("Failed to acquire cache lock".to_string()))?;
        Ok(cache.memory_usage)
    }

    fn schedule_index_rebuild(&mut self, content: String) -> PerformanceResult<()> {
        self.task_manager.schedule_task(PendingTask {
            task_type: TaskType::IndexRebuild,
            data: TaskData::IndexRebuild { content },
            priority: TaskPriority::Normal,
            created_at: Instant::now(),
        })
    }

    fn schedule_cache_warmup(&mut self, ranges: Vec<Range<usize>>) -> PerformanceResult<()> {
        self.task_manager.schedule_task(PendingTask {
            task_type: TaskType::CacheWarmup,
            data: TaskData::CacheWarmup { chunk_ranges: ranges },
            priority: TaskPriority::High,
            created_at: Instant::now(),
        })
    }

    fn schedule_chunk_preload(&mut self, chunk_ids: Vec<ChunkId>) -> PerformanceResult<()> {
        self.task_manager.schedule_task(PendingTask {
            task_type: TaskType::ChunkPreload,
            data: TaskData::ChunkPreload { chunk_ids },
            priority: TaskPriority::Normal,
            created_at: Instant::now(),
        })
    }

    fn schedule_memory_optimization(&mut self, target_size: usize) -> PerformanceResult<()> {
        self.task_manager.schedule_task(PendingTask {
            task_type: TaskType::MemoryOptimization,
            data: TaskData::MemoryOptimization { target_size },
            priority: TaskPriority::Low,
            created_at: Instant::now(),
        })
    }

    fn update_metrics_cache_hit(&self) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.cache_hits += 1;
        }
    }

    fn update_metrics_cache_miss(&self) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.cache_misses += 1;
        }
    }

    fn update_search_metrics(&self, search_time: Duration) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.search_count += 1;
            let search_time_ms = search_time.as_millis() as u64;
            metrics.total_search_time_ms += search_time_ms;
            metrics.average_search_time_ms = metrics.total_search_time_ms as f32 / metrics.search_count as f32;
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_chunks: usize,
    pub memory_usage_bytes: usize,
    pub hit_ratio: f32,
    pub total_hits: u64,
    pub total_misses: u64,
}

impl DocumentCache {
    fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            metadata: CacheMetadata {
                total_size: 0,
                chunk_count: 0,
                last_updated: current_timestamp(),
                hit_ratio: 0.0,
                total_hits: 0,
                total_misses: 0,
            },
            lru_tracker: VecDeque::new(),
            memory_usage: 0,
        }
    }

    fn clear(&mut self) {
        self.chunks.clear();
        self.lru_tracker.clear();
        self.memory_usage = 0;
        self.metadata.chunk_count = 0;
        self.metadata.total_size = 0;
    }
}

impl SearchIndex {
    fn new() -> Self {
        Self {
            word_index: HashMap::new(),
            ngram_index: HashMap::new(),
            line_index: BTreeMap::new(),
            search_cache: HashMap::new(),
            metadata: SearchIndexMetadata {
                word_count: 0,
                ngram_count: 0,
                index_size: 0,
                last_rebuilt: current_timestamp(),
                version: 1,
            },
        }
    }

    fn clear(&mut self) {
        self.word_index.clear();
        self.ngram_index.clear();
        self.line_index.clear();
        self.search_cache.clear();
        self.metadata.word_count = 0;
        self.metadata.ngram_count = 0;
        self.metadata.index_size = 0;
    }
}

impl BackgroundTaskManager {
    fn new(worker_count: usize) -> PerformanceResult<Self> {
        let mut manager = Self {
            tasks: HashMap::new(),
            task_queue: VecDeque::new(),
            workers: Vec::new(),
            results: Arc::new(Mutex::new(HashMap::new())),
            shutdown_sender: None,
        };

        manager.start_workers(worker_count)?;
        Ok(manager)
    }

    fn start_workers(&mut self, worker_count: usize) -> PerformanceResult<()> {
        let (shutdown_sender, shutdown_receiver) = mpsc::channel();
        self.shutdown_sender = Some(shutdown_sender);

        for _ in 0..worker_count {
            let (task_sender, task_receiver) = mpsc::channel();
            let results = Arc::clone(&self.results);
            let shutdown_rx = shutdown_receiver.try_clone()
                .map_err(|_| PerformanceError::BackgroundTaskError("Failed to clone shutdown receiver".to_string()))?;

            let handle = thread::spawn(move || {
                loop {
                    // Check for shutdown signal
                    if shutdown_rx.try_recv().is_ok() {
                        break;
                    }

                    // Process tasks
                    if let Ok(task) = task_receiver.recv_timeout(Duration::from_millis(100)) {
                        let task_id = Uuid::new_v4();
                        let start_time = Instant::now();
                        
                        // Execute task
                        let result = Self::execute_task(task);
                        let execution_time = start_time.elapsed();
                        
                        // Store result
                        if let Ok(mut results_guard) = results.lock() {
                            results_guard.insert(task_id, TaskResult {
                                task_id,
                                success: result.is_ok(),
                                data: result.ok(),
                                error: result.err().map(|e| e.to_string()),
                                execution_time,
                            });
                        }
                    }
                }
            });

            self.workers.push(WorkerThread {
                id: Uuid::new_v4(),
                handle,
                task_receiver,
            });
        }

        Ok(())
    }

    fn schedule_task(&mut self, task: PendingTask) -> PerformanceResult<()> {
        self.task_queue.push_back(task);
        Ok(())
    }

    fn execute_task(task: PendingTask) -> Result<serde_json::Value, String> {
        match task.task_type {
            TaskType::IndexRebuild => {
                // Placeholder for index rebuild
                Ok(serde_json::json!({"status": "completed"}))
            }
            TaskType::CacheWarmup => {
                // Placeholder for cache warmup
                Ok(serde_json::json!({"status": "completed"}))
            }
            TaskType::ChunkPreload => {
                // Placeholder for chunk preload
                Ok(serde_json::json!({"status": "completed"}))
            }
            TaskType::MemoryOptimization => {
                // Placeholder for memory optimization
                Ok(serde_json::json!({"status": "completed"}))
            }
            _ => Err("Task type not implemented".to_string()),
        }
    }
}

impl PerformanceMetrics {
    fn new() -> Self {
        Self {
            cache_hits: 0,
            cache_misses: 0,
            cache_evictions: 0,
            search_count: 0,
            total_search_time_ms: 0,
            average_search_time_ms: 0.0,
            peak_memory_usage: 0,
            current_memory_usage: 0,
            memory_allocations: 0,
            tasks_completed: 0,
            tasks_failed: 0,
            total_task_time_ms: 0,
        }
    }
}

impl Default for OptimizationConfig {
    fn default() -> Self {
        Self {
            max_cache_size_mb: 100,
            chunk_size_bytes: 64 * 1024, // 64KB chunks
            worker_thread_count: 2,
            index_rebuild_threshold: 1000,
            cache_eviction_strategy: CacheEvictionStrategy::LRU,
            search_optimization: SearchOptimizationConfig {
                enable_fuzzy_search: true,
                ngram_size: 3,
                max_search_cache_size: 1000,
                search_cache_ttl_seconds: 300, // 5 minutes
                precompute_searches: false,
            },
            memory_optimization: MemoryOptimizationConfig {
                auto_gc: true,
                gc_threshold: 0.8,
                enable_compression: false,
                compression_age_threshold_seconds: 3600, // 1 hour
            },
        }
    }
}

/// Get current timestamp in milliseconds since Unix epoch
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_optimizer_creation() {
        let config = OptimizationConfig::default();
        let optimizer = MarkdownPerformanceOptimizer::new(config);
        assert!(optimizer.is_ok());
    }

    #[test]
    fn test_chunk_creation() {
        let config = OptimizationConfig {
            chunk_size_bytes: 10,
            ..OptimizationConfig::default()
        };
        let optimizer = MarkdownPerformanceOptimizer::new(config).unwrap();
        
        let result = optimizer.create_document_chunks("Hello, world! This is a test.");
        assert!(result.is_ok());
    }

    #[test]
    fn test_search_options_hash() {
        let options1 = SearchOptions {
            case_sensitive: true,
            use_regex: false,
            whole_words: false,
            fuzzy_threshold: None,
            max_results: Some(10),
        };
        
        let options2 = options1.clone();
        assert_eq!(options1, options2);
    }

    #[test]
    fn test_cache_stats() {
        let cache = DocumentCache::new();
        assert_eq!(cache.chunks.len(), 0);
        assert_eq!(cache.memory_usage, 0);
    }
}