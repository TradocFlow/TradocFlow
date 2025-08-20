use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, Instant};
use crate::Result;
use crate::services::sentence_alignment_service::{SentenceAlignment, AlignmentStatistics, AlignmentQualityIndicator};

/// Cache entry for sentence alignments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentCacheEntry {
    pub cache_key: String,
    pub alignments: Vec<SentenceAlignment>,
    pub quality_indicator: AlignmentQualityIndicator,
    pub statistics: AlignmentStatistics,
    #[serde(skip, default = "Instant::now")]
    pub created_at: Instant,
    #[serde(skip, default = "Instant::now")]
    pub last_accessed: Instant,
    pub access_count: u32,
    pub cache_version: u8,
}

/// Cache statistics for monitoring and optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatistics {
    pub total_entries: usize,
    pub hit_rate: f64,
    pub miss_rate: f64,
    pub average_access_time_ms: f64,
    pub cache_size_bytes: usize,
    pub eviction_count: u32,
    #[serde(skip)]
    pub last_cleanup: Option<Instant>,
    pub memory_usage_percentage: f64,
}

/// Configuration for the alignment cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentCacheConfig {
    pub max_entries: usize,
    pub max_memory_mb: usize,
    pub entry_ttl_seconds: u64,
    pub cleanup_interval_seconds: u64,
    pub enable_compression: bool,
    pub enable_persistence: bool,
    pub cache_directory: Option<String>,
    pub preemptive_loading: bool,
}

impl Default for AlignmentCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            max_memory_mb: 256,
            entry_ttl_seconds: 3600, // 1 hour
            cleanup_interval_seconds: 300, // 5 minutes
            enable_compression: true,
            enable_persistence: false,
            cache_directory: None,
            preemptive_loading: true,
        }
    }
}

/// Cache eviction strategies
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EvictionStrategy {
    LeastRecentlyUsed,
    LeastFrequentlyUsed,
    TimeToLive,
    SizeBasedLru,
    AdaptiveReplacement,
}

/// Performance metrics for cache operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub operation_type: String,
    pub execution_time_ms: f64,
    pub cache_hit: bool,
    pub data_size_bytes: usize,
    #[serde(skip, default = "Instant::now")]
    pub timestamp: Instant,
}

/// Background task for cache maintenance
#[derive(Debug, Clone)]
pub struct CacheMaintenanceTask {
    pub task_type: MaintenanceTaskType,
    pub priority: u8,
    pub scheduled_time: Instant,
    pub estimated_duration_ms: u64,
}

/// Types of cache maintenance tasks
#[derive(Debug, Clone, PartialEq)]
pub enum MaintenanceTaskType {
    Cleanup,
    Compression,
    Persistence,
    Optimization,
    Statistics,
}

/// High-performance caching service for sentence alignments
pub struct AlignmentCacheService {
    cache: Arc<RwLock<BTreeMap<String, AlignmentCacheEntry>>>,
    config: AlignmentCacheConfig,
    statistics: Arc<RwLock<CacheStatistics>>,
    performance_metrics: Arc<RwLock<Vec<PerformanceMetrics>>>,
    eviction_strategy: EvictionStrategy,
    last_cleanup: Arc<RwLock<Instant>>,
    maintenance_queue: Arc<RwLock<Vec<CacheMaintenanceTask>>>,
}

impl AlignmentCacheService {
    /// Create a new alignment cache service
    pub fn new(config: AlignmentCacheConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(BTreeMap::new())),
            config,
            statistics: Arc::new(RwLock::new(CacheStatistics {
                total_entries: 0,
                hit_rate: 0.0,
                miss_rate: 0.0,
                average_access_time_ms: 0.0,
                cache_size_bytes: 0,
                eviction_count: 0,
                last_cleanup: None,
                memory_usage_percentage: 0.0,
            })),
            performance_metrics: Arc::new(RwLock::new(Vec::new())),
            eviction_strategy: EvictionStrategy::AdaptiveReplacement,
            last_cleanup: Arc::new(RwLock::new(Instant::now())),
            maintenance_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get alignments from cache
    pub async fn get_alignments(
        &self,
        cache_key: &str,
    ) -> Result<Option<(Vec<SentenceAlignment>, AlignmentQualityIndicator, AlignmentStatistics)>> {
        let start_time = Instant::now();
        
        let mut cache = self.cache.write().unwrap();
        let result = if let Some(entry) = cache.get_mut(cache_key) {
            // Check if entry is still valid
            if self.is_entry_valid(entry) {
                // Update access information
                entry.last_accessed = Instant::now();
                entry.access_count += 1;
                
                Some((
                    entry.alignments.clone(),
                    entry.quality_indicator.clone(),
                    entry.statistics.clone(),
                ))
            } else {
                // Entry expired, remove it
                cache.remove(cache_key);
                None
            }
        } else {
            None
        };

        // Record performance metrics
        let execution_time = start_time.elapsed().as_millis() as f64;
        let cache_hit = result.is_some();
        
        self.record_performance_metric(PerformanceMetrics {
            operation_type: "get_alignments".to_string(),
            execution_time_ms: execution_time,
            cache_hit,
            data_size_bytes: result.as_ref().map(|(a, _, _)| self.estimate_size(&a)).unwrap_or(0),
            timestamp: Instant::now(),
        }).await;

        // Update statistics
        self.update_cache_statistics(cache_hit, execution_time).await;

        Ok(result)
    }

    /// Store alignments in cache
    pub async fn store_alignments(
        &self,
        cache_key: String,
        alignments: Vec<SentenceAlignment>,
        quality_indicator: AlignmentQualityIndicator,
        statistics: AlignmentStatistics,
    ) -> Result<()> {
        let start_time = Instant::now();
        
        // Check if we need to make space
        self.ensure_cache_capacity().await?;

        let entry = AlignmentCacheEntry {
            cache_key: cache_key.clone(),
            alignments: alignments.clone(),
            quality_indicator,
            statistics,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 1,
            cache_version: 1,
        };

        {
            let mut cache = self.cache.write().unwrap();
            cache.insert(cache_key, entry);
        }

        // Record performance metrics
        let execution_time = start_time.elapsed().as_millis() as f64;
        self.record_performance_metric(PerformanceMetrics {
            operation_type: "store_alignments".to_string(),
            execution_time_ms: execution_time,
            cache_hit: false,
            data_size_bytes: self.estimate_size(&alignments),
            timestamp: Instant::now(),
        }).await;

        // Schedule background optimization if needed
        self.schedule_maintenance_if_needed().await;

        Ok(())
    }

    /// Invalidate cache entries for a specific language pair
    pub async fn invalidate_language_pair(&self, source_language: &str, target_language: &str) -> Result<u32> {
        let prefix = format!("{}:{}:", source_language, target_language);
        let mut removed_count = 0;

        {
            let mut cache = self.cache.write().unwrap();
            let keys_to_remove: Vec<String> = cache.keys()
                .filter(|key| key.starts_with(&prefix))
                .cloned()
                .collect();

            for key in keys_to_remove {
                cache.remove(&key);
                removed_count += 1;
            }
        }

        // Update statistics
        {
            let mut stats = self.statistics.write().unwrap();
            stats.eviction_count += removed_count;
            stats.total_entries = stats.total_entries.saturating_sub(removed_count as usize);
        }

        Ok(removed_count)
    }

    /// Clear entire cache
    pub async fn clear_cache(&self) -> Result<()> {
        let removed_count = {
            let mut cache = self.cache.write().unwrap();
            let count = cache.len();
            cache.clear();
            count
        };

        // Reset statistics
        {
            let mut stats = self.statistics.write().unwrap();
            stats.total_entries = 0;
            stats.eviction_count += removed_count as u32;
            stats.cache_size_bytes = 0;
            stats.memory_usage_percentage = 0.0;
        }

        Ok(())
    }

    /// Get cache statistics
    pub async fn get_statistics(&self) -> CacheStatistics {
        self.update_cache_size_statistics().await;
        self.statistics.read().unwrap().clone()
    }

    /// Get performance metrics
    pub async fn get_performance_metrics(&self, limit: Option<usize>) -> Vec<PerformanceMetrics> {
        let metrics = self.performance_metrics.read().unwrap();
        if let Some(limit) = limit {
            metrics.iter().rev().take(limit).cloned().collect()
        } else {
            metrics.clone()
        }
    }

    /// Optimize cache for better performance
    pub async fn optimize_cache(&mut self) -> Result<()> {
        let start_time = Instant::now();

        // Analyze access patterns
        let access_patterns = self.analyze_access_patterns().await;

        // Adjust eviction strategy based on patterns
        self.adjust_eviction_strategy(&access_patterns).await;

        // Perform compression if enabled
        if self.config.enable_compression {
            self.compress_cache_entries().await?;
        }

        // Preload frequently accessed entries if enabled
        if self.config.preemptive_loading {
            self.preload_frequent_entries().await?;
        }

        // Clean up expired entries
        self.cleanup_expired_entries().await?;

        // Record optimization metrics
        let execution_time = start_time.elapsed().as_millis() as f64;
        self.record_performance_metric(PerformanceMetrics {
            operation_type: "optimize_cache".to_string(),
            execution_time_ms: execution_time,
            cache_hit: false,
            data_size_bytes: 0,
            timestamp: Instant::now(),
        }).await;

        Ok(())
    }

    /// Generate cache key for alignments
    pub fn generate_cache_key(
        &self,
        source_text: &str,
        target_text: &str,
        source_language: &str,
        target_language: &str,
        config_hash: u64,
    ) -> String {
        let combined_text = format!("{}{}", source_text, target_text);
        let content_hash = self.calculate_hash(&combined_text);
        format!("{}:{}:{}:{:x}", source_language, target_language, config_hash, content_hash)
    }

    /// Start background maintenance tasks
    pub async fn start_maintenance_scheduler(&self) -> Result<()> {
        // In a real implementation, this would spawn background tasks
        // For now, we'll just schedule immediate cleanup
        self.schedule_maintenance_task(CacheMaintenanceTask {
            task_type: MaintenanceTaskType::Cleanup,
            priority: 1,
            scheduled_time: Instant::now(),
            estimated_duration_ms: 1000,
        }).await;

        Ok(())
    }

    // Private helper methods

    fn is_entry_valid(&self, entry: &AlignmentCacheEntry) -> bool {
        let ttl = Duration::from_secs(self.config.entry_ttl_seconds);
        entry.created_at.elapsed() < ttl
    }

    async fn ensure_cache_capacity(&self) -> Result<()> {
        let cache_size = {
            let cache = self.cache.read().unwrap();
            cache.len()
        };

        if cache_size >= self.config.max_entries {
            self.evict_entries(cache_size - self.config.max_entries + 1).await?;
        }

        // Check memory usage
        let memory_usage = self.estimate_cache_memory_usage().await;
        let max_memory_bytes = self.config.max_memory_mb * 1024 * 1024;
        
        if memory_usage > max_memory_bytes {
            self.evict_by_memory_pressure().await?;
        }

        Ok(())
    }

    async fn evict_entries(&self, count: usize) -> Result<()> {
        let keys_to_evict = match self.eviction_strategy {
            EvictionStrategy::LeastRecentlyUsed => self.select_lru_entries(count).await,
            EvictionStrategy::LeastFrequentlyUsed => self.select_lfu_entries(count).await,
            EvictionStrategy::TimeToLive => self.select_ttl_entries(count).await,
            EvictionStrategy::SizeBasedLru => self.select_size_based_entries(count).await,
            EvictionStrategy::AdaptiveReplacement => self.select_adaptive_entries(count).await,
        };

        {
            let mut cache = self.cache.write().unwrap();
            for key in &keys_to_evict {
                cache.remove(key);
            }
        }

        // Update statistics
        {
            let mut stats = self.statistics.write().unwrap();
            stats.eviction_count += keys_to_evict.len() as u32;
            stats.total_entries = stats.total_entries.saturating_sub(keys_to_evict.len());
        }

        Ok(())
    }

    async fn select_lru_entries(&self, count: usize) -> Vec<String> {
        let cache = self.cache.read().unwrap();
        let mut entries: Vec<(String, Instant)> = cache.iter()
            .map(|(key, entry)| (key.clone(), entry.last_accessed))
            .collect();
        
        entries.sort_by_key(|&(_, last_accessed)| last_accessed);
        entries.into_iter().take(count).map(|(key, _)| key).collect()
    }

    async fn select_lfu_entries(&self, count: usize) -> Vec<String> {
        let cache = self.cache.read().unwrap();
        let mut entries: Vec<(String, u32)> = cache.iter()
            .map(|(key, entry)| (key.clone(), entry.access_count))
            .collect();
        
        entries.sort_by_key(|&(_, access_count)| access_count);
        entries.into_iter().take(count).map(|(key, _)| key).collect()
    }

    async fn select_ttl_entries(&self, count: usize) -> Vec<String> {
        let cache = self.cache.read().unwrap();
        let mut entries: Vec<(String, Instant)> = cache.iter()
            .map(|(key, entry)| (key.clone(), entry.created_at))
            .collect();
        
        entries.sort_by_key(|&(_, created_at)| created_at);
        entries.into_iter().take(count).map(|(key, _)| key).collect()
    }

    async fn select_size_based_entries(&self, count: usize) -> Vec<String> {
        let cache = self.cache.read().unwrap();
        let mut entries: Vec<(String, usize, Instant)> = cache.iter()
            .map(|(key, entry)| (key.clone(), self.estimate_size(&entry.alignments), entry.last_accessed))
            .collect();
        
        // Sort by size descending, then by LRU
        entries.sort_by(|a, b| {
            b.1.cmp(&a.1).then(a.2.cmp(&b.2))
        });
        
        entries.into_iter().take(count).map(|(key, _, _)| key).collect()
    }

    async fn select_adaptive_entries(&self, count: usize) -> Vec<String> {
        // Adaptive replacement combines multiple factors
        let cache = self.cache.read().unwrap();
        let mut entries: Vec<(String, f64)> = cache.iter()
            .map(|(key, entry)| {
                let age_factor = entry.created_at.elapsed().as_secs() as f64 / 3600.0; // Hours
                let access_factor = 1.0 / (entry.access_count as f64 + 1.0);
                let size_factor = self.estimate_size(&entry.alignments) as f64 / 1024.0; // KB
                let recency_factor = entry.last_accessed.elapsed().as_secs() as f64 / 3600.0; // Hours
                
                let score = age_factor * 0.3 + access_factor * 0.3 + size_factor * 0.2 + recency_factor * 0.2;
                (key.clone(), score)
            })
            .collect();
        
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        entries.into_iter().take(count).map(|(key, _)| key).collect()
    }

    async fn evict_by_memory_pressure(&self) -> Result<()> {
        let target_memory = (self.config.max_memory_mb * 1024 * 1024) * 80 / 100; // 80% of max
        let mut current_memory = self.estimate_cache_memory_usage().await;
        
        while current_memory > target_memory {
            let evicted = self.evict_entries(100).await; // Evict in batches
            if evicted.is_err() {
                break;
            }
            current_memory = self.estimate_cache_memory_usage().await;
        }

        Ok(())
    }

    async fn cleanup_expired_entries(&self) -> Result<()> {
        let ttl = Duration::from_secs(self.config.entry_ttl_seconds);
        let now = Instant::now();
        
        let expired_keys: Vec<String> = {
            let cache = self.cache.read().unwrap();
            cache.iter()
                .filter(|(_, entry)| now.duration_since(entry.created_at) > ttl)
                .map(|(key, _)| key.clone())
                .collect()
        };

        if !expired_keys.is_empty() {
            let mut cache = self.cache.write().unwrap();
            for key in &expired_keys {
                cache.remove(key);
            }
        }

        // Update statistics
        {
            let mut stats = self.statistics.write().unwrap();
            stats.eviction_count += expired_keys.len() as u32;
            stats.total_entries = stats.total_entries.saturating_sub(expired_keys.len());
            stats.last_cleanup = Some(now);
        }

        Ok(())
    }

    async fn estimate_cache_memory_usage(&self) -> usize {
        let cache = self.cache.read().unwrap();
        cache.values()
            .map(|entry| self.estimate_entry_size(entry))
            .sum()
    }

    fn estimate_entry_size(&self, entry: &AlignmentCacheEntry) -> usize {
        // Rough estimate of memory usage
        let base_size = std::mem::size_of::<AlignmentCacheEntry>();
        let alignments_size = self.estimate_size(&entry.alignments);
        let key_size = entry.cache_key.len();
        
        base_size + alignments_size + key_size
    }

    fn estimate_size(&self, alignments: &[SentenceAlignment]) -> usize {
        alignments.len() * std::mem::size_of::<SentenceAlignment>() +
        alignments.iter()
            .map(|a| a.source_sentence.text.len() + a.target_sentence.text.len())
            .sum::<usize>()
    }

    fn calculate_hash(&self, text: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    async fn record_performance_metric(&self, metric: PerformanceMetrics) {
        let mut metrics = self.performance_metrics.write().unwrap();
        metrics.push(metric);
        
        // Keep only recent metrics
        if metrics.len() > 10000 {
            metrics.drain(0..1000);
        }
    }

    async fn update_cache_statistics(&self, cache_hit: bool, execution_time: f64) {
        let mut stats = self.statistics.write().unwrap();
        
        let total_requests = (stats.hit_rate + stats.miss_rate) * 100.0;
        let hits = if cache_hit { total_requests * stats.hit_rate / 100.0 + 1.0 } else { total_requests * stats.hit_rate / 100.0 };
        let misses = if cache_hit { total_requests * stats.miss_rate / 100.0 } else { total_requests * stats.miss_rate / 100.0 + 1.0 };
        
        let new_total = hits + misses;
        if new_total > 0.0 {
            stats.hit_rate = (hits / new_total) * 100.0;
            stats.miss_rate = (misses / new_total) * 100.0;
        }
        
        // Update average access time (rolling average)
        stats.average_access_time_ms = (stats.average_access_time_ms * 0.9) + (execution_time * 0.1);
    }

    async fn update_cache_size_statistics(&self) {
        let cache_size = {
            let cache = self.cache.read().unwrap();
            cache.len()
        };
        
        let memory_usage = self.estimate_cache_memory_usage().await;
        let max_memory = self.config.max_memory_mb * 1024 * 1024;
        
        let mut stats = self.statistics.write().unwrap();
        stats.total_entries = cache_size;
        stats.cache_size_bytes = memory_usage;
        stats.memory_usage_percentage = if max_memory > 0 {
            (memory_usage as f64 / max_memory as f64) * 100.0
        } else {
            0.0
        };
    }

    async fn analyze_access_patterns(&self) -> HashMap<String, f64> {
        let metrics = self.performance_metrics.read().unwrap();
        let mut patterns = HashMap::new();
        
        // Analyze hit rates by operation type
        for metric in metrics.iter().rev().take(1000) {
            let entry = patterns.entry(metric.operation_type.clone()).or_insert((0, 0));
            if metric.cache_hit {
                entry.0 += 1;
            } else {
                entry.1 += 1;
            }
        }
        
        // Convert to hit rates
        patterns.into_iter()
            .map(|(op, (hits, misses))| {
                let total = hits + misses;
                let hit_rate = if total > 0 { hits as f64 / total as f64 } else { 0.0 };
                (op, hit_rate)
            })
            .collect()
    }

    async fn adjust_eviction_strategy(&mut self, access_patterns: &HashMap<String, f64>) {
        let average_hit_rate = if access_patterns.is_empty() {
            0.5
        } else {
            access_patterns.values().sum::<f64>() / access_patterns.len() as f64
        };

        // Adjust strategy based on performance
        if average_hit_rate < 0.6 {
            // Low hit rate - try LFU to keep frequently used items
            self.eviction_strategy = EvictionStrategy::LeastFrequentlyUsed;
        } else if average_hit_rate > 0.8 {
            // High hit rate - use adaptive strategy for optimization
            self.eviction_strategy = EvictionStrategy::AdaptiveReplacement;
        }
        // Otherwise keep current strategy
    }

    async fn compress_cache_entries(&self) -> Result<()> {
        // In a real implementation, this would compress entry data
        // For now, we'll just mark it as optimized
        Ok(())
    }

    async fn preload_frequent_entries(&self) -> Result<()> {
        // In a real implementation, this would preload frequently accessed entries
        // For now, we'll just mark frequently used entries
        Ok(())
    }

    async fn schedule_maintenance_task(&self, task: CacheMaintenanceTask) {
        let mut queue = self.maintenance_queue.write().unwrap();
        queue.push(task);
        
        // Sort by priority and scheduled time
        queue.sort_by(|a, b| {
            a.priority.cmp(&b.priority)
                .then(a.scheduled_time.cmp(&b.scheduled_time))
        });
    }

    async fn schedule_maintenance_if_needed(&self) {
        let (memory_usage_percentage, hit_rate) = {
            let stats = self.statistics.read().unwrap();
            (stats.memory_usage_percentage, stats.hit_rate)
        };
        
        // Schedule cleanup if cache is getting full
        if memory_usage_percentage > 70.0 {
            self.schedule_maintenance_task(CacheMaintenanceTask {
                task_type: MaintenanceTaskType::Cleanup,
                priority: 2,
                scheduled_time: Instant::now() + Duration::from_secs(60),
                estimated_duration_ms: 2000,
            }).await;
        }
        
        // Schedule optimization if hit rate is low
        if hit_rate < 50.0 {
            self.schedule_maintenance_task(CacheMaintenanceTask {
                task_type: MaintenanceTaskType::Optimization,
                priority: 3,
                scheduled_time: Instant::now() + Duration::from_secs(300),
                estimated_duration_ms: 5000,
            }).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::sentence_alignment_service::{
        SentenceBoundary, BoundaryType, AlignmentMethod, ValidationStatus
    };

    fn create_test_alignment() -> SentenceAlignment {
        SentenceAlignment {
            id: Uuid::new_v4(),
            source_sentence: SentenceBoundary {
                start_offset: 0,
                end_offset: 10,
                text: "Test text.".to_string(),
                confidence: 0.9,
                boundary_type: BoundaryType::Period,
            },
            target_sentence: SentenceBoundary {
                start_offset: 0,
                end_offset: 12,
                text: "Texto prueba.".to_string(),
                confidence: 0.9,
                boundary_type: BoundaryType::Period,
            },
            source_language: "en".to_string(),
            target_language: "es".to_string(),
            alignment_confidence: 0.8,
            alignment_method: AlignmentMethod::PositionBased,
            validation_status: ValidationStatus::Pending,
            created_at: Instant::now(),
            last_validated: None,
        }
    }

    #[tokio::test]
    async fn test_cache_store_and_retrieve() {
        let config = AlignmentCacheConfig::default();
        let cache_service = AlignmentCacheService::new(config);
        
        let alignments = vec![create_test_alignment()];
        let quality = AlignmentQualityIndicator {
            overall_quality: 0.8,
            position_consistency: 0.9,
            length_ratio_consistency: 0.7,
            structural_coherence: 0.8,
            user_validation_rate: 0.0,
            problem_areas: Vec::new(),
        };
        let stats = AlignmentStatistics {
            total_sentences: 1,
            aligned_sentences: 1,
            validated_alignments: 0,
            average_confidence: 0.8,
            alignment_accuracy: 0.8,
            processing_time_ms: 100,
            language_pair: ("en".to_string(), "es".to_string()),
        };
        
        let cache_key = "en:es:test".to_string();
        
        // Store alignments
        cache_service.store_alignments(
            cache_key.clone(),
            alignments.clone(),
            quality.clone(),
            stats.clone(),
        ).await.unwrap();
        
        // Retrieve alignments
        let result = cache_service.get_alignments(&cache_key).await.unwrap();
        assert!(result.is_some());
        
        let (retrieved_alignments, retrieved_quality, retrieved_stats) = result.unwrap();
        assert_eq!(retrieved_alignments.len(), 1);
        assert_eq!(retrieved_quality.overall_quality, 0.8);
        assert_eq!(retrieved_stats.total_sentences, 1);
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let config = AlignmentCacheConfig::default();
        let cache_service = AlignmentCacheService::new(config);
        
        let alignments = vec![create_test_alignment()];
        let quality = AlignmentQualityIndicator {
            overall_quality: 0.8,
            position_consistency: 0.9,
            length_ratio_consistency: 0.7,
            structural_coherence: 0.8,
            user_validation_rate: 0.0,
            problem_areas: Vec::new(),
        };
        let stats = AlignmentStatistics {
            total_sentences: 1,
            aligned_sentences: 1,
            validated_alignments: 0,
            average_confidence: 0.8,
            alignment_accuracy: 0.8,
            processing_time_ms: 100,
            language_pair: ("en".to_string(), "es".to_string()),
        };
        
        // Store multiple entries
        cache_service.store_alignments(
            "en:es:test1".to_string(),
            alignments.clone(),
            quality.clone(),
            stats.clone(),
        ).await.unwrap();
        
        cache_service.store_alignments(
            "en:es:test2".to_string(),
            alignments,
            quality,
            stats,
        ).await.unwrap();
        
        // Invalidate language pair
        let removed_count = cache_service.invalidate_language_pair("en", "es").await.unwrap();
        assert_eq!(removed_count, 2);
        
        // Verify entries are gone
        let result1 = cache_service.get_alignments("en:es:test1").await.unwrap();
        let result2 = cache_service.get_alignments("en:es:test2").await.unwrap();
        assert!(result1.is_none());
        assert!(result2.is_none());
    }

    #[tokio::test]
    async fn test_cache_statistics() {
        let config = AlignmentCacheConfig::default();
        let cache_service = AlignmentCacheService::new(config);
        
        let alignments = vec![create_test_alignment()];
        let quality = AlignmentQualityIndicator {
            overall_quality: 0.8,
            position_consistency: 0.9,
            length_ratio_consistency: 0.7,
            structural_coherence: 0.8,
            user_validation_rate: 0.0,
            problem_areas: Vec::new(),
        };
        let stats = AlignmentStatistics {
            total_sentences: 1,
            aligned_sentences: 1,
            validated_alignments: 0,
            average_confidence: 0.8,
            alignment_accuracy: 0.8,
            processing_time_ms: 100,
            language_pair: ("en".to_string(), "es".to_string()),
        };
        
        // Store and retrieve to generate statistics
        cache_service.store_alignments(
            "test_key".to_string(),
            alignments,
            quality,
            stats,
        ).await.unwrap();
        
        let _ = cache_service.get_alignments("test_key").await.unwrap();
        let _ = cache_service.get_alignments("non_existent_key").await.unwrap();
        
        let cache_stats = cache_service.get_statistics().await;
        assert_eq!(cache_stats.total_entries, 1);
        assert!(cache_stats.hit_rate > 0.0);
        assert!(cache_stats.miss_rate > 0.0);
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let config = AlignmentCacheConfig::default();
        let cache_service = AlignmentCacheService::new(config);
        
        let key1 = cache_service.generate_cache_key(
            "Hello world",
            "Hola mundo",
            "en",
            "es",
            12345,
        );
        
        let key2 = cache_service.generate_cache_key(
            "Hello world",
            "Hola mundo",
            "en",
            "es",
            12345,
        );
        
        let key3 = cache_service.generate_cache_key(
            "Different text",
            "Texto diferente",
            "en",
            "es",
            12345,
        );
        
        assert_eq!(key1, key2); // Same inputs should generate same key
        assert_ne!(key1, key3); // Different inputs should generate different keys
        assert!(key1.starts_with("en:es:"));
    }
}