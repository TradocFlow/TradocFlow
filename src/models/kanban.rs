use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use super::Priority;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanCard {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: CardStatus,
    pub priority: Priority,
    pub assigned_to: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub due_date: Option<DateTime<Utc>>,
    pub position: i32, // For ordering within a column
    pub document_id: Option<Uuid>, // Link to a document if applicable
    pub metadata: HashMap<String, String>, // For additional card-specific data
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CardStatus {
    #[serde(rename = "todo")]
    Todo,
    #[serde(rename = "in_progress")]
    InProgress,
    #[serde(rename = "review")]
    Review,
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "blocked")]
    Blocked,
    #[serde(rename = "cancelled")]
    Cancelled,
}

impl CardStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            CardStatus::Todo => "todo",
            CardStatus::InProgress => "in_progress",
            CardStatus::Review => "review",
            CardStatus::Done => "done",
            CardStatus::Blocked => "blocked",
            CardStatus::Cancelled => "cancelled",
        }
    }
    
    pub fn from_str(s: &str) -> Self {
        match s {
            "todo" => CardStatus::Todo,
            "in_progress" => CardStatus::InProgress,
            "review" => CardStatus::Review,
            "done" => CardStatus::Done,
            "blocked" => CardStatus::Blocked,
            "cancelled" => CardStatus::Cancelled,
            _ => CardStatus::Todo, // Default fallback
        }
    }
    
    pub fn all() -> Vec<CardStatus> {
        vec![
            CardStatus::Todo,
            CardStatus::InProgress,
            CardStatus::Review,
            CardStatus::Done,
            CardStatus::Blocked,
            CardStatus::Cancelled,
        ]
    }
    
    pub fn default_columns() -> Vec<CardStatus> {
        vec![
            CardStatus::Todo,
            CardStatus::InProgress,
            CardStatus::Review,
            CardStatus::Done,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanBoard {
    pub project_id: Uuid,
    pub columns: HashMap<CardStatus, Vec<KanbanCard>>,
    pub project_name: String,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateKanbanCardRequest {
    pub title: String,
    pub description: Option<String>,
    pub priority: Priority,
    pub assigned_to: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub document_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateKanbanCardRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub assigned_to: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub document_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveCardRequest {
    pub card_id: Uuid,
    pub new_status: CardStatus,
    pub new_position: Option<i32>, // Optional position within the new column
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanColumn {
    pub status: CardStatus,
    pub title: String,
    pub cards: Vec<KanbanCard>,
    pub card_count: usize,
}

impl KanbanColumn {
    pub fn new(status: CardStatus, cards: Vec<KanbanCard>) -> Self {
        let title = match status {
            CardStatus::Todo => "To Do".to_string(),
            CardStatus::InProgress => "In Progress".to_string(),
            CardStatus::Review => "Review".to_string(),
            CardStatus::Done => "Done".to_string(),
            CardStatus::Blocked => "Blocked".to_string(),
            CardStatus::Cancelled => "Cancelled".to_string(),
        };
        
        let card_count = cards.len();
        
        Self {
            status,
            title,
            cards,
            card_count,
        }
    }
}