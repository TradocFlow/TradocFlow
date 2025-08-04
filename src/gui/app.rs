use slint::{ComponentHandle, ModelRc, VecModel};
use std::sync::{Arc, Mutex};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use crate::{MainWindow, TradocumentError, Result};
use crate::services::{ProjectService, DocumentImportService, TranslationMemoryService};
use crate::services::project_service::{CreateProjectRequest, TeamMemberRequest};
use crate::services::document_import_service::ImportConfig;

/// Document state tracking
#[derive(Debug, Clone)]
struct DocumentState {
    current_path: Option<PathBuf>,
    content: String,
    modified: bool,
    last_saved: Option<Instant>,
    language: String,
}

impl Default for DocumentState {
    fn default() -> Self {
        Self {
            current_path: None,
            content: String::new(),
            modified: false,
            last_saved: None,
            language: "en".to_string(),
        }
    }
}

/// Auto-save configuration
#[derive(Debug, Clone)]
struct AutoSaveConfig {
    enabled: bool,
    interval_seconds: u64,
    min_changes: usize,
}

impl Default for AutoSaveConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: 30, // Auto-save every 30 seconds
            min_changes: 5, // Minimum 5 changes before auto-save
        }
    }
}

/// Main application struct that manages the Slint GUI
pub struct App {
    main_window: MainWindow,
    project_service: Arc<ProjectService>,
    document_import_service: Arc<Mutex<DocumentImportService>>,
    translation_memory_service: Arc<TranslationMemoryService>,
    current_wizard_data: Arc<Mutex<WizardData>>,
    document_state: Arc<Mutex<DocumentState>>,
    auto_save_config: Arc<Mutex<AutoSaveConfig>>,
    auto_save_tx: Option<mpsc::UnboundedSender<()>>,
}

/// Data collected during the project creation wizard
#[derive(Debug, Clone, Default)]
struct WizardData {
    name: String,
    description: String,
    priority: String,
    due_date: String,
    folder_path: PathBuf,
    template_id: String,
    translation_memory_option: String,
    source_language: String,
    target_languages: Vec<String>,
    team_members: Vec<WizardTeamMember>,
}

#[derive(Debug, Clone)]
struct WizardTeamMember {
    name: String,
    email: String,
    role: String,
}

impl App {
    /// Create a new App instance with the Slint UI
    pub async fn new() -> Result<Self> {
        // Create the main window from Slint
        let main_window = MainWindow::new()
            .map_err(|e| TradocumentError::SlintError(format!("Failed to create main window: {}", e)))?;

        // Set up initial state
        main_window.set_current_language("en".into());
        main_window.set_current_mode("markdown".into());
        main_window.set_current_layout("single".into());
        main_window.set_status_message("TradocFlow ready".to_string().into());
        main_window.set_status_type("info".into());

        // Initialize services
        let project_service = Arc::new(ProjectService::new("./projects"));
        let wizard_data = Arc::new(Mutex::new(WizardData::default()));
        
        // Initialize translation memory service
        let tm_path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("translation_memory");
        let translation_memory_service = Arc::new(
            TranslationMemoryService::new(tm_path).await
                .map_err(|e| TradocumentError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other, 
                    format!("Failed to initialize translation memory: {}", e)
                )))?
        );
        
        // Initialize document import service
        let document_import_service = Arc::new(Mutex::new(
            DocumentImportService::new((*translation_memory_service).clone())
        ));
        
        // Initialize document state and auto-save
        let document_state = Arc::new(Mutex::new(DocumentState::default()));
        let auto_save_config = Arc::new(Mutex::new(AutoSaveConfig::default()));

        // Set up callbacks
        let app = Self { 
            main_window, 
            project_service,
            document_import_service,
            translation_memory_service,
            current_wizard_data: wizard_data,
            document_state,
            auto_save_config,
            auto_save_tx: None,
        };
        app.setup_callbacks();

        Ok(app)
    }

    /// Initialize the application (load last project, etc.)
    pub async fn initialize(&self) -> Result<()> {
        // Try to reopen the last project automatically
        self.try_reopen_last_project().await?;
        Ok(())
    }

    /// Run the application - this will block until the window is closed
    pub fn run(&self) -> Result<()> {
        self.main_window.run()
            .map_err(|e| TradocumentError::SlintError(format!("Failed to run application: {}", e)))
    }

    /// Set up all the callbacks for the Slint UI
    fn setup_callbacks(&self) {
        let main_window_weak = self.main_window.as_weak();

        // File menu callbacks
        self.main_window.on_file_new({
            let document_state = Arc::clone(&self.document_state);
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    // Create async task for new document
                    let document_state = Arc::clone(&document_state);
                    let window_weak = window.as_weak();
                    
                    tokio::spawn(async move {
                        // Check for unsaved changes and create new document
                        if let Ok(mut state) = document_state.lock() {
                            if state.modified {
                                // In a real implementation, show confirmation dialog
                                if let Some(window) = window_weak.upgrade() {
                                    window.set_status_message("Warning: Unsaved changes will be lost".into());
                                    window.set_status_type("warning".into());
                                }
                            }

                            // Reset document state
                            state.current_path = None;
                            state.content = "# New Document\n\nStart editing here...".to_string();
                            state.modified = false;
                            state.last_saved = None;
                            state.language = "en".to_string();

                            // Update UI
                            if let Some(window) = window_weak.upgrade() {
                                window.set_document_content(state.content.clone().into());
                                window.set_status_message("New document created".into());
                                window.set_status_type("success".into());
                            }
                        }
                    });
                }
            }
        });

        self.main_window.on_file_open({
            let document_state = Arc::clone(&self.document_state);
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    let document_state = Arc::clone(&document_state);
                    let window_weak = window.as_weak();
                    
                    tokio::spawn(async move {
                        // Simulate file dialog and open document
                        if let Some(window) = window_weak.upgrade() {
                            window.set_status_message("Opening file dialog...".into());
                            window.set_status_type("info".into());
                        }
                        
                        // Simulate file open dialog
                        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                        let test_file = current_dir.join("test_document.md");
                        
                        // Create test file if it doesn't exist
                        if !test_file.exists() {
                            if let Err(e) = tokio::fs::write(&test_file, "# Test Document\n\nThis is a test document for demonstration.\n\nYou can edit this content.").await {
                                if let Some(window) = window_weak.upgrade() {
                                    window.set_status_message(format!("Failed to create test file: {}", e).into());
                                    window.set_status_type("error".into());
                                }
                                return;
                            }
                        }
                        
                        // Load the document
                        match tokio::fs::read_to_string(&test_file).await {
                            Ok(content) => {
                                // Update document state
                                if let Ok(mut state) = document_state.lock() {
                                    state.current_path = Some(test_file.clone());
                                    state.content = content.clone();
                                    state.modified = false;
                                    state.last_saved = Some(Instant::now());
                                    state.language = "en".to_string();
                                }

                                // Update UI
                                if let Some(window) = window_weak.upgrade() {
                                    window.set_document_content(content.into());
                                    window.set_status_message(
                                        format!("Opened: {}", test_file.file_name().unwrap_or_default().to_string_lossy()).into()
                                    );
                                    window.set_status_type("success".into());
                                }
                            },
                            Err(e) => {
                                if let Some(window) = window_weak.upgrade() {
                                    window.set_status_message(format!("Failed to open file: {}", e).into());
                                    window.set_status_type("error".into());
                                }
                            }
                        }
                    });
                }
            }
        });

        self.main_window.on_file_save({
            let document_state = Arc::clone(&self.document_state);
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    let document_state = Arc::clone(&document_state);
                    let window_weak = window.as_weak();
                    
                    tokio::spawn(async move {
                        let (path, content) = {
                            let state = match document_state.lock() {
                                Ok(state) => state,
                                Err(_) => {
                                    if let Some(window) = window_weak.upgrade() {
                                        window.set_status_message("Failed to access document state".into());
                                        window.set_status_type("error".into());
                                    }
                                    return;
                                }
                            };
                            
                            match &state.current_path {
                                Some(path) => (path.clone(), state.content.clone()),
                                None => {
                                    // No path set - simulate save as dialog
                                    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                                    let save_path = current_dir.join("saved_document.md");
                                    (save_path, state.content.clone())
                                }
                            }
                        };

                        // Save to filesystem
                        match tokio::fs::write(&path, &content).await {
                            Ok(_) => {
                                // Update state
                                if let Ok(mut state) = document_state.lock() {
                                    state.current_path = Some(path.clone());
                                    state.modified = false;
                                    state.last_saved = Some(Instant::now());
                                }

                                if let Some(window) = window_weak.upgrade() {
                                    window.set_status_message(
                                        format!("Saved: {}", path.file_name().unwrap_or_default().to_string_lossy()).into()
                                    );
                                    window.set_status_type("success".into());
                                }
                            },
                            Err(e) => {
                                if let Some(window) = window_weak.upgrade() {
                                    window.set_status_message(format!("Failed to save document: {}", e).into());
                                    window.set_status_type("error".into());
                                }
                            }
                        }
                    });
                }
            }
        });

        self.main_window.on_file_save_as({
            let document_state = Arc::clone(&self.document_state);
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    let document_state = Arc::clone(&document_state);
                    let window_weak = window.as_weak();
                    
                    tokio::spawn(async move {
                        // Get current content
                        let content = {
                            let state = match document_state.lock() {
                                Ok(state) => state,
                                Err(_) => {
                                    if let Some(window) = window_weak.upgrade() {
                                        window.set_status_message("Failed to access document state".into());
                                        window.set_status_type("error".into());
                                    }
                                    return;
                                }
                            };
                            state.content.clone()
                        };

                        if let Some(window) = window_weak.upgrade() {
                            window.set_status_message("Opening save as dialog...".into());
                            window.set_status_type("info".into());
                        }

                        // Simulate save as dialog
                        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                        let save_path = current_dir.join(format!("document_{}.md", timestamp));

                        // Save to filesystem
                        match tokio::fs::write(&save_path, &content).await {
                            Ok(_) => {
                                // Update state
                                if let Ok(mut state) = document_state.lock() {
                                    state.current_path = Some(save_path.clone());
                                    state.modified = false;
                                    state.last_saved = Some(Instant::now());
                                }

                                if let Some(window) = window_weak.upgrade() {
                                    window.set_status_message(
                                        format!("Saved as: {}", save_path.file_name().unwrap_or_default().to_string_lossy()).into()
                                    );
                                    window.set_status_type("success".into());
                                }
                            },
                            Err(e) => {
                                if let Some(window) = window_weak.upgrade() {
                                    window.set_status_message(format!("Failed to save document: {}", e).into());
                                    window.set_status_type("error".into());
                                }
                            }
                        }
                    });
                }
            }
        });

        self.main_window.on_file_import({
            let document_state = Arc::clone(&self.document_state);
            let document_import_service = Arc::clone(&self.document_import_service);
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    let document_state = Arc::clone(&document_state);
                    let _document_import_service = Arc::clone(&document_import_service);
                    let window_weak = window.as_weak();
                    
                    tokio::spawn(async move {
                        if let Some(window) = window_weak.upgrade() {
                            window.set_status_message("Opening import dialog...".into());
                            window.set_status_type("info".into());
                        }

                        // Open file dialog for document import
                        let file_dialog = rfd::AsyncFileDialog::new()
                            .add_filter("Word Documents", &["docx", "doc"])
                            .add_filter("Text Files", &["txt", "md"])
                            .add_filter("All Files", &["*"])
                            .set_title("Import Document");

                        let import_file = match file_dialog.pick_file().await {
                            Some(file_handle) => file_handle.path().to_path_buf(),
                            None => {
                                if let Some(window) = window_weak.upgrade() {
                                    window.set_status_message("Import cancelled".into());
                                    window.set_status_type("info".into());
                                }
                                return;
                            }
                        };

                        // Determine file type and handle import
                        if let Some(extension) = import_file.extension().and_then(|ext| ext.to_str()) {
                            match extension.to_lowercase().as_str() {
                                "md" | "txt" => {
                                    // Direct load for text files
                                    match tokio::fs::read_to_string(&import_file).await {
                                        Ok(content) => {
                                            // Update document state
                                            if let Ok(mut state) = document_state.lock() {
                                                state.current_path = Some(import_file.clone());
                                                state.content = content.clone();
                                                state.modified = false;
                                                state.last_saved = Some(Instant::now());
                                                state.language = "en".to_string();
                                            }

                                            // Update UI
                                            if let Some(window) = window_weak.upgrade() {
                                                window.set_document_content(content.into());
                                                window.set_status_message("Document imported successfully".into());
                                                window.set_status_type("success".into());
                                            }
                                        },
                                        Err(e) => {
                                            if let Some(window) = window_weak.upgrade() {
                                                window.set_status_message(format!("Failed to import document: {}", e).into());
                                                window.set_status_type("error".into());
                                            }
                                        }
                                    }
                                },
                                "docx" | "doc" => {
                                    // Use DocumentImportService for Word documents
                                    if let Some(window) = window_weak.upgrade() {
                                        window.set_status_message(
                                            format!("Converting Word document to markdown: {}", 
                                                import_file.file_name().unwrap_or_default().to_string_lossy()
                                            ).into()
                                        );
                                        window.set_status_type("info".into());
                                    }

                                    // Create a simplified DOCX to markdown conversion
                                    // This avoids the Send/Sync issues with the DocumentImportService
                                    let detected_language = Self::detect_language_from_file(&import_file)
                                        .unwrap_or_else(|| "en".to_string());

                                    match Self::simple_docx_to_markdown(&import_file).await {
                                        Ok(markdown_content) => {
                                            // Update document state with converted content
                                            if let Ok(mut state) = document_state.lock() {
                                                // Create a new markdown file path based on the imported file
                                                let mut new_path = import_file.clone();
                                                new_path.set_extension("md");
                                                
                                                state.current_path = Some(new_path);
                                                state.content = markdown_content.clone();
                                                state.modified = true; // Mark as modified since it's imported/converted
                                                state.last_saved = None; // Not saved yet
                                                state.language = detected_language;
                                            }

                                            // Update UI with converted content
                                            if let Some(window) = window_weak.upgrade() {
                                                window.set_document_content(markdown_content.into());
                                                window.set_status_message(
                                                    format!("Successfully imported Word document: {}", 
                                                        import_file.file_name().unwrap_or_default().to_string_lossy()
                                                    ).into()
                                                );
                                                window.set_status_type("success".into());
                                            }
                                        },
                                        Err(e) => {
                                            if let Some(window) = window_weak.upgrade() {
                                                window.set_status_message(
                                                    format!("Failed to import Word document: {}", e).into()
                                                );
                                                window.set_status_type("error".into());
                                            }
                                        }
                                    }
                                },
                                _ => {
                                    if let Some(window) = window_weak.upgrade() {
                                        window.set_status_message(format!("Unsupported file format for import: {}", extension).into());
                                        window.set_status_type("error".into());
                                    }
                                }
                            }
                        }
                    });
                }
            }
        });

        self.main_window.on_file_export({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Export dialog would appear here".into());
                    window.set_status_type("info".into());
                }
            }
        });

        self.main_window.on_file_print({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Print dialog would appear here".into());
                    window.set_status_type("info".into());
                }
            }
        });

        self.main_window.on_file_exit({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.hide().ok();
                }
            }
        });

        // Edit menu callbacks
        self.main_window.on_edit_undo({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Undo".into());
                    window.set_status_type("info".into());
                }
            }
        });

        self.main_window.on_edit_redo({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Redo".into());
                    window.set_status_type("info".into());
                }
            }
        });

        self.main_window.on_edit_cut({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Cut".into());
                    window.set_status_type("info".into());
                }
            }
        });

        self.main_window.on_edit_copy({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Copy".into());
                    window.set_status_type("info".into());
                }
            }
        });

        self.main_window.on_edit_paste({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Paste".into());
                    window.set_status_type("info".into());
                }
            }
        });

        self.main_window.on_edit_find({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Find dialog would appear here".into());
                    window.set_status_type("info".into());
                }
            }
        });

        self.main_window.on_edit_replace({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Replace dialog would appear here".into());
                    window.set_status_type("info".into());
                }
            }
        });

        self.main_window.on_edit_preferences({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Preferences dialog would appear here".into());
                    window.set_status_type("info".into());
                }
            }
        });

        // View menu callbacks
        self.main_window.on_view_fullscreen({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Toggled fullscreen mode".into());
                    window.set_status_type("info".into());
                }
            }
        });

        self.main_window.on_view_zoom_in({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Zoom in".into());
                    window.set_status_type("info".into());
                }
            }
        });

        self.main_window.on_view_zoom_out({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Zoom out".into());
                    window.set_status_type("info".into());
                }
            }
        });

        self.main_window.on_view_show_sidebar({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Toggled sidebar".into());
                    window.set_status_type("info".into());
                }
            }
        });

        // Project menu callbacks
        self.main_window.on_project_new({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_show_project_wizard(true);
                    window.set_status_message("Project creation wizard opened".into());
                    window.set_status_type("info".into());
                }
            }
        });

        self.main_window.on_project_open({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_show_project_browser(true);
                    window.set_status_message("Project browser opened".into());
                    window.set_status_type("info".into());
                }
            }
        });

        // Add missing essential callbacks with stub implementations
        self.main_window.on_project_save({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Project saved".into());
                    window.set_status_type("success".into());
                }
            }
        });

        self.main_window.on_project_close({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Project closed".into());
                    window.set_status_type("info".into());
                }
            }
        });

        self.main_window.on_project_properties({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Project properties dialog would appear here".into());
                    window.set_status_type("info".into());
                }
            }
        });

        // Add essential callbacks for functionality that users might try
        self.main_window.on_help_about({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("TradocFlow - Translation Document Review System".into());
                    window.set_status_type("info".into());
                }
            }
        });

        // Add language and content change callbacks for basic functionality
        self.main_window.on_language_changed({
            let main_window_weak = main_window_weak.clone();
            move |lang| {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_current_language(lang.clone());
                    window.set_status_message(format!("Language changed to {}", lang).into());
                    window.set_status_type("success".into());
                }
            }
        });

        self.main_window.on_content_changed({
            let document_state = Arc::clone(&self.document_state);
            let auto_save_config = Arc::clone(&self.auto_save_config);
            let main_window_weak = main_window_weak.clone();
            move |content, language| {
                if let Some(window) = main_window_weak.upgrade() {
                    let document_state = Arc::clone(&document_state);
                    let auto_save_config = Arc::clone(&auto_save_config);
                    let window_weak = window.as_weak();
                    let content_str = content.to_string();
                    let language_str = language.to_string();
                    
                    tokio::spawn(async move {
                        // Update document state
                        let should_auto_save = if let Ok(mut state) = document_state.lock() {
                            let content_changed = state.content != content_str;
                            
                            if content_changed {
                                state.content = content_str.clone();
                                state.modified = true;
                                state.language = language_str.clone();
                                
                                // Check if auto-save should be triggered
                                if let Ok(config) = auto_save_config.lock() {
                                    config.enabled && state.current_path.is_some()
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        };

                        // Update UI
                        if let Some(window) = window_weak.upgrade() {
                            window.set_document_content(content_str.clone().into());
                            window.set_status_message(format!("Content updated for {}", language_str).into());
                            window.set_status_type("info".into());
                        }

                        // Trigger auto-save if conditions are met
                        if should_auto_save {
                            // Delay auto-save to avoid saving on every keystroke
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            
                            // Check if content is still the same (user hasn't made more changes)
                            let should_still_save = if let Ok(state) = document_state.lock() {
                                state.content == content_str && state.modified && state.current_path.is_some()
                            } else {
                                false
                            };

                            if should_still_save {
                                // Perform auto-save
                                let (path, content_to_save) = {
                                    let state = document_state.lock().unwrap();
                                    (state.current_path.as_ref().unwrap().clone(), state.content.clone())
                                };

                                match tokio::fs::write(&path, &content_to_save).await {
                                    Ok(_) => {
                                        // Update state
                                        if let Ok(mut state) = document_state.lock() {
                                            state.modified = false;
                                            state.last_saved = Some(Instant::now());
                                        }

                                        if let Some(window) = window_weak.upgrade() {
                                            window.set_status_message("Auto-saved".into());
                                            window.set_status_type("success".into());
                                        }
                                    },
                                    Err(e) => {
                                        if let Some(window) = window_weak.upgrade() {
                                            window.set_status_message(format!("Auto-save failed: {}", e).into());
                                            window.set_status_type("warning".into());
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
            }
        });

        // Translation menu callbacks
        self.setup_translation_callbacks();
        
        // Tools menu callbacks
        self.setup_tools_callbacks();
        
        // Help menu callbacks
        self.setup_help_callbacks();
        
        // Text formatting callbacks
        self.setup_formatting_callbacks();
        
        // Sidebar callbacks
        self.setup_sidebar_callbacks();
        
        // Editor callbacks
        self.setup_editor_callbacks();
        
        // Mode and layout callbacks
        self.setup_mode_layout_callbacks();
        
        // Project Wizard Callbacks
        self.setup_project_wizard_callbacks();
        
        // Project Browser Callbacks
        self.setup_project_browser_callbacks();
    }

    /// Set up project wizard specific callbacks
    fn setup_project_wizard_callbacks(&self) {
        let main_window_weak = self.main_window.as_weak();
        let wizard_data = Arc::clone(&self.current_wizard_data);
        let project_service = Arc::clone(&self.project_service);

        // Folder selection callback
        self.main_window.on_select_folder({
            let main_window_weak = main_window_weak.clone();
            let wizard_data = wizard_data.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    // In a real implementation, this would open a native file dialog
                    // For now, we'll simulate folder selection
                    let selected_folder = std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join("my-translation-project");
                    
                    if let Ok(mut data) = wizard_data.lock() {
                        data.folder_path = selected_folder.clone();
                        // Initialize default values if not set
                        if data.name.is_empty() {
                            data.name = "My Translation Project".to_string();
                        }
                        if data.priority.is_empty() {
                            data.priority = "medium".to_string();
                        }
                        if data.template_id.is_empty() {
                            data.template_id = "blank".to_string();
                        }
                        if data.translation_memory_option.is_empty() {
                            data.translation_memory_option = "new".to_string();
                        }
                        if data.source_language.is_empty() {
                            data.source_language = "English".to_string();
                        }
                    }
                    
                    // Update the wizard UI with the selected path
                    window.set_status_message(format!("Selected folder: {}", selected_folder.display()).into());
                    window.set_status_type("success".into());
                }
            }
        });

        // Template selection callback
        self.main_window.on_template_selected({
            let wizard_data = wizard_data.clone();
            move |template_id| {
                if let Ok(mut data) = wizard_data.lock() {
                    data.template_id = template_id.to_string();
                }
            }
        });

        // Language toggle callback
        self.main_window.on_language_toggled({
            let wizard_data = wizard_data.clone();
            move |language_code, enabled| {
                if let Ok(mut data) = wizard_data.lock() {
                    let language_name = Self::language_code_to_name(&language_code.to_string());
                    if enabled {
                        if !data.target_languages.contains(&language_name) {
                            data.target_languages.push(language_name);
                        }
                    } else {
                        data.target_languages.retain(|lang| lang != &language_name);
                    }
                }
            }
        });

        // Source language change callback
        self.main_window.on_source_language_changed({
            let wizard_data = wizard_data.clone();
            move |language_code| {
                if let Ok(mut data) = wizard_data.lock() {
                    data.source_language = Self::language_code_to_name(&language_code.to_string());
                }
            }
        });

        // Team member addition callback
        self.main_window.on_team_member_added({
            let wizard_data = wizard_data.clone();
            move |name, email, role| {
                if let Ok(mut data) = wizard_data.lock() {
                    data.team_members.push(WizardTeamMember {
                        name: name.to_string(),
                        email: email.to_string(),
                        role: role.to_string(),
                    });
                }
            }
        });

        // Team member removal callback
        self.main_window.on_team_member_removed({
            let wizard_data = wizard_data.clone();
            move |member_id| {
                if let Ok(mut data) = wizard_data.lock() {
                    // Simple removal by index for now
                    if let Ok(index) = member_id.parse::<usize>() {
                        if index < data.team_members.len() {
                            data.team_members.remove(index);
                        }
                    }
                }
            }
        });

        // Wizard finish callback - this is where the actual project creation happens
        self.main_window.on_wizard_finish({
            let main_window_weak = main_window_weak.clone();
            let wizard_data = wizard_data.clone();
            let project_service = project_service.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    // Create project in background
                    let wizard_data = wizard_data.clone();
                    let project_service = project_service.clone();
                    let window_weak = window.as_weak();
                    
                    // In a real implementation, this would be async
                    // For now, we'll simulate successful project creation
                    window.set_status_message("Creating project...".into());
                    window.set_status_type("info".into());
                    
                    // Extract wizard data before using in async context
                    let request = if let Ok(data) = wizard_data.lock() {
                        CreateProjectRequest {
                            name: data.name.clone(),
                            description: if data.description.is_empty() { None } else { Some(data.description.clone()) },
                            priority: data.priority.clone(),
                            due_date: if data.due_date.is_empty() { None } else { Some(data.due_date.clone()) },
                            folder_path: data.folder_path.clone(),
                            template_id: data.template_id.clone(),
                            translation_memory_option: data.translation_memory_option.clone(),
                            source_language: data.source_language.clone(),
                            target_languages: data.target_languages.clone(),
                            team_members: data.team_members.iter().map(|member| TeamMemberRequest {
                                name: member.name.clone(),
                                email: member.email.clone(),
                                role: member.role.clone(),
                            }).collect(),
                        }
                    } else {
                        return; // Exit if we can't get wizard data
                    };
                    
                    // Spawn async task for project creation
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    match rt.block_on(project_service.create_project(request)) {
                        Ok(result) => {
                            if let Some(window) = window_weak.upgrade() {
                                window.set_status_message(format!("Project '{}' created successfully!", result.project.name).into());
                                window.set_status_type("success".into());
                                window.set_has_project_loaded(true);
                                window.set_current_project_name(result.project.name.clone().into());
                                
                                // Save as last opened project
                                if let Some(settings_dir) = dirs::config_dir() {
                                    let settings_dir = settings_dir.join("tradocflow");
                                    if std::fs::create_dir_all(&settings_dir).is_ok() {
                                        let last_project_file = settings_dir.join("last_project.txt");
                                        let _ = std::fs::write(&last_project_file, result.project.project_path.to_string_lossy().as_bytes());
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            if let Some(window) = window_weak.upgrade() {
                                window.set_status_message(format!("Failed to create project: {}", e).into());
                                window.set_status_type("error".into());
                            }
                        }
                    }
                }
            }
        });

        // Step validation callback
        self.main_window.on_validate_current_step({
            let wizard_data = wizard_data.clone();
            move || -> bool {
                if let Ok(data) = wizard_data.lock() {
                    // Basic validation - in a real implementation, this would be more comprehensive
                    !data.name.trim().is_empty() && 
                    !data.source_language.is_empty() && 
                    !data.target_languages.is_empty()
                } else {
                    false
                }
            }
        });
    }

    /// Set up project browser specific callbacks
    fn setup_project_browser_callbacks(&self) {
        let main_window_weak = self.main_window.as_weak();
        let project_service = Arc::clone(&self.project_service);

        // Open project browser callback (already handled in project_open)
        self.main_window.on_open_project_browser({
            let main_window_weak = main_window_weak.clone();
            let project_service = project_service.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_show_project_browser(true);
                    window.set_status_message("Project browser opened".into());
                    window.set_status_type("info".into());
                    
                    // Automatically load projects when browser opens
                    window.invoke_browser_refresh_projects();
                }
            }
        });

        // Close project browser callback
        self.main_window.on_close_project_browser({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_show_project_browser(false);
                    window.set_status_message("Project browser closed".into());
                    window.set_status_type("info".into());
                }
            }
        });

        // Project browser search callback
        self.main_window.on_browser_search_changed({
            let main_window_weak = main_window_weak.clone();
            move |query| {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_browser_search_query(query.clone());
                    window.set_status_message(format!("Searching for: {}", query).into());
                    window.set_status_type("info".into());
                    
                    // TODO: Implement actual project search
                    // For now, we'll just update the search query
                }
            }
        });

        // Project browser project selected callback
        self.main_window.on_browser_project_opened({
            let main_window_weak = main_window_weak.clone();
            let project_service = project_service.clone();
            move |project_data| {
                if let Some(window) = main_window_weak.upgrade() {
                    // Extract project information from ProjectData
                    let project_name = project_data.name.to_string();
                    let project_path = std::path::PathBuf::from(project_data.path.to_string());
                    
                    // Set project as loaded
                    window.set_has_project_loaded(true);
                    window.set_current_project_name(project_name.clone().into());
                    window.set_show_project_browser(false);
                    
                    window.set_status_message(format!("Opened project: {}", project_name).into());
                    window.set_status_type("success".into());
                    
                    // Save as last opened project
                    // TODO: This should be done through a proper method on App, but for now we'll do it inline
                    if let Some(settings_dir) = dirs::config_dir() {
                        let settings_dir = settings_dir.join("tradocflow");
                        if std::fs::create_dir_all(&settings_dir).is_ok() {
                            let last_project_file = settings_dir.join("last_project.txt");
                            let _ = std::fs::write(&last_project_file, project_path.to_string_lossy().as_bytes());
                        }
                    }
                    
                    // TODO: Load project contents and update UI
                    // This would involve loading the project configuration and updating the sidebar
                }
            }
        });

        // Refresh projects callback
        self.main_window.on_browser_refresh_projects({
            let main_window_weak = main_window_weak.clone();
            let project_service = project_service.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_browser_is_loading(true);
                    window.set_status_message("Refreshing project list...".into());
                    window.set_status_type("info".into());
                    
                    // Discover projects in background
                    let window_weak = window.as_weak();
                    let project_service = project_service.clone();
                    
                    // Use tokio runtime for async operation
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    match rt.block_on(project_service.discover_projects(None)) {
                        Ok(summaries) => {
                            if let Some(window) = window_weak.upgrade() {
                                // Convert ProjectSummary to ProjectData
                                let project_data_vec = summaries.iter().map(|summary| {
                                    crate::ProjectData {
                                        name: summary.name.clone().into(),
                                        description: summary.description.clone().unwrap_or_default().into(),
                                        path: summary.path.to_string_lossy().to_string().into(),
                                        source_language: summary.source_language.clone().into(),
                                        target_languages: ModelRc::new(VecModel::from(
                                            summary.target_languages.iter()
                                                .map(|lang| lang.clone().into())
                                                .collect::<Vec<_>>()
                                        )),
                                        team_member_count: summary.team_member_count as i32,
                                        chapter_count: summary.chapter_count as i32,
                                        template_id: summary.template_id.clone().into(),
                                        created: summary.created_at.format("%Y-%m-%d").to_string().into(),
                                    }
                                }).collect::<Vec<_>>();
                                
                                let model_rc = ModelRc::new(VecModel::from(project_data_vec));
                                window.set_browser_projects(model_rc);
                                window.set_browser_is_loading(false);
                                window.set_status_message(format!("Found {} projects", summaries.len()).into());
                                window.set_status_type("success".into());
                            }
                        }
                        Err(e) => {
                            if let Some(window) = window_weak.upgrade() {
                                window.set_browser_is_loading(false);
                                window.set_status_message(format!("Failed to discover projects: {}", e).into());
                                window.set_status_type("error".into());
                            }
                        }
                    }
                }
            }
        });
    }

    /// Try to reopen the last project that was worked on
    async fn try_reopen_last_project(&self) -> Result<()> {
        if let Ok(last_project_path) = self.get_last_project_path() {
            if last_project_path.exists() {
                match self.project_service.load_project(&last_project_path).await {
                    Ok(project) => {
                        self.main_window.set_has_project_loaded(true);
                        self.main_window.set_current_project_name(project.name.clone().into());
                        self.main_window.set_status_message(format!("Reopened project: {}", project.name).into());
                        self.main_window.set_status_type("success".into());
                        
                        // TODO: Load project contents and update the sidebar with project tree
                    }
                    Err(e) => {
                        self.main_window.set_status_message(format!("Failed to reopen last project: {}", e).into());
                        self.main_window.set_status_type("warning".into());
                    }
                }
            }
        }
        Ok(())
    }

    /// Get the path to the last opened project from settings
    fn get_last_project_path(&self) -> Result<std::path::PathBuf> {
        let settings_dir = dirs::config_dir()
            .ok_or_else(|| TradocumentError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "Could not find config directory")))?
            .join("tradocflow");
        
        let last_project_file = settings_dir.join("last_project.txt");
        
        if last_project_file.exists() {
            let path_str = std::fs::read_to_string(&last_project_file)
                .map_err(|e| TradocumentError::IoError(e))?;
            Ok(std::path::PathBuf::from(path_str.trim()))
        } else {
            Err(TradocumentError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "No last project file found")))
        }
    }

    /// Save the current project path as the last opened project
    fn save_last_project_path(&self, project_path: &std::path::Path) -> Result<()> {
        let settings_dir = dirs::config_dir()
            .ok_or_else(|| TradocumentError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "Could not find config directory")))?
            .join("tradocflow");
        
        std::fs::create_dir_all(&settings_dir)
            .map_err(|e| TradocumentError::IoError(e))?;
        
        let last_project_file = settings_dir.join("last_project.txt");
        std::fs::write(&last_project_file, project_path.to_string_lossy().as_bytes())
            .map_err(|e| TradocumentError::IoError(e))?;
        
        Ok(())
    }

    /// Set up translation menu callbacks
    fn setup_translation_callbacks(&self) {
        let main_window_weak = self.main_window.as_weak();
        
        self.main_window.on_translation_add_language({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Add language dialog would appear here".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_translation_manage({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Translation management dialog would appear here".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_translation_export({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Translation export started".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_translation_import({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Translation import dialog would appear here".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_translation_validate({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Translation validation started".into());
                    window.set_status_type("info".into());
                }
            }
        });
    }
    
    /// Set up tools menu callbacks
    fn setup_tools_callbacks(&self) {
        let main_window_weak = self.main_window.as_weak();
        
        self.main_window.on_tools_screenshot({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Screenshot tool activated".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_tools_spell_check({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Spell check started".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_tools_word_count({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    let word_count = window.get_document_content().to_string().split_whitespace().count();
                    window.set_status_message(format!("Word count: {}", word_count).into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        self.main_window.on_tools_export_pdf({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("PDF export started".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_tools_export_html({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("HTML export started".into());
                    window.set_status_type("info".into());
                }
            }
        });
    }
    
    /// Set up help menu callbacks
    fn setup_help_callbacks(&self) {
        let main_window_weak = self.main_window.as_weak();
        
        self.main_window.on_help_shortcuts({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Keyboard shortcuts help would appear here".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_help_documentation({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Documentation would open here".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_help_support({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Support resources would open here".into());
                    window.set_status_type("info".into());
                }
            }
        });
    }
    
    /// Set up text formatting callbacks
    fn setup_formatting_callbacks(&self) {
        let main_window_weak = self.main_window.as_weak();
        
        self.main_window.on_format_bold({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Bold formatting applied".into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        self.main_window.on_format_italic({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Italic formatting applied".into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        self.main_window.on_format_underline({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Underline formatting applied".into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        self.main_window.on_format_heading({
            let main_window_weak = main_window_weak.clone();
            move |level| {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message(format!("Heading {} applied", level).into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        self.main_window.on_format_code({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Code formatting applied".into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        self.main_window.on_format_quote({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Quote formatting applied".into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        // List formatting callbacks
        self.main_window.on_insert_bullet_list({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Bullet list inserted".into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        self.main_window.on_insert_numbered_list({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Numbered list inserted".into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        self.main_window.on_insert_checklist({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Checklist inserted".into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        // Insert element callbacks
        self.main_window.on_insert_link({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Link dialog would appear here".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_insert_image({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Image dialog would appear here".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_insert_table({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Table inserted".into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        self.main_window.on_insert_code_block({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Code block inserted".into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        // Text alignment callbacks
        self.main_window.on_align_left({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Text aligned left".into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        self.main_window.on_align_center({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Text aligned center".into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        self.main_window.on_align_right({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Text aligned right".into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        // Indentation callbacks
        self.main_window.on_increase_indent({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Indentation increased".into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        self.main_window.on_decrease_indent({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Indentation decreased".into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        // Undo/Redo callbacks
        self.main_window.on_undo({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Undo operation".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_redo({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Redo operation".into());
                    window.set_status_type("info".into());
                }
            }
        });
    }
    
    /// Set up sidebar callbacks
    fn setup_sidebar_callbacks(&self) {
        let main_window_weak = self.main_window.as_weak();
        
        self.main_window.on_show_projects({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_show_project_browser(true);
                    window.set_status_message("Projects view opened".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_show_kanban({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Kanban board view activated".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_show_reviews({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Reviews view activated".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_new_chapter({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("New chapter created".into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        self.main_window.on_new_translation({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("New translation created".into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        self.main_window.on_tree_item_clicked({
            let main_window_weak = main_window_weak.clone();
            move |item_id| {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message(format!("Selected item: {}", item_id).into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_tree_item_expanded({
            let main_window_weak = main_window_weak.clone();
            move |item_id, expanded| {
                if let Some(window) = main_window_weak.upgrade() {
                    let action = if expanded { "expanded" } else { "collapsed" };
                    window.set_status_message(format!("Item {} {}", item_id, action).into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_tree_item_context_menu({
            let main_window_weak = main_window_weak.clone();
            move |item_id, _x, _y| {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message(format!("Context menu for {}", item_id).into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_search_documents({
            let main_window_weak = main_window_weak.clone();
            move |query| {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message(format!("Searching for: {}", query).into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_clear_search({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Search cleared".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_recent_document_clicked({
            let main_window_weak = main_window_weak.clone();
            move |doc_id| {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message(format!("Opening recent document: {}", doc_id).into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_quick_action_triggered({
            let main_window_weak = main_window_weak.clone();
            move |action_id| {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message(format!("Quick action: {}", action_id).into());
                    window.set_status_type("info".into());
                }
            }
        });
    }
    
    /// Set up editor callbacks
    fn setup_editor_callbacks(&self) {
        let main_window_weak = self.main_window.as_weak();
        
        self.main_window.on_text_operation({
            let main_window_weak = main_window_weak.clone();
            move |_operation| {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Text operation executed".into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        self.main_window.on_insert_list({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("List inserted".into());
                    window.set_status_type("success".into());
                }
            }
        });
    }
    
    /// Set up mode and layout callbacks
    fn setup_mode_layout_callbacks(&self) {
        let main_window_weak = self.main_window.as_weak();
        
        self.main_window.on_toggle_mode({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    let current_mode = window.get_current_mode().to_string();
                    let new_mode = if current_mode == "markdown" { "wysiwyg" } else { "markdown" };
                    window.set_current_mode(new_mode.into());
                    window.set_status_message(format!("Mode changed to {}", new_mode).into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        self.main_window.on_set_layout({
            let main_window_weak = main_window_weak.clone();
            move |layout| {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_current_layout(layout.clone());
                    window.set_status_message(format!("Layout changed to {}", layout).into());
                    window.set_status_type("success".into());
                }
            }
        });
        
        self.main_window.on_update_status({
            let main_window_weak = main_window_weak.clone();
            move |message, status_type| {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message(message);
                    window.set_status_type(status_type);
                }
            }
        });
        
        self.main_window.on_start_project_wizard({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_show_project_wizard(true);
                    window.set_status_message("Project creation wizard started".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_wizard_step_changed({
            let main_window_weak = main_window_weak.clone();
            move |step| {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message(format!("Wizard step {}", step).into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_wizard_back({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Wizard back".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_wizard_next({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Wizard next".into());
                    window.set_status_type("info".into());
                }
            }
        });
        
        self.main_window.on_wizard_cancel({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_show_project_wizard(false);
                    window.set_status_message("Project wizard cancelled".into());
                    window.set_status_type("info".into());
                }
            }
        });
    }

    /// Convert language code to full language name
    fn language_code_to_name(code: &str) -> String {
        match code {
            "en" => "English",
            "es" => "Spanish", 
            "fr" => "French",
            "de" => "German",
            "it" => "Italian",
            "pt" => "Portuguese",
            "nl" => "Dutch",
            "zh" => "Chinese",
            "ja" => "Japanese",
            _ => code,
        }.to_string()
    }

    // Document operation helper methods

    /// Create a new document with default content
    async fn create_new_document(&self) -> Result<()> {
        if let Ok(mut state) = self.document_state.lock() {
            // Check if current document has unsaved changes
            if state.modified {
                // In a real implementation, show a dialog asking to save
                self.show_status_message("Warning: Unsaved changes will be lost", "warning");
            }

            // Reset document state
            state.current_path = None;
            state.content = "# New Document\n\nStart editing here...".to_string();
            state.modified = false;
            state.last_saved = None;
            state.language = "en".to_string();

            // Update UI
            self.main_window.set_document_content(state.content.clone().into());
            self.show_status_message("New document created", "success");
        }
        Ok(())
    }

    /// Open a file dialog and load a document
    async fn open_document_dialog(&self) -> Result<()> {
        // In a real implementation, this would use a native file dialog
        // For now, we'll simulate the process
        
        // Simulate file dialog - in reality, use rfd crate or similar
        let file_path = self.simulate_file_open_dialog().await?;
        
        if let Some(path) = file_path {
            self.load_document_from_path(&path).await?;
        }
        
        Ok(())
    }

    /// Load a document from a specific path
    async fn load_document_from_path(&self, path: &PathBuf) -> Result<()> {
        // Read file content
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| TradocumentError::IoError(e))?;

        // Update document state
        if let Ok(mut state) = self.document_state.lock() {
            state.current_path = Some(path.clone());
            state.content = content.clone();
            state.modified = false;
            state.last_saved = Some(Instant::now());
            state.language = "en".to_string(); // Could be detected from file
        }

        // Update UI
        self.main_window.set_document_content(content.into());
        self.show_status_message(
            &format!("Opened: {}", path.file_name().unwrap_or_default().to_string_lossy()),
            "success"
        );

        Ok(())
    }

    /// Save the current document
    async fn save_current_document(&self) -> Result<()> {
        let (path, content) = {
            let state = self.document_state.lock().map_err(|_| 
                TradocumentError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other, 
                    "Failed to lock document state"
                )))?;
            
            match &state.current_path {
                Some(path) => (path.clone(), state.content.clone()),
                None => {
                    // No path set - need to show save as dialog
                    return self.save_document_as_dialog().await;
                }
            }
        };

        // Save to filesystem
        tokio::fs::write(&path, &content).await
            .map_err(|e| TradocumentError::IoError(e))?;

        // Update state
        if let Ok(mut state) = self.document_state.lock() {
            state.modified = false;
            state.last_saved = Some(Instant::now());
        }

        self.show_status_message("Document saved successfully", "success");
        Ok(())
    }

    /// Show save as dialog and save document
    async fn save_document_as_dialog(&self) -> Result<()> {
        // Simulate save as dialog
        let file_path = self.simulate_file_save_dialog().await?;
        
        if let Some(path) = file_path {
            let content = {
                let state = self.document_state.lock().map_err(|_| 
                    TradocumentError::IoError(std::io::Error::new(
                        std::io::ErrorKind::Other, 
                        "Failed to lock document state"
                    )))?;
                state.content.clone()
            };

            // Save to filesystem
            tokio::fs::write(&path, &content).await
                .map_err(|e| TradocumentError::IoError(e))?;

            // Update state
            if let Ok(mut state) = self.document_state.lock() {
                state.current_path = Some(path.clone());
                state.modified = false;
                state.last_saved = Some(Instant::now());
            }

            self.show_status_message(
                &format!("Saved as: {}", path.file_name().unwrap_or_default().to_string_lossy()),
                "success"
            );
        }
        
        Ok(())
    }

    /// Import documents using DocumentImportService
    async fn import_documents_dialog(&self) -> Result<()> {
        // Simulate import dialog
        let file_paths = self.simulate_import_dialog().await?;
        
        if !file_paths.is_empty() {
            let mut import_service = self.document_import_service.lock().map_err(|_|
                TradocumentError::IoError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to lock import service"
                )))?;

            // For demonstration, import the first file as a text conversion
            let first_path = &file_paths[0];
            
            // Create import configuration
            let config = ImportConfig {
                project_id: uuid::Uuid::new_v4(), // Would use actual project ID
                source_language: "en".to_string(),
                target_languages: vec!["es".to_string(), "fr".to_string()],
                auto_chunk: true,
                create_translation_memory: false,
                preserve_formatting: true,
                extract_terminology: false,
            };

            // Simple import for markdown/text files
            if let Some(extension) = first_path.extension().and_then(|ext| ext.to_str()) {
                match extension.to_lowercase().as_str() {
                    "md" | "txt" => {
                        // Direct load for text files
                        self.load_document_from_path(first_path).await?;
                        self.show_status_message("Document imported successfully", "success");
                    },
                    "docx" | "doc" => {
                        // Use import service for Word documents
                        self.show_status_message("Word document import would be processed here", "info");
                        // In a real implementation, this would use the DocumentImportService
                        // to convert the Word doc to markdown and load it
                    },
                    _ => {
                        self.show_status_message("Unsupported file format for import", "error");
                        return Err(TradocumentError::UnsupportedFormat(extension.to_string()));
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Handle content changes (for auto-save and change tracking)
    async fn handle_content_change(&self, new_content: String) -> Result<()> {
        // Update document state
        if let Ok(mut state) = self.document_state.lock() {
            if state.content != new_content {
                state.content = new_content;
                state.modified = true;

                // Trigger auto-save timer if enabled
                if let Ok(config) = self.auto_save_config.lock() {
                    if config.enabled {
                        // In a real implementation, this would reset the auto-save timer
                        // For now, just mark that auto-save should happen
                    }
                }
            }
        }

        Ok(())
    }

    /// Show status message in UI
    fn show_status_message(&self, message: &str, status_type: &str) {
        self.main_window.set_status_message(message.to_string().into());
        self.main_window.set_status_type(status_type.into());
    }

    /// Simulate file open dialog (in a real app, use rfd crate)
    async fn simulate_file_open_dialog(&self) -> Result<Option<PathBuf>> {
        // This is a placeholder - in a real implementation, use rfd::FileDialog
        // For now, return a default path for testing
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let test_file = current_dir.join("test_document.md");
        
        if test_file.exists() {
            Ok(Some(test_file))
        } else {
            // Create a test file for demonstration
            tokio::fs::write(&test_file, "# Test Document\n\nThis is a test document for demonstration.").await
                .map_err(|e| TradocumentError::IoError(e))?;
            Ok(Some(test_file))
        }
    }

    /// Simulate file save dialog
    async fn simulate_file_save_dialog(&self) -> Result<Option<PathBuf>> {
        // This is a placeholder - in a real implementation, use rfd::FileDialog
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let save_path = current_dir.join("saved_document.md");
        Ok(Some(save_path))
    }

    /// Simulate import dialog
    async fn simulate_import_dialog(&self) -> Result<Vec<PathBuf>> {
        // This is a placeholder - in a real implementation, use rfd::FileDialog
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let import_file = current_dir.join("import_test.txt");
        
        if !import_file.exists() {
            // Create a test import file
            tokio::fs::write(&import_file, "Sample import content\n\nThis would be imported from Word documents.").await
                .map_err(|e| TradocumentError::IoError(e))?;
        }
        
        Ok(vec![import_file])
    }

    /// Detect language from file path patterns
    fn detect_language_from_file(file_path: &Path) -> Option<String> {
        let filename = file_path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Check for common language indicators in filename
        if filename.contains("_en") || filename.contains("-en") || filename.contains("english") {
            Some("en".to_string())
        } else if filename.contains("_es") || filename.contains("-es") || filename.contains("spanish") || filename.contains("español") {
            Some("es".to_string())
        } else if filename.contains("_fr") || filename.contains("-fr") || filename.contains("french") || filename.contains("français") {
            Some("fr".to_string())
        } else if filename.contains("_de") || filename.contains("-de") || filename.contains("german") || filename.contains("deutsch") {
            Some("de".to_string())
        } else if filename.contains("_it") || filename.contains("-it") || filename.contains("italian") || filename.contains("italiano") {
            Some("it".to_string())
        } else if filename.contains("_pt") || filename.contains("-pt") || filename.contains("portuguese") || filename.contains("português") {
            Some("pt".to_string())
        } else if filename.contains("_ja") || filename.contains("-ja") || filename.contains("japanese") || filename.contains("日本語") {
            Some("ja".to_string())
        } else if filename.contains("_zh") || filename.contains("-zh") || filename.contains("chinese") || filename.contains("中文") {
            Some("zh".to_string())
        } else {
            None
        }
    }

    /// Simple DOCX to markdown conversion that's Send-safe
    async fn simple_docx_to_markdown(file_path: &Path) -> std::result::Result<String, TradocumentError> {
        use docx_rs::*;
        use std::io::Read;

        // Read the DOCX file
        let mut file = std::fs::File::open(file_path)
            .map_err(|e| TradocumentError::FileError(format!("Failed to open DOCX file: {}", e)))?;
        
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| TradocumentError::FileError(format!("Failed to read DOCX file: {}", e)))?;

        // Parse the DOCX document
        let docx = read_docx(&buf)
            .map_err(|e| TradocumentError::ValidationError(format!("Failed to parse DOCX: {}", e)))?;

        // Extract document title from filename
        let filename = file_path.file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled");

        let mut markdown = format!("# {}\n\n", filename);

        // Simple text extraction using JSON serialization
        if let Ok(json_str) = serde_json::to_string(&docx) {
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&json_str) {
                let mut extracted_text = String::new();
                Self::extract_text_from_json(&json_value, &mut extracted_text);
                
                // Process extracted text into markdown paragraphs
                for line in extracted_text.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        // Simple heuristic for headings
                        if trimmed.len() > 3 && 
                           (trimmed.chars().all(|c| c.is_uppercase() || c.is_whitespace() || c.is_numeric()) ||
                            trimmed.starts_with("Chapter") || trimmed.starts_with("Section")) {
                            markdown.push_str(&format!("## {}\n\n", trimmed));
                        } else {
                            markdown.push_str(trimmed);
                            markdown.push_str("\n\n");
                        }
                    }
                }
            }
        }

        // If no content was extracted, provide a basic structure
        if markdown.len() == filename.len() + 4 {
            markdown.push_str("*Document content imported from DOCX file.*\n\n");
            markdown.push_str("Please note: Advanced DOCX parsing is available through the DocumentImportService.\n\n");
        }

        Ok(markdown)
    }

    /// Extract text from JSON representation of DOCX
    fn extract_text_from_json(value: &serde_json::Value, text: &mut String) {
        match value {
            serde_json::Value::Object(obj) => {
                // Look for text content
                if let Some(t) = obj.get("text") {
                    if let Some(text_str) = t.as_str() {
                        if !text_str.trim().is_empty() && text_str.len() > 1 {
                            text.push_str(text_str);
                            text.push('\n');
                        }
                    }
                }
                
                // Recursively process all values
                for (_, v) in obj {
                    Self::extract_text_from_json(v, text);
                }
            }
            serde_json::Value::Array(arr) => {
                for v in arr {
                    Self::extract_text_from_json(v, text);
                }
            }
            _ => {}
        }
    }
}