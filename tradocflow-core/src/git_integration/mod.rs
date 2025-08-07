//! Git Integration Module
//! 
//! Provides Git-first integration for translation management with transparent
//! Git operations abstracted behind domain-specific interfaces.

pub mod workflow_manager;
pub mod models;
pub mod session_manager;
pub mod commit_builder;
pub mod task_manager;
pub mod comment_system;
pub mod kanban_sync;
pub mod toml_data;
pub mod toml_io;
pub mod diff_tools;
#[cfg(test)]
pub mod toml_tests;

pub use workflow_manager::{GitWorkflowManager, TranslationBranchInfo};
pub use models::{
    WorkSession, ReviewRequest, TranslationDiff, TranslationChange, 
    ChangeType, ProjectData, ProjectMetadata, ChapterData, ChapterMetadata,
    TranslationUnit, Todo, Comment, TranslationNote,
    // Re-export TOML data structures
    TomlProjectData, TomlChapterData, TomlProjectMetadata, TomlChapterMetadata,
    TomlTranslationUnit, TomlTranslationVersion, TomlProjectTodo, TomlChapterTodo,
    TomlUnitTodo, TomlComment, TomlTranslationNote,
    TomlProjectStatus, TomlChapterStatus, TomlTranslationStatus, TomlPriority, 
    TomlTodoStatus, TomlTodoType, TomlCommentType, TomlComplexityLevel, 
    TomlDifficultyLevel, TomlUnitType, TomlTranslationMethod, TomlNoteType, 
    TomlNoteVisibility,
    // Conversion and integration modules
    conversions, toml_integration,
};
pub use toml_data::{
    ProjectData as TomlProjectDataDirect, ChapterData as TomlChapterDataDirect,
    TomlDataError, Result as TomlResult,
};
pub use toml_io::{
    TomlFileManager, ValidationReport, ProjectStatistics, utils as toml_utils,
};
pub use session_manager::{
    SessionManager, SessionInfo, EnhancedSessionInfo, ActiveSession,
    SessionManagerConfig, SessionUpdateResult, SessionSaveResult,
    ProductivityMetrics, SessionRecoveryInfo, SessionMetadata, EditorPreferences,
};
pub use commit_builder::{CommitMessageBuilder, CommitTemplates, CommitType};
pub use task_manager::{
    TaskManager, CreateTodoRequest, UpdateTodoRequest, CreateCommentRequest,
    CreateCommentReplyRequest, TaskFilter, TaskNotification, TaskEventType
};
pub use comment_system::{
    CommentSystem, CommentThread, ThreadedComment, ThreadContext, ThreadStatus,
    ThreadPriority, CreateThreadRequest, UpdateCommentRequest, UpdateThreadRequest,
    CommentFilter, CommentSearchResults, CommentNotification, CommentEventType,
    CommentStatistics, CommentPosition, CommentAttachment, CommentMetadata,
    TextRange, ExternalReference, ThreadMetadata, CommentSource, CommentEdit,
    CommentSentiment
};
pub use kanban_sync::{
    KanbanGitSync, WorkflowMapping, WorkflowType, WorkflowMetadata, EventSubscriber,
    SyncEventType, SyncEvent, EventSource, SyncEventMetadata, CreateTranslationWorkflowRequest,
    WorkflowProgress, ProgressBottleneck, BottleneckType, SyncReport, SyncPerformanceMetrics
};
pub use diff_tools::{
    GitDiffTools, DetailedTranslationDiff, TranslationUnitDiff, TranslationHistoryEntry,
    QualityTrends, BranchComparisonReport, DiffOptions, TextDiff, QualityChange,
    StatusChange, MetadataChange, TranslationDiffStats, BranchComparisonStats,
};

use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use chrono::Utc;

/// Git integration error types
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Repository error: {0}")]
    Repository(#[from] git2::Error),
    #[error("Branch not found: {0}")]
    BranchNotFound(String),
    #[error("Merge conflict in {file}: {details}")]
    MergeConflict { file: String, details: String },
    #[error("Invalid Git operation: {0}")]
    InvalidOperation(String),
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("Remote operation failed: {0}")]
    RemoteError(String),
    #[error("Lock timeout: {0}")]
    LockTimeout(String),
}

// GitError conversion is handled in lib.rs

/// Configuration for Git repository setup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    pub repository_path: String,
    pub default_branch: String,
    pub auto_push: bool,
    pub remote_name: String,
    pub commit_signing: bool,
    pub branch_protection: bool,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            repository_path: "./project_repo".to_string(),
            default_branch: "main".to_string(),
            auto_push: true,
            remote_name: "origin".to_string(),
            commit_signing: false,
            branch_protection: true,
        }
    }
}

/// Initialize a new Git repository with the proper structure for translation projects
pub async fn initialize_translation_repository(
    path: &Path,
    project_name: &str,
    config: &GitConfig,
) -> Result<git2::Repository> {
    use std::fs;
    
    // Create directory structure
    fs::create_dir_all(path)?;
    fs::create_dir_all(path.join(".tradocument"))?;
    fs::create_dir_all(path.join("content/chapters"))?;
    fs::create_dir_all(path.join("content/assets/screenshots"))?;
    fs::create_dir_all(path.join("generated/markdown"))?;
    fs::create_dir_all(path.join("generated/exports"))?;
    fs::create_dir_all(path.join("docs"))?;

    // Initialize Git repository
    let repo = git2::Repository::init(path)
        .map_err(GitError::from)?;
    
    // Create .gitignore
    let gitignore_content = r#"
# Generated files
generated/
exports/
*.tmp
*.log

# IDE and editor files
.vscode/
.idea/
*.swp
*.swo
*~

# OS files
.DS_Store
Thumbs.db

# Build artifacts
target/
*.lock
"#;
    
    fs::write(path.join(".gitignore"), gitignore_content)?;
    
    // Create initial configuration
    let tradocument_config = format!(r#"
[project]
name = "{}"
version = "1.0.0"
created_at = "{}"

[git]
default_branch = "{}"
auto_push = {}
branch_protection = {}

[workflow]
require_review = true
auto_save_interval = 300
quality_threshold = 8.0
"#, 
        project_name,
        Utc::now().to_rfc3339(),
        config.default_branch,
        config.auto_push,
        config.branch_protection
    );
    
    fs::write(path.join(".tradocument/config.toml"), tradocument_config)?;
    
    // Create README
    let readme_content = format!(r#"# {project_name}

Git-integrated translation management project.

## Structure

- `content/` - Source TOML files (version controlled)
- `generated/` - Generated Markdown and exports (gitignored) 
- `.tradocument/` - Configuration and templates
- `docs/` - Project documentation

## Workflow

1. Editors create tasks and assign translations
2. Translators work in feature branches
3. Reviewers approve via pull requests
4. Approved translations merge to main

See [TRANSLATION_GUIDE.md](docs/TRANSLATION_GUIDE.md) for details.
"#);

    fs::write(path.join("README.md"), readme_content)?;
    
    // Create initial commit
    let mut index = repo.index().map_err(GitError::from)?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .map_err(GitError::from)?;
    index.write().map_err(GitError::from)?;
    
    let signature = git2::Signature::now("Tradocument System", "system@tradocument.local")
        .map_err(GitError::from)?;
    let tree_id = index.write_tree().map_err(GitError::from)?;
    {
        let tree = repo.find_tree(tree_id).map_err(GitError::from)?;
        
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &format!("Initial repository setup for {project_name}"),
            &tree,
            &[],
        ).map_err(GitError::from)?;
    }
    
    Ok(repo)
}