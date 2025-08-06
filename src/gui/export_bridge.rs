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

    pub fn setup_export_dialog<T: ComponentHandle + 'static>(
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
        _main_window: &Weak<T>,
    ) -> Result<()> {
        // TODO: Implement when UI export dialog methods are available in Slint component
        // Required methods: set_export_available_languages, set_export_selected_format,
        // set_export_selected_layout, set_export_filename_prefix, set_export_include_metadata,
        // set_export_include_table_of_contents, set_show_export_dialog
        println!("Export dialog requested - UI not implemented yet");
        Ok(())
    }

    pub fn handle_format_changed<T: ComponentHandle>(
        &self,
        _main_window: &Weak<T>,
        _format: ExportFormat,
    ) {
        // TODO: Implement when set_export_selected_format method is available
        println!("Format change requested - UI not implemented yet");
    }

    pub fn handle_layout_changed<T: ComponentHandle>(
        &self,
        _main_window: &Weak<T>,
        _layout: ExportLayout,
    ) {
        // TODO: Implement when set_export_selected_layout method is available
        println!("Layout change requested - UI not implemented yet");
    }

    pub fn handle_language_toggled<T: ComponentHandle>(
        &self,
        _main_window: &Weak<T>,
        _language_code: SharedString,
        _enabled: bool,
    ) {
        // TODO: Implement when get_export_available_languages method is available
        println!("Language toggle requested - UI not implemented yet");
    }

    pub fn handle_browse_output_directory<T: ComponentHandle>(
        &self,
        _main_window: &Weak<T>,
    ) {
        // TODO: Implement when set_export_output_directory method is available
        println!("Browse output directory requested - UI not implemented yet");
    }

    pub fn handle_browse_custom_css<T: ComponentHandle>(
        &self,
        _main_window: &Weak<T>,
    ) {
        // TODO: Implement when set_export_custom_css_path method is available
        println!("Browse custom CSS requested - UI not implemented yet");
    }

    pub async fn handle_start_export<T: ComponentHandle>(
        &self,
        _main_window: &Weak<T>,
    ) -> Result<()> {
        // TODO: Implement when all export UI methods are available
        // Required methods: get_export_output_directory, get_export_selected_format, 
        // get_export_selected_layout, get_export_include_metadata,
        // get_export_include_table_of_contents, get_export_custom_css_path,
        // get_export_filename_prefix, set_export_in_progress,
        // set_export_queue_position, set_export_queue_total, set_export_show_queue_info
        println!("Start export requested - UI not implemented yet");
        Ok(())
    }

    pub async fn handle_cancel_export<T: ComponentHandle>(
        &self,
        _main_window: &Weak<T>,
    ) -> Result<()> {
        // TODO: Implement when set_export_in_progress and set_export_progress methods are available
        println!("Cancel export requested - UI not implemented yet");
        Ok(())
    }

    pub fn handle_close_dialog<T: ComponentHandle>(
        &self,
        _main_window: &Weak<T>,
    ) {
        // TODO: Implement when set_show_export_dialog and set_export_in_progress methods are available
        println!("Close dialog requested - UI not implemented yet");
    }

    pub fn handle_view_export_history<T: ComponentHandle>(
        &self,
        _main_window: &Weak<T>,
    ) {
        // TODO: Implement when UI for export history is available
        println!("Export history requested - UI not implemented yet");
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

    fn get_selected_languages<T: ComponentHandle>(&self, _window: &T) -> Vec<String> {
        // TODO: Implement when get_export_available_languages method is available
        vec!["en".to_string()] // Default stub
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