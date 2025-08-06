use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use rusqlite::OptionalExtension;
use crate::models::{MemberRole, Permission};
use crate::database::DatabasePool;

/// Permission context for checking access
#[derive(Debug, Clone)]
pub struct PermissionContext {
    pub user_id: String,
    pub project_id: Uuid,
    pub resource: String,
    pub action: String,
}

/// Permission grant record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionGrant {
    pub id: Uuid,
    pub user_id: String,
    pub project_id: Uuid,
    pub permission: Permission,
    pub granted_by: String,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Request to grant permission to a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantPermissionRequest {
    pub user_id: String,
    pub permission: Permission,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Permission service for managing granular access control
pub struct PermissionService {
    pool: DatabasePool,
}

impl PermissionService {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }

    /// Check if user has permission for a specific action
    pub async fn has_permission(&self, context: &PermissionContext) -> Result<bool, PermissionError> {
        // First check role-based permissions
        if self.check_role_based_permission(context).await? {
            return Ok(true);
        }

        // Then check explicit permission grants
        self.check_explicit_permission(context).await
    }

    /// Check role-based permissions
    async fn check_role_based_permission(&self, context: &PermissionContext) -> Result<bool, PermissionError> {
        let conn = self.pool.lock().await;
        
        // Check if user is project owner
        let mut owner_stmt = conn.prepare(
            "SELECT owner_id FROM projects WHERE id = ?1"
        ).map_err(|e| PermissionError::DatabaseError(e.to_string()))?;
        
        let owner_id: Option<String> = owner_stmt.query_row(
            rusqlite::params![context.project_id.to_string()], 
            |row| row.get(0)
        ).optional().map_err(|e| PermissionError::DatabaseError(e.to_string()))?;
        
        if let Some(owner) = owner_id {
            if owner == context.user_id {
                return Ok(true); // Project owners have all permissions
            }
        }

        // Check member role permissions
        let mut role_stmt = conn.prepare(
            "SELECT role FROM project_members WHERE project_id = ?1 AND user_id = ?2"
        ).map_err(|e| PermissionError::DatabaseError(e.to_string()))?;
        
        let role: Option<String> = role_stmt.query_row(
            rusqlite::params![context.project_id.to_string(), context.user_id], 
            |row| row.get(0)
        ).optional().map_err(|e| PermissionError::DatabaseError(e.to_string()))?;
        
        if let Some(role_str) = role {
            let member_role = MemberRole::from_str(&role_str);
            return Ok(self.role_has_permission(&member_role, &context.resource, &context.action));
        }

        Ok(false)
    }

    /// Check explicit permission grants
    async fn check_explicit_permission(&self, context: &PermissionContext) -> Result<bool, PermissionError> {
        let conn = self.pool.lock().await;
        let mut stmt = conn.prepare(
            "SELECT COUNT(*) FROM user_permissions 
             WHERE user_id = ?1 AND project_id = ?2 AND permission = ?3 
             AND (expires_at IS NULL OR expires_at > datetime('now'))"
        ).map_err(|e| PermissionError::DatabaseError(e.to_string()))?;
        
        let permission_key = format!("{}:{}", context.resource, context.action);
        let count: i64 = stmt.query_row(
            rusqlite::params![context.user_id, context.project_id.to_string(), permission_key], 
            |row| row.get(0)
        ).map_err(|e| PermissionError::DatabaseError(e.to_string()))?;
        
        Ok(count > 0)
    }

    /// Check if a role has permission for a specific resource and action
    fn role_has_permission(&self, role: &MemberRole, resource: &str, action: &str) -> bool {
        match (resource, action) {
            // Project management permissions
            ("project", "manage") => role.can_manage_project(),
            ("project", "view") => role.can_view_project(),
            
            // Document permissions
            ("document", "edit") => role.can_edit_documents(),
            ("document", "view") => role.can_view_project(),
            ("document", "create") => role.can_edit_documents(),
            ("document", "delete") => role.can_manage_project(),
            
            // Translation permissions
            ("translation", "edit") => role.can_translate(),
            ("translation", "review") => role.can_review_documents(),
            ("translation", "approve") => role.can_review_documents(),
            
            // Team management permissions
            ("team", "manage") => role.can_manage_members(),
            ("team", "invite") => role.can_manage_members(),
            ("team", "remove") => role.can_manage_members(),
            
            // Terminology permissions
            ("terminology", "edit") => matches!(role, MemberRole::Admin | MemberRole::Owner),
            ("terminology", "view") => role.can_view_project(),
            
            // Export permissions
            ("export", "pdf") => role.can_view_project(),
            ("export", "all") => role.can_manage_project(),
            
            // Analytics permissions
            ("analytics", "view") => role.can_manage_project(),
            
            // Default: no permission
            _ => false,
        }
    }

    /// Grant explicit permission to a user
    pub async fn grant_permission(
        &self,
        project_id: Uuid,
        request: GrantPermissionRequest,
        granted_by: &str,
    ) -> Result<PermissionGrant, PermissionError> {
        // Verify granter has permission to grant permissions
        let granter_context = PermissionContext {
            user_id: granted_by.to_string(),
            project_id,
            resource: "team".to_string(),
            action: "manage".to_string(),
        };
        
        if !self.has_permission(&granter_context).await? {
            return Err(PermissionError::InsufficientPermissions);
        }

        let grant = PermissionGrant {
            id: Uuid::new_v4(),
            user_id: request.user_id,
            project_id,
            permission: request.permission.clone(),
            granted_by: granted_by.to_string(),
            granted_at: Utc::now(),
            expires_at: request.expires_at,
        };

        // Store in database
        let conn = self.pool.lock().await;
        let permission_key = format!("{}:{}", 
            self.permission_to_resource(&request.permission),
            self.permission_to_action(&request.permission)
        );
        
        conn.execute(
            "INSERT OR REPLACE INTO user_permissions 
             (id, user_id, project_id, permission, granted_by, granted_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                grant.id.to_string(),
                grant.user_id,
                grant.project_id.to_string(),
                permission_key,
                grant.granted_by,
                grant.granted_at.to_rfc3339(),
                grant.expires_at.map(|dt| dt.to_rfc3339()),
            ],
        ).map_err(|e| PermissionError::DatabaseError(e.to_string()))?;

        Ok(grant)
    }

    /// Revoke permission from a user
    pub async fn revoke_permission(
        &self,
        project_id: Uuid,
        user_id: &str,
        permission: &Permission,
        revoked_by: &str,
    ) -> Result<bool, PermissionError> {
        // Verify revoker has permission to revoke permissions
        let revoker_context = PermissionContext {
            user_id: revoked_by.to_string(),
            project_id,
            resource: "team".to_string(),
            action: "manage".to_string(),
        };
        
        if !self.has_permission(&revoker_context).await? {
            return Err(PermissionError::InsufficientPermissions);
        }

        let conn = self.pool.lock().await;
        let permission_key = format!("{}:{}", 
            self.permission_to_resource(permission),
            self.permission_to_action(permission)
        );
        
        let rows_affected = conn.execute(
            "DELETE FROM user_permissions 
             WHERE user_id = ?1 AND project_id = ?2 AND permission = ?3",
            rusqlite::params![user_id, project_id.to_string(), permission_key],
        ).map_err(|e| PermissionError::DatabaseError(e.to_string()))?;

        Ok(rows_affected > 0)
    }

    /// Get all permissions for a user in a project
    pub async fn get_user_permissions(&self, project_id: Uuid, user_id: &str) -> Result<Vec<PermissionGrant>, PermissionError> {
        let conn = self.pool.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, project_id, permission, granted_by, granted_at, expires_at
             FROM user_permissions 
             WHERE user_id = ?1 AND project_id = ?2 
             AND (expires_at IS NULL OR expires_at > datetime('now'))"
        ).map_err(|e| PermissionError::DatabaseError(e.to_string()))?;

        let permission_iter = stmt.query_map(
            rusqlite::params![user_id, project_id.to_string()], 
            |row| {
                let permission_key: String = row.get(3)?;
                let permission = self.parse_permission_key(&permission_key);
                
                Ok(PermissionGrant {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    user_id: row.get(1)?,
                    project_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap(),
                    permission,
                    granted_by: row.get(4)?,
                    granted_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .unwrap().with_timezone(&Utc),
                    expires_at: row.get::<_, Option<String>>(6)?
                        .map(|s| DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&Utc)),
                })
            }
        ).map_err(|e| PermissionError::DatabaseError(e.to_string()))?;

        let mut permissions = Vec::new();
        for permission in permission_iter {
            permissions.push(permission.map_err(|e| PermissionError::DatabaseError(e.to_string()))?);
        }

        Ok(permissions)
    }

    /// Convert permission enum to resource string
    fn permission_to_resource(&self, permission: &Permission) -> &'static str {
        match permission {
            Permission::EditTranslations => "translation",
            Permission::ReviewTranslations => "translation",
            Permission::ManageTerminology => "terminology",
            Permission::ExportDocuments => "export",
            Permission::ManageTeam => "team",
            Permission::ViewAnalytics => "analytics",
        }
    }

    /// Convert permission enum to action string
    fn permission_to_action(&self, permission: &Permission) -> &'static str {
        match permission {
            Permission::EditTranslations => "edit",
            Permission::ReviewTranslations => "review",
            Permission::ManageTerminology => "edit",
            Permission::ExportDocuments => "all",
            Permission::ManageTeam => "manage",
            Permission::ViewAnalytics => "view",
        }
    }

    /// Parse permission key back to Permission enum
    fn parse_permission_key(&self, key: &str) -> Permission {
        match key {
            "translation:edit" => Permission::EditTranslations,
            "translation:review" => Permission::ReviewTranslations,
            "terminology:edit" => Permission::ManageTerminology,
            "export:all" => Permission::ExportDocuments,
            "team:manage" => Permission::ManageTeam,
            "analytics:view" => Permission::ViewAnalytics,
            _ => Permission::EditTranslations, // Default fallback
        }
    }

    /// Clean up expired permissions
    pub async fn cleanup_expired_permissions(&self) -> Result<u64, PermissionError> {
        let conn = self.pool.lock().await;
        let rows_affected = conn.execute(
            "DELETE FROM user_permissions WHERE expires_at IS NOT NULL AND expires_at <= datetime('now')",
            [],
        ).map_err(|e| PermissionError::DatabaseError(e.to_string()))?;

        Ok(rows_affected as u64)
    }
}

/// Permission service errors
#[derive(Debug, thiserror::Error)]
pub enum PermissionError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("Insufficient permissions")]
    InsufficientPermissions,
    
    #[error("Permission not found")]
    PermissionNotFound,
    
    #[error("Invalid permission: {0}")]
    InvalidPermission(String),
}