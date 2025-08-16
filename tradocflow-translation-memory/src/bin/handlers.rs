//! HTTP handlers for the REST API endpoints

use axum::{
    extract::{Path, Query, State},
    response::Json,
    Extension,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    auth::{
        initialize_default_users, find_user_by_username, store_user, username_exists,
        generate_token, User, RegisterRequest, LoginRequest, AuthResponse, Claims
    },
    models::*,
    AppState,
};
use tradocflow_translation_memory::{
    models::{TranslationUnit, TranslationUnitBuilder},
    services::translation_memory::LanguagePair,
};

/// User registration handler
pub async fn register_user(
    State(_state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> Result<Json<ApiResponse<AuthResponse>>, ApiError> {
    // Initialize default users if needed
    initialize_default_users()
        .map_err(|e| ApiError::InternalError(format!("Failed to initialize users: {}", e)))?;

    // Validate input
    if request.username.is_empty() || request.email.is_empty() || request.password.is_empty() {
        return Err(ApiError::ValidationError("Username, email, and password are required".to_string()));
    }

    // Check if username already exists
    if username_exists(&request.username) {
        return Err(ApiError::ValidationError("Username already exists".to_string()));
    }

    // Create new user
    let user = User::new(request.username, request.email, request.password)
        .map_err(|e| ApiError::InternalError(format!("Failed to create user: {}", e)))?;

    // Store user
    store_user(user.clone())
        .map_err(|e| ApiError::InternalError(format!("Failed to store user: {}", e)))?;

    // Generate token
    let token = generate_token(&user, "default-secret")
        .map_err(|e| ApiError::InternalError(format!("Failed to generate token: {}", e)))?;

    let response = AuthResponse {
        token,
        user_id: user.id,
        username: user.username,
        expires_at: (chrono::Utc::now() + chrono::Duration::hours(24)).to_rfc3339(),
    };

    Ok(success_response(response))
}

/// User login handler
pub async fn login_user(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<ApiResponse<AuthResponse>>, ApiError> {
    // Find user
    let user = find_user_by_username(&request.username)
        .ok_or_else(|| ApiError::Unauthorized("Invalid credentials".to_string()))?;

    // Verify password
    if !user.verify_password(&request.password) {
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    }

    // Generate token
    let token = generate_token(&user, &state.jwt_secret)
        .map_err(|e| ApiError::InternalError(format!("Failed to generate token: {}", e)))?;

    let response = AuthResponse {
        token,
        user_id: user.id,
        username: user.username,
        expires_at: (chrono::Utc::now() + chrono::Duration::hours(24)).to_rfc3339(),
    };

    Ok(success_response(response))
}

/// Create a new translation unit
pub async fn create_translation_unit(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreateTranslationUnitRequest>,
) -> Result<Json<ApiResponse<TranslationUnitResponse>>, ApiError> {
    // Validate input
    if request.source_text.trim().is_empty() || request.target_text.trim().is_empty() {
        return Err(ApiError::ValidationError("Source and target text cannot be empty".to_string()));
    }

    // Create translation unit using language codes
    let mut builder = TranslationUnitBuilder::new()
        .source_text(request.source_text)
        .target_text(request.target_text)
        .source_language(&request.source_language)
        .target_language(&request.target_language);

    if let Some(context) = request.context {
        builder = builder.context(context);
    }

    let unit = builder.build()
        .map_err(|e| ApiError::ValidationError(format!("Invalid translation unit: {}", e)))?;

    // Add to translation memory
    state.translation_memory.translation_memory()
        .add_translation_unit(unit.clone()).await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    Ok(success_response(TranslationUnitResponse::from(unit)))
}

/// Get a translation unit by ID
pub async fn get_translation_unit(
    State(_state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<TranslationUnitResponse>>, ApiError> {
    // Parse UUID
    let _unit_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError::ValidationError("Invalid UUID format".to_string()))?;

    // TODO: Implement get_translation_unit_by_id in the service
    // For now, return not implemented
    Err(ApiError::InternalError("Get by ID not yet implemented".to_string()))
}

/// Update an existing translation unit
pub async fn update_translation_unit(
    State(_state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(id): Path<String>,
    Json(_request): Json<UpdateTranslationUnitRequest>,
) -> Result<Json<ApiResponse<TranslationUnitResponse>>, ApiError> {
    // Parse UUID
    let _unit_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError::ValidationError("Invalid UUID format".to_string()))?;

    // TODO: Implement update_translation_unit_by_id in the service
    // For now, return not implemented
    Err(ApiError::InternalError("Update by ID not yet implemented".to_string()))
}

/// Delete a translation unit
pub async fn delete_translation_unit(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<bool>>, ApiError> {
    // Parse UUID
    let unit_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError::ValidationError("Invalid UUID format".to_string()))?;

    // Delete from translation memory
    let deleted = state.translation_memory.translation_memory()
        .delete_translation_unit(unit_id).await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    if !deleted {
        return Err(ApiError::NotFound("Translation unit not found".to_string()));
    }

    Ok(success_response(true))
}

/// List translation units (with pagination)
pub async fn list_translation_units(
    State(_state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Query(_params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<TranslationUnitResponse>>>, ApiError> {
    // TODO: Implement list_translation_units with pagination
    // For now, return empty list
    Ok(paginated_response(Vec::<TranslationUnitResponse>::new(), 0, 1, 20))
}

/// Batch create translation units
pub async fn batch_create_translation_units(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Json(request): Json<BatchCreateRequest>,
) -> Result<Json<ApiResponse<Vec<TranslationUnitResponse>>>, ApiError> {
    if request.units.is_empty() {
        return Err(ApiError::ValidationError("No translation units provided".to_string()));
    }

    let mut units = Vec::new();
    
    // Convert requests to translation units
    for req in request.units {
        // Validate input
        if req.source_text.trim().is_empty() || req.target_text.trim().is_empty() {
            return Err(ApiError::ValidationError("Source and target text cannot be empty".to_string()));
        }

        // Create translation unit using language codes
        let mut builder = TranslationUnitBuilder::new()
            .source_text(req.source_text)
            .target_text(req.target_text)
            .source_language(&req.source_language)
            .target_language(&req.target_language);

        if let Some(context) = req.context {
            builder = builder.context(context);
        }

        let unit = builder.build()
            .map_err(|e| ApiError::ValidationError(format!("Invalid translation unit: {}", e)))?;

        units.push(unit);
    }

    // Add batch to translation memory
    let count = state.translation_memory.translation_memory()
        .add_translation_units_batch(units.clone()).await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    log::info!("Successfully added {} translation units in batch", count);

    let responses: Vec<TranslationUnitResponse> = units.into_iter()
        .map(TranslationUnitResponse::from)
        .collect();

    Ok(success_response(responses))
}

/// Search for translations
pub async fn search_translations(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Query(params): Query<SearchRequest>,
) -> Result<Json<ApiResponse<Vec<TranslationUnitResponse>>>, ApiError> {
    // Validate input
    if params.q.trim().is_empty() {
        return Err(ApiError::ValidationError("Query cannot be empty".to_string()));
    }

    // Parse languages
    let source_lang = parse_language(&params.source)?;
    let target_lang = parse_language(&params.target)?;

    // Search translations
    let threshold = params.threshold.unwrap_or(0.7) as f64;
    let results = state.translation_memory.translation_memory()
        .search(&params.q, source_lang, target_lang, threshold).await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let responses: Vec<TranslationUnitResponse> = results.into_iter()
        .take(params.limit.unwrap_or(20))
        .skip(params.offset.unwrap_or(0))
        .map(TranslationUnitResponse::from)
        .collect();

    Ok(success_response(responses))
}

/// Fuzzy search for translations
pub async fn fuzzy_search_translations(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Query(params): Query<SearchRequest>,
) -> Result<Json<ApiResponse<Vec<TranslationUnitResponse>>>, ApiError> {
    // Validate input
    if params.q.trim().is_empty() {
        return Err(ApiError::ValidationError("Query cannot be empty".to_string()));
    }

    // Parse languages
    let source_lang = parse_language(&params.source)?;
    let target_lang = parse_language(&params.target)?;

    // Create language pair
    let language_pair = LanguagePair::new(source_lang, target_lang);

    // Fuzzy search with lower threshold
    let threshold = params.threshold.unwrap_or(0.5);
    let matches = state.translation_memory.translation_memory()
        .search_similar_translations(&params.q, language_pair, Some(threshold)).await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    // Convert matches to translation units (simplified conversion)
    let responses: Vec<TranslationUnitResponse> = matches.into_iter()
        .take(params.limit.unwrap_or(20))
        .skip(params.offset.unwrap_or(0))
        .map(|m| TranslationUnitResponse {
            id: m.id.to_string(),
            source_text: m.source_text,
            target_text: m.target_text,
            source_language: m.language_pair.source.code().to_string(),
            target_language: m.language_pair.target.code().to_string(),
            confidence_score: m.confidence_score,
            quality_score: m.metadata.quality_score,
            context: m.context,
            created_at: m.metadata.created_at.to_rfc3339(),
            updated_at: m.metadata.updated_at.to_rfc3339(),
            metadata: None,
        })
        .collect();

    Ok(success_response(responses))
}

/// Get translation suggestions
pub async fn get_translation_suggestions(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Query(params): Query<SuggestionsRequest>,
) -> Result<Json<ApiResponse<Vec<TranslationSuggestionResponse>>>, ApiError> {
    // Validate input
    if params.text.trim().is_empty() {
        return Err(ApiError::ValidationError("Text cannot be empty".to_string()));
    }

    // Parse languages
    let target_lang = parse_language(&params.target_language)?;
    let source_lang = if let Some(src) = params.source_language {
        Some(parse_language(&src)?)
    } else {
        None
    };

    // Get suggestions
    let suggestions = state.translation_memory.translation_memory()
        .get_translation_suggestions(&params.text, target_lang, source_lang).await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    let responses: Vec<TranslationSuggestionResponse> = suggestions.into_iter()
        .take(params.limit.unwrap_or(10))
        .map(|s| TranslationSuggestionResponse {
            id: s.id.to_string(),
            source_text: s.source_text,
            suggested_text: s.suggested_text,
            confidence: s.confidence,
            similarity: s.similarity,
            context: s.context,
            source: format!("{:?}", s.source),
        })
        .collect();

    Ok(success_response(responses))
}

/// List translation memories
pub async fn list_translation_memories(
    State(_state): State<AppState>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, ApiError> {
    // TODO: Implement translation memory management
    // For now, return empty list
    Ok(success_response(Vec::<serde_json::Value>::new()))
}

/// Create translation memory
pub async fn create_translation_memory(
    State(_state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Json(_request): Json<CreateMemoryRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    // TODO: Implement translation memory creation
    Err(ApiError::InternalError("Translation memory creation not yet implemented".to_string()))
}

/// Get memory statistics
pub async fn get_memory_statistics(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(_memory_id): Path<String>,
) -> Result<Json<ApiResponse<MemoryStatsResponse>>, ApiError> {
    // Get cache statistics
    let (cache_entries, hits, misses, hit_ratio, last_updated) = state.translation_memory.translation_memory()
        .get_detailed_cache_stats().await;

    // Get database statistics
    let db_stats = state.translation_memory.translation_memory()
        .get_database_stats().await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

    // Get connection pool stats
    let (active_connections, idle_connections) = state.translation_memory.translation_memory()
        .get_connection_pool_stats().await;

    let stats = MemoryStatsResponse {
        total_units: db_stats.translation_units_count as usize,
        language_pairs: vec![], // TODO: Extract language pairs from database
        last_updated: last_updated.map(|dt| dt.to_rfc3339()),
        cache_stats: CacheStatsResponse {
            entries: cache_entries,
            hit_count: hits,
            miss_count: misses,
            hit_ratio,
        },
        database_stats: DatabaseStatsResponse {
            total_rows: db_stats.translation_units_count as usize,
            database_size: format!("{} bytes", db_stats.database_size_bytes),
            connection_pool_active: active_connections,
            connection_pool_idle: idle_connections,
        },
    };

    Ok(success_response(stats))
}

/// Import TMX file  
pub async fn import_tmx(
    State(_state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Json(_request): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    // TODO: Implement TMX import
    Err(ApiError::InternalError("TMX import not yet implemented".to_string()))
}

/// Export TMX file
pub async fn export_tmx(
    State(_state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(_memory_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    // TODO: Implement TMX export
    Err(ApiError::InternalError("TMX export not yet implemented".to_string()))
}