// Module declarations - individual imports are handled in each submodule

pub mod project;
pub mod project_browser;
pub mod project_template;
pub mod kanban;
pub mod member;
pub mod translation_progress;
pub mod document;
pub mod translation_models;

// Re-export the main models
pub use project::*;
pub use project_browser::*;
pub use project_template::*;
pub use kanban::*;
pub use member::*;
// Avoid re-exporting conflicting TranslationStatus from translation_progress
pub use translation_progress::{TranslationProgress, CreateTranslationProgressRequest, UpdateTranslationProgressRequest, TranslationProgressSummary};
// Re-export document models but avoid conflicts with translation_models
pub use document::{DocumentStatus, DocumentMetadata, Document, DocumentType, Chapter, ChapterStatus, TranslationUnit, TranslationVersion, TranslationNote, CreateDocumentRequest, CreateChapterRequest, ProjectStructure, ChapterInfo};
// Re-export translation models - these take precedence for translation system
pub use translation_models::*;