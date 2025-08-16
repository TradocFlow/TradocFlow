use anyhow::Result;
use tokio::time::{sleep, Duration};
use crate::services::focus_management_service::{FocusManagementService, EditorFocusState};
use crate::gui::focus_management_bridge::FocusManagementUIBridge;

/// Integration tests for the focus management system
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_single_pane_focus_management() {
        let mut bridge = FocusManagementUIBridge::new();
        
        // Initialize for single layout
        bridge.initialize_for_layout("single").unwrap();
        
        // Test basic focus management
        let result = bridge.request_editor_focus("single-pane", "single-editor").unwrap();
        assert_eq!(result.active_editor_id, Some("single-editor".to_string()));
        assert_eq!(result.active_pane_id, Some("single-pane".to_string()));
        
        // Check focus state
        assert!(bridge.has_editor_focus("single-editor"));
        
        // Test cursor position update
        bridge.update_cursor_position("single-editor", 42).unwrap();
        
        // Test selection update
        bridge.update_selection("single-editor", 10, 20).unwrap();
        
        println!("✓ Single pane focus management test passed");
    }
    
    #[tokio::test]
    async fn test_horizontal_split_focus_management() {
        let mut bridge = FocusManagementUIBridge::new();
        
        // Initialize for horizontal layout
        bridge.initialize_for_layout("horizontal").unwrap();
        
        // Test focus on left editor
        let result = bridge.request_editor_focus("left-pane", "left-editor").unwrap();
        assert_eq!(result.active_editor_id, Some("left-editor".to_string()));
        assert!(bridge.has_editor_focus("left-editor"));
        assert!(!bridge.has_editor_focus("right-editor"));
        
        // Tab to next editor (should switch to right)
        let result = bridge.tab_to_next_editor().unwrap();
        assert_eq!(result.active_editor_id, Some("right-editor".to_string()));
        assert!(!bridge.has_editor_focus("left-editor"));
        assert!(bridge.has_editor_focus("right-editor"));
        
        // Tab to next again (should wrap to left)
        let result = bridge.tab_to_next_editor().unwrap();
        assert_eq!(result.active_editor_id, Some("left-editor".to_string()));
        assert!(bridge.has_editor_focus("left-editor"));
        assert!(!bridge.has_editor_focus("right-editor"));
        
        // Tab to previous (should go to right)
        let result = bridge.tab_to_previous_editor().unwrap();
        assert_eq!(result.active_editor_id, Some("right-editor".to_string()));
        assert!(!bridge.has_editor_focus("left-editor"));
        assert!(bridge.has_editor_focus("right-editor"));
        
        println!("✓ Horizontal split focus management test passed");
    }
    
    #[tokio::test]
    async fn test_grid_2x2_focus_management() {
        let mut bridge = FocusManagementUIBridge::new();
        
        // Initialize for grid layout
        bridge.initialize_for_layout("grid_2x2").unwrap();
        
        // Test focus on each pane
        for i in 1..=4 {
            let editor_id = format!("pane-{}-editor", i);
            let pane_id = format!("pane-{}", i);
            
            let result = bridge.request_editor_focus(&pane_id, &editor_id).unwrap();
            assert_eq!(result.active_editor_id, Some(editor_id.clone()));
            assert_eq!(result.active_pane_id, Some(pane_id));
            assert!(bridge.has_editor_focus(&editor_id));
            
            // Check that other editors don't have focus
            for j in 1..=4 {
                if i != j {
                    let other_editor_id = format!("pane-{}-editor", j);
                    assert!(!bridge.has_editor_focus(&other_editor_id));
                }
            }
        }
        
        // Test tab navigation through all 4 panes
        bridge.request_editor_focus("pane-1", "pane-1-editor").unwrap();
        
        for expected_pane in 2..=4 {
            let result = bridge.tab_to_next_editor().unwrap();
            let expected_id = format!("pane-{}-editor", expected_pane);
            assert_eq!(result.active_editor_id, Some(expected_id));
        }
        
        // Tab once more should wrap to pane 1
        let result = bridge.tab_to_next_editor().unwrap();
        assert_eq!(result.active_editor_id, Some("pane-1-editor".to_string()));
        
        println!("✓ Grid 2x2 focus management test passed");
    }
    
    #[tokio::test]
    async fn test_layout_switching() {
        let mut bridge = FocusManagementUIBridge::new();
        
        // Start with single layout
        bridge.initialize_for_layout("single").unwrap();
        assert_eq!(bridge.get_active_editor_id().unwrap(), Some("single-editor".to_string()));
        
        // Switch to horizontal layout
        bridge.update_layout("horizontal").unwrap();
        let active_id = bridge.get_active_editor_id().unwrap();
        assert!(active_id == Some("left-editor".to_string()) || active_id == Some("right-editor".to_string()));
        
        // Switch to grid layout
        bridge.update_layout("grid_2x2").unwrap();
        let active_id = bridge.get_active_editor_id().unwrap();
        assert!(active_id.as_ref().map(|id| id.starts_with("pane-")).unwrap_or(false));
        
        // Switch back to single
        bridge.update_layout("single").unwrap();
        assert_eq!(bridge.get_active_editor_id().unwrap(), Some("single-editor".to_string()));
        
        println!("✓ Layout switching test passed");
    }
    
    #[tokio::test]
    async fn test_keyboard_shortcuts() {
        let mut bridge = FocusManagementUIBridge::new();
        bridge.initialize_for_layout("grid_2x2").unwrap();
        
        // Test Alt+1 through Alt+4 shortcuts
        for i in 1..=4 {
            let key = i.to_string();
            let handled = bridge.handle_keyboard_shortcut(&key, false, false, true).unwrap();
            assert!(handled, "Alt+{} should be handled", i);
            
            let expected_id = format!("pane-{}-editor", i);
            let active_id = bridge.get_active_editor_id().unwrap();
            assert_eq!(active_id, Some(expected_id), "Alt+{} should focus pane {}", i, i);
        }
        
        // Test Ctrl+Tab (next editor)
        bridge.focus_editor_by_index(0).unwrap(); // Start at pane 1
        let handled = bridge.handle_keyboard_shortcut("Tab", true, false, false).unwrap();
        assert!(handled, "Ctrl+Tab should be handled");
        
        let active_id = bridge.get_active_editor_id().unwrap();
        assert_eq!(active_id, Some("pane-2-editor".to_string()), "Ctrl+Tab should move to next editor");
        
        // Test Ctrl+Shift+Tab (previous editor)
        let handled = bridge.handle_keyboard_shortcut("Tab", true, true, false).unwrap();
        assert!(handled, "Ctrl+Shift+Tab should be handled");
        
        let active_id = bridge.get_active_editor_id().unwrap();
        assert_eq!(active_id, Some("pane-1-editor".to_string()), "Ctrl+Shift+Tab should move to previous editor");
        
        println!("✓ Keyboard shortcuts test passed");
    }
    
    #[tokio::test]
    async fn test_cursor_blinking() {
        let mut bridge = FocusManagementUIBridge::new();
        bridge.initialize_for_layout("single").unwrap();
        
        // Focus an editor
        bridge.request_editor_focus("single-pane", "single-editor").unwrap();
        
        // Enable cursor blinking
        bridge.set_cursor_blink_enabled(true).unwrap();
        
        // Test cursor blink tick
        bridge.tick_cursor_blink().unwrap();
        
        // Disable cursor blinking
        bridge.set_cursor_blink_enabled(false).unwrap();
        bridge.tick_cursor_blink().unwrap();
        
        println!("✓ Cursor blinking test passed");
    }
    
    #[tokio::test]
    async fn test_editor_pane_mapping() {
        let mut bridge = FocusManagementUIBridge::new();
        
        // Test single layout mapping
        bridge.initialize_for_layout("single").unwrap();
        assert_eq!(bridge.get_editor_id_for_pane("single-pane"), "single-editor");
        assert_eq!(bridge.get_pane_id_for_editor("single-editor"), "single-pane");
        
        // Test horizontal layout mapping
        bridge.initialize_for_layout("horizontal").unwrap();
        assert_eq!(bridge.get_editor_id_for_pane("left-pane"), "left-editor");
        assert_eq!(bridge.get_editor_id_for_pane("right-pane"), "right-editor");
        assert_eq!(bridge.get_pane_id_for_editor("left-editor"), "left-pane");
        assert_eq!(bridge.get_pane_id_for_editor("right-editor"), "right-pane");
        
        // Test grid layout mapping
        bridge.initialize_for_layout("grid_2x2").unwrap();
        for i in 1..=4 {
            let pane_id = format!("pane-{}", i);
            let editor_id = format!("pane-{}-editor", i);
            assert_eq!(bridge.get_editor_id_for_pane(&pane_id), editor_id);
            assert_eq!(bridge.get_pane_id_for_editor(&editor_id), pane_id);
        }
        
        println!("✓ Editor pane mapping test passed");
    }
    
    #[tokio::test]
    async fn test_focus_state_synchronization() {
        let mut bridge = FocusManagementUIBridge::new();
        bridge.initialize_for_layout("horizontal").unwrap();
        
        // Request focus for left editor
        bridge.request_editor_focus("left-pane", "left-editor").unwrap();
        
        // Check that focus states are synchronized
        let focus_states = bridge.get_editor_focus_states();
        assert_eq!(focus_states.get("left-editor"), Some(&true));
        assert_eq!(focus_states.get("right-editor"), Some(&false));
        
        // Switch focus to right editor
        bridge.request_editor_focus("right-pane", "right-editor").unwrap();
        
        // Check that focus states are updated
        let focus_states = bridge.get_editor_focus_states();
        assert_eq!(focus_states.get("left-editor"), Some(&false));
        assert_eq!(focus_states.get("right-editor"), Some(&true));
        
        // Release focus from right editor
        bridge.release_editor_focus("right-editor").unwrap();
        
        // Check that focus state is cleared
        let focus_states = bridge.get_editor_focus_states();
        assert_eq!(focus_states.get("right-editor"), Some(&false));
        
        println!("✓ Focus state synchronization test passed");
    }
    
    #[tokio::test]
    async fn test_cursor_and_selection_updates() {
        let mut bridge = FocusManagementUIBridge::new();
        bridge.initialize_for_layout("single").unwrap();
        
        // Focus an editor
        bridge.request_editor_focus("single-pane", "single-editor").unwrap();
        
        // Test cursor position updates
        bridge.update_cursor_position("single-editor", 100).unwrap();
        bridge.update_cursor_position("single-editor", 200).unwrap();
        
        // Test selection updates
        bridge.update_selection("single-editor", 50, 150).unwrap();
        bridge.update_selection("single-editor", 0, 0).unwrap(); // Clear selection
        
        println!("✓ Cursor and selection updates test passed");
    }
    
    #[tokio::test]
    async fn test_debug_information() {
        let mut bridge = FocusManagementUIBridge::new();
        bridge.initialize_for_layout("grid_2x2").unwrap();
        
        // Get debug information
        let debug_info = bridge.get_debug_info().unwrap();
        assert!(debug_info.contains("Layout: grid_2x2"));
        assert!(debug_info.contains("Pane Count: 4"));
        
        println!("Debug info: {}", debug_info);
        println!("✓ Debug information test passed");
    }
    
    /// Comprehensive integration test that simulates real usage scenarios
    #[tokio::test]
    async fn test_comprehensive_focus_workflow() {
        let mut bridge = FocusManagementUIBridge::new();
        
        // Scenario 1: User starts with single editor, types content
        bridge.initialize_for_layout("single").unwrap();
        bridge.request_editor_focus("single-pane", "single-editor").unwrap();
        bridge.update_cursor_position("single-editor", 0).unwrap();
        
        // Simulate typing
        for pos in 1..=50 {
            bridge.update_cursor_position("single-editor", pos).unwrap();
        }
        
        // Scenario 2: User switches to horizontal split for translation
        bridge.update_layout("horizontal").unwrap();
        assert!(bridge.has_editor_focus("left-editor")); // Should maintain focus
        
        // User clicks on right pane to start translating
        bridge.request_editor_focus("right-pane", "right-editor").unwrap();
        assert!(!bridge.has_editor_focus("left-editor"));
        assert!(bridge.has_editor_focus("right-editor"));
        
        // Simulate typing in right pane
        for pos in 1..=30 {
            bridge.update_cursor_position("right-editor", pos).unwrap();
        }
        
        // User uses Ctrl+Tab to switch between panes while working
        bridge.tab_to_next_editor().unwrap(); // Should go to left
        assert!(bridge.has_editor_focus("left-editor"));
        
        bridge.tab_to_next_editor().unwrap(); // Should go to right
        assert!(bridge.has_editor_focus("right-editor"));
        
        // Scenario 3: User switches to grid layout for multiple languages
        bridge.update_layout("grid_2x2").unwrap();
        
        // User uses Alt+1-4 to quickly navigate between panes
        bridge.handle_keyboard_shortcut("1", false, false, true).unwrap();
        assert!(bridge.has_editor_focus("pane-1-editor"));
        
        bridge.handle_keyboard_shortcut("3", false, false, true).unwrap();
        assert!(bridge.has_editor_focus("pane-3-editor"));
        
        // User selects text in current pane
        bridge.update_selection("pane-3-editor", 10, 25).unwrap();
        
        // Scenario 4: User switches back to single pane to focus
        bridge.update_layout("single").unwrap();
        assert!(bridge.has_editor_focus("single-editor"));
        
        println!("✓ Comprehensive focus workflow test passed");
    }
    
    /// Test error handling and edge cases
    #[tokio::test]
    async fn test_error_handling_and_edge_cases() {
        let mut bridge = FocusManagementUIBridge::new();
        bridge.initialize_for_layout("single").unwrap();
        
        // Test requesting focus for non-existent editor
        let result = bridge.request_editor_focus("nonexistent-pane", "nonexistent-editor");
        assert!(result.is_ok()); // Should handle gracefully by creating the editor
        
        // Test releasing focus from non-existent editor
        let result = bridge.release_editor_focus("nonexistent-editor");
        assert!(result.is_ok()); // Should handle gracefully
        
        // Test updating cursor position for non-existent editor
        let result = bridge.update_cursor_position("nonexistent-editor", 42);
        assert!(result.is_ok()); // Should handle gracefully
        
        // Test keyboard shortcuts with unsupported keys
        let handled = bridge.handle_keyboard_shortcut("x", false, false, true).unwrap();
        assert!(!handled); // Should not handle unsupported shortcuts
        
        // Test invalid layout
        let result = bridge.initialize_for_layout("invalid_layout");
        assert!(result.is_err()); // Should return error for invalid layout
        
        println!("✓ Error handling and edge cases test passed");
    }
}

/// Helper function to run all integration tests
pub async fn run_focus_management_integration_tests() -> Result<()> {
    println!("🧪 Running focus management integration tests...\n");
    
    // Run all tests
    tests::test_single_pane_focus_management().await;
    tests::test_horizontal_split_focus_management().await;
    tests::test_grid_2x2_focus_management().await;
    tests::test_layout_switching().await;
    tests::test_keyboard_shortcuts().await;
    tests::test_cursor_blinking().await;
    tests::test_editor_pane_mapping().await;
    tests::test_focus_state_synchronization().await;
    tests::test_cursor_and_selection_updates().await;
    tests::test_debug_information().await;
    tests::test_comprehensive_focus_workflow().await;
    tests::test_error_handling_and_edge_cases().await;
    
    println!("\n🎉 All focus management integration tests passed!");
    println!("\n📋 Test Summary:");
    println!("✅ Single pane focus management");
    println!("✅ Horizontal split focus management");
    println!("✅ Grid 2x2 focus management");
    println!("✅ Layout switching");
    println!("✅ Keyboard shortcuts (Ctrl+Tab, Alt+1-4)");
    println!("✅ Cursor blinking");
    println!("✅ Editor-pane mapping");
    println!("✅ Focus state synchronization");
    println!("✅ Cursor and selection updates");
    println!("✅ Debug information");
    println!("✅ Comprehensive workflow simulation");
    println!("✅ Error handling and edge cases");
    
    Ok(())
}

#[cfg(test)]
mod integration_test_runner {
    use super::*;
    
    #[tokio::test]
    async fn run_all_focus_management_tests() {
        run_focus_management_integration_tests().await.unwrap();
    }
}