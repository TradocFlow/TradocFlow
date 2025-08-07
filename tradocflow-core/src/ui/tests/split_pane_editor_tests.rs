use crate::services::{EditorSyncService, SplitOrientation, LanguageSyntaxService};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Test suite for split-pane editor functionality
pub struct SplitPaneEditorTests {
    sync_service: Arc<EditorSyncService>,
    syntax_service: Arc<LanguageSyntaxService>,
}

impl SplitPaneEditorTests {
    /// Create a new test suite instance
    pub fn new() -> Self {
        Self {
            sync_service: Arc::new(EditorSyncService::new()),
            syntax_service: Arc::new(LanguageSyntaxService::new()),
        }
    }
    
    /// Test basic split-pane configuration
    pub async fn test_split_pane_configuration(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Test horizontal split configuration
        let config = self.sync_service.get_config()?;
        assert_eq!(config.orientation, SplitOrientation::Horizontal);
        assert!(config.sync_enabled);
        assert!(config.sync_cursor);
        assert!(config.sync_scroll);
        
        // Test orientation change
        self.sync_service.set_orientation(SplitOrientation::Vertical)?;
        let updated_config = self.sync_service.get_config()?;
        assert_eq!(updated_config.orientation, SplitOrientation::Vertical);
        
        println!("✓ Split-pane configuration test passed");
        Ok(())
    }
    
    /// Test language synchronization
    pub async fn test_language_synchronization(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Set up test languages
        self.sync_service.set_languages("en", "de")?;
        
        // Create test content for both languages
        let en_content = "Hello world\nThis is a test\nWith multiple lines";
        let de_content = "Hallo Welt\nDas ist ein Test\nMit mehreren Zeilen";
        
        // Update pane states
        let en_state = crate::services::LanguagePaneState {
            language: "en".to_string(),
            content: en_content.to_string(),
            cursor_position: 12, // Position in "Hello world\n"
            scroll_position: 0.0,
            selection_start: None,
            selection_end: None,
            read_only: false,
            last_modified: chrono::Utc::now(),
        };
        
        let de_state = crate::services::LanguagePaneState {
            language: "de".to_string(),
            content: de_content.to_string(),
            cursor_position: 0,
            scroll_position: 0.0,
            selection_start: None,
            selection_end: None,
            read_only: false,
            last_modified: chrono::Utc::now(),
        };
        
        self.sync_service.update_pane_state("en", en_state)?;
        self.sync_service.update_pane_state("de", de_state)?;
        
        // Test cursor synchronization
        self.sync_service.sync_cursor_move("en", 12)?;
        
        // Verify the German pane was updated
        let updated_de_state = self.sync_service.get_pane_state("de")?.unwrap();
        assert!(updated_de_state.cursor_position > 0);
        
        println!("✓ Language synchronization test passed");
        Ok(())
    }
    
    /// Test cursor position synchronization
    pub async fn test_cursor_synchronization(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Set up test content with different lengths
        let short_content = "Short text";
        let long_content = "This is a much longer text with more content to test synchronization";
        
        let short_state = crate::services::LanguagePaneState {
            language: "en".to_string(),
            content: short_content.to_string(),
            cursor_position: 5, // Middle of "Short"
            scroll_position: 0.0,
            selection_start: None,
            selection_end: None,
            read_only: false,
            last_modified: chrono::Utc::now(),
        };
        
        let long_state = crate::services::LanguagePaneState {
            language: "de".to_string(),
            content: long_content.to_string(),
            cursor_position: 0,
            scroll_position: 0.0,
            selection_start: None,
            selection_end: None,
            read_only: false,
            last_modified: chrono::Utc::now(),
        };
        
        self.sync_service.update_pane_state("en", short_state)?;
        self.sync_service.update_pane_state("de", long_state)?;
        
        // Test cursor sync from short to long content
        self.sync_service.sync_cursor_move("en", 5)?;
        
        let updated_long_state = self.sync_service.get_pane_state("de")?.unwrap();
        
        // The cursor should be positioned proportionally in the longer text
        let expected_position = (5.0 / short_content.len() as f64 * long_content.len() as f64) as usize;
        let tolerance = 5; // Allow some tolerance for rounding
        
        assert!(
            (updated_long_state.cursor_position as i32 - expected_position as i32).abs() <= tolerance,
            "Cursor position {} not within tolerance of expected {}",
            updated_long_state.cursor_position,
            expected_position
        );
        
        println!("✓ Cursor synchronization test passed");
        Ok(())
    }
    
    /// Test scroll position synchronization
    pub async fn test_scroll_synchronization(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Create multi-line content for testing
        let en_lines = vec![
            "Line 1 in English",
            "Line 2 in English", 
            "Line 3 in English",
            "Line 4 in English",
            "Line 5 in English",
        ];
        let en_content = en_lines.join("\n");
        
        let de_lines = vec![
            "Zeile 1 auf Deutsch",
            "Zeile 2 auf Deutsch",
            "Zeile 3 auf Deutsch", 
            "Zeile 4 auf Deutsch",
            "Zeile 5 auf Deutsch",
            "Zeile 6 auf Deutsch", // Extra line in German
        ];
        let de_content = de_lines.join("\n");
        
        let en_state = crate::services::LanguagePaneState {
            language: "en".to_string(),
            content: en_content,
            cursor_position: 0,
            scroll_position: 2.0, // Scrolled to line 2
            selection_start: None,
            selection_end: None,
            read_only: false,
            last_modified: chrono::Utc::now(),
        };
        
        let de_state = crate::services::LanguagePaneState {
            language: "de".to_string(),
            content: de_content,
            cursor_position: 0,
            scroll_position: 0.0,
            selection_start: None,
            selection_end: None,
            read_only: false,
            last_modified: chrono::Utc::now(),
        };
        
        self.sync_service.update_pane_state("en", en_state)?;
        self.sync_service.update_pane_state("de", de_state)?;
        
        // Test scroll synchronization
        self.sync_service.sync_scroll_move("en", 2.0)?;
        
        let updated_de_state = self.sync_service.get_pane_state("de")?.unwrap();
        
        // The scroll position should be synchronized proportionally
        assert!(updated_de_state.scroll_position > 0.0);
        assert!(updated_de_state.scroll_position <= 6.0); // Max lines in German
        
        println!("✓ Scroll synchronization test passed");
        Ok(())
    }
    
    /// Test content change synchronization
    pub async fn test_content_change_synchronization(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Subscribe to sync events
        let mut event_receiver = self.sync_service.subscribe_to_events();
        
        // Test content change
        let new_content = "Updated content for testing";
        self.sync_service.sync_content_change("en", new_content)?;
        
        // Wait for event to be processed
        sleep(Duration::from_millis(10)).await;
        
        // Verify the content was updated
        let updated_state = self.sync_service.get_pane_state("en")?.unwrap();
        assert_eq!(updated_state.content, new_content);
        
        // Try to receive the sync event (non-blocking)
        if let Ok(event) = event_receiver.try_recv() {
            assert_eq!(event.source_language, "en");
            assert_eq!(event.content.unwrap(), new_content);
        }
        
        println!("✓ Content change synchronization test passed");
        Ok(())
    }
    
    /// Test sync toggle functionality
    pub async fn test_sync_toggle_functionality(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Test disabling sync
        self.sync_service.toggle_sync(false)?;
        let config = self.sync_service.get_config()?;
        assert!(!config.sync_enabled);
        
        // Test cursor sync when main sync is disabled
        self.sync_service.sync_cursor_move("en", 10)?; // Should not sync
        
        // Re-enable sync
        self.sync_service.toggle_sync(true)?;
        let config = self.sync_service.get_config()?;
        assert!(config.sync_enabled);
        
        // Test individual sync toggles
        self.sync_service.toggle_cursor_sync(false)?;
        let config = self.sync_service.get_config()?;
        assert!(!config.sync_cursor);
        
        self.sync_service.toggle_scroll_sync(false)?;
        let config = self.sync_service.get_config()?;
        assert!(!config.sync_scroll);
        
        println!("✓ Sync toggle functionality test passed");
        Ok(())
    }
    
    /// Test language-specific syntax highlighting
    pub async fn test_language_syntax_highlighting(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Test English configuration
        let en_config = self.syntax_service.get_language_config("en");
        assert!(en_config.is_some());
        
        let en_config = en_config.unwrap();
        assert_eq!(en_config.language_code, "en");
        assert_eq!(en_config.text_direction, crate::services::TextDirection::LeftToRight);
        
        // Test German special characters
        let de_config = self.syntax_service.get_language_config("de");
        assert!(de_config.is_some());
        
        let special_char = self.syntax_service.is_special_character("de", "ß");
        assert!(special_char.is_some());
        
        // Test CSS generation
        let css = self.syntax_service.generate_language_css("en", "light");
        assert!(css.contains(".editor-en"));
        assert!(css.contains("direction: ltr"));
        
        println!("✓ Language syntax highlighting test passed");
        Ok(())
    }
    
    /// Test reset synchronization functionality
    pub async fn test_reset_synchronization(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Set up some state first
        let test_state = crate::services::LanguagePaneState {
            language: "en".to_string(),
            content: "Test content".to_string(),
            cursor_position: 5,
            scroll_position: 2.0,
            selection_start: Some(0),
            selection_end: Some(4),
            read_only: false,
            last_modified: chrono::Utc::now(),
        };
        
        self.sync_service.update_pane_state("en", test_state)?;
        
        // Reset synchronization
        self.sync_service.reset_sync()?;
        
        // Verify state was reset
        let reset_state = self.sync_service.get_pane_state("en")?.unwrap();
        assert_eq!(reset_state.cursor_position, 0);
        assert_eq!(reset_state.scroll_position, 0.0);
        
        println!("✓ Reset synchronization test passed");
        Ok(())
    }
    
    /// Run all split-pane editor tests
    pub async fn run_all_tests(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Running split-pane editor tests...\n");
        
        self.test_split_pane_configuration().await?;
        self.test_language_synchronization().await?;
        self.test_cursor_synchronization().await?;
        self.test_scroll_synchronization().await?;
        self.test_content_change_synchronization().await?;
        self.test_sync_toggle_functionality().await?;
        self.test_language_syntax_highlighting().await?;
        self.test_reset_synchronization().await?;
        
        println!("\n✅ All split-pane editor tests passed!");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_split_pane_editor_functionality() {
        let test_suite = SplitPaneEditorTests::new();
        test_suite.run_all_tests().await.expect("Tests should pass");
    }
    
    #[tokio::test]
    async fn test_individual_sync_components() {
        let test_suite = SplitPaneEditorTests::new();
        
        // Test each component individually
        test_suite.test_split_pane_configuration().await.expect("Configuration test should pass");
        test_suite.test_cursor_synchronization().await.expect("Cursor sync test should pass");
        test_suite.test_scroll_synchronization().await.expect("Scroll sync test should pass");
    }
    
    #[tokio::test]
    async fn test_language_specific_features() {
        let test_suite = SplitPaneEditorTests::new();
        test_suite.test_language_syntax_highlighting().await.expect("Language syntax test should pass");
    }
}