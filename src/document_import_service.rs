use crate::{Document, DocumentStatus, DocumentImportRequest, DocumentImportResult, TradocumentError, Result};
use crate::models::translation_models::{Chapter, ChunkMetadata, ChunkType};
use crate::services::chapter_service::{ChapterService, CreateChapterRequest};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::sync::Arc;
use tempfile::NamedTempFile;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use tokio::sync::Mutex;

pub struct DocumentImportService {
    chapter_service: Option<Arc<ChapterService>>,
}

impl DocumentImportService {
    pub fn new() -> Self {
        Self {
            chapter_service: None,
        }
    }

    pub fn with_chapter_service(chapter_service: Arc<ChapterService>) -> Self {
        Self {
            chapter_service: Some(chapter_service),
        }
    }

    /// Import a DOCX file and convert it to a Document
    pub async fn import_docx_file<P: AsRef<Path>>(
        &mut self,
        file_path: P,
        import_request: DocumentImportRequest,
    ) -> Result<(Document, DocumentImportResult)> {
        let start_time = Instant::now();
        let mut messages = Vec::new();
        let mut warnings = Vec::new();
        let extracted_images = Vec::new();

        // Validate file exists and has correct extension
        let path = file_path.as_ref();
        if !path.exists() {
            return Err(TradocumentError::DocumentImport(
                "File does not exist".to_string()
            ));
        }

        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        if !matches!(extension.to_lowercase().as_str(), "docx" | "doc") {
            return Err(TradocumentError::UnsupportedFormat(
                format!("Unsupported file extension: {extension}")
            ));
        }

        messages.push("Starting DOCX file processing...".to_string());

        // Read the DOCX file
        let mut file = File::open(path)
            .map_err(|e| TradocumentError::DocumentImport(format!("Failed to open file: {e}")))?;
        
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| TradocumentError::DocumentImport(format!("Failed to read file: {e}")))?;

        messages.push("File read successfully".to_string());

        // Convert DOCX to markdown using markdownify
        let markdown_content = self.convert_docx_to_markdown_legacy(&buffer, &import_request, &mut warnings)?;
        
        messages.push("DOCX converted to markdown".to_string());

        // Create the document with multilingual support
        let document = self.create_multilingual_document(
            import_request,
            markdown_content,
            &mut messages,
        )?;

        let processing_time = start_time.elapsed().as_millis() as u64;
        messages.push(format!("Processing completed in {processing_time}ms"));

        let result = DocumentImportResult {
            document_id: document.id,
            success: true,
            messages,
            warnings,
            extracted_images,
            processing_time_ms: processing_time,
        };

        Ok((document, result))
    }

    /// Import from bytes (for API upload)
    pub async fn import_docx_bytes(
        &mut self,
        bytes: &[u8],
        filename: String,
        import_request: DocumentImportRequest,
    ) -> Result<(Document, DocumentImportResult)> {
        let start_time = Instant::now();
        let mut messages = Vec::new();
        let mut warnings = Vec::new();
        let extracted_images = Vec::new();

        // Validate file extension
        let extension = Path::new(&filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        if !matches!(extension.to_lowercase().as_str(), "docx" | "doc") {
            return Err(TradocumentError::UnsupportedFormat(
                format!("Unsupported file extension: {extension}")
            ));
        }

        messages.push(format!("Starting processing of uploaded file: {filename}"));

        // Convert DOCX to markdown using markdownify
        let markdown_content = self.convert_docx_to_markdown_legacy(bytes, &import_request, &mut warnings)?;
        
        messages.push("DOCX converted to markdown".to_string());

        // Create the document with multilingual support
        let document = self.create_multilingual_document(
            import_request,
            markdown_content,
            &mut messages,
        )?;

        let processing_time = start_time.elapsed().as_millis() as u64;
        messages.push(format!("Processing completed in {processing_time}ms"));

        let result = DocumentImportResult {
            document_id: document.id,
            success: true,
            messages,
            warnings,
            extracted_images,
            processing_time_ms: processing_time,
        };

        Ok((document, result))
    }

    fn convert_docx_to_markdown_legacy(
        &self,
        docx_bytes: &[u8],
        _import_request: &DocumentImportRequest,
        warnings: &mut Vec<String>,
    ) -> Result<String> {
        // Create a temporary file to work with the markdownify library
        let mut temp_file = NamedTempFile::new()
            .map_err(|e| TradocumentError::DocumentImport(format!("Failed to create temp file: {e}")))?;
        
        // Write the DOCX bytes to the temporary file
        temp_file.write_all(docx_bytes)
            .map_err(|e| TradocumentError::DocumentImport(format!("Failed to write to temp file: {e}")))?;
        
        // Get the path to the temporary file
        let temp_path = temp_file.path();
        
        // Use markdownify to convert DOCX to markdown
        match markdownify::convert(temp_path, None) {
            Ok(markdown) => {
                if markdown.trim().is_empty() {
                    warnings.push("Document appears to be empty or contains no convertible content".to_string());
                    Ok("# Imported Document\n\n*No content could be extracted from the document.*".to_string())
                } else {
                    Ok(markdown)
                }
            }
            Err(e) => {
                // Try to provide more specific error information
                if e.to_string().contains("empty") || e.to_string().contains("no content") {
                    warnings.push("Document appears to be empty".to_string());
                    Ok("# Imported Document\n\n*Document appears to be empty or contains no convertible content.*".to_string())
                } else {
                    Err(TradocumentError::DocumentImport(
                        format!("Failed to convert DOCX to markdown: {e}")
                    ))
                }
            }
        }
    }

    fn create_multilingual_document(
        &self,
        import_request: DocumentImportRequest,
        source_markdown: String,
        messages: &mut Vec<String>,
    ) -> Result<Document> {
        let mut content = HashMap::new();
        
        // Add the source content in the source language
        content.insert(import_request.source_language.clone(), source_markdown.clone());
        
        // Initialize placeholder content for target languages
        for lang in &import_request.target_languages {
            if lang != &import_request.source_language {
                content.insert(
                    lang.clone(),
                    format!(
                        "# {} ({})\n\n*This document needs to be translated from {}.*\n\n---\n\n{}",
                        import_request.title,
                        lang.to_uppercase(),
                        import_request.source_language.to_uppercase(),
                        source_markdown
                    )
                );
                messages.push(format!("Created placeholder content for language: {lang}"));
            }
        }

        let mut all_languages = import_request.target_languages.clone();
        if !all_languages.contains(&import_request.source_language) {
            all_languages.push(import_request.source_language.clone());
        }

        let document = Document {
            id: Uuid::new_v4(),
            title: import_request.title,
            content,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
            status: DocumentStatus::Draft,
            metadata: crate::DocumentMetadata {
                languages: all_languages,
                tags: vec!["imported".to_string(), "word-document".to_string()],
                project_id: None,
                screenshots: Vec::new(),
            },
        };

        messages.push(format!("Created document with ID: {}", document.id));
        
        Ok(document)
    }

    /// Import multiple Word documents with language mapping
    pub async fn import_multi_language_documents(
        &mut self,
        files: Vec<DocumentFile>,
        language_mapping: HashMap<String, String>,
        project_id: Uuid,
        progress_callback: Option<Arc<Mutex<dyn Fn(ImportProgress) + Send + Sync>>>,
    ) -> Result<MultiDocumentImportResult> {
        let start_time = Instant::now();
        let mut results = Vec::new();
        let mut errors = Vec::new();
        let warnings = Vec::new();
        let total_files = files.len();

        // Validate all files before processing
        for (index, file) in files.iter().enumerate() {
            if let Some(callback) = &progress_callback {
                let progress = ImportProgress {
                    step: format!("Validating file {} of {}", index + 1, total_files),
                    progress_percent: ((index as f32 / total_files as f32) * 10.0) as u8,
                    message: format!("Validating {}", file.filename),
                };
                callback.lock().await(progress);
            }

            if !Self::is_format_supported(&file.filename) {
                errors.push(ImportError {
                    filename: file.filename.clone(),
                    error: format!("Unsupported file format: {}", file.filename),
                });
                continue;
            }

            if !language_mapping.contains_key(&file.filename) {
                errors.push(ImportError {
                    filename: file.filename.clone(),
                    error: "No language mapping provided for file".to_string(),
                });
            }
        }

        // Process each file
        for (index, file) in files.into_iter().enumerate() {
            if let Some(callback) = &progress_callback {
                let progress = ImportProgress {
                    step: format!("Processing file {} of {}", index + 1, total_files),
                    progress_percent: (10.0 + ((index as f32 / total_files as f32) * 80.0)) as u8,
                    message: format!("Converting {}", file.filename),
                };
                callback.lock().await(progress);
            }

            // Skip files that failed validation
            if errors.iter().any(|e| e.filename == file.filename) {
                continue;
            }

            let language = language_mapping.get(&file.filename).unwrap().clone();
            
            let filename = file.filename.clone();
            match self.process_single_document(file, language, project_id).await {
                Ok(result) => {
                    results.push(result);
                }
                Err(e) => {
                    errors.push(ImportError {
                        filename,
                        error: e.to_string(),
                    });
                }
            }
        }

        // Final progress update
        if let Some(callback) = &progress_callback {
            let progress = ImportProgress {
                step: "Import completed".to_string(),
                progress_percent: 100,
                message: format!("Processed {} files with {} errors", results.len(), errors.len()),
            };
            callback.lock().await(progress);
        }

        let processing_time = start_time.elapsed().as_millis() as u64;

        Ok(MultiDocumentImportResult {
            project_id,
            successful_imports: results,
            errors,
            warnings,
            total_files,
            processing_time_ms: processing_time,
        })
    }

    /// Process a single document file
    async fn process_single_document(
        &mut self,
        file: DocumentFile,
        language: String,
        project_id: Uuid,
    ) -> Result<SingleDocumentImportResult> {
        let start_time = Instant::now();
        let mut messages = Vec::new();
        let mut warnings = Vec::new();

        messages.push(format!("Starting processing of {} for language {}", file.filename, language));

        // Convert document content
        let markdown_content = match file.content {
            DocumentContent::Bytes(bytes) => {
                self.convert_docx_to_markdown(&bytes, &file.filename, &mut warnings)?
            }
            DocumentContent::FilePath(path) => {
                let mut file_handle = File::open(&path)
                    .map_err(|e| TradocumentError::DocumentImport(format!("Failed to open file: {e}")))?;
                
                let mut buffer = Vec::new();
                file_handle.read_to_end(&mut buffer)
                    .map_err(|e| TradocumentError::DocumentImport(format!("Failed to read file: {e}")))?;
                
                self.convert_docx_to_markdown(&buffer, &file.filename, &mut warnings)?
            }
        };

        messages.push("Document converted to markdown".to_string());

        // Create chapter from document
        let chapter = self.create_chapter_from_document(
            project_id,
            file.filename.clone(),
            language.clone(),
            markdown_content,
            &mut messages,
        ).await?;

        let processing_time = start_time.elapsed().as_millis() as u64;
        messages.push(format!("Processing completed in {processing_time}ms"));

        Ok(SingleDocumentImportResult {
            filename: file.filename,
            language,
            chapter_id: chapter.id,
            success: true,
            messages,
            warnings,
            processing_time_ms: processing_time,
        })
    }

    /// Create a chapter from imported document content
    async fn create_chapter_from_document(
        &mut self,
        project_id: Uuid,
        filename: String,
        language: String,
        markdown_content: String,
        messages: &mut Vec<String>,
    ) -> Result<Chapter> {
        // Extract title from filename (remove extension)
        let title = Path::new(&filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&filename)
            .to_string();

        // Create slug from title
        let slug = title
            .to_lowercase()
            .replace(' ', "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>();

        // Process content into chunks - simplified for now
        let chunks = Vec::new();

        // Create chapter with multi-language support
        let mut chapter_title = HashMap::new();
        chapter_title.insert(language.clone(), title.clone());

        let mut chapter_content = HashMap::new();
        chapter_content.insert(language.clone(), markdown_content);

        // Use chapter service if available, otherwise create chapter directly
        let chapter = if let Some(chapter_service) = &self.chapter_service {
            let request = CreateChapterRequest {
                project_id,
                chapter_number: None, // Let service determine the number
                title: chapter_title,
                slug,
                content: chapter_content,
                chunks: Some(chunks),
            };

            chapter_service.create_chapter(request).await?
        } else {
            // Fallback to direct chapter creation
            Chapter {
                id: Uuid::new_v4(),
                project_id,
                chapter_number: 1, // This should be determined by existing chapters
                title: chapter_title,
                slug,
                content: chapter_content,
                chunks,
                status: crate::models::translation_models::ChapterStatus::Draft,
                assigned_translators: HashMap::new(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }
        };

        messages.push(format!("Created chapter with ID: {}", chapter.id));
        
        Ok(chapter)
    }

    /// Process markdown content into chunks for translation memory
    async fn process_content_into_chunks(
        &self,
        content: &str,
        messages: &mut Vec<String>,
    ) -> Result<Vec<ChunkMetadata>> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut position = 0;

        for (line_index, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let chunk_type = if trimmed.starts_with('#') {
                ChunkType::Heading
            } else if trimmed.starts_with('-') || trimmed.starts_with('*') || trimmed.starts_with('+') {
                ChunkType::ListItem
            } else if trimmed.starts_with("```") {
                ChunkType::CodeBlock
            } else if trimmed.contains('|') && lines.get(line_index + 1).is_some_and(|next| next.contains('|')) {
                ChunkType::Table
            } else {
                // Split paragraphs into sentences for better translation memory
                if trimmed.len() > 100 {
                    ChunkType::Sentence
                } else {
                    ChunkType::Paragraph
                }
            };

            // Create sentence boundaries for sentence-level chunks
            let sentence_boundaries = if chunk_type == ChunkType::Sentence {
                self.detect_sentence_boundaries(trimmed)
            } else {
                vec![0, trimmed.len()]
            };

            let chunk_metadata = ChunkMetadata::new(
                position,
                sentence_boundaries,
                chunk_type,
            ).map_err(|e| TradocumentError::Validation(e.to_string()))?;

            chunks.push(chunk_metadata);
            position += 1;
        }

        messages.push(format!("Created {} chunks from content", chunks.len()));
        Ok(chunks)
    }

    /// Detect sentence boundaries in text
    fn detect_sentence_boundaries(&self, text: &str) -> Vec<usize> {
        let mut boundaries = vec![0];
        let chars: Vec<char> = text.chars().collect();
        
        for (i, &ch) in chars.iter().enumerate() {
            if matches!(ch, '.' | '!' | '?') {
                // Check if this is likely the end of a sentence
                if i + 1 < chars.len() {
                    let next_char = chars[i + 1];
                    if next_char.is_whitespace() {
                        // Look ahead to see if next non-whitespace is uppercase
                        let mut j = i + 1;
                        while j < chars.len() && chars[j].is_whitespace() {
                            j += 1;
                        }
                        if j < chars.len() && chars[j].is_uppercase() {
                            boundaries.push(i + 1);
                        }
                    }
                }
            }
        }
        
        if boundaries.last() != Some(&text.len()) {
            boundaries.push(text.len());
        }
        
        boundaries
    }

    /// Enhanced DOCX to markdown conversion with better formatting preservation
    fn convert_docx_to_markdown(
        &self,
        docx_bytes: &[u8],
        filename: &str,
        warnings: &mut Vec<String>,
    ) -> Result<String> {
        // Extract document metadata first
        let metadata = self.extract_document_metadata(docx_bytes, filename, warnings)?;
        
        // Create a temporary file to work with the markdownify library
        let mut temp_file = NamedTempFile::new()
            .map_err(|e| TradocumentError::DocumentImport(format!("Failed to create temp file: {e}")))?;
        
        // Write the DOCX bytes to the temporary file
        temp_file.write_all(docx_bytes)
            .map_err(|e| TradocumentError::DocumentImport(format!("Failed to write to temp file: {e}")))?;
        
        // Get the path to the temporary file
        let temp_path = temp_file.path();
        
        // Use markdownify to convert DOCX to markdown
        // Note: markdownify doesn't have enhanced options, so we'll post-process
        match markdownify::convert(temp_path, None) {
            Ok(markdown) => {
                if markdown.trim().is_empty() {
                    warnings.push(format!("Document {filename} appears to be empty or contains no convertible content"));
                    Ok(format!("# {filename}\n\n*No content could be extracted from the document.*"))
                } else {
                    // Enhanced post-processing with metadata
                    let processed_markdown = self.enhanced_post_process_markdown(markdown, &metadata, warnings);
                    Ok(processed_markdown)
                }
            }
            Err(e) => {
                // Fallback to basic conversion if enhanced conversion fails
                warnings.push(format!("Enhanced conversion failed for {filename}, falling back to basic conversion: {e}"));
                self.fallback_docx_conversion(temp_path, filename, warnings)
            }
        }
    }

    /// Extract metadata from DOCX document
    fn extract_document_metadata(
        &self,
        docx_bytes: &[u8],
        filename: &str,
        warnings: &mut Vec<String>,
    ) -> Result<DocumentMetadata> {
        // This is a simplified metadata extraction
        // In a real implementation, you would parse the DOCX structure directly
        let mut metadata = DocumentMetadata {
            title: None,
            author: None,
            created_date: None,
            modified_date: None,
            word_count: 0,
            page_count: 0,
            has_tables: false,
            has_images: false,
            chapter_count: 0,
            languages: Vec::new(),
        };

        // Try to extract basic metadata by examining the document structure
        // This is a placeholder implementation - real implementation would use
        // a DOCX parsing library like docx-rs or similar
        
        // Estimate word count from file size (rough approximation)
        metadata.word_count = (docx_bytes.len() / 6).max(1); // Rough estimate
        
        // Set default values
        metadata.title = Some(filename.replace(".docx", "").replace(".doc", ""));
        metadata.author = Some("Unknown".to_string());
        
        // Check for common table and image indicators in the raw bytes
        let content_str = String::from_utf8_lossy(docx_bytes);
        metadata.has_tables = content_str.contains("<w:tbl>") || content_str.contains("table");
        metadata.has_images = content_str.contains("<w:drawing>") || content_str.contains("image") || content_str.contains("picture");
        
        // Detect potential chapters by looking for heading patterns
        let heading_count = content_str.matches("<w:pStyle w:val=\"Heading").count();
        metadata.chapter_count = heading_count.max(1);
        
        if metadata.has_tables {
            warnings.push("Document contains tables - formatting may need manual review".to_string());
        }
        
        if metadata.has_images {
            warnings.push("Document contains images - images will be referenced but not embedded".to_string());
        }
        
        Ok(metadata)
    }

    /// Fallback conversion method using basic markdownify
    fn fallback_docx_conversion(
        &self,
        temp_path: &Path,
        filename: &str,
        warnings: &mut Vec<String>,
    ) -> Result<String> {
        match markdownify::convert(temp_path, None) {
            Ok(markdown) => {
                if markdown.trim().is_empty() {
                    warnings.push(format!("Document {filename} appears to be empty"));
                    Ok(format!("# {filename}\n\n*Document appears to be empty or contains no convertible content.*"))
                } else {
                    // Basic post-processing
                    let processed_markdown = self.post_process_markdown(markdown, warnings);
                    Ok(processed_markdown)
                }
            }
            Err(e) => {
                Err(TradocumentError::DocumentImport(
                    format!("Failed to convert {filename} to markdown: {e}")
                ))
            }
        }
    }

    /// Enhanced post-processing with metadata awareness
    fn enhanced_post_process_markdown(
        &self,
        markdown: String,
        metadata: &DocumentMetadata,
        warnings: &mut Vec<String>,
    ) -> String {
        let mut processed = markdown;
        
        // Add document metadata header if available
        if let Some(title) = &metadata.title {
            if !processed.starts_with(&format!("# {title}")) {
                processed = format!("# {title}\n\n{processed}");
            }
        }
        
        // Clean up excessive whitespace
        processed = processed
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n");
        
        // Enhanced heading normalization with chapter detection
        processed = self.normalize_headings_with_chapters(processed, warnings);
        
        // Enhanced table formatting
        if metadata.has_tables {
            processed = self.enhance_table_formatting(processed, warnings);
        }
        
        // Enhanced list formatting
        processed = self.improve_list_formatting(processed);
        
        // Process image references
        if metadata.has_images {
            processed = self.process_image_references(processed, warnings);
        }
        
        // Add document structure comments for complex documents
        if metadata.chapter_count > 3 {
            processed = self.add_structure_comments(processed, metadata);
        }
        
        processed
    }

    /// Normalize headings with chapter detection
    fn normalize_headings_with_chapters(&self, content: String, warnings: &mut Vec<String>) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut processed_lines = Vec::new();
        let mut heading_levels = Vec::new();
        let mut chapter_count = 0;
        
        for line in lines {
            if line.trim().starts_with('#') {
                let level = line.chars().take_while(|&c| c == '#').count();
                let heading_text = line.trim_start_matches('#').trim();
                
                // Detect chapter headings (level 1 or specific keywords)
                if level == 1 || self.is_chapter_heading(heading_text) {
                    chapter_count += 1;
                    processed_lines.push(format!("# Chapter {chapter_count}: {heading_text}"));
                } else {
                    heading_levels.push(level);
                    
                    // Ensure heading hierarchy is logical
                    let normalized_level = if heading_levels.len() == 1 {
                        2 // Sub-headings start at level 2 after chapter headings
                    } else {
                        let prev_level = heading_levels[heading_levels.len() - 2];
                        if level > prev_level + 1 {
                            warnings.push("Heading hierarchy normalized - skipped levels detected".to_string());
                            prev_level + 1
                        } else {
                            level.min(6) // Maximum heading level
                        }
                    };
                    
                    processed_lines.push(format!("{} {}", "#".repeat(normalized_level), heading_text));
                }
            } else {
                processed_lines.push(line.to_string());
            }
        }
        
        processed_lines.join("\n")
    }

    /// Check if a heading text indicates a chapter
    fn is_chapter_heading(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();
        text_lower.contains("chapter") ||
        text_lower.contains("section") ||
        text_lower.starts_with("part ") ||
        text_lower.matches(char::is_numeric).count() > 0 && text_lower.len() < 50
    }

    /// Enhanced table formatting with better structure detection
    fn enhance_table_formatting(&self, content: String, warnings: &mut Vec<String>) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut processed_lines = Vec::new();
        let mut in_table = false;
        let mut table_rows = Vec::new();
        
        for line in lines {
            if line.contains('|') && !line.trim().starts_with('|') {
                // Potential table row
                if !in_table {
                    in_table = true;
                    table_rows.clear();
                }
                
                // Format table row
                let formatted_row = format!("| {} |", line.trim());
                table_rows.push(formatted_row);
            } else if in_table && line.trim().is_empty() {
                // End of table
                if table_rows.len() > 1 {
                    // Add table header separator if missing
                    if table_rows.len() > 1 && !table_rows[1].contains("---") {
                        let header_separator = self.generate_table_separator(&table_rows[0]);
                        table_rows.insert(1, header_separator);
                    }
                    
                    processed_lines.extend(table_rows.clone());
                    warnings.push("Table formatting enhanced - please verify structure".to_string());
                } else {
                    processed_lines.extend(table_rows.clone());
                }
                
                processed_lines.push(line.to_string());
                in_table = false;
                table_rows.clear();
            } else if in_table {
                // Non-table line while in table - end table
                if !table_rows.is_empty() {
                    processed_lines.extend(table_rows.clone());
                }
                processed_lines.push(line.to_string());
                in_table = false;
                table_rows.clear();
            } else {
                processed_lines.push(line.to_string());
            }
        }
        
        // Handle table at end of document
        if in_table && !table_rows.is_empty() {
            processed_lines.extend(table_rows);
        }
        
        processed_lines.join("\n")
    }

    /// Generate table separator row
    fn generate_table_separator(&self, header_row: &str) -> String {
        let column_count = header_row.matches('|').count().saturating_sub(1);
        let separators: Vec<String> = (0..column_count).map(|_| "---".to_string()).collect();
        format!("| {} |", separators.join(" | "))
    }

    /// Process image references and create placeholders
    fn process_image_references(&self, content: String, warnings: &mut Vec<String>) -> String {
        let processed = content;
        let mut image_count = 0;
        
        // Simple pattern matching for common image references
        let lines: Vec<&str> = processed.lines().collect();
        let mut processed_lines = Vec::new();
        
        for line in lines {
            let mut line_processed = line.to_string();
            
            // Look for HTML img tags
            if line.contains("<img") {
                image_count += 1;
                line_processed = format!("![Image {image_count}](images/image_{image_count}.png \"Image {image_count}\")");
            }
            // Look for custom image references
            else if line.contains("[image:") || line.contains("[Image:") {
                image_count += 1;
                line_processed = format!("![Image {image_count}](images/image_{image_count}.png \"Image {image_count}\")");
            }
            // Look for word-style image placeholders
            else if line.to_lowercase().contains("image") && (line.contains("[") || line.contains("{")) {
                image_count += 1;
                line_processed = format!("{line}\n\n![Image {image_count}](images/image_{image_count}.png \"Image {image_count}\")");
            }
            
            processed_lines.push(line_processed);
        }
        
        if image_count > 0 {
            warnings.push(format!("Found {image_count} image references - images need to be manually added to images/ directory"));
        }
        
        processed_lines.join("\n")
    }

    /// Add structure comments for complex documents
    fn add_structure_comments(&self, content: String, metadata: &DocumentMetadata) -> String {
        let mut result = String::new();
        
        // Add document metadata comment
        result.push_str("<!-- Document Structure Information -->\n");
        result.push_str(&format!("<!-- Chapters: {} -->\n", metadata.chapter_count));
        result.push_str(&format!("<!-- Word Count: {} -->\n", metadata.word_count));
        if metadata.has_tables {
            result.push_str("<!-- Contains Tables: Yes -->\n");
        }
        if metadata.has_images {
            result.push_str("<!-- Contains Images: Yes -->\n");
        }
        result.push_str("<!-- End Document Structure -->\n\n");
        
        result.push_str(&content);
        result
    }

    /// Post-process markdown to improve formatting and structure
    fn post_process_markdown(&self, markdown: String, warnings: &mut Vec<String>) -> String {
        let mut processed = markdown;
        
        // Clean up excessive whitespace
        processed = processed
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n");
        
        // Normalize heading levels (ensure proper hierarchy)
        processed = self.normalize_heading_levels(processed, warnings);
        
        // Improve table formatting if present
        if processed.contains('|') {
            processed = self.improve_table_formatting(processed);
        }
        
        // Clean up list formatting
        processed = self.improve_list_formatting(processed);
        
        processed
    }

    /// Normalize heading levels to ensure proper hierarchy
    fn normalize_heading_levels(&self, content: String, warnings: &mut Vec<String>) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut processed_lines = Vec::new();
        let mut heading_levels = Vec::new();
        
        for line in lines {
            if line.trim().starts_with('#') {
                let level = line.chars().take_while(|&c| c == '#').count();
                heading_levels.push(level);
                
                // Ensure heading hierarchy is logical
                let normalized_level = if heading_levels.len() == 1 {
                    1
                } else {
                    let prev_level = heading_levels[heading_levels.len() - 2];
                    if level > prev_level + 1 {
                        warnings.push("Heading hierarchy normalized - skipped levels detected".to_string());
                        prev_level + 1
                    } else {
                        level.min(6) // Maximum heading level
                    }
                };
                
                let heading_text = line.trim_start_matches('#').trim();
                processed_lines.push(format!("{} {}", "#".repeat(normalized_level), heading_text));
            } else {
                processed_lines.push(line.to_string());
            }
        }
        
        processed_lines.join("\n")
    }

    /// Improve table formatting
    fn improve_table_formatting(&self, content: String) -> String {
        // This is a basic implementation - could be enhanced further
        content
            .lines()
            .map(|line| {
                if line.contains('|') && !line.trim().starts_with('|') {
                    format!("| {} |", line.trim())
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Improve list formatting
    fn improve_list_formatting(&self, content: String) -> String {
        content
            .lines()
            .map(|line| {
                let trimmed = line.trim();
                if trimmed.starts_with('-') || trimmed.starts_with('*') || trimmed.starts_with('+') {
                    // Ensure consistent list marker
                    let content = trimmed[1..].trim();
                    format!("- {content}")
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get supported file formats
    pub fn supported_formats() -> Vec<&'static str> {
        vec!["docx", "doc"]
    }

    /// Validate if a file format is supported
    pub fn is_format_supported(filename: &str) -> bool {
        let extension = Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        
        Self::supported_formats().contains(&extension.as_str())
    }
}

impl Default for DocumentImportService {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a document file for import
#[derive(Debug, Clone)]
pub struct DocumentFile {
    pub filename: String,
    pub content: DocumentContent,
}

/// Content of a document file
#[derive(Debug, Clone)]
pub enum DocumentContent {
    Bytes(Vec<u8>),
    FilePath(PathBuf),
}

/// Result of importing multiple documents
#[derive(Debug, Clone)]
pub struct MultiDocumentImportResult {
    pub project_id: Uuid,
    pub successful_imports: Vec<SingleDocumentImportResult>,
    pub errors: Vec<ImportError>,
    pub warnings: Vec<String>,
    pub total_files: usize,
    pub processing_time_ms: u64,
}

/// Result of importing a single document
#[derive(Debug, Clone)]
pub struct SingleDocumentImportResult {
    pub filename: String,
    pub language: String,
    pub chapter_id: Uuid,
    pub success: bool,
    pub messages: Vec<String>,
    pub warnings: Vec<String>,
    pub processing_time_ms: u64,
}

/// Import error for a specific file
#[derive(Debug, Clone)]
pub struct ImportError {
    pub filename: String,
    pub error: String,
}

/// Progress information for import operations
#[derive(Debug, Clone)]
pub struct ImportProgress {
    pub step: String,
    pub progress_percent: u8,
    pub message: String,
}

/// Enhanced document metadata for conversion processing
#[derive(Debug, Clone)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub created_date: Option<DateTime<Utc>>,
    pub modified_date: Option<DateTime<Utc>>,
    pub word_count: usize,
    pub page_count: usize,
    pub has_tables: bool,
    pub has_images: bool,
    pub chapter_count: usize,
    pub languages: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[test]
    fn test_supported_formats() {
        let formats = DocumentImportService::supported_formats();
        assert!(formats.contains(&"docx"));
        assert!(formats.contains(&"doc"));
    }

    #[test]
    fn test_format_validation() {
        assert!(DocumentImportService::is_format_supported("document.docx"));
        assert!(DocumentImportService::is_format_supported("document.doc"));
        assert!(DocumentImportService::is_format_supported("Document.DOCX"));
        assert!(!DocumentImportService::is_format_supported("document.txt"));
        assert!(!DocumentImportService::is_format_supported("document.pdf"));
    }

    #[tokio::test]
    async fn test_import_nonexistent_file() {
        let mut service = DocumentImportService::new();
        let import_request = DocumentImportRequest {
            title: "Test Document".to_string(),
            target_languages: vec!["en".to_string(), "de".to_string()],
            source_language: "en".to_string(),
            extract_images: false,
            preserve_formatting: true,
        };

        let result = service.import_docx_file("nonexistent.docx", import_request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multi_document_import_validation() {
        let mut service = DocumentImportService::new();
        let project_id = Uuid::new_v4();
        
        // Create test files with unsupported format
        let files = vec![
            DocumentFile {
                filename: "test.txt".to_string(),
                content: DocumentContent::Bytes(b"test content".to_vec()),
            },
            DocumentFile {
                filename: "valid.docx".to_string(),
                content: DocumentContent::Bytes(b"valid content".to_vec()),
            },
        ];

        let language_mapping = HashMap::from([
            ("test.txt".to_string(), "en".to_string()),
            ("valid.docx".to_string(), "es".to_string()),
        ]);

        let result = service.import_multi_language_documents(
            files,
            language_mapping,
            project_id,
            None,
        ).await;

        assert!(result.is_ok());
        let import_result = result.unwrap();
        
        // Should have one error for unsupported format
        assert_eq!(import_result.errors.len(), 1);
        assert_eq!(import_result.errors[0].filename, "test.txt");
        assert!(import_result.errors[0].error.contains("Unsupported file format"));
    }

    #[tokio::test]
    async fn test_multi_document_import_missing_language_mapping() {
        let mut service = DocumentImportService::new();
        let project_id = Uuid::new_v4();
        
        let files = vec![
            DocumentFile {
                filename: "test.docx".to_string(),
                content: DocumentContent::Bytes(b"test content".to_vec()),
            },
        ];

        // Empty language mapping
        let language_mapping = HashMap::new();

        let result = service.import_multi_language_documents(
            files,
            language_mapping,
            project_id,
            None,
        ).await;

        assert!(result.is_ok());
        let import_result = result.unwrap();
        
        // Should have one error for missing language mapping
        assert_eq!(import_result.errors.len(), 1);
        assert_eq!(import_result.errors[0].filename, "test.docx");
        assert!(import_result.errors[0].error.contains("No language mapping"));
    }

    #[tokio::test]
    async fn test_progress_callback() {
        let mut service = DocumentImportService::new();
        let project_id = Uuid::new_v4();
        
        let files = vec![
            DocumentFile {
                filename: "test1.docx".to_string(),
                content: DocumentContent::Bytes(b"test content 1".to_vec()),
            },
            DocumentFile {
                filename: "test2.docx".to_string(),
                content: DocumentContent::Bytes(b"test content 2".to_vec()),
            },
        ];

        let language_mapping = HashMap::from([
            ("test1.docx".to_string(), "en".to_string()),
            ("test2.docx".to_string(), "es".to_string()),
        ]);

        let progress_updates = Arc::new(Mutex::new(Vec::new()));
        let progress_updates_clone = progress_updates.clone();
        
        let progress_callback = Arc::new(Mutex::new(move |progress: ImportProgress| {
            let updates = progress_updates_clone.clone();
            tokio::spawn(async move {
                updates.lock().await.push(progress);
            });
        }));

        let result = service.import_multi_language_documents(
            files,
            language_mapping,
            project_id,
            Some(progress_callback),
        ).await;

        assert!(result.is_ok());
        
        // Check that progress updates were called
        let updates = progress_updates.lock().await;
        assert!(!updates.is_empty());
        
        // Should have validation, processing, and completion updates
        assert!(updates.iter().any(|p| p.step.contains("Validating")));
        assert!(updates.iter().any(|p| p.step.contains("completed")));
    }

    #[test]
    fn test_sentence_boundary_detection() {
        let service = DocumentImportService::new();
        
        let text = "This is the first sentence. This is the second sentence! Is this a question?";
        let boundaries = service.detect_sentence_boundaries(text);
        
        // Should detect sentence boundaries at periods, exclamation marks, and question marks
        assert!(boundaries.len() > 3); // Start + at least 3 sentence endings
        assert_eq!(boundaries[0], 0); // Should start at 0
        assert_eq!(boundaries[boundaries.len() - 1], text.len()); // Should end at text length
    }

    #[test]
    fn test_heading_normalization() {
        let service = DocumentImportService::new();
        let mut warnings = Vec::new();
        
        let content = "# Main Title\n### Skipped Level\n## Proper Level".to_string();
        let normalized = service.normalize_heading_levels(content, &mut warnings);
        
        // Should normalize the skipped heading level
        assert!(normalized.contains("## Skipped Level"));
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_list_formatting_improvement() {
        let service = DocumentImportService::new();
        
        let content = "* First item\n+ Second item\n- Third item".to_string();
        let improved = service.improve_list_formatting(content);
        
        // Should normalize all list markers to dashes
        let lines: Vec<&str> = improved.lines().collect();
        assert!(lines.iter().all(|line| line.starts_with("- ")));
    }

    #[test]
    fn test_document_file_creation() {
        let file = DocumentFile {
            filename: "test.docx".to_string(),
            content: DocumentContent::Bytes(vec![1, 2, 3, 4]),
        };
        
        assert_eq!(file.filename, "test.docx");
        match file.content {
            DocumentContent::Bytes(bytes) => assert_eq!(bytes, vec![1, 2, 3, 4]),
            _ => panic!("Expected bytes content"),
        }
    }

    #[test]
    fn test_import_error_creation() {
        let error = ImportError {
            filename: "test.docx".to_string(),
            error: "Test error message".to_string(),
        };
        
        assert_eq!(error.filename, "test.docx");
        assert_eq!(error.error, "Test error message");
    }

    #[test]
    fn test_import_progress_creation() {
        let progress = ImportProgress {
            step: "Processing".to_string(),
            progress_percent: 50,
            message: "Processing file".to_string(),
        };
        
        assert_eq!(progress.step, "Processing");
        assert_eq!(progress.progress_percent, 50);
        assert_eq!(progress.message, "Processing file");
    }

    #[test]
    fn test_document_metadata_creation() {
        let metadata = DocumentMetadata {
            title: Some("Test Document".to_string()),
            author: Some("Test Author".to_string()),
            created_date: None,
            modified_date: None,
            word_count: 1000,
            page_count: 5,
            has_tables: true,
            has_images: false,
            chapter_count: 3,
            languages: vec!["en".to_string()],
        };
        
        assert_eq!(metadata.title, Some("Test Document".to_string()));
        assert_eq!(metadata.word_count, 1000);
        assert!(metadata.has_tables);
        assert!(!metadata.has_images);
        assert_eq!(metadata.chapter_count, 3);
    }

    #[test]
    fn test_chapter_heading_detection() {
        let service = DocumentImportService::new();
        
        assert!(service.is_chapter_heading("Chapter 1: Introduction"));
        assert!(service.is_chapter_heading("Section 2.1"));
        assert!(service.is_chapter_heading("Part A"));
        assert!(service.is_chapter_heading("1. Overview"));
        assert!(!service.is_chapter_heading("Simple heading without indicators"));
    }

    #[test]
    fn test_table_separator_generation() {
        let service = DocumentImportService::new();
        
        let header_row = "| Name | Age | City |";
        let separator = service.generate_table_separator(header_row);
        
        assert_eq!(separator, "| --- | --- | --- |");
    }

    #[test]
    fn test_enhanced_table_formatting() {
        let service = DocumentImportService::new();
        let mut warnings = Vec::new();
        
        let content = "Name | Age | City\nJohn | 25 | NYC\nJane | 30 | LA".to_string();
        let formatted = service.enhance_table_formatting(content, &mut warnings);
        
        assert!(formatted.contains("| Name | Age | City |"));
        assert!(formatted.contains("| John | 25 | NYC |"));
        assert!(formatted.contains("| Jane | 30 | LA |"));
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_image_reference_processing() {
        let service = DocumentImportService::new();
        let mut warnings = Vec::new();
        
        let content = "Here is an image: <img src=\"test.jpg\">\nAnd another: [image: diagram]".to_string();
        let processed = service.process_image_references(content, &mut warnings);
        
        assert!(processed.contains("![Image 1](images/image_1.png"));
        assert!(processed.contains("![Image 2](images/image_2.png"));
        assert!(!warnings.is_empty());
        assert!(warnings[0].contains("Found 2 image references"));
    }

    #[test]
    fn test_heading_normalization_with_chapters() {
        let service = DocumentImportService::new();
        let mut warnings = Vec::new();
        
        let content = "# Introduction\n## Overview\n# Chapter 2\n### Details".to_string();
        let normalized = service.normalize_headings_with_chapters(content, &mut warnings);
        
        assert!(normalized.contains("# Chapter 1: Introduction"));
        assert!(normalized.contains("# Chapter 2: Chapter 2"));
        assert!(normalized.contains("## Overview"));
    }

    #[test]
    fn test_structure_comments_addition() {
        let service = DocumentImportService::new();
        let metadata = DocumentMetadata {
            title: Some("Test".to_string()),
            author: None,
            created_date: None,
            modified_date: None,
            word_count: 1000,
            page_count: 5,
            has_tables: true,
            has_images: true,
            chapter_count: 5,
            languages: vec!["en".to_string()],
        };
        
        let content = "# Test Content".to_string();
        let with_comments = service.add_structure_comments(content, &metadata);
        
        assert!(with_comments.contains("<!-- Document Structure Information -->"));
        assert!(with_comments.contains("<!-- Chapters: 5 -->"));
        assert!(with_comments.contains("<!-- Word Count: 1000 -->"));
        assert!(with_comments.contains("<!-- Contains Tables: Yes -->"));
        assert!(with_comments.contains("<!-- Contains Images: Yes -->"));
    }

    #[tokio::test]
    async fn test_enhanced_docx_conversion_with_metadata() {
        let service = DocumentImportService::new();
        
        // Create a simple test document content
        let test_content = b"Test DOCX content with <w:tbl> table and <w:drawing> image";
        let mut warnings = Vec::new();
        
        // Test metadata extraction
        let metadata = service.extract_document_metadata(test_content, "test.docx", &mut warnings);
        assert!(metadata.is_ok());
        
        let metadata = metadata.unwrap();
        assert!(metadata.has_tables);
        assert!(metadata.has_images);
        assert_eq!(metadata.title, Some("test".to_string()));
        assert!(!warnings.is_empty()); // Should have warnings about tables and images
    }
}
