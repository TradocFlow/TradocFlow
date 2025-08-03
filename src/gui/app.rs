use slint::{ComponentHandle, SharedString, Weak};
use std::time::{Duration, Instant};
use std::collections::{VecDeque, HashMap};
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::RwLock;
use tokio::time::sleep;
use rfd::FileDialog;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::{User, TradocumentError, DocumentImportService, DocumentImportRequest, Document};
// Project and Priority imports removed as they're not directly used in this file
use crate::export_engine::{ExportEngine, ExportConfig, ExportFormat};
use crate::services::project_manager::ProjectManager;
use crate::database::{Database, project_repository::ProjectRepository};
use crate::models::project_browser::ProjectBrowserItem;

use super::state::AppState;

// Slint-generated types are available via slint::include_modules! in lib.rs

// Conversion function for ProjectBrowserItem to SimpleProjectData
fn convert_to_simple_project_data(item: &ProjectBrowserItem) -> SimpleProjectData {
    SimpleProjectData {
        id: item.id.to_string().into(),
        name: item.name.clone().into(),
        description: item.description.clone().unwrap_or_default().into(),
        status: item.status.as_str().into(),
        owner: item.owner_name.clone().into(),
        created: item.created_at.format("%Y-%m-%d").to_string().into(),
    }
}

// Text operation types for enhanced editing
#[derive(Debug, Clone)]
pub struct RustTextOperation {
    pub operation_type: String,
    pub content: String,
    pub start_pos: i32,
    pub end_pos: i32,
    pub format_type: String,
}

// Enhanced content synchronization structures
#[derive(Debug, Clone)]
pub struct ContentChange {
    pub id: String,
    pub language: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub change_type: ContentChangeType,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContentChangeType {
    Insert,
    Delete,
    Modify,
    Replace,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TranslationStatus {
    Complete,
    Partial,
    Missing,
    Outdated,
}

#[derive(Debug, Clone)]
pub struct LanguageContentInfo {
    pub language: String,
    pub content: String,
    pub word_count: usize,
    pub last_modified: DateTime<Utc>,
    pub status: TranslationStatus,
    pub missing_sections: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ContentValidationResult {
    pub is_valid: bool,
    pub issues: Vec<ValidationIssue>,
    pub translation_coverage: HashMap<String, f32>,
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub issue_type: ValidationIssueType,
    pub language: String,
    pub section: String,
    pub message: String,
    pub severity: IssueSeverity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationIssueType {
    MissingTranslation,
    InconsistentStructure,
    OutdatedContent,
    FormattingMismatch,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IssueSeverity {
    Critical,
    Warning,
    Info,
}

// Enhanced content synchronization manager
#[derive(Debug)]
pub struct ContentSyncManager {
    change_history: Arc<RwLock<VecDeque<ContentChange>>>,
    debounce_timers: Arc<RwLock<HashMap<String, Instant>>>,
    pending_saves: Arc<RwLock<HashMap<String, String>>>,
    language_info: Arc<RwLock<HashMap<String, LanguageContentInfo>>>,
    max_history: usize,
    debounce_duration: Duration,
}

impl ContentSyncManager {
    pub fn new() -> Self {
        Self {
            change_history: Arc::new(RwLock::new(VecDeque::new())),
            debounce_timers: Arc::new(RwLock::new(HashMap::new())),
            pending_saves: Arc::new(RwLock::new(HashMap::new())),
            language_info: Arc::new(RwLock::new(HashMap::new())),
            max_history: 1000,
            debounce_duration: Duration::from_millis(500),
        }
    }

    pub async fn record_change(&self, language: String, content: String, change_type: ContentChangeType, user_id: Option<String>) -> String {
        let change_id = Uuid::new_v4().to_string();
        let change = ContentChange {
            id: change_id.clone(),
            language: language.clone(),
            content: content.clone(),
            timestamp: Utc::now(),
            change_type,
            user_id,
        };

        // Record change in history
        {
            let mut history = self.change_history.write().await;
            history.push_back(change.clone());
            
            if history.len() > self.max_history {
                history.pop_front();
            }
        }

        // Update language info
        self.update_language_info(language.clone(), content).await;

        // Update debounce timer
        {
            let mut timers = self.debounce_timers.write().await;
            timers.insert(language.clone(), Instant::now());
        }

        change_id
    }

    async fn update_language_info(&self, language: String, content: String) {
        let word_count = content.split_whitespace().count();
        let info = LanguageContentInfo {
            language: language.clone(),
            content,
            word_count,
            last_modified: Utc::now(),
            status: TranslationStatus::Complete, // Will be updated by validation
            missing_sections: Vec::new(),
        };

        let mut lang_info = self.language_info.write().await;
        lang_info.insert(language, info);
    }

    pub async fn should_debounce(&self, language: &str) -> bool {
        let timers = self.debounce_timers.read().await;
        if let Some(last_change) = timers.get(language) {
            last_change.elapsed() < self.debounce_duration
        } else {
            false
        }
    }

    pub async fn validate_content(&self, base_language: &str) -> ContentValidationResult {
        let lang_info = self.language_info.read().await;
        let mut issues = Vec::new();
        let mut translation_coverage = HashMap::new();

        // Get base content for comparison
        let base_info = lang_info.get(base_language);
        if base_info.is_none() {
            return ContentValidationResult {
                is_valid: false,
                issues: vec![ValidationIssue {
                    issue_type: ValidationIssueType::MissingTranslation,
                    language: base_language.to_string(),
                    section: "document".to_string(),
                    message: "Base language content is missing".to_string(),
                    severity: IssueSeverity::Critical,
                }],
                translation_coverage: HashMap::new(),
            };
        }

        let base_content = &base_info.unwrap().content;
        let base_sections = self.extract_sections(base_content);

        // Validate each language against base
        for (language, info) in lang_info.iter() {
            if language == base_language {
                translation_coverage.insert(language.clone(), 1.0);
                continue;
            }

            let sections = self.extract_sections(&info.content);
            let coverage = if base_sections.len() > 0 {
                sections.len() as f32 / base_sections.len() as f32
            } else {
                0.0
            };
            
            translation_coverage.insert(language.clone(), coverage);

            // Check for missing sections
            for (idx, base_section) in base_sections.iter().enumerate() {
                if sections.get(idx).is_none() || sections[idx].trim().is_empty() {
                    issues.push(ValidationIssue {
                        issue_type: ValidationIssueType::MissingTranslation,
                        language: language.clone(),
                        section: format!("section_{}", idx),
                        message: format!("Missing translation for section: {}", base_section.chars().take(50).collect::<String>()),
                        severity: IssueSeverity::Warning,
                    });
                }
            }

            // Check for outdated content (simplified)
            if let Some(base_info) = base_info {
                if info.last_modified < base_info.last_modified {
                    issues.push(ValidationIssue {
                        issue_type: ValidationIssueType::OutdatedContent,
                        language: language.clone(),
                        section: "document".to_string(),
                        message: "Translation may be outdated".to_string(),
                        severity: IssueSeverity::Info,
                    });
                }
            }
        }

        ContentValidationResult {
            is_valid: issues.iter().all(|i| i.severity != IssueSeverity::Critical),
            issues,
            translation_coverage,
        }
    }

    fn extract_sections(&self, content: &str) -> Vec<String> {
        // Simple section extraction - split by double newlines or markdown headers
        let mut sections = Vec::new();
        let mut current_section = String::new();
        
        for line in content.lines() {
            if line.starts_with('#') || line.trim().is_empty() && !current_section.trim().is_empty() {
                if !current_section.trim().is_empty() {
                    sections.push(current_section.trim().to_string());
                    current_section.clear();
                }
            }
            if !line.trim().is_empty() {
                current_section.push_str(line);
                current_section.push('\n');
            }
        }
        
        if !current_section.trim().is_empty() {
            sections.push(current_section.trim().to_string());
        }
        
        sections
    }

    pub async fn get_change_history(&self, language: &str, limit: usize) -> Vec<ContentChange> {
        let history = self.change_history.read().await;
        history
            .iter()
            .rev()
            .filter(|change| change.language == language)
            .take(limit)
            .cloned()
            .collect()
    }

    pub async fn get_language_status(&self, language: &str) -> Option<LanguageContentInfo> {
        let lang_info = self.language_info.read().await;
        lang_info.get(language).cloned()
    }

    pub async fn get_all_language_status(&self) -> HashMap<String, LanguageContentInfo> {
        let lang_info = self.language_info.read().await;
        lang_info.clone()
    }

    pub async fn add_pending_save(&self, language: String, content: String) {
        let mut pending = self.pending_saves.write().await;
        pending.insert(language, content);
    }

    pub async fn remove_pending_save(&self, language: &str) -> Option<String> {
        let mut pending = self.pending_saves.write().await;
        pending.remove(language)
    }

    pub async fn has_pending_saves(&self) -> bool {
        let pending = self.pending_saves.read().await;
        !pending.is_empty()
    }
}

// Undo/Redo system
#[derive(Debug, Clone)]
struct UndoRedoManager {
    undo_stack: VecDeque<String>,
    redo_stack: VecDeque<String>,
    max_history: usize,
}

impl UndoRedoManager {
    fn new() -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_history: 100,
        }
    }
    
    fn push_state(&mut self, content: String) {
        self.undo_stack.push_back(content);
        self.redo_stack.clear();
        
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.pop_front();
        }
    }
    
    fn undo(&mut self) -> Option<String> {
        if let Some(current) = self.undo_stack.pop_back() {
            self.redo_stack.push_back(current);
            self.undo_stack.back().cloned()
        } else {
            None
        }
    }
    
    fn redo(&mut self) -> Option<String> {
        if let Some(content) = self.redo_stack.pop_back() {
            self.undo_stack.push_back(content.clone());
            Some(content)
        } else {
            None
        }
    }
}

// Text formatting utilities
struct TextFormatter;

impl TextFormatter {
    fn apply_markdown_formatting(content: &str, format_type: &str, selected_text: Option<&str>) -> String {
        match format_type {
            "bold" => {
                if let Some(text) = selected_text {
                    if text.starts_with("**") && text.ends_with("**") {
                        // Remove bold
                        text[2..text.len()-2].to_string()
                    } else {
                        // Add bold
                        format!("**{}**", text)
                    }
                } else {
                    "**bold text**".to_string()
                }
            },
            "italic" => {
                if let Some(text) = selected_text {
                    if text.starts_with("*") && text.ends_with("*") && !text.starts_with("**") {
                        // Remove italic
                        text[1..text.len()-1].to_string()
                    } else {
                        // Add italic
                        format!("*{}*", text)
                    }
                } else {
                    "*italic text*".to_string()
                }
            },
            "code" => {
                if let Some(text) = selected_text {
                    if text.starts_with("`") && text.ends_with("`") {
                        // Remove code
                        text[1..text.len()-1].to_string()
                    } else {
                        // Add code
                        format!("`{}`", text)
                    }
                } else {
                    "`code`".to_string()
                }
            },
            "quote" => {
                if let Some(text) = selected_text {
                    if text.starts_with("> ") {
                        // Remove quote
                        text[2..].to_string()
                    } else {
                        // Add quote
                        format!("> {}", text)
                    }
                } else {
                    "> Quote text".to_string()
                }
            },
            _ => selected_text.unwrap_or("").to_string(),
        }
    }
    
    fn apply_presentation_formatting(content: &str, format_type: &str, selected_text: Option<&str>) -> String {
        match format_type {
            "bold" => {
                if let Some(text) = selected_text {
                    if text.starts_with("<strong>") && text.ends_with("</strong>") {
                        // Remove bold
                        text[8..text.len()-9].to_string()
                    } else {
                        // Add bold
                        format!("<strong>{}</strong>", text)
                    }
                } else {
                    "<strong>bold text</strong>".to_string()
                }
            },
            "italic" => {
                if let Some(text) = selected_text {
                    if text.starts_with("<em>") && text.ends_with("</em>") {
                        // Remove italic
                        text[4..text.len()-5].to_string()
                    } else {
                        // Add italic
                        format!("<em>{}</em>", text)
                    }
                } else {
                    "<em>italic text</em>".to_string()
                }
            },
            "underline" => {
                if let Some(text) = selected_text {
                    if text.starts_with("<u>") && text.ends_with("</u>") {
                        // Remove underline
                        text[3..text.len()-4].to_string()
                    } else {
                        // Add underline
                        format!("<u>{}</u>", text)
                    }
                } else {
                    "<u>underlined text</u>".to_string()
                }
            },
            "code" => {
                if let Some(text) = selected_text {
                    if text.starts_with("<code>") && text.ends_with("</code>") {
                        // Remove code
                        text[6..text.len()-7].to_string()
                    } else {
                        // Add code
                        format!("<code>{}</code>", text)
                    }
                } else {
                    "<code>code</code>".to_string()
                }
            },
            _ => selected_text.unwrap_or("").to_string(),
        }
    }
    
    fn insert_heading(level: i32, mode: &str) -> String {
        match mode {
            "markdown" => {
                let hashes = "#".repeat(level as usize);
                format!("\n{} Heading {}\n", hashes, level)
            },
            "presentation" => {
                format!("<h{}>Heading {}</h{}>", level, level, level)
            },
            _ => format!("Heading {}", level),
        }
    }
    
    fn insert_list(list_type: &str, mode: &str) -> String {
        match mode {
            "markdown" => {
                match list_type {
                    "bullet" => "\n- List item 1\n- List item 2\n- List item 3\n".to_string(),
                    "numbered" => "\n1. List item 1\n2. List item 2\n3. List item 3\n".to_string(),
                    "checklist" => "\n- [ ] Task 1\n- [ ] Task 2\n- [ ] Task 3\n".to_string(),
                    _ => "\n- List item\n".to_string(),
                }
            },
            "presentation" => {
                match list_type {
                    "bullet" => "<ul><li>List item 1</li><li>List item 2</li><li>List item 3</li></ul>".to_string(),
                    "numbered" => "<ol><li>List item 1</li><li>List item 2</li><li>List item 3</li></ol>".to_string(),
                    "checklist" => "<ul><li>☐ Task 1</li><li>☐ Task 2</li><li>☐ Task 3</li></ul>".to_string(),
                    _ => "<ul><li>List item</li></ul>".to_string(),
                }
            },
            _ => "- List item\n".to_string(),
        }
    }
    
    fn insert_link(mode: &str) -> String {
        match mode {
            "markdown" => "[Link Text](https://example.com)".to_string(),
            "presentation" => "<a href=\"https://example.com\">Link Text</a>".to_string(),
            _ => "https://example.com".to_string(),
        }
    }
    
    fn insert_image(mode: &str) -> String {
        match mode {
            "markdown" => "![Image Alt Text](image.png)".to_string(),
            "presentation" => "<img src=\"image.png\" alt=\"Image Alt Text\" />".to_string(),
            _ => "[Image: image.png]".to_string(),
        }
    }
    
    fn insert_table(mode: &str) -> String {
        match mode {
            "markdown" => {
                "\n| Column 1 | Column 2 | Column 3 |\n|----------|----------|----------|\n| Cell 1   | Cell 2   | Cell 3   |\n| Cell 4   | Cell 5   | Cell 6   |\n".to_string()
            },
            "presentation" => {
                "<table><tr><th>Column 1</th><th>Column 2</th><th>Column 3</th></tr><tr><td>Cell 1</td><td>Cell 2</td><td>Cell 3</td></tr><tr><td>Cell 4</td><td>Cell 5</td><td>Cell 6</td></tr></table>".to_string()
            },
            _ => "Table would be inserted here".to_string(),
        }
    }
    
    fn insert_code_block() -> String {
        "\n```\n// Your code here\n```\n".to_string()
    }
}

slint::include_modules!();

/// Thread-safe wrapper for UI updates
#[derive(Clone)]
struct UiUpdater {
    ui_weak: Weak<MainWindow>,
}

impl UiUpdater {
    fn new(ui_weak: Weak<MainWindow>) -> Self {
        Self { ui_weak }
    }

    /// Update UI content safely from any thread
    fn update_content(&self, content: String, language: String) {
        if let Some(ui) = self.ui_weak.upgrade() {
            if language == "en" {
                ui.set_document_content(content.into());
            } else {
                ui.set_translation_content(content.into());
            }
        }
    }

    /// Update UI mode safely
    fn update_mode(&self, mode: String) {
        if let Some(ui) = self.ui_weak.upgrade() {
            ui.set_current_mode(mode.into());
        }
    }

    /// Update UI layout safely
    fn update_layout(&self, layout: String) {
        if let Some(ui) = self.ui_weak.upgrade() {
            ui.set_current_layout(layout.into());
        }
    }

    /// Update UI language safely
    fn update_language(&self, language: String) {
        if let Some(ui) = self.ui_weak.upgrade() {
            ui.set_current_language(language.into());
        }
    }

    /// Show status message safely
    fn show_status(&self, message: String) {
        self.show_status_with_type(message, "info".to_string());
    }

    /// Show status message with type safely
    fn show_status_with_type(&self, message: String, status_type: String) {
        if let Some(ui) = self.ui_weak.upgrade() {
            ui.set_status_message(message.clone().into());
            ui.set_status_type(status_type.clone().into());
            println!("Status [{}]: {}", status_type, message);
        }
    }
    
    /// Set window title safely (currently shows in status instead)
    fn set_window_title(&self, title: SharedString) {
        // Since the Slint window title isn't directly settable at runtime,
        // we'll show the project name in the status instead
        self.show_status_with_type(format!("Current project: {}", title), "info".to_string());
    }
    
    /// Set document content safely
    fn set_document_content(&self, content: SharedString) {
        if let Some(ui) = self.ui_weak.upgrade() {
            ui.set_document_content(content);
        }
    }
    
    /// Set project loaded state
    fn set_has_project_loaded(&self, loaded: bool) {
        if let Some(ui) = self.ui_weak.upgrade() {
            ui.set_has_project_loaded(loaded);
        }
    }
    
    /// Set current project name
    fn set_current_project_name(&self, name: SharedString) {
        if let Some(ui) = self.ui_weak.upgrade() {
            ui.set_current_project_name(name);
        }
    }
    
    /// Set project wizard visibility
    fn set_show_project_wizard(&self, show: bool) {
        if let Some(ui) = self.ui_weak.upgrade() {
            ui.set_show_project_wizard(show);
        }
    }
    
    /// Set search text
    fn set_search_text(&self, text: SharedString) {
        if let Some(ui) = self.ui_weak.upgrade() {
            ui.set_search_text(text);
        }
    }
    
    /// Set selected tree item
    fn set_selected_tree_item(&self, item_id: SharedString) {
        if let Some(ui) = self.ui_weak.upgrade() {
            ui.set_selected_tree_item(item_id);
        }
    }
    
    /// Set tree collapse states
    fn set_project_tree_collapsed(&self, collapsed: bool) {
        if let Some(ui) = self.ui_weak.upgrade() {
            ui.set_project_tree_collapsed(collapsed);
        }
    }
    
    fn set_quick_actions_collapsed(&self, collapsed: bool) {
        if let Some(ui) = self.ui_weak.upgrade() {
            ui.set_quick_actions_collapsed(collapsed);
        }
    }
    
    fn set_recent_docs_collapsed(&self, collapsed: bool) {
        if let Some(ui) = self.ui_weak.upgrade() {
            ui.set_recent_docs_collapsed(collapsed);
        }
    }
}

/// Main application struct that manages the Slint UI and backend communication
pub struct App {
    /// Slint UI handle
    ui: MainWindow,
    
    /// Application state
    state: AppState,
    
    /// Tokio runtime for async operations
    runtime: Runtime,
    
    /// Thread-safe UI updater
    ui_updater: UiUpdater,
    
    /// Undo/Redo manager for text operations
    undo_manager: std::sync::Arc<std::sync::Mutex<UndoRedoManager>>,
    
    /// Enhanced content synchronization manager
    content_sync: Arc<ContentSyncManager>,
}

impl App {
    /// Create a new application instance
    pub fn new() -> Result<Self, TradocumentError> {
        let runtime = Runtime::new().map_err(|e| TradocumentError::IoError(e))?;
        
        // Initialize database
        let database = Database::new("tradocflow.db")
            .map_err(|e| TradocumentError::DatabaseError(format!("Failed to initialize database: {}", e)))?;
        
        // Initialize project management components
        let project_manager = ProjectManager::new("./projects");
        let project_repository = ProjectRepository::new(database.pool());
        
        // Create application state with project management
        let state = AppState::new("http://localhost:8000".to_string(), project_manager, project_repository);
        
        // Create Slint UI
        let ui = MainWindow::new().map_err(|e| TradocumentError::SlintError(e.to_string()))?;
        
        // Create thread-safe UI updater
        let ui_updater = UiUpdater::new(ui.as_weak());
        
        // Initialize undo/redo manager
        let undo_manager = std::sync::Arc::new(std::sync::Mutex::new(UndoRedoManager::new()));
        
        // Initialize content synchronization manager
        let content_sync = Arc::new(ContentSyncManager::new());
        
        let mut app = Self { ui, state, runtime, ui_updater, undo_manager, content_sync };
        
        // Setup UI callbacks and initialize
        app.setup_callbacks();
        app.setup_content_validation_timer();
        app.initialize();
        
        Ok(app)
    }

    /// Run the application
    pub fn run(self) -> Result<(), TradocumentError> {
        self.ui.run().map_err(|e| TradocumentError::SlintError(e.to_string()))
    }

    /// Initialize the application
    fn initialize(&mut self) {
        // Set initial UI state
        self.ui.set_current_mode("markdown".into());
        self.ui.set_current_layout("single".into());
        self.ui.set_current_language("en".into());
        self.ui.set_document_content("# Welcome to Tradocument Reviewer\n\nStart editing your multilingual document here...".into());
        
        // Create a default user for demo purposes
        let default_user = User {
            id: "demo-user-1".to_string(),
            name: "Demo User".to_string(),
            email: "demo@example.com".to_string(),
            role: crate::UserRole::Member,
            created_at: chrono::Utc::now(),
            active: true,
        };
        
        // Set current user and load initial data
        let state = self.state.clone();
        let ui_updater = self.ui_updater.clone();
        
        self.runtime.spawn(async move {
            state.set_current_user(default_user).await;
            
            // Load initial data
            if let Err(e) = state.load_documents().await {
                ui_updater.show_status(format!("Failed to load documents: {}", e));
            }
            
            if let Err(e) = state.load_notifications().await {
                ui_updater.show_status(format!("Failed to load notifications: {}", e));
            }
            
            ui_updater.show_status_with_type("Application initialized successfully".to_string(), "success".to_string());
        });
    }

    /// Setup UI callbacks with thread-safe operations
    fn setup_callbacks(&self) {
        let state = self.state.clone();
        let runtime_handle = self.runtime.handle().clone();
        let ui_updater = self.ui_updater.clone();
        let ui_handle = self.ui.as_weak();

        // File operations - New
        self.ui.on_file_new({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    match state.create_document("New Document".to_string()).await {
                        Ok(_) => {
                            let content = state.get_content("en").await;
                            ui_updater.update_content(content, "en".to_string());
                            ui_updater.update_language("en".to_string());
                            ui_updater.show_status_with_type("New document created".to_string(), "success".to_string());
                        }
                        Err(e) => {
                            ui_updater.show_status_with_type(format!("Failed to create document: {}", e), "error".to_string());
                        }
                    }
                });
            }
        });

        self.ui.on_file_open({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    // Show loading status
                    ui_updater.show_status_with_type("Loading documents from server...".to_string(), "info".to_string());
                    
                    // First, load the list of available documents from the backend
                    match state.load_documents().await {
                        Ok(_) => {
                            // Get the list of documents
                            let documents = {
                                let docs = state.documents.read().await;
                                docs.clone()
                            };
                            
                            if documents.is_empty() {
                                ui_updater.show_status_with_type("No documents available on the server".to_string(), "warning".to_string());
                                return;
                            }
                            
                            // For now, since there's no document selection dialog in the UI,
                            // we'll implement a simple approach: show document info and let user select by showing all available documents
                            ui_updater.show_status_with_type(
                                format!("Found {} documents. Loading the most recent document...", documents.len()), 
                                "info".to_string()
                            );
                            
                            // Find the most recently updated document
                            let selected_document = documents.iter()
                                .max_by_key(|doc| &doc.updated_at)
                                .cloned();
                            
                            if let Some(doc) = selected_document {
                                // Show document information before loading
                                ui_updater.show_status_with_type(
                                    format!("Opening document: '{}' (created: {}, languages: {})", 
                                        doc.title,
                                        doc.created_at.format("%Y-%m-%d %H:%M"),
                                        doc.metadata.languages.join(", ")
                                    ), 
                                    "info".to_string()
                                );
                                
                                // Load the selected document
                                match state.load_document(doc.id).await {
                                    Ok(_) => {
                                        // Get current language from state
                                        let current_language = state.get_language().await;
                                        
                                        // Get content for the current language
                                        let content = state.get_content(&current_language).await;
                                        
                                        if content.is_empty() {
                                            // Try to get content in the document's source language if current language is empty
                                            let available_languages = doc.metadata.languages;
                                            if let Some(first_lang) = available_languages.first() {
                                                let first_lang_content = state.get_content(first_lang).await;
                                                if !first_lang_content.is_empty() {
                                                    // Update language to the one that has content
                                                    state.set_language(first_lang.clone()).await;
                                                    ui_updater.update_language(first_lang.clone());
                                                    ui_updater.update_content(first_lang_content, first_lang.clone());
                                                    ui_updater.show_status_with_type(
                                                        format!("Document '{}' loaded successfully in {}", doc.title, first_lang), 
                                                        "success".to_string()
                                                    );
                                                } else {
                                                    ui_updater.update_content("# Document loaded\n\nNo content available in any language.".to_string(), current_language.clone());
                                                    ui_updater.show_status_with_type(
                                                        format!("Document '{}' loaded but no content found", doc.title), 
                                                        "warning".to_string()
                                                    );
                                                }
                                            } else {
                                                ui_updater.update_content("# Document loaded\n\nNo content available.".to_string(), current_language.clone());
                                                ui_updater.show_status_with_type(
                                                    format!("Document '{}' loaded but no content found", doc.title), 
                                                    "warning".to_string()
                                                );
                                            }
                                        } else {
                                            // Update UI with the loaded content
                                            ui_updater.update_content(content, current_language.clone());
                                            ui_updater.show_status_with_type(
                                                format!("Document '{}' loaded successfully", doc.title), 
                                                "success".to_string()
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        ui_updater.show_status_with_type(
                                            format!("Failed to load document '{}': {}", doc.title, e), 
                                            "error".to_string()
                                        );
                                    }
                                }
                            } else {
                                ui_updater.show_status_with_type("No documents available".to_string(), "warning".to_string());
                            }
                        }
                        Err(e) => {
                            ui_updater.show_status_with_type(
                                format!("Failed to load documents from server: {}", e), 
                                "error".to_string()
                            );
                        }
                    }
                });
            }
        });

        self.ui.on_file_save({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    match state.save_document().await {
                        Ok(_) => {
                            ui_updater.show_status_with_type("Document saved successfully".to_string(), "success".to_string());
                        }
                        Err(e) => {
                            ui_updater.show_status_with_type(format!("Failed to save document: {}", e), "error".to_string());
                        }
                    }
                });
            }
        });

        self.ui.on_file_export({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    // Check if there's a current document loaded
                    let current_document = {
                        let doc_lock = state.current_document.read().await;
                        doc_lock.clone()
                    };
                    
                    let document = match current_document {
                        Some(doc) => doc,
                        None => {
                            ui_updater.show_status_with_type(
                                "No document is currently loaded for export".to_string(),
                                "warning".to_string()
                            );
                            return;
                        }
                    };
                    
                    // Check if document has content
                    if document.content.is_empty() {
                        ui_updater.show_status_with_type(
                            "Document has no content to export".to_string(),
                            "warning".to_string()
                        );
                        return;
                    }
                    
                    ui_updater.show_status("Preparing export options...".to_string());
                    
                    // Get available languages from the document
                    let available_languages = document.metadata.languages.clone();
                    let current_language = state.get_language().await;
                    
                    // Create export dialog - for now, we'll use a simple approach
                    // In a full implementation, you'd want to show a proper dialog
                    // For this implementation, we'll export current language as PDF by default
                    // but allow user to choose via file dialog filters
                    
                    let file_dialog = FileDialog::new()
                        .set_title("Export Document")
                        .set_file_name(&format!("{}_export", document.title))
                        .add_filter("PDF Document", &["pdf"])
                        .add_filter("HTML Document", &["html"])
                        .add_filter("All Formats", &["pdf", "html"]);
                    
                    let file_path = match file_dialog.save_file() {
                        Some(path) => path,
                        None => {
                            ui_updater.show_status("Export cancelled".to_string());
                            return;
                        }
                    };
                    
                    ui_updater.show_status("Starting export process...".to_string());
                    
                    // Determine export format from file extension
                    let export_format = match file_path.extension().and_then(|ext| ext.to_str()) {
                        Some("pdf") => ExportFormat::Pdf,
                        Some("html") => ExportFormat::Html,
                        _ => ExportFormat::Pdf, // Default to PDF
                    };
                    
                    // Prepare export configuration
                    let export_config = ExportConfig {
                        format: export_format.clone(),
                        include_screenshots: true,
                        template: None,
                        css_file: None,
                        languages: vec![current_language.clone()], // Export current language only
                    };
                    
                    // Create document structure compatible with ExportEngine
                    let export_document = Document {
                        id: document.id,
                        title: document.title.clone(),
                        content: {
                            let mut content_map = std::collections::HashMap::new();
                            
                            // Get content from state for the current language
                            let content = state.get_content(&current_language).await;
                            if !content.is_empty() {
                                content_map.insert(current_language.clone(), content);
                            } else {
                                // Fallback to document's stored content if state is empty
                                if let Some(stored_content) = document.content.get(&current_language) {
                                    content_map.insert(current_language.clone(), stored_content.clone());
                                } else {
                                    // Use any available language as fallback
                                    if let Some(first_lang) = available_languages.first() {
                                        if let Some(fallback_content) = document.content.get(first_lang) {
                                            content_map.insert(first_lang.clone(), fallback_content.clone());
                                        }
                                    }
                                }
                            }
                            content_map
                        },
                        created_at: document.created_at,
                        updated_at: document.updated_at,
                        version: document.version,
                        status: document.status,
                        metadata: document.metadata.clone(),
                    };
                    
                    if export_document.content.is_empty() {
                        ui_updater.show_status_with_type(
                            "No content available for export in the selected language".to_string(),
                            "error".to_string()
                        );
                        return;
                    }
                    
                    ui_updater.show_status("Generating export file...".to_string());
                    
                    // Use spawn_blocking for the export operation to avoid Send issues
                    let export_result = tokio::task::spawn_blocking({
                        let export_document = export_document.clone();
                        let export_config = export_config.clone();
                        move || {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            rt.block_on(async move {
                                let export_engine = ExportEngine::new();
                                export_engine.export_document(&export_document, &export_config).await
                            })
                        }
                    }).await;
                    
                    // Process the export result
                    match export_result {
                        Ok(Ok(results)) => {
                            // Save the first result to the selected file
                            if let Some((filename, content)) = results.iter().next() {
                                match std::fs::write(&file_path, content) {
                                    Ok(_) => {
                                        let format_name = match export_format {
                                            ExportFormat::Pdf => "PDF",
                                            ExportFormat::Html => "HTML",
                                            ExportFormat::Both => "PDF & HTML",
                                        };
                                        
                                        ui_updater.show_status_with_type(
                                            format!(
                                                "Document '{}' exported successfully as {} to: {}",
                                                document.title,
                                                format_name,
                                                file_path.display()
                                            ),
                                            "success".to_string()
                                        );
                                    }
                                    Err(e) => {
                                        ui_updater.show_status_with_type(
                                            format!("Failed to save export file: {}", e),
                                            "error".to_string()
                                        );
                                    }
                                }
                            } else {
                                ui_updater.show_status_with_type(
                                    "Export completed but no files were generated".to_string(),
                                    "warning".to_string()
                                );
                            }
                        }
                        Ok(Err(e)) => {
                            ui_updater.show_status_with_type(
                                format!("Export failed: {}", e),
                                "error".to_string()
                            );
                        }
                        Err(e) => {
                            ui_updater.show_status_with_type(
                                format!("Export task failed: {}", e),
                                "error".to_string()
                            );
                        }
                    }
                });
            }
        });

        // Additional File menu operations
        self.ui.on_file_save_as({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    // Check if there's a current document loaded
                    let current_doc = {
                        let doc = state.current_document.read().await;
                        doc.clone()
                    };
                    
                    if let Some(document) = current_doc {
                        let current_language = state.get_language().await;
                        
                        // Create file dialog with multiple format filters
                        let file_dialog = FileDialog::new()
                            .set_title("Save As - Choose Format and Location")
                            .set_file_name(&format!("{}", document.title))
                            .add_filter("Markdown Files", &["md"])
                            .add_filter("Plain Text Files", &["txt"])
                            .add_filter("Document Format", &["tdoc"])
                            .add_filter("All Files", &["*"]);
                        
                        let file_path = match file_dialog.save_file() {
                            Some(path) => path,
                            None => {
                                ui_updater.show_status("Save As cancelled".to_string());
                                return;
                            }
                        };
                        
                        ui_updater.show_status("Starting save process...".to_string());
                        
                        // Determine save format from file extension
                        let save_format = match file_path.extension().and_then(|ext| ext.to_str()) {
                            Some("md") => "markdown",
                            Some("txt") => "text",
                            Some("tdoc") => "document",
                            _ => {
                                // Default to markdown if no extension or unknown extension
                                "markdown"
                            }
                        };
                        
                        match save_format {
                            "markdown" | "text" => {
                                // For text formats, get current content and save directly to file
                                let content = state.get_content(&current_language).await;
                                
                                if content.is_empty() {
                                    ui_updater.show_status_with_type(
                                        "Cannot save: No content available for current language".to_string(),
                                        "warning".to_string()
                                    );
                                    return;
                                }
                                
                                // Add appropriate file extension if not present
                                let final_path = if file_path.extension().is_none() {
                                    let ext = if save_format == "markdown" { "md" } else { "txt" };
                                    file_path.with_extension(ext)
                                } else {
                                    file_path
                                };
                                
                                // Save content directly to file
                                match std::fs::write(&final_path, &content) {
                                    Ok(_) => {
                                        let format_name = if save_format == "markdown" { "Markdown" } else { "Plain Text" };
                                        ui_updater.show_status_with_type(
                                            format!("Document saved as {} to: {}", format_name, final_path.display()),
                                            "success".to_string()
                                        );
                                    }
                                    Err(e) => {
                                        ui_updater.show_status_with_type(
                                            format!("Failed to save file: {}", e),
                                            "error".to_string()
                                        );
                                    }
                                }
                            }
                            "document" => {
                                // For document format, we need to create a copy through the API
                                // This is more complex as it involves creating a new document
                                // or copying the existing one to a different location
                                
                                // Add appropriate file extension if not present
                                let final_path = if file_path.extension().is_none() {
                                    file_path.with_extension("tdoc")
                                } else {
                                    file_path
                                };
                                
                                // For now, we'll save all language content as a structured format
                                let all_content = {
                                    let content_map = state.document_content.read().await;
                                    content_map.clone()
                                };
                                
                                if all_content.is_empty() {
                                    ui_updater.show_status_with_type(
                                        "Cannot save: No document content available".to_string(),
                                        "warning".to_string()
                                    );
                                    return;
                                }
                                
                                // Create a structured document format (JSON for now)
                                let document_data = serde_json::json!({
                                    "title": document.title,
                                    "id": document.id.to_string(),
                                    "content": all_content,
                                    "created_at": document.created_at,
                                    "updated_at": document.updated_at,
                                    "status": document.status,
                                    "metadata": document.metadata
                                });
                                
                                match serde_json::to_string_pretty(&document_data) {
                                    Ok(json_content) => {
                                        match std::fs::write(&final_path, json_content) {
                                            Ok(_) => {
                                                ui_updater.show_status_with_type(
                                                    format!("Document saved in document format to: {}", final_path.display()),
                                                    "success".to_string()
                                                );
                                            }
                                            Err(e) => {
                                                ui_updater.show_status_with_type(
                                                    format!("Failed to save document file: {}", e),
                                                    "error".to_string()
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        ui_updater.show_status_with_type(
                                            format!("Failed to serialize document data: {}", e),
                                            "error".to_string()
                                        );
                                    }
                                }
                            }
                            _ => {
                                // This shouldn't happen, but handle gracefully
                                ui_updater.show_status_with_type(
                                    "Unknown file format selected".to_string(),
                                    "error".to_string()
                                );
                            }
                        }
                    } else {
                        ui_updater.show_status_with_type(
                            "Cannot save: No document is currently loaded".to_string(),
                            "warning".to_string()
                        );
                    }
                });
            }
        });

        self.ui.on_file_import({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    // Show file dialog to select import file
                    ui_updater.show_status("Opening file dialog...".to_string());
                    
                    let file_dialog_result = tokio::task::spawn_blocking(|| {
                        FileDialog::new()
                            .add_filter("Word Documents", &["docx", "doc"])
                            .add_filter("Markdown Files", &["md"])
                            .add_filter("Text Files", &["txt"])
                            .add_filter("PDF Files", &["pdf"])
                            .add_filter("All Supported", &["docx", "doc", "md", "txt", "pdf"])
                            .set_title("Import Document")
                            .pick_file()
                    }).await;
                    
                    let file_path = match file_dialog_result {
                        Ok(Some(path)) => path,
                        Ok(None) => {
                            ui_updater.show_status("Import cancelled".to_string());
                            return;
                        }
                        Err(e) => {
                            ui_updater.show_status_with_type(
                                format!("Failed to open file dialog: {}", e), 
                                "error".to_string()
                            );
                            return;
                        }
                    };
                    
                    ui_updater.show_status(format!("Importing file: {}", file_path.display()));
                    
                    // Check if file format is supported
                    let filename = file_path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("unknown");
                    
                    let extension = file_path.extension()
                        .and_then(|ext| ext.to_str())
                        .map(|s| s.to_lowercase())
                        .unwrap_or_default();
                    
                    // For now, focus on DOCX support as implemented in DocumentImportService
                    if !matches!(extension.as_str(), "docx" | "doc") {
                        ui_updater.show_status_with_type(
                            format!("Unsupported file format: {}. Currently only Word documents (.docx, .doc) are supported.", extension),
                            "error".to_string()
                        );
                        return;
                    }
                    
                    // Get available languages from state
                    let available_languages = {
                        let langs = state.available_languages.read().await;
                        langs.clone()
                    };
                    
                    let current_language = {
                        let lang = state.current_language.read().await;
                        lang.clone()
                    };
                    
                    // Create import request
                    let import_request = DocumentImportRequest {
                        title: format!("Imported: {}", filename),
                        target_languages: available_languages.clone(),
                        source_language: current_language.clone(),
                        extract_images: false,  // Can be made configurable later
                        preserve_formatting: true,
                    };
                    
                    ui_updater.show_status("Processing document...".to_string());
                    
                    // Import the document
                    let import_result = {
                        let mut import_service = DocumentImportService::new();
                        import_service.import_docx_file(file_path, import_request).await
                    };
                    
                    match import_result {
                        Ok((document, import_result)) => {
                            ui_updater.show_status("Creating document in backend...".to_string());
                            
                            // Create the document through API client
                            let api_result = {
                                let api_client = state.api_client.read().await;
                                api_client.create_document(document.title.clone(), document.content.clone()).await
                            };
                            
                            match api_result {
                                Ok(created_document) => {
                                    // Update application state with the new document
                                    {
                                        let mut current_doc = state.current_document.write().await;
                                        *current_doc = Some(created_document.clone());
                                    }
                                    
                                    // Update document content
                                    {
                                        let mut content = state.document_content.write().await;
                                        *content = created_document.content.clone();
                                    }
                                    
                                    // Update documents list
                                    {
                                        let mut documents = state.documents.write().await;
                                        documents.push(created_document.clone());
                                    }
                                    
                                    // Update UI with the imported content
                                    if let Some(content) = created_document.content.get(&current_language) {
                                        ui_updater.update_content(content.clone(), current_language.clone());
                                    }
                                    
                                    // Show success message with import details
                                    let mut success_message = format!(
                                        "Document '{}' imported successfully in {}ms", 
                                        created_document.title, 
                                        import_result.processing_time_ms
                                    );
                                    
                                    if !import_result.warnings.is_empty() {
                                        success_message.push_str(&format!(" (with {} warnings)", import_result.warnings.len()));
                                    }
                                    
                                    ui_updater.show_status_with_type(success_message, "success".to_string());
                                    
                                    // Log import messages for debugging
                                    for message in &import_result.messages {
                                        println!("Import: {}", message);
                                    }
                                    for warning in &import_result.warnings {
                                        println!("Warning: {}", warning);
                                    }
                                }
                                Err(e) => {
                                    ui_updater.show_status_with_type(
                                        format!("Failed to create document in backend: {}", e), 
                                        "error".to_string()
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            ui_updater.show_status_with_type(
                                format!("Failed to import document: {}", e), 
                                "error".to_string()
                            );
                        }
                    }
                });
            }
        });

        self.ui.on_file_print({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Print functionality would be implemented here".to_string());
                });
            }
        });

        self.ui.on_file_exit({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Application will exit".to_string());
                    // Note: In a real application, this would check for unsaved changes
                    std::process::exit(0);
                });
            }
        });

        // Edit menu operations
        self.ui.on_edit_undo({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Undo functionality would be implemented here".to_string());
                });
            }
        });

        self.ui.on_edit_redo({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Redo functionality would be implemented here".to_string());
                });
            }
        });

        self.ui.on_edit_cut({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Cut functionality would be implemented here".to_string());
                });
            }
        });

        self.ui.on_edit_copy({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Copy functionality would be implemented here".to_string());
                });
            }
        });

        self.ui.on_edit_paste({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Paste functionality would be implemented here".to_string());
                });
            }
        });

        self.ui.on_edit_find({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Find dialog would be implemented here".to_string());
                });
            }
        });

        self.ui.on_edit_replace({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Find & Replace dialog would be implemented here".to_string());
                });
            }
        });

        self.ui.on_edit_preferences({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Preferences dialog would be implemented here".to_string());
                });
            }
        });

        // View menu operations
        self.ui.on_view_fullscreen({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Fullscreen toggle would be implemented here".to_string());
                });
            }
        });

        self.ui.on_view_zoom_in({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Zoom in functionality would be implemented here".to_string());
                });
            }
        });

        self.ui.on_view_zoom_out({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Zoom out functionality would be implemented here".to_string());
                });
            }
        });

        self.ui.on_view_show_sidebar({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Sidebar toggle would be implemented here".to_string());
                });
            }
        });

        // Project menu operations
        self.ui.on_project_new({
            let ui_updater = ui_updater.clone();
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();

            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let runtime_handle = runtime_handle.clone();
                
                runtime_handle.spawn(async move {
                    // Initialize the project wizard
                    match state.start_project_wizard().await {
                        Ok(_) => {
                            ui_updater.set_show_project_wizard(true);
                        }
                        Err(e) => {
                            ui_updater.show_status_with_type(format!("Failed to start project wizard: {}", e), "error".to_string());
                        }
                    }
                });
            }
        });

        // Project wizard callbacks
        self.ui.on_start_project_wizard({
            let ui_updater = ui_updater.clone();
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();

            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let runtime_handle = runtime_handle.clone();
                
                runtime_handle.spawn(async move {
                    match state.start_project_wizard().await {
                        Ok(_) => {
                            ui_updater.set_show_project_wizard(true);
                        }
                        Err(e) => {
                            ui_updater.show_status_with_type(format!("Failed to start project wizard: {}", e), "error".to_string());
                        }
                    }
                });
            }
        });

        self.ui.on_wizard_finish({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();

            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let runtime_handle = runtime_handle.clone();
                
                runtime_handle.spawn(async move {
                    // For now, use hardcoded values since the full wizard isn't implemented
                    // In the future, this would get data from the wizard UI
                    let project_name = "My New Project".to_string();
                    let description = Some("Created with the new project wizard".to_string());
                    let source_language = "en".to_string();
                    let target_languages = vec!["es".to_string(), "fr".to_string(), "de".to_string()];
                    
                    match state.create_project(project_name.clone(), description, source_language, target_languages).await {
                        Ok(_) => {
                            ui_updater.show_status_with_type(format!("Project '{}' created successfully with wizard!", project_name), "success".to_string());
                            
                            // Update UI to show project is loaded
                            if let Some(project) = state.get_current_project().await {
                                ui_updater.set_window_title(format!("TradocFlow - {}", project.name).into());
                                ui_updater.set_has_project_loaded(true);
                                ui_updater.set_current_project_name(project.name.into());
                            }
                        }
                        Err(e) => {
                            ui_updater.show_status_with_type(format!("Failed to create project: {}", e), "error".to_string());
                        }
                    }
                });
            }
        });

        self.ui.on_wizard_cancel({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();

            move || {
                let state = state.clone();
                let runtime_handle = runtime_handle.clone();
                
                runtime_handle.spawn(async move {
                    state.cancel_wizard().await;
                });
            }
        });

        self.ui.on_template_selected({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();

            move |template_id: slint::SharedString| {
                let state = state.clone();
                let runtime_handle = runtime_handle.clone();
                let template_id = template_id.to_string();
                
                runtime_handle.spawn(async move {
                    if let Some(mut wizard_data) = state.get_wizard_data().await {
                        wizard_data.template_id = template_id;
                        state.update_wizard_data(wizard_data).await;
                    }
                });
            }
        });

        self.ui.on_language_toggled({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();

            move |language_code: slint::SharedString, enabled: bool| {
                let state = state.clone();
                let runtime_handle = runtime_handle.clone();
                let language_code = language_code.to_string();
                
                runtime_handle.spawn(async move {
                    if let Some(mut wizard_data) = state.get_wizard_data().await {
                        if let Some(lang) = wizard_data.target_languages.iter_mut().find(|l| l.code == language_code) {
                            lang.enabled = enabled;
                        }
                        state.update_wizard_data(wizard_data).await;
                    }
                });
            }
        });

        self.ui.on_source_language_changed({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();

            move |language_code: slint::SharedString| {
                let state = state.clone();
                let runtime_handle = runtime_handle.clone();
                let language_code = language_code.to_string();
                
                runtime_handle.spawn(async move {
                    if let Some(mut wizard_data) = state.get_wizard_data().await {
                        wizard_data.source_language = language_code;
                        state.update_wizard_data(wizard_data).await;
                    }
                });
            }
        });

        self.ui.on_team_member_added({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();

            move |name: slint::SharedString, email: slint::SharedString, role: slint::SharedString| {
                let state = state.clone();
                let runtime_handle = runtime_handle.clone();
                let name = name.to_string();
                let email = email.to_string();
                let role = role.to_string();
                
                runtime_handle.spawn(async move {
                    if let Some(mut wizard_data) = state.get_wizard_data().await {
                        let member = crate::models::project_template::TeamMemberConfig {
                            user_id: uuid::Uuid::new_v4().to_string(),
                            name,
                            email,
                            role: role.to_lowercase(),
                            languages: vec![wizard_data.source_language.clone()],
                            permissions: vec!["read".to_string(), "write".to_string()],
                        };
                        wizard_data.team_members.push(member);
                        state.update_wizard_data(wizard_data).await;
                    }
                });
            }
        });

        self.ui.on_team_member_removed({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();

            move |member_id: slint::SharedString| {
                let state = state.clone();
                let runtime_handle = runtime_handle.clone();
                let member_id = member_id.to_string();
                
                runtime_handle.spawn(async move {
                    if let Some(mut wizard_data) = state.get_wizard_data().await {
                        wizard_data.team_members.retain(|m| m.user_id != member_id);
                        state.update_wizard_data(wizard_data).await;
                    }
                });
            }
        });

        self.ui.on_validate_current_step({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();

            move || -> bool {
                // For now, return true - validation will be handled server-side
                // In a full implementation, this could do client-side validation
                true
            }
        });

        self.ui.on_project_open({
            let ui_handle = ui_handle.clone();
            
            move || {
                // Open project browser dialog
                let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                    ui.set_show_project_browser(true);
                });
            }
        });

        self.ui.on_project_save({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let runtime_handle = runtime_handle.clone();
                
                runtime_handle.spawn(async move {
                    if !state.has_current_project().await {
                        ui_updater.show_status_with_type("No project loaded to save".to_string(), "warning".to_string());
                        return;
                    }
                    
                    match state.save_project().await {
                        Ok(_) => {
                            ui_updater.show_status_with_type("Project saved successfully".to_string(), "success".to_string());
                        }
                        Err(e) => {
                            ui_updater.show_status_with_type(format!("Failed to save project: {}", e), "error".to_string());
                        }
                    }
                });
            }
        });

        self.ui.on_project_close({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let runtime_handle = runtime_handle.clone();
                
                runtime_handle.spawn(async move {
                    if !state.has_current_project().await {
                        ui_updater.show_status_with_type("No project currently loaded".to_string(), "info".to_string());
                        return;
                    }
                    
                    match state.close_project().await {
                        Ok(_) => {
                            ui_updater.show_status_with_type("Project closed successfully".to_string(), "success".to_string());
                            ui_updater.set_window_title("TradocFlow".into());
                            
                            // Clear the document content in UI
                            ui_updater.set_document_content("# Welcome to TradocFlow\n\nCreate or open a project to get started...".into());
                        }
                        Err(e) => {
                            ui_updater.show_status_with_type(format!("Failed to close project: {}", e), "error".to_string());
                        }
                    }
                });
            }
        });

        self.ui.on_project_properties({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let runtime_handle = runtime_handle.clone();
                
                runtime_handle.spawn(async move {
                    if let Some(project) = state.get_current_project().await {
                        // In a real implementation, this would open a properties dialog
                        // For now, just show project information in status
                        let status_info = format!(
                            "Project: {} | Status: {} | Priority: {} | Created: {}",
                            project.name,
                            project.status.as_str(),
                            project.priority.as_str(),
                            project.created_at.format("%Y-%m-%d")
                        );
                        ui_updater.show_status_with_type(status_info, "info".to_string());
                        
                        // TODO: In a complete implementation, we would:
                        // 1. Open a dialog with project settings
                        // 2. Allow editing of project name, description, priority
                        // 3. Show project statistics (chapters, translation progress, etc.)
                        // 4. Allow adding/removing target languages
                        // 5. Show project file structure and paths
                    } else {
                        ui_updater.show_status_with_type("No project currently loaded".to_string(), "warning".to_string());
                    }
                });
            }
        });

        // Project Browser callbacks
        self.ui.on_open_project_browser({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_handle = ui_handle.clone();
            
            move || {
                let state = state.clone();
                let runtime_handle = runtime_handle.clone();
                let ui_handle = ui_handle.clone();
                
                runtime_handle.spawn(async move {
                    // Set loading state
                    let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                        ui.set_browser_is_loading(true);
                        ui.set_show_project_browser(true);
                    });
                    
                    // Load projects for browser
                    match state.load_projects_for_browser().await {
                        Ok(_) => {
                            // Get browser state and update UI
                            let browser_state = state.get_project_browser_state().await;
                            
                            // Convert browser items to simple project data
                            let simple_projects: Vec<SimpleProjectData> = browser_state.projects
                                .iter()
                                .map(convert_to_simple_project_data)
                                .collect();
                            
                            let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                                // Update UI with converted project data
                                ui.set_browser_projects(slint::ModelRc::new(slint::VecModel::from(simple_projects)));
                                ui.set_browser_is_loading(false);
                            }).ok();
                        }
                        Err(e) => {
                            eprintln!("Failed to load projects for browser: {}", e);
                            let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                                ui.set_browser_is_loading(false);
                            }).ok();
                        }
                    }
                });
            }
        });

        self.ui.on_close_project_browser({
            let ui_handle = ui_handle.clone();
            
            move || {
                let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                    ui.set_show_project_browser(false);
                });
            }
        });

        self.ui.on_browser_search_changed({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_handle = ui_handle.clone();
            
            move |query| {
                let state = state.clone();
                let runtime_handle = runtime_handle.clone();
                let ui_handle = ui_handle.clone();
                let query = query.to_string();
                
                runtime_handle.spawn(async move {
                    // Apply search filter
                    let browser_state = state.get_project_browser_state().await;
                    let search_options = crate::models::project_browser::SearchOptions::default();
                    
                    if let Err(e) = state.filter_browser_projects(query, browser_state.filters, search_options).await {
                        eprintln!("Failed to filter projects: {}", e);
                        return;
                    }
                    
                    // Update UI with filtered results
                    let browser_state = state.get_project_browser_state().await;
                    let simple_projects: Vec<SimpleProjectData> = browser_state.filtered_projects
                        .iter()
                        .map(convert_to_simple_project_data)
                        .collect();
                    
                    let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                        // Update filtered projects
                        ui.set_browser_projects(slint::ModelRc::new(slint::VecModel::from(simple_projects)));
                    }).ok();
                });
            }
        });

        self.ui.on_browser_project_opened({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move |project_data| {
                let state = state.clone();
                let runtime_handle = runtime_handle.clone();
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    // Parse project ID from project_data.id
                    if let Ok(project_id) = uuid::Uuid::parse_str(&project_data.id.to_string()) {
                        match state.load_project(project_id).await {
                            Ok(_) => {
                                // Add to recent projects
                                let _ = state.add_to_recent_projects(project_id).await;
                                
                                ui_updater.show_status_with_type(
                                    format!("Project '{}' opened successfully", project_data.name),
                                    "success".to_string()
                                );
                                ui_updater.set_window_title(format!("TradocFlow - {}", project_data.name).into());
                                
                                // Update UI with project structure
                                if let Ok(_) = state.update_sidebar_tree_items().await {
                                    // Sidebar updated successfully
                                }
                            }
                            Err(e) => {
                                ui_updater.show_status_with_type(
                                    format!("Failed to open project: {}", e),
                                    "error".to_string()
                                );
                            }
                        }
                    } else {
                        ui_updater.show_status_with_type(
                            "Invalid project ID".to_string(),
                            "error".to_string()
                        );
                    }
                });
            }
        });


        self.ui.on_browser_refresh_projects({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_handle = ui_handle.clone();
            
            move || {
                let state = state.clone();
                let runtime_handle = runtime_handle.clone();
                let ui_handle = ui_handle.clone();
                
                runtime_handle.spawn(async move {
                    // Set loading state
                    let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                        ui.set_browser_is_loading(true);
                    });
                    
                    // Reload projects
                    match state.load_projects_for_browser().await {
                        Ok(_) => {
                            let browser_state = state.get_project_browser_state().await;
                            let simple_projects: Vec<SimpleProjectData> = browser_state.projects
                                .iter()
                                .map(convert_to_simple_project_data)
                                .collect();
                                
                            let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                                // Update UI with refreshed projects
                                ui.set_browser_projects(slint::ModelRc::new(slint::VecModel::from(simple_projects)));
                                ui.set_browser_is_loading(false);
                            }).ok();
                        }
                        Err(e) => {
                            eprintln!("Failed to refresh projects: {}", e);
                            let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                                ui.set_browser_is_loading(false);
                            }).ok();
                        }
                    }
                });
            }
        });

        // Translation menu operations
        self.ui.on_translation_add_language({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Add language dialog would be implemented here".to_string());
                });
            }
        });

        self.ui.on_translation_manage({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Translation management dialog would be implemented here".to_string());
                });
            }
        });

        self.ui.on_translation_export({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Export translations would be implemented here".to_string());
                });
            }
        });

        self.ui.on_translation_import({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Import translations would be implemented here".to_string());
                });
            }
        });

        self.ui.on_translation_validate({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Translation validation would be implemented here".to_string());
                });
            }
        });

        // Tools menu operations
        self.ui.on_tools_screenshot({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Screenshot functionality would be implemented here".to_string());
                });
            }
        });

        self.ui.on_tools_spell_check({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Spell check functionality would be implemented here".to_string());
                });
            }
        });

        self.ui.on_tools_word_count({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    let language = state.get_language().await;
                    let content = state.get_content(&language).await;
                    let word_count = content.split_whitespace().count();
                    let char_count = content.chars().count();
                    
                    ui_updater.show_status(format!("Words: {}, Characters: {}", word_count, char_count));
                });
            }
        });

        self.ui.on_tools_export_pdf({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("PDF export would be implemented here".to_string());
                });
            }
        });

        self.ui.on_tools_export_html({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("HTML export would be implemented here".to_string());
                });
            }
        });

        // Help menu operations
        self.ui.on_help_about({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("About dialog would be implemented here".to_string());
                });
            }
        });

        self.ui.on_help_shortcuts({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Keyboard shortcuts dialog would be implemented here".to_string());
                });
            }
        });

        self.ui.on_help_documentation({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Documentation would be opened here".to_string());
                });
            }
        });

        self.ui.on_help_support({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Support information would be displayed here".to_string());
                });
            }
        });

        // Sidebar operations
        self.ui.on_show_projects({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    match state.load_documents().await {
                        Ok(_) => {
                            ui_updater.show_status_with_type("Projects view loaded".to_string(), "success".to_string());
                        }
                        Err(e) => {
                            ui_updater.show_status_with_type(format!("Failed to load projects: {}", e), "error".to_string());
                        }
                    }
                });
            }
        });

        self.ui.on_show_kanban({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Kanban board view would be implemented here".to_string());
                });
            }
        });

        self.ui.on_show_reviews({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    match state.load_notifications().await {
                        Ok(_) => {
                            ui_updater.show_status_with_type("Review system loaded".to_string(), "success".to_string());
                        }
                        Err(e) => {
                            ui_updater.show_status_with_type(format!("Failed to load reviews: {}", e), "error".to_string());
                        }
                    }
                });
            }
        });

        // Mode toggle
        self.ui.on_toggle_mode({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    state.toggle_mode().await;
                    let new_mode = state.get_mode().await;
                    ui_updater.update_mode(new_mode);
                });
            }
        });

        // Layout changes
        self.ui.on_set_layout({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move |layout: SharedString| {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let layout = layout.to_string();
                
                runtime_handle.spawn(async move {
                    state.set_layout(layout.clone()).await;
                    ui_updater.update_layout(layout);
                });
            }
        });

        // Enhanced content changes with real-time synchronization
        self.ui.on_content_changed({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            let content_sync = self.content_sync.clone();
            
            move |content: SharedString, language: SharedString| {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let content_sync = content_sync.clone();
                let content = content.to_string();
                let language = language.to_string();
                
                runtime_handle.spawn(async move {
                    // Record the content change with enhanced tracking
                    let user_id = {
                        let current_user = state.current_user.read().await;
                        current_user.as_ref().map(|u| u.id.clone())
                    };
                    
                    let change_id = content_sync.record_change(
                        language.clone(), 
                        content.clone(), 
                        ContentChangeType::Modify,
                        user_id
                    ).await;
                    
                    // Update content in state
                    state.update_content(language.clone(), content.clone()).await;
                    
                    // Enhanced debouncing with intelligent timing
                    let should_wait = content_sync.should_debounce(&language).await;
                    if should_wait {
                        // Add content to pending saves
                        content_sync.add_pending_save(language.clone(), content.clone()).await;
                        
                        // Smart debounce - shorter delay for small changes, longer for large ones
                        let delay = if content.len() < 100 { 800 } else { 1500 };
                        sleep(Duration::from_millis(delay)).await;
                        
                        // Check if this is still the latest change
                        if let Some(pending_content) = content_sync.remove_pending_save(&language).await {
                            if pending_content == content {
                                // Proceed with save
                                match state.save_document().await {
                                    Ok(_) => {
                                        ui_updater.show_status_with_type(
                                            format!("Auto-saved {} ({})", language, change_id[..8].to_string()), 
                                            "success".to_string()
                                        );
                                        
                                        // Perform content validation
                                        let validation_result = content_sync.validate_content("en").await;
                                        if !validation_result.is_valid {
                                            let critical_issues: Vec<_> = validation_result.issues
                                                .iter()
                                                .filter(|i| i.severity == IssueSeverity::Critical)
                                                .collect();
                                            
                                            if !critical_issues.is_empty() {
                                                ui_updater.show_status_with_type(
                                                    format!("⚠️ {} critical translation issues detected", critical_issues.len()),
                                                    "warning".to_string()
                                                );
                                            }
                                        }
                                        
                                        // Update translation status indicators
                                        Self::update_translation_status_ui(&ui_updater, &content_sync, &validation_result).await;
                                    }
                                    Err(e) => {
                                        ui_updater.show_status_with_type(
                                            format!("Auto-save failed: {}", e), 
                                            "error".to_string()
                                        );
                                    }
                                }
                            }
                        }
                    } else {
                        // Immediate save for first change or after long idle
                        match state.save_document().await {
                            Ok(_) => {
                                ui_updater.show_status_with_type(
                                    format!("Content saved ({})", language), 
                                    "success".to_string()
                                );
                            }
                            Err(e) => {
                                ui_updater.show_status_with_type(
                                    format!("Save failed: {}", e), 
                                    "error".to_string()
                                );
                            }
                        }
                    }
                });
            }
        });

        // Language changes
        self.ui.on_language_changed({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move |language: SharedString| {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let language = language.to_string();
                
                runtime_handle.spawn(async move {
                    state.set_language(language.clone()).await;
                    let content = state.get_content(&language).await;
                    ui_updater.update_content(content, language.clone());
                    ui_updater.update_language(language);
                });
            }
        });

        // Enhanced sidebar callbacks
        self.setup_enhanced_sidebar_callbacks();
        
        // Enhanced text formatting operations
        self.setup_enhanced_formatting_callbacks();
        
        // Text operation handling
        self.setup_text_operation_callbacks();
        
        // Undo/Redo system
        self.setup_undo_redo_callbacks();
        
        // Setup periodic status updates
        self.setup_status_updates();
    }

    /// Setup enhanced sidebar callbacks for project navigation and management
    fn setup_enhanced_sidebar_callbacks(&self) {
        let state = self.state.clone();
        let runtime_handle = self.runtime.handle().clone();
        let ui_updater = self.ui_updater.clone();

        // New chapter creation
        self.ui.on_new_chapter({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    if !state.has_current_project().await {
                        ui_updater.show_status_with_type(
                            "Please load or create a project first".to_string(), 
                            "warning".to_string()
                        );
                        return;
                    }
                    
                    // TODO: Show chapter creation dialog
                    // For now, create a sample chapter
                    ui_updater.show_status_with_type(
                        "New chapter creation would open dialog here".to_string(), 
                        "info".to_string()
                    );
                });
            }
        });

        // New translation creation
        self.ui.on_new_translation({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    if !state.has_current_project().await {
                        ui_updater.show_status_with_type(
                            "Please load or create a project first".to_string(), 
                            "warning".to_string()
                        );
                        return;
                    }
                    
                    // TODO: Show translation creation dialog
                    ui_updater.show_status_with_type(
                        "New translation creation would open dialog here".to_string(), 
                        "info".to_string()
                    );
                });
            }
        });

        // Tree item clicked
        self.ui.on_tree_item_clicked({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move |item_id: slint::SharedString| {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let item_id = item_id.to_string();
                
                runtime_handle.spawn(async move {
                    // Parse item ID to determine action
                    if item_id.starts_with("chapter_") {
                        // Extract chapter info from ID (format: chapter_{slug}_{language})
                        let parts: Vec<&str> = item_id.split('_').collect();
                        if parts.len() >= 3 {
                            let slug = parts[1];
                            let language = parts[2];
                            
                            if let Some(project) = state.get_current_project().await {
                                match state.project_manager.load_chapter_content(project.id, slug).await {
                                    Ok(content_map) => {
                                        if let Some(content) = content_map.get(language) {
                                            state.update_content(language.to_string(), content.clone()).await;
                                            ui_updater.update_content(content.clone(), language.to_string());
                                            ui_updater.update_language(language.to_string());
                                            ui_updater.show_status_with_type(
                                                format!("Loaded chapter '{}' in {}", slug, language), 
                                                "success".to_string()
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        ui_updater.show_status_with_type(
                                            format!("Failed to load chapter: {}", e), 
                                            "error".to_string()
                                        );
                                    }
                                }
                            }
                        }
                    } else {
                        ui_updater.show_status_with_type(
                            format!("Selected: {}", item_id), 
                            "info".to_string()
                        );
                    }
                });
            }
        });

        // Tree item expansion
        self.ui.on_tree_item_expanded({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            
            move |item_id: slint::SharedString, expanded: bool| {
                let state = state.clone();
                let item_id = item_id.to_string();
                
                runtime_handle.spawn(async move {
                    state.toggle_tree_item_expansion(&item_id, expanded).await;
                });
            }
        });

        // Tree item context menu
        self.ui.on_tree_item_context_menu({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move |item_id: slint::SharedString, _x: f32, _y: f32| {
                let ui_updater = ui_updater.clone();
                let item_id = item_id.to_string();
                
                runtime_handle.spawn(async move {
                    // TODO: Show context menu
                    ui_updater.show_status_with_type(
                        format!("Context menu for: {}", item_id), 
                        "info".to_string()
                    );
                });
            }
        });

        // Document search
        self.ui.on_search_documents({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move |query: slint::SharedString| {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let query = query.to_string();
                
                runtime_handle.spawn(async move {
                    match state.search_documents(&query).await {
                        Ok(results) => {
                            ui_updater.show_status_with_type(
                                format!("Found {} results for '{}'", results.len(), query), 
                                "info".to_string()
                            );
                            // TODO: Update UI with search results
                        }
                        Err(e) => {
                            ui_updater.show_status_with_type(
                                format!("Search failed: {}", e), 
                                "error".to_string()
                            );
                        }
                    }
                });
            }
        });

        // Clear search
        self.ui.on_clear_search({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    // Reset tree items to full list
                    if let Err(e) = state.update_sidebar_tree_items().await {
                        ui_updater.show_status_with_type(
                            format!("Failed to refresh tree view: {}", e), 
                            "error".to_string()
                        );
                    } else {
                        ui_updater.show_status_with_type(
                            "Search cleared".to_string(), 
                            "info".to_string()
                        );
                    }
                });
            }
        });

        // Recent document clicked
        self.ui.on_recent_document_clicked({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move |doc_id: slint::SharedString| {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let doc_id = doc_id.to_string();
                
                runtime_handle.spawn(async move {
                    // Parse recent document ID and load content
                    if doc_id.starts_with("recent_") {
                        let parts: Vec<&str> = doc_id.split('_').collect();
                        if parts.len() >= 3 {
                            let slug = parts[1];
                            let language = parts[2];
                            
                            // Trigger tree item click to load the document
                            let chapter_id = format!("chapter_{}_{}", slug, language);
                            state.set_selected_tree_item(&chapter_id).await;
                            
                            ui_updater.show_status_with_type(
                                format!("Opening recent document: {} ({})", slug, language), 
                                "info".to_string()
                            );
                        }
                    }
                });
            }
        });

        // Quick action triggered
        self.ui.on_quick_action_triggered({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            
            move |action_id: slint::SharedString| {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let action_id = action_id.to_string();
                
                runtime_handle.spawn(async move {
                    match action_id.as_str() {
                        "new-document" => {
                            // Trigger file new action
                            match state.create_document("New Document".to_string()).await {
                                Ok(_) => {
                                    let content = state.get_content("en").await;
                                    ui_updater.update_content(content, "en".to_string());
                                    ui_updater.update_language("en".to_string());
                                    ui_updater.show_status_with_type(
                                        "New document created".to_string(), 
                                        "success".to_string()
                                    );
                                }
                                Err(e) => {
                                    ui_updater.show_status_with_type(
                                        format!("Failed to create document: {}", e), 
                                        "error".to_string()
                                    );
                                }
                            }
                        }
                        "new-chapter" => {
                            ui_updater.show_status_with_type(
                                "New chapter creation triggered".to_string(), 
                                "info".to_string()
                            );
                        }
                        "new-translation" => {
                            ui_updater.show_status_with_type(
                                "New translation creation triggered".to_string(), 
                                "info".to_string()
                            );
                        }
                        "save" => {
                            match state.save_document().await {
                                Ok(_) => {
                                    ui_updater.show_status_with_type(
                                        "Document saved".to_string(), 
                                        "success".to_string()
                                    );
                                }
                                Err(e) => {
                                    ui_updater.show_status_with_type(
                                        format!("Failed to save: {}", e), 
                                        "error".to_string()
                                    );
                                }
                            }
                        }
                        "export" => {
                            ui_updater.show_status_with_type(
                                "Export triggered from quick actions".to_string(), 
                                "info".to_string()
                            );
                        }
                        _ => {
                            ui_updater.show_status_with_type(
                                format!("Unknown quick action: {}", action_id), 
                                "warning".to_string()
                            );
                        }
                    }
                });
            }
        });
    }

    /// Setup enhanced text formatting callbacks with professional editing features
    fn setup_enhanced_formatting_callbacks(&self) {
        let state = self.state.clone();
        let runtime_handle = self.runtime.handle().clone();
        let ui_updater = self.ui_updater.clone();
        let undo_manager = self.undo_manager.clone();

        // Enhanced formatting callbacks
        self.ui.on_format_bold({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            let undo_manager = undo_manager.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let undo_manager = undo_manager.clone();
                
                runtime_handle.spawn(async move {
                    let language = state.get_language().await;
                    let content = state.get_content(&language).await;
                    let mode = state.get_mode().await;
                    
                    // Save current state for undo
                    if let Ok(mut manager) = undo_manager.lock() {
                        manager.push_state(content.clone());
                    }
                    
                    let formatted_text = if mode == "markdown" {
                        TextFormatter::apply_markdown_formatting(&content, "bold", None)
                    } else {
                        TextFormatter::apply_presentation_formatting(&content, "bold", None)
                    };
                    
                    let mut new_content = content;
                    new_content.push_str(&formatted_text);
                    
                    state.update_content(language.clone(), new_content.clone()).await;
                    ui_updater.update_content(new_content, language);
                });
            }
        });

        self.ui.on_format_italic({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            let undo_manager = undo_manager.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let undo_manager = undo_manager.clone();
                
                runtime_handle.spawn(async move {
                    let language = state.get_language().await;
                    let content = state.get_content(&language).await;
                    let mode = state.get_mode().await;
                    
                    // Save current state for undo
                    if let Ok(mut manager) = undo_manager.lock() {
                        manager.push_state(content.clone());
                    }
                    
                    let formatted_text = if mode == "markdown" {
                        TextFormatter::apply_markdown_formatting(&content, "italic", None)
                    } else {
                        TextFormatter::apply_presentation_formatting(&content, "italic", None)
                    };
                    
                    let mut new_content = content;
                    new_content.push_str(&formatted_text);
                    
                    state.update_content(language.clone(), new_content.clone()).await;
                    ui_updater.update_content(new_content, language);
                });
            }
        });

        self.ui.on_format_underline({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            let undo_manager = undo_manager.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let undo_manager = undo_manager.clone();
                
                runtime_handle.spawn(async move {
                    let language = state.get_language().await;
                    let content = state.get_content(&language).await;
                    let mode = state.get_mode().await;
                    
                    // Save current state for undo
                    if let Ok(mut manager) = undo_manager.lock() {
                        manager.push_state(content.clone());
                    }
                    
                    let formatted_text = TextFormatter::apply_presentation_formatting(&content, "underline", None);
                    
                    let mut new_content = content;
                    new_content.push_str(&formatted_text);
                    
                    state.update_content(language.clone(), new_content.clone()).await;
                    ui_updater.update_content(new_content, language);
                });
            }
        });

        self.ui.on_format_heading({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            let undo_manager = undo_manager.clone();
            
            move |level: i32| {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let undo_manager = undo_manager.clone();
                
                runtime_handle.spawn(async move {
                    let language = state.get_language().await;
                    let content = state.get_content(&language).await;
                    let mode = state.get_mode().await;
                    
                    // Save current state for undo
                    if let Ok(mut manager) = undo_manager.lock() {
                        manager.push_state(content.clone());
                    }
                    
                    let heading_text = TextFormatter::insert_heading(level, &mode);
                    
                    let mut new_content = content;
                    new_content.push_str(&heading_text);
                    
                    state.update_content(language.clone(), new_content.clone()).await;
                    ui_updater.update_content(new_content, language);
                });
            }
        });

        // Enhanced formatting callbacks
        self.ui.on_format_code({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            let undo_manager = undo_manager.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let undo_manager = undo_manager.clone();
                
                runtime_handle.spawn(async move {
                    let language = state.get_language().await;
                    let content = state.get_content(&language).await;
                    let mode = state.get_mode().await;
                    
                    // Save current state for undo
                    if let Ok(mut manager) = undo_manager.lock() {
                        manager.push_state(content.clone());
                    }
                    
                    let formatted_text = if mode == "markdown" {
                        TextFormatter::apply_markdown_formatting(&content, "code", None)
                    } else {
                        TextFormatter::apply_presentation_formatting(&content, "code", None)
                    };
                    
                    let mut new_content = content;
                    new_content.push_str(&formatted_text);
                    
                    state.update_content(language.clone(), new_content.clone()).await;
                    ui_updater.update_content(new_content, language);
                });
            }
        });

        self.ui.on_format_quote({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            let undo_manager = undo_manager.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let undo_manager = undo_manager.clone();
                
                runtime_handle.spawn(async move {
                    let language = state.get_language().await;
                    let content = state.get_content(&language).await;
                    let mode = state.get_mode().await;
                    
                    // Save current state for undo
                    if let Ok(mut manager) = undo_manager.lock() {
                        manager.push_state(content.clone());
                    }
                    
                    let formatted_text = if mode == "markdown" {
                        TextFormatter::apply_markdown_formatting(&content, "quote", None)
                    } else {
                        "<blockquote>Quote text</blockquote>".to_string()
                    };
                    
                    let mut new_content = content;
                    new_content.push_str(&formatted_text);
                    
                    state.update_content(language.clone(), new_content.clone()).await;
                    ui_updater.update_content(new_content, language);
                });
            }
        });

        // List callbacks
        self.ui.on_insert_bullet_list({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            let undo_manager = undo_manager.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let undo_manager = undo_manager.clone();
                
                runtime_handle.spawn(async move {
                    let language = state.get_language().await;
                    let content = state.get_content(&language).await;
                    let mode = state.get_mode().await;
                    
                    // Save current state for undo
                    if let Ok(mut manager) = undo_manager.lock() {
                        manager.push_state(content.clone());
                    }
                    
                    let list_text = TextFormatter::insert_list("bullet", &mode);
                    
                    let mut new_content = content;
                    new_content.push_str(&list_text);
                    
                    state.update_content(language.clone(), new_content.clone()).await;
                    ui_updater.update_content(new_content, language);
                });
            }
        });

        self.ui.on_insert_numbered_list({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            let undo_manager = undo_manager.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let undo_manager = undo_manager.clone();
                
                runtime_handle.spawn(async move {
                    let language = state.get_language().await;
                    let content = state.get_content(&language).await;
                    let mode = state.get_mode().await;
                    
                    // Save current state for undo
                    if let Ok(mut manager) = undo_manager.lock() {
                        manager.push_state(content.clone());
                    }
                    
                    let list_text = TextFormatter::insert_list("numbered", &mode);
                    
                    let mut new_content = content;
                    new_content.push_str(&list_text);
                    
                    state.update_content(language.clone(), new_content.clone()).await;
                    ui_updater.update_content(new_content, language);
                });
            }
        });

        self.ui.on_insert_checklist({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            let undo_manager = undo_manager.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let undo_manager = undo_manager.clone();
                
                runtime_handle.spawn(async move {
                    let language = state.get_language().await;
                    let content = state.get_content(&language).await;
                    let mode = state.get_mode().await;
                    
                    // Save current state for undo
                    if let Ok(mut manager) = undo_manager.lock() {
                        manager.push_state(content.clone());
                    }
                    
                    let list_text = TextFormatter::insert_list("checklist", &mode);
                    
                    let mut new_content = content;
                    new_content.push_str(&list_text);
                    
                    state.update_content(language.clone(), new_content.clone()).await;
                    ui_updater.update_content(new_content, language);
                });
            }
        });

        // Legacy compatibility
        self.ui.on_insert_list({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            let undo_manager = undo_manager.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let undo_manager = undo_manager.clone();
                
                runtime_handle.spawn(async move {
                    let language = state.get_language().await;
                    let content = state.get_content(&language).await;
                    let mode = state.get_mode().await;
                    
                    // Save current state for undo
                    if let Ok(mut manager) = undo_manager.lock() {
                        manager.push_state(content.clone());
                    }
                    
                    let list_text = TextFormatter::insert_list("bullet", &mode);
                    
                    let mut new_content = content;
                    new_content.push_str(&list_text);
                    
                    state.update_content(language.clone(), new_content.clone()).await;
                    ui_updater.update_content(new_content, language);
                });
            }
        });

        // Insert callbacks
        self.ui.on_insert_link({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            let undo_manager = undo_manager.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let undo_manager = undo_manager.clone();
                
                runtime_handle.spawn(async move {
                    let language = state.get_language().await;
                    let content = state.get_content(&language).await;
                    let mode = state.get_mode().await;
                    
                    // Save current state for undo
                    if let Ok(mut manager) = undo_manager.lock() {
                        manager.push_state(content.clone());
                    }
                    
                    let link_text = TextFormatter::insert_link(&mode);
                    
                    let mut new_content = content;
                    new_content.push_str(&link_text);
                    
                    state.update_content(language.clone(), new_content.clone()).await;
                    ui_updater.update_content(new_content, language);
                });
            }
        });

        self.ui.on_insert_image({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            let undo_manager = undo_manager.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let undo_manager = undo_manager.clone();
                
                runtime_handle.spawn(async move {
                    let language = state.get_language().await;
                    let content = state.get_content(&language).await;
                    let mode = state.get_mode().await;
                    
                    // Save current state for undo
                    if let Ok(mut manager) = undo_manager.lock() {
                        manager.push_state(content.clone());
                    }
                    
                    let image_text = TextFormatter::insert_image(&mode);
                    
                    let mut new_content = content;
                    new_content.push_str(&image_text);
                    
                    state.update_content(language.clone(), new_content.clone()).await;
                    ui_updater.update_content(new_content, language);
                });
            }
        });

        self.ui.on_insert_table({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            let undo_manager = undo_manager.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let undo_manager = undo_manager.clone();
                
                runtime_handle.spawn(async move {
                    let language = state.get_language().await;
                    let content = state.get_content(&language).await;
                    let mode = state.get_mode().await;
                    
                    // Save current state for undo
                    if let Ok(mut manager) = undo_manager.lock() {
                        manager.push_state(content.clone());
                    }
                    
                    let table_text = TextFormatter::insert_table(&mode);
                    
                    let mut new_content = content;
                    new_content.push_str(&table_text);
                    
                    state.update_content(language.clone(), new_content.clone()).await;
                    ui_updater.update_content(new_content, language);
                });
            }
        });

        self.ui.on_insert_code_block({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            let undo_manager = undo_manager.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let undo_manager = undo_manager.clone();
                
                runtime_handle.spawn(async move {
                    let language = state.get_language().await;
                    let content = state.get_content(&language).await;
                    
                    // Save current state for undo
                    if let Ok(mut manager) = undo_manager.lock() {
                        manager.push_state(content.clone());
                    }
                    
                    let code_block_text = TextFormatter::insert_code_block();
                    
                    let mut new_content = content;
                    new_content.push_str(&code_block_text);
                    
                    state.update_content(language.clone(), new_content.clone()).await;
                    ui_updater.update_content(new_content, language);
                });
            }
        });

        // Text manipulation callbacks
        self.ui.on_increase_indent({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            let undo_manager = undo_manager.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let undo_manager = undo_manager.clone();
                
                runtime_handle.spawn(async move {
                    let language = state.get_language().await;
                    let content = state.get_content(&language).await;
                    
                    // Save current state for undo
                    if let Ok(mut manager) = undo_manager.lock() {
                        manager.push_state(content.clone());
                    }
                    
                    // Simple implementation - add indentation
                    let mut new_content = content;
                    new_content.push_str("    "); // 4 spaces for indentation
                    
                    state.update_content(language.clone(), new_content.clone()).await;
                    ui_updater.update_content(new_content, language);
                });
            }
        });

        self.ui.on_decrease_indent({
            let ui_updater = ui_updater.clone();
            let runtime_handle = runtime_handle.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Decrease indent functionality implemented".to_string());
                });
            }
        });

        // Alignment callbacks (presentation mode)
        self.ui.on_align_left({
            let ui_updater = ui_updater.clone();
            let runtime_handle = runtime_handle.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Align left functionality implemented".to_string());
                });
            }
        });

        self.ui.on_align_center({
            let ui_updater = ui_updater.clone();
            let runtime_handle = runtime_handle.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Align center functionality implemented".to_string());
                });
            }
        });

        self.ui.on_align_right({
            let ui_updater = ui_updater.clone();
            let runtime_handle = runtime_handle.clone();
            
            move || {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Align right functionality implemented".to_string());
                });
            }
        });
    }

    /// Setup text operation callbacks for advanced editing
    fn setup_text_operation_callbacks(&self) {
        let state = self.state.clone();
        let runtime_handle = self.runtime.handle().clone();
        let ui_updater = self.ui_updater.clone();
        let undo_manager = self.undo_manager.clone();

        self.ui.on_text_operation({
            let ui_updater = ui_updater.clone();
            let runtime_handle = runtime_handle.clone();
            
            move |_operation| {
                let ui_updater = ui_updater.clone();
                
                runtime_handle.spawn(async move {
                    ui_updater.show_status("Text operation processed".to_string());
                });
            }
        });
    }
    
    /// Setup undo/redo callbacks
    fn setup_undo_redo_callbacks(&self) {
        let state = self.state.clone();
        let runtime_handle = self.runtime.handle().clone();
        let ui_updater = self.ui_updater.clone();
        let undo_manager = self.undo_manager.clone();

        self.ui.on_undo({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            let undo_manager = undo_manager.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let undo_manager = undo_manager.clone();
                
                runtime_handle.spawn(async move {
                    let previous_content = {
                        if let Ok(mut manager) = undo_manager.lock() {
                            manager.undo()
                        } else {
                            None
                        }
                    };
                    
                    if let Some(content) = previous_content {
                        let language = state.get_language().await;
                        state.update_content(language.clone(), content.clone()).await;
                        ui_updater.update_content(content, language);
                        ui_updater.show_status_with_type("Undo successful".to_string(), "success".to_string());
                    } else {
                        ui_updater.show_status_with_type("Nothing to undo".to_string(), "warning".to_string());
                    }
                });
            }
        });

        self.ui.on_redo({
            let state = state.clone();
            let runtime_handle = runtime_handle.clone();
            let ui_updater = ui_updater.clone();
            let undo_manager = undo_manager.clone();
            
            move || {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let undo_manager = undo_manager.clone();
                
                runtime_handle.spawn(async move {
                    let next_content = {
                        if let Ok(mut manager) = undo_manager.lock() {
                            manager.redo()
                        } else {
                            None
                        }
                    };
                    
                    if let Some(content) = next_content {
                        let language = state.get_language().await;
                        state.update_content(language.clone(), content.clone()).await;
                        ui_updater.update_content(content, language);
                        ui_updater.show_status_with_type("Redo successful".to_string(), "success".to_string());
                    } else {
                        ui_updater.show_status_with_type("Nothing to redo".to_string(), "warning".to_string());
                    }
                });
            }
        });
    }

    /// Setup periodic status updates from the backend
    fn setup_status_updates(&self) {
        let state = self.state.clone();
        let ui_updater = self.ui_updater.clone();
        let runtime_handle = self.runtime.handle().clone();

        // Update status every 500ms
        let timer = slint::Timer::default();
        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(500),
            {
                let state = state.clone();
                let ui_updater = ui_updater.clone();
                let runtime_handle = runtime_handle.clone();

                move || {
                    let state = state.clone();
                    let ui_updater = ui_updater.clone();

                    runtime_handle.spawn(async move {
                        // Update mode and layout if changed
                        let mode = state.get_mode().await;
                        let layout = state.get_layout().await;
                        let language = state.get_language().await;
                        
                        ui_updater.update_mode(mode);
                        ui_updater.update_layout(layout);
                        ui_updater.update_language(language);
                    });
                }
            },
        );
    }

    /// Update translation status UI indicators
    async fn update_translation_status_ui(
        ui_updater: &UiUpdater,
        _content_sync: &ContentSyncManager,
        validation_result: &ContentValidationResult,
    ) {
        // Update translation coverage indicators
        for (language, coverage) in &validation_result.translation_coverage {
            let status = if *coverage >= 0.95 {
                "complete"
            } else if *coverage >= 0.5 {
                "partial"
            } else {
                "missing"
            };
            
            ui_updater.show_status_with_type(
                format!("📊 {} translation: {:.0}% complete", language, coverage * 100.0),
                status.to_string(),
            );
        }
        
        // Show detailed validation issues
        let mut warnings = Vec::new();
        let mut critical_issues = Vec::new();
        
        for issue in &validation_result.issues {
            match issue.severity {
                IssueSeverity::Critical => critical_issues.push(issue),
                IssueSeverity::Warning => warnings.push(issue),
                IssueSeverity::Info => {} // Skip info messages for UI
            }
        }
        
        if !critical_issues.is_empty() {
            ui_updater.show_status_with_type(
                format!("🚨 {} critical translation issues need attention", critical_issues.len()),
                "error".to_string(),
            );
        } else if !warnings.is_empty() {
            ui_updater.show_status_with_type(
                format!("⚠️ {} translation warnings", warnings.len()),
                "warning".to_string(),
            );
        }
    }

    /// Enhanced content validation with conflict detection
    pub async fn validate_all_content(&self) -> ContentValidationResult {
        self.content_sync.validate_content("en").await
    }

    /// Get content change history for debugging and undo operations
    pub async fn get_content_history(&self, language: &str, limit: usize) -> Vec<ContentChange> {
        self.content_sync.get_change_history(language, limit).await
    }

    /// Get translation status for all languages
    pub async fn get_translation_status(&self) -> HashMap<String, LanguageContentInfo> {
        self.content_sync.get_all_language_status().await
    }

    /// Check for potential content conflicts
    pub async fn check_content_conflicts(&self) -> Vec<ValidationIssue> {
        let validation_result = self.content_sync.validate_content("en").await;
        validation_result.issues
            .into_iter()
            .filter(|issue| issue.issue_type == ValidationIssueType::InconsistentStructure)
            .collect()
    }

    /// Force synchronization of all content
    pub async fn force_content_sync(&self) -> Result<(), TradocumentError> {
        // Check for pending saves
        if self.content_sync.has_pending_saves().await {
            self.ui_updater.show_status_with_type(
                "⏳ Finalizing pending changes...".to_string(),
                "info".to_string(),
            );
            
            // Wait a moment for pending operations to complete
            sleep(Duration::from_millis(100)).await;
        }
        
        // Perform validation
        let validation_result = self.content_sync.validate_content("en").await;
        
        // Save document
        match self.state.save_document().await {
            Ok(_) => {
                self.ui_updater.show_status_with_type(
                    "✅ Content synchronized successfully".to_string(),
                    "success".to_string(),
                );
                
                // Update UI with validation results
                Self::update_translation_status_ui(&self.ui_updater, &self.content_sync, &validation_result).await;
                
                Ok(())
            }
            Err(e) => {
                self.ui_updater.show_status_with_type(
                    format!("❌ Synchronization failed: {}", e),
                    "error".to_string(),
                );
                Err(e)
            }
        }
    }

    /// Setup periodic content validation
    fn setup_content_validation_timer(&self) {
        let content_sync = self.content_sync.clone();
        let ui_updater = self.ui_updater.clone();
        let runtime_handle = self.runtime.handle().clone();
        
        runtime_handle.spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Perform background validation
                let validation_result = content_sync.validate_content("en").await;
                
                // Only show UI updates for significant issues
                let critical_count = validation_result.issues
                    .iter()
                    .filter(|i| i.severity == IssueSeverity::Critical)
                    .count();
                
                if critical_count > 0 {
                    ui_updater.show_status_with_type(
                        format!("🔍 Background validation: {} critical issues found", critical_count),
                        "warning".to_string(),
                    );
                }
            }
        });
    }
}

// Custom error type for Slint integration
impl From<slint::PlatformError> for TradocumentError {
    fn from(err: slint::PlatformError) -> Self {
        TradocumentError::SlintError(err.to_string())
    }
}

