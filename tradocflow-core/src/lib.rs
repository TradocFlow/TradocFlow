//! # Tradocument Reviewer
//! 
//! A collaborative document review system with multi-language support,
//! designed for creating technical manuals with WYSIWYG editing and 
//! Word-like review workflows.

// Initialize rust-i18n with our locale files
rust_i18n::i18n!("locales", fallback = "en");

pub mod review_system;
// pub mod translation_manager; // Temporarily disabled due to document references
pub mod export_engine;
pub mod integration;
// pub mod document_import_service; // Temporarily disabled
pub mod notification_system;
pub mod i18n;
pub mod database;
pub mod models;
pub mod services;
pub mod gui;
pub mod git_integration;

// Slint generated UI module - must be after all other modules
slint::include_modules!();

#[cfg(test)]
mod test_i18n;

pub use review_system::*;
// pub use translation_manager::{TranslationManager, TranslationProject}; // Temporarily disabled
pub use export_engine::*;
// pub use document_import_service::{
//     DocumentImportService, DocumentFile, DocumentContent, MultiDocumentImportResult,
//     SingleDocumentImportResult, ImportError
// }; // Temporarily disabled
pub use notification_system::{
    NotificationService, 
    Notification, 
    NotificationType, 
    NotificationPreferences,
    NotificationMetadata,
    NotificationPriority
};
pub use i18n::{Language, I18nContext};
pub use database::Database;
pub use models::*;
// Type alias for backward compatibility
pub use models::MemberRole as UserRole;
pub use git_integration::{
    GitWorkflowManager, 
    GitConfig,
    WorkSession,
    ReviewRequest,
    SessionManager,
    CommitMessageBuilder,
    CommitTemplates,
    initialize_translation_repository
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// User representation for Git integration and task management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub active: bool,
}

// Export-compatible Document type (kept for backward compatibility with export engine)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub title: String,
    pub content: HashMap<String, String>, // language -> markdown content
    pub metadata: DocumentMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub project_id: Option<String>,
    pub screenshots: Vec<ScreenshotReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotReference {
    pub id: String,
    pub language: String,
    pub screen_config: String, // JSON config for screenshot_creator
    pub generated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manual {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub sections: Vec<ManualSection>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: String,
    pub languages: Vec<String>,
    pub template_type: ManualTemplate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualSection {
    pub id: Uuid,
    pub title: String,
    pub order: u32,
    pub document_id: Option<Uuid>, // Links to a Document
    pub subsections: Vec<ManualSection>,
    pub section_type: SectionType,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SectionType {
    Introduction,
    Installation,
    Configuration,
    UserGuide,
    Troubleshooting,
    Reference,
    Appendix,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ManualTemplate {
    TechnicalManual,
    UserGuide,
    InstallationGuide,
    BellTowerController,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub type Result<T> = std::result::Result<T, TradocumentError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentImportRequest {
    pub title: String,
    pub target_languages: Vec<String>,
    pub source_language: String,
    pub extract_images: bool,
    pub preserve_formatting: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentImportResult {
    pub document_id: Uuid,
    pub success: bool,
    pub messages: Vec<String>,
    pub warnings: Vec<String>,
    pub extracted_images: Vec<String>,
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    pub step: String,
    pub progress_percent: u8,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TradocumentError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("PDF generation error: {0}")]
    Pdf(String),
    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("Document not found: {0}")]
    DocumentNotFound(String),
    #[error("Language not supported: {0}")]
    UnsupportedLanguage(String),
    #[error("Review error: {0}")]
    Review(String),
    #[error("Document import error: {0}")]
    DocumentImport(String),
    #[error("File format not supported: {0}")]
    UnsupportedFormat(String),
    #[error("Notification error: {0}")]
    Notification(String),
    #[error("API error: {0}")]
    ApiError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Slint UI error: {0}")]
    SlintError(String),
    #[error("Git integration error: {0}")]
    Git(#[from] git_integration::GitError),
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    #[error("Project error: {0}")]
    ProjectError(String),
    #[error("Project not found: {0}")]
    ProjectNotFound(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("File error: {0}")]
    FileError(String),
    #[error("Sync error: {0}")]
    SyncError(String),
    #[error("Translation memory error: {0}")]
    TranslationMemory(#[from] tradocflow_translation_memory::TranslationMemoryError),
    #[error("Terminology error: {0}")]
    Terminology(String),
    #[error("UI error: {0}")]
    Ui(String),
    // #[error("DuckDB error: {0}")]
    // DuckDB(#[from] duckdb::Error), // Temporarily disabled
    // #[error("Parquet error: {0}")]
    // Parquet(String), // Temporarily disabled
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
}