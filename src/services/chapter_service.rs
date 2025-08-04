use crate::models::translation_models::{Chapter, ChapterStatus, ChunkMetadata, ValidationError};
use crate::{TradocumentError, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Service for managing chapters with multi-language content support
pub struct ChapterService {
    base_path: PathBuf,
}

impl ChapterService {
    /// Create a new chapter service with the specified base path
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// Create a new chapter with multi-language content support
    pub async fn create_chapter(
        &self,
        request: CreateChapterRequest,
    ) -> Result<Chapter> {
        // Validate the request
        request.validate()?;

        // Generate chapter number if not provided
        let chapter_number = if let Some(num) = request.chapter_number {
            num
        } else {
            self.get_next_chapter_number(request.project_id).await?
        };

        // Create the chapter
        let chapter = Chapter {
            id: Uuid::new_v4(),
            project_id: request.project_id,
            chapter_number,
            title: request.title,
            slug: request.slug,
            content: request.content,
            chunks: request.chunks.unwrap_or_default(),
            status: ChapterStatus::Draft,
            assigned_translators: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Save chapter to file system
        self.save_chapter_to_files(&chapter).await?;

        // Save chapter metadata
        self.save_chapter_metadata(&chapter).await?;

        Ok(chapter)
    }

    /// Update an existing chapter
    pub async fn update_chapter(
        &self,
        chapter_id: Uuid,
        request: UpdateChapterRequest,
    ) -> Result<Chapter> {
        // Load existing chapter
        let mut chapter = self.load_chapter(chapter_id).await?;

        // Update fields if provided
        if let Some(title) = request.title {
            chapter.title = title;
        }

        if let Some(content) = request.content {
            chapter.content = content;
        }

        if let Some(status) = request.status {
            chapter.status = status;
        }

        if let Some(assigned_translators) = request.assigned_translators {
            chapter.assigned_translators = assigned_translators;
        }

        if let Some(chunks) = request.chunks {
            chapter.chunks = chunks;
        }

        chapter.updated_at = Utc::now();

        // Save updated chapter
        self.save_chapter_to_files(&chapter).await?;
        self.save_chapter_metadata(&chapter).await?;

        Ok(chapter)
    }

    /// Load a chapter by ID
    pub async fn load_chapter(&self, chapter_id: Uuid) -> Result<Chapter> {
        let metadata_path = self.get_chapter_metadata_path(chapter_id);
        
        if !metadata_path.exists() {
            return Err(TradocumentError::DocumentNotFound(
                format!("Chapter {} not found", chapter_id)
            ));
        }

        // Load metadata
        let metadata_content = fs::read_to_string(&metadata_path)
            .map_err(|e| TradocumentError::IoError(e))?;
        
        let mut chapter: Chapter = serde_json::from_str(&metadata_content)
            .map_err(|e| TradocumentError::Serialization(e))?;

        // Load content from individual language files
        chapter.content = self.load_chapter_content(chapter_id, &chapter.title).await?;

        Ok(chapter)
    }

    /// List all chapters for a project
    pub async fn list_chapters(&self, project_id: Uuid) -> Result<Vec<ChapterSummary>> {
        let project_path = self.get_project_path(project_id);
        
        if !project_path.exists() {
            return Ok(Vec::new());
        }

        let mut chapters = Vec::new();
        let entries = fs::read_dir(&project_path)
            .map_err(|e| TradocumentError::IoError(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| TradocumentError::IoError(e))?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Some(file_stem) = path.file_stem() {
                    if let Some(filename) = file_stem.to_str() {
                        if filename.starts_with("chapter_") && filename.ends_with("_metadata") {
                            if let Some(chapter_id) = filename
                                .strip_prefix("chapter_")
                                .and_then(|s| s.strip_suffix("_metadata"))
                                .and_then(|s| Uuid::parse_str(s).ok())
                            {
                                if let Ok(chapter) = self.load_chapter(chapter_id).await {
                                    chapters.push(ChapterSummary::from_chapter(&chapter));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Sort by chapter number
        chapters.sort_by_key(|c| c.chapter_number);
        Ok(chapters)
    }

    /// Delete a chapter
    pub async fn delete_chapter(&self, chapter_id: Uuid) -> Result<()> {
        let chapter = self.load_chapter(chapter_id).await?;
        
        // Delete content files
        for language in chapter.title.keys() {
            let content_path = self.get_chapter_content_path(chapter_id, language);
            if content_path.exists() {
                fs::remove_file(&content_path)
                    .map_err(|e| TradocumentError::IoError(e))?;
            }
        }

        // Delete metadata file
        let metadata_path = self.get_chapter_metadata_path(chapter_id);
        if metadata_path.exists() {
            fs::remove_file(&metadata_path)
                .map_err(|e| TradocumentError::IoError(e))?;
        }

        Ok(())
    }

    /// Reorder chapters
    pub async fn reorder_chapters(
        &self,
        _project_id: Uuid,
        chapter_order: Vec<(Uuid, u32)>,
    ) -> Result<()> {
        for (chapter_id, new_number) in chapter_order {
            let mut chapter = self.load_chapter(chapter_id).await?;
            chapter.chapter_number = new_number;
            chapter.updated_at = Utc::now();
            
            self.save_chapter_metadata(&chapter).await?;
        }

        Ok(())
    }

    /// Get chapter statistics
    pub async fn get_chapter_statistics(&self, chapter_id: Uuid) -> Result<ChapterStatistics> {
        let chapter = self.load_chapter(chapter_id).await?;
        
        let mut stats = ChapterStatistics {
            chapter_id,
            total_languages: chapter.title.len(),
            completed_languages: 0,
            total_chunks: chapter.chunks.len(),
            word_count_by_language: HashMap::new(),
            translation_progress: HashMap::new(),
            last_updated: chapter.updated_at,
        };

        // Calculate statistics for each language
        for (language, content) in &chapter.content {
            let word_count = content.split_whitespace().count();
            stats.word_count_by_language.insert(language.clone(), word_count);
            
            // Calculate translation progress (simplified)
            let progress = if content.trim().is_empty() {
                0.0
            } else if content.contains("*This document needs to be translated") {
                0.0
            } else {
                100.0
            };
            
            stats.translation_progress.insert(language.clone(), progress);
            
            if progress >= 100.0 {
                stats.completed_languages += 1;
            }
        }

        Ok(stats)
    }

    /// Search chapters by content
    pub async fn search_chapters(
        &self,
        project_id: Uuid,
        query: &str,
        language: Option<&str>,
    ) -> Result<Vec<ChapterSearchResult>> {
        let chapters = self.list_chapters(project_id).await?;
        let mut results = Vec::new();

        for chapter_summary in chapters {
            let chapter = self.load_chapter(chapter_summary.id).await?;
            
            // Search in titles
            for (lang, title) in &chapter.title {
                if language.map_or(true, |l| l == lang) && 
                   title.to_lowercase().contains(&query.to_lowercase()) {
                    results.push(ChapterSearchResult {
                        chapter_id: chapter.id,
                        chapter_number: chapter.chapter_number,
                        title: title.clone(),
                        language: lang.clone(),
                        match_type: SearchMatchType::Title,
                        context: title.clone(),
                    });
                }
            }

            // Search in content
            for (lang, content) in &chapter.content {
                if language.map_or(true, |l| l == lang) && 
                   content.to_lowercase().contains(&query.to_lowercase()) {
                    // Find context around the match
                    let context = self.extract_search_context(content, query);
                    results.push(ChapterSearchResult {
                        chapter_id: chapter.id,
                        chapter_number: chapter.chapter_number,
                        title: chapter.title.get(lang).cloned().unwrap_or_default(),
                        language: lang.clone(),
                        match_type: SearchMatchType::Content,
                        context,
                    });
                }
            }
        }

        Ok(results)
    }

    // Private helper methods

    async fn get_next_chapter_number(&self, project_id: Uuid) -> Result<u32> {
        let chapters = self.list_chapters(project_id).await?;
        let max_number = chapters.iter().map(|c| c.chapter_number).max().unwrap_or(0);
        Ok(max_number + 1)
    }

    async fn save_chapter_to_files(&self, chapter: &Chapter) -> Result<()> {
        // Ensure project directory exists
        let project_path = self.get_project_path(chapter.project_id);
        fs::create_dir_all(&project_path)
            .map_err(|e| TradocumentError::IoError(e))?;

        // Save content for each language
        for (language, content) in &chapter.content {
            let content_path = self.get_chapter_content_path(chapter.id, language);
            
            // Ensure language directory exists
            if let Some(parent) = content_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| TradocumentError::IoError(e))?;
            }

            fs::write(&content_path, content)
                .map_err(|e| TradocumentError::IoError(e))?;
        }

        Ok(())
    }

    async fn save_chapter_metadata(&self, chapter: &Chapter) -> Result<()> {
        let metadata_path = self.get_chapter_metadata_path(chapter.id);
        
        // Create a metadata-only version of the chapter (without content)
        let metadata = ChapterMetadata {
            id: chapter.id,
            project_id: chapter.project_id,
            chapter_number: chapter.chapter_number,
            title: chapter.title.clone(),
            slug: chapter.slug.clone(),
            chunks: chapter.chunks.clone(),
            status: chapter.status.clone(),
            assigned_translators: chapter.assigned_translators.clone(),
            created_at: chapter.created_at,
            updated_at: chapter.updated_at,
            languages: chapter.content.keys().cloned().collect(),
        };

        let metadata_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| TradocumentError::Serialization(e))?;

        fs::write(&metadata_path, metadata_json)
            .map_err(|e| TradocumentError::IoError(e))?;

        Ok(())
    }

    async fn load_chapter_content(&self, chapter_id: Uuid, titles: &HashMap<String, String>) -> Result<HashMap<String, String>> {
        let mut content = HashMap::new();

        for language in titles.keys() {
            let content_path = self.get_chapter_content_path(chapter_id, language);
            
            if content_path.exists() {
                let file_content = fs::read_to_string(&content_path)
                    .map_err(|e| TradocumentError::IoError(e))?;
                content.insert(language.clone(), file_content);
            }
        }

        Ok(content)
    }

    fn get_project_path(&self, project_id: Uuid) -> PathBuf {
        self.base_path.join("chapters").join(project_id.to_string())
    }

    fn get_chapter_metadata_path(&self, chapter_id: Uuid) -> PathBuf {
        self.base_path.join("chapters").join(format!("chapter_{}_metadata.json", chapter_id))
    }

    fn get_chapter_content_path(&self, chapter_id: Uuid, language: &str) -> PathBuf {
        self.base_path.join("chapters")
            .join(language)
            .join(format!("chapter_{}.md", chapter_id))
    }

    fn extract_search_context(&self, content: &str, query: &str) -> String {
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();
        
        if let Some(pos) = content_lower.find(&query_lower) {
            let start = pos.saturating_sub(50);
            let end = (pos + query.len() + 50).min(content.len());
            let context = &content[start..end];
            
            format!("...{}...", context)
        } else {
            content.chars().take(100).collect::<String>() + "..."
        }
    }
}

/// Request to create a new chapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChapterRequest {
    pub project_id: Uuid,
    pub chapter_number: Option<u32>,
    pub title: HashMap<String, String>,
    pub slug: String,
    pub content: HashMap<String, String>,
    pub chunks: Option<Vec<ChunkMetadata>>,
}

impl CreateChapterRequest {
    pub fn validate(&self) -> Result<()> {
        if self.title.is_empty() {
            return Err(TradocumentError::ValidationError(
                "Chapter must have at least one title".to_string()
            ));
        }

        if self.slug.trim().is_empty() {
            return Err(TradocumentError::ValidationError(
                "Chapter slug cannot be empty".to_string()
            ));
        }

        // Validate slug format
        if !self.slug.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(TradocumentError::ValidationError(
                "Chapter slug can only contain alphanumeric characters, underscores, and hyphens".to_string()
            ));
        }

        Ok(())
    }
}

/// Request to update an existing chapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateChapterRequest {
    pub title: Option<HashMap<String, String>>,
    pub content: Option<HashMap<String, String>>,
    pub status: Option<ChapterStatus>,
    pub assigned_translators: Option<HashMap<String, String>>,
    pub chunks: Option<Vec<ChunkMetadata>>,
}

/// Chapter metadata for storage (without content)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChapterMetadata {
    pub id: Uuid,
    pub project_id: Uuid,
    pub chapter_number: u32,
    pub title: HashMap<String, String>,
    pub slug: String,
    pub chunks: Vec<ChunkMetadata>,
    pub status: ChapterStatus,
    pub assigned_translators: HashMap<String, String>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub languages: Vec<String>,
}

/// Summary information about a chapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSummary {
    pub id: Uuid,
    pub project_id: Uuid,
    pub chapter_number: u32,
    pub title: HashMap<String, String>,
    pub slug: String,
    pub status: ChapterStatus,
    pub languages: Vec<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

impl ChapterSummary {
    fn from_chapter(chapter: &Chapter) -> Self {
        Self {
            id: chapter.id,
            project_id: chapter.project_id,
            chapter_number: chapter.chapter_number,
            title: chapter.title.clone(),
            slug: chapter.slug.clone(),
            status: chapter.status.clone(),
            languages: chapter.content.keys().cloned().collect(),
            created_at: chapter.created_at,
            updated_at: chapter.updated_at,
        }
    }
}

/// Chapter statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterStatistics {
    pub chapter_id: Uuid,
    pub total_languages: usize,
    pub completed_languages: usize,
    pub total_chunks: usize,
    pub word_count_by_language: HashMap<String, usize>,
    pub translation_progress: HashMap<String, f32>,
    pub last_updated: chrono::DateTime<Utc>,
}

/// Search result for chapter content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterSearchResult {
    pub chapter_id: Uuid,
    pub chapter_number: u32,
    pub title: String,
    pub language: String,
    pub match_type: SearchMatchType,
    pub context: String,
}

/// Type of search match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchMatchType {
    Title,
    Content,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::collections::HashMap;

    fn create_test_service() -> (ChapterService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let service = ChapterService::new(temp_dir.path().to_path_buf());
        (service, temp_dir)
    }

    fn create_test_chapter_request() -> CreateChapterRequest {
        let mut title = HashMap::new();
        title.insert("en".to_string(), "Test Chapter".to_string());
        title.insert("es".to_string(), "Capítulo de Prueba".to_string());

        let mut content = HashMap::new();
        content.insert("en".to_string(), "# Test Chapter\n\nThis is test content.".to_string());
        content.insert("es".to_string(), "# Capítulo de Prueba\n\nEste es contenido de prueba.".to_string());

        CreateChapterRequest {
            project_id: Uuid::new_v4(),
            chapter_number: Some(1),
            title,
            slug: "test-chapter".to_string(),
            content,
            chunks: None,
        }
    }

    #[tokio::test]
    async fn test_create_chapter() {
        let (service, _temp_dir) = create_test_service();
        let request = create_test_chapter_request();

        let result = service.create_chapter(request.clone()).await;
        assert!(result.is_ok());

        let chapter = result.unwrap();
        assert_eq!(chapter.project_id, request.project_id);
        assert_eq!(chapter.chapter_number, 1);
        assert_eq!(chapter.title, request.title);
        assert_eq!(chapter.slug, request.slug);
        assert_eq!(chapter.content, request.content);
        assert_eq!(chapter.status, ChapterStatus::Draft);
    }

    #[tokio::test]
    async fn test_load_chapter() {
        let (service, _temp_dir) = create_test_service();
        let request = create_test_chapter_request();

        let created_chapter = service.create_chapter(request).await.unwrap();
        let loaded_chapter = service.load_chapter(created_chapter.id).await.unwrap();

        assert_eq!(created_chapter.id, loaded_chapter.id);
        assert_eq!(created_chapter.title, loaded_chapter.title);
        assert_eq!(created_chapter.content, loaded_chapter.content);
    }

    #[tokio::test]
    async fn test_update_chapter() {
        let (service, _temp_dir) = create_test_service();
        let request = create_test_chapter_request();

        let created_chapter = service.create_chapter(request).await.unwrap();

        let mut new_title = HashMap::new();
        new_title.insert("en".to_string(), "Updated Chapter".to_string());

        let update_request = UpdateChapterRequest {
            title: Some(new_title.clone()),
            content: None,
            status: Some(ChapterStatus::InTranslation),
            assigned_translators: None,
            chunks: None,
        };

        let updated_chapter = service.update_chapter(created_chapter.id, update_request).await.unwrap();

        assert_eq!(updated_chapter.title.get("en"), Some(&"Updated Chapter".to_string()));
        assert_eq!(updated_chapter.status, ChapterStatus::InTranslation);
        assert!(updated_chapter.updated_at > created_chapter.updated_at);
    }

    #[tokio::test]
    async fn test_list_chapters() {
        let (service, _temp_dir) = create_test_service();
        let project_id = Uuid::new_v4();

        // Create multiple chapters
        for i in 1..=3 {
            let mut request = create_test_chapter_request();
            request.project_id = project_id;
            request.chapter_number = Some(i);
            request.slug = format!("chapter-{}", i);
            service.create_chapter(request).await.unwrap();
        }

        let chapters = service.list_chapters(project_id).await.unwrap();
        assert_eq!(chapters.len(), 3);

        // Check they're sorted by chapter number
        for (i, chapter) in chapters.iter().enumerate() {
            assert_eq!(chapter.chapter_number, (i + 1) as u32);
        }
    }

    #[tokio::test]
    async fn test_delete_chapter() {
        let (service, _temp_dir) = create_test_service();
        let request = create_test_chapter_request();

        let created_chapter = service.create_chapter(request).await.unwrap();
        
        // Verify chapter exists
        assert!(service.load_chapter(created_chapter.id).await.is_ok());

        // Delete chapter
        service.delete_chapter(created_chapter.id).await.unwrap();

        // Verify chapter no longer exists
        assert!(service.load_chapter(created_chapter.id).await.is_err());
    }

    #[tokio::test]
    async fn test_chapter_statistics() {
        let (service, _temp_dir) = create_test_service();
        let request = create_test_chapter_request();

        let created_chapter = service.create_chapter(request).await.unwrap();
        let stats = service.get_chapter_statistics(created_chapter.id).await.unwrap();

        assert_eq!(stats.chapter_id, created_chapter.id);
        assert_eq!(stats.total_languages, 2);
        assert!(stats.word_count_by_language.contains_key("en"));
        assert!(stats.word_count_by_language.contains_key("es"));
        assert!(stats.translation_progress.contains_key("en"));
        assert!(stats.translation_progress.contains_key("es"));
    }

    #[tokio::test]
    async fn test_search_chapters() {
        let (service, _temp_dir) = create_test_service();
        let request = create_test_chapter_request();
        let project_id = request.project_id;

        service.create_chapter(request).await.unwrap();

        // Search in titles
        let results = service.search_chapters(project_id, "Test", None).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| matches!(r.match_type, SearchMatchType::Title)));

        // Search in content
        let results = service.search_chapters(project_id, "content", None).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| matches!(r.match_type, SearchMatchType::Content)));

        // Language-specific search
        let results = service.search_chapters(project_id, "Prueba", Some("es")).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.language == "es"));
    }

    #[test]
    fn test_create_chapter_request_validation() {
        let mut request = create_test_chapter_request();
        
        // Valid request should pass
        assert!(request.validate().is_ok());

        // Empty title should fail
        request.title.clear();
        assert!(request.validate().is_err());

        // Reset title and test empty slug
        request.title.insert("en".to_string(), "Test".to_string());
        request.slug = "".to_string();
        assert!(request.validate().is_err());

        // Invalid slug characters should fail
        request.slug = "invalid slug!".to_string();
        assert!(request.validate().is_err());

        // Valid slug should pass
        request.slug = "valid-slug_123".to_string();
        assert!(request.validate().is_ok());
    }
}