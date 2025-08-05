use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub content: HashMap<String, String>, // language -> markdown content
    pub document_type: DocumentType,
    pub source_language: String, // ISO 639-1 code
    pub target_languages: Vec<String>, // ISO 639-1 codes
    pub status: DocumentStatus,
    pub metadata: DocumentMetadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentType {
    Manual,
    Guide,
    Specification,
    Report,
    Article,
    Book,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DocumentStatus {
    Draft,
    InTranslation,
    UnderReview,
    Approved,
    Published,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub tags: Vec<String>,
    pub category: Option<String>,
    pub author: String,
    pub contributors: Vec<String>,
    pub version: String,
    pub word_count: HashMap<String, usize>, // language -> word count
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: Uuid,
    pub document_id: Uuid,
    pub chapter_number: u32,
    pub title: HashMap<String, String>, // language -> title
    pub slug: String, // URL-friendly identifier
    pub content: HashMap<String, String>, // language -> markdown content
    pub order: u32,
    pub status: ChapterStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChapterStatus {
    Draft,
    InTranslation,
    UnderReview,
    Approved,
    Published,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationUnit {
    pub id: Uuid,
    pub chapter_id: Uuid,
    pub paragraph_id: String, // Unique identifier within chapter
    pub source_language: String,
    pub source_text: String,
    pub translations: HashMap<String, TranslationVersion>, // language -> translation
    pub context: Option<String>,
    pub notes: Vec<TranslationNote>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationVersion {
    pub text: String,
    pub translator: String,
    pub status: TranslationStatus,
    pub quality_score: Option<f32>,
    pub created_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub reviewer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TranslationStatus {
    Pending,
    InProgress,
    Completed,
    UnderReview,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationNote {
    pub id: Uuid,
    pub author: String,
    pub note: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDocumentRequest {
    pub title: String,
    pub description: Option<String>,
    pub document_type: DocumentType,
    pub source_language: String,
    pub target_languages: Vec<String>,
    pub metadata: DocumentMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChapterRequest {
    pub title: HashMap<String, String>,
    pub slug: String,
    pub content: HashMap<String, String>,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStructure {
    pub project_id: Uuid,
    pub base_path: String,
    pub languages: Vec<String>,
    pub chapters: Vec<ChapterInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterInfo {
    pub chapter_number: u32,
    pub slug: String,
    pub title: HashMap<String, String>,
    pub file_paths: HashMap<String, String>, // language -> file path
}

impl DocumentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentStatus::Draft => "draft",
            DocumentStatus::InTranslation => "in_translation",
            DocumentStatus::UnderReview => "under_review",
            DocumentStatus::Approved => "approved",
            DocumentStatus::Published => "published",
            DocumentStatus::Archived => "archived",
        }
    }
}

impl ChapterStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChapterStatus::Draft => "draft",
            ChapterStatus::InTranslation => "in_translation",
            ChapterStatus::UnderReview => "under_review",
            ChapterStatus::Approved => "approved",
            ChapterStatus::Published => "published",
        }
    }
}

impl TranslationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TranslationStatus::Pending => "pending",
            TranslationStatus::InProgress => "in_progress",
            TranslationStatus::Completed => "completed",
            TranslationStatus::UnderReview => "under_review",
            TranslationStatus::Approved => "approved",
            TranslationStatus::Rejected => "rejected",
        }
    }
}