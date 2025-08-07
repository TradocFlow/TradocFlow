use std::collections::HashMap;
use uuid::Uuid;
use anyhow::{Result, Context};
use chrono::Utc;

use crate::models::document::{TranslationUnit, TranslationVersion, TranslationStatus, TranslationNote};
use crate::services::project_manager::ProjectManager;

pub struct TranslationService {
    project_manager: ProjectManager,
}

impl TranslationService {
    pub fn new(project_manager: ProjectManager) -> Self {
        Self { project_manager }
    }
    
    /// Extract paragraphs from a chapter and create translation units
    pub async fn extract_paragraphs_for_translation(&self, project_id: Uuid, chapter_id: Uuid, chapter_slug: &str, source_language: &str) -> Result<Vec<TranslationUnit>> {
        // Load chapter content
        let content = self.project_manager.load_chapter_content(project_id, chapter_slug).await?;
        
        let source_content = content.get(source_language)
            .context("Source language content not found")?;
        
        let mut units = self.project_manager.extract_paragraphs_for_translation(chapter_id, source_content);
        
        // Set the correct source language
        for unit in &mut units {
            unit.source_language = source_language.to_string();
        }
        
        // Load existing translation units and merge
        let mut existing_units = self.project_manager.load_translation_units(project_id).await?;
        
        // Filter out units for this chapter to avoid duplicates
        existing_units.retain(|u| u.chapter_id != chapter_id);
        
        // Add new units
        existing_units.extend(units.clone());
        
        // Save updated units
        self.project_manager.save_translation_units(project_id, &existing_units).await?;
        
        Ok(units)
    }
    
    /// Add or update a translation for a specific paragraph
    pub async fn add_translation(&self, project_id: Uuid, unit_id: Uuid, target_language: &str, translation_text: &str, translator: &str) -> Result<()> {
        let mut units = self.project_manager.load_translation_units(project_id).await?;
        
        // Find the unit and add/update translation
        for unit in &mut units {
            if unit.id == unit_id {
                let translation = TranslationVersion {
                    text: translation_text.to_string(),
                    translator: translator.to_string(),
                    status: TranslationStatus::Completed,
                    quality_score: None,
                    created_at: Utc::now(),
                    reviewed_at: None,
                    reviewer: None,
                };
                
                unit.translations.insert(target_language.to_string(), translation);
                unit.updated_at = Utc::now();
                break;
            }
        }
        
        // Save updated units
        self.project_manager.save_translation_units(project_id, &units).await?;
        
        Ok(())
    }
    
    /// Update translation status (e.g., for review workflow)
    pub async fn update_translation_status(&self, project_id: Uuid, unit_id: Uuid, target_language: &str, status: TranslationStatus, reviewer: Option<&str>) -> Result<()> {
        let mut units = self.project_manager.load_translation_units(project_id).await?;
        
        for unit in &mut units {
            if unit.id == unit_id {
                if let Some(translation) = unit.translations.get_mut(target_language) {
                    translation.status = status;
                    if let Some(reviewer_name) = reviewer {
                        translation.reviewer = Some(reviewer_name.to_string());
                        translation.reviewed_at = Some(Utc::now());
                    }
                }
                unit.updated_at = Utc::now();
                break;
            }
        }
        
        self.project_manager.save_translation_units(project_id, &units).await?;
        Ok(())
    }
    
    /// Add a note to a translation unit
    pub async fn add_translation_note(&self, project_id: Uuid, unit_id: Uuid, author: &str, note: &str) -> Result<()> {
        let mut units = self.project_manager.load_translation_units(project_id).await?;
        
        for unit in &mut units {
            if unit.id == unit_id {
                let translation_note = TranslationNote {
                    id: Uuid::new_v4(),
                    author: author.to_string(),
                    note: note.to_string(),
                    created_at: Utc::now(),
                };
                
                unit.notes.push(translation_note);
                unit.updated_at = Utc::now();
                break;
            }
        }
        
        self.project_manager.save_translation_units(project_id, &units).await?;
        Ok(())
    }
    
    /// Generate chapter markdown from completed translations
    pub async fn generate_translated_chapter(&self, project_id: Uuid, chapter_id: Uuid, chapter_slug: &str, target_language: &str) -> Result<String> {
        let units = self.project_manager.load_translation_units(project_id).await?;
        
        // Filter units for this chapter
        let chapter_units: Vec<TranslationUnit> = units.into_iter()
            .filter(|u| u.chapter_id == chapter_id)
            .collect();
        
        let translated_content = self.project_manager.generate_chapter_from_translations(&chapter_units, target_language);
        
        // Save the generated content
        let mut content_map = HashMap::new();
        content_map.insert(target_language.to_string(), translated_content.clone());
        
        // Load existing chapter content to preserve other languages
        if let Ok(existing_content) = self.project_manager.load_chapter_content(project_id, chapter_slug).await {
            for (lang, content) in existing_content {
                if lang != target_language {
                    content_map.insert(lang, content);
                }
            }
        }
        
        // Determine chapter number from existing structure or default to 1
        let chapter_number = self.get_chapter_number(project_id, chapter_slug).await.unwrap_or(1);
        
        self.project_manager.save_chapter_content(project_id, chapter_number, chapter_slug, content_map).await?;
        
        Ok(translated_content)
    }
    
    /// Get translation progress for a project
    pub async fn get_translation_progress(&self, project_id: Uuid) -> Result<TranslationProgressReport> {
        let units = self.project_manager.load_translation_units(project_id).await?;
        let structure = self.project_manager.get_project_structure(project_id).await?;
        
        let mut progress = TranslationProgressReport {
            project_id,
            total_units: units.len(),
            progress_by_language: HashMap::new(),
            progress_by_chapter: HashMap::new(),
        };
        
        // Calculate progress by language
        for language in &structure.languages {
            if language == &structure.languages[0] {
                continue; // Skip source language
            }
            
            let completed = units.iter()
                .filter(|u| {
                    u.translations.get(language)
                        .map(|t| matches!(t.status, TranslationStatus::Completed | TranslationStatus::Approved))
                        .unwrap_or(false)
                })
                .count();
            
            let in_progress = units.iter()
                .filter(|u| {
                    u.translations.get(language)
                        .map(|t| matches!(t.status, TranslationStatus::InProgress | TranslationStatus::UnderReview))
                        .unwrap_or(false)
                })
                .count();
            
            progress.progress_by_language.insert(language.clone(), LanguageProgress {
                language: language.clone(),
                total_units: units.len(),
                completed_units: completed,
                in_progress_units: in_progress,
                pending_units: units.len() - completed - in_progress,
                completion_percentage: if units.is_empty() { 0.0 } else { (completed as f32 / units.len() as f32) * 100.0 },
            });
        }
        
        // Calculate progress by chapter
        let mut chapters_map: HashMap<Uuid, Vec<&TranslationUnit>> = HashMap::new();
        for unit in &units {
            chapters_map.entry(unit.chapter_id).or_default().push(unit);
        }
        
        for (chapter_id, chapter_units) in chapters_map {
            let mut chapter_progress = HashMap::new();
            
            for language in &structure.languages {
                if language == &structure.languages[0] {
                    continue; // Skip source language
                }
                
                let completed = chapter_units.iter()
                    .filter(|u| {
                        u.translations.get(language)
                            .map(|t| matches!(t.status, TranslationStatus::Completed | TranslationStatus::Approved))
                            .unwrap_or(false)
                    })
                    .count();
                
                chapter_progress.insert(language.clone(), ChapterLanguageProgress {
                    language: language.clone(),
                    completed_units: completed,
                    total_units: chapter_units.len(),
                    completion_percentage: if chapter_units.is_empty() { 0.0 } else { (completed as f32 / chapter_units.len() as f32) * 100.0 },
                });
            }
            
            progress.progress_by_chapter.insert(chapter_id, chapter_progress);
        }
        
        Ok(progress)
    }
    
    /// Get translation units for a specific chapter and language
    pub async fn get_chapter_translation_units(&self, project_id: Uuid, chapter_id: Uuid, target_language: Option<&str>) -> Result<Vec<TranslationUnit>> {
        let units = self.project_manager.load_translation_units(project_id).await?;
        
        let mut chapter_units: Vec<TranslationUnit> = units.into_iter()
            .filter(|u| u.chapter_id == chapter_id)
            .collect();
        
        // If target language specified, filter to only units that need translation or have translations
        if let Some(lang) = target_language {
            chapter_units.retain(|u| {
                u.translations.contains_key(lang) || u.translations.is_empty()
            });
        }
        
        // Sort by paragraph ID for consistent order
        chapter_units.sort_by(|a, b| a.paragraph_id.cmp(&b.paragraph_id));
        
        Ok(chapter_units)
    }
    
    /// Helper to get chapter number from project structure
    async fn get_chapter_number(&self, project_id: Uuid, chapter_slug: &str) -> Result<u32> {
        let structure = self.project_manager.get_project_structure(project_id).await?;
        
        for chapter in &structure.chapters {
            if chapter.slug == chapter_slug {
                return Ok(chapter.chapter_number);
            }
        }
        
        Err(anyhow::anyhow!("Chapter not found: {}", chapter_slug))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TranslationProgressReport {
    pub project_id: Uuid,
    pub total_units: usize,
    pub progress_by_language: HashMap<String, LanguageProgress>,
    pub progress_by_chapter: HashMap<Uuid, HashMap<String, ChapterLanguageProgress>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LanguageProgress {
    pub language: String,
    pub total_units: usize,
    pub completed_units: usize,
    pub in_progress_units: usize,
    pub pending_units: usize,
    pub completion_percentage: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChapterLanguageProgress {
    pub language: String,
    pub completed_units: usize,
    pub total_units: usize,
    pub completion_percentage: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::models::project::{Project, ProjectStatus, Priority};
    
    #[tokio::test]
    async fn test_paragraph_extraction() {
        let temp_dir = TempDir::new().unwrap();
        let project_manager = ProjectManager::new(temp_dir.path());
        let translation_service = TranslationService::new(project_manager);
        
        let project_id = Uuid::new_v4();
        let chapter_id = Uuid::new_v4();
        
        // Create a test project
        let project = Project {
            id: project_id,
            name: "Test Project".to_string(),
            description: None,
            status: ProjectStatus::Active,
            owner_id: "test".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            due_date: None,
            priority: Priority::Medium,
            metadata: HashMap::new(),
        };
        
        let _ = translation_service.project_manager.initialize_project(&project, "en", &["es".to_string()]).await.unwrap();
        
        // Create test chapter content
        let content = "# Introduction\n\nThis is the first paragraph.\n\nThis is the second paragraph with more content.\n\n## Section 1\n\nAnother paragraph in a section.".to_string();
        let mut chapter_content = HashMap::new();
        chapter_content.insert("en".to_string(), content);
        
        translation_service.project_manager.save_chapter_content(project_id, 1, "introduction", chapter_content).await.unwrap();
        
        // Extract paragraphs
        let units = translation_service.extract_paragraphs_for_translation(project_id, chapter_id, "introduction", "en").await.unwrap();
        
        assert!(!units.is_empty());
        assert!(units.iter().any(|u| u.source_text.contains("first paragraph")));
        assert!(units.iter().any(|u| u.source_text.contains("second paragraph")));
    }
}