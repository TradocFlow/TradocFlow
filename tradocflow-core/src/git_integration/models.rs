//! Git Integration Models
//! 
//! Data structures for Git-based translation workflow management.
//! This module integrates with the TOML-based data layer for persistent storage.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// Re-export TOML data structures for convenience
pub use crate::git_integration::toml_data::{
    ProjectData as TomlProjectData,
    ChapterData as TomlChapterData,
    ProjectMetadata as TomlProjectMetadata,
    ChapterMetadata as TomlChapterMetadata,
    TranslationUnit as TomlTranslationUnit,
    TranslationVersion as TomlTranslationVersion,
    ProjectTodo as TomlProjectTodo,
    ChapterTodo as TomlChapterTodo,
    UnitTodo as TomlUnitTodo,
    Comment as TomlComment,
    TranslationNote as TomlTranslationNote,
    ProjectStatus as TomlProjectStatus, 
    ChapterStatus as TomlChapterStatus, 
    TranslationStatus as TomlTranslationStatus,
    Priority as TomlPriority, 
    TodoStatus as TomlTodoStatus, 
    TodoType as TomlTodoType, 
    CommentType as TomlCommentType,
    ComplexityLevel as TomlComplexityLevel, 
    DifficultyLevel as TomlDifficultyLevel, 
    UnitType as TomlUnitType,
    TranslationMethod as TomlTranslationMethod, 
    NoteType as TomlNoteType, 
    NoteVisibility as TomlNoteVisibility,
};

/// Work session for a translator working on a specific chapter/language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkSession {
    pub id: Uuid,
    pub branch: String,
    pub chapter: String,
    pub language: String,
    pub user_id: String,
    pub markdown_path: String,
    pub started_at: DateTime<Utc>,
    pub last_save: Option<DateTime<Utc>>,
    pub auto_save_enabled: bool,
}

/// Review request for completed translation work
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRequest {
    pub id: Uuid,
    pub pr_number: u64,
    pub branch: String,
    pub chapter: String,
    pub language: String,
    pub translator: String,
    pub reviewer: Option<String>,
    pub status: ReviewStatus,
    pub created_at: DateTime<Utc>,
    pub changes_summary: String,
}

/// Status of a review request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReviewStatus {
    Pending,
    InReview,
    Approved,
    ChangesRequested,
    Rejected,
}

/// Diff between translation versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationDiff {
    pub chapter: String,
    pub from_commit: String,
    pub to_commit: String,
    pub changes: Vec<TranslationChange>,
    pub stats: DiffStats,
}

/// Individual change in a translation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationChange {
    pub unit_id: String,
    pub change_type: ChangeType,
    pub old_text: Option<String>,
    pub new_text: Option<String>,
    pub author: String,
    pub timestamp: DateTime<Utc>,
}

/// Type of change made to a translation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Moved,
}

/// Statistics for a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffStats {
    pub files_changed: u32,
    pub additions: u32,
    pub deletions: u32,
}

/// TOML-based project data structure matching the PRD schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectData {
    pub project: ProjectMetadata,
    pub todos: Vec<Todo>,
}

/// Project metadata from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: ProjectStatus,
    pub languages: ProjectLanguages,
    pub team: ProjectTeam,
    pub settings: ProjectSettings,
    pub metadata: Option<ProjectMetadataExtra>,
}

/// Project status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Active,
    Completed,
    Archived,
    OnHold,
}

/// Project language configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectLanguages {
    pub source: String,
    pub targets: Vec<String>,
}

/// Project team assignments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTeam {
    pub editor: String,
    pub translators: HashMap<String, String>, // language -> user_id
    pub reviewers: HashMap<String, String>,   // language -> user_id
    pub contributors: Option<Vec<String>>,
}

/// Project settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub auto_save_interval: u32,
    pub quality_threshold: f32,
    pub require_review: bool,
    pub export_on_approval: bool,
    pub git_strategy: Option<String>,
}

/// Additional project metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadataExtra {
    pub estimated_word_count: Option<u32>,
    pub estimated_completion_date: Option<String>,
    pub budget: Option<Budget>,
    pub client_info: Option<ClientInfo>,
}

/// Budget information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub currency: String,
    pub amount: f64,
}

/// Client information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub contact: String,
}

/// Chapter data structure from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterData {
    pub chapter: ChapterMetadata,
    pub units: Vec<TranslationUnit>,
    pub todos: Vec<Todo>,
    pub comments: Vec<Comment>,
}

/// Chapter metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterMetadata {
    pub number: u32,
    pub slug: String,
    pub status: ChapterStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub git_branch: Option<String>,
    pub last_git_commit: Option<String>,
    pub title: HashMap<String, String>, // language -> title
    pub metadata: ChapterMetadataExtra,
}

/// Chapter status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChapterStatus {
    Draft,
    InTranslation,
    InReview,
    Approved,
    Published,
}

/// Additional chapter metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterMetadataExtra {
    pub word_count: HashMap<String, u32>, // language -> word count
    pub difficulty: DifficultyLevel,
    pub estimated_translation_time: HashMap<String, u32>, // language -> hours
    pub requires_screenshots: bool,
    pub screenshot_count: u32,
    pub last_reviewed: HashMap<String, DateTime<Utc>>, // language -> date
}

/// Difficulty level for translation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DifficultyLevel {
    Beginner,
    Intermediate,
    Advanced,
    Expert,
}

/// Translation unit - core structure for paragraph-level translations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationUnit {
    pub id: String,
    pub order: u32,
    pub source_language: String,
    pub source_text: String,
    pub context: Option<String>,
    pub complexity: ComplexityLevel,
    pub requires_review: bool,
    pub unit_type: UnitType,
    pub translations: HashMap<String, TranslationVersion>,
    pub todos: Vec<Todo>,
    pub comments: Vec<Comment>,
    pub notes: Vec<TranslationNote>,
}

/// Complexity level for translation units
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComplexityLevel {
    Low,
    Medium,
    High,
    Expert,
}

/// Type of translation unit
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitType {
    Paragraph,
    Heading,
    ListItem,
    Code,
    Caption,
}

/// Translation version with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationVersion {
    pub text: String,
    pub translator: String,
    pub status: TranslationUnitStatus,
    pub quality_score: Option<f32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub reviewer: Option<String>,
    pub revision_count: u32,
    pub metadata: TranslationMetadata,
}

/// Status of a translation unit
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationUnitStatus {
    Draft,
    InProgress,
    Completed,
    UnderReview,
    Approved,
    Rejected,
}

/// Metadata for a translation version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationMetadata {
    pub terminology_verified: bool,
    pub style_guide_compliant: bool,
    pub review_notes: Option<String>,
    pub translation_method: TranslationMethod,
    pub confidence_score: f32,
}

/// Method used for translation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationMethod {
    Human,
    AiAssisted,
    Machine,
}

/// Todo/task item with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub created_by: String,
    pub assigned_to: Option<String>,
    pub priority: Priority,
    pub status: TodoStatus,
    pub todo_type: TodoType,
    pub context: TodoContext,
    pub created_at: DateTime<Utc>,
    pub due_date: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution: Option<String>,
    pub metadata: Option<TodoMetadata>,
}

/// Priority level for todos
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// Status of a todo
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Open,
    InProgress,
    Completed,
    Cancelled,
}

/// Type of todo
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TodoType {
    Translation,
    Review,
    Terminology,
    Revision,
    Screenshot,
    Formatting,
    Research,
}

/// Context for a todo
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TodoContext {
    #[serde(rename = "project")]
    Project,
    #[serde(rename = "chapter")]
    Chapter,
    #[serde(rename = "paragraph")]
    Paragraph { unit_id: String },
    #[serde(rename = "translation")]
    Translation { unit_id: String, language: String },
}

/// Additional todo metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoMetadata {
    pub estimated_hours: Option<f32>,
    pub actual_hours: Option<f32>,
    pub progress_percent: Option<u32>,
    pub dependencies: Option<Vec<String>>,
}

/// Comment with threading support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub author: String,
    pub content: String,
    pub comment_type: CommentType,
    pub context: CommentContext,
    pub created_at: DateTime<Utc>,
    pub resolved: bool,
    pub thread_id: Option<String>,
    pub replies: Vec<CommentReply>,
}

/// Type of comment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CommentType {
    Suggestion,
    Question,
    Approval,
    Issue,
    Context,
    Terminology,
}

/// Context for a comment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CommentContext {
    #[serde(rename = "translation")]
    Translation { paragraph: String, language: String },
    #[serde(rename = "chapter")]
    Chapter,
    #[serde(rename = "project")]
    Project,
}

/// Reply to a comment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentReply {
    pub author: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub reply_to: Option<String>,
}

/// Translation note for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationNote {
    pub id: String,
    pub author: String,
    pub content: String,
    pub note_type: NoteType,
    pub created_at: DateTime<Utc>,
    pub language: String,
    pub visibility: NoteVisibility,
}

/// Type of translation note
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteType {
    Terminology,
    Grammar,
    Context,
    Cultural,
    Technical,
}

/// Visibility level for notes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteVisibility {
    Public,
    Team,
    Private,
}

/// Conversion utilities between Git models and TOML data structures
pub mod conversions {
    use super::*;
    use crate::git_integration::toml_data;
    
    /// Utility functions for converting between Git workflow models and TOML storage format
    /// This module provides helper functions for transforming data between the runtime
    /// Git models and the persistent TOML structures.
    
    pub fn convert_git_todo_context_to_toml(context: &TodoContext) -> Option<toml_data::TodoContext> {
        match context {
            TodoContext::Paragraph { unit_id } => {
                Some(toml_data::TodoContext::Paragraph { 
                    paragraph: unit_id.clone() 
                })
            }
            TodoContext::Translation { unit_id, language } => {
                Some(toml_data::TodoContext::Translation { 
                    translation: toml_data::TranslationContext {
                        paragraph: unit_id.clone(),
                        language: language.clone(),
                    }
                })
            }
            _ => None, // Project and Chapter contexts don't convert to unit contexts
        }
    }
    
    pub fn convert_toml_todo_context_to_git(context: &toml_data::TodoContext) -> TodoContext {
        match context {
            toml_data::TodoContext::Paragraph { paragraph } => {
                TodoContext::Paragraph { unit_id: paragraph.clone() }
            }
            toml_data::TodoContext::Translation { translation } => {
                TodoContext::Translation { 
                    unit_id: translation.paragraph.clone(), 
                    language: translation.language.clone() 
                }
            }
        }
    }
}

/// Integration helpers for working with TOML data in Git workflows
pub mod toml_integration {
    use super::*;
    use crate::git_integration::toml_io::TomlFileManager;
    use std::path::Path;

    /// Git-aware TOML data manager that combines file operations with Git integration
    #[derive(Debug, Clone)]
    pub struct GitTomlManager {
        pub toml_manager: TomlFileManager,
        pub repo_path: std::path::PathBuf,
    }

    impl GitTomlManager {
        /// Create a new Git-TOML manager
        pub fn new<P: AsRef<Path>>(repo_path: P) -> Self {
            let path = repo_path.as_ref().to_path_buf();
            let toml_manager = TomlFileManager::new(&path);
            
            Self {
                toml_manager,
                repo_path: path,
            }
        }

        /// Initialize the Git repository with TOML directory structure
        pub fn init_git_toml_structure(&self) -> Result<(), Box<dyn std::error::Error>> {
            // Initialize TOML directories
            self.toml_manager.init_directories()?;

            // Create .gitignore for generated files
            let gitignore_content = r#"# Generated files (not tracked in Git)
/generated/
*.tmp
*.backup.*

# IDE and editor files
.vscode/
.idea/
*.swp
*.swo
*~

# OS files
.DS_Store
Thumbs.db
"#;
            std::fs::write(self.repo_path.join(".gitignore"), gitignore_content)?;

            // Create README for the content structure
            let readme_content = r#"# Translation Project Content

This directory contains the source-of-truth TOML files for the translation project.

## Structure

- `project.toml` - Project metadata, settings, and project-level todos
- `chapters/` - Chapter-specific TOML files with translation units, todos, and comments
- `assets/` - Static assets like screenshots and fragments

## Important Notes

- These TOML files are the authoritative source for all translation data
- Markdown files in `/generated/` are created on-demand for editing
- Always use the application interface to modify these files
- Manual edits may be overwritten by the application

## Git Workflow

- Each translation session creates a feature branch
- Changes are automatically committed to TOML files
- Pull requests are used for translation review
- Approved translations are merged to the main branch
"#;
    
            std::fs::write(self.repo_path.join("content").join("README.md"), readme_content)?;

            Ok(())
        }

        /// Load project data with Git context
        pub fn load_project_with_git_info(&self) -> Result<(TomlProjectData, GitProjectInfo), Box<dyn std::error::Error>> {
            let project_data = self.toml_manager.read_project()?;
            
            // Get Git information
            let git_info = GitProjectInfo {
                current_branch: self.get_current_branch()?,
                last_commit: self.get_last_commit()?,
                is_dirty: self.is_working_directory_dirty()?,
                active_branches: self.get_active_translation_branches()?,
            };

            Ok((project_data, git_info))
        }

        /// Load chapter data with Git context
        pub fn load_chapter_with_git_info(&self, chapter_number: u32, chapter_slug: &str) -> Result<(TomlChapterData, GitChapterInfo), Box<dyn std::error::Error>> {
            let chapter_data = self.toml_manager.read_chapter(chapter_number, chapter_slug)?;
            
            // Get Git information specific to this chapter
            let git_info = GitChapterInfo {
                last_modified: self.get_file_last_modified(&self.toml_manager.chapter_toml_path(chapter_number, chapter_slug))?,
                active_sessions: self.get_active_sessions_for_chapter(chapter_slug)?,
                pending_reviews: self.get_pending_reviews_for_chapter(chapter_slug)?,
            };

            Ok((chapter_data, git_info))
        }

        // Git integration helper methods
        fn get_current_branch(&self) -> Result<String, git2::Error> {
            let repo = git2::Repository::open(&self.repo_path)?;
            let head = repo.head()?;
            Ok(head.shorthand().unwrap_or("unknown").to_string())
        }

        fn get_last_commit(&self) -> Result<String, git2::Error> {
            let repo = git2::Repository::open(&self.repo_path)?;
            let head = repo.head()?;
            let commit = head.peel_to_commit()?;
            Ok(commit.id().to_string())
        }

        fn is_working_directory_dirty(&self) -> Result<bool, git2::Error> {
            let repo = git2::Repository::open(&self.repo_path)?;
            let statuses = repo.statuses(None)?;
            Ok(!statuses.is_empty())
        }

        fn get_active_translation_branches(&self) -> Result<Vec<String>, git2::Error> {
            let repo = git2::Repository::open(&self.repo_path)?;
            let branches = repo.branches(Some(git2::BranchType::Local))?;
            
            let mut translation_branches = Vec::new();
            for branch in branches {
                let (branch, _) = branch?;
                if let Some(name) = branch.name()? {
                    if name.starts_with("translate/") {
                        translation_branches.push(name.to_string());
                    }
                }
            }
            
            Ok(translation_branches)
        }

        fn get_file_last_modified(&self, file_path: &Path) -> Result<DateTime<Utc>, Box<dyn std::error::Error>> {
            let metadata = std::fs::metadata(file_path)?;
            let modified = metadata.modified()?;
            Ok(DateTime::from(modified))
        }

        fn get_active_sessions_for_chapter(&self, _chapter_slug: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
            // TODO: Implement session tracking
            Ok(Vec::new())
        }

        fn get_pending_reviews_for_chapter(&self, _chapter_slug: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
            // TODO: Implement review tracking
            Ok(Vec::new())
        }
    }

    /// Git context information for projects
    #[derive(Debug, Clone)]
    pub struct GitProjectInfo {
        pub current_branch: String,
        pub last_commit: String,
        pub is_dirty: bool,
        pub active_branches: Vec<String>,
    }

    /// Git context information for chapters
    #[derive(Debug, Clone)]  
    pub struct GitChapterInfo {
        pub last_modified: DateTime<Utc>,
        pub active_sessions: Vec<String>,
        pub pending_reviews: Vec<String>,
    }
}