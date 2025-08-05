use crate::{
    export_engine::{ExportConfig, ExportEngine, ExportFormat},
    models::{document::Document, project::Project},
    Result, TradocumentError, DocumentMetadata, ScreenshotReference,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use uuid::Uuid;

// Document types for export engine use crate::Document

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    pub id: Uuid,
    pub project_id: Uuid,
    pub document_id: Option<Uuid>, // None for full project export
    pub config: ExportConfiguration,
    pub output_directory: PathBuf,
    pub filename_prefix: String,
    pub requested_at: DateTime<Utc>,
    pub requested_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfiguration {
    pub format: ExportFormat,
    pub layout: ExportLayout,
    pub languages: Vec<String>,
    pub include_metadata: bool,
    pub include_table_of_contents: bool,
    pub custom_css_path: Option<PathBuf>,
    pub template_options: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportLayout {
    SingleLanguage,
    SideBySide,
    Sequential,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportJob {
    pub request: ExportRequest,
    pub status: ExportStatus,
    pub progress: ExportProgress,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub output_files: Vec<ExportedFile>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportStatus {
    Queued,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for ExportStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportStatus::Queued => write!(f, "Queued"),
            ExportStatus::InProgress => write!(f, "In Progress"),
            ExportStatus::Completed => write!(f, "Completed"),
            ExportStatus::Failed => write!(f, "Failed"),
            ExportStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportProgress {
    pub current_step: String,
    pub progress: f32, // 0.0 to 1.0
    pub total_files: usize,
    pub completed_files: usize,
    pub estimated_remaining_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedFile {
    pub filename: String,
    pub file_path: PathBuf,
    pub language: String,
    pub format: String,
    pub file_size: u64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportHistory {
    pub exports: Vec<ExportJob>,
    pub total_exports: usize,
    pub successful_exports: usize,
    pub failed_exports: usize,
}

pub struct ExportService {
    export_engine: Arc<ExportEngine>,
    job_queue: Arc<Mutex<Vec<ExportJob>>>,
    active_jobs: Arc<Mutex<HashMap<Uuid, JoinHandle<()>>>>,
    progress_sender: Arc<Mutex<Option<mpsc::UnboundedSender<ExportProgress>>>>,
    max_concurrent_jobs: usize,
}

impl ExportService {
    pub fn new() -> Self {
        Self {
            export_engine: Arc::new(ExportEngine::new()),
            job_queue: Arc::new(Mutex::new(Vec::new())),
            active_jobs: Arc::new(Mutex::new(HashMap::new())),
            progress_sender: Arc::new(Mutex::new(None)),
            max_concurrent_jobs: 3, // Configurable limit
        }
    }

    pub async fn queue_export(&self, request: ExportRequest) -> Result<Uuid> {
        let job = ExportJob {
            request: request.clone(),
            status: ExportStatus::Queued,
            progress: ExportProgress {
                current_step: "Queued for processing".to_string(),
                progress: 0.0,
                total_files: 0,
                completed_files: 0,
                estimated_remaining_seconds: None,
            },
            started_at: None,
            completed_at: None,
            output_files: Vec::new(),
            error_message: None,
        };

        {
            let mut queue = self.job_queue.lock().unwrap();
            queue.push(job);
        }

        // Try to start processing if we have capacity
        self.process_queue().await?;

        Ok(request.id)
    }

    pub async fn cancel_export(&self, export_id: Uuid) -> Result<()> {
        // Remove from queue if not started
        {
            let mut queue = self.job_queue.lock().unwrap();
            queue.retain(|job| job.request.id != export_id);
        }

        // Cancel active job if running
        {
            let mut active_jobs = self.active_jobs.lock().unwrap();
            if let Some(handle) = active_jobs.remove(&export_id) {
                handle.abort();
            }
        }

        Ok(())
    }

    pub fn get_export_status(&self, export_id: Uuid) -> Option<ExportJob> {
        let queue = self.job_queue.lock().unwrap();
        queue.iter().find(|job| job.request.id == export_id).cloned()
    }

    pub fn get_queue_status(&self) -> (usize, usize) {
        let queue = self.job_queue.lock().unwrap();
        let active_jobs = self.active_jobs.lock().unwrap();
        (queue.len(), active_jobs.len())
    }

    pub fn get_export_history(&self, limit: Option<usize>) -> ExportHistory {
        let queue = self.job_queue.lock().unwrap();
        let completed_jobs: Vec<ExportJob> = queue
            .iter()
            .filter(|job| matches!(job.status, ExportStatus::Completed | ExportStatus::Failed))
            .cloned()
            .collect();

        let total_exports = completed_jobs.len();
        let successful_exports = completed_jobs
            .iter()
            .filter(|job| matches!(job.status, ExportStatus::Completed))
            .count();
        let failed_exports = total_exports - successful_exports;

        let exports = if let Some(limit) = limit {
            completed_jobs.into_iter().take(limit).collect()
        } else {
            completed_jobs
        };

        ExportHistory {
            exports,
            total_exports,
            successful_exports,
            failed_exports,
        }
    }

    pub fn subscribe_to_progress(&self) -> mpsc::UnboundedReceiver<ExportProgress> {
        let (sender, receiver) = mpsc::unbounded_channel();
        *self.progress_sender.lock().unwrap() = Some(sender);
        receiver
    }

    async fn process_queue(&self) -> Result<()> {
        let active_count = self.active_jobs.lock().unwrap().len();
        if active_count >= self.max_concurrent_jobs {
            return Ok(());
        }

        let next_job = {
            let mut queue = self.job_queue.lock().unwrap();
            queue
                .iter_mut()
                .find(|job| matches!(job.status, ExportStatus::Queued))
                .map(|job| {
                    job.status = ExportStatus::InProgress;
                    job.started_at = Some(Utc::now());
                    job.clone()
                })
        };

        if let Some(job) = next_job {
            let export_engine = Arc::clone(&self.export_engine);
            let job_queue = Arc::clone(&self.job_queue);
            let active_jobs = Arc::clone(&self.active_jobs);
            let progress_sender = Arc::clone(&self.progress_sender);

            let handle = tokio::spawn(async move {
                let result = Self::execute_export_job(
                    Arc::clone(&export_engine),
                    job.clone(),
                    Arc::clone(&progress_sender),
                ).await;

                // Update job status
                match result {
                    Ok(updated_job) => {
                        let mut queue = job_queue.lock().unwrap();
                        if let Some(queue_job) = queue.iter_mut().find(|j| j.request.id == updated_job.request.id) {
                            *queue_job = updated_job;
                        }
                    }
                    Err(e) => {
                        let mut queue = job_queue.lock().unwrap();
                        if let Some(queue_job) = queue.iter_mut().find(|j| j.request.id == job.request.id) {
                            queue_job.status = ExportStatus::Failed;
                            queue_job.error_message = Some(e.to_string());
                            queue_job.completed_at = Some(Utc::now());
                        }
                    }
                }

                // Remove from active jobs
                {
                    let mut active = active_jobs.lock().unwrap();
                    active.remove(&job.request.id);
                }
            });

            let job_id = job.request.id;
            self.active_jobs.lock().unwrap().insert(job_id, handle);
        }

        Ok(())
    }

    async fn execute_export_job(
        export_engine: Arc<ExportEngine>,
        mut job: ExportJob,
        progress_sender: Arc<Mutex<Option<mpsc::UnboundedSender<ExportProgress>>>>,
    ) -> Result<ExportJob> {
        let send_progress = {
            let progress_sender = Arc::clone(&progress_sender);
            move |progress: ExportProgress| {
                if let Some(sender) = progress_sender.lock().unwrap().as_ref() {
                    let _ = sender.send(progress);
                }
            }
        };

        // Step 1: Validate configuration
        job.progress.current_step = "Validating export configuration".to_string();
        job.progress.progress = 0.1;
        send_progress(job.progress.clone());

        Self::validate_export_config(&job.request.config)?;

        // Step 2: Load document(s)
        job.progress.current_step = "Loading documents".to_string();
        job.progress.progress = 0.2;
        send_progress(job.progress.clone());

        let documents = Self::load_documents_for_export(&job.request).await?;
        job.progress.total_files = documents.len() * job.request.config.languages.len();

        // Step 3: Process each document and language combination
        let mut output_files = Vec::new();
        let total_combinations = documents.len() * job.request.config.languages.len();

        for (doc_index, document) in documents.iter().enumerate() {
            for (lang_index, language) in job.request.config.languages.iter().enumerate() {
                let combination_index = doc_index * job.request.config.languages.len() + lang_index;
                
                job.progress.current_step = format!(
                    "Exporting {} ({})",
                    document.title,
                    language
                );
                job.progress.progress = 0.3 + (0.6 * combination_index as f32 / total_combinations as f32);
                job.progress.completed_files = combination_index;
                send_progress(job.progress.clone());

                let export_config = Self::create_export_config(&job.request.config, vec![language.clone()]);
                let lib_document = Self::convert_to_lib_document(document, language);
                let exported_data = export_engine.export_document(&lib_document, &export_config).await?;

                for (filename, data) in exported_data {
                    let output_path = job.request.output_directory.join(&filename);
                    let file_size = data.len() as u64;
                    tokio::fs::write(&output_path, data).await?;

                    output_files.push(ExportedFile {
                        filename: filename.clone(),
                        file_path: output_path,
                        language: language.clone(),
                        format: Self::get_format_extension(&job.request.config.format),
                        file_size,
                        created_at: Utc::now(),
                    });
                }
            }
        }

        // Step 4: Finalize
        job.progress.current_step = "Export completed successfully".to_string();
        job.progress.progress = 1.0;
        job.progress.completed_files = job.progress.total_files;
        job.status = ExportStatus::Completed;
        job.completed_at = Some(Utc::now());
        job.output_files = output_files;
        send_progress(job.progress.clone());

        Ok(job)
    }

    pub fn validate_export_config(config: &ExportConfiguration) -> Result<()> {
        if config.languages.is_empty() {
            return Err(TradocumentError::Validation(
                "At least one language must be selected for export".to_string(),
            ));
        }

        if let Some(css_path) = &config.custom_css_path {
            if !css_path.exists() {
                return Err(TradocumentError::Validation(
                    format!("Custom CSS file not found: {}", css_path.display()),
                ));
            }
        }

        Ok(())
    }

    async fn load_documents_for_export(request: &ExportRequest) -> Result<Vec<crate::models::document::Document>> {
        // This would typically load from the database
        // For now, return a placeholder implementation
        // TODO: Integrate with actual document repository
        Ok(vec![])
    }

    fn create_export_config(config: &ExportConfiguration, languages: Vec<String>) -> ExportConfig {
        ExportConfig {
            format: config.format.clone(),
            include_screenshots: true, // Could be configurable
            template: None,
            css_file: config.custom_css_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            languages,
        }
    }

    pub fn get_format_extension(format: &ExportFormat) -> String {
        match format {
            ExportFormat::Html => "html".to_string(),
            ExportFormat::Pdf => "pdf".to_string(),
            ExportFormat::Both => "mixed".to_string(),
        }
    }

    fn convert_to_lib_document(document: &crate::models::document::Document, language: &str) -> crate::Document {
        use std::collections::HashMap;
        
        let mut content = HashMap::new();
        if let Some(lang_content) = document.content.get(language) {
            content.insert(language.to_string(), lang_content.clone());
        } else {
            // Fallback content if language not found
            content.insert(language.to_string(), format!("# {}\n\nContent not available in {}", document.title, language));
        }
        
        crate::Document {
            title: document.title.clone(),
            content,
            metadata: crate::DocumentMetadata {
                project_id: Some(document.project_id.to_string()),
                screenshots: Vec::new(), // TODO: Convert screenshots if needed
            },
        }
    }

    pub async fn cleanup_old_exports(&self, older_than_days: u64) -> Result<usize> {
        let cutoff_date = Utc::now() - chrono::Duration::days(older_than_days as i64);
        let mut cleaned_count = 0;

        {
            let mut queue = self.job_queue.lock().unwrap();
            let initial_len = queue.len();
            
            queue.retain(|job| {
                if let Some(completed_at) = job.completed_at {
                    if completed_at < cutoff_date {
                        // Clean up output files
                        for output_file in &job.output_files {
                            if output_file.file_path.exists() {
                                let _ = std::fs::remove_file(&output_file.file_path);
                            }
                        }
                        return false;
                    }
                }
                true
            });

            cleaned_count = initial_len - queue.len();
        }

        Ok(cleaned_count)
    }
}

impl Default for ExportService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_export_service_creation() {
        let service = ExportService::new();
        let (queued, active) = service.get_queue_status();
        assert_eq!(queued, 0);
        assert_eq!(active, 0);
    }

    #[tokio::test]
    async fn test_export_request_validation() {
        let config = ExportConfiguration {
            format: ExportFormat::Pdf,
            layout: ExportLayout::SingleLanguage,
            languages: vec![],
            include_metadata: true,
            include_table_of_contents: true,
            custom_css_path: None,
            template_options: HashMap::new(),
        };

        let result = ExportService::validate_export_config(&config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_export_queue_management() {
        let service = ExportService::new();
        let temp_dir = TempDir::new().unwrap();

        let request = ExportRequest {
            id: Uuid::new_v4(),
            project_id: Uuid::new_v4(),
            document_id: None,
            config: ExportConfiguration {
                format: ExportFormat::Pdf,
                layout: ExportLayout::SingleLanguage,
                languages: vec!["en".to_string()],
                include_metadata: true,
                include_table_of_contents: true,
                custom_css_path: None,
                template_options: HashMap::new(),
            },
            output_directory: temp_dir.path().to_path_buf(),
            filename_prefix: "test".to_string(),
            requested_at: Utc::now(),
            requested_by: "test_user".to_string(),
        };

        let export_id = service.queue_export(request).await.unwrap();
        let status = service.get_export_status(export_id);
        assert!(status.is_some());
    }
}