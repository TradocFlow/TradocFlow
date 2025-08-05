//! Comment System with Threading Support
//! 
//! Provides comprehensive comment management with threading, collaboration,
//! and Git integration for translation projects according to PRD specifications.

use super::{
    GitWorkflowManager, models::{
        Comment, CommentType, CommentContext,
        ChapterData
    }
};
use crate::{Result, TradocumentError, User};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Enhanced comment system with full threading and collaboration support
#[derive(Debug)]
pub struct CommentSystem {
    repo_path: String,
    current_user: User,
    // Thread cache for performance
    thread_cache: Arc<RwLock<HashMap<String, CommentThread>>>,
    // User mentions tracking
    mention_tracker: Arc<RwLock<HashMap<String, Vec<String>>>>, // comment_id -> user_ids
}

/// Enhanced comment thread with full threading support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentThread {
    pub id: String,
    pub root_comment_id: String,
    pub context: ThreadContext,
    pub status: ThreadStatus,
    pub participants: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub priority: ThreadPriority,
    pub tags: Vec<String>,
    pub metadata: ThreadMetadata,
}

/// Enhanced comment with threading and context support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadedComment {
    pub id: String,
    pub thread_id: Option<String>,
    pub parent_id: Option<String>, // For nested replies
    pub author: String,
    pub content: String,
    pub comment_type: CommentType,
    pub context: CommentContext,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub edited: bool,
    pub resolved: bool,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub replies: Vec<ThreadedComment>, // Nested structure for display
    pub mentions: Vec<String>, // User IDs mentioned in comment
    pub attachments: Vec<CommentAttachment>,
    pub reactions: HashMap<String, Vec<String>>, // reaction -> user_ids
    pub position: Option<CommentPosition>,
    pub metadata: CommentMetadata,
}

/// Thread context linking to specific content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ThreadContext {
    #[serde(rename = "translation")]
    Translation { 
        unit_id: String, 
        language: String,
        chapter: String,
        line_number: Option<u32>,
        text_range: Option<TextRange>,
    },
    #[serde(rename = "paragraph")]
    Paragraph { 
        unit_id: String,
        chapter: String,
        line_number: Option<u32>,
    },
    #[serde(rename = "chapter")]
    Chapter { 
        chapter: String,
        section: Option<String>,
    },
    #[serde(rename = "project")]
    Project {
        topic: Option<String>,
    },
}

/// Thread status for workflow management
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ThreadStatus {
    Open,
    InProgress,
    Resolved,
    Closed,
    Archived,
}

/// Thread priority for organization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ThreadPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Thread metadata for additional information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadMetadata {
    pub assignee: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub estimated_time: Option<u32>, // minutes
    pub actual_time: Option<u32>, // minutes
    pub labels: Vec<String>,
    pub external_refs: Vec<ExternalReference>,
}

/// Text range for precise positioning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRange {
    pub start: u32,
    pub end: u32,
}

/// Comment position for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentPosition {
    pub line: u32,
    pub column: u32,
    pub offset: u32,
}

/// Comment attachment support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentAttachment {
    pub id: String,
    pub filename: String,
    pub mime_type: String,
    pub size: u64,
    pub url: String,
    pub uploaded_at: DateTime<Utc>,
}

/// Comment metadata for additional features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentMetadata {
    pub edited_at: Option<DateTime<Utc>>,
    pub edit_history: Vec<CommentEdit>,
    pub language: Option<String>,
    pub sentiment: Option<CommentSentiment>,
    pub confidence: Option<f32>,
    pub source: CommentSource,
}

/// Comment edit history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentEdit {
    pub timestamp: DateTime<Utc>,
    pub editor: String,
    pub previous_content: String,
    pub reason: Option<String>,
}

/// Comment sentiment analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommentSentiment {
    Positive,
    Neutral,
    Negative,
}

/// Comment source tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommentSource {
    Manual,
    Import,
    Api,
    Integration,
}

/// External reference linking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalReference {
    pub ref_type: String, // "issue", "ticket", "doc", etc.
    pub ref_id: String,
    pub url: Option<String>,
    pub title: Option<String>,
}

/// Request to create a new comment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
    pub comment_type: CommentType,
    pub context: CommentContext,
    pub thread_id: Option<String>,
    pub parent_id: Option<String>,
    pub mentions: Option<Vec<String>>,
    pub attachments: Option<Vec<CommentAttachment>>,
    pub position: Option<CommentPosition>,
}

/// Request to create a new thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateThreadRequest {
    pub context: ThreadContext,
    pub initial_comment: CreateCommentRequest,
    pub priority: Option<ThreadPriority>,
    pub assignee: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub tags: Option<Vec<String>>,
    pub labels: Option<Vec<String>>,
}

/// Request to update a comment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCommentRequest {
    pub content: Option<String>,
    pub comment_type: Option<CommentType>,
    pub resolved: Option<bool>,
    pub mentions: Option<Vec<String>>,
    pub reason: Option<String>,
}

/// Request to update a thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateThreadRequest {
    pub status: Option<ThreadStatus>,
    pub priority: Option<ThreadPriority>,
    pub assignee: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub tags: Option<Vec<String>>,
    pub labels: Option<Vec<String>>,
}

/// Comment search and filter options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct CommentFilter {
    pub author: Option<String>,
    pub comment_type: Option<CommentType>,
    pub context_type: Option<String>,
    pub thread_id: Option<String>,
    pub thread_status: Option<ThreadStatus>,
    pub resolved: Option<bool>,
    pub mentions: Option<String>, // User mentioned in comments
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub content_search: Option<String>,
    pub tags: Option<Vec<String>>,
    pub priority: Option<ThreadPriority>,
    pub language: Option<String>,
    pub chapter: Option<String>,
    pub unit_id: Option<String>,
}

/// Search results with pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentSearchResults {
    pub comments: Vec<ThreadedComment>,
    pub threads: Vec<CommentThread>,
    pub total_count: u32,
    pub page: u32,
    pub per_page: u32,
    pub has_more: bool,
}

/// Comment notification event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentNotification {
    pub event_type: CommentEventType,
    pub comment_id: String,
    pub thread_id: Option<String>,
    pub actor: String,
    pub affected_users: Vec<String>,
    pub content_preview: String,
    pub context: CommentContext,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Types of comment events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommentEventType {
    CommentCreated,
    CommentUpdated,
    CommentDeleted,
    CommentResolved,
    CommentReplied,
    CommentMentioned,
    CommentReaction,
    ThreadCreated,
    ThreadUpdated,
    ThreadResolved,
    ThreadClosed,
    ThreadAssigned,
}

/// User role permissions for comments
#[derive(Debug, Clone)]
pub enum CommentPermission {
    Read,
    Create,
    Edit,
    Delete,
    Resolve,
    Moderate,
    Admin,
}

impl CommentSystem {
    /// Create a new comment system instance
    pub async fn new(
        _git_manager: Arc<GitWorkflowManager>,
        _project_id: Uuid,
        repo_path: &str,
        current_user: User,
    ) -> Result<Self> {
        Ok(Self {
            repo_path: repo_path.to_string(),
            current_user,
            thread_cache: Arc::new(RwLock::new(HashMap::new())),
            mention_tracker: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create a new comment thread
    pub async fn create_thread(
        &self,
        request: CreateThreadRequest,
    ) -> Result<(CommentThread, ThreadedComment)> {
        // Validate permissions
        // TODO: Implement permission validation for thread creation
        // self.validate_create_permission(&request.context).await?;

        // Generate IDs
        let thread_id = Uuid::new_v4().to_string();
        let comment_id = Uuid::new_v4().to_string();

        // Create thread
        let thread = CommentThread {
            id: thread_id.clone(),
            root_comment_id: comment_id.clone(),
            context: request.context.clone(),
            status: ThreadStatus::Open,
            participants: vec![self.current_user.id.clone()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            resolved_by: None,
            resolved_at: None,
            priority: request.priority.unwrap_or(ThreadPriority::Normal),
            tags: request.tags.unwrap_or_default(),
            metadata: ThreadMetadata {
                assignee: request.assignee,
                due_date: request.due_date,
                estimated_time: None,
                actual_time: None,
                labels: request.labels.unwrap_or_default(),
                external_refs: Vec::new(),
            },
        };

        // Create initial comment
        let comment = ThreadedComment {
            id: comment_id.clone(),
            thread_id: Some(thread_id.clone()),
            parent_id: None,
            author: self.current_user.id.clone(),
            content: request.initial_comment.content.clone(),
            comment_type: request.initial_comment.comment_type,
            context: request.initial_comment.context.clone(),
            created_at: Utc::now(),
            updated_at: None,
            edited: false,
            resolved: false,
            resolved_by: None,
            resolved_at: None,
            replies: Vec::new(),
            mentions: request.initial_comment.mentions.unwrap_or_default(),
            attachments: request.initial_comment.attachments.unwrap_or_default(),
            reactions: HashMap::new(),
            position: request.initial_comment.position,
            metadata: CommentMetadata {
                edited_at: None,
                edit_history: Vec::new(),
                language: self.detect_language(&request.initial_comment.content),
                sentiment: None,
                confidence: None,
                source: CommentSource::Manual,
            },
        };

        // Save to TOML
        self.add_thread_to_toml(&thread).await?;
        self.add_comment_to_toml(&comment).await?;

        // Update mention tracking
        self.update_mention_tracking(&comment).await?;

        // Cache the thread
        {
            let mut cache = self.thread_cache.write().await;
            cache.insert(thread_id.clone(), thread.clone());
        }

        // Create Git commit
        self.commit_comment_operation(
            &format!("comment: create {} thread", self.format_comment_type(&comment.comment_type)),
            &format!(
                "Created new comment thread\n\n\
                 Type: {:?}\n\
                 Context: {}\n\
                 Content preview: {}\n\n\
                 Author: {}\n\
                 Thread-ID: {}\n\
                 Comment-ID: {}",
                comment.comment_type,
                self.format_thread_context(&thread.context),
                self.truncate_content(&comment.content, 100),
                comment.author,
                thread.id,
                comment.id
            ),
        ).await?;

        // Send notifications
        self.send_comment_notification(
            CommentEventType::ThreadCreated,
            &comment,
            Some(&thread),
        ).await?;

        Ok((thread, comment))
    }

    /// Add a comment to an existing thread
    pub async fn add_comment(
        &self,
        request: CreateCommentRequest,
    ) -> Result<ThreadedComment> {
        // Validate context and permissions
        self.validate_create_permission(&request.context).await?;

        let comment_id = Uuid::new_v4().to_string();

        // If thread_id is provided, validate it exists
        let thread_id = if let Some(ref tid) = request.thread_id {
            self.validate_thread_exists(tid).await?;
            Some(tid.clone())
        } else {
            None
        };

        // If parent_id is provided, validate it exists
        if let Some(ref parent_id) = request.parent_id {
            self.validate_comment_exists(parent_id).await?;
        }

        let comment = ThreadedComment {
            id: comment_id.clone(),
            thread_id: thread_id.clone(),
            parent_id: request.parent_id.clone(),
            author: self.current_user.id.clone(),
            content: request.content.clone(),
            comment_type: request.comment_type,
            context: request.context.clone(),
            created_at: Utc::now(),
            updated_at: None,
            edited: false,
            resolved: false,
            resolved_by: None,
            resolved_at: None,
            replies: Vec::new(),
            mentions: request.mentions.unwrap_or_default(),
            attachments: request.attachments.unwrap_or_default(),
            reactions: HashMap::new(),
            position: request.position,
            metadata: CommentMetadata {
                edited_at: None,
                edit_history: Vec::new(),
                language: self.detect_language(&request.content),
                sentiment: None,
                confidence: None,
                source: CommentSource::Manual,
            },
        };

        // Save to TOML
        self.add_comment_to_toml(&comment).await?;

        // Update thread participants if thread exists
        if let Some(ref tid) = thread_id {
            self.update_thread_participants(tid, &comment.author).await?;
        }

        // Update mention tracking
        self.update_mention_tracking(&comment).await?;

        // Create Git commit
        let event_type = if request.parent_id.is_some() {
            "reply to comment"
        } else {
            "add comment"
        };

        self.commit_comment_operation(
            &format!("comment: {event_type}"),
            &format!(
                "Added comment to thread\n\n\
                 Type: {:?}\n\
                 Thread ID: {}\n\
                 Parent ID: {}\n\
                 Content preview: {}\n\n\
                 Author: {}\n\
                 Comment-ID: {}",
                comment.comment_type,
                thread_id.clone().unwrap_or_else(|| "none".to_string()),
                request.parent_id.clone().unwrap_or_else(|| "none".to_string()),
                self.truncate_content(&comment.content, 100),
                comment.author,
                comment.id
            ),
        ).await?;

        // Send notifications
        let notification_type = if request.parent_id.is_some() {
            CommentEventType::CommentReplied
        } else {
            CommentEventType::CommentCreated
        };

        let thread = if let Some(ref tid) = thread_id {
            self.get_thread(tid).await.ok()
        } else {
            None
        };

        self.send_comment_notification(
            notification_type,
            &comment,
            thread.as_ref(),
        ).await?;

        Ok(comment)
    }

    /// Update an existing comment
    pub async fn update_comment(
        &self,
        comment_id: &str,
        request: UpdateCommentRequest,
    ) -> Result<ThreadedComment> {
        // Load current comment
        let mut comment = self.get_comment(comment_id).await?;
        
        // Validate permissions
        self.validate_edit_permission(&comment).await?;

        // Track changes for commit message
        let mut changes = Vec::new();
        let mut content_changed = false;

        // Apply updates
        if let Some(content) = request.content {
            if content != comment.content {
                // Store edit history
                let edit = CommentEdit {
                    timestamp: Utc::now(),
                    editor: self.current_user.id.clone(),
                    previous_content: comment.content.clone(),
                    reason: request.reason.clone(),
                };
                comment.metadata.edit_history.push(edit);
                
                comment.content = content;
                comment.updated_at = Some(Utc::now());
                comment.edited = true;
                comment.metadata.edited_at = Some(Utc::now());
                
                changes.push("updated content".to_string());
                content_changed = true;
            }
        }

        if let Some(comment_type) = request.comment_type {
            if comment_type != comment.comment_type {
                changes.push(format!("type: {:?} → {:?}", comment.comment_type, comment_type));
                comment.comment_type = comment_type;
            }
        }

        if let Some(resolved) = request.resolved {
            if resolved != comment.resolved {
                comment.resolved = resolved;
                if resolved {
                    comment.resolved_by = Some(self.current_user.id.clone());
                    comment.resolved_at = Some(Utc::now());
                    changes.push("resolved".to_string());
                } else {
                    comment.resolved_by = None;
                    comment.resolved_at = None;
                    changes.push("reopened".to_string());
                }
            }
        }

        if let Some(mentions) = request.mentions {
            comment.mentions = mentions;
            changes.push("updated mentions".to_string());
        }

        // Update language detection if content changed
        if content_changed {
            comment.metadata.language = self.detect_language(&comment.content);
        }

        // Update TOML file
        self.update_comment_in_toml(&comment).await?;

        // Update mention tracking
        self.update_mention_tracking(&comment).await?;

        // Create Git commit
        if !changes.is_empty() {
            self.commit_comment_operation(
                &format!("comment: update comment {comment_id}"),
                &format!(
                    "Updated comment\n\n\
                     Changes:\n{}\n\
                     Reason: {}\n\n\
                     Updated-By: {}\n\
                     Comment-ID: {}",
                    changes.into_iter().map(|c| format!("- {c}")).collect::<Vec<_>>().join("\n"),
                    request.reason.unwrap_or_else(|| "No reason provided".to_string()),
                    self.current_user.id,
                    comment.id
                ),
            ).await?;
        }

        // Send notifications
        let notification_type = if request.resolved == Some(true) {
            CommentEventType::CommentResolved
        } else {
            CommentEventType::CommentUpdated
        };

        let thread = if let Some(ref tid) = comment.thread_id {
            self.get_thread(tid).await.ok()
        } else {
            None
        };

        self.send_comment_notification(
            notification_type,
            &comment,
            thread.as_ref(),
        ).await?;

        Ok(comment)
    }

    /// Get a specific comment by ID
    pub async fn get_comment(&self, comment_id: &str) -> Result<ThreadedComment> {
        self.find_comment_in_toml(comment_id).await?.ok_or_else(|| {
            TradocumentError::ApiError(format!("Comment {comment_id} not found"))
        })
    }

    /// Get a specific thread by ID
    pub async fn get_thread(&self, thread_id: &str) -> Result<CommentThread> {
        // Check cache first
        {
            let cache = self.thread_cache.read().await;
            if let Some(thread) = cache.get(thread_id) {
                return Ok(thread.clone());
            }
        }

        // Load from TOML
        if let Some(thread) = self.find_thread_in_toml(thread_id).await? {
            // Cache the thread
            {
                let mut cache = self.thread_cache.write().await;
                cache.insert(thread_id.to_string(), thread.clone());
            }
            Ok(thread)
        } else {
            Err(TradocumentError::ApiError(
                format!("Thread {thread_id} not found")
            ))
        }
    }

    /// Get all comments in a thread with proper nesting
    pub async fn get_thread_comments(&self, thread_id: &str) -> Result<Vec<ThreadedComment>> {
        // Validate thread exists
        self.validate_thread_exists(thread_id).await?;

        // Load all comments for this thread
        let mut comments = self.find_comments_by_thread(thread_id).await?;

        // Sort by creation time
        comments.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        // Build nested structure
        self.build_comment_tree(comments)
    }

    /// Search comments and threads with advanced filtering
    pub async fn search_comments(
        &self,
        filter: CommentFilter,
        page: u32,
        per_page: u32,
    ) -> Result<CommentSearchResults> {
        let mut all_comments = self.load_all_comments().await?;
        let mut all_threads = self.load_all_threads().await?;

        // Apply filters
        all_comments.retain(|comment| self.matches_comment_filter(comment, &filter));

        all_threads.retain(|thread| self.matches_thread_filter(thread, &filter));

        // Calculate pagination
        let total_count = all_comments.len() as u32;
        let start = ((page - 1) * per_page) as usize;
        let end = (start + per_page as usize).min(all_comments.len());
        
        let comments = if start < all_comments.len() {
            all_comments[start..end].to_vec()
        } else {
            Vec::new()
        };

        let has_more = end < all_comments.len();

        Ok(CommentSearchResults {
            comments,
            threads: all_threads,
            total_count,
            page,
            per_page,
            has_more,
        })
    }

    /// Resolve a comment thread
    pub async fn resolve_thread(&self, thread_id: &str, resolution: Option<String>) -> Result<CommentThread> {
        let mut thread = self.get_thread(thread_id).await?;
        
        // Validate permissions
        self.validate_resolve_permission(&thread).await?;

        thread.status = ThreadStatus::Resolved;
        thread.resolved_by = Some(self.current_user.id.clone());
        thread.resolved_at = Some(Utc::now());
        thread.updated_at = Utc::now();

        // Update thread in TOML
        self.update_thread_in_toml(&thread).await?;

        // Update cache
        {
            let mut cache = self.thread_cache.write().await;
            cache.insert(thread_id.to_string(), thread.clone());
        }

        // Create Git commit
        self.commit_comment_operation(
            &format!("thread: resolve thread {thread_id}"),
            &format!(
                "Resolved comment thread\n\n\
                 Thread ID: {}\n\
                 Resolution: {}\n\
                 Context: {}\n\n\
                 Resolved-By: {}",
                thread.id,
                resolution.unwrap_or_else(|| "No resolution note provided".to_string()),
                self.format_thread_context(&thread.context),
                self.current_user.id
            ),
        ).await?;

        // Send notifications
        self.send_thread_notification(
            CommentEventType::ThreadResolved,
            &thread,
        ).await?;

        Ok(thread)
    }

    /// Close a comment thread
    pub async fn close_thread(&self, thread_id: &str, reason: Option<String>) -> Result<CommentThread> {
        let mut thread = self.get_thread(thread_id).await?;
        
        // Validate permissions
        self.validate_resolve_permission(&thread).await?;

        thread.status = ThreadStatus::Closed;
        thread.updated_at = Utc::now();

        // Update thread in TOML
        self.update_thread_in_toml(&thread).await?;

        // Update cache
        {
            let mut cache = self.thread_cache.write().await;
            cache.insert(thread_id.to_string(), thread.clone());
        }

        // Create Git commit
        self.commit_comment_operation(
            &format!("thread: close thread {thread_id}"),
            &format!(
                "Closed comment thread\n\n\
                 Thread ID: {}\n\
                 Reason: {}\n\
                 Context: {}\n\n\
                 Closed-By: {}",
                thread.id,
                reason.unwrap_or_else(|| "No reason provided".to_string()),
                self.format_thread_context(&thread.context),
                self.current_user.id
            ),
        ).await?;

        // Send notifications
        self.send_thread_notification(
            CommentEventType::ThreadClosed,
            &thread,
        ).await?;

        Ok(thread)
    }

    /// Add reaction to a comment
    pub async fn add_reaction(&self, comment_id: &str, reaction: &str) -> Result<()> {
        let mut comment = self.get_comment(comment_id).await?;
        
        // Add user to reaction list
        comment.reactions.entry(reaction.to_string())
            .or_insert_with(Vec::new)
            .push(self.current_user.id.clone());

        // Remove duplicates
        if let Some(users) = comment.reactions.get_mut(reaction) {
            users.sort();
            users.dedup();
        }

        // Update comment in TOML
        self.update_comment_in_toml(&comment).await?;

        // Send notifications
        let thread = if let Some(ref tid) = comment.thread_id {
            self.get_thread(tid).await.ok()
        } else {
            None
        };

        self.send_comment_notification(
            CommentEventType::CommentReaction,
            &comment,
            thread.as_ref(),
        ).await?;

        Ok(())
    }

    /// Remove reaction from a comment
    pub async fn remove_reaction(&self, comment_id: &str, reaction: &str) -> Result<()> {
        let mut comment = self.get_comment(comment_id).await?;
        
        // Remove user from reaction list
        if let Some(users) = comment.reactions.get_mut(reaction) {
            users.retain(|user| user != &self.current_user.id);
            
            // Remove empty reaction lists
            if users.is_empty() {
                comment.reactions.remove(reaction);
            }
        }

        // Update comment in TOML
        self.update_comment_in_toml(&comment).await?;

        Ok(())
    }

    /// Get comments by user mentions
    pub async fn get_mentions_for_user(&self, user_id: &str) -> Result<Vec<ThreadedComment>> {
        let filter = CommentFilter {
            mentions: Some(user_id.to_string()),
            ..Default::default()
        };

        let results = self.search_comments(filter, 1, 1000).await?;
        Ok(results.comments)
    }

    /// Get comment statistics
    pub async fn get_comment_statistics(&self) -> Result<CommentStatistics> {
        let all_comments = self.load_all_comments().await?;
        let all_threads = self.load_all_threads().await?;

        let total_comments = all_comments.len() as u32;
        let total_threads = all_threads.len() as u32;
        
        let resolved_comments = all_comments.iter()
            .filter(|c| c.resolved)
            .count() as u32;
            
        let resolved_threads = all_threads.iter()
            .filter(|t| t.status == ThreadStatus::Resolved)
            .count() as u32;

        let comments_by_type: HashMap<CommentType, u32> = all_comments.iter()
            .fold(HashMap::new(), |mut acc, comment| {
                *acc.entry(comment.comment_type.clone()).or_insert(0) += 1;
                acc
            });

        let active_participants: HashSet<String> = all_comments.iter()
            .map(|c| c.author.clone())
            .collect();

        Ok(CommentStatistics {
            total_comments,
            total_threads,
            resolved_comments,
            resolved_threads,
            resolution_rate: if total_comments > 0 {
                (resolved_comments as f32 / total_comments as f32) * 100.0
            } else {
                0.0
            },
            comments_by_type,
            active_participants: active_participants.len() as u32,
            avg_resolution_time: None, // TODO: Calculate from resolved comments
        })
    }

    // Private helper methods continue...

    /// Validate user can create comments in this context
    async fn validate_create_permission(&self, context: &CommentContext) -> Result<()> {
        // Basic permission check - in real implementation would check user roles
        match context {
            CommentContext::Project => {
                // All users can comment on project level
                Ok(())
            }
            CommentContext::Chapter => {
                // All users can comment on chapters
                Ok(())
            }
            CommentContext::Translation { language, .. } => {
                // Check if user is assigned to this language
                if self.is_assigned_to_language(language) {
                    Ok(())
                } else {
                    Err(TradocumentError::ApiError(
                        "Not authorized to comment on this translation".to_string()
                    ))
                }
            }
        }
    }

    /// Validate user can edit this comment
    async fn validate_edit_permission(&self, comment: &ThreadedComment) -> Result<()> {
        if comment.author == self.current_user.id || self.is_moderator() {
            Ok(())
        } else {
            Err(TradocumentError::ApiError(
                "Cannot edit comments you didn't create".to_string()
            ))
        }
    }

    /// Validate user can resolve threads
    async fn validate_resolve_permission(&self, _thread: &CommentThread) -> Result<()> {
        // In real implementation, check if user is reviewer/editor
        if self.is_reviewer() || self.is_moderator() {
            Ok(())
        } else {
            Err(TradocumentError::ApiError(
                "Insufficient permissions to resolve threads".to_string()
            ))
        }
    }

    /// Check if user is assigned to language
    fn is_assigned_to_language(&self, language: &str) -> bool {
        // Simplified check - in real implementation would check project assignments
        self.current_user.id.contains(language) || self.is_moderator()
    }

    /// Check if user is reviewer
    fn is_reviewer(&self) -> bool {
        self.current_user.id.contains("reviewer") || self.is_moderator()
    }

    /// Check if user is moderator
    fn is_moderator(&self) -> bool {
        self.current_user.id.contains("admin") || self.current_user.id.contains("editor")
    }

    // Helper method implementations continue...
}

/// Comment statistics for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentStatistics {
    pub total_comments: u32,
    pub total_threads: u32,
    pub resolved_comments: u32,
    pub resolved_threads: u32,
    pub resolution_rate: f32, // percentage
    pub comments_by_type: HashMap<CommentType, u32>,
    pub active_participants: u32,
    pub avg_resolution_time: Option<f32>, // hours
}


impl CommentSystem {
    /// Validate thread exists
    async fn validate_thread_exists(&self, thread_id: &str) -> Result<()> {
        self.get_thread(thread_id).await?;
        Ok(())
    }

    /// Validate comment exists
    async fn validate_comment_exists(&self, comment_id: &str) -> Result<()> {
        self.get_comment(comment_id).await?;
        Ok(())
    }

    /// Detect language of content
    fn detect_language(&self, content: &str) -> Option<String> {
        // Simple language detection - in real implementation would use proper language detection
        if content.chars().any(|c| matches!(c, 'ä' | 'ö' | 'ü' | 'ß')) {
            Some("de".to_string())
        } else if content.chars().any(|c| matches!(c, 'à' | 'é' | 'è' | 'ç')) {
            Some("fr".to_string())
        } else if content.chars().any(|c| matches!(c, 'ñ' | 'á' | 'é' | 'í' | 'ó' | 'ú')) {
            Some("es".to_string())
        } else {
            Some("en".to_string())
        }
    }

    /// Truncate content for previews
    fn truncate_content(&self, content: &str, max_len: usize) -> String {
        if content.len() <= max_len {
            content.to_string()
        } else {
            format!("{}...", &content[..max_len])
        }
    }

    /// Format comment type for display
    fn format_comment_type(&self, comment_type: &CommentType) -> String {
        match comment_type {
            CommentType::Suggestion => "suggestion",
            CommentType::Question => "question",
            CommentType::Approval => "approval",
            CommentType::Issue => "issue",
            CommentType::Context => "context",
            CommentType::Terminology => "terminology",
        }.to_string()
    }

    /// Format thread context for display
    fn format_thread_context(&self, context: &ThreadContext) -> String {
        match context {
            ThreadContext::Translation { unit_id, language, chapter, .. } => {
                format!("translation:{chapter}:{unit_id}:{language}")
            }
            ThreadContext::Paragraph { unit_id, chapter, .. } => {
                format!("paragraph:{chapter}:{unit_id}")
            }
            ThreadContext::Chapter { chapter, section } => {
                format!("chapter:{}:{}", chapter, section.as_deref().unwrap_or(""))
            }
            ThreadContext::Project { topic } => {
                format!("project:{}", topic.as_deref().unwrap_or(""))
            }
        }
    }

    /// Update thread participants
    async fn update_thread_participants(&self, thread_id: &str, user_id: &str) -> Result<()> {
        if let Ok(mut thread) = self.get_thread(thread_id).await {
            if !thread.participants.contains(&user_id.to_string()) {
                thread.participants.push(user_id.to_string());
                thread.updated_at = Utc::now();
                self.update_thread_in_toml(&thread).await?;

                // Update cache
                {
                    let mut cache = self.thread_cache.write().await;
                    cache.insert(thread_id.to_string(), thread);
                }
            }
        }
        Ok(())
    }

    /// Update mention tracking
    async fn update_mention_tracking(&self, comment: &ThreadedComment) -> Result<()> {
        if !comment.mentions.is_empty() {
            let mut tracker = self.mention_tracker.write().await;
            tracker.insert(comment.id.clone(), comment.mentions.clone());
        }
        Ok(())
    }

    /// Build comment tree with proper nesting
    fn build_comment_tree(&self, comments: Vec<ThreadedComment>) -> Result<Vec<ThreadedComment>> {
        let mut comment_map: HashMap<String, ThreadedComment> = comments
            .into_iter()
            .map(|c| (c.id.clone(), c))
            .collect();

        let mut root_comments = Vec::new();

        // First pass: identify root comments
        let comment_ids: Vec<String> = comment_map.keys().cloned().collect();
        for id in comment_ids {
            if let Some(comment) = comment_map.remove(&id) {
                if comment.parent_id.is_none() {
                    root_comments.push(comment);
                } else {
                    // This is a reply, we'll handle it in the second pass
                    comment_map.insert(id, comment);
                }
            }
        }

        // Second pass: attach replies to parents
        for root_comment in &mut root_comments {
            self.attach_replies(root_comment, &mut comment_map);
        }

        Ok(root_comments)
    }

    /// Recursively attach replies to comments
    fn attach_replies(&self, parent: &mut ThreadedComment, comment_map: &mut HashMap<String, ThreadedComment>) {
        let parent_id = parent.id.clone();
        let mut replies = Vec::new();

        // Find all direct replies to this comment
        let keys: Vec<String> = comment_map.keys().cloned().collect();
        for key in keys {
            if let Some(comment) = comment_map.get(&key) {
                if comment.parent_id.as_ref() == Some(&parent_id) {
                    replies.push(key);
                }
            }
        }

        // Remove replies from map and attach to parent
        for reply_id in replies {
            if let Some(mut reply) = comment_map.remove(&reply_id) {
                // Recursively attach replies to this reply
                self.attach_replies(&mut reply, comment_map);
                parent.replies.push(reply);
            }
        }

        // Sort replies by creation time
        parent.replies.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    }

    /// Load all comments from TOML files
    async fn load_all_comments(&self) -> Result<Vec<ThreadedComment>> {
        let mut all_comments = Vec::new();

        // Load from chapter TOML files
        let chapters_dir = Path::new(&self.repo_path).join("content/chapters");
        if chapters_dir.exists() {
            for entry in std::fs::read_dir(chapters_dir)? {
                let entry = entry?;
                if entry.path().extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Ok(chapter_data) = self.load_chapter_toml_by_path(&entry.path()).await {
                        // Convert legacy comments to threaded comments
                        for comment in chapter_data.comments {
                            all_comments.push(self.convert_legacy_comment(comment));
                        }
                        
                        // Load comments from units
                        for unit in chapter_data.units {
                            for comment in unit.comments {
                                all_comments.push(self.convert_legacy_comment(comment));
                            }
                        }
                    }
                }
            }
        }

        Ok(all_comments)
    }

    /// Load all threads from TOML files
    async fn load_all_threads(&self) -> Result<Vec<CommentThread>> {
        // For now, we'll create threads from root comments
        // In a full implementation, threads would be stored separately
        let comments = self.load_all_comments().await?;
        let mut threads = Vec::new();

        for comment in comments {
            if comment.parent_id.is_none() && comment.thread_id.is_some() {
                // This is a root comment, create a thread for it
                let thread = CommentThread {
                    id: comment.thread_id.unwrap(),
                    root_comment_id: comment.id.clone(),
                    context: self.convert_comment_context_to_thread_context(&comment.context),
                    status: if comment.resolved { ThreadStatus::Resolved } else { ThreadStatus::Open },
                    participants: vec![comment.author.clone()],
                    created_at: comment.created_at,
                    updated_at: comment.updated_at.unwrap_or(comment.created_at),
                    resolved_by: comment.resolved_by.clone(),
                    resolved_at: comment.resolved_at,
                    priority: ThreadPriority::Normal,
                    tags: Vec::new(),
                    metadata: ThreadMetadata {
                        assignee: None,
                        due_date: None,
                        estimated_time: None,
                        actual_time: None,
                        labels: Vec::new(),
                        external_refs: Vec::new(),
                    },
                };
                threads.push(thread);
            }
        }

        Ok(threads)
    }

    /// Convert legacy comment to threaded comment
    fn convert_legacy_comment(&self, comment: Comment) -> ThreadedComment {
        let comment_id = comment.id.clone();
        let comment_context = comment.context.clone();
        
        ThreadedComment {
            id: comment.id,
            thread_id: comment.thread_id,
            parent_id: None, // Legacy comments don't have parent_id
            author: comment.author,
            content: comment.content,
            comment_type: comment.comment_type,
            context: comment.context,
            created_at: comment.created_at,
            updated_at: None,
            edited: false,
            resolved: comment.resolved,
            resolved_by: None,
            resolved_at: None,
            replies: comment.replies.into_iter().map(|reply| {
                ThreadedComment {
                    id: Uuid::new_v4().to_string(),
                    thread_id: None,
                    parent_id: Some(comment_id.clone()),
                    author: reply.author,
                    content: reply.content,
                    comment_type: CommentType::Context,
                    context: comment_context.clone(),
                    created_at: reply.created_at,
                    updated_at: None,
                    edited: false,
                    resolved: false,
                    resolved_by: None,
                    resolved_at: None,
                    replies: Vec::new(),
                    mentions: Vec::new(),
                    attachments: Vec::new(),
                    reactions: HashMap::new(),
                    position: None,
                    metadata: CommentMetadata {
                        edited_at: None,
                        edit_history: Vec::new(),
                        language: None,
                        sentiment: None,
                        confidence: None,
                        source: CommentSource::Manual,
                    },
                }
            }).collect(),
            mentions: Vec::new(),
            attachments: Vec::new(),
            reactions: HashMap::new(),
            position: None,
            metadata: CommentMetadata {
                edited_at: None,
                edit_history: Vec::new(),
                language: None,
                sentiment: None,
                confidence: None,
                source: CommentSource::Manual,
            },
        }
    }

    /// Convert comment context to thread context
    fn convert_comment_context_to_thread_context(&self, context: &CommentContext) -> ThreadContext {
        match context {
            CommentContext::Project => ThreadContext::Project { topic: None },
            CommentContext::Chapter => ThreadContext::Chapter { 
                chapter: "unknown".to_string(), 
                section: None 
            },
            CommentContext::Translation { paragraph, language } => ThreadContext::Translation {
                unit_id: paragraph.clone(),
                language: language.clone(),
                chapter: "unknown".to_string(),
                line_number: None,
                text_range: None,
            },
        }
    }

    /// Check if comment matches filter
    fn matches_comment_filter(&self, comment: &ThreadedComment, filter: &CommentFilter) -> bool {
        if let Some(ref author) = filter.author {
            if comment.author != *author {
                return false;
            }
        }

        if let Some(ref comment_type) = filter.comment_type {
            if comment.comment_type != *comment_type {
                return false;
            }
        }

        if let Some(ref resolved) = filter.resolved {
            if comment.resolved != *resolved {
                return false;
            }
        }

        if let Some(ref mentions) = filter.mentions {
            if !comment.mentions.contains(mentions) {
                return false;
            }
        }

        if let Some(ref content_search) = filter.content_search {
            if !comment.content.to_lowercase().contains(&content_search.to_lowercase()) {
                return false;
            }
        }

        if let Some(created_after) = filter.created_after {
            if comment.created_at <= created_after {
                return false;
            }
        }

        if let Some(created_before) = filter.created_before {
            if comment.created_at >= created_before {
                return false;
            }
        }

        true
    }

    /// Check if thread matches filter
    fn matches_thread_filter(&self, thread: &CommentThread, filter: &CommentFilter) -> bool {
        if let Some(ref thread_status) = filter.thread_status {
            if thread.status != *thread_status {
                return false;
            }
        }

        if let Some(ref priority) = filter.priority {
            if thread.priority != *priority {
                return false;
            }
        }

        if let Some(ref tags) = filter.tags {
            if !tags.iter().any(|tag| thread.tags.contains(tag)) {
                return false;
            }
        }

        true
    }

    /// Find comments by thread ID
    async fn find_comments_by_thread(&self, thread_id: &str) -> Result<Vec<ThreadedComment>> {
        let all_comments = self.load_all_comments().await?;
        Ok(all_comments
            .into_iter()
            .filter(|c| c.thread_id.as_ref() == Some(&thread_id.to_string()))
            .collect())
    }

    /// Find comment in TOML files
    async fn find_comment_in_toml(&self, comment_id: &str) -> Result<Option<ThreadedComment>> {
        let all_comments = self.load_all_comments().await?;
        Ok(all_comments.into_iter().find(|c| c.id == comment_id))
    }

    /// Find thread in TOML files
    async fn find_thread_in_toml(&self, thread_id: &str) -> Result<Option<CommentThread>> {
        let all_threads = self.load_all_threads().await?;
        Ok(all_threads.into_iter().find(|t| t.id == thread_id))
    }

    /// Add thread to TOML (placeholder - threads are derived from comments)
    async fn add_thread_to_toml(&self, _thread: &CommentThread) -> Result<()> {
        // Threads are currently derived from comments, so no separate storage needed
        Ok(())
    }

    /// Add comment to appropriate TOML file
    async fn add_comment_to_toml(&self, comment: &ThreadedComment) -> Result<()> {
        // Convert to legacy comment format for TOML storage
        let legacy_comment = Comment {
            id: comment.id.clone(),
            author: comment.author.clone(),
            content: comment.content.clone(),
            comment_type: comment.comment_type.clone(),
            context: comment.context.clone(),
            created_at: comment.created_at,
            resolved: comment.resolved,
            thread_id: comment.thread_id.clone(),
            replies: Vec::new(), // Replies will be added separately
        };

        match &comment.context {
            CommentContext::Translation { paragraph, .. } => {
                let chapter_name = self.infer_chapter_from_unit_id(paragraph)?;
                let mut chapter_data = self.load_chapter_toml(&chapter_name).await?;
                
                // Find the unit and add comment
                if let Some(unit) = chapter_data.units.iter_mut().find(|u| u.id == *paragraph) {
                    unit.comments.push(legacy_comment);
                } else {
                    return Err(TradocumentError::ApiError(
                        format!("Unit {paragraph} not found")
                    ));
                }
                
                self.save_chapter_toml(&chapter_name, &chapter_data).await?;
            }
            CommentContext::Chapter => {
                // Add to a default chapter or infer from context
                let chapter_name = "default_chapter";
                let mut chapter_data = self.load_chapter_toml(chapter_name).await?;
                chapter_data.comments.push(legacy_comment);
                self.save_chapter_toml(chapter_name, &chapter_data).await?;
            }
            CommentContext::Project => {
                // Project-level comments could go in project TOML
                return Err(TradocumentError::ApiError(
                    "Project-level comments not yet implemented".to_string()
                ));
            }
        }

        Ok(())
    }

    /// Update comment in TOML
    async fn update_comment_in_toml(&self, comment: &ThreadedComment) -> Result<()> {
        // For now, we'll replace the comment by removing and re-adding
        // In a full implementation, this would be more efficient
        self.remove_comment_from_toml(&comment.id).await?;
        self.add_comment_to_toml(comment).await?;
        Ok(())
    }

    /// Update thread in TOML
    async fn update_thread_in_toml(&self, _thread: &CommentThread) -> Result<()> {
        // Threads are derived from comments, so we don't store them separately
        Ok(())
    }

    /// Remove comment from TOML
    async fn remove_comment_from_toml(&self, comment_id: &str) -> Result<()> {
        // Search through all TOML files to find and remove the comment
        let chapters_dir = Path::new(&self.repo_path).join("content/chapters");
        if chapters_dir.exists() {
            for entry in std::fs::read_dir(chapters_dir)? {
                let entry = entry?;
                if entry.path().extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Ok(mut chapter_data) = self.load_chapter_toml_by_path(&entry.path()).await {
                        let mut found = false;
                        
                        // Remove from chapter comments
                        chapter_data.comments.retain(|c| c.id != comment_id);
                        
                        // Remove from unit comments
                        for unit in &mut chapter_data.units {
                            let original_len = unit.comments.len();
                            unit.comments.retain(|c| c.id != comment_id);
                            if unit.comments.len() != original_len {
                                found = true;
                            }
                        }
                        
                        if found {
                            let path = entry.path();
                            let chapter_name = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .ok_or_else(|| TradocumentError::ApiError("Invalid chapter filename".to_string()))?;
                            self.save_chapter_toml(chapter_name, &chapter_data).await?;
                            return Ok(());
                        }
                    }
                }
            }
        }

        Err(TradocumentError::ApiError(
            format!("Comment {comment_id} not found for removal")
        ))
    }

    /// Commit comment operation to Git
    async fn commit_comment_operation(&self, title: &str, message: &str) -> Result<()> {
        // In a real implementation, this would use the GitWorkflowManager
        println!("Git commit: {title}");
        println!("Message: {message}");
        
        // TODO: Implement actual Git commit through GitWorkflowManager
        // self.git_manager.commit_changes(title, message).await?;
        
        Ok(())
    }

    /// Send comment notification
    async fn send_comment_notification(
        &self,
        event_type: CommentEventType,
        comment: &ThreadedComment,
        thread: Option<&CommentThread>,
    ) -> Result<()> {
        let mut affected_users = comment.mentions.clone();
        
        // Add thread participants
        if let Some(thread) = thread {
            for participant in &thread.participants {
                if !affected_users.contains(participant) && participant != &comment.author {
                    affected_users.push(participant.clone());
                }
            }
        }

        let notification = CommentNotification {
            event_type,
            comment_id: comment.id.clone(),
            thread_id: comment.thread_id.clone(),
            actor: comment.author.clone(),
            affected_users,
            content_preview: self.truncate_content(&comment.content, 100),
            context: comment.context.clone(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        // TODO: Integrate with notification system
        println!("Comment notification: {notification:?}");
        
        Ok(())
    }

    /// Send thread notification
    async fn send_thread_notification(
        &self,
        event_type: CommentEventType,
        thread: &CommentThread,
    ) -> Result<()> {
        let notification = CommentNotification {
            event_type,
            comment_id: thread.root_comment_id.clone(),
            thread_id: Some(thread.id.clone()),
            actor: self.current_user.id.clone(),
            affected_users: thread.participants.clone(),
            content_preview: format!("Thread: {}", thread.id),
            context: CommentContext::Project, // Placeholder
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        // TODO: Integrate with notification system
        println!("Thread notification: {notification:?}");
        
        Ok(())
    }

    /// Load chapter TOML data by name
    async fn load_chapter_toml(&self, chapter_name: &str) -> Result<ChapterData> {
        let chapter_path = Path::new(&self.repo_path)
            .join("content/chapters")
            .join(format!("{chapter_name}.toml"));
            
        self.load_chapter_toml_by_path(&chapter_path).await
    }

    /// Load chapter TOML data by path
    async fn load_chapter_toml_by_path(&self, chapter_path: &Path) -> Result<ChapterData> {
        if chapter_path.exists() {
            let toml_content = std::fs::read_to_string(chapter_path)?;
            let chapter_data: ChapterData = toml::from_str(&toml_content)
                .map_err(|e| TradocumentError::ApiError(format!("Failed to parse chapter TOML: {e}")))?;
            Ok(chapter_data)
        } else {
            // Create default chapter data
            let chapter_name = chapter_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("default");
            Ok(self.create_default_chapter_data(chapter_name).await?)
        }
    }

    /// Save chapter TOML data
    async fn save_chapter_toml(&self, chapter_name: &str, chapter_data: &ChapterData) -> Result<()> {
        let chapter_path = Path::new(&self.repo_path)
            .join("content/chapters")
            .join(format!("{chapter_name}.toml"));
        
        // Ensure directory exists
        if let Some(parent) = chapter_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let toml_content = toml::to_string_pretty(chapter_data)
            .map_err(|e| TradocumentError::ApiError(format!("Failed to serialize chapter TOML: {e}")))?;
            
        std::fs::write(&chapter_path, toml_content)?;
        Ok(())
    }

    /// Create default chapter data
    async fn create_default_chapter_data(&self, chapter_name: &str) -> Result<ChapterData> {
        use super::models::{ChapterMetadata, ChapterStatus, ChapterMetadataExtra, DifficultyLevel};
        
        let chapter_metadata = ChapterMetadata {
            number: 1,
            slug: chapter_name.to_string(),
            status: ChapterStatus::Draft,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            git_branch: None,
            last_git_commit: None,
            title: HashMap::new(),
            metadata: ChapterMetadataExtra {
                word_count: HashMap::new(),
                difficulty: DifficultyLevel::Beginner,
                estimated_translation_time: HashMap::new(),
                requires_screenshots: false,
                screenshot_count: 0,
                last_reviewed: HashMap::new(),
            },
        };

        Ok(ChapterData {
            chapter: chapter_metadata,
            units: Vec::new(),
            todos: Vec::new(),
            comments: Vec::new(),
        })
    }

    /// Infer chapter name from unit ID
    fn infer_chapter_from_unit_id(&self, unit_id: &str) -> Result<String> {
        // Parse unit ID to extract chapter (e.g., "intro_p001" -> "intro")
        if let Some(underscore_pos) = unit_id.find('_') {
            Ok(unit_id[..underscore_pos].to_string())
        } else {
            Ok("default_chapter".to_string())
        }
    }
}