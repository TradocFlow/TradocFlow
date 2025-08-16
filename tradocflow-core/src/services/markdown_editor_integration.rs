use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use super::document_state_manager::{
    DocumentStateManager, DocumentStateConfig, DocumentChange, DocumentChangeType,
    AutoSaveConfig, ConflictResolutionStrategy, UserSession
};
use super::markdown_processor::MarkdownProcessor;
use super::markdown_text_processor::{CursorPosition, TextSelection, SearchOptions};
use super::document_processing::{ThreadSafeDocumentProcessor, DocumentProcessingConfig};
use crate::TradocumentError;

/// Integration layer for markdown editor backend services
/// Connects the new text processing and state management with existing systems
pub struct MarkdownEditorIntegration {
    /// Document state manager
    state_manager: Arc<Mutex<DocumentStateManager>>,
    /// Document processor for imports
    document_processor: Option<ThreadSafeDocumentProcessor>,
    /// UI event sender
    ui_event_sender: Option<mpsc::Sender<UIEvent>>,
    /// Background task handles
    background_tasks: Vec<BackgroundTask>,
    /// Configuration
    config: EditorConfig,
}

/// Configuration for markdown editor integration
#[derive(Debug, Clone)]
pub struct EditorConfig {
    /// User information
    pub user_id: Uuid,
    pub user_name: String,
    /// Auto-save settings
    pub autosave_enabled: bool,
    pub autosave_interval_seconds: u64,
    pub autosave_directory: Option<PathBuf>,
    /// Collaboration settings
    pub collaboration_enabled: bool,
    pub conflict_resolution: ConflictResolutionStrategy,
    /// Performance settings
    pub lazy_loading_enabled: bool,
    pub background_processing: bool,
    pub cache_size_mb: usize,
    /// UI integration settings
    pub ui_update_interval_ms: u64,
    pub batch_ui_updates: bool,
}

/// Events sent to UI for updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIEvent {
    /// Event type
    pub event_type: UIEventType,
    /// Document ID
    pub document_id: Uuid,
    /// Event data
    pub data: UIEventData,
    /// Timestamp
    pub timestamp: u64,
}

/// Types of UI events
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UIEventType {
    /// Content changed
    ContentChanged,
    /// Cursor position changed
    CursorChanged,
    /// Selection changed
    SelectionChanged,
    /// Document saved
    DocumentSaved,
    /// Document loaded
    DocumentLoaded,
    /// Auto-save occurred
    AutoSaved,
    /// Validation results updated
    ValidationUpdated,
    /// User joined collaboration
    UserJoined,
    /// User left collaboration
    UserLeft,
    /// Conflict detected
    ConflictDetected,
    /// Status message
    StatusMessage,
    /// Error occurred
    Error,
}

/// Data payload for UI events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UIEventData {
    /// Content change data
    ContentChange {
        position: usize,
        length: usize,
        new_content: String,
        old_content: String,
    },
    /// Cursor change data
    CursorChange {
        positions: Vec<CursorPosition>,
    },
    /// Selection change data
    SelectionChange {
        selections: Vec<TextSelection>,
    },
    /// File operation data
    FileOperation {
        file_path: String,
        success: bool,
        message: String,
    },
    /// Validation data
    Validation {
        is_valid: bool,
        errors: Vec<String>,
        warnings: Vec<String>,
    },
    /// User data
    User {
        user_id: Uuid,
        user_name: String,
        action: String,
    },
    /// Status data
    Status {
        message: String,
        status_type: String, // "info", "warning", "error", "success"
    },
    /// Error data
    Error {
        message: String,
        error_type: String,
    },
}

/// Background task handle
struct BackgroundTask {
    /// Task ID
    id: Uuid,
    /// Task name
    name: String,
    /// Cancellation sender
    cancel_sender: mpsc::Sender<()>,
    /// Thread handle
    thread_handle: thread::JoinHandle<()>,
}

/// Editor command for UI operations
#[derive(Debug, Clone)]
pub enum EditorCommand {
    /// Text editing commands
    InsertText(String),
    DeleteText(usize),
    ReplaceText { start: usize, end: usize, replacement: String },
    
    /// Cursor and selection commands
    SetCursor(usize),
    AddCursor(usize),
    SetSelection { start: usize, end: usize },
    ClearSelections,
    
    /// Find and replace commands
    Find { pattern: String, options: SearchOptions },
    FindNext,
    FindPrevious,
    Replace { replacement: String },
    ReplaceAll { replacement: String },
    
    /// Formatting commands
    ApplyBold,
    ApplyItalic,
    ApplyStrikethrough,
    ApplyInlineCode,
    ApplyHeading(usize),
    ApplyList { ordered: bool },
    InsertLink { text: String, url: String, title: Option<String> },
    InsertImage { alt_text: String, url: String, title: Option<String> },
    InsertCodeBlock { language: Option<String>, content: String },
    InsertTable { rows: usize, cols: usize, headers: bool },
    
    /// Document commands
    FormatDocument,
    Validate,
    Save(Option<PathBuf>),
    Load(PathBuf),
    Export { format: String, path: PathBuf },
    
    /// History commands
    Undo,
    Redo,
    CreateVersion(String),
    RestoreVersion(u64),
    
    /// Collaboration commands
    EnableCollaboration,
    DisableCollaboration,
    AddCollaborator(UserSession),
    RemoveCollaborator(Uuid),
}

/// Result of editor command execution
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Whether command succeeded
    pub success: bool,
    /// Result message
    pub message: String,
    /// Optional data
    pub data: Option<serde_json::Value>,
}

/// Thread-safe editor integration
#[derive(Clone)]
pub struct ThreadSafeEditorIntegration {
    integration: Arc<Mutex<MarkdownEditorIntegration>>,
}

/// Result type for editor operations
pub type EditorResult<T> = Result<T, EditorError>;

/// Errors that can occur in editor integration
#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    #[error("Document state error: {0}")]
    DocumentStateError(#[from] super::document_state_manager::DocumentStateError),
    #[error("Markdown processor error: {0}")]
    MarkdownProcessorError(#[from] super::markdown_processor::MarkdownProcessorError),
    #[error("Text processor error: {0}")]
    TextProcessorError(#[from] super::markdown_text_processor::TextProcessorError),
    #[error("Document processing error: {0}")]
    DocumentProcessingError(#[from] TradocumentError),
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    #[error("UI communication error: {0}")]
    UICommunicationError(String),
    #[error("Background task error: {0}")]
    BackgroundTaskError(String),
}

impl MarkdownEditorIntegration {
    /// Create new editor integration
    pub fn new(config: EditorConfig) -> EditorResult<Self> {
        let document_processor = ThreadSafeDocumentProcessor::new()
            .map_err(EditorError::DocumentProcessingError)?;

        let state_config = DocumentStateConfig {
            autosave: AutoSaveConfig {
                enabled: config.autosave_enabled,
                interval_seconds: config.autosave_interval_seconds,
                max_versions: 10,
                autosave_dir: config.autosave_directory.clone(),
                use_temp_files: true,
            },
            collaboration_enabled: config.collaboration_enabled,
            conflict_strategy: config.conflict_resolution.clone(),
            max_versions: 100,
            user_id: config.user_id,
            user_name: config.user_name.clone(),
        };

        let state_manager = Arc::new(Mutex::new(
            DocumentStateManager::new(state_config)?
        ));

        Ok(Self {
            state_manager,
            document_processor: Some(document_processor),
            ui_event_sender: None,
            background_tasks: Vec::new(),
            config,
        })
    }

    /// Set UI event sender for communication with UI layer
    pub fn set_ui_event_sender(&mut self, sender: mpsc::Sender<UIEvent>) {
        self.ui_event_sender = Some(sender);
        
        // Set up change listener
        if let Ok(mut manager) = self.state_manager.lock() {
            let event_sender = sender.clone();
            let document_id = manager.metadata().id;
            
            manager.add_change_listener(move |change| {
                let ui_event = Self::convert_document_change_to_ui_event(change, document_id);
                let _ = event_sender.send(ui_event);
            });
        }
    }

    /// Execute editor command
    pub fn execute_command(&mut self, command: EditorCommand) -> EditorResult<CommandResult> {
        match command {
            EditorCommand::InsertText(text) => {
                let mut manager = self.state_manager.lock().unwrap();
                let processor = manager.document_mut();
                processor.text_processor_mut().insert_text(&text)?;
                Ok(CommandResult {
                    success: true,
                    message: "Text inserted".to_string(),
                    data: None,
                })
            }

            EditorCommand::DeleteText(length) => {
                let mut manager = self.state_manager.lock().unwrap();
                let processor = manager.document_mut();
                processor.text_processor_mut().delete_text(length)?;
                Ok(CommandResult {
                    success: true,
                    message: "Text deleted".to_string(),
                    data: None,
                })
            }

            EditorCommand::ReplaceText { start, end, replacement } => {
                let mut manager = self.state_manager.lock().unwrap();
                let processor = manager.document_mut();
                processor.text_processor_mut().replace_text(start, end, &replacement)?;
                Ok(CommandResult {
                    success: true,
                    message: "Text replaced".to_string(),
                    data: None,
                })
            }

            EditorCommand::SetCursor(position) => {
                let mut manager = self.state_manager.lock().unwrap();
                let processor = manager.document_mut();
                processor.text_processor_mut().set_cursor(position)?;
                Ok(CommandResult {
                    success: true,
                    message: "Cursor position set".to_string(),
                    data: None,
                })
            }

            EditorCommand::SetSelection { start, end } => {
                let mut manager = self.state_manager.lock().unwrap();
                let processor = manager.document_mut();
                processor.text_processor_mut().set_selection(start, end)?;
                Ok(CommandResult {
                    success: true,
                    message: "Selection set".to_string(),
                    data: None,
                })
            }

            EditorCommand::Find { pattern, options } => {
                let mut manager = self.state_manager.lock().unwrap();
                let processor = manager.document_mut();
                let matches = processor.text_processor_mut().find(&pattern, options)?;
                Ok(CommandResult {
                    success: true,
                    message: format!("Found {} matches", matches.len()),
                    data: Some(serde_json::json!({ "matches": matches.len() })),
                })
            }

            EditorCommand::FindNext => {
                let mut manager = self.state_manager.lock().unwrap();
                let processor = manager.document_mut();
                if let Some(range) = processor.text_processor_mut().find_next() {
                    Ok(CommandResult {
                        success: true,
                        message: "Found next match".to_string(),
                        data: Some(serde_json::json!({ "start": range.start, "end": range.end })),
                    })
                } else {
                    Ok(CommandResult {
                        success: false,
                        message: "No more matches".to_string(),
                        data: None,
                    })
                }
            }

            EditorCommand::ApplyBold => {
                let mut manager = self.state_manager.lock().unwrap();
                let processor = manager.document_mut();
                processor.apply_bold()?;
                Ok(CommandResult {
                    success: true,
                    message: "Bold formatting applied".to_string(),
                    data: None,
                })
            }

            EditorCommand::ApplyItalic => {
                let mut manager = self.state_manager.lock().unwrap();
                let processor = manager.document_mut();
                processor.apply_italic()?;
                Ok(CommandResult {
                    success: true,
                    message: "Italic formatting applied".to_string(),
                    data: None,
                })
            }

            EditorCommand::ApplyHeading(level) => {
                let mut manager = self.state_manager.lock().unwrap();
                let processor = manager.document_mut();
                processor.apply_heading(level)?;
                Ok(CommandResult {
                    success: true,
                    message: format!("Heading level {} applied", level),
                    data: None,
                })
            }

            EditorCommand::InsertLink { text, url, title } => {
                let mut manager = self.state_manager.lock().unwrap();
                let processor = manager.document_mut();
                processor.insert_link(&text, &url, title.as_deref())?;
                Ok(CommandResult {
                    success: true,
                    message: "Link inserted".to_string(),
                    data: None,
                })
            }

            EditorCommand::Save(path) => {
                let mut manager = self.state_manager.lock().unwrap();
                manager.save_to_file(path.as_deref())?;
                
                self.send_ui_event(UIEvent {
                    event_type: UIEventType::DocumentSaved,
                    document_id: manager.metadata().id,
                    data: UIEventData::FileOperation {
                        file_path: path.as_ref()
                            .or(manager.metadata().file_path.as_ref())
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| "untitled".to_string()),
                        success: true,
                        message: "Document saved successfully".to_string(),
                    },
                    timestamp: Self::current_timestamp(),
                });

                Ok(CommandResult {
                    success: true,
                    message: "Document saved".to_string(),
                    data: None,
                })
            }

            EditorCommand::Load(path) => {
                let mut manager = self.state_manager.lock().unwrap();
                manager.load_from_file(&path)?;
                
                self.send_ui_event(UIEvent {
                    event_type: UIEventType::DocumentLoaded,
                    document_id: manager.metadata().id,
                    data: UIEventData::FileOperation {
                        file_path: path.to_string_lossy().to_string(),
                        success: true,
                        message: "Document loaded successfully".to_string(),
                    },
                    timestamp: Self::current_timestamp(),
                });

                Ok(CommandResult {
                    success: true,
                    message: "Document loaded".to_string(),
                    data: None,
                })
            }

            EditorCommand::Undo => {
                let mut manager = self.state_manager.lock().unwrap();
                let processor = manager.document_mut();
                processor.text_processor_mut().undo()?;
                Ok(CommandResult {
                    success: true,
                    message: "Undo successful".to_string(),
                    data: None,
                })
            }

            EditorCommand::Redo => {
                let mut manager = self.state_manager.lock().unwrap();
                let processor = manager.document_mut();
                processor.text_processor_mut().redo()?;
                Ok(CommandResult {
                    success: true,
                    message: "Redo successful".to_string(),
                    data: None,
                })
            }

            EditorCommand::Validate => {
                let mut manager = self.state_manager.lock().unwrap();
                let processor = manager.document_mut();
                let validation = processor.validate()?;
                
                self.send_ui_event(UIEvent {
                    event_type: UIEventType::ValidationUpdated,
                    document_id: manager.metadata().id,
                    data: UIEventData::Validation {
                        is_valid: validation.is_valid,
                        errors: validation.errors.iter().map(|e| e.message.clone()).collect(),
                        warnings: validation.warnings.iter().map(|w| w.message.clone()).collect(),
                    },
                    timestamp: Self::current_timestamp(),
                });

                Ok(CommandResult {
                    success: true,
                    message: format!("Validation complete: {} errors, {} warnings", 
                        validation.errors.len(), validation.warnings.len()),
                    data: Some(serde_json::json!({
                        "is_valid": validation.is_valid,
                        "errors": validation.errors.len(),
                        "warnings": validation.warnings.len()
                    })),
                })
            }

            EditorCommand::EnableCollaboration => {
                let mut manager = self.state_manager.lock().unwrap();
                manager.enable_collaboration()?;
                Ok(CommandResult {
                    success: true,
                    message: "Collaboration enabled".to_string(),
                    data: None,
                })
            }

            EditorCommand::AddCollaborator(user) => {
                let mut manager = self.state_manager.lock().unwrap();
                let user_id = user.user_id;
                let user_name = user.user_name.clone();
                manager.add_collaborator(user)?;
                
                self.send_ui_event(UIEvent {
                    event_type: UIEventType::UserJoined,
                    document_id: manager.metadata().id,
                    data: UIEventData::User {
                        user_id,
                        user_name,
                        action: "joined".to_string(),
                    },
                    timestamp: Self::current_timestamp(),
                });

                Ok(CommandResult {
                    success: true,
                    message: "Collaborator added".to_string(),
                    data: None,
                })
            }

            _ => Ok(CommandResult {
                success: false,
                message: "Command not implemented".to_string(),
                data: None,
            })
        }
    }

    /// Get current document content
    pub fn get_content(&self) -> EditorResult<String> {
        let manager = self.state_manager.lock().unwrap();
        Ok(manager.document().content().to_string())
    }

    /// Get document metadata
    pub fn get_metadata(&self) -> EditorResult<serde_json::Value> {
        let manager = self.state_manager.lock().unwrap();
        let metadata = manager.metadata();
        Ok(serde_json::json!({
            "id": metadata.id,
            "title": metadata.title,
            "file_path": metadata.file_path,
            "created_at": metadata.created_at,
            "modified_at": metadata.modified_at,
            "version": metadata.version,
            "author": metadata.author,
            "is_dirty": manager.is_dirty()
        }))
    }

    /// Get cursor positions
    pub fn get_cursors(&self) -> EditorResult<Vec<CursorPosition>> {
        let manager = self.state_manager.lock().unwrap();
        Ok(manager.document().text_processor().cursors().to_vec())
    }

    /// Get text selections
    pub fn get_selections(&self) -> EditorResult<Vec<TextSelection>> {
        let manager = self.state_manager.lock().unwrap();
        Ok(manager.document().text_processor().selections().to_vec())
    }

    /// Import document using document processor
    pub fn import_document(&mut self, file_path: &Path) -> EditorResult<()> {
        if let Some(ref processor) = self.document_processor {
            let config = DocumentProcessingConfig::default();
            
            // Create progress callback
            let ui_sender = self.ui_event_sender.clone();
            let document_id = {
                let manager = self.state_manager.lock().unwrap();
                manager.metadata().id
            };
            
            let progress_callback = Arc::new(move |progress| {
                if let Some(ref sender) = ui_sender {
                    let event = UIEvent {
                        event_type: UIEventType::StatusMessage,
                        document_id,
                        data: UIEventData::Status {
                            message: progress.message.clone(),
                            status_type: "info".to_string(),
                        },
                        timestamp: Self::current_timestamp(),
                    };
                    let _ = sender.send(event);
                }
            });

            let result = processor.process_document_sync(file_path, config, Some(progress_callback))?;
            
            // Load the processed content
            let mut manager = self.state_manager.lock().unwrap();
            *manager.document_mut() = MarkdownProcessor::with_content(result.content);
            
            Ok(())
        } else {
            Err(EditorError::InvalidOperation("Document processor not available".to_string()))
        }
    }

    /// Start background processing tasks
    pub fn start_background_tasks(&mut self) -> EditorResult<()> {
        if self.config.background_processing {
            self.start_auto_validation_task()?;
            if self.config.autosave_enabled {
                self.start_auto_save_monitoring()?;
            }
        }
        Ok(())
    }

    /// Stop all background tasks
    pub fn stop_background_tasks(&mut self) {
        for task in &self.background_tasks {
            let _ = task.cancel_sender.send(());
        }
        
        // Wait for tasks to complete
        let mut tasks = std::mem::take(&mut self.background_tasks);
        for task in tasks.drain(..) {
            let _ = task.thread_handle.join();
        }
    }

    // Private helper methods

    fn send_ui_event(&self, event: UIEvent) {
        if let Some(ref sender) = self.ui_event_sender {
            let _ = sender.send(event);
        }
    }

    fn convert_document_change_to_ui_event(change: &DocumentChange, document_id: Uuid) -> UIEvent {
        let (event_type, data) = match change.change_type {
            DocumentChangeType::Insert | DocumentChangeType::Delete | DocumentChangeType::Replace => {
                (UIEventType::ContentChanged, UIEventData::ContentChange {
                    position: change.position.unwrap_or(0),
                    length: change.length.unwrap_or(0),
                    new_content: change.new_content.clone().unwrap_or_default(),
                    old_content: change.old_content.clone().unwrap_or_default(),
                })
            }
            DocumentChangeType::Save => {
                (UIEventType::DocumentSaved, UIEventData::Status {
                    message: "Document saved".to_string(),
                    status_type: "success".to_string(),
                })
            }
            DocumentChangeType::AutoSave => {
                (UIEventType::AutoSaved, UIEventData::Status {
                    message: "Auto-save completed".to_string(),
                    status_type: "info".to_string(),
                })
            }
            _ => {
                (UIEventType::StatusMessage, UIEventData::Status {
                    message: "Document updated".to_string(),
                    status_type: "info".to_string(),
                })
            }
        };

        UIEvent {
            event_type,
            document_id,
            data,
            timestamp: change.timestamp,
        }
    }

    fn start_auto_validation_task(&mut self) -> EditorResult<()> {
        let state_manager = Arc::clone(&self.state_manager);
        let ui_sender = self.ui_event_sender.clone();
        let (cancel_sender, cancel_receiver) = mpsc::channel();

        let thread_handle = thread::spawn(move || {
            let mut last_validation = 0u64;
            
            loop {
                // Check for cancellation
                if cancel_receiver.try_recv().is_ok() {
                    break;
                }

                // Check if validation is needed
                if let Ok(manager) = state_manager.lock() {
                    let last_modified = manager.document().text_processor().last_modified();
                    
                    if last_modified > last_validation {
                        drop(manager); // Release lock before validation
                        
                        if let Ok(mut manager) = state_manager.lock() {
                            if let Ok(validation) = manager.document_mut().validate() {
                                if let Some(ref sender) = ui_sender {
                                    let event = UIEvent {
                                        event_type: UIEventType::ValidationUpdated,
                                        document_id: manager.metadata().id,
                                        data: UIEventData::Validation {
                                            is_valid: validation.is_valid,
                                            errors: validation.errors.iter().map(|e| e.message.clone()).collect(),
                                            warnings: validation.warnings.iter().map(|w| w.message.clone()).collect(),
                                        },
                                        timestamp: Self::current_timestamp(),
                                    };
                                    let _ = sender.send(event);
                                }
                                last_validation = last_modified;
                            }
                        }
                    }
                }

                thread::sleep(std::time::Duration::from_millis(1000));
            }
        });

        self.background_tasks.push(BackgroundTask {
            id: Uuid::new_v4(),
            name: "auto_validation".to_string(),
            cancel_sender,
            thread_handle,
        });

        Ok(())
    }

    fn start_auto_save_monitoring(&mut self) -> EditorResult<()> {
        let state_manager = Arc::clone(&self.state_manager);
        let ui_sender = self.ui_event_sender.clone();
        let interval = std::time::Duration::from_secs(self.config.autosave_interval_seconds);
        let (cancel_sender, cancel_receiver) = mpsc::channel();

        let thread_handle = thread::spawn(move || {
            loop {
                // Wait for interval or cancellation
                match cancel_receiver.recv_timeout(interval) {
                    Ok(_) => break, // Cancellation received
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // Auto-save interval reached
                        if let Ok(mut manager) = state_manager.lock() {
                            if manager.is_dirty() {
                                if let Ok(_) = manager.auto_save() {
                                    if let Some(ref sender) = ui_sender {
                                        let event = UIEvent {
                                            event_type: UIEventType::AutoSaved,
                                            document_id: manager.metadata().id,
                                            data: UIEventData::Status {
                                                message: "Auto-save completed".to_string(),
                                                status_type: "info".to_string(),
                                            },
                                            timestamp: Self::current_timestamp(),
                                        };
                                        let _ = sender.send(event);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        self.background_tasks.push(BackgroundTask {
            id: Uuid::new_v4(),
            name: "auto_save_monitor".to_string(),
            cancel_sender,
            thread_handle,
        });

        Ok(())
    }

    fn current_timestamp() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

impl ThreadSafeEditorIntegration {
    /// Create new thread-safe editor integration
    pub fn new(config: EditorConfig) -> EditorResult<Self> {
        let integration = MarkdownEditorIntegration::new(config)?;
        Ok(Self {
            integration: Arc::new(Mutex::new(integration)),
        })
    }

    /// Execute command thread-safely
    pub fn execute_command(&self, command: EditorCommand) -> EditorResult<CommandResult> {
        let mut integration = self.integration.lock()
            .map_err(|_| EditorError::UICommunicationError("Failed to acquire lock".to_string()))?;
        integration.execute_command(command)
    }

    /// Set UI event sender
    pub fn set_ui_event_sender(&self, sender: mpsc::Sender<UIEvent>) -> EditorResult<()> {
        let mut integration = self.integration.lock()
            .map_err(|_| EditorError::UICommunicationError("Failed to acquire lock".to_string()))?;
        integration.set_ui_event_sender(sender);
        Ok(())
    }

    /// Get content thread-safely
    pub fn get_content(&self) -> EditorResult<String> {
        let integration = self.integration.lock()
            .map_err(|_| EditorError::UICommunicationError("Failed to acquire lock".to_string()))?;
        integration.get_content()
    }

    /// Start background tasks
    pub fn start_background_tasks(&self) -> EditorResult<()> {
        let mut integration = self.integration.lock()
            .map_err(|_| EditorError::UICommunicationError("Failed to acquire lock".to_string()))?;
        integration.start_background_tasks()
    }
}

impl Drop for MarkdownEditorIntegration {
    fn drop(&mut self) {
        self.stop_background_tasks();
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            user_name: "User".to_string(),
            autosave_enabled: true,
            autosave_interval_seconds: 30,
            autosave_directory: None,
            collaboration_enabled: false,
            conflict_resolution: ConflictResolutionStrategy::LastWriteWins,
            lazy_loading_enabled: true,
            background_processing: true,
            cache_size_mb: 100,
            ui_update_interval_ms: 100,
            batch_ui_updates: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_editor_integration_creation() {
        let config = EditorConfig::default();
        let integration = MarkdownEditorIntegration::new(config);
        assert!(integration.is_ok());
    }

    #[test]
    fn test_insert_text_command() {
        let config = EditorConfig::default();
        let mut integration = MarkdownEditorIntegration::new(config).unwrap();
        
        let result = integration.execute_command(EditorCommand::InsertText("Hello".to_string()));
        assert!(result.is_ok());
        assert!(result.unwrap().success);
        
        let content = integration.get_content().unwrap();
        assert_eq!(content, "Hello");
    }

    #[test]
    fn test_formatting_commands() {
        let config = EditorConfig::default();
        let mut integration = MarkdownEditorIntegration::new(config).unwrap();
        
        // Insert text and select it
        integration.execute_command(EditorCommand::InsertText("Hello".to_string())).unwrap();
        integration.execute_command(EditorCommand::SetSelection { start: 0, end: 5 }).unwrap();
        
        // Apply bold formatting
        let result = integration.execute_command(EditorCommand::ApplyBold);
        assert!(result.is_ok());
        
        let content = integration.get_content().unwrap();
        assert_eq!(content, "**Hello**");
    }

    #[test]
    fn test_thread_safe_integration() {
        let config = EditorConfig::default();
        let integration = ThreadSafeEditorIntegration::new(config).unwrap();
        
        let result = integration.execute_command(EditorCommand::InsertText("Test".to_string()));
        assert!(result.is_ok());
        
        let content = integration.get_content().unwrap();
        assert_eq!(content, "Test");
    }
}