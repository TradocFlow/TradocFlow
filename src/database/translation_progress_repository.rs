use rusqlite::{params, Row, Result as SqlResult};
use uuid::Uuid;
use chrono::Utc;
use crate::database::{DatabasePool, datetime_to_string, string_to_datetime};
use rusqlite::OptionalExtension;
use crate::models::{TranslationProgress, CreateTranslationProgressRequest, UpdateTranslationProgressRequest, TranslationProgressSummary};
use crate::models::translation_progress::{LanguageProgress};
use crate::models::translation_progress::TranslationStatus;

pub struct TranslationProgressRepository {
    pool: DatabasePool,
}

impl TranslationProgressRepository {
    pub fn new(pool: DatabasePool) -> Self {
        Self { pool }
    }
    
    pub async fn create(&self, project_id: Uuid, request: CreateTranslationProgressRequest) -> SqlResult<TranslationProgress> {
        let conn = self.pool.lock().await;
        let id = Uuid::new_v4();
        let now = Utc::now();
        
        let progress = TranslationProgress {
            id,
            project_id,
            document_id: request.document_id,
            source_language: request.source_language.clone(),
            target_language: request.target_language.clone(),
            status: TranslationStatus::NotStarted,
            assigned_translator: request.assigned_translator.clone(),
            progress_percentage: 0,
            created_at: now,
            updated_at: now,
            due_date: request.due_date,
            completed_at: None,
            quality_score: None,
            notes: request.notes.clone(),
        };
        
        conn.execute(
            "INSERT INTO translation_progress (id, project_id, document_id, source_language, target_language, status, assigned_translator, progress_percentage, created_at, updated_at, due_date, completed_at, quality_score, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                id.to_string(),
                project_id.to_string(),
                request.document_id.map(|id| id.to_string()),
                request.source_language,
                request.target_language,
                progress.status.as_str(),
                request.assigned_translator,
                progress.progress_percentage,
                datetime_to_string(now),
                datetime_to_string(now),
                request.due_date.map(datetime_to_string),
                None::<String>,
                None::<u8>,
                request.notes
            ],
        )?;
        
        Ok(progress)
    }
    
    pub async fn get_by_id(&self, id: Uuid) -> SqlResult<Option<TranslationProgress>> {
        let conn = self.pool.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, document_id, source_language, target_language, status, assigned_translator, progress_percentage, created_at, updated_at, due_date, completed_at, quality_score, notes
             FROM translation_progress WHERE id = ?1"
        )?;
        
        let progress = stmt.query_row(params![id.to_string()], |row| {
            self.row_to_progress(row)
        }).optional()?;
        
        Ok(progress)
    }
    
    pub async fn get_by_project(&self, project_id: Uuid) -> SqlResult<Vec<TranslationProgress>> {
        let conn = self.pool.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, document_id, source_language, target_language, status, assigned_translator, progress_percentage, created_at, updated_at, due_date, completed_at, quality_score, notes
             FROM translation_progress WHERE project_id = ?1 ORDER BY created_at DESC"
        )?;
        
        let progress_iter = stmt.query_map(params![project_id.to_string()], |row| {
            self.row_to_progress(row)
        })?;
        
        let mut progress_list = Vec::new();
        for progress in progress_iter {
            progress_list.push(progress?);
        }
        
        Ok(progress_list)
    }
    
    pub async fn get_by_translator(&self, translator_id: &str) -> SqlResult<Vec<TranslationProgress>> {
        let conn = self.pool.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, document_id, source_language, target_language, status, assigned_translator, progress_percentage, created_at, updated_at, due_date, completed_at, quality_score, notes
             FROM translation_progress WHERE assigned_translator = ?1 ORDER BY created_at DESC"
        )?;
        
        let progress_iter = stmt.query_map(params![translator_id], |row| {
            self.row_to_progress(row)
        })?;
        
        let mut progress_list = Vec::new();
        for progress in progress_iter {
            progress_list.push(progress?);
        }
        
        Ok(progress_list)
    }
    
    pub async fn update(&self, id: Uuid, request: UpdateTranslationProgressRequest) -> SqlResult<Option<TranslationProgress>> {
        let conn = self.pool.lock().await;
        let now = Utc::now();
        
        // Build dynamic update query using individual SQL statements for simplicity
        if let Some(status) = &request.status {
            conn.execute(
                "UPDATE translation_progress SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params![status.as_str(), datetime_to_string(now), id.to_string()],
            )?;
            
            // If status is completed, set completed_at
            if status.is_completed() {
                conn.execute(
                    "UPDATE translation_progress SET completed_at = ?1, updated_at = ?2 WHERE id = ?3",
                    params![datetime_to_string(now), datetime_to_string(now), id.to_string()],
                )?;
            }
        }
        
        if let Some(assigned_translator) = &request.assigned_translator {
            conn.execute(
                "UPDATE translation_progress SET assigned_translator = ?1, updated_at = ?2 WHERE id = ?3",
                params![assigned_translator, datetime_to_string(now), id.to_string()],
            )?;
        }
        
        if let Some(progress_percentage) = &request.progress_percentage {
            conn.execute(
                "UPDATE translation_progress SET progress_percentage = ?1, updated_at = ?2 WHERE id = ?3",
                params![*progress_percentage, datetime_to_string(now), id.to_string()],
            )?;
        }
        
        if let Some(due_date) = &request.due_date {
            conn.execute(
                "UPDATE translation_progress SET due_date = ?1, updated_at = ?2 WHERE id = ?3",
                params![datetime_to_string(*due_date), datetime_to_string(now), id.to_string()],
            )?;
        }
        
        if let Some(quality_score) = &request.quality_score {
            conn.execute(
                "UPDATE translation_progress SET quality_score = ?1, updated_at = ?2 WHERE id = ?3",
                params![*quality_score, datetime_to_string(now), id.to_string()],
            )?;
        }
        
        if let Some(notes) = &request.notes {
            conn.execute(
                "UPDATE translation_progress SET notes = ?1, updated_at = ?2 WHERE id = ?3",
                params![notes, datetime_to_string(now), id.to_string()],
            )?;
        }
        
        self.get_by_id(id).await
    }
    
    pub async fn delete(&self, id: Uuid) -> SqlResult<bool> {
        let conn = self.pool.lock().await;
        let rows_affected = conn.execute(
            "DELETE FROM translation_progress WHERE id = ?1",
            params![id.to_string()],
        )?;
        
        Ok(rows_affected > 0)
    }
    
    pub async fn get_project_summary(&self, project_id: Uuid) -> SqlResult<Option<TranslationProgressSummary>> {
        let conn = self.pool.lock().await;
        
        // Get project name
        let mut project_stmt = conn.prepare("SELECT name FROM projects WHERE id = ?1")?;
        let project_name: String = project_stmt.query_row(params![project_id.to_string()], |row| {
            row.get(0)
        })?;
        
        // Get overall statistics
        let mut stats_stmt = conn.prepare(
            "SELECT 
                COUNT(*) as total,
                SUM(CASE WHEN status = 'not_started' THEN 1 ELSE 0 END) as not_started,
                SUM(CASE WHEN status = 'in_progress' THEN 1 ELSE 0 END) as in_progress,
                SUM(CASE WHEN status = 'completed' OR status = 'reviewed' THEN 1 ELSE 0 END) as completed,
                SUM(CASE WHEN status = 'approved' THEN 1 ELSE 0 END) as approved,
                AVG(progress_percentage) as avg_progress
             FROM translation_progress WHERE project_id = ?1"
        )?;
        
        let (total, not_started, in_progress, completed, approved, avg_progress) = stats_stmt.query_row(
            params![project_id.to_string()], 
            |row| {
                Ok((
                    row.get::<_, i64>(0)? as usize,
                    row.get::<_, i64>(1)? as usize,
                    row.get::<_, i64>(2)? as usize,
                    row.get::<_, i64>(3)? as usize,
                    row.get::<_, i64>(4)? as usize,
                    row.get::<_, Option<f64>>(5)?.unwrap_or(0.0) as f32,
                ))
            }
        )?;
        
        // Get language-specific progress
        let mut lang_stmt = conn.prepare(
            "SELECT 
                target_language,
                COUNT(*) as total,
                SUM(CASE WHEN status = 'completed' OR status = 'reviewed' OR status = 'approved' THEN 1 ELSE 0 END) as completed,
                AVG(CASE WHEN quality_score IS NOT NULL THEN quality_score ELSE NULL END) as avg_quality
             FROM translation_progress 
             WHERE project_id = ?1 
             GROUP BY target_language"
        )?;
        
        let lang_iter = lang_stmt.query_map(params![project_id.to_string()], |row| {
            let total = row.get::<_, i64>(1)? as usize;
            let completed = row.get::<_, i64>(2)? as usize;
            Ok(LanguageProgress {
                language: row.get(0)?,
                total_translations: total,
                completed_translations: completed,
                progress_percentage: if total > 0 { (completed as f32 / total as f32) * 100.0 } else { 0.0 },
                average_quality_score: row.get::<_, Option<f64>>(3)?.map(|s| s as f32),
            })
        })?;
        
        let mut languages = Vec::new();
        for lang in lang_iter {
            languages.push(lang?);
        }
        
        Ok(Some(TranslationProgressSummary {
            project_id,
            project_name,
            total_translations: total,
            not_started,
            in_progress,
            completed,
            approved,
            overall_progress_percentage: avg_progress,
            languages,
        }))
    }
    
    fn row_to_progress(&self, row: &Row) -> SqlResult<TranslationProgress> {
        Ok(TranslationProgress {
            id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
            project_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap(),
            document_id: row.get::<_, Option<String>>(2)?
                .and_then(|s| Uuid::parse_str(&s).ok()),
            source_language: row.get(3)?,
            target_language: row.get(4)?,
            status: TranslationStatus::from_str(&row.get::<_, String>(5)?),
            assigned_translator: row.get(6)?,
            progress_percentage: row.get::<_, i64>(7)? as u8,
            created_at: string_to_datetime(&row.get::<_, String>(8)?).unwrap(),
            updated_at: string_to_datetime(&row.get::<_, String>(9)?).unwrap(),
            due_date: row.get::<_, Option<String>>(10)?
                .and_then(|s| string_to_datetime(&s).ok()),
            completed_at: row.get::<_, Option<String>>(11)?
                .and_then(|s| string_to_datetime(&s).ok()),
            quality_score: row.get::<_, Option<i64>>(12)?.map(|s| s as u8),
            notes: row.get(13)?,
        })
    }
}