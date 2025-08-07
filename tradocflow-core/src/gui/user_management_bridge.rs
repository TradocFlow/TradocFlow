use std::sync::Arc;
use uuid::Uuid;
use crate::services::{
    UserManagementService, PermissionService, User, CreateUserRequest, UpdateUserRequest,
    InviteTeamMemberRequest, TeamInvitation, PermissionContext
};
use crate::models::{MemberRole, MemberWithUserInfo};
use crate::database::DatabasePool;

/// Bridge between Slint UI and user management services
pub struct UserManagementBridge {
    user_service: Arc<UserManagementService>,
    permission_service: Arc<PermissionService>,
    current_user_id: Option<String>,
    current_project_id: Option<Uuid>,
}

impl UserManagementBridge {
    pub fn new(pool: DatabasePool) -> Self {
        Self {
            user_service: Arc::new(UserManagementService::new(pool.clone())),
            permission_service: Arc::new(PermissionService::new(pool)),
            current_user_id: None,
            current_project_id: None,
        }
    }

    pub fn set_current_user(&mut self, user_id: String) {
        self.current_user_id = Some(user_id);
    }

    pub fn set_current_project(&mut self, project_id: Uuid) {
        self.current_project_id = Some(project_id);
    }

    /// Convert MemberWithUserInfo to a simple representation
    fn member_to_info(&self, member: &MemberWithUserInfo) -> String {
        format!("{} ({}) - {:?}", member.user_name, member.user_email, member.role)
    }

    /// Invite a team member (simplified version without UI callbacks)
    pub async fn invite_team_member(
        &self,
        project_id: Uuid,
        email: String,
        role: MemberRole,
        message: Option<String>,
    ) -> Result<TeamInvitation, String> {
        if let Some(current_user_id) = &self.current_user_id {
            let request = InviteTeamMemberRequest {
                email,
                role,
                message,
            };
            
            self.user_service.invite_team_member(project_id, current_user_id, request).await
                .map_err(|e| e.to_string())
        } else {
            Err("No current user set".to_string())
        }
    }

    /// Check if current user has permission for an action
    pub async fn check_permission(&self, resource: &str, action: &str) -> bool {
        if let (Some(user_id), Some(project_id)) = (&self.current_user_id, &self.current_project_id) {
            let context = PermissionContext {
                user_id: user_id.clone(),
                project_id: *project_id,
                resource: resource.to_string(),
                action: action.to_string(),
            };
            
            self.permission_service.has_permission(&context).await.unwrap_or(false)
        } else {
            false
        }
    }

    /// Get current user information
    pub async fn get_current_user(&self) -> Option<User> {
        if let Some(user_id) = &self.current_user_id {
            self.user_service.get_user(user_id).await.ok().flatten()
        } else {
            None
        }
    }

    /// Create a new user
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User, String> {
        self.user_service.create_user(request).await
            .map_err(|e| e.to_string())
    }

    /// Update user information
    pub async fn update_user(&self, user_id: &str, request: UpdateUserRequest) -> Result<User, String> {
        self.user_service.update_user(user_id, request).await
            .map_err(|e| e.to_string())
    }

    /// Get pending invitations for a user
    pub async fn get_pending_invitations(&self, _email: &str) -> Vec<TeamInvitation> {
        // TODO: Implement get_pending_invitations in UserManagementService
        vec![]
    }

    /// Accept an invitation
    pub async fn accept_invitation(&self, invitation_id: Uuid) -> Result<(), String> {
        if let Some(user_id) = &self.current_user_id {
            self.user_service.accept_invitation(invitation_id, user_id).await
                .map(|_| ())
                .map_err(|e| e.to_string())
        } else {
            Err("No current user set".to_string())
        }
    }

    /// Decline an invitation
    pub async fn decline_invitation(&self, invitation_id: Uuid) -> Result<(), String> {
        self.user_service.decline_invitation(invitation_id).await
            .map_err(|e| e.to_string())
    }
}