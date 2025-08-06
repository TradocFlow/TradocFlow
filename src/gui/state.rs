use std::collections::HashMap;
use uuid::Uuid;
use tokio::sync::RwLock;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use chrono::Utc;
use rand;

use crate::{User, TradocumentError};
use crate::models::document::Document;
use crate::models::project::{Project, CreateProjectRequest, UpdateProjectRequest, Priority};
use crate::models::project_browser::{ProjectBrowserState, ProjectBrowserItem, RecentProject, ProjectFilters, SortConfig, ViewMode, AccessLevel, SearchOptions};
use crate::models::project_template::{ProjectWizardData, TemplateManager, get_available_languages};
use crate::services::project_manager::ProjectManager;
use crate::database::project_repository::ProjectRepository;
use crate::models::document::{ProjectStructure, ChapterInfo};
use super::client::{ApiClient, NotificationResponse};

/// Application state for the GUI
#[derive(Clone)]
pub struct AppState {
    /// API client for backend communication
    pub api_client: Arc<RwLock<ApiClient>>,
    
    /// Current user
    pub current_user: Arc<RwLock<Option<User>>>,
    
    /// Currently opened document
    pub current_document: Arc<RwLock<Option<Document>>>,
    
    /// Document content for each language
    pub document_content: Arc<RwLock<HashMap<String, String>>>,
    
    /// Current editing mode: "markdown" or "presentation"
    pub current_mode: Arc<RwLock<String>>,
    
    /// Current layout: "single", "horizontal", "vertical"
    pub current_layout: Arc<RwLock<String>>,
    
    /// Current primary language being edited
    pub current_language: Arc<RwLock<String>>,
    
    /// Current secondary language (for split view)
    pub secondary_language: Arc<RwLock<String>>,
    
    /// List of available documents
    pub documents: Arc<RwLock<Vec<Document>>>,
    
    /// List of available projects
    pub projects: Arc<RwLock<Vec<Project>>>,
    
    /// Available languages for translation
    pub available_languages: Arc<RwLock<Vec<String>>>,
    
    /// Currently loaded project
    pub current_project: Arc<RwLock<Option<Project>>>,
    
    /// Project manager for file operations
    pub project_manager: Arc<ProjectManager>,
    
    /// Project repository for database operations
    pub project_repository: Arc<ProjectRepository>,
    
    /// Current project structure
    pub project_structure: Arc<RwLock<Option<ProjectStructure>>>,
    
    /// Sidebar state
    pub sidebar_state: Arc<RwLock<SidebarState>>,
    
    /// Notifications
    pub notifications: Arc<RwLock<Vec<NotificationResponse>>>,
    
    /// Unread notification count
    pub unread_count: Arc<RwLock<u32>>,
    
    /// Unsaved changes flag
    pub has_unsaved_changes: Arc<RwLock<bool>>,
    
    /// Application status message
    pub status_message: Arc<RwLock<String>>,
    
    /// Project creation wizard state
    pub wizard_data: Arc<RwLock<Option<ProjectWizardData>>>,
    
    /// Template manager
    pub template_manager: Arc<TemplateManager>,
    
    /// Project browser state
    pub project_browser_state: Arc<RwLock<ProjectBrowserState>>,
}

impl AppState {
    /// Create a new application state
    pub fn new(api_base_url: String, project_manager: ProjectManager, project_repository: ProjectRepository) -> Self {
        let api_client = ApiClient::new(api_base_url);
        
        Self {
            api_client: Arc::new(RwLock::new(api_client)),
            current_user: Arc::new(RwLock::new(None)),
            current_document: Arc::new(RwLock::new(None)),
            document_content: Arc::new(RwLock::new(HashMap::new())),
            current_mode: Arc::new(RwLock::new("markdown".to_string())),
            current_layout: Arc::new(RwLock::new("single".to_string())),
            current_language: Arc::new(RwLock::new("en".to_string())),
            secondary_language: Arc::new(RwLock::new("de".to_string())),
            documents: Arc::new(RwLock::new(Vec::new())),
            projects: Arc::new(RwLock::new(Vec::new())),
            available_languages: Arc::new(RwLock::new(vec![
                "en".to_string(),
                "de".to_string(),
                "fr".to_string(),
                "es".to_string(),
                "it".to_string(),
                "nl".to_string(),
            ])),
            notifications: Arc::new(RwLock::new(Vec::new())),
            unread_count: Arc::new(RwLock::new(0)),
            has_unsaved_changes: Arc::new(RwLock::new(false)),
            status_message: Arc::new(RwLock::new("Ready".to_string())),
            current_project: Arc::new(RwLock::new(None)),
            project_manager: Arc::new(project_manager),
            project_repository: Arc::new(project_repository),
            project_structure: Arc::new(RwLock::new(None)),
            sidebar_state: Arc::new(RwLock::new(SidebarState::default())),
            wizard_data: Arc::new(RwLock::new(None)),
            template_manager: Arc::new(TemplateManager::new()),
            project_browser_state: Arc::new(RwLock::new(ProjectBrowserState::default())),
        }
    }

    /// Set the current user and authenticate API client
    pub async fn set_current_user(&self, user: User) {
        {
            let mut api_client = self.api_client.write().await;
            api_client.set_user_id(user.id.clone());
        }
        
        {
            let mut current_user = self.current_user.write().await;
            *current_user = Some(user);
        }
    }

    /// Load documents from the API
    pub async fn load_documents(&self) -> Result<(), crate::TradocumentError> {
        let api_client = self.api_client.read().await;
        let documents = api_client.get_documents().await?;
        
        {
            let mut docs = self.documents.write().await;
            *docs = documents;
        }
        
        Ok(())
    }

    /// Load a specific document
    pub async fn load_document(&self, doc_id: Uuid) -> Result<(), crate::TradocumentError> {
        let api_client = self.api_client.read().await;
        let document = api_client.get_document(doc_id).await?;
        
        // Update document content
        {
            let mut content = self.document_content.write().await;
            *content = document.content.clone();
        }
        
        // Set current document
        {
            let mut current_doc = self.current_document.write().await;
            *current_doc = Some(document);
        }
        
        // Clear unsaved changes flag
        {
            let mut unsaved = self.has_unsaved_changes.write().await;
            *unsaved = false;
        }
        
        Ok(())
    }

    /// Create a new document
    pub async fn create_document(&self, title: String) -> Result<(), crate::TradocumentError> {
        let mut content = HashMap::new();
        content.insert("en".to_string(), format!("# {title}\n\nStart writing your document here..."));
        
        let api_client = self.api_client.read().await;
        let document = api_client.create_document(title, content.clone()).await?;
        
        // Set as current document
        {
            let mut current_doc = self.current_document.write().await;
            *current_doc = Some(document.clone());
        }
        
        // Update document content
        {
            let mut doc_content = self.document_content.write().await;
            *doc_content = content;
        }
        
        // Add to documents list
        {
            let mut docs = self.documents.write().await;
            docs.push(document);
        }
        
        // Clear unsaved changes flag
        {
            let mut unsaved = self.has_unsaved_changes.write().await;
            *unsaved = false;
        }
        
        Ok(())
    }

    /// Save current document content
    pub async fn save_document(&self) -> Result<(), crate::TradocumentError> {
        let current_doc = {
            let doc = self.current_document.read().await;
            doc.clone()
        };
        
        if let Some(document) = current_doc {
            let content = self.document_content.read().await;
            let api_client = self.api_client.read().await;
            
            // Save content for each language
            for (language, text) in content.iter() {
                api_client.save_document_content(document.id, text.clone(), language.clone()).await?;
            }
            
            // Clear unsaved changes flag
            {
                let mut unsaved = self.has_unsaved_changes.write().await;
                *unsaved = false;
            }
            
            // Update status message
            {
                let mut status = self.status_message.write().await;
                *status = "Document saved".to_string();
            }
            
            Ok(())
        } else {
            Err(crate::TradocumentError::DocumentNotFound("No document currently loaded".to_string()))
        }
    }

    /// Update document content for a specific language
    pub async fn update_content(&self, language: String, content: String) {
        {
            let mut doc_content = self.document_content.write().await;
            doc_content.insert(language, content);
        }
        
        // Mark as having unsaved changes
        {
            let mut unsaved = self.has_unsaved_changes.write().await;
            *unsaved = true;
        }
    }

    /// Get content for a specific language
    pub async fn get_content(&self, language: &str) -> String {
        let content = self.document_content.read().await;
        content.get(language).cloned().unwrap_or_default()
    }

    /// Toggle editing mode between markdown and presentation
    pub async fn toggle_mode(&self) {
        let mut mode = self.current_mode.write().await;
        *mode = if *mode == "markdown" {
            "presentation".to_string()
        } else {
            "markdown".to_string()
        };
    }

    /// Set layout mode
    pub async fn set_layout(&self, layout: String) {
        let mut current_layout = self.current_layout.write().await;
        *current_layout = layout;
    }

    /// Set current primary language
    pub async fn set_language(&self, language: String) {
        let mut current_lang = self.current_language.write().await;
        *current_lang = language.clone();
        
        // Also update API language preference
        let api_client = self.api_client.read().await;
        let _ = api_client.set_language(language).await;
    }

    /// Set secondary language for split view
    pub async fn set_secondary_language(&self, language: String) {
        let mut secondary_lang = self.secondary_language.write().await;
        *secondary_lang = language;
    }

    /// Load notifications
    pub async fn load_notifications(&self) -> Result<(), crate::TradocumentError> {
        let api_client = self.api_client.read().await;
        let notifications = api_client.get_notifications().await?;
        let unread_count = api_client.get_unread_count().await?;
        
        {
            let mut notifs = self.notifications.write().await;
            *notifs = notifications;
        }
        
        {
            let mut count = self.unread_count.write().await;
            *count = unread_count;
        }
        
        Ok(())
    }

    /// Update status message
    pub async fn set_status(&self, message: String) {
        let mut status = self.status_message.write().await;
        *status = message;
    }

    /// Check if there are unsaved changes
    pub async fn has_unsaved_changes(&self) -> bool {
        let unsaved = self.has_unsaved_changes.read().await;
        *unsaved
    }

    /// Get current mode
    pub async fn get_mode(&self) -> String {
        let mode = self.current_mode.read().await;
        mode.clone()
    }

    /// Get current layout
    pub async fn get_layout(&self) -> String {
        let layout = self.current_layout.read().await;
        layout.clone()
    }

    /// Get current language
    pub async fn get_language(&self) -> String {
        let language = self.current_language.read().await;
        language.clone()
    }

    /// Get secondary language
    pub async fn get_secondary_language(&self) -> String {
        let language = self.secondary_language.read().await;
        language.clone()
    }

    /// Get status message
    pub async fn get_status(&self) -> String {
        let status = self.status_message.read().await;
        status.clone()
    }
    
    // Project Management Methods
    
    /// Create a new project
    pub async fn create_project(&self, name: String, description: Option<String>, source_language: String, target_languages: Vec<String>) -> Result<(), TradocumentError> {
        let user = {
            let current_user = self.current_user.read().await;
            current_user.clone().ok_or_else(|| TradocumentError::AuthenticationError("No user logged in".to_string()))?
        };
        
        let request = CreateProjectRequest {
            name: name.clone(),
            description: description.clone(),
            due_date: None,
            priority: Priority::Medium,
        };
        
        // Create project in database
        let project = self.project_repository.create(request, user.id.clone()).await
            .map_err(|e| TradocumentError::DatabaseError(format!("Failed to create project: {e}")))?;
        
        // Initialize project file structure
        let project_structure = self.project_manager.initialize_project(&project, &source_language, &target_languages).await
            .map_err(|e| TradocumentError::ProjectError(format!("Failed to initialize project structure: {e}")))?;
        
        // Set as current project
        {
            let mut current_project = self.current_project.write().await;
            *current_project = Some(project.clone());
        }
        
        {
            let mut structure = self.project_structure.write().await;
            *structure = Some(project_structure);
        }
        
        // Add to projects list
        {
            let mut projects = self.projects.write().await;
            projects.push(project);
        }
        
        self.set_status(format!("Project '{name}' created successfully")).await;
        Ok(())
    }
    
    /// Load an existing project by ID
    pub async fn load_project(&self, project_id: uuid::Uuid) -> Result<(), TradocumentError> {
        // Load project from database
        let project = self.project_repository.get_by_id(project_id).await
            .map_err(|e| TradocumentError::DatabaseError(format!("Failed to load project: {e}")))?
            .ok_or_else(|| TradocumentError::ProjectNotFound(format!("Project with ID {project_id} not found")))?;
        
        // Load project structure
        let project_structure = self.project_manager.get_project_structure(project_id).await
            .map_err(|e| TradocumentError::ProjectError(format!("Failed to load project structure: {e}")))?;
        
        // Set as current project
        {
            let mut current_project = self.current_project.write().await;
            *current_project = Some(project.clone());
        }
        
        {
            let mut structure = self.project_structure.write().await;
            *structure = Some(project_structure);
        }
        
        // Clear any existing document content
        {
            let mut content = self.document_content.write().await;
            content.clear();
        }
        
        {
            let mut current_doc = self.current_document.write().await;
            *current_doc = None;
        }
        
        // Clear unsaved changes
        {
            let mut unsaved = self.has_unsaved_changes.write().await;
            *unsaved = false;
        }
        
        self.set_status(format!("Project '{}' loaded successfully", project.name)).await;
        Ok(())
    }
    
    /// Load projects list for current user
    pub async fn load_projects(&self) -> Result<(), TradocumentError> {
        let user = {
            let current_user = self.current_user.read().await;
            current_user.clone().ok_or_else(|| TradocumentError::AuthenticationError("No user logged in".to_string()))?
        };
        
        let projects = self.project_repository.list_by_member(&user.id, None, None).await
            .map_err(|e| TradocumentError::DatabaseError(format!("Failed to load projects: {e}")))?;
        
        {
            let mut project_list = self.projects.write().await;
            *project_list = projects;
        }
        
        Ok(())
    }
    
    /// Save current project state
    pub async fn save_project(&self) -> Result<(), TradocumentError> {
        let current_project = {
            let project = self.current_project.read().await;
            project.clone().ok_or_else(|| TradocumentError::ProjectError("No project currently loaded".to_string()))?
        };
        
        // If there's a current document, save its content
        if let Ok(_) = self.save_document().await {
            // Document was saved successfully
        }
        
        // Update project's updated_at timestamp
        let update_request = UpdateProjectRequest {
            name: None,
            description: None,
            status: None,
            due_date: None,
            priority: None,
        };
        
        self.project_repository.update(current_project.id, update_request).await
            .map_err(|e| TradocumentError::DatabaseError(format!("Failed to update project: {e}")))?;
        
        // Clear unsaved changes
        {
            let mut unsaved = self.has_unsaved_changes.write().await;
            *unsaved = false;
        }
        
        self.set_status(format!("Project '{}' saved successfully", current_project.name)).await;
        Ok(())
    }
    
    /// Close current project
    pub async fn close_project(&self) -> Result<(), TradocumentError> {
        // Check for unsaved changes
        let has_unsaved = self.has_unsaved_changes().await;
        if has_unsaved {
            // In a real implementation, we would show a dialog asking if user wants to save
            // For now, we'll auto-save
            self.save_project().await?;
        }
        
        // Clear current project state
        {
            let mut current_project = self.current_project.write().await;
            *current_project = None;
        }
        
        {
            let mut structure = self.project_structure.write().await;
            *structure = None;
        }
        
        {
            let mut current_doc = self.current_document.write().await;
            *current_doc = None;
        }
        
        {
            let mut content = self.document_content.write().await;
            content.clear();
        }
        
        {
            let mut unsaved = self.has_unsaved_changes.write().await;
            *unsaved = false;
        }
        
        self.set_status("Project closed".to_string()).await;
        Ok(())
    }
    
    /// Update project properties
    pub async fn update_project_properties(&self, name: Option<String>, description: Option<String>, priority: Option<Priority>) -> Result<(), TradocumentError> {
        let current_project = {
            let project = self.current_project.read().await;
            project.clone().ok_or_else(|| TradocumentError::ProjectError("No project currently loaded".to_string()))?
        };
        
        let update_request = UpdateProjectRequest {
            name: name.clone(),
            description: description.clone(),
            status: None,
            due_date: None,
            priority,
        };
        
        let updated_project = self.project_repository.update(current_project.id, update_request).await
            .map_err(|e| TradocumentError::DatabaseError(format!("Failed to update project: {e}")))?
            .ok_or_else(|| TradocumentError::ProjectError("Failed to retrieve updated project".to_string()))?;
        
        // Update current project state
        {
            let mut current_project = self.current_project.write().await;
            *current_project = Some(updated_project.clone());
        }
        
        // Update in projects list
        {
            let mut projects = self.projects.write().await;
            if let Some(index) = projects.iter().position(|p| p.id == updated_project.id) {
                projects[index] = updated_project;
            }
        }
        
        self.set_status("Project properties updated successfully".to_string()).await;
        Ok(())
    }
    
    /// Get current project
    pub async fn get_current_project(&self) -> Option<Project> {
        let project = self.current_project.read().await;
        project.clone()
    }
    
    /// Get current project structure
    pub async fn get_current_project_structure(&self) -> Option<ProjectStructure> {
        let structure = self.project_structure.read().await;
        structure.clone()
    }
    
    /// Check if a project is currently loaded
    pub async fn has_current_project(&self) -> bool {
        let project = self.current_project.read().await;
        project.is_some()
    }

    /// Get all available languages with content
    pub async fn get_languages_with_content(&self) -> Vec<String> {
        let content = self.document_content.read().await;
        content.keys().cloned().collect()
    }

    /// Check if content exists for a specific language
    pub async fn has_content_for_language(&self, language: &str) -> bool {
        let content = self.document_content.read().await;
        content.contains_key(language) && !content.get(language).unwrap_or(&String::new()).trim().is_empty()
    }

    /// Get content statistics for a language
    pub async fn get_content_stats(&self, language: &str) -> ContentStats {
        let content = self.get_content(language).await;
        let lines = content.lines().count();
        let words = content.split_whitespace().count();
        let chars = content.chars().count();
        let chars_no_spaces = content.chars().filter(|c| !c.is_whitespace()).count();

        ContentStats {
            language: language.to_string(),
            lines,
            words,
            characters: chars,
            characters_no_spaces: chars_no_spaces,
            is_empty: content.trim().is_empty(),
        }
    }

    /// Get content statistics for all languages
    pub async fn get_all_content_stats(&self) -> HashMap<String, ContentStats> {
        let mut stats = HashMap::new();
        let languages = self.get_languages_with_content().await;
        
        for language in languages {
            let content_stats = self.get_content_stats(&language).await;
            stats.insert(language, content_stats);
        }
        
        stats
    }

    /// Compare content between two languages to detect structural differences
    pub async fn compare_content_structure(&self, base_language: &str, target_language: &str) -> ContentComparison {
        let base_content = self.get_content(base_language).await;
        let target_content = self.get_content(target_language).await;
        
        let base_sections = self.extract_content_sections(&base_content);
        let target_sections = self.extract_content_sections(&target_content);
        
        let mut missing_sections = Vec::new();
        let mut extra_sections = Vec::new();
        let mut different_sections = Vec::new();
        
        // Check for missing sections in target
        for (idx, base_section) in base_sections.iter().enumerate() {
            if let Some(target_section) = target_sections.get(idx) {
                if base_section.section_type != target_section.section_type {
                    different_sections.push(ContentSectionDifference {
                        section_index: idx,
                        base_type: base_section.section_type.clone(),
                        target_type: target_section.section_type.clone(),
                        severity: if base_section.section_type == "header" { "high" } else { "medium" }.to_string(),
                    });
                }
            } else {
                missing_sections.push(ContentSectionInfo {
                    index: idx,
                    section_type: base_section.section_type.clone(),
                    content_preview: base_section.content.chars().take(100).collect(),
                });
            }
        }
        
        // Check for extra sections in target
        for (idx, target_section) in target_sections.iter().enumerate() {
            if idx >= base_sections.len() {
                extra_sections.push(ContentSectionInfo {
                    index: idx,
                    section_type: target_section.section_type.clone(),
                    content_preview: target_section.content.chars().take(100).collect(),
                });
            }
        }
        
        let similarity_score = if !base_sections.is_empty() {
            let matching_sections = base_sections.len() - missing_sections.len() - different_sections.len();
            matching_sections as f32 / base_sections.len() as f32
        } else {
            1.0
        };
        
        let recommendations = self.generate_content_recommendations(&missing_sections, &different_sections);
        
        ContentComparison {
            base_language: base_language.to_string(),
            target_language: target_language.to_string(),
            similarity_score,
            missing_sections,
            extra_sections,
            different_sections,
            recommendations,
        }
    }

    /// Extract content sections for structural analysis
    fn extract_content_sections(&self, content: &str) -> Vec<ContentSection> {
        let mut sections = Vec::new();
        let mut current_section = String::new();
        let mut section_type = "paragraph";
        let mut section_index = 0;
        
        for line in content.lines() {
            let trimmed = line.trim();
            
            if trimmed.starts_with('#') {
                // Save previous section if it exists
                if !current_section.trim().is_empty() {
                    sections.push(ContentSection {
                        index: section_index,
                        section_type: section_type.to_string(),
                        content: current_section.trim().to_string(),
                    });
                    section_index += 1;
                    current_section.clear();
                }
                
                // Start new header section
                section_type = "header";
                current_section.push_str(line);
                current_section.push('\n');
            } else if trimmed.starts_with('-') || trimmed.starts_with('*') || trimmed.starts_with('+') {
                // List item
                if section_type != "list" {
                    // Save previous section
                    if !current_section.trim().is_empty() {
                        sections.push(ContentSection {
                            index: section_index,
                            section_type: section_type.to_string(),
                            content: current_section.trim().to_string(),
                        });
                        section_index += 1;
                        current_section.clear();
                    }
                    section_type = "list";
                }
                current_section.push_str(line);
                current_section.push('\n');
            } else if trimmed.starts_with("```") {
                // Code block
                if section_type != "code" {
                    if !current_section.trim().is_empty() {
                        sections.push(ContentSection {
                            index: section_index,
                            section_type: section_type.to_string(),
                            content: current_section.trim().to_string(),
                        });
                        section_index += 1;
                        current_section.clear();
                    }
                    section_type = "code";
                }
                current_section.push_str(line);
                current_section.push('\n');
            } else if !trimmed.is_empty() {
                // Regular paragraph
                if section_type != "paragraph" && section_type != "header" {
                    if !current_section.trim().is_empty() {
                        sections.push(ContentSection {
                            index: section_index,
                            section_type: section_type.to_string(),
                            content: current_section.trim().to_string(),
                        });
                        section_index += 1;
                        current_section.clear();
                    }
                    section_type = "paragraph";
                }
                current_section.push_str(line);
                current_section.push('\n');
            } else if !current_section.trim().is_empty() {
                // Empty line - might indicate section break
                sections.push(ContentSection {
                    index: section_index,
                    section_type: section_type.to_string(),  
                    content: current_section.trim().to_string(),
                });
                section_index += 1;
                current_section.clear();
                section_type = "paragraph";
            }
        }
        
        // Don't forget the last section
        if !current_section.trim().is_empty() {
            sections.push(ContentSection {
                index: section_index,
                section_type: section_type.to_string(),
                content: current_section.trim().to_string(),
            });
        }
        
        sections
    }

    /// Generate recommendations based on content comparison
    fn generate_content_recommendations(&self, missing_sections: &[ContentSectionInfo], different_sections: &[ContentSectionDifference]) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        if !missing_sections.is_empty() {
            recommendations.push(format!("Add {} missing sections to maintain document structure", missing_sections.len()));
            
            for missing in missing_sections {
                if missing.section_type == "header" {
                    recommendations.push(format!("Missing header at position {}: {}", missing.index, missing.content_preview));
                }
            }
        }
        
        if !different_sections.is_empty() {
            recommendations.push(format!("Review {} structural differences between languages", different_sections.len()));
            
            for diff in different_sections {
                if diff.severity == "high" {
                    recommendations.push(format!("Critical structural difference at section {}: {} vs {}", diff.section_index, diff.base_type, diff.target_type));
                }
            }
        }
        
        if missing_sections.is_empty() && different_sections.is_empty() {
            recommendations.push("Content structure looks consistent across languages".to_string());
        }
        
        recommendations
    }
    
    /// Update sidebar tree view items based on current project structure
    pub async fn update_sidebar_tree_items(&self) -> Result<(), TradocumentError> {
        let project_structure = {
            let structure = self.project_structure.read().await;
            structure.clone()
        };
        
        if let Some(structure) = project_structure {
            let mut tree_items = Vec::new();
            
            // Add project root
            tree_items.push(TreeViewItem {
                id: structure.project_id.to_string(),
                name: "Project Root".to_string(),
                item_type: "folder".to_string(),
                path: structure.base_path.clone(),
                status: "".to_string(),
                language: "".to_string(),
                icon: "📁".to_string(),
                expanded: true,
                level: 0,
                parent_id: "".to_string(),
                has_children: !structure.chapters.is_empty(),
                word_count: 0,
                progress: 0.0,
                last_modified: "Recently".to_string(),
            });
            
            // Add language folders
            for (lang_index, language) in structure.languages.iter().enumerate() {
                let lang_id = format!("lang_{language}");
                tree_items.push(TreeViewItem {
                    id: lang_id.clone(),
                    name: format!("{} ({})", self.get_language_name(language), language.to_uppercase()),
                    item_type: "folder".to_string(),
                    path: format!("{}/chapters/{}", structure.base_path, language),
                    status: "".to_string(),
                    language: language.clone(),
                    icon: "🌐".to_string(),
                    expanded: lang_index == 0, // Expand source language by default
                    level: 1,
                    parent_id: structure.project_id.to_string(),
                    has_children: !structure.chapters.is_empty(),
                    word_count: 0,
                    progress: self.calculate_language_progress(language, &structure.chapters).await,
                    last_modified: "Recently".to_string(),
                });
                
                // Add chapters for this language
                for chapter in &structure.chapters {
                    if chapter.file_paths.contains_key(language) {
                        let chapter_id = format!("chapter_{}_{}", chapter.slug, language);
                        let title = chapter.title.get(language)
                            .unwrap_or(&format!("Chapter {}", chapter.chapter_number))
                            .clone();
                        
                        tree_items.push(TreeViewItem {
                            id: chapter_id,
                            name: format!("{:02}. {}", chapter.chapter_number, title),
                            item_type: "document".to_string(),
                            path: chapter.file_paths.get(language).unwrap_or(&"".to_string()).clone(),
                            status: self.get_chapter_status(&chapter.slug, language).await,
                            language: language.clone(),
                            icon: "📄".to_string(),
                            expanded: false,
                            level: 2,
                            parent_id: lang_id.clone(),
                            has_children: false,
                            word_count: self.get_chapter_word_count(&chapter.slug, language).await,
                            progress: self.get_chapter_translation_progress(&chapter.slug, language).await,
                            last_modified: self.get_chapter_last_modified(&chapter.slug, language).await,
                        });
                    }
                }
            }
            
            // Update sidebar state
            {
                let mut sidebar_state = self.sidebar_state.write().await;
                sidebar_state.tree_items = tree_items;
                sidebar_state.search_results.clear();
            }
        }
        
        Ok(())
    }
    
    /// Update recent documents list
    pub async fn update_recent_documents(&self) -> Result<(), TradocumentError> {
        // In a real implementation, this would load from a persistent store
        let mut recent_docs = Vec::new();
        
        // Add some sample recent documents based on current project
        if let Some(project) = self.get_current_project().await {
            if let Some(structure) = self.get_current_project_structure().await {
                for (index, chapter) in structure.chapters.iter().take(5).enumerate() {
                    for language in &structure.languages {
                        if let Some(title) = chapter.title.get(language) {
                            recent_docs.push(RecentDocument {
                                id: format!("recent_{}_{}", chapter.slug, language),
                                name: title.clone(),
                                path: chapter.file_paths.get(language).unwrap_or(&"".to_string()).clone(),
                                language: language.clone(),
                                thumbnail: "".to_string(),
                                last_opened: format!("{} hours ago", index + 1),
                            });
                        }
                    }
                }
            }
        }
        
        {
            let mut sidebar_state = self.sidebar_state.write().await;
            sidebar_state.recent_documents = recent_docs;
        }
        
        Ok(())
    }
    
    /// Update project statistics
    pub async fn update_project_stats(&self) -> Result<(), TradocumentError> {
        let mut stats = ProjectStats::default();
        
        if let Some(structure) = self.get_current_project_structure().await {
            stats.total_documents = structure.chapters.len() as i32;
            stats.languages_count = structure.languages.len() as i32;
            
            // Calculate total words across all chapters and languages
            let mut total_words = 0;
            for chapter in &structure.chapters {
                for language in &structure.languages {
                    total_words += self.get_chapter_word_count(&chapter.slug, language).await;
                }
            }
            stats.total_words = total_words;
            
            // Calculate overall completion rate
            let mut total_progress = 0.0;
            let mut progress_count = 0;
            for chapter in &structure.chapters {
                for language in &structure.languages {
                    total_progress += self.get_chapter_translation_progress(&chapter.slug, language).await;
                    progress_count += 1;
                }
            }
            
            if progress_count > 0 {
                stats.completion_rate = total_progress / progress_count as f32;
            }
            
            stats.recent_activity = "Active development".to_string();
        }
        
        {
            let mut sidebar_state = self.sidebar_state.write().await;
            sidebar_state.project_stats = stats;
        }
        
        Ok(())
    }
    
    /// Search documents based on query
    pub async fn search_documents(&self, query: &str) -> Result<Vec<TreeViewItem>, TradocumentError> {
        let sidebar_state = self.sidebar_state.read().await;
        let mut results = Vec::new();
        
        if query.trim().is_empty() {
            return Ok(sidebar_state.tree_items.clone());
        }
        
        let query_lower = query.to_lowercase();
        
        for item in &sidebar_state.tree_items {
            if item.name.to_lowercase().contains(&query_lower) ||
               item.path.to_lowercase().contains(&query_lower) ||
               item.language.to_lowercase().contains(&query_lower) {
                results.push(item.clone());
            }
        }
        
        Ok(results)
    }
    
    /// Get sidebar state
    pub async fn get_sidebar_state(&self) -> SidebarState {
        let sidebar_state = self.sidebar_state.read().await;
        sidebar_state.clone()
    }
    
    /// Toggle tree item expansion
    pub async fn toggle_tree_item_expansion(&self, item_id: &str, expanded: bool) {
        let mut sidebar_state = self.sidebar_state.write().await;
        
        for item in &mut sidebar_state.tree_items {
            if item.id == item_id {
                item.expanded = expanded;
                break;
            }
        }
    }
    
    /// Set selected tree item
    pub async fn set_selected_tree_item(&self, item_id: &str) {
        let mut sidebar_state = self.sidebar_state.write().await;
        sidebar_state.selected_item_id = item_id.to_string();
    }
    
    /// Helper methods for chapter data
    async fn get_chapter_status(&self, _chapter_slug: &str, _language: &str) -> String {
        // In a real implementation, this would check the actual chapter status
        "draft".to_string()
    }
    
    async fn get_chapter_word_count(&self, _chapter_slug: &str, _language: &str) -> i32 {
        // In a real implementation, this would calculate actual word count
        150 + (rand::random::<u32>() % 500) as i32
    }
    
    async fn get_chapter_translation_progress(&self, _chapter_slug: &str, language: &str) -> f32 {
        // In a real implementation, this would check translation progress
        if language == "en" {
            1.0 // Source language is always 100%
        } else {
            0.3 + (rand::random::<f32>() * 0.7) // Random progress between 30% and 100%
        }
    }
    
    async fn get_chapter_last_modified(&self, _chapter_slug: &str, _language: &str) -> String {
        // In a real implementation, this would check file modification time
        "2 hours ago".to_string()
    }
    
    async fn calculate_language_progress(&self, language: &str, chapters: &[ChapterInfo]) -> f32 {
        if language == "en" {
            return 1.0; // Source language is always complete
        }
        
        let mut total_progress = 0.0;
        for chapter in chapters {
            total_progress += self.get_chapter_translation_progress(&chapter.slug, language).await;
        }
        
        if chapters.is_empty() {
            0.0
        } else {
            total_progress / chapters.len() as f32
        }
    }
    
    fn get_language_name(&self, language_code: &str) -> &'static str {
        match language_code {
            "en" => "English",
            "de" => "German",
            "fr" => "French",
            "es" => "Spanish",
            "it" => "Italian",
            "nl" => "Dutch",
            _ => "Unknown",
        }
    }
    
    // Project Creation Wizard Methods
    
    /// Initialize project creation wizard
    pub async fn start_project_wizard(&self) -> Result<(), TradocumentError> {
        let mut wizard_data = ProjectWizardData::default();
        
        // Initialize with available languages
        let available_languages = get_available_languages();
        wizard_data.target_languages = available_languages;
        
        {
            let mut wizard = self.wizard_data.write().await;
            *wizard = Some(wizard_data);
        }
        
        self.set_status("Project creation wizard started".to_string()).await;
        Ok(())
    }
    
    /// Get wizard data
    pub async fn get_wizard_data(&self) -> Option<ProjectWizardData> {
        let wizard = self.wizard_data.read().await;
        wizard.clone()
    }
    
    /// Update wizard data
    pub async fn update_wizard_data(&self, data: ProjectWizardData) {
        {
            let mut wizard = self.wizard_data.write().await;
            *wizard = Some(data);
        }
    }
    
    /// Validate wizard step
    pub fn validate_wizard_step(&self, step: u32) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, TradocumentError>> + Send + '_>> {
        Box::pin(async move {
        let wizard_data = {
            let wizard = self.wizard_data.read().await;
            wizard.clone().ok_or_else(|| TradocumentError::ProjectError("Wizard not initialized".to_string()))?
        };
        
        let mut errors = Vec::new();
        
        match step {
            1 => {
                // Validate project details
                if wizard_data.name.trim().is_empty() {
                    errors.push("Project name is required".to_string());
                }
                if wizard_data.name.trim().len() < 3 {
                    errors.push("Project name must be at least 3 characters long".to_string());
                }
                if wizard_data.name.trim().len() > 100 {
                    errors.push("Project name must be less than 100 characters".to_string());
                }
                // Validate project name doesn't contain invalid characters
                if wizard_data.name.chars().any(|c| "<>:\"/\\|?*".contains(c)) {
                    errors.push("Project name contains invalid characters".to_string());
                }
            }
            2 => {
                // Validate template selection
                if self.template_manager.get_template(&wizard_data.template_id).is_none() {
                    errors.push("Please select a valid template".to_string());
                }
            }
            3 => {
                // Validate language configuration
                if wizard_data.source_language.is_empty() {
                    errors.push("Source language is required".to_string());
                }
                let enabled_target_languages = wizard_data.target_languages.iter()
                    .filter(|lang| lang.enabled && lang.code != wizard_data.source_language)
                    .count();
                if enabled_target_languages == 0 {
                    errors.push("At least one target language must be selected".to_string());
                }
            }
            4 => {
                // Validate team setup (optional step, no hard requirements)
                for member in &wizard_data.team_members {
                    if member.name.trim().is_empty() {
                        errors.push("Team member name cannot be empty".to_string());
                    }
                    if member.email.trim().is_empty() || !member.email.contains('@') {
                        errors.push(format!("Invalid email address for team member: {}", member.name));
                    }
                    if member.languages.is_empty() {
                        errors.push(format!("Team member {} must be assigned to at least one language", member.name));
                    }
                }
            }
            5 => {
                // Validate project structure (mostly defaults are fine)
                if wizard_data.export_formats.is_empty() {
                    errors.push("At least one export format must be selected".to_string());
                }
            }
            6 => {
                // Final validation - all previous steps
                for prev_step in 1..6 {
                    let step_errors = self.validate_wizard_step(prev_step).await?;
                    errors.extend(step_errors);
                }
            }
            _ => {
                errors.push("Invalid wizard step".to_string());
            }
        }
        
        Ok(errors)
        })
    }
    
    /// Complete wizard and create project
    pub async fn complete_wizard(&self) -> Result<Project, TradocumentError> {
        let wizard_data = {
            let wizard = self.wizard_data.read().await;
            wizard.clone().ok_or_else(|| TradocumentError::ProjectError("Wizard not initialized".to_string()))?
        };
        
        // Final validation
        let errors = self.validate_wizard_step(6).await?;
        if !errors.is_empty() {
            return Err(TradocumentError::Validation(format!("Validation failed: {}", errors.join(", "))));
        }
        
        let user = {
            let current_user = self.current_user.read().await;
            current_user.clone().ok_or_else(|| TradocumentError::AuthenticationError("No user logged in".to_string()))?
        };
        
        // Create project request
        let request = CreateProjectRequest {
            name: wizard_data.name.clone(),
            description: wizard_data.description.clone(),
            due_date: wizard_data.due_date,
            priority: wizard_data.priority,
        };
        
        // Create project in database
        let project = self.project_repository.create(request, user.id.clone()).await
            .map_err(|e| TradocumentError::DatabaseError(format!("Failed to create project: {e}")))?;
        
        // Get target languages that are enabled
        let target_languages: Vec<String> = wizard_data.target_languages.iter()
            .filter(|lang| lang.enabled && lang.code != wizard_data.source_language)
            .map(|lang| lang.code.clone())
            .collect();
        
        // Initialize project file structure
        let project_structure = self.project_manager.initialize_project(
            &project, 
            &wizard_data.source_language, 
            &target_languages
        ).await.map_err(|e| TradocumentError::ProjectError(format!("Failed to initialize project structure: {e}")))?;
        
        // Create initial chapters from template
        if let Some(template) = self.template_manager.get_template(&wizard_data.template_id) {
            for chapter_template in &template.initial_chapters {
                let chapter = crate::models::document::Chapter {
                    id: uuid::Uuid::new_v4(),
                    document_id: project.id,
                    chapter_number: chapter_template.chapter_number,
                    title: {
                        let mut titles = HashMap::new();
                        titles.insert(wizard_data.source_language.clone(), chapter_template.title.clone());
                        titles
                    },
                    slug: chapter_template.slug.clone(),
                    content: {
                        let mut content = HashMap::new();
                        content.insert(wizard_data.source_language.clone(), chapter_template.content_template.clone());
                        content
                    },
                    order: chapter_template.chapter_number,
                    status: crate::models::document::ChapterStatus::Draft,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };
                
                self.project_manager.create_chapter(project.id, &chapter).await
                    .map_err(|e| TradocumentError::ProjectError(format!("Failed to create chapter: {e}")))?;
            }
        }
        
        // Set as current project
        {
            let mut current_project = self.current_project.write().await;
            *current_project = Some(project.clone());
        }
        
        {
            let mut structure = self.project_structure.write().await;
            *structure = Some(project_structure);
        }
        
        // Add to projects list
        {
            let mut projects = self.projects.write().await;
            projects.push(project.clone());
        }
        
        // Clear wizard data
        {
            let mut wizard = self.wizard_data.write().await;
            *wizard = None;
        }
        
        // Update sidebar with new project structure
        self.update_sidebar_tree_items().await?;
        self.update_recent_documents().await?;
        self.update_project_stats().await?;
        
        self.set_status(format!("Project '{}' created successfully!", project.name)).await;
        Ok(project)
    }
    
    /// Cancel wizard and clear data
    pub async fn cancel_wizard(&self) {
        {
            let mut wizard = self.wizard_data.write().await;
            *wizard = None;
        }
        
        self.set_status("Project creation cancelled".to_string()).await;
    }
    
    /// Get available templates
    pub fn get_available_templates(&self) -> &TemplateManager {
        &self.template_manager
    }
    
    /// Check if wizard is active
    pub async fn is_wizard_active(&self) -> bool {
        let wizard = self.wizard_data.read().await;
        wizard.is_some()
    }
    
    // Project Browser Methods
    
    /// Load projects for browser
    pub async fn load_projects_for_browser(&self) -> Result<(), TradocumentError> {
        let user = {
            let current_user = self.current_user.read().await;
            current_user.clone().ok_or_else(|| TradocumentError::AuthenticationError("No user logged in".to_string()))?
        };
        
        // Load projects from repository
        let projects = self.project_repository.list_by_member(&user.id, None, None).await
            .map_err(|e| TradocumentError::DatabaseError(format!("Failed to load projects: {e}")))?;
        
        // Convert to browser items
        let mut browser_items = Vec::new();
        let mut recent_projects = Vec::new();
        
        for project in projects {
            // Get project summary
            let summary = self.project_repository.get_summary(project.id).await
                .map_err(|e| TradocumentError::DatabaseError(format!("Failed to get project summary: {e}")))?
                .unwrap_or_else(|| crate::models::project::ProjectSummary {
                    id: project.id,
                    name: project.name.clone(),
                    status: project.status.clone(),
                    priority: project.priority.clone(),
                    owner_id: project.owner_id.clone(),
                    member_count: 1,
                    document_count: 0,
                    kanban_card_count: 0,
                    created_at: project.created_at,
                    due_date: project.due_date,
                });
            
            // Determine access level (simplified logic)
            let access_level = if project.owner_id == user.id {
                AccessLevel::Owner
            } else {
                AccessLevel::Editor
            };
            
            // Check if favorite (would come from user preferences)
            let is_favorite = false; // Placeholder
            
            let browser_item = ProjectBrowserItem::from_project_and_summary(
                &project,
                &summary,
                &project.owner_id, // Would lookup actual owner name
                is_favorite,
                access_level,
            );
            
            browser_items.push(browser_item.clone());
            
            // Add to recent projects if recently accessed
            let days_since_update = (chrono::Utc::now() - project.updated_at).num_days();
            if days_since_update <= 30 {
                recent_projects.push(RecentProject {
                    project_id: project.id,
                    name: project.name.clone(),
                    last_accessed: project.updated_at,
                    access_count: 1, // Would track actual access count
                    thumbnail: None,
                });
            }
        }
        
        // Sort recent projects by last accessed
        recent_projects.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
        recent_projects.truncate(10); // Keep only top 10
        
        // Update browser state
        {
            let mut browser_state = self.project_browser_state.write().await;
            browser_state.projects = browser_items.clone();
            browser_state.filtered_projects = browser_items;
            browser_state.recent_projects = recent_projects;
            browser_state.is_loading = false;
        }
        
        Ok(())
    }
    
    /// Search and filter projects in browser
    pub async fn filter_browser_projects(&self, search_query: String, filters: ProjectFilters, search_options: SearchOptions) -> Result<(), TradocumentError> {
        let browser_state = self.project_browser_state.read().await;
        let all_projects = browser_state.projects.clone();
        drop(browser_state);
        
        // Apply search and filters
        let filtered_projects: Vec<ProjectBrowserItem> = all_projects
            .into_iter()
            .filter(|project| {
                // Apply search filter
                if !project.matches_search(&search_query, &search_options) {
                    return false;
                }
                
                // Apply other filters
                project.matches_filters(&filters)
            })
            .collect();
        
        // Update filtered results
        {
            let mut browser_state = self.project_browser_state.write().await;
            browser_state.filtered_projects = filtered_projects;
            browser_state.search_query = search_query;
            browser_state.filters = filters;
            
            // Update pagination
            let total_items = browser_state.filtered_projects.len();
            browser_state.total_pages = total_items.div_ceil(browser_state.items_per_page);
            if browser_state.total_pages == 0 {
                browser_state.total_pages = 1;
            }
            if browser_state.current_page > browser_state.total_pages {
                browser_state.current_page = 1;
            }
        }
        
        Ok(())
    }
    
    /// Sort projects in browser
    pub async fn sort_browser_projects(&self, sort_config: SortConfig) -> Result<(), TradocumentError> {
        let mut browser_state = self.project_browser_state.write().await;
        
        browser_state.filtered_projects.sort_by(|a, b| {
            use crate::models::project_browser::{SortField, SortDirection};
            
            let comparison = match sort_config.field {
                SortField::Name => a.name.cmp(&b.name),
                SortField::Created => a.created_at.cmp(&b.created_at),
                SortField::Updated => a.updated_at.cmp(&b.updated_at),
                SortField::DueDate => {
                    match (&a.due_date, &b.due_date) {
                        (Some(a_date), Some(b_date)) => a_date.cmp(b_date),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                },
                SortField::Priority => a.priority.as_number().cmp(&b.priority.as_number()),
                SortField::Status => a.status.as_str().cmp(b.status.as_str()),
                SortField::Progress => a.progress_percentage.partial_cmp(&b.progress_percentage).unwrap_or(std::cmp::Ordering::Equal),
                SortField::LastActivity => a.updated_at.cmp(&b.updated_at),
                SortField::MemberCount => a.member_count.cmp(&b.member_count),
                SortField::DocumentCount => a.document_count.cmp(&b.document_count),
            };
            
            match sort_config.direction {
                SortDirection::Ascending => comparison,
                SortDirection::Descending => comparison.reverse(),
            }
        });
        
        browser_state.sort_config = sort_config;
        
        Ok(())
    }
    
    /// Toggle project favorite status
    pub async fn toggle_project_favorite(&self, project_id: uuid::Uuid) -> Result<(), TradocumentError> {
        let mut browser_state = self.project_browser_state.write().await;
        
        // Update in projects list
        for project in &mut browser_state.projects {
            if project.id == project_id {
                project.is_favorite = !project.is_favorite;
                
                // Update favorites list
                if project.is_favorite {
                    if !browser_state.favorite_projects.contains(&project_id) {
                        browser_state.favorite_projects.push(project_id);
                    }
                } else {
                    browser_state.favorite_projects.retain(|&id| id != project_id);
                }
                break;
            }
        }
        
        // Update in filtered projects
        for project in &mut browser_state.filtered_projects {
            if project.id == project_id {
                project.is_favorite = !project.is_favorite;
                break;
            }
        }
        
        // In a real implementation, this would be persisted to database/preferences
        
        Ok(())
    }
    
    /// Get project browser state
    pub async fn get_project_browser_state(&self) -> ProjectBrowserState {
        let browser_state = self.project_browser_state.read().await;
        browser_state.clone()
    }
    
    /// Set browser view mode
    pub async fn set_browser_view_mode(&self, view_mode: ViewMode) {
        let mut browser_state = self.project_browser_state.write().await;
        browser_state.view_mode = view_mode;
    }
    
    /// Set browser page
    pub async fn set_browser_page(&self, page: usize) {
        let mut browser_state = self.project_browser_state.write().await;
        if page >= 1 && page <= browser_state.total_pages {
            browser_state.current_page = page;
        }
    }
    
    /// Get paginated projects for current page
    pub async fn get_paginated_browser_projects(&self) -> Vec<ProjectBrowserItem> {
        let browser_state = self.project_browser_state.read().await;
        let start_index = (browser_state.current_page - 1) * browser_state.items_per_page;
        let end_index = std::cmp::min(start_index + browser_state.items_per_page, browser_state.filtered_projects.len());
        
        if start_index < browser_state.filtered_projects.len() {
            browser_state.filtered_projects[start_index..end_index].to_vec()
        } else {
            Vec::new()
        }
    }
    
    /// Set browser loading state
    pub async fn set_browser_loading(&self, loading: bool) {
        let mut browser_state = self.project_browser_state.write().await;
        browser_state.is_loading = loading;
    }
    
    /// Select project in browser
    pub async fn select_browser_project(&self, project_id: uuid::Uuid) {
        let mut browser_state = self.project_browser_state.write().await;
        browser_state.selected_project_id = Some(project_id);
    }
    
    /// Clear browser selection
    pub async fn clear_browser_selection(&self) {
        let mut browser_state = self.project_browser_state.write().await;
        browser_state.selected_project_id = None;
    }
    
    /// Add to recent projects
    pub async fn add_to_recent_projects(&self, project_id: uuid::Uuid) -> Result<(), TradocumentError> {
        let project = self.project_repository.get_by_id(project_id).await
            .map_err(|e| TradocumentError::DatabaseError(format!("Failed to get project: {e}")))?
            .ok_or_else(|| TradocumentError::ProjectNotFound(format!("Project {project_id} not found")))?;
        
        let mut browser_state = self.project_browser_state.write().await;
        
        // Remove if already exists
        browser_state.recent_projects.retain(|recent| recent.project_id != project_id);
        
        // Add to front
        browser_state.recent_projects.insert(0, RecentProject {
            project_id,
            name: project.name,
            last_accessed: chrono::Utc::now(),
            access_count: 1, // Would increment actual count
            thumbnail: None,
        });
        
        // Keep only top 10
        browser_state.recent_projects.truncate(10);
        
        Ok(())
    }
}

/// Content statistics for a specific language
#[derive(Debug, Clone)]
pub struct ContentStats {
    pub language: String,
    pub lines: usize,
    pub words: usize,
    pub characters: usize,
    pub characters_no_spaces: usize,
    pub is_empty: bool,
}

/// Content section for structural analysis
#[derive(Debug, Clone)]
pub struct ContentSection {
    pub index: usize,
    pub section_type: String,
    pub content: String,
}

/// Information about a missing or extra content section
#[derive(Debug, Clone)]
pub struct ContentSectionInfo {
    pub index: usize,
    pub section_type: String,
    pub content_preview: String,
}

/// Structural difference between content sections
#[derive(Debug, Clone)]
pub struct ContentSectionDifference {
    pub section_index: usize,
    pub base_type: String,
    pub target_type: String,
    pub severity: String,
}

/// Comparison result between two language contents
#[derive(Debug, Clone)]
pub struct ContentComparison {
    pub base_language: String,
    pub target_language: String,
    pub similarity_score: f32,
    pub missing_sections: Vec<ContentSectionInfo>,
    pub extra_sections: Vec<ContentSectionInfo>,
    pub different_sections: Vec<ContentSectionDifference>,
    pub recommendations: Vec<String>,
}

/// Enhanced sidebar state management
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct SidebarState {
    pub tree_items: Vec<TreeViewItem>,
    pub recent_documents: Vec<RecentDocument>,
    pub project_stats: ProjectStats,
    pub search_query: String,
    pub search_results: Vec<TreeViewItem>,
    pub selected_item_id: String,
    pub expanded_items: std::collections::HashSet<String>,
    pub sections_collapsed: SectionCollapseState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct SectionCollapseState {
    pub project_tree: bool,
    pub quick_actions: bool,
    pub recent_documents: bool,
    pub statistics: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeViewItem {
    pub id: String,
    pub name: String,
    pub item_type: String, // "folder", "document", "chapter"
    pub path: String,
    pub status: String, // "draft", "in_translation", "under_review", "approved", "published"
    pub language: String,
    pub icon: String,
    pub expanded: bool,
    pub level: i32,
    pub parent_id: String,
    pub has_children: bool,
    pub word_count: i32,
    pub progress: f32, // 0.0 - 1.0 for translation progress
    pub last_modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentDocument {
    pub id: String,
    pub name: String,
    pub path: String,
    pub language: String,
    pub thumbnail: String,
    pub last_opened: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickAction {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub shortcut: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStats {
    pub total_documents: i32,
    pub total_words: i32,
    pub completion_rate: f32,
    pub languages_count: i32,
    pub recent_activity: String,
}



impl Default for ProjectStats {
    fn default() -> Self {
        Self {
            total_documents: 0,
            total_words: 0,
            completion_rate: 0.0,
            languages_count: 0,
            recent_activity: "No recent activity".to_string(),
        }
    }
}