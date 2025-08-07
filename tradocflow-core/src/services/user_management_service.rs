use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use rusqlite::OptionalExtension;
use crate::models::{MemberRole, ProjectMember, AddMemberRequest};
use crate::database::{DatabasePool, member_repository::MemberRepository};

/// Enhanced user model for translation system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub active: bool,
    pub profile: UserProfile,
}

/// User profile with translation-specific information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub languages: Vec<String>,
    pub specializations: Vec<String>,
    pub timezone: Option<String>,
    pub preferences: UserPreferences,
}

/// User preferences for the translation system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub default_language: Option<String>,
    pub editor_theme: String,
    pub auto_save: bool,
    pub notifications_enabled: bool,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            default_language: None,
            editor_theme: "default".to_string(),
            auto_save: true,
            notifications_enabled: true,
        }
    }
}

/// Request to create a new user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    pub languages: Vec<String>,
    pub specializations: Vec<String>,
    pub timezone: Option<String>,
}

/// Request to update user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub languages: Option<Vec<String>>,
    pub specializations: Option<Vec<String>>,
    pub timezone: Option<String>,
    pub preferences: Option<UserPreferences>,
}

/// Team member invitation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamInvitation {
    pub id: Uuid,
    pub project_id: Uuid,
    pub inviter_id: String,
    pub invitee_email: String,
    pub role: MemberRole,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: InvitationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Declined,
    Expired,
}

/// Request to invite a team member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteTeamMemberRequest {
    pub email: String,
    pub role: MemberRole,
    pub message: Option<String>,
}

/// User management service
pub struct UserManagementService {
    pool: DatabasePool,
    member_repository: Arc<MemberRepository>,
}

impl UserManagementService {
    pub fn new(pool: DatabasePool) -> Self {
        let member_repository = Arc::new(MemberRepository::new(pool.clone()));
        Self {
            pool,
            member_repository,
        }
    }

    /// Create a new user
    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User, UserManagementError> {
        self.validate_create_user_request(&request)?;
        
        let user_id = format!("user_{}", Uuid::new_v4());
        let now = Utc::now();
        
        let user = User {
            id: user_id.clone(),
            name: request.name,
            email: request.email,
            created_at: now,
            updated_at: now,
            active: true,
            profile: UserProfile {
                languages: request.languages,
                specializations: request.specializations,
                timezone: request.timezone,
                preferences: UserPreferences::default(),
            },
        };

        // Store user in database
        let conn = self.pool.lock().await;
        conn.execute(
            "INSERT INTO users (id, name, email, created_at, updated_at, active, languages, specializations, timezone, preferences)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                user.id,
                user.name,
                user.email,
                user.created_at.to_rfc3339(),
                user.updated_at.to_rfc3339(),
                user.active,
                serde_json::to_string(&user.profile.languages).unwrap(),
                serde_json::to_string(&user.profile.specializations).unwrap(),
                user.profile.timezone,
                serde_json::to_string(&user.profile.preferences).unwrap(),
            ],
        ).map_err(|e| UserManagementError::DatabaseError(e.to_string()))?;

        Ok(user)
    }

    /// Get user by ID
    pub async fn get_user(&self, user_id: &str) -> Result<Option<User>, UserManagementError> {
        let conn = self.pool.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, name, email, created_at, updated_at, active, languages, specializations, timezone, preferences
             FROM users WHERE id = ?1"
        ).map_err(|e| UserManagementError::DatabaseError(e.to_string()))?;

        let user = stmt.query_row(rusqlite::params![user_id], |row| {
            let languages: Vec<String> = serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default();
            let specializations: Vec<String> = serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default();
            let preferences: UserPreferences = serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default();
            
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .unwrap().with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .unwrap().with_timezone(&Utc),
                active: row.get(5)?,
                profile: UserProfile {
                    languages,
                    specializations,
                    timezone: row.get(8)?,
                    preferences,
                },
            })
        }).optional().map_err(|e| UserManagementError::DatabaseError(e.to_string()))?;

        Ok(user)
    }

    /// Update user information
    pub async fn update_user(&self, user_id: &str, request: UpdateUserRequest) -> Result<User, UserManagementError> {
        let mut user = self.get_user(user_id).await?
            .ok_or_else(|| UserManagementError::UserNotFound(user_id.to_string()))?;

        // Update fields if provided
        if let Some(name) = request.name {
            user.name = name;
        }
        if let Some(email) = request.email {
            user.email = email;
        }
        if let Some(languages) = request.languages {
            user.profile.languages = languages;
        }
        if let Some(specializations) = request.specializations {
            user.profile.specializations = specializations;
        }
        if let Some(timezone) = request.timezone {
            user.profile.timezone = Some(timezone);
        }
        if let Some(preferences) = request.preferences {
            user.profile.preferences = preferences;
        }

        user.updated_at = Utc::now();

        // Save to database
        let conn = self.pool.lock().await;
        conn.execute(
            "UPDATE users SET name = ?1, email = ?2, updated_at = ?3, languages = ?4, specializations = ?5, timezone = ?6, preferences = ?7
             WHERE id = ?8",
            rusqlite::params![
                user.name,
                user.email,
                user.updated_at.to_rfc3339(),
                serde_json::to_string(&user.profile.languages).unwrap(),
                serde_json::to_string(&user.profile.specializations).unwrap(),
                user.profile.timezone,
                serde_json::to_string(&user.profile.preferences).unwrap(),
                user_id,
            ],
        ).map_err(|e| UserManagementError::DatabaseError(e.to_string()))?;

        Ok(user)
    }

    /// Invite a team member to a project
    pub async fn invite_team_member(
        &self,
        project_id: Uuid,
        inviter_id: &str,
        request: InviteTeamMemberRequest,
    ) -> Result<TeamInvitation, UserManagementError> {
        // Validate inviter has permission to invite
        let inviter_role = self.member_repository.get_member_role(project_id, inviter_id).await
            .map_err(|e| UserManagementError::DatabaseError(e.to_string()))?;
        
        if !inviter_role.map_or(false, |role| role.can_manage_members()) {
            return Err(UserManagementError::InsufficientPermissions);
        }

        let invitation = TeamInvitation {
            id: Uuid::new_v4(),
            project_id,
            inviter_id: inviter_id.to_string(),
            invitee_email: request.email,
            role: request.role,
            message: request.message,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::days(7), // 7 days expiry
            status: InvitationStatus::Pending,
        };

        // Store invitation in database
        let conn = self.pool.lock().await;
        conn.execute(
            "INSERT INTO team_invitations (id, project_id, inviter_id, invitee_email, role, message, created_at, expires_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                invitation.id.to_string(),
                invitation.project_id.to_string(),
                invitation.inviter_id,
                invitation.invitee_email,
                invitation.role.as_str(),
                invitation.message,
                invitation.created_at.to_rfc3339(),
                invitation.expires_at.to_rfc3339(),
                "pending",
            ],
        ).map_err(|e| UserManagementError::DatabaseError(e.to_string()))?;

        Ok(invitation)
    }

    /// Accept a team invitation
    pub async fn accept_invitation(&self, invitation_id: Uuid, user_id: &str) -> Result<ProjectMember, UserManagementError> {
        let invitation = self.get_invitation(invitation_id).await?
            .ok_or_else(|| UserManagementError::InvitationNotFound(invitation_id.to_string()))?;

        if invitation.status != InvitationStatus::Pending {
            return Err(UserManagementError::InvitationAlreadyProcessed);
        }

        if invitation.expires_at < Utc::now() {
            return Err(UserManagementError::InvitationExpired);
        }

        // Add user to project
        let add_request = AddMemberRequest {
            user_id: user_id.to_string(),
            role: invitation.role,
        };

        let member = self.member_repository.add_member(
            invitation.project_id,
            add_request,
            invitation.inviter_id.clone(),
        ).await.map_err(|e| UserManagementError::DatabaseError(e.to_string()))?;

        // Update invitation status
        self.update_invitation_status(invitation_id, InvitationStatus::Accepted).await?;

        Ok(member)
    }

    /// Decline a team invitation
    pub async fn decline_invitation(&self, invitation_id: Uuid) -> Result<(), UserManagementError> {
        self.update_invitation_status(invitation_id, InvitationStatus::Declined).await
    }

    /// Get invitation by ID
    pub async fn get_invitation(&self, invitation_id: Uuid) -> Result<Option<TeamInvitation>, UserManagementError> {
        let conn = self.pool.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, inviter_id, invitee_email, role, message, created_at, expires_at, status
             FROM team_invitations WHERE id = ?1"
        ).map_err(|e| UserManagementError::DatabaseError(e.to_string()))?;

        let invitation = stmt.query_row(rusqlite::params![invitation_id.to_string()], |row| {
            Ok(TeamInvitation {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                inviter_id: row.get(2)?,
                invitee_email: row.get(3)?,
                role: MemberRole::from_str(&row.get::<_, String>(4)?),
                message: row.get(5)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .unwrap().with_timezone(&Utc),
                expires_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .unwrap().with_timezone(&Utc),
                status: match row.get::<_, String>(8)?.as_str() {
                    "pending" => InvitationStatus::Pending,
                    "accepted" => InvitationStatus::Accepted,
                    "declined" => InvitationStatus::Declined,
                    "expired" => InvitationStatus::Expired,
                    _ => InvitationStatus::Pending,
                },
            })
        }).optional().map_err(|e| UserManagementError::DatabaseError(e.to_string()))?;

        Ok(invitation)
    }

    /// Update invitation status
    async fn update_invitation_status(&self, invitation_id: Uuid, status: InvitationStatus) -> Result<(), UserManagementError> {
        let status_str = match status {
            InvitationStatus::Pending => "pending",
            InvitationStatus::Accepted => "accepted",
            InvitationStatus::Declined => "declined",
            InvitationStatus::Expired => "expired",
        };

        let conn = self.pool.lock().await;
        conn.execute(
            "UPDATE team_invitations SET status = ?1 WHERE id = ?2",
            rusqlite::params![status_str, invitation_id.to_string()],
        ).map_err(|e| UserManagementError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Validate create user request
    fn validate_create_user_request(&self, request: &CreateUserRequest) -> Result<(), UserManagementError> {
        if request.name.trim().is_empty() {
            return Err(UserManagementError::InvalidInput("Name cannot be empty".to_string()));
        }

        if request.email.trim().is_empty() || !request.email.contains('@') {
            return Err(UserManagementError::InvalidInput("Invalid email address".to_string()));
        }

        Ok(())
    }
}

/// User management errors
#[derive(Debug, thiserror::Error)]
pub enum UserManagementError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    
    #[error("User not found: {0}")]
    UserNotFound(String),
    
    #[error("Invitation not found: {0}")]
    InvitationNotFound(String),
    
    #[error("Invitation already processed")]
    InvitationAlreadyProcessed,
    
    #[error("Invitation expired")]
    InvitationExpired,
    
    #[error("Insufficient permissions")]
    InsufficientPermissions,
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}