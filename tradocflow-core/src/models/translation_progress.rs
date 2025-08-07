use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationProgress {
    pub id: Uuid,
    pub project_id: Uuid,
    pub document_id: Option<Uuid>,
    pub source_language: String,
    pub target_language: String,
    pub status: TranslationStatus,
    pub assigned_translator: Option<String>,
    pub progress_percentage: u8, // 0-100
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub due_date: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub quality_score: Option<u8>, // 0-100 quality score
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TranslationStatus {
    #[serde(rename = "not_started")]
    NotStarted,
    #[serde(rename = "in_progress")]
    InProgress,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "reviewed")]
    Reviewed,
    #[serde(rename = "approved")]
    Approved,
    #[serde(rename = "rejected")]
    Rejected,
    #[serde(rename = "on_hold")]
    OnHold,
    #[serde(rename = "cancelled")]
    Cancelled,
}

impl TranslationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TranslationStatus::NotStarted => "not_started",
            TranslationStatus::InProgress => "in_progress",
            TranslationStatus::Completed => "completed",
            TranslationStatus::Reviewed => "reviewed",
            TranslationStatus::Approved => "approved",
            TranslationStatus::Rejected => "rejected",
            TranslationStatus::OnHold => "on_hold",
            TranslationStatus::Cancelled => "cancelled",
        }
    }
    
    pub fn from_str(s: &str) -> Self {
        match s {
            "not_started" => TranslationStatus::NotStarted,
            "in_progress" => TranslationStatus::InProgress,
            "completed" => TranslationStatus::Completed,
            "reviewed" => TranslationStatus::Reviewed,
            "approved" => TranslationStatus::Approved,
            "rejected" => TranslationStatus::Rejected,
            "on_hold" => TranslationStatus::OnHold,
            "cancelled" => TranslationStatus::Cancelled,
            _ => TranslationStatus::NotStarted, // Default fallback
        }
    }
    
    pub fn is_active(&self) -> bool {
        matches!(self, TranslationStatus::NotStarted | TranslationStatus::InProgress)
    }
    
    pub fn is_completed(&self) -> bool {
        matches!(self, TranslationStatus::Completed | TranslationStatus::Reviewed | TranslationStatus::Approved)
    }
    
    pub fn is_final(&self) -> bool {
        matches!(self, TranslationStatus::Approved | TranslationStatus::Rejected | TranslationStatus::Cancelled)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTranslationProgressRequest {
    pub document_id: Option<Uuid>,
    pub source_language: String,
    pub target_language: String,
    pub assigned_translator: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTranslationProgressRequest {
    pub status: Option<TranslationStatus>,
    pub assigned_translator: Option<String>,
    pub progress_percentage: Option<u8>,
    pub due_date: Option<DateTime<Utc>>,
    pub quality_score: Option<u8>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationProgressSummary {
    pub project_id: Uuid,
    pub project_name: String,
    pub total_translations: usize,
    pub not_started: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub approved: usize,
    pub overall_progress_percentage: f32,
    pub languages: Vec<LanguageProgress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageProgress {
    pub language: String,
    pub total_translations: usize,
    pub completed_translations: usize,
    pub progress_percentage: f32,
    pub average_quality_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationWorkload {
    pub translator_id: String,
    pub translator_name: String,
    pub active_translations: usize,
    pub overdue_translations: usize,
    pub average_progress: f32,
    pub assignments: Vec<TranslationAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationAssignment {
    pub id: Uuid,
    pub project_name: String,
    pub document_title: Option<String>,
    pub source_language: String,
    pub target_language: String,
    pub status: TranslationStatus,
    pub progress_percentage: u8,
    pub due_date: Option<DateTime<Utc>>,
    pub is_overdue: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationMetrics {
    pub total_projects: usize,
    pub total_translations: usize,
    pub active_translations: usize,
    pub completed_translations: usize,
    pub overdue_translations: usize,
    pub average_completion_time_days: Option<f32>,
    pub average_quality_score: Option<f32>,
    pub top_languages: Vec<(String, usize)>,
    pub productivity_by_translator: Vec<(String, usize)>,
}