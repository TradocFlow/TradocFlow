use crate::services::{EditorSyncService, SplitOrientation, LanguageSyntaxService, LanguagePaneState};
use std::sync::Arc;

/// Integration test for split-pane editor functionality
pub async fn test_split_pane_editor_integration() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing split-pane editor integration...");
    
    // Create services
    let sync_service = Arc::new(EditorSyncService::new());
    let syntax_service = Arc::new(LanguageSyntaxService::new());
    
    // Test 1: Basic configuration
    let config = sync_service.get_config()?;
    assert_eq!(config.orientation, SplitOrientation::Horizontal);
    assert!(config.sync_enabled);
    println!("✓ Basic configuration test passed");
    
    // Test 2: Language setup
    sync_service.set_languages("en", "de")?;
    let updated_config = sync_service.get_config()?;
    assert_eq!(updated_config.left_language, "en");
    assert_eq!(updated_config.right_language, "de");
    println!("✓ Language setup test passed");
    
    // Test 3: Pane state management
    let en_state = LanguagePaneState {
        language: "en".to_string(),
        content: "Hello world\nThis is a test".to_string(),
        cursor_position: 12,
        scroll_position: 0.0,
        selection_start: None,
        selection_end: None,
        read_only: false,
        last_modified: chrono::Utc::now(),
    };
    
    sync_service.update_pane_state("en", en_state.clone())?;
    let retrieved_state = sync_service.get_pane_state("en")?.unwrap();
    assert_eq!(retrieved_state.content, en_state.content);
    println!("✓ Pane state management test passed");
    
    // Test 4: Cursor synchronization
    sync_service.sync_cursor_move("en", 5)?;
    println!("✓ Cursor synchronization test passed");
    
    // Test 5: Scroll synchronization
    sync_service.sync_scroll_move("en", 1.0)?;
    println!("✓ Scroll synchronization test passed");
    
    // Test 6: Content change synchronization
    sync_service.sync_content_change("en", "Updated content")?;
    let updated_state = sync_service.get_pane_state("en")?.unwrap();
    assert_eq!(updated_state.content, "Updated content");
    println!("✓ Content change synchronization test passed");
    
    // Test 7: Orientation change
    sync_service.set_orientation(SplitOrientation::Vertical)?;
    let config = sync_service.get_config()?;
    assert_eq!(config.orientation, SplitOrientation::Vertical);
    println!("✓ Orientation change test passed");
    
    // Test 8: Sync toggles
    sync_service.toggle_sync(false)?;
    let config = sync_service.get_config()?;
    assert!(!config.sync_enabled);
    
    sync_service.toggle_sync(true)?;
    let config = sync_service.get_config()?;
    assert!(config.sync_enabled);
    println!("✓ Sync toggle test passed");
    
    // Test 9: Language syntax service
    let en_config = syntax_service.get_language_config("en");
    assert!(en_config.is_some());
    
    let css = syntax_service.generate_language_css("en", "light");
    assert!(css.contains(".editor-en"));
    println!("✓ Language syntax service test passed");
    
    // Test 10: Reset functionality
    sync_service.reset_sync()?;
    let reset_state = sync_service.get_pane_state("en")?.unwrap();
    assert_eq!(reset_state.cursor_position, 0);
    assert_eq!(reset_state.scroll_position, 0.0);
    println!("✓ Reset functionality test passed");
    
    println!("✅ All split-pane editor integration tests passed!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_split_pane_integration() {
        test_split_pane_editor_integration().await.expect("Integration test should pass");
    }
}