use rusqlite::{Connection, Result as SqlResult};
use crate::database::DatabasePool;

pub fn run_all_migrations(pool: &DatabasePool) -> SqlResult<()> {
    let conn = pool.try_lock().map_err(|_| rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
        Some("Database is busy".to_string())
    ))?;
    
    // Create migration tracking table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS migrations (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            applied_at TEXT NOT NULL
        )",
        [],
    )?;
    
    // Run each migration if not already applied
    run_migration(&conn, "001_create_projects", create_projects_table)?;
    run_migration(&conn, "002_create_project_members", create_project_members_table)?;
    run_migration(&conn, "003_create_kanban_cards", create_kanban_cards_table)?;
    run_migration(&conn, "004_create_translation_progress", create_translation_progress_table)?;
    run_migration(&conn, "005_create_documents", create_documents_table)?;
    run_migration(&conn, "006_create_users", create_users_table)?;
    run_migration(&conn, "007_create_translation_projects", create_translation_projects_table)?;
    run_migration(&conn, "008_create_chapters", create_chapters_table)?;
    run_migration(&conn, "009_create_team_members", create_team_members_table)?;
    run_migration(&conn, "010_create_terminology", create_terminology_table)?;
    run_migration(&conn, "011_enhance_users_table", enhance_users_table)?;
    run_migration(&conn, "012_create_team_invitations", create_team_invitations_table)?;
    run_migration(&conn, "013_create_user_permissions", create_user_permissions_table)?;
    
    Ok(())
}

fn run_migration<F>(conn: &Connection, name: &str, migration_fn: F) -> SqlResult<()>
where
    F: FnOnce(&Connection) -> SqlResult<()>,
{
    // Check if migration already applied
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM migrations WHERE name = ?1")?;
    let count: i64 = stmt.query_row([name], |row| row.get(0))?;
    
    if count == 0 {
        // Run the migration
        migration_fn(conn)?;
        
        // Mark as applied
        conn.execute(
            "INSERT INTO migrations (name, applied_at) VALUES (?1, datetime('now'))",
            [name],
        )?;
        
        println!("Applied migration: {name}");
    }
    
    Ok(())
}

fn create_projects_table(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            owner_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            due_date TEXT,
            priority TEXT NOT NULL DEFAULT 'medium',
            metadata TEXT -- JSON metadata
        )",
        [],
    )?;
    Ok(())
}

fn create_project_members_table(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE project_members (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'member',
            added_at TEXT NOT NULL,
            added_by TEXT NOT NULL,
            FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE,
            UNIQUE(project_id, user_id)
        )",
        [],
    )?;
    Ok(())
}

fn create_kanban_cards_table(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE kanban_cards (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT,
            status TEXT NOT NULL DEFAULT 'todo',
            priority TEXT NOT NULL DEFAULT 'medium',
            assigned_to TEXT,
            created_by TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            due_date TEXT,
            position INTEGER NOT NULL DEFAULT 0,
            document_id TEXT, -- Link to document if applicable
            metadata TEXT, -- JSON metadata for additional data
            FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
        )",
        [],
    )?;
    Ok(())
}

fn create_translation_progress_table(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE translation_progress (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            document_id TEXT,
            source_language TEXT NOT NULL,
            target_language TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'not_started',
            assigned_translator TEXT,
            progress_percentage INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            due_date TEXT,
            completed_at TEXT,
            quality_score INTEGER, -- 0-100 quality score
            notes TEXT,
            FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE,
            UNIQUE(project_id, document_id, source_language, target_language)
        )",
        [],
    )?;
    Ok(())
}

fn create_documents_table(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE documents (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            content TEXT, -- JSON content for multi-language support
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            version INTEGER NOT NULL DEFAULT 1,
            status TEXT NOT NULL DEFAULT 'draft',
            project_id TEXT, -- Link to project
            metadata TEXT, -- JSON metadata
            FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE SET NULL
        )",
        [],
    )?;
    Ok(())
}

fn create_users_table(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE users (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE,
            role TEXT NOT NULL DEFAULT 'viewer',
            created_at TEXT NOT NULL,
            active BOOLEAN NOT NULL DEFAULT 1,
            last_login TEXT,
            preferences TEXT -- JSON preferences
        )",
        [],
    )?;
    Ok(())
}

fn create_translation_projects_table(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE translation_projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            source_language TEXT NOT NULL,
            target_languages TEXT NOT NULL, -- JSON array
            project_path TEXT NOT NULL,
            settings TEXT NOT NULL, -- JSON project settings
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active'
        )",
        [],
    )?;
    Ok(())
}

fn create_chapters_table(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE chapters (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            chapter_number INTEGER NOT NULL,
            slug TEXT NOT NULL,
            title TEXT NOT NULL, -- JSON object with language -> title mapping
            status TEXT NOT NULL DEFAULT 'draft',
            assigned_translators TEXT, -- JSON object with language -> user_id mapping
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (project_id) REFERENCES translation_projects (id) ON DELETE CASCADE,
            UNIQUE(project_id, chapter_number),
            UNIQUE(project_id, slug)
        )",
        [],
    )?;
    Ok(())
}

fn create_team_members_table(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE team_members (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            name TEXT NOT NULL,
            email TEXT NOT NULL,
            role TEXT NOT NULL, -- ProjectManager, Translator, Reviewer, Admin
            languages TEXT NOT NULL, -- JSON array of language codes
            permissions TEXT NOT NULL, -- JSON array of permissions
            joined_at TEXT NOT NULL,
            FOREIGN KEY (project_id) REFERENCES translation_projects (id) ON DELETE CASCADE,
            UNIQUE(project_id, user_id)
        )",
        [],
    )?;
    Ok(())
}

fn create_terminology_table(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE terminology (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            term TEXT NOT NULL,
            definition TEXT,
            do_not_translate BOOLEAN DEFAULT FALSE,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (project_id) REFERENCES translation_projects (id) ON DELETE CASCADE,
            UNIQUE(project_id, term)
        )",
        [],
    )?;
    
    // Create indexes for efficient terminology lookups
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_terminology_project_id ON terminology(project_id)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_terminology_term ON terminology(term)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_terminology_do_not_translate 
         ON terminology(project_id, do_not_translate) WHERE do_not_translate = TRUE",
        [],
    )?;
    
    Ok(())
}
fn enhance_users_table(conn: &Connection) -> SqlResult<()> {
    // Add new columns to users table for enhanced user management
    conn.execute(
        "ALTER TABLE users ADD COLUMN updated_at TEXT",
        [],
    ).ok(); // Ignore error if column already exists
    
    conn.execute(
        "ALTER TABLE users ADD COLUMN languages TEXT DEFAULT '[]'",
        [],
    ).ok();
    
    conn.execute(
        "ALTER TABLE users ADD COLUMN specializations TEXT DEFAULT '[]'",
        [],
    ).ok();
    
    conn.execute(
        "ALTER TABLE users ADD COLUMN timezone TEXT",
        [],
    ).ok();
    
    // Update existing users with default values
    conn.execute(
        "UPDATE users SET updated_at = created_at WHERE updated_at IS NULL",
        [],
    )?;
    
    Ok(())
}

fn create_team_invitations_table(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE team_invitations (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            inviter_id TEXT NOT NULL,
            invitee_email TEXT NOT NULL,
            role TEXT NOT NULL,
            message TEXT,
            created_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            FOREIGN KEY (project_id) REFERENCES translation_projects (id) ON DELETE CASCADE,
            FOREIGN KEY (inviter_id) REFERENCES users (id) ON DELETE CASCADE
        )",
        [],
    )?;
    
    // Create indexes for efficient lookups
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_team_invitations_project_id ON team_invitations(project_id)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_team_invitations_invitee_email ON team_invitations(invitee_email)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_team_invitations_status ON team_invitations(status)",
        [],
    )?;
    
    Ok(())
}

fn create_user_permissions_table(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE user_permissions (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            project_id TEXT NOT NULL,
            permission TEXT NOT NULL,
            granted_by TEXT NOT NULL,
            granted_at TEXT NOT NULL,
            expires_at TEXT,
            FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
            FOREIGN KEY (project_id) REFERENCES translation_projects (id) ON DELETE CASCADE,
            FOREIGN KEY (granted_by) REFERENCES users (id) ON DELETE CASCADE,
            UNIQUE(user_id, project_id, permission)
        )",
        [],
    )?;
    
    // Create indexes for efficient permission lookups
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_user_permissions_user_project 
         ON user_permissions(user_id, project_id)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_user_permissions_permission 
         ON user_permissions(permission)",
        [],
    )?;
    
    Ok(())
}