use std::collections::HashMap;
use std::sync::Arc;
use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use tower_http::cors::{CorsLayer, Any};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// Import the application modules
use tradocflow::{
    Database, DocumentImportService, 
    User, Document, DocumentStatus, DocumentMetadata,
    CreateProjectRequest, UpdateProjectRequest, Priority,
    services::project_manager::ProjectManager,
    database::{
        project_repository::ProjectRepository,
        kanban_repository::KanbanRepository,
        member_repository::MemberRepository,
        translation_progress_repository::TranslationProgressRepository,
    },
    models::{
        member::MemberRole,
        kanban::{CreateKanbanCardRequest, UpdateKanbanCardRequest, MoveCardRequest},
    }
};

/// Application state shared across all handlers
#[derive(Clone)]
pub struct ApiState {
    pub project_manager: Arc<ProjectManager>,
    pub project_repository: Arc<ProjectRepository>,
    pub kanban_repository: Arc<KanbanRepository>,
    pub member_repository: Arc<MemberRepository>,
    pub translation_progress_repository: Arc<TranslationProgressRepository>,
    pub import_service: Arc<DocumentImportService>,
}

/// Request/Response structures

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDocumentRequest {
    pub title: String,
    pub content: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveContentRequest {
    pub content: String,
    pub language: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetLanguageRequest {
    pub language: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateProjectApiRequest {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationResponse {
    pub id: Uuid,
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnreadCountResponse {
    pub count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Main server function
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Initialize database
    let database = Database::new("tradocflow.db")
        .map_err(|e| format!("Failed to initialize database: {}", e))?;
    
    let pool = database.pool();
    
    // Initialize repositories
    let project_repository = Arc::new(ProjectRepository::new(pool.clone()));
    let kanban_repository = Arc::new(KanbanRepository::new(pool.clone()));
    let member_repository = Arc::new(MemberRepository::new(pool.clone()));
    let translation_progress_repository = Arc::new(TranslationProgressRepository::new(pool.clone()));
    
    // Initialize services  
    let project_manager = Arc::new(ProjectManager::new("./projects"));
    let import_service = Arc::new(DocumentImportService::new());
    
    // Create application state
    let state = ApiState {
        project_manager,
        project_repository,
        kanban_repository,
        member_repository,
        translation_progress_repository,
        import_service,
    };

    // Build the router
    let app = Router::new()
        // Document endpoints
        .route("/api/documents", get(get_documents))
        .route("/api/documents", post(create_document))
        .route("/api/documents/:id", get(get_document))
        .route("/api/documents/:id", put(update_document))
        .route("/api/documents/:id", delete(delete_document))
        .route("/api/editor/:id/content", post(save_document_content))
        
        // User endpoints
        .route("/api/users", get(get_users))
        .route("/api/users", post(create_user))
        .route("/api/users/:id", get(get_user))
        .route("/api/users/:id", put(update_user))
        .route("/api/users/:id", delete(delete_user))
        
        // Language endpoints
        .route("/api/languages", get(get_languages))
        .route("/api/language", post(set_language))
        
        // Project endpoints
        .route("/api/projects", get(get_projects))
        .route("/api/projects", post(create_project))
        .route("/api/projects/:id", get(get_project))
        .route("/api/projects/:id", put(update_project))
        .route("/api/projects/:id", delete(delete_project))
        .route("/api/projects/:id/members", get(get_project_members))
        .route("/api/projects/:id/members", post(add_project_member))
        .route("/api/projects/:id/members/:user_id", delete(remove_project_member))
        .route("/api/projects/:id/structure", get(get_project_structure))
        .route("/api/projects/:id/summary", get(get_project_summary))
        
        // Kanban endpoints
        .route("/api/projects/:id/kanban", get(get_kanban_cards))
        .route("/api/projects/:id/kanban", post(create_kanban_card))
        .route("/api/kanban/:id", get(get_kanban_card))
        .route("/api/kanban/:id", put(update_kanban_card))
        .route("/api/kanban/:id", delete(delete_kanban_card))
        .route("/api/kanban/:id/move", put(move_kanban_card))
        .route("/api/projects/:id/events", get(project_events_stream))
        
        // Translation progress endpoints
        .route("/api/projects/:id/translation-progress", get(get_translation_progress))
        .route("/api/projects/:id/translation-progress", post(create_translation_progress))
        .route("/api/translation-progress/:id", get(get_translation_progress_by_id))
        .route("/api/translation-progress/:id", put(update_translation_progress))
        .route("/api/translation-progress/:id", delete(delete_translation_progress))
        
        // Notification endpoints
        .route("/api/notifications", get(get_notifications))
        .route("/api/notifications/unread", get(get_unread_count))
        .route("/api/notifications/:id/read", post(mark_notification_read))
        
        // Import/Export endpoints
        .route("/api/import", post(import_document))
        .route("/api/export/:id", post(export_document))
        .route("/api/export/:id/download", get(download_export))
        
        // Health check
        .route("/health", get(health_check))
        
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        )
        .with_state(state);

    // Start the server
    let listener = TcpListener::bind("0.0.0.0:8001").await?;
    println!("🚀 API Server listening on http://localhost:8001");
    println!("📋 Available endpoints:");
    println!("  - GET  /api/documents");
    println!("  - POST /api/documents");
    println!("  - GET  /api/documents/:id");
    println!("  - POST /api/editor/:id/content");
    println!("  - GET  /api/users");
    println!("  - POST /api/users");
    println!("  - GET  /api/languages");
    println!("  - POST /api/language");
    println!("  - GET  /api/projects");
    println!("  - POST /api/projects");
    println!("  - GET  /api/notifications");
    println!("  - GET  /api/notifications/unread");
    println!("  - GET  /health");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": Utc::now().to_rfc3339(),
        "service": "tradocflow-api"
    }))
}

// Document endpoints
async fn get_documents(State(_state): State<ApiState>) -> Result<Json<Vec<Document>>, StatusCode> {
    // For now, return sample documents - in real implementation would query database
    let documents = vec![
        Document {
            id: Uuid::new_v4(),
            title: "Sample Document 1".to_string(),
            content: {
                let mut content = HashMap::new();
                content.insert("en".to_string(), "# Sample Document\n\nThis is a sample document.".to_string());
                content.insert("de".to_string(), "# Beispieldokument\n\nDies ist ein Beispieldokument.".to_string());
                content
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
            status: DocumentStatus::Draft,
            metadata: DocumentMetadata {
                languages: vec!["en".to_string(), "de".to_string()],
                tags: vec!["sample".to_string()],
                project_id: None,
                screenshots: vec![],
            },
        }
    ];
    
    Ok(Json(documents))
}

async fn create_document(
    State(_state): State<ApiState>,
    Json(request): Json<CreateDocumentRequest>,
) -> Result<Json<Document>, StatusCode> {
    let document = Document {
        id: Uuid::new_v4(),
        title: request.title,
        content: request.content.clone(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
        status: DocumentStatus::Draft,
        metadata: DocumentMetadata {
            languages: request.content.keys().cloned().collect(),
            tags: vec![],
            project_id: None,
            screenshots: vec![],
        },
    };
    
    Ok(Json(document))
}

async fn get_document(
    State(_state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Document>, StatusCode> {
    // Sample document response - in real implementation would query database
    let document = Document {
        id,
        title: "Retrieved Document".to_string(),
        content: {
            let mut content = HashMap::new();
            content.insert("en".to_string(), "# Retrieved Document\n\nContent loaded from API.".to_string());
            content
        },
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
        status: DocumentStatus::Draft,
        metadata: DocumentMetadata {
            languages: vec!["en".to_string()],
            tags: vec![],
            project_id: None,
            screenshots: vec![],
        },
    };
    
    Ok(Json(document))
}

async fn update_document(
    State(_state): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(request): Json<CreateDocumentRequest>,
) -> Result<Json<Document>, StatusCode> {
    let document = Document {
        id,
        title: request.title,
        content: request.content.clone(),
        created_at: Utc::now() - chrono::Duration::days(1), // Simulate creation time
        updated_at: Utc::now(),
        version: 2,
        status: DocumentStatus::Draft,
        metadata: DocumentMetadata {
            languages: request.content.keys().cloned().collect(),
            tags: vec![],
            project_id: None,
            screenshots: vec![],
        },
    };
    
    Ok(Json(document))
}

async fn delete_document(
    State(_state): State<ApiState>,
    Path(_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    // In real implementation would delete from database
    Ok(StatusCode::NO_CONTENT)
}

async fn save_document_content(
    State(_state): State<ApiState>,
    Path(doc_id): Path<Uuid>,
    Json(request): Json<SaveContentRequest>,
) -> Result<StatusCode, StatusCode> {
    // In real implementation would save to database
    println!("Saving content for document {} in language {}: {} chars", 
             doc_id, request.language, request.content.len());
    Ok(StatusCode::OK)
}

// User endpoints
async fn get_users(State(_state): State<ApiState>) -> Result<Json<Vec<User>>, StatusCode> {
    let users = vec![
        User {
            id: "user1".to_string(),
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            role: MemberRole::Member,
            created_at: Utc::now(),
            active: true,
        }
    ];
    
    Ok(Json(users))
}

async fn create_user(
    State(_state): State<ApiState>,
    Json(request): Json<CreateUserRequest>,
) -> Result<Json<User>, StatusCode> {
    let user = User {
        id: Uuid::new_v4().to_string(),
        name: request.name,
        email: request.email,
        role: MemberRole::Viewer,
        created_at: Utc::now(),
        active: true,
    };
    
    Ok(Json(user))
}

async fn get_user(
    State(_state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<User>, StatusCode> {
    let user = User {
        id: id.clone(),
        name: "Retrieved User".to_string(),
        email: "user@example.com".to_string(),
        role: MemberRole::Member,
        created_at: Utc::now(),
        active: true,
    };
    
    Ok(Json(user))
}

async fn update_user(
    State(_state): State<ApiState>,
    Path(id): Path<String>,
    Json(request): Json<CreateUserRequest>,
) -> Result<Json<User>, StatusCode> {
    let user = User {
        id,
        name: request.name,
        email: request.email,
        role: MemberRole::Member,
        created_at: Utc::now() - chrono::Duration::days(30),
        active: true,
    };
    
    Ok(Json(user))
}

async fn delete_user(
    State(_state): State<ApiState>,
    Path(_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    Ok(StatusCode::NO_CONTENT)
}

// Language endpoints
async fn get_languages(State(_state): State<ApiState>) -> Result<Json<Vec<String>>, StatusCode> {
    let languages = vec![
        "en".to_string(),
        "de".to_string(),
        "fr".to_string(),
        "es".to_string(),
        "it".to_string(),
        "nl".to_string(),
    ];
    
    Ok(Json(languages))
}

async fn set_language(
    State(_state): State<ApiState>,
    Json(request): Json<SetLanguageRequest>,
) -> Result<StatusCode, StatusCode> {
    println!("Setting application language to: {}", request.language);
    Ok(StatusCode::OK)
}

// Project endpoints
async fn get_projects(State(state): State<ApiState>) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.project_repository.list_by_member("default_user", None, None).await {
        Ok(projects) => Ok(Json(serde_json::to_value(projects).unwrap_or(serde_json::json!([])))),
        Err(_) => {
            // Return empty list if no projects found
            Ok(Json(serde_json::json!([])))
        }
    }
}

async fn create_project(
    State(state): State<ApiState>,
    Json(request): Json<CreateProjectApiRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let create_request = CreateProjectRequest {
        name: request.title,
        description: Some(request.description),
        due_date: None,
        priority: Priority::Medium,
    };
    
    match state.project_repository.create(create_request, "default_user".to_string()).await {
        Ok(project) => Ok(Json(serde_json::to_value(project).unwrap())),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn get_project(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.project_repository.get_by_id(id).await {
        Ok(Some(project)) => Ok(Json(serde_json::to_value(project).unwrap())),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn update_project(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateProjectRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.project_repository.update(id, request).await {
        Ok(Some(project)) => Ok(Json(serde_json::to_value(project).unwrap())),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn delete_project(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    match state.project_repository.delete(id).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn get_project_members(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.member_repository.get_project_members(id).await {
        Ok(members) => Ok(Json(serde_json::to_value(members).unwrap_or(serde_json::json!([])))),
        Err(_) => Ok(Json(serde_json::json!([]))),  
    }
}

async fn add_project_member(
    State(_state): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(_request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Placeholder response
    Ok(Json(serde_json::json!({
        "id": Uuid::new_v4(),
        "project_id": id,
        "user_id": "sample_user",
        "role": "editor"
    })))
}

async fn remove_project_member(
    State(_state): State<ApiState>,
    Path((_project_id, _user_id)): Path<(Uuid, String)>,
) -> Result<StatusCode, StatusCode> {
    // Placeholder - would remove member from project
    Ok(StatusCode::NO_CONTENT)
}

async fn get_project_structure(
    State(_state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Return sample project structure - in real implementation would use ProjectManager
    let structure = serde_json::json!({
        "project_id": id,
        "base_path": format!("/projects/{}", id),
        "languages": ["en", "de", "fr"],
        "chapters": [
            {
                "slug": "introduction",
                "chapter_number": 1,
                "title": {
                    "en": "Introduction",
                    "de": "Einführung",
                    "fr": "Introduction"
                },
                "file_paths": {
                    "en": format!("/projects/{}/chapters/en/01-introduction.md", id),
                    "de": format!("/projects/{}/chapters/de/01-introduction.md", id),
                    "fr": format!("/projects/{}/chapters/fr/01-introduction.md", id)
                }
            }
        ]
    });
    
    Ok(Json(structure))
}

async fn get_project_summary(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.project_repository.get_summary(id).await {
        Ok(Some(summary)) => Ok(Json(serde_json::to_value(summary).unwrap())),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// Kanban endpoints
async fn get_kanban_cards(
    State(state): State<ApiState>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.kanban_repository.get_board(project_id).await {
        Ok(board) => Ok(Json(serde_json::to_value(board).unwrap())),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn create_kanban_card(
    State(state): State<ApiState>,
    Path(project_id): Path<Uuid>,
    Json(request): Json<CreateKanbanCardRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let metadata = std::collections::HashMap::new();
    match state.kanban_repository.create_card(&project_id, &request, "system", metadata).await {
        Ok(card) => Ok(Json(serde_json::to_value(card).unwrap())),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn get_kanban_card(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.kanban_repository.get_card(&id).await {
        Ok(card) => Ok(Json(serde_json::to_value(card).unwrap())),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn update_kanban_card(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateKanbanCardRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.kanban_repository.update_card(id, request).await {
        Ok(Some(card)) => Ok(Json(serde_json::to_value(card).unwrap())),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn delete_kanban_card(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    match state.kanban_repository.delete_card(id).await {
        Ok(true) => Ok(StatusCode::NO_CONTENT),
        Ok(false) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn move_kanban_card(
    State(state): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(mut request): Json<MoveCardRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    request.card_id = id; // Ensure the card_id matches the path parameter
    match state.kanban_repository.move_card(request).await {
        Ok(Some(card)) => Ok(Json(serde_json::to_value(card).unwrap())),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn project_events_stream(
    State(_state): State<ApiState>,
    Path(_project_id): Path<Uuid>,
) -> Result<axum::response::Response, StatusCode> {
    use axum::response::Response;
    use axum::http::HeaderValue;
    use std::time::Duration;
    use tokio::time::interval;
    use futures::stream::StreamExt;

    // Create a simple SSE response with manual headers
    let headers = [
        ("Content-Type", "text/event-stream"),
        ("Cache-Control", "no-cache"),
        ("Connection", "keep-alive"),
        ("Access-Control-Allow-Origin", "*"),
    ];

    // Create a heartbeat stream that yields Result<String, Error>
    let stream = async_stream::stream! {
        let mut interval = interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let data = serde_json::json!({
                "type": "heartbeat",
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            yield Ok::<String, std::io::Error>(format!("data: {}\n\n", data));
        }
    };

    let body = axum::body::Body::from_stream(stream);
    let mut response = Response::new(body);
    
    for (key, value) in headers {
        response.headers_mut().insert(key, HeaderValue::from_static(value));
    }

    Ok(response)
}

// Translation progress endpoints
async fn get_translation_progress(
    State(_state): State<ApiState>,
    Path(_project_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Placeholder response
    Ok(Json(serde_json::json!([])))
}

async fn create_translation_progress(
    State(_state): State<ApiState>,
    Path(project_id): Path<Uuid>,
    Json(_request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Placeholder response
    Ok(Json(serde_json::json!({
        "id": Uuid::new_v4(),
        "project_id": project_id,
        "status": "not_started",
        "progress_percentage": 0
    })))
}

async fn get_translation_progress_by_id(
    State(_state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Placeholder response
    Ok(Json(serde_json::json!({
        "id": id,
        "status": "in_progress",
        "progress_percentage": 50
    })))
}

async fn update_translation_progress(
    State(_state): State<ApiState>,
    Path(id): Path<Uuid>,
    Json(_request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Placeholder response
    Ok(Json(serde_json::json!({
        "id": id,
        "status": "completed",
        "progress_percentage": 100
    })))
}

async fn delete_translation_progress(
    State(_state): State<ApiState>,
    Path(_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    // Placeholder response
    Ok(StatusCode::NO_CONTENT)
}

// Notification endpoints
async fn get_notifications(State(_state): State<ApiState>) -> Result<Json<Vec<NotificationResponse>>, StatusCode> {
    // Sample notifications - in real implementation would query database
    let notifications = vec![
        NotificationResponse {
            id: Uuid::new_v4(),
            title: "Welcome".to_string(),
            message: "Welcome to Tradocflow!".to_string(),
            notification_type: "info".to_string(),
            read: false,
            created_at: Utc::now(),
        }
    ];
    
    Ok(Json(notifications))
}

async fn get_unread_count(State(_state): State<ApiState>) -> Result<Json<UnreadCountResponse>, StatusCode> {
    Ok(Json(UnreadCountResponse { count: 1 }))
}

async fn mark_notification_read(
    State(_state): State<ApiState>,
    Path(_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    // In real implementation would update database
    Ok(StatusCode::OK)
}

// Import/Export endpoints
async fn import_document(State(_state): State<ApiState>) -> Result<Json<Document>, StatusCode> {
    // Placeholder for document import
    let document = Document {
        id: Uuid::new_v4(),
        title: "Imported Document".to_string(),
        content: {
            let mut content = HashMap::new();
            content.insert("en".to_string(), "# Imported Document\n\nThis document was imported.".to_string());
            content
        },
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: 1,
        status: DocumentStatus::Draft,
        metadata: DocumentMetadata {
            languages: vec!["en".to_string()],
            tags: vec!["imported".to_string()],
            project_id: None,
            screenshots: vec![],
        },
    };
    
    Ok(Json(document))
}

async fn export_document(
    State(_state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "export_id": Uuid::new_v4(),
        "status": "completed",
        "download_url": format!("/api/export/{}/download", id)
    })))
}

async fn download_export(
    State(_state): State<ApiState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Simplified response for now
    Ok(Json(serde_json::json!({
        "message": "Export download would be here",
        "id": id
    })))
}