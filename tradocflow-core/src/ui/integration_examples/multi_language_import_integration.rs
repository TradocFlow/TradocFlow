// Multi-Language Import Dialog Integration Example
// 
// This file demonstrates how to integrate the MultiLanguageImportDialog Slint component
// with the MultiLanguageManualImportService backend in a real Rust application.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use slint::{Model, VecModel, ComponentHandle};

use crate::services::multi_language_manual_import::{
    MultiLanguageManualImportService, MultiLanguageImportConfig, SupportedLanguage,
    FolderScanResult, MultiLanguageImportResult, LanguageConflict
};

// Import the Slint generated types (these would be generated from the .slint files)
use crate::ui::{MainWindow, LanguageFileMapping, ImportConfiguration, ImportProgress, ImportStage, ValidationError};

/// Integration handler for multi-language import functionality
pub struct MultiLanguageImportHandler {
    import_service: Arc<MultiLanguageManualImportService>,
    main_window: slint::Weak<MainWindow>,
    current_scan_result: Arc<Mutex<Option<FolderScanResult>>>,
}

impl MultiLanguageImportHandler {
    /// Create a new integration handler
    pub fn new(main_window: &MainWindow) -> Result<Self, Box<dyn std::error::Error>> {
        let import_service = Arc::new(MultiLanguageManualImportService::new()?);
        let main_window_weak = main_window.as_weak();
        
        let handler = Self {
            import_service,
            main_window: main_window_weak,
            current_scan_result: Arc::new(Mutex::new(None)),
        };
        
        // Set up callbacks
        handler.setup_callbacks()?;
        
        Ok(handler)
    }
    
    /// Set up the Slint callback handlers
    fn setup_callbacks(&self) -> Result<(), Box<dyn std::error::Error>> {
        let main_window = self.main_window.upgrade()
            .ok_or("Main window reference lost")?;
        
        // Clone necessary data for closures
        let import_service = Arc::clone(&self.import_service);
        let scan_result_ref = Arc::clone(&self.current_scan_result);
        let main_window_weak = main_window.as_weak();
        
        // Browse folder callback
        let browse_service = Arc::clone(&import_service);
        let browse_window = main_window.as_weak();
        main_window.on_import_browse_folder(move || {
            if let Some(window) = browse_window.upgrade() {
                // Open native file dialog for folder selection
                if let Some(folder_path) = Self::open_folder_dialog() {
                    window.set_import_selected_folder_path(folder_path.to_string_lossy().into());
                }
            }
        });
        
        // Scan folder callback
        let scan_service = Arc::clone(&import_service);
        let scan_window = main_window.as_weak();
        let scan_result_clone = Arc::clone(&scan_result_ref);
        main_window.on_import_scan_folder(move || {
            if let Some(window) = scan_window.upgrade() {
                let folder_path = window.get_import_selected_folder_path();
                let config = Self::convert_slint_to_rust_config(&window.get_import_configuration());
                
                // Perform scan in background thread
                let path = PathBuf::from(folder_path.as_str());
                match scan_service.scan_folder(&path, &config) {
                    Ok(scan_result) => {
                        // Update UI with scan results
                        let mappings = Self::convert_scan_result_to_mappings(&scan_result);
                        let all_files = Self::extract_all_detected_files(&scan_result);
                        
                        window.set_import_language_mappings(mappings.into());
                        window.set_import_all_detected_files(all_files.into());
                        window.set_import_unmatched_files(
                            scan_result.unmatched_files.iter()
                                .map(|p| p.to_string_lossy().to_string().into())
                                .collect::<Vec<slint::SharedString>>().into()
                        );
                        window.set_import_folder_scanned(true);
                        window.set_import_scan_successful(scan_result.conflicts.is_empty() && scan_result.missing_languages.is_empty());
                        window.set_import_conflicts_count(scan_result.conflicts.len() as i32);
                        window.set_import_missing_required_count(scan_result.missing_languages.len() as i32);
                        
                        // Store scan result for later use
                        *scan_result_clone.lock().unwrap() = Some(scan_result);
                    }
                    Err(e) => {
                        // Handle scan error
                        let validation_errors = vec![ValidationError {
                            field: "folder-scan".into(),
                            message: format!("Scan failed: {}", e).into(),
                            severity: "error".into(),
                        }];
                        window.set_import_validation_errors(validation_errors.into());
                        window.set_import_folder_scanned(false);
                        window.set_import_scan_successful(false);
                    }
                }
            }
        });
        
        // Start import callback
        let import_service_clone = Arc::clone(&import_service);
        let import_window = main_window.as_weak();
        let import_scan_result = Arc::clone(&scan_result_ref);
        main_window.on_import_start_import(move || {
            if let Some(window) = import_window.upgrade() {
                let folder_path = PathBuf::from(window.get_import_selected_folder_path().as_str());
                let config = Self::convert_slint_to_rust_config(&window.get_import_configuration());
                
                // Set import in progress
                window.set_import_in_progress(true);
                
                // Create progress callback
                let progress_window = window.as_weak();
                let progress_callback = Some(Box::new(move |progress_info| {
                    if let Some(pw) = progress_window.upgrade() {
                        let import_progress = Self::convert_progress_info_to_slint(&progress_info);
                        pw.set_import_progress(import_progress);
                    }
                }) as Box<dyn Fn(_) + Send + Sync>);
                
                // Perform import in background thread
                std::thread::spawn(move || {
                    match import_service_clone.import_multi_language_manual(&folder_path, config, progress_callback) {
                        Ok(import_result) => {
                            // Handle successful import
                            if let Some(pw) = progress_window.upgrade() {
                                pw.set_import_in_progress(false);
                                let final_progress = ImportProgress {
                                    current_stage: ImportStage::Completed,
                                    current_file: "Import completed successfully".into(),
                                    progress_percent: 1.0,
                                    total_files: import_result.imported_languages.len() as i32,
                                    completed_files: import_result.imported_languages.len() as i32,
                                    current_language: "".into(),
                                    message: format!("Successfully imported {} languages", import_result.imported_languages.len()).into(),
                                    warnings: import_result.warnings.iter().map(|w| w.as_str().into()).collect::<Vec<slint::SharedString>>().into(),
                                    errors: import_result.failed_languages.values().map(|e| e.as_str().into()).collect::<Vec<slint::SharedString>>().into(),
                                    is_complete: true,
                                    can_cancel: false,
                                };
                                pw.set_import_progress(final_progress);
                            }
                        }
                        Err(e) => {
                            // Handle import error
                            if let Some(pw) = progress_window.upgrade() {
                                pw.set_import_in_progress(false);
                                let error_progress = ImportProgress {
                                    current_stage: ImportStage::Failed,
                                    current_file: "Import failed".into(),
                                    progress_percent: 0.0,
                                    total_files: 0,
                                    completed_files: 0,
                                    current_language: "".into(),
                                    message: format!("Import failed: {}", e).into(),
                                    warnings: Vec::<slint::SharedString>::new().into(),
                                    errors: vec![format!("Import error: {}", e).into()].into(),
                                    is_complete: false,
                                    can_cancel: false,
                                };
                                pw.set_import_progress(error_progress);
                            }
                        }
                    }
                });
            }
        });
        
        // File mapping changed callback
        main_window.on_import_file_mapping_changed(move |language, file| {
            // Handle language file mapping changes
            println!("File mapping changed: {:?} -> {}", language, file);
        });
        
        // Validation callback
        let validation_window = main_window.as_weak();
        main_window.on_import_validation_requested(move || {
            if let Some(window) = validation_window.upgrade() {
                // Perform validation and update validation errors
                let mut errors = Vec::new();
                
                // Validate folder path
                let folder_path = window.get_import_selected_folder_path();
                if folder_path.is_empty() {
                    errors.push(ValidationError {
                        field: "folder-path".into(),
                        message: "Please select a folder to scan".into(),
                        severity: "error".into(),
                    });
                }
                
                // Validate scan results
                if window.get_import_folder_scanned() {
                    if window.get_import_conflicts_count() > 0 && !window.get_import_configuration().auto_resolve_conflicts {
                        errors.push(ValidationError {
                            field: "conflicts".into(),
                            message: format!("{} conflicts need manual resolution", window.get_import_conflicts_count()).into(),
                            severity: "warning".into(),
                        });
                    }
                    
                    if window.get_import_missing_required_count() > 0 && !window.get_import_configuration().allow_partial_import {
                        errors.push(ValidationError {
                            field: "missing-languages".into(),
                            message: format!("{} required languages are missing", window.get_import_missing_required_count()).into(),
                            severity: "error".into(),
                        });
                    }
                }
                
                window.set_import_validation_errors(errors.into());
            }
        });
        
        Ok(())
    }
    
    /// Convert Slint ImportConfiguration to Rust MultiLanguageImportConfig
    fn convert_slint_to_rust_config(slint_config: &ImportConfiguration) -> MultiLanguageImportConfig {
        MultiLanguageImportConfig {
            required_languages: slint_config.required_languages.iter()
                .filter_map(|lang| Self::convert_slint_to_rust_language(*lang))
                .collect(),
            optional_languages: slint_config.optional_languages.iter()
                .filter_map(|lang| Self::convert_slint_to_rust_language(*lang))
                .collect(),
            allow_partial_import: slint_config.allow_partial_import,
            recursive_scan: slint_config.recursive_scan,
            max_depth: if slint_config.max_scan_depth > 100 { None } else { Some(slint_config.max_scan_depth as usize) },
            processing_config: crate::services::document_processing::DocumentProcessingConfig::default(),
            resolve_conflicts_automatically: slint_config.auto_resolve_conflicts,
        }
    }
    
    /// Convert Slint SupportedLanguage enum to Rust SupportedLanguage enum
    fn convert_slint_to_rust_language(slint_lang: crate::ui::SupportedLanguage) -> Option<SupportedLanguage> {
        match slint_lang {
            crate::ui::SupportedLanguage::English => Some(SupportedLanguage::English),
            crate::ui::SupportedLanguage::German => Some(SupportedLanguage::German),
            crate::ui::SupportedLanguage::Spanish => Some(SupportedLanguage::Spanish),
            crate::ui::SupportedLanguage::French => Some(SupportedLanguage::French),
            crate::ui::SupportedLanguage::Dutch => Some(SupportedLanguage::Dutch),
        }
    }
    
    /// Convert scan result to Slint language mappings
    fn convert_scan_result_to_mappings(scan_result: &FolderScanResult) -> Vec<LanguageFileMapping> {
        let mut mappings = Vec::new();
        
        // Add detected languages
        for (language, files) in &scan_result.language_files {
            let has_conflict = files.len() > 1;
            let selected_file = files.first()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            
            mappings.push(LanguageFileMapping {
                language: Self::convert_rust_to_slint_language(language),
                detected_files: files.iter()
                    .map(|p| p.to_string_lossy().to_string().into())
                    .collect::<Vec<slint::SharedString>>().into(),
                selected_file: selected_file.into(),
                has_conflict,
                is_missing: false,
                override_enabled: false,
            });
        }
        
        // Add missing required languages
        for missing_lang in &scan_result.missing_languages {
            mappings.push(LanguageFileMapping {
                language: Self::convert_rust_to_slint_language(missing_lang),
                detected_files: Vec::<slint::SharedString>::new().into(),
                selected_file: "".into(),
                has_conflict: false,
                is_missing: true,
                override_enabled: false,
            });
        }
        
        mappings
    }
    
    /// Convert Rust SupportedLanguage to Slint SupportedLanguage
    fn convert_rust_to_slint_language(rust_lang: &SupportedLanguage) -> crate::ui::SupportedLanguage {
        match rust_lang {
            SupportedLanguage::English => crate::ui::SupportedLanguage::English,
            SupportedLanguage::German => crate::ui::SupportedLanguage::German,
            SupportedLanguage::Spanish => crate::ui::SupportedLanguage::Spanish,
            SupportedLanguage::French => crate::ui::SupportedLanguage::French,
            SupportedLanguage::Dutch => crate::ui::SupportedLanguage::Dutch,
        }
    }
    
    /// Extract all detected files from scan result
    fn extract_all_detected_files(scan_result: &FolderScanResult) -> Vec<slint::SharedString> {
        let mut all_files = Vec::new();
        
        for files in scan_result.language_files.values() {
            for file in files {
                all_files.push(file.to_string_lossy().to_string().into());
            }
        }
        
        for file in &scan_result.unmatched_files {
            all_files.push(file.to_string_lossy().to_string().into());
        }
        
        all_files.sort();
        all_files.dedup();
        all_files
    }
    
    /// Convert progress info from Rust to Slint
    fn convert_progress_info_to_slint(progress_info: &crate::services::document_processing::ImportProgressInfo) -> ImportProgress {
        ImportProgress {
            current_stage: match progress_info.stage {
                crate::services::document_processing::ImportStage::Validating => ImportStage::Scanning,
                crate::services::document_processing::ImportStage::Processing => ImportStage::Processing,
                crate::services::document_processing::ImportStage::Converting => ImportStage::Converting,
                crate::services::document_processing::ImportStage::Completed => ImportStage::Completed,
                crate::services::document_processing::ImportStage::Failed => ImportStage::Failed,
            },
            current_file: progress_info.current_file.clone().into(),
            progress_percent: progress_info.progress_percent as f32 / 100.0,
            total_files: 0, // Would be set by the import service
            completed_files: 0, // Would be calculated based on progress
            current_language: "".into(), // Would be extracted from progress info
            message: progress_info.message.clone().into(),
            warnings: progress_info.warnings.iter().map(|w| w.as_str().into()).collect::<Vec<slint::SharedString>>().into(),
            errors: progress_info.errors.iter().map(|e| e.as_str().into()).collect::<Vec<slint::SharedString>>().into(),
            is_complete: progress_info.stage == crate::services::document_processing::ImportStage::Completed,
            can_cancel: true,
        }
    }
    
    /// Open native folder selection dialog
    fn open_folder_dialog() -> Option<PathBuf> {
        // This would use a native file dialog library like nfd or rfd
        // For example, using rfd:
        /*
        use rfd::FileDialog;
        FileDialog::new()
            .set_title("Select Multi-Language Manual Folder")
            .pick_folder()
        */
        
        // Placeholder implementation
        Some(PathBuf::from("/home/user/manuals"))
    }
}

/// Example usage of the integration
pub fn setup_multi_language_import_integration(main_window: &MainWindow) -> Result<(), Box<dyn std::error::Error>> {
    let _handler = MultiLanguageImportHandler::new(main_window)?;
    
    // The handler sets up all the callbacks and maintains the connection
    // between the Slint UI and the Rust backend service
    
    println!("Multi-language import integration setup complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_language_conversion() {
        let rust_lang = SupportedLanguage::English;
        let slint_lang = MultiLanguageImportHandler::convert_rust_to_slint_language(&rust_lang);
        let converted_back = MultiLanguageImportHandler::convert_slint_to_rust_language(slint_lang).unwrap();
        assert_eq!(rust_lang, converted_back);
    }
}