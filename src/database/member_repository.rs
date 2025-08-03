use rusqlite::{params, Row, Result as SqlResult};
use uuid::Uuid;
use chrono::Utc;
use crate::database::{DatabasePool, datetime_to_string, string_to_datetime};
use rusqlite::OptionalExtension;
use crate::models::{ProjectMember, MemberRole, AddMemberRequest, MemberWithUserInfo, ProjectMembershipInfo};

pub struct MemberRepository {
    pool: DatabasePool,
}

impl MemberRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
    
    pub async fn add_member(&self, project_id: Uuid, request: AddMemberRequest, added_by: String) -> SqlResult<ProjectMember> {
        let conn = self.pool.lock().await;
        let id = Uuid::new_v4();
        let now = Utc::now();
        
        let member = ProjectMember {
            id,
            project_id,
            user_id: request.user_id.clone(),
            role: request.role,
            added_at: now,
            added_by: added_by.clone(),
        };
        
        conn.execute(
            "INSERT INTO project_members (id, project_id, user_id, role, added_at, added_by)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id.to_string(),
                project_id.to_string(),
                request.user_id,
                member.role.as_str(),
                datetime_to_string(now),
                added_by
            ],
        )?;
        
        Ok(member)
    }
    
    pub async fn get_project_members(&self, project_id: Uuid) -> SqlResult<Vec<MemberWithUserInfo>> {
        let conn = self.pool.lock().await;
        let mut stmt = conn.prepare(
            "SELECT 
                pm.id, pm.project_id, pm.user_id, pm.role, pm.added_at, pm.added_by,
                u.name as user_name, u.email as user_email,
                adder.name as added_by_name
             FROM project_members pm
             JOIN users u ON pm.user_id = u.id
             LEFT JOIN users adder ON pm.added_by = adder.id
             WHERE pm.project_id = ?1
             ORDER BY pm.added_at DESC"
        )?;
        
        let member_iter = stmt.query_map(params![project_id.to_string()], |row| {
            Ok(MemberWithUserInfo {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
                user_id: row.get(2)?,
                role: MemberRole::from_str(&row.get::<_, String>(3)?),
                added_at: string_to_datetime(&row.get::<_, String>(4)?).unwrap(),
                added_by: row.get(5)?,
                user_name: row.get(6)?,
                user_email: row.get(7)?,
                added_by_name: row.get::<_, Option<String>>(8)?.unwrap_or_else(|| "Unknown".to_string()),
            })
        })?;
        
        let mut members = Vec::new();
        for member in member_iter {
            members.push(member?);
        }
        
        Ok(members)
    }
    
    pub async fn get_user_projects(&self, user_id: &str) -> SqlResult<Vec<ProjectMembershipInfo>> {
        let conn = self.pool.lock().await;
        let mut stmt = conn.prepare(
            "SELECT pm.project_id, pm.user_id, pm.role, pm.added_at
             FROM project_members pm
             WHERE pm.user_id = ?1
             ORDER BY pm.added_at DESC"
        )?;
        
        let membership_iter = stmt.query_map(params![user_id], |row| {
            let role = MemberRole::from_str(&row.get::<_, String>(2)?);
            Ok(ProjectMembershipInfo {
                project_id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                user_id: row.get(1)?,
                role: role.clone(),
                permissions: role.into(),
                joined_at: string_to_datetime(&row.get::<_, String>(3)?).unwrap(),
            })
        })?;
        
        let mut memberships = Vec::new();
        for membership in membership_iter {
            memberships.push(membership?);
        }
        
        Ok(memberships)
    }
    
    pub async fn update_member_role(&self, project_id: Uuid, user_id: &str, new_role: MemberRole) -> SqlResult<Option<ProjectMember>> {
        let conn = self.pool.lock().await;
        let rows_affected = conn.execute(
            "UPDATE project_members SET role = ?1 WHERE project_id = ?2 AND user_id = ?3",
            params![new_role.as_str(), project_id.to_string(), user_id],
        )?;
        
        if rows_affected > 0 {
            self.get_member(project_id, user_id).await
        } else {
            Ok(None)
        }
    }
    
    pub async fn remove_member(&self, project_id: Uuid, user_id: &str) -> SqlResult<bool> {
        let conn = self.pool.lock().await;
        let rows_affected = conn.execute(
            "DELETE FROM project_members WHERE project_id = ?1 AND user_id = ?2",
            params![project_id.to_string(), user_id],
        )?;
        
        Ok(rows_affected > 0)
    }
    
    pub async fn get_member(&self, project_id: Uuid, user_id: &str) -> SqlResult<Option<ProjectMember>> {
        let conn = self.pool.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, user_id, role, added_at, added_by
             FROM project_members WHERE project_id = ?1 AND user_id = ?2"
        )?;
        
        let member = stmt.query_row(params![project_id.to_string(), user_id], |row| {
            self.row_to_member(row)
        }).optional()?;
        
        Ok(member)
    }
    
    pub async fn is_member(&self, project_id: Uuid, user_id: &str) -> SqlResult<bool> {
        let conn = self.pool.lock().await;
        let mut stmt = conn.prepare(
            "SELECT COUNT(*) FROM project_members WHERE project_id = ?1 AND user_id = ?2"
        )?;
        
        let count: i64 = stmt.query_row(params![project_id.to_string(), user_id], |row| {
            Ok(row.get(0)?)
        })?;
        
        Ok(count > 0)
    }
    
    pub async fn get_member_role(&self, project_id: Uuid, user_id: &str) -> SqlResult<Option<MemberRole>> {
        // First check if user is project owner
        let conn = self.pool.lock().await;
        let mut owner_stmt = conn.prepare(
            "SELECT owner_id FROM projects WHERE id = ?1"
        )?;
        
        let owner_id: String = owner_stmt.query_row(params![project_id.to_string()], |row| {
            Ok(row.get(0)?)
        })?;
        
        if owner_id == user_id {
            return Ok(Some(MemberRole::Owner));
        }
        
        // Then check project membership
        let mut stmt = conn.prepare(
            "SELECT role FROM project_members WHERE project_id = ?1 AND user_id = ?2"
        )?;
        
        let role = stmt.query_row(params![project_id.to_string(), user_id], |row| {
            Ok(MemberRole::from_str(&row.get::<_, String>(0)?))
        }).optional()?;
        
        Ok(role)
    }
    
    fn row_to_member(&self, row: &Row) -> SqlResult<ProjectMember> {
        Ok(ProjectMember {
            id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
            project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
            user_id: row.get(2)?,
            role: MemberRole::from_str(&row.get::<_, String>(3)?),
            added_at: string_to_datetime(&row.get::<_, String>(4)?).unwrap(),
            added_by: row.get(5)?,
        })
    }
}