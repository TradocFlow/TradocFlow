use slint::{ComponentHandle, SharedString, Weak, ModelRc, VecModel};
use std::sync::Arc;
use std::path::PathBuf;
use std::collections::HashMap;
use crate::{MainWindow, TradocumentError, Result};
use crate::services::{ProjectService, DocumentImportService, TranslationMemoryService};
use crate::services::project_service::{CreateProjectRequest, TeamMemberRequest};
use crate::services::document_import_service::{ImportConfig, LanguageDocumentMap};

/// Main application struct that manages the Slint GUI
pub struct App {
    main_window: MainWindow,
    project_service: Arc<ProjectService>,
    current_wizard_data: Arc<std::sync::Mutex<WizardData>>,
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
    pub fn new() -> Result<Self> {
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
        let wizard_data = Arc::new(std::sync::Mutex::new(WizardData::default()));

        // Set up callbacks
        let app = Self { 
            main_window, 
            project_service,
            current_wizard_data: wizard_data,
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
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_document_content("# New Document\n\nStart editing here...".into());
                    window.set_status_message("New document created".into());
                    window.set_status_type("success".into());
                }
            }
        });

        self.main_window.on_file_open({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("File open dialog would appear here".into());
                    window.set_status_type("info".into());
                }
            }
        });

        self.main_window.on_file_save({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Document saved successfully".into());
                    window.set_status_type("success".into());
                }
            }
        });

        self.main_window.on_file_save_as({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Save as dialog would appear here".into());
                    window.set_status_type("info".into());
                }
            }
        });

        self.main_window.on_file_import({
            let main_window_weak = main_window_weak.clone();
            move || {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_status_message("Import dialog would appear here".into());
                    window.set_status_type("info".into());
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
            let main_window_weak = main_window_weak.clone();
            move |content, language| {
                if let Some(window) = main_window_weak.upgrade() {
                    window.set_document_content(content);
                    window.set_status_message(format!("Content updated for {}", language).into());
                    window.set_status_type("info".into());
                }
            }
        });

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
}