//! Authentication middleware for protecting routes

use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};
use super::auth::verify_token;
use super::models::ApiError;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub translation_memory: std::sync::Arc<crate::TradocFlowTranslationMemory>,
    pub jwt_secret: String,
}

/// Authentication middleware that validates JWT tokens
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract the Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized("Missing authorization header".to_string()))?;

    // Validate Bearer token format
    if !auth_header.starts_with("Bearer ") {
        return Err(ApiError::Unauthorized("Invalid authorization header format".to_string()));
    }

    let token = &auth_header[7..]; // Remove "Bearer " prefix

    // Verify the token
    let token_data = verify_token(token, &state.jwt_secret)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    // Add user information to request extensions for use in handlers
    request.extensions_mut().insert(token_data.claims);

    Ok(next.run(request).await)
}