use crate::gui::focus_management_bridge::FocusManagementUIBridge;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use anyhow::Result;

/// Focus management bridge specifically for the main window
/// Coordinates cursor blinking and focus switching across all editor layouts
pub struct MainWindowFocusBridge {
    focus_bridge: Arc<Mutex<FocusManagementUIBridge>>,
    layout_initialized: bool,
}

impl MainWindowFocusBridge {
    pub fn new() -> Self {
        Self {
            focus_bridge: Arc::new(Mutex::new(FocusManagementUIBridge::new())),
            layout_initialized: false,
        }
    }
    
    /// Initialize focus management for a specific layout
    pub fn initialize_layout(&mut self, layout: &str) -> Result<()> {
        if let Ok(mut bridge) = self.focus_bridge.lock() {
            bridge.initialize_for_layout(layout)?;
            self.layout_initialized = true;
        }
        Ok(())
    }
    
    /// Handle layout change - reinitialize focus management
    pub fn handle_layout_change(&mut self, new_layout: &str) -> Result<()> {
        if let Ok(mut bridge) = self.focus_bridge.lock() {
            bridge.update_layout(new_layout)?;
        }
        Ok(())
    }
    
    /// Request focus for a specific editor
    pub fn request_editor_focus(&self, editor_id: &str, pane_id: &str) -> Result<HashMap<String, bool>> {
        if let Ok(bridge) = self.focus_bridge.lock() {
            let result = bridge.request_editor_focus(pane_id, editor_id)?;
            
            // Convert to UI state map
            let mut ui_state = HashMap::new();
            for (id, state) in &result.editor_states {
                ui_state.insert(id.clone(), state.has_focus);
            }
            
            Ok(ui_state)
        } else {
            Ok(HashMap::new())
        }
    }
    
    /// Release focus from a specific editor
    pub fn release_editor_focus(&self, editor_id: &str) -> Result<HashMap<String, bool>> {
        if let Ok(bridge) = self.focus_bridge.lock() {
            let result = bridge.release_editor_focus(editor_id)?;
            
            // Convert to UI state map
            let mut ui_state = HashMap::new();
            for (id, state) in &result.editor_states {
                ui_state.insert(id.clone(), state.has_focus);
            }
            
            Ok(ui_state)
        } else {
            Ok(HashMap::new())
        }
    }
    
    /// Tab to next editor (Ctrl+Tab or Tab navigation)
    pub fn tab_to_next_editor(&self) -> Result<(String, HashMap<String, bool>)> {
        if let Ok(bridge) = self.focus_bridge.lock() {
            let result = bridge.tab_to_next_editor()?;
            
            // Convert to UI state map
            let mut ui_state = HashMap::new();
            for (id, state) in &result.editor_states {
                ui_state.insert(id.clone(), state.has_focus);
            }
            
            let active_editor = result.active_editor_id.unwrap_or_default();
            Ok((active_editor, ui_state))
        } else {
            Ok((String::new(), HashMap::new()))
        }
    }
    
    /// Tab to previous editor (Ctrl+Shift+Tab)
    pub fn tab_to_previous_editor(&self) -> Result<(String, HashMap<String, bool>)> {
        if let Ok(bridge) = self.focus_bridge.lock() {
            let result = bridge.tab_to_previous_editor()?;
            
            // Convert to UI state map
            let mut ui_state = HashMap::new();
            for (id, state) in &result.editor_states {
                ui_state.insert(id.clone(), state.has_focus);
            }
            
            let active_editor = result.active_editor_id.unwrap_or_default();
            Ok((active_editor, ui_state))
        } else {
            Ok((String::new(), HashMap::new()))
        }
    }
    
    /// Focus editor by index (Alt+1, Alt+2, etc.)
    pub fn focus_editor_by_index(&self, index: usize) -> Result<(String, HashMap<String, bool>)> {
        if let Ok(bridge) = self.focus_bridge.lock() {
            let result = bridge.focus_editor_by_index(index)?;
            
            // Convert to UI state map
            let mut ui_state = HashMap::new();
            for (id, state) in &result.editor_states {
                ui_state.insert(id.clone(), state.has_focus);
            }
            
            let active_editor = result.active_editor_id.unwrap_or_default();
            Ok((active_editor, ui_state))
        } else {
            Ok((String::new(), HashMap::new()))
        }
    }
    
    /// Update cursor position for an editor
    pub fn update_cursor_position(&self, editor_id: &str, position: usize) -> Result<()> {
        if let Ok(bridge) = self.focus_bridge.lock() {
            bridge.update_cursor_position(editor_id, position)?;
        }
        Ok(())
    }
    
    /// Update text selection for an editor
    pub fn update_selection(&self, editor_id: &str, start: usize, end: usize) -> Result<()> {
        if let Ok(bridge) = self.focus_bridge.lock() {
            bridge.update_selection(editor_id, start, end)?;
        }
        Ok(())
    }
    
    /// Enable or disable cursor blinking globally
    pub fn set_cursor_blink_enabled(&self, enabled: bool) -> Result<()> {
        if let Ok(bridge) = self.focus_bridge.lock() {
            bridge.set_cursor_blink_enabled(enabled)?;
        }
        Ok(())
    }
    
    /// Process cursor blink tick (should be called every 500ms)
    pub fn tick_cursor_blink(&self) -> Result<HashMap<String, bool>> {
        if let Ok(bridge) = self.focus_bridge.lock() {
            bridge.tick_cursor_blink()?;
            
            // Return cursor visibility state for each editor
            let focus_states = bridge.get_editor_focus_states();
            let mut cursor_states = HashMap::new();
            for (editor_id, has_focus) in focus_states {
                cursor_states.insert(editor_id, has_focus);
            }
            
            Ok(cursor_states)
        } else {
            Ok(HashMap::new())
        }
    }
    
    /// Handle keyboard shortcuts for focus navigation
    pub fn handle_keyboard_shortcut(&self, key: &str, ctrl: bool, shift: bool, alt: bool) -> Result<Option<(String, HashMap<String, bool>)>> {
        if let Ok(bridge) = self.focus_bridge.lock() {
            if bridge.handle_keyboard_shortcut(key, ctrl, shift, alt)? {
                // Get current state to return UI updates
                let ui_state = bridge.get_editor_focus_states();
                let active_editor = bridge.get_active_editor_id()?.unwrap_or_default();
                Ok(Some((active_editor, ui_state)))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
    
    /// Get the currently active editor and pane IDs
    pub fn get_active_editor_info(&self) -> Result<(String, String)> {
        if let Ok(bridge) = self.focus_bridge.lock() {
            let active_editor = bridge.get_active_editor_id()?.unwrap_or_default();
            let active_pane = bridge.get_active_pane_id()?.unwrap_or_default();
            Ok((active_editor, active_pane))
        } else {
            Ok((String::new(), String::new()))
        }
    }
    
    /// Get focus states for all editors
    pub fn get_all_focus_states(&self) -> HashMap<String, bool> {
        if let Ok(bridge) = self.focus_bridge.lock() {
            bridge.get_editor_focus_states()
        } else {
            HashMap::new()
        }
    }
    
    /// Check if a specific editor has focus
    pub fn has_editor_focus(&self, editor_id: &str) -> bool {
        if let Ok(bridge) = self.focus_bridge.lock() {
            bridge.has_editor_focus(editor_id)
        } else {
            false
        }
    }
    
    /// Check if a specific editor should show cursor
    pub fn should_show_cursor(&self, editor_id: &str) -> bool {
        self.has_editor_focus(editor_id)
    }
    
    /// Get editor ID for a specific pane (based on current layout)
    pub fn get_editor_id_for_pane(&self, pane_id: &str) -> String {
        if let Ok(bridge) = self.focus_bridge.lock() {
            bridge.get_editor_id_for_pane(pane_id)
        } else {
            format!("{}-editor", pane_id)
        }
    }
    
    /// Get pane ID for a specific editor (based on current layout)
    pub fn get_pane_id_for_editor(&self, editor_id: &str) -> String {
        if let Ok(bridge) = self.focus_bridge.lock() {
            bridge.get_pane_id_for_editor(editor_id)
        } else {
            editor_id.replace("-editor", "-pane")
        }
    }
    
    /// Start background processing for focus management
    pub async fn start_background_processing(&mut self) -> Result<()> {
        if let Ok(mut bridge) = self.focus_bridge.lock() {
            bridge.start_background_processing().await?;
        }
        Ok(())
    }
    
    /// Get debug information about current focus state
    pub fn get_debug_info(&self) -> String {
        if let Ok(bridge) = self.focus_bridge.lock() {
            bridge.get_debug_info().unwrap_or_else(|e| format!("Error getting debug info: {}", e))
        } else {
            "Failed to lock focus bridge".to_string()
        }
    }
}

impl Default for MainWindowFocusBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for Slint integration
pub mod slint_integration {
    use super::*;
    use slint::SharedString;
    
    /// Convert focus state to SharedString for Slint
    pub fn focus_state_to_shared_string(has_focus: bool) -> SharedString {
        SharedString::from(if has_focus { "true" } else { "false" })
    }
    
    /// Convert editor ID to SharedString for Slint
    pub fn editor_id_to_shared_string(editor_id: &str) -> SharedString {
        SharedString::from(editor_id)
    }
    
    /// Create focus state update callback for Slint
    pub fn create_focus_state_callback<F>(callback: F) -> impl FnMut(HashMap<String, bool>) 
    where 
        F: FnMut(&str, bool) + 'static
    {
        let mut callback = callback;
        move |focus_states: HashMap<String, bool>| {
            for (editor_id, has_focus) in focus_states {
                callback(&editor_id, has_focus);
            }
        }
    }
    
    /// Map layout string to editor count
    pub fn get_editor_count_for_layout(layout: &str) -> usize {
        match layout {
            "single" => 1,
            "horizontal" | "vertical" => 2,
            "grid_2x2" => 4,
            _ => 1,
        }
    }
    
    /// Get all editor IDs for a layout
    pub fn get_editor_ids_for_layout(layout: &str) -> Vec<String> {
        match layout {
            "single" => vec!["single-editor".to_string()],
            "horizontal" | "vertical" => vec![
                "left-editor".to_string(),
                "right-editor".to_string(),
            ],
            "grid_2x2" => vec![
                "pane-1-editor".to_string(),
                "pane-2-editor".to_string(),
                "pane-3-editor".to_string(),
                "pane-4-editor".to_string(),
            ],
            _ => vec!["single-editor".to_string()],
        }
    }
    
    /// Get all pane IDs for a layout
    pub fn get_pane_ids_for_layout(layout: &str) -> Vec<String> {
        match layout {
            "single" => vec!["single-pane".to_string()],
            "horizontal" | "vertical" => vec![
                "left-pane".to_string(),
                "right-pane".to_string(),
            ],
            "grid_2x2" => vec![
                "pane-1".to_string(),
                "pane-2".to_string(),
                "pane-3".to_string(),
                "pane-4".to_string(),
            ],
            _ => vec!["single-pane".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    
    #[tokio::test]
    async fn test_main_window_focus_bridge() {
        let mut bridge = MainWindowFocusBridge::new();
        
        // Test layout initialization
        bridge.initialize_layout("single").unwrap();
        let (active_editor, _) = bridge.get_active_editor_info().unwrap();
        assert_eq!(active_editor, "single-editor");
        
        // Test layout change
        bridge.handle_layout_change("horizontal").unwrap();
        let focus_states = bridge.get_all_focus_states();
        assert!(focus_states.contains_key("left-editor"));
        assert!(focus_states.contains_key("right-editor"));
        
        // Test focus switching
        let focus_states = bridge.request_editor_focus("right-editor", "right-pane").unwrap();
        assert_eq!(focus_states.get("right-editor"), Some(&true));
        assert_eq!(focus_states.get("left-editor"), Some(&false));
        
        // Test tab navigation
        let (active_editor, focus_states) = bridge.tab_to_next_editor().unwrap();
        assert_eq!(active_editor, "left-editor");
        assert_eq!(focus_states.get("left-editor"), Some(&true));
        assert_eq!(focus_states.get("right-editor"), Some(&false));
    }
    
    #[tokio::test]
    async fn test_keyboard_shortcuts() {
        let mut bridge = MainWindowFocusBridge::new();
        bridge.initialize_layout("grid_2x2").unwrap();
        
        // Test Alt+1 shortcut
        let result = bridge.handle_keyboard_shortcut("1", false, false, true).unwrap();
        assert!(result.is_some());
        
        let (active_editor, focus_states) = result.unwrap();
        assert_eq!(active_editor, "pane-1-editor");
        assert_eq!(focus_states.get("pane-1-editor"), Some(&true));
        
        // Test Alt+2 shortcut
        let result = bridge.handle_keyboard_shortcut("2", false, false, true).unwrap();
        assert!(result.is_some());
        
        let (active_editor, focus_states) = result.unwrap();
        assert_eq!(active_editor, "pane-2-editor");
        assert_eq!(focus_states.get("pane-2-editor"), Some(&true));
        assert_eq!(focus_states.get("pane-1-editor"), Some(&false));
    }
    
    #[tokio::test]
    async fn test_cursor_blinking() {
        let mut bridge = MainWindowFocusBridge::new();
        bridge.initialize_layout("single").unwrap();
        
        // Enable cursor blinking
        bridge.set_cursor_blink_enabled(true).unwrap();
        
        // Request focus for an editor
        bridge.request_editor_focus("single-editor", "single-pane").unwrap();
        
        // Test cursor position update
        bridge.update_cursor_position("single-editor", 42).unwrap();
        
        // Test selection update
        bridge.update_selection("single-editor", 10, 20).unwrap();
        
        // Test cursor blink tick
        let cursor_states = bridge.tick_cursor_blink().unwrap();
        assert!(cursor_states.contains_key("single-editor"));
    }
    
    #[test]
    fn test_slint_integration_helpers() {
        use slint_integration::*;
        
        // Test editor count for layouts
        assert_eq!(get_editor_count_for_layout("single"), 1);
        assert_eq!(get_editor_count_for_layout("horizontal"), 2);
        assert_eq!(get_editor_count_for_layout("grid_2x2"), 4);
        
        // Test editor IDs for layouts
        let single_editors = get_editor_ids_for_layout("single");
        assert_eq!(single_editors.len(), 1);
        assert_eq!(single_editors[0], "single-editor");
        
        let grid_editors = get_editor_ids_for_layout("grid_2x2");
        assert_eq!(grid_editors.len(), 4);
        assert!(grid_editors.contains(&"pane-1-editor".to_string()));
        assert!(grid_editors.contains(&"pane-4-editor".to_string()));
        
        // Test pane IDs for layouts
        let horizontal_panes = get_pane_ids_for_layout("horizontal");
        assert_eq!(horizontal_panes.len(), 2);
        assert!(horizontal_panes.contains(&"left-pane".to_string()));
        assert!(horizontal_panes.contains(&"right-pane".to_string()));
    }
}