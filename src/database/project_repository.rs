use rusqlite::{params, Row, Result as SqlResult};
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;
use crate::database::{DatabasePool, datetime_to_string, string_to_datetime};
use rusqlite::OptionalExtension;
use crate::models::{Project, ProjectStatus, Priority, CreateProjectRequest, UpdateProjectRequest, ProjectSummary};

pub struct ProjectRepository {
    pool: DatabasePool,
}

impl ProjectRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
    
    pub async fn create(&self, request: CreateProjectRequest, owner_id: String) -> SqlResult<Project> {
        let conn = self.pool.lock().await;
        let id = Uuid::new_v4();
        let now = Utc::now();
        
        let project = Project {
            id,
            name: request.name.clone(),
            description: request.description.clone(),
            status: ProjectStatus::Active,
            owner_id: owner_id.clone(),
            created_at: now,
            updated_at: now,
            due_date: request.due_date,
            priority: request.priority,
            metadata: HashMap::new(),
        };
        
        conn.execute(
            "INSERT INTO projects (id, name, description, status, owner_id, created_at, updated_at, due_date, priority, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id.to_string(),
                request.name,
                request.description,
                project.status.as_str(),
                owner_id,
                datetime_to_string(now),
                datetime_to_string(now),
                request.due_date.map(datetime_to_string),
                project.priority.as_str(),
                serde_json::to_string(&project.metadata).unwrap_or_default()
            ],
        )?;
        
        Ok(project)
    }
    
    pub async fn get_by_id(&self, id: Uuid) -> SqlResult<Option<Project>> {
        let conn = self.pool.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, name, description, status, owner_id, created_at, updated_at, due_date, priority, metadata
             FROM projects WHERE id = ?1"
        )?;
        
        let project = stmt.query_row(params![id.to_string()], |row| {
            self.row_to_project(row)
        }).optional()?;
        
        Ok(project)
    }
    
    pub async fn list_by_owner(&self, owner_id: &str, limit: Option<usize>, offset: Option<usize>) -> SqlResult<Vec<Project>> {
        let conn = self.pool.lock().await;
        let query = match (limit, offset) {
            (Some(l), Some(o)) => format!(
                "SELECT id, name, description, status, owner_id, created_at, updated_at, due_date, priority, metadata
                 FROM projects WHERE owner_id = ?1 ORDER BY created_at DESC LIMIT {} OFFSET {}", l, o
            ),
            (Some(l), None) => format!(
                "SELECT id, name, description, status, owner_id, created_at, updated_at, due_date, priority, metadata
                 FROM projects WHERE owner_id = ?1 ORDER BY created_at DESC LIMIT {}", l
            ),
            _ => "SELECT id, name, description, status, owner_id, created_at, updated_at, due_date, priority, metadata
                  FROM projects WHERE owner_id = ?1 ORDER BY created_at DESC".to_string(),
        };
        
        let mut stmt = conn.prepare(&query)?;
        let project_iter = stmt.query_map(params![owner_id], |row| {
            self.row_to_project(row)
        })?;
        
        let mut projects = Vec::new();
        for project in project_iter {
            projects.push(project?);
        }
        
        Ok(projects)
    }
    
    pub async fn list_by_member(&self, user_id: &str, limit: Option<usize>, offset: Option<usize>) -> SqlResult<Vec<Project>> {
        let conn = self.pool.lock().await;
        let query = match (limit, offset) {
            (Some(l), Some(o)) => format!(
                "SELECT p.id, p.name, p.description, p.status, p.owner_id, p.created_at, p.updated_at, p.due_date, p.priority, p.metadata
                 FROM projects p
                 LEFT JOIN project_members pm ON p.id = pm.project_id
                 WHERE p.owner_id = ?1 OR pm.user_id = ?1
                 GROUP BY p.id
                 ORDER BY p.created_at DESC LIMIT {} OFFSET {}", l, o
            ),
            (Some(l), None) => format!(
                "SELECT p.id, p.name, p.description, p.status, p.owner_id, p.created_at, p.updated_at, p.due_date, p.priority, p.metadata
                 FROM projects p
                 LEFT JOIN project_members pm ON p.id = pm.project_id
                 WHERE p.owner_id = ?1 OR pm.user_id = ?1
                 GROUP BY p.id
                 ORDER BY p.created_at DESC LIMIT {}", l
            ),
            _ => "SELECT p.id, p.name, p.description, p.status, p.owner_id, p.created_at, p.updated_at, p.due_date, p.priority, p.metadata
                  FROM projects p
                  LEFT JOIN project_members pm ON p.id = pm.project_id
                  WHERE p.owner_id = ?1 OR pm.user_id = ?1
                  GROUP BY p.id
                  ORDER BY p.created_at DESC".to_string(),
        };
        
        let mut stmt = conn.prepare(&query)?;
        let project_iter = stmt.query_map(params![user_id], |row| {
            self.row_to_project(row)
        })?;
        
        let mut projects = Vec::new();
        for project in project_iter {
            projects.push(project?);
        }
        
        Ok(projects)
    }
    
    pub async fn update(&self, id: Uuid, request: UpdateProjectRequest) -> SqlResult<Option<Project>> {
        let conn = self.pool.lock().await;
        let now = Utc::now();
        
        // Build dynamic update query using individual SQL statements for simplicity
        if let Some(name) = &request.name {
            conn.execute(
                "UPDATE projects SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![name, datetime_to_string(now), id.to_string()],
            )?;
        }
        
        if let Some(description) = &request.description {
            conn.execute(
                "UPDATE projects SET description = ?1, updated_at = ?2 WHERE id = ?3",
                params![description, datetime_to_string(now), id.to_string()],
            )?;
        }
        
        if let Some(status) = &request.status {
            conn.execute(
                "UPDATE projects SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params![status.as_str(), datetime_to_string(now), id.to_string()],
            )?;
        }
        
        if let Some(priority) = &request.priority {
            conn.execute(
                "UPDATE projects SET priority = ?1, updated_at = ?2 WHERE id = ?3",
                params![priority.as_str(), datetime_to_string(now), id.to_string()],
            )?;
        }
        
        if let Some(due_date) = &request.due_date {
            conn.execute(
                "UPDATE projects SET due_date = ?1, updated_at = ?2 WHERE id = ?3",
                params![datetime_to_string(*due_date), datetime_to_string(now), id.to_string()],
            )?;
        }
        
        // Return the updated project
        self.get_by_id(id).await
    }
    
    pub async fn delete(&self, id: Uuid) -> SqlResult<bool> {
        let conn = self.pool.lock().await;
        let rows_affected = conn.execute(
            "DELETE FROM projects WHERE id = ?1",
            params![id.to_string()],
        )?;
        
        Ok(rows_affected > 0)
    }
    
    pub async fn get_summary(&self, id: Uuid) -> SqlResult<Option<ProjectSummary>> {
        let conn = self.pool.lock().await;
        let mut stmt = conn.prepare(
            "SELECT 
                p.id, p.name, p.status, p.priority, p.owner_id, p.created_at, p.due_date,
                COUNT(DISTINCT pm.user_id) as member_count,
                COUNT(DISTINCT d.id) as document_count,
                COUNT(DISTINCT kc.id) as kanban_card_count
             FROM projects p
             LEFT JOIN project_members pm ON p.id = pm.project_id
             LEFT JOIN documents d ON p.id = d.project_id
             LEFT JOIN kanban_cards kc ON p.id = kc.project_id
             WHERE p.id = ?1
             GROUP BY p.id"
        )?;
        
        let summary = stmt.query_row(params![id.to_string()], |row| {
            Ok(ProjectSummary {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                name: row.get(1)?,
                status: ProjectStatus::from_str(&row.get::<_, String>(2)?),
                priority: Priority::from_str(&row.get::<_, String>(3)?),
                owner_id: row.get(4)?,
                member_count: row.get::<_, i64>(6)? as usize,
                document_count: row.get::<_, i64>(7)? as usize,
                kanban_card_count: row.get::<_, i64>(8)? as usize,
                created_at: string_to_datetime(&row.get::<_, String>(5)?).unwrap(),
                due_date: row.get::<_, Option<String>>(9)?
                    .and_then(|s| string_to_datetime(&s).ok()),
            })
        }).optional()?;
        
        Ok(summary)
    }
    
    fn row_to_project(&self, row: &Row) -> SqlResult<Project> {
        let metadata_str: String = row.get(9)?;
        let metadata: HashMap<String, String> = serde_json::from_str(&metadata_str)
            .unwrap_or_default();
        
        Ok(Project {
            id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
            name: row.get(1)?,
            description: row.get(2)?,
            status: ProjectStatus::from_str(&row.get::<_, String>(3)?),
            owner_id: row.get(4)?,
            created_at: string_to_datetime(&row.get::<_, String>(5)?).unwrap(),
            updated_at: string_to_datetime(&row.get::<_, String>(6)?).unwrap(),
            due_date: row.get::<_, Option<String>>(7)?
                .and_then(|s| string_to_datetime(&s).ok()),
            priority: Priority::from_str(&row.get::<_, String>(8)?),
            metadata,
        })
    }
}