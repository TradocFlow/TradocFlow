use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};

/// Synchronization event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncEventType {
    CursorMove,
    ScrollMove,
    ContentChange,
    SelectionChange,
}

/// Synchronization event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEvent {
    pub event_type: SyncEventType,
    pub source_language: String,
    pub target_language: Option<String>,
    pub cursor_position: usize,
    pub scroll_position: f64,
    pub content: Option<String>,
    pub selection_start: Option<usize>,
    pub selection_end: Option<usize>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Split pane configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitPaneConfig {
    pub orientation: SplitOrientation,
    pub sync_enabled: bool,
    pub sync_cursor: bool,
    pub sync_scroll: bool,
    pub sync_selection: bool,
    pub left_language: String,
    pub right_language: String,
}

/// Split orientation options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SplitOrientation {
    Horizontal, // Side by side
    Vertical,   // Top and bottom
}

/// Language pane state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguagePaneState {
    pub language: String,
    pub content: String,
    pub cursor_position: usize,
    pub scroll_position: f64,
    pub selection_start: Option<usize>,
    pub selection_end: Option<usize>,
    pub read_only: bool,
    pub last_modified: chrono::DateTime<chrono::Utc>,
}

/// Editor synchronization service
pub struct EditorSyncService {
    config: Arc<Mutex<SplitPaneConfig>>,
    pane_states: Arc<Mutex<HashMap<String, LanguagePaneState>>>,
    event_sender: broadcast::Sender<SyncEvent>,
    event_receiver: broadcast::Receiver<SyncEvent>,
}

impl EditorSyncService {
    /// Create a new editor synchronization service
    pub fn new() -> Self {
        let (event_sender, event_receiver) = broadcast::channel(100);
        
        let default_config = SplitPaneConfig {
            orientation: SplitOrientation::Horizontal,
            sync_enabled: true,
            sync_cursor: true,
            sync_scroll: true,
            sync_selection: false,
            left_language: "en".to_string(),
            right_language: "de".to_string(),
        };
        
        Self {
            config: Arc::new(Mutex::new(default_config)),
            pane_states: Arc::new(Mutex::new(HashMap::new())),
            event_sender,
            event_receiver,
        }
    }
    
    /// Get current configuration
    pub fn get_config(&self) -> Result<SplitPaneConfig, crate::TradocumentError> {
        self.config.lock()
            .map(|config| config.clone())
            .map_err(|e| crate::TradocumentError::SyncError(format!("Failed to get config: {}", e)))
    }
    
    /// Update configuration
    pub fn update_config(&self, new_config: SplitPaneConfig) -> Result<(), crate::TradocumentError> {
        self.config.lock()
            .map(|mut config| *config = new_config)
            .map_err(|e| crate::TradocumentError::SyncError(format!("Failed to update config: {}", e)))
    }
    
    /// Get pane state for a language
    pub fn get_pane_state(&self, language: &str) -> Result<Option<LanguagePaneState>, crate::TradocumentError> {
        self.pane_states.lock()
            .map(|states| states.get(language).cloned())
            .map_err(|e| crate::TradocumentError::SyncError(format!("Failed to get pane state: {}", e)))
    }
    
    /// Update pane state for a language
    pub fn update_pane_state(&self, language: &str, state: LanguagePaneState) -> Result<(), crate::TradocumentError> {
        self.pane_states.lock()
            .map(|mut states| {
                states.insert(language.to_string(), state);
            })
            .map_err(|e| crate::TradocumentError::SyncError(format!("Failed to update pane state: {}", e)))
    }
    
    /// Handle cursor movement synchronization
    pub fn sync_cursor_move(&self, source_language: &str, cursor_position: usize) -> Result<(), crate::TradocumentError> {
        let config = self.get_config()?;
        
        if !config.sync_enabled || !config.sync_cursor {
            return Ok(());
        }
        
        // Determine target language
        let target_language = if source_language == config.left_language {
            config.right_language.clone()
        } else {
            config.left_language.clone()
        };
        
        // Calculate synchronized cursor position
        let sync_position = self.calculate_sync_cursor_position(
            source_language,
            &target_language,
            cursor_position,
        )?;
        
        // Create sync event
        let event = SyncEvent {
            event_type: SyncEventType::CursorMove,
            source_language: source_language.to_string(),
            target_language: Some(target_language.clone()),
            cursor_position: sync_position,
            scroll_position: 0.0,
            content: None,
            selection_start: None,
            selection_end: None,
            timestamp: chrono::Utc::now(),
        };
        
        // Send sync event
        self.event_sender.send(event)
            .map_err(|e| crate::TradocumentError::SyncError(format!("Failed to send sync event: {}", e)))?;
        
        // Update target pane state
        if let Some(mut target_state) = self.get_pane_state(&target_language)? {
            target_state.cursor_position = sync_position;
            target_state.last_modified = chrono::Utc::now();
            self.update_pane_state(&target_language, target_state)?;
        }
        
        Ok(())
    }
    
    /// Handle scroll synchronization
    pub fn sync_scroll_move(&self, source_language: &str, scroll_position: f64) -> Result<(), crate::TradocumentError> {
        let config = self.get_config()?;
        
        if !config.sync_enabled || !config.sync_scroll {
            return Ok(());
        }
        
        // Determine target language
        let target_language = if source_language == config.left_language {
            config.right_language.clone()
        } else {
            config.left_language.clone()
        };
        
        // Calculate synchronized scroll position
        let sync_position = self.calculate_sync_scroll_position(
            source_language,
            &target_language,
            scroll_position,
        )?;
        
        // Create sync event
        let event = SyncEvent {
            event_type: SyncEventType::ScrollMove,
            source_language: source_language.to_string(),
            target_language: Some(target_language.clone()),
            cursor_position: 0,
            scroll_position: sync_position,
            content: None,
            selection_start: None,
            selection_end: None,
            timestamp: chrono::Utc::now(),
        };
        
        // Send sync event
        self.event_sender.send(event)
            .map_err(|e| crate::TradocumentError::SyncError(format!("Failed to send sync event: {}", e)))?;
        
        // Update target pane state
        if let Some(mut target_state) = self.get_pane_state(&target_language)? {
            target_state.scroll_position = sync_position;
            target_state.last_modified = chrono::Utc::now();
            self.update_pane_state(&target_language, target_state)?;
        }
        
        Ok(())
    }
    
    /// Handle content change synchronization
    pub fn sync_content_change(&self, source_language: &str, content: &str) -> Result<(), crate::TradocumentError> {
        let config = self.get_config()?;
        
        if !config.sync_enabled {
            return Ok(());
        }
        
        // Update source pane state
        let mut source_state = self.get_pane_state(source_language)?
            .unwrap_or_else(|| LanguagePaneState {
                language: source_language.to_string(),
                content: String::new(),
                cursor_position: 0,
                scroll_position: 0.0,
                selection_start: None,
                selection_end: None,
                read_only: false,
                last_modified: chrono::Utc::now(),
            });
        
        source_state.content = content.to_string();
        source_state.last_modified = chrono::Utc::now();
        self.update_pane_state(source_language, source_state)?;
        
        // Create sync event for content change notification
        let event = SyncEvent {
            event_type: SyncEventType::ContentChange,
            source_language: source_language.to_string(),
            target_language: None,
            cursor_position: 0,
            scroll_position: 0.0,
            content: Some(content.to_string()),
            selection_start: None,
            selection_end: None,
            timestamp: chrono::Utc::now(),
        };
        
        // Send sync event
        self.event_sender.send(event)
            .map_err(|e| crate::TradocumentError::SyncError(format!("Failed to send sync event: {}", e)))?;
        
        Ok(())
    }
    
    /// Subscribe to sync events
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<SyncEvent> {
        self.event_sender.subscribe()
    }
    
    /// Calculate synchronized cursor position between languages
    fn calculate_sync_cursor_position(
        &self,
        source_language: &str,
        target_language: &str,
        cursor_position: usize,
    ) -> Result<usize, crate::TradocumentError> {
        // Get both pane states
        let source_state = self.get_pane_state(source_language)?;
        let target_state = self.get_pane_state(target_language)?;
        
        if let (Some(source), Some(target)) = (source_state, target_state) {
            // Enhanced cursor synchronization using line-based positioning
            let source_lines: Vec<&str> = source.content.lines().collect();
            let target_lines: Vec<&str> = target.content.lines().collect();
            
            if source_lines.is_empty() {
                return Ok(0);
            }
            
            // Find which line the cursor is on in the source
            let mut current_pos = 0;
            let mut source_line_index = 0;
            let mut cursor_offset_in_line = 0;
            
            for (line_idx, line) in source_lines.iter().enumerate() {
                let line_end = current_pos + line.len() + 1; // +1 for newline
                if cursor_position <= line_end {
                    source_line_index = line_idx;
                    cursor_offset_in_line = cursor_position - current_pos;
                    break;
                }
                current_pos = line_end;
            }
            
            // Calculate corresponding position in target
            if source_line_index < target_lines.len() {
                let target_line = target_lines[source_line_index];
                let target_line_start = target_lines.iter()
                    .take(source_line_index)
                    .map(|line| line.len() + 1)
                    .sum::<usize>();
                
                // Calculate relative position within the line
                let source_line = source_lines[source_line_index];
                let relative_offset = if source_line.is_empty() {
                    0
                } else {
                    (cursor_offset_in_line as f64 / source_line.len() as f64 * target_line.len() as f64) as usize
                };
                
                let sync_position = target_line_start + relative_offset.min(target_line.len());
                Ok(sync_position.min(target.content.len()))
            } else {
                // If target has fewer lines, position at end
                Ok(target.content.len())
            }
        } else {
            // If we don't have state for both panes, just return the original position
            Ok(cursor_position)
        }
    }
    
    /// Calculate synchronized scroll position between languages
    fn calculate_sync_scroll_position(
        &self,
        source_language: &str,
        target_language: &str,
        scroll_position: f64,
    ) -> Result<f64, crate::TradocumentError> {
        // Get both pane states
        let source_state = self.get_pane_state(source_language)?;
        let target_state = self.get_pane_state(target_language)?;
        
        if let (Some(source), Some(target)) = (source_state, target_state) {
            // Enhanced scroll synchronization using content-aware positioning
            let source_lines: Vec<&str> = source.content.lines().collect();
            let target_lines: Vec<&str> = target.content.lines().collect();
            
            if source_lines.is_empty() {
                return Ok(0.0);
            }
            
            // Calculate which line is at the top of the viewport
            let source_line_at_top = (scroll_position as usize).min(source_lines.len().saturating_sub(1));
            
            // Find corresponding line in target based on content similarity
            let corresponding_target_line = if source_line_at_top < target_lines.len() {
                source_line_at_top
            } else {
                // If target has fewer lines, scale proportionally
                let ratio = target_lines.len() as f64 / source_lines.len() as f64;
                (source_line_at_top as f64 * ratio) as usize
            };
            
            // Calculate fine-grained scroll position within the line
            let line_offset = scroll_position - scroll_position.floor();
            let sync_position = corresponding_target_line as f64 + line_offset;
            
            Ok(sync_position.max(0.0).min(target_lines.len().saturating_sub(1) as f64))
        } else {
            // If we don't have state for both panes, just return the original position
            Ok(scroll_position)
        }
    }
    
    /// Reset synchronization state
    pub fn reset_sync(&self) -> Result<(), crate::TradocumentError> {
        // Clear all pane states
        self.pane_states.lock()
            .map(|mut states| states.clear())
            .map_err(|e| crate::TradocumentError::SyncError(format!("Failed to reset sync: {}", e)))?;
        
        // Reset cursor and scroll positions to 0
        let config = self.get_config()?;
        
        let default_state = LanguagePaneState {
            language: String::new(),
            content: String::new(),
            cursor_position: 0,
            scroll_position: 0.0,
            selection_start: None,
            selection_end: None,
            read_only: false,
            last_modified: chrono::Utc::now(),
        };
        
        let mut left_state = default_state.clone();
        left_state.language = config.left_language.clone();
        self.update_pane_state(&config.left_language, left_state)?;
        
        let mut right_state = default_state;
        right_state.language = config.right_language.clone();
        self.update_pane_state(&config.right_language, right_state)?;
        
        Ok(())
    }
    
    /// Toggle synchronization on/off
    pub fn toggle_sync(&self, enabled: bool) -> Result<(), crate::TradocumentError> {
        let mut config = self.get_config()?;
        config.sync_enabled = enabled;
        self.update_config(config)
    }
    
    /// Toggle cursor synchronization
    pub fn toggle_cursor_sync(&self, enabled: bool) -> Result<(), crate::TradocumentError> {
        let mut config = self.get_config()?;
        config.sync_cursor = enabled;
        self.update_config(config)
    }
    
    /// Toggle scroll synchronization
    pub fn toggle_scroll_sync(&self, enabled: bool) -> Result<(), crate::TradocumentError> {
        let mut config = self.get_config()?;
        config.sync_scroll = enabled;
        self.update_config(config)
    }
    
    /// Change split orientation
    pub fn set_orientation(&self, orientation: SplitOrientation) -> Result<(), crate::TradocumentError> {
        let mut config = self.get_config()?;
        config.orientation = orientation;
        self.update_config(config)
    }
    
    /// Set languages for the panes
    pub fn set_languages(&self, left_language: &str, right_language: &str) -> Result<(), crate::TradocumentError> {
        let mut config = self.get_config()?;
        config.left_language = left_language.to_string();
        config.right_language = right_language.to_string();
        self.update_config(config)
    }
}

impl Default for EditorSyncService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sync_service_creation() {
        let service = EditorSyncService::new();
        let config = service.get_config().unwrap();
        
        assert_eq!(config.orientation, SplitOrientation::Horizontal);
        assert!(config.sync_enabled);
        assert!(config.sync_cursor);
        assert!(config.sync_scroll);
        assert_eq!(config.left_language, "en");
        assert_eq!(config.right_language, "de");
    }
    
    #[test]
    fn test_config_update() {
        let service = EditorSyncService::new();
        
        let mut new_config = service.get_config().unwrap();
        new_config.orientation = SplitOrientation::Vertical;
        new_config.sync_enabled = false;
        
        service.update_config(new_config.clone()).unwrap();
        
        let updated_config = service.get_config().unwrap();
        assert_eq!(updated_config.orientation, SplitOrientation::Vertical);
        assert!(!updated_config.sync_enabled);
    }
    
    #[test]
    fn test_pane_state_management() {
        let service = EditorSyncService::new();
        
        let state = LanguagePaneState {
            language: "en".to_string(),
            content: "Hello world".to_string(),
            cursor_position: 5,
            scroll_position: 10.0,
            selection_start: Some(0),
            selection_end: Some(5),
            read_only: false,
            last_modified: chrono::Utc::now(),
        };
        
        service.update_pane_state("en", state.clone()).unwrap();
        
        let retrieved_state = service.get_pane_state("en").unwrap().unwrap();
        assert_eq!(retrieved_state.language, "en");
        assert_eq!(retrieved_state.content, "Hello world");
        assert_eq!(retrieved_state.cursor_position, 5);
        assert_eq!(retrieved_state.scroll_position, 10.0);
    }
    
    #[test]
    fn test_cursor_position_calculation() {
        let service = EditorSyncService::new();
        
        // Set up source and target states
        let source_state = LanguagePaneState {
            language: "en".to_string(),
            content: "Hello world".to_string(), // 11 characters
            cursor_position: 5,
            scroll_position: 0.0,
            selection_start: None,
            selection_end: None,
            read_only: false,
            last_modified: chrono::Utc::now(),
        };
        
        let target_state = LanguagePaneState {
            language: "de".to_string(),
            content: "Hallo Welt".to_string(), // 10 characters
            cursor_position: 0,
            scroll_position: 0.0,
            selection_start: None,
            selection_end: None,
            read_only: false,
            last_modified: chrono::Utc::now(),
        };
        
        service.update_pane_state("en", source_state).unwrap();
        service.update_pane_state("de", target_state).unwrap();
        
        // Calculate sync position (5/11 * 10 ≈ 4.5 → 4)
        let sync_position = service.calculate_sync_cursor_position("en", "de", 5).unwrap();
        assert_eq!(sync_position, 4);
    }
    
    #[test]
    fn test_toggle_functions() {
        let service = EditorSyncService::new();
        
        // Test sync toggle
        service.toggle_sync(false).unwrap();
        assert!(!service.get_config().unwrap().sync_enabled);
        
        service.toggle_sync(true).unwrap();
        assert!(service.get_config().unwrap().sync_enabled);
        
        // Test cursor sync toggle
        service.toggle_cursor_sync(false).unwrap();
        assert!(!service.get_config().unwrap().sync_cursor);
        
        // Test scroll sync toggle
        service.toggle_scroll_sync(false).unwrap();
        assert!(!service.get_config().unwrap().sync_scroll);
    }
    
    #[test]
    fn test_orientation_change() {
        let service = EditorSyncService::new();
        
        service.set_orientation(SplitOrientation::Vertical).unwrap();
        assert_eq!(service.get_config().unwrap().orientation, SplitOrientation::Vertical);
        
        service.set_orientation(SplitOrientation::Horizontal).unwrap();
        assert_eq!(service.get_config().unwrap().orientation, SplitOrientation::Horizontal);
    }
    
    #[test]
    fn test_language_setting() {
        let service = EditorSyncService::new();
        
        service.set_languages("fr", "es").unwrap();
        let config = service.get_config().unwrap();
        assert_eq!(config.left_language, "fr");
        assert_eq!(config.right_language, "es");
    }
}