use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMember {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: String,
    pub role: MemberRole,
    pub added_at: DateTime<Utc>,
    pub added_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemberRole {
    #[serde(rename = "owner")]
    Owner,
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "member")]
    Member,
    #[serde(rename = "viewer")]
    Viewer,
    #[serde(rename = "translator")]
    Translator,
    #[serde(rename = "reviewer")]
    Reviewer,
}

impl MemberRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemberRole::Owner => "owner",
            MemberRole::Admin => "admin",
            MemberRole::Member => "member",
            MemberRole::Viewer => "viewer",
            MemberRole::Translator => "translator",
            MemberRole::Reviewer => "reviewer",
        }
    }
    
    pub fn from_str(s: &str) -> Self {
        match s {
            "owner" => MemberRole::Owner,
            "admin" => MemberRole::Admin,
            "member" => MemberRole::Member,
            "viewer" => MemberRole::Viewer,
            "translator" => MemberRole::Translator,
            "reviewer" => MemberRole::Reviewer,
            _ => MemberRole::Member, // Default fallback
        }
    }
    
    pub fn can_manage_project(&self) -> bool {
        matches!(self, MemberRole::Owner | MemberRole::Admin)
    }
    
    pub fn can_edit_documents(&self) -> bool {
        matches!(self, MemberRole::Owner | MemberRole::Admin | MemberRole::Member | MemberRole::Translator)
    }
    
    pub fn can_review_documents(&self) -> bool {
        matches!(self, MemberRole::Owner | MemberRole::Admin | MemberRole::Reviewer)
    }
    
    pub fn can_manage_members(&self) -> bool {
        matches!(self, MemberRole::Owner | MemberRole::Admin)
    }
    
    pub fn can_view_project(&self) -> bool {
        true // All project members can view the project
    }
    
    pub fn can_manage_projects(&self) -> bool {
        matches!(self, MemberRole::Owner | MemberRole::Admin)
    }
    
    pub fn can_translate(&self) -> bool {
        matches!(self, MemberRole::Owner | MemberRole::Admin | MemberRole::Member | MemberRole::Translator)
    }
    
    pub fn hierarchy_level(&self) -> u8 {
        match self {
            MemberRole::Owner => 5,
            MemberRole::Admin => 4,
            MemberRole::Member => 3,
            MemberRole::Translator => 3,
            MemberRole::Reviewer => 3,
            MemberRole::Viewer => 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: String,
    pub role: MemberRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMemberRoleRequest {
    pub role: MemberRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberWithUserInfo {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: String,
    pub user_name: String,
    pub user_email: String,
    pub role: MemberRole,
    pub added_at: DateTime<Utc>,
    pub added_by: String,
    pub added_by_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMembershipInfo {
    pub project_id: Uuid,
    pub user_id: String,
    pub role: MemberRole,
    pub permissions: MemberPermissions,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberPermissions {
    pub can_manage_project: bool,
    pub can_edit_documents: bool,
    pub can_review_documents: bool,
    pub can_manage_members: bool,
    pub can_view_project: bool,
}

impl From<MemberRole> for MemberPermissions {
    fn from(role: MemberRole) -> Self {
        Self {
            can_manage_project: role.can_manage_project(),
            can_edit_documents: role.can_edit_documents(),
            can_review_documents: role.can_review_documents(),
            can_manage_members: role.can_manage_members(),
            can_view_project: role.can_view_project(),
        }
    }
}