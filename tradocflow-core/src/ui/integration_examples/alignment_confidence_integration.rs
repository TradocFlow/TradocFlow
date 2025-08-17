use std::collections::HashMap;
use crate::gui::AlignmentConfidenceBridge;
use crate::Result;

/// Example integration of alignment confidence scoring and visual feedback
/// Demonstrates how to use the new components in a real application
pub struct AlignmentConfidenceIntegration {
    bridge: AlignmentConfidenceBridge,
    current_documents: HashMap<String, String>,
    active_languages: (String, String),
}

impl AlignmentConfidenceIntegration {
    /// Create a new alignment confidence integration
    pub fn new() -> Self {
        Self {
            bridge: AlignmentConfidenceBridge::new(),
            current_documents: HashMap::new(),
            active_languages: ("en".to_string(), "es".to_string()),
        }
    }
    
    /// Load documents for alignment analysis
    pub async fn load_documents(
        &mut self,
        source_text: String,
        target_text: String,
        source_language: String,
        target_language: String,
    ) -> Result<()> {
        // Store documents
        self.current_documents.insert("source".to_string(), source_text.clone());
        self.current_documents.insert("target".to_string(), target_text.clone());
        self.active_languages = (source_language.clone(), target_language.clone());
        
        // Process alignment data to generate confidence indicators
        self.bridge.process_alignment_data(
            &source_text,
            &target_text,
            &source_language,
            &target_language,
        ).await?;
        
        println!("✅ Documents loaded and alignment confidence calculated");
        Ok(())
    }
    
    /// Simulate user threshold adjustment
    pub async fn adjust_confidence_threshold(
        &self,
        threshold_type: &str,
        new_value: f64,
    ) -> Result<()> {
        self.bridge.handle_threshold_change(threshold_type, new_value).await?;
        println!("🎯 Confidence threshold '{}' adjusted to {}", threshold_type, new_value);
        Ok(())
    }
    
    /// Simulate auto-fix operation
    pub async fn attempt_auto_fix(&self, problem_index: usize) -> Result<()> {
        let success = self.bridge.handle_auto_fix_request(problem_index).await?;
        
        if success {
            println!("🔧 Auto-fix successful for problem #{}", problem_index);
        } else {
            println!("⚠️ Auto-fix not available for problem #{} - manual correction needed", problem_index);
        }
        
        Ok(())
    }
    
    /// Simulate manual correction operation
    pub async fn perform_manual_correction(
        &self,
        operation_type: &str,
        source_sentence: usize,
        target_sentence: usize,
        notes: &str,
    ) -> Result<()> {
        let source_selections = vec![("pane-1".to_string(), source_sentence)];
        let target_selections = vec![("pane-2".to_string(), target_sentence)];
        
        let success = self.bridge.handle_correction_operation(
            operation_type,
            source_selections,
            target_selections,
            notes,
        ).await?;
        
        if success {
            println!("✏️ Manual correction '{}' applied successfully", operation_type);
        } else {
            println!("❌ Manual correction '{}' failed", operation_type);
        }
        
        Ok(())
    }
    
    /// Simulate sentence boundary synchronization
    pub async fn sync_sentence_boundaries(&self, cursor_position: usize) -> Result<()> {
        let sync_positions = self.bridge.handle_sentence_boundary_sync(
            self.current_documents.clone(),
            cursor_position,
            &self.active_languages.0,
        ).await?;
        
        println!("🔄 Sentence boundary synchronization completed:");
        for (language, position) in sync_positions {
            println!("  {} -> position {}", language, position);
        }
        
        Ok(())
    }
    
    /// Get alignment statistics
    pub async fn get_statistics(&self) -> Result<()> {
        if let Some(stats) = self.bridge.get_alignment_statistics(self.active_languages.clone()).await? {
            println!("📊 Alignment Statistics:");
            println!("  Total sentences: {}", stats.total_sentences);
            println!("  Aligned sentences: {}", stats.aligned_sentences);
            println!("  Validated alignments: {}", stats.validated_alignments);
            println!("  Average confidence: {:.2}%", stats.average_confidence * 100.0);
            println!("  Alignment accuracy: {:.2}%", stats.alignment_accuracy * 100.0);
            println!("  Processing time: {}ms", stats.processing_time_ms);
        } else {
            println!("📊 No statistics available for language pair {:?}", self.active_languages);
        }
        
        Ok(())
    }
    
    /// Demonstrate complete workflow
    pub async fn demonstrate_workflow() -> Result<()> {
        println!("🚀 Starting alignment confidence demonstration...\n");
        
        let mut integration = Self::new();
        
        // Load sample documents
        let source_text = "Hello world. How are you today? This is a test document. It contains multiple sentences for alignment testing.";
        let target_text = "Hola mundo. ¿Cómo estás hoy? Este es un documento de prueba. Contiene múltiples oraciones para pruebas de alineación.";
        
        integration.load_documents(
            source_text.to_string(),
            target_text.to_string(),
            "en".to_string(),
            "es".to_string(),
        ).await?;
        
        // Show initial statistics
        integration.get_statistics().await?;
        println!();
        
        // Adjust confidence thresholds
        integration.adjust_confidence_threshold("good", 0.75).await?;
        integration.adjust_confidence_threshold("excellent", 0.95).await?;
        println!();
        
        // Attempt auto-fixes
        integration.attempt_auto_fix(0).await?;
        integration.attempt_auto_fix(1).await?;
        println!();
        
        // Perform manual corrections
        integration.perform_manual_correction(
            "align",
            1,
            1,
            "Manual alignment correction for improved accuracy"
        ).await?;
        
        integration.perform_manual_correction(
            "validate",
            2,
            2,
            "Validated alignment as correct"
        ).await?;
        println!();
        
        // Test sentence boundary synchronization
        integration.sync_sentence_boundaries(25).await?; // Cursor in second sentence
        integration.sync_sentence_boundaries(60).await?; // Cursor in third sentence
        println!();
        
        // Show final statistics
        println!("📈 Final statistics after corrections:");
        integration.get_statistics().await?;
        
        println!("\n✨ Alignment confidence demonstration completed!");
        
        Ok(())
    }
}

impl Default for AlignmentConfidenceIntegration {
    fn default() -> Self {
        Self::new()
    }
}

/// Example usage patterns for UI integration
pub mod examples {
    use super::*;
    
    /// Example: Real-time confidence monitoring
    pub async fn real_time_monitoring_example() -> Result<()> {
        println!("🔍 Real-time confidence monitoring example");
        
        let integration = AlignmentConfidenceIntegration::new();
        
        // Simulate document editing with real-time updates
        let documents = vec![
            ("Hello.", "Hola."),
            ("Hello world.", "Hola mundo."),
            ("Hello world. How are you?", "Hola mundo. ¿Cómo estás?"),
        ];
        
        for (i, (source, target)) in documents.iter().enumerate() {
            println!("\n📝 Document update #{}", i + 1);
            
            // In a real application, this would be triggered by text changes
            integration.bridge.process_alignment_data(
                source,
                target,
                "en",
                "es",
            ).await?;
            
            println!("  Source: {}", source);
            println!("  Target: {}", target);
            println!("  ✅ Confidence indicators updated");
        }
        
        Ok(())
    }
    
    /// Example: Batch correction workflow
    pub async fn batch_correction_example() -> Result<()> {
        println!("📦 Batch correction workflow example");
        
        let integration = AlignmentConfidenceIntegration::new();
        
        // Simulate batch corrections
        let corrections = vec![
            ("align", 0, 0, "Connect first sentences"),
            ("align", 1, 1, "Connect second sentences"),
            ("validate", 2, 2, "Confirm third sentence alignment"),
            ("split", 3, 3, "Split compound sentence"),
        ];
        
        for (operation, source_idx, target_idx, notes) in corrections {
            println!("\n🔧 Applying correction: {}", operation);
            integration.perform_manual_correction(operation, source_idx, target_idx, notes).await?;
        }
        
        println!("\n✅ Batch corrections completed");
        Ok(())
    }
    
    /// Example: Interactive threshold tuning
    pub async fn threshold_tuning_example() -> Result<()> {
        println!("🎛️ Interactive threshold tuning example");
        
        let integration = AlignmentConfidenceIntegration::new();
        
        // Simulate threshold adjustments for different quality levels
        let threshold_adjustments = vec![
            ("excellent", 0.95),
            ("good", 0.80),
            ("moderate", 0.60),
            ("poor", 0.40),
        ];
        
        for (threshold_type, value) in threshold_adjustments {
            integration.adjust_confidence_threshold(threshold_type, value).await?;
            
            // Show how this affects the confidence categorization
            println!("  {} threshold set to {:.0}%", threshold_type, value * 100.0);
        }
        
        println!("\n🎯 Threshold tuning completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_integration_creation() {
        let integration = AlignmentConfidenceIntegration::new();
        assert_eq!(integration.current_documents.len(), 0);
        assert_eq!(integration.active_languages, ("en".to_string(), "es".to_string()));
    }
    
    #[tokio::test]
    async fn test_document_loading() {
        let mut integration = AlignmentConfidenceIntegration::new();
        let result = integration.load_documents(
            "Test source".to_string(),
            "Fuente de prueba".to_string(),
            "en".to_string(),
            "es".to_string(),
        ).await;
        
        assert!(result.is_ok());
        assert_eq!(integration.current_documents.len(), 2);
        assert_eq!(integration.active_languages, ("en".to_string(), "es".to_string()));
    }
    
    #[tokio::test]
    async fn test_threshold_adjustment() {
        let integration = AlignmentConfidenceIntegration::new();
        let result = integration.adjust_confidence_threshold("good", 0.75).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_sentence_sync() {
        let mut integration = AlignmentConfidenceIntegration::new();
        
        // Load documents first
        integration.load_documents(
            "Hello world.".to_string(),
            "Hola mundo.".to_string(),
            "en".to_string(),
            "es".to_string(),
        ).await.unwrap();
        
        let result = integration.sync_sentence_boundaries(5).await;
        assert!(result.is_ok());
    }
}