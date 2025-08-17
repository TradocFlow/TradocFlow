use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tokio::time::{Duration, Instant};
use crate::Result;
use crate::services::{
    sentence_alignment_service::{
        SentenceAlignmentService, SentenceAlignment, AlignmentConfig, 
        AlignmentQualityIndicator, AlignmentStatistics, SentenceBoundary,
        AlignmentCorrection
    },
    text_structure_analyzer::{
        TextStructureAnalyzer, StructureAnalysisConfig, StructureAnalysisResult,
        TextStructure, TextStructureType
    },
    alignment_cache_service::{
        AlignmentCacheService, AlignmentCacheConfig
    }
};

/// Configuration for multi-pane alignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiPaneAlignmentConfig {
    pub max_panes: u8,
    pub default_source_language: String,
    pub supported_languages: Vec<String>,
    pub enable_real_time_sync: bool,
    pub sync_delay_ms: u64,
    pub enable_quality_monitoring: bool,
    pub auto_validation_threshold: f64,
    pub structure_analysis_config: StructureAnalysisConfig,
    pub alignment_config: AlignmentConfig,
    pub cache_config: AlignmentCacheConfig,
}

impl Default for MultiPaneAlignmentConfig {
    fn default() -> Self {
        Self {
            max_panes: 4,
            default_source_language: "en".to_string(),
            supported_languages: vec![
                "en".to_string(), "es".to_string(), "fr".to_string(), 
                "de".to_string(), "it".to_string(), "pt".to_string()
            ],
            enable_real_time_sync: true,
            sync_delay_ms: 100,
            enable_quality_monitoring: true,
            auto_validation_threshold: 0.85,
            structure_analysis_config: StructureAnalysisConfig::default(),
            alignment_config: AlignmentConfig::default(),
            cache_config: AlignmentCacheConfig::default(),
        }
    }
}

/// Represents a text pane in the multi-language editor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPane {
    pub id: Uuid,
    pub language: String,
    pub content: String,
    pub cursor_position: usize,
    pub selection_range: Option<(usize, usize)>,
    #[serde(skip)]
    pub last_modified: Instant,
    pub is_source: bool,
    pub structure_analysis: Option<StructureAnalysisResult>,
}

/// Synchronization event between panes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEvent {
    pub event_id: Uuid,
    pub source_pane_id: Uuid,
    pub event_type: SyncEventType,
    #[serde(skip)]
    pub timestamp: Instant,
    pub affected_panes: Vec<Uuid>,
}

/// Types of synchronization events
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SyncEventType {
    CursorMove { position: usize },
    TextChange { start: usize, end: usize, new_text: String },
    Selection { start: usize, end: usize },
    ScrollSync { offset: usize },
    StructureChange,
    QualityAlert { issue_type: String, severity: f64 },
}

/// Real-time synchronization state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub synchronized_positions: HashMap<Uuid, usize>, // pane_id -> cursor position
    pub synchronized_selections: HashMap<Uuid, (usize, usize)>, // pane_id -> selection range
    #[serde(skip)]
    pub last_sync_time: Instant,
    pub sync_quality: f64,
    pub pending_syncs: Vec<SyncEvent>,
}

/// Quality monitoring results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMonitoringResult {
    pub overall_quality: f64,
    pub pane_qualities: HashMap<Uuid, f64>, // pane_id -> quality score
    pub alignment_qualities: HashMap<String, AlignmentQualityIndicator>, // language_pair -> quality
    pub issues: Vec<QualityIssue>,
    pub recommendations: Vec<QualityRecommendation>,
}

/// Quality issues detected during monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIssue {
    pub issue_type: QualityIssueType,
    pub severity: QualityIssueSeverity,
    pub description: String,
    pub affected_panes: Vec<Uuid>,
    pub suggested_actions: Vec<String>,
    pub auto_fixable: bool,
}

/// Types of quality issues
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QualityIssueType {
    AlignmentMismatch,
    StructuralInconsistency,
    LengthRatioOutlier,
    MissingTranslation,
    InconsistentTerminology,
    FormattingIssue,
    SynchronizationError,
}

/// Severity levels for quality issues
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QualityIssueSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Quality improvement recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityRecommendation {
    pub recommendation_type: RecommendationType,
    pub priority: u8,
    pub description: String,
    pub estimated_impact: f64,
    pub implementation_effort: ImplementationEffort,
}

/// Types of quality recommendations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecommendationType {
    ImproveAlignment,
    ReviewTranslation,
    FixStructure,
    UpdateTerminology,
    OptimizePerformance,
    ConfigureSettings,
}

/// Implementation effort levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImplementationEffort {
    Minimal,
    Low,
    Medium,
    High,
    Extensive,
}

/// Performance metrics for the alignment system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub alignment_time_ms: f64,
    pub sync_time_ms: f64,
    pub cache_hit_rate: f64,
    pub memory_usage_mb: f64,
    pub processing_rate_chars_per_sec: f64,
    pub error_rate: f64,
    pub quality_score: f64,
}

/// Main service for multi-pane sentence alignment
pub struct MultiPaneAlignmentService {
    config: MultiPaneAlignmentConfig,
    sentence_alignment_service: Arc<SentenceAlignmentService>,
    text_structure_analyzer: Arc<TextStructureAnalyzer>,
    cache_service: Arc<AlignmentCacheService>,
    active_panes: Arc<RwLock<HashMap<Uuid, TextPane>>>,
    alignments_cache: Arc<RwLock<HashMap<String, Vec<SentenceAlignment>>>>,
    sync_state: Arc<RwLock<SyncState>>,
    quality_monitor: Arc<RwLock<QualityMonitoringResult>>,
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
}

impl MultiPaneAlignmentService {
    /// Create a new multi-pane alignment service
    pub fn new(config: MultiPaneAlignmentConfig) -> Result<Self> {
        let sentence_alignment_service = Arc::new(
            SentenceAlignmentService::new(config.alignment_config.clone())
        );
        
        let text_structure_analyzer = Arc::new(
            TextStructureAnalyzer::new(config.structure_analysis_config.clone())?
        );
        
        let cache_service = Arc::new(
            AlignmentCacheService::new(config.cache_config.clone())
        );

        Ok(Self {
            config,
            sentence_alignment_service,
            text_structure_analyzer,
            cache_service,
            active_panes: Arc::new(RwLock::new(HashMap::new())),
            alignments_cache: Arc::new(RwLock::new(HashMap::new())),
            sync_state: Arc::new(RwLock::new(SyncState {
                synchronized_positions: HashMap::new(),
                synchronized_selections: HashMap::new(),
                last_sync_time: Instant::now(),
                sync_quality: 1.0,
                pending_syncs: Vec::new(),
            })),
            quality_monitor: Arc::new(RwLock::new(QualityMonitoringResult {
                overall_quality: 1.0,
                pane_qualities: HashMap::new(),
                alignment_qualities: HashMap::new(),
                issues: Vec::new(),
                recommendations: Vec::new(),
            })),
            performance_metrics: Arc::new(RwLock::new(PerformanceMetrics {
                alignment_time_ms: 0.0,
                sync_time_ms: 0.0,
                cache_hit_rate: 0.0,
                memory_usage_mb: 0.0,
                processing_rate_chars_per_sec: 0.0,
                error_rate: 0.0,
                quality_score: 1.0,
            })),
        })
    }

    /// Add a new text pane
    pub async fn add_pane(
        &self,
        language: String,
        content: String,
        is_source: bool,
    ) -> Result<Uuid> {
        // Check if we're at the maximum number of panes
        {
            let panes = self.active_panes.read().unwrap();
            if panes.len() >= self.config.max_panes as usize {
                return Err(crate::TradocumentError::Validation(
                    format!("Maximum number of panes ({}) exceeded", self.config.max_panes)
                ));
            }
        }

        // Validate language support
        if !self.config.supported_languages.contains(&language) {
            return Err(crate::TradocumentError::UnsupportedLanguage(language));
        }

        let pane_id = Uuid::new_v4();
        
        // Analyze text structure
        let structure_analysis = if self.config.structure_analysis_config.analyze_language_features {
            Some(self.text_structure_analyzer.analyze_structure(&content, Some(&language)).await?)
        } else {
            None
        };

        let pane = TextPane {
            id: pane_id,
            language,
            content,
            cursor_position: 0,
            selection_range: None,
            last_modified: Instant::now(),
            is_source,
            structure_analysis,
        };

        // Add to active panes
        {
            let mut panes = self.active_panes.write().unwrap();
            panes.insert(pane_id, pane);
        }

        // If this is not the first pane, create alignments with existing panes
        if self.get_pane_count().await > 1 {
            self.update_alignments_for_new_pane(pane_id).await?;
        }

        // Initialize sync state for the new pane
        {
            let mut sync_state = self.sync_state.write().unwrap();
            sync_state.synchronized_positions.insert(pane_id, 0);
        }

        Ok(pane_id)
    }

    /// Remove a text pane
    pub async fn remove_pane(&self, pane_id: Uuid) -> Result<()> {
        // Remove from active panes
        let removed_pane = {
            let mut panes = self.active_panes.write().unwrap();
            panes.remove(&pane_id)
        };

        if removed_pane.is_none() {
            return Err(crate::TradocumentError::Validation("Pane not found".to_string()));
        }

        // Clean up sync state
        {
            let mut sync_state = self.sync_state.write().unwrap();
            sync_state.synchronized_positions.remove(&pane_id);
            sync_state.synchronized_selections.remove(&pane_id);
        }

        // Clean up quality monitoring
        {
            let mut quality_monitor = self.quality_monitor.write().unwrap();
            quality_monitor.pane_qualities.remove(&pane_id);
        }

        // Invalidate related alignments in cache
        if let Some(pane) = removed_pane {
            self.invalidate_alignments_for_language(&pane.language).await?;
        }

        Ok(())
    }

    /// Update content of a pane
    pub async fn update_pane_content(
        &self,
        pane_id: Uuid,
        new_content: String,
        cursor_position: Option<usize>,
    ) -> Result<()> {
        let start_time = Instant::now();

        // Update the pane
        {
            let mut panes = self.active_panes.write().unwrap();
            if let Some(pane) = panes.get_mut(&pane_id) {
                pane.content = new_content;
                pane.last_modified = Instant::now();
                
                if let Some(pos) = cursor_position {
                    pane.cursor_position = pos;
                }

                // Re-analyze structure if content changed significantly
                if self.config.structure_analysis_config.analyze_language_features {
                    let structure_analysis = self.text_structure_analyzer
                        .analyze_structure(&pane.content, Some(&pane.language)).await?;
                    pane.structure_analysis = Some(structure_analysis);
                }
            } else {
                return Err(crate::TradocumentError::Validation("Pane not found".to_string()));
            }
        }

        // Update alignments with other panes
        self.update_alignments_for_pane(pane_id).await?;

        // Trigger real-time synchronization if enabled
        if self.config.enable_real_time_sync {
            self.synchronize_panes(pane_id, cursor_position).await?;
        }

        // Update performance metrics
        {
            let mut metrics = self.performance_metrics.write().unwrap();
            metrics.alignment_time_ms = start_time.elapsed().as_millis() as f64;
        }

        Ok(())
    }

    /// Synchronize cursor positions across all panes
    pub async fn synchronize_cursor_position(
        &self,
        source_pane_id: Uuid,
        cursor_position: usize,
    ) -> Result<HashMap<Uuid, usize>> {
        let start_time = Instant::now();

        // Get all pane contents for synchronization
        let pane_contents = self.get_all_pane_contents().await;
        
        // Get source language
        let source_language = {
            let panes = self.active_panes.read().unwrap();
            panes.get(&source_pane_id)
                .map(|p| p.language.clone())
                .ok_or_else(|| crate::TradocumentError::Validation("Source pane not found".to_string()))?
        };

        // Use sentence alignment service to synchronize positions
        let synchronized_positions = self.sentence_alignment_service
            .synchronize_sentence_boundaries(&pane_contents, cursor_position, &source_language)
            .await?;

        // Convert language-based positions to pane-based positions
        let mut pane_positions = HashMap::new();
        
        {
            let panes = self.active_panes.read().unwrap();
            for (pane_id, pane) in panes.iter() {
                if let Some(&position) = synchronized_positions.get(&pane.language) {
                    pane_positions.insert(*pane_id, position);
                }
            }
        }

        // Update sync state
        {
            let mut sync_state = self.sync_state.write().unwrap();
            sync_state.synchronized_positions = pane_positions.clone();
            sync_state.last_sync_time = Instant::now();
            sync_state.sync_quality = self.calculate_sync_quality(&pane_positions).await;
        }

        // Update performance metrics
        {
            let mut metrics = self.performance_metrics.write().unwrap();
            metrics.sync_time_ms = start_time.elapsed().as_millis() as f64;
        }

        // Create sync event
        let sync_event = SyncEvent {
            event_id: Uuid::new_v4(),
            source_pane_id,
            event_type: SyncEventType::CursorMove { position: cursor_position },
            timestamp: Instant::now(),
            affected_panes: pane_positions.keys().cloned().collect(),
        };

        self.record_sync_event(sync_event).await;

        Ok(pane_positions)
    }

    /// Get real-time alignment quality indicators
    pub async fn get_real_time_quality_indicators(&self) -> Result<HashMap<String, AlignmentQualityIndicator>> {
        let mut quality_indicators = HashMap::new();

        // Get all pane combinations for quality analysis
        let panes = {
            let panes_guard = self.active_panes.read().unwrap();
            panes_guard.values().cloned().collect::<Vec<_>>()
        };

        // Calculate quality indicators for each language pair
        for i in 0..panes.len() {
            for j in i + 1..panes.len() {
                let source_pane = &panes[i];
                let target_pane = &panes[j];
                
                let language_pair = format!("{}:{}", source_pane.language, target_pane.language);
                
                // Get or create alignments
                let alignments = self.get_alignments_between_panes(
                    source_pane.id,
                    target_pane.id,
                ).await?;

                // Calculate quality indicators
                let quality_indicator = self.sentence_alignment_service
                    .calculate_quality_indicators(&alignments).await?;

                quality_indicators.insert(language_pair, quality_indicator);
            }
        }

        // Update quality monitor
        {
            let mut quality_monitor = self.quality_monitor.write().unwrap();
            quality_monitor.alignment_qualities = quality_indicators.clone();
            quality_monitor.overall_quality = self.calculate_overall_quality(&quality_indicators);
        }

        Ok(quality_indicators)
    }

    /// Learn from user corrections to improve alignment quality
    pub async fn learn_from_user_correction(
        &self,
        source_pane_id: Uuid,
        target_pane_id: Uuid,
        original_alignment: SentenceAlignment,
        corrected_alignment: SentenceAlignment,
        correction_reason: String,
    ) -> Result<()> {
        // Pass the correction to the sentence alignment service for learning
        self.sentence_alignment_service.learn_from_correction(
            original_alignment,
            corrected_alignment,
            correction_reason,
        ).await?;

        // Invalidate related cache entries
        let (source_language, target_language) = {
            let panes = self.active_panes.read().unwrap();
            let source_lang = panes.get(&source_pane_id)
                .map(|p| p.language.clone())
                .unwrap_or_default();
            let target_lang = panes.get(&target_pane_id)
                .map(|p| p.language.clone())
                .unwrap_or_default();
            (source_lang, target_lang)
        };

        self.cache_service.invalidate_language_pair(&source_language, &target_language).await?;

        // Re-calculate alignments for affected panes
        self.update_alignments_between_panes(source_pane_id, target_pane_id).await?;

        Ok(())
    }

    /// Get performance metrics
    pub async fn get_performance_metrics(&self) -> PerformanceMetrics {
        // Update current metrics
        self.update_performance_metrics().await;
        
        let metrics = self.performance_metrics.read().unwrap();
        metrics.clone()
    }

    /// Perform quality monitoring and generate recommendations
    pub async fn perform_quality_monitoring(&self) -> Result<QualityMonitoringResult> {
        let start_time = Instant::now();

        // Get current quality indicators
        let quality_indicators = self.get_real_time_quality_indicators().await?;

        // Identify quality issues
        let issues = self.identify_quality_issues(&quality_indicators).await;

        // Generate recommendations
        let recommendations = self.generate_quality_recommendations(&issues).await;

        // Calculate pane-specific quality scores
        let pane_qualities = self.calculate_pane_qualities().await;

        // Calculate overall quality score
        let overall_quality = self.calculate_overall_quality(&quality_indicators);

        let monitoring_result = QualityMonitoringResult {
            overall_quality,
            pane_qualities,
            alignment_qualities: quality_indicators,
            issues,
            recommendations,
        };

        // Update quality monitor
        {
            let mut quality_monitor = self.quality_monitor.write().unwrap();
            *quality_monitor = monitoring_result.clone();
        }

        // Update performance metrics
        {
            let mut metrics = self.performance_metrics.write().unwrap();
            metrics.quality_score = overall_quality;
        }

        Ok(monitoring_result)
    }

    /// Get current sync state
    pub async fn get_sync_state(&self) -> SyncState {
        let sync_state = self.sync_state.read().unwrap();
        sync_state.clone()
    }

    /// Get all active panes
    pub async fn get_active_panes(&self) -> HashMap<Uuid, TextPane> {
        let panes = self.active_panes.read().unwrap();
        panes.clone()
    }

    /// Get alignment statistics for a language pair
    pub async fn get_alignment_statistics(&self, source_language: &str, target_language: &str) -> Result<Option<AlignmentStatistics>> {
        self.sentence_alignment_service
            .get_alignment_statistics((source_language.to_string(), target_language.to_string()))
            .await
    }

    // Private helper methods

    async fn get_pane_count(&self) -> usize {
        let panes = self.active_panes.read().unwrap();
        panes.len()
    }

    async fn get_all_pane_contents(&self) -> HashMap<String, String> {
        let panes = self.active_panes.read().unwrap();
        panes.values()
            .map(|pane| (pane.language.clone(), pane.content.clone()))
            .collect()
    }

    async fn update_alignments_for_new_pane(&self, new_pane_id: Uuid) -> Result<()> {
        // Get the new pane
        let new_pane = {
            let panes = self.active_panes.read().unwrap();
            panes.get(&new_pane_id).cloned()
        }.ok_or_else(|| crate::TradocumentError::Validation("Pane not found".to_string()))?;

        // Create alignments with all existing panes
        let existing_panes: Vec<TextPane> = {
            let panes = self.active_panes.read().unwrap();
            panes.values()
                .filter(|p| p.id != new_pane_id)
                .cloned()
                .collect()
        };

        for existing_pane in existing_panes {
            self.create_alignments_between_panes(&new_pane, &existing_pane).await?;
        }

        Ok(())
    }

    async fn update_alignments_for_pane(&self, pane_id: Uuid) -> Result<()> {
        // Get the updated pane
        let updated_pane = {
            let panes = self.active_panes.read().unwrap();
            panes.get(&pane_id).cloned()
        }.ok_or_else(|| crate::TradocumentError::Validation("Pane not found".to_string()))?;

        // Update alignments with all other panes
        let other_panes: Vec<TextPane> = {
            let panes = self.active_panes.read().unwrap();
            panes.values()
                .filter(|p| p.id != pane_id)
                .cloned()
                .collect()
        };

        for other_pane in other_panes {
            self.create_alignments_between_panes(&updated_pane, &other_pane).await?;
        }

        Ok(())
    }

    async fn update_alignments_between_panes(&self, pane1_id: Uuid, pane2_id: Uuid) -> Result<()> {
        let (pane1, pane2) = {
            let panes = self.active_panes.read().unwrap();
            let p1 = panes.get(&pane1_id).cloned()
                .ok_or_else(|| crate::TradocumentError::Validation("Pane 1 not found".to_string()))?;
            let p2 = panes.get(&pane2_id).cloned()
                .ok_or_else(|| crate::TradocumentError::Validation("Pane 2 not found".to_string()))?;
            (p1, p2)
        };

        self.create_alignments_between_panes(&pane1, &pane2).await?;
        Ok(())
    }

    async fn create_alignments_between_panes(&self, pane1: &TextPane, pane2: &TextPane) -> Result<()> {
        // Generate cache key
        let cache_key = self.cache_service.generate_cache_key(
            &pane1.content,
            &pane2.content,
            &pane1.language,
            &pane2.language,
            self.calculate_config_hash(),
        );

        // Check cache first
        if let Some((cached_alignments, quality, stats)) = 
            self.cache_service.get_alignments(&cache_key).await? {
            
            // Store in local cache
            let alignment_key = format!("{}:{}", pane1.id, pane2.id);
            {
                let mut alignments_cache = self.alignments_cache.write().unwrap();
                alignments_cache.insert(alignment_key, cached_alignments);
            }
            
            return Ok(());
        }

        // Create new alignments
        let alignments = self.sentence_alignment_service.align_sentences(
            &pane1.content,
            &pane2.content,
            &pane1.language,
            &pane2.language,
        ).await?;

        // Calculate quality indicators
        let quality_indicator = self.sentence_alignment_service
            .calculate_quality_indicators(&alignments).await?;

        // Get statistics
        let statistics = self.sentence_alignment_service
            .get_alignment_statistics((pane1.language.clone(), pane2.language.clone()))
            .await?
            .unwrap_or_else(|| AlignmentStatistics {
                total_sentences: alignments.len(),
                aligned_sentences: alignments.len(),
                validated_alignments: 0,
                average_confidence: alignments.iter().map(|a| a.alignment_confidence).sum::<f64>() / alignments.len() as f64,
                alignment_accuracy: 0.8,
                processing_time_ms: 0,
                language_pair: (pane1.language.clone(), pane2.language.clone()),
            });

        // Store in cache
        self.cache_service.store_alignments(
            cache_key,
            alignments.clone(),
            quality_indicator,
            statistics,
        ).await?;

        // Store in local cache
        let alignment_key = format!("{}:{}", pane1.id, pane2.id);
        {
            let mut alignments_cache = self.alignments_cache.write().unwrap();
            alignments_cache.insert(alignment_key, alignments);
        }

        Ok(())
    }

    async fn get_alignments_between_panes(&self, pane1_id: Uuid, pane2_id: Uuid) -> Result<Vec<SentenceAlignment>> {
        let alignment_key = format!("{}:{}", pane1_id, pane2_id);
        let reverse_key = format!("{}:{}", pane2_id, pane1_id);

        let alignments_cache = self.alignments_cache.read().unwrap();
        
        if let Some(alignments) = alignments_cache.get(&alignment_key) {
            Ok(alignments.clone())
        } else if let Some(alignments) = alignments_cache.get(&reverse_key) {
            // Return reversed alignments
            Ok(alignments.iter().map(|a| {
                let mut reversed = a.clone();
                std::mem::swap(&mut reversed.source_sentence, &mut reversed.target_sentence);
                std::mem::swap(&mut reversed.source_language, &mut reversed.target_language);
                reversed
            }).collect())
        } else {
            // Force update alignments
            self.update_alignments_between_panes(pane1_id, pane2_id).await?;
            
            let alignments_cache = self.alignments_cache.read().unwrap();
            Ok(alignments_cache.get(&alignment_key).cloned().unwrap_or_default())
        }
    }

    async fn synchronize_panes(&self, source_pane_id: Uuid, cursor_position: Option<usize>) -> Result<()> {
        if let Some(pos) = cursor_position {
            self.synchronize_cursor_position(source_pane_id, pos).await?;
        }

        // Add delay if configured
        if self.config.sync_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.config.sync_delay_ms)).await;
        }

        Ok(())
    }

    async fn calculate_sync_quality(&self, synchronized_positions: &HashMap<Uuid, usize>) -> f64 {
        // Simple quality calculation based on position consistency
        if synchronized_positions.len() < 2 {
            return 1.0;
        }

        let positions: Vec<usize> = synchronized_positions.values().cloned().collect();
        let mean_position = positions.iter().sum::<usize>() as f64 / positions.len() as f64;
        
        let variance = positions.iter()
            .map(|&pos| (pos as f64 - mean_position).powi(2))
            .sum::<f64>() / positions.len() as f64;
        
        // Convert variance to quality score (lower variance = higher quality)
        1.0 - (variance.sqrt() / 1000.0).min(1.0)
    }

    async fn record_sync_event(&self, event: SyncEvent) {
        let mut sync_state = self.sync_state.write().unwrap();
        sync_state.pending_syncs.push(event);
        
        // Keep only recent events
        if sync_state.pending_syncs.len() > 100 {
            sync_state.pending_syncs.drain(0..50);
        }
    }

    async fn invalidate_alignments_for_language(&self, language: &str) -> Result<()> {
        // Invalidate cache entries for this language
        let panes = self.active_panes.read().unwrap();
        for pane in panes.values() {
            if pane.language != language {
                self.cache_service.invalidate_language_pair(language, &pane.language).await?;
                self.cache_service.invalidate_language_pair(&pane.language, language).await?;
            }
        }

        // Clear local alignments cache
        {
            let mut alignments_cache = self.alignments_cache.write().unwrap();
            let keys_to_remove: Vec<String> = alignments_cache.keys()
                .filter(|key| {
                    let parts: Vec<&str> = key.split(':').collect();
                    parts.len() == 2 && {
                        let pane1_id = Uuid::parse_str(parts[0]).ok();
                        let pane2_id = Uuid::parse_str(parts[1]).ok();
                        
                        if let (Some(p1_id), Some(p2_id)) = (pane1_id, pane2_id) {
                            panes.get(&p1_id).map_or(false, |p| p.language == language) ||
                            panes.get(&p2_id).map_or(false, |p| p.language == language)
                        } else {
                            false
                        }
                    }
                })
                .cloned()
                .collect();

            for key in keys_to_remove {
                alignments_cache.remove(&key);
            }
        }

        Ok(())
    }

    async fn identify_quality_issues(&self, quality_indicators: &HashMap<String, AlignmentQualityIndicator>) -> Vec<QualityIssue> {
        let mut issues = Vec::new();

        for (language_pair, indicator) in quality_indicators {
            // Check overall quality
            if indicator.overall_quality < 0.6 {
                issues.push(QualityIssue {
                    issue_type: QualityIssueType::AlignmentMismatch,
                    severity: if indicator.overall_quality < 0.4 {
                        QualityIssueSeverity::Critical
                    } else {
                        QualityIssueSeverity::High
                    },
                    description: format!("Low alignment quality for language pair: {}", language_pair),
                    affected_panes: Vec::new(), // Would be populated with actual pane IDs
                    suggested_actions: vec![
                        "Review sentence boundaries".to_string(),
                        "Check for structural differences".to_string(),
                    ],
                    auto_fixable: false,
                });
            }

            // Check position consistency
            if indicator.position_consistency < 0.7 {
                issues.push(QualityIssue {
                    issue_type: QualityIssueType::StructuralInconsistency,
                    severity: QualityIssueSeverity::Medium,
                    description: format!("Poor position consistency for language pair: {}", language_pair),
                    affected_panes: Vec::new(),
                    suggested_actions: vec![
                        "Review document structure".to_string(),
                        "Check for missing or extra sentences".to_string(),
                    ],
                    auto_fixable: false,
                });
            }

            // Check length ratio consistency
            if indicator.length_ratio_consistency < 0.6 {
                issues.push(QualityIssue {
                    issue_type: QualityIssueType::LengthRatioOutlier,
                    severity: QualityIssueSeverity::Medium,
                    description: format!("Inconsistent length ratios for language pair: {}", language_pair),
                    affected_panes: Vec::new(),
                    suggested_actions: vec![
                        "Review translation completeness".to_string(),
                        "Check for overly long or short translations".to_string(),
                    ],
                    auto_fixable: false,
                });
            }
        }

        issues
    }

    async fn generate_quality_recommendations(&self, issues: &[QualityIssue]) -> Vec<QualityRecommendation> {
        let mut recommendations = Vec::new();

        for issue in issues {
            match issue.issue_type {
                QualityIssueType::AlignmentMismatch => {
                    recommendations.push(QualityRecommendation {
                        recommendation_type: RecommendationType::ImproveAlignment,
                        priority: match issue.severity {
                            QualityIssueSeverity::Critical => 1,
                            QualityIssueSeverity::High => 2,
                            _ => 3,
                        },
                        description: "Improve sentence alignment by reviewing boundary detection settings".to_string(),
                        estimated_impact: 0.8,
                        implementation_effort: ImplementationEffort::Medium,
                    });
                },
                QualityIssueType::StructuralInconsistency => {
                    recommendations.push(QualityRecommendation {
                        recommendation_type: RecommendationType::FixStructure,
                        priority: 2,
                        description: "Review document structure and ensure consistent formatting".to_string(),
                        estimated_impact: 0.6,
                        implementation_effort: ImplementationEffort::High,
                    });
                },
                QualityIssueType::LengthRatioOutlier => {
                    recommendations.push(QualityRecommendation {
                        recommendation_type: RecommendationType::ReviewTranslation,
                        priority: 3,
                        description: "Review translations for completeness and accuracy".to_string(),
                        estimated_impact: 0.5,
                        implementation_effort: ImplementationEffort::High,
                    });
                },
                _ => {}
            }
        }

        // Sort by priority
        recommendations.sort_by_key(|r| r.priority);
        recommendations
    }

    async fn calculate_pane_qualities(&self) -> HashMap<Uuid, f64> {
        let mut pane_qualities = HashMap::new();
        
        let panes = {
            let panes_guard = self.active_panes.read().unwrap();
            panes_guard.clone()
        };

        for (pane_id, pane) in &panes {
            let mut quality_sum = 0.0;
            let mut quality_count = 0;

            // Calculate quality based on alignments with other panes
            for (other_pane_id, other_pane) in &panes {
                if pane_id != other_pane_id {
                    if let Ok(alignments) = self.get_alignments_between_panes(*pane_id, *other_pane_id).await {
                        if !alignments.is_empty() {
                            let avg_confidence = alignments.iter()
                                .map(|a| a.alignment_confidence)
                                .sum::<f64>() / alignments.len() as f64;
                            quality_sum += avg_confidence;
                            quality_count += 1;
                        }
                    }
                }
            }

            let quality = if quality_count > 0 {
                quality_sum / quality_count as f64
            } else {
                1.0 // Default quality for single pane
            };

            pane_qualities.insert(*pane_id, quality);
        }

        pane_qualities
    }

    fn calculate_overall_quality(&self, quality_indicators: &HashMap<String, AlignmentQualityIndicator>) -> f64 {
        if quality_indicators.is_empty() {
            return 1.0;
        }

        let total_quality = quality_indicators.values()
            .map(|indicator| indicator.overall_quality)
            .sum::<f64>();

        total_quality / quality_indicators.len() as f64
    }

    async fn update_performance_metrics(&self) {
        let cache_stats = self.cache_service.get_statistics().await;
        
        let mut metrics = self.performance_metrics.write().unwrap();
        metrics.cache_hit_rate = cache_stats.hit_rate;
        metrics.memory_usage_mb = cache_stats.cache_size_bytes as f64 / (1024.0 * 1024.0);
        metrics.error_rate = 0.0; // Would be calculated based on actual errors
        
        // Calculate processing rate based on recent operations
        let panes = self.active_panes.read().unwrap();
        let total_chars: usize = panes.values().map(|p| p.content.len()).sum();
        
        if metrics.alignment_time_ms > 0.0 {
            metrics.processing_rate_chars_per_sec = (total_chars as f64) / (metrics.alignment_time_ms / 1000.0);
        }
    }

    fn calculate_config_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        
        // Hash relevant config parameters
        self.config.alignment_config.confidence_threshold.to_bits().hash(&mut hasher);
        self.config.alignment_config.position_weight.to_bits().hash(&mut hasher);
        self.config.alignment_config.length_weight.to_bits().hash(&mut hasher);
        self.config.alignment_config.structure_weight.to_bits().hash(&mut hasher);
        
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_multi_pane_service_creation() {
        let config = MultiPaneAlignmentConfig::default();
        let service = MultiPaneAlignmentService::new(config).unwrap();
        
        let panes = service.get_active_panes().await;
        assert!(panes.is_empty());
    }

    #[tokio::test]
    async fn test_add_and_remove_panes() {
        let config = MultiPaneAlignmentConfig::default();
        let service = MultiPaneAlignmentService::new(config).unwrap();
        
        // Add first pane
        let pane1_id = service.add_pane(
            "en".to_string(),
            "Hello world. How are you?".to_string(),
            true,
        ).await.unwrap();
        
        // Add second pane
        let pane2_id = service.add_pane(
            "es".to_string(),
            "Hola mundo. ¿Cómo estás?".to_string(),
            false,
        ).await.unwrap();
        
        let panes = service.get_active_panes().await;
        assert_eq!(panes.len(), 2);
        
        // Remove a pane
        service.remove_pane(pane1_id).await.unwrap();
        
        let panes = service.get_active_panes().await;
        assert_eq!(panes.len(), 1);
        assert!(panes.contains_key(&pane2_id));
    }

    #[tokio::test]
    async fn test_cursor_synchronization() {
        let config = MultiPaneAlignmentConfig::default();
        let service = MultiPaneAlignmentService::new(config).unwrap();
        
        // Add panes
        let pane1_id = service.add_pane(
            "en".to_string(),
            "First sentence. Second sentence.".to_string(),
            true,
        ).await.unwrap();
        
        let pane2_id = service.add_pane(
            "es".to_string(),
            "Primera oración. Segunda oración.".to_string(),
            false,
        ).await.unwrap();
        
        // Synchronize cursor position
        let sync_positions = service.synchronize_cursor_position(pane1_id, 20).await.unwrap();
        
        assert!(sync_positions.contains_key(&pane1_id));
        assert!(sync_positions.contains_key(&pane2_id));
    }

    #[tokio::test]
    async fn test_quality_monitoring() {
        let config = MultiPaneAlignmentConfig::default();
        let service = MultiPaneAlignmentService::new(config).unwrap();
        
        // Add panes
        service.add_pane(
            "en".to_string(),
            "Test sentence for quality monitoring.".to_string(),
            true,
        ).await.unwrap();
        
        service.add_pane(
            "es".to_string(),
            "Oración de prueba para monitoreo de calidad.".to_string(),
            false,
        ).await.unwrap();
        
        // Perform quality monitoring
        let quality_result = service.perform_quality_monitoring().await.unwrap();
        
        assert!(quality_result.overall_quality >= 0.0);
        assert!(quality_result.overall_quality <= 1.0);
        assert!(!quality_result.alignment_qualities.is_empty());
    }
}