//! REST API Server for TradocFlow Translation Memory
//!
//! This server exposes the translation memory functionality through HTTP endpoints
//! with JWT authentication, comprehensive error handling, and OpenAPI documentation.

use anyhow::Result;
use axum::{
    http::{header, StatusCode},
    middleware,
    response::{IntoResponse, Json},
    routing::{delete, get, post, put},
    Router,
};
use clap::{Arg, Command};
use std::{
    net::SocketAddr,
    sync::Arc,
};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tradocflow_translation_memory::TradocFlowTranslationMemory;

use tradocflow_translation_memory::api::*;
use tradocflow_translation_memory::api::handlers::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Parse command line arguments
    let matches = Command::new("tradocflow-tm-server")
        .version("0.1.0")
        .about("REST API server for TradocFlow Translation Memory")
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Port to listen on")
                .default_value("8080"),
        )
        .arg(
            Arg::new("host")
                .long("host")
                .value_name("HOST")
                .help("Host to bind to")
                .default_value("127.0.0.1"),
        )
        .arg(
            Arg::new("database")
                .short('d')
                .long("database")
                .value_name("PATH")
                .help("Database path")
                .default_value("./translation_memory.db"),
        )
        .arg(
            Arg::new("jwt-secret")
                .long("jwt-secret")
                .value_name("SECRET")
                .help("JWT secret key")
                .default_value("your-secret-key-change-this-in-production"),
        )
        .get_matches();

    let host = matches.get_one::<String>("host").unwrap();
    let port = matches.get_one::<String>("port").unwrap();
    let database_path = matches.get_one::<String>("database").unwrap();
    let jwt_secret = matches.get_one::<String>("jwt-secret").unwrap().to_string();

    // Initialize translation memory
    let translation_memory = TradocFlowTranslationMemory::new(database_path).await?;
    translation_memory.initialize().await?;

    let app_state = AppState {
        translation_memory: Arc::new(translation_memory),
        jwt_secret,
    };

    // Build the application router
    let app = create_router(app_state).await;

    // Create the socket address
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    
    println!("🚀 TradocFlow Translation Memory REST API server starting...");
    println!("📍 Listening on: http://{}", addr);
    println!("📚 API Documentation: http://{}/api/docs", addr);
    println!("🗄️  Database: {}", database_path);

    // Start the server
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Create the application router with all routes and middleware
async fn create_router(state: AppState) -> Router {
    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/api/docs", get(api_documentation))
        .route("/api/auth/register", post(register_user))
        .route("/api/auth/login", post(login_user));

    // Protected routes (authentication required)
    let protected_routes = Router::new()
        .route("/api/v1/translation-units", post(create_translation_unit))
        .route("/api/v1/translation-units", get(list_translation_units))
        .route("/api/v1/translation-units/:id", get(get_translation_unit))
        .route("/api/v1/translation-units/:id", put(update_translation_unit))
        .route("/api/v1/translation-units/:id", delete(delete_translation_unit))
        .route("/api/v1/translation-units/batch", post(batch_create_translation_units))
        .route("/api/v1/search", get(search_translations))
        .route("/api/v1/search/fuzzy", get(fuzzy_search_translations))
        .route("/api/v1/suggestions", get(get_translation_suggestions))
        .route("/api/v1/memories", get(list_translation_memories))
        .route("/api/v1/memories", post(create_translation_memory))
        .route("/api/v1/memories/:id/stats", get(get_memory_statistics))
        .route("/api/v1/import/tmx", post(import_tmx))
        .route("/api/v1/export/tmx/:memory_id", get(export_tmx))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    // Combine all routes
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(vec![
                            header::AUTHORIZATION,
                            header::CONTENT_TYPE,
                        ]),
                )
        )
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "tradocflow-translation-memory",
        "version": "0.1.0",
        "timestamp": chrono::Utc::now()
    }))
}

/// API documentation endpoint
async fn api_documentation() -> impl IntoResponse {
    let docs = r#"
# TradocFlow Translation Memory REST API

## Health Check
GET /health - Returns server status

## Authentication
POST /api/auth/register - Register a new user
POST /api/auth/login - Login and get JWT token

## Translation Units
POST /api/v1/translation-units - Create translation unit
GET /api/v1/translation-units/{id} - Get translation unit
PUT /api/v1/translation-units/{id} - Update translation unit  
DELETE /api/v1/translation-units/{id} - Delete translation unit
POST /api/v1/translation-units/batch - Batch create units

## Search
GET /api/v1/search - Search translations
GET /api/v1/search/fuzzy - Fuzzy search
GET /api/v1/suggestions - Get translation suggestions

All protected endpoints require Authorization: Bearer <token>
"#;
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/markdown")],
        docs
    )
}