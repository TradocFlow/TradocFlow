use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use uuid::Uuid;
use serde_json;
use anyhow::{Result, Context};
use chrono::Utc;

use crate::models::project::Project;
use crate::models::document::{Chapter, TranslationUnit, ProjectStructure, ChapterInfo};

#[derive(Clone)]
pub struct ProjectManager {
    projects_root: PathBuf,
}

impl ProjectManager {
    pub fn new(projects_root: impl AsRef<Path>) -> Self {
        Self {
            projects_root: projects_root.as_ref().to_path_buf(),
        }
    }

    /// Initialize a new project with the multilingual folder structure
    pub async fn initialize_project(&self, project: &Project, source_language: &str, target_languages: &[String]) -> Result<ProjectStructure> {
        let project_path = self.projects_root.join(project.id.to_string());
        
        // Create project directory structure
        fs::create_dir_all(&project_path)
            .context("Failed to create project directory")?;
        
        // Create subdirectories
        let translations_path = project_path.join("translations");
        let chapters_path = project_path.join("chapters");
        
        fs::create_dir_all(&translations_path)
            .context("Failed to create translations directory")?;
        
        fs::create_dir_all(&chapters_path)
            .context("Failed to create chapters directory")?;
        
        // Create language-specific chapter directories
        let mut all_languages = vec![source_language.to_string()];
        all_languages.extend_from_slice(target_languages);
        
        for language in &all_languages {
            let lang_path = chapters_path.join(language);
            fs::create_dir_all(&lang_path)
                .context(format!("Failed to create directory for language: {}", language))?;
        }
        
        // Create project metadata file
        let project_metadata = ProjectMetadata {
            project: project.clone(),
            source_language: source_language.to_string(),
            target_languages: target_languages.to_vec(),
            created_at: Utc::now(),
            structure_version: "1.0".to_string(),
        };
        
        let metadata_path = project_path.join("project.json");
        let metadata_json = serde_json::to_string_pretty(&project_metadata)
            .context("Failed to serialize project metadata")?;
        
        fs::write(&metadata_path, metadata_json)
            .context("Failed to write project metadata")?;
        
        // Initialize empty translation files
        let translation_units_path = translations_path.join("translation_units.json");
        let translation_memory_path = translations_path.join("translation_memory.json");
        
        fs::write(&translation_units_path, "[]")
            .context("Failed to create empty translation units file")?;
        
        fs::write(&translation_memory_path, "{}")
            .context("Failed to create empty translation memory file")?;
        
        // Create project structure info
        let structure = ProjectStructure {
            project_id: project.id,
            base_path: project_path.to_string_lossy().to_string(),
            languages: all_languages,
            chapters: Vec::new(),
        };
        
        Ok(structure)
    }
    
    /// Create a new chapter and generate markdown files for all languages
    pub async fn create_chapter(&self, project_id: Uuid, chapter: &Chapter) -> Result<()> {
        let project_path = self.projects_root.join(project_id.to_string());
        let chapters_path = project_path.join("chapters");
        
        // Load project metadata to get available languages
        let metadata = self.load_project_metadata(project_id).await?;
        let mut all_languages = vec![metadata.source_language.clone()];
        all_languages.extend(metadata.target_languages);
        
        // Create markdown files for each language
        for language in &all_languages {
            let lang_path = chapters_path.join(language);
            let filename = format!("{:02}_{}.md", chapter.chapter_number, chapter.slug);
            let file_path = lang_path.join(&filename);
            
            let default_title = "Untitled".to_string();
            let title = chapter.title.get(language).unwrap_or(&default_title);
            let default_content = format!("# {}\n\n*Content not available in {}*", title, language);
            let content = chapter.content.get(language)
                .unwrap_or(&default_content);
            
            fs::write(&file_path, content)
                .context(format!("Failed to write chapter file for language: {}", language))?;
        }
        
        Ok(())
    }
    
    /// Load chapter content from markdown files
    pub async fn load_chapter_content(&self, project_id: Uuid, chapter_slug: &str) -> Result<HashMap<String, String>> {
        let project_path = self.projects_root.join(project_id.to_string());
        let chapters_path = project_path.join("chapters");
        
        let metadata = self.load_project_metadata(project_id).await?;
        let mut all_languages = vec![metadata.source_language.clone()];
        all_languages.extend(metadata.target_languages);
        
        let mut content = HashMap::new();
        
        for language in &all_languages {
            let lang_path = chapters_path.join(language);
            
            // Find the chapter file (pattern: {number}_{slug}.md)
            if let Ok(entries) = fs::read_dir(&lang_path) {
                for entry in entries.flatten() {
                    if let Some(filename) = entry.file_name().to_str() {
                        if filename.ends_with(&format!("_{}.md", chapter_slug)) {
                            let file_content = fs::read_to_string(entry.path())
                                .context(format!("Failed to read chapter file: {}", filename))?;
                            content.insert(language.clone(), file_content);
                            break;
                        }
                    }
                }
            }
        }
        
        Ok(content)
    }
    
    /// Save chapter content to markdown files
    pub async fn save_chapter_content(&self, project_id: Uuid, chapter_number: u32, chapter_slug: &str, content: HashMap<String, String>) -> Result<()> {
        let project_path = self.projects_root.join(project_id.to_string());
        let chapters_path = project_path.join("chapters");
        
        for (language, text) in content {
            let lang_path = chapters_path.join(&language);
            let filename = format!("{:02}_{}.md", chapter_number, chapter_slug);
            let file_path = lang_path.join(&filename);
            
            fs::write(&file_path, text)
                .context(format!("Failed to write chapter content for language: {}", language))?;
        }
        
        Ok(())
    }
    
    /// Load translation units from the translations folder
    pub async fn load_translation_units(&self, project_id: Uuid) -> Result<Vec<TranslationUnit>> {
        let project_path = self.projects_root.join(project_id.to_string());
        let translation_units_path = project_path.join("translations/translation_units.json");
        
        if !translation_units_path.exists() {
            return Ok(Vec::new());
        }
        
        let content = fs::read_to_string(&translation_units_path)
            .context("Failed to read translation units file")?;
        
        let units: Vec<TranslationUnit> = serde_json::from_str(&content)
            .context("Failed to parse translation units JSON")?;
        
        Ok(units)
    }
    
    /// Save translation units to the translations folder
    pub async fn save_translation_units(&self, project_id: Uuid, units: &[TranslationUnit]) -> Result<()> {
        let project_path = self.projects_root.join(project_id.to_string());
        let translation_units_path = project_path.join("translations/translation_units.json");
        
        let content = serde_json::to_string_pretty(units)
            .context("Failed to serialize translation units")?;
        
        fs::write(&translation_units_path, content)
            .context("Failed to write translation units file")?;
        
        Ok(())
    }
    
    /// Extract paragraphs from markdown content for translation
    pub fn extract_paragraphs_for_translation(&self, chapter_id: Uuid, markdown_content: &str) -> Vec<TranslationUnit> {
        let mut units = Vec::new();
        let mut paragraph_counter = 0;
        
        // Split content into paragraphs (separated by double newlines)
        let paragraphs: Vec<&str> = markdown_content
            .split("\n\n")
            .filter(|p| !p.trim().is_empty())
            .collect();
        
        for paragraph in paragraphs {
            paragraph_counter += 1;
            
            // Skip markdown headers, code blocks, etc. for now
            let trimmed = paragraph.trim();
            if trimmed.starts_with('#') || trimmed.starts_with("```") {
                continue;
            }
            
            let unit = TranslationUnit {
                id: Uuid::new_v4(),
                chapter_id,
                paragraph_id: format!("p{}", paragraph_counter),
                source_language: "en".to_string(), // Default source language
                source_text: trimmed.to_string(),
                translations: HashMap::new(),
                context: None,
                notes: Vec::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            
            units.push(unit);
        }
        
        units
    }
    
    /// Generate chapter markdown from translation units
    pub fn generate_chapter_from_translations(&self, units: &[TranslationUnit], target_language: &str) -> String {
        let mut content = String::new();
        
        for unit in units {
            if let Some(translation) = unit.translations.get(target_language) {
                content.push_str(&translation.text);
                content.push_str("\n\n");
            } else {
                // Fallback to source text with note
                content.push_str(&format!("*[Translation needed: {}]*\n\n", unit.source_text));
            }
        }
        
        content
    }
    
    /// Load project metadata
    async fn load_project_metadata(&self, project_id: Uuid) -> Result<ProjectMetadata> {
        let project_path = self.projects_root.join(project_id.to_string());
        let metadata_path = project_path.join("project.json");
        
        let content = fs::read_to_string(&metadata_path)
            .context("Failed to read project metadata")?;
        
        let metadata: ProjectMetadata = serde_json::from_str(&content)
            .context("Failed to parse project metadata")?;
        
        Ok(metadata)
    }
    
    /// Get project structure information
    pub async fn get_project_structure(&self, project_id: Uuid) -> Result<ProjectStructure> {
        let project_path = self.projects_root.join(project_id.to_string());
        let chapters_path = project_path.join("chapters");
        let metadata = self.load_project_metadata(project_id).await?;
        
        let mut all_languages = vec![metadata.source_language.clone()];
        all_languages.extend(metadata.target_languages);
        
        let mut chapters = Vec::new();
        
        // Read chapters from the source language directory
        let source_lang_path = chapters_path.join(&metadata.source_language);
        if let Ok(entries) = fs::read_dir(&source_lang_path) {
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str() {
                    if filename.ends_with(".md") {
                        // Parse filename: {number}_{slug}.md
                        if let Some(name_part) = filename.strip_suffix(".md") {
                            let parts: Vec<&str> = name_part.splitn(2, '_').collect();
                            if parts.len() == 2 {
                                if let Ok(chapter_number) = parts[0].parse::<u32>() {
                                    let slug = parts[1].to_string();
                                    
                                    // Load titles from all language files
                                    let mut titles = HashMap::new();
                                    let mut file_paths = HashMap::new();
                                    
                                    for language in &all_languages {
                                        let lang_file_path = chapters_path.join(language).join(filename);
                                        file_paths.insert(language.clone(), lang_file_path.to_string_lossy().to_string());
                                        
                                        // Extract title from markdown (first # header)
                                        if let Ok(content) = fs::read_to_string(&lang_file_path) {
                                            for line in content.lines() {
                                                if line.starts_with("# ") {
                                                    titles.insert(language.clone(), line[2..].trim().to_string());
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    
                                    chapters.push(ChapterInfo {
                                        chapter_number,
                                        slug,
                                        title: titles,
                                        file_paths,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Sort chapters by number
        chapters.sort_by_key(|c| c.chapter_number);
        
        Ok(ProjectStructure {
            project_id,
            base_path: project_path.to_string_lossy().to_string(),
            languages: all_languages,
            chapters,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ProjectMetadata {
    project: Project,
    source_language: String,
    target_languages: Vec<String>,
    created_at: chrono::DateTime<Utc>,
    structure_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_project_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ProjectManager::new(temp_dir.path());
        
        let project = Project {
            id: Uuid::new_v4(),
            name: "Test Project".to_string(),
            description: Some("A test project".to_string()),
            status: ProjectStatus::Active,
            owner_id: "test_user".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            due_date: None,
            priority: Priority::Medium,
            metadata: HashMap::new(),
        };
        
        let structure = manager.initialize_project(&project, "en", &["es".to_string(), "fr".to_string()])
            .await
            .unwrap();
        
        assert_eq!(structure.project_id, project.id);
        assert_eq!(structure.languages, vec!["en", "es", "fr"]);
        
        // Verify directory structure exists
        let project_path = temp_dir.path().join(project.id.to_string());
        assert!(project_path.exists());
        assert!(project_path.join("translations").exists());
        assert!(project_path.join("chapters/en").exists());
        assert!(project_path.join("chapters/es").exists());
        assert!(project_path.join("chapters/fr").exists());
        assert!(project_path.join("project.json").exists());
    }
    
    #[tokio::test]
    async fn test_multilingual_project_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let project_manager = ProjectManager::new(temp_dir.path());
        let translation_service = crate::services::TranslationService::new(project_manager.clone());
        
        // Create a test project
        let project_id = Uuid::new_v4();
        let project = Project {
            id: project_id,
            name: "Multilingual Manual".to_string(),
            description: Some("A test of the multilingual system".to_string()),
            status: ProjectStatus::Active,
            owner_id: "test_user".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            due_date: None,
            priority: Priority::High,
            metadata: HashMap::new(),
        };
        
        // Initialize project with English source and Spanish/French targets
        let structure = project_manager.initialize_project(&project, "en", &["es".to_string(), "fr".to_string()])
            .await
            .unwrap();
        
        println!("✅ Project initialized with structure: {:?}", structure);
        
        // Create a test chapter
        let chapter_id = Uuid::new_v4();
        let mut chapter_titles = HashMap::new();
        chapter_titles.insert("en".to_string(), "Getting Started".to_string());
        
        let mut chapter_content = HashMap::new();
        chapter_content.insert("en".to_string(), "# Getting Started\n\nWelcome to our system.\n\nThis guide will help you get started.".to_string());
        
        let chapter = Chapter {
            id: chapter_id,
            document_id: project_id,
            chapter_number: 1,
            title: chapter_titles,
            slug: "getting-started".to_string(),
            content: chapter_content,
            order: 1,
            status: crate::models::document::ChapterStatus::Draft,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        // Create chapter files
        project_manager.create_chapter(project_id, &chapter).await.unwrap();
        println!("✅ Chapter created successfully");
        
        // Extract paragraphs for translation
        let units = translation_service.extract_paragraphs_for_translation(
            project_id,
            chapter_id,
            "getting-started",
            "en"
        ).await.unwrap();
        
        assert!(!units.is_empty(), "Should extract translation units");
        println!("✅ Extracted {} translation units", units.len());
        
        // Add Spanish translations
        for (i, unit) in units.iter().enumerate() {
            let spanish_text = match i {
                0 => "Bienvenido a nuestro sistema.",
                1 => "Esta guía te ayudará a comenzar.",
                _ => "Texto traducido al español.",
            };
            
            translation_service.add_translation(
                project_id,
                unit.id,
                "es",
                spanish_text,
                "test_translator"
            ).await.unwrap();
        }
        println!("✅ Added Spanish translations");
        
        // Generate translated chapter
        let spanish_content = translation_service.generate_translated_chapter(
            project_id,
            chapter_id,
            "getting-started",
            "es"
        ).await.unwrap();
        
        assert!(spanish_content.contains("Bienvenido"), "Should contain Spanish text");
        println!("✅ Generated Spanish chapter: {}", spanish_content);
        
        // Check translation progress
        let progress = translation_service.get_translation_progress(project_id).await.unwrap();
        assert!(progress.progress_by_language.contains_key("es"), "Should track Spanish progress");
        
        let spanish_progress = &progress.progress_by_language["es"];
        assert!(spanish_progress.completion_percentage > 0.0, "Should show translation progress");
        
        println!("✅ Translation progress - Spanish: {:.1}% complete", spanish_progress.completion_percentage);
        println!("📊 Final project structure verified successfully!");
    }
}