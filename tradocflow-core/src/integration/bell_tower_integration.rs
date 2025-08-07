use crate::{Document, ScreenshotReference, Result, TradocumentError};
use screenshot_creator::{MarkdownProcessor, ScreenshotRequest, BellTowerParams};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedBellTowerIntegration {
    pub bell_tower_url: String,
    pub screenshot_output_dir: PathBuf,
    pub api_key: Option<String>,
    pub markdown_processor: Option<PathBuf>, // Path to building blocks
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationRequest {
    pub title: String,
    pub languages: Vec<String>,
    pub template_type: String,
    pub include_live_screenshots: bool,
    pub interface_modes: Vec<String>,
    pub project_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotGenerationResult {
    pub screenshot_id: String,
    pub filename: String,
    pub language: String,
    pub interface_mode: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub parameters_used: BellTowerParams,
}

impl EnhancedBellTowerIntegration {
    pub fn new(bell_tower_url: String, screenshot_output_dir: PathBuf) -> Self {
        Self {
            bell_tower_url,
            screenshot_output_dir,
            api_key: None,
            markdown_processor: None,
        }
    }

    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    pub fn with_markdown_processor(mut self, building_blocks_path: PathBuf) -> Self {
        self.markdown_processor = Some(building_blocks_path);
        self
    }

    pub async fn create_multilingual_manual(&self, request: DocumentationRequest) -> Result<Document> {
        let mut content = HashMap::new();
        let mut screenshots = Vec::new();

        // Generate content for each language
        for language in &request.languages {
            let manual_content = self.generate_manual_with_screenshots(&request, language).await?;
            content.insert(language.clone(), manual_content);

            // Generate screenshots for this language
            if request.include_live_screenshots {
                let screenshot_results = self.generate_interface_screenshots(&request.interface_modes, language).await?;
                screenshots.extend(screenshot_results.into_iter().map(|result| ScreenshotReference {
                    id: result.screenshot_id,
                    language: result.language,
                    screen_config: serde_json::to_string(&result.parameters_used).unwrap_or_default(),
                    generated_at: Some(result.generated_at),
                }));
            }
        }

        let document = Document {
            id: uuid::Uuid::new_v4(),
            title: request.title.clone(),
            content,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            version: 1,
            status: crate::DocumentStatus::Draft,
            metadata: crate::DocumentMetadata {
                languages: request.languages.clone(),
                tags: vec!["manual".to_string(), "bell-tower".to_string(), "multilingual".to_string()],
                project_id: Some(request.project_id.clone()),
                screenshots,
            },
        };

        Ok(document)
    }

    async fn generate_manual_with_screenshots(&self, request: &DocumentationRequest, language: &str) -> Result<String> {
        let mut content = String::new();
        
        // Generate title and introduction
        content.push_str(&format!("# {} ({})\n\n", request.title, language.to_uppercase()));
        
        // Add generated introduction based on language
        let intro = match language {
            "de" => "Dieses Handbuch beschreibt die Bedienung des Glockenturm-Controllers.",
            "fr" => "Ce manuel décrit l'utilisation du contrôleur de clocher.",
            "es" => "Este manual describe el funcionamiento del controlador de campanario.",
            "it" => "Questo manuale descrive il funzionamento del controller del campanile.",
            "nl" => "Deze handleiding beschrijft de bediening van de klokkentoren controller.",
            _ => "This manual describes the operation of the bell tower controller.",
        };
        content.push_str(&format!("{}\n\n", intro));

        // Add interface screenshots with markdown syntax
        if request.include_live_screenshots {
            for interface_mode in &request.interface_modes {
                let section_title = self.get_interface_title(interface_mode, language);
                content.push_str(&format!("## {}\n\n", section_title));
                
                // Add screenshot with live parameters
                content.push_str(&format!(
                    "![{}](screenshot://type:{}/lang:{}/interface_mode:{}/live:true)\n\n",
                    section_title, interface_mode, language, interface_mode
                ));
                
                // Add interface description
                let description = self.get_interface_description(interface_mode, language);
                content.push_str(&format!("{}\n\n", description));
            }
        }

        // Add operational instructions
        content.push_str(&self.generate_operational_instructions(language));

        Ok(content)
    }

    async fn generate_interface_screenshots(&self, interface_modes: &[String], language: &str) -> Result<Vec<ScreenshotGenerationResult>> {
        let mut results = Vec::new();

        // First, fetch current system state from bell tower controller
        let system_params = self.fetch_live_system_state().await?;

        if let Some(building_blocks_path) = &self.markdown_processor {
            let processor = MarkdownProcessor::with_bell_tower_endpoint(
                building_blocks_path.clone(),
                self.screenshot_output_dir.clone(),
                Some(self.bell_tower_url.clone())
            ).map_err(|e| TradocumentError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

            for interface_mode in interface_modes {
                // Create screenshot request with live parameters
                let request = ScreenshotRequest {
                    screen_type: interface_mode.clone(),
                    language: Some(language.to_string()),
                    size: Some((800, 600)),
                    theme: None,
                    footer: Some(true),
                    custom_params: HashMap::new(),
                    bell_tower_params: Some(system_params.clone()),
                };

                // Generate screenshot filename
                let filename = format!("{}_{}_live.svg", interface_mode, language);
                
                let result = ScreenshotGenerationResult {
                    screenshot_id: format!("{}_{}_{}", interface_mode, language, chrono::Utc::now().timestamp()),
                    filename,
                    language: language.to_string(),
                    interface_mode: interface_mode.clone(),
                    generated_at: chrono::Utc::now(),
                    parameters_used: system_params.clone(),
                };

                results.push(result);
            }
        }

        Ok(results)
    }

    async fn fetch_live_system_state(&self) -> Result<BellTowerParams> {
        // TODO: Implement actual HTTP request to bell tower controller
        // This would make a request to /api/v1/screenshot/system-state endpoint
        
        // For now, return a mock state
        Ok(BellTowerParams {
            system_state: Some("live".to_string()),
            active_program: Some("default_program".to_string()),
            bell_count: Some(6),
            current_sequence: Some("morning_bells".to_string()),
            interface_mode: Some("main".to_string()),
            switch_states: Some({
                let mut states = HashMap::new();
                states.insert("switch_1".to_string(), true);
                states.insert("switch_2".to_string(), false);
                states
            }),
            program_status: Some({
                let mut status = HashMap::new();
                status.insert("program_1".to_string(), "active".to_string());
                status.insert("program_2".to_string(), "disabled".to_string());
                status
            }),
        })
    }

    fn get_interface_title(&self, interface_mode: &str, language: &str) -> String {
        match (interface_mode, language) {
            ("main", "de") => "Hauptbenutzeroberfläche".to_string(),
            ("main", "fr") => "Interface Principal".to_string(),
            ("main", "es") => "Interfaz Principal".to_string(),
            ("main", "it") => "Interfaccia Principale".to_string(),
            ("main", "nl") => "Hoofdinterface".to_string(),
            ("settings", "de") => "Einstellungen".to_string(),
            ("settings", "fr") => "Paramètres".to_string(),
            ("settings", "es") => "Configuración".to_string(),
            ("settings", "it") => "Impostazioni".to_string(),
            ("settings", "nl") => "Instellingen".to_string(),
            ("programs", "de") => "Programme".to_string(),
            ("programs", "fr") => "Programmes".to_string(),
            ("programs", "es") => "Programas".to_string(),
            ("programs", "it") => "Programmi".to_string(),
            ("programs", "nl") => "Programma's".to_string(),
            (mode, _) => {
                let title = mode.replace('_', " ");
                format!("{} Interface", title.chars().map(|c| if c.is_ascii_lowercase() { c.to_uppercase().to_string() } else { c.to_string() }).collect::<String>())
            },
        }
    }

    fn get_interface_description(&self, interface_mode: &str, language: &str) -> String {
        match (interface_mode, language) {
            ("main", "de") => "Die Hauptbenutzeroberfläche bietet Zugriff auf alle wichtigen Funktionen des Glockenturm-Controllers.".to_string(),
            ("main", "fr") => "L'interface principale donne accès à toutes les fonctions importantes du contrôleur de clocher.".to_string(),
            ("main", "es") => "La interfaz principal proporciona acceso a todas las funciones importantes del controlador de campanario.".to_string(),
            ("main", "it") => "L'interfaccia principale fornisce accesso a tutte le funzioni importanti del controller del campanile.".to_string(),
            ("main", "nl") => "De hoofdinterface biedt toegang tot alle belangrijke functies van de klokkentoren controller.".to_string(),
            ("settings", "de") => "Über die Einstellungen können Systemparameter konfiguriert werden.".to_string(),
            ("settings", "fr") => "Les paramètres permettent de configurer les paramètres du système.".to_string(),
            ("settings", "es") => "La configuración permite establecer los parámetros del sistema.".to_string(),
            ("settings", "it") => "Le impostazioni permettono di configurare i parametri del sistema.".to_string(),
            ("settings", "nl") => "De instellingen maken het mogelijk om systeemparameters te configureren.".to_string(),
            (_, _) => format!("This interface provides access to {} functionality.", interface_mode.replace('_', " ")),
        }
    }

    fn generate_operational_instructions(&self, language: &str) -> String {
        match language {
            "de" => r#"## Bedienungsanleitung

### Grundlegende Bedienung
1. Öffnen Sie die Hauptbenutzeroberfläche
2. Wählen Sie das gewünschte Programm aus
3. Starten Sie die Ausführung über die Steuerungsschaltflächen

### Erweiterte Funktionen
- Programmverwaltung über das Programm-Interface
- Einstellungen über das Einstellungs-Interface
- Überwachung über das Debug-Interface
"#.to_string(),
            "fr" => r#"## Instructions d'Utilisation

### Utilisation de Base
1. Ouvrez l'interface principale
2. Sélectionnez le programme désiré
3. Démarrez l'exécution via les boutons de contrôle

### Fonctions Avancées
- Gestion des programmes via l'interface des programmes
- Paramètres via l'interface des paramètres
- Surveillance via l'interface de débogage
"#.to_string(),
            _ => r#"## Operating Instructions

### Basic Operation
1. Open the main interface
2. Select the desired program
3. Start execution using the control buttons

### Advanced Features
- Program management through the programs interface
- Settings through the settings interface
- Monitoring through the debug interface
"#.to_string(),
        }
    }

    pub async fn process_markdown_with_screenshots(&self, content: &str) -> Result<String> {
        if let Some(building_blocks_path) = &self.markdown_processor {
            let processor = MarkdownProcessor::with_bell_tower_endpoint(
                building_blocks_path.clone(),
                self.screenshot_output_dir.clone(),
                Some(self.bell_tower_url.clone())
            ).map_err(|e| TradocumentError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

            processor.process_markdown_content(content)
                .map_err(|e| TradocumentError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))
        } else {
            Ok(content.to_string())
        }
    }

    pub async fn update_document_screenshots(&self, document: &mut Document) -> Result<()> {
        // Process each language version and update screenshots
        for (_language, content) in &mut document.content {
            *content = self.process_markdown_with_screenshots(content).await?;
        }
        
        document.updated_at = chrono::Utc::now();
        Ok(())
    }
}