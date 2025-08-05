use crate::{Result, TradocumentError, User};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    pub recipient_id: String,
    pub sender_id: Option<String>,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub metadata: NotificationMetadata,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
    pub delivered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    ReviewAssigned,
    ReviewCompleted,
    CommentAdded,
    CommentReply,
    DocumentApproved,
    DocumentRejected,
    ChangesRequested,
    ReviewStatusChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationMetadata {
    pub document_id: Option<Uuid>,
    pub document_title: Option<String>,
    pub review_id: Option<Uuid>,
    pub comment_id: Option<Uuid>,
    pub priority: NotificationPriority,
    pub action_required: bool,
    pub action_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub user_id: String,
    pub email_enabled: bool,
    pub web_enabled: bool,
    pub notification_types: HashMap<NotificationType, bool>,
    pub quiet_hours_start: Option<String>, // HH:MM format
    pub quiet_hours_end: Option<String>,   // HH:MM format
}

#[derive(Debug, Clone)]
pub enum NotificationChannel {
    Email { enabled: bool },
    Web { enabled: bool },
}

impl NotificationChannel {
    pub fn new_email() -> Self {
        Self::Email { enabled: true }
    }
    
    pub fn new_web() -> Self {
        Self::Web { enabled: true }
    }
    
    pub async fn send_notification(&self, notification: &Notification, recipient: &User) -> Result<()> {
        match self {
            NotificationChannel::Email { enabled } => {
                if !enabled {
                    return Ok(());
                }
                
                // In a real implementation, this would send an actual email
                println!(
                    "EMAIL: Sending {} notification to {} ({}): {}",
                    notification.notification_type,
                    recipient.name,
                    recipient.email,
                    notification.title
                );
                
                Ok(())
            }
            NotificationChannel::Web { enabled } => {
                if !enabled {
                    return Ok(());
                }
                
                // In a real implementation, this would send a web push notification or websocket message
                println!(
                    "WEB: Sending {} notification to {}: {}",
                    notification.notification_type,
                    recipient.name,
                    notification.title
                );
                
                Ok(())
            }
        }
    }
    
    pub fn channel_type(&self) -> &'static str {
        match self {
            NotificationChannel::Email { .. } => "email",
            NotificationChannel::Web { .. } => "web",
        }
    }
}

#[derive(Debug)]
pub struct NotificationService {
    notifications: Arc<Mutex<HashMap<Uuid, Notification>>>,
    user_notifications: Arc<Mutex<HashMap<String, Vec<Uuid>>>>, // user_id -> notification_ids
    preferences: Arc<Mutex<HashMap<String, NotificationPreferences>>>,
    channels: Vec<NotificationChannel>,
}

impl NotificationService {
    pub fn new() -> Self {
        let mut service = Self {
            notifications: Arc::new(Mutex::new(HashMap::new())),
            user_notifications: Arc::new(Mutex::new(HashMap::new())),
            preferences: Arc::new(Mutex::new(HashMap::new())),
            channels: Vec::new(),
        };
        
        // Add default channels
        service.channels.push(NotificationChannel::new_email());
        service.channels.push(NotificationChannel::new_web());
        
        service
    }
    
    pub async fn send_notification(&self, mut notification: Notification, recipient: &User) -> Result<()> {
        // Check user preferences
        let preferences = self.get_user_preferences(&recipient.id).await;
        
        if !self.should_send_notification(&notification, &preferences) {
            return Ok(());
        }
        
        // Set delivered flag
        notification.delivered = true;
        let notification_id = notification.id;
        
        // Store notification
        {
            let mut notifications = self.notifications.lock().await;
            notifications.insert(notification_id, notification.clone());
        }
        
        // Add to user's notification list
        {
            let mut user_notifications = self.user_notifications.lock().await;
            let user_notifs = user_notifications.entry(recipient.id.clone()).or_insert_with(Vec::new);
            user_notifs.push(notification_id);
        }
        
        // Send through all enabled channels
        for channel in &self.channels {
            if let Err(e) = channel.send_notification(&notification, recipient).await {
                eprintln!("Failed to send notification via {}: {}", channel.channel_type(), e);
                // Continue with other channels instead of failing completely
            }
        }
        
        Ok(())
    }
    
    pub async fn create_review_assigned_notification(
        &self,
        reviewer: &User,
        document_id: Uuid,
        document_title: &str,
        review_id: Uuid,
        assigner_id: Option<String>,
    ) -> Result<()> {
        let notification = Notification {
            id: Uuid::new_v4(),
            recipient_id: reviewer.id.clone(),
            sender_id: assigner_id,
            notification_type: NotificationType::ReviewAssigned,
            title: "New Review Assignment".to_string(),
            message: format!("You have been assigned to review the document '{document_title}'"),
            metadata: NotificationMetadata {
                document_id: Some(document_id),
                document_title: Some(document_title.to_string()),
                review_id: Some(review_id),
                comment_id: None,
                priority: NotificationPriority::Normal,
                action_required: true,
                action_url: Some(format!("/documents/{document_id}/review/{review_id}")),
            },
            created_at: Utc::now(),
            read_at: None,
            delivered: false,
        };
        
        self.send_notification(notification, reviewer).await
    }
    
    pub async fn create_comment_notification(
        &self,
        recipient: &User,
        commenter: &User,
        document_id: Uuid,
        document_title: &str,
        review_id: Uuid,
        comment_id: Uuid,
        comment_content: &str,
    ) -> Result<()> {
        let notification = Notification {
            id: Uuid::new_v4(),
            recipient_id: recipient.id.clone(),
            sender_id: Some(commenter.id.clone()),
            notification_type: NotificationType::CommentAdded,
            title: "New Comment Added".to_string(),
            message: format!(
                "{} added a comment on '{}': {}",
                commenter.name,
                document_title,
                if comment_content.len() > 100 {
                    format!("{}...", &comment_content[..97])
                } else {
                    comment_content.to_string()
                }
            ),
            metadata: NotificationMetadata {
                document_id: Some(document_id),
                document_title: Some(document_title.to_string()),
                review_id: Some(review_id),
                comment_id: Some(comment_id),
                priority: NotificationPriority::Normal,
                action_required: false,
                action_url: Some(format!("/documents/{document_id}/review/{review_id}#comment-{comment_id}")),
            },
            created_at: Utc::now(),
            read_at: None,
            delivered: false,
        };
        
        self.send_notification(notification, recipient).await
    }
    
    pub async fn create_review_status_change_notification(
        &self,
        recipient: &User,
        reviewer: &User,
        document_id: Uuid,
        document_title: &str,
        review_id: Uuid,
        new_status: &crate::review_system::ReviewStatus,
        reason: Option<&str>,
    ) -> Result<()> {
        let (notification_type, title, message, priority) = match new_status {
            crate::review_system::ReviewStatus::Approved => (
                NotificationType::DocumentApproved,
                "Document Approved".to_string(),
                format!("{} approved the document '{}'", reviewer.name, document_title),
                NotificationPriority::Normal,
            ),
            crate::review_system::ReviewStatus::Rejected => (
                NotificationType::DocumentRejected,
                "Document Rejected".to_string(),
                format!(
                    "{} rejected the document '{}'{}", 
                    reviewer.name, 
                    document_title,
                    if let Some(reason) = reason {
                        format!(": {reason}")
                    } else {
                        String::new()
                    }
                ),
                NotificationPriority::High,
            ),
            crate::review_system::ReviewStatus::ChangesRequested => (
                NotificationType::ChangesRequested,
                "Changes Requested".to_string(),
                format!("{} requested changes to the document '{}'", reviewer.name, document_title),
                NotificationPriority::High,
            ),
            _ => (
                NotificationType::ReviewStatusChanged,
                "Review Status Changed".to_string(),
                format!("{} updated the review status for '{}'", reviewer.name, document_title),
                NotificationPriority::Normal,
            ),
        };
        
        let notification = Notification {
            id: Uuid::new_v4(),
            recipient_id: recipient.id.clone(),
            sender_id: Some(reviewer.id.clone()),
            notification_type,
            title,
            message,
            metadata: NotificationMetadata {
                document_id: Some(document_id),
                document_title: Some(document_title.to_string()),
                review_id: Some(review_id),
                comment_id: None,
                priority,
                action_required: matches!(new_status, crate::review_system::ReviewStatus::ChangesRequested),
                action_url: Some(format!("/documents/{document_id}/review/{review_id}")),
            },
            created_at: Utc::now(),
            read_at: None,
            delivered: false,
        };
        
        self.send_notification(notification, recipient).await
    }
    
    pub async fn get_user_notifications(&self, user_id: &str, unread_only: bool) -> Result<Vec<Notification>> {
        let user_notifications = self.user_notifications.lock().await;
        let notifications = self.notifications.lock().await;
        
        if let Some(notification_ids) = user_notifications.get(user_id) {
            let mut result = Vec::new();
            for &notification_id in notification_ids {
                if let Some(notification) = notifications.get(&notification_id) {
                    if !unread_only || notification.read_at.is_none() {
                        result.push(notification.clone());
                    }
                }
            }
            // Sort by created_at desc (newest first)
            result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            Ok(result)
        } else {
            Ok(Vec::new())
        }
    }
    
    pub async fn mark_notification_read(&self, notification_id: Uuid, user_id: &str) -> Result<()> {
        let mut notifications = self.notifications.lock().await;
        
        if let Some(notification) = notifications.get_mut(&notification_id) {
            if notification.recipient_id == user_id {
                notification.read_at = Some(Utc::now());
                Ok(())
            } else {
                Err(TradocumentError::Notification("Permission denied".to_string()))
            }
        } else {
            Err(TradocumentError::Notification("Notification not found".to_string()))
        }
    }
    
    pub async fn get_unread_count(&self, user_id: &str) -> Result<usize> {
        let notifications = self.get_user_notifications(user_id, true).await?;
        Ok(notifications.len())
    }
    
    async fn get_user_preferences(&self, user_id: &str) -> NotificationPreferences {
        let preferences = self.preferences.lock().await;
        preferences.get(user_id).cloned().unwrap_or_else(|| {
            // Default preferences
            let mut notification_types = HashMap::new();
            notification_types.insert(NotificationType::ReviewAssigned, true);
            notification_types.insert(NotificationType::ReviewCompleted, true);
            notification_types.insert(NotificationType::CommentAdded, true);
            notification_types.insert(NotificationType::CommentReply, true);
            notification_types.insert(NotificationType::DocumentApproved, true);
            notification_types.insert(NotificationType::DocumentRejected, true);
            notification_types.insert(NotificationType::ChangesRequested, true);
            notification_types.insert(NotificationType::ReviewStatusChanged, true);
            
            NotificationPreferences {
                user_id: user_id.to_string(),
                email_enabled: true,
                web_enabled: true,
                notification_types,
                quiet_hours_start: None,
                quiet_hours_end: None,
            }
        })
    }
    
    fn should_send_notification(&self, notification: &Notification, preferences: &NotificationPreferences) -> bool {
        // Check if notification type is enabled
        preferences.notification_types
            .get(&notification.notification_type)
            .copied()
            .unwrap_or(true)
    }
    
    pub async fn update_user_preferences(&self, preferences: NotificationPreferences) -> Result<()> {
        let mut prefs = self.preferences.lock().await;
        prefs.insert(preferences.user_id.clone(), preferences);
        Ok(())
    }
}

impl Default for NotificationService {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationType::ReviewAssigned => write!(f, "ReviewAssigned"),
            NotificationType::ReviewCompleted => write!(f, "ReviewCompleted"),
            NotificationType::CommentAdded => write!(f, "CommentAdded"),
            NotificationType::CommentReply => write!(f, "CommentReply"),
            NotificationType::DocumentApproved => write!(f, "DocumentApproved"),
            NotificationType::DocumentRejected => write!(f, "DocumentRejected"),
            NotificationType::ChangesRequested => write!(f, "ChangesRequested"),
            NotificationType::ReviewStatusChanged => write!(f, "ReviewStatusChanged"),
        }
    }
}

// Add a hash implementation for NotificationType to use in HashMap
impl std::hash::Hash for NotificationType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
    }
}

impl PartialEq for NotificationType {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Eq for NotificationType {}