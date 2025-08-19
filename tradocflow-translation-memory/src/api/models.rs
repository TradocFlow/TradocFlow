//! API models and error handling

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::models::{Language, TranslationUnit};

/// Standard API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub meta: Option<ApiMeta>,
}

/// Metadata for paginated responses
#[derive(Debug, Serialize)]
pub struct ApiMeta {
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
}

/// Translation unit creation request
#[derive(Debug, Deserialize)]
pub struct CreateTranslationUnitRequest {
    pub source_text: String,
    pub target_text: String,
    pub source_language: String,
    pub target_language: String,
    pub context: Option<String>,
    pub quality_score: Option<f32>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Translation unit update request
#[derive(Debug, Deserialize)]
pub struct UpdateTranslationUnitRequest {
    pub source_text: Option<String>,
    pub target_text: Option<String>,
    pub context: Option<String>,
    pub quality_score: Option<f32>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Batch translation unit creation request
#[derive(Debug, Deserialize)]
pub struct BatchCreateRequest {
    pub units: Vec<CreateTranslationUnitRequest>,
}

/// Search request parameters
#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub q: String,                    // Query text
    pub source: String,               // Source language
    pub target: String,               // Target language
    pub threshold: Option<f32>,       // Similarity threshold (0.0-1.0)
    pub limit: Option<usize>,         // Max results
    pub offset: Option<usize>,        // Pagination offset
}

/// Translation suggestions request parameters
#[derive(Debug, Deserialize)]
pub struct SuggestionsRequest {
    pub text: String,                 // Source text
    pub target_language: String,      // Target language
    pub source_language: Option<String>, // Source language (optional)
    pub limit: Option<usize>,         // Max suggestions
}

/// Translation memory creation request
#[derive(Debug, Deserialize)]
pub struct CreateMemoryRequest {
    pub name: String,
    pub description: Option<String>,
    pub source_language: String,
    pub target_language: String,
}

/// Translation unit response
#[derive(Debug, Serialize)]
pub struct TranslationUnitResponse {
    pub id: String,
    pub source_text: String,
    pub target_text: String,
    pub source_language: String,
    pub target_language: String,
    pub confidence_score: f32,
    pub quality_score: Option<f32>,
    pub context: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl From<TranslationUnit> for TranslationUnitResponse {
    fn from(unit: TranslationUnit) -> Self {
        Self {
            id: unit.id.to_string(),
            source_text: unit.source_text,
            target_text: unit.target_text,
            source_language: unit.source_language.code().to_string(),
            target_language: unit.target_language.code().to_string(),
            confidence_score: unit.confidence_score,
            quality_score: unit.metadata.quality_score,
            context: unit.context,
            created_at: unit.created_at.to_rfc3339(),
            updated_at: unit.updated_at.to_rfc3339(),
            metadata: None, // Convert metadata if needed
        }
    }
}

/// Translation suggestion response
#[derive(Debug, Serialize)]
pub struct TranslationSuggestionResponse {
    pub id: String,
    pub source_text: String,
    pub suggested_text: String,
    pub confidence: f32,
    pub similarity: f32,
    pub context: Option<String>,
    pub source: String,
}

/// Memory statistics response
#[derive(Debug, Serialize)]
pub struct MemoryStatsResponse {
    pub total_units: usize,
    pub language_pairs: Vec<String>,
    pub last_updated: Option<String>,
    pub cache_stats: CacheStatsResponse,
    pub database_stats: DatabaseStatsResponse,
}

/// Cache statistics response
#[derive(Debug, Serialize)]
pub struct CacheStatsResponse {
    pub entries: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_ratio: f64,
}

/// Database statistics response
#[derive(Debug, Serialize)]
pub struct DatabaseStatsResponse {
    pub total_rows: usize,
    pub database_size: String,
    pub connection_pool_active: usize,
    pub connection_pool_idle: usize,
}

/// API Error types
#[derive(Debug)]
pub enum ApiError {
    ValidationError(String),
    NotFound(String),
    Unauthorized(String),
    InternalError(String),
    DatabaseError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            ApiError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::DatabaseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", msg)),
        };

        let body = Json(ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(error_message),
            meta: None,
        });

        (status, body).into_response()
    }
}

/// Helper function to create successful API responses
pub fn success_response<T: Serialize>(data: T) -> Json<ApiResponse<T>> {
    Json(ApiResponse {
        success: true,
        data: Some(data),
        error: None,
        meta: None,
    })
}

/// Helper function to create paginated API responses
pub fn paginated_response<T: Serialize>(
    data: T,
    total: usize,
    page: usize,
    per_page: usize,
) -> Json<ApiResponse<T>> {
    let total_pages = (total + per_page - 1) / per_page;
    
    Json(ApiResponse {
        success: true,
        data: Some(data),
        error: None,
        meta: Some(ApiMeta {
            total,
            page,
            per_page,
            total_pages,
        }),
    })
}

/// Language parsing helper
pub fn parse_language(lang_code: &str) -> Result<Language, ApiError> {
    Language::from_code(lang_code)
        .or_else(|| Some(Language::Custom(lang_code.to_string())))
        .ok_or_else(|| ApiError::ValidationError(format!("Invalid language code: {}", lang_code)))
}