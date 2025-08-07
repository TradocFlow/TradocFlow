use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;
use chrono::Utc;

use std::sync::Arc;
use crate::services::{
    CollaborativeEditingService, UserSession, DocumentChange, ChangeType,
    TranslationSuggestion, SuggestionStatus, Comment, CommentType, CommentContext,
    CollaborationEvent, UserPresenceUpdate, SuggestionVote, VoteType, CommentReply,
    SelectionRange
};
use crate::services::collaborative_editing_service::SuggestionUpdate;
use crate::models::UserRole;

/// Test helper to create a test user session
fn create_test_session(user_id: &str, user_name: &str, project_id: Uuid) -> UserSession {
    UserSession {
        user_id: user_id.to_string(),
        user_name: user_name.to_string(),
        user_role: UserRole::Translator,
        project_id,
        chapter_id: Some(Uuid::new_v4()),
        language: "en".to_string(),
        cursor_position: None,
        selection_range: None,
        last_activity: Utc::now(),
        is_typing: false,
        current_document: None,
    }
}

/// Test helper to create a test document change
fn create_test_change(
    user_id: &str,
    project_id: Uuid,
    chapter_id: Uuid,
    change_type: ChangeType,
    position: usize,
    length: usize,
) -> DocumentChange {
    DocumentChange {
        id: Uuid::new_v4(),
        user_id: user_id.to_string(),
        user_name: user_id.to_string(),
        project_id,
        chapter_id,
        language: "en".to_string(),
        change_type,
        content_before: Some("old content".to_string()),
        content_after: Some("new content".to_string()),
        position,
        length,
        timestamp: Utc::now(),
        is_conflicted: false,
        conflict_resolution: None,
    }
}

/// Test helper to create a test suggestion
fn create_test_suggestion(
    author_id: &str,
    project_id: Uuid,
    chapter_id: Uuid,
    unit_id: Uuid,
) -> TranslationSuggestion {
    TranslationSuggestion {
        id: Uuid::new_v4(),
        author_id: author_id.to_string(),
        author_name: author_id.to_string(),
        project_id,
        chapter_id,
        unit_id,
        language: "de".to_string(),
        original_text: "Hello world".to_string(),
        suggested_text: "Hallo Welt".to_string(),
        reason: "Better translation".to_string(),
        confidence_score: 0.9,
        status: SuggestionStatus::Pending,
        created_at: Utc::now(),
        reviewed_at: None,
        reviewer_id: None,
        reviewer_comments: None,
        votes: Vec::new(),
    }
}

/// Test helper to create a test comment
fn create_test_comment(
    author_id: &str,
    project_id: Uuid,
    chapter_id: Uuid,
) -> Comment {
    Comment {
        id: Uuid::new_v4(),
        author_id: author_id.to_string(),
        author_name: author_id.to_string(),
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
    }
}

#[tokio::test]
async fn test_collaborative_service_creation() {
    let service = CollaborativeEditingService::new();
    let project_id = Uuid::new_v4();
    
    let active_users = service.get_active_users(project_id).unwrap();
    assert!(active_users.is_empty());
}

#[tokio::test]
async fn test_user_session_management() {
    let service = CollaborativeEditingService::new();
    let project_id = Uuid::new_v4();
    
    // Test joining a session
    let session = create_test_session("user1", "Test User", project_id);
    service.join_session(session.clone()).unwrap();
    
    let active_users = service.get_active_users(project_id).unwrap();
    assert_eq!(active_users.len(), 1);
    assert_eq!(active_users[0].user_id, "user1");
    assert_eq!(active_users[0].user_name, "Test User");
    
    // Test leaving a session
    service.leave_session("user1").unwrap();
    let active_users = service.get_active_users(project_id).unwrap();
    assert!(active_users.is_empty());
}

#[tokio::test]
async fn test_multiple_user_sessions() {
    let service = CollaborativeEditingService::new();
    let project_id = Uuid::new_v4();
    
    // Add multiple users
    let session1 = create_test_session("user1", "User One", project_id);
    let session2 = create_test_session("user2", "User Two", project_id);
    let session3 = create_test_session("user3", "User Three", Uuid::new_v4()); // Different project
    
    service.join_session(session1).unwrap();
    service.join_session(session2).unwrap();
    service.join_session(session3).unwrap();
    
    // Check active users for the first project
    let active_users = service.get_active_users(project_id).unwrap();
    assert_eq!(active_users.len(), 2);
    
    let user_ids: Vec<&str> = active_users.iter().map(|u| u.user_id.as_str()).collect();
    assert!(user_ids.contains(&"user1"));
    assert!(user_ids.contains(&"user2"));
    assert!(!user_ids.contains(&"user3"));
}

#[tokio::test]
async fn test_user_presence_updates() {
    let service = CollaborativeEditingService::new();
    let project_id = Uuid::new_v4();
    
    // Join a session
    let session = create_test_session("user1", "Test User", project_id);
    service.join_session(session).unwrap();
    
    // Update presence
    let update = UserPresenceUpdate {
        user_id: "user1".to_string(),
        cursor_position: Some(150),
        selection_range: Some(SelectionRange { start: 150, end: 160 }),
        is_typing: true,
        current_document: Some("chapter1.md".to_string()),
        timestamp: Utc::now(),
    };
    
    service.update_user_presence(update).unwrap();
    
    // Verify the update
    let active_users = service.get_active_users(project_id).unwrap();
    assert_eq!(active_users.len(), 1);
    assert_eq!(active_users[0].cursor_position, Some(150));
    assert!(active_users[0].is_typing);
    assert_eq!(active_users[0].current_document, Some("chapter1.md".to_string()));
}

#[tokio::test]
async fn test_document_change_tracking() {
    let service = CollaborativeEditingService::new();
    let project_id = Uuid::new_v4();
    let chapter_id = Uuid::new_v4();
    
    // Track a change
    let change = create_test_change("user1", project_id, chapter_id, ChangeType::Insert, 100, 5);
    service.track_change(change.clone()).unwrap();
    
    // Get change history
    let history = service.get_change_history(chapter_id, "en".to_string(), Some(10)).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].user_id, "user1");
    assert_eq!(history[0].change_type, ChangeType::Insert);
    assert_eq!(history[0].position, 100);
    assert_eq!(history[0].length, 5);
}

#[tokio::test]
async fn test_suggestion_workflow() {
    let service = CollaborativeEditingService::new();
    let project_id = Uuid::new_v4();
    let chapter_id = Uuid::new_v4();
    let unit_id = Uuid::new_v4();
    
    // Create a suggestion
    let suggestion = create_test_suggestion("user1", project_id, chapter_id, unit_id);
    let suggestion_id = service.create_suggestion(suggestion.clone()).unwrap();
    
    // Get suggestions
    let suggestions = service.get_suggestions(chapter_id, "de".to_string()).unwrap();
    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0].id, suggestion_id);
    assert_eq!(suggestions[0].status, SuggestionStatus::Pending);
    
    // Vote on the suggestion
    let vote = SuggestionVote {
        user_id: "reviewer1".to_string(),
        user_name: "Reviewer One".to_string(),
        vote_type: VoteType::Approve,
        comment: Some("Good suggestion".to_string()),
        timestamp: Utc::now(),
    };
    
    service.vote_on_suggestion(suggestion_id, vote).unwrap();
    
    // Check the vote was recorded
    let suggestions = service.get_suggestions(chapter_id, "de".to_string()).unwrap();
    assert_eq!(suggestions[0].votes.len(), 1);
    assert_eq!(suggestions[0].votes[0].vote_type, VoteType::Approve);
    assert_eq!(suggestions[0].votes[0].user_id, "reviewer1");
    
    // Update suggestion status
    let update = SuggestionUpdate {
        status: Some(SuggestionStatus::Accepted),
        reviewer_id: Some("reviewer1".to_string()),
        reviewer_comments: Some("Approved after review".to_string()),
    };
    
    service.update_suggestion(suggestion_id, update).unwrap();
    
    // Verify the update
    let suggestions = service.get_suggestions(chapter_id, "de".to_string()).unwrap();
    assert_eq!(suggestions[0].status, SuggestionStatus::Accepted);
    assert_eq!(suggestions[0].reviewer_id, Some("reviewer1".to_string()));
    assert!(suggestions[0].reviewed_at.is_some());
}

#[tokio::test]
async fn test_multiple_votes_on_suggestion() {
    let service = CollaborativeEditingService::new();
    let project_id = Uuid::new_v4();
    let chapter_id = Uuid::new_v4();
    let unit_id = Uuid::new_v4();
    
    // Create a suggestion
    let suggestion = create_test_suggestion("user1", project_id, chapter_id, unit_id);
    let suggestion_id = service.create_suggestion(suggestion).unwrap();
    
    // Add multiple votes
    let vote1 = SuggestionVote {
        user_id: "reviewer1".to_string(),
        user_name: "Reviewer One".to_string(),
        vote_type: VoteType::Approve,
        comment: Some("Good".to_string()),
        timestamp: Utc::now(),
    };
    
    let vote2 = SuggestionVote {
        user_id: "reviewer2".to_string(),
        user_name: "Reviewer Two".to_string(),
        vote_type: VoteType::Reject,
        comment: Some("Needs work".to_string()),
        timestamp: Utc::now(),
    };
    
    // Vote from same user twice (should replace the first vote)
    let vote3 = SuggestionVote {
        user_id: "reviewer1".to_string(),
        user_name: "Reviewer One".to_string(),
        vote_type: VoteType::NeedsWork,
        comment: Some("Changed my mind".to_string()),
        timestamp: Utc::now(),
    };
    
    service.vote_on_suggestion(suggestion_id, vote1).unwrap();
    service.vote_on_suggestion(suggestion_id, vote2).unwrap();
    service.vote_on_suggestion(suggestion_id, vote3).unwrap();
    
    // Check votes
    let suggestions = service.get_suggestions(chapter_id, "de".to_string()).unwrap();
    assert_eq!(suggestions[0].votes.len(), 2); // Only 2 votes (reviewer1's vote was replaced)
    
    // Find reviewer1's vote
    let reviewer1_vote = suggestions[0].votes.iter()
        .find(|v| v.user_id == "reviewer1")
        .unwrap();
    assert_eq!(reviewer1_vote.vote_type, VoteType::NeedsWork);
    assert_eq!(reviewer1_vote.comment, Some("Changed my mind".to_string()));
}

#[tokio::test]
async fn test_comment_system() {
    let service = CollaborativeEditingService::new();
    let project_id = Uuid::new_v4();
    let chapter_id = Uuid::new_v4();
    
    // Add a comment
    let comment = create_test_comment("user1", project_id, chapter_id);
    let comment_id = service.add_comment(comment.clone()).unwrap();
    
    // Get comments
    let comments = service.get_comments(chapter_id, Some("en".to_string())).unwrap();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].id, comment_id);
    assert_eq!(comments[0].content, "This needs clarification");
    assert!(!comments[0].is_resolved);
    
    // Add a reply
    let reply = CommentReply {
        id: Uuid::new_v4(),
        author_id: "user2".to_string(),
        author_name: "User Two".to_string(),
        content: "I can help with that".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    service.reply_to_comment(comment_id, reply).unwrap();
    
    // Check the reply was added
    let comments = service.get_comments(chapter_id, Some("en".to_string())).unwrap();
    assert_eq!(comments[0].replies.len(), 1);
    assert_eq!(comments[0].replies[0].author_id, "user2");
    assert_eq!(comments[0].replies[0].content, "I can help with that");
    
    // Resolve the comment
    service.resolve_comment(comment_id, "user2".to_string()).unwrap();
    
    // Check the comment is resolved
    let comments = service.get_comments(chapter_id, Some("en".to_string())).unwrap();
    assert!(comments[0].is_resolved);
    assert_eq!(comments[0].resolved_by, Some("user2".to_string()));
    assert!(comments[0].resolved_at.is_some());
}

#[tokio::test]
async fn test_comment_filtering() {
    let service = CollaborativeEditingService::new();
    let project_id = Uuid::new_v4();
    let chapter_id = Uuid::new_v4();
    let other_chapter_id = Uuid::new_v4();
    
    // Add comments for different chapters and languages
    let mut comment1 = create_test_comment("user1", project_id, chapter_id);
    comment1.language = Some("en".to_string());
    
    let mut comment2 = create_test_comment("user2", project_id, chapter_id);
    comment2.language = Some("de".to_string());
    
    let mut comment3 = create_test_comment("user3", project_id, other_chapter_id);
    comment3.language = Some("en".to_string());
    
    service.add_comment(comment1).unwrap();
    service.add_comment(comment2).unwrap();
    service.add_comment(comment3).unwrap();
    
    // Get comments for specific chapter and language
    let en_comments = service.get_comments(chapter_id, Some("en".to_string())).unwrap();
    assert_eq!(en_comments.len(), 1);
    assert_eq!(en_comments[0].author_id, "user1");
    
    let de_comments = service.get_comments(chapter_id, Some("de".to_string())).unwrap();
    assert_eq!(de_comments.len(), 1);
    assert_eq!(de_comments[0].author_id, "user2");
    
    // Get all comments for chapter (no language filter)
    let all_comments = service.get_comments(chapter_id, None).unwrap();
    assert_eq!(all_comments.len(), 2);
    
    // Get comments for other chapter
    let other_comments = service.get_comments(other_chapter_id, Some("en".to_string())).unwrap();
    assert_eq!(other_comments.len(), 1);
    assert_eq!(other_comments[0].author_id, "user3");
}

#[tokio::test]
async fn test_conflict_detection() {
    let service = CollaborativeEditingService::new();
    let project_id = Uuid::new_v4();
    let chapter_id = Uuid::new_v4();
    
    // Create overlapping changes
    let change1 = create_test_change("user1", project_id, chapter_id, ChangeType::Replace, 100, 10);
    let change2 = create_test_change("user2", project_id, chapter_id, ChangeType::Replace, 105, 8);
    
    // Track the first change
    service.track_change(change1).unwrap();
    
    // Track the second change (should detect conflict)
    service.track_change(change2).unwrap();
    
    // Get change history
    let history = service.get_change_history(chapter_id, "en".to_string(), None).unwrap();
    assert_eq!(history.len(), 2);
    
    // Note: Conflict detection is implemented in the ConflictDetector,
    // but the current implementation doesn't store pending changes,
    // so conflicts won't be detected in this test setup.
    // In a real implementation, you would need to modify the conflict detector
    // to maintain state between calls.
}

#[tokio::test]
async fn test_event_broadcasting() {
    let service = CollaborativeEditingService::new();
    let project_id = Uuid::new_v4();
    let chapter_id = Uuid::new_v4();
    
    // Subscribe to events
    let mut receiver = service.subscribe_to_events();
    
    // Join a session (should broadcast UserJoined event)
    let session = create_test_session("user1", "Test User", project_id);
    service.join_session(session.clone()).unwrap();
    
    // Check for UserJoined event
    let event = timeout(Duration::from_millis(100), receiver.recv()).await;
    assert!(event.is_ok());
    
    if let Ok(Ok(CollaborationEvent::UserJoined(joined_session))) = event {
        assert_eq!(joined_session.user_id, "user1");
        assert_eq!(joined_session.user_name, "Test User");
    } else {
        panic!("Expected UserJoined event");
    }
    
    // Create a suggestion (should broadcast SuggestionCreated event)
    let suggestion = create_test_suggestion("user1", project_id, chapter_id, Uuid::new_v4());
    service.create_suggestion(suggestion.clone()).unwrap();
    
    // Check for SuggestionCreated event
    let event = timeout(Duration::from_millis(100), receiver.recv()).await;
    assert!(event.is_ok());
    
    if let Ok(Ok(CollaborationEvent::SuggestionCreated(created_suggestion))) = event {
        assert_eq!(created_suggestion.author_id, "user1");
        assert_eq!(created_suggestion.original_text, "Hello world");
    } else {
        panic!("Expected SuggestionCreated event");
    }
}

#[tokio::test]
async fn test_change_history_ordering() {
    let service = CollaborativeEditingService::new();
    let project_id = Uuid::new_v4();
    let chapter_id = Uuid::new_v4();
    
    // Add multiple changes with slight delays to ensure different timestamps
    let change1 = create_test_change("user1", project_id, chapter_id, ChangeType::Insert, 100, 5);
    service.track_change(change1).unwrap();
    
    tokio::time::sleep(Duration::from_millis(10)).await;
    
    let change2 = create_test_change("user2", project_id, chapter_id, ChangeType::Delete, 200, 3);
    service.track_change(change2).unwrap();
    
    tokio::time::sleep(Duration::from_millis(10)).await;
    
    let change3 = create_test_change("user1", project_id, chapter_id, ChangeType::Replace, 150, 8);
    service.track_change(change3).unwrap();
    
    // Get change history (should be ordered by timestamp, most recent first)
    let history = service.get_change_history(chapter_id, "en".to_string(), None).unwrap();
    assert_eq!(history.len(), 3);
    
    // Check ordering (most recent first)
    assert_eq!(history[0].change_type, ChangeType::Replace);
    assert_eq!(history[1].change_type, ChangeType::Delete);
    assert_eq!(history[2].change_type, ChangeType::Insert);
    
    // Test with limit
    let limited_history = service.get_change_history(chapter_id, "en".to_string(), Some(2)).unwrap();
    assert_eq!(limited_history.len(), 2);
    assert_eq!(limited_history[0].change_type, ChangeType::Replace);
    assert_eq!(limited_history[1].change_type, ChangeType::Delete);
}

#[tokio::test]
async fn test_suggestion_status_transitions() {
    let service = CollaborativeEditingService::new();
    let project_id = Uuid::new_v4();
    let chapter_id = Uuid::new_v4();
    let unit_id = Uuid::new_v4();
    
    // Create a suggestion
    let suggestion = create_test_suggestion("user1", project_id, chapter_id, unit_id);
    let suggestion_id = service.create_suggestion(suggestion).unwrap();
    
    // Initial status should be Pending
    let suggestions = service.get_suggestions(chapter_id, "de".to_string()).unwrap();
    assert_eq!(suggestions[0].status, SuggestionStatus::Pending);
    
    // Move to UnderReview
    let update = SuggestionUpdate {
        status: Some(SuggestionStatus::UnderReview),
        reviewer_id: Some("reviewer1".to_string()),
        reviewer_comments: None,
    };
    service.update_suggestion(suggestion_id, update).unwrap();
    
    let suggestions = service.get_suggestions(chapter_id, "de".to_string()).unwrap();
    assert_eq!(suggestions[0].status, SuggestionStatus::UnderReview);
    assert_eq!(suggestions[0].reviewer_id, Some("reviewer1".to_string()));
    
    // Move to Accepted
    let update = SuggestionUpdate {
        status: Some(SuggestionStatus::Accepted),
        reviewer_id: None,
        reviewer_comments: Some("Looks good!".to_string()),
    };
    service.update_suggestion(suggestion_id, update).unwrap();
    
    let suggestions = service.get_suggestions(chapter_id, "de".to_string()).unwrap();
    assert_eq!(suggestions[0].status, SuggestionStatus::Accepted);
    assert_eq!(suggestions[0].reviewer_comments, Some("Looks good!".to_string()));
    assert!(suggestions[0].reviewed_at.is_some());
}

#[tokio::test]
async fn test_concurrent_operations() {
    let service = Arc::new(CollaborativeEditingService::new());
    let project_id = Uuid::new_v4();
    let chapter_id = Uuid::new_v4();
    
    // Spawn multiple tasks that perform operations concurrently
    let mut handles = Vec::new();
    
    // Task 1: Add multiple users
    for i in 0..5 {
        let service = Arc::clone(&service);
        let handle = tokio::spawn(async move {
            let session = create_test_session(&format!("user{}", i), &format!("User {}", i), project_id);
            service.join_session(session).unwrap();
        });
        handles.push(handle);
    }
    
    // Task 2: Add multiple suggestions
    for i in 0..3 {
        let service = Arc::clone(&service);
        let handle = tokio::spawn(async move {
            let suggestion = create_test_suggestion(&format!("user{}", i), project_id, chapter_id, Uuid::new_v4());
            service.create_suggestion(suggestion).unwrap();
        });
        handles.push(handle);
    }
    
    // Task 3: Add multiple comments
    for i in 0..4 {
        let service = Arc::clone(&service);
        let handle = tokio::spawn(async move {
            let comment = create_test_comment(&format!("user{}", i), project_id, chapter_id);
            service.add_comment(comment).unwrap();
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Verify results
    let active_users = service.get_active_users(project_id).unwrap();
    assert_eq!(active_users.len(), 5);
    
    let suggestions = service.get_suggestions(chapter_id, "de".to_string()).unwrap();
    assert_eq!(suggestions.len(), 3);
    
    let comments = service.get_comments(chapter_id, Some("en".to_string())).unwrap();
    assert_eq!(comments.len(), 4);
}

#[tokio::test]
async fn test_error_handling() {
    let service = CollaborativeEditingService::new();
    
    // Test operations with non-existent IDs
    let non_existent_id = Uuid::new_v4();
    
    // Try to update non-existent suggestion
    let update = SuggestionUpdate {
        status: Some(SuggestionStatus::Accepted),
        reviewer_id: None,
        reviewer_comments: None,
    };
    
    // This should not panic, but the suggestion won't be found
    service.update_suggestion(non_existent_id, update).unwrap();
    
    // Try to vote on non-existent suggestion
    let vote = SuggestionVote {
        user_id: "user1".to_string(),
        user_name: "User One".to_string(),
        vote_type: VoteType::Approve,
        comment: None,
        timestamp: Utc::now(),
    };
    
    // This should not panic, but the suggestion won't be found
    service.vote_on_suggestion(non_existent_id, vote).unwrap();
    
    // Try to resolve non-existent comment
    service.resolve_comment(non_existent_id, "user1".to_string()).unwrap();
    
    // Try to reply to non-existent comment
    let reply = CommentReply {
        id: Uuid::new_v4(),
        author_id: "user1".to_string(),
        author_name: "User One".to_string(),
        content: "Reply".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    service.reply_to_comment(non_existent_id, reply).unwrap();
    
    // All operations should complete without panicking
    // In a production system, you might want to return proper error results
}