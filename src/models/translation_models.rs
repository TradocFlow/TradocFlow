use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tradocflow_translation_memory::TranslationUnit;
use tradocflow_translation_memory::TranslationMetadata;

/// Translation project model with language configuration and team members
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationProject {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub source_language: String,
    pub target_languages: Vec<String>,
    pub project_path: PathBuf,
    pub team_members: Vec<TeamMember>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub settings: ProjectSettings,
}

impl TranslationProject {
    /// Create a new translation project with validation
    pub fn new(
        name: String,
        description: Option<String>,
        source_language: String,
        target_languages: Vec<String>,
        project_path: PathBuf,
    ) -> Result<Self, ValidationError> {
        // Validate project name
        if name.trim().is_empty() {
            return Err(ValidationError::InvalidProjectName("Project name cannot be empty".to_string()));
        }

        // Validate source language
        if source_language.trim().is_empty() {
            return Err(ValidationError::InvalidLanguage("Source language cannot be empty".to_string()));
        }

        // Validate target languages
        if target_languages.is_empty() {
            return Err(ValidationError::InvalidLanguage("At least one target language must be specified".to_string()));
        }

        // Ensure source language is not in target languages
        if target_languages.contains(&source_language) {
            return Err(ValidationError::InvalidLanguage("Source language cannot be a target language".to_string()));
        }

        // Validate project path
        if !project_path.is_absolute() {
            return Err(ValidationError::InvalidPath("Project path must be absolute".to_string()));
        }

        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            name,
            description,
            source_language,
            target_languages,
            project_path,
            team_members: Vec::new(),
            created_at: now,
            updated_at: now,
            settings: ProjectSettings::default(),
        })
    }

    /// Add a team member to the project
    pub fn add_team_member(&mut self, member: TeamMember) -> Result<(), ValidationError> {
        // Check if member already exists
        if self.team_members.iter().any(|m| m.id == member.id) {
            return Err(ValidationError::DuplicateMember("Team member already exists".to_string()));
        }

        // Validate member languages are supported by project
        let all_languages: Vec<String> = std::iter::once(self.source_language.clone())
            .chain(self.target_languages.iter().cloned())
            .collect();

        for lang in &member.languages {
            if !all_languages.contains(lang) {
                return Err(ValidationError::InvalidLanguage(
                    format!("Language '{lang}' is not supported by this project")
                ));
            }
        }

        self.team_members.push(member);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Remove a team member from the project
    pub fn remove_team_member(&mut self, member_id: &str) -> Result<(), ValidationError> {
        let initial_len = self.team_members.len();
        self.team_members.retain(|m| m.id != member_id);
        
        if self.team_members.len() == initial_len {
            return Err(ValidationError::MemberNotFound("Team member not found".to_string()));
        }

        self.updated_at = Utc::now();
        Ok(())
    }

    /// Get team members by role
    pub fn get_members_by_role(&self, role: &UserRole) -> Vec<&TeamMember> {
        self.team_members.iter().filter(|m| &m.role == role).collect()
    }

    /// Get team members assigned to a specific language
    pub fn get_members_by_language(&self, language: &str) -> Vec<&TeamMember> {
        self.team_members.iter().filter(|m| m.languages.contains(&language.to_string())).collect()
    }

    /// Validate the entire project configuration
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Re-validate basic fields
        if self.name.trim().is_empty() {
            return Err(ValidationError::InvalidProjectName("Project name cannot be empty".to_string()));
        }

        if self.source_language.trim().is_empty() {
            return Err(ValidationError::InvalidLanguage("Source language cannot be empty".to_string()));
        }

        if self.target_languages.is_empty() {
            return Err(ValidationError::InvalidLanguage("At least one target language must be specified".to_string()));
        }

        // Validate team member assignments
        for member in &self.team_members {
            member.validate()?;
        }

        Ok(())
    }
}

/// Project-specific settings for translation workflows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub auto_save_interval: std::time::Duration,
    pub translation_memory_threshold: f32,
    pub chunk_size_preference: ChunkSizePreference,
    pub export_settings: ExportSettings,
    pub collaboration_settings: CollaborationSettings,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            auto_save_interval: std::time::Duration::from_secs(30),
            translation_memory_threshold: 0.8,
            chunk_size_preference: ChunkSizePreference::Sentence,
            export_settings: ExportSettings::default(),
            collaboration_settings: CollaborationSettings::default(),
        }
    }
}

/// Team member with role-based permissions and language assignments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: UserRole,
    pub languages: Vec<String>,
    pub permissions: Vec<Permission>,
    pub joined_at: DateTime<Utc>,
}

impl TeamMember {
    /// Create a new team member with validation
    pub fn new(
        id: String,
        name: String,
        email: String,
        role: UserRole,
        languages: Vec<String>,
    ) -> Result<Self, ValidationError> {
        // Validate ID
        if id.trim().is_empty() {
            return Err(ValidationError::InvalidMemberId("Member ID cannot be empty".to_string()));
        }

        // Validate name
        if name.trim().is_empty() {
            return Err(ValidationError::InvalidMemberName("Member name cannot be empty".to_string()));
        }

        // Validate email format (basic validation)
        if !email.contains('@') || email.trim().is_empty() {
            return Err(ValidationError::InvalidEmail("Invalid email format".to_string()));
        }

        // Validate languages
        if languages.is_empty() {
            return Err(ValidationError::InvalidLanguage("At least one language must be assigned".to_string()));
        }

        // Set permissions based on role
        let permissions = Self::get_default_permissions(&role);

        Ok(Self {
            id,
            name,
            email,
            role,
            languages,
            permissions,
            joined_at: Utc::now(),
        })
    }

    /// Get default permissions for a role
    pub fn get_default_permissions(role: &UserRole) -> Vec<Permission> {
        match role {
            UserRole::Admin => vec![
                Permission::EditTranslations,
                Permission::ReviewTranslations,
                Permission::ManageTerminology,
                Permission::ExportDocuments,
                Permission::ManageTeam,
                Permission::ViewAnalytics,
            ],
            UserRole::ProjectManager => vec![
                Permission::ReviewTranslations,
                Permission::ManageTerminology,
                Permission::ExportDocuments,
                Permission::ManageTeam,
                Permission::ViewAnalytics,
            ],
            UserRole::Reviewer => vec![
                Permission::ReviewTranslations,
                Permission::ViewAnalytics,
            ],
            UserRole::Translator => vec![
                Permission::EditTranslations,
            ],
        }
    }

    /// Check if member has a specific permission
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    /// Add a permission to the member
    pub fn add_permission(&mut self, permission: Permission) {
        if !self.permissions.contains(&permission) {
            self.permissions.push(permission);
        }
    }

    /// Remove a permission from the member
    pub fn remove_permission(&mut self, permission: &Permission) {
        self.permissions.retain(|p| p != permission);
    }

    /// Validate the team member
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.id.trim().is_empty() {
            return Err(ValidationError::InvalidMemberId("Member ID cannot be empty".to_string()));
        }

        if self.name.trim().is_empty() {
            return Err(ValidationError::InvalidMemberName("Member name cannot be empty".to_string()));
        }

        if !self.email.contains('@') || self.email.trim().is_empty() {
            return Err(ValidationError::InvalidEmail("Invalid email format".to_string()));
        }

        if self.languages.is_empty() {
            return Err(ValidationError::InvalidLanguage("At least one language must be assigned".to_string()));
        }

        Ok(())
    }
}

/// User roles in the translation system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    ProjectManager,
    Translator,
    Reviewer,
    Admin,
}

impl UserRole {
    /// Get all available roles
    pub fn all() -> Vec<UserRole> {
        vec![
            UserRole::Admin,
            UserRole::ProjectManager,
            UserRole::Reviewer,
            UserRole::Translator,
        ]
    }

    /// Get role description
    pub fn description(&self) -> &'static str {
        match self {
            UserRole::Admin => "Full system access and administration",
            UserRole::ProjectManager => "Project management and team coordination",
            UserRole::Reviewer => "Review and approve translations",
            UserRole::Translator => "Create and edit translations",
        }
    }
}

/// Granular permissions for team members
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Permission {
    EditTranslations,
    ReviewTranslations,
    ManageTerminology,
    ExportDocuments,
    ManageTeam,
    ViewAnalytics,
}

impl Permission {
    /// Get all available permissions
    pub fn all() -> Vec<Permission> {
        vec![
            Permission::EditTranslations,
            Permission::ReviewTranslations,
            Permission::ManageTerminology,
            Permission::ExportDocuments,
            Permission::ManageTeam,
            Permission::ViewAnalytics,
        ]
    }

    /// Get permission description
    pub fn description(&self) -> &'static str {
        match self {
            Permission::EditTranslations => "Create and modify translations",
            Permission::ReviewTranslations => "Review and approve translation changes",
            Permission::ManageTerminology => "Add and modify terminology databases",
            Permission::ExportDocuments => "Export documents to various formats",
            Permission::ManageTeam => "Add, remove, and modify team members",
            Permission::ViewAnalytics => "View project analytics and progress reports",
        }
    }
}

/// Validation errors for translation models
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid project name: {0}")]
    InvalidProjectName(String),
    
    #[error("Invalid language: {0}")]
    InvalidLanguage(String),
    
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("Duplicate member: {0}")]
    DuplicateMember(String),
    
    #[error("Member not found: {0}")]
    MemberNotFound(String),
    
    #[error("Invalid member ID: {0}")]
    InvalidMemberId(String),
    
    #[error("Invalid member name: {0}")]
    InvalidMemberName(String),
    
    #[error("Invalid email: {0}")]
    InvalidEmail(String),
    
    #[error("Invalid translation unit: {0}")]
    InvalidTranslationUnit(String),
    
    #[error("Invalid chunk: {0}")]
    InvalidChunk(String),
    
    #[error("Invalid term: {0}")]
    InvalidTerm(String),
}

// TranslationUnit and TranslationMetadata are now provided by the new translation memory crate
// Use: tradocflow_translation_memory::{TranslationUnit, TranslationMetadata}


/// Chunk metadata for sentence chunking and linking information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub id: Uuid,
    pub original_position: usize,
    pub sentence_boundaries: Vec<usize>,
    pub linked_chunks: Vec<Uuid>,
    pub chunk_type: ChunkType,
    pub processing_notes: Vec<String>,
}

impl ChunkMetadata {
    /// Create new chunk metadata with validation
    pub fn new(
        original_position: usize,
        sentence_boundaries: Vec<usize>,
        chunk_type: ChunkType,
    ) -> Result<Self, ValidationError> {
        // Validate sentence boundaries are sorted
        if !sentence_boundaries.windows(2).all(|w| w[0] <= w[1]) {
            return Err(ValidationError::InvalidChunk("Sentence boundaries must be sorted".to_string()));
        }

        Ok(Self {
            id: Uuid::new_v4(),
            original_position,
            sentence_boundaries,
            linked_chunks: Vec::new(),
            chunk_type,
            processing_notes: Vec::new(),
        })
    }

    /// Link this chunk with another chunk
    pub fn link_with_chunk(&mut self, chunk_id: Uuid) -> Result<(), ValidationError> {
        if chunk_id == self.id {
            return Err(ValidationError::InvalidChunk("Cannot link chunk with itself".to_string()));
        }

        if !self.linked_chunks.contains(&chunk_id) {
            self.linked_chunks.push(chunk_id);
        }

        Ok(())
    }

    /// Unlink this chunk from another chunk
    pub fn unlink_from_chunk(&mut self, chunk_id: &Uuid) {
        self.linked_chunks.retain(|id| id != chunk_id);
    }

    /// Add a processing note
    pub fn add_processing_note(&mut self, note: String) {
        if !note.trim().is_empty() {
            self.processing_notes.push(note);
        }
    }

    /// Check if this chunk is linked to another chunk
    pub fn is_linked_to(&self, chunk_id: &Uuid) -> bool {
        self.linked_chunks.contains(chunk_id)
    }

    /// Get the number of linked chunks
    pub fn linked_chunk_count(&self) -> usize {
        self.linked_chunks.len()
    }

    /// Validate the chunk metadata
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Validate sentence boundaries are sorted
        if !self.sentence_boundaries.windows(2).all(|w| w[0] <= w[1]) {
            return Err(ValidationError::InvalidChunk("Sentence boundaries must be sorted".to_string()));
        }

        // Validate no self-links
        if self.linked_chunks.contains(&self.id) {
            return Err(ValidationError::InvalidChunk("Chunk cannot be linked to itself".to_string()));
        }

        Ok(())
    }
}

/// Types of content chunks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChunkType {
    Sentence,
    Paragraph,
    Heading,
    ListItem,
    CodeBlock,
    Table,
    LinkedPhrase,
    Code,
    Link,
}

impl ChunkType {
    /// Get all available chunk types
    pub fn all() -> Vec<ChunkType> {
        vec![
            ChunkType::Sentence,
            ChunkType::Paragraph,
            ChunkType::Heading,
            ChunkType::ListItem,
            ChunkType::CodeBlock,
            ChunkType::Table,
            ChunkType::LinkedPhrase,
            ChunkType::Code,
            ChunkType::Link,
        ]
    }

    /// Get chunk type description
    pub fn description(&self) -> &'static str {
        match self {
            ChunkType::Sentence => "Individual sentence",
            ChunkType::Paragraph => "Complete paragraph",
            ChunkType::Heading => "Section heading",
            ChunkType::ListItem => "List item",
            ChunkType::CodeBlock => "Code block",
            ChunkType::Table => "Table content",
            ChunkType::LinkedPhrase => "Linked phrase group",
            ChunkType::Code => "Inline code",
            ChunkType::Link => "Link or reference",
        }
    }

    /// Check if this chunk type can be linked with others
    pub fn can_be_linked(&self) -> bool {
        matches!(self, ChunkType::Sentence | ChunkType::ListItem | ChunkType::LinkedPhrase)
    }
}

/// Chapter model with multi-language content support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: Uuid,
    pub project_id: Uuid,
    pub chapter_number: u32,
    pub title: HashMap<String, String>, // language -> title
    pub slug: String,
    pub content: HashMap<String, String>, // language -> markdown content
    pub chunks: Vec<ChunkMetadata>,
    pub status: ChapterStatus,
    pub assigned_translators: HashMap<String, String>, // language -> user_id
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Status of chapter translation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChapterStatus {
    Draft,
    ReadyForTranslation,
    InTranslation,
    InReview,
    Approved,
    Published,
}

// Term is now provided by the new translation memory crate
// Use: tradocflow_translation_memory::Term

// LanguagePair is now provided by the new translation memory crate
// Use: tradocflow_translation_memory::LanguagePair
// Keeping a local type alias for backward compatibility
pub use tradocflow_translation_memory::LanguagePair;

/// Chunk size preferences for content processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkSizePreference {
    Sentence,
    Paragraph,
    Custom(usize),
}

/// Export settings for PDF and other formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSettings {
    pub include_metadata: bool,
    pub pdf_layout: PdfLayout,
    pub font_settings: FontSettings,
    pub page_settings: PageSettings,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            include_metadata: true,
            pdf_layout: PdfLayout::SingleColumn,
            font_settings: FontSettings::default(),
            page_settings: PageSettings::default(),
        }
    }
}

/// PDF layout options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PdfLayout {
    SingleColumn,
    SideBySide,
    MultiColumn(u8),
}

/// Font settings for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontSettings {
    pub font_family: String,
    pub font_size: u8,
    pub line_height: f32,
}

impl Default for FontSettings {
    fn default() -> Self {
        Self {
            font_family: "Liberation Sans".to_string(),
            font_size: 12,
            line_height: 1.5,
        }
    }
}

/// Page settings for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSettings {
    pub page_size: PageSize,
    pub margins: Margins,
    pub orientation: PageOrientation,
}

impl Default for PageSettings {
    fn default() -> Self {
        Self {
            page_size: PageSize::A4,
            margins: Margins::default(),
            orientation: PageOrientation::Portrait,
        }
    }
}

/// Page size options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PageSize {
    A4,
    Letter,
    Legal,
    Custom { width: f32, height: f32 },
}

/// Page margins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Margins {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

impl Default for Margins {
    fn default() -> Self {
        Self {
            top: 2.5,
            bottom: 2.5,
            left: 2.5,
            right: 2.5,
        }
    }
}

/// Page orientation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PageOrientation {
    Portrait,
    Landscape,
}

/// Collaboration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationSettings {
    pub enable_real_time_sync: bool,
    pub auto_save_changes: bool,
    pub notification_preferences: NotificationPreferences,
    pub review_workflow: ReviewWorkflow,
}

impl Default for CollaborationSettings {
    fn default() -> Self {
        Self {
            enable_real_time_sync: true,
            auto_save_changes: true,
            notification_preferences: NotificationPreferences::default(),
            review_workflow: ReviewWorkflow::default(),
        }
    }
}

/// Notification preferences for collaboration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub email_notifications: bool,
    pub in_app_notifications: bool,
    pub notify_on_changes: bool,
    pub notify_on_reviews: bool,
}

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            email_notifications: true,
            in_app_notifications: true,
            notify_on_changes: true,
            notify_on_reviews: true,
        }
    }
}

/// Review workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewWorkflow {
    pub require_review: bool,
    pub auto_approve_threshold: Option<f32>,
    pub reviewers_per_language: u8,
}

impl Default for ReviewWorkflow {
    fn default() -> Self {
        Self {
            require_review: true,
            auto_approve_threshold: None,
            reviewers_per_language: 1,
        }
    }
}

/// Parquet schema structures for efficient storage

/// Translation unit optimized for Parquet storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationUnitParquet {
    pub id: String,
    pub project_id: String,
    pub chapter_id: String,
    pub chunk_id: String,
    pub source_language: String,
    pub source_text: String,
    pub target_language: String,
    pub target_text: String,
    pub confidence_score: f32,
    pub context: Option<String>,
    pub translator_id: Option<String>,
    pub reviewer_id: Option<String>,
    pub quality_score: Option<f32>,
    pub created_at: i64, // Unix timestamp
    pub updated_at: i64,
}

impl From<TranslationUnit> for TranslationUnitParquet {
    fn from(unit: TranslationUnit) -> Self {
        Self {
            id: unit.id.to_string(),
            project_id: unit.project_id.to_string(),
            chapter_id: unit.chapter_id.to_string(),
            chunk_id: unit.chunk_id.to_string(),
            source_language: unit.source_language.code().to_string(),
            source_text: unit.source_text,
            target_language: unit.target_language.code().to_string(),
            target_text: unit.target_text,
            confidence_score: unit.confidence_score,
            context: unit.context,
            translator_id: unit.metadata.translator_id,
            reviewer_id: unit.metadata.reviewer_id,
            quality_score: unit.metadata.quality_score,
            created_at: unit.created_at.timestamp(),
            updated_at: unit.updated_at.timestamp(),
        }
    }
}

impl TryFrom<TranslationUnitParquet> for TranslationUnit {
    type Error = ValidationError;

    fn try_from(parquet: TranslationUnitParquet) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&parquet.id)
            .map_err(|_| ValidationError::InvalidTranslationUnit("Invalid UUID format".to_string()))?;
        let project_id = Uuid::parse_str(&parquet.project_id)
            .map_err(|_| ValidationError::InvalidTranslationUnit("Invalid project UUID format".to_string()))?;
        let chapter_id = Uuid::parse_str(&parquet.chapter_id)
            .map_err(|_| ValidationError::InvalidTranslationUnit("Invalid chapter UUID format".to_string()))?;
        let chunk_id = Uuid::parse_str(&parquet.chunk_id)
            .map_err(|_| ValidationError::InvalidTranslationUnit("Invalid chunk UUID format".to_string()))?;

        let created_at = DateTime::from_timestamp(parquet.created_at, 0)
            .ok_or_else(|| ValidationError::InvalidTranslationUnit("Invalid created_at timestamp".to_string()))?;
        let updated_at = DateTime::from_timestamp(parquet.updated_at, 0)
            .ok_or_else(|| ValidationError::InvalidTranslationUnit("Invalid updated_at timestamp".to_string()))?;

        let metadata = TranslationMetadata {
            translator_id: parquet.translator_id,
            reviewer_id: parquet.reviewer_id,
            quality_score: parquet.quality_score,
            notes: Vec::new(),
            tags: Vec::new(),
        };

        Ok(Self {
            id,
            project_id,
            chapter_id,
            chunk_id,
            source_language: tradocflow_translation_memory::Language::Custom(parquet.source_language),
            source_text: parquet.source_text,
            target_language: tradocflow_translation_memory::Language::Custom(parquet.target_language),
            target_text: parquet.target_text,
            confidence_score: parquet.confidence_score,
            context: parquet.context,
            metadata,
            created_at,
            updated_at,
        })
    }
}

/// Chunk metadata optimized for Parquet storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadataParquet {
    pub id: String,
    pub chapter_id: String,
    pub original_position: i32,
    pub sentence_boundaries: Vec<i32>,
    pub linked_chunks: Vec<String>,
    pub chunk_type: String,
    pub processing_notes: Vec<String>,
}

impl From<ChunkMetadata> for ChunkMetadataParquet {
    fn from(chunk: ChunkMetadata) -> Self {
        Self {
            id: chunk.id.to_string(),
            chapter_id: "".to_string(), // Will be set by the caller
            original_position: chunk.original_position as i32,
            sentence_boundaries: chunk.sentence_boundaries.iter().map(|&x| x as i32).collect(),
            linked_chunks: chunk.linked_chunks.iter().map(|id| id.to_string()).collect(),
            chunk_type: serde_json::to_string(&chunk.chunk_type).unwrap_or_default(),
            processing_notes: chunk.processing_notes,
        }
    }
}

impl TryFrom<ChunkMetadataParquet> for ChunkMetadata {
    type Error = ValidationError;

    fn try_from(parquet: ChunkMetadataParquet) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&parquet.id)
            .map_err(|_| ValidationError::InvalidChunk("Invalid UUID format".to_string()))?;

        let linked_chunks: Result<Vec<Uuid>, _> = parquet.linked_chunks
            .iter()
            .map(|s| Uuid::parse_str(s))
            .collect();
        let linked_chunks = linked_chunks
            .map_err(|_| ValidationError::InvalidChunk("Invalid linked chunk UUID format".to_string()))?;

        let chunk_type: ChunkType = serde_json::from_str(&parquet.chunk_type)
            .map_err(|_| ValidationError::InvalidChunk("Invalid chunk type format".to_string()))?;

        Ok(Self {
            id,
            original_position: parquet.original_position as usize,
            sentence_boundaries: parquet.sentence_boundaries.iter().map(|&x| x as usize).collect(),
            linked_chunks,
            chunk_type,
            processing_notes: parquet.processing_notes,
        })
    }
}

/// CSV import/export data structures for terminology management

// TerminologyCsvRecord is now provided by the new translation memory crate
// Use: tradocflow_translation_memory::TerminologyCsvRecord

// TerminologyImportResult and TerminologyImportError are now provided by the new translation memory crate
// Use: tradocflow_translation_memory::{TerminologyImportResult, TerminologyImportError}

// ConflictResolution is now provided by the new translation memory crate
// Use: tradocflow_translation_memory::ConflictResolution

// TerminologyValidationConfig is now provided by the new translation memory crate
// Use: tradocflow_translation_memory::TerminologyValidationConfig

/// Translation status tracking for workflow management
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TranslationStatus {
    NotStarted,
    InProgress,
    PendingReview,
    Approved,
    Rejected,
    Published,
}

impl TranslationStatus {
    /// Get all available translation statuses
    pub fn all() -> Vec<TranslationStatus> {
        vec![
            TranslationStatus::NotStarted,
            TranslationStatus::InProgress,
            TranslationStatus::PendingReview,
            TranslationStatus::Approved,
            TranslationStatus::Rejected,
            TranslationStatus::Published,
        ]
    }

    /// Get status description
    pub fn description(&self) -> &'static str {
        match self {
            TranslationStatus::NotStarted => "Translation not started",
            TranslationStatus::InProgress => "Translation in progress",
            TranslationStatus::PendingReview => "Pending review",
            TranslationStatus::Approved => "Approved by reviewer",
            TranslationStatus::Rejected => "Rejected by reviewer",
            TranslationStatus::Published => "Published and finalized",
        }
    }

    /// Check if status allows editing
    pub fn allows_editing(&self) -> bool {
        matches!(self, TranslationStatus::NotStarted | TranslationStatus::InProgress | TranslationStatus::Rejected)
    }

    /// Check if status requires review
    pub fn requires_review(&self) -> bool {
        matches!(self, TranslationStatus::PendingReview)
    }

    /// Get next possible statuses
    pub fn next_statuses(&self) -> Vec<TranslationStatus> {
        match self {
            TranslationStatus::NotStarted => vec![TranslationStatus::InProgress],
            TranslationStatus::InProgress => vec![TranslationStatus::PendingReview],
            TranslationStatus::PendingReview => vec![TranslationStatus::Approved, TranslationStatus::Rejected],
            TranslationStatus::Approved => vec![TranslationStatus::Published],
            TranslationStatus::Rejected => vec![TranslationStatus::InProgress],
            TranslationStatus::Published => vec![], // Final status
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_translation_project_creation() {
        let temp_dir = tempdir().unwrap();
        let project_path = temp_dir.path().to_path_buf();

        let project = TranslationProject::new(
            "Test Project".to_string(),
            Some("A test translation project".to_string()),
            "en".to_string(),
            vec!["es".to_string(), "fr".to_string()],
            project_path.clone(),
        );

        assert!(project.is_ok());
        let project = project.unwrap();
        assert_eq!(project.name, "Test Project");
        assert_eq!(project.source_language, "en");
        assert_eq!(project.target_languages, vec!["es", "fr"]);
        assert_eq!(project.project_path, project_path);
        assert!(project.team_members.is_empty());
    }

    #[test]
    fn test_translation_project_validation_empty_name() {
        let temp_dir = tempdir().unwrap();
        let project_path = temp_dir.path().to_path_buf();

        let result = TranslationProject::new(
            "".to_string(),
            None,
            "en".to_string(),
            vec!["es".to_string()],
            project_path,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::InvalidProjectName(_) => (),
            _ => panic!("Expected InvalidProjectName error"),
        }
    }

    #[test]
    fn test_team_member_creation() {
        let member = TeamMember::new(
            "user123".to_string(),
            "John Doe".to_string(),
            "john@example.com".to_string(),
            UserRole::Translator,
            vec!["en".to_string(), "es".to_string()],
        );

        assert!(member.is_ok());
        let member = member.unwrap();
        assert_eq!(member.id, "user123");
        assert_eq!(member.name, "John Doe");
        assert_eq!(member.email, "john@example.com");
        assert_eq!(member.role, UserRole::Translator);
        assert_eq!(member.languages, vec!["en", "es"]);
        assert!(member.has_permission(&Permission::EditTranslations));
        assert!(!member.has_permission(&Permission::ManageTeam));
    }

    #[test]
    fn test_add_team_member_to_project() {
        let temp_dir = tempdir().unwrap();
        let project_path = temp_dir.path().to_path_buf();

        let mut project = TranslationProject::new(
            "Test Project".to_string(),
            None,
            "en".to_string(),
            vec!["es".to_string(), "fr".to_string()],
            project_path,
        ).unwrap();

        let member = TeamMember::new(
            "user123".to_string(),
            "John Doe".to_string(),
            "john@example.com".to_string(),
            UserRole::Translator,
            vec!["en".to_string(), "es".to_string()],
        ).unwrap();

        let result = project.add_team_member(member);
        assert!(result.is_ok());
        assert_eq!(project.team_members.len(), 1);
        assert_eq!(project.team_members[0].id, "user123");
    }

    #[test]
    fn test_user_role_default_permissions() {
        let admin_perms = TeamMember::get_default_permissions(&UserRole::Admin);
        let pm_perms = TeamMember::get_default_permissions(&UserRole::ProjectManager);
        let reviewer_perms = TeamMember::get_default_permissions(&UserRole::Reviewer);
        let translator_perms = TeamMember::get_default_permissions(&UserRole::Translator);

        assert_eq!(admin_perms.len(), 6); // All permissions
        assert!(admin_perms.contains(&Permission::ManageTeam));
        assert!(admin_perms.contains(&Permission::EditTranslations));

        assert_eq!(pm_perms.len(), 5); // All except EditTranslations
        assert!(pm_perms.contains(&Permission::ManageTeam));
        assert!(!pm_perms.contains(&Permission::EditTranslations));

        assert_eq!(reviewer_perms.len(), 2);
        assert!(reviewer_perms.contains(&Permission::ReviewTranslations));
        assert!(reviewer_perms.contains(&Permission::ViewAnalytics));

        assert_eq!(translator_perms.len(), 1);
        assert!(translator_perms.contains(&Permission::EditTranslations));
    }

    #[test]
    fn test_serialization_deserialization() {
        let temp_dir = tempdir().unwrap();
        let project_path = temp_dir.path().to_path_buf();

        let mut project = TranslationProject::new(
            "Test Project".to_string(),
            Some("Description".to_string()),
            "en".to_string(),
            vec!["es".to_string()],
            project_path,
        ).unwrap();

        let member = TeamMember::new(
            "user123".to_string(),
            "John Doe".to_string(),
            "john@example.com".to_string(),
            UserRole::Translator,
            vec!["en".to_string()],
        ).unwrap();

        project.add_team_member(member).unwrap();

        // Serialize to JSON
        let json = serde_json::to_string(&project).unwrap();
        
        // Deserialize from JSON
        let deserialized: TranslationProject = serde_json::from_str(&json).unwrap();
        
        assert_eq!(project.id, deserialized.id);
        assert_eq!(project.name, deserialized.name);
        assert_eq!(project.source_language, deserialized.source_language);
        assert_eq!(project.target_languages, deserialized.target_languages);
        assert_eq!(project.team_members.len(), deserialized.team_members.len());
    }

    #[test]
    fn test_translation_unit_creation() {
        let unit = TranslationUnit::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "en".to_string(),
            "Hello world".to_string(),
            "es".to_string(),
            "Hola mundo".to_string(),
            0.95,
            Some("Greeting context".to_string()),
        );

        assert!(unit.is_ok());
        let unit = unit.unwrap();
        assert_eq!(unit.source_language, "en");
        assert_eq!(unit.source_text, "Hello world");
        assert_eq!(unit.target_language, "es");
        assert_eq!(unit.target_text, "Hola mundo");
        assert_eq!(unit.confidence_score, 0.95);
        assert_eq!(unit.context, Some("Greeting context".to_string()));
    }

    #[test]
    fn test_translation_unit_validation_empty_source() {
        let result = TranslationUnit::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "en".to_string(),
            "".to_string(),
            "es".to_string(),
            "Hola mundo".to_string(),
            0.95,
            None,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::InvalidTranslationUnit(_) => (),
            _ => panic!("Expected InvalidTranslationUnit error"),
        }
    }

    #[test]
    fn test_translation_unit_validation_invalid_confidence() {
        let result = TranslationUnit::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "en".to_string(),
            "Hello world".to_string(),
            "es".to_string(),
            "Hola mundo".to_string(),
            1.5, // Invalid confidence score
            None,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::InvalidTranslationUnit(_) => (),
            _ => panic!("Expected InvalidTranslationUnit error"),
        }
    }

    #[test]
    fn test_translation_unit_validation_same_languages() {
        let result = TranslationUnit::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "en".to_string(),
            "Hello world".to_string(),
            "en".to_string(), // Same as source
            "Hello world".to_string(),
            0.95,
            None,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::InvalidTranslationUnit(_) => (),
            _ => panic!("Expected InvalidTranslationUnit error"),
        }
    }

    #[test]
    fn test_chunk_metadata_creation() {
        let chunk = ChunkMetadata::new(
            0,
            vec![0, 12, 18],
            ChunkType::Sentence,
        );

        assert!(chunk.is_ok());
        let chunk = chunk.unwrap();
        assert_eq!(chunk.original_position, 0);
        assert_eq!(chunk.sentence_boundaries, vec![0, 12, 18]);
        assert_eq!(chunk.chunk_type, ChunkType::Sentence);
        assert!(chunk.linked_chunks.is_empty());
        assert!(chunk.processing_notes.is_empty());
    }

    #[test]
    fn test_chunk_metadata_validation_unsorted_boundaries() {
        let result = ChunkMetadata::new(
            0,
            vec![12, 0, 18], // Unsorted boundaries
            ChunkType::Sentence,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::InvalidChunk(_) => (),
            _ => panic!("Expected InvalidChunk error"),
        }
    }

    #[test]
    fn test_chunk_linking() {
        let mut chunk1 = ChunkMetadata::new(0, vec![0, 12], ChunkType::Sentence).unwrap();
        let chunk2_id = Uuid::new_v4();

        let result = chunk1.link_with_chunk(chunk2_id);
        assert!(result.is_ok());
        assert!(chunk1.is_linked_to(&chunk2_id));
        assert_eq!(chunk1.linked_chunk_count(), 1);

        // Test unlinking
        chunk1.unlink_from_chunk(&chunk2_id);
        assert!(!chunk1.is_linked_to(&chunk2_id));
        assert_eq!(chunk1.linked_chunk_count(), 0);
    }

    #[test]
    fn test_chunk_self_linking_prevention() {
        let mut chunk = ChunkMetadata::new(0, vec![0, 12], ChunkType::Sentence).unwrap();
        let result = chunk.link_with_chunk(chunk.id);

        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::InvalidChunk(_) => (),
            _ => panic!("Expected InvalidChunk error"),
        }
    }

    #[test]
    fn test_chunk_type_properties() {
        assert!(ChunkType::Sentence.can_be_linked());
        assert!(ChunkType::ListItem.can_be_linked());
        assert!(ChunkType::LinkedPhrase.can_be_linked());
        assert!(!ChunkType::Heading.can_be_linked());
        assert!(!ChunkType::CodeBlock.can_be_linked());

        assert_eq!(ChunkType::Sentence.description(), "Individual sentence");
        assert_eq!(ChunkType::Paragraph.description(), "Complete paragraph");
    }

    #[test]
    fn test_translation_status_workflow() {
        let status = TranslationStatus::NotStarted;
        assert!(status.allows_editing());
        assert!(!status.requires_review());

        let next_statuses = status.next_statuses();
        assert_eq!(next_statuses, vec![TranslationStatus::InProgress]);

        let in_progress = TranslationStatus::InProgress;
        assert!(in_progress.allows_editing());
        assert!(!in_progress.requires_review());

        let pending_review = TranslationStatus::PendingReview;
        assert!(!pending_review.allows_editing());
        assert!(pending_review.requires_review());

        let published = TranslationStatus::Published;
        assert!(!published.allows_editing());
        assert!(published.next_statuses().is_empty());
    }

    #[test]
    fn test_parquet_conversion() {
        let unit = TranslationUnit::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            "en".to_string(),
            "Hello world".to_string(),
            "es".to_string(),
            "Hola mundo".to_string(),
            0.95,
            Some("Context".to_string()),
        ).unwrap();

        // Convert to Parquet format
        let parquet: TranslationUnitParquet = unit.clone().into();
        assert_eq!(parquet.source_text, "Hello world");
        assert_eq!(parquet.target_text, "Hola mundo");
        assert_eq!(parquet.confidence_score, 0.95);

        // Convert back from Parquet format
        let converted: TranslationUnit = parquet.try_into().unwrap();
        assert_eq!(converted.source_text, unit.source_text);
        assert_eq!(converted.target_text, unit.target_text);
        assert_eq!(converted.confidence_score, unit.confidence_score);
    }

    #[test]
    fn test_term_creation() {
        let term = Term::new(
            "API".to_string(),
            Some("Application Programming Interface".to_string()),
            true,
        );

        assert!(term.is_ok());
        let term = term.unwrap();
        assert_eq!(term.term, "API");
        assert_eq!(term.definition, Some("Application Programming Interface".to_string()));
        assert!(term.do_not_translate);
    }

    #[test]
    fn test_term_validation_empty_term() {
        let result = Term::new(
            "".to_string(),
            Some("Definition".to_string()),
            false,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::InvalidTerm(_) => (),
            _ => panic!("Expected InvalidTerm error"),
        }
    }

    #[test]
    fn test_term_validation_empty_definition() {
        let result = Term::new(
            "Term".to_string(),
            Some("".to_string()),
            false,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::InvalidTerm(_) => (),
            _ => panic!("Expected InvalidTerm error"),
        }
    }

    #[test]
    fn test_term_update_definition() {
        let mut term = Term::new(
            "API".to_string(),
            Some("Old definition".to_string()),
            false,
        ).unwrap();

        let result = term.update_definition(Some("New definition".to_string()));
        assert!(result.is_ok());
        assert_eq!(term.definition, Some("New definition".to_string()));

        // Test removing definition
        let result = term.update_definition(None);
        assert!(result.is_ok());
        assert_eq!(term.definition, None);
    }

    #[test]
    fn test_terminology_csv_record_conversion() {
        let csv_record = TerminologyCsvRecord {
            term: "API".to_string(),
            definition: Some("Application Programming Interface".to_string()),
            do_not_translate: Some("true".to_string()),
            category: Some("Technical".to_string()),
            notes: Some("Commonly used in software development".to_string()),
        };

        let term = csv_record.to_term().unwrap();
        assert_eq!(term.term, "API");
        assert!(term.do_not_translate);
        assert!(term.definition.as_ref().unwrap().contains("Application Programming Interface"));
        assert!(term.definition.as_ref().unwrap().contains("Commonly used in software development"));

        // Test conversion back to CSV
        let csv_back = TerminologyCsvRecord::from_term(&term);
        assert_eq!(csv_back.term, "API");
        assert_eq!(csv_back.do_not_translate, Some("true".to_string()));
    }

    #[test]
    fn test_terminology_csv_boolean_parsing() {
        let test_cases = vec![
            ("true", true),
            ("false", false),
            ("yes", true),
            ("no", false),
            ("1", true),
            ("0", false),
            ("True", true),
            ("False", false),
        ];

        for (input, expected) in test_cases {
            let csv_record = TerminologyCsvRecord {
                term: "Test".to_string(),
                definition: None,
                do_not_translate: Some(input.to_string()),
                category: None,
                notes: None,
            };

            let term = csv_record.to_term().unwrap();
            assert_eq!(term.do_not_translate, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_terminology_csv_invalid_boolean() {
        let csv_record = TerminologyCsvRecord {
            term: "Test".to_string(),
            definition: None,
            do_not_translate: Some("invalid".to_string()),
            category: None,
            notes: None,
        };

        let result = csv_record.to_term();
        assert!(result.is_err());
        match result.unwrap_err() {
            ValidationError::InvalidTerm(_) => (),
            _ => panic!("Expected InvalidTerm error"),
        }
    }

    #[test]
    fn test_terminology_validation_config() {
        let config = TerminologyValidationConfig::default();
        
        // Test valid term
        let valid_term = Term::new(
            "API".to_string(),
            Some("Application Programming Interface".to_string()),
            false,
        ).unwrap();
        assert!(config.validate_term(&valid_term).is_ok());

        // Test term too long
        let long_term = Term::new(
            "A".repeat(config.max_term_length + 1),
            None,
            false,
        ).unwrap();
        assert!(config.validate_term(&long_term).is_err());

        // Test duplicate detection
        assert!(config.are_duplicates("API", "api")); // Case insensitive by default
        
        let case_sensitive_config = TerminologyValidationConfig {
            case_sensitive: true,
            ..Default::default()
        };
        assert!(!case_sensitive_config.are_duplicates("API", "api")); // Case sensitive
    }

    #[test]
    fn test_terminology_import_result() {
        let mut result = TerminologyImportResult::new();
        assert_eq!(result.success_count(), 0);
        assert_eq!(result.error_count(), 0);
        assert!(!result.has_errors());

        result.successful_imports.push(Term::new("Test".to_string(), None, false).unwrap());
        assert_eq!(result.success_count(), 1);

        result.failed_imports.push(TerminologyImportError {
            row_number: 1,
            term: "Invalid".to_string(),
            error: ValidationError::InvalidTerm("Test error".to_string()),
        });
        assert_eq!(result.error_count(), 1);
        assert!(result.has_errors());
    }

    #[test]
    fn test_conflict_resolution_strategies() {
        let strategies = ConflictResolution::all();
        assert_eq!(strategies.len(), 4);
        assert!(strategies.contains(&ConflictResolution::Skip));
        assert!(strategies.contains(&ConflictResolution::Overwrite));
        assert!(strategies.contains(&ConflictResolution::Merge));
        assert!(strategies.contains(&ConflictResolution::CreateVariant));

        assert_eq!(ConflictResolution::Skip.description(), "Skip conflicting terms");
        assert_eq!(ConflictResolution::Overwrite.description(), "Overwrite existing terms");
    }
}