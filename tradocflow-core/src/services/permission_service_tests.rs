use super::*;
use crate::database::{DatabasePool, migrations::run_all_migrations, member_repository::MemberRepository};
use crate::models::{MemberRole, Permission, AddMemberRequest};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

async fn setup_test_db() -> DatabasePool {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    let pool = Arc::new(Mutex::new(conn));
    run_all_migrations(&pool).unwrap();
    pool
}

async fn setup_test_project_with_members(pool: &DatabasePool) -> (Uuid, String, String, String) {
    let project_id = Uuid::new_v4();
    let owner_id = "owner_user";
    let admin_id = "admin_user";
    let translator_id = "translator_user";
    
    // Insert project
    {
        let conn = pool.lock().await;
        conn.execute(
            "INSERT INTO projects (id, name, owner_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                project_id.to_string(),
                "Test Project",
                owner_id,
                chrono::Utc::now().to_rfc3339(),
                chrono::Utc::now().to_rfc3339(),
            ],
        ).unwrap();
    }

    // Add members
    let member_repo = MemberRepository::new(pool.clone());
    
    let admin_request = AddMemberRequest {
        user_id: admin_id.to_string(),
        role: MemberRole::Admin,
    };
    member_repo.add_member(project_id, admin_request, owner_id.to_string()).await.unwrap();
    
    let translator_request = AddMemberRequest {
        user_id: translator_id.to_string(),
        role: MemberRole::Translator,
    };
    member_repo.add_member(project_id, translator_request, owner_id.to_string()).await.unwrap();

    (project_id, owner_id.to_string(), admin_id.to_string(), translator_id.to_string())
}

#[tokio::test]
async fn test_owner_has_all_permissions() {
    let pool = setup_test_db().await;
    let service = PermissionService::new(pool.clone());
    let (project_id, owner_id, _, _) = setup_test_project_with_members(&pool).await;

    let context = PermissionContext {
        user_id: owner_id.clone(),
        project_id,
        resource: "project".to_string(),
        action: "manage".to_string(),
    };

    let has_permission = service.has_permission(&context).await.unwrap();
    assert!(has_permission);

    // Test other permissions
    let contexts = vec![
        ("document", "edit"),
        ("translation", "review"),
        ("team", "manage"),
        ("export", "all"),
        ("analytics", "view"),
    ];

    for (resource, action) in contexts {
        let context = PermissionContext {
            user_id: owner_id.clone(),
            project_id,
            resource: resource.to_string(),
            action: action.to_string(),
        };
        let has_permission = service.has_permission(&context).await.unwrap();
        assert!(has_permission, "Owner should have permission for {}:{}", resource, action);
    }
}

#[tokio::test]
async fn test_admin_role_permissions() {
    let pool = setup_test_db().await;
    let service = PermissionService::new(pool.clone());
    let (project_id, _, admin_id, _) = setup_test_project_with_members(&pool).await;

    // Admin should have most permissions
    let allowed_contexts = vec![
        ("project", "manage"),
        ("document", "edit"),
        ("translation", "edit"),
        ("translation", "review"),
        ("team", "manage"),
        ("terminology", "edit"),
        ("export", "all"),
        ("analytics", "view"),
    ];

    for (resource, action) in allowed_contexts {
        let context = PermissionContext {
            user_id: admin_id.clone(),
            project_id,
            resource: resource.to_string(),
            action: action.to_string(),
        };
        let has_permission = service.has_permission(&context).await.unwrap();
        assert!(has_permission, "Admin should have permission for {}:{}", resource, action);
    }
}

#[tokio::test]
async fn test_translator_role_permissions() {
    let pool = setup_test_db().await;
    let service = PermissionService::new(pool.clone());
    let (project_id, _, _, translator_id) = setup_test_project_with_members(&pool).await;

    // Translator should have limited permissions
    let allowed_contexts = vec![
        ("document", "view"),
        ("document", "edit"),
        ("translation", "edit"),
        ("terminology", "view"),
        ("export", "pdf"),
    ];

    for (resource, action) in allowed_contexts {
        let context = PermissionContext {
            user_id: translator_id.clone(),
            project_id,
            resource: resource.to_string(),
            action: action.to_string(),
        };
        let has_permission = service.has_permission(&context).await.unwrap();
        assert!(has_permission, "Translator should have permission for {}:{}", resource, action);
    }

    // Translator should NOT have these permissions
    let denied_contexts = vec![
        ("project", "manage"),
        ("translation", "review"),
        ("team", "manage"),
        ("terminology", "edit"),
        ("analytics", "view"),
    ];

    for (resource, action) in denied_contexts {
        let context = PermissionContext {
            user_id: translator_id.clone(),
            project_id,
            resource: resource.to_string(),
            action: action.to_string(),
        };
        let has_permission = service.has_permission(&context).await.unwrap();
        assert!(!has_permission, "Translator should NOT have permission for {}:{}", resource, action);
    }
}

#[tokio::test]
async fn test_grant_explicit_permission() {
    let pool = setup_test_db().await;
    let service = PermissionService::new(pool.clone());
    let (project_id, _, admin_id, translator_id) = setup_test_project_with_members(&pool).await;

    // Initially translator cannot review
    let context = PermissionContext {
        user_id: translator_id.clone(),
        project_id,
        resource: "translation".to_string(),
        action: "review".to_string(),
    };
    let has_permission = service.has_permission(&context).await.unwrap();
    assert!(!has_permission);

    // Grant review permission
    let grant_request = GrantPermissionRequest {
        user_id: translator_id.clone(),
        permission: Permission::ReviewTranslations,
        expires_at: None,
    };
    let grant = service.grant_permission(project_id, grant_request, &admin_id).await.unwrap();
    
    assert_eq!(grant.user_id, translator_id);
    assert_eq!(grant.project_id, project_id);
    assert_eq!(grant.permission, Permission::ReviewTranslations);
    assert_eq!(grant.granted_by, admin_id);

    // Now translator should have review permission
    let has_permission = service.has_permission(&context).await.unwrap();
    assert!(has_permission);
}

#[tokio::test]
async fn test_grant_permission_insufficient_permissions() {
    let pool = setup_test_db().await;
    let service = PermissionService::new(pool.clone());
    let (project_id, _, _, translator_id) = setup_test_project_with_members(&pool).await;

    // Translator tries to grant permission (should fail)
    let grant_request = GrantPermissionRequest {
        user_id: "some_user".to_string(),
        permission: Permission::EditTranslations,
        expires_at: None,
    };
    
    let result = service.grant_permission(project_id, grant_request, &translator_id).await;
    assert!(matches!(result, Err(PermissionError::InsufficientPermissions)));
}

#[tokio::test]
async fn test_revoke_permission() {
    let pool = setup_test_db().await;
    let service = PermissionService::new(pool.clone());
    let (project_id, _, admin_id, translator_id) = setup_test_project_with_members(&pool).await;

    // Grant permission first
    let grant_request = GrantPermissionRequest {
        user_id: translator_id.clone(),
        permission: Permission::ReviewTranslations,
        expires_at: None,
    };
    service.grant_permission(project_id, grant_request, &admin_id).await.unwrap();

    // Verify permission exists
    let context = PermissionContext {
        user_id: translator_id.clone(),
        project_id,
        resource: "translation".to_string(),
        action: "review".to_string(),
    };
    let has_permission = service.has_permission(&context).await.unwrap();
    assert!(has_permission);

    // Revoke permission
    let revoked = service.revoke_permission(
        project_id, 
        &translator_id, 
        &Permission::ReviewTranslations, 
        &admin_id
    ).await.unwrap();
    assert!(revoked);

    // Verify permission removed
    let has_permission = service.has_permission(&context).await.unwrap();
    assert!(!has_permission);
}

#[tokio::test]
async fn test_get_user_permissions() {
    let pool = setup_test_db().await;
    let service = PermissionService::new(pool.clone());
    let (project_id, _, admin_id, translator_id) = setup_test_project_with_members(&pool).await;

    // Grant multiple permissions
    let permissions = vec![
        Permission::ReviewTranslations,
        Permission::ManageTerminology,
        Permission::ViewAnalytics,
    ];

    for permission in &permissions {
        let grant_request = GrantPermissionRequest {
            user_id: translator_id.clone(),
            permission: permission.clone(),
            expires_at: None,
        };
        service.grant_permission(project_id, grant_request, &admin_id).await.unwrap();
    }

    // Get user permissions
    let user_permissions = service.get_user_permissions(project_id, &translator_id).await.unwrap();
    
    assert_eq!(user_permissions.len(), 3);
    
    let granted_permissions: Vec<Permission> = user_permissions.iter()
        .map(|p| p.permission.clone())
        .collect();
    
    for permission in permissions {
        assert!(granted_permissions.contains(&permission));
    }
}

#[tokio::test]
async fn test_expired_permissions() {
    let pool = setup_test_db().await;
    let service = PermissionService::new(pool.clone());
    let (project_id, _, admin_id, translator_id) = setup_test_project_with_members(&pool).await;

    // Grant permission with past expiry date
    let grant_request = GrantPermissionRequest {
        user_id: translator_id.clone(),
        permission: Permission::ReviewTranslations,
        expires_at: Some(chrono::Utc::now() - chrono::Duration::hours(1)), // Expired 1 hour ago
    };
    service.grant_permission(project_id, grant_request, &admin_id).await.unwrap();

    // Permission should not be effective
    let context = PermissionContext {
        user_id: translator_id.clone(),
        project_id,
        resource: "translation".to_string(),
        action: "review".to_string(),
    };
    let has_permission = service.has_permission(&context).await.unwrap();
    assert!(!has_permission);

    // Cleanup expired permissions
    let cleaned_up = service.cleanup_expired_permissions().await.unwrap();
    assert_eq!(cleaned_up, 1);
}

#[tokio::test]
async fn test_non_member_no_permissions() {
    let pool = setup_test_db().await;
    let service = PermissionService::new(pool.clone());
    let (project_id, _, _, _) = setup_test_project_with_members(&pool).await;

    let non_member_id = "non_member_user";
    
    let context = PermissionContext {
        user_id: non_member_id.to_string(),
        project_id,
        resource: "document".to_string(),
        action: "view".to_string(),
    };

    let has_permission = service.has_permission(&context).await.unwrap();
    assert!(!has_permission);
}