use crate::services::focus_management_service::{
    FocusManagementBridge as FocusBridge, 
    FocusUpdateResult, 
    EditorFocusState
};
use slint::{ComponentHandle, SharedString, VecModel, ModelRc};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use anyhow::Result;

/// Bridge struct for integrating focus management with Slint UI
pub struct FocusManagementUIBridge {
    focus_bridge: Arc<Mutex<FocusBridge>>,
    current_layout: String,
    pane_count: usize,
    editor_focus_states: Arc<Mutex<HashMap<String, bool>>>,
}

impl FocusManagementUIBridge {
    pub fn new() -> Self {
        Self {
            focus_bridge: Arc::new(Mutex::new(FocusBridge::new())),
            current_layout: "single".to_string(),
            pane_count: 1,
            editor_focus_states: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Initialize the focus management system for a specific layout
    pub fn initialize_for_layout(&mut self, layout: &str) -> Result<()> {
        self.current_layout = layout.to_string();
        
        // Determine pane count and register editors based on layout
        match layout {
            "single" => {
                self.pane_count = 1;
                self.focus_bridge.lock().unwrap().register_editor(
                    "single-editor".to_string(),
                    "single-pane".to_string(),
                    "en".to_string()
                )?;
            }
            "horizontal" | "vertical" => {
                self.pane_count = 2;
                self.focus_bridge.lock().unwrap().register_editor(
                    "left-editor".to_string(),
                    "left-pane".to_string(),
                    "en".to_string()
                )?;
                self.focus_bridge.lock().unwrap().register_editor(
                    "right-editor".to_string(),
                    "right-pane".to_string(),
                    "de".to_string()
                )?;
            }
            "grid_2x2" => {
                self.pane_count = 4;
                let languages = ["en", "de", "fr", "es"];
                for i in 1..=4 {
                    self.focus_bridge.lock().unwrap().register_editor(
                        format!("pane-{}-editor", i),
                        format!("pane-{}", i),
                        languages[i - 1].to_string()
                    )?;
                }
            }
            _ => {
                return Err(anyhow::anyhow!("Unsupported layout: {}", layout));
            }
        }
        
        // Update layout in the service
        self.focus_bridge.lock().unwrap().set_layout(layout.to_string(), self.pane_count)?;
        
        Ok(())
    }
    
    /// Handle focus request from an editor pane
    pub fn request_editor_focus(&self, pane_id: &str, editor_id: &str) -> Result<FocusUpdateResult> {
        let result = self.focus_bridge.lock().unwrap().request_focus(editor_id.to_string(), pane_id.to_string())?;
        
        // Update local state
        if let Ok(mut states) = self.editor_focus_states.lock() {
            // Clear all focus states
            for (_, has_focus) in states.iter_mut() {
                *has_focus = false;
            }
            // Set focus for the requested editor
            states.insert(editor_id.to_string(), true);
        }
        
        Ok(result)
    }
    
    /// Handle focus release from an editor pane
    pub fn release_editor_focus(&self, editor_id: &str) -> Result<FocusUpdateResult> {
        let result = self.focus_bridge.lock().unwrap().release_focus(editor_id.to_string())?;
        
        // Update local state
        if let Ok(mut states) = self.editor_focus_states.lock() {
            states.insert(editor_id.to_string(), false);
        }
        
        Ok(result)
    }
    
    /// Tab to the next editor
    pub fn tab_to_next_editor(&self) -> Result<FocusUpdateResult> {
        let result = self.focus_bridge.lock().unwrap().tab_to_next_editor()?;
        self.update_focus_states_from_result(&result)?;
        Ok(result)
    }
    
    /// Tab to the previous editor
    pub fn tab_to_previous_editor(&self) -> Result<FocusUpdateResult> {
        let result = self.focus_bridge.lock().unwrap().tab_to_previous_editor()?;
        self.update_focus_states_from_result(&result)?;
        Ok(result)
    }
    
    /// Focus a specific editor by index (0-based)
    pub fn focus_editor_by_index(&self, index: usize) -> Result<FocusUpdateResult> {
        let result = self.focus_bridge.lock().unwrap().focus_editor_by_index(index)?;
        self.update_focus_states_from_result(&result)?;
        Ok(result)
    }
    
    /// Update cursor position for an editor
    pub fn update_cursor_position(&self, editor_id: &str, position: usize) -> Result<()> {
        self.focus_bridge.lock().unwrap().update_cursor_position(editor_id.to_string(), position)?;
        Ok(())
    }
    
    /// Update selection for an editor
    pub fn update_selection(&self, editor_id: &str, start: usize, end: usize) -> Result<()> {
        self.focus_bridge.lock().unwrap().update_selection(editor_id.to_string(), start, end)?;
        Ok(())
    }
    
    /// Enable or disable cursor blinking globally
    pub fn set_cursor_blink_enabled(&self, enabled: bool) -> Result<()> {
        self.focus_bridge.lock().unwrap().set_cursor_blink_enabled(enabled)?;
        Ok(())
    }
    
    /// Process cursor blink tick (should be called periodically)
    pub fn tick_cursor_blink(&self) -> Result<()> {
        self.focus_bridge.lock().unwrap().tick_cursor_blink()?;
        Ok(())
    }
    
    /// Check if a specific editor has focus
    pub fn has_editor_focus(&self, editor_id: &str) -> bool {
        if let Ok(states) = self.editor_focus_states.lock() {
            states.get(editor_id).copied().unwrap_or(false)
        } else {
            false
        }
    }
    
    /// Get the currently active editor ID
    pub fn get_active_editor_id(&self) -> Result<Option<String>> {
        let state = self.focus_bridge.lock().unwrap().get_current_state()?;
        Ok(state.active_editor_id)
    }
    
    /// Get the currently active pane ID
    pub fn get_active_pane_id(&self) -> Result<Option<String>> {
        let state = self.focus_bridge.lock().unwrap().get_current_state()?;
        Ok(state.active_pane_id)
    }
    
    /// Handle keyboard shortcuts for focus navigation
    pub fn handle_keyboard_shortcut(&self, key: &str, ctrl: bool, shift: bool, alt: bool) -> Result<bool> {
        let mut modifiers = Vec::new();
        if ctrl { modifiers.push("Ctrl"); }
        if shift { modifiers.push("Shift"); }
        if alt { modifiers.push("Alt"); }
        
        if let Some(result) = self.focus_bridge.lock().unwrap().handle_keyboard_shortcut(key, &modifiers)? {
            self.update_focus_states_from_result(&result)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Update layout configuration
    pub fn update_layout(&mut self, layout: &str) -> Result<()> {
        if layout != self.current_layout {
            // Re-initialize for the new layout
            self.initialize_for_layout(layout)?;
        }
        Ok(())
    }
    
    /// Get editor focus states for UI updates
    pub fn get_editor_focus_states(&self) -> HashMap<String, bool> {
        if let Ok(states) = self.editor_focus_states.lock() {
            states.clone()
        } else {
            HashMap::new()
        }
    }
    
    /// Get editor ID for a pane based on current layout
    pub fn get_editor_id_for_pane(&self, pane_id: &str) -> String {
        match self.current_layout.as_str() {
            "single" => "single-editor".to_string(),
            "horizontal" | "vertical" => {
                match pane_id {
                    "left-pane" | "top-pane" => "left-editor".to_string(),
                    "right-pane" | "bottom-pane" => "right-editor".to_string(),
                    _ => format!("{}-editor", pane_id),
                }
            }
            "grid_2x2" => {
                match pane_id {
                    "pane-1" => "pane-1-editor".to_string(),
                    "pane-2" => "pane-2-editor".to_string(),
                    "pane-3" => "pane-3-editor".to_string(),
                    "pane-4" => "pane-4-editor".to_string(),
                    _ => format!("{}-editor", pane_id),
                }
            }
            _ => format!("{}-editor", pane_id),
        }
    }
    
    /// Get pane ID for an editor based on current layout
    pub fn get_pane_id_for_editor(&self, editor_id: &str) -> String {
        match self.current_layout.as_str() {
            "single" => "single-pane".to_string(),
            "horizontal" | "vertical" => {
                match editor_id {
                    "left-editor" => "left-pane".to_string(),
                    "right-editor" => "right-pane".to_string(),
                    _ => editor_id.replace("-editor", "-pane"),
                }
            }
            "grid_2x2" => {
                match editor_id {
                    "pane-1-editor" => "pane-1".to_string(),
                    "pane-2-editor" => "pane-2".to_string(),
                    "pane-3-editor" => "pane-3".to_string(),
                    "pane-4-editor" => "pane-4".to_string(),
                    _ => editor_id.replace("-editor", ""),
                }
            }
            _ => editor_id.replace("-editor", "-pane"),
        }
    }
    
    /// Start the background processing task
    pub async fn start_background_processing(&mut self) -> Result<()> {
        // Start the focus management event processing
        self.focus_bridge.lock().unwrap().start_event_processing().await?;
        
        // Start cursor blink timer (would be integrated with Slint's timer system)
        let focus_bridge = self.focus_bridge.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
            loop {
                interval.tick().await;
                if let Ok(bridge) = focus_bridge.lock() {
                    if let Err(e) = bridge.tick_cursor_blink() {
                        eprintln!("Error ticking cursor blink: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Update local focus states from a FocusUpdateResult
    fn update_focus_states_from_result(&self, result: &FocusUpdateResult) -> Result<()> {
        if let Ok(mut states) = self.editor_focus_states.lock() {
            // Clear all focus states
            for (_, has_focus) in states.iter_mut() {
                *has_focus = false;
            }
            
            // Set focus for the active editor
            for (editor_id, editor_state) in &result.editor_states {
                states.insert(editor_id.clone(), editor_state.has_focus);
            }
        }
        Ok(())
    }
    
    /// Get debug information about the current focus state
    pub fn get_debug_info(&self) -> Result<String> {
        let state = self.focus_bridge.lock().unwrap().get_current_state()?;
        let focus_states = self.get_editor_focus_states();
        
        Ok(format!(
            "Layout: {}, Pane Count: {}, Active Editor: {:?}, Active Pane: {:?}, Focus States: {:?}",
            self.current_layout,
            self.pane_count,
            state.active_editor_id,
            state.active_pane_id,
            focus_states
        ))
    }
}

impl Default for FocusManagementUIBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience functions for Slint integration
pub mod slint_helpers {
    use super::*;
    use slint::{SharedString, Weak, ComponentHandle};
    
    /// Create shared string from editor ID
    pub fn editor_id_to_shared_string(editor_id: &str) -> SharedString {
        SharedString::from(editor_id)
    }
    
    /// Create shared string from pane ID
    pub fn pane_id_to_shared_string(pane_id: &str) -> SharedString {
        SharedString::from(pane_id)
    }
    
    /// Convert focus state to boolean for Slint
    pub fn has_focus_to_bool(has_focus: bool) -> bool {
        has_focus
    }
    
    /// Get editor IDs for a specific layout
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
            _ => vec![],
        }
    }
    
    /// Get pane IDs for a specific layout
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
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    
    #[tokio::test]
    async fn test_focus_bridge_initialization() {
        let mut bridge = FocusManagementUIBridge::new();
        
        // Test single layout
        bridge.initialize_for_layout("single").unwrap();
        assert_eq!(bridge.current_layout, "single");
        assert_eq!(bridge.pane_count, 1);
        
        // Test horizontal layout
        bridge.initialize_for_layout("horizontal").unwrap();
        assert_eq!(bridge.current_layout, "horizontal");
        assert_eq!(bridge.pane_count, 2);
        
        // Test grid layout
        bridge.initialize_for_layout("grid_2x2").unwrap();
        assert_eq!(bridge.current_layout, "grid_2x2");
        assert_eq!(bridge.pane_count, 4);
    }
    
    #[tokio::test]
    async fn test_focus_management() {
        let mut bridge = FocusManagementUIBridge::new();
        bridge.initialize_for_layout("horizontal").unwrap();
        
        // Request focus for left editor
        let result = bridge.request_editor_focus("left-pane", "left-editor").unwrap();
        assert_eq!(result.active_editor_id, Some("left-editor".to_string()));
        
        // Check focus state
        assert!(bridge.has_editor_focus("left-editor"));
        assert!(!bridge.has_editor_focus("right-editor"));
        
        // Tab to next editor
        let result = bridge.tab_to_next_editor().unwrap();
        assert_eq!(result.active_editor_id, Some("right-editor".to_string()));
        
        // Check focus state
        assert!(!bridge.has_editor_focus("left-editor"));
        assert!(bridge.has_editor_focus("right-editor"));
    }
    
    #[tokio::test]
    async fn test_keyboard_shortcuts() {
        let mut bridge = FocusManagementUIBridge::new();
        bridge.initialize_for_layout("grid_2x2").unwrap();
        
        // Test Alt+1 shortcut
        let handled = bridge.handle_keyboard_shortcut("1", false, false, true).unwrap();
        assert!(handled);
        
        let active_id = bridge.get_active_editor_id().unwrap();
        assert_eq!(active_id, Some("pane-1-editor".to_string()));
        
        // Test Alt+2 shortcut
        let handled = bridge.handle_keyboard_shortcut("2", false, false, true).unwrap();
        assert!(handled);
        
        let active_id = bridge.get_active_editor_id().unwrap();
        assert_eq!(active_id, Some("pane-2-editor".to_string()));
    }
    
    #[tokio::test]
    async fn test_editor_pane_mapping() {
        let mut bridge = FocusManagementUIBridge::new();
        bridge.initialize_for_layout("grid_2x2").unwrap();
        
        // Test editor ID to pane ID mapping
        assert_eq!(bridge.get_pane_id_for_editor("pane-1-editor"), "pane-1");
        assert_eq!(bridge.get_pane_id_for_editor("pane-2-editor"), "pane-2");
        
        // Test pane ID to editor ID mapping
        assert_eq!(bridge.get_editor_id_for_pane("pane-1"), "pane-1-editor");
        assert_eq!(bridge.get_editor_id_for_pane("pane-2"), "pane-2-editor");
    }
    
    #[test]
    fn test_slint_helpers() {
        use slint_helpers::*;
        
        // Test editor ID conversion
        let shared_id = editor_id_to_shared_string("test-editor");
        assert_eq!(shared_id.as_str(), "test-editor");
        
        // Test layout helpers
        let editor_ids = get_editor_ids_for_layout("grid_2x2");
        assert_eq!(editor_ids.len(), 4);
        assert!(editor_ids.contains(&"pane-1-editor".to_string()));
        
        let pane_ids = get_pane_ids_for_layout("horizontal");
        assert_eq!(pane_ids.len(), 2);
        assert!(pane_ids.contains(&"left-pane".to_string()));
    }
}