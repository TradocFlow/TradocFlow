use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use anyhow::Result;

/// Focus state for individual editor instances
#[derive(Debug, Clone, PartialEq)]
pub struct EditorFocusState {
    pub editor_id: String,
    pub pane_id: String,
    pub language: String,
    pub has_focus: bool,
    pub cursor_position: usize,
    pub selection_start: usize,
    pub selection_end: usize,
    pub cursor_visible: bool,
    pub is_blinking: bool,
    pub last_activity: Instant,
}

impl EditorFocusState {
    pub fn new(editor_id: String, pane_id: String, language: String) -> Self {
        Self {
            editor_id,
            pane_id,
            language,
            has_focus: false,
            cursor_position: 0,
            selection_start: 0,
            selection_end: 0,
            cursor_visible: true,
            is_blinking: false,
            last_activity: Instant::now(),
        }
    }
    
    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
        self.cursor_visible = true;
    }
    
    pub fn has_selection(&self) -> bool {
        self.selection_start != self.selection_end
    }
}

/// Focus management events
#[derive(Debug, Clone)]
pub enum FocusEvent {
    RequestFocus {
        editor_id: String,
        pane_id: String,
    },
    ReleaseFocus {
        editor_id: String,
    },
    UpdateCursorPosition {
        editor_id: String,
        position: usize,
    },
    UpdateSelection {
        editor_id: String,
        start: usize,
        end: usize,
    },
    TabToNext,
    TabToPrevious,
    FocusEditorByIndex(usize),
    EnableCursorBlink(bool),
    LayoutChanged {
        layout: String,
        pane_count: usize,
    },
}

/// Focus management service result
#[derive(Debug, Clone)]
pub struct FocusUpdateResult {
    pub active_editor_id: Option<String>,
    pub active_pane_id: Option<String>,
    pub cursor_blink_enabled: bool,
    pub editor_states: HashMap<String, EditorFocusState>,
}

/// Focus management service for coordinating focus across multiple editor panes
pub struct FocusManagementService {
    /// Current active editor
    active_editor_id: Option<String>,
    
    /// All registered editor states
    editor_states: HashMap<String, EditorFocusState>,
    
    /// Editor order for tab navigation
    editor_order: Vec<String>,
    
    /// Current layout configuration
    current_layout: String,
    
    /// Cursor blink settings
    cursor_blink_enabled: bool,
    cursor_blink_interval: Duration,
    
    /// Event sender for notifications
    event_sender: Option<mpsc::UnboundedSender<FocusEvent>>,
    
    /// Last blink toggle time
    last_blink_toggle: Instant,
}

impl FocusManagementService {
    pub fn new() -> Self {
        Self {
            active_editor_id: None,
            editor_states: HashMap::new(),
            editor_order: Vec::new(),
            current_layout: "single".to_string(),
            cursor_blink_enabled: true,
            cursor_blink_interval: Duration::from_millis(500),
            event_sender: None,
            last_blink_toggle: Instant::now(),
        }
    }
    
    /// Register a new editor instance
    pub fn register_editor(&mut self, editor_id: String, pane_id: String, language: String) -> Result<()> {
        let editor_state = EditorFocusState::new(editor_id.clone(), pane_id, language);
        self.editor_states.insert(editor_id.clone(), editor_state);
        
        // Add to tab order if not already present
        if !self.editor_order.contains(&editor_id) {
            self.editor_order.push(editor_id.clone());
        }
        
        // If this is the first editor, make it active
        if self.active_editor_id.is_none() {
            self.set_active_editor(editor_id)?;
        }
        
        Ok(())
    }
    
    /// Unregister an editor instance
    pub fn unregister_editor(&mut self, editor_id: &str) -> Result<()> {
        self.editor_states.remove(editor_id);
        self.editor_order.retain(|id| id != editor_id);
        
        // If the active editor was removed, switch to the next available
        if Some(editor_id.to_string()) == self.active_editor_id {
            self.active_editor_id = None;
            if let Some(next_editor_id) = self.editor_order.first() {
                self.set_active_editor(next_editor_id.clone())?;
            }
        }
        
        Ok(())
    }
    
    /// Set the active editor
    pub fn set_active_editor(&mut self, editor_id: String) -> Result<FocusUpdateResult> {
        // Release focus from previous editor
        if let Some(prev_id) = &self.active_editor_id {
            if let Some(prev_state) = self.editor_states.get_mut(prev_id) {
                prev_state.has_focus = false;
                prev_state.is_blinking = false;
                prev_state.cursor_visible = false;
            }
        }
        
        // Release focus from all other editors
        for (id, state) in self.editor_states.iter_mut() {
            if *id != editor_id {
                state.has_focus = false;
                state.is_blinking = false;
                state.cursor_visible = false;
            }
        }
        
        // Set focus to new editor
        if let Some(new_state) = self.editor_states.get_mut(&editor_id) {
            new_state.has_focus = true;
            new_state.is_blinking = self.cursor_blink_enabled;
            new_state.cursor_visible = true;
            new_state.update_activity();
            
            self.active_editor_id = Some(editor_id.clone());
            
            // Reset blink timer
            self.last_blink_toggle = Instant::now();
            
            // Send focus event if we have a sender
            if let Some(sender) = &self.event_sender {
                let _ = sender.send(FocusEvent::RequestFocus {
                    editor_id: editor_id.clone(),
                    pane_id: new_state.pane_id.clone(),
                });
            }
        }
        
        Ok(self.get_current_state())
    }
    
    /// Handle focus request from an editor
    pub fn request_focus(&mut self, editor_id: String, pane_id: String) -> Result<FocusUpdateResult> {
        // Ensure editor is registered
        if !self.editor_states.contains_key(&editor_id) {
            // Try to register with default language
            self.register_editor(editor_id.clone(), pane_id, "en".to_string())?;
        }
        
        self.set_active_editor(editor_id)
    }
    
    /// Handle focus release from an editor
    pub fn release_focus(&mut self, editor_id: String) -> Result<FocusUpdateResult> {
        if let Some(state) = self.editor_states.get_mut(&editor_id) {
            state.has_focus = false;
            state.is_blinking = false;
        }
        
        // If this was the active editor, clear it
        if Some(editor_id) == self.active_editor_id {
            self.active_editor_id = None;
        }
        
        Ok(self.get_current_state())
    }
    
    /// Tab to next editor
    pub fn tab_to_next_editor(&mut self) -> Result<FocusUpdateResult> {
        if self.editor_order.is_empty() {
            return Ok(self.get_current_state());
        }
        
        let current_index = if let Some(active_id) = &self.active_editor_id {
            self.editor_order.iter()
                .position(|id| id == active_id)
                .unwrap_or(0)
        } else {
            0
        };
        
        let next_index = (current_index + 1) % self.editor_order.len();
        let next_editor_id = self.editor_order[next_index].clone();
        
        self.set_active_editor(next_editor_id)
    }
    
    /// Tab to previous editor
    pub fn tab_to_previous_editor(&mut self) -> Result<FocusUpdateResult> {
        if self.editor_order.is_empty() {
            return Ok(self.get_current_state());
        }
        
        let current_index = if let Some(active_id) = &self.active_editor_id {
            self.editor_order.iter()
                .position(|id| id == active_id)
                .unwrap_or(0)
        } else {
            0
        };
        
        let prev_index = if current_index == 0 {
            self.editor_order.len() - 1
        } else {
            current_index - 1
        };
        
        let prev_editor_id = self.editor_order[prev_index].clone();
        self.set_active_editor(prev_editor_id)
    }
    
    /// Focus editor by index in tab order
    pub fn focus_editor_by_index(&mut self, index: usize) -> Result<FocusUpdateResult> {
        if index < self.editor_order.len() {
            let editor_id = self.editor_order[index].clone();
            self.set_active_editor(editor_id)
        } else {
            Ok(self.get_current_state())
        }
    }
    
    /// Update cursor position for an editor
    pub fn update_cursor_position(&mut self, editor_id: String, position: usize) -> Result<FocusUpdateResult> {
        if let Some(state) = self.editor_states.get_mut(&editor_id) {
            state.cursor_position = position;
            state.update_activity();
        }
        
        Ok(self.get_current_state())
    }
    
    /// Update selection for an editor
    pub fn update_selection(&mut self, editor_id: String, start: usize, end: usize) -> Result<FocusUpdateResult> {
        if let Some(state) = self.editor_states.get_mut(&editor_id) {
            state.selection_start = start;
            state.selection_end = end;
            state.update_activity();
        }
        
        Ok(self.get_current_state())
    }
    
    /// Enable or disable cursor blinking globally
    pub fn set_cursor_blink_enabled(&mut self, enabled: bool) -> Result<FocusUpdateResult> {
        self.cursor_blink_enabled = enabled;
        
        // Update all editor states
        for state in self.editor_states.values_mut() {
            state.is_blinking = enabled && state.has_focus;
        }
        
        Ok(self.get_current_state())
    }
    
    /// Update layout configuration
    pub fn set_layout(&mut self, layout: String, pane_count: usize) -> Result<FocusUpdateResult> {
        self.current_layout = layout.clone();
        
        // Reorder editors based on layout
        self.reorder_editors_for_layout(&layout, pane_count);
        
        Ok(self.get_current_state())
    }
    
    /// Process cursor blink tick
    pub fn tick_cursor_blink(&mut self) -> Result<FocusUpdateResult> {
        let now = Instant::now();
        
        if now.duration_since(self.last_blink_toggle) >= self.cursor_blink_interval {
            self.last_blink_toggle = now;
            
            // Toggle cursor visibility for active editor only
            if let Some(active_id) = &self.active_editor_id {
                if let Some(state) = self.editor_states.get_mut(active_id) {
                    if state.is_blinking && !state.has_selection() && state.has_focus {
                        state.cursor_visible = !state.cursor_visible;
                    } else if state.has_focus {
                        state.cursor_visible = true;
                    }
                }
            }
            
            // Ensure all other editors have invisible cursors
            for (id, state) in self.editor_states.iter_mut() {
                if Some(id.clone()) != self.active_editor_id {
                    state.cursor_visible = false;
                    state.is_blinking = false;
                }
            }
        }
        
        Ok(self.get_current_state())
    }
    
    /// Get current focus state
    pub fn get_current_state(&self) -> FocusUpdateResult {
        let active_pane_id = if let Some(active_id) = &self.active_editor_id {
            self.editor_states.get(active_id)
                .map(|state| state.pane_id.clone())
        } else {
            None
        };
        
        FocusUpdateResult {
            active_editor_id: self.active_editor_id.clone(),
            active_pane_id,
            cursor_blink_enabled: self.cursor_blink_enabled,
            editor_states: self.editor_states.clone(),
        }
    }
    
    /// Get active editor state
    pub fn get_active_editor_state(&self) -> Option<&EditorFocusState> {
        if let Some(active_id) = &self.active_editor_id {
            self.editor_states.get(active_id)
        } else {
            None
        }
    }
    
    /// Get editor state by ID
    pub fn get_editor_state(&self, editor_id: &str) -> Option<&EditorFocusState> {
        self.editor_states.get(editor_id)
    }
    
    /// Check if an editor should show cursor
    pub fn should_show_cursor(&self, editor_id: &str) -> bool {
        if let Some(state) = self.editor_states.get(editor_id) {
            state.has_focus && (state.cursor_visible || state.has_selection())
        } else {
            false
        }
    }
    
    /// Check if an editor is currently active
    pub fn is_editor_active(&self, editor_id: &str) -> bool {
        self.active_editor_id.as_ref() == Some(&editor_id.to_string())
    }
    
    /// Get all editor states for UI updates
    pub fn get_editor_states_for_ui(&self) -> Vec<(String, EditorFocusState)> {
        self.editor_states.iter()
            .map(|(id, state)| (id.clone(), state.clone()))
            .collect()
    }
    
    /// Set event sender for notifications
    pub fn set_event_sender(&mut self, sender: mpsc::UnboundedSender<FocusEvent>) {
        self.event_sender = Some(sender);
    }
    
    /// Handle keyboard shortcut for editor navigation
    pub fn handle_keyboard_shortcut(&mut self, key: &str, modifiers: &[&str]) -> Result<Option<FocusUpdateResult>> {
        match key {
            "Tab" if modifiers.contains(&"Ctrl") => {
                // Ctrl+Tab: Next editor
                Ok(Some(self.tab_to_next_editor()?))
            }
            "Tab" if modifiers.contains(&"Ctrl") && modifiers.contains(&"Shift") => {
                // Ctrl+Shift+Tab: Previous editor
                Ok(Some(self.tab_to_previous_editor()?))
            }
            "1" | "2" | "3" | "4" if modifiers.contains(&"Alt") => {
                // Alt+1-4: Focus specific editor
                if let Ok(index) = key.parse::<usize>() {
                    if index > 0 && index <= 4 {
                        Ok(Some(self.focus_editor_by_index(index - 1)?))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None)
        }
    }
    
    /// Reorder editors based on layout configuration
    fn reorder_editors_for_layout(&mut self, layout: &str, _pane_count: usize) {
        match layout {
            "single" => {
                // Keep first editor only in tab order
                if self.editor_order.len() > 1 {
                    self.editor_order.truncate(1);
                }
            }
            "horizontal" | "vertical" => {
                // Ensure we have exactly 2 editors in tab order
                while self.editor_order.len() < 2 && self.editor_order.len() < self.editor_states.len() {
                    for (id, _) in &self.editor_states {
                        if !self.editor_order.contains(id) {
                            self.editor_order.push(id.clone());
                            break;
                        }
                    }
                }
                if self.editor_order.len() > 2 {
                    self.editor_order.truncate(2);
                }
            }
            "grid_2x2" => {
                // Ensure we have exactly 4 editors in tab order
                while self.editor_order.len() < 4 && self.editor_order.len() < self.editor_states.len() {
                    for (id, _) in &self.editor_states {
                        if !self.editor_order.contains(id) {
                            self.editor_order.push(id.clone());
                            break;
                        }
                    }
                }
                if self.editor_order.len() > 4 {
                    self.editor_order.truncate(4);
                }
            }
            _ => {
                // Custom layout - use all available editors
            }
        }
    }
}

impl Default for FocusManagementService {
    fn default() -> Self {
        Self::new()
    }
}

/// Focus management service bridge for Slint integration
pub struct FocusManagementBridge {
    service: Arc<Mutex<FocusManagementService>>,
    event_receiver: Option<mpsc::UnboundedReceiver<FocusEvent>>,
    runtime_handle: tokio::runtime::Handle,
}

impl FocusManagementBridge {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let service = Arc::new(Mutex::new(FocusManagementService::new()));
        
        // Set the event sender in the service
        if let Ok(mut service_guard) = service.lock() {
            service_guard.set_event_sender(sender);
        }
        
        Self {
            service,
            event_receiver: Some(receiver),
            runtime_handle: tokio::runtime::Handle::current(),
        }
    }
    
    /// Register a new editor
    pub fn register_editor(&self, editor_id: String, pane_id: String, language: String) -> Result<()> {
        let mut service = self.service.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock focus service: {}", e)
        })?;
        service.register_editor(editor_id, pane_id, language)
    }
    
    /// Request focus for an editor
    pub fn request_focus(&self, editor_id: String, pane_id: String) -> Result<FocusUpdateResult> {
        let mut service = self.service.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock focus service: {}", e)
        })?;
        service.request_focus(editor_id, pane_id)
    }
    
    /// Release focus from an editor
    pub fn release_focus(&self, editor_id: String) -> Result<FocusUpdateResult> {
        let mut service = self.service.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock focus service: {}", e)
        })?;
        service.release_focus(editor_id)
    }
    
    /// Tab to next editor
    pub fn tab_to_next_editor(&self) -> Result<FocusUpdateResult> {
        let mut service = self.service.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock focus service: {}", e)
        })?;
        service.tab_to_next_editor()
    }
    
    /// Tab to previous editor
    pub fn tab_to_previous_editor(&self) -> Result<FocusUpdateResult> {
        let mut service = self.service.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock focus service: {}", e)
        })?;
        service.tab_to_previous_editor()
    }
    
    /// Focus a specific editor by index (0-based)
    pub fn focus_editor_by_index(&self, index: usize) -> Result<FocusUpdateResult> {
        let mut service = self.service.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock focus service: {}", e)
        })?;
        service.focus_editor_by_index(index)
    }
    
    /// Update layout
    pub fn set_layout(&self, layout: String, pane_count: usize) -> Result<FocusUpdateResult> {
        let mut service = self.service.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock focus service: {}", e)
        })?;
        service.set_layout(layout, pane_count)
    }
    
    /// Update cursor position
    pub fn update_cursor_position(&self, editor_id: String, position: usize) -> Result<FocusUpdateResult> {
        let mut service = self.service.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock focus service: {}", e)
        })?;
        service.update_cursor_position(editor_id, position)
    }
    
    /// Update selection
    pub fn update_selection(&self, editor_id: String, start: usize, end: usize) -> Result<FocusUpdateResult> {
        let mut service = self.service.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock focus service: {}", e)
        })?;
        service.update_selection(editor_id, start, end)
    }
    
    /// Enable/disable cursor blink
    pub fn set_cursor_blink_enabled(&self, enabled: bool) -> Result<FocusUpdateResult> {
        let mut service = self.service.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock focus service: {}", e)
        })?;
        service.set_cursor_blink_enabled(enabled)
    }
    
    /// Process cursor blink tick
    pub fn tick_cursor_blink(&self) -> Result<FocusUpdateResult> {
        let mut service = self.service.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock focus service: {}", e)
        })?;
        service.tick_cursor_blink()
    }
    
    /// Get current state
    pub fn get_current_state(&self) -> Result<FocusUpdateResult> {
        let service = self.service.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock focus service: {}", e)
        })?;
        Ok(service.get_current_state())
    }
    
    /// Handle keyboard shortcut
    pub fn handle_keyboard_shortcut(&self, key: &str, modifiers: &[&str]) -> Result<Option<FocusUpdateResult>> {
        let mut service = self.service.lock().map_err(|e| {
            anyhow::anyhow!("Failed to lock focus service: {}", e)
        })?;
        service.handle_keyboard_shortcut(key, modifiers)
    }
    
    /// Start the background event processing task
    pub async fn start_event_processing(&mut self) -> Result<()> {
        if let Some(mut receiver) = self.event_receiver.take() {
            let service = self.service.clone();
            
            tokio::spawn(async move {
                while let Some(event) = receiver.recv().await {
                    if let Ok(mut service_guard) = service.lock() {
                        match event {
                            FocusEvent::RequestFocus { editor_id, pane_id } => {
                                let _ = service_guard.request_focus(editor_id, pane_id);
                            }
                            FocusEvent::ReleaseFocus { editor_id } => {
                                let _ = service_guard.release_focus(editor_id);
                            }
                            FocusEvent::UpdateCursorPosition { editor_id, position } => {
                                let _ = service_guard.update_cursor_position(editor_id, position);
                            }
                            FocusEvent::UpdateSelection { editor_id, start, end } => {
                                let _ = service_guard.update_selection(editor_id, start, end);
                            }
                            FocusEvent::TabToNext => {
                                let _ = service_guard.tab_to_next_editor();
                            }
                            FocusEvent::TabToPrevious => {
                                let _ = service_guard.tab_to_previous_editor();
                            }
                            FocusEvent::FocusEditorByIndex(index) => {
                                let _ = service_guard.focus_editor_by_index(index);
                            }
                            FocusEvent::EnableCursorBlink(enabled) => {
                                let _ = service_guard.set_cursor_blink_enabled(enabled);
                            }
                            FocusEvent::LayoutChanged { layout, pane_count } => {
                                let _ = service_guard.set_layout(layout, pane_count);
                            }
                        }
                    }
                }
            });
        }
        Ok(())
    }
}

impl Default for FocusManagementBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    
    #[tokio::test]
    async fn test_focus_management_basic() {
        let mut service = FocusManagementService::new();
        
        // Register editors
        service.register_editor("editor1".to_string(), "pane1".to_string(), "en".to_string()).unwrap();
        service.register_editor("editor2".to_string(), "pane2".to_string(), "de".to_string()).unwrap();
        
        // Check that first editor is active by default
        let state = service.get_current_state();
        assert_eq!(state.active_editor_id, Some("editor1".to_string()));
        
        // Switch focus to second editor
        let result = service.set_active_editor("editor2".to_string()).unwrap();
        assert_eq!(result.active_editor_id, Some("editor2".to_string()));
        
        // Check that first editor lost focus
        let editor1_state = service.get_editor_state("editor1").unwrap();
        assert!(!editor1_state.has_focus);
        
        // Check that second editor gained focus
        let editor2_state = service.get_editor_state("editor2").unwrap();
        assert!(editor2_state.has_focus);
    }
    
    #[tokio::test]
    async fn test_tab_navigation() {
        let mut service = FocusManagementService::new();
        
        // Register three editors
        service.register_editor("editor1".to_string(), "pane1".to_string(), "en".to_string()).unwrap();
        service.register_editor("editor2".to_string(), "pane2".to_string(), "de".to_string()).unwrap();
        service.register_editor("editor3".to_string(), "pane3".to_string(), "fr".to_string()).unwrap();
        
        // Tab to next editor
        let result = service.tab_to_next_editor().unwrap();
        assert_eq!(result.active_editor_id, Some("editor2".to_string()));
        
        // Tab to next again
        let result = service.tab_to_next_editor().unwrap();
        assert_eq!(result.active_editor_id, Some("editor3".to_string()));
        
        // Tab to next should wrap around
        let result = service.tab_to_next_editor().unwrap();
        assert_eq!(result.active_editor_id, Some("editor1".to_string()));
        
        // Tab to previous
        let result = service.tab_to_previous_editor().unwrap();
        assert_eq!(result.active_editor_id, Some("editor3".to_string()));
    }
    
    #[tokio::test]
    async fn test_cursor_position_updates() {
        let mut service = FocusManagementService::new();
        
        service.register_editor("editor1".to_string(), "pane1".to_string(), "en".to_string()).unwrap();
        
        // Update cursor position
        service.update_cursor_position("editor1".to_string(), 42).unwrap();
        
        let state = service.get_editor_state("editor1").unwrap();
        assert_eq!(state.cursor_position, 42);
        
        // Update selection
        service.update_selection("editor1".to_string(), 10, 20).unwrap();
        
        let state = service.get_editor_state("editor1").unwrap();
        assert_eq!(state.selection_start, 10);
        assert_eq!(state.selection_end, 20);
        assert!(state.has_selection());
    }
    
    #[tokio::test]
    async fn test_cursor_blinking() {
        let mut service = FocusManagementService::new();
        
        service.register_editor("editor1".to_string(), "pane1".to_string(), "en".to_string()).unwrap();
        
        // Enable cursor blinking
        service.set_cursor_blink_enabled(true).unwrap();
        
        let state = service.get_editor_state("editor1").unwrap();
        assert!(state.is_blinking);
        assert!(state.cursor_visible);
        
        // Simulate blink tick
        sleep(Duration::from_millis(600)).await;
        service.tick_cursor_blink().unwrap();
        
        let state = service.get_editor_state("editor1").unwrap();
        // Note: cursor visibility would toggle in real implementation
    }
    
    #[tokio::test]
    async fn test_layout_changes() {
        let mut service = FocusManagementService::new();
        
        // Register 4 editors
        for i in 1..=4 {
            service.register_editor(
                format!("editor{}", i),
                format!("pane{}", i),
                "en".to_string()
            ).unwrap();
        }
        
        // Set grid layout
        service.set_layout("grid_2x2".to_string(), 4).unwrap();
        
        // Should have 4 editors in tab order
        assert_eq!(service.editor_order.len(), 4);
        
        // Set horizontal layout
        service.set_layout("horizontal".to_string(), 2).unwrap();
        
        // Should have 2 editors in tab order
        assert_eq!(service.editor_order.len(), 2);
        
        // Set single layout
        service.set_layout("single".to_string(), 1).unwrap();
        
        // Should have 1 editor in tab order
        assert_eq!(service.editor_order.len(), 1);
    }
}