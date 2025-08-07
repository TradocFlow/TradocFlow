//! TOML Data Structures
//! 
//! Core TOML data structures for the Git-Integrated Translation Management System.
//! These structures match the TOML schema specifications from the PRD.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Root project data structure matching project.toml schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectData {
    pub project: ProjectMetadata,
    #[serde(default)]
    pub todos: Vec<ProjectTodo>,
}

/// Project metadata section
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ProjectMetadataExtra>,
}

/// Project status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ProjectStatus {
    #[default]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contributors: Option<Vec<String>>,
}

/// Project settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    #[serde(default = "default_auto_save_interval")]
    pub auto_save_interval: u32, // seconds
    #[serde(default = "default_quality_threshold")]
    pub quality_threshold: f32,  // 0-10 scale
    #[serde(default = "default_require_review")]
    pub require_review: bool,
    #[serde(default)]
    pub export_on_approval: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_strategy: Option<String>,
}

/// Additional project metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadataExtra {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_word_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_completion_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
    #[serde(skip_serializing_if = "Option::is_none")]
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

/// Project-level todo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTodo {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<String>,
    pub priority: Priority,
    pub status: TodoStatus,
    #[serde(rename = "type")]
    pub todo_type: TodoType,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<TodoMetadata>,
}

/// Root chapter data structure matching XX_chapter-name.toml schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterData {
    pub chapter: ChapterMetadata,
    #[serde(default)]
    pub units: Vec<TranslationUnit>,
    #[serde(default)]
    pub todos: Vec<ChapterTodo>,
}

/// Chapter metadata section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterMetadata {
    pub number: u32,
    pub slug: String,
    pub status: ChapterStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_git_commit: Option<String>,
    pub title: HashMap<String, String>, // language -> title
    pub metadata: ChapterMetadataExtra,
}

/// Chapter status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ChapterStatus {
    #[default]
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
    #[serde(default)]
    pub requires_screenshots: bool,
    #[serde(default)]
    pub screenshot_count: u32,
    #[serde(default)]
    pub last_reviewed: HashMap<String, DateTime<Utc>>, // language -> date
}

/// Difficulty level enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum DifficultyLevel {
    Beginner,
    #[default]
    Intermediate,
    Advanced,
    Expert,
}

/// Chapter-level todo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterTodo {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<String>,
    pub priority: Priority,
    pub status: TodoStatus,
    #[serde(rename = "type")]
    pub todo_type: TodoType,
    pub context: String, // Always "chapter" for chapter todos
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
}

/// Translation unit - core structure for paragraph-level translations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationUnit {
    pub id: String,
    pub order: u32,
    pub source_language: String,
    pub source_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    pub complexity: ComplexityLevel,
    #[serde(default = "default_requires_review")]
    pub requires_review: bool,
    #[serde(default = "default_unit_type")]
    pub unit_type: UnitType,
    #[serde(default)]
    pub translations: HashMap<String, TranslationVersion>,
    #[serde(default)]
    pub todos: Vec<UnitTodo>,
    #[serde(default)]
    pub comments: Vec<Comment>,
    #[serde(default)]
    pub notes: Vec<TranslationNote>,
}

/// Complexity level for translation units
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ComplexityLevel {
    Low,
    #[default]
    Medium,
    High,
    Expert,
}

/// Type of translation unit
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum UnitType {
    Paragraph,
    Heading,
    ListItem,
    Code,
    Caption,
}

/// Translation version with complete metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationVersion {
    pub text: String,
    pub translator: String,
    pub status: TranslationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_score: Option<f32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewer: Option<String>,
    #[serde(default)]
    pub revision_count: u32,
    pub metadata: TranslationMetadata,
}

/// Status of a translation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TranslationStatus {
    #[default]
    Draft,
    InProgress,
    Completed,
    UnderReview,
    Approved,
    Rejected,
    Archived,
}

/// Metadata for a translation version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationMetadata {
    #[serde(default)]
    pub terminology_verified: bool,
    #[serde(default)]
    pub style_guide_compliant: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_notes: Option<String>,
    #[serde(default = "default_translation_method")]
    pub translation_method: TranslationMethod,
    #[serde(default = "default_confidence_score")]
    pub confidence_score: f32,
}

/// Method used for translation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TranslationMethod {
    #[default]
    Human,
    AiAssisted,
    Machine,
}

/// Unit-level todo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitTodo {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<String>,
    pub priority: Priority,
    pub status: TodoStatus,
    #[serde(rename = "type")]
    pub todo_type: TodoType,
    pub context: TodoContext,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
}

/// Comment with threading support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub author: String,
    pub content: String,
    #[serde(rename = "type")]
    pub comment_type: CommentType,
    pub context: CommentContext,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub resolved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(default)]
    pub replies: Vec<CommentReply>,
}

/// Translation note for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationNote {
    pub id: String,
    pub author: String,
    pub content: String,
    #[serde(rename = "type")]
    pub note_type: NoteType,
    pub created_at: DateTime<Utc>,
    pub language: String,
    #[serde(default = "default_note_visibility")]
    pub visibility: NoteVisibility,
}

/// Priority level enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Priority {
    Low,
    #[default]
    Medium,
    High,
    Critical,
}

/// Todo status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TodoStatus {
    #[default]
    Open,
    InProgress,
    Completed,
    Cancelled,
}

/// Todo type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TodoType {
    #[default]
    Translation,
    Review,
    Terminology,
    Revision,
    Screenshot,
    Formatting,
    Research,
}

/// Context for todos
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TodoContext {
    Paragraph { paragraph: String },
    Translation { translation: TranslationContext },
}

/// Translation context for todos
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationContext {
    pub paragraph: String,
    pub language: String,
}

/// Comment type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum CommentType {
    Suggestion,
    Question,
    Approval,
    Issue,
    #[default]
    Context,
    Terminology,
}

/// Context for comments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CommentContext {
    Translation { translation: TranslationContext },
    Chapter,
    Project,
}

/// Reply to a comment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentReply {
    pub author: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// Note type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum NoteType {
    Terminology,
    Grammar,
    #[default]
    Context,
    Cultural,
    Technical,
}

/// Note visibility enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum NoteVisibility {
    Public,
    Team,
    Private,
}

/// Todo metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_hours: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_hours: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_percent: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<String>>,
}

// Default value functions
fn default_auto_save_interval() -> u32 { 300 }
fn default_quality_threshold() -> f32 { 8.0 }
fn default_require_review() -> bool { true }
fn default_requires_review() -> bool { true }
fn default_unit_type() -> UnitType { UnitType::Paragraph }
fn default_translation_method() -> TranslationMethod { TranslationMethod::Human }
fn default_confidence_score() -> f32 { 0.95 }
fn default_note_visibility() -> NoteVisibility { NoteVisibility::Team }












/// Error types for TOML data operations
#[derive(Debug, thiserror::Error)]
pub enum TomlDataError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] toml::ser::Error),
    
    #[error("Deserialization error: {0}")]
    Deserialization(#[from] toml::de::Error),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("Invalid enum value: {0}")]
    InvalidEnum(String),
}

pub type Result<T> = std::result::Result<T, TomlDataError>;

impl ProjectData {
    /// Create a new project with minimal required fields
    pub fn new(
        id: String,
        name: String,
        description: String,
        source_language: String,
        target_languages: Vec<String>,
        editor: String,
    ) -> Self {
        let now = Utc::now();
        
        Self {
            project: ProjectMetadata {
                id,
                name,
                description,
                version: "1.0.0".to_string(),
                created_at: now,
                updated_at: now,
                status: ProjectStatus::Active,
                languages: ProjectLanguages {
                    source: source_language,
                    targets: target_languages,
                },
                team: ProjectTeam {
                    editor,
                    translators: HashMap::new(),
                    reviewers: HashMap::new(),
                    contributors: None,
                },
                settings: ProjectSettings {
                    auto_save_interval: default_auto_save_interval(),
                    quality_threshold: default_quality_threshold(),
                    require_review: default_require_review(),
                    export_on_approval: false,
                    git_strategy: Some("feature_branch".to_string()),
                },
                metadata: None,
            },
            todos: Vec::new(),
        }
    }
    
    /// Validate the project data structure
    pub fn validate(&self) -> Result<()> {
        if self.project.id.is_empty() {
            return Err(TomlDataError::MissingField("project.id".to_string()));
        }
        
        if self.project.name.is_empty() {
            return Err(TomlDataError::MissingField("project.name".to_string()));
        }
        
        if self.project.languages.source.is_empty() {
            return Err(TomlDataError::MissingField("project.languages.source".to_string()));
        }
        
        if self.project.languages.targets.is_empty() {
            return Err(TomlDataError::Validation("At least one target language is required".to_string()));
        }
        
        if self.project.team.editor.is_empty() {
            return Err(TomlDataError::MissingField("project.team.editor".to_string()));
        }
        
        // Validate quality threshold range
        if self.project.settings.quality_threshold < 0.0 || self.project.settings.quality_threshold > 10.0 {
            return Err(TomlDataError::Validation("Quality threshold must be between 0.0 and 10.0".to_string()));
        }
        
        Ok(())
    }
}

impl ChapterData {
    /// Create a new chapter with minimal required fields
    pub fn new(
        number: u32,
        slug: String,
        titles: HashMap<String, String>,
        _source_language: String,
    ) -> Self {
        let now = Utc::now();
        
        Self {
            chapter: ChapterMetadata {
                number,
                slug,
                status: ChapterStatus::Draft,
                created_at: now,
                updated_at: now,
                git_branch: None,
                last_git_commit: None,
                title: titles,
                metadata: ChapterMetadataExtra {
                    word_count: HashMap::new(),
                    difficulty: DifficultyLevel::Intermediate,
                    estimated_translation_time: HashMap::new(),
                    requires_screenshots: false,
                    screenshot_count: 0,
                    last_reviewed: HashMap::new(),
                },
            },
            units: Vec::new(),
            todos: Vec::new(),
        }
    }
    
    /// Validate the chapter data structure
    pub fn validate(&self) -> Result<()> {
        if self.chapter.slug.is_empty() {
            return Err(TomlDataError::MissingField("chapter.slug".to_string()));
        }
        
        if self.chapter.title.is_empty() {
            return Err(TomlDataError::MissingField("chapter.title".to_string()));
        }
        
        // Validate translation units
        for (index, unit) in self.units.iter().enumerate() {
            if unit.id.is_empty() {
                return Err(TomlDataError::MissingField(format!("units[{index}].id")));
            }
            
            if unit.source_text.is_empty() {
                return Err(TomlDataError::MissingField(format!("units[{index}].source_text")));
            }
            
            // Validate translation versions
            for (lang, translation) in &unit.translations {
                if translation.text.is_empty() {
                    return Err(TomlDataError::MissingField(format!("units[{index}].translations.{lang}.text")));
                }
                
                if translation.translator.is_empty() {
                    return Err(TomlDataError::MissingField(format!("units[{index}].translations.{lang}.translator")));
                }
                
                // Validate quality score range
                if let Some(score) = translation.quality_score {
                    if !(0.0..=10.0).contains(&score) {
                        return Err(TomlDataError::Validation(format!("Quality score for units[{index}].translations.{lang} must be between 0.0 and 10.0")));
                    }
                }
                
                // Validate confidence score range
                if translation.metadata.confidence_score < 0.0 || translation.metadata.confidence_score > 1.0 {
                    return Err(TomlDataError::Validation(format!("Confidence score for units[{index}].translations.{lang} must be between 0.0 and 1.0")));
                }
            }
        }
        
        Ok(())
    }
    
    /// Add a new translation unit to the chapter
    pub fn add_unit(&mut self, unit: TranslationUnit) {
        self.units.push(unit);
        self.chapter.updated_at = Utc::now();
    }
    
    /// Get unit by ID
    pub fn get_unit(&self, unit_id: &str) -> Option<&TranslationUnit> {
        self.units.iter().find(|unit| unit.id == unit_id)
    }
    
    /// Get mutable unit by ID
    pub fn get_unit_mut(&mut self, unit_id: &str) -> Option<&mut TranslationUnit> {
        self.units.iter_mut().find(|unit| unit.id == unit_id)
    }
    
    /// Update word count for a language
    pub fn update_word_count(&mut self, language: &str, count: u32) {
        self.chapter.metadata.word_count.insert(language.to_string(), count);
        self.chapter.updated_at = Utc::now();
    }
}

impl TranslationUnit {
    /// Create a new translation unit
    pub fn new(
        id: String,
        order: u32,
        source_language: String,
        source_text: String,
        complexity: ComplexityLevel,
    ) -> Self {
        Self {
            id,
            order,
            source_language,
            source_text,
            context: None,
            complexity,
            requires_review: default_requires_review(),
            unit_type: default_unit_type(),
            translations: HashMap::new(),
            todos: Vec::new(),
            comments: Vec::new(),
            notes: Vec::new(),
        }
    }
    
    /// Add a translation for a specific language
    pub fn add_translation(&mut self, language: String, translation: TranslationVersion) {
        self.translations.insert(language, translation);
    }
    
    /// Get translation for a specific language
    pub fn get_translation(&self, language: &str) -> Option<&TranslationVersion> {
        self.translations.get(language)
    }
    
    /// Get mutable translation for a specific language
    pub fn get_translation_mut(&mut self, language: &str) -> Option<&mut TranslationVersion> {
        self.translations.get_mut(language)
    }
}

impl TranslationVersion {
    /// Create a new translation version
    pub fn new(
        text: String,
        translator: String,
        status: TranslationStatus,
    ) -> Self {
        let now = Utc::now();
        
        Self {
            text,
            translator,
            status,
            quality_score: None,
            created_at: now,
            updated_at: now,
            reviewed_at: None,
            reviewer: None,
            revision_count: 0,
            metadata: TranslationMetadata {
                terminology_verified: false,
                style_guide_compliant: false,
                review_notes: None,
                translation_method: default_translation_method(),
                confidence_score: default_confidence_score(),
            },
        }
    }
    
    /// Update the translation text and increment revision count
    pub fn update_text(&mut self, new_text: String) {
        self.text = new_text;
        self.updated_at = Utc::now();
        self.revision_count += 1;
    }
    
    /// Mark as reviewed
    pub fn mark_reviewed(&mut self, reviewer: String, quality_score: Option<f32>, notes: Option<String>) {
        self.reviewer = Some(reviewer);
        self.reviewed_at = Some(Utc::now());
        self.quality_score = quality_score;
        self.metadata.review_notes = notes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_project_data_creation() {
        let project = ProjectData::new(
            "test-id".to_string(),
            "Test Project".to_string(),
            "A test project".to_string(),
            "en".to_string(),
            vec!["de".to_string(), "fr".to_string()],
            "editor1".to_string(),
        );
        
        assert_eq!(project.project.id, "test-id");
        assert_eq!(project.project.name, "Test Project");
        assert_eq!(project.project.languages.source, "en");
        assert_eq!(project.project.languages.targets.len(), 2);
        assert_eq!(project.project.team.editor, "editor1");
        assert!(project.todos.is_empty());
    }
    
    #[test]
    fn test_project_validation() {
        let project = ProjectData::new(
            "test-id".to_string(),
            "Test Project".to_string(),
            "A test project".to_string(),
            "en".to_string(),
            vec!["de".to_string()],
            "editor1".to_string(),
        );
        
        assert!(project.validate().is_ok());
        
        // Test empty name validation
        let mut invalid_project = project.clone();
        invalid_project.project.name = String::new();
        assert!(invalid_project.validate().is_err());
    }
    
    #[test]
    fn test_chapter_data_creation() {
        let mut titles = HashMap::new();
        titles.insert("en".to_string(), "Introduction".to_string());
        titles.insert("de".to_string(), "Einführung".to_string());
        
        let chapter = ChapterData::new(
            1,
            "introduction".to_string(),
            titles,
            "en".to_string(),
        );
        
        assert_eq!(chapter.chapter.number, 1);
        assert_eq!(chapter.chapter.slug, "introduction");
        assert_eq!(chapter.chapter.title.len(), 2);
        assert!(chapter.units.is_empty());
    }
    
    #[test]
    fn test_translation_unit_operations() {
        let mut unit = TranslationUnit::new(
            "unit-1".to_string(),
            1,
            "en".to_string(),
            "Hello world".to_string(),
            ComplexityLevel::Low,
        );
        
        let translation = TranslationVersion::new(
            "Hallo Welt".to_string(),
            "translator1".to_string(),
            TranslationStatus::Completed,
        );
        
        unit.add_translation("de".to_string(), translation);
        
        assert!(unit.get_translation("de").is_some());
        assert_eq!(unit.get_translation("de").unwrap().text, "Hallo Welt");
        assert!(unit.get_translation("fr").is_none());
    }
    
    #[test]
    fn test_translation_version_updates() {
        let mut translation = TranslationVersion::new(
            "Original text".to_string(),
            "translator1".to_string(),
            TranslationStatus::Draft,
        );
        
        let original_revision = translation.revision_count;
        translation.update_text("Updated text".to_string());
        
        assert_eq!(translation.text, "Updated text");
        assert_eq!(translation.revision_count, original_revision + 1);
        
        translation.mark_reviewed(
            "reviewer1".to_string(),
            Some(9.0),
            Some("Great work!".to_string()),
        );
        
        assert_eq!(translation.reviewer, Some("reviewer1".to_string()));
        assert_eq!(translation.quality_score, Some(9.0));
        assert!(translation.reviewed_at.is_some());
    }
}