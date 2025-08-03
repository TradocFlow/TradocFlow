//! Task Management System
//! 
//! Provides comprehensive todo and task management with Git integration,
//! multi-role support, and TOML persistence according to PRD specifications.

use super::{
    GitWorkflowManager, ProjectData, ChapterData, Todo, KanbanGitSync
};
use super::models::{
    TodoType, TodoStatus, TodoContext, Priority, Comment, CommentType, 
    CommentContext, CommentReply, TodoMetadata, ChapterStatus, TranslationUnitStatus
};
use crate::{Result, TradocumentError, User, NotificationService};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Task management service with Git integration and multi-role support
#[derive(Debug)]
pub struct TaskManager {
    git_manager: Arc<GitWorkflowManager>,
    kanban_sync: Arc<KanbanGitSync>,
    notification_service: Arc<NotificationService>,
    project_id: Uuid,
    repo_path: String,
    current_user: User,
    // Cache for frequently accessed todos
    todo_cache: Arc<RwLock<HashMap<String, Todo>>>,
}

/// Request to create a new todo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTodoRequest {
    pub title: String,
    pub description: Option<String>,
    pub assigned_to: Option<String>,
    pub priority: Priority,
    pub todo_type: TodoType,
    pub context: TodoContext,
    pub due_date: Option<DateTime<Utc>>,
    pub metadata: Option<TodoMetadata>,
}

/// Request to update an existing todo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTodoRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub assigned_to: Option<String>,
    pub priority: Option<Priority>,
    pub status: Option<TodoStatus>,
    pub due_date: Option<DateTime<Utc>>,
    pub resolution: Option<String>,
    pub metadata: Option<TodoMetadata>,
}

/// Request to create a comment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
    pub comment_type: CommentType,
    pub context: CommentContext,
    pub thread_id: Option<String>,
}

/// Reply to a comment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommentReplyRequest {
    pub content: String,
    pub reply_to: Option<String>,
}

/// Role-based permissions for task operations
#[derive(Debug, Clone)]
pub enum UserRole {
    Editor,
    Translator,
    Reviewer,
    Admin,
}

/// Notification event for task operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNotification {
    pub event_type: TaskEventType,
    pub todo_id: String,
    pub created_by: String,
    pub affected_users: Vec<String>,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// Types of task events that trigger notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskEventType {
    TodoCreated,
    TodoAssigned,
    TodoCompleted,
    TodoUpdated,
    CommentAdded,
    CommentReplied,
    CommentResolved,
}

/// Project progress overview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectProgress {
    pub project_id: Uuid,
    pub overall_completion: f32, // 0.0 to 1.0
    pub language_progress: HashMap<String, LanguageProgress>,
    pub chapter_progress: HashMap<String, ChapterProgress>,
    pub team_stats: TeamStats,
    pub timeline: ProgressTimeline,
    pub quality_metrics: QualityMetrics,
    pub updated_at: DateTime<Utc>,
}

/// Progress for a specific language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageProgress {
    pub language: String,
    pub completion: f32, // 0.0 to 1.0
    pub total_units: u32,
    pub completed_units: u32,
    pub in_progress_units: u32,
    pub under_review_units: u32,
    pub approved_units: u32,
    pub word_count: u32,
    pub translated_words: u32,
    pub estimated_hours_remaining: f32,
    pub quality_score: Option<f32>,
}

/// Progress for a specific chapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterProgress {
    pub chapter: String,
    pub completion: f32, // 0.0 to 1.0
    pub language_status: HashMap<String, ChapterLanguageStatus>,
    pub todo_stats: TodoStats,
    pub last_activity: DateTime<Utc>,
}

/// Status of a chapter in a specific language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterLanguageStatus {
    pub status: ChapterStatus,
    pub completion: f32,
    pub translator: Option<String>,
    pub reviewer: Option<String>,
    pub last_updated: DateTime<Utc>,
}

/// Team performance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamStats {
    pub total_members: u32,
    pub active_members: u32,
    pub productivity_scores: HashMap<String, f32>,
    pub workload_distribution: HashMap<String, WorkloadInfo>,
    pub average_completion_time: HashMap<TodoType, f32>, // hours per todo type
}

/// Workload information for a team member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadInfo {
    pub assigned_todos: u32,
    pub completed_todos: u32,
    pub overdue_todos: u32,
    pub estimated_hours: f32,
    pub last_activity: DateTime<Utc>,
}

/// Progress timeline tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressTimeline {
    pub start_date: DateTime<Utc>,
    pub target_completion: Option<DateTime<Utc>>,
    pub estimated_completion: DateTime<Utc>,
    pub milestones: Vec<Milestone>,
    pub velocity: Velocity,
}

/// Project milestone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub name: String,
    pub target_date: DateTime<Utc>,
    pub completion: f32,
    pub dependencies: Vec<String>,
    pub status: MilestoneStatus,
}

/// Status of a milestone
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MilestoneStatus {
    NotStarted,
    InProgress,
    Completed,
    Delayed,
    AtRisk,
}

/// Team velocity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Velocity {
    pub todos_per_week: f32,
    pub words_per_day: HashMap<String, f32>, // language -> words per day
    pub quality_trend: Vec<QualityDataPoint>,
    pub efficiency_trend: Vec<EfficiencyDataPoint>,
}

/// Quality metrics point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityDataPoint {
    pub date: DateTime<Utc>,
    pub average_score: f32,
    pub language_scores: HashMap<String, f32>,
}

/// Efficiency metrics point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EfficiencyDataPoint {
    pub date: DateTime<Utc>,
    pub todos_completed: u32,
    pub average_completion_time: f32,
}

/// Overall quality metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub average_score: f32,
    pub language_scores: HashMap<String, f32>,
    pub reviewer_scores: HashMap<String, f32>,
    pub quality_trend: QualityTrend,
    pub issues_identified: u32,
    pub issues_resolved: u32,
}

/// Quality trend direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityTrend {
    Improving,
    Stable,
    Declining,
}

/// Todo statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoStats {
    pub total: u32,
    pub open: u32,
    pub in_progress: u32,
    pub completed: u32,
    pub overdue: u32,
    pub by_type: HashMap<TodoType, u32>,
    pub by_priority: HashMap<Priority, u32>,
}

/// Task information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub todo: Todo,
    pub context_info: TaskContextInfo,
    pub progress_info: TaskProgressInfo,
}

/// Context information for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContextInfo {
    pub chapter_name: Option<String>,
    pub unit_id: Option<String>,
    pub language: Option<String>,
    pub dependencies: Vec<String>,
    pub blocking: Vec<String>,
}

/// Progress information for a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgressInfo {
    pub time_spent: Option<f32>,
    pub estimated_remaining: Option<f32>,
    pub completion_percentage: Option<u32>,
    pub last_activity: DateTime<Utc>,
}

/// Task query filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFilter {
    pub assigned_to: Option<String>,
    pub created_by: Option<String>,
    pub status: Option<TodoStatus>,
    pub priority: Option<Priority>,
    pub todo_type: Option<TodoType>,
    pub context_type: Option<String>, // "project", "chapter", "paragraph", "translation"
    pub due_before: Option<DateTime<Utc>>,
    pub created_after: Option<DateTime<Utc>>,
}

impl TaskManager {
    /// Create a new task manager instance
    pub async fn new(
        git_manager: Arc<GitWorkflowManager>,
        kanban_sync: Arc<KanbanGitSync>,
        notification_service: Arc<NotificationService>,
        project_id: Uuid,
        repo_path: &str,
        current_user: User,
    ) -> Result<Self> {
        Ok(Self {
            git_manager,
            kanban_sync,
            notification_service,
            project_id,
            repo_path: repo_path.to_string(),
            current_user,
            todo_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create a new todo with Git commit integration
    pub async fn create_todo(
        &self,
        request: CreateTodoRequest,
    ) -> Result<Todo> {
        // Validate permissions
        self.validate_create_permission(&request.context, &request.todo_type)?;

        // Generate unique ID
        let todo_id = Uuid::new_v4().to_string();
        
        // Create todo instance
        let todo = Todo {
            id: todo_id.clone(),
            title: request.title.clone(),
            description: request.description,
            created_by: self.current_user.id.clone(),
            assigned_to: request.assigned_to.clone(),
            priority: request.priority,
            status: TodoStatus::Open,
            todo_type: request.todo_type,
            context: request.context.clone(),
            created_at: Utc::now(),
            due_date: request.due_date,
            resolved_at: None,
            resolution: None,
            metadata: request.metadata,
        };

        // Add todo to appropriate TOML file based on context
        self.add_todo_to_toml(&todo).await?;

        // Create Git commit for todo creation
        self.commit_todo_operation(
            &format!("task: create {} todo '{}'", 
                     self.format_todo_type(&todo.todo_type), 
                     todo.title),
            &format!(
                "Created new {} todo: {}\n\n\
                 - Priority: {:?}\n\
                 - Assigned to: {}\n\
                 - Context: {}\n\
                 - Due date: {}\n\n\
                 Created-By: {}\n\
                 Todo-ID: {}\n\
                 Context: {}",
                self.format_todo_type(&todo.todo_type),
                todo.title,
                todo.priority,
                todo.assigned_to.as_deref().unwrap_or("unassigned"),
                self.format_context(&todo.context),
                todo.due_date.map(|d| d.to_rfc3339()).unwrap_or_else(|| "none".to_string()),
                todo.created_by,
                todo.id,
                self.format_context_detailed(&todo.context)
            ),
        ).await?;

        // Cache the todo
        {
            let mut cache = self.todo_cache.write().await;
            cache.insert(todo_id.clone(), todo.clone());
        }

        // Send notifications
        self.send_todo_notification(TaskEventType::TodoCreated, &todo).await?;

        Ok(todo)
    }

    /// Update an existing todo
    pub async fn update_todo(
        &self,
        todo_id: &str,
        request: UpdateTodoRequest,
    ) -> Result<Todo> {
        // Load current todo
        let mut todo = self.get_todo(todo_id).await?;
        
        // Validate permissions
        self.validate_update_permission(&todo)?;

        // Track what changed for commit message
        let mut changes = Vec::new();

        // Apply updates
        if let Some(title) = request.title {
            if title != todo.title {
                changes.push(format!("title: '{}' → '{}'", todo.title, title));
                todo.title = title;
            }
        }

        if let Some(description) = request.description {
            todo.description = Some(description);
            changes.push("updated description".to_string());
        }

        if let Some(assigned_to) = request.assigned_to {
            if todo.assigned_to.as_ref() != Some(&assigned_to) {
                changes.push(format!(
                    "assignee: {} → {}", 
                    todo.assigned_to.as_deref().unwrap_or("unassigned"),
                    assigned_to
                ));
                todo.assigned_to = Some(assigned_to);
                
                // Send assignment notification
                self.send_todo_notification(TaskEventType::TodoAssigned, &todo).await?;
            }
        }

        if let Some(priority) = request.priority {
            if priority != todo.priority {
                changes.push(format!("priority: {:?} → {:?}", todo.priority, priority));
                todo.priority = priority;
            }
        }

        if let Some(status) = request.status {
            if status != todo.status {
                changes.push(format!("status: {:?} → {:?}", todo.status, status));
                todo.status = status.clone();
                
                // Handle completion
                if status == TodoStatus::Completed {
                    todo.resolved_at = Some(Utc::now());
                    if let Some(resolution) = &request.resolution {
                        todo.resolution = Some(resolution.clone());
                    } else {
                        todo.resolution = Some("Completed".to_string());
                    }
                    
                    // Send completion notification
                    self.send_todo_notification(TaskEventType::TodoCompleted, &todo).await?;
                }
            }
        }

        if let Some(due_date) = request.due_date {
            changes.push("updated due date".to_string());
            todo.due_date = Some(due_date);
        }

        if let Some(resolution) = request.resolution {
            todo.resolution = Some(resolution);
            changes.push("added resolution".to_string());
        }

        if let Some(metadata) = request.metadata {
            todo.metadata = Some(metadata);
            changes.push("updated metadata".to_string());
        }

        // Update TOML file
        self.update_todo_in_toml(&todo).await?;

        // Create Git commit
        if !changes.is_empty() {
            self.commit_todo_operation(
                &format!("task: update todo '{}'", todo.title),
                &format!(
                    "Updated todo: {}\n\n\
                     Changes:\n{}\n\n\
                     Updated-By: {}\n\
                     Todo-ID: {}",
                    todo.title,
                    changes.into_iter().map(|c| format!("- {}", c)).collect::<Vec<_>>().join("\n"),
                    self.current_user.id,
                    todo.id
                ),
            ).await?;
        }

        // Update cache
        {
            let mut cache = self.todo_cache.write().await;
            cache.insert(todo_id.to_string(), todo.clone());
        }

        // Send update notification
        self.send_todo_notification(TaskEventType::TodoUpdated, &todo).await?;

        Ok(todo)
    }

    /// Complete a todo (convenience method)
    pub async fn complete_todo(
        &self,
        todo_id: &str,
        resolution: Option<String>,
    ) -> Result<Todo> {
        let update_request = UpdateTodoRequest {
            title: None,
            description: None,
            assigned_to: None,
            priority: None,
            status: Some(TodoStatus::Completed),
            due_date: None,
            resolution,
            metadata: None,
        };

        self.update_todo(todo_id, update_request).await
    }

    /// Cancel a todo
    pub async fn cancel_todo(
        &self,
        todo_id: &str,
        reason: Option<String>,
    ) -> Result<Todo> {
        let update_request = UpdateTodoRequest {
            title: None,
            description: None,
            assigned_to: None,
            priority: None,
            status: Some(TodoStatus::Cancelled),
            due_date: None,
            resolution: reason,
            metadata: None,
        };

        self.update_todo(todo_id, update_request).await
    }

    /// Get a specific todo by ID
    pub async fn get_todo(&self, todo_id: &str) -> Result<Todo> {
        // Check cache first
        {
            let cache = self.todo_cache.read().await;
            if let Some(todo) = cache.get(todo_id) {
                return Ok(todo.clone());
            }
        }

        // Search in TOML files
        if let Some(todo) = self.find_todo_in_toml(todo_id).await? {
            // Cache the found todo
            {
                let mut cache = self.todo_cache.write().await;
                cache.insert(todo_id.to_string(), todo.clone());
            }
            Ok(todo)
        } else {
            Err(TradocumentError::ApiError(
                format!("Todo {} not found", todo_id)
            ))
        }
    }

    /// List todos with filtering support
    pub async fn list_todos(&self, filter: TaskFilter) -> Result<Vec<Todo>> {
        let mut all_todos = Vec::new();

        // Load todos from project TOML
        if let Ok(project_data) = self.load_project_toml().await {
            all_todos.extend(project_data.todos);
        }

        // Load todos from chapter TOML files
        let chapters_dir = Path::new(&self.repo_path).join("content/chapters");
        if chapters_dir.exists() {
            for entry in std::fs::read_dir(chapters_dir)? {
                let entry = entry?;
                if entry.path().extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Ok(chapter_data) = self.load_chapter_toml_by_path(&entry.path()).await {
                        all_todos.extend(chapter_data.todos);
                        
                        // Add unit-level todos
                        for unit in chapter_data.units {
                            all_todos.extend(unit.todos);
                        }
                    }
                }
            }
        }

        // Apply filters
        let filtered_todos = all_todos
            .into_iter()
            .filter(|todo| self.matches_filter(todo, &filter))
            .collect();

        Ok(filtered_todos)
    }

    /// Get todos assigned to a specific user
    pub async fn get_user_todos(&self, user_id: &str) -> Result<Vec<Todo>> {
        let filter = TaskFilter {
            assigned_to: Some(user_id.to_string()),
            created_by: None,
            status: None,
            priority: None,
            todo_type: None,
            context_type: None,
            due_before: None,
            created_after: None,
        };

        self.list_todos(filter).await
    }

    /// Get todos created by a specific user
    pub async fn get_created_todos(&self, user_id: &str) -> Result<Vec<Todo>> {
        let filter = TaskFilter {
            assigned_to: None,
            created_by: Some(user_id.to_string()),
            status: None,
            priority: None,
            todo_type: None,
            context_type: None,
            due_before: None,
            created_after: None,
        };

        self.list_todos(filter).await
    }

    /// Get overdue todos
    pub async fn get_overdue_todos(&self) -> Result<Vec<Todo>> {
        let filter = TaskFilter {
            assigned_to: None,
            created_by: None,
            status: Some(TodoStatus::Open),
            priority: None,
            todo_type: None,
            context_type: None,
            due_before: Some(Utc::now()),
            created_after: None,
        };

        self.list_todos(filter).await
    }

    /// Delete a todo (soft delete by setting status to Cancelled)
    pub async fn delete_todo(&self, todo_id: &str) -> Result<()> {
        // Load current todo to validate permissions
        let todo = self.get_todo(todo_id).await?;
        
        // Validate delete permission (only creator or admin can delete)
        if todo.created_by != self.current_user.id && !self.is_admin() {
            return Err(TradocumentError::ApiError(
                "Insufficient permissions to delete todo".to_string()
            ));
        }

        // Soft delete by cancelling
        self.cancel_todo(todo_id, Some("Deleted by user".to_string())).await?;

        // Remove from cache
        {
            let mut cache = self.todo_cache.write().await;
            cache.remove(todo_id);
        }

        Ok(())
    }

    /// Get comprehensive project progress overview
    pub async fn get_project_progress(&self) -> Result<ProjectProgress> {
        // Load all project data
        let project_data = self.load_project_toml().await?;
        let all_todos = self.list_todos(TaskFilter {
            assigned_to: None,
            created_by: None,
            status: None,
            priority: None,
            todo_type: None,
            context_type: None,
            due_before: None,
            created_after: None,
        }).await?;

        // Calculate language progress
        let mut language_progress = HashMap::new();
        for language in &project_data.project.languages.targets {
            let lang_progress = self.calculate_language_progress(language).await?;
            language_progress.insert(language.clone(), lang_progress);
        }

        // Calculate chapter progress
        let mut chapter_progress = HashMap::new();
        let chapters_dir = Path::new(&self.repo_path).join("content/chapters");
        if chapters_dir.exists() {
            for entry in std::fs::read_dir(chapters_dir)? {
                let entry = entry?;
                if entry.path().extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Some(chapter_name) = entry.path().file_stem().and_then(|s| s.to_str()) {
                        let chap_progress = self.calculate_chapter_progress(chapter_name).await?;
                        chapter_progress.insert(chapter_name.to_string(), chap_progress);
                    }
                }
            }
        }

        // Calculate team stats
        let team_stats = self.calculate_team_stats(&all_todos).await?;

        // Calculate overall completion
        let overall_completion = if language_progress.is_empty() {
            0.0
        } else {
            language_progress.values().map(|p| p.completion).sum::<f32>() / language_progress.len() as f32
        };

        // Create timeline
        let timeline = self.calculate_timeline(&all_todos, &project_data).await?;

        // Calculate quality metrics
        let quality_metrics = self.calculate_quality_metrics().await?;

        Ok(ProjectProgress {
            project_id: self.project_id,
            overall_completion,
            language_progress,
            chapter_progress,
            team_stats,
            timeline,
            quality_metrics,
            updated_at: Utc::now(),
        })
    }

    /// Get tasks assigned to a specific user with enhanced context
    pub async fn get_user_tasks(&self, user_id: &str) -> Result<Vec<Task>> {
        let todos = self.get_user_todos(user_id).await?;
        let mut tasks = Vec::new();

        for todo in todos {
            let context_info = self.build_task_context_info(&todo).await?;
            let progress_info = self.build_task_progress_info(&todo).await?;

            tasks.push(Task {
                todo,
                context_info,
                progress_info,
            });
        }

        Ok(tasks)
    }

    /// Assign a todo to a user
    pub async fn assign_todo(&self, todo_id: &str, assignee: &str) -> Result<()> {
        let update_request = UpdateTodoRequest {
            title: None,
            description: None,
            assigned_to: Some(assignee.to_string()),
            priority: None,
            status: None,
            due_date: None,
            resolution: None,
            metadata: None,
        };

        self.update_todo(todo_id, update_request).await?;

        // Sync with Kanban board if configured
        if let Err(e) = self.kanban_sync.sync_todo_assignment(todo_id, assignee).await {
            // Log error but don't fail the operation
            eprintln!("Failed to sync todo assignment to Kanban: {}", e);
        }

        Ok(())
    }

    /// Add a comment to a todo or translation unit
    pub async fn add_comment(
        &self,
        request: CreateCommentRequest,
    ) -> Result<Comment> {
        let comment_id = Uuid::new_v4().to_string();
        
        let comment = Comment {
            id: comment_id.clone(),
            author: self.current_user.id.clone(),
            content: request.content.clone(),
            comment_type: request.comment_type,
            context: request.context.clone(),
            created_at: Utc::now(),
            resolved: false,
            thread_id: request.thread_id,
            replies: Vec::new(),
        };

        // Add comment to appropriate TOML file
        self.add_comment_to_toml(&comment).await?;

        // Create Git commit
        self.commit_todo_operation(
            &format!("comment: add {} comment", self.format_comment_type(&comment.comment_type)),
            &format!(
                "Added {} comment\n\n\
                 Content: {}\n\
                 Context: {}\n\n\
                 Author: {}\n\
                 Comment-ID: {}",
                self.format_comment_type(&comment.comment_type),
                comment.content,
                self.format_comment_context(&comment.context),
                comment.author,
                comment.id
            ),
        ).await?;

        // Send notification
        self.send_comment_notification(TaskEventType::CommentAdded, &comment).await?;

        Ok(comment)
    }

    /// Reply to a comment
    pub async fn reply_to_comment(
        &self,
        comment_id: &str,
        request: CreateCommentReplyRequest,
    ) -> Result<CommentReply> {
        let reply = CommentReply {
            author: self.current_user.id.clone(),
            content: request.content.clone(),
            created_at: Utc::now(),
            reply_to: request.reply_to,
        };

        // Add reply to comment in TOML
        self.add_reply_to_comment_in_toml(comment_id, &reply).await?;

        // Create Git commit
        self.commit_todo_operation(
            &format!("comment: reply to comment {}", comment_id),
            &format!(
                "Added reply to comment\n\n\
                 Reply content: {}\n\
                 Original comment: {}\n\n\
                 Author: {}",
                reply.content,
                comment_id,
                reply.author
            ),
        ).await?;

        // Send notification
        let comment = Comment {
            id: comment_id.to_string(),
            author: "".to_string(), // We don't have the original comment loaded
            content: "".to_string(),
            comment_type: CommentType::Context,
            context: CommentContext::Project,
            created_at: Utc::now(),
            resolved: false,
            thread_id: None,
            replies: vec![reply.clone()],
        };
        
        self.send_comment_notification(TaskEventType::CommentReplied, &comment).await?;

        Ok(reply)
    }

    /// Resolve a comment
    pub async fn resolve_comment(&self, comment_id: &str) -> Result<()> {
        // Update comment resolution status in TOML
        self.update_comment_resolution_in_toml(comment_id, true).await?;

        // Create Git commit
        self.commit_todo_operation(
            &format!("comment: resolve comment {}", comment_id),
            &format!(
                "Resolved comment {}\n\n\
                 Resolved-By: {}",
                comment_id,
                self.current_user.id
            ),
        ).await?;

        // Send notification
        let comment = Comment {
            id: comment_id.to_string(),
            author: "".to_string(),
            content: "".to_string(),
            comment_type: CommentType::Context,
            context: CommentContext::Project,
            created_at: Utc::now(),
            resolved: true,
            thread_id: None,
            replies: Vec::new(),
        };
        
        self.send_comment_notification(TaskEventType::CommentResolved, &comment).await?;

        Ok(())
    }

    // Private helper methods

    /// Validate permission to create a todo
    fn validate_create_permission(&self, context: &TodoContext, todo_type: &TodoType) -> Result<()> {
        let user_role = self.get_user_role();
        
        match (user_role, context, todo_type) {
            // Admins and editors can create any todo
            (UserRole::Admin, _, _) | (UserRole::Editor, _, _) => Ok(()),
            
            // Translators can create translation-related todos in their assigned contexts
            (UserRole::Translator, TodoContext::Translation { language, .. }, TodoType::Translation) => {
                if self.is_assigned_to_language(language) {
                    Ok(())
                } else {
                    Err(TradocumentError::ApiError(
                        "Not assigned to this language".to_string()
                    ))
                }
            }
            
            // Reviewers can create review todos
            (UserRole::Reviewer, _, TodoType::Review) => Ok(()),
            
            // Default deny
            _ => Err(TradocumentError::ApiError(
                "Insufficient permissions to create this todo".to_string()
            ))
        }
    }

    /// Validate permission to update a todo
    fn validate_update_permission(&self, todo: &Todo) -> Result<()> {
        let user_role = self.get_user_role();
        
        match user_role {
            // Admins can update any todo
            UserRole::Admin => Ok(()),
            
            // Editors can update project and chapter todos
            UserRole::Editor => match todo.context {
                TodoContext::Project | TodoContext::Chapter => Ok(()),
                _ => Err(TradocumentError::ApiError(
                    "Insufficient permissions to update this todo".to_string()
                ))
            },
            
            // Users can update todos they created or are assigned to
            UserRole::Translator | UserRole::Reviewer => {
                if todo.created_by == self.current_user.id || 
                   todo.assigned_to.as_ref() == Some(&self.current_user.id) {
                    Ok(())
                } else {
                    Err(TradocumentError::ApiError(
                        "Can only update todos you created or are assigned to".to_string()
                    ))
                }
            }
        }
    }

    /// Get user role (simplified implementation)
    fn get_user_role(&self) -> UserRole {
        // In a real implementation, this would check the user's role in the project
        // For now, assume based on user ID patterns or default to Translator
        if self.current_user.id.contains("admin") {
            UserRole::Admin
        } else if self.current_user.id.contains("editor") {
            UserRole::Editor
        } else if self.current_user.id.contains("reviewer") {
            UserRole::Reviewer
        } else {
            UserRole::Translator
        }
    }

    /// Check if user is admin
    fn is_admin(&self) -> bool {
        matches!(self.get_user_role(), UserRole::Admin)
    }

    /// Check if user is assigned to a language
    fn is_assigned_to_language(&self, language: &str) -> bool {
        // In a real implementation, this would check project assignments
        // For now, assume translators are assigned to languages based on their ID
        self.current_user.id.contains(language) || self.is_admin()
    }

    /// Add todo to appropriate TOML file based on context
    async fn add_todo_to_toml(&self, todo: &Todo) -> Result<()> {
        match &todo.context {
            TodoContext::Project => {
                let mut project_data = self.load_project_toml().await?;
                project_data.todos.push(todo.clone());
                self.save_project_toml(&project_data).await?;
            }
            
            TodoContext::Chapter => {
                // Determine chapter from current context or use default
                let chapter_name = self.infer_chapter_from_context(&todo.context)?;
                let mut chapter_data = self.load_chapter_toml(&chapter_name).await?;
                chapter_data.todos.push(todo.clone());
                self.save_chapter_toml(&chapter_name, &chapter_data).await?;
            }
            
            TodoContext::Paragraph { unit_id } | TodoContext::Translation { unit_id, .. } => {
                let chapter_name = self.infer_chapter_from_unit_id(unit_id)?;
                let mut chapter_data = self.load_chapter_toml(&chapter_name).await?;
                
                // Find the unit and add todo to it
                if let Some(unit) = chapter_data.units.iter_mut().find(|u| u.id == *unit_id) {
                    unit.todos.push(todo.clone());
                } else {
                    return Err(TradocumentError::ApiError(
                        format!("Unit {} not found", unit_id)
                    ));
                }
                
                self.save_chapter_toml(&chapter_name, &chapter_data).await?;
            }
        }
        
        Ok(())
    }

    /// Update todo in TOML file
    async fn update_todo_in_toml(&self, todo: &Todo) -> Result<()> {
        match &todo.context {
            TodoContext::Project => {
                let mut project_data = self.load_project_toml().await?;
                if let Some(existing_todo) = project_data.todos.iter_mut().find(|t| t.id == todo.id) {
                    *existing_todo = todo.clone();
                }
                self.save_project_toml(&project_data).await?;
            }
            
            TodoContext::Chapter => {
                let chapter_name = self.infer_chapter_from_context(&todo.context)?;
                let mut chapter_data = self.load_chapter_toml(&chapter_name).await?;
                if let Some(existing_todo) = chapter_data.todos.iter_mut().find(|t| t.id == todo.id) {
                    *existing_todo = todo.clone();
                }
                self.save_chapter_toml(&chapter_name, &chapter_data).await?;
            }
            
            TodoContext::Paragraph { unit_id } | TodoContext::Translation { unit_id, .. } => {
                let chapter_name = self.infer_chapter_from_unit_id(unit_id)?;
                let mut chapter_data = self.load_chapter_toml(&chapter_name).await?;
                
                // Find the unit and update todo
                if let Some(unit) = chapter_data.units.iter_mut().find(|u| u.id == *unit_id) {
                    if let Some(existing_todo) = unit.todos.iter_mut().find(|t| t.id == todo.id) {
                        *existing_todo = todo.clone();
                    }
                }
                
                self.save_chapter_toml(&chapter_name, &chapter_data).await?;
            }
        }
        
        Ok(())
    }

    /// Find todo in TOML files
    async fn find_todo_in_toml(&self, todo_id: &str) -> Result<Option<Todo>> {
        // Search in project TOML
        if let Ok(project_data) = self.load_project_toml().await {
            if let Some(todo) = project_data.todos.into_iter().find(|t| t.id == todo_id) {
                return Ok(Some(todo));
            }
        }

        // Search in chapter TOML files
        let chapters_dir = Path::new(&self.repo_path).join("content/chapters");
        if chapters_dir.exists() {
            for entry in std::fs::read_dir(chapters_dir)? {
                let entry = entry?;
                if entry.path().extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Ok(chapter_data) = self.load_chapter_toml_by_path(&entry.path()).await {
                        // Search chapter-level todos
                        if let Some(todo) = chapter_data.todos.into_iter().find(|t| t.id == todo_id) {
                            return Ok(Some(todo));
                        }
                        
                        // Search unit-level todos
                        for unit in chapter_data.units {
                            if let Some(todo) = unit.todos.into_iter().find(|t| t.id == todo_id) {
                                return Ok(Some(todo));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Add comment to appropriate TOML file
    async fn add_comment_to_toml(&self, comment: &Comment) -> Result<()> {
        match &comment.context {
            CommentContext::Project => {
                // Comments at project level would go in project TOML
                // For now, we'll add them to a default chapter
                return Err(TradocumentError::ApiError(
                    "Project-level comments not yet implemented".to_string()
                ));
            }
            
            CommentContext::Chapter => {
                // Infer chapter from context
                return Err(TradocumentError::ApiError(
                    "Chapter-level comments need chapter specification".to_string()
                ));
            }
            
            CommentContext::Translation { paragraph, language: _ } => {
                let chapter_name = self.infer_chapter_from_unit_id(paragraph)?;
                let mut chapter_data = self.load_chapter_toml(&chapter_name).await?;
                
                // Find the unit and add comment
                if let Some(unit) = chapter_data.units.iter_mut().find(|u| u.id == *paragraph) {
                    unit.comments.push(comment.clone());
                } else {
                    return Err(TradocumentError::ApiError(
                        format!("Unit {} not found", paragraph)
                    ));
                }
                
                self.save_chapter_toml(&chapter_name, &chapter_data).await?;
            }
        }
        
        Ok(())
    }

    /// Add reply to comment in TOML
    async fn add_reply_to_comment_in_toml(&self, comment_id: &str, reply: &CommentReply) -> Result<()> {
        // Search through all TOML files to find the comment
        let chapters_dir = Path::new(&self.repo_path).join("content/chapters");
        if chapters_dir.exists() {
            for entry in std::fs::read_dir(chapters_dir)? {
                let entry = entry?;
                if entry.path().extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Ok(mut chapter_data) = self.load_chapter_toml_by_path(&entry.path()).await {
                        let mut found = false;
                        
                        // Search for comment in units
                        for unit in &mut chapter_data.units {
                            if let Some(comment) = unit.comments.iter_mut().find(|c| c.id == comment_id) {
                                comment.replies.push(reply.clone());
                                found = true;
                                break;
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
            format!("Comment {} not found", comment_id)
        ))
    }

    /// Update comment resolution status in TOML
    async fn update_comment_resolution_in_toml(&self, comment_id: &str, resolved: bool) -> Result<()> {
        // Search through all TOML files to find the comment
        let chapters_dir = Path::new(&self.repo_path).join("content/chapters");
        if chapters_dir.exists() {
            for entry in std::fs::read_dir(chapters_dir)? {
                let entry = entry?;
                if entry.path().extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Ok(mut chapter_data) = self.load_chapter_toml_by_path(&entry.path()).await {
                        let mut found = false;
                        
                        // Search for comment in units
                        for unit in &mut chapter_data.units {
                            if let Some(comment) = unit.comments.iter_mut().find(|c| c.id == comment_id) {
                                comment.resolved = resolved;
                                found = true;
                                break;
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
            format!("Comment {} not found", comment_id)
        ))
    }

    /// Load project TOML data
    async fn load_project_toml(&self) -> Result<ProjectData> {
        let project_path = Path::new(&self.repo_path).join("content/project.toml");
        
        if project_path.exists() {
            let toml_content = std::fs::read_to_string(&project_path)?;
            let project_data: ProjectData = toml::from_str(&toml_content)
                .map_err(|e| TradocumentError::ApiError(format!("Failed to parse project TOML: {}", e)))?;
            Ok(project_data)
        } else {
            // Create default project data
            Ok(self.create_default_project_data().await?)
        }
    }

    /// Save project TOML data
    async fn save_project_toml(&self, project_data: &ProjectData) -> Result<()> {
        let project_path = Path::new(&self.repo_path).join("content/project.toml");
        
        // Ensure directory exists
        if let Some(parent) = project_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let toml_content = toml::to_string_pretty(project_data)
            .map_err(|e| TradocumentError::ApiError(format!("Failed to serialize project TOML: {}", e)))?;
            
        std::fs::write(&project_path, toml_content)?;
        Ok(())
    }

    /// Load chapter TOML data
    async fn load_chapter_toml(&self, chapter_name: &str) -> Result<ChapterData> {
        let chapter_path = Path::new(&self.repo_path)
            .join("content/chapters")
            .join(format!("{}.toml", chapter_name));
            
        self.load_chapter_toml_by_path(&chapter_path).await
    }

    /// Load chapter TOML data by path
    async fn load_chapter_toml_by_path(&self, chapter_path: &Path) -> Result<ChapterData> {
        if chapter_path.exists() {
            let toml_content = std::fs::read_to_string(chapter_path)?;
            let chapter_data: ChapterData = toml::from_str(&toml_content)
                .map_err(|e| TradocumentError::ApiError(format!("Failed to parse chapter TOML: {}", e)))?;
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
            .join(format!("{}.toml", chapter_name));
        
        // Ensure directory exists
        if let Some(parent) = chapter_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let toml_content = toml::to_string_pretty(chapter_data)
            .map_err(|e| TradocumentError::ApiError(format!("Failed to serialize chapter TOML: {}", e)))?;
            
        std::fs::write(&chapter_path, toml_content)?;
        Ok(())
    }

    /// Create default project data
    async fn create_default_project_data(&self) -> Result<ProjectData> {
        use super::models::{ProjectMetadata, ProjectStatus, ProjectLanguages, ProjectTeam, ProjectSettings};
        
        let project_metadata = ProjectMetadata {
            id: self.project_id.to_string(),
            name: "Translation Project".to_string(),
            description: "Git-integrated translation project".to_string(),
            version: "1.0.0".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status: ProjectStatus::Active,
            languages: ProjectLanguages {
                source: "en".to_string(),
                targets: vec!["de".to_string(), "fr".to_string(), "es".to_string()],
            },
            team: ProjectTeam {
                editor: "editor".to_string(),
                translators: HashMap::new(),
                reviewers: HashMap::new(),
                contributors: None,
            },
            settings: ProjectSettings {
                auto_save_interval: 300,
                quality_threshold: 8.0,
                require_review: true,
                export_on_approval: false,
                git_strategy: Some("feature_branch".to_string()),
            },
            metadata: None,
        };

        Ok(ProjectData {
            project: project_metadata,
            todos: Vec::new(),
        })
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

    /// Infer chapter name from context
    fn infer_chapter_from_context(&self, context: &TodoContext) -> Result<String> {
        match context {
            TodoContext::Chapter => Ok("default_chapter".to_string()),
            TodoContext::Paragraph { unit_id } | TodoContext::Translation { unit_id, .. } => {
                self.infer_chapter_from_unit_id(unit_id)
            }
            _ => Err(TradocumentError::ApiError(
                "Cannot infer chapter from this context".to_string()
            ))
        }
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

    /// Check if todo matches filter
    fn matches_filter(&self, todo: &Todo, filter: &TaskFilter) -> bool {
        if let Some(ref assigned_to) = filter.assigned_to {
            if todo.assigned_to.as_ref() != Some(assigned_to) {
                return false;
            }
        }

        if let Some(ref created_by) = filter.created_by {
            if todo.created_by != *created_by {
                return false;
            }
        }

        if let Some(ref status) = filter.status {
            if todo.status != *status {
                return false;
            }
        }

        if let Some(ref priority) = filter.priority {
            if todo.priority != *priority {
                return false;
            }
        }

        if let Some(ref todo_type) = filter.todo_type {
            if todo.todo_type != *todo_type {
                return false;
            }
        }

        if let Some(ref context_type) = filter.context_type {
            let matches = match (&todo.context, context_type.as_str()) {
                (TodoContext::Project, "project") => true,
                (TodoContext::Chapter, "chapter") => true,
                (TodoContext::Paragraph { .. }, "paragraph") => true,
                (TodoContext::Translation { .. }, "translation") => true,
                _ => false,
            };
            if !matches {
                return false;
            }
        }

        if let Some(due_before) = filter.due_before {
            if let Some(due_date) = todo.due_date {
                if due_date > due_before {
                    return false;
                }
            } else {
                return false; // No due date means it doesn't match "due before" filter
            }
        }

        if let Some(created_after) = filter.created_after {
            if todo.created_at <= created_after {
                return false;
            }
        }

        true
    }

    /// Create Git commit for todo operations
    async fn commit_todo_operation(&self, title: &str, message: &str) -> Result<()> {
        // Use GitWorkflowManager to commit changes
        match self.git_manager.create_commit_with_message(message).await {
            Ok(_) => {
                // Sync with Kanban if the operation was successful
                if let Err(e) = self.kanban_sync.handle_git_commit(title, message).await {
                    eprintln!("Failed to sync Git commit to Kanban: {}", e);
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("Git commit failed for '{}': {}", title, e);
                // Return the original error
                Err(e)
            }
        }
    }

    /// Send notification for todo events
    async fn send_todo_notification(&self, event_type: TaskEventType, todo: &Todo) -> Result<()> {
        let mut affected_users = Vec::new();
        
        // Add assignee
        if let Some(ref assigned_to) = todo.assigned_to {
            affected_users.push(assigned_to.clone());
        }
        
        // Add creator if different from current user
        if todo.created_by != self.current_user.id {
            affected_users.push(todo.created_by.clone());
        }

        let message = match event_type {
            TaskEventType::TodoCreated => format!("New {} todo created: {}", self.format_todo_type(&todo.todo_type), todo.title),
            TaskEventType::TodoAssigned => format!("Todo assigned to you: {}", todo.title),
            TaskEventType::TodoCompleted => format!("Todo completed: {}", todo.title),
            TaskEventType::TodoUpdated => format!("Todo updated: {}", todo.title),
            _ => format!("Todo event: {}", todo.title),
        };

        // Send notifications to all affected users
        for user_id in &affected_users {
            let notification_content = format!("{}\n\nTodo ID: {}\nContext: {}", 
                message, 
                todo.id, 
                self.format_context_detailed(&todo.context)
            );

            // Create notification struct
            let notification = crate::Notification {
                id: Uuid::new_v4(),
                recipient_id: user_id.clone(),
                sender_id: Some(self.current_user.id.clone()),
                notification_type: crate::NotificationType::ChangesRequested, // TODO: Add task-specific types
                title: format!("Task: {}", todo.title),
                message: notification_content,
                metadata: crate::NotificationMetadata {
                    document_id: None,
                    document_title: None,
                    review_id: None,
                    comment_id: None,
                    priority: crate::NotificationPriority::Normal,
                    action_required: false,
                    action_url: None,
                },
                created_at: Utc::now(),
                read_at: None,
                delivered: false,
            };
            
            // Create temporary user object - TODO: Replace with proper user lookup
            let temp_user = User {
                id: user_id.clone(),
                name: format!("User {}", user_id),
                email: format!("{}@example.com", user_id),
                role: crate::UserRole::Translator, // Default role
                created_at: Utc::now(),
                active: true,
            };

            if let Err(e) = self.notification_service.send_notification(
                notification,
                &temp_user,
            ).await {
                eprintln!("Failed to send notification to user {}: {}", user_id, e);
            }
        }

        // Create task notification record
        let task_notification = TaskNotification {
            event_type,
            todo_id: todo.id.clone(),
            created_by: self.current_user.id.clone(),
            affected_users,
            message,
            timestamp: Utc::now(),
        };

        // Log for debugging/audit trail
        println!("Task notification sent: {:?}", task_notification);
        
        Ok(())
    }

    /// Send notification for comment events
    async fn send_comment_notification(&self, event_type: TaskEventType, comment: &Comment) -> Result<()> {
        // Determine affected users based on comment context
        let mut affected_users = Vec::new();
        
        // For translation comments, notify the translator and reviewer for that language
        if let CommentContext::Translation { paragraph: _, language } = &comment.context {
            // Load project data to get team assignments
            if let Ok(project_data) = self.load_project_toml().await {
                if let Some(translator) = project_data.project.team.translators.get(language) {
                    if translator != &self.current_user.id {
                        affected_users.push(translator.clone());
                    }
                }
                if let Some(reviewer) = project_data.project.team.reviewers.get(language) {
                    if reviewer != &self.current_user.id && !affected_users.contains(reviewer) {
                        affected_users.push(reviewer.clone());
                    }
                }
            }
        }

        // Add comment author if different from current user
        if comment.author != self.current_user.id && !affected_users.contains(&comment.author) {
            affected_users.push(comment.author.clone());
        }

        let action = match event_type {
            TaskEventType::CommentAdded => "added",
            TaskEventType::CommentReplied => "replied to",
            TaskEventType::CommentResolved => "resolved",
            _ => "updated",
        };

        let message = format!("Comment {}: {}", action, comment.content);

        // Send notifications to affected users
        for user_id in &affected_users {
            let notification_content = format!("{}\n\nComment ID: {}\nContext: {}\nType: {}", 
                message, 
                comment.id, 
                self.format_comment_context(&comment.context),
                self.format_comment_type(&comment.comment_type)
            );

            // Create notification struct
            let notification = crate::Notification {
                id: Uuid::new_v4(),
                recipient_id: user_id.clone(),
                sender_id: Some(self.current_user.id.clone()),
                notification_type: crate::NotificationType::CommentAdded,
                title: format!("Comment: {}", action),
                message: notification_content,
                metadata: crate::NotificationMetadata {
                    document_id: None,
                    document_title: None,
                    review_id: None,
                    comment_id: Some(comment.id.parse().unwrap_or_else(|_| Uuid::new_v4())),
                    priority: crate::NotificationPriority::Normal,
                    action_required: false,
                    action_url: None,
                },
                created_at: Utc::now(),
                read_at: None,
                delivered: false,
            };
            
            // Create temporary user object - TODO: Replace with proper user lookup
            let temp_user = User {
                id: user_id.clone(),
                name: format!("User {}", user_id),
                email: format!("{}@example.com", user_id),
                role: crate::UserRole::Translator, // Default role
                created_at: Utc::now(),
                active: true,
            };

            if let Err(e) = self.notification_service.send_notification(
                notification,
                &temp_user,
            ).await {
                eprintln!("Failed to send comment notification to user {}: {}", user_id, e);
            }
        }

        // Create task notification record
        let task_notification = TaskNotification {
            event_type: event_type.clone(),
            todo_id: comment.id.clone(),
            created_by: self.current_user.id.clone(),
            affected_users,
            message,
            timestamp: Utc::now(),
        };

        // Log for debugging/audit trail
        println!("Comment notification sent: {:?}", task_notification);
        
        Ok(())
    }

    // Helper methods for progress calculation

    /// Calculate progress for a specific language
    async fn calculate_language_progress(&self, language: &str) -> Result<LanguageProgress> {
        let mut total_units = 0;
        let mut completed_units = 0;
        let mut in_progress_units = 0;
        let mut under_review_units = 0;
        let mut approved_units = 0;
        let mut word_count = 0;
        let mut translated_words = 0;

        // Iterate through all chapters to calculate language progress
        let chapters_dir = Path::new(&self.repo_path).join("content/chapters");
        if chapters_dir.exists() {
            for entry in std::fs::read_dir(chapters_dir)? {
                let entry = entry?;
                if entry.path().extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Ok(chapter_data) = self.load_chapter_toml_by_path(&entry.path()).await {
                        for unit in &chapter_data.units {
                            total_units += 1;
                            word_count += unit.source_text.split_whitespace().count() as u32;

                            if let Some(translation) = unit.translations.get(language) {
                                translated_words += translation.text.split_whitespace().count() as u32;
                                
                                match translation.status {
                                    TranslationUnitStatus::Completed => completed_units += 1,
                                    TranslationUnitStatus::InProgress => in_progress_units += 1,
                                    TranslationUnitStatus::UnderReview => under_review_units += 1,
                                    TranslationUnitStatus::Approved => approved_units += 1,
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        let completion = if total_units > 0 {
            (approved_units + completed_units) as f32 / total_units as f32
        } else {
            0.0
        };

        // Estimate remaining hours (simplified calculation)
        let remaining_units = total_units - completed_units - approved_units;
        let estimated_hours_remaining = remaining_units as f32 * 0.5; // 30 minutes per unit average

        // Calculate quality score from translation metadata
        let quality_score = self.calculate_language_quality_score(language).await.ok();

        Ok(LanguageProgress {
            language: language.to_string(),
            completion,
            total_units,
            completed_units,
            in_progress_units,
            under_review_units,
            approved_units,
            word_count,
            translated_words,
            estimated_hours_remaining,
            quality_score,
        })
    }

    /// Calculate progress for a specific chapter
    async fn calculate_chapter_progress(&self, chapter_name: &str) -> Result<ChapterProgress> {
        let chapter_data = self.load_chapter_toml(chapter_name).await?;
        let mut language_status = HashMap::new();

        // Load project data to get target languages
        let project_data = self.load_project_toml().await?;

        for language in &project_data.project.languages.targets {
            let total_units = chapter_data.units.len() as u32;
            let mut completed_units = 0;

            for unit in &chapter_data.units {
                if let Some(translation) = unit.translations.get(language) {
                    if matches!(translation.status, TranslationUnitStatus::Completed | TranslationUnitStatus::Approved) {
                        completed_units += 1;
                    }
                }
            }

            let completion = if total_units > 0 {
                completed_units as f32 / total_units as f32
            } else {
                0.0
            };

            let status = self.determine_chapter_status_for_language(&chapter_data, language);
            let translator = project_data.project.team.translators.get(language).cloned();
            let reviewer = project_data.project.team.reviewers.get(language).cloned();

            language_status.insert(language.clone(), ChapterLanguageStatus {
                status,
                completion,
                translator,
                reviewer,
                last_updated: chapter_data.chapter.updated_at,
            });
        }

        // Calculate todo stats for this chapter
        let todo_stats = self.calculate_todo_stats(&chapter_data.todos);

        // Overall chapter completion (average across all languages)
        let completion = if language_status.is_empty() {
            0.0
        } else {
            language_status.values().map(|s| s.completion).sum::<f32>() / language_status.len() as f32
        };

        Ok(ChapterProgress {
            chapter: chapter_name.to_string(),
            completion,
            language_status,
            todo_stats,
            last_activity: chapter_data.chapter.updated_at,
        })
    }

    /// Calculate team statistics
    async fn calculate_team_stats(&self, all_todos: &[Todo]) -> Result<TeamStats> {
        let mut productivity_scores = HashMap::new();
        let mut workload_distribution = HashMap::new();
        let mut average_completion_time = HashMap::new();

        // Get unique team members from todos
        let mut all_members = std::collections::HashSet::new();
        for todo in all_todos {
            all_members.insert(todo.created_by.clone());
            if let Some(ref assigned_to) = todo.assigned_to {
                all_members.insert(assigned_to.clone());
            }
        }

        let total_members = all_members.len() as u32;
        let mut active_members = 0;

        // Calculate stats for each member
        for member in &all_members {
            let member_todos: Vec<_> = all_todos
                .iter()
                .filter(|t| t.assigned_to.as_ref() == Some(member) || t.created_by == *member)
                .collect();

            if !member_todos.is_empty() {
                active_members += 1;
            }

            let assigned_todos = all_todos
                .iter()
                .filter(|t| t.assigned_to.as_ref() == Some(member))
                .count() as u32;

            let completed_todos = all_todos
                .iter()
                .filter(|t| t.assigned_to.as_ref() == Some(member) && t.status == TodoStatus::Completed)
                .count() as u32;

            let overdue_todos = all_todos
                .iter()
                .filter(|t| {
                    t.assigned_to.as_ref() == Some(member) 
                    && t.status == TodoStatus::Open 
                    && t.due_date.map_or(false, |due| due < Utc::now())
                })
                .count() as u32;

            // Calculate estimated hours from metadata
            let estimated_hours = member_todos
                .iter()
                .filter_map(|t| t.metadata.as_ref().and_then(|m| m.estimated_hours))
                .sum::<f32>();

            let last_activity = member_todos
                .iter()
                .map(|t| t.created_at)
                .max()
                .unwrap_or_else(Utc::now);

            // Calculate productivity score (completed todos / assigned todos)
            let productivity = if assigned_todos > 0 {
                completed_todos as f32 / assigned_todos as f32
            } else {
                0.0
            };

            productivity_scores.insert(member.clone(), productivity);
            workload_distribution.insert(member.clone(), WorkloadInfo {
                assigned_todos,
                completed_todos,
                overdue_todos,
                estimated_hours,
                last_activity,
            });
        }

        // Calculate average completion time by todo type
        for todo_type in [TodoType::Translation, TodoType::Review, TodoType::Terminology, 
                         TodoType::Revision, TodoType::Screenshot, TodoType::Formatting, TodoType::Research] {
            let completed_of_type: Vec<_> = all_todos
                .iter()
                .filter(|t| t.todo_type == todo_type && t.status == TodoStatus::Completed)
                .collect();

            if !completed_of_type.is_empty() {
                let total_time: f32 = completed_of_type
                    .iter()
                    .filter_map(|t| {
                        if let (Some(created), Some(resolved)) = (Some(t.created_at), t.resolved_at) {
                            Some((resolved - created).num_hours() as f32)
                        } else {
                            None
                        }
                    })
                    .sum();

                let avg_time = total_time / completed_of_type.len() as f32;
                average_completion_time.insert(todo_type, avg_time);
            }
        }

        Ok(TeamStats {
            total_members,
            active_members,
            productivity_scores,
            workload_distribution,
            average_completion_time,
        })
    }

    /// Calculate project timeline
    async fn calculate_timeline(&self, all_todos: &[Todo], project_data: &ProjectData) -> Result<ProgressTimeline> {
        let start_date = project_data.project.created_at;
        
        // Calculate velocity based on completed todos
        let completed_todos = all_todos
            .iter()
            .filter(|t| t.status == TodoStatus::Completed)
            .count();

        let weeks_elapsed = (Utc::now() - start_date).num_weeks() as f32;
        let todos_per_week = if weeks_elapsed > 0.0 {
            completed_todos as f32 / weeks_elapsed
        } else {
            0.0
        };

        // Estimate completion based on remaining todos and current velocity
        let remaining_todos = all_todos
            .iter()
            .filter(|t| t.status == TodoStatus::Open || t.status == TodoStatus::InProgress)
            .count();

        let weeks_to_completion = if todos_per_week > 0.0 {
            remaining_todos as f32 / todos_per_week
        } else {
            52.0 // Default to 1 year if no velocity
        };

        let estimated_completion = Utc::now() + chrono::Duration::weeks(weeks_to_completion as i64);

        // Create basic milestones
        let milestones = vec![
            Milestone {
                name: "Translation Phase Complete".to_string(),
                target_date: estimated_completion - chrono::Duration::weeks(4),
                completion: 0.6, // Estimate based on current progress
                dependencies: vec!["all_translation_todos".to_string()],
                status: MilestoneStatus::InProgress,
            },
            Milestone {
                name: "Review Phase Complete".to_string(),
                target_date: estimated_completion - chrono::Duration::weeks(2),
                completion: 0.3,
                dependencies: vec!["translation_complete".to_string()],
                status: MilestoneStatus::NotStarted,
            },
            Milestone {
                name: "Project Complete".to_string(),
                target_date: estimated_completion,
                completion: 0.1,
                dependencies: vec!["review_complete".to_string()],
                status: MilestoneStatus::NotStarted,
            },
        ];

        let velocity = Velocity {
            todos_per_week,
            words_per_day: HashMap::new(), // TODO: Calculate from translation data
            quality_trend: Vec::new(), // TODO: Calculate from historical data
            efficiency_trend: Vec::new(), // TODO: Calculate from historical data
        };

        Ok(ProgressTimeline {
            start_date,
            target_completion: project_data.project.metadata
                .as_ref()
                .and_then(|m| m.estimated_completion_date.as_ref())
                .and_then(|d| d.parse::<DateTime<Utc>>().ok()),
            estimated_completion,
            milestones,
            velocity,
        })
    }

    /// Calculate quality metrics
    async fn calculate_quality_metrics(&self) -> Result<QualityMetrics> {
        // Simplified quality metrics calculation
        // In a real implementation, this would analyze translation quality scores
        
        Ok(QualityMetrics {
            average_score: 8.0, // TODO: Calculate from actual quality data
            language_scores: HashMap::new(), // TODO: Calculate per language
            reviewer_scores: HashMap::new(), // TODO: Calculate per reviewer
            quality_trend: QualityTrend::Stable, // TODO: Calculate trend
            issues_identified: 0, // TODO: Count from comments
            issues_resolved: 0, // TODO: Count resolved comments
        })
    }

    /// Build context information for a task
    async fn build_task_context_info(&self, todo: &Todo) -> Result<TaskContextInfo> {
        let mut chapter_name = None;
        let mut unit_id = None;
        let mut language = None;

        match &todo.context {
            TodoContext::Chapter => {
                chapter_name = Some("default_chapter".to_string()); // TODO: Determine actual chapter
            }
            TodoContext::Paragraph { unit_id: uid } => {
                unit_id = Some(uid.clone());
                chapter_name = Some(self.infer_chapter_from_unit_id(uid)?);
            }
            TodoContext::Translation { unit_id: uid, language: lang } => {
                unit_id = Some(uid.clone());
                language = Some(lang.clone());
                chapter_name = Some(self.infer_chapter_from_unit_id(uid)?);
            }
            TodoContext::Project => {}
        }

        // TODO: Calculate dependencies and blocking relationships
        let dependencies = Vec::new();
        let blocking = Vec::new();

        Ok(TaskContextInfo {
            chapter_name,
            unit_id,
            language,
            dependencies,
            blocking,
        })
    }

    /// Build progress information for a task
    async fn build_task_progress_info(&self, todo: &Todo) -> Result<TaskProgressInfo> {
        let time_spent = todo.metadata.as_ref().and_then(|m| m.actual_hours);
        let estimated_remaining = todo.metadata.as_ref().and_then(|m| m.estimated_hours);
        let completion_percentage = todo.metadata.as_ref().and_then(|m| m.progress_percent);

        // Use the most recent timestamp as last activity
        let last_activity = todo.resolved_at.unwrap_or_else(|| {
            // Use creation time if not resolved
            todo.created_at
        });

        Ok(TaskProgressInfo {
            time_spent,
            estimated_remaining,
            completion_percentage,
            last_activity,
        })
    }

    /// Calculate language quality score
    async fn calculate_language_quality_score(&self, _language: &str) -> Result<f32> {
        // TODO: Implement actual quality score calculation from translation metadata
        Ok(8.0) // Placeholder
    }

    /// Determine chapter status for a specific language
    fn determine_chapter_status_for_language(&self, chapter_data: &ChapterData, language: &str) -> ChapterStatus {
        let total_units = chapter_data.units.len();
        if total_units == 0 {
            return ChapterStatus::Draft;
        }

        let mut completed = 0;
        let mut in_progress = 0;
        let mut approved = 0;

        for unit in &chapter_data.units {
            if let Some(translation) = unit.translations.get(language) {
                match translation.status {
                    TranslationUnitStatus::Completed => completed += 1,
                    TranslationUnitStatus::InProgress => in_progress += 1,
                    TranslationUnitStatus::Approved => approved += 1,
                    _ => {}
                }
            }
        }

        if approved == total_units {
            ChapterStatus::Published
        } else if approved > 0 || completed == total_units {
            ChapterStatus::Approved
        } else if completed > 0 || in_progress > 0 {
            ChapterStatus::InTranslation
        } else {
            ChapterStatus::Draft
        }
    }

    /// Calculate todo statistics
    fn calculate_todo_stats(&self, todos: &[Todo]) -> TodoStats {
        let total = todos.len() as u32;
        let mut open = 0;
        let mut in_progress = 0;
        let mut completed = 0;
        let mut overdue = 0;
        let mut by_type = HashMap::new();
        let mut by_priority = HashMap::new();

        for todo in todos {
            match todo.status {
                TodoStatus::Open => open += 1,
                TodoStatus::InProgress => in_progress += 1,
                TodoStatus::Completed => completed += 1,
                _ => {}
            }

            if todo.status == TodoStatus::Open && todo.due_date.map_or(false, |due| due < Utc::now()) {
                overdue += 1;
            }

            *by_type.entry(todo.todo_type.clone()).or_insert(0) += 1;
            *by_priority.entry(todo.priority.clone()).or_insert(0) += 1;
        }

        TodoStats {
            total,
            open,
            in_progress,
            completed,
            overdue,
            by_type,
            by_priority,
        }
    }

    /// Format todo type for display
    fn format_todo_type(&self, todo_type: &TodoType) -> String {
        match todo_type {
            TodoType::Translation => "translation",
            TodoType::Review => "review",
            TodoType::Terminology => "terminology",
            TodoType::Revision => "revision",
            TodoType::Screenshot => "screenshot",
            TodoType::Formatting => "formatting",
            TodoType::Research => "research",
        }.to_string()
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

    /// Format context for display
    fn format_context(&self, context: &TodoContext) -> String {
        match context {
            TodoContext::Project => "project".to_string(),
            TodoContext::Chapter => "chapter".to_string(),
            TodoContext::Paragraph { unit_id } => format!("paragraph:{}", unit_id),
            TodoContext::Translation { unit_id, language } => format!("translation:{}:{}", unit_id, language),
        }
    }

    /// Format context with details
    fn format_context_detailed(&self, context: &TodoContext) -> String {
        match context {
            TodoContext::Project => "project-level".to_string(),
            TodoContext::Chapter => "chapter-level".to_string(),
            TodoContext::Paragraph { unit_id } => format!("paragraph-{}", unit_id),
            TodoContext::Translation { unit_id, language } => format!("translation-{}-{}", unit_id, language),
        }
    }

    /// Format comment context for display
    fn format_comment_context(&self, context: &CommentContext) -> String {
        match context {
            CommentContext::Project => "project".to_string(),
            CommentContext::Chapter => "chapter".to_string(),
            CommentContext::Translation { paragraph, language } => format!("translation:{}:{}", paragraph, language),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_task_manager() -> (TaskManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_string_lossy().to_string();
        
        // Create test user
        let user = User {
            id: "test_user".to_string(),
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            role: crate::UserRole::Translator,
            created_at: chrono::Utc::now(),
            active: true,
        };

        // Create mock GitWorkflowManager
        let git_config = super::super::GitConfig::default();
        let git_manager = Arc::new(
            GitWorkflowManager::new(
                temp_dir.path(),
                Uuid::new_v4(),
                user.clone(),
                git_config,
            ).await.unwrap()
        );

        // Create mock KanbanGitSync
        let kanban_sync = Arc::new(
            KanbanGitSync::new(
                temp_dir.path(),
                Uuid::new_v4(),
                git_manager.clone(),
            ).await.unwrap()
        );

        // Create mock NotificationService
        let notification_service = Arc::new(NotificationService::new());

        let task_manager = TaskManager::new(
            git_manager,
            kanban_sync,
            notification_service,
            Uuid::new_v4(),
            &repo_path,
            user,
        ).await.unwrap();

        (task_manager, temp_dir)
    }

    #[tokio::test]
    async fn test_create_todo() {
        let (task_manager, _temp_dir) = create_test_task_manager().await;

        let request = CreateTodoRequest {
            title: "Test todo".to_string(),
            description: Some("Test description".to_string()),
            assigned_to: Some("translator_user".to_string()),
            priority: Priority::High,
            todo_type: TodoType::Translation,
            context: TodoContext::Project,
            due_date: None,
            metadata: None,
        };

        let todo = task_manager.create_todo(request).await.unwrap();
        
        assert_eq!(todo.title, "Test todo");
        assert_eq!(todo.priority, Priority::High);
        assert_eq!(todo.status, TodoStatus::Open);
        assert_eq!(todo.created_by, "test_user");
    }

    #[tokio::test]
    async fn test_filter_todos() {
        let (task_manager, _temp_dir) = create_test_task_manager().await;

        // Create a test todo
        let request = CreateTodoRequest {
            title: "Filtered todo".to_string(),
            description: None,
            assigned_to: Some("test_user".to_string()),
            priority: Priority::Medium,
            todo_type: TodoType::Review,
            context: TodoContext::Chapter,
            due_date: None,
            metadata: None,
        };

        let _todo = task_manager.create_todo(request).await.unwrap();

        // Test filter
        let filter = TaskFilter {
            assigned_to: Some("test_user".to_string()),
            created_by: None,
            status: Some(TodoStatus::Open),
            priority: None,
            todo_type: None,
            context_type: None,
            due_before: None,
            created_after: None,
        };

        let todos = task_manager.list_todos(filter).await.unwrap();
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].title, "Filtered todo");
    }

    #[tokio::test]
    async fn test_complete_todo() {
        let (task_manager, _temp_dir) = create_test_task_manager().await;

        // Create a todo
        let request = CreateTodoRequest {
            title: "Todo to complete".to_string(),
            description: None,
            assigned_to: Some("test_user".to_string()),
            priority: Priority::Low,
            todo_type: TodoType::Translation,
            context: TodoContext::Project,
            due_date: None,
            metadata: None,
        };

        let todo = task_manager.create_todo(request).await.unwrap();

        // Complete the todo
        let completed_todo = task_manager.complete_todo(
            &todo.id,
            Some("Completed successfully".to_string())
        ).await.unwrap();

        assert_eq!(completed_todo.status, TodoStatus::Completed);
        assert!(completed_todo.resolved_at.is_some());
        assert_eq!(completed_todo.resolution, Some("Completed successfully".to_string()));
    }
}