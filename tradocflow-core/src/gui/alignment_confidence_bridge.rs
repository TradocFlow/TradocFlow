use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::rc::Rc;
use slint::{ComponentHandle, Model, VecModel, Weak};
use crate::services::sentence_alignment_service::{
    SentenceAlignmentService, 
    AlignmentConfig, 
    SentenceAlignment, 
    AlignmentQualityIndicator,
    ProblemArea as ServiceProblemArea,
    AlignmentIssue,
    ValidationStatus,
    AlignmentMethod
};
use crate::Result;

/// Bridge between Slint UI and Rust sentence alignment service
/// Provides real-time confidence scoring and visual feedback
pub struct AlignmentConfidenceBridge {
    alignment_service: Arc<SentenceAlignmentService>,
    current_alignments: Arc<RwLock<Vec<SentenceAlignment>>>,
    confidence_cache: Arc<RwLock<HashMap<String, f64>>>,
    problem_areas_cache: Arc<RwLock<Vec<ServiceProblemArea>>>,
    ui_handle: Option<Weak<slint::ComponentHandle<crate::ui::MainWindow>>>,
}

impl AlignmentConfidenceBridge {
    /// Create a new alignment confidence bridge
    pub fn new() -> Self {
        let config = AlignmentConfig::default();
        let alignment_service = Arc::new(SentenceAlignmentService::new(config));
        
        Self {
            alignment_service,
            current_alignments: Arc::new(RwLock::new(Vec::new())),
            confidence_cache: Arc::new(RwLock::new(HashMap::new())),
            problem_areas_cache: Arc::new(RwLock::new(Vec::new())),
            ui_handle: None,
        }
    }
    
    /// Set the UI handle for updates
    pub fn set_ui_handle(&mut self, handle: Weak<slint::ComponentHandle<crate::ui::MainWindow>>) {
        self.ui_handle = Some(handle);
    }
    
    /// Process new alignment data and update confidence indicators
    pub async fn process_alignment_data(
        &self,
        source_text: &str,
        target_text: &str,
        source_language: &str,
        target_language: &str,
    ) -> Result<()> {
        // Generate sentence alignments
        let alignments = self.alignment_service
            .align_sentences(source_text, target_text, source_language, target_language)
            .await?;
        
        // Calculate quality indicators
        let quality_indicators = self.alignment_service
            .calculate_quality_indicators(&alignments)
            .await?;
        
        // Update caches
        {
            let mut current_alignments = self.current_alignments.write().unwrap();
            *current_alignments = alignments.clone();
        }
        
        // Convert to UI structures and update interface
        let confidence_indicators = self.convert_to_confidence_indicators(&alignments);
        let problem_areas = self.convert_to_problem_areas(&quality_indicators.problem_areas);
        let alignment_connections = self.convert_to_alignment_connections(&alignments);
        let statistics = self.convert_to_confidence_statistics(&alignments, &quality_indicators);
        
        // Update UI
        self.update_ui_confidence_data(
            confidence_indicators,
            problem_areas,
            alignment_connections,
            statistics,
        );
        
        Ok(())
    }
    
    /// Handle confidence threshold changes from UI
    pub async fn handle_threshold_change(
        &self,
        threshold_type: &str,
        new_value: f64,
    ) -> Result<()> {
        // Update service configuration
        // Note: Would need to modify SentenceAlignmentService to support dynamic config updates
        
        // Recalculate confidence with new thresholds
        let alignments = self.current_alignments.read().unwrap().clone();
        if !alignments.is_empty() {
            let quality_indicators = self.alignment_service
                .calculate_quality_indicators(&alignments)
                .await?;
            
            let confidence_indicators = self.convert_to_confidence_indicators(&alignments);
            let statistics = self.convert_to_confidence_statistics(&alignments, &quality_indicators);
            
            self.update_ui_confidence_indicators(confidence_indicators, statistics);
        }
        
        Ok(())
    }
    
    /// Handle problem area auto-fix requests
    pub async fn handle_auto_fix_request(
        &self,
        problem_index: usize,
    ) -> Result<bool> {
        let problem_areas = self.problem_areas_cache.read().unwrap();
        
        if problem_index < problem_areas.len() {
            let problem = &problem_areas[problem_index];
            
            // Implement auto-fix logic based on problem type
            match problem.issue_type {
                AlignmentIssue::LengthMismatch => {
                    // Auto-fix length mismatches by adjusting alignment method
                    self.fix_length_mismatch(problem).await?;
                    return Ok(true);
                }
                AlignmentIssue::BoundaryDetectionError => {
                    // Auto-fix boundary detection errors
                    self.fix_boundary_detection(problem).await?;
                    return Ok(true);
                }
                _ => {
                    // Other issues require manual intervention
                    return Ok(false);
                }
            }
        }
        
        Ok(false)
    }
    
    /// Handle manual correction operations
    pub async fn handle_correction_operation(
        &self,
        operation_type: &str,
        source_selections: Vec<(String, usize)>, // (pane_id, sentence_index)
        target_selections: Vec<(String, usize)>,
        user_notes: &str,
    ) -> Result<bool> {
        match operation_type {
            "align" => {
                self.create_manual_alignment(source_selections, target_selections, user_notes).await
            }
            "unalign" => {
                self.remove_alignment(source_selections).await
            }
            "merge" => {
                self.merge_sentences(source_selections, user_notes).await
            }
            "split" => {
                self.split_sentence(source_selections, user_notes).await
            }
            "validate" => {
                self.validate_alignment(source_selections, true, user_notes).await
            }
            "reject" => {
                self.validate_alignment(source_selections, false, user_notes).await
            }
            _ => Ok(false),
        }
    }
    
    /// Handle sentence boundary synchronization
    pub async fn handle_sentence_boundary_sync(
        &self,
        pane_contents: HashMap<String, String>,
        cursor_position: usize,
        source_language: &str,
    ) -> Result<HashMap<String, usize>> {
        self.alignment_service
            .synchronize_sentence_boundaries(&pane_contents, cursor_position, source_language)
            .await
    }
    
    /// Get alignment statistics for reporting
    pub async fn get_alignment_statistics(
        &self,
        language_pair: (String, String),
    ) -> Result<Option<crate::services::sentence_alignment_service::AlignmentStatistics>> {
        self.alignment_service.get_alignment_statistics(language_pair).await
    }
    
    /// Learn from user corrections
    pub async fn learn_from_correction(
        &self,
        original_alignment: SentenceAlignment,
        corrected_alignment: SentenceAlignment,
        correction_reason: String,
    ) -> Result<()> {
        self.alignment_service
            .learn_from_correction(original_alignment, corrected_alignment, correction_reason)
            .await
    }
    
    // Private helper methods
    
    fn convert_to_confidence_indicators(&self, alignments: &[SentenceAlignment]) -> Vec<slint::SharedString> {
        // Convert Rust alignment data to Slint confidence indicators
        // This would create the JSON/struct data that Slint expects
        alignments.iter().enumerate().map(|(index, alignment)| {
            format!(
                "{{\"sentence_index\":{},\"confidence_score\":{},\"confidence_level\":\"{}\",\"alignment_method\":\"{}\",\"validation_status\":\"{}\",\"position_start\":{},\"position_end\":{},\"is_problematic\":{},\"last_updated\":\"{}\"}}",
                index,
                alignment.alignment_confidence,
                self.get_confidence_level(alignment.alignment_confidence),
                self.get_alignment_method_string(&alignment.alignment_method),
                self.get_validation_status_string(&alignment.validation_status),
                alignment.source_sentence.start_offset,
                alignment.source_sentence.end_offset,
                alignment.alignment_confidence < 0.5,
                "now"
            ).into()
        }).collect()
    }
    
    fn convert_to_problem_areas(&self, problem_areas: &[ServiceProblemArea]) -> Vec<slint::SharedString> {
        problem_areas.iter().map(|problem| {
            format!(
                "{{\"id\":\"{}\",\"start_position\":{},\"end_position\":{},\"sentence_index\":{},\"severity\":\"{}\",\"issue_type\":\"{}\",\"confidence_score\":{},\"description\":\"{}\",\"suggestion\":\"{}\",\"auto_fixable\":{},\"affected_pane_ids\":[\"{}\"],\"created_at\":\"{}\"}}",
                uuid::Uuid::new_v4(),
                problem.start_position,
                problem.end_position,
                0, // Would need to calculate from position
                self.get_severity_string(problem.severity),
                self.get_issue_type_string(&problem.issue_type),
                0.5, // Default confidence for problems
                problem.suggestion,
                problem.suggestion,
                self.is_auto_fixable(&problem.issue_type),
                "pane-1", // Would need actual pane mapping
                "now"
            ).into()
        }).collect()
    }
    
    fn convert_to_alignment_connections(&self, alignments: &[SentenceAlignment]) -> Vec<slint::SharedString> {
        alignments.iter().enumerate().map(|(index, alignment)| {
            format!(
                "{{\"source_sentence_index\":{},\"target_sentence_index\":{},\"source_pane_id\":\"pane-1\",\"target_pane_id\":\"pane-2\",\"confidence_score\":{},\"connection_type\":\"{}\",\"is_validated\":{},\"has_problems\":{}}}",
                index,
                index, // Simplified 1:1 mapping
                alignment.alignment_confidence,
                self.get_connection_type(alignment.alignment_confidence),
                alignment.validation_status == ValidationStatus::Validated,
                alignment.alignment_confidence < 0.7
            ).into()
        }).collect()
    }
    
    fn convert_to_confidence_statistics(
        &self,
        alignments: &[SentenceAlignment],
        quality_indicators: &AlignmentQualityIndicator,
    ) -> slint::SharedString {
        let total = alignments.len();
        let excellent_count = alignments.iter().filter(|a| a.alignment_confidence >= 0.9).count();
        let good_count = alignments.iter().filter(|a| a.alignment_confidence >= 0.7 && a.alignment_confidence < 0.9).count();
        let moderate_count = alignments.iter().filter(|a| a.alignment_confidence >= 0.5 && a.alignment_confidence < 0.7).count();
        let poor_count = alignments.iter().filter(|a| a.alignment_confidence >= 0.3 && a.alignment_confidence < 0.5).count();
        let critical_count = alignments.iter().filter(|a| a.alignment_confidence < 0.3).count();
        
        let average_confidence = if total > 0 {
            alignments.iter().map(|a| a.alignment_confidence).sum::<f64>() / total as f64
        } else {
            0.0
        };
        
        format!(
            "{{\"total_alignments\":{},\"excellent_count\":{},\"good_count\":{},\"moderate_count\":{},\"poor_count\":{},\"critical_count\":{},\"average_confidence\":{},\"improvement_trend\":0.0}}",
            total, excellent_count, good_count, moderate_count, poor_count, critical_count, average_confidence
        ).into()
    }
    
    fn update_ui_confidence_data(
        &self,
        confidence_indicators: Vec<slint::SharedString>,
        problem_areas: Vec<slint::SharedString>,
        alignment_connections: Vec<slint::SharedString>,
        statistics: slint::SharedString,
    ) {
        if let Some(ui_handle) = &self.ui_handle {
            if let Some(ui) = ui_handle.upgrade() {
                // Update the UI with new data
                // This would use the Slint component's methods to update the data
                // Note: Actual implementation would depend on how the UI properties are exposed
            }
        }
    }
    
    fn update_ui_confidence_indicators(
        &self,
        confidence_indicators: Vec<slint::SharedString>,
        statistics: slint::SharedString,
    ) {
        if let Some(ui_handle) = &self.ui_handle {
            if let Some(ui) = ui_handle.upgrade() {
                // Update only confidence indicators and statistics
            }
        }
    }
    
    // Helper methods for string conversions
    
    fn get_confidence_level(&self, confidence: f64) -> &str {
        if confidence >= 0.9 { "Excellent" }
        else if confidence >= 0.7 { "Good" }
        else if confidence >= 0.5 { "Moderate" }
        else if confidence >= 0.3 { "Poor" }
        else { "Critical" }
    }
    
    fn get_alignment_method_string(&self, method: &AlignmentMethod) -> &str {
        match method {
            AlignmentMethod::PositionBased => "position",
            AlignmentMethod::LengthRatio => "length",
            AlignmentMethod::MachineLearning => "ml",
            AlignmentMethod::UserValidated => "user",
            AlignmentMethod::Hybrid => "hybrid",
        }
    }
    
    fn get_validation_status_string(&self, status: &ValidationStatus) -> &str {
        match status {
            ValidationStatus::Pending => "pending",
            ValidationStatus::Validated => "validated",
            ValidationStatus::Rejected => "rejected",
            ValidationStatus::NeedsReview => "review",
        }
    }
    
    fn get_severity_string(&self, severity: f64) -> &str {
        if severity >= 0.8 { "Critical" }
        else if severity >= 0.6 { "Error" }
        else if severity >= 0.4 { "Warning" }
        else { "Info" }
    }
    
    fn get_issue_type_string(&self, issue_type: &AlignmentIssue) -> &str {
        match issue_type {
            AlignmentIssue::LengthMismatch => "length_mismatch",
            AlignmentIssue::StructuralDivergence => "structure_divergence",
            AlignmentIssue::MissingSentence => "missing_sentence",
            AlignmentIssue::ExtraSentence => "extra_sentence",
            AlignmentIssue::OrderMismatch => "order_mismatch",
            AlignmentIssue::BoundaryDetectionError => "boundary_detection_error",
        }
    }
    
    fn get_connection_type(&self, confidence: f64) -> &str {
        if confidence >= 0.8 { "strong" }
        else if confidence >= 0.6 { "weak" }
        else if confidence >= 0.4 { "uncertain" }
        else { "broken" }
    }
    
    fn is_auto_fixable(&self, issue_type: &AlignmentIssue) -> bool {
        matches!(issue_type, 
            AlignmentIssue::LengthMismatch | 
            AlignmentIssue::BoundaryDetectionError
        )
    }
    
    // Auto-fix implementation methods
    
    async fn fix_length_mismatch(&self, problem: &ServiceProblemArea) -> Result<()> {
        // Implement length mismatch auto-fix logic
        // This might involve adjusting alignment algorithm parameters
        // or re-running alignment with different settings
        Ok(())
    }
    
    async fn fix_boundary_detection(&self, problem: &ServiceProblemArea) -> Result<()> {
        // Implement boundary detection auto-fix logic
        // This might involve adjusting sentence boundary detection parameters
        Ok(())
    }
    
    // Manual correction operation implementations
    
    async fn create_manual_alignment(
        &self,
        source_selections: Vec<(String, usize)>,
        target_selections: Vec<(String, usize)>,
        user_notes: &str,
    ) -> Result<bool> {
        // Implement manual alignment creation
        // This would create new alignments based on user selections
        Ok(true)
    }
    
    async fn remove_alignment(&self, selections: Vec<(String, usize)>) -> Result<bool> {
        // Implement alignment removal
        Ok(true)
    }
    
    async fn merge_sentences(&self, selections: Vec<(String, usize)>, user_notes: &str) -> Result<bool> {
        // Implement sentence merging
        Ok(true)
    }
    
    async fn split_sentence(&self, selections: Vec<(String, usize)>, user_notes: &str) -> Result<bool> {
        // Implement sentence splitting
        Ok(true)
    }
    
    async fn validate_alignment(
        &self,
        selections: Vec<(String, usize)>,
        is_valid: bool,
        user_notes: &str,
    ) -> Result<bool> {
        // Implement alignment validation
        // This would update the validation status and potentially learn from the feedback
        Ok(true)
    }
}

impl Default for AlignmentConfidenceBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_alignment_confidence_bridge_creation() {
        let bridge = AlignmentConfidenceBridge::new();
        assert!(bridge.current_alignments.read().unwrap().is_empty());
    }
    
    #[tokio::test]
    async fn test_process_alignment_data() {
        let bridge = AlignmentConfidenceBridge::new();
        let result = bridge.process_alignment_data(
            "Hello world. How are you?",
            "Hola mundo. ¿Cómo estás?",
            "en",
            "es"
        ).await;
        
        assert!(result.is_ok());
        assert!(!bridge.current_alignments.read().unwrap().is_empty());
    }
    
    #[test]
    fn test_confidence_level_conversion() {
        let bridge = AlignmentConfidenceBridge::new();
        assert_eq!(bridge.get_confidence_level(0.95), "Excellent");
        assert_eq!(bridge.get_confidence_level(0.75), "Good");
        assert_eq!(bridge.get_confidence_level(0.55), "Moderate");
        assert_eq!(bridge.get_confidence_level(0.35), "Poor");
        assert_eq!(bridge.get_confidence_level(0.15), "Critical");
    }
}