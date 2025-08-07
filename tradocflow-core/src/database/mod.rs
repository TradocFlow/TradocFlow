use rusqlite::{Connection, Result as SqlResult};
use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use std::sync::Arc;

pub mod migrations;
pub mod project_repository;
pub mod kanban_repository;
pub mod member_repository;
pub mod translation_progress_repository;

pub type DatabasePool = Arc<Mutex<Connection>>;

pub struct Database {
    pool: DatabasePool,
}

impl Database {
    pub fn new(database_path: &str) -> SqlResult<Self> {
        let conn = Connection::open(database_path)?;
        
        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])?;
        
        let pool = Arc::new(Mutex::new(conn));
        let db = Database { pool };
        
        // Run migrations
        db.run_migrations()?;
        
        Ok(db)
    }
    
    pub fn in_memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute("PRAGMA foreign_keys = ON", [])?;
        
        let pool = Arc::new(Mutex::new(conn));
        let db = Database { pool };
        
        // Run migrations
        db.run_migrations()?;
        
        Ok(db)
    }
    
    pub fn pool(&self) -> DatabasePool {
        self.pool.clone()
    }
    
    fn run_migrations(&self) -> SqlResult<()> {
        migrations::run_all_migrations(&self.pool)
    }
}

// Helper function to convert DateTime<Utc> to string
pub fn datetime_to_string(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

// Helper function to convert string to DateTime<Utc>
pub fn string_to_datetime(s: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc))
}