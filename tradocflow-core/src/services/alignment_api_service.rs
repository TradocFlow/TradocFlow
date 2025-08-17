use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tokio::time::Instant;
use crate::Result;
use crate::services::{
    multi_pane_alignment_service::{
        MultiPaneAlignmentService, MultiPaneAlignmentConfig, TextPane,
        SyncEvent, SyncState, QualityMonitoringResult, PerformanceMetrics as AlignmentPerformanceMetrics
    },
    sentence_alignment_service::{
        AlignmentStatistics, AlignmentQualityIndicator, SentenceAlignment
    }
};

/// Simplified API request for adding a pane
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPaneRequest {
    pub language: String,
    pub content: String,
    pub is_source: bool,
}

/// Simplified API response for adding a pane
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddPaneResponse {
    pub pane_id: Uuid,
    pub success: bool,
    pub message: String,
}

/// API request for updating pane content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePaneRequest {
    pub pane_id: Uuid,
    pub content: String,
    pub cursor_position: Option<usize>,
}

/// API response for updating pane content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePaneResponse {
    pub success: bool,
    pub message: String,
    pub synchronized_positions: Option<HashMap<Uuid, usize>>,
}

/// API request for cursor synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCursorRequest {
    pub source_pane_id: Uuid,
    pub cursor_position: usize,
}

/// API response for cursor synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCursorResponse {
    pub success: bool,
    pub synchronized_positions: HashMap<Uuid, usize>,
    pub sync_quality: f64,
}

/// API request for user corrections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCorrectionRequest {
    pub source_pane_id: Uuid,
    pub target_pane_id: Uuid,
    pub original_source_text: String,
    pub original_target_text: String,
    pub corrected_source_text: String,
    pub corrected_target_text: String,
    pub correction_reason: String,
}

/// API response for user corrections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCorrectionResponse {
    pub success: bool,
    pub message: String,
    pub learning_applied: bool,
}

/// Comprehensive status response for the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatusResponse {
    pub active_panes: HashMap<Uuid, PaneInfo>,
    pub sync_state: SyncStateInfo,
    pub quality_monitoring: QualityMonitoringInfo,
    pub performance_metrics: PerformanceMetricsInfo,
    pub system_health: SystemHealth,
}

/// Simplified pane information for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneInfo {
    pub id: Uuid,
    pub language: String,
    pub content_length: usize,
    pub cursor_position: usize,
    pub is_source: bool,
    pub last_modified: u64, // Unix timestamp
    pub quality_score: Option<f64>,
}

/// Simplified sync state for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStateInfo {
    pub is_synchronized: bool,
    pub last_sync_time: u64, // Unix timestamp
    pub sync_quality: f64,
    pub pending_events: usize,
}

/// Quality monitoring information for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMonitoringInfo {
    pub overall_quality: f64,
    pub total_issues: usize,
    pub critical_issues: usize,
    pub recommendations_count: usize,
    pub alignment_qualities: HashMap<String, f64>, // language_pair -> quality
}

/// Performance metrics for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetricsInfo {
    pub alignment_time_ms: f64,
    pub sync_time_ms: f64,
    pub cache_hit_rate: f64,
    pub memory_usage_mb: f64,
    pub processing_rate: f64,
    pub error_rate: f64,
}

/// System health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub status: HealthStatus,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub cache_efficiency: f64,
    pub error_count: u32,
    pub warnings: Vec<String>,
}

/// Health status levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Excellent,
    Good,
    Fair,
    Poor,
    Critical,
}

/// Real-time alignment update for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentUpdate {
    pub update_type: AlignmentUpdateType,
    pub affected_panes: Vec<Uuid>,
    pub quality_change: Option<f64>,
    pub timestamp: u64,
    pub data: AlignmentUpdateData,
}

/// Types of alignment updates
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlignmentUpdateType {
    QualityChange,
    SyncUpdate,
    NewAlignment,
    CorrectionApplied,
    PerformanceAlert,
}

/// Data payload for alignment updates
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum AlignmentUpdateData {
    QualityChange {
        old_quality: f64,
        new_quality: f64,
        language_pair: String,
    },
    SyncUpdate {
        synchronized_positions: HashMap<Uuid, usize>,
        sync_quality: f64,
    },
    NewAlignment {
        source_pane: Uuid,
        target_pane: Uuid,
        alignment_count: usize,
        average_confidence: f64,
    },
    CorrectionApplied {
        correction_count: u32,
        improvement_estimate: f64,
    },
    PerformanceAlert {
        metric_name: String,
        current_value: f64,
        threshold: f64,
        severity: String,
    },
}

/// Configuration for the API service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentApiConfig {
    pub enable_real_time_updates: bool,
    pub update_interval_ms: u64,
    pub max_concurrent_requests: usize,
    pub enable_performance_monitoring: bool,
    pub enable_detailed_logging: bool,
    pub quality_threshold_warning: f64,
    pub quality_threshold_critical: f64,
}

impl Default for AlignmentApiConfig {
    fn default() -> Self {
        Self {
            enable_real_time_updates: true,
            update_interval_ms: 500,
            max_concurrent_requests: 100,
            enable_performance_monitoring: true,
            enable_detailed_logging: false,
            quality_threshold_warning: 0.7,
            quality_threshold_critical: 0.5,
        }
    }
}

/// High-level API service for UI integration
pub struct AlignmentApiService {
    alignment_service: Arc<MultiPaneAlignmentService>,
    config: AlignmentApiConfig,
    update_subscribers: Arc<std::sync::RwLock<Vec<tokio::sync::mpsc::UnboundedSender<AlignmentUpdate>>>>,
    request_counter: Arc<std::sync::atomic::AtomicU64>,
    error_counter: Arc<std::sync::atomic::AtomicU32>,
}

impl AlignmentApiService {
    /// Create a new alignment API service
    pub fn new(
        multi_pane_config: MultiPaneAlignmentConfig,
        api_config: AlignmentApiConfig,
    ) -> Result<Self> {
        let alignment_service = Arc::new(
            MultiPaneAlignmentService::new(multi_pane_config)?
        );

        Ok(Self {
            alignment_service,
            config: api_config,
            update_subscribers: Arc::new(std::sync::RwLock::new(Vec::new())),
            request_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            error_counter: Arc::new(std::sync::atomic::AtomicU32::new(0)),
        })
    }

    /// Add a new text pane
    pub async fn add_pane(&self, request: AddPaneRequest) -> Result<AddPaneResponse> {
        self.increment_request_counter();

        match self.alignment_service.add_pane(
            request.language.clone(),
            request.content,
            request.is_source,
        ).await {
            Ok(pane_id) => {
                // Send real-time update
                if self.config.enable_real_time_updates {
                    self.send_update(AlignmentUpdate {
                        update_type: AlignmentUpdateType::NewAlignment,
                        affected_panes: vec![pane_id],
                        quality_change: None,
                        timestamp: self.current_timestamp(),
                        data: AlignmentUpdateData::NewAlignment {
                            source_pane: pane_id,
                            target_pane: pane_id, // Placeholder
                            alignment_count: 0,
                            average_confidence: 0.0,
                        },
                    }).await;
                }

                Ok(AddPaneResponse {
                    pane_id,
                    success: true,
                    message: format!("Pane added successfully for language: {}", request.language),
                })
            },
            Err(e) => {
                self.increment_error_counter();
                Ok(AddPaneResponse {
                    pane_id: Uuid::nil(),
                    success: false,
                    message: format!("Failed to add pane: {}", e),
                })
            }
        }
    }

    /// Remove a text pane
    pub async fn remove_pane(&self, pane_id: Uuid) -> Result<bool> {
        self.increment_request_counter();

        match self.alignment_service.remove_pane(pane_id).await {
            Ok(_) => Ok(true),
            Err(_) => {
                self.increment_error_counter();
                Ok(false)
            }
        }
    }

    /// Update pane content
    pub async fn update_pane_content(&self, request: UpdatePaneRequest) -> Result<UpdatePaneResponse> {
        self.increment_request_counter();

        match self.alignment_service.update_pane_content(
            request.pane_id,
            request.content,
            request.cursor_position,
        ).await {
            Ok(_) => {
                // Get synchronized positions if cursor position was provided
                let synchronized_positions = if let Some(cursor_pos) = request.cursor_position {
                    match self.alignment_service.synchronize_cursor_position(
                        request.pane_id,
                        cursor_pos,
                    ).await {
                        Ok(positions) => {
                            // Send real-time sync update
                            if self.config.enable_real_time_updates {
                                let sync_state = self.alignment_service.get_sync_state().await;
                                self.send_update(AlignmentUpdate {
                                    update_type: AlignmentUpdateType::SyncUpdate,
                                    affected_panes: positions.keys().cloned().collect(),
                                    quality_change: None,
                                    timestamp: self.current_timestamp(),
                                    data: AlignmentUpdateData::SyncUpdate {
                                        synchronized_positions: positions.clone(),
                                        sync_quality: sync_state.sync_quality,
                                    },
                                }).await;
                            }
                            Some(positions)
                        },
                        Err(_) => None,
                    }
                } else {
                    None
                };

                Ok(UpdatePaneResponse {
                    success: true,
                    message: "Pane content updated successfully".to_string(),
                    synchronized_positions,
                })
            },
            Err(e) => {
                self.increment_error_counter();
                Ok(UpdatePaneResponse {
                    success: false,
                    message: format!("Failed to update pane content: {}", e),
                    synchronized_positions: None,
                })
            }
        }
    }

    /// Synchronize cursor position across panes
    pub async fn synchronize_cursor(&self, request: SyncCursorRequest) -> Result<SyncCursorResponse> {
        self.increment_request_counter();

        match self.alignment_service.synchronize_cursor_position(
            request.source_pane_id,
            request.cursor_position,
        ).await {
            Ok(synchronized_positions) => {
                let sync_state = self.alignment_service.get_sync_state().await;

                // Send real-time update
                if self.config.enable_real_time_updates {
                    self.send_update(AlignmentUpdate {
                        update_type: AlignmentUpdateType::SyncUpdate,
                        affected_panes: synchronized_positions.keys().cloned().collect(),
                        quality_change: None,
                        timestamp: self.current_timestamp(),
                        data: AlignmentUpdateData::SyncUpdate {
                            synchronized_positions: synchronized_positions.clone(),
                            sync_quality: sync_state.sync_quality,
                        },
                    }).await;
                }

                Ok(SyncCursorResponse {
                    success: true,
                    synchronized_positions,
                    sync_quality: sync_state.sync_quality,
                })
            },
            Err(e) => {
                self.increment_error_counter();
                Ok(SyncCursorResponse {
                    success: false,
                    synchronized_positions: HashMap::new(),
                    sync_quality: 0.0,
                })
            }
        }
    }

    /// Apply user correction for learning
    pub async fn apply_user_correction(&self, request: UserCorrectionRequest) -> Result<UserCorrectionResponse> {
        self.increment_request_counter();

        // Create mock alignment objects for the correction
        // In a real implementation, these would be retrieved from the current alignments
        let original_alignment = self.create_mock_alignment(
            &request.original_source_text,
            &request.original_target_text,
        );
        
        let corrected_alignment = self.create_mock_alignment(
            &request.corrected_source_text,
            &request.corrected_target_text,
        );

        match self.alignment_service.learn_from_user_correction(
            request.source_pane_id,
            request.target_pane_id,
            original_alignment,
            corrected_alignment,
            request.correction_reason.clone(),
        ).await {
            Ok(_) => {
                // Send real-time update
                if self.config.enable_real_time_updates {
                    self.send_update(AlignmentUpdate {
                        update_type: AlignmentUpdateType::CorrectionApplied,
                        affected_panes: vec![request.source_pane_id, request.target_pane_id],
                        quality_change: Some(0.05), // Estimated improvement
                        timestamp: self.current_timestamp(),
                        data: AlignmentUpdateData::CorrectionApplied {
                            correction_count: 1,
                            improvement_estimate: 0.05,
                        },
                    }).await;
                }

                Ok(UserCorrectionResponse {
                    success: true,
                    message: "User correction applied successfully".to_string(),
                    learning_applied: true,
                })
            },
            Err(e) => {
                self.increment_error_counter();
                Ok(UserCorrectionResponse {
                    success: false,
                    message: format!("Failed to apply correction: {}", e),
                    learning_applied: false,
                })
            }
        }
    }

    /// Get comprehensive system status
    pub async fn get_system_status(&self) -> Result<SystemStatusResponse> {
        self.increment_request_counter();

        // Get active panes
        let active_panes = self.alignment_service.get_active_panes().await;
        let pane_info = self.convert_panes_to_info(active_panes).await;

        // Get sync state
        let sync_state = self.alignment_service.get_sync_state().await;
        let sync_info = self.convert_sync_state_to_info(sync_state);

        // Get quality monitoring
        let quality_monitoring = self.alignment_service.perform_quality_monitoring().await
            .unwrap_or_else(|_| {
                // Return default quality monitoring result on error
                QualityMonitoringResult {
                    overall_quality: 0.5,
                    pane_qualities: HashMap::new(),
                    alignment_qualities: HashMap::new(),
                    issues: Vec::new(),
                    recommendations: Vec::new(),
                }
            });
        let quality_info = self.convert_quality_monitoring_to_info(quality_monitoring);

        // Get performance metrics
        let performance_metrics = self.alignment_service.get_performance_metrics().await;
        let performance_info = self.convert_performance_metrics_to_info(performance_metrics);

        // Calculate system health
        let system_health = self.calculate_system_health(&quality_info, &performance_info).await;

        Ok(SystemStatusResponse {
            active_panes: pane_info,
            sync_state: sync_info,
            quality_monitoring: quality_info,
            performance_metrics: performance_info,
            system_health,
        })
    }

    /// Subscribe to real-time updates
    pub async fn subscribe_to_updates(&self) -> tokio::sync::mpsc::UnboundedReceiver<AlignmentUpdate> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        
        {
            let mut subscribers = self.update_subscribers.write().unwrap();
            subscribers.push(sender);
        }

        receiver
    }

    /// Get alignment statistics for a language pair
    pub async fn get_alignment_statistics(&self, source_language: &str, target_language: &str) -> Result<Option<AlignmentStatistics>> {
        self.increment_request_counter();
        self.alignment_service.get_alignment_statistics(source_language, target_language).await
    }

    /// Get real-time quality indicators
    pub async fn get_quality_indicators(&self) -> Result<HashMap<String, AlignmentQualityIndicator>> {
        self.increment_request_counter();
        self.alignment_service.get_real_time_quality_indicators().await
    }

    // Private helper methods

    async fn convert_panes_to_info(&self, panes: HashMap<Uuid, TextPane>) -> HashMap<Uuid, PaneInfo> {
        let mut pane_info = HashMap::new();
        
        for (id, pane) in panes {
            let quality_score = self.get_pane_quality_score(id).await;
            
            pane_info.insert(id, PaneInfo {
                id: pane.id,
                language: pane.language,
                content_length: pane.content.len(),
                cursor_position: pane.cursor_position,
                is_source: pane.is_source,
                last_modified: pane.last_modified.elapsed().as_secs(),
                quality_score,
            });
        }

        pane_info
    }

    fn convert_sync_state_to_info(&self, sync_state: SyncState) -> SyncStateInfo {
        SyncStateInfo {
            is_synchronized: !sync_state.synchronized_positions.is_empty(),
            last_sync_time: sync_state.last_sync_time.elapsed().as_secs(),
            sync_quality: sync_state.sync_quality,
            pending_events: sync_state.pending_syncs.len(),
        }
    }

    fn convert_quality_monitoring_to_info(&self, quality_monitoring: QualityMonitoringResult) -> QualityMonitoringInfo {
        let critical_issues = quality_monitoring.issues.iter()
            .filter(|issue| matches!(issue.severity, crate::services::multi_pane_alignment_service::QualityIssueSeverity::Critical))
            .count();

        let alignment_qualities = quality_monitoring.alignment_qualities.iter()
            .map(|(pair, indicator)| (pair.clone(), indicator.overall_quality))
            .collect();

        QualityMonitoringInfo {
            overall_quality: quality_monitoring.overall_quality,
            total_issues: quality_monitoring.issues.len(),
            critical_issues,
            recommendations_count: quality_monitoring.recommendations.len(),
            alignment_qualities,
        }
    }

    fn convert_performance_metrics_to_info(&self, metrics: AlignmentPerformanceMetrics) -> PerformanceMetricsInfo {
        PerformanceMetricsInfo {
            alignment_time_ms: metrics.alignment_time_ms,
            sync_time_ms: metrics.sync_time_ms,
            cache_hit_rate: metrics.cache_hit_rate,
            memory_usage_mb: metrics.memory_usage_mb,
            processing_rate: metrics.processing_rate_chars_per_sec,
            error_rate: metrics.error_rate,
        }
    }

    async fn calculate_system_health(&self, quality_info: &QualityMonitoringInfo, performance_info: &PerformanceMetricsInfo) -> SystemHealth {
        let mut warnings = Vec::new();
        
        // Determine health status based on various factors
        let status = if quality_info.critical_issues > 0 {
            warnings.push("Critical quality issues detected".to_string());
            HealthStatus::Critical
        } else if quality_info.overall_quality < self.config.quality_threshold_critical {
            warnings.push("Overall quality below critical threshold".to_string());
            HealthStatus::Poor
        } else if quality_info.overall_quality < self.config.quality_threshold_warning {
            warnings.push("Overall quality below warning threshold".to_string());
            HealthStatus::Fair
        } else if performance_info.error_rate > 0.05 {
            warnings.push("High error rate detected".to_string());
            HealthStatus::Fair
        } else if performance_info.cache_hit_rate < 0.7 {
            warnings.push("Low cache efficiency".to_string());
            HealthStatus::Good
        } else {
            HealthStatus::Excellent
        };

        // Calculate resource usage metrics
        let cpu_usage = performance_info.processing_rate / 10000.0; // Simplified calculation
        let memory_usage = performance_info.memory_usage_mb / 1024.0; // Convert to GB percentage
        let cache_efficiency = performance_info.cache_hit_rate;

        let error_count = self.error_counter.load(std::sync::atomic::Ordering::Relaxed);

        SystemHealth {
            status,
            cpu_usage: cpu_usage.min(100.0),
            memory_usage: memory_usage.min(100.0),
            cache_efficiency,
            error_count,
            warnings,
        }
    }

    async fn get_pane_quality_score(&self, pane_id: Uuid) -> Option<f64> {
        // In a real implementation, this would calculate the quality score for a specific pane
        // For now, return a default score
        Some(0.8)
    }

    async fn send_update(&self, update: AlignmentUpdate) {
        let subscribers = self.update_subscribers.read().unwrap();
        
        // Remove disconnected subscribers and send updates to active ones
        let mut active_subscribers = Vec::new();
        
        for sender in subscribers.iter() {
            if sender.send(update.clone()).is_ok() {
                active_subscribers.push(sender.clone());
            }
        }
        
        // Update the subscriber list to remove disconnected ones
        drop(subscribers);
        let mut subscribers = self.update_subscribers.write().unwrap();
        *subscribers = active_subscribers;
    }

    fn create_mock_alignment(&self, source_text: &str, target_text: &str) -> SentenceAlignment {
        use crate::services::sentence_alignment_service::{
            SentenceBoundary, BoundaryType, AlignmentMethod, ValidationStatus
        };

        SentenceAlignment {
            id: Uuid::new_v4(),
            source_sentence: SentenceBoundary {
                start_offset: 0,
                end_offset: source_text.len(),
                text: source_text.to_string(),
                confidence: 0.9,
                boundary_type: BoundaryType::Period,
            },
            target_sentence: SentenceBoundary {
                start_offset: 0,
                end_offset: target_text.len(),
                text: target_text.to_string(),
                confidence: 0.9,
                boundary_type: BoundaryType::Period,
            },
            source_language: "en".to_string(),
            target_language: "es".to_string(),
            alignment_confidence: 0.8,
            alignment_method: AlignmentMethod::UserValidated,
            validation_status: ValidationStatus::Validated,
            created_at: Instant::now(),
            last_validated: Some(Instant::now()),
        }
    }

    fn increment_request_counter(&self) {
        self.request_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn increment_error_counter(&self) {
        self.error_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn current_timestamp(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Get API usage statistics
    pub async fn get_api_statistics(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        
        stats.insert("total_requests".to_string(), 
            self.request_counter.load(std::sync::atomic::Ordering::Relaxed));
        stats.insert("total_errors".to_string(), 
            self.error_counter.load(std::sync::atomic::Ordering::Relaxed) as u64);
        stats.insert("active_subscribers".to_string(), 
            self.update_subscribers.read().unwrap().len() as u64);

        stats
    }

    /// Health check endpoint
    pub async fn health_check(&self) -> Result<HealthStatus> {
        let system_status = self.get_system_status().await?;
        Ok(system_status.system_health.status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{
        multi_pane_alignment_service::MultiPaneAlignmentConfig,
        sentence_alignment_service::AlignmentConfig,
        text_structure_analyzer::StructureAnalysisConfig,
        alignment_cache_service::AlignmentCacheConfig,
    };

    fn create_test_service() -> AlignmentApiService {
        let multi_pane_config = MultiPaneAlignmentConfig {
            max_panes: 4,
            default_source_language: "en".to_string(),
            supported_languages: vec!["en".to_string(), "es".to_string()],
            enable_real_time_sync: true,
            sync_delay_ms: 100,
            enable_quality_monitoring: true,
            auto_validation_threshold: 0.85,
            structure_analysis_config: StructureAnalysisConfig::default(),
            alignment_config: AlignmentConfig::default(),
            cache_config: AlignmentCacheConfig::default(),
        };

        let api_config = AlignmentApiConfig::default();
        
        AlignmentApiService::new(multi_pane_config, api_config).unwrap()
    }

    #[tokio::test]
    async fn test_add_pane_api() {
        let service = create_test_service();
        
        let request = AddPaneRequest {
            language: "en".to_string(),
            content: "Hello world. How are you?".to_string(),
            is_source: true,
        };

        let response = service.add_pane(request).await.unwrap();
        
        assert!(response.success);
        assert_ne!(response.pane_id, Uuid::nil());
        assert!(response.message.contains("successfully"));
    }

    #[tokio::test]
    async fn test_update_pane_content_api() {
        let service = create_test_service();
        
        // Add a pane first
        let add_request = AddPaneRequest {
            language: "en".to_string(),
            content: "Original content.".to_string(),
            is_source: true,
        };
        let add_response = service.add_pane(add_request).await.unwrap();
        
        // Update the pane content
        let update_request = UpdatePaneRequest {
            pane_id: add_response.pane_id,
            content: "Updated content with more text.".to_string(),
            cursor_position: Some(10),
        };

        let update_response = service.update_pane_content(update_request).await.unwrap();
        
        assert!(update_response.success);
        assert!(update_response.message.contains("successfully"));
    }

    #[tokio::test]
    async fn test_system_status_api() {
        let service = create_test_service();
        
        // Add some panes for a more realistic status
        let _pane1 = service.add_pane(AddPaneRequest {
            language: "en".to_string(),
            content: "English content.".to_string(),
            is_source: true,
        }).await.unwrap();

        let _pane2 = service.add_pane(AddPaneRequest {
            language: "es".to_string(),
            content: "Contenido en español.".to_string(),
            is_source: false,
        }).await.unwrap();

        let status = service.get_system_status().await.unwrap();
        
        assert_eq!(status.active_panes.len(), 2);
        assert!(status.quality_monitoring.overall_quality >= 0.0);
        assert!(status.quality_monitoring.overall_quality <= 1.0);
        assert!(!matches!(status.system_health.status, HealthStatus::Critical));
    }

    #[tokio::test]
    async fn test_cursor_synchronization_api() {
        let service = create_test_service();
        
        // Add panes
        let pane1 = service.add_pane(AddPaneRequest {
            language: "en".to_string(),
            content: "First sentence. Second sentence.".to_string(),
            is_source: true,
        }).await.unwrap();

        let _pane2 = service.add_pane(AddPaneRequest {
            language: "es".to_string(),
            content: "Primera oración. Segunda oración.".to_string(),
            is_source: false,
        }).await.unwrap();

        // Synchronize cursor
        let sync_request = SyncCursorRequest {
            source_pane_id: pane1.pane_id,
            cursor_position: 20,
        };

        let sync_response = service.synchronize_cursor(sync_request).await.unwrap();
        
        assert!(sync_response.success);
        assert!(!sync_response.synchronized_positions.is_empty());
        assert!(sync_response.sync_quality >= 0.0);
    }

    #[tokio::test]
    async fn test_health_check() {
        let service = create_test_service();
        
        let health_status = service.health_check().await.unwrap();
        
        // Should not be critical for a new service
        assert!(!matches!(health_status, HealthStatus::Critical));
    }

    #[tokio::test]
    async fn test_api_statistics() {
        let service = create_test_service();
        
        // Make some API calls
        let _ = service.add_pane(AddPaneRequest {
            language: "en".to_string(),
            content: "Test content.".to_string(),
            is_source: true,
        }).await.unwrap();

        let _ = service.get_system_status().await.unwrap();

        let stats = service.get_api_statistics().await;
        
        assert!(stats.contains_key("total_requests"));
        assert!(stats.contains_key("total_errors"));
        assert!(stats.contains_key("active_subscribers"));
        assert!(stats["total_requests"] >= 2); // At least 2 requests made
    }
}