use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{User, Project, TradocumentError};
use crate::models::document::Document;

/// HTTP client for communicating with the Tradocument API
#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
    user_id: Option<String>,
}

impl ApiClient {
    /// Create a new API client
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            user_id: None,
        }
    }

    /// Set the authenticated user ID
    pub fn set_user_id(&mut self, user_id: String) {
        self.user_id = Some(user_id);
    }

    /// Get request builder with authentication headers
    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut builder = self.client.request(method, &url);
        
        if let Some(user_id) = &self.user_id {
            builder = builder.header("X-User-ID", user_id);
        }
        
        builder
    }

    // Document API methods
    
    /// Get all documents
    pub async fn get_documents(&self) -> Result<Vec<Document>, TradocumentError> {
        let response = self.request(reqwest::Method::GET, "/api/documents")
            .send()
            .await?;
            
        if response.status().is_success() {
            let documents = response.json::<Vec<Document>>().await?;
            Ok(documents)
        } else {
            Err(TradocumentError::ApiError(format!("Failed to get documents: {}", response.status())))
        }
    }

    /// Get a specific document by ID
    pub async fn get_document(&self, id: Uuid) -> Result<Document, TradocumentError> {
        let response = self.request(reqwest::Method::GET, &format!("/api/documents/{id}"))
            .send()
            .await?;
            
        if response.status().is_success() {
            let document = response.json::<Document>().await?;
            Ok(document)
        } else {
            Err(TradocumentError::ApiError(format!("Failed to get document: {}", response.status())))
        }
    }

    /// Create a new document
    pub async fn create_document(&self, title: String, content: HashMap<String, String>) -> Result<Document, TradocumentError> {
        #[derive(Serialize)]
        struct CreateDocumentRequest {
            title: String,
            content: HashMap<String, String>,
        }

        let request_body = CreateDocumentRequest { title, content };
        
        let response = self.request(reqwest::Method::POST, "/api/documents")
            .json(&request_body)
            .send()
            .await?;
            
        if response.status().is_success() {
            let document = response.json::<Document>().await?;
            Ok(document)
        } else {
            Err(TradocumentError::ApiError(format!("Failed to create document: {}", response.status())))
        }
    }

    /// Save document content
    pub async fn save_document_content(&self, doc_id: Uuid, content: String, language: String) -> Result<(), TradocumentError> {
        #[derive(Serialize)]
        struct SaveContentRequest {
            content: String,
            language: String,
        }

        let request_body = SaveContentRequest { content, language };
        
        let response = self.request(reqwest::Method::POST, &format!("/api/editor/{doc_id}/content"))
            .json(&request_body)
            .send()
            .await?;
            
        if response.status().is_success() {
            Ok(())
        } else {
            Err(TradocumentError::ApiError(format!("Failed to save content: {}", response.status())))
        }
    }

    // User API methods
    
    /// Get all users
    pub async fn get_users(&self) -> Result<Vec<User>, TradocumentError> {
        let response = self.request(reqwest::Method::GET, "/api/users")
            .send()
            .await?;
            
        if response.status().is_success() {
            let users = response.json::<Vec<User>>().await?;
            Ok(users)
        } else {
            Err(TradocumentError::ApiError(format!("Failed to get users: {}", response.status())))
        }
    }

    /// Create a new user
    pub async fn create_user(&self, name: String, email: String) -> Result<User, TradocumentError> {
        #[derive(Serialize)]
        struct CreateUserRequest {
            name: String,
            email: String,
        }

        let request_body = CreateUserRequest { name, email };
        
        let response = self.request(reqwest::Method::POST, "/api/users")
            .json(&request_body)
            .send()
            .await?;
            
        if response.status().is_success() {
            let user = response.json::<User>().await?;
            Ok(user)
        } else {
            Err(TradocumentError::ApiError(format!("Failed to create user: {}", response.status())))
        }
    }

    // Language API methods
    
    /// Get available languages
    pub async fn get_languages(&self) -> Result<Vec<String>, TradocumentError> {
        let response = self.request(reqwest::Method::GET, "/api/languages")
            .send()
            .await?;
            
        if response.status().is_success() {
            let languages = response.json::<Vec<String>>().await?;
            Ok(languages)
        } else {
            Err(TradocumentError::ApiError(format!("Failed to get languages: {}", response.status())))
        }
    }

    /// Set application language
    pub async fn set_language(&self, language: String) -> Result<(), TradocumentError> {
        #[derive(Serialize)]
        struct SetLanguageRequest {
            language: String,
        }

        let request_body = SetLanguageRequest { language };
        
        let response = self.request(reqwest::Method::POST, "/api/language")
            .json(&request_body)
            .send()
            .await?;
            
        if response.status().is_success() {
            Ok(())
        } else {
            Err(TradocumentError::ApiError(format!("Failed to set language: {}", response.status())))
        }
    }

    // Project API methods
    
    /// Create a new project
    pub async fn create_project(&self, title: String, description: String) -> Result<Project, TradocumentError> {
        #[derive(Serialize)]
        struct CreateProjectRequest {
            title: String,
            description: String,
        }

        let request_body = CreateProjectRequest { title, description };
        
        let response = self.request(reqwest::Method::POST, "/api/projects")
            .json(&request_body)
            .send()
            .await?;
            
        if response.status().is_success() {
            let project = response.json::<Project>().await?;
            Ok(project)
        } else {
            Err(TradocumentError::ApiError(format!("Failed to create project: {}", response.status())))
        }
    }

    // Notification API methods
    
    /// Get notifications for the current user
    pub async fn get_notifications(&self) -> Result<Vec<NotificationResponse>, TradocumentError> {
        let response = self.request(reqwest::Method::GET, "/api/notifications")
            .send()
            .await?;
            
        if response.status().is_success() {
            let notifications = response.json::<Vec<NotificationResponse>>().await?;
            Ok(notifications)
        } else {
            Err(TradocumentError::ApiError(format!("Failed to get notifications: {}", response.status())))
        }
    }

    /// Get unread notification count
    pub async fn get_unread_count(&self) -> Result<u32, TradocumentError> {
        let response = self.request(reqwest::Method::GET, "/api/notifications/unread")
            .send()
            .await?;
            
        if response.status().is_success() {
            #[derive(Deserialize)]
            struct UnreadCountResponse {
                count: u32,
            }
            
            let unread_response = response.json::<UnreadCountResponse>().await?;
            Ok(unread_response.count)
        } else {
            Err(TradocumentError::ApiError(format!("Failed to get unread count: {}", response.status())))
        }
    }
}

/// Response structure for notifications
#[derive(Debug, Deserialize)]
pub struct NotificationResponse {
    pub id: Uuid,
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub read: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<reqwest::Error> for TradocumentError {
    fn from(err: reqwest::Error) -> Self {
        TradocumentError::ApiError(err.to_string())
    }
}