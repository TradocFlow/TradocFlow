use crate::{Result, NotificationService, User};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub id: Uuid,
    pub document_id: Uuid,
    pub reviewer_id: String,
    pub status: ReviewStatus,
    pub comments: Vec<Comment>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReviewStatus {
    Pending,
    InProgress,
    Approved,
    Rejected,
    ChangesRequested,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: Uuid,
    pub author_id: String,
    pub content: String,
    pub position: CommentPosition,
    pub created_at: DateTime<Utc>,
    pub resolved: bool,
    pub replies: Vec<CommentReply>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentPosition {
    pub line_start: u32,
    pub line_end: u32,
    pub column_start: u32,
    pub column_end: u32,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentReply {
    pub author_id: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRequest {
    pub id: Uuid,
    pub document_id: Uuid,
    pub author_id: String,
    pub change_type: ChangeType,
    pub old_content: String,
    pub new_content: String,
    pub position: CommentPosition,
    pub status: ChangeStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Insert,
    Delete,
    Replace,
    FormatChange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeStatus {
    Proposed,
    Accepted,
    Rejected,
}

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug)]
pub struct ReviewSystem {
    reviews: HashMap<Uuid, Review>,
    comments: HashMap<Uuid, Vec<Comment>>,
    change_requests: HashMap<Uuid, Vec<ChangeRequest>>,
    notification_service: Option<Arc<NotificationService>>,
}

impl ReviewSystem {
    pub fn new() -> Self {
        Self {
            reviews: HashMap::new(),
            comments: HashMap::new(),
            change_requests: HashMap::new(),
            notification_service: None,
        }
    }
    
    pub fn with_notifications(mut self, notification_service: Arc<NotificationService>) -> Self {
        self.notification_service = Some(notification_service);
        self
    }
    
    pub fn set_notification_service(&mut self, notification_service: Arc<NotificationService>) {
        self.notification_service = Some(notification_service);
    }

    pub async fn create_review(
        &mut self, 
        document_id: Uuid, 
        reviewer_id: String,
        document_title: &str,
        reviewer: &User,
        assigner_id: Option<String>,
    ) -> Result<Review> {
        let review = Review {
            id: Uuid::new_v4(),
            document_id,
            reviewer_id,
            status: ReviewStatus::Pending,
            comments: Vec::new(),
            created_at: Utc::now(),
            completed_at: None,
        };
        
        self.reviews.insert(review.id, review.clone());
        self.comments.insert(review.id, Vec::new());
        
        // Send notification if service is available
        if let Some(notification_service) = &self.notification_service {
            if let Err(e) = notification_service.create_review_assigned_notification(
                reviewer,
                document_id,
                document_title,
                review.id,
                assigner_id,
            ).await {
                // Log error but don't fail the review creation
                eprintln!("Failed to send review assignment notification: {}", e);
            }
        }
        
        Ok(review)
    }
    
    // Keep the sync version for backward compatibility
    pub fn create_review_sync(&mut self, document_id: Uuid, reviewer_id: String) -> Result<Review> {
        let review = Review {
            id: Uuid::new_v4(),
            document_id,
            reviewer_id,
            status: ReviewStatus::Pending,
            comments: Vec::new(),
            created_at: Utc::now(),
            completed_at: None,
        };
        
        self.reviews.insert(review.id, review.clone());
        self.comments.insert(review.id, Vec::new());
        Ok(review)
    }

    pub async fn add_comment(
        &mut self, 
        review_id: Uuid, 
        comment: Comment,
        document_title: &str,
        commenter: &User,
        recipients: &[User],
    ) -> Result<()> {
        if let Some(comments) = self.comments.get_mut(&review_id) {
            comments.push(comment.clone());
            
            // Update review status if it was pending
            let review = if let Some(review) = self.reviews.get_mut(&review_id) {
                if review.status == ReviewStatus::Pending {
                    review.status = ReviewStatus::InProgress;
                }
                review.clone()
            } else {
                return Err(crate::TradocumentError::Review("Review not found".to_string()));
            };
            
            // Send notifications if service is available
            if let Some(notification_service) = &self.notification_service {
                for recipient in recipients {
                    // Don't notify the commenter about their own comment
                    if recipient.id != commenter.id {
                        if let Err(e) = notification_service.create_comment_notification(
                            recipient,
                            commenter,
                            review.document_id,
                            document_title,
                            review_id,
                            comment.id,
                            &comment.content,
                        ).await {
                            eprintln!("Failed to send comment notification to {}: {}", recipient.name, e);
                        }
                    }
                }
            }
            
            Ok(())
        } else {
            Err(crate::TradocumentError::Review("Review not found".to_string()))
        }
    }
    
    // Keep the sync version for backward compatibility
    pub fn add_comment_sync(&mut self, review_id: Uuid, comment: Comment) -> Result<()> {
        if let Some(comments) = self.comments.get_mut(&review_id) {
            comments.push(comment);
            
            // Update review status if it was pending
            if let Some(review) = self.reviews.get_mut(&review_id) {
                if review.status == ReviewStatus::Pending {
                    review.status = ReviewStatus::InProgress;
                }
            }
            Ok(())
        } else {
            Err(crate::TradocumentError::Review("Review not found".to_string()))
        }
    }

    pub async fn approve_document(
        &mut self, 
        document_id: Uuid, 
        reviewer_id: String,
        document_title: &str,
        reviewer: &User,
        document_author: &User,
    ) -> Result<()> {
        // Find the review for this document and reviewer
        for review in self.reviews.values_mut() {
            if review.document_id == document_id && review.reviewer_id == reviewer_id {
                review.status = ReviewStatus::Approved;
                review.completed_at = Some(Utc::now());
                
                // Send notification if service is available
                if let Some(notification_service) = &self.notification_service {
                    if let Err(e) = notification_service.create_review_status_change_notification(
                        document_author,
                        reviewer,
                        document_id,
                        document_title,
                        review.id,
                        &ReviewStatus::Approved,
                        None,
                    ).await {
                        eprintln!("Failed to send approval notification: {}", e);
                    }
                }
                
                return Ok(());
            }
        }
        Err(crate::TradocumentError::Review("Review not found for document and reviewer".to_string()))
    }
    
    // Keep the sync version for backward compatibility
    pub fn approve_document_sync(&mut self, document_id: Uuid, reviewer_id: String) -> Result<()> {
        // Find the review for this document and reviewer
        for review in self.reviews.values_mut() {
            if review.document_id == document_id && review.reviewer_id == reviewer_id {
                review.status = ReviewStatus::Approved;
                review.completed_at = Some(Utc::now());
                return Ok(());
            }
        }
        Err(crate::TradocumentError::Review("Review not found for document and reviewer".to_string()))
    }

    pub async fn reject_document(
        &mut self, 
        document_id: Uuid, 
        reviewer_id: String, 
        reason: String,
        document_title: &str,
        reviewer: &User,
        document_author: &User,
    ) -> Result<()> {
        for review in self.reviews.values_mut() {
            if review.document_id == document_id && review.reviewer_id == reviewer_id {
                review.status = ReviewStatus::Rejected;
                review.completed_at = Some(Utc::now());
                
                // Add a comment with the rejection reason
                let comment = Comment {
                    id: Uuid::new_v4(),
                    author_id: reviewer_id.clone(),
                    content: format!("Document rejected: {}", reason),
                    position: CommentPosition {
                        line_start: 0,
                        line_end: 0,
                        column_start: 0,
                        column_end: 0,
                        language: "en".to_string(),
                    },
                    created_at: Utc::now(),
                    resolved: false,
                    replies: Vec::new(),
                };
                
                if let Some(comments) = self.comments.get_mut(&review.id) {
                    comments.push(comment);
                }
                
                // Send notification if service is available
                if let Some(notification_service) = &self.notification_service {
                    if let Err(e) = notification_service.create_review_status_change_notification(
                        document_author,
                        reviewer,
                        document_id,
                        document_title,
                        review.id,
                        &ReviewStatus::Rejected,
                        Some(&reason),
                    ).await {
                        eprintln!("Failed to send rejection notification: {}", e);
                    }
                }
                
                return Ok(());
            }
        }
        Err(crate::TradocumentError::Review("Review not found for document and reviewer".to_string()))
    }
    
    // Keep the sync version for backward compatibility
    pub fn reject_document_sync(&mut self, document_id: Uuid, reviewer_id: String, reason: String) -> Result<()> {
        for review in self.reviews.values_mut() {
            if review.document_id == document_id && review.reviewer_id == reviewer_id {
                review.status = ReviewStatus::Rejected;
                review.completed_at = Some(Utc::now());
                
                // Add a comment with the rejection reason
                let comment = Comment {
                    id: Uuid::new_v4(),
                    author_id: reviewer_id,
                    content: format!("Document rejected: {}", reason),
                    position: CommentPosition {
                        line_start: 0,
                        line_end: 0,
                        column_start: 0,
                        column_end: 0,
                        language: "en".to_string(),
                    },
                    created_at: Utc::now(),
                    resolved: false,
                    replies: Vec::new(),
                };
                
                if let Some(comments) = self.comments.get_mut(&review.id) {
                    comments.push(comment);
                }
                
                return Ok(());
            }
        }
        Err(crate::TradocumentError::Review("Review not found for document and reviewer".to_string()))
    }

    pub async fn request_changes(
        &mut self, 
        document_id: Uuid, 
        reviewer_id: String, 
        change_request: ChangeRequest,
        document_title: &str,
        reviewer: &User,
        document_author: &User,
    ) -> Result<()> {
        for review in self.reviews.values_mut() {
            if review.document_id == document_id && review.reviewer_id == reviewer_id {
                review.status = ReviewStatus::ChangesRequested;
                
                if let Some(requests) = self.change_requests.get_mut(&review.id) {
                    requests.push(change_request);
                } else {
                    self.change_requests.insert(review.id, vec![change_request]);
                }
                
                // Send notification if service is available
                if let Some(notification_service) = &self.notification_service {
                    if let Err(e) = notification_service.create_review_status_change_notification(
                        document_author,
                        reviewer,
                        document_id,
                        document_title,
                        review.id,
                        &ReviewStatus::ChangesRequested,
                        None,
                    ).await {
                        eprintln!("Failed to send changes requested notification: {}", e);
                    }
                }
                
                return Ok(());
            }
        }
        Err(crate::TradocumentError::Review("Review not found for document and reviewer".to_string()))
    }
    
    // Keep the sync version for backward compatibility
    pub fn request_changes_sync(&mut self, document_id: Uuid, reviewer_id: String, change_request: ChangeRequest) -> Result<()> {
        for review in self.reviews.values_mut() {
            if review.document_id == document_id && review.reviewer_id == reviewer_id {
                review.status = ReviewStatus::ChangesRequested;
                
                if let Some(requests) = self.change_requests.get_mut(&review.id) {
                    requests.push(change_request);
                } else {
                    self.change_requests.insert(review.id, vec![change_request]);
                }
                
                return Ok(());
            }
        }
        Err(crate::TradocumentError::Review("Review not found for document and reviewer".to_string()))
    }

    pub fn get_reviews_for_document(&self, document_id: Uuid) -> Vec<&Review> {
        self.reviews.values().filter(|r| r.document_id == document_id).collect()
    }

    pub fn get_comments_for_review(&self, review_id: Uuid) -> Option<&Vec<Comment>> {
        self.comments.get(&review_id)
    }

    pub fn get_change_requests_for_review(&self, review_id: Uuid) -> Option<&Vec<ChangeRequest>> {
        self.change_requests.get(&review_id)
    }

    pub fn resolve_comment(&mut self, review_id: Uuid, comment_id: Uuid) -> Result<()> {
        if let Some(comments) = self.comments.get_mut(&review_id) {
            for comment in comments.iter_mut() {
                if comment.id == comment_id {
                    comment.resolved = true;
                    return Ok(());
                }
            }
        }
        Err(crate::TradocumentError::Review("Comment not found".to_string()))
    }

    pub fn add_comment_reply(&mut self, review_id: Uuid, comment_id: Uuid, reply: CommentReply) -> Result<()> {
        if let Some(comments) = self.comments.get_mut(&review_id) {
            for comment in comments.iter_mut() {
                if comment.id == comment_id {
                    comment.replies.push(reply);
                    return Ok(());
                }
            }
        }
        Err(crate::TradocumentError::Review("Comment not found".to_string()))
    }
}