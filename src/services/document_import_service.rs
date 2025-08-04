use crate::models::translation_models::{TranslationProject, Chapter, ChunkMetadata, ChunkType, ChapterStatus};
use crate::services::translation_memory_service::TranslationMemoryService;
use crate::TradocumentError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use uuid::Uuid;

/// Represents a Word document that will be imported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportDocument {
    pub file_path: PathBuf,
    pub language_code: String,
    pub chapter_number: i32,
    pub title: String,
    pub is_source: bool,
}

/// Configuration for the document import process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportConfig {
    pub project_id: Uuid,
    pub source_language: String,
    pub target_languages: Vec<String>,
    pub auto_chunk: bool,
    pub create_translation_memory: bool,
    pub preserve_formatting: bool,
    pub extract_terminology: bool,
}

/// Result of the document import process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub imported_chapters: Vec<Chapter>,
    pub created_chunks: Vec<ChunkMetadata>,
    pub extracted_terms: Vec<String>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Mapping of languages to documents for parallel import
#[derive(Debug, Clone)]
pub struct LanguageDocumentMap {
    pub documents: HashMap<String, Vec<ImportDocument>>,
    pub source_language: String,
}

/// Service for importing Word documents and converting them to markdown chapters
pub struct DocumentImportService {
    translation_memory: TranslationMemoryService,
}

impl DocumentImportService {
    /// Create a new document import service
    pub fn new(translation_memory: TranslationMemoryService) -> Self {
        Self {
            translation_memory,
        }
    }

    /// Import multiple Word documents in parallel languages
    pub async fn import_parallel_documents(
        &mut self,
        documents: LanguageDocumentMap,
        config: ImportConfig,
    ) -> Result<ImportResult, TradocumentError> {
        let mut result = ImportResult {
            success: true,
            imported_chapters: Vec::new(),
            created_chunks: Vec::new(),
            extracted_terms: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        // Validate input documents
        self.validate_document_mapping(&documents, &mut result)?;
        if !result.errors.is_empty() {
            result.success = false;
            return Ok(result);
        }

        // Import source language documents first
        let source_docs = documents.documents.get(&config.source_language)
            .ok_or_else(|| TradocumentError::ValidationError(
                format!("No documents found for source language: {}", config.source_language)
            ))?;

        for doc in source_docs {
            match self.import_single_document(doc, &config, true).await {
                Ok(mut chapter_result) => {
                    result.imported_chapters.append(&mut chapter_result.imported_chapters);
                    result.created_chunks.append(&mut chapter_result.created_chunks);
                    result.extracted_terms.append(&mut chapter_result.extracted_terms);
                    result.warnings.append(&mut chapter_result.warnings);
                }
                Err(e) => {
                    result.errors.push(format!("Failed to import {}: {}", 
                        doc.file_path.display(), e));
                    result.success = false;
                }
            }
        }

        // Import target language documents
        for language_code in &config.target_languages {
            if let Some(target_docs) = documents.documents.get(language_code) {
                for doc in target_docs {
                    match self.import_single_document(doc, &config, false).await {
                        Ok(mut chapter_result) => {
                            result.imported_chapters.append(&mut chapter_result.imported_chapters);
                            result.created_chunks.append(&mut chapter_result.created_chunks);
                            result.warnings.append(&mut chapter_result.warnings);
                        }
                        Err(e) => {
                            result.errors.push(format!("Failed to import {}: {}", 
                                doc.file_path.display(), e));
                            // Don't fail completely for target language imports
                            result.warnings.push(format!("Target language import failed: {}", e));
                        }
                    }
                }
            } else {
                result.warnings.push(format!(
                    "No documents found for target language: {}", language_code
                ));
            }
        }

        // Create translation memory entries if requested
        if config.create_translation_memory && !result.imported_chapters.is_empty() {
            match self.create_translation_memory_from_chapters(&result.imported_chapters, &config).await {
                Ok(tm_chunks) => {
                    result.created_chunks.extend(tm_chunks);
                }
                Err(e) => {
                    result.warnings.push(format!("Failed to create translation memory: {}", e));
                }
            }
        }

        Ok(result)
    }

    /// Import a single Word document and convert to markdown chapter
    async fn import_single_document(
        &mut self,
        document: &ImportDocument,
        config: &ImportConfig,
        is_source: bool,
    ) -> Result<ImportResult, TradocumentError> {
        let mut result = ImportResult {
            success: true,
            imported_chapters: Vec::new(),
            created_chunks: Vec::new(),
            extracted_terms: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        // Convert Word document to markdown
        let markdown_content = self.convert_word_to_markdown(&document.file_path, config).await?;
        
        // Create chapter
        let mut title_map = HashMap::new();
        title_map.insert(document.language_code.clone(), document.title.clone());
        
        let mut content_map = HashMap::new();
        content_map.insert(document.language_code.clone(), markdown_content.clone());
        
        let chapter = Chapter {
            id: Uuid::new_v4(),
            project_id: config.project_id,
            chapter_number: document.chapter_number as u32,
            title: title_map,
            slug: self.generate_slug(&document.title),
            content: content_map,
            chunks: Vec::new(),
            status: if is_source { 
                ChapterStatus::Draft 
            } else { 
                ChapterStatus::InTranslation 
            },
            assigned_translators: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        result.imported_chapters.push(chapter);

        // Auto-chunk content if requested
        if config.auto_chunk {
            let chunks = self.create_chunks_from_content(
                &markdown_content,
                &document.language_code,
                document.chapter_number,
                is_source,
            ).await?;
            result.created_chunks.extend(chunks);
        }

        // Extract terminology if requested
        if config.extract_terminology && is_source {
            let terms = self.extract_terminology_from_content(&markdown_content).await?;
            result.extracted_terms.extend(terms);
        }

        Ok(result)
    }

    /// Convert Word document to markdown format
    async fn convert_word_to_markdown(
        &self,
        file_path: &Path,
        config: &ImportConfig,
    ) -> Result<String, TradocumentError> {
        // Check if file exists
        if !file_path.exists() {
            return Err(TradocumentError::FileError(
                format!("Document not found: {}", file_path.display())
            ));
        }

        // For now, implement a basic text extraction
        // In a real implementation, you would use a library like:
        // - docx crate for parsing Word documents
        // - pandoc for conversion
        // - python-docx via PyO3
        
        let file_extension = file_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        match file_extension.to_lowercase().as_str() {
            "docx" => self.convert_docx_to_markdown(file_path, config).await,
            "doc" => self.convert_doc_to_markdown(file_path, config).await,
            "txt" => self.convert_txt_to_markdown(file_path).await,
            "md" => {
                // Already markdown, just read it
                fs::read_to_string(file_path)
                    .map_err(|e| TradocumentError::FileError(format!("Failed to read markdown file: {}", e)))
            }
            _ => Err(TradocumentError::ValidationError(
                format!("Unsupported file format: {}", file_extension)
            )),
        }
    }

    /// Convert DOCX file to markdown (placeholder implementation)
    async fn convert_docx_to_markdown(
        &self,
        file_path: &Path,
        config: &ImportConfig,
    ) -> Result<String, TradocumentError> {
        // This is a placeholder implementation
        // In a real system, you would use a proper DOCX parser
        
        // For demonstration, create a basic markdown structure
        let filename = file_path.file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled");

        let mut markdown = format!("# {}\n\n", filename);
        
        // Add placeholder content structure
        markdown.push_str("## Introduction\n\n");
        markdown.push_str("This document was imported from a Word file.\n\n");
        markdown.push_str("## Content\n\n");
        markdown.push_str("The actual content would be extracted from the DOCX file using a proper parser.\n\n");
        
        if config.preserve_formatting {
            markdown.push_str("*Formatting preservation enabled*\n\n");
        }
        
        markdown.push_str("## Implementation Note\n\n");
        markdown.push_str("To fully implement DOCX conversion, consider using:\n");
        markdown.push_str("- docx crate for Rust\n");
        markdown.push_str("- pandoc for universal document conversion\n");
        markdown.push_str("- python-docx via PyO3 bindings\n\n");

        Ok(markdown)
    }

    /// Convert DOC file to markdown (placeholder implementation)
    async fn convert_doc_to_markdown(
        &self,
        file_path: &Path,
        config: &ImportConfig,
    ) -> Result<String, TradocumentError> {
        // Legacy DOC format is more complex to parse
        // Typically requires LibreOffice or similar for conversion
        let filename = file_path.file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled");

        let mut markdown = format!("# {}\n\n", filename);
        markdown.push_str("## Legacy DOC Import\n\n");
        markdown.push_str("This document was imported from a legacy DOC file.\n\n");
        markdown.push_str("**Note**: Full DOC support requires LibreOffice or similar conversion tools.\n\n");

        Ok(markdown)
    }

    /// Convert text file to markdown
    async fn convert_txt_to_markdown(&self, file_path: &Path) -> Result<String, TradocumentError> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| TradocumentError::FileError(format!("Failed to read text file: {}", e)))?;

        let filename = file_path.file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled");

        let mut markdown = format!("# {}\n\n", filename);
        
        // Convert plain text to markdown paragraphs
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                markdown.push_str(trimmed);
                markdown.push_str("\n\n");
            }
        }

        Ok(markdown)
    }

    /// Create chunks from markdown content
    async fn create_chunks_from_content(
        &mut self,
        content: &str,
        language_code: &str,
        chapter_number: i32,
        is_source: bool,
    ) -> Result<Vec<ChunkMetadata>, TradocumentError> {
        let mut chunks = Vec::new();
        let mut chunk_index = 0;

        // Split content into sentences/paragraphs for chunking
        let sentences = self.split_content_into_sentences(content);

        for sentence in sentences {
            if sentence.trim().is_empty() {
                continue;
            }

            let chunk_id = Uuid::new_v4();
            let chunk = ChunkMetadata {
                id: chunk_id,
                original_position: chunk_index,
                sentence_boundaries: vec![0, sentence.len()],
                linked_chunks: Vec::new(),
                chunk_type: self.determine_chunk_type(&sentence),
                processing_notes: vec![
                    format!("Chapter: {}", chapter_number),
                    format!("Language: {}", language_code),
                    format!("Source: {}", is_source),
                    format!("Content: {}", sentence.clone()),
                ],
            };

            chunks.push(chunk);
            chunk_index += 1;
        }

        Ok(chunks)
    }

    /// Split content into sentences for chunking
    fn split_content_into_sentences(&self, content: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        
        // Simple sentence splitting (in a real implementation, use a proper NLP library)
        for paragraph in content.split("\n\n") {
            let paragraph = paragraph.trim();
            if paragraph.is_empty() {
                continue;
            }

            // Split on sentence boundaries
            for sentence in paragraph.split('.') {
                let sentence = sentence.trim();
                if !sentence.is_empty() && sentence.len() > 10 {
                    sentences.push(format!("{}.", sentence));
                }
            }
        }

        sentences
    }

    /// Determine the type of content chunk
    fn determine_chunk_type(&self, content: &str) -> ChunkType {
        let content_lower = content.to_lowercase();
        
        if content_lower.starts_with('#') {
            ChunkType::Heading
        } else if content_lower.contains("```") || content_lower.contains("`") {
            ChunkType::Code
        } else if content_lower.contains("http") || content_lower.contains("www") {
            ChunkType::Link
        } else {
            ChunkType::Paragraph
        }
    }

    /// Extract terminology from content
    async fn extract_terminology_from_content(
        &self,
        content: &str,
    ) -> Result<Vec<String>, TradocumentError> {
        let mut terms = Vec::new();

        // Simple terminology extraction (in a real implementation, use NLP)
        let words: Vec<&str> = content.split_whitespace().collect();
        
        for word in words {
            let cleaned = word.trim_matches(|c: char| !c.is_alphabetic()).to_lowercase();
            
            // Extract potential technical terms (capitalized, long words, etc.)
            if cleaned.len() > 6 && !self.is_common_word(&cleaned) {
                if !terms.contains(&cleaned) {
                    terms.push(cleaned);
                }
            }
        }

        Ok(terms)
    }

    /// Check if a word is a common word (not terminology)
    fn is_common_word(&self, word: &str) -> bool {
        let common_words = [
            "the", "and", "for", "are", "but", "not", "you", "all", "can", "had", 
            "her", "was", "one", "our", "out", "day", "get", "has", "him", "his",
            "how", "its", "may", "new", "now", "old", "see", "two", "who", "boy",
            "did", "does", "each", "from", "have", "into", "like", "more", "over",
            "such", "take", "than", "that", "this", "very", "want", "well", "were",
        ];
        
        common_words.contains(&word)
    }

    /// Create translation memory from imported chapters
    async fn create_translation_memory_from_chapters(
        &mut self,
        chapters: &[Chapter],
        config: &ImportConfig,
    ) -> Result<Vec<ChunkMetadata>, TradocumentError> {
        let mut tm_chunks = Vec::new();

        // Group chapters by source and target language pairs
        for chapter in chapters {
            if let Some(source_content) = chapter.content.get(&config.source_language) {
                if !source_content.is_empty() {
                    // Process source content
                    let source_chunks = self.create_chunks_from_content(
                        source_content,
                        &config.source_language,
                        chapter.chapter_number as i32,
                        true,
                    ).await?;

                    tm_chunks.extend(source_chunks);
                }
            }

            // Process translations
            for (lang_code, translation) in &chapter.content {
                if lang_code != &config.source_language {
                    let target_chunks = self.create_chunks_from_content(
                        translation,
                        lang_code,
                        chapter.chapter_number as i32,
                        false,
                    ).await?;

                    tm_chunks.extend(target_chunks);
                }
            }
        }

        // Store in translation memory service
        if let Err(e) = self.translation_memory.add_chunks_batch(tm_chunks.clone()).await {
            // Log warning but don't fail
            eprintln!("Warning: Failed to add chunks to translation memory: {}", e);
        }

        Ok(tm_chunks)
    }

    /// Validate the document mapping for consistency
    fn validate_document_mapping(
        &self,
        documents: &LanguageDocumentMap,
        result: &mut ImportResult,
    ) -> Result<(), TradocumentError> {
        // Check that source language has documents
        if !documents.documents.contains_key(&documents.source_language) {
            result.errors.push(format!(
                "No documents found for source language: {}", 
                documents.source_language
            ));
        }

        // Check that all document files exist
        for (lang_code, docs) in &documents.documents {
            for doc in docs {
                if !doc.file_path.exists() {
                    result.errors.push(format!(
                        "Document not found for {}: {}", 
                        lang_code, 
                        doc.file_path.display()
                    ));
                }
            }
        }

        // Check for consistent chapter numbering across languages
        if let Some(source_docs) = documents.documents.get(&documents.source_language) {
            let source_chapters: Vec<i32> = source_docs.iter()
                .map(|doc| doc.chapter_number)
                .collect();

            for (lang_code, docs) in &documents.documents {
                if lang_code == &documents.source_language {
                    continue;
                }

                let target_chapters: Vec<i32> = docs.iter()
                    .map(|doc| doc.chapter_number)
                    .collect();

                for &chapter_num in &source_chapters {
                    if !target_chapters.contains(&chapter_num) {
                        result.warnings.push(format!(
                            "Chapter {} missing in {} language", 
                            chapter_num, 
                            lang_code
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Generate a URL-safe slug from a title
    fn generate_slug(&self, title: &str) -> String {
        title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join("-")
    }

    /// Create language document mapping from folder scan
    pub fn scan_folder_for_documents(
        folder_path: &Path,
        language_mappings: HashMap<String, String>, // filename pattern -> language code
    ) -> Result<LanguageDocumentMap, TradocumentError> {
        let mut documents: HashMap<String, Vec<ImportDocument>> = HashMap::new();
        
        if !folder_path.exists() || !folder_path.is_dir() {
            return Err(TradocumentError::FileError(
                format!("Invalid folder path: {}", folder_path.display())
            ));
        }

        // Scan directory for supported document files
        let entries = fs::read_dir(folder_path)
            .map_err(|e| TradocumentError::FileError(format!("Failed to read directory: {}", e)))?;

        let mut chapter_counter = 1;

        for entry in entries {
            let entry = entry.map_err(|e| TradocumentError::FileError(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();

            if path.is_file() {
                if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
                    if matches!(extension.to_lowercase().as_str(), "docx" | "doc" | "txt" | "md") {
                        // Determine language based on filename pattern
                        let filename = path.file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or("");

                        let language_code = Self::detect_language_from_filename(filename, &language_mappings)
                            .unwrap_or_else(|| "unknown".to_string());

                        let title = path.file_stem()
                            .and_then(|stem| stem.to_str())
                            .unwrap_or("Untitled")
                            .to_string();

                        let import_doc = ImportDocument {
                            file_path: path,
                            language_code: language_code.clone(),
                            chapter_number: chapter_counter,
                            title,
                            is_source: false, // Will be set later based on source language
                        };

                        documents.entry(language_code)
                            .or_insert_with(Vec::new)
                            .push(import_doc);

                        chapter_counter += 1;
                    }
                }
            }
        }

        // Use the first language found as source (this should be configurable)
        let source_language = documents.keys().next()
            .ok_or_else(|| TradocumentError::ValidationError("No documents found".to_string()))?
            .clone();

        Ok(LanguageDocumentMap {
            documents,
            source_language,
        })
    }

    /// Detect language from filename patterns
    fn detect_language_from_filename(
        filename: &str,
        language_mappings: &HashMap<String, String>,
    ) -> Option<String> {
        let filename_lower = filename.to_lowercase();

        // Check for explicit language mappings first
        for (pattern, lang_code) in language_mappings {
            if filename_lower.contains(&pattern.to_lowercase()) {
                return Some(lang_code.clone());
            }
        }

        // Check for common language indicators in filename
        if filename_lower.contains("_en") || filename_lower.contains("-en") {
            Some("en".to_string())
        } else if filename_lower.contains("_es") || filename_lower.contains("-es") {
            Some("es".to_string())
        } else if filename_lower.contains("_fr") || filename_lower.contains("-fr") {
            Some("fr".to_string())
        } else if filename_lower.contains("_de") || filename_lower.contains("-de") {
            Some("de".to_string())
        } else if filename_lower.contains("_it") || filename_lower.contains("-it") {
            Some("it".to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

    #[tokio::test]
    async fn test_convert_txt_to_markdown() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "This is a test document.").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "It has multiple paragraphs.").unwrap();

        let tm_service = TranslationMemoryService::new("memory://test".to_string()).await.unwrap();
        let import_service = DocumentImportService::new(tm_service);
        
        let result = import_service.convert_txt_to_markdown(&file_path).await.unwrap();
        
        assert!(result.contains("# test"));
        assert!(result.contains("This is a test document."));
        assert!(result.contains("It has multiple paragraphs."));
    }

    #[test]
    fn test_generate_slug() {
        let tm_service = TranslationMemoryService::new("memory://test".to_string()).await.unwrap();
        let import_service = DocumentImportService::new(tm_service);
        
        assert_eq!(import_service.generate_slug("Hello World"), "hello-world");
        assert_eq!(import_service.generate_slug("API Reference Guide"), "api-reference-guide");
        assert_eq!(import_service.generate_slug("Chapter 1: Introduction"), "chapter-1-introduction");
    }

    #[test]
    fn test_detect_language_from_filename() {
        let mappings = HashMap::new();
        
        assert_eq!(
            DocumentImportService::detect_language_from_filename("document_en.docx", &mappings),
            Some("en".to_string())
        );
        assert_eq!(
            DocumentImportService::detect_language_from_filename("document-es.docx", &mappings),
            Some("es".to_string())
        );
        assert_eq!(
            DocumentImportService::detect_language_from_filename("document.docx", &mappings),
            None
        );
    }
}