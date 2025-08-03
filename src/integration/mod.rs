
// use crate::Document;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualSection {
    pub id: String,
    pub title: HashMap<String, String>, // language -> title
    pub content: HashMap<String, String>, // language -> content
    pub screenshots: Vec<String>, // screenshot IDs
    pub order: u32,
}

// impl ManualSection {
//
// pub fn new(base_url: String) -> Self {
//     Self {
//         base_url,
//         api_key: None,
//     }
// }
//
// pub fn with_api_key(mut self, api_key: String) -> Self {
//     self.api_key = Some(api_key);
//     self
// }
//
// pub async fn generate_manual_document(&self, project_id: &str) -> Result<Document> {
//     let languages = vec!["en", "de", "fr", "es", "it", "nl"];
//     let mut content = HashMap::new();
//     let mut screenshots = Vec::new();
//
//     for language in &languages {
//         let manual_content = self.generate_manual_content(project_id, language).await?;
//         content.insert(language.to_string(), manual_content);
//
//         let screenshot_refs = self.generate_screenshots_for_language(project_id, language).await?;
//         screenshots.extend(screenshot_refs);
//     }
//
//     let document = Document {
//         id: uuid::Uuid::new_v4(),
//         title: "Bell Tower Controller Manual".to_string(),
//         content,
//         created_at: chrono::Utc::now(),
//         updated_at: chrono::Utc::now(),
//         version: 1,
//         status: crate::DocumentStatus::Draft,
//         metadata: crate::DocumentMetadata {
//             languages: languages.iter().map(|s| s.to_string()).collect(),
//             tags: vec!["manual".to_string(), "bell-tower".to_string()],
//             project_id: Some(project_id.to_string()),
//             screenshots,
//         },
//     };
//
//     Ok(document)
// }

// async fn generate_manual_content(&self, project_id: &str, language: &str) -> Result<String> {
//     let sections = self.get_manual_sections(project_id, language).await?;
//
//     let mut content = String::new();
//     content.push_str(&format!("# Bell Tower Controller Manual ({})\n\n", language.to_uppercase()));
//
//     for section in sections {
//         if let Some(title) = section.title.get(language) {
//             content.push_str(&format!("## {}\n\n", title));
//         }
//
//         if let Some(section_content) = section.content.get(language) {
//             content.push_str(section_content);
//             content.push_str("\n\n");
//         }
//
//         for screenshot_id in &section.screenshots {
//             content.push_str(&format!("{{screenshot:{}}}\n\n", screenshot_id));
//         }
//     }
//
//     Ok(content)
// }

// async fn get_manual_sections(&self, _project_id: &str, _language: &str) -> Result<Vec<ManualSection>> {
//     let mut sections = Vec::new();
//
//     // Introduction section
//     let mut intro_title = HashMap::new();
//     intro_title.insert("en".to_string(), "Introduction".to_string());
//     intro_title.insert("de".to_string(), "Einführung".to_string());
//     intro_title.insert("fr".to_string(), "Introduction".to_string());
//     intro_title.insert("es".to_string(), "Introducción".to_string());
//     intro_title.insert("it".to_string(), "Introduzione".to_string());
//     intro_title.insert("nl".to_string(), "Inleiding".to_string());
//
//     let mut intro_content = HashMap::new();
//     intro_content.insert("en".to_string(), "The Bell Tower Controller is a sophisticated system for managing bell sequences and programs.".to_string());
//     intro_content.insert("de".to_string(), "Der Glockenturm-Controller ist ein ausgeklügeltes System zur Verwaltung von Glockensequenzen und -programmen.".to_string());
//     intro_content.insert("fr".to_string(), "Le contrôleur de clocher est un système sophistiqué pour gérer les séquences et programmes de cloches.".to_string());
//     intro_content.insert("es".to_string(), "El controlador de campanario es un sistema sofisticado para gestionar secuencias y programas de campanas.".to_string());
//     intro_content.insert("it".to_string(), "Il controller del campanile è un sistema sofisticato per gestire sequenze e programmi di campane.".to_string());
//     intro_content.insert("nl".to_string(), "De klokkentoren controller is een geavanceerd systeem voor het beheren van kloksequenties en programma's.".to_string());
//
//     sections.push(ManualSection {
//         id: "introduction".to_string(),
//         title: intro_title,
//         content: intro_content,
//         screenshots: vec!["main_screen".to_string()],
//         order: 1,
//     });
//
//     // Main Interface section
//     let mut interface_title = HashMap::new();
//     interface_title.insert("en".to_string(), "Main Interface".to_string());
//     interface_title.insert("de".to_string(), "Hauptbenutzeroberfläche".to_string());
//     interface_title.insert("fr".to_string(), "Interface principale".to_string());
//     interface_title.insert("es".to_string(), "Interfaz principal".to_string());
//     interface_title.insert("it".to_string(), "Interfaccia principale".to_string());
//     interface_title.insert("nl".to_string(), "Hoofdinterface".to_string());
//
//     let mut interface_content = HashMap::new();
//     interface_content.insert("en".to_string(), "The main interface provides access to all system functions through an intuitive web-based control panel.".to_string());
//     interface_content.insert("de".to_string(), "Die Hauptbenutzeroberfläche bietet Zugriff auf alle Systemfunktionen über ein intuitives webbasiertes Bedienfeld.".to_string());
//     interface_content.insert("fr".to_string(), "L'interface principale donne accès à toutes les fonctions du système via un panneau de contrôle web intuitif.".to_string());
//     interface_content.insert("es".to_string(), "La interfaz principal proporciona acceso a todas las funciones del sistema a través de un panel de control web intuitivo.".to_string());
//     interface_content.insert("it".to_string(), "L'interfaccia principale fornisce accesso a tutte le funzioni del sistema attraverso un pannello di controllo web intuitivo.".to_string());
//     interface_content.insert("nl".to_string(), "De hoofdinterface biedt toegang tot alle systeemfuncties via een intuïtief webgebaseerd bedieningspaneel.".to_string());
//
//     sections.push(ManualSection {
//         id: "main_interface".to_string(),
//         title: interface_title,
//         content: interface_content,
//         screenshots: vec!["interface_overview".to_string(), "control_panel".to_string()],
//         order: 2,
//     });
//
//     Ok(sections)
// }


// pub async fn sync_with_bell_tower(&self, document: &mut Document) -> Result<()> {
//     // Sync document content with live bell tower controller data
//     // This could fetch current system state, update screenshots, etc.
//     document.updated_at = chrono::Utc::now();
//     Ok(())
// }
// }
