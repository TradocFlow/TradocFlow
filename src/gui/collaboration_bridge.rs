use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use slint::{ComponentHandle, ModelRc, VecModel, SharedString};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tokio::sync::broadcast;

use crate::services::{
    CollaborativeEditingService, UserSession, DocumentChange, ChangeType,
    TranslationSuggestion, SuggestionStatus, Comment, CommentType, CommentContext,
    CollaborationEvent, UserPresenceUpdate, SuggestionVote, VoteType, CommentReply,
    SelectionRange
};
use crate::models::{UserRole, Permission};
use crate::{MainWindow, TradocumentError};

/// Bridge between Slint UI and collaborative editing service
pub struct CollaborationBridge {
    collaboration_service: Arc<CollaborativeEditingService>,
    main_window: slint::Weak<MainWindow>,
    current_user_id: Arc<Mutex<Option<String>>>,
    current_project_id: Arc<Mutex<Option<Uuid>>>,
    current_chapter_id: Arc<Mutex<Option<Uuid>>>,
    current_language: Arc<Mutex<String>>,
    event_receiver: Arc<Mutex<Option<broadcast::Receiver<CollaborationEvent>>>>,
}

/// Slint model structures for UI data
#[derive(Clone, Debug)]
pub struct SlintUserPresence {
    pub user_id: SharedString,
    pub user_name: SharedString,
    pub user_role: SharedString,
    pub is_typing: bool,
    pub is_active: bool,
}

#[derive(Clone, Debug)]
pub struct SlintSuggestion {
    pub id: SharedString,
    pub author_name: SharedString,
    pub original_text: SharedString,
    pub suggested_text: SharedString,
    pub reason: SharedString,
    pub status: SharedString,
    pub confidence_score: f32,
}

#[derive(Clone, Debug)]
pub struct SlintComment {
    pub id: SharedString,
    pub author_name: SharedString,
    pub content: SharedString,
    pub comment_type: SharedString,
    pub timestamp: SharedString,
    pub is_resolved: bool,
    pub reply_count: i32,
}

impl CollaborationBridge {
    /// Create a new collaboration bridge
    pub fn new(
        collaboration_service: Arc<CollaborativeEditingService>,
        main_window: slint::Weak<MainWindow>,
    ) -> Self {
        Self {
            collaboration_service,
            main_window,
            current_user_id: Arc::new(Mutex::new(None)),
            current_project_id: Arc::new(Mutex::new(None)),
            current_chapter_id: Arc::new(Mutex::new(None)),
            current_language: Arc::new(Mutex::new("en".to_string())),
            event_receiver: Arc::new(Mutex::new(None)),
        }
    }

    /// Initialize the bridge and set up event handling
    pub async fn initialize(&self) -> Result<(), TradocumentError> {
        // Subscribe to collaboration events
        let mut receiver = self.collaboration_service.subscribe_to_events();
        *self.event_receiver.lock().unwrap() = Some(receiver);

        // Set up UI callbacks
        self.setup_ui_callbacks().await?;

        // Start event processing task
        self.start_event_processing().await;

        Ok(())
    }

    /// Set up UI callbacks for collaboration features
    async fn setup_ui_callbacks(&self) -> Result<(), TradocumentError> {
        let window = self.main_window.upgrade()
            .ok_or_else(|| TradocumentError::SlintError("Main window not available".to_string()))?;

        // Set up collaboration panel callbacks
        let collaboration_service = Arc::clone(&self.collaboration_service);
        let current_user_id = Arc::clone(&self.current_user_id);
        let current_project_id = Arc::clone(&self.current_project_id);
        let current_chapter_id = Arc::clone(&self.current_chapter_id);
        let current_language = Arc::clone(&self.current_language);

        // Accept suggestion callback
        window.global::<crate::CollaborationCallbacks>().on_accept_suggestion({
            let service = Arc::clone(&collaboration_service);
            move |suggestion_id_str| {
                let service = Arc::clone(&service);
                let suggestion_id_str = suggestion_id_str.to_string();
                
                tokio::spawn(async move {
                    if let Ok(suggestion_id) = Uuid::parse_str(&suggestion_id_str) {
                        let update = crate::services::SuggestionUpdate {
                            status: Some(SuggestionStatus::Accepted),
                            reviewer_id: None, // Would be set from current user
                            reviewer_comments: None,
                        };
                        
                        if let Err(e) = service.update_suggestion(suggestion_id, update) {
                            eprintln!("Failed to accept suggestion: {}", e);
                        }
                    }
                });
            }
        });

        // Reject suggestion callback
        window.global::<crate::CollaborationCallbacks>().on_reject_suggestion({
            let service = Arc::clone(&collaboration_service);
            move |suggestion_id_str| {
                let service = Arc::clone(&service);
                let suggestion_id_str = suggestion_id_str.to_string();
                
                tokio::spawn(async move {
                    if let Ok(suggestion_id) = Uuid::parse_str(&suggestion_id_str) {
                        let update = crate::services::SuggestionUpdate {
                            status: Some(SuggestionStatus::Rejected),
                            reviewer_id: None, // Would be set from current user
                            reviewer_comments: None,
                        };
                        
                        if let Err(e) = service.update_suggestion(suggestion_id, update) {
                            eprintln!("Failed to reject suggestion: {}", e);
                        }
                    }
                });
            }
        });

        // Vote on suggestion callback
        window.global::<crate::CollaborationCallbacks>().on_vote_on_suggestion({
            let service = Arc::clone(&collaboration_service);
            let user_id = Arc::clone(&current_user_id);
            move |suggestion_id_str, vote_type_str| {
                let service = Arc::clone(&service);
                let user_id = Arc::clone(&user_id);
                let suggestion_id_str = suggestion_id_str.to_string();
                let vote_type_str = vote_type_str.to_string();
                
                tokio::spawn(async move {
                    if let (Ok(suggestion_id), Some(current_user)) = (
                        Uuid::parse_str(&suggestion_id_str),
                        user_id.lock().unwrap().clone()
                    ) {
                        let vote_type = match vote_type_str.as_str() {
                            "approve" => VoteType::Approve,
                            "reject" => VoteType::Reject,
                            _ => VoteType::NeedsWork,
                        };

                        let vote = SuggestionVote {
                            user_id: current_user.clone(),
                            user_name: current_user, // Would be actual name
                            vote_type,
                            comment: None,
                            timestamp: Utc::now(),
                        };

                        if let Err(e) = service.vote_on_suggestion(suggestion_id, vote) {
                            eprintln!("Failed to vote on suggestion: {}", e);
                        }
                    }
                });
            }
        });

        // Resolve comment callback
        window.global::<crate::CollaborationCallbacks>().on_resolve_comment({
            let service = Arc::clone(&collaboration_service);
            let user_id = Arc::clone(&current_user_id);
            move |comment_id_str| {
                let service = Arc::clone(&service);
                let user_id = Arc::clone(&user_id);
                let comment_id_str = comment_id_str.to_string();
                
                tokio::spawn(async move {
                    if let (Ok(comment_id), Some(current_user)) = (
                        Uuid::parse_str(&comment_id_str),
                        user_id.lock().unwrap().clone()
                    ) {
                        if let Err(e) = service.resolve_comment(comment_id, current_user) {
                            eprintln!("Failed to resolve comment: {}", e);
                        }
                    }
                });
            }
        });

        // Add comment callback
        window.global::<crate::CollaborationCallbacks>().on_add_comment({
            let service = Arc::clone(&collaboration_service);
            let user_id = Arc::clone(&current_user_id);
            let project_id = Arc::clone(&current_project_id);
            let chapter_id = Arc::clone(&current_chapter_id);
            let language = Arc::clone(&current_language);
            
            move |content_str, comment_type_str| {
                let service = Arc::clone(&service);
                let user_id = Arc::clone(&user_id);
                let project_id = Arc::clone(&project_id);
                let chapter_id = Arc::clone(&chapter_id);
                let language = Arc::clone(&language);
                let content_str = content_str.to_string();
                let comment_type_str = comment_type_str.to_string();
                
                tokio::spawn(async move {
                    if let (Some(current_user), Some(proj_id), Some(chap_id)) = (
                        user_id.lock().unwrap().clone(),
                        project_id.lock().unwrap().clone(),
                        chapter_id.lock().unwrap().clone()
                    ) {
                        let comment_type = match comment_type_str.as_str() {
                            "suggestion" => CommentType::Suggestion,
                            "question" => CommentType::Question,
                            "issue" => CommentType::Issue,
                            "approval" => CommentType::Approval,
                            _ => CommentType::General,
                        };

                        let comment = Comment {
                            id: Uuid::new_v4(),
                            author_id: current_user.clone(),
                            author_name: current_user, // Would be actual name
                            project_id: proj_id,
                            chapter_id: chap_id,
                            language: Some(language.lock().unwrap().clone()),
                            content: content_str,
                            comment_type,
                            context: CommentContext::Document,
                            position: None,
                            selection_range: None,
                            created_at: Utc::now(),
                            updated_at: Utc::now(),
                            is_resolved: false,
                            resolved_by: None,
                            resolved_at: None,
                            thread_id: None,
                            replies: Vec::new(),
                        };

                        if let Err(e) = service.add_comment(comment) {
                            eprintln!("Failed to add comment: {}", e);
                        }
                    }
                });
            }
        });

        // Create suggestion callback
        window.global::<crate::CollaborationCallbacks>().on_create_suggestion({
            let service = Arc::clone(&collaboration_service);
            let user_id = Arc::clone(&current_user_id);
            let project_id = Arc::clone(&current_project_id);
            let chapter_id = Arc::clone(&current_chapter_id);
            let language = Arc::clone(&current_language);
            
            move |original_text_str, suggested_text_str, reason_str| {
                let service = Arc::clone(&service);
                let user_id = Arc::clone(&user_id);
                let project_id = Arc::clone(&project_id);
                let chapter_id = Arc::clone(&chapter_id);
                let language = Arc::clone(&language);
                let original_text_str = original_text_str.to_string();
                let suggested_text_str = suggested_text_str.to_string();
                let reason_str = reason_str.to_string();
                
                tokio::spawn(async move {
                    if let (Some(current_user), Some(proj_id), Some(chap_id)) = (
                        user_id.lock().unwrap().clone(),
                        project_id.lock().unwrap().clone(),
                        chapter_id.lock().unwrap().clone()
                    ) {
                        let suggestion = TranslationSuggestion {
                            id: Uuid::new_v4(),
                            author_id: current_user.clone(),
                            author_name: current_user, // Would be actual name
                            project_id: proj_id,
                            chapter_id: chap_id,
                            unit_id: Uuid::new_v4(), // Would be actual unit ID
                            language: language.lock().unwrap().clone(),
                            original_text: original_text_str,
                            suggested_text: suggested_text_str,
                            reason: reason_str,
                            confidence_score: 0.8, // Default confidence
                            status: SuggestionStatus::Pending,
                            created_at: Utc::now(),
                            reviewed_at: None,
                            reviewer_id: None,
                            reviewer_comments: None,
                            votes: Vec::new(),
                        };

                        if let Err(e) = service.create_suggestion(suggestion) {
                            eprintln!("Failed to create suggestion: {}", e);
                        }
                    }
                });
            }
        });

        Ok(())
    }

    /// Start processing collaboration events
    async fn start_event_processing(&self) {
        let main_window = self.main_window.clone();
        let event_receiver = Arc::clone(&self.event_receiver);

        tokio::spawn(async move {
            loop {
                if let Some(mut receiver) = event_receiver.lock().unwrap().take() {
                    match receiver.recv().await {
                        Ok(event) => {
                            if let Some(window) = main_window.upgrade() {
                                Self::handle_collaboration_event(&window, event).await;
                            }
                            *event_receiver.lock().unwrap() = Some(receiver);
                        }
                        Err(_) => {
                            // Channel closed, break the loop
                            break;
                        }
                    }
                } else {
                    // No receiver available, wait a bit
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        });
    }

    /// Handle collaboration events and update UI
    async fn handle_collaboration_event(window: &MainWindow, event: CollaborationEvent) {
        match event {
            CollaborationEvent::UserJoined(session) => {
                // Update active users list
                Self::update_active_users(window, vec![session]).await;
            }
            CollaborationEvent::UserLeft(user_id) => {
                // Remove user from active users list
                println!("User left: {}", user_id);
            }
            CollaborationEvent::UserPresenceUpdate(update) => {
                // Update user presence indicators
                println!("User presence update: {:?}", update);
            }
            CollaborationEvent::SuggestionCreated(suggestion) => {
                // Add suggestion to UI
                Self::update_suggestions(window, vec![suggestion]).await;
            }
            CollaborationEvent::SuggestionUpdated(suggestion) => {
                // Update suggestion in UI
                Self::update_suggestions(window, vec![suggestion]).await;
            }
            CollaborationEvent::CommentAdded(comment) => {
                // Add comment to UI
                Self::update_comments(window, vec![comment]).await;
            }
            CollaborationEvent::CommentUpdated(comment) => {
                // Update comment in UI
                Self::update_comments(window, vec![comment]).await;
            }
            CollaborationEvent::ConflictDetected(conflict) => {
                // Show conflict notification
                println!("Conflict detected: {:?}", conflict);
            }
            CollaborationEvent::ConflictResolved(resolution) => {
                // Show conflict resolution
                println!("Conflict resolved: {:?}", resolution);
            }
            CollaborationEvent::DocumentChange(change) => {
                // Handle document change
                println!("Document change: {:?}", change);
            }
        }
    }

    /// Update active users in the UI
    async fn update_active_users(window: &MainWindow, sessions: Vec<UserSession>) {
        let users: Vec<SlintUserPresence> = sessions.into_iter().map(|session| {
            SlintUserPresence {
                user_id: session.user_id.into(),
                user_name: session.user_name.into(),
                user_role: format!("{:?}", session.user_role).into(),
                is_typing: session.is_typing,
                is_active: true,
            }
        }).collect();

        // Update the UI model (this would need to be implemented in the Slint interface)
        // window.set_active_users(ModelRc::new(VecModel::from(users)));
    }

    /// Update suggestions in the UI
    async fn update_suggestions(window: &MainWindow, suggestions: Vec<TranslationSuggestion>) {
        let slint_suggestions: Vec<SlintSuggestion> = suggestions.into_iter().map(|suggestion| {
            SlintSuggestion {
                id: suggestion.id.to_string().into(),
                author_name: suggestion.author_name.into(),
                original_text: suggestion.original_text.into(),
                suggested_text: suggestion.suggested_text.into(),
                reason: suggestion.reason.into(),
                status: format!("{:?}", suggestion.status).to_lowercase().into(),
                confidence_score: suggestion.confidence_score,
            }
        }).collect();

        // Update the UI model
        // window.set_suggestions(ModelRc::new(VecModel::from(slint_suggestions)));
    }

    /// Update comments in the UI
    async fn update_comments(window: &MainWindow, comments: Vec<Comment>) {
        let slint_comments: Vec<SlintComment> = comments.into_iter().map(|comment| {
            SlintComment {
                id: comment.id.to_string().into(),
                author_name: comment.author_name.into(),
                content: comment.content.into(),
                comment_type: format!("{:?}", comment.comment_type).to_lowercase().into(),
                timestamp: comment.created_at.format("%Y-%m-%d %H:%M").to_string().into(),
                is_resolved: comment.is_resolved,
                reply_count: comment.replies.len() as i32,
            }
        }).collect();

        // Update the UI model
        // window.set_comments(ModelRc::new(VecModel::from(slint_comments)));
    }

    /// Set current user context
    pub fn set_current_user(&self, user_id: String) {
        *self.current_user_id.lock().unwrap() = Some(user_id);
    }

    /// Set current project context
    pub fn set_current_project(&self, project_id: Uuid) {
        *self.current_project_id.lock().unwrap() = Some(project_id);
    }

    /// Set current chapter context
    pub fn set_current_chapter(&self, chapter_id: Uuid) {
        *self.current_chapter_id.lock().unwrap() = Some(chapter_id);
    }

    /// Set current language context
    pub fn set_current_language(&self, language: String) {
        *self.current_language.lock().unwrap() = language;
    }

    /// Join a collaboration session
    pub async fn join_session(&self, user_name: String, user_role: UserRole) -> Result<(), TradocumentError> {
        if let (Some(user_id), Some(project_id)) = (
            self.current_user_id.lock().unwrap().clone(),
            self.current_project_id.lock().unwrap().clone()
        ) {
            let session = UserSession {
                user_id,
                user_name,
                user_role,
                project_id,
                chapter_id: self.current_chapter_id.lock().unwrap().clone(),
                language: self.current_language.lock().unwrap().clone(),
                cursor_position: None,
                selection_range: None,
                last_activity: Utc::now(),
                is_typing: false,
                current_document: None,
            };

            self.collaboration_service.join_session(session)
                .map_err(|e| TradocumentError::ServiceError(format!("Failed to join session: {}", e)))?;
        }

        Ok(())
    }

    /// Leave the current collaboration session
    pub async fn leave_session(&self) -> Result<(), TradocumentError> {
        if let Some(user_id) = self.current_user_id.lock().unwrap().clone() {
            self.collaboration_service.leave_session(&user_id)
                .map_err(|e| TradocumentError::ServiceError(format!("Failed to leave session: {}", e)))?;
        }

        Ok(())
    }

    /// Update user presence (cursor position, typing status, etc.)
    pub async fn update_presence(&self, cursor_position: Option<usize>, is_typing: bool) -> Result<(), TradocumentError> {
        if let Some(user_id) = self.current_user_id.lock().unwrap().clone() {
            let update = UserPresenceUpdate {
                user_id,
                cursor_position,
                selection_range: None,
                is_typing,
                current_document: None,
                timestamp: Utc::now(),
            };

            self.collaboration_service.update_user_presence(update)
                .map_err(|e| TradocumentError::ServiceError(format!("Failed to update presence: {}", e)))?;
        }

        Ok(())
    }

    /// Track a document change
    pub async fn track_change(
        &self,
        change_type: ChangeType,
        content_before: Option<String>,
        content_after: Option<String>,
        position: usize,
        length: usize,
    ) -> Result<(), TradocumentError> {
        if let (Some(user_id), Some(project_id), Some(chapter_id)) = (
            self.current_user_id.lock().unwrap().clone(),
            self.current_project_id.lock().unwrap().clone(),
            self.current_chapter_id.lock().unwrap().clone()
        ) {
            let change = DocumentChange {
                id: Uuid::new_v4(),
                user_id: user_id.clone(),
                user_name: user_id, // Would be actual name
                project_id,
                chapter_id,
                language: self.current_language.lock().unwrap().clone(),
                change_type,
                content_before,
                content_after,
                position,
                length,
                timestamp: Utc::now(),
                is_conflicted: false,
                conflict_resolution: None,
            };

            self.collaboration_service.track_change(change)
                .map_err(|e| TradocumentError::ServiceError(format!("Failed to track change: {}", e)))?;
        }

        Ok(())
    }
}

/// Slint global for collaboration callbacks
slint::slint! {
    export global CollaborationCallbacks {
        callback accept-suggestion(string);
        callback reject-suggestion(string);
        callback vote-on-suggestion(string, string);
        callback resolve-comment(string);
        callback reply-to-comment(string);
        callback add-comment(string, string);
        callback create-suggestion(string, string, string);
    }
}