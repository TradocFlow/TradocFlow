use crate::{
    export_engine::ExportFormat as EngineExportFormat,
    services::export_service::{
        ExportConfiguration, ExportLayout as ServiceExportLayout, ExportRequest, ExportService,
    },
    Result, TradocumentError,
};
use chrono::Utc;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel, Weak};
use std::{
    collections::HashMap,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

slint::include_modules!();

pub struct ExportBridge {
    export_service: Arc<ExportService>,
    current_project_id: Arc<Mutex<Option<Uuid>>>,
    current_document_id: Arc<Mutex<Option<Uuid>>>,
    available_languages: Arc<Mutex<Vec<String>>>,
}

impl ExportBridge {
    pub fn new() -> Self {
        Self {
            export_service: Arc::new(ExportService::new()),
            current_project_id: Arc::new(Mutex::new(None)),
            current_document_id: Arc::new(Mutex::new(None)),
            available_languages: Arc::new(Mutex::new(vec![
                "en".to_string(),
                "es".to_string(),
                "fr".to_string(),
                "de".to_string(),
                "it".to_string(),
                "nl".to_string(),
            ])),
        }
    }

    pub fn setup_export_dialog<T: ComponentHandle>(
        &self,
        main_window: &Weak<T>,
    ) -> Result<()> {
        let export_service = Arc::clone(&self.export_service);
        let current_project_id = Arc::clone(&self.current_project_id);
        let current_document_id = Arc::clone(&self.current_document_id);
        let available_languages = Arc::clone(&self.available_languages);

        // Set up progress monitoring
        let progress_receiver = export_service.subscribe_to_progress();
        let main_window_weak = main_window.clone();

        tokio::spawn(async move {
            let mut receiver = progress_receiver;
            while let Some(progress) = receiver.recv().await {
                if let Some(window) = main_window_weak.upgrade() {
                    let _ = window.invoke_update_export_progress(
                        progress.current_step.into(),
                        progress.progress,
                        progress.total_files as i32,
                        progress.completed_files as i32,
                    );
                }
            }
        });

        Ok(())
    }

    pub fn show_export_dialog<T: ComponentHandle>(
        &self,
        main_window: &Weak<T>,
    ) -> Result<()> {
        if let Some(window) = main_window.upgrade() {
            // Initialize available languages
            let languages = self.available_languages.lock().unwrap();
            let language_options: Vec<ExportLanguageOption> = languages
                .iter()
                .map(|lang| ExportLanguageOption {
                    code: lang.clone().into(),
                    name: self.get_language_name(lang).into(),
                    enabled: lang == "en", // Default to English enabled
                })
                .collect();

            let language_model = Rc::new(VecModel::from(language_options));
            window.set_export_available_languages(ModelRc::from(language_model));

            // Set default values
            window.set_export_selected_format(ExportFormat::PDF);
            window.set_export_selected_layout(ExportLayout::SingleLanguage);
            window.set_export_filename_prefix("document".into());
            window.set_export_include_metadata(true);
            window.set_export_include_table_of_contents(true);

            // Show the dialog
            window.set_show_export_dialog(true);
        }

        Ok(())
    }

    pub fn handle_format_changed<T: ComponentHandle>(
        &self,
        main_window: &Weak<T>,
        format: ExportFormat,
    ) {
        if let Some(window) = main_window.upgrade() {
            window.set_export_selected_format(format);
        }
    }

    pub fn handle_layout_changed<T: ComponentHandle>(
        &self,
        main_window: &Weak<T>,
        layout: ExportLayout,
    ) {
        if let Some(window) = main_window.upgrade() {
            window.set_export_selected_layout(layout);
        }
    }

    pub fn handle_language_toggled<T: ComponentHandle>(
        &self,
        main_window: &Weak<T>,
        language_code: SharedString,
        enabled: bool,
    ) {
        if let Some(window) = main_window.upgrade() {
            let languages = window.get_export_available_languages();
            for i in 0..languages.row_count() {
                if let Some(mut lang) = languages.row_data(i) {
                    if lang.code == language_code {
                        lang.enabled = enabled;
                        languages.set_row_data(i, lang);
                        break;
                    }
                }
            }
        }
    }

    pub fn handle_browse_output_directory<T: ComponentHandle>(
        &self,
        main_window: &Weak<T>,
    ) {
        // This would typically open a native file dialog
        // For now, we'll use a placeholder implementation
        if let Some(window) = main_window.upgrade() {
            // In a real implementation, you would use a file dialog crate like rfd
            let default_path = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("exports");
            
            window.set_export_output_directory(
                default_path.to_string_lossy().to_string().into()
            );
        }
    }

    pub fn handle_browse_custom_css<T: ComponentHandle>(
        &self,
        main_window: &Weak<T>,
    ) {
        // This would typically open a native file dialog for CSS files
        // For now, we'll use a placeholder implementation
        if let Some(window) = main_window.upgrade() {
            // In a real implementation, you would use a file dialog crate like rfd
            // with CSS file filters
            window.set_export_custom_css_path("".into());
        }
    }

    pub async fn handle_start_export<T: ComponentHandle>(
        &self,
        main_window: &Weak<T>,
    ) -> Result<()> {
        let window = main_window.upgrade().ok_or_else(|| {
            TradocumentError::Ui("Main window not available".to_string())
        })?;

        // Collect export configuration from UI
        let selected_languages = self.get_selected_languages(&window);
        if selected_languages.is_empty() {
            return Err(TradocumentError::Validation(
                "Please select at least one language to export".to_string(),
            ));
        }

        let output_directory = PathBuf::from(window.get_export_output_directory().to_string());
        if output_directory.to_string_lossy().is_empty() {
            return Err(TradocumentError::Validation(
                "Please select an output directory".to_string(),
            ));
        }

        // Create output directory if it doesn't exist
        tokio::fs::create_dir_all(&output_directory).await?;

        let config = ExportConfiguration {
            format: self.convert_export_format(window.get_export_selected_format()),
            layout: self.convert_export_layout(window.get_export_selected_layout()),
            languages: selected_languages,
            include_metadata: window.get_export_include_metadata(),
            include_table_of_contents: window.get_export_include_table_of_contents(),
            custom_css_path: {
                let css_path = window.get_export_custom_css_path().to_string();
                if css_path.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(css_path))
                }
            },
            template_options: HashMap::new(),
        };

        let request = ExportRequest {
            id: Uuid::new_v4(),
            project_id: self.current_project_id.lock().unwrap()
                .ok_or_else(|| TradocumentError::Validation("No project loaded".to_string()))?,
            document_id: *self.current_document_id.lock().unwrap(),
            config,
            output_directory,
            filename_prefix: window.get_export_filename_prefix().to_string(),
            requested_at: Utc::now(),
            requested_by: "current_user".to_string(), // TODO: Get from user session
        };

        // Start the export
        window.set_export_in_progress(true);
        let export_id = self.export_service.queue_export(request).await?;

        // Update queue information
        let (queued, active) = self.export_service.get_queue_status();
        window.set_export_queue_position(queued as i32);
        window.set_export_queue_total((queued + active) as i32);
        window.set_export_show_queue_info(queued > 1);

        Ok(())
    }

    pub async fn handle_cancel_export<T: ComponentHandle>(
        &self,
        main_window: &Weak<T>,
    ) -> Result<()> {
        if let Some(window) = main_window.upgrade() {
            // In a real implementation, we would track the current export ID
            // For now, we'll just reset the UI state
            window.set_export_in_progress(false);
            window.set_export_progress(ExportProgress {
                current_step: "".into(),
                progress: 0.0,
                total_files: 0,
                completed_files: 0,
                is_complete: false,
                error_message: "".into(),
            });
        }

        Ok(())
    }

    pub fn handle_close_dialog<T: ComponentHandle>(
        &self,
        main_window: &Weak<T>,
    ) {
        if let Some(window) = main_window.upgrade() {
            window.set_show_export_dialog(false);
            window.set_export_in_progress(false);
        }
    }

    pub fn handle_view_export_history<T: ComponentHandle>(
        &self,
        main_window: &Weak<T>,
    ) {
        let history = self.export_service.get_export_history(Some(50));
        
        // In a real implementation, you would show this in a separate dialog
        // or panel. For now, we'll just log it.
        println!("Export History:");
        println!("Total exports: {}", history.total_exports);
        println!("Successful: {}", history.successful_exports);
        println!("Failed: {}", history.failed_exports);
        
        for export in &history.exports {
            println!(
                "- {} ({}): {} files exported",
                export.request.filename_prefix,
                export.status,
                export.output_files.len()
            );
        }
    }

    pub fn set_current_project(&self, project_id: Uuid) {
        *self.current_project_id.lock().unwrap() = Some(project_id);
    }

    pub fn set_current_document(&self, document_id: Option<Uuid>) {
        *self.current_document_id.lock().unwrap() = document_id;
    }

    pub fn set_available_languages(&self, languages: Vec<String>) {
        *self.available_languages.lock().unwrap() = languages;
    }

    fn get_selected_languages<T: ComponentHandle>(&self, window: &T) -> Vec<String> {
        let languages = window.get_export_available_languages();
        let mut selected = Vec::new();

        for i in 0..languages.row_count() {
            if let Some(lang) = languages.row_data(i) {
                if lang.enabled {
                    selected.push(lang.code.to_string());
                }
            }
        }

        selected
    }

    fn convert_export_format(&self, format: ExportFormat) -> EngineExportFormat {
        match format {
            ExportFormat::PDF => EngineExportFormat::Pdf,
            ExportFormat::HTML => EngineExportFormat::Html,
            ExportFormat::Both => EngineExportFormat::Both,
        }
    }

    fn convert_export_layout(&self, layout: ExportLayout) -> ServiceExportLayout {
        match layout {
            ExportLayout::SingleLanguage => ServiceExportLayout::SingleLanguage,
            ExportLayout::SideBySide => ServiceExportLayout::SideBySide,
            ExportLayout::Sequential => ServiceExportLayout::Sequential,
        }
    }

    fn get_language_name(&self, code: &str) -> String {
        match code {
            "en" => "English",
            "es" => "Spanish",
            "fr" => "French",
            "de" => "German",
            "it" => "Italian",
            "nl" => "Dutch",
            _ => code,
        }.to_string()
    }
}

impl Default for ExportBridge {
    fn default() -> Self {
        Self::new()
    }
}

// Extension trait to add export-related methods to the main window
pub trait ExportDialogExt {
    fn invoke_update_export_progress(
        &self,
        current_step: SharedString,
        progress: f32,
        total_files: i32,
        completed_files: i32,
    ) -> Result<()>;
}

// This would be implemented for the actual main window component
// For now, it's a placeholder trait
impl<T> ExportDialogExt for T
where
    T: ComponentHandle,
{
    fn invoke_update_export_progress(
        &self,
        _current_step: SharedString,
        _progress: f32,
        _total_files: i32,
        _completed_files: i32,
    ) -> Result<()> {
        // In a real implementation, this would update the UI
        Ok(())
    }
}