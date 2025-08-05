use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::{UserRole, Permission};

/// Real-time change tracking and conflict detection service
pub struct CollaborativeEditingService {
    /// Active user sessions
    user_sessions: Arc<Mutex<HashMap<String, UserSession>>>,
    /// Document change history
    change_history: Arc<Mutex<Vec<DocumentChange>>>,
    /// Active suggestions and reviews
    suggestions: Arc<Mutex<HashMap<Uuid, TranslationSuggestion>>>,
    /// Comments and annotations
    comments: Arc<Mutex<HashMap<Uuid, Comment>>>,
    /// Real-time event broadcaster
    event_sender: broadcast::Sender<CollaborationEvent>,
    /// Conflict detection engine
    conflict_detector: Arc<ConflictDetector>,
}

/// User session information for presence tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub user_id: String,
    pub user_name: String,
    pub user_role: UserRole,
    pub project_id: Uuid,
    pub chapter_id: Option<Uuid>,
    pub language: String,
    pub cursor_position: Option<usize>,
    pub selection_range: Option<SelectionRange>,
    pub last_activity: DateTime<Utc>,
    pub is_typing: bool,
    pub current_document: Option<String>,
}

/// Text selection range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionRange {
    pub start: usize,
    pub end: usize,
}

/// Document change for tracking modifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChange {
    pub id: Uuid,
    pub user_id: String,
    pub user_name: String,
    pub project_id: Uuid,
    pub chapter_id: Uuid,
    pub language: String,
    pub change_type: ChangeType,
    pub content_before: Option<String>,
    pub content_after: Option<String>,
    pub position: usize,
    pub length: usize,
    pub timestamp: DateTime<Utc>,
    pub is_conflicted: bool,
    pub conflict_resolution: Option<ConflictResolution>,
}

/// Types of document changes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeType {
    Insert,
    Delete,
    Replace,
    Move,
    Format,
}

/// Translation suggestion for collaborative review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationSuggestion {
    pub id: Uuid,
    pub author_id: String,
    pub author_name: String,
    pub project_id: Uuid,
    pub chapter_id: Uuid,
    pub unit_id: Uuid,
    pub language: String,
    pub original_text: String,
    pub suggested_text: String,
    pub reason: String,
    pub confidence_score: f32,
    pub status: SuggestionStatus,
    pub created_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub reviewer_id: Option<String>,
    pub reviewer_comments: Option<String>,
    pub votes: Vec<SuggestionVote>,
}

/// Status of a translation suggestion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SuggestionStatus {
    Pending,
    UnderReview,
    Accepted,
    Rejected,
    Superseded,
}

/// Vote on a translation suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionVote {
    pub user_id: String,
    pub user_name: String,
    pub vote_type: VoteType,
    pub comment: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Types of votes on suggestions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VoteType {
    Approve,
    Reject,
    NeedsWork,
}

/// Comment or annotation on content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: Uuid,
    pub author_id: String,
    pub author_name: String,
    pub project_id: Uuid,
    pub chapter_id: Uuid,
    pub language: Option<String>,
    pub content: String,
    pub comment_type: CommentType,
    pub context: CommentContext,
    pub position: Option<usize>,
    pub selection_range: Option<SelectionRange>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_resolved: bool,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub thread_id: Option<Uuid>,
    pub replies: Vec<CommentReply>,
}

/// Types of comments
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommentType {
    General,
    Suggestion,
    Question,
    Issue,
    Approval,
    Terminology,
    Grammar,
    Style,
}

/// Context for comments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommentContext {
    Document,
    Translation { unit_id: Uuid },
    Selection { start: usize, end: usize },
    Line { line_number: usize },
}

/// Reply to a comment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentReply {
    pub id: Uuid,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Real-time collaboration events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollaborationEvent {
    UserJoined(UserSession),
    UserLeft(String),
    UserPresenceUpdate(UserPresenceUpdate),
    DocumentChange(DocumentChange),
    SuggestionCreated(TranslationSuggestion),
    SuggestionUpdated(TranslationSuggestion),
    CommentAdded(Comment),
    CommentUpdated(Comment),
    ConflictDetected(ConflictNotification),
    ConflictResolved(ConflictResolution),
}

/// User presence update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPresenceUpdate {
    pub user_id: String,
    pub cursor_position: Option<usize>,
    pub selection_range: Option<SelectionRange>,
    pub is_typing: bool,
    pub current_document: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Conflict detection and resolution
#[derive(Debug, Clone)]
pub struct ConflictDetector {
    /// Pending changes that might conflict
    pending_changes: Arc<Mutex<Vec<DocumentChange>>>,
}

/// Conflict notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictNotification {
    pub id: Uuid,
    pub conflicting_changes: Vec<Uuid>,
    pub affected_users: Vec<String>,
    pub conflict_type: ConflictType,
    pub suggested_resolution: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Types of conflicts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictType {
    OverlappingEdits,
    SimultaneousChanges,
    VersionMismatch,
    PermissionConflict,
}

/// Conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub id: Uuid,
    pub conflict_id: Uuid,
    pub resolution_type: ResolutionType,
    pub resolved_by: String,
    pub resolution_content: String,
    pub affected_changes: Vec<Uuid>,
    pub resolved_at: DateTime<Utc>,
}

/// Types of conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResolutionType {
    AcceptMine,
    AcceptTheirs,
    Merge,
    Manual,
}

impl CollaborativeEditingService {
    /// Create a new collaborative editing service
    pub fn new() -> Self {
        let (event_sender, _) = broadcast::channel(1000);
        
        Self {
            user_sessions: Arc::new(Mutex::new(HashMap::new())),
            change_history: Arc::new(Mutex::new(Vec::new())),
            suggestions: Arc::new(Mutex::new(HashMap::new())),
            comments: Arc::new(Mutex::new(HashMap::new())),
            event_sender,
            conflict_detector: Arc::new(ConflictDetector::new()),
        }
    }

    /// Register a user session for presence tracking
    pub fn join_session(&self, session: UserSession) -> Result<(), CollaborationError> {
        let mut sessions = self.user_sessions.lock()
            .map_err(|_| CollaborationError::LockError("Failed to acquire sessions lock".to_string()))?;
        
        sessions.insert(session.user_id.clone(), session.clone());
        
        // Broadcast user joined event
        let _ = self.event_sender.send(CollaborationEvent::UserJoined(session));
        
        Ok(())
    }

    /// Remove a user session
    pub fn leave_session(&self, user_id: &str) -> Result<(), CollaborationError> {
        let mut sessions = self.user_sessions.lock()
            .map_err(|_| CollaborationError::LockError("Failed to acquire sessions lock".to_string()))?;
        
        sessions.remove(user_id);
        
        // Broadcast user left event
        let _ = self.event_sender.send(CollaborationEvent::UserLeft(user_id.to_string()));
        
        Ok(())
    }

    /// Update user presence information
    pub fn update_user_presence(&self, update: UserPresenceUpdate) -> Result<(), CollaborationError> {
        let mut sessions = self.user_sessions.lock()
            .map_err(|_| CollaborationError::LockError("Failed to acquire sessions lock".to_string()))?;
        
        if let Some(session) = sessions.get_mut(&update.user_id) {
            session.cursor_position = update.cursor_position;
            session.selection_range = update.selection_range.clone();
            session.is_typing = update.is_typing;
            session.current_document = update.current_document.clone();
            session.last_activity = update.timestamp;
        }
        
        // Broadcast presence update
        let _ = self.event_sender.send(CollaborationEvent::UserPresenceUpdate(update));
        
        Ok(())
    }

    /// Get active users in a project
    pub fn get_active_users(&self, project_id: Uuid) -> Result<Vec<UserSession>, CollaborationError> {
        let sessions = self.user_sessions.lock()
            .map_err(|_| CollaborationError::LockError("Failed to acquire sessions lock".to_string()))?;
        
        let active_users: Vec<UserSession> = sessions.values()
            .filter(|session| session.project_id == project_id)
            .cloned()
            .collect();
        
        Ok(active_users)
    }

    /// Track a document change
    pub fn track_change(&self, change: DocumentChange) -> Result<(), CollaborationError> {
        // Check for conflicts before recording the change
        let conflicts = self.conflict_detector.detect_conflicts(&change)?;
        
        let mut change = change;
        change.is_conflicted = !conflicts.is_empty();
        
        // Record the change
        let mut history = self.change_history.lock()
            .map_err(|_| CollaborationError::LockError("Failed to acquire history lock".to_string()))?;
        
        history.push(change.clone());
        
        // Broadcast change event
        let _ = self.event_sender.send(CollaborationEvent::DocumentChange(change));
        
        // Handle conflicts if detected
        for conflict in conflicts {
            let _ = self.event_sender.send(CollaborationEvent::ConflictDetected(conflict));
        }
        
        Ok(())
    }

    /// Create a translation suggestion
    pub fn create_suggestion(&self, suggestion: TranslationSuggestion) -> Result<Uuid, CollaborationError> {
        let mut suggestions = self.suggestions.lock()
            .map_err(|_| CollaborationError::LockError("Failed to acquire suggestions lock".to_string()))?;
        
        let suggestion_id = suggestion.id;
        suggestions.insert(suggestion_id, suggestion.clone());
        
        // Broadcast suggestion created event
        let _ = self.event_sender.send(CollaborationEvent::SuggestionCreated(suggestion));
        
        Ok(suggestion_id)
    }

    /// Update a translation suggestion
    pub fn update_suggestion(&self, suggestion_id: Uuid, updates: SuggestionUpdate) -> Result<(), CollaborationError> {
        let mut suggestions = self.suggestions.lock()
            .map_err(|_| CollaborationError::LockError("Failed to acquire suggestions lock".to_string()))?;
        
        if let Some(suggestion) = suggestions.get_mut(&suggestion_id) {
            if let Some(status) = updates.status {
                suggestion.status = status;
            }
            if let Some(reviewer_id) = updates.reviewer_id {
                suggestion.reviewer_id = Some(reviewer_id);
                suggestion.reviewed_at = Some(Utc::now());
            }
            if let Some(comments) = updates.reviewer_comments {
                suggestion.reviewer_comments = Some(comments);
            }
            
            // Broadcast suggestion updated event
            let _ = self.event_sender.send(CollaborationEvent::SuggestionUpdated(suggestion.clone()));
        }
        
        Ok(())
    }

    /// Vote on a translation suggestion
    pub fn vote_on_suggestion(&self, suggestion_id: Uuid, vote: SuggestionVote) -> Result<(), CollaborationError> {
        let mut suggestions = self.suggestions.lock()
            .map_err(|_| CollaborationError::LockError("Failed to acquire suggestions lock".to_string()))?;
        
        if let Some(suggestion) = suggestions.get_mut(&suggestion_id) {
            // Remove any existing vote from this user
            suggestion.votes.retain(|v| v.user_id != vote.user_id);
            
            // Add the new vote
            suggestion.votes.push(vote);
            
            // Broadcast suggestion updated event
            let _ = self.event_sender.send(CollaborationEvent::SuggestionUpdated(suggestion.clone()));
        }
        
        Ok(())
    }

    /// Add a comment or annotation
    pub fn add_comment(&self, comment: Comment) -> Result<Uuid, CollaborationError> {
        let mut comments = self.comments.lock()
            .map_err(|_| CollaborationError::LockError("Failed to acquire comments lock".to_string()))?;
        
        let comment_id = comment.id;
        comments.insert(comment_id, comment.clone());
        
        // Broadcast comment added event
        let _ = self.event_sender.send(CollaborationEvent::CommentAdded(comment));
        
        Ok(comment_id)
    }

    /// Reply to a comment
    pub fn reply_to_comment(&self, comment_id: Uuid, reply: CommentReply) -> Result<(), CollaborationError> {
        let mut comments = self.comments.lock()
            .map_err(|_| CollaborationError::LockError("Failed to acquire comments lock".to_string()))?;
        
        if let Some(comment) = comments.get_mut(&comment_id) {
            comment.replies.push(reply);
            comment.updated_at = Utc::now();
            
            // Broadcast comment updated event
            let _ = self.event_sender.send(CollaborationEvent::CommentUpdated(comment.clone()));
        }
        
        Ok(())
    }

    /// Resolve a comment
    pub fn resolve_comment(&self, comment_id: Uuid, resolved_by: String) -> Result<(), CollaborationError> {
        let mut comments = self.comments.lock()
            .map_err(|_| CollaborationError::LockError("Failed to acquire comments lock".to_string()))?;
        
        if let Some(comment) = comments.get_mut(&comment_id) {
            comment.is_resolved = true;
            comment.resolved_by = Some(resolved_by);
            comment.resolved_at = Some(Utc::now());
            comment.updated_at = Utc::now();
            
            // Broadcast comment updated event
            let _ = self.event_sender.send(CollaborationEvent::CommentUpdated(comment.clone()));
        }
        
        Ok(())
    }

    /// Get comments for a chapter
    pub fn get_comments(&self, chapter_id: Uuid, language: Option<String>) -> Result<Vec<Comment>, CollaborationError> {
        let comments = self.comments.lock()
            .map_err(|_| CollaborationError::LockError("Failed to acquire comments lock".to_string()))?;
        
        let filtered_comments: Vec<Comment> = comments.values()
            .filter(|comment| {
                comment.chapter_id == chapter_id &&
                (language.is_none() || comment.language == language)
            })
            .cloned()
            .collect();
        
        Ok(filtered_comments)
    }

    /// Get suggestions for a chapter
    pub fn get_suggestions(&self, chapter_id: Uuid, language: String) -> Result<Vec<TranslationSuggestion>, CollaborationError> {
        let suggestions = self.suggestions.lock()
            .map_err(|_| CollaborationError::LockError("Failed to acquire suggestions lock".to_string()))?;
        
        let filtered_suggestions: Vec<TranslationSuggestion> = suggestions.values()
            .filter(|suggestion| {
                suggestion.chapter_id == chapter_id && suggestion.language == language
            })
            .cloned()
            .collect();
        
        Ok(filtered_suggestions)
    }

    /// Subscribe to collaboration events
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<CollaborationEvent> {
        self.event_sender.subscribe()
    }

    /// Get change history for a document
    pub fn get_change_history(&self, chapter_id: Uuid, language: String, limit: Option<usize>) -> Result<Vec<DocumentChange>, CollaborationError> {
        let history = self.change_history.lock()
            .map_err(|_| CollaborationError::LockError("Failed to acquire history lock".to_string()))?;
        
        let mut filtered_changes: Vec<DocumentChange> = history.iter()
            .filter(|change| change.chapter_id == chapter_id && change.language == language)
            .cloned()
            .collect();
        
        // Sort by timestamp (most recent first)
        filtered_changes.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        // Apply limit if specified
        if let Some(limit) = limit {
            filtered_changes.truncate(limit);
        }
        
        Ok(filtered_changes)
    }
}

/// Update parameters for suggestions
#[derive(Debug, Clone)]
pub struct SuggestionUpdate {
    pub status: Option<SuggestionStatus>,
    pub reviewer_id: Option<String>,
    pub reviewer_comments: Option<String>,
}

impl ConflictDetector {
    /// Create a new conflict detector
    pub fn new() -> Self {
        Self {
            pending_changes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Detect conflicts for a new change
    pub fn detect_conflicts(&self, new_change: &DocumentChange) -> Result<Vec<ConflictNotification>, CollaborationError> {
        let pending = self.pending_changes.lock()
            .map_err(|_| CollaborationError::LockError("Failed to acquire pending changes lock".to_string()))?;
        
        let mut conflicts = Vec::new();
        
        for existing_change in pending.iter() {
            if self.changes_conflict(existing_change, new_change) {
                let conflict = ConflictNotification {
                    id: Uuid::new_v4(),
                    conflicting_changes: vec![existing_change.id, new_change.id],
                    affected_users: vec![existing_change.user_id.clone(), new_change.user_id.clone()],
                    conflict_type: self.determine_conflict_type(existing_change, new_change),
                    suggested_resolution: self.suggest_resolution(existing_change, new_change),
                    created_at: Utc::now(),
                };
                conflicts.push(conflict);
            }
        }
        
        Ok(conflicts)
    }

    /// Check if two changes conflict
    fn changes_conflict(&self, change1: &DocumentChange, change2: &DocumentChange) -> bool {
        // Same document and language
        if change1.chapter_id != change2.chapter_id || change1.language != change2.language {
            return false;
        }

        // Same user doesn't conflict with themselves
        if change1.user_id == change2.user_id {
            return false;
        }

        // Check for overlapping positions
        let range1_start = change1.position;
        let range1_end = change1.position + change1.length;
        let range2_start = change2.position;
        let range2_end = change2.position + change2.length;

        // Ranges overlap if they intersect
        !(range1_end <= range2_start || range2_end <= range1_start)
    }

    /// Determine the type of conflict
    fn determine_conflict_type(&self, _change1: &DocumentChange, _change2: &DocumentChange) -> ConflictType {
        // For now, assume overlapping edits
        ConflictType::OverlappingEdits
    }

    /// Suggest a resolution for the conflict
    fn suggest_resolution(&self, _change1: &DocumentChange, _change2: &DocumentChange) -> Option<String> {
        Some("Manual review required - changes overlap in the same text region".to_string())
    }
}

/// Errors that can occur during collaboration
#[derive(Debug, thiserror::Error)]
pub enum CollaborationError {
    #[error("Lock error: {0}")]
    LockError(String),
    
    #[error("User not found: {0}")]
    UserNotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Suggestion not found: {0}")]
    SuggestionNotFound(Uuid),
    
    #[error("Comment not found: {0}")]
    CommentNotFound(Uuid),
    
    #[error("Conflict resolution failed: {0}")]
    ConflictResolutionFailed(String),
    
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

impl Default for CollaborativeEditingService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_creation() {
        let service = CollaborativeEditingService::new();
        assert!(service.get_active_users(Uuid::new_v4()).unwrap().is_empty());
    }

    #[test]
    fn test_user_session_management() {
        let service = CollaborativeEditingService::new();
        let project_id = Uuid::new_v4();
        
        let session = UserSession {
            user_id: "user1".to_string(),
            user_name: "Test User".to_string(),
            user_role: UserRole::Translator,
            project_id,
            chapter_id: None,
            language: "en".to_string(),
            cursor_position: None,
            selection_range: None,
            last_activity: Utc::now(),
            is_typing: false,
            current_document: None,
        };

        service.join_session(session).unwrap();
        
        let active_users = service.get_active_users(project_id).unwrap();
        assert_eq!(active_users.len(), 1);
        assert_eq!(active_users[0].user_id, "user1");

        service.leave_session("user1").unwrap();
        
        let active_users = service.get_active_users(project_id).unwrap();
        assert!(active_users.is_empty());
    }

    #[test]
    fn test_suggestion_workflow() {
        let service = CollaborativeEditingService::new();
        let project_id = Uuid::new_v4();
        let chapter_id = Uuid::new_v4();
        let unit_id = Uuid::new_v4();
        
        let suggestion = TranslationSuggestion {
            id: Uuid::new_v4(),
            author_id: "user1".to_string(),
            author_name: "Test User".to_string(),
            project_id,
            chapter_id,
            unit_id,
            language: "de".to_string(),
            original_text: "Hello".to_string(),
            suggested_text: "Hallo".to_string(),
            reason: "Better translation".to_string(),
            confidence_score: 0.9,
            status: SuggestionStatus::Pending,
            created_at: Utc::now(),
            reviewed_at: None,
            reviewer_id: None,
            reviewer_comments: None,
            votes: Vec::new(),
        };

        let suggestion_id = service.create_suggestion(suggestion).unwrap();
        
        let suggestions = service.get_suggestions(chapter_id, "de".to_string()).unwrap();
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].id, suggestion_id);

        // Test voting
        let vote = SuggestionVote {
            user_id: "reviewer1".to_string(),
            user_name: "Reviewer".to_string(),
            vote_type: VoteType::Approve,
            comment: Some("Good suggestion".to_string()),
            timestamp: Utc::now(),
        };

        service.vote_on_suggestion(suggestion_id, vote).unwrap();
        
        let suggestions = service.get_suggestions(chapter_id, "de".to_string()).unwrap();
        assert_eq!(suggestions[0].votes.len(), 1);
        assert_eq!(suggestions[0].votes[0].vote_type, VoteType::Approve);
    }

    #[test]
    fn test_comment_system() {
        let service = CollaborativeEditingService::new();
        let project_id = Uuid::new_v4();
        let chapter_id = Uuid::new_v4();
        
        let comment = Comment {
            id: Uuid::new_v4(),
            author_id: "user1".to_string(),
            author_name: "Test User".to_string(),
            project_id,
            chapter_id,
            language: Some("en".to_string()),
            content: "This needs clarification".to_string(),
            comment_type: CommentType::Question,
            context: CommentContext::Document,
            position: Some(100),
            selection_range: Some(SelectionRange { start: 100, end: 120 }),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            is_resolved: false,
            resolved_by: None,
            resolved_at: None,
            thread_id: None,
            replies: Vec::new(),
        };

        let comment_id = service.add_comment(comment).unwrap();
        
        let comments = service.get_comments(chapter_id, Some("en".to_string())).unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].id, comment_id);

        // Test reply
        let reply = CommentReply {
            id: Uuid::new_v4(),
            author_id: "user2".to_string(),
            author_name: "Another User".to_string(),
            content: "I can help with that".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        service.reply_to_comment(comment_id, reply).unwrap();
        
        let comments = service.get_comments(chapter_id, Some("en".to_string())).unwrap();
        assert_eq!(comments[0].replies.len(), 1);

        // Test resolution
        service.resolve_comment(comment_id, "user2".to_string()).unwrap();
        
        let comments = service.get_comments(chapter_id, Some("en".to_string())).unwrap();
        assert!(comments[0].is_resolved);
        assert_eq!(comments[0].resolved_by, Some("user2".to_string()));
    }

    #[test]
    fn test_conflict_detection() {
        let detector = ConflictDetector::new();
        
        let change1 = DocumentChange {
            id: Uuid::new_v4(),
            user_id: "user1".to_string(),
            user_name: "User 1".to_string(),
            project_id: Uuid::new_v4(),
            chapter_id: Uuid::new_v4(),
            language: "en".to_string(),
            change_type: ChangeType::Replace,
            content_before: Some("old text".to_string()),
            content_after: Some("new text".to_string()),
            position: 100,
            length: 8,
            timestamp: Utc::now(),
            is_conflicted: false,
            conflict_resolution: None,
        };

        let change2 = DocumentChange {
            id: Uuid::new_v4(),
            user_id: "user2".to_string(),
            user_name: "User 2".to_string(),
            project_id: change1.project_id,
            chapter_id: change1.chapter_id,
            language: "en".to_string(),
            change_type: ChangeType::Replace,
            content_before: Some("old text".to_string()),
            content_after: Some("different text".to_string()),
            position: 105, // Overlaps with change1
            length: 5,
            timestamp: Utc::now(),
            is_conflicted: false,
            conflict_resolution: None,
        };

        assert!(detector.changes_conflict(&change1, &change2));
        
        let conflicts = detector.detect_conflicts(&change2).unwrap();
        // No conflicts detected because change1 is not in pending changes
        assert!(conflicts.is_empty());
    }
}