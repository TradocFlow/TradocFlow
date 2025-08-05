use super::*;
use crate::database::{DatabasePool, migrations::run_all_migrations, member_repository::MemberRepository};
use crate::models::MemberRole;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

async fn setup_test_db() -> DatabasePool {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let pool = Arc::new(Mutex::new(conn));
    run_all_migrations(&pool).unwrap();
    pool
}

#[tokio::test]
async fn test_create_user() {
    let pool = setup_test_db().await;
    let service = UserManagementService::new(pool);

    let request = CreateUserRequest {
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        languages: vec!["en".to_string(), "es".to_string()],
        specializations: vec!["technical".to_string()],
        timezone: Some("UTC".to_string()),
    };

    let user = service.create_user(request).await.unwrap();
    
    assert_eq!(user.name, "John Doe");
    assert_eq!(user.email, "john@example.com");
    assert_eq!(user.profile.languages, vec!["en", "es"]);
    assert_eq!(user.profile.specializations, vec!["technical"]);
    assert_eq!(user.profile.timezone, Some("UTC".to_string()));
    assert!(user.active);
}

#[tokio::test]
async fn test_create_user_validation() {
    let pool = setup_test_db().await;
    let service = UserManagementService::new(pool);

    // Test empty name
    let request = CreateUserRequest {
        name: "".to_string(),
        email: "john@example.com".to_string(),
        languages: vec![],
        specializations: vec![],
        timezone: None,
    };

    let result = service.create_user(request).await;
    assert!(matches!(result, Err(UserManagementError::InvalidInput(_))));

    // Test invalid email
    let request = CreateUserRequest {
        name: "John Doe".to_string(),
        email: "invalid-email".to_string(),
        languages: vec![],
        specializations: vec![],
        timezone: None,
    };

    let result = service.create_user(request).await;
    assert!(matches!(result, Err(UserManagementError::InvalidInput(_))));
}

#[tokio::test]
async fn test_get_user() {
    let pool = setup_test_db().await;
    let service = UserManagementService::new(pool);

    let request = CreateUserRequest {
        name: "Jane Smith".to_string(),
        email: "jane@example.com".to_string(),
        languages: vec!["fr".to_string()],
        specializations: vec!["legal".to_string()],
        timezone: Some("Europe/Paris".to_string()),
    };

    let created_user = service.create_user(request).await.unwrap();
    let retrieved_user = service.get_user(&created_user.id).await.unwrap().unwrap();
    
    assert_eq!(created_user.id, retrieved_user.id);
    assert_eq!(created_user.name, retrieved_user.name);
    assert_eq!(created_user.email, retrieved_user.email);
    assert_eq!(created_user.profile.languages, retrieved_user.profile.languages);
}

#[tokio::test]
async fn test_update_user() {
    let pool = setup_test_db().await;
    let service = UserManagementService::new(pool);

    let request = CreateUserRequest {
        name: "Bob Wilson".to_string(),
        email: "bob@example.com".to_string(),
        languages: vec!["en".to_string()],
        specializations: vec![],
        timezone: None,
    };

    let user = service.create_user(request).await.unwrap();

    let update_request = UpdateUserRequest {
        name: Some("Robert Wilson".to_string()),
        email: None,
        languages: Some(vec!["en".to_string(), "de".to_string()]),
        specializations: Some(vec!["medical".to_string()]),
        timezone: Some("America/New_York".to_string()),
        preferences: None,
    };

    let updated_user = service.update_user(&user.id, update_request).await.unwrap();
    
    assert_eq!(updated_user.name, "Robert Wilson");
    assert_eq!(updated_user.email, "bob@example.com"); // Unchanged
    assert_eq!(updated_user.profile.languages, vec!["en", "de"]);
    assert_eq!(updated_user.profile.specializations, vec!["medical"]);
    assert_eq!(updated_user.profile.timezone, Some("America/New_York".to_string()));
}

#[tokio::test]
async fn test_invite_team_member() {
    let pool = setup_test_db().await;
    let service = UserManagementService::new(pool.clone());
    let member_repo = MemberRepository::new(pool.clone());

    // Create a project and add an admin user
    let project_id = Uuid::new_v4();
    let admin_user_id = "admin_user";
    
    // Insert project
    {
        let conn = pool.lock().await;
        conn.execute(
            "INSERT INTO translation_projects (id, name, source_language, target_languages, project_path, settings, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                project_id.to_string(),
                "Test Project",
                "en",
                "[]",
                "/test/path",
                "{}",
                chrono::Utc::now().to_rfc3339(),
                chrono::Utc::now().to_rfc3339(),
            ],
        ).unwrap();
    }

    // Add admin user to project
    let add_request = crate::models::AddMemberRequest {
        user_id: admin_user_id.to_string(),
        role: MemberRole::Admin,
    };
    member_repo.add_member(project_id, add_request, "system".to_string()).await.unwrap();

    // Test invitation
    let invite_request = InviteTeamMemberRequest {
        email: "newuser@example.com".to_string(),
        role: MemberRole::Translator,
        message: Some("Welcome to the team!".to_string()),
    };

    let invitation = service.invite_team_member(project_id, admin_user_id, invite_request).await.unwrap();
    
    assert_eq!(invitation.project_id, project_id);
    assert_eq!(invitation.inviter_id, admin_user_id);
    assert_eq!(invitation.invitee_email, "newuser@example.com");
    assert_eq!(invitation.role, MemberRole::Translator);
    assert_eq!(invitation.message, Some("Welcome to the team!".to_string()));
    assert_eq!(invitation.status, InvitationStatus::Pending);
}

#[tokio::test]
async fn test_invite_insufficient_permissions() {
    let pool = setup_test_db().await;
    let service = UserManagementService::new(pool.clone());
    let member_repo = MemberRepository::new(pool.clone());

    // Create a project and add a translator user (no management permissions)
    let project_id = Uuid::new_v4();
    let translator_user_id = "translator_user";
    
    // Insert project
    {
        let conn = pool.lock().await;
        conn.execute(
            "INSERT INTO translation_projects (id, name, source_language, target_languages, project_path, settings, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                project_id.to_string(),
                "Test Project",
                "en",
                "[]",
                "/test/path",
                "{}",
                chrono::Utc::now().to_rfc3339(),
                chrono::Utc::now().to_rfc3339(),
            ],
        ).unwrap();
    }

    // Add translator user to project
    let add_request = crate::models::AddMemberRequest {
        user_id: translator_user_id.to_string(),
        role: MemberRole::Translator,
    };
    member_repo.add_member(project_id, add_request, "system".to_string()).await.unwrap();

    // Test invitation should fail
    let invite_request = InviteTeamMemberRequest {
        email: "newuser@example.com".to_string(),
        role: MemberRole::Translator,
        message: None,
    };

    let result = service.invite_team_member(project_id, translator_user_id, invite_request).await;
    assert!(matches!(result, Err(UserManagementError::InsufficientPermissions)));
}

#[tokio::test]
async fn test_accept_invitation() {
    let pool = setup_test_db().await;
    let service = UserManagementService::new(pool.clone());
    let member_repo = MemberRepository::new(pool.clone());

    // Create a project and admin user
    let project_id = Uuid::new_v4();
    let admin_user_id = "admin_user";
    let new_user_id = "new_user";
    
    // Insert project
    {
        let conn = pool.lock().await;
        conn.execute(
            "INSERT INTO translation_projects (id, name, source_language, target_languages, project_path, settings, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                project_id.to_string(),
                "Test Project",
                "en",
                "[]",
                "/test/path",
                "{}",
                chrono::Utc::now().to_rfc3339(),
                chrono::Utc::now().to_rfc3339(),
            ],
        ).unwrap();
    }

    // Add admin user
    let add_request = crate::models::AddMemberRequest {
        user_id: admin_user_id.to_string(),
        role: MemberRole::Admin,
    };
    member_repo.add_member(project_id, add_request, "system".to_string()).await.unwrap();

    // Create invitation
    let invite_request = InviteTeamMemberRequest {
        email: "newuser@example.com".to_string(),
        role: MemberRole::Reviewer,
        message: None,
    };
    let invitation = service.invite_team_member(project_id, admin_user_id, invite_request).await.unwrap();

    // Accept invitation
    let member = service.accept_invitation(invitation.id, new_user_id).await.unwrap();
    
    assert_eq!(member.project_id, project_id);
    assert_eq!(member.user_id, new_user_id);
    assert_eq!(member.role, MemberRole::Reviewer);

    // Verify invitation status updated
    let updated_invitation = service.get_invitation(invitation.id).await.unwrap().unwrap();
    assert_eq!(updated_invitation.status, InvitationStatus::Accepted);
}

#[tokio::test]
async fn test_decline_invitation() {
    let pool = setup_test_db().await;
    let service = UserManagementService::new(pool.clone());
    let member_repo = MemberRepository::new(pool.clone());

    // Create a project and admin user
    let project_id = Uuid::new_v4();
    let admin_user_id = "admin_user";
    
    // Insert project
    {
        let conn = pool.lock().await;
        conn.execute(
            "INSERT INTO translation_projects (id, name, source_language, target_languages, project_path, settings, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                project_id.to_string(),
                "Test Project",
                "en",
                "[]",
                "/test/path",
                "{}",
                chrono::Utc::now().to_rfc3339(),
                chrono::Utc::now().to_rfc3339(),
            ],
        ).unwrap();
    }

    // Add admin user
    let add_request = crate::models::AddMemberRequest {
        user_id: admin_user_id.to_string(),
        role: MemberRole::Admin,
    };
    member_repo.add_member(project_id, add_request, "system".to_string()).await.unwrap();

    // Create invitation
    let invite_request = InviteTeamMemberRequest {
        email: "newuser@example.com".to_string(),
        role: MemberRole::Translator,
        message: None,
    };
    let invitation = service.invite_team_member(project_id, admin_user_id, invite_request).await.unwrap();

    // Decline invitation
    service.decline_invitation(invitation.id).await.unwrap();

    // Verify invitation status updated
    let updated_invitation = service.get_invitation(invitation.id).await.unwrap().unwrap();
    assert_eq!(updated_invitation.status, InvitationStatus::Declined);
}