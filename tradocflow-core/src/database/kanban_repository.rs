use rusqlite::{params, Row, Result as SqlResult};
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;
use crate::database::{DatabasePool, datetime_to_string, string_to_datetime};
use rusqlite::OptionalExtension;
use crate::models::{KanbanCard, CardStatus, Priority, CreateKanbanCardRequest, UpdateKanbanCardRequest, MoveCardRequest, KanbanBoard};

#[derive(Debug)]
pub struct KanbanRepository {
    pool: DatabasePool,
}

impl KanbanRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
    
    pub async fn create_card(
        &self, 
        project_id: &Uuid, 
        request: &CreateKanbanCardRequest, 
        created_by: &str,
        metadata: HashMap<String, String>
    ) -> SqlResult<KanbanCard> {
        let conn = self.pool.lock().await;
        let id = Uuid::new_v4();
        let now = Utc::now();
        
        // Get next position for the default status (Todo)
        let position = self.get_next_position(&conn, *project_id, CardStatus::Todo)?;
        
        let card = KanbanCard {
            id,
            project_id: *project_id,
            title: request.title.clone(),
            description: request.description.clone(),
            status: CardStatus::Todo,
            priority: request.priority.clone(),
            assigned_to: request.assigned_to.clone(),
            created_by: created_by.to_string(),
            created_at: now,
            updated_at: now,
            due_date: request.due_date,
            position,
            document_id: request.document_id,
            metadata,
        };
        
        conn.execute(
            "INSERT INTO kanban_cards (id, project_id, title, description, status, priority, assigned_to, created_by, created_at, updated_at, due_date, position, document_id, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                id.to_string(),
                project_id.to_string(),
                request.title,
                request.description,
                card.status.as_str(),
                card.priority.as_str(),
                request.assigned_to,
                created_by,
                datetime_to_string(now),
                datetime_to_string(now),
                request.due_date.map(datetime_to_string),
                position,
                request.document_id.map(|id| id.to_string()),
                serde_json::to_string(&card.metadata).unwrap_or_default()
            ],
        )?;
        
        Ok(card)
    }
    
    pub async fn get_board(&self, project_id: Uuid) -> SqlResult<KanbanBoard> {
        let conn = self.pool.lock().await;
        
        // Get project name
        let mut project_stmt = conn.prepare("SELECT name FROM projects WHERE id = ?1")?;
        let project_name: String = project_stmt.query_row(params![project_id.to_string()], |row| {
            row.get(0)
        })?;
        
        // Get all cards for the project
        let mut stmt = conn.prepare(
            "SELECT id, project_id, title, description, status, priority, assigned_to, created_by, created_at, updated_at, due_date, position, document_id, metadata
             FROM kanban_cards WHERE project_id = ?1 ORDER BY status, position"
        )?;
        
        let card_iter = stmt.query_map(params![project_id.to_string()], |row| {
            self.row_to_card(row)
        })?;
        
        let mut cards = Vec::new();
        for card in card_iter {
            cards.push(card?);
        }
        
        // Group cards by status
        let mut columns = HashMap::new();
        for status in CardStatus::default_columns() {
            let status_cards: Vec<KanbanCard> = cards.iter()
                .filter(|card| card.status == status)
                .cloned()
                .collect();
            columns.insert(status, status_cards);
        }
        
        Ok(KanbanBoard {
            project_id,
            columns,
            project_name,
            last_updated: Utc::now(),
        })
    }
    
    pub async fn update_card(&self, card_id: Uuid, request: UpdateKanbanCardRequest) -> SqlResult<Option<KanbanCard>> {
        let conn = self.pool.lock().await;
        let now = Utc::now();
        
        // Build dynamic update query using individual SQL statements for simplicity
        if let Some(title) = &request.title {
            conn.execute(
                "UPDATE kanban_cards SET title = ?1, updated_at = ?2 WHERE id = ?3",
                params![title, datetime_to_string(now), card_id.to_string()],
            )?;
        }
        
        if let Some(description) = &request.description {
            conn.execute(
                "UPDATE kanban_cards SET description = ?1, updated_at = ?2 WHERE id = ?3",
                params![description, datetime_to_string(now), card_id.to_string()],
            )?;
        }
        
        if let Some(priority) = &request.priority {
            conn.execute(
                "UPDATE kanban_cards SET priority = ?1, updated_at = ?2 WHERE id = ?3",
                params![priority.as_str(), datetime_to_string(now), card_id.to_string()],
            )?;
        }
        
        if let Some(assigned_to) = &request.assigned_to {
            conn.execute(
                "UPDATE kanban_cards SET assigned_to = ?1, updated_at = ?2 WHERE id = ?3",
                params![assigned_to, datetime_to_string(now), card_id.to_string()],
            )?;
        }
        
        if let Some(due_date) = &request.due_date {
            conn.execute(
                "UPDATE kanban_cards SET due_date = ?1, updated_at = ?2 WHERE id = ?3",
                params![datetime_to_string(*due_date), datetime_to_string(now), card_id.to_string()],
            )?;
        }
        
        if let Some(document_id) = &request.document_id {
            conn.execute(
                "UPDATE kanban_cards SET document_id = ?1, updated_at = ?2 WHERE id = ?3",
                params![document_id.to_string(), datetime_to_string(now), card_id.to_string()],
            )?;
        }
        
        self.get_card_by_id(card_id).await
    }
    
    pub async fn move_card(&self, request: MoveCardRequest) -> SqlResult<Option<KanbanCard>> {
        let conn = self.pool.lock().await;
        let now = Utc::now();
        
        // Get current card to get project_id
        let current_card = self.get_card_by_id(request.card_id).await?;
        if current_card.is_none() {
            return Ok(None);
        }
        let card = current_card.unwrap();
        
        let new_position = match request.new_position {
            Some(pos) => pos,
            None => self.get_next_position(&conn, card.project_id, request.new_status.clone())?,
        };
        
        conn.execute(
            "UPDATE kanban_cards SET status = ?1, position = ?2, updated_at = ?3 WHERE id = ?4",
            params![
                request.new_status.as_str(),
                new_position,
                datetime_to_string(now),
                request.card_id.to_string()
            ],
        )?;
        
        self.get_card_by_id(request.card_id).await
    }
    
    pub async fn delete_card(&self, card_id: Uuid) -> SqlResult<bool> {
        let conn = self.pool.lock().await;
        let rows_affected = conn.execute(
            "DELETE FROM kanban_cards WHERE id = ?1",
            params![card_id.to_string()],
        )?;
        
        Ok(rows_affected > 0)
    }
    
    async fn get_card_by_id(&self, card_id: Uuid) -> SqlResult<Option<KanbanCard>> {
        let conn = self.pool.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, title, description, status, priority, assigned_to, created_by, created_at, updated_at, due_date, position, document_id, metadata
             FROM kanban_cards WHERE id = ?1"
        )?;
        
        let card = stmt.query_row(params![card_id.to_string()], |row| {
            self.row_to_card(row)
        }).optional()?;
        
        Ok(card)
    }
    
    fn get_next_position(&self, conn: &rusqlite::Connection, project_id: Uuid, status: CardStatus) -> SqlResult<i32> {
        let mut stmt = conn.prepare(
            "SELECT COALESCE(MAX(position), 0) + 1 FROM kanban_cards WHERE project_id = ?1 AND status = ?2"
        )?;
        
        let position: i32 = stmt.query_row(params![project_id.to_string(), status.as_str()], |row| {
            row.get(0)
        })?;
        
        Ok(position)
    }
    
    fn row_to_card(&self, row: &Row) -> SqlResult<KanbanCard> {
        let metadata_str: String = row.get(13)?;
        let metadata: HashMap<String, String> = serde_json::from_str(&metadata_str)
            .unwrap_or_default();
        
        Ok(KanbanCard {
            id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
            project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
            title: row.get(2)?,
            description: row.get(3)?,
            status: CardStatus::from_str(&row.get::<_, String>(4)?),
            priority: Priority::from_str(&row.get::<_, String>(5)?),
            assigned_to: row.get(6)?,
            created_by: row.get(7)?,
            created_at: string_to_datetime(&row.get::<_, String>(8)?).unwrap(),
            updated_at: string_to_datetime(&row.get::<_, String>(9)?).unwrap(),
            due_date: row.get::<_, Option<String>>(10)?
                .and_then(|s| string_to_datetime(&s).ok()),
            position: row.get(11)?,
            document_id: row.get::<_, Option<String>>(12)?
                .and_then(|s| Uuid::parse_str(&s).ok()),
            metadata,
        })
    }

    /// Get a specific card by ID
    pub async fn get_card(&self, card_id: &Uuid) -> SqlResult<KanbanCard> {
        self.get_card_by_id(*card_id).await?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)
    }

    /// Get all cards for a project
    pub async fn get_project_cards(&self, project_id: &Uuid) -> SqlResult<Vec<KanbanCard>> {
        let conn = self.pool.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, title, description, status, priority, assigned_to, created_by, created_at, updated_at, due_date, position, document_id, metadata
             FROM kanban_cards WHERE project_id = ?1 ORDER BY position"
        )?;
        
        let card_iter = stmt.query_map(params![project_id.to_string()], |row| {
            self.row_to_card(row)
        })?;
        
        let mut cards = Vec::new();
        for card in card_iter {
            cards.push(card?);
        }
        
        Ok(cards)
    }
}